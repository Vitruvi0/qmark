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

#[test]
fn explain_is_an_honest_stub_for_now() {
    qmark()
        .args(["explain", "--", "tar -xzf archive.tar.gz"])
        .assert()
        .success()
        .stdout(predicate::str::contains("tar -xzf archive.tar.gz"))
        .stdout(predicate::str::contains("not wired up yet"));
}
