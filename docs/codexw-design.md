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
   `runtime.rs`, `runtime_process.rs`, and `runtime_input.rs` start `codex app-server`, wire stdio, forward key environment such as proxy variables, own raw-mode lifecycle, and manage stdin/stdout/tick event sources. `runtime.rs` is the thin facade over the split process/input helpers.

2. JSON-RPC transport
   `rpc.rs` defines the wire-level request, response, notification, and request-id types, plus JSON parsing for inbound lines.

3. Outbound request construction
   `requests.rs`, `requests/request_types.rs`, `requests/bootstrap_init.rs`, `requests/bootstrap_account.rs`, `requests/bootstrap_catalog.rs`, `requests/bootstrap_catalog_core.rs`, `requests/bootstrap_catalog_lists.rs`, `requests/bootstrap_search.rs`, `requests/thread_switch.rs`, `requests/thread_maintenance.rs`, `requests/thread_activity.rs`, `requests/turn_requests.rs`, and `requests/command_requests.rs` own JSON-RPC request building and pending-request bookkeeping for initialize, account/catalog bootstrap, thread lifecycle, turn, command, review, and realtime actions. `bootstrap_catalog.rs` and `thread_lifecycle.rs` are now thin compatibility facades over the split concrete helpers, and production code imports the concrete request surface through `requests.rs`.

4. Inbound event handling
   `events.rs`, `event_requests.rs`, `responses.rs`, `response_success.rs`, `response_bootstrap.rs`, `response_bootstrap_init.rs`, `response_bootstrap_catalog.rs`, `response_threads.rs`, `response_thread_switch.rs`, `response_thread_activity.rs`, `response_error.rs`, `notifications.rs`, `notification_realtime.rs`, `notification_turn_lifecycle.rs`, `notification_turn_items.rs`, `notification_item_updates.rs`, `notification_item_completion.rs`, and `notification_turns.rs` own inbound JSON-RPC routing, server-request handling, response handling, notification handling, approval-request handling, realtime events, turn/item events, delta/status buffering, item-completion rendering, and response success/error paths. `events.rs`, `responses.rs`, `notifications.rs`, `response_success.rs`, `notification_turns.rs`, and `response_threads.rs` are compatibility/router facades over the split inbound handlers.

5. Catalog parsing
   `catalog.rs` owns app and skill catalog parsing from app-server payloads.

6. Shared state and text/buffer helpers
   `state.rs`, `state_core.rs`, and `state_helpers.rs` own `AppState`, process-output buffering, attachment queues, request bookkeeping, and shared utility helpers such as response-path string extraction, summarized status text, and streamed item/process delta buffering. `state.rs` is the facade over the split core/helper modules.

7. Runtime policy
   `policy.rs` owns approval policy, sandbox policy, reasoning-summary policy, shell selection, and approval-decision preference logic shared by requests, status rendering, and approval handling.

8. Session and turn orchestration
   `model_session.rs`, `model_catalog.rs`, `model_personality.rs`, `collaboration.rs`, `collaboration_preset.rs`, `collaboration_actions.rs`, `session_prompt_status.rs`, `session_realtime.rs`, `session_snapshot.rs`, and `session_status.rs` own model metadata, personality selection, collaboration mode handling, prompt/realtime status rendering, and status snapshot generation. `model_session.rs`, `collaboration.rs`, and `session_status.rs` remain the small facades over the split helpers.

9. App runtime loop
   `app.rs`, `app_input.rs`, `app_input_editor.rs`, and `app_input_interrupt.rs` own process wiring, the main event loop, keyboard-event dispatch, editor-key actions, and interrupt behavior for the live interactive session. `app.rs` holds the top-level loop, `app_input.rs` is the input facade, and the smaller helper modules own editor and interrupt behavior.

10. Resume and history rendering
   `history.rs`, `history_render.rs`, and `history_state.rs` own resumed-thread state seeding, compact conversation-history extraction, and resumed history rendering. `history.rs` is the thin facade over the split render/state helpers.

