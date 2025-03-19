use anyhow::Context;
use std::fs;

pub fn invoke() -> anyhow::Result<()> {
    fs::create_dir(".git").context("Failed to create .git directory")?;
    fs::create_dir(".git/objects").context("Failed to create .git/objects directory")?;
    fs::create_dir(".git/refs").context("Failed to create .git/refs directory")?;
    fs::write(".git/HEAD", "ref: refs/heads/main\n").context("Failed to initialize HEAD ref")?;
    println!("Git repo inited!");
    Ok(())
}
