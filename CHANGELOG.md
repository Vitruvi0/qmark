# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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

- Repository scaffold: README, MIT license, contributing guide, code of conduct, security
  policy, issue/PR templates, CI workflow.
- `qmark` CLI skeleton (Rust + clap) with three subcommands:
  - `suggest` — contextual help for a (partial) command line, based on `--help` harvesting;
  - `explain` — plain-English explanation of a command line (AI backend not wired up yet);
  - `init` — prints the shell integration snippet for zsh or bash.
- zsh and bash integrations: Cisco-style `?` at the end of a command shows contextual help;
  `?` inside a word is inserted normally so globs keep working.
