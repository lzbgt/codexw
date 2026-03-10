# codexw

`codexw` is an inline terminal client for the official `codex app-server`.

It does not patch Codex. It uses the Homebrew-installed vanilla `codex` binary as the backend and uses the upstream source clone under `ref/` only as protocol reference material.

The repo also includes a companion skill at `skills/session-autopilot/`. That skill does not try to monitor session lifecycle by itself; instead it defines the cooperative end-of-turn policy that `codexw` can invoke from its synthesized continuation prompt when the skill is installed. `codexw` still works on hosts without that skill because the generated continuation prompt also embeds the same core policy text directly.

For a fuller design and internals walkthrough, see [docs/codexw-design.md](docs/codexw-design.md).
For the repo-level backlog of still-open work, see [TODOS.md](TODOS.md).
For the main non-broker remaining gaps, see
[docs/codexw-native-gap-assessment.md](docs/codexw-native-gap-assessment.md).
For the remote-access track specifically, see [docs/codexw-broker-connectivity.md](docs/codexw-broker-connectivity.md): `codexw` now has an initial disabled-by-default loopback local API with health/session inspection, turn start/interrupt control, structured orchestration/shell/service/capability query routes, and a semantic SSE session event stream intended to become the broker-facing foundation.
For a small broker-style client fixture that drives the connector outside the
test suite, see [docs/codexw-broker-client-fixture.md](docs/codexw-broker-client-fixture.md).
For a concise implementation/proof snapshot of the broker/local-API prototype,
see [docs/codexw-broker-prototype-status.md](docs/codexw-broker-prototype-status.md).
For the current broker client/lease policy contract and explicit unsupported
boundary, see [docs/codexw-broker-client-policy.md](docs/codexw-broker-client-policy.md)
and [docs/codexw-broker-out-of-scope.md](docs/codexw-broker-out-of-scope.md).
For the criteria that would promote the current broker/local-API stack from a
strong prototype into a supported adapter layer, see
[docs/codexw-broker-adapter-promotion.md](docs/codexw-broker-adapter-promotion.md).

For the current recommendation on that promotion question, see
[docs/codexw-broker-promotion-recommendation.md](docs/codexw-broker-promotion-recommendation.md).
For the current frozen broker-facing adapter contract itself, see
[docs/codexw-broker-adapter-contract.md](docs/codexw-broker-adapter-contract.md).
For the operational meaning of "supported experimental adapter", including
stability and breaking-change expectations, see
[docs/codexw-broker-support-policy.md](docs/codexw-broker-support-policy.md).
For a concise mapping from those criteria to the currently verified proof
surface, see [docs/codexw-broker-proof-matrix.md](docs/codexw-broker-proof-matrix.md).
For optional broker hardening work that is useful but not a blocker for the
current recommendation, see
[docs/codexw-broker-hardening-catalog.md](docs/codexw-broker-hardening-catalog.md).

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
Long raw text output is abbreviated by default when it exceeds 80 lines, showing the first 20 lines, `...`, and the last 20 lines. That default applies to command output and text-heavy read/search-style tool results, but not to structured file-change artifacts such as diffs or update payloads. `--verbose` and `--verbose-events` both disable that abbreviation.

