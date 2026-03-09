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

Deliverables:

- `POST /api/v1/session/new`
- `POST /api/v1/session/attach`
- `GET /api/v1/session/{session_id}`
- `POST /api/v1/turn/start`
- `POST /api/v1/turn/interrupt`

Acceptance criteria:

- API routes can drive the same underlying thread/session lifecycle as the TTY
- responses include `session_id`, `thread_id`, and `turn_id` where relevant

### Phase 3: Event Stream

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

### Phase 4: Orchestration And Shell Surfaces

Deliverables:

- `GET /api/v1/session/{session_id}/orchestration/status`
- `GET /api/v1/session/{session_id}/orchestration/workers`
- `GET /api/v1/session/{session_id}/orchestration/dependencies`
- `GET /api/v1/session/{session_id}/shells`
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

## Candidate Code Ownership

### New Modules Likely Needed

- `wrapper/src/local_api.rs`
  - namespace root for the local API server
- `wrapper/src/local_api/server.rs`
  - listener start/stop and route wiring
- `wrapper/src/local_api/routes/session.rs`
- `wrapper/src/local_api/routes/turn.rs`
- `wrapper/src/local_api/routes/orchestration.rs`
- `wrapper/src/local_api/routes/shells.rs`
- `wrapper/src/local_api/routes/services.rs`
- `wrapper/src/local_api/events.rs`
  - SSE fanout and event serialization
- `wrapper/src/local_api/errors.rs`
  - stable JSON error shapes

### Existing Modules To Reuse

- `wrapper/src/state.rs`
- `wrapper/src/requests/thread_switch_common/*`
- `wrapper/src/background_shells/*`
- `wrapper/src/orchestration_view/*`
- `wrapper/src/orchestration_registry/*`
- `wrapper/src/state_helpers/*`
- `wrapper/src/events/*`

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

Phase 1 should standardize:

- `session_*`
- `thread_*`
- `turn_*`
- `job_*`
- `service_*`
- `capability_*`
- `validation_*`

Errors should always be JSON and should never expose raw terminal-only wording
as the primary contract.

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
