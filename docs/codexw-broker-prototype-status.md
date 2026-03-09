# codexw Broker Prototype Status

This document is the concise implementation-status companion to the broader
design set:

- [codexw-broker-connectivity.md](codexw-broker-connectivity.md)
- [codexw-local-api-implementation-plan.md](codexw-local-api-implementation-plan.md)
- [codexw-broker-connector-prototype-plan.md](codexw-broker-connector-prototype-plan.md)
- [codexw-broker-client-fixture.md](codexw-broker-client-fixture.md)

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
- broker-style SSE consumption
- broker-style SSE resume through `Last-Event-ID`
- structured lease-conflict propagation
- focused service-detail and capability-detail reads after mutation workflows
- client-event publish and replay/resume
- one combined leased workflow that mixes:
  - initial event consumption
  - lease-owned service mutation
  - focused service-detail inspection
  - resumed `Last-Event-ID` event consumption

This process-level proof comes from two complementary surfaces:

- Rust connector smoke tests under `wrapper/tests/connector_prototype_smoke/*`
- the standalone Python broker client fixture in
  `scripts/codexw_broker_client.py`, also exercised by process-level smoke

## What Is Stable Enough For Prototype Consumers

For prototype or lab use, the following are now strong enough to build against:

- the local API route family and structured error envelope
- the connector alias surface for session/turn/orchestration/shell/service work
- lease-aware mutation behavior and conflict reporting
- SSE resume semantics for remote clients
- the Python fixture as a reference client

These are still prototype surfaces, but they are no longer speculative.

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

1. explicit client-policy and attachment semantics beyond the current
   process-scoped lease model
2. connector behavior under more realistic long-lived multi-client workflows
3. a clearer statement of which broker/client surfaces are intentionally out of
   scope for `codexw`
4. eventual promotion from prototype connector to a more formal adapter layer

## Recommended Next Work

If continuing on this track, the highest-leverage next tasks are:

1. tighten the connector/client policy contract, especially around lease
   ownership and competing clients
2. add more adversarial multi-client workflows, especially overlapping leases
   and competing long-lived event consumers instead of only single-client
   sequential resume paths
3. keep the design docs aligned so this status file remains the concise source
   of truth while the other docs stay architectural