11. View and transcript rendering helpers
   `catalog_views.rs`, `catalog_lists.rs`, `catalog_app_views.rs`, `catalog_backend_views.rs`, `catalog_threads.rs`, `status_views.rs`, `status_config.rs`, `status_account.rs`, `status_limits.rs`, `status_rate_limits.rs`, `status_token_usage.rs`, `transcript_views.rs`, `transcript_render.rs`, `transcript_completion_render.rs`, `transcript_plan_render.rs`, `transcript_summary.rs`, `transcript_approval_summary.rs`, `transcript_item_summary.rs`, and `transcript_status_summary.rs` own app-server-facing display helpers for catalogs, status summaries, thread listings, token/rate-limit rendering, item completion blocks, and approval/request summaries. `catalog_views.rs`, `catalog_lists.rs`, `status_views.rs`, `status_limits.rs`, `transcript_views.rs`, `transcript_render.rs`, and `transcript_summary.rs` remain narrow compatibility facades over the split helpers.

12. Human input handling
   `editor.rs`, `editor_buffer.rs`, `editor_history.rs`, `editor_graphemes.rs`, `editor_tests.rs`, `input.rs`, `input/input_types.rs`, `input/input_decode.rs`, `input/input_decode_mentions.rs`, `input/input_decode_inline.rs`, `input/input_decode_inline_mentions.rs`, `input/input_decode_tokens.rs`, `input/input_resolve.rs`, `input/input_resolve_tools.rs`, `input/input_resolve_catalog.rs`, `input/input_build.rs`, `dispatch.rs`, `dispatch_submit.rs`, `dispatch_commands.rs`, `dispatch_command_thread.rs`, `dispatch_command_thread_flow.rs`, `dispatch_command_thread_navigation.rs`, `dispatch_command_thread_actions.rs`, `dispatch_command_thread_workspace.rs`, `dispatch_command_session.rs`, `dispatch_command_session_info.rs`, `dispatch_command_session_control.rs`, `dispatch_command_session_modes.rs`, `dispatch_command_session_meta.rs`, `dispatch_command_utils.rs`, `prompt_state.rs`, `prompt_completion.rs`, `prompt_file_completions.rs`, and `prompting.rs` implement the inline editor, editor regression coverage, grapheme-aware cursor helpers, command dispatch, slash/file completion, linked-mention decoding, inline-file/token decoding, attachment handling, catalog-driven mention resolution, prompt visibility/redraw, and structured app-server user input construction. `editor.rs`, `input.rs`, `input/input_decode.rs`, `input/input_resolve.rs`, `dispatch.rs`, `dispatch_commands.rs`, `dispatch_command_thread.rs`, `dispatch_command_session.rs`, and `prompting.rs` are compatibility facades over those splits.

13. Human output handling
   `output.rs`, `output_prompt.rs`, `output_stream.rs`, `render.rs`, `render_prompt.rs`, `render_blocks.rs`, `render_block_common.rs`, `render_block_markdown.rs`, `render_markdown_code.rs`, `render_markdown_inline.rs`, `render_block_structured.rs`, and `render_ansi.rs` convert app-server events into readable terminal output with markdown-like styling, colored diffs, command blocks, status lines, and a single-line prompt redraw path. `output.rs`, `render.rs`, and `render_blocks.rs` are compatibility facades over that split.

