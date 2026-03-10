# codexw Broker Connectivity Design

## Objective

Define how `codexw` could evolve from a local terminal wrapper into a remotely reachable Codex runtime that can be driven through a broker by multiple client types such as:

- a mobile app
- a browser UI
- a remote terminal
- other automation clients

This document started as an investigation and design-planning artifact.
It now serves as the high-level broker/local-API overview and index into the
more specific design, implementation, proof, and support-policy documents.

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
- [codexw-broker-client-fixture.md](codexw-broker-client-fixture.md)
- [codexw-broker-adapter-status.md](codexw-broker-adapter-status.md)
- [codexw-broker-adapter-promotion.md](codexw-broker-adapter-promotion.md)
- [codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md)

The current implementation is now past pure route sketches:

For the concise implementation/proof snapshot, see
[codexw-broker-adapter-status.md](codexw-broker-adapter-status.md).

For the current recommendation on whether that proof is already sufficient to
promote the stack beyond prototype framing, see
[codexw-broker-promotion-recommendation.md](codexw-broker-promotion-recommendation.md).
For the operational meaning of that recommended support level, see
[codexw-broker-support-policy.md](codexw-broker-support-policy.md).

- `codexw` has a working loopback HTTP/SSE local API
- the tracked connector prototype supports both raw `/proxy` passthrough and a
  broker-style alias surface for session, turn, transcript,
  orchestration, event, shell, service, and capability flows, including
  focused service-detail and capability-detail inspection routes
- the repo also includes a small broker-style client fixture script that drives
  the connector outside the test suite for manual and prototype remote-control
  flows
- that fixture is also exercised by a process-level connector smoke test, so it
  is no longer just a convenience script but a verified consumer-side artifact
- the fixture-backed smoke coverage now proves not only session create / turn /
  transcript, but also shell start, service attach / wait / run, and
  structured lease-conflict propagation through the broker-style alias surface
- the fixture-backed smoke coverage also now proves focused service-detail and
  capability-detail reads plus event-stream resume through `Last-Event-ID`,
  using the standalone broker-style client fixture rather than only raw test
  harness helpers
- the standalone broker-style client fixture is also now process-level verified
  for service mutation flows (`provide` / `depend` / `contract` / `relabel`),
  not only read/attach/run paths
- the standalone broker-style client fixture is also now process-level verified
  for attachment lease lifecycle flows (`renew` / `release`) with session
  snapshot confirmation after each transition
- the standalone broker-style client fixture is also now process-level verified
  for `client-event` publish plus event-stream replay/resume, so the
  collaboration/event-ingest path is no longer just a local-API primitive
- the standalone broker-style client fixture is also now process-level verified
  for one combined leased workflow that mixes initial event consumption,
  lease-owned service mutation, focused service-detail inspection, and resumed
  `Last-Event-ID` event consumption through the broker-style alias surface
- the standalone broker-style client fixture is also now process-level verified
  for one adversarial multi-client workflow with owner lease control, observer
  event consumption, structured rival conflict propagation, owner mutation
  recovery, and resumed `Last-Event-ID` event consumption
- the standalone broker-style client fixture is also now process-level verified
  for a lease-handoff workflow with two independent observers, explicit owner
  release, rival lease takeover, rival mutation success, and dual resumed
  `Last-Event-ID` event consumers after the handoff
- the standalone broker-style client fixture is also now process-level verified
  for a repeated role-reversal workflow where the original owner releases,
  the rival takes over and mutates, the former owner is then blocked, the rival
  releases, and the owner retakes the lease before a resumed observer sees the
  post-retake capability event
- the standalone broker-style client fixture is also now process-level verified
  for session listing and turn interrupt flows, session attach plus
  orchestration status / workers / dependencies inspection, and shell list /
  detail / send / poll / terminate control paths
- the broker-style alias surface now also includes attachment lease lifecycle
  routes for renew/release, so remote clients do not need raw `/proxy/...`
  access for normal lease management
- the repo now also has a process-level connector smoke test proving that a
  remote client can drive the connector over TCP while a fake local API is
  served behind it, including broker-style session create and attachment lease
  lifecycle aliases, a realistic create -> turn -> transcript ->
  orchestration inspection workflow, a realistic create -> shell ->
  service attach/wait/run -> capability inspection workflow, a realistic
  service-mutation workflow (`provide` / `depend` / `contract` / `relabel`),
  alias-based event-stream resume behavior via `Last-Event-ID`, and structured
  `attachment_conflict` propagation with lease-holder details on conflicting
  turn/service mutations through the broker-style alias surface
- that smoke coverage now also proves a single broker-style workflow can mix
  service attach / wait / run control with SSE resume, which is the closest
  current approximation of a real remote client following live state while
  interacting with reusable services
- and it now also proves a single broker-style workflow can mix lease-owned
  capability mutation, focused service/capability detail aliases, and resumed
  SSE capability-state observation after the mutation path

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
| local prompt/editor state | terminal-only editor modules | no public API in the current supported adapter scope | Treat as local-only presentation detail |

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

