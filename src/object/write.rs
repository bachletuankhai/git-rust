use anyhow::Context;
use flate2::{write::ZlibEncoder, Compression};
use sha1::Digest;
use std::{
    fs::{self, File},
    io::Write,
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

fn write_tree(path: &Path) {}

fn write_blob(file_path: &Path) -> anyhow::Result<String> {
    calc_hash_object(file_path, true)
}

pub(crate) fn calc_hash_object(file_path: &Path, save_file: bool) -> anyhow::Result<String> {
    let tmp_file_path = Path::new(".git/objects/.tmp").join(file_path);
    let tmp_dir_path = tmp_file_path.parent().unwrap();

    let metadata = fs::metadata(&file_path).context("Stating the file")?;
    let size = metadata.len();
    let mut file = File::open(file_path).context("Opening file")?;
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
    let hash = writer.hasher.finalize();
    let hash = hex::encode(hash);
    if save_file {
        fs::create_dir_all(format!(".git/objects/{}", &hash[..2]))
            .context("Creating object dir")?;
        fs::rename(
            &tmp_file_path,
            format!(".git/objects/{}/{}", &hash[..2], &hash[2..]),
        )
        .context("Move temp file to actual file")?;
    }
    Ok(hash)
}
