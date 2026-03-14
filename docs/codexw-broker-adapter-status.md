# codexw Broker Adapter Status

This document is the concise implementation-status companion to the broader
design set:

- [../TODOS.md](../TODOS.md)
- [codexw-broker-connectivity.md](codexw-broker-connectivity.md)
- [codexw-local-api-implementation-plan.md](codexw-local-api-implementation-plan.md)
- [codexw-broker-connector-adapter-plan.md](codexw-broker-connector-adapter-plan.md)
- [codexw-broker-client-fixture.md](codexw-broker-client-fixture.md)
- [codexw-broker-adapter-promotion.md](codexw-broker-adapter-promotion.md)
- [codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md)
- [codexw-broker-support-policy.md](codexw-broker-support-policy.md)
- [codexw-broker-promotion-recommendation.md](codexw-broker-promotion-recommendation.md)
- [codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md)
- [codexw-broker-hardening-catalog.md](codexw-broker-hardening-catalog.md)
- [codexw-support-claim-checklist.md](codexw-support-claim-checklist.md)

Its goal is simple:

- record what is actually implemented now
- distinguish route availability from process-level proof
- make the remaining gaps explicit

## Current Status

`codexw` now has a real, working remote-control broker/local-API adapter stack:

1. a loopback local HTTP/SSE API in the main `codexw` runtime
2. a standalone connector adapter that exposes broker-style alias routes
3. standalone broker-style client fixtures in Python and Node
4. process-level smoke coverage for the connector and the fixture

This is no longer just a design exploration.

## Current Architectural Posture

The adapter should now be read against the broader broker-client requirement in
[codexw-broker-client-architecture.md](codexw-broker-client-architecture.md):

- broker-backed clients such as app/WebUI are part of the intended architecture
- host shell examination is part of that broker-facing client surface
- the verified shell/service/orchestration/transcript routes are therefore not
  incidental plumbing; they are the current remote host-inspection foundation

What is still incomplete is not whether broker-facing clients can touch host
state at all. The remaining gap is whether the current broker-visible shell,
transcript, and result surfaces are sufficient for rich client UX without
direct terminal access, especially for artifact-heavy inspection workflows.

## Implemented Local API Surface

The current local API includes:

- health:
  - `GET /healthz`
- session lifecycle:
  - `POST /api/v1/session/new`
  - `POST /api/v1/session/attach`
  - `GET /api/v1/session`
  - `GET /api/v1/session/{session_id}`
  - `POST /api/v1/session/{session_id}/attachment/renew`
  - `POST /api/v1/session/{session_id}/attachment/release`
- turn control:
  - `POST /api/v1/turn/start`
  - `POST /api/v1/turn/interrupt`
  - `POST /api/v1/session/{session_id}/turn/start`
  - `POST /api/v1/session/{session_id}/turn/interrupt`
- transcript and events:
  - `GET /api/v1/session/{session_id}/transcript`
  - `GET /api/v1/session/{session_id}/events`
  - `POST /api/v1/session/client_event`
  - `POST /api/v1/session/{session_id}/client_event`
- orchestration:
  - `GET /api/v1/session/{session_id}/orchestration/status`
  - `GET /api/v1/session/{session_id}/orchestration/workers`
  - `GET /api/v1/session/{session_id}/orchestration/dependencies`
- shells:
  - `GET /api/v1/session/{session_id}/shells`
  - `GET /api/v1/session/{session_id}/shells/{job_ref}`
  - `POST /api/v1/session/{session_id}/shells/start`
  - `POST /api/v1/session/{session_id}/shells/{job_ref}/poll`
  - `POST /api/v1/session/{session_id}/shells/{job_ref}/send`
  - `POST /api/v1/session/{session_id}/shells/{job_ref}/terminate`
- services and capabilities:
  - `GET /api/v1/session/{session_id}/services`
  - `GET /api/v1/session/{session_id}/services/{job_ref}`
  - `GET /api/v1/session/{session_id}/capabilities`
  - `GET /api/v1/session/{session_id}/capabilities/{capability}`
  - `POST /api/v1/session/{session_id}/services/{job_ref}/provide`
  - `POST /api/v1/session/{session_id}/services/{job_ref}/depend`
  - `POST /api/v1/session/{session_id}/services/{job_ref}/contract`
  - `POST /api/v1/session/{session_id}/services/{job_ref}/relabel`
  - `POST /api/v1/session/{session_id}/services/{job_ref}/attach`
  - `POST /api/v1/session/{session_id}/services/{job_ref}/wait`
  - `POST /api/v1/session/{session_id}/services/{job_ref}/run`

