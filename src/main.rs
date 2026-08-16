mod ai;
mod cli;
mod shells;
mod suggest;

use anyhow::Result;
use clap::Parser;

fn main() {
    if let Err(err) = run() {
        eprintln!("qmark: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = cli::Cli::parse();
    match args.command {
        cli::Command::Suggest { line } => suggest::run(&line.join(" ")),
        cli::Command::Explain { line } => ai::explain(&line.join(" ")),
        cli::Command::Init { shell } => shells::print_init(shell),
    }
}
