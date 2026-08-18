# `qmark explain` — AI backend design (v0.2)

Status: implemented in v0.2 (`src/ai.rs`).
Supersedes the "The explain command (AI)" sketch in `docs/ARCHITECTURE.md`.

## Goals

1. `qmark explain "<command line>"` returns a plain-English explanation, 2–4 sentences,
   with an explicit warning when the command line contains destructive flags.
2. **Local by default.** Nothing leaves the machine unless the user points qmark at a
   remote endpoint on purpose.
3. **The AI is an add-on.** `qmark suggest` and the `?` key binding never touch this code
   and never depend on it. A machine with no model installed has a fully working qmark.
4. **Nothing hard-coded.** Endpoint and model are configuration with sensible defaults;
   someone who clones the repo runs whatever model they like, local or remote.
5. Picking and installing a model is a two-keystroke operation, not a documentation
   scavenger hunt.

## Non-goals

- A `Provider` trait. One wire format is implemented; an interface with a single
  implementation is speculative abstraction. It arrives with the second format, if ever.
- In-process inference (`candle`, `mistral.rs`, llama.cpp bindings). It needs a C++
  toolchain to build, GPU feature flags, and gigabytes of weights fetched separately —
  it would break "single static binary, no runtime required". qmark speaks HTTP to a
  runtime that is already installed.
- Streaming responses. A 2–4 sentence answer does not need incremental rendering.
- A general config file. See "Model persistence" — this spec adds one single-line file,
  not the `config.toml` scheduled for v0.4.

## 1. One backend, not three

`docs/ARCHITECTURE.md` listed three planned backends: Anthropic, OpenAI-compatible, and
Ollama. Two of those three are the same thing. Ollama, llama.cpp's server, LM Studio and
vLLM all serve the OpenAI chat-completions schema:

```
Ollama     http://localhost:11434/v1/chat/completions
llama.cpp  http://localhost:8080/v1/chat/completions
LM Studio  http://localhost:1234/v1/chat/completions
```

So "local" is not a separate implementation — it is the same request with a different
base URL. v0.2 implements **one** wire format, OpenAI chat-completions, and gets every
local runtime plus most hosted aggregators (Groq, Together, OpenRouter, vLLM) at no extra
cost. Anthropic's `/v1/messages` is a genuinely different schema and is out of scope
until someone asks for it.

### Transport

`ureq` (blocking, rustls) rather than `reqwest`. `explain` is a single synchronous
one-shot call; `reqwest` would pull in tokio and an async runtime to perform one POST.
New dependencies, in full: `ureq`, `serde` (derive), `serde_json`.

### Request

```
POST {base_url}/chat/completions
Content-Type: application/json
Authorization: Bearer {key}      # only when QMARK_AI_API_KEY is set
```

```json
{
  "model": "<resolved model>",
  "messages": [
    {"role": "system", "content": "<fixed prompt, see §4>"},
    {"role": "user",   "content": "<command line + grounding block>"}
  ],
  "stream": false,
  "max_tokens": 300,
  "temperature": 0.2
}
```

The answer is `choices[0].message.content`, trimmed. An empty or missing content field is
an error, never an empty success.

## 2. Configuration

Environment variables only. The config file stays a v0.4 item; the one exception is the
model, which needs to persist for the selection UX to feel like selection (§3).

| Variable             | Default                        | Purpose |
| -------------------- | ------------------------------ | ------- |
| `QMARK_AI_BASE_URL`  | `http://localhost:11434/v1`    | Endpoint. Trailing slash tolerated. |
| `QMARK_AI_MODEL`     | *(see resolution order below)* | Model id sent in the request. |
| `QMARK_AI_API_KEY`   | *(unset)*                      | Sent as `Authorization: Bearer` **only when set**. Never logged, never echoed. |
| `QMARK_AI_TIMEOUT`   | `60`                           | Request timeout in seconds. |

`QMARK_AI_PROVIDER` is **removed**. With a single wire format it selects nothing. It is
currently documented but unimplemented, so removing it breaks no existing user.

### Model resolution order

1. `QMARK_AI_MODEL` if set — an explicit environment variable always wins.
2. The contents of `$XDG_CONFIG_HOME/qmark/model` (falling back to `~/.config/qmark/model`),
   a single line holding a model id.
