# codexw Broker Connectivity Design

## Objective

Define how `codexw` could evolve from a local terminal wrapper into a remotely reachable Codex runtime that can be driven through a broker by multiple client types such as:

- a mobile app
- a browser UI
- a remote terminal
- other automation clients

This document is an investigation and design-planning artifact. It is not a commitment to immediate implementation.

## Why This Matters

`codexw` already has several properties that make remote access plausible:

- a serialized internal state model in `wrapper/src/state.rs`
- structured transcript and status rendering rather than raw log dumping
- explicit orchestration and worker views
- wrapper-owned async shell and service-control surfaces
- clear turn/session lifecycle boundaries

That means the remaining work is not inventing all remote concepts from scratch. The higher-value problem is deciding which existing local concepts should become stable remote APIs.

## Reference Baseline From `~/work/agent`

The sibling `~/work/agent` project already defines a concrete daemon/broker/client model that is relevant here:

- `/Users/zongbaolu/work/agent/DESIGN.md`
  - daemon-first multi-client architecture
  - broker relay flow
  - outbound agent connectivity
- `/Users/zongbaolu/work/agent/broker/README.md`
  - `wss://.../v1/agent/connect`
  - `/v1/agents/{agent_id}/proxy/...`
  - `/v1/agents/{agent_id}/proxy_sse/...`
  - OIDC/token auth and optional mTLS
- `/Users/zongbaolu/work/agent/docs/PROTOCOL.md`
  - run/event envelopes
  - session-safe event flows
  - artifact signaling
- `/Users/zongbaolu/work/agent/docs/CLIENT.md`
  - client identity
  - client events
  - bidirectional UI/agent collaboration model

Those documents make brokered connectivity for `codexw` a fact-based design investigation, not a generic wishlist item.

## Non-Goals For The First Design Slice

- replacing `codex app-server`
- forcing full protocol compatibility with `~/work/agent` before a gap analysis
- redesigning the local inline terminal UX first
- exposing every internal render detail as a public API

## Constraints

Any broker-connected `codexw` design has to respect these realities:

1. `codexw` is currently an app-server client, not a standalone agent daemon.
2. Some async execution behavior is wrapper-owned because app-server does not expose public control of model-owned `item/commandExecution` sessions.
3. `codexw` today is terminal-first and scrollback-first, not a browser-first service runtime.
4. Remote APIs must map to stable state transitions, not terminal presentation artifacts.

## Architectural Options

### Option A: Local API + Separate Connector

Shape:

- `codexw` exposes a local HTTP/SSE API
- a connector process speaks the cloud broker protocol
- remote clients go through the connector

Pros:

- keeps broker logic out of the main wrapper process
- easiest path to partial compatibility with `~/work/agent`
- lower risk to the local interactive terminal workflow

Cons:

- two-process deployment model
- more moving parts for local development
- another compatibility boundary to maintain

### Option B: Direct Broker Connectivity In `codexw`

Shape:

- `codexw` itself maintains an outbound broker connection
- clients proxy into the running wrapper directly

Pros:

- simplest runtime graph once running
- fewer components to deploy
- closest to "one runtime, many clients"

Cons:

- mixes local TTY concerns with remote control-plane concerns
- harder security boundary
- more invasive lifecycle and reconnect logic inside the wrapper

### Option C: Compatibility Layer Only

Shape:

- `codexw` keeps working locally
- it exposes a protocol shaped like the `agent` broker/client surfaces
- another host runtime can translate or embed it

Pros:

- lowest implementation risk
- allows iterative compatibility experiments

Cons:

- does not actually make `codexw` remotely reachable by itself
- lower product value than Options A or B

## Recommended Direction

The current highest-leverage path is:

1. define a local `codexw` HTTP/SSE API first
2. model it against the `~/work/agent` broker/client contract
3. keep direct broker connectivity as a second-phase decision

That keeps the first implementation aligned with the existing wrapper architecture and still preserves a path to broker compatibility.

## Related Design Set

The broker/local-API design is now split across concrete implementation-facing
documents:

- [codexw-local-api-sketch.md](codexw-local-api-sketch.md)
- [codexw-local-api-route-matrix.md](codexw-local-api-route-matrix.md)
- [codexw-local-api-event-sourcing.md](codexw-local-api-event-sourcing.md)
- [codexw-local-api-implementation-plan.md](codexw-local-api-implementation-plan.md)
- [codexw-broker-event-envelope.md](codexw-broker-event-envelope.md)
- [codexw-broker-endpoint-audit.md](codexw-broker-endpoint-audit.md)
- [codexw-broker-connector-decision.md](codexw-broker-connector-decision.md)
- [codexw-broker-connector-mapping.md](codexw-broker-connector-mapping.md)
- [codexw-broker-connector-prototype-plan.md](codexw-broker-connector-prototype-plan.md)
- [codexw-broker-session-identity.md](codexw-broker-session-identity.md)
- [codexw-broker-compatibility-target.md](codexw-broker-compatibility-target.md)
- [codexw-broker-shared-assumptions.md](codexw-broker-shared-assumptions.md)

## Compatibility Matrix

### Areas That Already Map Reasonably Well

- session lifecycle
  - `codexw` has thread start/resume/fork and explicit status surfaces
- event streaming
  - `codexw` already has structured item, turn, status, and orchestration events internally
- orchestration inspection
  - `:status`, `:ps`, dependency views, capability views, and worker summaries are already modeled
- background execution
  - wrapper-owned shell/service control already exists

### Areas That Need Explicit Design

- remote session identity
  - mapping local thread ids, wrapper session state, and remote client sessions
- auth model
  - local-only token, browser-safe auth, broker token auth, or mTLS
- event/public API stability
  - which internal state transitions are safe to make public
- approval semantics
  - remote clients may need visibility into approval posture even if local defaults stay automated
- multi-client concurrency
  - steering, interrupting, and session mutation need ordering rules

### Areas That Are Still Architectural Gaps

- alternate-screen/native-TUI parity
- upstream audio/realtime UX parity
- app-server-owned command session reuse

Those are important, but they are not blockers for a first remote-control API if the scope is state/control/event transport rather than full UI parity.

## Surface Mapping: Current `codexw` To Remote API

The following table captures the highest-value existing local surfaces that could become remote API surfaces first.

| Current local surface | Internal source of truth | Remote shape to design | Notes |
| --- | --- | --- | --- |
| `:status` | `state.rs`, `session_snapshot_overview.rs`, `session_snapshot_runtime.rs` | `GET /api/v1/status` + `status.updated` events | Good first remote inspection surface because it is already summary-oriented |
| `:ps` worker views | `orchestration_view/*`, `orchestration_registry/*` | `GET /api/v1/orchestration/workers?filter=...` | Maps naturally to client dashboards and remote terminals |
| `:ps dependencies` | `orchestration_view/dependencies.rs` | `GET /api/v1/orchestration/dependencies?filter=...` | Already structured enough to avoid parsing terminal text |
| transcript scrollback | item/turn state + transcript render helpers | `GET /api/v1/transcript` + SSE event stream | Remote clients should consume semantic item events, not ANSI blocks |
| turn submit / steer / interrupt | request builders + `dispatch_submit_*` | `POST /api/v1/turn/start`, `POST /api/v1/turn/steer`, `POST /api/v1/turn/interrupt` | Existing local lifecycle is already explicit |
| background shell jobs | `background_shells/*` | `POST /api/v1/shells/*`, `GET /api/v1/shells` | One of the highest-value remote-control surfaces |
| reusable services / capabilities | `background_shells/services/*` | `GET /api/v1/services`, `GET /api/v1/capabilities` | Needed for mobile/WebUI service attachment and orchestration reuse |
| model / personality / approvals | selection flow + request overrides | `GET /api/v1/session/config`, `POST /api/v1/session/config` | Useful, but lower priority than status and turn control |
| local prompt/editor state | terminal-only editor modules | no public API in phase 1 | Treat as local-only presentation detail |

