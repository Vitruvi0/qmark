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
        .stdout(predicate::str::contains("bindkey '?'"));
}

#[test]
fn init_bash_prints_widget() {
    qmark()
        .args(["init", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("__qmark_widget"))
        .stdout(predicate::str::contains("bind -x"));
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
fn suggest_unknown_command_fails_gracefully() {
    qmark()
        .args(["suggest", "--", "definitely-not-a-real-command-qmark"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found in PATH"));
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
