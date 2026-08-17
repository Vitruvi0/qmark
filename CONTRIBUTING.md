# Contributing to qmark

Thanks for your interest in qmark! The repository is currently private and pre-alpha, but we
develop it as if it were public — the same rules below apply to the core team and, later, to
external contributors.

## Development setup

1. Install a stable Rust toolchain via [rustup](https://rustup.rs). The pinned version is in
   `rust-toolchain.toml` and rustup will pick it up automatically.
2. Clone the repo and build:

   ```sh
   git clone https://github.com/Vitruvi0/qmark.git
   cd qmark
   cargo build
   cargo test
   ```

3. Try the binary without installing:

   ```sh
   cargo run -- suggest "git"
   cargo run -- init zsh
   ```

## Before you push

CI enforces all of these, so run them locally first:

```sh
cargo fmt --all            # formatting (rustfmt, default style)
cargo clippy --all-targets -- -D warnings
cargo test
```

## Commit messages

We use [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/):

```
feat: add zsh widget for the ? binding
fix: do not trigger help inside quoted strings
docs: expand architecture notes on the suggest engine
chore: bump clap to 4.x
```

Types we use: `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `chore`, `ci`.
Keep the subject line under ~72 characters, imperative mood, no trailing period.

## Pull requests

- Branch from `main`; name branches `feat/...`, `fix/...`, `docs/...`.
- Keep PRs small and focused — one logical change per PR.
- Fill in the PR template; link the issue it closes if there is one.
- Update `CHANGELOG.md` under `[Unreleased]` for user-visible changes.
- New behavior needs a test; bug fixes need a regression test where practical.

## Design discussions

Anything that changes the CLI surface (new subcommands, flags, key bindings) or the shell
integration contract should start as an issue before code is written. The `?` binding in
particular has sharp UX edges (globs, quoting, non-interactive shells) — see
`docs/ARCHITECTURE.md` for the current rules before proposing changes.

## Code style

- Rust 2024 edition, rustfmt defaults, clippy clean at `-D warnings`.
- Prefer small modules with one clear responsibility (`cli`, `suggest`, `ai`, `shells`).
- Error handling with `anyhow` at the binary boundary; typed errors inside modules when it
  earns its keep.
- Shell snippets in `shell/` must stay POSIX-clean where possible, be shellcheck-friendly,
  and never break the user's prompt if `qmark` is missing from `PATH`.