The right first implementation is not “remote terminal mirroring.” It is exposing the stable semantic layers already present underneath the terminal renderer.

## API Surface To Design

The first remote API proposal should cover:

### Session Endpoints

- create wrapper session
- list resumable threads
- attach to thread
- expose cwd/objective/current model/personality/collab state

### Turn Endpoints

- submit turn
- steer turn
- interrupt turn
- fetch last assistant reply / last diff / last status

### Stream Endpoints

- transcript/event stream
- orchestration stream
- status stream

### Orchestration Endpoints

- `orchestration_status`
- worker views
- dependency views
- guidance/actions views

### Background Shell Endpoints

- start/poll/send/terminate
- attach/wait/run recipe
- provide/depend/relabel/contract
- list services and capabilities

## First-Pass Local API Sketch

This is not the final wire contract. It is a candidate shape for the first audit/implementation pass.

### Session

- `POST /api/v1/session/new`
  - creates a remote session handle bound to a local cwd/objective context
- `GET /api/v1/session`
  - returns current session summary, active thread id, cwd, model, personality, collaboration mode
- `POST /api/v1/session/attach`
  - attaches the remote client to an existing local thread or wrapper session

### Turns

- `POST /api/v1/turn/start`
  - starts a new turn with prompt text and optional attachments/mentions
- `POST /api/v1/turn/steer`
  - adds steer input to the active turn
- `POST /api/v1/turn/interrupt`
  - interrupts active work

### Transcript And Events

- `GET /api/v1/transcript`
  - snapshot view of recent transcript items
- `GET /api/v1/events`
  - SSE stream of semantic updates

### Orchestration

- `GET /api/v1/orchestration/status`
- `GET /api/v1/orchestration/workers?filter=...`
- `GET /api/v1/orchestration/dependencies?filter=...`
- `GET /api/v1/orchestration/actions`

### Background Shells

- `GET /api/v1/shells`
- `POST /api/v1/shells/start`
- `POST /api/v1/shells/{job}/poll`
- `POST /api/v1/shells/{job}/send`
- `POST /api/v1/shells/{job}/terminate`
- `POST /api/v1/shells/{job}/attach`
- `POST /api/v1/shells/{job}/wait_ready`
- `POST /api/v1/shells/{job}/invoke_recipe`

### Reusable Services And Capabilities

- `GET /api/v1/services`
- `GET /api/v1/services/{job}`
- `GET /api/v1/capabilities`
- `GET /api/v1/capabilities/{capability}`
- `POST /api/v1/services/{job}/update`
- `POST /api/v1/services/{job}/update_dependencies`

## First-Pass Event Model Sketch

The first remote stream should prefer a small set of stable semantic events:

- `session.attached`
- `turn.started`
- `turn.completed`
- `transcript.item`
- `status.updated`
- `orchestration.updated`
- `shell.updated`
- `service.updated`
- `capability.updated`

For phase 1, every event should satisfy two constraints:

1. it can be generated from existing internal state without depending on terminal-render logic
2. it is useful to more than one client type

That avoids overfitting the first API to a single WebUI or terminal proxy.

## Example Local API Payloads

These examples are intentionally small and semantic. They are not final protocol commitments.

### `POST /api/v1/session/new`

Request:

```json
{
  "cwd": "/work/repo",
  "objective": "Continue the highest-leverage engineering work."
}
```

Response:

```json
{
  "ok": true,
  "session_id": "sess_01HX...",
  "cwd": "/work/repo",
  "active_thread_id": null,
  "automation_mode": "auto"
}
```

### `POST /api/v1/session/attach`

Request:

```json
{
  "session_id": "sess_01HX...",
  "thread_id": "thread_abc123"
}
```

Response:

```json
{
  "ok": true,
  "session_id": "sess_01HX...",
  "thread_id": "thread_abc123",
  "model": "gpt-5-codex",
  "personality": "balanced",
  "collaboration_mode": "plan"
}
```

### `POST /api/v1/turn/start`

Request:

```json
{
  "session_id": "sess_01HX...",
  "text": "Review the recent changes and continue with the next highest-leverage task."
}
```

