mod ai;
mod cli;
mod curated;
mod find;
mod menu;
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
        cli::Command::Suggest { interactive, line } => suggest::run(&line.join(" "), interactive),
        cli::Command::Explain { line } => ai::explain(&line.join(" ")),
        cli::Command::Init { shell } => shells::print_init(shell),
        cli::Command::Find { name, exact, roots } => find::run(&name, &roots, exact),
        cli::Command::Ai { command } => match command {
            cli::AiCommand::Status => ai::status(),
            cli::AiCommand::Model { name } => ai::model(name),
        },
    }
}
