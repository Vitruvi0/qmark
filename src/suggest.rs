use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

/// Maximum number of harvested help lines to show before truncating.
const MAX_LINES: usize = 40;

/// Show contextual help for a (partial) command line.
///
/// Strategy: take the `base [subcommand]` chain already typed, harvest the help
/// output for the deepest chain that yields structured entries, and show those.
/// With `interactive`, entries become a picker on the tty and the chosen entry
/// is printed to stdout (the shell widget inserts it into the line).
pub fn run(line: &str, interactive: bool) -> Result<()> {
    // In interactive mode stdout is captured by the shell widget and inserted
    // into the command line, so anything that is not a chosen entry must go to
    // stderr (which stays on the tty).
    let mut out: Box<dyn Write> = if interactive {
        Box::new(std::io::stderr())
    } else {
        Box::new(std::io::stdout())
    };

    let line = line.trim();
    if line.is_empty() {
        writeln!(out, "Type part of a command first, then `?` — e.g. `git ?`")?;
        return Ok(());
    }

    let chain = command_chain(line);
    let base = chain[0];

    let (title, text, mut entries) = if in_path(base) {
        harvest(&chain)?
    } else if let Some(text) = builtin_help(base) {
        // Shell builtins (cd, export, alias, ...) are not files in PATH;
        // bash's `help` documents them (options are tab-separated, which the
        // parser handles).
        let entries = parse_entries(&text);
        (base.to_string(), text, entries)
    } else if crate::curated::entries(base).is_some() {
        // Not installed, but we ship a curated table (e.g. security tools) —
        // still worth answering `?`.
        (base.to_string(), String::new(), Vec::new())
    } else {
        bail!("`{base}` not found in PATH — no help available");
    };
    if low_quality(&entries) {
        // Built-in rows for commands whose help is unparseable (or parses
        // into description-less junk, e.g. GNU find's operator summary).
        if let Some(rows) = crate::curated::entries(base) {
            entries = rows;
        }
    }

    if interactive && !entries.is_empty() {
        if let Some(mut tty) = crate::menu::tty() {
            if let Some(choice) = crate::menu::pick(&mut tty, &title, &entries)? {
                println!("{choice}");
            }
            return Ok(());
        }
        // No usable tty (piped, tests, dumb terminal) — fall through to the list.
    }

    print_plain(&mut out, &title, &text, &entries, line)
}

/// Resolve the command chain for `line` and harvest its help entries, for use
/// as grounding by `ai::explain`. Reuses the same resolution order as `run`
/// (real `--help`, shell builtin, curated fallback) but never prompts, never
/// prints, and never fails loudly: any failure — unknown command, unparseable
/// help, no entries at all — is simply `None`. Grounding is a nice-to-have,
/// not a requirement (spec §4).
pub fn harvest_entries(line: &str) -> Option<(String, Vec<Entry>)> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let chain = command_chain(line);
    // A line that starts with a flag (`-x foo`, bare `-`) yields an empty
    // chain — nothing to look up, and no base command to panic on.
    let &base = chain.first()?;

    let (title, mut entries) = if in_path(base) {
        let (title, _text, entries) = harvest(&chain).ok()?;
        (title, entries)
    } else if let Some(text) = builtin_help(base) {
        (base.to_string(), parse_entries(&text))
    } else {
        (base.to_string(), crate::curated::entries(base)?)
    };

    if low_quality(&entries) {
        if let Some(rows) = crate::curated::entries(base) {
            entries = rows;
        }
    }
    if entries.is_empty() {
        return None;
    }
    Some((title, entries))
}

