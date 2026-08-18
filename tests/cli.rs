use assert_cmd::Command;
use predicates::prelude::*;

fn qmark() -> Command {
    Command::cargo_bin("qmark").expect("binary builds")
}

#[test]
fn version_works() {
    qmark()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("qmark"));
}

#[test]
fn help_lists_subcommands() {
    qmark()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("suggest"))
        .stdout(predicate::str::contains("explain"))
        .stdout(predicate::str::contains("init"));
}

#[test]
fn init_zsh_prints_widget() {
    qmark()
        .args(["init", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("qmark-widget"))
        .stdout(predicate::str::contains("bindkey '?'"))
        .stdout(predicate::str::contains("suggest --interactive"));
}

#[test]
fn init_bash_prints_widget() {
    qmark()
        .args(["init", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("__qmark_widget"))
        .stdout(predicate::str::contains("bind -x"))
        .stdout(predicate::str::contains("suggest --interactive"));
}

#[test]
fn init_rejects_unknown_shell() {
    qmark().args(["init", "fish"]).assert().failure();
}

#[test]
fn suggest_known_command_shows_help() {
    // `cargo` is guaranteed to be on PATH when tests run under cargo,
    // and its --help is pager-free on every platform we support.
    qmark()
        .args(["suggest", "--", "cargo "])
        .assert()
        .success()
        .stdout(predicate::str::contains("help for `cargo`"));
}

#[test]
fn suggest_is_subcommand_aware() {
    // `cargo build --help` exists and lists `--release`; the header must show
    // the full chain, not just the base command.
    qmark()
        .args(["suggest", "--", "cargo build "])
        .assert()
        .success()
        .stdout(predicate::str::contains("help for `cargo build`"))
        .stdout(predicate::str::contains("--release"));
}

#[test]
fn suggest_interactive_without_tty_falls_back_to_list_on_stderr() {
    // In interactive mode the widget captures stdout with $(...) and inserts
    // it into the command line, so the fallback list must go to stderr.
    qmark()
        .args(["suggest", "--interactive", "--", "cargo "])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("help for `cargo`"));
}

#[test]
fn suggest_interactive_empty_line_keeps_stdout_clean() {
    qmark()
        .args(["suggest", "--interactive", "--", "   "])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("Type part of a command"));
}

#[test]
fn suggest_ssh_uses_curated_entries() {
    // ssh has no --help; qmark falls back to its built-in curated table.
    qmark()
        .args(["suggest", "--", "ssh "])
        .assert()
        .success()
        .stdout(predicate::str::contains("-p"))
        .stdout(predicate::str::contains("-i"));
}

#[test]
fn suggest_find_prefers_curated_over_descriptionless_rows() {
    qmark()
        .args(["suggest", "--", "find "])
        .assert()
        .success()
        .stdout(predicate::str::contains("-iname"))
        .stdout(predicate::str::contains("case insensitive"));
}

#[test]
fn suggest_openssl_lists_subcommands() {
    // openssl's `--help` is an uncolumned grid; the curated table lists its
    // common subcommands instead.
    qmark()
        .args(["suggest", "--", "openssl "])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "s_client  test/debug a TLS connection to a server",
        ));
}

#[test]
fn suggest_ps_uses_curated_entries() {
    qmark()
        .args(["suggest", "--", "ps "])
        .assert()
        .success()
        .stdout(predicate::str::contains("aux"));
}

#[test]
fn suggest_handles_shell_builtins() {
    // `cd` is not a file in PATH; help comes from bash's `help` builtin.
    qmark()
        .args(["suggest", "--", "cd "])
        .assert()
        .success()
        .stdout(predicate::str::contains("help for `cd`"))
        .stdout(predicate::str::contains("-P"));
}

#[test]
fn suggest_curated_covers_uninstalled_security_tools() {
    // nmap may not be installed, but its curated table still answers `nmap ?`.
    qmark()
        .args(["suggest", "--", "nmap "])
        .assert()
        .success()
        .stdout(predicate::str::contains("-sV"))
        .stdout(predicate::str::contains("-p"));
}

#[test]
fn suggest_unknown_command_fails_gracefully() {
    qmark()
        .args(["suggest", "--", "definitely-not-a-real-command-qmark"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no help available"));
}

#[test]
fn suggest_empty_line_is_friendly() {
    qmark()
        .args(["suggest", "--", "   "])
        .assert()
        .success()
        .stdout(predicate::str::contains("Type part of a command"));
}

/// A temp dir for one test's `$XDG_CACHE_HOME`, so a cache hit from an
/// earlier run (or a concurrently running test) can never make this test's
/// backend call silently not happen. Cleaned up on drop.
struct TempCacheDir(std::path::PathBuf);

impl TempCacheDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "qmark-cli-test-cache-{name}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        TempCacheDir(dir)
    }
}

impl Drop for TempCacheDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Accept one connection on an ephemeral port and write back `response`
/// (a raw HTTP/1.1 response, status line through body) verbatim. Returns
/// the port to point `QMARK_AI_BASE_URL` at.
fn spawn_fake_backend(response: String) -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        use std::io::{Read, Write};
        if let Ok((mut stream, _)) = listener.accept() {
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(500)));
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf); // drain the request; content doesn't matter here
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    port
}

#[test]
fn explain_calls_local_backend_and_prints_the_explanation() {
    let explanation = "This extracts a gzip-compressed tar archive into /tmp.";
    let json_body = format!(r#"{{"choices":[{{"message":{{"content":"{explanation}"}}}}]}}"#);
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        json_body.len(),
        json_body
    );
    let port = spawn_fake_backend(response);
    let cache = TempCacheDir::new("success");

    qmark()
        .args(["explain", "--", "tar -xzvf archive.tar.gz -C /tmp"])
        .env("QMARK_AI_BASE_URL", format!("http://127.0.0.1:{port}/v1"))
        .env("QMARK_AI_MODEL", "test-model")
        .env("XDG_CACHE_HOME", &cache.0)
        .assert()
        .success()
        .stdout(predicate::str::contains(explanation));
}

#[test]
fn explain_against_a_closed_port_fails_instructively() {
    // Bind then immediately drop, guaranteeing the port is closed.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let cache = TempCacheDir::new("closed-port");

    qmark()
        .args(["explain", "--", "tar -xzf archive.tar.gz"])
        .env("QMARK_AI_BASE_URL", format!("http://127.0.0.1:{port}/v1"))
        .env("QMARK_AI_MODEL", "test-model")
        .env("XDG_CACHE_HOME", &cache.0)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("qmark ai model"))
        .stderr(predicate::str::contains("suggest"));
}

#[test]
fn explain_on_a_flag_first_line_fails_gracefully_without_panicking() {
    // A command line that starts with a flag (`-x foo`, bare `-`) makes
    // suggest::command_chain return an empty Vec; grounding must not panic
    // indexing into it — it should just fall through to the ordinary
    // backend-unreachable error, not a Rust panic (exit code 101).
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let cache = TempCacheDir::new("flag-first-line");

    qmark()
        .args(["explain", "--", "-x foo"])
        .env("QMARK_AI_BASE_URL", format!("http://127.0.0.1:{port}/v1"))
        .env("QMARK_AI_MODEL", "test-model")
        .env("XDG_CACHE_HOME", &cache.0)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("qmark ai model"));
}
