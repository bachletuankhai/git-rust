use std::{cmp::min, io::{BufRead, BufReader, Read, Take}};

struct BlobReader<R: Read> {
    source: R,
}

struct TreeReader<R: Read> {
    source: BufReader<R>,
}
impl<R: Read> TreeReader<R> {
    fn new(size: u64, source: R) -> TreeReader<Take<R>> {
        TreeReader {
            source: BufReader::new(source.take(size)),
        }
    }
}

struct CommitReader {}

impl<R: Read> Read for BlobReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.source.read(buf)
    }
}

impl<R: Read> Read for TreeReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let size = buf.len();
        let mut write_idx = 0;
        let mut buf_vec: Vec<u8> = Vec::new();
        let mut buf_sha: [u8; 20] = [0; 20];
        while write_idx < size {
            // Each iteration tries to write 1 tree entry
            // <mode> <name>\0<20_byte_sha>

            // Write <mode> <name>
            let mut entry = self.source.read_until(0, &mut buf_vec)?;
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
            if entry > (size - write_idx) {
                buf_vec.truncate(size - write_idx);
            }
            let slice = buf_vec.as_slice();
            let write_end = write_idx + slice.len();
            buf[write_idx..write_end].copy_from_slice(slice);

            write_idx = write_end;

            buf[write_idx] = b' ';
            write_idx += 1;

            buf_vec.clear();
            
            // Read 20 bytes SHA
            self.source.read_exact(&mut buf_sha)?;
            let sha_str = hex::encode(&buf_sha);
            let sha_str_bytes = sha_str.as_bytes();
            
            let max_write = min(size - write_idx, 40);
            buf[write_idx..(write_idx + max_write)].copy_from_slice(&sha_str_bytes[..max_write]);
            buf[write_idx] = b'\n';
            write_idx += max_write + 1;
        }
        Ok(write_idx)
    }
}

pub fn reader<R: Read>(kind: crate::object::ObjectKind, source: R, size: u64) -> Box<dyn Read> {
    match kind {
        crate::object::ObjectKind::Blob => Box::new(BlobReader { source }),
        crate::object::ObjectKind::Tree => Box::new(TreeReader::new(size, source)),
        crate::object::ObjectKind::Commit => Box::new(CommitReader {}),
    }
}
