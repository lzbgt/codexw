# codexw Design, Internals, and Features

## Overview

`codexw` is a local terminal client for the official `codex app-server`.

It does not patch the upstream Codex binary. Instead, it:

- starts the vanilla `codex app-server` process
- speaks JSON-RPC over stdio
- renders Codex activity into normal terminal scrollback
- keeps an inline prompt/editor active for new turns and steer input
- auto-continues work between turns unless the final assistant reply explicitly ends with `AUTO_MODE_NEXT=stop`

The implementation lives in `wrapper/`, while `ref/` is only a local upstream reference checkout.

## Design Goals

The current design optimizes for:

- using the official installed `codex` backend instead of a fork
- scroll-native terminal behavior rather than an alternate-screen TUI
- high observability without dumping raw protocol noise by default
- fully automated operation by default
- native-feeling inline interaction for submit, steer, interrupt, resume, and command workflows
- explicit auto-continue control through an assistant footer marker instead of implicit idle heuristics

## High-Level Architecture

The runtime has thirteen main layers.

1. Backend process management
   `runtime_process.rs`, `runtime_event_sources.rs`, and `runtime_keys.rs` start `codex app-server`, wire stdio, forward key environment such as proxy variables, own raw-mode lifecycle, and manage stdin/stdout/tick event sources. Callers now import the process/input helpers directly from `runtime_event_sources.rs` and `runtime_keys.rs` instead of routing through an extra facade.

2. JSON-RPC transport
   `rpc.rs` defines the wire-level request, response, notification, and request-id types, plus JSON parsing for inbound lines.

3. Outbound request construction
   `requests.rs`, `requests/request_types.rs`, `requests/bootstrap_init.rs`, `requests/bootstrap_account.rs`, `requests/bootstrap_catalog_core.rs`, `requests/bootstrap_catalog_lists.rs`, `requests/bootstrap_search.rs`, `requests/thread_switch_common.rs`, `requests/thread_maintenance.rs`, `requests/thread_realtime.rs`, `requests/thread_review.rs`, `requests/turn_start.rs`, `requests/turn_control.rs`, and `requests/command_requests.rs` own JSON-RPC request building and pending-request bookkeeping for initialize, account/catalog bootstrap, thread lifecycle, turn, command, review, and realtime actions. `requests.rs` now imports the catalog, thread start/resume/fork, realtime/review, and turn request builders directly instead of routing through extra middle facades.

4. Inbound event handling
   `events.rs`, `event_request_approvals.rs`, `event_request_tools.rs`, `response_bootstrap_init.rs`, `response_bootstrap_catalog_state.rs`, `response_bootstrap_catalog_views.rs`, `response_thread_runtime.rs`, `response_thread_loaded.rs`, `response_thread_maintenance.rs`, `response_error_session.rs`, `response_error_runtime.rs`, `notification_realtime.rs`, `notification_turn_started.rs`, `notification_turn_completed.rs`, `notification_item_buffers.rs`, `notification_item_status.rs`, and `notification_item_completion.rs` own inbound JSON-RPC routing, server-request handling, approval-request handling, tool/user-input request handling, response handling, notification handling, realtime events, turn/item events, delta/status buffering, and item-completion rendering. `events.rs` now routes directly to approval/tool requests, realtime notifications, turn lifecycle handlers, item-completion rendering, item buffer/status updates, bootstrap response handlers, and concrete thread response helpers without extra request/response router facades.

5. Catalog parsing
   `catalog.rs` owns app and skill catalog parsing from app-server payloads.

6. Shared state and text/buffer helpers
   `state.rs` and `state_helpers.rs` own `AppState`, process-output buffering, attachment queues, request bookkeeping, request-id generation, state reset/attachment mutations, and shared utility helpers such as response-path string extraction, summarized status text, and streamed item/process delta buffering. `state.rs` is the concrete shared runtime surface.

7. Runtime policy
   `policy.rs` owns approval policy, sandbox policy, reasoning-summary policy, shell selection, and approval-decision preference logic shared by requests, status rendering, and approval handling.

8. Session and turn orchestration
   `model_catalog.rs`, `model_personality_view.rs`, `model_personality_actions.rs`, `collaboration_preset.rs`, `collaboration_view.rs`, `collaboration_apply.rs`, `session_prompt_status_active.rs`, `session_prompt_status_ready.rs`, `session_realtime_status.rs`, `session_realtime_item.rs`, `session_snapshot_overview.rs`, `session_snapshot_runtime.rs`, `response_realtime_activity.rs`, `response_turn_activity.rs`, and `response_local_command.rs` own model metadata, personality selection, collaboration mode handling, prompt/realtime status rendering, status snapshot generation, and the concrete thread-activity success handlers for realtime, turns, reviews, and local commands. Prompt-status callers now import the concrete helpers directly through `prompt_state.rs`.

9. App runtime loop
   `app.rs`, `app_input_editor.rs`, `app_input_editing.rs`, `app_input_controls.rs`, and `app_input_interrupt.rs` own process wiring, the main event loop, keyboard-event dispatch, editor-key actions, submit/escape/interrupt behavior, and control-key routing for the live interactive session. `app.rs` now owns input-key routing directly while the smaller helper modules own editor, editing, and control behavior.

10. Resume and history rendering
   `history_render.rs`, `history_state.rs`, and `history_text.rs` own resumed-thread state seeding, compact conversation-history extraction, resumed history rendering, and shared history text normalization/user-message rendering. Callers now import the concrete render/state helpers directly instead of routing through a separate `history.rs` facade.

11. View and transcript rendering helpers
  `catalog_connector_views.rs`, `catalog_feature_views.rs`, `catalog_backend_views.rs`, `catalog_thread_list.rs`, `catalog_file_search.rs`, `status_value.rs`, `status_config.rs`, `status_account.rs`, `status_rate_windows.rs`, `status_rate_credits.rs`, `status_token_usage.rs`, `transcript_completion_render.rs`, `transcript_plan_render.rs`, `transcript_approval_summary.rs`, `transcript_item_summary.rs`, and `transcript_status_summary.rs` own app-server-facing display helpers for catalogs, status summaries, thread listings, token/rate-limit rendering, item completion blocks, and approval/request summaries. Generic JSON value formatting now lives in `status_value.rs`, while transcript callers import the concrete helper modules directly.

12. Human input handling
   `editor.rs`, `editor_buffer.rs`, `editor_history.rs`, `editor_graphemes.rs`, `editor_tests.rs`, `input.rs`, `input/input_types.rs`, `input/input_decode_mentions.rs`, `input/input_decode_mention_links.rs`, `input/input_decode_mention_paths.rs`, `input/input_decode_inline_mentions.rs`, `input/input_decode_inline_paths.rs`, `input/input_decode_inline_skills.rs`, `input/input_decode_tokens.rs`, `input/input_resolve_tools.rs`, `input/input_resolve_catalog.rs`, `input/input_build_items.rs`, `input/input_build_mentions.rs`, `dispatch_submit_commands.rs`, `dispatch_submit_turns.rs`, `dispatch_commands.rs`, `dispatch_command_thread_common.rs`, `dispatch_command_thread_navigation_session.rs`, `dispatch_command_thread_navigation_identity.rs`, `dispatch_command_thread_review.rs`, `dispatch_command_thread_control.rs`, `dispatch_command_thread_view.rs`, `dispatch_command_thread_draft.rs`, `dispatch_command_session_catalog_lists.rs`, `dispatch_command_session_catalog_models.rs`, `dispatch_command_session_status.rs`, `dispatch_command_session_collab.rs`, `dispatch_command_session_realtime.rs`, `dispatch_command_session_ps.rs`, `dispatch_command_session_meta.rs`, `dispatch_command_utils.rs`, `prompt_state.rs`, `prompt_file_completions_token.rs`, and `prompt_file_completions_search.rs` implement the inline editor, editor regression coverage, grapheme-aware cursor helpers, command dispatch, slash/file completion, linked-mention decoding, inline-file/token decoding, attachment handling, catalog-driven mention resolution, prompt visibility/redraw, and structured app-server user input construction. The editor now supports real multiline vertical navigation inside the draft, and the control path ignores submit while a local command owns the prompt so invisible buffered input does not leak into later turns. `editor.rs` remains the public editor surface, `input.rs` remains the input namespace root and structured `UserInput` entrypoint, and `dispatch_commands.rs` remains the top-level slash-command router.

