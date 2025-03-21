use clap::Subcommand;
use std::path::PathBuf;

mod cat_file;
mod hash_object;
mod init;
mod ls_tree;

#[derive(Subcommand, Debug)]
pub enum Command {
    Init,
    CatFile {
        #[clap(short = 'p')]
        pretty_print: bool,

        object_key: String,
    },
    HashObject {
        file_path: PathBuf,

        #[clap(short = 'w')]
        write: bool,
    },
    LsTree {
        #[clap(long = "name-only")]
        name_only: bool,

        tree_hash: String,
    }
}

impl Command {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            Command::Init => init::invoke(),
            Command::CatFile {
                pretty_print,
                object_key,
            } => cat_file::invoke(pretty_print, &object_key),
            Command::HashObject { file_path, write } => hash_object::invoke(&file_path, write),
            Command::LsTree { name_only , tree_hash} => ls_tree::invoke(name_only, tree_hash)
        }
    }
}
