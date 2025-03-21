use std::{
    cmp::min,
    io::{BufRead, BufReader, Read, Take},
};

use crate::object::ObjectKind;

use super::TreeEntry;

pub struct GitObjectReader<R: Read> {
    kind: ObjectKind,
    source: BufReader<Take<R>>,
}

impl<R: Read> GitObjectReader<R> {
    pub fn new(kind: ObjectKind, source: R, size: u64) -> GitObjectReader<R> {
        GitObjectReader {
            kind,
            source: BufReader::new(source.take(size)),
        }
    }
}

fn read_tree<R: Read>(source: &mut BufReader<R>, buf: &mut [u8]) -> std::io::Result<usize> {
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
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid tree entry data"));
        };

        let line = format!("{tree_entry}\n");
        let line = line.as_bytes();
        let max_write_len = min(size - write_pos, line.len());
        buf[write_pos..(write_pos + max_write_len)].copy_from_slice(&line[..max_write_len]);
        write_pos += max_write_len;

        buf_vec.clear();
    }
    Ok(write_pos)
}

impl<R: Read> Read for GitObjectReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.kind {
            ObjectKind::Blob => self.source.read(buf),
            ObjectKind::Commit => self.source.read(buf),
            ObjectKind::Tree => read_tree(&mut self.source, buf),
        }
    }
}
