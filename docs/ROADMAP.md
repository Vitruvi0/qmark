# Roadmap

Living document — items move as we learn. Versions are milestones, not promises.

## v0.1 — scaffold (this repo, done)

- [x] Repository hygiene: README, MIT license, contributing guide, code of conduct,
      security policy, issue/PR templates, CI (fmt, clippy, tests on Linux+macOS,
      shellcheck).
- [x] Rust CLI skeleton (`clap`): `suggest`, `explain` (stub), `init`.
- [x] zsh + bash `?` integrations with safe fallback rules.
- [x] `--help` harvesting with pager/stdin protections.

## v0.2 — the AI half (done)

- [x] One wire format (OpenAI chat-completions) over `ureq`, local by default
      (`http://localhost:11434/v1`, i.e. Ollama). No `Provider` trait — a single
      implementation covers local runtimes and most hosted aggregators; it was speculative
      abstraction with one caller. `QMARK_AI_PROVIDER` is gone.
- [x] `QMARK_AI_BASE_URL` / `QMARK_AI_MODEL` / `QMARK_AI_API_KEY` / `QMARK_AI_TIMEOUT`
      wiring, with instructive errors on unreachable/timeout/404/empty-response.
- [x] Fixed prompt: easy English, 2–4 sentences, explicit warning on destructive flags
      (`rm -rf`, `dd`, `chmod -R`, force pushes...).
- [x] Grounding: `explain` harvests the real `--help` options of the target command (reusing
      `suggest`'s parser) so a small local model doesn't have to invent flags.
- [x] `qmark ai model` — interactive picker (reuses the `?` picker) listing installed models
      plus a curated download list; confirms before `ollama pull`. `qmark ai status` —
      endpoint, model + source, reachability, cache size.
- [x] On-disk response cache (works offline for repeated questions), keyed by model +
      command line.
- [x] Redaction pass: strip obvious secrets (tokens, passwords in flags) before sending —
      applied only to non-local endpoints, since local traffic never leaves the machine.

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