3. The built-in default, `qwen2.5-coder:3b`.

A one-line file, not a config format. It exists so that `qmark ai model` can persist a
choice; when the v0.4 config file lands it absorbs this and the file is read for
backwards compatibility.

## 3. Model selection and installation

This is the part the user interacts with, so it reuses the interaction the whole project
is built around: **the same picker as the `?` key** (`src/menu.rs` — ↑↓ to move, Enter to
choose, Esc to cancel). No new interaction vocabulary to learn.

```
qmark ai model            interactive picker
qmark ai model <name>     set directly, non-interactive (scripts, dotfiles)
```

The picker lists two groups in one list:

```
── qmark ── model ── ↑↓ move · ⏎ select · Esc cancel · 2/6
  qwen2.5-coder:3b     installed · current
  llama3.2:3b          installed
  ─ available to download ─
  qwen2.5-coder:1.5b   ~1 GB   · smallest, fastest
  qwen2.5-coder:7b     ~5 GB   · best quality of the three
  gemma3:4b            ~3 GB
  ...or run `qmark ai model <any-name>` for a model not listed
```

- **Installed models** come from `GET {base_url}/models`, the OpenAI-compatible listing
  endpoint that Ollama, llama.cpp and LM Studio all expose. This works for any backend,
  not just Ollama.
- **The download list** is a short curated set of small models suited to explaining a
  command line, shipped in the binary. Sizes are approximate and labelled as such. The
  list is a starting point, never a restriction — the last row states plainly that any
  name can be passed directly.
- Selecting an installed model writes it to `~/.config/qmark/model` and exits.
- Selecting a model that is not installed **asks for confirmation** and then runs
  `ollama pull <name>`, streaming its progress, before writing the selection. If `ollama`
  is not on `PATH`, it prints the install instructions for the detected runtime instead
  of failing silently.

`qmark ai model` on a fresh machine *is* the setup flow, so there is no separate
`qmark ai setup` command. One command, one mental model.

### Confirmation before pulling

`ollama pull` is an external program that downloads gigabytes. It runs only after an
explicit `y/N` confirmation, never automatically and never as a side effect of
`qmark explain`. `SECURITY.md` gains a scope note: `qmark ai model` may invoke `ollama`.

## 4. Grounding with harvested help

A 3B model asked to explain `tar -xzvf archive.tar.gz -C /tmp` from memory will invent
flags. qmark already has the fix: `suggest`'s parser knows the real options of the real
binary on this machine.

Before calling the model, `explain` resolves the command chain and harvests entries the
same way `suggest` does, then includes them as context:

```
Command: tar -xzvf archive.tar.gz -C /tmp

Known options for `tar` on this system:
  -x  extract files from an archive
  -z  filter the archive through gzip
  -v  verbosely list files processed
  -f  use the given archive file
  -C  change to directory
```

The model no longer needs to recall what `-z` does — it is looking at it. This is the
single highest-leverage decision in the design for small local models, and it reuses code
that already exists and is already tested.

Bounds: at most 25 entries and 1500 characters of grounding. Harvesting failures are
non-fatal — the block is simply omitted and the explanation proceeds ungrounded.

### System prompt

Fixed, not configurable in v0.2:

> Explain what the command line does, in easy English, in 2–4 sentences. Assume the reader
> does not live in the terminal. If the command is destructive or irreversible — deleting
> files, overwriting data, force-pushing, writing to a raw device — say so plainly in the
> first sentence. Do not restate the command. Do not add a preamble.

## 5. Response cache

`$XDG_CACHE_HOME/qmark/explain/<hash>.txt` (falling back to `~/.cache/...`).

- The hash comes from `std::hash::DefaultHasher` over model + command line. **No crypto
  dependency**; the cache is a local convenience, not a security boundary.
- The first line of each file records the exact model and command line; it is verified on
  read. A hash collision is therefore a cache miss, never a wrong answer served
  confidently.
- The key includes the model, so switching models does not serve a previous model's
  answers.
- Written on success only. A failed, refused, or empty response is never cached, so a
  transient backend problem cannot pin a bad answer in place.
- No expiry. What `tar -xzvf` does today is what it did yesterday. `qmark ai status`
  reports the entry count and size; deleting the directory clears it.

## 6. Secret redaction