The client follows the normal terminal scrollback model instead of a fixed alternate-screen viewport, so you can use the terminal’s native scroll behavior rather than in-app paging.
It renders a plain inline prompt/composer in the normal terminal flow and keeps a separate transient status line above the prompt while a turn or local command is active. The prompt now wraps to the available terminal width during redraws instead of forcing a single-row preview, so long drafts remain readable and cursor movement stays aligned with what the editor is actually doing. Multiline drafts render as actual visual lines, so `Ctrl-J` moves the cursor onto the next prompt row instead of showing a synthetic newline marker in a flattened preview. The input line supports left/right arrows, Home, End, Backspace, Delete, up/down history recall, `Esc` to clear the current draft when idle and interrupt an active turn or local command when running, `Tab` for inline slash-command and `@file` completion, `Ctrl-J` to insert a newline into a multiline draft, plus common terminal shortcuts like `Ctrl-A`, `Ctrl-E`, `Ctrl-U`, and `Ctrl-W`. In multiline drafts, `Up` and `Down` now move within the draft instead of jumping into history, and Home, End, `Ctrl-A`, `Ctrl-E`, and `Ctrl-U` operate on the current line segment around the cursor instead of the whole buffer. While a turn or local command is active, `Esc` and `Ctrl-C` interrupt the work without discarding the draft you have already typed, and `Enter` is ignored while a local command owns the prompt so hidden input is not accidentally submitted. Prompt editing and cursor placement now follow grapheme/display-width boundaries rather than raw Unicode scalar counts, which makes CJK, emoji, and combining-mark input behave much more predictably even when the draft visually wraps.
The separate status line shows live request progress, including a spinner, elapsed request time, turn count, and important active detail such as waiting-on-approval state, so quiet backend work does not look like a dead terminal without bloating the prompt widget itself. That transient status is also width-bounded to a single terminal row so long path-heavy updates do not wrap and smear across redraws while the prompt itself can wrap below it. Prompt redraws are frame-deduplicated, so repeated tick events no longer rewrite an identical prompt/status frame unnecessarily.
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
- for command approvals, if the server exposes `availableDecisions`, `codexw` now prefers the strongest non-restrictive allow path first: network-policy allow amendments, execpolicy amendments, session-wide accepts, then one-shot accepts
- best-effort auto-answer for `item/tool/requestUserInput`, preferring permissive choices such as allow/accept/continue instead of blindly taking the first option
- schema-driven auto-answer for MCP form elicitations, with URL-mode elicitations cancelled safely for unattended runs
- non-hanging dynamic-tool fallback responses so the backend request lifecycle resolves cleanly

`--yolo` is still accepted as a compatibility flag, but the client already behaves in the fully automated mode by default.

## Usage

Build locally:

```bash
cd /Users/zongbaolu/work/codexw
./scripts/install-codexw
```

That installer rebuilds the release binary, copies it into `bin/codexw`, applies an ad-hoc `codesign`, installs it to `/opt/homebrew/bin/codexw`, and signs the installed copy as well.

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

If you omit the thread id, `codexw` now opens a startup resume picker for the current working directory, lists the most recently updated threads first, and lets you enter either the displayed number or a raw thread id:

```bash
/Users/zongbaolu/work/codexw/bin/codexw resume
```

Global flags such as `--cwd` can be placed either before or after the startup `resume` token. For example, `codexw resume --cwd /path/to/repo` and `codexw --cwd /path/to/repo resume` now both open the cwd-scoped resume picker when no thread id is provided.

Because the app-server `thread/list` `cwd` filter is an exact match, `codexw` now automatically falls back to an all-workspaces recent-thread list when the cwd-scoped lookup is empty, instead of leaving the startup resume picker without selectable sessions.
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
- `:clean [blockers|shells|services [@capability]|terminals]` or `/clean [blockers|shells|services [@capability]|terminals]`
- `:threads` or `/threads [query]`
- `:mention [query|n]` or `/mention [query|n]`
- `:diff` or `/diff`
- `:rollout` or `/rollout`
- `:attach-image <path>`
- `:attach-url <url>`
- `:attachments` or `/attachments`
- `:clear-attachments`
- `:auto on|off` or `/auto on|off`
- `:interrupt` or `/interrupt`
- `:status` or `/status`
- `:statusline`
- `:settings`
- `:init`
- `:feedback <category> [reason] [--logs]`
- `:logout`
- `:approvals` or `/permissions`
- `:debug-config`
- `:quit` or `/quit`

`/help` now renders the full built-in command catalog from the same metadata used by slash completion, so recognized native-style commands such as `/fast`, `/plan`, `/ps`, `/realtime`, `/theme`, `/experimental`, and related workflows appear consistently as real commands or as explicit, scoped limitations instead of drifting into unknown-command behavior.

Submission features:

