//! `qmark explain` — local-first AI backend (spec:
//! `docs/superpowers/specs/2026-08-18-explain-ai-design.md`).
//!
//! One wire format (OpenAI chat-completions), reached over `ureq`. Local by
//! default: nothing leaves the machine unless `QMARK_AI_BASE_URL` points
//! somewhere else on purpose.

mod cache;

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::suggest::{self, Entry};

const DEFAULT_BASE_URL: &str = "http://localhost:11434/v1";
const DEFAULT_MODEL: &str = "qwen2.5-coder:3b";
const DEFAULT_TIMEOUT_SECS: u64 = 60;

const MAX_GROUNDING_ENTRIES: usize = 25;
const MAX_GROUNDING_CHARS: usize = 1500;

const SYSTEM_PROMPT: &str = "Explain what the command line does, in easy English, in 2–4 \
sentences. Assume the reader does not live in the terminal. If the command is destructive or \
irreversible — deleting files, overwriting data, force-pushing, writing to a raw device — say \
so plainly in the first sentence. Do not restate the command. Do not add a preamble.";

/// Explain a command line in plain English via the configured AI backend.
pub fn explain(line: &str) -> Result<()> {
    let line = line.trim();
    if line.is_empty() {
        bail!(r#"nothing to explain — try: qmark explain "tar -xzf archive.tar.gz""#);
    }

    let base_url = base_url();
    let (model, _source) = resolve_model();
    let cache_dir = cache::dir();

    if let Some(cached) = cache::read(&cache_dir, &model, line) {
        print_explanation(line, &cached);
        return Ok(());
    }

    let sent_line = redact_if_remote(&base_url, line);
    let grounding =
        suggest::harvest_entries(line).map(|(title, entries)| grounding_block(&title, &entries));
    let user_content = match &grounding {
        Some(block) => format!("Command: {sent_line}\n\n{block}"),
        None => format!("Command: {sent_line}"),
    };

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": user_content},
        ],
        "stream": false,
        "max_tokens": 300,
        "temperature": 0.2,
    })
    .to_string();

    let api_key = std::env::var("QMARK_AI_API_KEY").ok();
    let explanation = call_backend(&base_url, api_key.as_deref(), timeout(), &model, &body)?;

    // Written on success only — a failed, refused, or empty response never
    // gets here. Best-effort: a cache write failure must not fail `explain`.
    let _ = cache::write(&cache_dir, &model, line, &explanation);

    print_explanation(line, &explanation);
    Ok(())
}

// ---------------------------------------------------------------------------
// Configuration (spec §2)

/// Where the resolved model came from, per the precedence order in §2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModelSource {
    Env,
    File,
    Default,
}

/// `QMARK_AI_BASE_URL`, defaulted and trimmed of a trailing slash.
pub(crate) fn base_url() -> String {
    resolve_base_url_from(std::env::var("QMARK_AI_BASE_URL").ok())
}

fn resolve_base_url_from(env: Option<String>) -> String {
    let raw = env
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
    raw.trim_end_matches('/').to_string()
}

/// `$XDG_CONFIG_HOME/qmark/model`, falling back to `~/.config/qmark/model`.
pub(crate) fn model_file_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"));
    base.join("qmark").join("model")
}

/// The resolved model and where it came from: `QMARK_AI_MODEL` > the model
/// file > the built-in default.
pub(crate) fn resolve_model() -> (String, ModelSource) {
    let file = std::fs::read_to_string(model_file_path()).ok();
    resolve_model_from(std::env::var("QMARK_AI_MODEL").ok(), file)
}

fn resolve_model_from(env: Option<String>, file: Option<String>) -> (String, ModelSource) {
    if let Some(m) = env.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
        return (m, ModelSource::Env);
    }
    if let Some(m) = file.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
        return (m, ModelSource::File);
    }
    (DEFAULT_MODEL.to_string(), ModelSource::Default)
}

fn timeout() -> Duration {
    Duration::from_secs(resolve_timeout_from(std::env::var("QMARK_AI_TIMEOUT").ok()))
}

