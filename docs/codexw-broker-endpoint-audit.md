# codexw Broker Endpoint Audit

This document is the concrete Phase 0 audit worksheet referenced from
`docs/codexw-broker-connectivity.md`.

Its purpose is to map relevant `~/work/agent` broker/client surfaces to current
`codexw` capabilities and classify each surface as:

- `direct fit`
- `adapter fit`
- `out of scope`

## Classification Meanings

- `direct fit`
  - `codexw` already has equivalent local semantics and mostly needs an API wrapper
- `adapter fit`
  - the concept is valid for `codexw`, but the shapes differ enough that a bridge or connector layer is expected
- `out of scope`
  - not a sensible first-phase target for `codexw`

## Broker Surfaces

| Source surface | Classification | `codexw` owner/source | Notes |
| --- | --- | --- | --- |
| `GET /v1/agents/{agent_id}/proxy/...` | `adapter fit` | future local API layer | Useful once `codexw` exposes local HTTP endpoints; likely served through a connector first |
| `GET /v1/agents/{agent_id}/proxy_sse/...` | `adapter fit` | future SSE/event layer | Fits the intended transcript/status/orchestration streams, but no public local stream exists yet |
| `GET /v1/events` | `adapter fit` | future broker connector/event bridge | Broker-level event stream is plausible, but `codexw` needs its own stable event vocabulary first |
| outbound `GET /v1/agent/connect` websocket | `adapter fit` | future connector or direct broker mode | Probably phase 2 or 3, not the first implementation |

## Session And Client Surfaces

| Source surface | Classification | `codexw` owner/source | Notes |
| --- | --- | --- | --- |
| `POST /api/v1/session/new` | `direct fit` | `state.rs`, thread/session lifecycle | `codexw` already has explicit thread/session concepts and can mint wrapper session handles |
| `POST /api/v1/session/client_event` | `adapter fit` | future collaboration/event-ingest layer | The concept fits, but `codexw` does not yet expose client event ingestion |
| `GET /api/v1/session/scene` | `out of scope` | none | No durable scene/entity model in `codexw` today |
| `POST /api/v1/session/scene/apply` | `out of scope` | none | Same reason; not needed for first remote control |

## Run And Event Surfaces

| Source surface | Classification | `codexw` owner/source | Notes |
| --- | --- | --- | --- |
| `POST /api/v1/run` | `adapter fit` | request builders + turn lifecycle | `codexw` is thread/turn-centric, not `agentd` run-centric, but a remote turn API is clearly feasible |
| run-event envelopes | `adapter fit` | item/turn/status/orchestration state | Internal semantic events exist, but a stable public envelope still needs definition |
| artifact signaling | `adapter fit` | transcript/item surfaces + attachment state | Likely useful later, but not required for the first status/control API |

## `codexw`-Specific Surfaces

These are high-value `codexw` capabilities that do not map 1:1 to the current
`~/work/agent` broker/client docs and therefore should not be forced into a bad
compatibility shape:

| `codexw` surface | Likely remote shape | Notes |
| --- | --- | --- |
| orchestration worker views | `GET /api/v1/orchestration/workers` | Existing local abstraction is already structured and useful |
| dependency graph views | `GET /api/v1/orchestration/dependencies` | Better exposed directly than hidden inside a generic event stream |
| wrapper-owned background shells | `GET/POST /api/v1/shells/*` | One of the strongest differentiators versus plain app-server clients |
| reusable service capability registry | `GET /api/v1/capabilities` | Valuable for mobile/WebUI attachment and orchestration |
| service mutation controls (`provide`, `depend`, `contract`, `relabel`) | `POST /api/v1/services/{job}/...` | Probably remains `codexw`-specific even if broker-compatible transport is adopted |

## Recommended First Audit Outputs

When this worksheet is revisited during implementation planning, each row should
gain:

- concrete request/response candidates
- auth expectations
- ownership inside `codexw`
- replay/reconnect semantics
- whether it belongs in:
  - loopback-only local API
  - local API plus connector
  - or direct broker mode
