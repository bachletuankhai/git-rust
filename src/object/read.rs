use std::{
    cmp::min,
    io::{BufRead, BufReader, Read, Take},
};

use crate::object::ObjectKind;

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
    let size = buf.len();
    let mut write_pos = 0;
    let mut buf_vec: Vec<u8> = Vec::new();
    let mut buf_sha: [u8; 20] = [0; 20];
    while write_pos < size {
        // Each iteration tries to write 1 tree entry
        // <mode> <name>\0<20_byte_sha>

        // Write <mode> <name>
        let mut entry = source.read_until(0, &mut buf_vec)?;
        if entry == 0 {
            return Ok(write_pos);
        }
        let Some(last_item) = buf_vec.get(entry - 1) else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Read to buf_vec failed",
            ));
        };
        // Leave out the \0
        if *last_item == b'\0' {
            buf_vec.truncate(entry - 1);
            entry -= 1;
        }
        if entry > (size - write_pos) {
            buf_vec.truncate(size - write_pos);
        }
        let slice = buf_vec.as_slice();
        let write_end = write_pos + slice.len();
        buf[write_pos..write_end].copy_from_slice(slice);

        write_pos = write_end;

        buf[write_pos] = b'\t';
        write_pos += 1;

        buf_vec.clear();

        // Read 20 bytes SHA
        source.read_exact(&mut buf_sha)?;
        let sha_str = hex::encode(&buf_sha);
        let sha_str_bytes = sha_str.as_bytes();

        let max_write = min(size - write_pos, 40);
        buf[write_pos..(write_pos + max_write)].copy_from_slice(&sha_str_bytes[..max_write]);
        write_pos += max_write;

        buf[write_pos] = b'\n';
        write_pos += 1;
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
