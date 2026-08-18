# qmark

> Cisco-style `?` help, right in your terminal — with AI explanations in plain English.

[![CI](https://github.com/Vitruvi0/qmark/actions/workflows/ci.yml/badge.svg)](https://github.com/Vitruvi0/qmark/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
![Status: pre-alpha](https://img.shields.io/badge/status-pre--alpha-red)

<p align="center">
  <img src="assets/demo.svg" alt="Typing `git ` and pressing ? opens the qmark picker; arrow keys move the selection to `add`, Enter inserts it, leaving `git add ` on the prompt." width="820">
</p>

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

Press `?` after a space and an interactive picker opens on the spot (↑↓ to move, Enter to
insert, Esc to close) — that is the animation above. The same engine is subcommand-aware,
so it answers for the command you are actually typing:

```console
$ git mv ?
── qmark ── help for `git mv` ──────────────────────────────
  -v, --[no-]verbose  be verbose
  -n, --[no-]dry-run  dry run
  -f, --[no-]force    force move/rename even if target exists
  -k                  skip move/rename errors
  --[no-]sparse       allow updating entries outside of the sparse-checkout cone

Tip: `qmark explain "git mv"` gives a plain-English explanation (AI).
```

`explain` calls a local model by default — nothing leaves your machine unless you point
`QMARK_AI_BASE_URL` somewhere else on purpose:

```console
$ qmark explain "rm -rf ./build"
── qmark ── explain ────────────────────────────────────────

    rm -rf ./build

The command `rm -rf ./build` deletes the directory named `./build` and all its contents
recursively. The `-r` option tells `rm` to remove directories and their contents, while the
`-f` option forces deletion without prompting for confirmation. This is a destructive
operation that can't be undone, so it's important to double-check the command before
running it.
```

## Features

- **`?` at the prompt** — Cisco-style contextual help for the command you are typing,
  wired into your shell (zsh and bash in v1). The `?` stays visible on the line and an
  interactive picker opens; choosing an entry inserts it in place of the `?`.
- **Subcommand-aware** — `git mv ?` shows what can follow `git mv` (its flags), not just
  the generic `git` command list.
- **`qmark suggest`** — the same help engine, callable directly: pass any partial command
  line and get a readable summary of what it does and which options exist.
- **`qmark explain`** — AI-powered, plain-English explanation of a full command line.
  Local by default (talks to Ollama on `localhost`), grounded in the real `--help` output of
  the command so a small model does not have to guess what a flag does, and cached on disk
  so repeated questions are free and offline.
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

After that, ending a command with a space and pressing `?` opens an interactive menu of
what can come next (↑↓ to move, Enter to insert, Esc to close). A `?` typed inside a word,
e.g. `ls file?.txt`, is inserted normally, so globs keep working.

## Usage

```console
qmark suggest "git"          # contextual help for a (partial) command line
qmark explain "rm -rf ./x"   # plain-English explanation (AI)
qmark ai status              # endpoint, model, reachability, cache size
qmark ai model               # pick/install a model interactively
qmark init zsh|bash          # print the shell integration snippet
qmark --help                 # full CLI reference
```

## AI setup (local-first)

`explain` talks to an OpenAI-compatible chat-completions endpoint — by default
`http://localhost:11434/v1`, i.e. [Ollama](https://ollama.com) running on your own machine.
Nothing is sent anywhere until you ask it to be.

```sh
# 1. install Ollama (see https://ollama.com/download)
# 2. pick a model — interactive picker, lists what's installed plus a
#    curated download list; downloading asks for confirmation first
qmark ai model
# 3. ask away
qmark explain "tar -xzvf archive.tar.gz -C /tmp"
```

`qmark ai status` reports the endpoint, the resolved model and where it came from,
whether the endpoint is reachable, and the on-disk cache size — the diagnostic to run
first when something looks wrong. Any OpenAI chat-completions-compatible server works
(llama.cpp, LM Studio, vLLM, or a hosted aggregator such as Groq/Together/OpenRouter) —
just point `QMARK_AI_BASE_URL` at it.

## Configuration

Configuration lives in environment variables for now (a config file is on the
[roadmap](docs/ROADMAP.md)); the one exception is the selected model, which persists to
`~/.config/qmark/model` so `qmark ai model` behaves like a real selection.

| Variable            | Default                      | Purpose                                                            |
| ------------------- | ----------------------------- | ------------------------------------------------------------------ |
| `QMARK_AI_BASE_URL` | `http://localhost:11434/v1`   | Endpoint `explain` talks to. Trailing slash tolerated.              |
| `QMARK_AI_MODEL`    | *(see below)*                  | Model id sent in the request; wins over the persisted selection.   |
| `QMARK_AI_API_KEY`  | *(unset)*                      | Sent as `Authorization: Bearer` only when set. Never logged.       |
| `QMARK_AI_TIMEOUT`  | `60`                           | Request timeout, in seconds.                                       |
| `QMARK_NO_BIND`     | *(unset)*                      | Set to `1` to load the shell integration without the `?` key binding. |

Model resolution order: `QMARK_AI_MODEL` → `~/.config/qmark/model` (written by
`qmark ai model`) → the built-in default, `qwen2.5-coder:3b`.

When the endpoint is not local (not `localhost`/`127.0.0.1`/`::1`), qmark redacts obvious
secrets — password/token/API-key flag values, `sk-`/`ghp_`-style tokens, `KEY=`/`SECRET=`
assignments — from the command line before sending it. See [SECURITY.md](SECURITY.md).

## How it works

A small Rust binary does the heavy lifting; thin shell snippets (a ZLE widget in zsh,
`bind -x` in bash) intercept `?` at the prompt and call the binary with the current line
buffer.

```mermaid
flowchart LR
    A["you type<br><code>git mv </code>"] --> B(["press <code>?</code>"])
    B --> C["shell widget<br><i>ZLE · bind -x</i>"]
    C --> D["<code>qmark suggest</code><br>harvests <code>git mv --help</code>"]
    D --> E["interactive picker<br>↑↓ · ⏎ · Esc"]
    E --> F["choice inserted<br><code>git mv --dry-run </code>"]
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full design — the `?` binding
rules, the help-resolution ladder, and the `explain` AI backend. For a walkthrough of the
project as slides: [`docs/SLIDES.md`](docs/SLIDES.md) (or the rendered
[PDF](docs/qmark-deck.pdf)).

## Roadmap

The full roadmap is in [docs/ROADMAP.md](docs/ROADMAP.md). Highlights:

- v0: repo scaffold, CLI skeleton, zsh/bash `?` binding, `--help`-based suggestions
- v0.2: local-first AI backend for `explain`, offline cache, `qmark ai status`/`qmark ai model`
- v0.3: PowerShell support, richer help sources (tldr pages, man parsing)

## Contributing

Contributions are welcome once the repository is public. In the meantime, the workflow we
follow ourselves is documented in [CONTRIBUTING.md](CONTRIBUTING.md). Please also read the
[Code of Conduct](CODE_OF_CONDUCT.md).

## Security

See [SECURITY.md](SECURITY.md) for how to report vulnerabilities.

## License

[MIT](LICENSE) © 2026 Vitruvi0
