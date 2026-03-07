# codexw

`codexw` is an inline terminal client for the official `codex app-server`.

It does not patch Codex. It uses the Homebrew-installed vanilla `codex` binary as the backend and uses the upstream source clone under `ref/` only as protocol reference material.

The repo also includes a companion skill at `skills/session-autopilot/`. That skill does not try to monitor session lifecycle by itself; instead it defines the cooperative end-of-turn policy that `codexw` can invoke from its synthesized continuation prompt when the skill is installed. `codexw` still works on hosts without that skill because the generated continuation prompt also embeds the same core policy text directly.

For a fuller design and internals walkthrough, see [docs/codexw-design.md](docs/codexw-design.md).

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

By default `codexw` renders the structured app-server event stream into normal terminal scrollback in a richer human-oriented form, using a palette intentionally closer to the upstream Codex TUI markdown and diff styling, including:

- the final assistant reply as a styled markdown-like block
- completed reasoning summaries in dimmed secondary text
- full shell command lines with command/status/output sections styled separately
- completed shell command output blocks with shell-oriented highlighting
- colored file-change diffs and completed file-change output blocks
- turn diff snapshots
- token usage updates
- approval requests
- local `!cmd` execution results via `command/exec`

This is protocol-level observability from Codex itself, not `RUST_LOG` tracing. The renderer now uses the same `ratatui` text primitives (`Text`, `Line`, `Span`) that the upstream Codex TUI is built around, but emits them into normal terminal scrollback instead of taking over the terminal with an alternate screen.

By default the client does not dump raw JSON payloads into the terminal. Raw JSON is reserved for `--raw-json`, and lower-level protocol events are hidden unless you opt into `--verbose-events`.

The client follows the normal terminal scrollback model instead of a fixed alternate-screen viewport, so you can use the terminal’s native scroll behavior rather than in-app paging.
It renders a plain inline prompt/composer in the normal terminal flow and keeps a separate transient status line above the prompt while a turn or local command is active. The prompt is intentionally elided to a single terminal row during redraws so long drafts do not wrap and leave duplicated prompt lines behind. The input line supports left/right arrows, Home, End, Backspace, Delete, up/down history recall, `Esc` to clear the current draft when idle and interrupt an active turn or local command when running, `Tab` for inline slash-command and `@file` completion, `Ctrl-J` to insert a newline into a multiline draft, plus common terminal shortcuts like `Ctrl-A`, `Ctrl-E`, `Ctrl-U`, and `Ctrl-W`. Prompt editing and cursor placement now follow grapheme/display-width boundaries rather than raw Unicode scalar counts, which makes CJK, emoji, and combining-mark input behave much more predictably.
The separate status line shows live request progress, including a spinner, elapsed request time, turn count, and important active detail such as waiting-on-approval state, so quiet backend work does not look like a dead terminal without bloating the prompt widget itself.
To reduce transcript duplication, codexw now prefers that transient status line over appending `[status] ...` chatter to scrollback, and it avoids printing separate "started" transcript blocks for commands and file changes that already produce a completed result block later.

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

Codex-style startup resume is also accepted:

```bash
/Users/zongbaolu/work/codexw/bin/codexw resume <thread-id>
```

On resume, `codexw` now renders the latest 10 conversation messages from the stored thread so you get immediate context before entering a new prompt, without replaying the full internal reasoning/tool trace.
Resume startup is also faster now: `codexw` sends the thread create or resume request before non-critical catalog and account lookups, and it only scans the minimum recent conversation history needed for the preview and continuation state.

Install the companion `session-autopilot` skill on another host:

```bash
python3 ~/.codex/skills/.system/skill-installer/scripts/install-skill-from-github.py \
  --repo lzbgt/codexw \
  --path skills/session-autopilot
```

That installs the skill into `~/.codex/skills/session-autopilot/`. Restart Codex after installing so the new skill is loaded.

If that host does not have the installer helper available, the manual fallback is:

```bash
mkdir -p ~/.codex/skills/session-autopilot/agents
curl -L https://raw.githubusercontent.com/lzbgt/codexw/main/skills/session-autopilot/SKILL.md \
  -o ~/.codex/skills/session-autopilot/SKILL.md
curl -L https://raw.githubusercontent.com/lzbgt/codexw/main/skills/session-autopilot/agents/openai.yaml \
  -o ~/.codex/skills/session-autopilot/agents/openai.yaml
```

Useful interactive commands:

