use clap::{Parser, Subcommand, ValueEnum};

/// Cisco-style `?` help for your terminal, with AI explanations in plain English.
#[derive(Parser)]
#[command(name = "qmark", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Show contextual help for a (partial) command line, Cisco-style
    ///
    /// This is what the `?` key binding calls under the hood. You can also use it
    /// directly: `qmark suggest git` or `qmark suggest -- "ls -la"`.
    Suggest {
        /// The (partial) command line to get help for
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        line: Vec<String>,
    },
    /// Explain what a command line does, in plain English (AI-powered)
    Explain {
        /// The command line to explain
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        line: Vec<String>,
    },
    /// Print the shell integration snippet (add `eval "$(qmark init zsh)"` to your rc file)
    Init {
        /// Which shell to emit the snippet for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum Shell {
    Zsh,
    Bash,
}
