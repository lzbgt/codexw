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

For the short implementer-facing summary of what the sibling `~/work/agent`
workspace should build against today, see
[codexw-broker-integration-handoff.md](codexw-broker-integration-handoff.md).

## Initial Goal

This section records the original minimum route goal for the first local-API
implementation slice. It is historical context now, not a claim that the API
is still at that stage.

The initial slice was supposed to expose only the routes needed to prove that
`codexw` can be controlled remotely without scraping terminal output:

- session create/attach/inspect
- turn start/interrupt
- transcript snapshot
- SSE event stream
- orchestration inspection
- wrapper-owned shell/service inspection and core mutation

## Route Ownership Matrix

| Route | Original phase | Existing source of truth | Proposed local API handler | Minimum verification |
| --- | --- | --- | --- | --- |
| `GET /healthz` | 1 | none | `local_api/routes/dispatch.rs`, `local_api/server.rs` | basic route smoke test |
| `POST /api/v1/session/new` | 2 | `state.rs`, `requests/thread_switch_common/*`, `response_thread_runtime.rs` | `local_api/routes/session.rs`, `local_api/control.rs` | Implemented. Reuses the current process-scoped local API session and queues a fresh Codex thread start |
| `POST /api/v1/session/attach` | 2 | `state.rs`, `requests/thread_switch_common/*` | `local_api/routes/session.rs`, `local_api/control.rs` | Implemented. Reuses the current process-scoped local API session and queues resume of an existing thread id |
| `GET /api/v1/session/{session_id}` | 2 | `state.rs`, `session_snapshot_overview.rs`, `session_snapshot_runtime.rs` | `local_api/routes/session.rs` | Implemented. Returns structured `session` + explicit process-scoped `attachment` metadata while preserving compatibility summary fields |
| `POST /api/v1/session/{session_id}/attachment/renew` | 2 | `local_api/control.rs`, `local_api/events.rs` | `local_api/routes/session.rs`, `local_api/control.rs` | Implemented. Renews the current process-scoped attachment lease with optional `client_id` ownership checks |
| `POST /api/v1/session/{session_id}/attachment/release` | 2 | `local_api/control.rs`, `local_api/events.rs` | `local_api/routes/session.rs`, `local_api/control.rs` | Implemented. Clears the current process-scoped attachment lease with optional `client_id` ownership checks |
| `POST /api/v1/turn/start` | 2 | `dispatch_submit_turns.rs`, `input/*`, `requests/turn_start.rs` | `local_api/routes/turn.rs` | Implemented. Also honors the active attachment lease via optional `client_id` |
| `POST /api/v1/turn/interrupt` | 2 | `dispatch_command_thread_control.rs`, `requests/turn_control.rs`, `app_input_interrupt.rs` | `local_api/routes/turn.rs` | Implemented. Also honors the active attachment lease via optional `client_id` |
| `POST /api/v1/session/{session_id}/turn/start` | 2 | `dispatch_submit_turns.rs`, `input/*`, `requests/turn_start.rs` | `local_api/routes/turn.rs` | Implemented. Session-scoped alias over turn start avoids redundant session ids in connector clients and honors the active attachment lease |
| `POST /api/v1/session/{session_id}/turn/interrupt` | 2 | `dispatch_command_thread_control.rs`, `requests/turn_control.rs`, `app_input_interrupt.rs` | `local_api/routes/turn.rs` | Implemented. Session-scoped alias over turn interrupt preserves the same runtime control path and honors the active attachment lease |
| `GET /api/v1/session/{session_id}/transcript` | 3 | transcript state + `transcript_*` summaries | `local_api/routes/transcript.rs` | bounded semantic snapshot without ANSI |
| `GET /api/v1/session/{session_id}/events` | 3 | runtime mutations across `events/*`, `notification_*`, `background_shells/*`, `orchestration_registry/*` | `local_api/events.rs`, `local_api/routes/event_stream.rs`, `local_api/server.rs` | Implemented. SSE stream emits semantic envelopes, supports `Last-Event-ID` replay, survives idle time with heartbeats, and now includes `status.updated` for async-tool supervision-classification changes |
| `GET /api/v1/session/{session_id}/orchestration/status` | 4 | `orchestration_view/summary/*` | `local_api/routes/orchestration.rs` | compact orchestration summary matches local status view semantics |
| `GET /api/v1/session/{session_id}/orchestration/workers` | 4 | `orchestration_view/workers/*` | `local_api/routes/orchestration.rs` | filter parsing and focused worker render correctness |
| `GET /api/v1/session/{session_id}/orchestration/dependencies` | 4 | `orchestration_view/dependencies.rs` | `local_api/routes/orchestration.rs` | dependency filters and focused capability queries work |
| `GET /api/v1/session/{session_id}/shells` | 4 | `background_shells/execution/manage/lifecycle/list.rs` | `local_api/routes/shells.rs` | lists current shell jobs without terminal formatting assumptions |
| `GET /api/v1/session/{session_id}/shells/{job_ref}` | 4 | `background_shells/execution/manage/lifecycle/list.rs`, `local_api/snapshot.rs` | `local_api/routes/shells.rs` | Implemented. Resolves one shell/job ref through the same id/alias/capability/index semantics and returns a structured `shell` snapshot |
| `POST /api/v1/session/{session_id}/shells/start` | 4 | `background_shells/execution/manage/lifecycle/start.rs` | `local_api/routes/shells.rs`, `local_api/control.rs` | Implemented. Queues wrapper-owned shell startup with existing tool validation, local-API origin tagging, and active lease enforcement |
| `POST /api/v1/session/{session_id}/shells/{job_ref}/poll` | 4 | `background_shells/execution/interact/tools/jobs.rs` | `local_api/routes/shells.rs`, `local_api/snapshot.rs` | Implemented. Resolves `job_ref` by id, alias, index, or unique capability and returns semantic shell snapshot data |
| `POST /api/v1/session/{session_id}/shells/{job_ref}/send` | 4 | `background_shells/execution/interact/tools/jobs.rs` | `local_api/routes/shells.rs`, `local_api/control.rs` | Implemented. Queues stdin writes against resolved shell jobs and enforces the active attachment lease |
| `POST /api/v1/session/{session_id}/shells/{job_ref}/terminate` | 4 | `background_shells/execution/interact/tools/jobs.rs` | `local_api/routes/shells.rs`, `local_api/control.rs` | Implemented. Queues one-job termination against resolved shell refs and enforces the active attachment lease |
| `GET /api/v1/session/{session_id}/services` | 4 | `background_shells/services/render/views/services/tool.rs` | `local_api/routes/services.rs` | service-state filters match existing dynamic-tool semantics |
| `GET /api/v1/session/{session_id}/services/{job_ref}` | 4 | `background_shells/services/render/views/services/tool.rs`, `local_api/snapshot.rs` | `local_api/routes/services.rs` | Implemented. Resolves a single service/job ref through the same id/alias/capability semantics and returns a structured `service` snapshot |
| `GET /api/v1/session/{session_id}/capabilities` | 4 | `background_shells/services/render/views/capabilities/list.rs` | `local_api/routes/services.rs` | capability-state filters and focused refs work |
| `GET /api/v1/session/{session_id}/capabilities/{capability}` | 4 | `background_shells/services/render/views/capabilities/detail.rs`, `local_api/snapshot.rs` | `local_api/routes/services.rs` | Implemented. Returns one exact capability entry with providers/consumers and stable `capability_not_found` behavior |
| `POST /api/v1/session/{session_id}/services/{job_ref}/provide` | 5 | `background_shells/services/updates/service/apply/*` | `local_api/routes/services.rs`, `local_api/control.rs` | Implemented. Capability mutation queues the same update path as `:ps provide` / `background_shell_update_service` and enforces the active attachment lease |
| `POST /api/v1/session/{session_id}/services/{job_ref}/depend` | 5 | `background_shells/services/updates/dependencies/apply.rs` | `local_api/routes/services.rs`, `local_api/control.rs` | Implemented. Dependency retargeting queues the same update path as `:ps depend` / `background_shell_update_dependencies` and enforces the active attachment lease |
| `POST /api/v1/session/{session_id}/services/{job_ref}/contract` | 5 | `background_shells/services/updates/service/apply/*` | `local_api/routes/services.rs`, `local_api/control.rs` | Implemented. Contract mutation requires at least one mutable contract field, reuses live service update validation, and enforces the active attachment lease |
| `POST /api/v1/session/{session_id}/services/{job_ref}/relabel` | 5 | `background_shells/services/updates/service/apply/*` | `local_api/routes/services.rs`, `local_api/control.rs` | Implemented. Label mutation queues the same update path as `:ps relabel` and enforces the active attachment lease |
| `POST /api/v1/session/{session_id}/services/{job_ref}/attach` | 5 | `background_shells/execution/interact/tools/services.rs` | `local_api/routes/services.rs` | Implemented. Resolves `job_ref` through the snapshot, returns structured `service` + `interaction` payloads while preserving the legacy attachment summary text, and enforces the active attachment lease |
| `POST /api/v1/session/{session_id}/services/{job_ref}/wait` | 5 | `background_shells/execution/interact/tools/services.rs` | `local_api/routes/services.rs` | Implemented. Waits on live service readiness with optional `timeoutMs`, returns structured `service` + `interaction` payloads while preserving the legacy result text, and enforces the active attachment lease |
| `POST /api/v1/session/{session_id}/services/{job_ref}/run` | 5 | `background_shells/execution/interact/tools/services.rs` | `local_api/routes/services.rs` | Implemented. Invokes a service recipe with optional `args` / `waitForReadyMs`, returns structured `service` + `recipe` + `interaction` payloads while preserving the legacy result text, and enforces the active attachment lease |
| `POST /api/v1/session/client_event` | 5 | `local_api/events.rs`, attachment lease policy in `local_api/routes/shared.rs` | `local_api/routes/client_events.rs` | Implemented. Top-level compatibility route validates `session_id`, publishes replayable `client.event` entries, and enforces the active attachment lease |
| `POST /api/v1/session/{session_id}/client_event` | 5 | `local_api/events.rs`, attachment lease policy in `local_api/routes/shared.rs` | `local_api/routes/client_events.rs` | Implemented. Session-scoped route publishes replayable `client.event` entries with optional structured `data` and enforces the active attachment lease |

