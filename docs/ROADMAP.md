# Roadmap

Living document — items move as we learn. Versions are milestones, not promises.

## v0.1 — scaffold (this repo, done)

- [x] Repository hygiene: README, MIT license, contributing guide, code of conduct,
      security policy, issue/PR templates, CI (fmt, clippy, tests on Linux+macOS,
      shellcheck).
- [x] Rust CLI skeleton (`clap`): `suggest`, `explain` (stub), `init`.
- [x] zsh + bash `?` integrations with safe fallback rules.
- [x] `--help` harvesting with pager/stdin protections.

## v0.2 — the AI half

- [ ] `Provider` trait + first backend (Anthropic API), then OpenAI-compatible + Ollama.
- [ ] `QMARK_AI_PROVIDER` / `QMARK_AI_API_KEY` wiring, clear errors when unset.
- [ ] Fixed prompt: easy English, 2–4 sentences, explicit warning on destructive flags
      (`rm -rf`, `dd`, `chmod -R`, force pushes...).
- [ ] On-disk response cache (works offline for repeated questions).
- [ ] Redaction pass: strip obvious secrets (tokens, passwords in flags) before sending.

## v0.3 — better help sources

- [ ] Subcommand-aware suggestions (curated strategy per tool family: git, docker, kubectl).
- [ ] tldr pages as an offline, structured source alongside `--help`.
- [ ] Timeout wrapper around help harvesting.
- [ ] PowerShell integration (PSReadLine key handler) — opens the Windows story.

## v0.4 — distribution & polish

- [ ] Release workflow: tagged releases with prebuilt binaries (Linux x86_64/arm64,
      macOS universal), `cargo-binstall` metadata.
- [ ] Homebrew tap; AUR; Scoop/winget once PowerShell lands.
- [ ] Shell completions for qmark itself (`clap_complete`).
- [ ] Config file (`~/.config/qmark/config.toml`) replacing/augmenting env vars.

## Before going public

- [ ] Name collision check for `qmark` on crates.io / Homebrew / npm and a decision on
      whether to keep the name.
- [ ] Second pass on README (demo GIF/asciinema), triage labels, branch protection on
      `main`, enable the issue templates.
- [ ] License header / provenance review of any vendored content.
