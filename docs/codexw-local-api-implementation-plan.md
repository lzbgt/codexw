# codexw Local API Implementation Plan

This document turns the local API sketch into an implementation-facing plan for
the first `codexw` daemon/control surface spike.

It is intentionally narrow:

- loopback only
- HTTP + SSE only
- no broker transport yet
- no auth model beyond optional local bearer token

## Objective

Build the smallest useful local API that can:

1. create and attach wrapper sessions
2. start and interrupt turns
3. stream transcript/status/orchestration events
4. inspect and control wrapper-owned shell/service state

If this spike succeeds, `codexw` will be in a position to support:

- a local WebUI or browser client
- local automation
- a future broker connector

without scraping terminal scrollback.

## Current Implementation Status

The first code slice has started.

Current implemented scope:

- disabled-by-default loopback local API startup
- configurable bind address and optional bearer token
- `GET /healthz`
- `POST /api/v1/session/new`
- `POST /api/v1/session/attach`
- `GET /api/v1/session`
- `GET /api/v1/session/{session_id}`
- `GET /api/v1/session/{session_id}/transcript`
- `GET /api/v1/session/{session_id}/orchestration/status`
- `GET /api/v1/session/{session_id}/orchestration/dependencies`
- `GET /api/v1/session/{session_id}/orchestration/workers`
- `GET /api/v1/session/{session_id}/shells`
- `POST /api/v1/session/{session_id}/shells/start`
- `POST /api/v1/session/{session_id}/shells/{job_ref}/poll`
- `POST /api/v1/session/{session_id}/shells/{job_ref}/send`
- `POST /api/v1/session/{session_id}/shells/{job_ref}/terminate`
- `GET /api/v1/session/{session_id}/services`
- `GET /api/v1/session/{session_id}/capabilities`
- `POST /api/v1/turn/start`
- `POST /api/v1/turn/interrupt`
- `GET /api/v1/session/{session_id}/events`
- internal API command queue from the HTTP listener into the main runtime loop
- shared semantic event log with `Last-Event-ID` replay
- loopback SSE event stream for session, turn, orchestration, worker, and
  capability updates
- structured snapshot export for orchestration, shell, service, capability, and
  transcript state

Current non-goals of the landed slice:

- no turn steer route yet
- shell recipe/attach/wait routes now exist through explicit service interaction endpoints
- explicit service mutation routes now exist for `provide`, `depend`,
  `contract`, and `relabel`

That means the next implementation step is no longer transcript or basic shell
control. It is higher-level session attachment semantics for connectors and
broader remote-client coverage above the now-usable route surface.

## Scope

### In Scope

- loopback HTTP listener
- loopback SSE event streaming
- session/turn lifecycle
- orchestration read APIs
- background shell/service read + control APIs
- stable JSON errors

### Out Of Scope

- direct broker connectivity
- websocket transport
- browser cookie auth
- audio/media streaming
- artifact upload/download system
- scene/entity APIs
- deployment routing

## Proposed Runtime Shape

Recommended first spike:

- keep the current TTY client path intact
- add an optional embedded local API server inside `codexw`
- expose the same underlying state/request builders through:
  - terminal commands
  - local HTTP routes

This avoids creating a second daemon binary too early.

## Phase Breakdown

### Phase 1: API Skeleton

Deliverables:

- HTTP listener startup config
- route registry
- JSON response helper
- SSE event stream helper
- minimal auth gate abstraction

Acceptance criteria:

- local process can start with API disabled or enabled
- disabled mode keeps current behavior unchanged
- enabled mode serves `/healthz` and a stub SSE stream

### Phase 2: Session And Turn Control

Status: initial slice landed

Deliverables:

- `POST /api/v1/session/new`
- `POST /api/v1/session/attach`
- `GET /api/v1/session/{session_id}`
- `POST /api/v1/turn/start`
- `POST /api/v1/turn/interrupt`
- `POST /api/v1/session/{session_id}/turn/start`
- `POST /api/v1/session/{session_id}/turn/interrupt`

Acceptance criteria:

- API routes can drive the same underlying thread/session lifecycle as the TTY
- responses include `session_id`, `thread_id`, and `turn_id` where relevant
- session-bearing routes expose an explicit process-scoped `session` contract
  instead of relying on clients to infer attachment semantics from prose

Current landed behavior:

- `POST /api/v1/session/new` now means “reuse the current process-scoped local
  API session and start a fresh Codex thread”
- `POST /api/v1/session/attach` now means “reuse the current process-scoped
  local API session and resume a specific existing thread id”
- `GET /api/v1/session/{session_id}` now returns a structured `session` object
  with nested process-scoped `attachment` metadata
- `POST /api/v1/session/new` and `POST /api/v1/session/attach` now accept
  optional `client_id` and `lease_seconds`
- `POST /api/v1/session/{session_id}/turn/start` and
  `POST /api/v1/session/{session_id}/turn/interrupt` now exist as session-scoped
  aliases over the same turn control path, so connector clients can stay within
  a single session-rooted route namespace
- `POST /api/v1/session/{session_id}/attachment/renew` and
  `POST /api/v1/session/{session_id}/attachment/release` now exist for explicit
  attachment lease management within the process-scoped session model
- all mutating turn, shell, and service routes now enforce the active
  attachment lease:
  - optional `client_id` is accepted on those mutation routes
  - anonymous or mismatched callers receive `409 attachment_conflict`
- both routes enqueue onto the same runtime request path used by local
  thread-switch handling rather than inventing a second session model

### Phase 3: Event Stream

Status: initial slice landed

Deliverables:

- `GET /api/v1/session/{session_id}/events`
- first stable event families:
  - `session.*`
  - `turn.*`
  - `transcript.item`
  - `status.updated`
  - `orchestration.updated`

Acceptance criteria:

- local client can attach and receive semantic events without reading terminal
  output
- event stream survives ordinary quiet periods without false disconnect

Current landed behavior:

- `GET /api/v1/session/{session_id}/events` serves `text/event-stream`
- replay is supported through the `Last-Event-ID` header
- current event families are:
  - `session.updated`
  - `turn.updated`
  - `orchestration.updated`
  - `workers.updated`
  - `capabilities.updated`
- the stream is sourced from semantic snapshot deltas, not ANSI terminal output
- heartbeat comments keep the connection alive during quiet periods
- `session.updated` now carries the same explicit `session` + `attachment`
  structure returned by the session snapshot routes

### Phase 4: Orchestration And Shell Surfaces

Deliverables:

- `GET /api/v1/session/{session_id}/orchestration/status`
- `GET /api/v1/session/{session_id}/orchestration/workers`
- `GET /api/v1/session/{session_id}/orchestration/dependencies`
- `GET /api/v1/session/{session_id}/shells`
- `POST /api/v1/session/{session_id}/shells/start`
- `POST /api/v1/session/{session_id}/shells/{job_ref}/poll`
- `POST /api/v1/session/{session_id}/shells/{job_ref}/send`
- `POST /api/v1/session/{session_id}/shells/{job_ref}/terminate`
- `GET /api/v1/session/{session_id}/services`
- `GET /api/v1/session/{session_id}/capabilities`

Acceptance criteria:

- a local browser or remote terminal can inspect blocker state, services, and
  shell jobs without relying on `:ps`

### Phase 5: Service Mutation Controls

Deliverables:

- `POST /api/v1/session/{session_id}/services/{job_ref}/provide`
- `POST /api/v1/session/{session_id}/services/{job_ref}/depend`
- `POST /api/v1/session/{session_id}/services/{job_ref}/contract`
- `POST /api/v1/session/{session_id}/services/{job_ref}/relabel`

Acceptance criteria:

- service/capability orchestration can be driven entirely through the local API
- no shell/service operation still requires terminal-only command parsing

Current status:

- implemented through the loopback API and queued onto the same runtime mutation
  path used by the existing local shell/service control surfaces
- synchronous service interaction routes now exist for `attach`, `wait`, and
  `run`, backed by the live `BackgroundShellManager` rather than the fire-and-
  forget command queue
- those service interaction routes now return structured machine-usable payloads
  alongside compatibility text fields:
  - `service` snapshot payload
  - `interaction` metadata
  - `recipe` metadata for `run`
  - legacy `attachment` / `result` text preserved as `attachment_text` /
    `result_text`
