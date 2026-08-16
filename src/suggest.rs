use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

/// Maximum number of harvested help lines to show before truncating.
const MAX_LINES: usize = 40;

/// Show contextual help for a (partial) command line.
///
/// v0 strategy: take the base command, run `<base> --help` (stdin closed, pagers
/// disabled) and show the first lines of its output. Subcommand-aware help and
/// richer sources (tldr pages, man) are on the roadmap.
pub fn run(line: &str) -> Result<()> {
    let line = line.trim();
    if line.is_empty() {
        println!("Type part of a command first, then `?` — e.g. `git ?`");
        return Ok(());
    }

    // `split_whitespace` yields at least one item for a non-empty trimmed string.
    let base = line
        .split_whitespace()
        .next()
        .expect("non-empty line has a first token");

    if !in_path(base) {
        bail!("`{base}` not found in PATH — no help available");
    }

    let output = Command::new(base)
        .arg("--help")
        .stdin(Stdio::null())
        // Some tools (git, systemctl, ...) page their help output; force plain stdout.
        .env("PAGER", "cat")
        .env("GIT_PAGER", "cat")
        .output()
        .with_context(|| format!("failed to run `{base} --help`"))?;

    // Plenty of tools print usage on stderr (and/or exit non-zero) — use whatever we got.
    let raw = if output.stdout.is_empty() {
        output.stderr
    } else {
        output.stdout
    };
    let text = String::from_utf8_lossy(&raw);
    let lines: Vec<&str> = text.lines().collect();

    if lines.is_empty() {
        bail!("`{base} --help` produced no output — try `man {base}`");
    }

    println!("── qmark ── help for `{base}` {}", "─".repeat(30));
    for l in lines.iter().take(MAX_LINES) {
        println!("{l}");
    }
    if lines.len() > MAX_LINES {
        println!(
            "… {} more lines — run `{base} --help` for the full text.",
            lines.len() - MAX_LINES
        );
    }
    println!();
    println!("Tip: `qmark explain \"{line}\"` gives a plain-English explanation (AI).");
    Ok(())
}

/// True if `cmd` resolves to a file (either a path, or found on `PATH`).
fn in_path(cmd: &str) -> bool {
    if cmd.contains('/') {
        return std::path::Path::new(cmd).is_file();
    }
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths).any(|dir| dir.join(cmd).is_file())
}
