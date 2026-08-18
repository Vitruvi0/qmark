# Security Policy

## Supported Versions

qmark is pre-alpha; only the latest commit on `main` is supported.

| Version        | Supported |
| -------------- | --------- |
| `main` (HEAD)  | ✅        |
| anything else  | ❌        |

## Reporting a Vulnerability

Please **do not open a public issue** for security problems.

Email **diegno.ragni@gmail.com** with:

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
- `qmark explain` sends command lines to an AI backend (local by default — Ollama on
  `localhost` — or a remote endpoint the user configured via `QMARK_AI_BASE_URL`). Only the
  command line the user asked about is ever sent; leaking more than that (environment,
  files, history) is in scope. For non-local endpoints, obvious secrets (password/token/
  API-key flag values, `sk-`/`ghp_`-style tokens, `KEY=`/`SECRET=`-style assignments) are
  redacted before the request is sent — a gap in that redaction is in scope. Local traffic
  is not redacted, since it never leaves the machine.
- `qmark ai model` may invoke the external `ollama` binary (`ollama pull <name>`) to install
  a model. This runs only after an explicit `y/N` confirmation — never automatically and
  never as a side effect of `qmark explain`. Anything that would trigger a pull without that
  confirmation is in scope.
