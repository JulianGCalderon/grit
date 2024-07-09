use std::path::PathBuf;

use clap::{Parser, Subcommand};
use grit::{command, repository::GitResult};

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
        path: PathBuf,
    },
    CatFile {
        hash: String,
    },
    UpdateIndex {
        path: PathBuf,
    },
    WriteTree,
    CommitTree {
        hash: String,
        #[arg(short, long)]
        parent: Option<String>,
        #[arg(short, long)]
        message: String,
    },
}

fn main() -> GitResult<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init => command::init()?,
        Command::HashObject { path } => command::hash_object(path)?,
        Command::CatFile { hash } => command::cat_file(hash)?,
        Command::UpdateIndex { path } => command::update_index(path)?,
        Command::WriteTree => command::write_tree()?,
        Command::CommitTree {
            hash,
            message,
            parent,
        } => command::commit_tree(hash, parent, message)?,
    }

    Ok(())
}
