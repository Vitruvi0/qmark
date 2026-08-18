//! Response cache for `qmark explain` (spec §5).
//!
//! `$XDG_CACHE_HOME/qmark/explain/<hash>.txt`, falling back to
//! `~/.cache/qmark/explain`. The hash comes from `DefaultHasher` over
//! model + command line — no crypto dependency, the cache is a local
//! convenience, not a security boundary. The first line of each file records
//! the exact model and command line and is verified on read, so a hash
//! collision is a cache miss, never a wrong answer served confidently.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use anyhow::Result;

/// The cache directory for the current environment. Callers resolve this
/// once and pass it down, so tests can point it at a temp dir instead of
/// racing on `$XDG_CACHE_HOME`/`$HOME`.
pub(crate) fn dir() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .unwrap_or_else(|| PathBuf::from(".cache"));
    base.join("qmark").join("explain")
}

/// The header line a cache file must carry to be considered a hit for
/// `model` + `line`.
fn header(model: &str, line: &str) -> String {
    format!("{model}\t{line}")
}

fn path(dir: &Path, model: &str, line: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    model.hash(&mut hasher);
    line.hash(&mut hasher);
    dir.join(format!("{:x}.txt", hasher.finish()))
}

/// Look up a cached explanation for `model` + `line`. `None` on any miss,
/// including a hash collision (the file exists but its header does not
/// match) — a collision must never serve a wrong answer confidently.
pub(crate) fn read(dir: &Path, model: &str, line: &str) -> Option<String> {
    let text = std::fs::read_to_string(path(dir, model, line)).ok()?;
    let (first, rest) = text.split_once('\n')?;
    if first != header(model, line) {
        return None;
    }
    let explanation = rest.trim();
    if explanation.is_empty() {
        return None;
    }
    Some(explanation.to_string())
}

/// Write `explanation` to the cache. Callers write on success only — a
/// failed, refused, or empty response is never cached.
pub(crate) fn write(dir: &Path, model: &str, line: &str, explanation: &str) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(
        path(dir, model, line),
        format!("{}\n{explanation}\n", header(model, line)),
    )?;
    Ok(())
}

/// Entry count and total size in bytes of the cache directory, for `qmark ai
/// status` (Task 2). A missing directory reports as empty rather than an
/// error.
// ponytail: not called yet — `qmark ai status` (Task 2) is its first caller.
#[allow(dead_code)]
pub(crate) fn stats(dir: &Path) -> (usize, u64) {
    let mut count = 0usize;
    let mut bytes = 0u64;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    count += 1;
                    bytes += meta.len();
                }
            }
        }
    }
    (count, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh temp directory for one test, cleaned up on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "qmark-ai-cache-test-{name}-{}-{:?}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn miss_on_empty_cache() {
        let dir = TempDir::new("miss");
        assert!(read(&dir.0, "qwen2.5-coder:3b", "tar -xzf a.tar.gz").is_none());
    }

    #[test]
    fn write_then_read_round_trips() {
        let dir = TempDir::new("roundtrip");
        write(
            &dir.0,
            "qwen2.5-coder:3b",
            "tar -xzf a.tar.gz",
            "Extracts a tar archive.",
        )
        .unwrap();
        let got = read(&dir.0, "qwen2.5-coder:3b", "tar -xzf a.tar.gz");
        assert_eq!(got.as_deref(), Some("Extracts a tar archive."));
    }

    #[test]
    fn switching_models_is_a_miss() {
        // The key includes the model, so switching models never serves a
        // previous model's answer.
        let dir = TempDir::new("model-switch");
        write(&dir.0, "model-a", "ls -la", "Lists files.").unwrap();
        assert!(read(&dir.0, "model-b", "ls -la").is_none());
    }

    #[test]
    fn hash_collision_is_a_miss_not_a_wrong_answer() {
        // Simulate a collision: a file sits at the exact path our key would
        // hash to, but its header names a different model + command line.
        let dir = TempDir::new("collision");
        let model = "qwen2.5-coder:3b";
        let line = "rm -rf /tmp/x";
        std::fs::write(
            path(&dir.0, model, line),
            "some-other-model\tsome other command\nStale, wrong explanation.\n",
        )
        .unwrap();
        assert!(read(&dir.0, model, line).is_none());
    }

    #[test]
    fn stats_counts_entries_and_bytes() {
        let dir = TempDir::new("stats");
        assert_eq!(stats(&dir.0), (0, 0));
        write(&dir.0, "m", "a", "short").unwrap();
        write(&dir.0, "m", "b", "also short").unwrap();
        let (count, bytes) = stats(&dir.0);
        assert_eq!(count, 2);
        assert!(bytes > 0);
    }
}