Response:

```json
{
  "ok": true,
  "session_id": "sess_01HX...",
  "thread_id": "thread_abc123",
  "turn_id": "turn_def456"
}
```

### `GET /api/v1/orchestration/status`

Response:

```json
{
  "ok": true,
  "session_id": "sess_01HX...",
  "main_agent": {
    "state": "blocked",
    "reason": "waiting_on_shell_prerequisite"
  },
  "counts": {
    "waits": 1,
    "sidecar_agents": 2,
    "exec_prereqs": 1,
    "exec_services": 1,
    "terminals": 0
  },
  "next_action": {
    "kind": "tool_call",
    "tool": "background_shell_wait_ready",
    "arguments": {
      "jobId": "bg-2",
      "timeoutMs": 5000
    }
  }
}
```

### `GET /api/v1/events`

Representative SSE event:

```text
event: orchestration.updated
data: {"session_id":"sess_01HX...","counts":{"waits":1,"exec_prereqs":1},"next_action":{"kind":"tool_call","tool":"background_shell_poll","arguments":{"jobId":"bg-1"}}}
```

## Event Envelope Candidate

The likely phase-1 event envelope should stay simple:

```json
{
  "type": "status.updated",
  "session_id": "sess_01HX...",
  "thread_id": "thread_abc123",
  "turn_id": "turn_def456",
  "ts_unix_ms": 1760000000000,
  "data": {
    "working": true,
    "elapsed_ms": 12450
  }
}
```

Candidate required fields:

- `type`
- `session_id`
- `ts_unix_ms`
- `data`

Candidate optional fields:

- `thread_id`
- `turn_id`
- `item_id`
- `source`

For compatibility with the `~/work/agent` direction, the envelope should remain:

- machine-readable
- append-only
- reconnect-friendly
- usable without parsing terminal-formatted text

## Security And Deployment Matrix

The first implementation should choose one row explicitly rather than leaving auth/deployment implicit.

| Mode | Exposure | Auth model | Browser support | Broker-ready | Recommended use |
| --- | --- | --- | --- | --- | --- |
| Loopback-only local API | localhost only | optional local bearer token | limited, same-host only | not directly | safest phase-1 path |
| Local API + connector | localhost API, remote via connector | local token + connector auth | yes, via connector/broker | yes | best compatibility path |
| Direct broker in `codexw` | remote reachable | broker token and/or mTLS | yes | yes | later phase only |

Recommended initial choice:

- phase 1: loopback-only local API, optional bearer token
- phase 2: local API plus connector
- phase 3: evaluate whether direct broker connectivity is worth the complexity

## Open Implementation Risks

- reconnect and replay semantics may be harder than the initial API shape suggests
- multi-client mutation ordering could force explicit actor ownership rules
- exposing shell/service control remotely increases the blast radius of auth mistakes
- app-server version drift may make some internal events harder to stabilize as public API contracts
- browser-safe auth may pressure the API shape earlier than expected if a WebUI becomes the first client

## Mapping Against `~/work/agent`

The likely compatibility plan is partial, not immediate full parity.

### Directly Reusable Ideas

- outbound broker connection model
- HTTP plus SSE relay split
- explicit client identity
- session-safe event transport
- durable machine-readable event envelopes instead of UI heuristics

### Likely Different In `codexw`

- `codexw` is not a standalone agent daemon; it is a wrapper around `codex app-server`
- background-shell execution is wrapper-owned, not backend-native
- transcript semantics are shaped by app-server item/turn events rather than a custom run engine
- local thread ids and resume flows already exist and should not be hidden behind a totally unrelated session abstraction

### Likely First Compatibility Target

The most realistic initial target is:

- reuse the broker/client conceptual model
- reuse the event and session vocabulary where it fits
- allow a connector or bridge to adapt between the systems
- avoid promising strict wire compatibility until the local API exists and the gap analysis is complete

## Phase 0 Audit Worksheet

The first audit should classify each relevant `~/work/agent` surface into one of:

- `direct fit`
  - `codexw` already has an equivalent semantic surface and only needs API exposure
- `adapter fit`
  - the concept fits, but needs a bridge or reshaping layer
- `out of scope`
  - not a sensible first target for `codexw`

### Broker Surfaces

| `~/work/agent` surface | Initial classification | Why |
| --- | --- | --- |
| `GET /v1/agents/{agent_id}/proxy/...` | adapter fit | `codexw` does not yet expose a local HTTP API, but a connector could map future local endpoints into this broker path |
| `GET /v1/agents/{agent_id}/proxy_sse/...` | adapter fit | `codexw` can plausibly expose SSE event streams, but does not have a public stream API yet |
| `GET /v1/events` | adapter fit | the broker event stream concept fits well, but `codexw` first needs its own stable event vocabulary |
| outbound `wss://.../v1/agent/connect` | adapter fit | likely phase-2+ unless `codexw` adopts direct broker connectivity |

### Session And Client Surfaces

| `~/work/agent` surface | Initial classification | Why |
| --- | --- | --- |
| `POST /api/v1/session/new` | direct fit | `codexw` already has explicit thread/session lifecycle concepts and can define a local wrapper session handle |
| `POST /api/v1/session/client_event` | adapter fit | the collaboration idea fits, but `codexw` does not yet have a public client-event ingest model |
| `GET /api/v1/session/scene` | out of scope | `codexw` currently has no durable scene/entity model comparable to `agentd` |
| `POST /api/v1/session/scene/apply` | out of scope | same reason; not a first-phase remote-control requirement |

### Run And Event Surfaces

| `~/work/agent` surface | Initial classification | Why |
| --- | --- | --- |
| `POST /api/v1/run` | adapter fit | `codexw` can expose turn-start semantics, but its runtime is thread/turn-based rather than an `agentd` run engine |
| run-event envelopes | adapter fit | item/turn/status/orchestration events already exist internally, but need a public envelope contract |
| artifact signaling | adapter fit | `codexw` has transcript and attachment semantics but not a standalone artifact API yet |

### `codexw`-Specific High-Value Surfaces That `agent` Does Not Define Directly

These should not be forced into the `agent` shape blindly:

- orchestration worker views
- dependency graph views
- wrapper-owned background shell jobs
- reusable service capability registry
- `:ps`-style service mutation controls

Those are likely to require either:

- new `codexw`-specific endpoints
- or a thin compatibility vocabulary layered on top of the more generic `agent` event model

## Concrete Phase 0 Deliverables

The first investigation pass should produce these artifacts:

1. `endpoint-audit.md`
   - now tracked in [docs/codexw-broker-endpoint-audit.md](docs/codexw-broker-endpoint-audit.md)

2. `event-envelope-sketch.md`
   - now tracked in [docs/codexw-broker-event-envelope.md](docs/codexw-broker-event-envelope.md)

3. `session-identity-note.md`
   - now tracked in [docs/codexw-broker-session-identity.md](docs/codexw-broker-session-identity.md)

4. `connector-decision.md`
   - now tracked in [docs/codexw-broker-connector-decision.md](docs/codexw-broker-connector-decision.md)

## Session Model Questions

These questions must be decided before implementation:

1. Is a remote session just a view over one local thread, or a separate wrapper-side collaboration context?
2. Can multiple clients attach to the same active thread concurrently?
3. If yes, which actor is authoritative for interrupt, steer, or shell mutation actions?
4. How are local resume commands and remote session identifiers related?

## Security Questions

These need explicit design before any implementation work:

- whether the first remote API binds only to loopback
- whether browser access is supported in phase 1
- whether auth is bearer-token only or broker-shaped from the start
- whether outbound broker identity should reuse the `agent` project’s mTLS/client-auth assumptions

## Event Model Questions

The remote event stream should prefer stable semantic events such as:

- session attached
- turn started
- turn completed
- transcript item appended
- status updated
- orchestration changed
- background shell changed
- service capability changed

It should not expose terminal-only concerns such as wrapped prompt layout or ANSI block formatting as public protocol.