## Implemented Connector Surface

The standalone connector adapter now supports both raw passthrough and
broker-style alias routes.

Implemented broker-style aliases include:

- session create/list/inspect/attach
- attachment renew/release
- turn start/interrupt
- transcript
- session event SSE
- orchestration status/workers/dependencies
- shell list/start/detail/poll/send/terminate
- service list/detail/attach/wait/run
- service mutation:
  - `provide`
  - `depend`
  - `contract`
  - `relabel`
- capability list/detail

The connector also supports:

- `X-Codexw-Client-Id`
- `X-Codexw-Lease-Seconds`

for supported mutating JSON routes, with header-to-body projection when the
outgoing local-API body does not already provide those fields.
That mutating-route projection policy now shares the same local route-shape
classifier as the connector allowlist instead of being maintained as a second
parallel match list.

## Process-Level Verified Coverage

The following are not just implemented; they are exercised end to end against
the real connector binary:

- session create / attach / list / inspect
- attachment renew / release
- turn start / interrupt
- transcript fetch
- orchestration status / workers / dependencies
- shell list / start / detail / poll / send / terminate
- service list / detail / attach / wait / run
- service mutation:
  - `provide`
  - `depend`
  - `contract`
  - `relabel`
- capability list / detail
- explicit rejection of unsupported broker-style aliases and out-of-allowlist
  raw proxy / raw proxy SSE routes
- explicit `method_not_allowed` rejection for non-`GET` broker-style and raw
  proxy SSE event routes
- method-sensitive alias resolution for read-only broker routes such as session
  inspect, transcript, orchestration, services, and capabilities
- method-sensitive alias resolution for write-only broker routes such as
  attach, renew/release, turn start, client-events, shell/service actions, and
  the `/sessions` collection root
- explicit rejection of malformed percent-encoded broker alias path segments
- broker-style SSE consumption
- broker-style SSE resume through `Last-Event-ID`
- structured lease-conflict propagation
- structured connector-local validation failures for malformed injected request
  bodies and malformed client/lease headers
- preserved local field-level validation failures through the connector
- timeout-tolerant HTTP request parsing for fragmented request headers/bodies in
  both the local API server and the connector adapter, with shared parser logic
  so the same reliability fix is not maintained in two divergent copies
- focused service-detail and capability-detail reads after mutation workflows
- client-event publish and replay/resume
- explicit route-by-route local-API session lifecycle assertions
- explicit route-by-route connector allowlist and broker-alias mapping,
  including an invariant that every claimed broker-style alias resolves only to
  a local target the connector allowlist still permits
- percent-decoded `job_ref`/capability path handling across both detail reads
  and mutating shell/service alias routes
- shared route-shape classification for connector allowlist checks and
  client/lease header-to-body injection eligibility, including the supported
  raw proxy turn-control routes plus top-level raw proxy session attach and
  `client_event`
- thin raw-proxy pass-through proof for canonical session inspect, transcript,
  session list, orchestration, shell, service, and capability reads, plus
  `Last-Event-ID` SSE replay routes; raw proxy canonical detail reads preserve
  encoded path segments instead of applying alias-level decode rules
- one combined leased workflow that mixes:
  - initial event consumption
  - lease-owned service mutation
  - focused service-detail inspection
  - resumed `Last-Event-ID` event consumption
- one adversarial multi-client workflow that mixes:
  - owner-created leased session
  - observer event consumption
  - conflicting rival mutation with structured lease conflict details
  - owner mutation recovery
  - observer `Last-Event-ID` resume
- one observer-readable contention workflow that mixes:
  - owner-created leased session
  - observer session/orchestration/shell/service/capability reads
  - conflicting rival mutation with structured lease conflict details
  - observer reads remaining available after the conflict
- one anonymous observer/rival workflow that mixes:
  - owner-created leased session
  - anonymous event/session/orchestration/service/capability reads
  - conflicting anonymous mutation with structured `attachment_conflict`
  - anonymous reads remaining available after the conflict
