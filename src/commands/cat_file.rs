use std::io::{stdout, BufReader};

use anyhow::Context;
use flate2::read::ZlibDecoder;

pub fn invoke(pretty_print: bool, object_key: &str) -> anyhow::Result<()> {
    anyhow::ensure!(pretty_print, "Missing flag: -p");

    let file = crate::object::open(&object_key)
        .with_context(|| format!("Not a valid object name: {object_key}"))?;

    let zlib_decoder = ZlibDecoder::new(file);
    let mut reader = BufReader::new(zlib_decoder);

    let (object_kind, size) =
        crate::object::read::parse_header(&mut reader).context("Parsing object header")?;

    let mut reader = crate::object::read::GitObjectReader::new(object_kind, reader, size, None);
    let mut stdout = stdout().lock();

    // TODO: proper handling of commit and tree objects
    std::io::copy(&mut reader, &mut stdout).context("Printing file content")?;
    Ok(())
}