- Plain input while idle starts a new turn.
- Plain input while a turn is running is sent as steer input.
- `!<shell command>` runs a local command via `command/exec` and prints the completed stdout/stderr block when it finishes.
- Inline `@path/to/file` references are resolved against the current working directory before submit when they point to a real file or directory. This gives `codexw` a scroll-native equivalent of Codex’s file-path insertion flow even without the native popup picker.
- Pressing `Tab` in the prompt completes unique slash commands like `/co -> /compact ` and unique `@file` prefixes like `@src/ma -> src/main.rs `. For ambiguous slash commands like `/re`, `codexw` now prints a numbered command shortlist with descriptions. It also surfaces fuzzy slash matches like `/ac` in a scroll-native shortlist instead of failing silently. If multiple file matches exist, `codexw` extends the common prefix and prints a short candidate list into scrollback.
- `:mention` with no args behaves like native Codex’s mention command and seeds `@` back into the prompt so you can keep typing a file reference immediately.
- `:mention <query>` runs app-server fuzzy file search and prints numbered repo paths. `:mention <n>` inserts one of those cached matches back into the current prompt draft.
- `:resume` with no id lists recent threads for the current cwd. `:resume <n>` resumes one of those cached numbered threads, which is a scroll-native equivalent of a resume picker.
- `:plan` now toggles a real collaboration-mode override through app-server. `:collab` lists available collaboration mode presets from `collaborationMode/list`, and `:collab <name|mode|default>` switches the active mode for future turns.
- `:experimental` now lists experimental feature flags directly from `experimentalFeature/list`, including lifecycle stage and enabled/default state.
- `:model` now opens a scroll-native numbered picker instead of a read-only list. It follows the native Codex TUI flow: choose a model first, then choose a reasoning effort when the model exposes multiple supported levels. Direct forms like `:model <id> [effort]` and `default` also work, and successful selections now persist `model` plus `model_reasoning_effort` into `~/.codex/config.toml`.
- `:personality` now opens a numbered picker by default, while `:personality <friendly|pragmatic|none|default>` still works for direct selection. The chosen personality is sent through `turn/start.personality` for the current session and is also persisted into `~/.codex/config.toml` for future sessions.
- `:approvals` and `:permissions` now open a numbered preset picker that updates the approval policy plus sandbox posture for later turns and local shell commands.
- `:fast` now toggles the active `serviceTier` override for later turns, matching the native “fast mode” intent instead of acting as a placeholder, and it persists the saved `service_tier` default into `~/.codex/config.toml`.
- `:theme` now opens a numbered picker over bundled syntax-highlighting themes, applies the selection immediately to rendered code blocks, persists `[tui].theme` into `~/.codex/config.toml`, and reloads that saved theme at startup.
- `:init` now follows the native Codex flow instead of stopping at a limitation message. It refuses to overwrite an existing `AGENTS.md`; otherwise it submits the upstream repository-guidelines prompt as a user turn so Codex can draft the file in-context.
- `:agent` and `:multi-agents` now use the upstream `thread/list.sourceKinds=["subAgentThreadSpawn"]` filter to show recent spawned agent threads and let you switch into one with `:resume <n>`.
- `:rollout` now follows the native Codex behavior and prints the current rollout file path when the backend has one; otherwise it explains that the path is not available yet.
- New threads now advertise a client-side dynamic tool bundle to app-server. In addition to the read-only workspace tools (`workspace_list_dir`, `workspace_stat_path`, `workspace_read_file`, `workspace_find_files`, and `workspace_search_text`), `codexw` now exposes `orchestration_status`, `orchestration_list_workers`, `background_shell_start`, `background_shell_poll`, `background_shell_send`, `background_shell_set_alias`, `background_shell_list_capabilities`, `background_shell_list_services`, `background_shell_update_service`, `background_shell_update_dependencies`, `background_shell_inspect_capability`, `background_shell_attach`, `background_shell_wait_ready`, `background_shell_invoke_recipe`, `background_shell_list`, `background_shell_terminate`, and `background_shell_clean`. That gives the model a real same-turn async workaround for long-running shell work and a first-class view into the wrapper’s orchestration graph: it can inspect worker state and next-step guidance directly, launch a background command, continue inspecting specs or code, poll logs later in the same turn, send targeted stdin back into a known shell job without blocking the whole turn, assign or clear stable in-session aliases for later reuse, list reusable service capabilities by health state, list reusable service shells by `ready` / `booting` / `untracked` / `conflicts`, update mutable metadata on a running service shell without restarting it, retarget declared dependency capabilities on a running shell job without a restart, inspect one reusable capability directly, ask for structured attachment metadata on reusable services, wait explicitly for a service `readyPattern`, invoke declared service recipes through a typed surface instead of only free-form stdin, and clean local shell work by scope when conflict resolution or dependency reset is needed. `background_shell_start` also accepts an explicit `intent` (`prerequisite`, `observation`, or `service`) plus an optional `label`, and jobs can declare `dependsOnCapabilities` so the orchestration graph can retain durable dependency intent between future work and reusable services. Service jobs can additionally declare `capabilities`, `readyPattern`, `protocol`, `endpoint`, `attachHint`, and structured `recipes` so the wrapper can distinguish booting versus ready reusable helpers, expose how later work should interact with them, and let later turns refer to them by `@capability` instead of only job ids. Those same attachment-contract fields can now be updated live on a running service shell through `background_shell_update_service` and `:ps contract`, so reusable helpers can be repurposed without restarting the underlying process; the live update path now covers `readyPattern` and `recipes` as well, and changing `readyPattern` re-evaluates readiness against already buffered service output instead of waiting for brand-new log lines. Recipes can now also declare optional `parameters`, so one service verb can be reused with call-time arguments instead of cloning near-identical recipes. Each recipe may remain descriptive-only or declare an executable `action` such as `stdin`, `http`, `tcp`, or `redis`; HTTP actions support request headers, request bodies, and `expectedStatus` validation, TCP actions support bounded payload/response probes for raw socket services and simple line protocols, and Redis actions speak RESP directly so common Redis verbs do not have to be hand-encoded over raw TCP. Executable network recipes (`http`, `tcp`, `redis`) now automatically wait for service readiness when the job declares a `readyPattern`, and `background_shell_invoke_recipe` can override or disable that wait with `waitForReadyMs`.
- In practice, that means long IO-bound or external-wait work should go through the background shell tools, while `:agent` and `:multi-agents` stay focused on parallel reasoning, investigation, and decomposition rather than just waiting on a single shell task. Use `intent=prerequisite` only when the current turn is actually gated on that shell result, `intent=observation` for non-blocking sidecar jobs such as tests or searches, and `intent=service` for helpers such as dev servers that later work may attach to. When a service shell has a concrete log milestone such as `Listening on` or `READY`, pass it as `readyPattern` so the wrapper can surface `booting` versus `ready` state instead of just “service running.”
- `thread/start`, `thread/resume`, and `thread/fork` requests now opt into upstream `persistExtendedHistory: true`, so later resume/fork/read operations can reconstruct richer thread history instead of relying on the thinner default rollout persistence.
- `:setup-default-sandbox` now uses the real app-server `windowsSandbox/setupStart` flow and persists `windows.sandbox = "elevated"` into `~/.codex/config.toml` when setup completes successfully. It remains intentionally scoped to Windows.
- `:realtime` is now a real experimental text workflow instead of a placeholder. `:realtime start [prompt...]` starts a thread-scoped realtime session, `:realtime send <text>` appends text, `:realtime stop` closes the session, and bare `:realtime`, `:realtime status`, or `:realtime show` prints the current realtime status block. Audio output deltas are intentionally not rendered in `codexw`.
- `:ps` now shows the tracked worker snapshot rather than only execution output: cached sub-agent threads from `:agent` or `:multi-agents`, server-observed thread background terminals from live `item/started`, `item/commandExecution/terminalInteraction`, and `item/completed` signals, and wrapper-owned local background shell jobs started through the dynamic-tool workaround. `:ps clean` terminates local background shell jobs immediately and, when experimental API support is enabled, also uses the real `thread/backgroundTerminals/clean` API to stop thread background terminals for the current thread.
- `:ps` also supports orchestration-aware filters now: `:ps blockers`, `:ps dependencies`, `:ps agents`, `:ps shells`, `:ps services`, `:ps capabilities`, and `:ps terminals`. That makes it possible to inspect only blocking prerequisites, only dependency edges, only cognitive workers, only local shell jobs, only reusable service shells, the live capability-to-service index, or only server-observed terminals without digging through the full mixed worker snapshot. `:ps blockers [@capability]` now narrows the blocking view to one reusable dependency role, `:ps dependencies blocking|sidecars|missing|booting|ambiguous|satisfied [@capability]` filters the dependency graph by issue/state and can also narrow it to one reusable role, `:ps capabilities @api.http` drills into one reusable role directly while `:ps capabilities healthy|missing|booting|untracked|ambiguous` filters the capability registry down to one issue class, and `:ps services ready|booting|untracked|conflicts [@capability]` now drills directly into service-shell state and can also narrow the provider set to one reusable role instead of forcing you to scan the full service list. Service shells now render `booting`, `ready`, or `untracked` explicitly when a readiness contract is available, conflict-focused service views show only the ambiguous providers, and capability views now also show which running shell jobs currently consume each reusable service role.
- `:ps guidance` now turns that same worker graph into a suggested next step instead of only a filtered snapshot. It prefers hard blockers first, then ready reusable services, then non-blocking sidecars, so the wrapper can tell you whether the highest-leverage move is to inspect blockers, wait for a service, attach to a ready helper, or simply continue foreground work. `:ps guidance @capability` now focuses that hint on one reusable role, including the case where that role is currently backed by an untracked provider that still needs live contract metadata. Global guidance now also handles untracked services directly instead of letting them disappear into generic sidecar guidance, so a lone under-described provider can surface `:ps contract ...` and `:ps relabel ...` follow-up steps before it is treated as reusable. `:ps actions` is the companion remediation view: it renders concrete operator follow-up commands such as `:ps capabilities @api.http`, `:ps provide <jobId|alias|n> <@capability...|none>`, `:ps depend <jobId|alias|n> <@capability...|none>`, `:ps contract <jobId|alias|@capability|n> <json-object>`, `:clean services @api.http`, or `:ps wait bg-1 5000` when the wrapper can infer a specific next step from the orchestration state, and untracked services now trigger explicit contract-fix suggestions instead of falling through as generic sidecars. `:ps actions @capability` narrows those suggestions to one reusable role and now surfaces those same contract-fix suggestions when the focused provider is untracked. When the wrapper already knows the exact provider or blocker job for a focused role, including booting, untracked, healthy, missing, or ambiguous focused views, those remediation hints now use that concrete mutable ref directly instead of a generic placeholder. The same concrete-ref behavior now also applies to global single-provider booting, untracked, and healthy service branches, to global booting blocker remediation when one provider is already known, to the generic single-blocker prerequisite branch, and to missing-role retarget suggestions when there is exactly one running service shell available to repurpose. When a unique ready provider also declares recipes, those global and focused guidance/action hints now prefer the best known executable recipe rather than just the first declared one, favor health/status-style verbs when available, and include concrete example JSON arguments when the selected recipe has required or defaulted parameters. They still fall back to attach-only guidance when the declared recipes are descriptive-only or otherwise non-executable. Global missing-blocker guidance now uses that same concrete service ref as well, so it can point directly at `:ps provide bg-* @capability` when a single retargetable service is already running. Generic blocker remediation now prefers concrete `poll` steps instead of implying `wait_ready` for plain prerequisite shells that do not have a readiness contract. Operator-facing guidance and prompt/status hints now consistently use session-command syntax like `:ps`, `:clean`, and `:multi-agents` instead of mixing slash and colon forms. When that focused role is missing or ambiguous, the live retarget suggestions still intentionally avoid `@capability` itself as a mutation target unless the wrapper has a unique mutable target, because the capability selector is not uniquely resolvable in those states, and ambiguous-role remediation now explicitly recommends moving one provider to `@other.role` or clearing its capability set rather than reapplying the same conflicting role.
- The model-side dynamic tool layer now has matching orchestration visibility: `orchestration_status` returns the compact orchestration summary plus a tool-native concrete `next action`, `orchestration_list_workers` renders the same worker graph with optional filters such as `blockers`, `dependencies`, `services`, `capabilities`, `terminals`, `guidance`, or `actions`, and when `filter=guidance` or `filter=actions` it now returns concrete dynamic-tool follow-up calls instead of operator session commands. That includes live retarget steps such as `background_shell_update_service` and `background_shell_update_dependencies` when a missing, ambiguous, mis-targeted, or under-described service capability can be fixed in place, and attach/wait/recipe calls when one ready or booting provider is already known. `orchestration_status.next action` now uses that same actionable tool-native surface rather than echoing the first diagnostic guidance sentence, and specifically reports the first concrete tool step rather than a descriptive headline. `filter=blockers`, `filter=guidance`, and `filter=actions` can now also take `capability=@...` to focus the result on one reusable role. `orchestration_suggest_actions` is the focused companion view for those model-side remediation steps and now also accepts an optional `capability` selector, `orchestration_list_dependencies` renders only the dependency graph with optional issue filters such as `missing`, `booting`, `ambiguous`, `blocking`, or `satisfied` and can optionally narrow the result to one `@capability`, and `background_shell_list_services` renders only reusable service shells with optional `ready`, `booting`, `untracked`, or `conflicts` filtering and can also narrow the result to a specific `@capability`. In missing or ambiguous states, those model-side mutation suggestions now likewise avoid `jobId=@capability` and instead point to concrete mutable job refs whenever the wrapper knows a unique mutable target, and the same concrete-ref behavior now also applies to global single-provider booting, untracked, and healthy guidance/action branches, global booting blocker remediation, the generic single-blocker prerequisite branch, and missing-role retarget suggestions when there is exactly one running service shell to repurpose. When a unique ready provider also declares recipes, those global and focused tool-native guidance/action paths now prefer the best known executable recipe instead of `"..."`, favor health/status-style verbs when available, and include concrete example `args` payloads when the selected recipe has required or defaulted parameters. They still fall back to attach-only guidance when the declared recipes are descriptive-only or otherwise non-executable. Generic blocker remediation now prefers concrete `background_shell_poll` steps instead of suggesting readiness waits for plain prerequisite shells. Ambiguous-role remediation still suggests either retargeting to `@other.role` or clearing `capabilities` with `null` instead of reapplying the same conflicting role.
- `:ps poll <jobId|alias|@capability|n>`, `:ps send <jobId|alias|@capability|n> <text>`, `:ps attach <jobId|alias|@capability|n>`, `:ps wait <jobId|alias|@capability|n> [timeoutMs]`, `:ps run <jobId|alias|@capability|n> <recipe> [json-args]`, and `:ps terminate <jobId|alias|@capability|n>` now provide direct per-job control for local background shell jobs. Job references accept either a stable id like `bg-2`, a session-local alias, a declared service capability like `@api.http`, or a 1-based index from the current sorted shell list.
- `:ps relabel <jobId|alias|@capability|n> <label|none>` now updates or clears the live label on a running service shell without restarting it, which is useful when a reusable helper changes role or needs a more readable operator-facing name.
- `:ps depend <jobId|alias|@capability|n> <@capability...|none>` now updates the declared dependency capabilities on a running shell job without restarting it, which lets long-lived prerequisite, observation, or service jobs retarget the orchestration graph when the dependency they are waiting on changes.
- `:ps contract <jobId|alias|@capability|n> <json-object>` now updates live attachment-contract metadata on a running service shell without restart. The JSON object can now set or clear `protocol`, `endpoint`, `attachHint`, `readyPattern`, and `recipes`, which is useful when a reusable helper changes transport, bind address, readiness signal, operator instructions, or typed interaction surface after startup.
- `:ps alias <jobId|alias|@capability|n> <name>` and `:ps unalias <name|jobId|alias|@capability|n>` now let you promote a local shell job, especially a long-lived service shell, into a stable in-session attachment target and later clear that alias either by the alias token itself or by resolving the same job through a stable id, numeric index, or unique `@capability`. Aliases use a simple token syntax and can be reused anywhere `jobId` is accepted, including later `:ps poll ...` / `:ps send ...` / `:ps attach ...` / `:ps run ...` / `:ps terminate ...` calls and the dynamic tools `background_shell_poll` / `background_shell_send` / `background_shell_attach` / `background_shell_invoke_recipe` / `background_shell_terminate`. The model-side equivalent is `background_shell_set_alias`, which assigns or clears an alias through the same reference-resolution rules. Service shells can also declare reusable `capabilities`, so later turns can resolve a single service by what it provides, for example `@api.http` or `@frontend`, instead of only by `bg-*` ids or aliases. Service attach summaries can now include a structured `protocol` plus named `recipes`, and those recipes can optionally declare executable `stdin`, `http`, `tcp`, or `redis` actions so later work can reuse a service contract like `health`, `metrics`, `seed`, `query`, `ping`, or `redis_ping` through a typed invoke surface instead of scraping raw logs or hand-crafting shell input each time. Recipes may also declare named parameters with defaults or required flags, and both `:ps run ... [json-args]` and `background_shell_invoke_recipe.args` feed those parameters into placeholder substitution. HTTP recipes can carry request headers, request bodies, and expected-status checks, TCP recipes can carry payload text, newline handling, expectation checks, and bounded read timeouts for lightweight raw-socket verification, and Redis recipes can carry RESP command arrays plus expectation checks without dropping down to raw TCP payload construction.
- `:ps provide <jobId|alias|@capability|n> <@capability...|none>` now updates the declared reusable capabilities on a running service shell in place, so the operator can retarget a reusable role without restarting the underlying job. The companion commands `:ps relabel <jobId|alias|@capability|n> <label|none>` and `:ps contract <jobId|alias|@capability|n> <json-object>` update the live service label and attachment contract in the same non-destructive way, and `:ps depend <jobId|alias|@capability|n> <@capability...|none>` updates the declared dependency-capability set for any running shell job. The model-side equivalents are `background_shell_update_service` for service metadata and `background_shell_update_dependencies` for declared dependency edges; those tool calls now also accept `null` for `capabilities` or `dependsOnCapabilities` to clear them explicitly instead of requiring an empty array workaround.
- Capability reuse is now guarded proactively too: `:ps capabilities` and `background_shell_list` render the live capability index for running service shells, including current consumers of each capability when jobs declare `dependsOnCapabilities`, and when multiple running services claim the same capability, `:ps services`, `:ps capabilities`, `background_shell_list`, and orchestration guidance surface that conflict before a later `@capability` reference fails as ambiguous. Capability resolution now intentionally ignores completed or terminated services so stale helpers do not hijack later reuse. Blocking shell jobs that depend on missing, ambiguous, or still-booting capabilities are surfaced explicitly in orchestration guidance and dependency edges instead of appearing only as generic prerequisite shells, and compact status summaries now report `cap_deps_missing`, `cap_deps_booting`, and `cap_deps_ambiguous` counts so dependency health is visible without opening `:ps`.
- Cleanup is now scoped too: `:clean blockers` terminates only prerequisite shells, `:clean blockers @api.http` terminates only the blocking prerequisite shells tied to one reusable capability dependency, `:clean shells` terminates all local background shell jobs, `:clean services` terminates only service shells, `:clean services @api.http` terminates all running service providers for one reusable role in a single step, and `:clean terminals` uses the backend terminal-cleanup API without touching local shell jobs. The same scopes work under `:ps clean ...`. The model-side equivalent is `background_shell_clean`, which supports `scope=all|blockers|shells|services`, `scope=blockers` can optionally take `capability=@...` to clear only blocker jobs tied to one reusable dependency, and `scope=services` can optionally take `capability=@...` for service-role cleanup. Agent waits remain visible in `:ps blockers`, but they are not terminable from the wrapper.
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
- `:status` now renders a richer session snapshot including cwd, thread/turn ids, started/completed turn counts, active request time, effective model capability state, personality, collaboration mode, realtime session state, automation mode, sandbox/approval posture, attachment counts, catalog counts, account/auth state, per-window remaining rate-limit capacity with reset times, token usage totals, and the last ready/working status line when available.
- `:status` now also exposes a small orchestration breakdown so the wrapper distinguishes worker classes explicitly: whether the main foreground agent is runnable or blocked, active `wait` dependencies on subagents, non-blocking sidecar subagent work, cached sub-agent threads from `:agent` or `:multi-agents`, wrapper-owned background-shell prerequisites versus sidecars versus reusable services, backend-observed thread background terminals, and explicit dependency-edge counts between tracked workers.
- Internally, that orchestration view now comes from one shared state container instead of several unrelated top-level trackers. Cached agent threads, live collab-agent tasks, backend-observed terminals, and wrapper-owned background shell jobs all live under a single orchestration model, so `:ps`, `:status`, prompt suffixes, and transcript summaries are reading the same worker graph.
- `:status` and the runtime snapshot now also expose a `next action` line derived from that orchestration state, so the wrapper surfaces the highest-priority concrete operator step directly instead of making the operator infer it from counts and dependency edges alone.
- That guidance now also accounts for service-registry health: capability conflicts are surfaced ahead of “ready for reuse” hints, so the wrapper points you to ambiguous reusable services before encouraging later turns to attach to them by role.
- The ready prompt now uses that orchestration state too instead of flattening everything into a generic background counter. When async work is still running, it distinguishes blocking prerequisite shells from sidecars, reusable services, and server terminals directly in the prompt suffix, and service shells can now show up as `booting`, `ready`, `untracked`, or `conflicted`. `:status` adds a compact `background cls` line for the same class breakdown plus service-readiness/conflict counts.
- Active turn and realtime prompt status now use the native Codex-style rolling braille spinner cadence (`⠋ … ⠏`) instead of the older ASCII spinner.
- Unknown app-server requests now receive an explicit JSON-RPC "method not implemented" error instead of being ignored, which avoids hangs from unanswered server requests.
- Full file contents are not always available from the app-server protocol. The client shows full command lines, command output, diffs, and file-change payloads that Codex emits.
- Upstream app-server does not currently expose a public client request for writing to or polling model-owned `item/commandExecution` sessions directly. The same-turn async shell workflow in `codexw` is therefore implemented as a client-side dynamic-tool workaround rather than a true reuse of the server's internal unified-exec process handles.
- `:quit` exits immediately. `Ctrl+C` preserves Codex-like semantics: the first press interrupts a running turn or terminates an active `!command` without discarding the current draft, and only exits when the client is idle with no active draft or background work. On exit, `codexw` now prints a copy-pasteable full resume command, including cwd and thread id, when one is available.
- `:sandbox-add-read-dir <absolute-directory-path>` now follows the native Windows TUI model instead of staying a placeholder: on Windows it validates the requested directory locally and refreshes sandbox read grants client-side; on non-Windows it stays hidden from help/completion and reports that the workflow is Windows-only if typed explicitly.
- The remaining behavior differences against upstream are now mostly architectural or UX-level rather than missing slash-command side effects: `codexw` still uses a scrollback-style inline terminal UI instead of the native alternate-screen widget tree, and the realtime path remains text-only rather than implementing the upstream audio UX.
- While a thread switch or local command is in flight, `codexw` hides the prompt and ignores text editing keys instead of buffering invisible input that would appear later unexpectedly.

In other words, `codexw` is now much closer to an app-server-backed Codex client than to a thin compatibility shim. The remaining high-leverage work is mostly architectural parity and UX depth, not missing command dispatch behavior.

One concrete future-work direction is brokered remote connectivity: letting `codexw` act as a remotely reachable runtime behind a cloud relay so other clients such as a mobile app, web UI, or remote terminal can drive it. The sibling `~/work/agent` project already has a broker/control-plane design and client protocol docs, so this is being treated as a real investigation item rather than an abstract wishlist. The dedicated design set now lives under [docs/codexw-broker-connectivity.md](docs/codexw-broker-connectivity.md), [docs/codexw-local-api-sketch.md](docs/codexw-local-api-sketch.md), and the related broker/local-API implementation notes, with the higher-level architecture context still summarized in [docs/codexw-design.md](docs/codexw-design.md).