## Phased Plan

### Phase 0: Compatibility Audit

- compare `codexw` state/events against `~/work/agent` protocol and client expectations
- list exact mismatches instead of assuming compatibility
- write a surface inventory:
  - local-only
  - remotely exposable in phase 1
  - exposed later only with architectural changes

### Phase 1: Local API Design

- define a local HTTP/SSE API for `codexw`
- decide auth and session identity
- specify event envelopes
- decide event ordering and replay rules for reconnecting clients

### Phase 2: Prototype Connector

- adapt the local API to the broker relay model
- test remote terminal and browser-driven operation
- evaluate whether the `~/work/agent` C framework can consume the same event/session contract without wrapper-specific hacks

### Phase 3: Decide Native Broker Support

- after real connector experience, choose whether direct broker connectivity belongs in `codexw`

## TODOs

- Audit `codexw` state and event surfaces against:
  - `/Users/zongbaolu/work/agent/DESIGN.md`
  - `/Users/zongbaolu/work/agent/broker/README.md`
  - `/Users/zongbaolu/work/agent/docs/PROTOCOL.md`
  - `/Users/zongbaolu/work/agent/docs/CLIENT.md`
- Write a first `codexw` local API sketch covering:
  - now tracked in [docs/codexw-local-api-sketch.md](docs/codexw-local-api-sketch.md)
- Decide whether the first implementation target is:
  - now tracked in [docs/codexw-broker-connector-decision.md](docs/codexw-broker-connector-decision.md)
- Define the minimal compatibility target:
  - now tracked in [docs/codexw-broker-compatibility-target.md](docs/codexw-broker-compatibility-target.md)
- Evaluate whether the user’s C agent framework can share:
  - now tracked in [docs/codexw-broker-shared-assumptions.md](docs/codexw-broker-shared-assumptions.md)

## Current Status

This is now an explicit tracked design area, not an informal idea.

The next high-leverage step is no longer broad design exploration. The
remaining work is now implementation-facing:

- route ownership and delivery order, now captured in
  [docs/codexw-local-api-route-matrix.md](codexw-local-api-route-matrix.md)
- semantic event publication strategy, now captured in
  [docs/codexw-local-api-event-sourcing.md](codexw-local-api-event-sourcing.md)
- a real implementation spike for the loopback local API, now framed in
  [docs/codexw-local-api-implementation-plan.md](docs/codexw-local-api-implementation-plan.md)
- a connector prototype after that, now framed in
  [docs/codexw-broker-connector-prototype-plan.md](docs/codexw-broker-connector-prototype-plan.md)

The local API spike has now started with a minimal loopback skeleton:

- disabled-by-default listener
- optional bearer token
- `GET /healthz`
- `GET /api/v1/session`
- `GET /api/v1/session/{session_id}`
- `GET /api/v1/session/{session_id}/transcript`
- `POST /api/v1/turn/start`
- `POST /api/v1/turn/interrupt`
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
- `POST /api/v1/session/{session_id}/services/{job_ref}/provide`
- `POST /api/v1/session/{session_id}/services/{job_ref}/depend`
- `POST /api/v1/session/{session_id}/services/{job_ref}/contract`
- `POST /api/v1/session/{session_id}/services/{job_ref}/relabel`
- `POST /api/v1/session/{session_id}/services/{job_ref}/attach`
- `POST /api/v1/session/{session_id}/services/{job_ref}/wait`
- `POST /api/v1/session/{session_id}/services/{job_ref}/run`
- internal command queue from the HTTP listener into the main runtime loop

That semantic event-stream milestone is now partially landed:

- `GET /api/v1/session/{session_id}/events` exists in the loopback local API
- replay works through `Last-Event-ID`
- the current stream emits `session.updated`, `turn.updated`,
  `orchestration.updated`, `workers.updated`, `capabilities.updated`, and
  `transcript.updated`

The next concrete code step is now session create/attach semantics plus
connector-facing API coverage, not more route discovery. Structured service
interaction payloads are already in place for `attach`, `wait`, and `run`.