- `:help` or `/help`
- `:new` or `/new`
- `:resume <thread-id>` or `/resume <thread-id>`
- `:fork` or `/fork`
- `:compact` or `/compact`
- `:review [instructions]` or `/review [instructions]`
- `:clear` or `/clear`
- `:copy` or `/copy`
- `:rename <name>` or `/rename <name>`
- `:apps` or `/apps`
- `:skills` or `/skills`
- `:models` or `/models`
- `:model` or `/model`
- `:mcp` or `/mcp`
- `:clean` or `/clean`
- `:threads` or `/threads [query]`
- `:mention [query|n]` or `/mention [query|n]`
- `:diff` or `/diff`
- `:attach-image <path>`
- `:attach-url <url>`
- `:attachments` or `/attachments`
- `:clear-attachments`
- `:auto on|off` or `/auto on|off`
- `:interrupt` or `/interrupt`
- `:status` or `/status`
- `:statusline`
- `:settings`
- `:feedback <category> [reason] [--logs]`
- `:logout`
- `:approvals` or `/permissions`
- `:debug-config`
- `:quit` or `/quit`

`/help` now renders the full built-in command catalog from the same metadata used by slash completion, so recognized native-style commands such as `/fast`, `/plan`, `/ps`, `/realtime`, `/theme`, `/experimental`, and related workflows appear there even when `codexw` can currently only explain their limitation rather than reproduce the native popup UI.

Submission features:

- Plain input while idle starts a new turn.
- Plain input while a turn is running is sent as steer input.
- `!<shell command>` runs a local command via `command/exec` and prints the completed stdout/stderr block when it finishes.
- Inline `@path/to/file` references are resolved against the current working directory before submit when they point to a real file or directory. This gives `codexw` a scroll-native equivalent of Codex’s file-path insertion flow even without the native popup picker.
- Pressing `Tab` in the prompt completes unique slash commands like `/co -> /compact ` and unique `@file` prefixes like `@src/ma -> src/main.rs `. For ambiguous slash commands like `/re`, `codexw` now prints a numbered command shortlist with descriptions. It also surfaces fuzzy slash matches like `/ac` in a scroll-native shortlist instead of failing silently. If multiple file matches exist, `codexw` extends the common prefix and prints a short candidate list into scrollback.
- `:mention` with no args behaves like native Codex’s mention command and seeds `@` back into the prompt so you can keep typing a file reference immediately.
- `:mention <query>` runs app-server fuzzy file search and prints numbered repo paths. `:mention <n>` inserts one of those cached matches back into the current prompt draft.
- `:resume` with no id lists recent threads for the current cwd. `:resume <n>` resumes one of those cached numbered threads, which is a scroll-native equivalent of a resume picker.
- `:diff` prints the latest aggregated turn diff snapshot emitted by app-server.
- `:apps`, `:skills`, `:models`, `:mcp`, and `:threads` expose the most useful app-server discovery surfaces directly from the inline client.
- `:settings` loads the effective backend config snapshot, `:statusline` aliases `:status`, and `:logout` signs out through app-server then refreshes account/rate-limit state.
- `:feedback <category> [reason] [--logs]` submits feedback through app-server. Supported categories match upstream Codex classifications: `bug`, `bad_result`, `good_result`, `safety_check`, and `other`. Short aliases like `good`, `bad`, and `safety` are accepted.
- `:review` with no args reviews uncommitted changes; with args it runs a custom inline review request through `review/start`.
- `:compact`, `:fork`, `:rename`, and `:clean` are backed by the corresponding app-server thread APIs.
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
- `:status` now renders a richer session snapshot including cwd, thread/turn ids, started/completed turn counts, active request time, automation mode, sandbox/approval posture, attachment counts, catalog counts, account/auth state, per-window remaining rate-limit capacity with reset times, token usage totals, and the last ready/working status line when available.
- Unknown app-server requests now receive an explicit JSON-RPC "method not implemented" error instead of being ignored, which avoids hangs from unanswered server requests.
- Full file contents are not always available from the app-server protocol. The client shows full command lines, command output, diffs, and file-change payloads that Codex emits.
- `:quit` exits immediately. `Ctrl+C` preserves Codex-like semantics: the first press interrupts a running turn, terminates an active `!command`, and only exits when the client is idle with no active draft or background work.
- Some native Codex slash commands still map to informative placeholders in `codexw` instead of full popup UIs. The client now recognizes those commands explicitly rather than treating them as unknown, and where app-server does not expose the necessary backend state cleanly, the message explains that limitation directly.
- While a thread switch or local command is in flight, `codexw` hides the prompt and ignores text editing keys instead of buffering invisible input that would appear later unexpectedly.
