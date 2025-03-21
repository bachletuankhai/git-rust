use std::io::BufReader;

use anyhow::Context;
use flate2::read::ZlibDecoder;

use crate::object::read::ReadOptions;

pub fn invoke(name_only: bool, tree_hash: String) -> anyhow::Result<()> {
    let object_key = tree_hash;
    let file = crate::object::open(&object_key)
        .with_context(|| format!("Not a valid object name: {object_key}"))?;

    let zlib_decoder = ZlibDecoder::new(file);
    let mut reader = BufReader::new(zlib_decoder);

    let (object_kind, size) =
        crate::object::read::parse_header(&mut reader).context("Parsing object header")?;

    match object_kind {
        crate::object::ObjectKind::Tree => {
            let opt = ReadOptions {
                tree_name_only: name_only,
            };
            let mut reader =
                crate::object::read::GitObjectReader::new(object_kind, reader, size, Some(&opt));
            let mut stdout = std::io::stdout().lock();

            // TODO: proper handling of commit and tree objects
            std::io::copy(&mut reader, &mut stdout).context("Printing file content")?;
            Ok(())
        }
        _ => {
            anyhow::bail!("not a tree object");
        }
    }
}
