use std::path::PathBuf;

use clap::{Parser, Subcommand};
use grit::command::{self, GitResult};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Init,
    HashObject {
        #[arg()]
        path: PathBuf,
    },
}

fn main() -> GitResult<()> {
    let cli = Cli::parse();

    match &cli.command {
        Command::Init => command::init()?,
        Command::HashObject { path } => command::hash_object(&path)?,
    }

    Ok(())
}
