use std::path::Path;

use anyhow::Ok;

use crate::object::write::write_tree;

pub(crate) fn invoke() -> anyhow::Result<()> {
    let hash = write_tree(Path::new("./"))?;
    let Some(hash) = hash else {
        anyhow::bail!("Empty dir")
    };
    println!("{}", hex::encode(hash));
    Ok(())
}