Session feature helpers are split across `model_catalog.rs`, `model_personality.rs`, `collaboration_preset.rs`, `collaboration_actions.rs`, `session_prompt_status.rs`, `session_realtime.rs`, and `session_snapshot.rs`, with `model_session.rs`, `collaboration.rs`, and `session_status.rs` kept as thin facades.
Runtime policy helpers live in `policy.rs`: approval, sandbox, reasoning-summary, shell-program, and approval-choice logic.
App loop helpers are split across `app.rs`, `app_input.rs`, `app_input_editor.rs`, and `app_input_interrupt.rs`: `app.rs` owns backend/session startup and the top-level runtime loop, `app_input.rs` is the input facade, `app_input_editor.rs` owns editor-key behavior and submit handling, and `app_input_interrupt.rs` owns interrupt and exit behavior.
Resume-preview helpers live across `history_render.rs` and `history_state.rs`, with `history.rs` kept as the thin facade for recent conversation extraction, resumed objective/last-reply seeding, and resumed transcript rendering.
Catalog display helpers are split across `catalog_app_views.rs`, `catalog_backend_views.rs`, and `catalog_threads.rs`, with `catalog_lists.rs` and `catalog_views.rs` kept as thin facades over app/skill/model/MCP display plus thread/search rendering.
Status display helpers are split across `status_config.rs`, `status_account.rs`, `status_rate_limits.rs`, and `status_token_usage.rs`, with `status_views.rs` and `status_limits.rs` kept as thin facades plus the generic value summarizer.
Transcript display helpers are split across `transcript_completion_render.rs`, `transcript_plan_render.rs`, `transcript_approval_summary.rs`, `transcript_item_summary.rs`, and `transcript_status_summary.rs`, with `transcript_render.rs`, `transcript_summary.rs`, and `transcript_views.rs` kept as thin compatibility facades over item completion blocks, plan/reasoning rendering, approval/request summaries, and thread-status summarization.
Runtime helpers live across `runtime_process.rs`, `runtime_input.rs`, `runtime_event_sources.rs`, and `runtime_keys.rs`, with `runtime.rs` kept as the thin facade over backend process startup, raw terminal mode, key mapping, and event-source threads.
Catalog helpers live in `catalog.rs`: app and skill list extraction for the current workspace.
Shared state helpers are split across `state_core.rs` and `state_helpers.rs`, with `state.rs` kept as the thin facade over `AppState`, buffer/state types, and common text/path helper functions used across modules.
Command catalog helpers are split across `commands_entries.rs` and `commands_catalog.rs`: entry data now lives in `commands_entries.rs`, while `commands_catalog.rs` keeps the public entrypoint and stable command-name ordering.
Command metadata helpers live in `commands_metadata.rs`: command descriptions and help-line generation over the shared command catalog.
Command completion helpers live in `commands_completion.rs` and `commands_match.rs`: slash completion rendering stays in the facade while cursor parsing, fuzzy scoring, and prefix logic live in the extracted matcher module.
Command-dispatch helpers are split across `dispatch_submit.rs`, `dispatch_command_thread_navigation.rs`, `dispatch_command_thread_actions.rs`, `dispatch_command_thread_workspace.rs`, `dispatch_command_session_info.rs`, `dispatch_command_session_modes.rs`, `dispatch_command_session_meta.rs`, and `dispatch_command_utils.rs`, with `dispatch.rs`, `dispatch_commands.rs`, `dispatch_command_thread.rs`, `dispatch_command_thread_flow.rs`, `dispatch_command_session.rs`, and `dispatch_command_session_control.rs` kept as thin compatibility facades for imports and tests.
Input helpers are split across `input/input_types.rs`, `input/input_decode_mentions.rs`, `input/input_decode_inline.rs`, `input/input_decode_inline_mentions.rs`, `input/input_decode_tokens.rs`, `input/input_decode.rs`, `input/input_resolve.rs`, `input/input_resolve_tools.rs`, `input/input_resolve_catalog.rs`, and `input/input_build.rs`, with `input.rs`, `input/input_decode.rs`, and `input/input_resolve.rs` kept as thin compatibility facades for imports and `input_tests.rs` holding the crate-level regression suite.
Prompt helpers live across `prompt_state.rs`, `prompt_completion.rs`, and `prompt_file_completions.rs`, with `prompting.rs` kept as the thin facade over prompt visibility/input gating, prompt redraw, slash completion, and `@file` completion.
Response helpers are split across `response_success.rs`, `response_error.rs`, `response_bootstrap_init.rs`, `response_bootstrap_catalog.rs`, `response_thread_switch.rs`, and `response_thread_activity.rs`, with `responses.rs`, `response_success.rs`, `response_bootstrap.rs`, and `response_threads.rs` kept as thin compatibility facades for JSON-RPC success/error handling of pending outbound requests.
Notification helpers are split across `notification_realtime.rs`, `notification_turn_lifecycle.rs`, `notification_turn_items.rs`, `notification_item_updates.rs`, and `notification_item_completion.rs`, with `notifications.rs` and `notification_turns.rs` kept as thin compatibility facades over realtime, turn, item, and status notifications plus auto-continue turn chaining.

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

On successful `thread/resume`, `codexw`:

- resets thread-local state
- stores the resumed thread id
- renders a compact recent history preview
- optionally starts a turn if a prompt was supplied

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

`editor.rs` implements the editor facade over `editor_buffer.rs` and `editor_history.rs`, and together they provide:

- insertion
- left and right navigation
- Home and End
- Backspace and Delete
- history navigation
- multiline drafting via `Ctrl-J`
- `Ctrl-A`, `Ctrl-E`, `Ctrl-U`, `Ctrl-W`
- `Esc` to clear draft
- `Ctrl-C` to clear draft or propagate interrupt when empty

The editor and prompt renderer now operate on grapheme boundaries and display width rather than raw Unicode scalar counts. That makes cursor movement, backspace, delete, and prompt cursor placement behave correctly for CJK text, emoji, and combining characters.

The prompt stays scroll-native. Instead of owning a fixed alternate screen, `output.rs` plus `output_prompt.rs` redraw a single prompt line in place, while `output_stream.rs` handles committed transcript/status/block writes into normal terminal history.

Long drafts are visually elided to the current terminal width so redraw does not wrap and corrupt the transcript.

## Input Construction

`input.rs` is the compatibility surface for the structured `UserInput` builder split. `input/input_build.rs` constructs app-server payload items, `input/input_decode_mentions.rs` owns linked-tool mention decoding, `input/input_decode_inline_mentions.rs` owns inline `@file` expansion, `input/input_decode_tokens.rs` owns token/env-var scanning helpers, `input/input_decode_inline.rs` and `input/input_decode.rs` are the compatibility facades over those decode helpers, `input/input_resolve_tools.rs` owns tool-mention extraction, `input/input_resolve_catalog.rs` owns catalog-based mention selection, `input/input_resolve.rs` is the compatibility facade over those resolution helpers, and `input/input_types.rs` owns the input-layer data types.

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

`dispatch.rs` sits above that payload construction layer. It decides whether a line is:

- a built-in slash command
- a local `!command`
- a normal user turn submission

That keeps command workflows separate from lower-level input item construction.

## Output and Rendering

`output.rs` is the thin facade over `output_prompt.rs` and `output_stream.rs`, which together own terminal writes and prompt redraw ordering.

Important properties:

- one ordered output path for transcript and prompt control
- explicit CRLF normalization for committed output
- prompt hide and redraw before emitted transcript blocks
- no mixed stdout/stderr interleaving for user-visible UI

`render.rs` is the compatibility facade for the split render layer. `render_prompt.rs` owns prompt fitting and committed prompt rendering. `render_blocks.rs` is now the compatibility facade for block rendering, while `render_block_common.rs` owns block classification/title/status styling, `render_block_markdown.rs` owns markdown block assembly, `render_markdown_code.rs` owns syntax-highlighted code rendering, `render_markdown_inline.rs` owns inline markdown parsing/tinting, `render_block_structured.rs` owns diff/command/plain block rendering, and `render_ansi.rs` owns ANSI serialization for `ratatui` text primitives such as `Text`, `Line`, and `Span`.

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
- backend-backed personality selection through startup-warmed `model/list` metadata plus `turn/start.personality`
- backend-backed background-terminal cleanup through `/ps clean` and `thread/backgroundTerminals/clean`
- `/diff`, `/apps`, `/skills`, `/models`, `/mcp`, `/threads`, `/feedback`, `/logout`, and related backend-backed commands
- automatic approval handling for supported approval request shapes
- auto-continue between turns

Some native Codex slash commands still remain informational placeholders because app-server does not expose the same internal UI state or backend surfaces that the native upstream TUI uses, but collaboration-mode switching, experimental-feature discovery, personality selection, background-terminal cleanup, and a text-only realtime flow are no longer in that category. `codexw` still does not implement the upstream audio UX; it surfaces realtime state and text transport only.

## Approval and Automation Posture

`codexw` defaults to a fully automated posture.

That includes:

- approval policy `never`
- danger-full-access sandbox posture
- automatic handling for command approvals
- automatic handling for file-change approvals
- best-effort answers for simple backend user-input requests

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
- Some commands can only explain their limitation because app-server does not expose the needed internal state.
- Rendering is richer than plain logs, but it is still terminal-scrollback based rather than a full alternate-screen widget tree.

## File Map

- `wrapper/src/main.rs`
  Thin entrypoint, module wiring, CLI flag definitions, and `main()`.
- `wrapper/src/main_tests.rs`
  Crate-level regression test hub for the split test modules.
- `wrapper/src/main_test_approvals.rs`
  Approval-decision and auto-approval regression tests.