Applied **only when the endpoint is not local** — not `localhost`, `127.0.0.1`, or `::1`.
With the default Ollama setup there is nothing to redact: the bytes never leave the
machine, and redacting would only degrade the explanation.

Token-wise scan, no `regex` dependency:

- the value following `-p`, `--password`, `--token`, `--api-key`, `--secret`;
- tokens beginning `sk-`, `ghp_`, `gho_`, `xoxb-`, `AKIA`;
- assignments whose key contains `PASSWORD`, `TOKEN`, `SECRET`, or `KEY`
  (case-insensitive) — the value is replaced, the key is kept.

Replaced with `<redacted>`.

Deliberately **not** redacted: `-P`. In `hydra -l root -P rockyou.txt` that is a wordlist
path, not a password; redacting it would damage the explanation while protecting nothing.

Only the command line the user asked about is ever sent. Never the environment, never
shell history, never file contents — unchanged from `SECURITY.md`.

## 7. CLI surface

```
qmark ai status           endpoint, model and its source, reachability, cache size
qmark ai model            interactive picker (§3)
qmark ai model <name>     set the model directly
```

Reachability is probed with `GET {base_url}/models` — the same call the picker uses, so
"reachable" in `status` means exactly "the picker will work". The probe uses a short fixed
timeout (5s) rather than `QMARK_AI_TIMEOUT`, which sizes generation, not a liveness check.

`qmark ai status` is the diagnostic to ask for in a bug report:

```
── qmark ── ai status ──────────────────────────────
  endpoint   http://localhost:11434/v1        reachable
  model      qwen2.5-coder:3b                 from ~/.config/qmark/model
  api key    not set                          (not needed for a local endpoint)
  cache      41 entries, 96 KB                ~/.cache/qmark/explain
```

## 8. Failure modes

`explain` never panics and never prints a fabricated explanation.

| Situation                    | Behaviour |
| ---------------------------- | --------- |
| Endpoint unreachable         | Explain what is missing and how to fix it — `qmark ai model` to install one, or `QMARK_AI_BASE_URL` to point elsewhere. State that `suggest` and `?` work regardless. Exit 1. |
| 404 / model not found        | Name the model and suggest `qmark ai model`. |
| Timeout                      | Suggest raising `QMARK_AI_TIMEOUT` or choosing a smaller model. |
| Non-2xx                      | Status code plus the first 200 characters of the body. |
| Empty or malformed response  | Explicit error. Never an empty success. |

## 9. Testing

- **Unit**: configuration resolution and precedence; redaction (including the `-P`
  non-case); cache key derivation and first-line verification; response parsing against a
  recorded JSON body; grounding block construction and its bounds.
- **Integration**: a `std::net::TcpListener` on an ephemeral port in a thread, returning a
  canned HTTP response — a real end-to-end exercise of the request path with **no mocking
  crate and no network in CI**. Plus a test pointing at a closed port that asserts the
  instructive failure message and a non-zero exit.
- The interactive picker is not driven in tests; `menu.rs` already falls back to a plain
  listing when there is no tty, and that path is asserted.

## 10. Files touched

```
src/ai.rs               rewritten (~280 lines: config, HTTP, cache, redaction, errors)
src/cli.rs              + `Ai { Status | Model { name: Option<String> } }`
src/main.rs             + dispatch
src/menu.rs             unchanged — reused as-is by the model picker
Cargo.toml              + ureq, serde, serde_json
tests/cli.rs            + explain and `ai status` coverage
README.md               configuration table rewritten; local-first quickstart
docs/ARCHITECTURE.md    "The explain command (AI)" rewritten + a mermaid diagram
docs/ROADMAP.md         v0.2 updated: Provider trait dropped, local promoted to default
docs/SLIDES.md          slide 6 rewritten; images and PDF regenerated
SECURITY.md             scope note: `qmark ai model` may invoke `ollama`
CHANGELOG.md            entries
```

If `src/ai.rs` passes ~300 lines, the cache moves to `src/ai/cache.rs`. Not before.

## 11. Deferred

- Anthropic `/v1/messages`, and with it any `Provider` trait.
- `config.toml` (v0.4) absorbing `~/.config/qmark/model`.
- A configurable system prompt.
- Streaming responses.
- Explaining a command line by asking the model to *use* the `?` engine as a tool.
