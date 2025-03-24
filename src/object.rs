use core::ffi;
use std::{fmt::Display, fs};

use anyhow::Context;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ObjectKind {
    Blob,
    Tree,
    Commit,
}

#[derive(Debug)]
pub(crate) struct FileTypeParseError;

impl std::fmt::Display for FileTypeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown object type")
    }
}

impl std::error::Error for FileTypeParseError {}
impl std::str::FromStr for ObjectKind {
    type Err = FileTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "blob" => Ok(ObjectKind::Blob),
            "tree" => Ok(ObjectKind::Tree),
            "commit" => Ok(ObjectKind::Commit),
            _ => Err(FileTypeParseError),
        }
    }
}

pub(crate) mod read;
pub(crate) mod write;

pub(crate) fn open(object_hash: &str) -> anyhow::Result<std::fs::File> {
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

pub(crate) struct TreeEntry<'a> {
    mode: &'a str,
    kind: String,
    name: &'a str,
    hash: String,
}

impl<'a> TreeEntry<'a> {
    pub(crate) fn parse(mode_name: &'a Vec<u8>, hash: &[u8; 20]) -> anyhow::Result<TreeEntry<'a>> {
        let str = ffi::CStr::from_bytes_with_nul(mode_name)?;
        let str = str.to_str().context("Converting Ctr to str")?;
        let Some((mode, name)) = str.split_once(' ') else {
            anyhow::bail!("Unknown tree entry header: {str}");
        };
        let kind = match mode {
            "40000" => String::from("tree"),
            "100644" | "100755" | "120000" => String::from("blob"),
            _ => anyhow::bail!("Unknown file mode: {mode}"),
        };
        Ok(TreeEntry {
            mode,
            kind,
            name,
            hash: hex::encode(hash),
        })
    }
}

impl<'a> Display for TreeEntry<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:0>6} {} {}\t{}",
            self.mode, self.kind, self.hash, self.name
        )
    }
}