- `wrapper/src/main_test_catalog.rs`
  Catalog/help/completion, thread-list, rate-limit, and resume-preview regression tests.
- `wrapper/src/main_test_runtime.rs`
  Runtime/editor/completion normalization and feedback-argument regression tests.
- `wrapper/src/main_test_session.rs`
  Session status, collaboration, personality, realtime-item, and tool-summary regression tests.
- `wrapper/src/app.rs`
  Top-level runtime loop and backend wiring.
- `wrapper/src/app_input.rs`
  Input-key dispatch for the live interactive session.
- `wrapper/src/policy.rs`
  Approval/sandbox/reasoning policy helpers and approval decision preferences.
- `wrapper/src/runtime.rs`
- `wrapper/src/runtime_input.rs`
- `wrapper/src/runtime_process.rs`
  Backend process startup, raw-mode lifecycle, keyboard mapping, and stdin/stdout/tick event threads.
- `wrapper/src/events.rs`
  Inbound JSON-RPC routing plus server-request handling and approval helpers.
- `wrapper/src/responses.rs`
  Compatibility facade for the split JSON-RPC response handlers.
- `wrapper/src/response_success.rs`
  Pending-request success handling, thread/session transitions, and post-response follow-up actions.
- `wrapper/src/response_error.rs`
  Pending-request error handling and recovery/reporting paths.
- `wrapper/src/notifications.rs`
  Compatibility facade for the split notification handlers.
- `wrapper/src/notification_realtime.rs`
  Realtime, account, app-list, and thread-status notification handling.
- `wrapper/src/notification_turn_lifecycle.rs`
  Skill-change, turn lifecycle, and auto-continue notification handling.
- `wrapper/src/notification_turn_items.rs`
  Thin router for split turn-item update and completion handlers.
- `wrapper/src/notification_item_updates.rs`
  Turn-item delta/status updates, verbose event reporting, and live status buffering.
- `wrapper/src/notification_item_completion.rs`
  Turn-item completion rendering for assistant text, commands, file changes, reasoning, and tool items.
- `wrapper/src/notification_turns.rs`
  Compatibility facade routing turn notifications to lifecycle and item handlers.
- `wrapper/src/catalog.rs`
  App and skill catalog parsing for app-server payloads.
- `wrapper/src/history.rs`
- `wrapper/src/history_render.rs`
- `wrapper/src/history_state.rs`
  Resume-preview extraction, resumed objective/reply seeding, and resumed conversation rendering.
- `wrapper/src/catalog_views.rs`
  Compatibility facade for the split catalog display helpers.
- `wrapper/src/catalog_lists.rs`
  Compatibility facade for the split catalog list renderers.
- `wrapper/src/catalog_app_views.rs`
  Apps, skills, and experimental-feature rendering helpers.
- `wrapper/src/catalog_backend_views.rs`
  Models and MCP server rendering helpers.
- `wrapper/src/catalog_threads.rs`
  Thread list, file-search rendering, and extracted id/path helpers.
- `wrapper/src/status_views.rs`
  Compatibility facade re-exporting the split status helpers plus generic JSON value summarization.
- `wrapper/src/status_config.rs`
  Permission and config rendering plus sandbox-policy summarization.
- `wrapper/src/status_account.rs`
  Account summary rendering.
- `wrapper/src/status_limits.rs`
  Compatibility facade for the split rate-limit and token-usage helpers.
- `wrapper/src/status_rate_limits.rs`
  Rate-limit window, credit, and reset-time rendering helpers.
- `wrapper/src/status_token_usage.rs`
  Token-usage summary rendering helpers.
- `wrapper/src/transcript_views.rs`
  Compatibility facade re-exporting the split transcript render and summary helpers.
- `wrapper/src/transcript_render.rs`
  Compatibility facade for the split transcript render helpers.
- `wrapper/src/transcript_completion_render.rs`
  Command/file-change completion and pending-attachment rendering helpers.
- `wrapper/src/transcript_plan_render.rs`
  Plan, reasoning, and structured tool-user-input response helpers.
- `wrapper/src/transcript_summary.rs`
  Approval/request/status summarizers, item-type humanization, and file-change/tool summary helpers.
- `wrapper/src/session_status.rs`
  Compatibility facade re-exporting split session prompt/realtime/snapshot helpers.
