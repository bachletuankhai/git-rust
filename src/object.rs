use std::fs;

use anyhow::Context;

pub enum FileType {
    Blob,
    Tree,
    Commit,
}

#[derive(Debug)]
pub struct FileTypeParseError;

impl std::fmt::Display for FileTypeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown object type")
    }
}

impl std::error::Error for FileTypeParseError {}
impl std::str::FromStr for FileType {
    type Err = FileTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "blob" => Ok(FileType::Blob),
            "tree" => Ok(FileType::Tree),
            "commit" => Ok(FileType::Commit),
            _ => Err(FileTypeParseError),
        }
    }
}

pub fn open(object_hash: &str) -> anyhow::Result<std::fs::File> {
    if object_hash.len() < 4 || object_hash.len() > 40 {
        anyhow::bail!("");
    }
    hex::decode(object_hash).context("Non-hex object hash")?;
    let dir_name = format!(".git/objects/{}", &object_hash[..2]);
    let object_name_pref = &object_hash[2..];
    if !fs::exists(&dir_name)? {
        anyhow::bail!("Not found: {object_hash}");
    }
    let mut count = 0;
    let mut buf: Vec<String> = Vec::new();
    for file in fs::read_dir(&dir_name)? {
        let file = file?;
        let file_name = file.file_name();
        let file_name = file_name
            .into_string()
            .expect("Filename should contain valid Unicode");
        if file_name.starts_with(object_name_pref) {
            count += 1;
            if count > 1 {
                anyhow::bail!("");
            }
            buf.push(file_name);
        }
    }
    let Some(file_name) = buf.first() else {
        anyhow::bail!("Not found: {object_hash}");
    };
    fs::File::open(format!("{dir_name}/{}", file_name)).context("Opening object file")
}

pub fn create_hex(object_hash: &[u8; 20]) -> anyhow::Result<fs::File> {
    let string_name = hex::encode(object_hash);
    crate::object::create_str(&string_name)
}

fn create_str(object_key: &str) -> anyhow::Result<fs::File> {
    fs::create_dir_all(format!(".git/objects/{}", &object_key[..2]))
        .with_context(|| format!("Create dir .git/objects/{}", &object_key[..2]))?;
    fs::File::create(format!(
        ".git/objects/{}/{}",
        &object_key[..2],
        &object_key[2..]
    ))
    .context("Create object file")
}

pub fn create(object_hash: &str) -> anyhow::Result<fs::File> {
    if object_hash.len() != 40 {
        anyhow::bail!("Object hash must be 40 hex char");
    }
    hex::decode(object_hash).context("Non-hex object hash")?;
    create_str(object_hash)
}
