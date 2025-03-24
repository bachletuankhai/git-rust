use std::path::Path;

use crate::object::write::calc_hash_object;

pub(crate) fn invoke(file_path: &Path, write: bool) -> anyhow::Result<()> {
    let hash = calc_hash_object(file_path, write)?;
    println!("{hash}");
    Ok(())
}