13. Human output handling
   `output.rs`, `render_prompt.rs`, `render_block_common.rs`, `render_block_markdown.rs`, `render_markdown_block_structures.rs`, `render_markdown_code.rs`, `render_markdown_inline.rs`, `render_markdown_links.rs`, `render_markdown_styles.rs`, `render_block_structured.rs`, and `render_ansi.rs` convert app-server events into readable terminal output with markdown-like styling, colored diffs, command blocks, status lines, and a wrapped inline prompt redraw path. `output.rs` now owns prompt redraw, committed stream output, block-level ANSI assembly, wrapped prompt row management, and frame deduplication so periodic ticks only repaint when the prompt or transient status actually changes, while `render_prompt.rs` owns prompt fitting and committed prompt rendering directly.

Session feature helpers are split across `model_catalog.rs`, `model_personality_view.rs`, `model_personality_actions.rs`, `collaboration_preset.rs`, `collaboration_view.rs`, `collaboration_apply.rs`, `session_prompt_status_active.rs`, `session_prompt_status_ready.rs`, `session_realtime_status.rs`, `session_realtime_item.rs`, `session_snapshot_overview.rs`, and `session_snapshot_runtime.rs`, with prompt-state callers now routing directly through `prompt_state.rs`.
Runtime policy helpers live in `policy.rs`: approval, sandbox, reasoning-summary, shell-program, and approval-choice logic.
App loop helpers are split across `app.rs`, `app_input_editor.rs`, `app_input_editing.rs`, `app_input_controls.rs`, and `app_input_interrupt.rs`: `app.rs` owns backend/session startup, the top-level runtime loop, and input-key routing; `app_input_editor.rs` owns editor-key behavior and submit handling; `app_input_editing.rs` routes editing/navigation keys; and `app_input_controls.rs` plus `app_input_interrupt.rs` own control, interrupt, and exit behavior.
Resume-preview helpers live across `history_render.rs`, `history_state.rs`, and `history_text.rs`, and callers now import those concrete helpers directly for recent conversation extraction, resumed objective/last-reply seeding, resumed transcript rendering, and shared history text formatting.
Catalog display helpers are split across `catalog_connector_views.rs`, `catalog_feature_views.rs`, `catalog_backend_views.rs`, `catalog_thread_list.rs`, and `catalog_file_search.rs`, and callers now import those concrete app/skill/experimental/thread/search helpers directly.
Status display helpers are split across `status_value.rs`, `status_config.rs`, `status_account.rs`, `status_rate_windows.rs`, `status_rate_credits.rs`, and `status_token_usage.rs`, with rate-limit and token-usage callers importing the concrete helpers directly.
Transcript display helpers now live directly across `transcript_completion_render.rs`, `transcript_plan_render.rs`, `transcript_approval_summary.rs`, `transcript_item_summary.rs`, and `transcript_status_summary.rs`, without an extra transcript compatibility layer in the runtime path.
Runtime helpers live across `runtime_process.rs`, `runtime_event_sources.rs`, and `runtime_keys.rs`, with backend process startup, raw terminal mode, key mapping, and event-source threads now imported directly from those concrete modules.
Catalog helpers live in `catalog.rs`: app and skill list extraction for the current workspace.
Shared state helpers now live across `state.rs` and `state_helpers.rs`, with `state.rs` owning `AppState`, `ProcessOutputBuffer`, request-id generation, constructor/reset helpers, and attachment transfer behavior directly.
Command catalog helpers are split across `commands_entry_session_catalog.rs`, `commands_entry_session_modes.rs`, `commands_entry_thread.rs`, `commands_entry_runtime.rs`, and `commands_catalog.rs`: grouped command-entry data lives in the `commands_entry_*` modules, and `commands_catalog.rs` assembles the shared table directly while providing the public entrypoint, descriptions, help-line generation, and stable command-name ordering.
Command completion helpers live in `commands_completion_apply.rs`, `commands_completion_render.rs`, and `commands_match.rs`: completion application stays in the extracted apply helper, rendering and quoting stay in the render helper, and cursor parsing, fuzzy scoring, and prefix logic live in the matcher module.
Command-dispatch helpers are split across `dispatch_submit_commands.rs`, `dispatch_submit_turns.rs`, `dispatch_command_thread_common.rs`, `dispatch_command_thread_navigation_session.rs`, `dispatch_command_thread_navigation_identity.rs`, `dispatch_command_thread_review.rs`, `dispatch_command_thread_control.rs`, `dispatch_command_thread_view.rs`, `dispatch_command_thread_draft.rs`, `dispatch_command_session_catalog_lists.rs`, `dispatch_command_session_catalog_models.rs`, `dispatch_command_session_status.rs`, `dispatch_command_session_collab.rs`, `dispatch_command_session_realtime.rs`, `dispatch_command_session_ps.rs`, `dispatch_command_session_meta.rs`, and `dispatch_command_utils.rs`, with `dispatch_commands.rs` kept as the top-level slash-command router.
Input helpers are split across `input/input_types.rs`, `input/input_decode_mentions.rs`, `input/input_decode_mention_links.rs`, `input/input_decode_mention_paths.rs`, `input/input_decode_inline_mentions.rs`, `input/input_decode_inline_paths.rs`, `input/input_decode_inline_skills.rs`, `input/input_decode_tokens.rs`, `input/input_resolve_tools.rs`, `input/input_resolve_catalog.rs`, `input/input_build_items.rs`, and `input/input_build_mentions.rs`, with `input.rs` kept as the namespace root and public structured-input entrypoint.
Prompt helpers live across `prompt_state.rs`, `prompt_file_completions_token.rs`, and `prompt_file_completions_search.rs`, with prompt visibility/input gating, prompt redraw, slash completion, and `@file` completion now imported directly from the concrete helper modules.
Render helpers live across `render_prompt.rs`, `render_markdown_block_structures.rs`, `render_markdown_links.rs`, and `render_markdown_styles.rs`, with `render_prompt.rs` owning wrapped prompt-layout and committed-prompt behavior directly while `render_block_markdown.rs` and `render_markdown_inline.rs` still wrap the markdown subhelpers.
Response helpers are split across `response_bootstrap_init.rs`, `response_bootstrap_catalog_state.rs`, `response_bootstrap_catalog_views.rs`, `response_thread_maintenance.rs`, `response_thread_runtime.rs`, `response_thread_loaded.rs`, `response_error_session.rs`, `response_error_runtime.rs`, `response_realtime_activity.rs`, `response_turn_activity.rs`, and `response_local_command.rs`, with `events.rs` routing directly to those concrete success and error handlers for pending outbound requests.
Notification helpers are split across `notification_realtime.rs`, `notification_turn_started.rs`, `notification_turn_completed.rs`, `notification_item_buffers.rs`, `notification_item_status.rs`, and `notification_item_completion.rs`, with `events.rs` routing directly to realtime, turn, and item handlers without extra notification-router facades.

## Process Model

At startup, `codexw`:

1. resolves the effective working directory
2. spawns `codex app-server`
3. enters terminal raw mode
4. starts three event sources:
   - server stdout reader
   - keyboard input reader
   - periodic tick timer
