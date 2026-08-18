//! `qmark ai status` — endpoint, model and its source, reachability, and
//! cache size (spec §7). A diagnostic, not a failure: it always exits 0,
//! even when the backend is unreachable.

use std::path::{Path, PathBuf};

use anyhow::Result;

use super::{
    ModelSource, base_url, cache, fetch_models_body, is_local, model_file_path, resolve_model,
};

pub(crate) fn run() -> Result<()> {
    let base_url = base_url();
    let reachable = fetch_models_body(&base_url).is_ok();
    let (model, source) = resolve_model();
    let api_key_set = std::env::var("QMARK_AI_API_KEY")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    let cache_dir = cache::dir();
    let (count, bytes) = cache::stats(&cache_dir);

    println!("── qmark ── ai status ──────────────────────────────");
    println!(
        "  {:<10} {:<32} {}",
        "endpoint",
        base_url,
        if reachable {
            "reachable"
        } else {
            "unreachable"
        }
    );
    println!(
        "  {:<10} {:<32} {}",
        "model",
        model,
        describe_source(source)
    );
    println!(
        "  {:<10} {:<32} {}",
        "api key",
        if api_key_set { "set" } else { "not set" },
        api_key_note(api_key_set, &base_url)
    );
    println!(
        "  {:<10} {:<32} {}",
        "cache",
        format!("{count} entries, {}", human_size(bytes)),
        display_path(&cache_dir)
    );
    Ok(())
}

fn describe_source(source: ModelSource) -> String {
    match source {
        ModelSource::Env => "from $QMARK_AI_MODEL".to_string(),
        ModelSource::File => format!("from {}", display_path(&model_file_path())),
        ModelSource::Default => "default (none set)".to_string(),
    }
}

fn api_key_note(set: bool, base_url: &str) -> &'static str {
    if set {
        "(hidden)"
    } else if is_local(base_url) {
        "(not needed for a local endpoint)"
    } else {
        "(needed for remote endpoints)"
    }
}

// ponytail: the cache holds only small text files, so KB is the coarsest
// unit worth showing — no need for MB handling.
fn human_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else {
        format!("{} KB", bytes / 1024)
    }
}

/// Collapse the user's home directory to `~` for display, matching the
/// spec's example output.
fn display_path(path: &Path) -> String {
    if let Some(home) = std::env::var_os("HOME") {
        if let Ok(rest) = path.strip_prefix(&home) {
            return PathBuf::from("~").join(rest).display().to_string();
        }
    }
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_size_switches_units_at_1024_bytes() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(1023), "1023 B");
        assert_eq!(human_size(2048), "2 KB");
    }

    #[test]
    fn describe_source_names_env_file_and_default() {
        assert_eq!(describe_source(ModelSource::Env), "from $QMARK_AI_MODEL");
        assert_eq!(describe_source(ModelSource::Default), "default (none set)");
        assert!(describe_source(ModelSource::File).starts_with("from "));
    }

    #[test]
    fn api_key_note_distinguishes_local_from_remote() {
        assert_eq!(
            api_key_note(false, "http://localhost:11434/v1"),
            "(not needed for a local endpoint)"
        );
        assert_eq!(
            api_key_note(false, "https://api.groq.com/openai/v1"),
            "(needed for remote endpoints)"
        );
        assert_eq!(api_key_note(true, "http://localhost:11434/v1"), "(hidden)");
    }
}