## Event Model Sketch

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

For the current supported adapter scope, every event should satisfy two
constraints:

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

The current adapter event envelope should stay simple:

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
| Loopback-only local API | localhost only | optional local bearer token | limited, same-host only | not directly | safest initial rollout path |
| Local API + connector | localhost API, remote via connector | local token + connector auth | yes, via connector/broker | yes | best compatibility path |
| Direct broker in `codexw` | remote reachable | broker token and/or mTLS | yes | yes | later phase only |

Recommended initial choice:

- initial rollout: loopback-only local API, optional bearer token
- current supported adapter shape: local API plus connector
- deferred expansion: evaluate whether direct broker connectivity is worth the complexity

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

## Historical Audit Worksheet

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
| `GET /v1/agents/{agent_id}/proxy/...` | adapter fit | implemented through the current connector prototype against the loopback local HTTP API; still an adapter layer rather than a direct native broker surface |
| `GET /v1/agents/{agent_id}/proxy_sse/...` | adapter fit | implemented through the current connector prototype against the loopback local SSE API; replay and `Last-Event-ID` behavior are already process-level proven |
| `GET /v1/events` | adapter fit | the broker event stream concept fits well, but `codexw` first needs its own stable event vocabulary |
| outbound `wss://.../v1/agent/connect` | adapter fit | likely a later transport step unless `codexw` adopts direct broker connectivity |

### Session And Client Surfaces

| `~/work/agent` surface | Initial classification | Why |
| --- | --- | --- |
| `POST /api/v1/session/new` | direct fit | `codexw` already has explicit thread/session lifecycle concepts and can define a local wrapper session handle |
| `POST /api/v1/session/client_event` | direct fit | implemented in the local API and exposed through the broker-style fixture client | The collaboration idea now has a public local-API ingest route plus connector/fixture coverage |
| `GET /api/v1/session/scene` | out of scope | `codexw` currently has no durable scene/entity model comparable to `agentd` |
| `POST /api/v1/session/scene/apply` | out of scope | same reason; not part of the current supported remote-control scope |

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

## Concrete Historical Audit Deliverables

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
- whether browser access is supported in the current adapter scope
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

## Historical Phased Plan

### Historical Step 0: Compatibility Audit

- compare `codexw` state/events against `~/work/agent` protocol and client expectations
- list exact mismatches instead of assuming compatibility
- write a surface inventory:
  - local-only
  - remotely exposable in the initial supported adapter scope
  - exposed later only with architectural changes

### Historical Step 1: Local API Design

- define a local HTTP/SSE API for `codexw`
- decide auth and session identity
- specify event envelopes
- decide event ordering and replay rules for reconnecting clients

### Historical Step 2: Prototype Connector

- adapt the local API to the broker relay model
- test remote terminal and browser-driven operation
- evaluate whether the `~/work/agent` C framework can consume the same event/session contract without wrapper-specific hacks

### Historical Step 3: Decide Native Broker Support

Those labels are retained here as implementation history and design sequencing,
not as the current project state. The local API and connector prototype
described in Historical Steps 1 and 2 now exist; the remaining open question is whether
any further promotion or transport expansion beyond the supported experimental
adapter is warranted.

- after real connector experience, choose whether direct broker connectivity belongs in `codexw`

## Historical Design Checklist

The earlier open-ended design TODOs from this document have now been broken
out into tracked artifacts. The relevant current sources of truth are:

- endpoint compatibility audit:
  [docs/codexw-broker-endpoint-audit.md](docs/codexw-broker-endpoint-audit.md)
- local API route and implementation planning:
  [docs/codexw-local-api-route-matrix.md](docs/codexw-local-api-route-matrix.md),
  [docs/codexw-local-api-implementation-plan.md](docs/codexw-local-api-implementation-plan.md)
- connector architecture and mapping:
  [docs/codexw-broker-connector-decision.md](docs/codexw-broker-connector-decision.md),
  [docs/codexw-broker-connector-mapping.md](docs/codexw-broker-connector-mapping.md),
  [docs/codexw-broker-connector-prototype-plan.md](docs/codexw-broker-connector-prototype-plan.md)
- frozen adapter contract, client policy, and explicit unsupported boundary:
  [docs/codexw-broker-adapter-contract.md](docs/codexw-broker-adapter-contract.md),
  [docs/codexw-broker-client-policy.md](docs/codexw-broker-client-policy.md),
  [docs/codexw-broker-out-of-scope.md](docs/codexw-broker-out-of-scope.md)
- current implementation/proof/support state:
  [docs/codexw-broker-adapter-status.md](docs/codexw-broker-adapter-status.md),
  [docs/codexw-broker-proof-matrix.md](docs/codexw-broker-proof-matrix.md),
  [docs/codexw-broker-support-policy.md](docs/codexw-broker-support-policy.md)
