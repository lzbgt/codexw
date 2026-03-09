# codexw Local API Route Matrix

This document turns the local API sketch into an implementation-facing route
inventory.

It answers four practical questions:

1. which routes belong in the first spike
2. which existing `codexw` modules already own the underlying behavior
3. which new local-API modules should expose that behavior
4. what minimum verification should exist per route family

This is intentionally narrower than a full protocol spec. The wire shapes live
in [codexw-local-api-sketch.md](codexw-local-api-sketch.md). This document is
about implementation ownership and delivery order.

## Phase 1 Goal

The first implementation slice should expose only the routes needed to prove
that `codexw` can be controlled remotely without scraping terminal output:

- session create/attach/inspect
- turn start/interrupt
- transcript snapshot
- SSE event stream
- orchestration inspection
- wrapper-owned shell/service inspection and core mutation

## Route Ownership Matrix

| Route | Phase | Existing source of truth | Proposed local API handler | Minimum verification |
| --- | --- | --- | --- | --- |
| `GET /healthz` | 1 | none | `local_api/routes/system.rs` | basic route smoke test |
| `POST /api/v1/session/new` | 2 | `state.rs`, `requests/thread_switch_common/*`, `response_thread_runtime.rs` | `local_api/routes/session.rs` | creates session id and optional attached thread |
| `POST /api/v1/session/attach` | 2 | `state.rs`, `requests/thread_switch_common/*` | `local_api/routes/session.rs` | attach existing thread id to session |
| `GET /api/v1/session/{session_id}` | 2 | `state.rs`, `session_snapshot_overview.rs`, `session_snapshot_runtime.rs` | `local_api/routes/session.rs` | summary payload is stable and session-scoped |
| `POST /api/v1/turn/start` | 2 | `dispatch_submit_turns.rs`, `input/*`, `requests/turn_start.rs` | `local_api/routes/turn.rs` | prompt text becomes a real turn request |
| `POST /api/v1/turn/interrupt` | 2 | `dispatch_command_thread_control.rs`, `requests/turn_control.rs`, `app_input_interrupt.rs` | `local_api/routes/turn.rs` | active turn is interrupted through the same control path |
| `GET /api/v1/session/{session_id}/transcript` | 3 | transcript state + `transcript_*` summaries | `local_api/routes/transcript.rs` | bounded semantic snapshot without ANSI |
| `GET /api/v1/session/{session_id}/events` | 3 | runtime mutations across `events/*`, `notification_*`, `background_shells/*`, `orchestration_registry/*` | `local_api/events.rs`, `local_api/server.rs` | Implemented. SSE stream emits semantic envelopes, supports `Last-Event-ID` replay, and survives idle time with heartbeats |
| `GET /api/v1/session/{session_id}/orchestration/status` | 4 | `orchestration_view/summary/*` | `local_api/routes/orchestration.rs` | compact orchestration summary matches local status view semantics |
| `GET /api/v1/session/{session_id}/orchestration/workers` | 4 | `orchestration_view/workers/*` | `local_api/routes/orchestration.rs` | filter parsing and focused worker render correctness |
| `GET /api/v1/session/{session_id}/orchestration/dependencies` | 4 | `orchestration_view/dependencies.rs` | `local_api/routes/orchestration.rs` | dependency filters and focused capability queries work |
| `GET /api/v1/session/{session_id}/shells` | 4 | `background_shells/execution/manage/lifecycle/list.rs` | `local_api/routes/shells.rs` | lists current shell jobs without terminal formatting assumptions |
| `POST /api/v1/session/{session_id}/shells/start` | 4 | `background_shells/execution/manage/lifecycle/start.rs` | `local_api/server.rs`, `local_api/control.rs` | Implemented. Queues wrapper-owned shell startup with existing tool validation and local-API origin tagging |
| `POST /api/v1/session/{session_id}/shells/{job_ref}/poll` | 4 | `background_shells/execution/interact/tools/jobs.rs` | `local_api/server.rs`, `local_api/snapshot.rs` | Implemented. Resolves `job_ref` by id, alias, index, or unique capability and returns semantic shell snapshot data |
| `POST /api/v1/session/{session_id}/shells/{job_ref}/send` | 4 | `background_shells/execution/interact/tools/jobs.rs` | `local_api/server.rs`, `local_api/control.rs` | Implemented. Queues stdin writes against resolved shell jobs |
| `POST /api/v1/session/{session_id}/shells/{job_ref}/terminate` | 4 | `background_shells/execution/interact/tools/jobs.rs` | `local_api/server.rs`, `local_api/control.rs` | Implemented. Queues one-job termination against resolved shell refs |
| `GET /api/v1/session/{session_id}/services` | 4 | `background_shells/services/render/views/services/tool.rs` | `local_api/routes/services.rs` | service-state filters match existing dynamic-tool semantics |
| `GET /api/v1/session/{session_id}/capabilities` | 4 | `background_shells/services/render/views/capabilities/list.rs` | `local_api/routes/services.rs` | capability-state filters and focused refs work |
| `POST /api/v1/session/{session_id}/services/{job_ref}/provide` | 5 | `background_shells/services/updates/service/apply/*` | `local_api/server.rs`, `local_api/control.rs` | Implemented. Capability mutation queues the same update path as `:ps provide` / `background_shell_update_service` |
| `POST /api/v1/session/{session_id}/services/{job_ref}/depend` | 5 | `background_shells/services/updates/dependencies/apply.rs` | `local_api/server.rs`, `local_api/control.rs` | Implemented. Dependency retargeting queues the same update path as `:ps depend` / `background_shell_update_dependencies` |
| `POST /api/v1/session/{session_id}/services/{job_ref}/contract` | 5 | `background_shells/services/updates/service/apply/*` | `local_api/server.rs`, `local_api/control.rs` | Implemented. Contract mutation requires at least one mutable contract field and reuses live service update validation |
| `POST /api/v1/session/{session_id}/services/{job_ref}/relabel` | 5 | `background_shells/services/updates/service/apply/*` | `local_api/server.rs`, `local_api/control.rs` | Implemented. Label mutation queues the same update path as `:ps relabel` |
| `POST /api/v1/session/{session_id}/services/{job_ref}/attach` | 5 | `background_shells/execution/interact/tools/services.rs` | `local_api/server.rs` | Implemented. Resolves `job_ref` through the snapshot and returns the same attachment summary as `background_shell_attach` |
| `POST /api/v1/session/{session_id}/services/{job_ref}/wait` | 5 | `background_shells/execution/interact/tools/services.rs` | `local_api/server.rs` | Implemented. Waits on live service readiness with optional `timeoutMs` and returns the same summary as `background_shell_wait_ready` |
| `POST /api/v1/session/{session_id}/services/{job_ref}/run` | 5 | `background_shells/execution/interact/tools/services.rs` | `local_api/server.rs` | Implemented. Invokes a service recipe with optional `args` / `waitForReadyMs` and returns the same result text as `background_shell_invoke_recipe` |