5. runs one main event loop that consumes all app, backend, and user events

The main event enum is `AppEvent` in `runtime_event_sources.rs` and is consumed by the top-level loop in `app.rs`. It merges:

- server JSON-RPC lines
- normalized keyboard events
- periodic ticks for spinner and elapsed-time updates
- backend or stdin closure notifications

This keeps all user-visible state transitions serialized through one place.

## Core State

`AppState` in `state.rs` is the central state store. It tracks:

- active thread id
- active turn id
- active local command process id
- whether a thread switch is in flight
- whether a turn is currently running
- elapsed activity start time
- started and completed turn counters
- auto-continue mode
- session objective
- latest assistant message
- latest turn diff
- token usage
- account and rate-limit state
- attachment queues
- app, plugin, and skill catalogs
- experimental realtime session state
- recent thread-list and file-search caches
- the last status line
- pending JSON-RPC requests keyed by request id
- streamed command/file/process output buffers used to coalesce incremental backend events into completed render blocks
- cached agent-thread ids from the most recent `/agent` or `/multi-agents` listing

## Orchestration Model

`codexw` should be understood as a control plane around Codex, not as an extra reasoning agent.

The intended runtime roles are:

- main agent
  - the foreground Codex thread created or resumed through app-server
  - owns the user conversation and the critical path
- cognitive workers
  - spawned sub-agent threads created by Codex's collaboration tools such as `spawn_agent`
  - used for bounded investigation, decomposition, alternative approaches, or disjoint code work
- execution workers
  - wrapper-owned background shell jobs started through client dynamic tools
  - used for long-running shell work that would otherwise block the main turn on IO or elapsed time
- observed backend terminals
  - thread-scoped background terminals surfaced by app-server notifications
  - visible to the wrapper, but not directly controlled with native unified-exec semantics

This is deliberately close to upstream Codex's split:

- `/agent` and `/multi-agents` are thread-selection UI, not the orchestration engine
- the real multi-agent workflow in upstream Codex is the collaboration tool surface:
  - `spawn_agent`
  - `send_input`
  - `resume_agent`
  - `wait`
  - `close_agent`
- app-server does not currently expose a public client request that can write to or poll model-owned `item/commandExecution` sessions directly

Because of that protocol boundary, `codexw` uses wrapper-owned background shell tools as the execution-worker workaround for same-turn async shell behavior.

### Scheduling policy

The intended orchestration policy in `codexw` is:

- keep the immediate blocking task on the main agent
- use sub-agents for self-contained cognitive sidecar work that can run in parallel
- use background shell jobs for long IO-bound or external-wait tasks such as builds, tests, servers, crawls, downloads, or large searches
- avoid waiting by reflex; only wait when a result becomes a critical-path dependency
- only parallelize code edits when write scopes are disjoint
- prefer reusing an existing sub-agent with `send_input` when context continuity matters

In practical terms:

- `/agent` and `/multi-agents` should expose and switch cognitive workers
- `/ps` should expose execution workers and observed backend terminals
- `/status` should summarize the orchestration state explicitly instead of collapsing it into a generic background counter

### Current implementation state

The current `codexw` implementation now reflects that model partially:

- new threads advertise client dynamic tools for wrapper-owned background shells:
  - `orchestration_status`
  - `orchestration_list_workers`
  - `background_shell_start`
  - `background_shell_poll`
  - `background_shell_send`
  - `background_shell_list_capabilities`
  - `background_shell_list_services`
  - `background_shell_attach`
  - `background_shell_wait_ready`
  - `background_shell_invoke_recipe`
  - `background_shell_list`
  - `background_shell_terminate`
- `background_shell_start` now accepts explicit orchestration intent:
  - `prerequisite` for critical-path shell work that should count as blocking
  - `observation` for non-blocking sidecar execution such as tests, searches, or crawls
  - `service` for reusable long-lived helpers such as dev servers
  - optional `label` text for operator-facing status and `/ps` output
  - optional `capabilities` array for service jobs, so later turns can resolve helpers by role such as `@api.http` or `@frontend`
  - optional `readyPattern` text for service jobs, so the wrapper can promote them from `booting` to `ready` when logs match a concrete milestone
  - optional `protocol`, `endpoint`, and `attachHint` text for service jobs, so the wrapper can expose structured attachment metadata for later turns and worker tasks
  - optional `recipes` array for service jobs, so the wrapper can expose named interaction verbs such as `health`, `metrics`, `query`, or `seed`
  - optional per-recipe `parameters` array, so one declared service verb can be reused with call-time arguments instead of cloning separate recipes for each key/path/payload variant
  - each recipe may remain descriptive-only or declare an executable `action`, currently `stdin`, plain `http`, `tcp`, or `redis`
  - `http` actions may also declare request `headers`, request `body`, and `expectedStatus` validation so the recipe contract can describe authenticated or mutating service interactions, not just basic GETs
  - `tcp` actions may declare bounded `payload`, `appendNewline`, `expectSubstring`, and `readTimeoutMs` fields so the recipe contract can describe raw socket probes and simple line protocols without falling back to ad hoc shell commands
  - `redis` actions may declare RESP command arrays plus expectation checks so Redis-family services can expose typed verbs without forcing the caller to manually serialize raw RESP over a generic `tcp` action
- live `collabAgentToolCall` items are now tracked as in-turn cognitive work:
  - active collab-agent calls are kept in a live registry while the turn is running
  - `receiverThreadIds` and `agentsStates` opportunistically refresh the cached agent-thread view even before the user runs `/multi-agents`
  - `wait` calls are tracked separately as main-agent dependencies, so the wrapper can tell the difference between "subagents are running" and "the foreground agent is actually blocked waiting on them"
- `/ps` renders the tracked worker snapshot: cached cognitive workers from `/agent` or `/multi-agents`, backend-observed background terminals, and wrapper-owned background shell jobs
- `/ps` now also supports worker-class filters:
  - `:ps blockers` for blocking waits and prerequisite shells
  - `:ps dependencies` for the current dependency-edge graph without the rest of the worker snapshot
    - `:ps dependencies blocking|sidecars|missing|booting|ambiguous|satisfied [@capability]` filters the dependency graph to one issue/state class and can narrow it to a single reusable role
  - `:ps agents` for cognitive workers only
  - `:ps shells` for wrapper-owned local shell jobs only
  - `:ps services` for reusable service shells only
    - `:ps services ready|booting|untracked|conflicts` filters the service-shell registry to one service-health class
  - `:ps capabilities` for the live capability-to-service index across reusable service shells
    - `:ps capabilities @api.http` drills into one capability directly and shows providers plus current consumers
    - `:ps capabilities healthy|missing|booting|ambiguous` filters the capability index to one health/issue class
  - `:ps terminals` for backend-observed terminals only
- `/ps` also has per-job local-shell actions now:
  - `:ps attach <jobId|alias|@capability|n>` renders the structured attachment metadata for one service shell job
  - `:ps wait <jobId|alias|@capability|n> [timeoutMs]` blocks until one service shell reaches its declared `readyPattern`
  - `:ps run <jobId|alias|@capability|n> <recipe> [json-args]` invokes one declared service recipe through the wrapper-owned typed action layer, with optional per-invocation arguments
  - `:ps poll <jobId|alias|@capability|n>` renders the full current poll snapshot for one wrapper-owned shell job
  - `:ps send <jobId|alias|@capability|n> <text>` sends targeted stdin back into one wrapper-owned shell job without blocking the turn
  - `:ps terminate <jobId|alias|@capability|n>` stops one wrapper-owned shell job without touching the others
  - job references accept either stable ids like `bg-2`, session-local aliases, declared service capabilities prefixed with `@`, or the current 1-based sorted shell index