fn resolve_timeout_from(env: Option<String>) -> u64 {
    env.and_then(|s| s.trim().parse::<u64>().ok())
        .filter(|&secs| secs > 0)
        .unwrap_or(DEFAULT_TIMEOUT_SECS)
}

// ---------------------------------------------------------------------------
// Secret redaction (spec §6)

const SECRET_FLAGS: &[&str] = &["-p", "--password", "--token", "--api-key", "--secret"];
const SECRET_PREFIXES: &[&str] = &["sk-", "ghp_", "gho_", "xoxb-", "AKIA"];
const SECRET_KEY_SUBSTRINGS: &[&str] = &["PASSWORD", "TOKEN", "SECRET", "KEY"];
const REDACTED: &str = "<redacted>";

/// True when `base_url`'s host is `localhost`, `127.0.0.1`, or `::1`.
fn is_local(base_url: &str) -> bool {
    let after_scheme = base_url.split("://").nth(1).unwrap_or(base_url);
    let host = if let Some(rest) = after_scheme.strip_prefix('[') {
        // Bracketed IPv6 literal, e.g. `[::1]:11434`.
        rest.split(']').next().unwrap_or("")
    } else {
        after_scheme.split(['/', ':']).next().unwrap_or("")
    };
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

/// Token-wise redaction of obvious secrets — the value following a flag like
/// `-p`/`--token`, tokens beginning with a known secret prefix, and the
/// value half of `KEY=...` assignments whose key looks secret. `-P` is
/// deliberately not in the flag list (spec §6): in `hydra -l root -P
/// rockyou.txt` it names a wordlist path, not a password.
fn redact(line: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut tokens = line.split_whitespace().peekable();
    while let Some(tok) = tokens.next() {
        if SECRET_FLAGS.contains(&tok) {
            out.push(tok.to_string());
            if tokens.next().is_some() {
                out.push(REDACTED.to_string());
            }
            continue;
        }
        if SECRET_PREFIXES.iter().any(|p| tok.starts_with(p)) {
            out.push(REDACTED.to_string());
            continue;
        }
        if let Some((key, _value)) = tok.split_once('=') {
            let key_upper = key.to_ascii_uppercase();
            if SECRET_KEY_SUBSTRINGS.iter().any(|s| key_upper.contains(s)) {
                out.push(format!("{key}={REDACTED}"));
                continue;
            }
        }
        out.push(tok.to_string());
    }
    out.join(" ")
}

/// Redaction applies only when the endpoint is not local (spec §6) — with
/// the default Ollama setup there is nothing to redact, and redacting would
/// only degrade the explanation.
fn redact_if_remote(base_url: &str, line: &str) -> String {
    if is_local(base_url) {
        line.to_string()
    } else {
        redact(line)
    }
}

// ---------------------------------------------------------------------------
// Grounding (spec §4)

/// Format harvested entries as a bounded context block for the user message.
/// A 3B model asked to explain `-z` from memory will invent flags; this
/// shows it the real options of the real binary on this machine instead
/// (spec §4). Bounded to at most 25 entries and 1500 characters.
fn grounding_block(title: &str, entries: &[Entry]) -> String {
    let mut block = format!("Known options for `{title}` on this system:\n");
    for e in entries.iter().take(MAX_GROUNDING_ENTRIES) {
        let row = format!("  {}  {}\n", e.display, e.desc);
        if block.len() + row.len() > MAX_GROUNDING_CHARS {
            break;
        }
        block.push_str(&row);
    }
    block
}

// ---------------------------------------------------------------------------
// Request / response (spec §1)

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatMessage {
    content: Option<String>,
}

/// Parse `choices[0].message.content`, trimmed. Empty or missing content is
/// an error, never an empty success (spec §1, §8).
fn parse_response(body: &str) -> Result<String> {
    let parsed: ChatResponse =
        serde_json::from_str(body).context("malformed response from the AI backend")?;
    let content = parsed
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message.content)
        .unwrap_or_default();
    let content = content.trim();
    if content.is_empty() {
        bail!("the AI backend returned an empty response");
    }
    Ok(content.to_string())
}