## Suggested Module Layout

Recommended new module tree:

- `wrapper/src/local_api.rs`
- `wrapper/src/local_api/server.rs`
- `wrapper/src/local_api/auth.rs`
- `wrapper/src/local_api/errors.rs`
- `wrapper/src/local_api/events.rs`
- `wrapper/src/local_api/routes/system.rs`
- `wrapper/src/local_api/routes/session.rs`
- `wrapper/src/local_api/routes/turn.rs`
- `wrapper/src/local_api/routes/transcript.rs`
- `wrapper/src/local_api/routes/orchestration.rs`
- `wrapper/src/local_api/routes/shells.rs`
- `wrapper/src/local_api/routes/services.rs`

This keeps the local API aligned with the current `codexw` separation:

- request building remains under `requests/*`
- runtime mutation remains under `events/*`, `notification_*`, and
  `background_shells/*`
- local API becomes a thin transport layer over those existing semantics

## Delivery Order

The most efficient route order is:

1. `GET /healthz`
2. session routes
3. turn routes
4. event stream
5. orchestration views
6. shell views and control
7. services and capabilities
8. service mutation routes

This minimizes churn because later routes depend on:

- stable session id handling
- stable JSON error shapes
- stable SSE fanout

## Cross-Cutting Requirements

Every route family should share:

- one JSON error contract
- one session lookup path
- one thread/session identity model
- one auth gate abstraction
- one job-ref parser policy

Do not duplicate route-specific versions of:

- `session_id` resolution
- `job_ref` parsing
- capability selector parsing
- worker/dependency filter parsing

Those should stay aligned with the existing internal surfaces or be factored
into shared local-API helpers.

## Minimum Test Inventory

### Session

- create unattached session
- create attached session
- attach existing thread
- inspect missing session id returns stable JSON error

### Turn

- start turn through API
- interrupt active turn through API
- reject turn start on unattached session

### Events

- SSE connect works
- idle connection stays open
- at least one real turn emits:
  - `turn.started`
  - `transcript.item`
  - `turn.completed`

### Orchestration

- `status` matches expected summary keys
- worker filter parsing matches local semantics
- dependency filter parsing matches local semantics

### Shells/Services

- start/poll/send/terminate one shell
- list services and capabilities
- mutate service capability/label/contract
- retarget service dependencies

## Exit Criteria

The route matrix phase is complete when:

1. every Phase 1 route has a named implementation owner
2. every Phase 1 route has a test target
3. local API modules can be created without inventing a second state model
4. the implementation sequence is obvious enough that the next step is code,
   not more route discovery