- service shells now have explicit readiness state when the job declared `readyPattern`:
  - `booting` while the process is running but the ready pattern has not been observed yet
  - `ready` once output matches the declared pattern
  - `untracked` when the job is a service shell but no readiness contract was declared
- service shells can also declare explicit attachment metadata:
  - `dependsOnCapabilities` for durable dependency intent from later background work to reusable service roles, so orchestration can preserve that relationship even before a specific shell or agent explicitly polls the result again
  - `capabilities` for role-based reuse, so later workers can target one service by what it provides instead of whichever concrete shell id owns it
  - `protocol` for the interaction family, such as `http`, `postgres`, `redis`, or another repo-specific label
  - `endpoint` for the canonical URL/socket/target that later work should use
  - `attachHint` for the operator or agent-facing next-step instruction, such as “send HTTP requests to /health” or “attach with psql”
  - `recipes` for structured interaction verbs with optional description/example text, so later work can ask “what can I do with this service?” without scraping raw logs
  - `parameters` on a recipe for reusable call-time arguments:
    - each parameter has a `name`
    - optional `description`
    - optional `default`
    - optional `required` flag
    - when no default is provided, parameters are treated as required unless explicitly marked otherwise
  - recipe `action` for typed invocation when the service contract is executable instead of purely descriptive:
    - `stdin` for reusable command phrases that should be sent back to the service shell process
    - `http` for plain `http://` endpoint requests derived from the declared service endpoint
      - optional request headers for auth or content negotiation
      - optional request body for POST/PUT-style recipes
      - optional expected status validation so the wrapper can fail fast when the service contract is violated
    - `tcp` for `tcp://host:port` or `host:port` endpoint probes derived from the declared service endpoint
      - optional outbound payload
      - optional newline appending for line-oriented protocols
      - optional expected substring validation
      - optional bounded read timeout so the wrapper can probe long-lived sockets safely
    - `redis` for RESP command execution against Redis-compatible endpoints
      - command arrays are serialized as RESP bulk-string arrays automatically
      - optional expected substring validation
      - optional bounded read timeout
      - responses are parsed into structured Redis reply types instead of left as raw socket bytes
    - descriptive-only recipes are still valid and remain attach-only instead of invokable
  - recipe actions now support placeholder substitution using `{{name}}` syntax driven by declared parameter values
    - operator path: `:ps run <job> <recipe> {"name":"value"}`
    - model path: `background_shell_invoke_recipe` with optional `args` object and `waitForReadyMs`
    - defaults are applied automatically, missing required args fail early, and unknown args are rejected instead of being silently ignored
  - executable network recipes (`http`, `tcp`, `redis`) now automatically wait for service readiness when the service declared `readyPattern`
    - explicit wait is also available through `:ps wait ...` and `background_shell_wait_ready`
    - `background_shell_invoke_recipe.waitForReadyMs` can lengthen or disable that auto-wait (`0` disables it)
  - capability conflicts are surfaced proactively in service listings, the dedicated capability index, and orchestration guidance, so the wrapper shows `@capability` ambiguity before later reuse fails at resolution time
  - service-shell listings can now also be filtered directly by health/conflict state (`ready`, `booting`, `untracked`, `conflicts`) instead of forcing the operator or model to scan the full reusable-service registry
  - the capability index also shows current consumers of each capability when running jobs declare `dependsOnCapabilities`, so provider and consumer sides of reusable service roles are visible in one place
  - the model-facing dynamic tool layer now includes capability inspection too, so orchestration can inspect one reusable service role directly without scraping the whole shell list
  - the model-facing dynamic tool layer now also includes filtered capability listing, so orchestration can ask for only missing, booting, ambiguous, or healthy service roles instead of scraping the full reusable-service registry
  - capability resolution is intentionally restricted to running service shells, so completed or terminated helpers do not keep satisfying `@capability` references after they are no longer reusable
- `/ps` also has in-session attachment naming now:
  - `:ps alias <jobId|n> <name>` assigns a stable alias to one local shell job
  - `:ps unalias <name>` removes that alias
  - aliases are session-local, operator-visible in `/ps`, and reusable anywhere the existing `jobId` reference surface is accepted, including poll, send, and terminate flows from both operator commands and dynamic tools
  - this gives long-lived service shells a first-class continuity handle without pretending they are repo-global or backend-native objects
- cleanup is now scoped along the same control boundary:
  - `:clean blockers` and `:ps clean blockers` terminate only wrapper-owned prerequisite shells
  - `:clean shells` terminates all wrapper-owned local shell jobs
  - `:clean services` terminates only wrapper-owned service shells
  - `:clean terminals` uses the backend `thread/backgroundTerminals/clean` API without touching local shell jobs
  - agent waits remain inspectable but are not directly terminable from the wrapper, so blocker cleanup is honest about targeting only controllable prerequisite shells
- `/status` now reports an orchestration breakdown with:
  - `main=1`
  - blocking and sidecar dependency-edge counts derived from the live orchestration graph
  - live wait count from in-progress `wait` collab calls
  - live sidecar-agent count from `spawnAgent` / `sendInput` / `resumeAgent`
  - live execution-prerequisite count from wrapper background shells marked `prerequisite`
  - live execution-sidecar count from wrapper background shells marked `observation`
  - live execution-service count from wrapper background shells marked `service`
  - live ready, booting, and untracked service-shell counts within that execution-service class
  - live conflicted-service count for reusable helpers that currently participate in capability collisions
  - live collab-agent task count from the current turn
  - cached agent-thread count from the latest `/agent` or `/multi-agents` listing
  - wrapper-owned background shell count
  - backend thread-terminal count
- the ready prompt now consumes the same orchestration state:
  - it reports when the main agent is blocked on prerequisite shells or agent waits
  - it distinguishes sidecars, reusable services, and server terminals instead of showing only a flat background-task count
  - reusable services can now surface as `booting`, `ready`, or `untracked` directly in that prompt suffix
  - reusable services can also surface as `conflicted` when capability reuse is ambiguous
  - it keeps `/ps` and `/clean` as the action hints for inspecting or stopping async work
- that orchestration state is now actionable from `/ps` itself, not just visible:
  - the full worker view remains the default
  - filtered `/ps` views let the operator jump directly to the class of worker they care about instead of scanning a mixed snapshot
  - `/ps guidance` now uses the same graph to emit a highest-priority next-step hint instead of only another raw worker listing
- `/status` runtime output also exposes a compact `background cls` line with the shell-intent, service-readiness, and terminal class counts, so the operator-facing summary does not require opening `/ps` just to tell whether async work is blocking, merely observational, or a service that is still booting
- that compact summary now also carries capability-dependency issue counts (`cap_deps_missing`, `cap_deps_booting`, `cap_deps_ambiguous`), so durable service dependency health is visible without opening the detailed capability registry
- that compact summary now also carries conflicted-service counts, so capability-collision health is visible without opening the detailed service registry
- `/status` overview/runtime output now also exposes a `next action` line derived from that same state, so the unified orchestration model drives operator guidance as well as raw counts and dependency edges
- the same orchestration graph is now available to the model-side dynamic tool layer too:
  - `orchestration_status` mirrors the compact orchestration summary plus next-action hint
  - `orchestration_list_workers` mirrors the `/ps` worker graph with optional filters such as `blockers`, `dependencies`, `agents`, `services`, `capabilities`, `terminals`, or `guidance`
  - `orchestration_list_dependencies` mirrors the focused dependency-edge view with optional filters such as `blocking`, `sidecars`, `missing`, `booting`, `ambiguous`, or `satisfied`, plus an optional `capability` selector for one reusable role
  - `background_shell_list_services` mirrors the focused reusable-service registry with optional filters such as `ready`, `booting`, `untracked`, or `conflicts`

