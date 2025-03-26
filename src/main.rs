use clap::Parser;

use git_rust::commands::Command;
use ignore::{gitignore::GitignoreBuilder, WalkBuilder};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    args.command.execute()
}