- `wrapper/src/session_prompt_status.rs`
  Prompt status rendering, spinner selection, and elapsed-time formatting.
- `wrapper/src/session_realtime.rs`
  Realtime session status and realtime item rendering helpers.
- `wrapper/src/session_snapshot.rs`
  Full `/status` session snapshot rendering.
- `wrapper/src/requests.rs`
  Compatibility facade for the split outbound-request layer.
- `wrapper/src/requests/request_types.rs`
  `PendingRequest` variants used to track in-flight JSON-RPC work.
- `wrapper/src/requests/bootstrap_init.rs`
  Initialize request and initialized notification builders.
- `wrapper/src/requests/bootstrap_account.rs`
  Account, logout, feedback-upload, and rate-limit request builders.
- `wrapper/src/requests/bootstrap_catalog.rs`
- `wrapper/src/requests/bootstrap_catalog_core.rs`
- `wrapper/src/requests/bootstrap_catalog_lists.rs`
- `wrapper/src/requests/bootstrap_search.rs`
  Catalog, config, model, collaboration-mode, thread-list, and file-search request builders.
- `wrapper/src/requests/thread_lifecycle.rs`
- `wrapper/src/requests/thread_switch.rs`
- `wrapper/src/requests/thread_maintenance.rs`
  Thread lifecycle request builders such as start, resume, fork, compact, rename, and background-terminal cleanup.
- `wrapper/src/requests/thread_activity.rs`
  Thread realtime and review request builders.
- `wrapper/src/requests/turn_requests.rs`
  Turn start, steer, and interrupt request builders.
- `wrapper/src/requests/command_requests.rs`
  Local command exec and terminate request builders.
- `wrapper/src/rpc.rs`
  JSON-RPC wire types and line parsing.
- `wrapper/src/response_bootstrap.rs`
  Successful initialize, catalog/config/account, and discovery response handling.
- `wrapper/src/response_threads.rs`
  Compatibility facade for the split thread/realtime/turn/local-command response layer.
- `wrapper/src/response_thread_switch.rs`
  Successful thread start, resume, fork, resumed-history rendering, and initial prompt handoff.
- `wrapper/src/response_thread_activity.rs`
  Successful realtime, review, turn, interrupt, and local-command response handling.
- `wrapper/src/commands.rs`
  Compatibility facade for the split command metadata/completion layer.
- `wrapper/src/commands_entries.rs`
  Static builtin command entry table.
- `wrapper/src/commands_catalog.rs`
  Command catalog facade and stable command-name ordering over the shared builtin entry table.
- `wrapper/src/commands_metadata.rs`
  Command descriptions and help-line generation over the shared command catalog.
- `wrapper/src/commands_completion.rs`
  Slash-completion facade and candidate rendering over the split command-matching helpers.
- `wrapper/src/commands_match.rs`
  Slash-command cursor parsing, fuzzy scoring, and longest-common-prefix helpers.
- `wrapper/src/input.rs`
  Compatibility facade for the split input layer.
- `wrapper/src/input/input_types.rs`
  Input-layer data types such as parsed payloads and catalog entries.
- `wrapper/src/input/input_decode.rs`
  Compatibility facade for the split decode layer.
- `wrapper/src/input/input_decode_mentions.rs`
  Linked-tool mention decoding and tool-path parsing helpers.
- `wrapper/src/input/input_decode_inline.rs`
  Compatibility facade for the split inline decode helpers.
- `wrapper/src/input/input_decode_inline_mentions.rs`
  Inline `@file` expansion and file-mention path rendering helpers.
- `wrapper/src/input/input_decode_tokens.rs`
  Prefixed-token collection plus low-level token/env-var classification helpers.
- `wrapper/src/input/input_resolve.rs`
  Compatibility facade for the split mention-resolution helpers.
- `wrapper/src/input/input_resolve_tools.rs`
  Tool-mention extraction from text and linked mentions.
- `wrapper/src/input/input_resolve_catalog.rs`
  Catalog-driven app/plugin/skill mention selection.
- `wrapper/src/input/input_build.rs`
  Structured turn payload construction for app-server `UserInput`.
- `wrapper/src/dispatch.rs`
  Compatibility facade for the split dispatch layer.
- `wrapper/src/dispatch_submit.rs`
  Normal prompt submission, slash-command detection, local `!command` launch, and turn/steer handoff.
