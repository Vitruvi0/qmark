//! `qmark ai model` — pick a model interactively, or set one directly by
//! name (spec §3).

use std::io::Write;

use anyhow::{Result, bail};
use serde::Deserialize;

use crate::suggest::Entry;

use super::{base_url, fetch_models_body, model_file_path, resolve_model};

/// Curated download candidates, shipped in the binary — small models suited
/// to explaining a command line. Sizes are approximate (spec §3); the list
/// is a starting point, never a restriction — see [`HINT`].
const CURATED: &[(&str, &str, &str)] = &[
    ("qwen2.5-coder:1.5b", "~1 GB", "smallest, fastest"),
    ("qwen2.5-coder:7b", "~5 GB", "best quality of the three"),
    ("gemma3:4b", "~3 GB", ""),
];

const SEPARATOR: &str = "─ available to download ─";
const HINT: &str = "...or run `qmark ai model <any-name>` for a model not listed";

pub(crate) fn run(name: Option<String>) -> Result<()> {
    match name {
        Some(name) => set(&name),
        None => pick(),
    }
}

/// `qmark ai model <name>` — set directly, non-interactive, no validation
/// against any list (spec §3).
fn set(name: &str) -> Result<()> {
    write_model_file(name)?;
    println!("model set to `{name}`");
    Ok(())
}

fn write_model_file(name: &str) -> Result<()> {
    let path = model_file_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, format!("{name}\n"))?;
    Ok(())
}

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<ModelInfo>,
}

#[derive(Deserialize)]
struct ModelInfo {
    id: String,
}

/// Installed models from `GET {base_url}/models`, best-effort: any failure
/// (unreachable, malformed body) yields an empty list rather than an error —
/// the picker still works, showing only the curated download list.
fn fetch_installed(base_url: &str) -> Vec<String> {
    fetch_models_body(base_url)
        .ok()
        .and_then(|text| serde_json::from_str::<ModelsResponse>(&text).ok())
        .map(|r| r.data.into_iter().map(|m| m.id).collect())
        .unwrap_or_default()
}

/// Build the picker rows: installed models (marking `current`), a
/// separator, the curated download list, and a final hint row (spec §3).
/// The separator and hint rows carry an empty `insert` so picking one is a
/// no-op rather than writing a blank model.
fn build_entries(installed: &[String], current: &str) -> Vec<Entry> {
    let mut entries = Vec::new();
    for id in installed {
        let desc = if id == current {
            "installed · current"
        } else {
            "installed"
        };
        entries.push(Entry {
            insert: id.clone(),
            display: id.clone(),
            desc: desc.to_string(),
        });
    }
    entries.push(Entry {
        insert: String::new(),
        display: SEPARATOR.to_string(),
        desc: String::new(),
    });
    for (name, size, note) in CURATED {
        let desc = if note.is_empty() {
            (*size).to_string()
        } else {
            format!("{size} · {note}")
        };
        entries.push(Entry {
            insert: (*name).to_string(),
            display: (*name).to_string(),
            desc,
        });
    }
    entries.push(Entry {
        insert: String::new(),
        display: HINT.to_string(),
        desc: String::new(),
    });
    entries
}

fn pick() -> Result<()> {
    let base_url = base_url();
    let (current, _source) = resolve_model();
    let installed = fetch_installed(&base_url);
    let entries = build_entries(&installed, &current);

    if let Some(mut tty) = crate::menu::tty() {
        if let Some(choice) = crate::menu::pick(&mut tty, "model", &entries)? {
            if !choice.is_empty() {
                apply(&choice, &installed)?;
            }
        }
        return Ok(());
    }

    // No tty (piped, tests): plain grouped listing, no prompt, exit 0.
    print_plain(&entries);
    Ok(())
}

fn print_plain(entries: &[Entry]) {
    println!("── qmark ── model {}", "─".repeat(30));
    let width = entries
        .iter()
        .filter(|e| !e.desc.is_empty())
        .map(|e| e.display.chars().count())
        .max()
        .unwrap_or(0);
    for e in entries {
        if e.desc.is_empty() {
            println!("  {}", e.display);
        } else {
            println!("  {:<width$}  {}", e.display, e.desc);
        }
    }
}

fn apply(choice: &str, installed: &[String]) -> Result<()> {
    if installed.iter().any(|m| m == choice) {
        write_model_file(choice)?;
        println!("model set to `{choice}`");
        return Ok(());
    }
    if !confirm_pull(choice)? {
        println!("cancelled");
        return Ok(());
    }
    pull(choice)?;
    write_model_file(choice)?;
    println!("model set to `{choice}`");
    Ok(())
}

/// Explicit y/N confirmation before ever invoking `ollama pull` (spec §3):
/// it is an external program that downloads gigabytes, so it runs only on
/// an explicit yes — never automatically, never as a side effect.
fn confirm_pull(name: &str) -> Result<bool> {
    print!("`{name}` is not installed — pull it with `ollama pull {name}`? [y/N] ");
    std::io::stdout().flush()?;
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    Ok(answer.trim().eq_ignore_ascii_case("y"))
}

/// Run `ollama pull <name>`, inheriting stdio so its progress streams live.
/// `ollama` missing from `PATH` prints install instructions rather than
/// failing silently (spec §3).
fn pull(name: &str) -> Result<()> {
    let status = std::process::Command::new("ollama")
        .args(["pull", name])
        .status();
    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => bail!("`ollama pull {name}` exited with {status}"),
        Err(_) => bail!(
            "`ollama` not found on PATH — install it from https://ollama.com/download, then run \
             `qmark ai model {name}` again"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_entries_marks_current_installed_model() {
        let entries = build_entries(
            &["qwen2.5-coder:3b".to_string(), "llama3.2:3b".to_string()],
            "qwen2.5-coder:3b",
        );
        let current = entries
            .iter()
            .find(|e| e.display == "qwen2.5-coder:3b")
            .unwrap();
        assert_eq!(current.desc, "installed · current");
        let other = entries.iter().find(|e| e.display == "llama3.2:3b").unwrap();
        assert_eq!(other.desc, "installed");
    }

    #[test]
    fn build_entries_separator_and_hint_are_not_selectable_targets() {
        let entries = build_entries(&[], "qwen2.5-coder:3b");
        let sep = entries.iter().find(|e| e.display == SEPARATOR).unwrap();
        assert!(sep.insert.is_empty());
        let hint = entries.iter().find(|e| e.display == HINT).unwrap();
        assert!(hint.insert.is_empty());
    }

    #[test]
    fn build_entries_includes_curated_downloads() {
        let entries = build_entries(&[], "qwen2.5-coder:3b");
        assert!(entries.iter().any(|e| e.display == "qwen2.5-coder:1.5b"));
        assert!(entries.iter().any(|e| e.display == "gemma3:4b"));
    }
}