## Suggested Module Layout

Current module tree:

- `wrapper/src/local_api.rs`
- `wrapper/src/local_api/server.rs`
- `wrapper/src/local_api/routes.rs`
- `wrapper/src/local_api/routes/client_events.rs`
- `wrapper/src/local_api/routes/dispatch.rs`
- `wrapper/src/local_api/routes/event_stream.rs`
- `wrapper/src/local_api/routes/shared.rs`
- `wrapper/src/local_api/routes/session.rs`
- `wrapper/src/local_api/routes/turn.rs`
- `wrapper/src/local_api/routes/transcript.rs`
- `wrapper/src/local_api/routes/orchestration.rs`
- `wrapper/src/local_api/routes/shells.rs`
- `wrapper/src/local_api/routes/services.rs`
- `wrapper/src/local_api/control.rs`
- `wrapper/src/local_api/snapshot.rs`
- `wrapper/src/local_api/events.rs`

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

Every route family currently shares:

- one JSON error contract from `local_api/routes/shared.rs`
- one session lookup path
- one thread/session identity model
- one auth gate abstraction in `local_api/routes/dispatch.rs`
- one job-ref parser policy

Connector-facing consistency already depends on two explicit choices that are
now implemented:

- session snapshots expose a process-scoped `attachment` object instead of
  leaving attachment semantics implicit
