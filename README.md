# qmark

> Cisco-style `?` help, right in your terminal — with AI explanations in plain English.

[![CI](https://github.com/Vitruvi0/qmark/actions/workflows/ci.yml/badge.svg)](https://github.com/Vitruvi0/qmark/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
![Status: pre-alpha](https://img.shields.io/badge/status-pre--alpha-red)

If you have ever configured a Cisco device, you know the feeling: you type `?` and the CLI
tells you exactly what you can do next. `qmark` brings that experience to your everyday
terminal. Type a command, hit `?`, and get context-sensitive help — plus an `explain`
command that uses AI to describe what a command line does in easy English, before you run it.

> **Project status:** pre-alpha, under active development. APIs, commands and key bindings
> will change without notice. Not ready for production use.

## Why

- `man` pages are complete but overwhelming; `--help` output is dense and inconsistent.
- Cisco solved discoverability decades ago with a single keystroke: `?`.
- LLMs are very good at turning `tar -xzvf archive.tar.gz -C /tmp` into a sentence a human
  can read. There is no reason your shell should not do that for you.

## What it looks like

```console
$ git ?
  ── qmark ────────────────────────────────────────────
  git — the stupid content tracker

  Common subcommands:
    clone      Clone a repository into a new directory
    status     Show the working tree status
    commit     Record changes to the repository
    ...

$ qmark explain "tar -xzvf archive.tar.gz -C /tmp"
  This extracts the compressed archive "archive.tar.gz" into the
  /tmp folder, showing each file name while it works.
```

## Features

- **`?` at the prompt** — Cisco-style contextual help for the command you are typing,
  wired into your shell (zsh and bash in v1).
- **`qmark suggest`** — the same help engine, callable directly: pass any partial command
  line and get a readable summary of what it does and which options exist.
- **`qmark explain`** — AI-powered, plain-English explanation of a full command line.
  Designed for people who do not live in the terminal all day.
- **Single static binary** — written in Rust, no runtime required.

## Installation

Pre-built binaries are not published yet. Build from source:

```sh
git clone https://github.com/Vitruvi0/qmark.git
cd qmark
cargo install --path .
```

Requires a recent stable Rust toolchain (see `rust-toolchain.toml`).

## Shell integration

`qmark init` prints the integration snippet for your shell. Add one line to your rc file:

```sh
# ~/.zshrc
eval "$(qmark init zsh)"

# ~/.bashrc
eval "$(qmark init bash)"
```

After that, ending a command with a space and pressing `?` shows contextual help
(a `?` typed inside a word, e.g. `ls file?.txt`, is inserted normally, so globs keep working).

## Usage

```console
qmark suggest "git"          # contextual help for a (partial) command line
qmark explain "rm -rf ./x"   # plain-English explanation (AI)
qmark init zsh|bash          # print the shell integration snippet
qmark --help                 # full CLI reference
```

## Configuration

Configuration lives in environment variables for now (a config file is on the
[roadmap](docs/ROADMAP.md)):

| Variable            | Purpose                                              |
| ------------------- | ---------------------------------------------------- |
| `QMARK_AI_PROVIDER` | AI backend for `explain` (not wired up yet)          |
| `QMARK_AI_API_KEY`  | API key for the AI backend                           |
| `QMARK_NO_BIND`     | Set to `1` to load the integration without the `?` key binding |

## How it works

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the design. In short: a small Rust
binary does the heavy lifting; thin shell snippets (a ZLE widget in zsh, `bind -x` in bash)
intercept `?` at the prompt and call the binary with the current line buffer.

## Roadmap

The full roadmap is in [docs/ROADMAP.md](docs/ROADMAP.md). Highlights:

- v0: repo scaffold, CLI skeleton, zsh/bash `?` binding, `--help`-based suggestions
- v0.2: AI backend for `explain` (provider-agnostic), offline cache
- v0.3: PowerShell support, richer help sources (tldr pages, man parsing)

## Contributing

Contributions are welcome once the repository is public. In the meantime, the workflow we
follow ourselves is documented in [CONTRIBUTING.md](CONTRIBUTING.md). Please also read the
[Code of Conduct](CODE_OF_CONDUCT.md).

## Security

See [SECURITY.md](SECURITY.md) for how to report vulnerabilities.

## License

[MIT](LICENSE) © 2026 Vitruvi0
