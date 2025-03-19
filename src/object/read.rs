use std::io::Read;

use flate2::read::ZlibDecoder;

struct BlobReader {
    cap: u64,
    len: u64,
    reader: Box<dyn Read>
}
struct TreeReader {}
struct CommitReader {}

impl Read for BlobReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.len == 0 {
            return Ok(0);
        }
        
        self.reader.read(buf)
    }
}

impl Read for TreeReader {
    fn read
}