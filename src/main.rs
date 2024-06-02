use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Init,
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Command::Init => {
            println!("init");
        }
    }
}
