use anyhow::Context;
use flate2::{write::ZlibEncoder, Compression};
use ignore::WalkBuilder;
use sha1::Digest;
use std::{
    fs::{self, File, FileType},
    io::{Read, Write},
    path::Path,
};

struct HashObjectWriter {
    hasher: sha1::Sha1,
    writer: Box<dyn Write>,
}

impl Write for HashObjectWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        sha1::digest::Update::update(&mut self.hasher, buf);
        self.writer.write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn write_tree(path: &Path) -> anyhow::Result<[u8; 20]> {
    let read_dir = fs::read_dir(".").with_context(|| format!("Reading dir: {:?}", path))?;
    let mut buf: Vec<u8> = Vec::new();
    for entry in WalkBuilder::new(".")
        .standard_filters(false)
        .hidden(false)
        .parents(false)
        .ignore(false)
        .git_exclude(true)
        .git_ignore(true)
        .git_global(true)
        .filter_entry(|path| path.file_name() != ".git")
        .build()
    {
        let Ok(entry) = entry else {
            continue;
        };
        let metadata = entry.metadata()?;
        let entry_path = entry.path();
        let file_name = entry.file_name().to_string_lossy();

        let Some(file_type) = entry.file_type() else {
            continue;
        };
        let mode = if file_type.is_dir() {
            "40000"
        } else if file_type.is_file() {
            "100644"
        } else if file_type.is_symlink() {
            "120000"
        } else {
            anyhow::bail!("Unknown file type: {:?}", file_type)
        };
        // TODO: To append buf with entry data
        write!(&mut buf, "{mode} {file_name}")?;

        let entry_hash = if file_type.is_dir() {
            write_tree(entry_path)
        } else if file_type.is_file() || file_type.is_symlink() {
            write_blob(entry_path)
        } else {
            Err(anyhow::format_err!("Unknown file type {:?}", file_type))
        }?;
        write!(&mut buf, entry_hash)?;

    }
    Ok([0; 20])
}

fn write_blob(file_path: &Path) -> anyhow::Result<[u8; 20]> {
    calc_hash_object(file_path, true)
}

pub(crate) fn calc_hash_object(file_path: &Path, save_file: bool) -> anyhow::Result<[u8; 20]> {
    let metadata = fs::metadata(&file_path).context("Stating the file")?;
    let size = metadata.len();
    let mut file = File::open(file_path).context("Opening file")?;

    // create tmp file
    let tmp_file_path = Path::new(".git/objects/.tmp").join(file_path);
    let tmp_dir_path = tmp_file_path.parent().unwrap();
    let content_sink: Box<dyn Write> = if save_file {
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
