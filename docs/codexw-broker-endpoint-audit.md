# codexw Broker Endpoint Audit

This document is the concrete historical audit worksheet referenced from
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
| `GET /v1/agents/{agent_id}/proxy/...` | `adapter fit` | local API plus connector | `codexw` now exposes local HTTP endpoints, and the connector already proxies an allowlisted subset of them |
| `GET /v1/agents/{agent_id}/proxy_sse/...` | `adapter fit` | local SSE/event layer plus connector | Public local SSE exists today, and the connector already bridges allowlisted event streams with replay support |
| `GET /v1/events` | `adapter fit` | current connector plus future broker event bridge | Broker-level fan-in event streaming is still a future aggregation layer; current implementation is session-scoped event streaming carried by the existing connector |
| outbound `GET /v1/agent/connect` websocket | `adapter fit` | deferred direct broker mode or future connector transport | Still a deferred transport option rather than part of the current loopback HTTP/SSE plus connector design |

## Session And Client Surfaces

| Source surface | Classification | `codexw` owner/source | Notes |
| --- | --- | --- | --- |
| `POST /api/v1/session/new` | `direct fit` | `state.rs`, thread/session lifecycle | `codexw` already has explicit thread/session concepts and can mint wrapper session handles |
| `POST /api/v1/session/client_event` | `direct fit` | `local_api/events.rs`, `local_api/routes/client_events.rs` | Public local-API client-event ingestion now exists and can be consumed through the connector/fixture path |
| `GET /api/v1/session/scene` | `out of scope` | none | No durable scene/entity model in `codexw` today |
| `POST /api/v1/session/scene/apply` | `out of scope` | none | Same reason; not needed for first remote control |

## Run And Event Surfaces

| Source surface | Classification | `codexw` owner/source | Notes |
| --- | --- | --- | --- |
| `POST /api/v1/run` | `adapter fit` | request builders + turn lifecycle | `codexw` is thread/turn-centric, not `agentd` run-centric, but a remote turn API is clearly feasible |
| run-event envelopes | `adapter fit` | item/turn/status/orchestration state | Stable public semantic event envelopes now exist for the supported local API and connector surface, but they remain intentionally narrower than a generic broker-wide run stream |
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

When this worksheet is revisited for hardening or promotion decisions, each row should
continue to gain:

- concrete request/response candidates
- auth expectations
- ownership inside `codexw`
- replay/reconnect semantics
- whether it belongs in:
  - loopback-only local API
  - local API plus connector
  - or direct broker mode
