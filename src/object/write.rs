use anyhow::Context;
use flate2::{write::ZlibEncoder, Compression};
use ignore::WalkBuilder;
use sha1::Digest;
use std::{
    cmp::Ordering,
    fs::{self, File},
    fmt::Write,
    io::Write as IOWrite,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use crate::config::parse_config_from_file;

// TODO: reorganize tree and commit writing code, isolate file writing functionality
// to use in all object writing code (write to tmp file and copy to real file + chmod to read-only)
struct HashObjectWriter {
    hasher: sha1::Sha1,
    writer: Box<dyn IOWrite>,
}

impl IOWrite for HashObjectWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        sha1::digest::Update::update(&mut self.hasher, buf);
        self.writer.write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(crate) fn write_tree(path: &Path) -> anyhow::Result<Option<[u8; 20]>> {
    let mut buf: Vec<u8> = Vec::new();
    for entry in WalkBuilder::new(path)
        .max_depth(Some(1))
        .standard_filters(false)
        .hidden(false)
        .parents(false)
        .ignore(false)
        .git_exclude(true)
        .git_ignore(true)
        .git_global(true)
        .filter_entry(|path| path.file_name() != ".git")
        .sort_by_file_path(|file1, file2| {
            // can unwrap since these files are not .. (parent)
            let filename1 = file1.file_name().unwrap();
            let filename1 = filename1.as_encoded_bytes();
            let filename2 = file2.file_name().unwrap();
            let filename2 = filename2.as_encoded_bytes();

            let len = std::cmp::min(filename1.len(), filename2.len());
            let cmp = filename1[..len].cmp(&filename2[..len]);
            match cmp {
                Ordering::Equal => {
                    let c1 = if filename1.len() == len && file1.is_dir() {
                        '/'
                    } else {
                        '\0'
                    };
                    let c2 = if filename2.len() == len && file2.is_dir() {
                        '/'
                    } else {
                        '\0'
                    };
                    c1.cmp(&c2)
                }
                _ => cmp,
            }
        })
        .build()
        .skip(1)
    {
        let Ok(entry) = entry else {
            continue;
        };
        let metadata = entry.metadata()?;
        let entry_path = entry.path();
        let file_name = entry.file_name();

        let Some(file_type) = entry.file_type() else {
            continue;
        };
        let mode = if file_type.is_dir() {
            40000
        } else if file_type.is_file() {
            // TODO: support readonly for windows
            if metadata.permissions().mode() & 0o111 != 0 {
                100755
            } else {
                100644
            }
        } else if file_type.is_symlink() {
            120000
        } else {
            anyhow::bail!("Unknown file type: {:?}", file_type)
        };
        // TODO: To append buf with entry data
        buf.extend(mode.to_string().as_bytes());
        buf.push(b' ');
        buf.extend(file_name.as_encoded_bytes());
        buf.push(0);

        let entry_hash = if file_type.is_dir() {
            let Some(sub_dir_hash) =
                write_tree(entry_path).context("Calculate hash for subdirectory")?
            else {
                continue;
            };
            sub_dir_hash
        } else if file_type.is_file() || file_type.is_symlink() {
            write_blob(entry_path)?
        } else {
            anyhow::bail!("Unknown file type {:?}", file_type)
        };
        buf.extend(&entry_hash);
    }
    if buf.is_empty() {
        return Ok(None);
    }
    let size = buf.len();
    let header = format!("tree {size}\0");
    let mut hasher = sha1::Sha1::new();

    hasher.update(header.as_bytes());
    hasher.update(&buf);

    let hash_bytes = hasher.finalize();
    let hash = hex::encode(hash_bytes);

    let new_path = format!(".git/objects/{}/{}", &hash[..2], &hash[2..]);
    let new_path = PathBuf::new().join(new_path);
    let tmp_path = Path::new(".git/objects/.tmp_dir").join(path).join("tmp_file");
    fs::create_dir_all(tmp_path.parent().unwrap()).context("Create tmp dir")?;
    let write_file = File::create(&tmp_path).context("Create tmp file")?;
    let mut zlib_encoder = ZlibEncoder::new(write_file, Compression::default());

    write!(zlib_encoder, "tree {size}\0")?;
    zlib_encoder.write_all(&buf)?;
    fs::create_dir_all(new_path.parent().unwrap()).context("Create real dir")?;
    fs::rename(&tmp_path, &new_path).context("Copy tmp file to real location")?;

    Ok(Some(hash_bytes.into()))
}

fn write_blob(file_path: &Path) -> anyhow::Result<[u8; 20]> {
    calc_hash_object(file_path, true)
}

pub(crate) fn write_commit(tree_hash: String, parent: Option<String>, message: String) -> anyhow::Result<[u8; 20]> {
    let git_config = parse_config_from_file(File::open(".git/config").context("Open git config file")?);
    let Some(author) = git_config.get("user.name") else {
        anyhow::bail!("No author name found in config");
        };
    let Some(email) = git_config.get("user.email") else {
        anyhow::bail!("No author email found in config");
        };
    let mut commit = String::new();
    writeln!(commit, "tree {tree_hash}")?;
    if let Some(parent) = parent {
        writeln!(commit, "parent {parent}")?;
    };
    let now = chrono::Local::now();
    let timezone = now.offset().local_minus_utc() / 3600;
    let timezone = if timezone >= 0 {
        format!("+{:0>2}", timezone)
    } else {
        format!("-{:0>2}", -timezone)
    };
    writeln!(
        commit,
        "author {} <{}> {} {}00",
        author,
        email,
        now.timestamp(),
        timezone
    )?;
    writeln!(
        commit,
        "committer {} <{}> {} {}00\n",
        author,
        email,
        now.timestamp(),
        timezone
    )?;
    writeln!(commit, "{}", message)?;
    let size = commit.len();
    let header = format!("commit {size}\0");
    let mut hasher = sha1::Sha1::new();
    hasher.update(header.as_bytes());
    hasher.update(commit.as_bytes());
    let hash_bytes = hasher.finalize();
    let hash = hex::encode(hash_bytes);

    let tmp_file_path = Path::new(".git/objects/.tmp").join("commit_tmp");
    let tmp_dir_path = tmp_file_path.parent().unwrap();
    fs::create_dir_all(tmp_dir_path).context("Create temp path")?;
    let write_file = File::create(&tmp_file_path).context("Create tmp file")?;
    let mut zlib_encoder = ZlibEncoder::new(write_file, Compression::default());

    write!(zlib_encoder, "commit {size}\0")?;
    zlib_encoder.write_all(commit.as_bytes())?;
    fs::create_dir_all(format!(".git/objects/{}", &hash[..2]))
        .context("Creating object dir")?;
    fs::rename(
        &tmp_file_path,
        format!(".git/objects/{}/{}", &hash[..2], &hash[2..]),
    )
    .context("Move temp file to actual file")?;

    Ok(hash_bytes.into())
}

pub(crate) fn calc_hash_object(file_path: &Path, save_file: bool) -> anyhow::Result<[u8; 20]> {
    let metadata = fs::metadata(&file_path).context("Stating the file")?;
    let size = metadata.len();
    let mut file = File::open(file_path).context("Opening file")?;

    // create tmp file
    let tmp_file_path = Path::new(".git/objects/.tmp").join(file_path);
    let tmp_dir_path = tmp_file_path.parent().unwrap();
    let content_sink: Box<dyn IOWrite> = if save_file {
        fs::create_dir_all(tmp_dir_path).context("Create temp path")?;
        Box::new(File::create_new(&tmp_file_path).context("Creating temp file")?)
    } else {
        Box::new(std::io::sink())
    };
    let mut writer = HashObjectWriter {
        hasher: sha1::Sha1::new(),
        writer: Box::new(ZlibEncoder::new(content_sink, Compression::default())),
    };

    write!(writer, "blob {size}\0").context("Writing header")?;
    std::io::copy(&mut file, &mut writer).context("Writing file content")?;
    writer.flush()?;
    let hash_arr = writer.hasher.finalize();
    let hash = hex::encode(hash_arr);

    if save_file {
        fs::create_dir_all(format!(".git/objects/{}", &hash[..2]))
            .context("Creating object dir")?;
        fs::rename(
            &tmp_file_path,
            format!(".git/objects/{}/{}", &hash[..2], &hash[2..]),
        )
        .context("Move temp file to actual file")?;
    }
    Ok(hash_arr.into())
}
