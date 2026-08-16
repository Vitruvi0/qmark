use anyhow::Result;

use crate::cli::Shell;

/// The shell snippets live in `shell/` (version-controlled, shellcheck'd in CI)
/// and are embedded into the binary at compile time, so `qmark init` works
/// without any runtime files.
const ZSH_SNIPPET: &str = include_str!("../shell/qmark.zsh");
const BASH_SNIPPET: &str = include_str!("../shell/qmark.bash");

pub fn print_init(shell: Shell) -> Result<()> {
    let snippet = match shell {
        Shell::Zsh => ZSH_SNIPPET,
        Shell::Bash => BASH_SNIPPET,
    };
    print!("{snippet}");
    Ok(())
}
