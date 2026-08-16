# Security Policy

## Supported Versions

qmark is pre-alpha; only the latest commit on `main` is supported.

| Version        | Supported |
| -------------- | --------- |
| `main` (HEAD)  | ✅        |
| anything else  | ❌        |

## Reporting a Vulnerability

Please **do not open a public issue** for security problems.

Email **rd.team@advaisor.it** with:

- a description of the issue and its impact,
- steps to reproduce (a minimal PoC helps a lot),
- the commit hash or version you tested.

We will acknowledge your report within 5 working days and keep you updated while we work on
a fix. Once the repository is public, we will credit reporters in the release notes unless
they prefer to stay anonymous.

## Scope notes

Things we consider security-relevant in this project:

- `qmark suggest` executes the target program with `--help` to harvest its output. Anything
  that lets an attacker leverage this to run *other* commands or arguments is in scope.
- The shell snippets printed by `qmark init` run inside the user's interactive shell.
  Injection into that snippet (e.g. via environment variables) is in scope.
- `qmark explain` will send command lines to an AI backend. Leaking more than the user
  asked to share (environment, files, history) is in scope.