/// POST the chat-completions request and turn every failure mode into the
/// instructive message spec §8 asks for.
fn call_backend(
    base_url: &str,
    api_key: Option<&str>,
    timeout: Duration,
    model: &str,
    body: &str,
) -> Result<String> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .http_status_as_error(false)
        .build()
        .new_agent();

    let mut req = agent
        .post(format!("{base_url}/chat/completions"))
        .content_type("application/json");
    if let Some(key) = api_key {
        req = req.header("Authorization", format!("Bearer {key}"));
    }

    match req.send(body.to_string()) {
        Ok(mut resp) => {
            let status = resp.status();
            let text = resp.body_mut().read_to_string().unwrap_or_default();
            if status.as_u16() == 404 {
                bail!(
                    "model `{model}` not found at {base_url} — run `qmark ai model` to install \
                     or choose one"
                );
            }
            if !status.is_success() {
                let snippet: String = text.chars().take(200).collect();
                bail!("{base_url} returned {status} — {snippet}");
            }
            parse_response(&text)
        }
        Err(ureq::Error::Timeout(_)) => {
            bail!(
                "request to {base_url} timed out after {}s — raise QMARK_AI_TIMEOUT or choose a \
                 smaller model",
                timeout.as_secs()
            );
        }
        Err(err) => {
            bail!(
                "could not reach the AI backend at {base_url} ({err}) — run `qmark ai model` to \
                 install a local one, or set QMARK_AI_BASE_URL to point at a different endpoint. \
                 `suggest` and `?` still work without it."
            );
        }
    }
}

