# codexw

`codexw` is an inline terminal client for the official `codex app-server`.

It does not patch Codex. It uses the Homebrew-installed vanilla `codex` binary as the backend and uses the upstream source clone under `ref/` only as protocol reference material.

## Architecture

- `codex` runs `app-server` over `stdio`
- `codexw` is the interactive terminal client
- user input while a turn is running is sent with `turn/steer`
- user input while idle is sent with `turn/start`
- `Ctrl-C` interrupts the active turn
- `Ctrl-C` while idle exits the client
- when a turn completes, `codexw` checks the final assistant response
- if the final response ends with `AUTO_MODE_NEXT=stop`, auto mode stops
- otherwise `codexw` synthesizes a continuation prompt and starts the next turn automatically

## Observability

By default `codexw` renders the structured app-server event stream into normal terminal scrollback in a human-oriented form, including:

- the final assistant reply as one complete message
- completed reasoning summaries when the model/server emits them
- full shell command lines
- completed shell command output blocks
- file-change diffs and completed file-change output blocks
- turn diff snapshots
- token usage updates
- approval requests
- local `!cmd` execution results via `command/exec`

This is protocol-level observability from Codex itself, not `RUST_LOG` tracing.

By default the client does not dump raw JSON payloads into the terminal. Raw JSON is reserved for `--raw-json`, and lower-level protocol events are hidden unless you opt into `--verbose-events`.

The client follows the normal terminal scrollback model instead of a fixed alternate-screen viewport, so you can use the terminal’s native scroll behavior rather than in-app paging.
It keeps the prompt intentionally minimal: `> ` when ready and also while a turn is running so you can steer inline. The input line now supports normal inline editing keys such as left/right arrows, Home, End, Backspace, Delete, up/down history recall, `Esc` to clear the current draft, plus common terminal shortcuts like `Ctrl-A`, `Ctrl-E`, `Ctrl-U`, and `Ctrl-W`.

## Automation Defaults

`codexw` now defaults to a fully automated posture instead of requiring `--yolo`:

- `approvalPolicy=never`
- thread sandbox mode `danger-full-access`
- turn sandbox policy `dangerFullAccess`
- automatic approval handling for:
  - `item/commandExecution/requestApproval`
  - `item/fileChange/requestApproval`
  - legacy `execCommandApproval`
  - legacy `applyPatchApproval`
- for command approvals, if the server exposes `availableDecisions`, `codexw` prefers allow decisions such as `acceptForSession`, `acceptWithExecpolicyAmendment`, `applyNetworkPolicyAmendment`, and `accept`
- best-effort auto-answer for `item/tool/requestUserInput` using the first offered option per question
- fail-safe cancellation for MCP elicitations and a non-hanging failure response for unsupported dynamic tool calls

`--yolo` is still accepted as a compatibility flag, but the client already behaves in the fully automated mode by default.

## Usage

Build locally:

```bash
cd /Users/zongbaolu/work/codexw/wrapper
cargo build --release
cp target/release/codexw /Users/zongbaolu/work/codexw/bin/codexw
```

Start a new interactive session:

```bash
/Users/zongbaolu/work/codexw/bin/codexw --cwd /path/to/repo
```

If `--cwd` is omitted, `codexw` uses the shell's current working directory and passes that resolved path to the Codex app-server explicitly.

`codexw` also forwards the standard proxy environment variables to the child `codex app-server` explicitly: `HTTPS_PROXY`, `https_proxy`, `HTTP_PROXY`, `http_proxy`, `ALL_PROXY`, `all_proxy`, `NO_PROXY`, and `no_proxy`.

Start with an initial prompt:

```bash
/Users/zongbaolu/work/codexw/bin/codexw --cwd /path/to/repo "Continue the highest-leverage engineering work."
```

Resume a thread:

```bash
/Users/zongbaolu/work/codexw/bin/codexw --resume <thread-id>
```

Useful interactive commands:

- `:help` or `/help`
- `:new` or `/new`
- `:resume <thread-id>` or `/resume <thread-id>`
- `:apps` or `/apps`
- `:skills` or `/skills`
- `:models` or `/models`
- `:mcp` or `/mcp`
- `:threads` or `/threads [query]`
- `:mention <query>` or `/mention <query>`
- `:diff` or `/diff`
- `:attach-image <path>`
- `:attach-url <url>`
- `:attachments` or `/attachments`
- `:clear-attachments`
- `:auto on|off` or `/auto on|off`
- `:interrupt` or `/interrupt`
- `:status` or `/status`
- `:quit` or `/quit`

Submission features:

- Plain input while idle starts a new turn.
- Plain input while a turn is running is sent as steer input.
- `!<shell command>` runs a local command via `command/exec` and prints the completed stdout/stderr block when it finishes.
- `:mention <query>` runs app-server fuzzy file search and prints the best matching repo paths you can paste back into a prompt.
- `:diff` prints the latest aggregated turn diff snapshot emitted by app-server.
- `:apps`, `:skills`, `:models`, `:mcp`, and `:threads` expose the most useful app-server discovery surfaces directly from the inline client.
- Raw tool mentions are resolved against the live app and skill catalogs loaded from app-server. Plugin mentions are only auto-resolved when the connected Codex build exposes plugin discovery.
- Raw app mentions work with `$<app-slug>`, for example `$demo-app`.
- Raw plugin mentions work when the connected Codex app-server exposes plugin discovery. On older builds, use explicit linked mentions such as `[$sample](plugin://sample@test)`.
- Raw skill mentions work with `$<skill-name>`, for example `$deploy`.
- Explicit linked mentions are decoded into structured app-server inputs. For example:
  - `[$figma](app://connector_1)` becomes visible text `$figma` plus a structured `mention` item.
  - `[$sample](plugin://sample@test)` becomes visible text `$sample` plus a structured `mention` item.
  - `[$my-skill](/path/to/SKILL.md)` becomes visible text `$my-skill` plus a structured `skill` item.
- `:attach-image` and `:attach-url` queue image inputs for the next submit or steer, then clear after they are sent.

## Notes

- The official `codex app-server` websocket transport exists, but upstream marks it experimental. `codexw` uses the default `stdio` transport.
- The client defaults to detailed reasoning summaries when available, but presents them as completed blocks instead of token-by-token output.
- `:status` now renders a richer session snapshot including cwd, thread/turn ids, automation mode, sandbox/approval posture, attachment counts, catalog counts, account/auth state, per-window remaining rate-limit capacity with reset times, token usage totals, and the last ready/working status line when available.
- Unknown app-server requests now receive an explicit JSON-RPC "method not implemented" error instead of being ignored, which avoids hangs from unanswered server requests.
- Full file contents are not always available from the app-server protocol. The client shows full command lines, command output, diffs, and file-change payloads that Codex emits.
- `:quit` exits immediately. `Ctrl+C` preserves Codex-like semantics: the first press interrupts a running turn, terminates an active `!command`, and only exits when the client is idle with no active draft or background work.
- While a thread switch or local command is in flight, `codexw` hides the prompt and ignores text editing keys instead of buffering invisible input that would appear later unexpectedly.