That orchestration state now lives under one internal container rather than several unrelated top-level fields. The wrapper keeps backend-observed terminals, wrapper-owned background shell jobs, cached agent-thread summaries, and live collab-agent tasks inside one `OrchestrationState`, and `/multi-agents`, `/ps`, ready status, and transcript summaries all read from that same model.
`codexw` also derives an explicit dependency graph from that state: `main -> agent:*` edges for collab waits and sidecar agent work, plus attributed `thread|agent -> shell:*` edges for running wrapper-owned background shell jobs. Background-shell edge semantics now come from explicit job intent rather than heuristics, so `backgroundShell:prerequisite` edges are blocking while `backgroundShell:observation` and `backgroundShell:service` stay sidecar.

The next architectural step, if deeper orchestration is needed, is not basic state unification anymore. It is richer policy and scheduling on top of the unified state, for example:

- explicit task identities above raw live worker items
- durable dependency intent between future work and running shells/services:
  - partially implemented now through `dependsOnCapabilities` on background shell jobs
  - still open for richer task identities and dependency persistence above individual shell jobs
- reusable orchestration policies for when the main agent should wait, delegate, or attach

Two design choices matter here:

- State is in-memory and session-local. `codexw` does not depend on repo-local cache directories to operate.
- Resume startup reconstructs only the minimum recent conversation context needed for display and auto-continue state.

## Initialization Flow

Initialization is deliberately split into fast-path and ancillary work.

After `initialize` succeeds, `codexw` immediately:

- sends `initialized`
- sends `thread/start` or `thread/resume` first if a startup thread action is requested

Then it sends non-critical background lookups:

- `app/list`
- `skills/list`
- `account/read`
- `account/rateLimits/read`

This ordering matters because resume latency is user-visible, while catalog and account loading can complete afterward without blocking the first interactive view.

## Resume Flow

Resume is handled in two layers.

### Startup resume

Supported forms include:

- `codexw --resume <thread-id>`
- `codexw resume <thread-id>`
- `codexw resume`

Global flags such as `--cwd` may appear before or after the startup `resume` token. `codexw resume --cwd /repo` is normalized to the same startup mode as `codexw --cwd /repo resume`.

When startup resume is invoked without a thread id, `codexw` does not create a new thread. It loads the recent thread list for the resolved cwd, sorts it newest-first by `updatedAt`, and exposes a prompt-local picker that accepts `/resume <n>`, a raw thread id, or a bare numeric selection.

On successful `thread/resume`, `codexw`:

- resets thread-local state
- stores the resumed thread id
- renders a compact recent history preview
- optionally starts a turn if a prompt was supplied

When the inline client exits with an active thread id, it also prints a copy-pasteable `codexw --cwd <path> resume <thread-id>` hint so the session can be resumed directly. Idle `Ctrl-C` emits that full command immediately before exit as well, instead of relying only on the post-loop shutdown path.

### Interactive resume

Inside the client, `/resume` supports:

- `/resume <thread-id>` for explicit ids
- `/resume` to list recent threads in the current cwd
- `/resume <n>` to resume one of the cached numbered thread results

### Resume history optimization

Resume no longer flattens and clones the full thread history. Instead it:

- scans backward through turns
- extracts only the latest conversation items needed for preview
- seeds only the latest useful user message and latest assistant reply into state
- renders only the latest 10 conversation messages

That keeps long sessions responsive.

## Turn Lifecycle

`codexw` uses app-server turns as the unit of work.

- Idle user input becomes `turn/start`
- User input while running becomes `turn/steer`
- `Ctrl-C` while a turn is running becomes `turn/interrupt`

Pending turn state is tracked in `PendingRequest` and `AppState`.

The client also tracks local shell commands started via `command/exec`, which are treated similarly to turns for status and interrupt behavior.

## Auto-Continue Model

Auto-continue is explicit and cooperative.

- The assistant is expected to end with `AUTO_MODE_NEXT=continue` or `AUTO_MODE_NEXT=stop`.
- `prompt.rs` parses only the final non-empty line to detect an explicit stop marker.
- Missing marker defaults to continue.
- The synthesized continuation prompt explicitly invokes `$session-autopilot` when available, but also embeds the continuation policy text directly so hosts without that installed skill still behave correctly.
- The next prompt is synthesized from:
  - the stored session objective
  - the latest assistant response
  - a continuation policy that prioritizes explicit user requests, TODOs, concrete remaining tasks, and verification

The companion skill in `skills/session-autopilot/` provides the model-side policy for this behavior when available, while `codexw` provides the runtime-side turn detection and resubmission. The runtime prompt remains self-sufficient so portability does not depend on the skill being installed.

## Inline Editor and Prompt Model

`editor.rs` defines the editor surface and shared state, while `editor_buffer.rs` and `editor_history.rs` implement the editing/history behavior. Together they provide:

- insertion
- left and right navigation
- Home and End
- Backspace and Delete
- history navigation
- multiline drafting via `Ctrl-J`
- `Ctrl-A`, `Ctrl-E`, `Ctrl-U`, `Ctrl-W`
- `Esc` to clear draft
- `Ctrl-C` to clear draft when idle, or interrupt active work without discarding the draft

The editor and prompt renderer now operate on grapheme boundaries and display width rather than raw Unicode scalar counts. That makes cursor movement, backspace, delete, and prompt cursor placement behave correctly for CJK text, emoji, and combining characters.

The prompt stays scroll-native. Instead of owning a fixed alternate screen, `output.rs` redraws a single prompt line in place and also handles committed transcript/status/block writes into normal terminal history.

Long drafts now wrap to the current terminal width so redraw stays aligned with the actual editor buffer instead of compressing everything into a single preview row. Explicit multiline drafts now render as real visual prompt rows, so `Ctrl-J` moves the cursor to the next line instead of showing a synthetic newline marker inside a flattened preview. In that multiline mode, Home/End and `Ctrl-A`/`Ctrl-E` operate on the current line segment rather than the full buffer, and `Ctrl-U` clears back to the current line start.

## Input Construction

`input.rs` is the structured `UserInput` entrypoint and namespace root. It assembles the turn payload directly, while `input/input_build_items.rs` owns attachment/text/linked-mention item emission and `input/input_build_mentions.rs` owns catalog-derived mention emission. `input/input_decode_mention_links.rs` owns linked-tool mention parsing, `input/input_decode_mention_paths.rs` owns tool-path classification, `input/input_decode_mentions.rs` owns linked mention decoding, `input/input_decode_inline_mentions.rs` owns inline `@file` expansion, `input/input_decode_inline_paths.rs` owns file-path resolution and shell-safe rendering for inline mentions, `input/input_decode_inline_skills.rs` owns skill-path detection, `input/input_decode_tokens.rs` owns token/env-var scanning helpers, `input/input_resolve_tools.rs` owns tool-mention extraction, `input/input_resolve_catalog.rs` owns catalog-based mention selection, and `input/input_types.rs` owns the input-layer data types.

It supports:

- plain text input
- local image attachments
- remote image attachments
- linked mentions
- raw `$app`, `$skill`, and plugin-style mentions when resolvable
- inline `@path/to/file` expansion against the current cwd

The core function is `build_turn_input(...)`, which returns:

- `display_text` for user-facing state
- structured `items` for app-server submission

This keeps the visible prompt text and the actual submitted protocol payload aligned.

`dispatch_commands.rs` plus the submit handlers in `dispatch_submit_commands.rs` and `dispatch_submit_turns.rs` sit above that payload construction layer. They decide whether a line is:

- a built-in slash command
- a local `!command`
- a normal user turn submission

That keeps command workflows separate from lower-level input item construction.

## Output and Rendering

`output.rs` owns terminal writes, prompt redraw ordering, and committed stream output directly.

