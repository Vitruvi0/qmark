# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Local-first AI backend for `qmark explain` (`src/ai.rs`): one wire format, OpenAI
  chat-completions, over `ureq` — talks to Ollama on `localhost` by default, or any
  OpenAI-compatible endpoint (llama.cpp, LM Studio, vLLM, hosted aggregators) via
  `QMARK_AI_BASE_URL`. Grounded in the target command's real `--help` options (reusing
  `suggest`'s harvester) so a small local model doesn't have to invent flags. Responses are
  cached on disk (`~/.cache/qmark/explain`), keyed by model + command line. Obvious secrets
  (password/token/API-key flags, `sk-`/`ghp_`-style tokens, `KEY=`/`SECRET=` assignments)
  are redacted before the request when the endpoint is not local.
- `qmark ai status`: endpoint, resolved model and its source, reachability, and cache size —
  the diagnostic to ask for in a bug report.
- `qmark ai model`: interactive picker (reuses the `?` key's picker) listing installed
  models plus a small curated download list; picking an uninstalled one confirms before
  running `ollama pull`. `qmark ai model <name>` sets one directly, non-interactively.
- Subcommand-aware suggestions: `git mv ?` harvests help for `git mv` (with `-h` and
  base-command fallbacks), parsed into structured `entry — description` rows.
- Interactive picker (`qmark suggest --interactive`): `?` opens an arrow-key menu on the
  tty; the chosen entry is inserted into the command line, replacing the `?`. Esc cancels.
  Falls back to the plain list when no tty is available.
- Parser hardening for base Linux commands: ANSI/OSC escapes stripped (GNU coreutils ≥9.x
  hyperlinked help), tab column separators (gawk), single-space separators (procps/which),
  descriptions on the following line (coreutils two-line style), `a | b` syntax summaries
  rejected (iproute2).
- Curated built-in entries for commands whose help is unparseable — ssh, scp, ip, ps,
  pacman, find — used only when `--help` harvesting yields nothing useful.
- Shell builtins (cd, export, alias, source, history, jobs, ...): when the command is not
  a file in PATH, help is harvested from bash's `help` builtin.
- Curated subcommand list for `openssl` (its `--help` is an uncolumned grid): `openssl ?`
  lists s_client, x509, req, enc, dgst, …; `openssl <sub> ?` still harvests the real flags.
- Curated entries for common security/pentest tools (defensive & educational use): nmap,
  masscan, sqlmap, hydra, john, hashcat, gobuster, ffuf, feroxbuster, nikto, wpscan,
  nuclei, tcpdump, nc/ncat/netcat, aircrack-ng. These answer `?` even when the tool is
  not installed locally.

- Animated terminal demo (`assets/demo.svg`, hand-written SVG, no recorder toolchain) shown
  at the top of the README: typing `git `, pressing `?`, moving the selection and inserting
  the choice. Its text is the binary's real output.
- Diagrams in `docs/ARCHITECTURE.md` (mermaid, rendered natively by GitHub): shell layer ↔
  binary dataflow, the `?` binding decision rules, and the help-resolution ladder.
- Slide deck rendered to images (`docs/slides/slide.001.png` … `012.png`) and to
  `docs/qmark-deck.pdf`, linked from the README rather than embedded in it — the deck lives
  under `docs/`. `docs/SLIDES.md` stays the source; the render commands are documented in
  its header.
- Two new slides: the interactive picker and subcommand awareness (`git mv ?`).

- Repository scaffold: README, MIT license, contributing guide, code of conduct, security
  policy, issue/PR templates, CI workflow.

### Changed

- `qmark explain` is no longer a stub: it calls the configured AI backend (local by default)
  and prints a real explanation. README, `docs/ARCHITECTURE.md` and `docs/SLIDES.md` updated
  to match.
- `QMARK_AI_PROVIDER` removed — with a single wire format it selected nothing, and it was
  documented but never implemented. Replaced by `QMARK_AI_BASE_URL`, `QMARK_AI_MODEL`,
  `QMARK_AI_API_KEY`, `QMARK_AI_TIMEOUT`.
- README, `docs/ARCHITECTURE.md` and `docs/SLIDES.md` now describe what actually ships: the
  `?` examples show the real picker output instead of invented help text, `explain` is shown
  as the stub it currently is, and "base command only" is gone from the known limitations
  (subcommand awareness landed).
- `qmark` CLI skeleton (Rust + clap) with three subcommands:
  - `suggest` — contextual help for a (partial) command line, based on `--help` harvesting;
  - `explain` — plain-English explanation of a command line (AI backend not wired up yet);
  - `init` — prints the shell integration snippet for zsh or bash.
- zsh and bash integrations: Cisco-style `?` at the end of a command shows contextual help;
  `?` inside a word is inserted normally so globs keep working.