- session snapshots also expose explicit attachment ownership metadata:
  `client_id`, `lease_seconds`, `lease_expires_at_ms`, and `lease_active`
- mutating routes now share one stable JSON error envelope:
  `error.status`, `error.code`, `error.message`, `error.retryable`, and
  `error.details`
- attachment conflicts now include structured lease-holder details instead of
  only a text message
- turn control is available in both global and session-scoped forms so clients
  can choose between low-level compatibility and a cleaner session-rooted
  namespace

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

### Future Artifact Track

This route matrix is about the currently implemented local API plus its thin
planning boundary. The artifact surface is not implemented yet, but it is no
longer an unnamed future bucket.

Until that track is implemented, local-API consumers should still read the
current remote host-examination model as the shell-first foundation of the
current supported experimental adapter rather than as an implied artifact list/detail/content API.

If the artifact track moves into implementation, the first candidate route
family should be:

- `GET /api/v1/session/{session_id}/artifacts`
- `GET /api/v1/session/{session_id}/artifacts/{artifact_id}`
- `GET /api/v1/session/{session_id}/artifacts/{artifact_id}/content`

That work should follow the dedicated artifact docs rather than being designed
ad hoc here:

- [docs/codexw-broker-artifact-contract-sketch.md](docs/codexw-broker-artifact-contract-sketch.md)
- [docs/codexw-broker-artifact-implementation-plan.md](docs/codexw-broker-artifact-implementation-plan.md)

### Future Project/Dependency Collaboration Track

The local API also now has an explicit design-only collaboration lane for
cross-project work across multiple broker-routed deployments that may not share
one host.

If that track moves into implementation, the first candidate route family
should be:

- `POST /api/v1/session/{session_id}/project`
- `GET /api/v1/session/{session_id}/project`
- `POST /api/v1/projects/{project_id}/dependencies`
- `GET /api/v1/projects/{project_id}/dependencies`
- `GET /api/v1/dependencies/{dependency_id}`

That work should follow the dedicated project/dependency docs rather than being
designed ad hoc here:

- [docs/codexw-cross-project-dependency-contract-sketch.md](docs/codexw-cross-project-dependency-contract-sketch.md)
- [docs/codexw-cross-project-dependency-implementation-plan.md](docs/codexw-cross-project-dependency-implementation-plan.md)

The intended split stays explicit:

- project/dependency routes describe structural work relationships
- handoff routes describe concrete collaboration requests between deployments

## Exit Criteria

The original route-matrix phase is complete when:

1. every initial route has a named implementation owner
2. every initial route has a test target
3. local API modules can be created without inventing a second state model
4. the implementation sequence is obvious enough that the next step is code,
   not more route discovery