Important properties:

- one ordered output path for transcript and prompt control
- explicit CRLF normalization for committed output
- transient status is width-bounded to a single terminal row so long path-heavy updates do not wrap and smear across redraws above the wrapped prompt
- prompt hide and redraw before emitted transcript blocks
- no mixed stdout/stderr interleaving for user-visible UI

`render_prompt.rs` owns prompt fitting and committed prompt rendering. `output.rs` now also owns block-level ANSI assembly, while `render_block_common.rs` owns block classification/title/status styling, `render_block_markdown.rs` owns markdown block assembly, `render_markdown_code.rs` owns syntax-highlighted code rendering, `render_markdown_inline.rs` owns inline markdown parsing/tinting, `render_block_structured.rs` owns diff/command/plain block rendering, and `render_ansi.rs` owns ANSI serialization for `ratatui` text primitives such as `Text`, `Line`, and `Span`.

It renders:

- assistant blocks
- dimmed reasoning summaries
- command blocks
- shell output
- colored diffs
- a transient status line above the prompt
- the inline prompt label

The design goal is richer terminal output without switching to an alternate-screen viewport.

To reduce transcript duplication, the client now prefers the transient status line over committed `[status] ...` chatter in scrollback, and it avoids emitting redundant "started" transcript blocks for commands and file changes that already produce a completed result block.
For long raw textual output, the renderer now uses a default 80-line threshold and collapses oversized command/read/search-style results to the first 20 lines, `...`, and the last 20 lines. Structured file-change artifacts are excluded from that rule so diffs and update payloads remain fully reviewable, and `--verbose`/`--verbose-events` bypass the abbreviation entirely.

## Human-Facing Features

Current user-facing capabilities include:

- start, resume, fork, compact, rename, review, clean, and interrupt thread workflows
- scroll-native inline prompt and steer input
- local `!command` execution through app-server `command/exec`
- attachment queueing for local and remote images
- `@file` inline path expansion
- slash-command help and completion
- fuzzy and ambiguous slash suggestions
- numbered file mention and thread resume pickers in scrollback
- rich `/status` output, including turn counts, active request time, token usage, account state, and rate limits
- real collaboration-mode controls through `/plan` and `/collab`, backed by `collaborationMode/list` plus `turn/start.collaborationMode`
- backend-backed `/experimental` listing through `experimentalFeature/list`
- backend-backed text realtime controls through `/realtime start|send|stop`, backed by `thread/realtime/*`
- startup-warmed model metadata plus in-session numbered pickers for `/model` and `/personality`, backed by `model/list`, `turn/start` overrides, and persisted `config.toml` defaults
- in-session numbered permissions presets for `/approvals` and `/permissions`, backed by `turn/start.approvalPolicy`, `turn/start.sandboxPolicy`, and matching thread/local-command overrides
- local runtime toggles for `/fast` service-tier selection and `/theme` syntax-highlighting theme selection, both aligned with persisted Codex config state
- native-style `/init` behavior that skips when `AGENTS.md` already exists and otherwise submits the upstream repository-guidelines prompt as a normal turn
- backend-backed `/agent` and `/multi-agents` switching via filtered `thread/list` results for spawned subagent threads, with `:resume <n>` as the attach path
- native-style `/rollout` behavior that reports the current thread rollout path when available from app-server thread state
- client dynamic tools on new threads via `thread/start.dynamicTools`, covering both read-only workspace inspection (`workspace_list_dir`, `workspace_stat_path`, `workspace_read_file`, `workspace_find_files`, `workspace_search_text`) and wrapper-owned background shell control (`background_shell_start`, `background_shell_poll`, `background_shell_send`, `background_shell_attach`, `background_shell_wait_ready`, `background_shell_invoke_recipe`, `background_shell_list`, `background_shell_terminate`), including service readiness contracts via `readyPattern`, explicit readiness waits, and service attachment metadata via `protocol` / `endpoint` / `attachHint` / `recipes`
- richer stored thread history via `persistExtendedHistory: true` on `thread/start`, `thread/resume`, and `thread/fork`
- backend-backed Windows sandbox setup through `windowsSandbox/setupStart`, with successful completion persisted into Codex config
- live background-terminal tracking from command item lifecycle and terminal-interaction notifications, plus wrapper-owned local background shell jobs for same-turn async shell work, with scoped cleanup for local shell classes and backend-tracked terminals
- `/diff`, `/apps`, `/skills`, `/models`, `/mcp`, `/threads`, `/feedback`, `/logout`, and related backend-backed commands
- automatic approval handling for supported approval request shapes
- auto-continue between turns

`codexw` no longer leaves any of the user-facing slash-command side effects in a generic placeholder state. The Windows-only `:sandbox-add-read-dir` path is now handled client-side like upstream: it validates the requested absolute directory and refreshes sandbox read grants locally rather than going through app-server. The main remaining gaps are architectural or UX-level instead of command dispatch parity. `codexw` still does not implement the upstream audio UX; it surfaces realtime state and text transport only. Upstream app-server also still does not expose a public client request that can write to or poll model-owned `item/commandExecution` sessions directly, so same-turn async shell work is implemented through wrapper-owned dynamic tools instead of reusing the backend's internal unified-exec handles.

## Approval and Automation Posture

`codexw` defaults to a fully automated posture.

That includes:

- approval policy `never`
- danger-full-access sandbox posture
- automatic handling for command approvals, preferring allow-amendment decisions when the backend offers them
- automatic handling for file-change approvals
- best-effort answers for backend user-input requests, preferring permissive option labels plus schema-driven MCP form replies

This makes `codexw` suitable for unattended continuation workflows, while still allowing interactive steer input.

## Error Handling and Robustness

Important robustness choices include:

- unsupported incoming server requests receive explicit JSON-RPC method-not-implemented errors instead of silently hanging
- empty terminal-interaction noise is suppressed from the normal transcript
- prompt input is hidden or disabled during thread switches and local-command states where invisible buffered input would be dangerous
- resume preview work is bounded instead of replaying all internal history
- prompt redraw and transcript output share one output channel

## Current Limits

The biggest known limits are architectural, not accidental.

- `codexw` is not the native upstream Codex TUI.
- It depends only on app-server surfaces, so popup-heavy native workflows cannot always be reproduced exactly.
- It cannot directly control model-owned `item/commandExecution` sessions through app-server today; the background-shell workflow is a wrapper-side workaround built on dynamic tools.
- A small number of platform-specific commands can only explain their limitation because app-server does not expose the needed internal state, notably the Windows additional read-root grant workflow.
- Rendering is richer than plain logs, but it is still terminal-scrollback based rather than a full alternate-screen widget tree.

## File Map

- `wrapper/src/main.rs`
  Thin entrypoint, module wiring, CLI flag definitions, and `main()`.
- `wrapper/src/main_test_approvals.rs`
  Approval-decision and auto-approval regression tests.
- `wrapper/src/main_test_catalog.rs`
  Resume-preview and rate-limit regression tests.
- `wrapper/src/main_test_catalog_render.rs`
  Catalog rendering regression tests for apps, experimental features, and models.
- `wrapper/src/main_test_catalog_threads.rs`
  Thread-list, file-search extraction, and fuzzy file-search regression tests.
- `wrapper/src/main_test_commands.rs`
  Slash-command completion, ordering, help-line, and quoting regression tests.
- `wrapper/src/main_test_runtime_commands.rs`
  Builtin-command detection and slash-completion regression tests.
- `wrapper/src/main_test_runtime_prompt.rs`
  File-completion and prompt visibility/input gating regression tests.
- `wrapper/src/main_test_runtime_cli.rs`
  CLI normalization, feedback parsing, and quoting regression tests.