- promotion criteria and recommendation:
  [docs/codexw-broker-adapter-promotion.md](docs/codexw-broker-adapter-promotion.md),
  [docs/codexw-broker-promotion-recommendation.md](docs/codexw-broker-promotion-recommendation.md)

## Current Status

This is now an explicit tracked design and implementation area, not an
informal idea or a pre-implementation hypothesis.

The broad design exploration phase is largely complete. The remaining work is
mostly implementation hardening, support-level judgment, and connector/client
proof expansion:

- route ownership and delivery order, now captured in
  [docs/codexw-local-api-route-matrix.md](codexw-local-api-route-matrix.md)
- semantic event publication strategy, now captured in
  [docs/codexw-local-api-event-sourcing.md](codexw-local-api-event-sourcing.md)
- a real implementation spike for the loopback local API, now framed in
  [docs/codexw-local-api-implementation-plan.md](docs/codexw-local-api-implementation-plan.md)
- a connector prototype after that, now framed in
  [docs/codexw-broker-connector-prototype-plan.md](docs/codexw-broker-connector-prototype-plan.md)
- the connector/client policy contract, now captured in
  [docs/codexw-broker-client-policy.md](docs/codexw-broker-client-policy.md)
- the frozen broker-facing adapter contract, now captured in
  [docs/codexw-broker-adapter-contract.md](docs/codexw-broker-adapter-contract.md)
- the explicit broker/client boundary, now captured in
  [docs/codexw-broker-out-of-scope.md](docs/codexw-broker-out-of-scope.md)
- the criteria for promoting the current broker/local-API stack into a
  supported adapter layer, now captured in
  [docs/codexw-broker-adapter-promotion.md](docs/codexw-broker-adapter-promotion.md)
- the mapping from those criteria to current process-level proof, now captured
  in [docs/codexw-broker-proof-matrix.md](docs/codexw-broker-proof-matrix.md)
- the current promotion recommendation, now captured in
  [docs/codexw-broker-promotion-recommendation.md](docs/codexw-broker-promotion-recommendation.md)
- the operational meaning of the current support level, now captured in
  [docs/codexw-broker-support-policy.md](docs/codexw-broker-support-policy.md)

The local API spike has now started with a minimal loopback skeleton:

- disabled-by-default listener
- optional bearer token
- `GET /healthz`
- `GET /api/v1/session`
- `GET /api/v1/session/{session_id}`
- `GET /api/v1/session/{session_id}/transcript`
- `POST /api/v1/turn/start`
- `POST /api/v1/turn/interrupt`
- `POST /api/v1/session/{session_id}/turn/start`
- `POST /api/v1/session/{session_id}/turn/interrupt`
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
- explicit process-scoped session lifecycle routes now exist:
  - `POST /api/v1/session/new`
  - `POST /api/v1/session/attach`
- explicit process-scoped attachment lease routes now exist:
  - `POST /api/v1/session/{session_id}/attachment/renew`
  - `POST /api/v1/session/{session_id}/attachment/release`
- session snapshots and `session.updated` events now carry explicit
  process-scoped `session` and `attachment` metadata
- that attachment metadata now includes:
  - `client_id`
  - `lease_seconds`
  - `lease_expires_at_ms`
  - `lease_active`
- turn control is available in both global and session-scoped route forms, so a
  connector can remain inside one `/api/v1/session/{session_id}` namespace once
  attached
- mutating local-API routes now honor the active attachment lease:
  - turn start / interrupt
  - shell start / send / terminate
  - service provide / depend / contract / relabel
  - service attach / wait / run
  - anonymous or mismatched `client_id` calls now receive `409 attachment_conflict`
- local-API failures now use a stable machine-usable error contract:
  - `error.status`
  - `error.code`
  - `error.message`
  - `error.retryable`
  - `error.details`
- `attachment_conflict` now includes structured lease-holder context in
  `error.details.current_attachment`, so a connector can make policy decisions
  without scraping error prose

The next concrete code step is now connector-facing API coverage above the
implemented session, transcript, orchestration, shell, and service surface, not
more route discovery. Structured service interaction payloads are already in
place for `attach`, `wait`, and `run`, and explicit attach/lease semantics now
exist for the current process-scoped session contract. The next gap is
connector-specific client/lease policy above that single-process model.

That connector-facing work has now started with a first standalone prototype:

- binary: `cargo run --bin codexw-connector-prototype -- ...`
- broker-facing HTTP prefix:
  - `/v1/agents/{agent_id}/proxy/...`
- broker-facing SSE prefix:
  - `/v1/agents/{agent_id}/proxy_sse/...`
- current prototype behavior:
  - forwards an allowlisted subset of local-API routes through a generic proxy prefix
  - bridges SSE from the local API
  - wraps SSE payloads with `source` and `broker` metadata
  - supports optional incoming connector bearer auth
  - supports optional outgoing local-API bearer auth
  - supports connector-side `X-Codexw-Client-Id` and `X-Codexw-Lease-Seconds`
    header injection into supported mutating local-API JSON routes
  - rejects proxy requests outside the approved session/turn/orchestration/shell/service route families
