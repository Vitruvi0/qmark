# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Repository scaffold: README, MIT license, contributing guide, code of conduct, security
  policy, issue/PR templates, CI workflow.
- `qmark` CLI skeleton (Rust + clap) with three subcommands:
  - `suggest` — contextual help for a (partial) command line, based on `--help` harvesting;
  - `explain` — plain-English explanation of a command line (AI backend not wired up yet);
  - `init` — prints the shell integration snippet for zsh or bash.
- zsh and bash integrations: Cisco-style `?` at the end of a command shows contextual help;
  `?` inside a word is inserted normally so globs keep working.