- `wrapper/src/input_test_mentions.rs`
  Linked-mention decoding regression tests.
- `wrapper/src/input_test_build_items.rs`
  Structured turn-input item construction and inline file-mention expansion regression tests.
- `wrapper/src/input_test_build_mentions.rs`
  Catalog-driven mention construction regression tests.
- `wrapper/src/main_test_session_render.rs`
  Tool-user-input, reasoning, terminal-interaction, and realtime-item rendering regression tests.
- `wrapper/src/main_test_session_collaboration.rs`
  Collaboration preset extraction, rendering, and prompt-status regression tests.
- `wrapper/src/main_test_session_model_catalog.rs`
  Model catalog extraction regression tests.
- `wrapper/src/main_test_session_personality_status.rs`
  Personality rendering, prompt-status, and status-snapshot regression tests.
- `wrapper/src/main_test_session_status.rs`
  Thread-status, prompt-status, realtime-status snapshot, and ready-state prompt regression tests.
- `wrapper/src/app.rs`
  Top-level runtime loop and backend wiring.
- `wrapper/src/app_input_editing.rs`
  Editing/navigation key routing for prompt-accepting states.
- `wrapper/src/app_input_controls.rs`
  Enter/Esc/Ctrl-C control routing across submit, interrupt, and exit paths.
- `wrapper/src/policy.rs`
  Approval/sandbox/reasoning policy helpers and approval decision preferences.
- `wrapper/src/runtime_process.rs`
  Backend process startup and child shutdown lifecycle.
- `wrapper/src/runtime_event_sources.rs`
  `AppEvent` sources for server stdout, keyboard input, periodic ticks, and stream closure events.
- `wrapper/src/runtime_keys.rs`
  Raw terminal key normalization into `InputKey`.
- `wrapper/src/events.rs`
  Inbound JSON-RPC routing plus server-request handling and approval helpers.
- `wrapper/src/notification_realtime.rs`
  Realtime, account, app-list, and thread-status notification handling.
- `wrapper/src/notification_turn_started.rs`
  Turn start state-reset and active-turn bookkeeping.
- `wrapper/src/notification_turn_completed.rs`
  Turn completion status handling, ready-state reporting, and auto-continue turn chaining.
- `wrapper/src/notification_item_buffers.rs`
  Turn-item delta buffering, plan/diff streaming, terminal-interaction logging, and task-complete event capture.
- `wrapper/src/notification_item_status.rs`
  Item-start status updates, reroute reporting, approval-resolution reporting, and turn-error reporting.
- `wrapper/src/notification_item_completion.rs`
  Turn-item completion rendering for assistant text, commands, file changes, reasoning, and tool items.
- `wrapper/src/catalog.rs`
  App and skill catalog parsing for app-server payloads.
- `wrapper/src/history_render.rs`
- `wrapper/src/history_state.rs`
  Resume-preview extraction, resumed objective/reply seeding, and resumed conversation rendering.
- `wrapper/src/catalog_connector_views.rs`
  Apps and skills rendering helpers.
- `wrapper/src/catalog_feature_views.rs`
  Experimental-feature rendering helpers.
- `wrapper/src/catalog_thread_list.rs`
  Thread-list rendering and thread-id extraction helpers.
- `wrapper/src/catalog_file_search.rs`
  File-search rendering and extracted-path helpers for `/mention`.
- `wrapper/src/catalog_backend_views.rs`
  Models and MCP server rendering helpers.
- `wrapper/src/status_value.rs`
  Generic JSON value summarization helpers shared by status, transcript, and server-request rendering.
- `wrapper/src/status_config.rs`
  Permission and config rendering plus sandbox-policy summarization.
- `wrapper/src/status_account.rs`
  Account summary rendering.
- `wrapper/src/status_rate_windows.rs`
  Rate-limit window rendering and local reset-time formatting.
- `wrapper/src/status_rate_credits.rs`
  Credit-balance and unlimited-credit rendering helpers.
- `wrapper/src/status_token_usage.rs`
  Token-usage summary rendering helpers.
- `wrapper/src/transcript_completion_render.rs`
  Command/file-change completion and pending-attachment rendering helpers.
- `wrapper/src/transcript_plan_render.rs`
  Plan, reasoning, and structured tool-user-input response helpers.
- `wrapper/src/session_prompt_status_active.rs`
  Prompt-status rendering for active command, turn, and realtime states plus shared spinner/elapsed helpers.
- `wrapper/src/session_prompt_status_ready.rs`
  Prompt-status rendering for idle ready state, including collaboration/personality summaries.
- `wrapper/src/session_realtime_status.rs`
  Realtime session status rendering for `/realtime` state, prompt, elapsed time, and last error.
- `wrapper/src/session_realtime_item.rs`
  Realtime item rendering plus text/transcript extraction from streamed realtime payloads.
- `wrapper/src/session_snapshot_overview.rs`
  Core `/status` overview lines for cwd, thread, sandbox, model, collaboration, and attachment state.
- `wrapper/src/session_snapshot_runtime.rs`
  Runtime `/status` lines for realtime state, account, activity timing, rate limits, token usage, and last reply summaries.
- `wrapper/src/requests.rs`
  Outbound request namespace root plus shared `send_json(...)` transport helper.
- `wrapper/src/requests/request_types.rs`
  `PendingRequest` variants used to track in-flight JSON-RPC work.
- `wrapper/src/requests/bootstrap_init.rs`
  Initialize request and initialized notification builders.
- `wrapper/src/requests/bootstrap_account.rs`
  Account, logout, feedback-upload, and rate-limit request builders.
- `wrapper/src/requests/bootstrap_catalog_core.rs`
  Model and collaboration-mode bootstrap request builders.
- `wrapper/src/requests/bootstrap_catalog_lists.rs`
  App, skill, config, MCP-server, and experimental-feature bootstrap request builders.
- `wrapper/src/requests/bootstrap_search.rs`
  Thread-list and fuzzy file-search request builders.
- `wrapper/src/requests/thread_switch_common.rs`
  Shared thread-switch request-id, pending-state helpers, and thread start/resume/fork request builders.
- `wrapper/src/requests/thread_maintenance.rs`
  Thread compact, rename, and background-terminal cleanup request builders.
- `wrapper/src/requests/thread_realtime.rs`
  Thread realtime start, append-text, and stop request builders.
- `wrapper/src/requests/thread_review.rs`
  Inline review-start request builder.
- `wrapper/src/requests/turn_start.rs`
  Turn start and steer request builders.
- `wrapper/src/requests/turn_control.rs`
  Turn interrupt request builder.
- `wrapper/src/requests/command_requests.rs`
  Local command exec and terminate request builders.
- `wrapper/src/rpc.rs`
  JSON-RPC wire types and line parsing.
- `wrapper/src/response_bootstrap_catalog_state.rs`
  Bootstrap response handling that mutates cached apps, skills, account, rate-limit, model, and collaboration state.
- `wrapper/src/response_bootstrap_catalog_views.rs`
  Bootstrap response rendering for config, experimental flags, MCP servers, thread lists, and file-search results.
- `wrapper/src/response_thread_maintenance.rs`
  Compact, rename, and background-terminal cleanup success reporting.
- `wrapper/src/response_thread_runtime.rs`
  Runtime response handling for realtime, review, turn, interrupt, and local-command flows.
- `wrapper/src/response_thread_loaded.rs`
  Shared thread-load reset, successful thread start/resume/fork handling, resumed-history rendering, and initial prompt handoff helpers.
- `wrapper/src/response_error_session.rs`
  Session/bootstrap/thread-switch error handling for account, models, collaboration, logout, feedback, and thread changes.
- `wrapper/src/response_error_runtime.rs`
  Runtime error handling for realtime and local-command failures.