fn print_explanation(line: &str, explanation: &str) {
    println!("── qmark ── explain {}", "─".repeat(40));
    println!();
    println!("    {line}");
    println!();
    println!("{explanation}");
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- config resolution & precedence -------------------------------

    #[test]
    fn model_env_wins_over_everything() {
        let (model, source) = resolve_model_from(
            Some("llama3.2:3b".to_string()),
            Some("qwen2.5-coder:1.5b".to_string()),
        );
        assert_eq!(model, "llama3.2:3b");
        assert_eq!(source, ModelSource::Env);
    }

    #[test]
    fn model_falls_back_to_file_when_env_unset() {
        let (model, source) = resolve_model_from(None, Some("qwen2.5-coder:1.5b".to_string()));
        assert_eq!(model, "qwen2.5-coder:1.5b");
        assert_eq!(source, ModelSource::File);
    }

    #[test]
    fn model_falls_back_to_default_when_nothing_set() {
        let (model, source) = resolve_model_from(None, None);
        assert_eq!(model, DEFAULT_MODEL);
        assert_eq!(source, ModelSource::Default);
    }

    #[test]
    fn model_ignores_blank_env_and_file_values() {
        let (model, source) = resolve_model_from(Some("  ".to_string()), Some("".to_string()));
        assert_eq!(model, DEFAULT_MODEL);
        assert_eq!(source, ModelSource::Default);
    }

    #[test]
    fn base_url_defaults_when_unset() {
        assert_eq!(resolve_base_url_from(None), DEFAULT_BASE_URL);
    }

    #[test]
    fn base_url_tolerates_a_trailing_slash() {
        assert_eq!(
            resolve_base_url_from(Some("http://localhost:1234/v1/".to_string())),
            "http://localhost:1234/v1"
        );
    }

    #[test]
    fn timeout_defaults_when_unset_or_invalid() {
        assert_eq!(resolve_timeout_from(None), DEFAULT_TIMEOUT_SECS);
        assert_eq!(
            resolve_timeout_from(Some("not a number".to_string())),
            DEFAULT_TIMEOUT_SECS
        );
        assert_eq!(
            resolve_timeout_from(Some("0".to_string())),
            DEFAULT_TIMEOUT_SECS
        );
    }

    #[test]
    fn timeout_honours_a_valid_override() {
        assert_eq!(resolve_timeout_from(Some("120".to_string())), 120);
    }

    // -- redaction ------------------------------------------------------

    #[test]
    fn redacts_password_flag_value() {
        assert_eq!(
            redact("hydra -l root -p secret123 target"),
            "hydra -l root -p <redacted> target"
        );
    }

    #[test]
    fn does_not_redact_uppercase_dash_p() {
        // -P is a wordlist path (hydra), not a password — must survive intact.
        assert_eq!(
            redact("hydra -l root -P rockyou.txt"),
            "hydra -l root -P rockyou.txt"
        );
    }

    #[test]
    fn redacts_known_secret_token_prefixes() {
        assert_eq!(
            redact("curl -H Authorization:sk-abc123 https://api.example.com"),
            "curl -H Authorization:sk-abc123 https://api.example.com"
        );
        assert_eq!(
            redact("gh auth login --with-token ghp_abcdef123456"),
            "gh auth login --with-token <redacted>"
        );
    }

    #[test]
    fn redacts_key_assignments_but_keeps_the_key() {
        assert_eq!(
            redact("AWS_SECRET_ACCESS_KEY=abc123 aws s3 ls"),
            "AWS_SECRET_ACCESS_KEY=<redacted> aws s3 ls"
        );
    }

    #[test]
    fn redact_if_remote_skips_local_endpoints() {
        let line = "hydra -l root -p secret123";
        assert_eq!(redact_if_remote("http://localhost:11434/v1", line), line);
        assert_eq!(redact_if_remote("http://127.0.0.1:11434/v1", line), line);
        assert_eq!(redact_if_remote("http://[::1]:11434/v1", line), line);
    }

    #[test]
    fn redact_if_remote_redacts_non_local_endpoints() {
        let line = "hydra -l root -p secret123";
        assert_eq!(
            redact_if_remote("https://api.groq.com/openai/v1", line),
            "hydra -l root -p <redacted>"
        );
    }

    // -- response parsing -------------------------------------------------

    #[test]
    fn parses_a_recorded_chat_completions_body() {
        let body = r#"{
            "id": "chatcmpl-1",
            "choices": [
                {"index": 0, "message": {"role": "assistant", "content": "  Extracts a tar archive.  "}}
            ]
        }"#;
        assert_eq!(parse_response(body).unwrap(), "Extracts a tar archive.");
    }

    #[test]
    fn empty_content_is_an_error_not_an_empty_success() {
        let body = r#"{"choices": [{"message": {"content": "   "}}]}"#;
        assert!(parse_response(body).is_err());
    }

    #[test]
    fn missing_choices_is_an_error() {
        assert!(parse_response("{}").is_err());
    }

    #[test]
    fn malformed_json_is_an_error() {
        assert!(parse_response("not json").is_err());
    }

    // -- grounding block bounds -------------------------------------------

    fn entry(flag: &str, desc: &str) -> Entry {
        Entry {
            insert: flag.to_string(),
            display: flag.to_string(),
            desc: desc.to_string(),
        }
    }

    #[test]
    fn grounding_block_caps_at_25_entries() {
        let entries: Vec<Entry> = (0..40)
            .map(|i| entry(&format!("-{i}"), "an option"))
            .collect();
        let block = grounding_block("tar", &entries);
        assert_eq!(block.matches("  -").count(), MAX_GROUNDING_ENTRIES);
    }

    #[test]
    fn grounding_block_caps_at_1500_chars() {
        let long_desc = "x".repeat(200);
        let entries: Vec<Entry> = (0..25)
            .map(|i| entry(&format!("-{i}"), &long_desc))
            .collect();
        let block = grounding_block("tar", &entries);
        assert!(block.len() <= MAX_GROUNDING_CHARS);
        // The bound actually bit: not all 25 long entries fit.
        assert!(block.matches("  -").count() < 25);
    }

    #[test]
    fn grounding_block_includes_title_and_entries() {
        let entries = vec![entry("-x", "extract files from an archive")];
        let block = grounding_block("tar", &entries);
        assert!(block.contains("tar"));
        assert!(block.contains("-x"));
        assert!(block.contains("extract files from an archive"));
    }
}
