pub(crate) fn invoke(
    tree_hash: String,
    parent: Option<String>,
    message: String,
) -> anyhow::Result<()> {
    let commit_hash = crate::object::write::write_commit(tree_hash, parent, message)?;
    let commit_hash = hex::encode(commit_hash);
    println!("{commit_hash}");
    Ok(())
}