- `wrapper/src/commands_entry_session_catalog.rs`
  Session catalog, status, and personality builtin command entries.
- `wrapper/src/commands_entry_session_modes.rs`
  Session collaboration, auto, and attachment builtin command entries.
- `wrapper/src/commands_entry_thread.rs`
  Thread/workspace-oriented builtin command entries.
- `wrapper/src/commands_entry_runtime.rs`
  Runtime/meta/control builtin command entries.
- `wrapper/src/commands_catalog.rs`
  Shared builtin command catalog, command descriptions, help-line generation, and stable command-name ordering.
- `wrapper/src/commands_completion_apply.rs`
  Slash-command completion application, prefix expansion, and fuzzy-match selection.
- `wrapper/src/commands_completion_render.rs`
  Slash-completion candidate rendering and shell-style quoting helper.
- `wrapper/src/commands_match.rs`
  Slash-command cursor parsing, fuzzy scoring, and longest-common-prefix helpers.
- `wrapper/src/input.rs`
  Structured `UserInput` entrypoint and input namespace root.
- `wrapper/src/input/input_types.rs`
  Input-layer data types such as parsed payloads and catalog entries.
- `wrapper/src/input/input_decode_mentions.rs`
  Linked-tool mention decoding plus linked mention parsing helpers.
- `wrapper/src/input/input_decode_mention_links.rs`
  Linked-tool mention parsing helpers.
- `wrapper/src/input/input_decode_mention_paths.rs`
  Tool-path classification helpers for linked mentions.
- `wrapper/src/input/input_decode_inline_mentions.rs`
  Inline `@file` expansion helpers layered on token scanning and file-path rendering.
- `wrapper/src/input/input_decode_inline_paths.rs`
  Filesystem-backed path resolution and shell-safe rendering helpers for inline `@file` mentions.
- `wrapper/src/input/input_decode_inline_skills.rs`
  Skill-path detection helpers for linked and raw skill mentions.
- `wrapper/src/input/input_decode_tokens.rs`
  Prefixed-token collection plus low-level token/env-var classification helpers.
- `wrapper/src/input/input_resolve_tools.rs`
  Tool-mention extraction from text and linked mentions.
- `wrapper/src/input/input_resolve_catalog.rs`
  Catalog-driven app/plugin/skill mention selection.
- `wrapper/src/input/input_build_items.rs`
  Attachment, text, and linked-mention payload item assembly.
- `wrapper/src/input/input_build_mentions.rs`
  Catalog-driven app/plugin/skill mention payload assembly.
- `wrapper/src/dispatch_submit_commands.rs`
  Prefixed slash/local-command submission routing.
- `wrapper/src/dispatch_submit_turns.rs`
  Structured turn submission and steer handoff.
- `wrapper/src/dispatch_commands.rs`
  Slash-command dispatcher that routes directly to concrete thread/session handlers.
- `wrapper/src/dispatch_command_thread_common.rs`
  Shared thread command guards and cached thread-reference resolution helpers.
- `wrapper/src/dispatch_command_thread_navigation_session.rs`
  New/resume/threads command workflows and cached-thread resolution handoff.
- `wrapper/src/dispatch_command_thread_navigation_identity.rs`
  Fork and rename command workflows for the current thread.
- `wrapper/src/dispatch_command_thread_review.rs`
  Review request workflow for current changes or custom review instructions.
- `wrapper/src/dispatch_command_thread_control.rs`
  Thread compaction, background-terminal cleanup, and interrupt workflows.
- `wrapper/src/dispatch_command_thread_view.rs`
  Mention insertion/search, diff display, and copy slash-command workflows.
- `wrapper/src/dispatch_command_thread_draft.rs`
  Attachment-queue draft slash-command workflows.
- `wrapper/src/dispatch_command_session_catalog_lists.rs`
  Session catalog-list commands such as apps, skills, MCP, and experimental flags.
- `wrapper/src/dispatch_command_session_catalog_models.rs`
  Session model-display commands such as models, model selection, and personality workflows.
- `wrapper/src/dispatch_command_session_status.rs`
  Session display/status commands such as attachments, permissions, `/status`, config display, and realtime status.
- `wrapper/src/dispatch_command_session_collab.rs`
  Collaboration and plan-mode session command workflows.
- `wrapper/src/dispatch_command_session_ps.rs`
  Background-terminal cleanup and `/ps` status messaging.
- `wrapper/src/dispatch_command_session_meta.rs`
  Session meta workflows such as feedback, logout, and recognized-but-unported native popup paths.
- `wrapper/src/dispatch_command_utils.rs`
  Shared slash-command helpers such as built-in detection, feedback parsing, prompt joining, and clipboard handling.
- `wrapper/src/prompt_state.rs`
  Prompt visibility/input gating, prompt redraw orchestration, and live status-line coordination.
- `wrapper/src/prompt_file_completions_token.rs`
  `@file` token boundary detection for prompt completion.
- `wrapper/src/prompt_file_completions_search.rs`
  Filesystem-backed prompt completion lookup for `@file` tokens.
- `wrapper/src/model_catalog.rs`
  Model catalog extraction and effective-model selection.
- `wrapper/src/model_personality_view.rs`
  Personality labeling, active-personality summaries, and `/personality` option rendering.
- `wrapper/src/model_personality_actions.rs`
  Personality validation, selection application, and load-model action handling.
- `wrapper/src/collaboration_preset.rs`
  Collaboration preset extraction, summaries, and selector matching.
- `wrapper/src/collaboration_view.rs`
  Active collaboration-mode summaries, labels, and collaboration preset list rendering.
- `wrapper/src/collaboration_apply.rs`
  Collaboration-mode toggle and selection application logic.
- `wrapper/src/editor.rs`
  Inline line editor and editing semantics.
- `wrapper/src/editor_graphemes.rs`
  Grapheme-aware byte-index, counting, and whitespace helpers used by the editor.
- `wrapper/src/state.rs`
  `AppState`, `ProcessOutputBuffer`, request-id generation, constructor/reset helpers, and attachment transfer behavior.
- `wrapper/src/state_helpers.rs`
  Shared state/text/buffer helper functions such as `thread_id`, `get_string`, delta buffering, status dedupe, and path canonicalization.
- `wrapper/src/editor_tests.rs`
  Crate-level regression tests for the inline editor.
- `wrapper/src/render_tests.rs`
  Crate-level regression tests for ANSI block rendering and prompt layout behavior.
- `wrapper/src/output.rs`
  Prompt redraw, committed output, prompt visibility, output ordering.
- `wrapper/src/render_prompt.rs`
  Prompt fitting, grapheme-aware cursor positioning, and committed prompt rendering.
- `wrapper/src/render_block_common.rs`
  Block classification, title styling, and status-line formatting helpers.
- `wrapper/src/render_block_markdown.rs`
  Markdown-like block assembly and thinking tinting.
- `wrapper/src/render_markdown_code.rs`
  Syntax-highlighted fenced-code and single-line code rendering.
- `wrapper/src/render_markdown_inline.rs`
  Inline markdown parsing, link rendering, and span tinting helpers.
- `wrapper/src/render_block_structured.rs`
  Diff, command, and plain-text block rendering.
- `wrapper/src/render_ansi.rs`
  ANSI serialization helpers for `ratatui` text structures and styles.
- `wrapper/src/prompt.rs`
  Auto-continue prompt synthesis and stop-marker parsing.
- `skills/session-autopilot/`
  Companion cooperative skill for end-of-turn continuation policy.

## Practical Summary

`codexw` is best understood as a thin but capable interactive client around `codex app-server`:

- upstream Codex remains the execution engine
- `codexw` owns interaction, observability, and continuation policy runtime
- the bundled `session-autopilot` skill owns model-side continuation guidance

That separation is the central design decision of the project.
