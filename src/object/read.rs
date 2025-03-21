use std::{
    cmp::min,
    ffi::CStr,
    io::{BufRead, BufReader, Read, Take},
};

use crate::object::ObjectKind;

use super::TreeEntry;

use anyhow::Context;

pub struct GitObjectReader<'a, R: Read> {
    kind: ObjectKind,
    source: BufReader<Take<R>>,
    opt: Option<&'a ReadOptions>,
}

pub struct ReadOptions {
    pub tree_name_only: bool,
}

impl<'a, R: Read> GitObjectReader<'a, R> {
    pub fn new(
        kind: ObjectKind,
        source: R,
        size: u64,
        opt: Option<&'a ReadOptions>,
    ) -> GitObjectReader<'a, R> {
        GitObjectReader {
            kind,
            opt,
            source: BufReader::new(source.take(size)),
        }
    }
}

fn read_tree<R: Read>(
    source: &mut BufReader<R>,
    buf: &mut [u8],
    opt: Option<&ReadOptions>,
) -> std::io::Result<usize> {
    let mut write_pos = 0;
    let mut buf_vec: Vec<u8> = Vec::new();
    let mut buf_sha: [u8; 20] = [0; 20];
    while write_pos < buf.len() {
        let size = buf.len();
        // Each iteration tries to write 1 tree entry
        // <mode> <name>\0<20_byte_sha>

        // Write <mode> <name>
        let entry = source.read_until(0, &mut buf_vec)?;
        if entry == 0 {
            return Ok(write_pos);
        }

        // Read 20 bytes SHA
        source.read_exact(&mut buf_sha)?;

        let Ok(tree_entry) = TreeEntry::parse(&buf_vec, &buf_sha) else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid tree entry data",
            ));
        };

        let line = opt
            .filter(|opt| opt.tree_name_only)
            .map_or_else(
                || Some(format!("{}\n", tree_entry)),
                |_| Some(format!("{}\n", tree_entry.name)),
            )
            .unwrap(); // must be Some here
        let line = line.as_bytes();
        let max_write_len = min(size - write_pos, line.len());
        buf[write_pos..(write_pos + max_write_len)].copy_from_slice(&line[..max_write_len]);
        write_pos += max_write_len;

        buf_vec.clear();
    }
    Ok(write_pos)
}

pub fn parse_header<R: Read>(
    source: &mut BufReader<R>,
) -> anyhow::Result<(crate::object::ObjectKind, u64)> {
    let mut buf: Vec<u8> = Vec::new();
    source
        .read_until(0, &mut buf)
        .context("Reading git object header")?;

    let str = CStr::from_bytes_with_nul(&buf)
        .context("header should end with nul")?
        .to_str()
        .context("Convert CStr to str")?;

    let Some((file_type, size)) = str.split_once(' ') else {
        anyhow::bail!("Unknown header format: {str}, expecting '<object_type> <size>'");
    };
    let object_kind = file_type
        .parse::<crate::object::ObjectKind>()
        .context("Unknown object type: {file_type}")?;
    let size = size.parse::<u64>().context("Parsing content size")?;

    return Ok((object_kind, size));
}

impl<'a, R: Read> Read for GitObjectReader<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.kind {
            ObjectKind::Blob => self.source.read(buf),
            ObjectKind::Commit => self.source.read(buf),
            ObjectKind::Tree => read_tree(&mut self.source, buf, self.opt),
        }
    }
}
