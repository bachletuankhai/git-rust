use std::{
    ffi::CStr,
    io::{stdout, BufRead, BufReader, Read},
};

use anyhow::Context;
use flate2::read::ZlibDecoder;

pub fn invoke(pretty_print: bool, object_key: &str) -> anyhow::Result<()> {
    anyhow::ensure!(pretty_print, "Missing flag: -p");

    let file = crate::object::open(&object_key)
        .with_context(|| format!("Not a valid object name: {object_key}"))?;

    let zlib_decoder = ZlibDecoder::new(file);
    let mut reader = BufReader::new(zlib_decoder);
    let mut buf: Vec<u8> = Vec::new();
    reader
        .read_until(0, &mut buf)
        .context("Reading git object header")?;

    let str = CStr::from_bytes_with_nul(&buf)
        .context("header should end with nul")?
        .to_str()
        .context("Convert CStr to str")?;

    let Some((file_type, size)) = str.split_once(' ') else {
        anyhow::bail!("Unknown header format: {str}, expecting '<object_type> <size>'");
    };
    file_type
        .parse::<crate::object::ObjectKind>()
        .context("Unknown object type: {file_type}")?;

    // TODO: dynamic type for size, big files might need more than usize for content size
    let size = size.parse::<u64>().context("Parsing content size")?;

    let mut reader = reader.take(size);
    let mut stdout = stdout().lock();

    // TODO: proper handling of commit and tree objects
    std::io::copy(&mut reader, &mut stdout).context("Printing file content")?;
    Ok(())
}