- `wrapper/src/dispatch_commands.rs`
  Compatibility facade for the split slash-command layer.
- `wrapper/src/dispatch_command_thread.rs`
  Compatibility facade for the split thread-oriented slash-command workflows.
- `wrapper/src/dispatch_command_thread_flow.rs`
  Compatibility facade for the split thread-flow command helpers.
- `wrapper/src/dispatch_command_thread_navigation.rs`
  Thread lifecycle, rename, resume/new/fork, and recent-thread listing workflows.
- `wrapper/src/dispatch_command_thread_actions.rs`
  Thread action workflows such as review, interrupt, compact, and cleanup.
- `wrapper/src/dispatch_command_thread_workspace.rs`
  Mention/diff/copy/clear and attachment-queue slash-command workflows.
- `wrapper/src/dispatch_command_session.rs`
  Compatibility facade for the split session/catalog/realtime slash-command workflows.
- `wrapper/src/dispatch_command_session_info.rs`
  Session information and display slash-command workflows such as status, permissions, models, personality, apps, skills, MCP, and attachments.
- `wrapper/src/dispatch_command_session_control.rs`
  Compatibility facade for the split session-control command helpers.
- `wrapper/src/dispatch_command_session_modes.rs`
  Session mode and runtime-control workflows such as auto mode, collaboration, realtime control, and `/ps`.
- `wrapper/src/dispatch_command_session_meta.rs`
  Session meta workflows such as feedback, logout, and recognized-but-unported native popup paths.
- `wrapper/src/dispatch_command_utils.rs`
  Shared slash-command helpers such as built-in detection, feedback parsing, prompt joining, and clipboard handling.
- `wrapper/src/prompt_state.rs`
- `wrapper/src/prompt_completion.rs`
  Prompt completion facade and tab-completion entrypoint.
- `wrapper/src/prompt_file_completions.rs`
  `@file` token parsing, filesystem completion, and candidate rendering.
- `wrapper/src/prompting.rs`
  Prompt visibility/input gating, prompt redraw, slash completion, and `@file` completion helpers.
- `wrapper/src/model_catalog.rs`
  Model catalog extraction and effective-model selection.
- `wrapper/src/model_personality.rs`
  Personality rendering, validation, and model/personality action handling.
- `wrapper/src/model_session.rs`
  Compatibility facade for the split model catalog and personality helpers.
- `wrapper/src/collaboration_preset.rs`
  Collaboration preset extraction, summaries, and selector matching.
- `wrapper/src/collaboration_actions.rs`
  Active collaboration-mode value/label rendering and collaboration-mode action application.
- `wrapper/src/collaboration.rs`
  Compatibility facade for the split collaboration preset/action helpers.
- `wrapper/src/editor.rs`
  Inline line editor and editing semantics.
- `wrapper/src/editor_graphemes.rs`
  Grapheme-aware byte-index, counting, and whitespace helpers used by the editor.
- `wrapper/src/state.rs`
  Compatibility facade for the split shared-state layer.
- `wrapper/src/state_core.rs`
  `AppState`, process-output buffer types, and core state mutation methods.
- `wrapper/src/state_helpers.rs`
  Shared state/text/buffer helper functions such as `thread_id`, `get_string`, delta buffering, status dedupe, and path canonicalization.
- `wrapper/src/editor_tests.rs`
  Crate-level regression tests for the inline editor.
- `wrapper/src/output.rs`
  Prompt redraw, committed output, prompt visibility, output ordering.
- `wrapper/src/render.rs`
  Compatibility facade for the split render layer.
- `wrapper/src/render_prompt.rs`
  Prompt fitting, grapheme-aware cursor positioning, and committed prompt rendering.
- `wrapper/src/render_blocks.rs`
  Compatibility facade for the split rich block render layer.
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
- `wrapper/src/input_tests.rs`
  Crate-level regression tests for the split input layer facade and helpers.
- `skills/session-autopilot/`
  Companion cooperative skill for end-of-turn continuation policy.

## Practical Summary

`codexw` is best understood as a thin but capable interactive client around `codex app-server`:

- upstream Codex remains the execution engine
- `codexw` owns interaction, observability, and continuation policy runtime
- the bundled `session-autopilot` skill owns model-side continuation guidance

That separation is the central design decision of the project.
