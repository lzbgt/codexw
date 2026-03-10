# codexw Broker Prototype Status

This document is the concise implementation-status companion to the broader
design set:

- [codexw-broker-connectivity.md](codexw-broker-connectivity.md)
- [codexw-local-api-implementation-plan.md](codexw-local-api-implementation-plan.md)
- [codexw-broker-connector-prototype-plan.md](codexw-broker-connector-prototype-plan.md)
- [codexw-broker-client-fixture.md](codexw-broker-client-fixture.md)
- [codexw-broker-adapter-promotion.md](codexw-broker-adapter-promotion.md)
- [codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md)
- [codexw-broker-support-policy.md](codexw-broker-support-policy.md)
- [codexw-broker-promotion-recommendation.md](codexw-broker-promotion-recommendation.md)
- [codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md)

Its goal is simple:

- record what is actually implemented now
- distinguish route availability from process-level proof
- make the remaining gaps explicit

## Current Status

`codexw` now has a real, working remote-control prototype stack:

1. a loopback local HTTP/SSE API in the main `codexw` runtime
2. a standalone connector prototype that exposes broker-style alias routes
3. a standalone broker-style Python client fixture
4. process-level smoke coverage for the connector and the fixture

This is no longer just a design exploration.

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

The standalone connector prototype now supports both raw passthrough and
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
- broker-style SSE consumption
- broker-style SSE resume through `Last-Event-ID`
- structured lease-conflict propagation
- structured connector-local validation failures for malformed injected request
  bodies and malformed client/lease headers
- preserved local field-level validation failures through the connector
- focused service-detail and capability-detail reads after mutation workflows
- client-event publish and replay/resume
- explicit route-by-route local-API session lifecycle assertions
- explicit route-by-route connector allowlist and broker-alias mapping
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
- the standalone Python broker client fixture in
  `scripts/codexw_broker_client.py`, also exercised by process-level smoke

For a promotion-oriented mapping from those workflows back to route, error,
event, policy, and unsupported-boundary claims, see
[codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md).

## What Is Stable Enough For Prototype Consumers

For prototype or lab use, the following are now strong enough to build against:

- the local API route family and structured error envelope
- the connector alias surface for session/turn/orchestration/shell/service work
- lease-aware mutation behavior and conflict reporting
- SSE resume semantics for remote clients
- the Python fixture as a reference client

These are still prototype surfaces, but they are no longer speculative.

The same is now true for the current validation/error surface: malformed
connector-side injected request bodies and malformed client/lease headers return
structured `validation_error` responses, and structured local field-validation
errors are preserved instead of being collapsed into generic transport failures.

## What Is Still Prototype-Grade

The current stack is still intentionally limited:

- one process-scoped local runtime session model
- no full broker deployment/auth implementation
- no multi-runtime or multi-daemon coordination
- no production SDK
- no compatibility promise with every `agentd` surface
- no browser/mobile UX layer in this repo yet

## Highest-Leverage Remaining Gaps

The biggest remaining gaps are above the route layer, not below it:

1. explicit client-policy and attachment semantics are now stronger for named
   and anonymous callers, and are now frozen explicitly in
   [codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md);
   the remaining question is whether the current proof level is already enough
   to promote that contract
2. broader connector behavior under sustained multi-client contention beyond the
   now-covered observer-readable contention, conflict/recovery, explicit
   handoff, repeated role-reversal, and client-event handoff workflows
3. eventual promotion from prototype connector to a more formal adapter layer
   is still a deliberate decision point, but the current recommendation is now
   written down explicitly in
   [codexw-broker-promotion-recommendation.md](codexw-broker-promotion-recommendation.md),
   and the operational meaning of that support level is now frozen in
   [codexw-broker-support-policy.md](codexw-broker-support-policy.md)

The remaining gaps are therefore no longer basic validation fidelity or missing
contract language. They are promotion judgment and broader architectural
choices above the current route/error/event surface.

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
2. add more adversarial multi-client workflows, especially longer-lived lease
   churn and more complex observer/rival/owner permutations beyond the
   now-covered observer-readable contention, conflict/recovery, explicit
   handoff, repeated role-reversal, and client-event handoff paths
3. keep the out-of-scope boundary explicit through
   [codexw-broker-out-of-scope.md](codexw-broker-out-of-scope.md) so prototype
   expansion does not drift into parity assumptions
4. keep
   [codexw-broker-adapter-promotion.md](codexw-broker-adapter-promotion.md)
   as the explicit checklist for deciding whether the connector stays
   prototype-grade or becomes a supported adapter layer, together with
   [codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md)