- attach/lease semantics now exist for the current process-scoped model:
  - optional `client_id`
  - optional `lease_seconds`
  - derived `lease_expires_at_ms`
  - explicit renew/release routes
- the next connector-facing gap is no longer lease metadata. It is
  multi-client or connector-specific attachment policy above the current
  single process-scoped control context

## Candidate Code Ownership

### Current Module Ownership

- `wrapper/src/local_api.rs`
  - namespace root and test-only re-exports
- `wrapper/src/local_api/server.rs`
  - TCP listener lifecycle, HTTP parsing, connection handling, and response writeout
- `wrapper/src/local_api/routes.rs`
  - shared auth, JSON helpers, session lookup, `job_ref` resolution, and top-level route dispatch
- `wrapper/src/local_api/routes/session.rs`
  - session inspect/new/attach route handlers
- `wrapper/src/local_api/routes/turn.rs`
  - turn start/interrupt route handlers
- `wrapper/src/local_api/routes/transcript.rs`
  - transcript snapshot route handler
- `wrapper/src/local_api/routes/orchestration.rs`
  - orchestration status/worker/dependency route handlers
- `wrapper/src/local_api/routes/shells.rs`
  - shell list/start/poll/send/terminate route handlers
- `wrapper/src/local_api/routes/services.rs`
  - service/capability list and service mutation/interaction route handlers
- `wrapper/src/local_api/control.rs`
  - queued runtime control commands for session, turn, shell, and service mutations
- `wrapper/src/local_api/snapshot.rs`
  - structured snapshot assembly for sessions, orchestration, transcript, shells, services, and capabilities
- `wrapper/src/local_api/events.rs`
  - SSE event log, replay, and semantic local-API event publication

### Existing Modules Reused By The Local API

- `wrapper/src/state.rs`
- `wrapper/src/requests/thread_switch_common/*`
- `wrapper/src/background_shells/*`
- `wrapper/src/orchestration_view/*`
- `wrapper/src/orchestration_registry/*`
- `wrapper/src/state_helpers/*`
- `wrapper/src/events/*`

The remaining code-level follow-up is not creating more top-level local-API
modules. It is keeping the current split stable and only extracting smaller
helpers if one of the route-family files becomes a hotspot again.

## Critical Integration Constraint

The local API must not fork a separate state model.

It must read and mutate the same runtime state already used by:

- terminal prompt/status rendering
- request dispatch
- orchestration snapshots
- background shell/service control

If the spike starts building a second state cache, it is going in the wrong
direction.

## Event Source Strategy

The API stream should not scrape rendered transcript text.

Recommended approach:

- fan out semantic runtime updates near the same points that already mutate:
  - transcript state
  - turn status
  - orchestration state
  - background shell/service state

This likely means introducing a small event-bus abstraction attached to
`AppState` or the runtime layer rather than trying to derive events from
terminal-render code.

## Error Model Requirements

Phase 1 should standardize, through the shared helpers already living in
`wrapper/src/local_api/routes.rs`:

- `session_*`
- `thread_*`
- `turn_*`
- `job_*`
- `service_*`
- `capability_*`
- `validation_*`

Errors should always be JSON and should never expose raw terminal-only wording
as the primary contract. A dedicated `local_api/errors.rs` module is not
required unless the current shared helper layer becomes too large or divergent.

## Security Defaults

Phase 1 recommendation:

- bind only to `127.0.0.1`
- local API disabled by default
- optional bearer token
- explicit warning in docs that this is local-control only, not an internet
  service

## Verification Requirements

The spike is not done unless it has:

- route-level tests for the new handlers
- an SSE smoke test
- no regression to existing TTY behavior
- `cargo test`
- `cargo build`
- `./scripts/install-codexw` when code changes land

## Exit Criteria

This local API spike is successful if all of the following are true:

1. a local client can create/attach a wrapper session
2. a local client can start and interrupt turns
3. a local client can consume semantic transcript/status/orchestration events
4. a local client can inspect and control wrapper-owned shell/service state
5. no broker transport is required to validate the model