/// Run the deepest help invocation that yields structured entries.
///
/// Candidates, in order: `<base> <sub> --help`, `<base> <sub> -h` (git-style
/// short help), `<base> --help`. Returns (title, raw help text, parsed entries).
fn harvest(chain: &[&str]) -> Result<(String, String, Vec<Entry>)> {
    let mut candidates: Vec<(Vec<&str>, &str)> = vec![(chain.to_vec(), "--help")];
    if chain.len() > 1 {
        candidates.push((chain.to_vec(), "-h"));
        candidates.push((vec![chain[0]], "--help"));
    }

    let mut first: Option<(String, String)> = None;
    for (cmd, flag) in candidates {
        let Ok(text) = help_output(&cmd, flag) else {
            continue;
        };
        let title = cmd.join(" ");
        let entries = parse_entries(&text);
        if !entries.is_empty() {
            return Ok((title, text, entries));
        }
        first.get_or_insert((title, text));
    }
    let (title, text) =
        first.with_context(|| format!("`{} --help` produced no output", chain.join(" ")))?;
    Ok((title, text, Vec::new()))
}

/// Run `<cmd...> <flag>` (stdin closed, pagers disabled) and return its output.
fn help_output(cmd: &[&str], flag: &str) -> Result<String> {
    let output = Command::new(cmd[0])
        .args(&cmd[1..])
        .arg(flag)
        .stdin(Stdio::null())
        // Some tools (git, systemctl, ...) page their help output; force plain stdout.
        .env("PAGER", "cat")
        .env("GIT_PAGER", "cat")
        .output()
        .with_context(|| format!("failed to run `{} {flag}`", cmd.join(" ")))?;

    // Plenty of tools print usage on stderr (and/or exit non-zero) — use whatever we got.
    let raw = if output.stdout.is_empty() {
        output.stderr
    } else {
        output.stdout
    };
    let text = String::from_utf8_lossy(&raw).into_owned();
    if text.trim().is_empty() {
        bail!("no output");
    }
    Ok(text)
}

