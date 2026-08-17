---
marp: true
theme: default
paginate: true
class: invert
---

<!-- _class: lead invert -->

# `qmark`

### Cisco-style `?` help, right in your terminal

AI explanations in plain English — before you run the command.

---

## The problem

- `man` pages are complete but **overwhelming**
- `--help` output is dense and **inconsistent** between tools
- New users copy-paste commands they **don't understand**

You shouldn't need to leave the terminal to learn the terminal.

---

## The idea

Cisco solved discoverability decades ago with **one keystroke**:

```
Router(config)# ip ?
  address        Set the IP address of an interface
  route          Establish static routes
  ...
```

`qmark` brings that experience to your everyday shell.

---

## What it looks like

```console
$ git ?
  ── qmark ────────────────────────────────────────────
  git — the stupid content tracker

  Common subcommands:
    clone      Clone a repository into a new directory
    status     Show the working tree status
    commit     Record changes to the repository
```

Type a command, hit `?`, get context-sensitive help. That's it.

---

## AI explanations

```console
$ qmark explain "tar -xzvf archive.tar.gz -C /tmp"
  This extracts the compressed archive "archive.tar.gz" into the
  /tmp folder, showing each file name while it works.
```

Plain English, **before** you run it.
Designed for people who don't live in the terminal all day.

---

## Features

- **`?` at the prompt** — contextual help wired into zsh and bash
- **`qmark suggest`** — the same help engine, callable directly
- **`qmark explain`** — AI-powered plain-English explanations
- **Single static binary** — written in Rust, no runtime required

Globs keep working: `ls file?.txt` inserts `?` normally.

---

## How it works

```
┌─ shell (zsh ZLE widget / bash bind -x) ─┐
│  intercepts `?` at the prompt           │
│  passes the current line buffer to…     │
└──────────────────┬──────────────────────┘
                   ▼
        ┌─ qmark (Rust binary) ─┐
        │  parses the command    │
        │  renders help / calls  │
        │  the AI backend        │
        └────────────────────────┘
```

Thin shell snippets, one small binary. Details in `docs/ARCHITECTURE.md`.

---

## Getting started

```sh
git clone https://github.com/Vitruvi0/qmark.git
cd qmark
cargo install --path .

# ~/.zshrc or ~/.bashrc
eval "$(qmark init zsh)"   # or: qmark init bash
```

---

## Roadmap

- **v0** — repo scaffold, CLI skeleton, zsh/bash `?` binding
- **v0.2** — AI backend for `explain`, offline cache
- **v0.3** — PowerShell support, tldr pages, man parsing

> Status: **pre-alpha**, under active development.

---

<!-- _class: lead invert -->

# Try it. Type `?`.

**github.com/Vitruvi0/qmark** — MIT licensed

Contributions welcome once public — see `CONTRIBUTING.md`.