- one lease-handoff workflow that mixes:
  - owner-created leased session
  - two independent observers consuming the same initial event state
  - conflicting rival mutation before release
  - explicit owner release
  - rival lease takeover and successful mutation
  - dual-observer `Last-Event-ID` resume after the handoff
- one repeated role-reversal workflow that mixes:
  - owner-created leased session
  - observer event consumption
  - rival conflict before release
  - owner release
  - rival takeover and successful mutation
  - former-owner conflict while rival holds the lease
  - rival release
  - owner retake and successful mutation
  - observer `Last-Event-ID` resume after the second role change
- one client-event lease-handoff workflow that mixes:
  - owner-created leased session
  - observer initial event consumption
  - rival `client-event` conflict before release
  - explicit owner release
  - rival lease acquisition and successful `client-event` publish
  - observer `Last-Event-ID` resume after the handoff

This process-level proof comes from two complementary surfaces:

- Rust connector smoke tests under `wrapper/tests/connector_prototype_smoke/*`
- the standalone broker client fixtures in
  `scripts/codexw_broker_client.py` and
  `scripts/codexw_broker_client_node.mjs`, both exercised by process-level smoke

For a promotion-oriented mapping from those workflows back to route, error,
event, policy, and unsupported-boundary claims, see
[codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md).

## What Is Stable Enough To Build Against

Within the current supported experimental adapter scope, the following are now
strong enough to build against:

- the local API route family and structured error envelope
- the connector alias surface for session/turn/orchestration/shell/service work
- lease-aware mutation behavior and conflict reporting
- SSE resume semantics for remote clients
- the standalone broker fixtures as reference clients

These are still explicitly experimental surfaces, but they are no longer
speculative.

The same is now true for the current validation/error surface: malformed
connector-side injected request bodies and malformed client/lease headers return
structured `validation_error` responses, and structured local field-validation
errors are preserved instead of being collapsed into generic transport failures.

The same is also now true for the request-ingest path itself: transient
socket-level read fragmentation no longer causes the local API server or
connector adapter to reject otherwise valid requests as generic bad requests
after one timeout window. Both now share the same bounded timeout-tolerant HTTP
request reader.

## What Remains Explicitly Limited

The current stack is still intentionally limited:

- one process-scoped local runtime session model
- no full broker deployment/auth implementation
- no multi-runtime or multi-daemon coordination
- no production SDK
- no compatibility promise with every `agentd` surface
- no browser/mobile UX layer in this repo yet

## Highest-Leverage Remaining Gaps

The biggest remaining gaps are now mostly judgment, maintenance, and optional
hardening above the already-proven route surface:

1. keep the supported experimental adapter claim coherent across README, status docs,
   proof docs, and future release-facing language
2. preserve the explicit unsupported boundary whenever the connector alias
   surface grows
3. treat additional churn, replay, and adversarial workflows as optional
   hardening tracked in
   [codexw-broker-hardening-catalog.md](codexw-broker-hardening-catalog.md),
   not as evidence that the adapter contract is still undefined

The remaining gaps are therefore no longer basic validation fidelity, missing
contract language, or missing multi-client proof for the currently claimed
surface. They are support follow-through and optional confidence hardening.

The unsupported boundary itself is now also process-level defended through the
connector smoke suite, including explicit rejection of out-of-scope broker-style
`scene` routes, unsupported global broker routes, unknown broker aliases, and
out-of-allowlist raw proxy paths.

## Recommended Next Work

If continuing on this track, the highest-leverage next tasks are:

1. use
   [codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md),
   [codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md), and
   [codexw-broker-promotion-recommendation.md](codexw-broker-promotion-recommendation.md)
   together to validate or challenge the current recommendation
2. keep the out-of-scope boundary explicit through
   [codexw-broker-out-of-scope.md](codexw-broker-out-of-scope.md) so adapter
   scope expansion does not drift into parity assumptions
3. keep optional churn, replay, and adversarial ideas in
   [codexw-broker-hardening-catalog.md](codexw-broker-hardening-catalog.md)
   unless they become active blockers because of a regression or contradiction
4. keep
   [codexw-broker-adapter-promotion.md](codexw-broker-adapter-promotion.md)
   as the explicit checklist for validating or challenging the current
   supported experimental adapter recommendation, together with
   [codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md)