/// Help text for a shell builtin via bash's `help`, or `None` if bash does
/// not know the name. The name is passed as a positional argument, never
/// interpolated into the script.
fn builtin_help(base: &str) -> Option<String> {
    let output = Command::new("bash")
        .args(["-c", r#"help "$1""#, "bash", base])
        .stdin(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() || output.stdout.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Non-interactive output: the structured list when we have one, otherwise the
/// raw help dump (old v0 behaviour).
fn print_plain(
    out: &mut dyn Write,
    title: &str,
    text: &str,
    entries: &[Entry],
    line: &str,
) -> Result<()> {
    writeln!(out, "── qmark ── help for `{title}` {}", "─".repeat(30))?;
    if entries.is_empty() {
        let lines: Vec<&str> = text.lines().collect();
        for l in lines.iter().take(MAX_LINES) {
            writeln!(out, "{l}")?;
        }
        if lines.len() > MAX_LINES {
            writeln!(
                out,
                "… {} more lines — run `{title} --help` for the full text.",
                lines.len() - MAX_LINES
            )?;
        }
    } else {
        let width = entries
            .iter()
            .take(MAX_LINES)
            .map(|e| e.display.chars().count())
            .max()
            .unwrap_or(0);
        for e in entries.iter().take(MAX_LINES) {
            writeln!(out, "  {:<width$}  {}", e.display, e.desc)?;
        }
        if entries.len() > MAX_LINES {
            writeln!(
                out,
                "… {} more — run `{title} --help`.",
                entries.len() - MAX_LINES
            )?;
        }
    }
    writeln!(out)?;
    writeln!(
        out,
        "Tip: `qmark explain \"{line}\"` gives a plain-English explanation (AI)."
    )?;
    Ok(())
}

/// One selectable suggestion harvested from help output.
#[derive(Debug, Clone)]
pub struct Entry {
    /// Text to insert into the command line when chosen (e.g. `clone`, `-f`).
    pub insert: String,
    /// Full left column as printed by the tool (e.g. `-f, --force`).
    pub display: String,
    /// Description column.
    pub desc: String,
}

/// The `base [subcommand...]` chain already typed: leading tokens up to the
/// first one that looks like a flag. `git mv ` → ["git", "mv"].
fn command_chain(line: &str) -> Vec<&str> {
    let mut chain = Vec::new();
    for tok in line.split_whitespace() {
        if tok.starts_with('-') {
            break;
        }
        chain.push(tok);
        // ponytail: two levels (base + subcommand) cover git/docker/cargo; deeper
        // chains (git stash push) just fall back to the parent's help.
        if chain.len() == 2 {
            break;
        }
    }
    chain
}

/// Heuristic parser for `--help` output: harvest indented `name  description`
/// and `-f, --flag  description` lines into structured entries.
///
/// Handles GNU coreutils ≥9.x quirks: ANSI/OSC escapes are stripped, and a
/// flag line without columns takes its description from the next line.
fn parse_entries(help: &str) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    // Index of a just-pushed flag entry still waiting for its description.
    let mut pending_desc: Option<usize> = None;
    for line in help.lines() {
        let line = strip_ansi(line);
        let indented = line.starts_with(' ') || line.starts_with('\t');
        let trimmed = line.trim();
        let columns = split_columns(trimmed);
        if trimmed.starts_with('-') && !trimmed.contains(" | ") {
            // No column separator: take the leading option-ish tokens as the
            // flag list, the rest as description (procps/which single-space
            // style; GNU two-line style leaves the description empty).
            let (left, desc) = columns.unwrap_or_else(|| split_options(trimmed));
            // git prints toggle flags as `--[no-]force`; insert the positive form.
            let positive = left.replace("[no-]", "");
            let first = positive
                .split([',', ' ', '=', '['])
                .next()
                .unwrap_or(&positive);
            // A prose bullet (`- see the manual`) parses to a bare `-`;
            // that is not a flag row.
            if first.trim_matches('-').is_empty() {
                pending_desc = None;
                continue;
            }
            entries.push(Entry {
                insert: first.to_string(),
                display: left.to_string(),
                desc: desc.to_string(),
            });
            pending_desc = desc.is_empty().then_some(entries.len() - 1);
            continue;
        }
        if let Some((left, desc)) = columns {
            if indented {
                // Subcommand rows: a single word in the left column (skip pure
                // numbers — those are "Exit status:" rows, not subcommands).
                let name = left.split_whitespace().next().unwrap_or("");
                if left == name
                    && name.chars().any(|c| c.is_ascii_alphabetic())
                    && name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                {
                    entries.push(Entry {
                        insert: name.to_string(),
                        display: name.to_string(),
                        desc: desc.to_string(),
                    });
                }
            }
        } else if indented && !trimmed.is_empty() {
            // GNU two-line style: bare indented text right after a desc-less flag.
            if let Some(i) = pending_desc {
                entries[i].desc = trimmed.to_string();
            }
        }
        pending_desc = None;
    }
    entries
}

/// Remove ANSI CSI sequences (`ESC [ ... letter`) and OSC sequences
/// (`ESC ] ... BEL` / `ESC ] ... ESC \`) — GNU coreutils decorates --help
/// with bold and OSC-8 hyperlinks.
fn strip_ansi(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some('[') => {
                chars.next();
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            Some(']') => {
                chars.next();
                while let Some(c) = chars.next() {
                    if c == '\x07' || (c == '\x1b' && chars.peek() == Some(&'\\')) {
                        chars.next_if_eq(&'\\');
                        break;
                    }
                }
            }
            _ => {
                chars.next();
            }
        }
    }
    out
}

/// Split a separator-less flag row after its leading option tokens: a token
/// belongs to the option list if it starts with `-` or ends with `,`.
fn split_options(row: &str) -> (&str, &str) {
    let mut end = 0;
    for tok in row.split_whitespace() {
        if !(tok.starts_with('-') || tok.ends_with(',')) {
            break;
        }
        // Tokens come from `row` itself, so pointer math gives the offset.
        end = tok.as_ptr() as usize - row.as_ptr() as usize + tok.len();
    }
    let (left, desc) = row.split_at(end);
    (left, desc.trim_start())
}

/// A harvest with no entries — or mostly description-less ones — is not worth
/// showing over a curated table (GNU find's operator summary parses into rows
/// with empty or placeholder descriptions).
fn low_quality(entries: &[Entry]) -> bool {
    let junk = entries
        .iter()
        // Empty, or "description" that is itself a run of options.
        .filter(|e| e.desc.is_empty() || e.desc.matches(" -").count() >= 2)
        .count();
    junk * 2 >= entries.len()
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

/// Split a help row into (left column, description) at the first run of
/// 2+ spaces or a tab (gawk separates columns with tabs).
fn split_columns(row: &str) -> Option<(&str, &str)> {
    let idx = match (row.find("  "), row.find('\t')) {
        (Some(s), Some(t)) => s.min(t),
        (s, t) => s.or(t)?,
    };
    let (left, rest) = row.split_at(idx);
    let desc = rest.trim_start_matches([' ', '\t']);
    if left.is_empty() || desc.is_empty() {
        return None;
    }
    Some((left, desc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_chain_takes_base_and_subcommands() {
        assert_eq!(command_chain("git mv "), vec!["git", "mv"]);
    }

    #[test]
    fn command_chain_stops_at_flags() {
        assert_eq!(command_chain("git -C repo mv "), vec!["git"]);
    }

    #[test]
    fn command_chain_single_token() {
        assert_eq!(command_chain("git "), vec!["git"]);
    }

    #[test]
    fn parse_entries_finds_subcommands() {
        let help = "\
usage: git [-v | --version] <command> [<args>]

These are common Git commands used in various situations:

start a working area (see also: git help tutorial)
   clone     Clone a repository into a new directory
   init      Create an empty Git repository
";
        let entries = parse_entries(help);
        assert_eq!(entries[0].insert, "clone");
        assert_eq!(entries[0].desc, "Clone a repository into a new directory");
        assert_eq!(entries[1].insert, "init");
    }

    #[test]
    fn parse_entries_finds_flags() {
        let help = "\
usage: git mv [<options>] <source>... <destination>

    -v, --verbose         be verbose
    -n, --dry-run         dry run
    -f, --force           force move/rename even if target exists
";
        let entries = parse_entries(help);
        assert_eq!(entries[0].insert, "-v");
        assert_eq!(entries[0].display, "-v, --verbose");
        assert_eq!(entries[0].desc, "be verbose");
        assert_eq!(entries[2].insert, "-f");
    }

    #[test]
    fn parse_entries_unwraps_git_no_prefix_flags() {
        let help = "    --[no-]sparse         allow updating entries\n";
        let entries = parse_entries(help);
        assert_eq!(entries[0].insert, "--sparse");
    }

    #[test]
    fn parse_entries_strips_ansi_and_reads_next_line_desc() {
        // GNU coreutils ≥9.x style: OSC-8 hyperlink + bold around the flag,
        // description alone on the following line.
        let help = "\
  \x1b]8;;https://gnu.org/ls-a\x1b\\\x1b[1m-a, --all\x1b[0m\x1b]8;;\x1b\\
         do not ignore entries starting with .
  \x1b]8;;https://gnu.org/ls-A\x1b\\\x1b[1m-A, --almost-all\x1b[0m\x1b]8;;\x1b\\
         do not list implied . and ..
";
        let entries = parse_entries(help);
        assert_eq!(entries[0].insert, "-a");
        assert_eq!(entries[0].display, "-a, --all");
        assert_eq!(entries[0].desc, "do not ignore entries starting with .");
        assert_eq!(entries[1].insert, "-A");
        assert_eq!(entries[1].desc, "do not list implied . and ..");
    }

    #[test]
    fn parse_entries_skips_numeric_exit_status_rows() {
        let help = "\
Exit status:
 0  if OK,
 1  if minor problems (e.g., cannot access subdirectory),
";
        assert!(parse_entries(help).is_empty());
    }

    #[test]
    fn parse_entries_splits_on_tabs() {
        // gawk separates columns with tabs, not spaces.
        let help = "\t-f progfile\t\t--file=progfile\n";
        let entries = parse_entries(help);
        assert_eq!(entries[0].insert, "-f");
        assert_eq!(entries[0].display, "-f progfile");
        assert_eq!(entries[0].desc, "--file=progfile");
    }

    #[test]
    fn parse_entries_rejects_alternation_lines() {
        // iproute2 usage style: `-h[uman-readable] | -iec | -j[son]` is a
        // syntax summary, not an option row.
        let help = "  -h[uman-readable] | -iec | -j[son] |\n";
        assert!(parse_entries(help).is_empty());
    }

    #[test]
    fn descriptionless_harvest_is_low_quality() {
        let junk = vec![Entry {
            insert: "-daystart".into(),
            display: "-daystart -follow -nowarn".into(),
            desc: String::new(),
        }];
        assert!(low_quality(&junk));
        assert!(low_quality(&[]));
        let good = vec![Entry {
            insert: "-v".into(),
            display: "-v, --verbose".into(),
            desc: "be verbose".into(),
        }];
        assert!(!low_quality(&good));
        // Half described, half bare (find-style) → still low quality.
        let mixed = [junk, good].concat();
        assert!(low_quality(&mixed));
        // find's operator summary: the "description" is just more options.
        let optiony = vec![Entry {
            insert: "-amin".into(),
            display: "-amin".into(),
            desc: "N -anewer FILE -atime N -cmin N".into(),
        }];
        assert!(low_quality(&optiony));
    }

    #[test]
    fn parse_entries_handles_single_space_separator() {
        // procps/which style: one space between the option list and the text.
        let help = "\
  -c, --container show container uptime
 --version, -[vV] Print version and exit successfully.
";
        let entries = parse_entries(help);
        assert_eq!(entries[0].display, "-c, --container");
        assert_eq!(entries[0].desc, "show container uptime");
        assert_eq!(entries[1].insert, "--version");
        assert_eq!(entries[1].desc, "Print version and exit successfully.");
    }

    #[test]
    fn parse_entries_rejects_dash_bullet_prose() {
        // A dash bullet with single spaces is prose, not a `-` flag.
        let help = "  - see the manual for details\n";
        assert!(parse_entries(help).is_empty());
    }

    #[test]
    fn harvest_entries_grounds_a_real_command() {
        let (title, entries) = harvest_entries("cargo ").expect("cargo is on PATH in tests");
        assert_eq!(title, "cargo");
        assert!(!entries.is_empty());
    }

    #[test]
    fn harvest_entries_falls_back_to_curated_table() {
        // ssh has no --help; the curated table still grounds it.
        let (title, entries) = harvest_entries("ssh -p 22 ").expect("curated ssh entries");
        assert_eq!(title, "ssh");
        assert!(entries.iter().any(|e| e.insert == "-p"));
    }

    #[test]
    fn harvest_entries_none_for_unknown_command() {
        assert!(harvest_entries("definitely-not-a-real-command-qmark").is_none());
    }

    #[test]
    fn harvest_entries_none_for_empty_line() {
        assert!(harvest_entries("   ").is_none());
    }

    #[test]
    fn harvest_entries_none_for_flag_first_line() {
        // command_chain returns an empty Vec when the line starts with a
        // flag; harvest_entries must not panic indexing into it.
        assert!(harvest_entries("-x foo").is_none());
        assert!(harvest_entries("-la").is_none());
        assert!(harvest_entries("-").is_none());
    }

    #[test]
    fn parse_entries_ignores_prose() {
        let help = "\
usage: thing

Some long prose paragraph that is not indented and has no columns.
";
        assert!(parse_entries(help).is_empty());
    }
}
