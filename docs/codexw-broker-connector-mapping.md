# codexw Broker Connector Mapping

This document describes the current mapping from the implemented `codexw`
local API to the broker-facing connector model inspired by `~/work/agent`.

It is not a wire-compatibility promise. It is the adapter mapping record for the
current connector-oriented architecture.

## Purpose

The connector exists to translate between:

- a `codexw`-native local daemon API
- a broker-oriented remote access model with:
  - proxied HTTP routes
  - SSE event streaming
  - deployment-aware agent routing

The connector should be a narrow translation layer, not a second runtime.

## Mapping Principles

1. `codexw` local API remains the canonical runtime contract
2. broker compatibility is achieved through explicit route and event translation
3. wrapper-native semantics are preserved in payloads when they matter
4. unknown or unsupported broker concepts should fail explicitly, not silently
   degrade

## Identity Mapping

| Connector concept | Local `codexw` source | Notes |
| --- | --- | --- |
| broker `agent_id` | deployment registration, not local thread id | one broker-visible agent may front one `codexw` daemon instance |
| broker deployment id | connector/deployment identity | should remain separate from `session_id` and `thread_id` |
| remote session handle | `session_id` | canonical remote-control handle |
| underlying conversation | `thread_id` | preserved in payloads and events when attached |
| client identity | `client_id` / attachment id | optional in the current adapter surface, useful for audit and lock rules |

## Route Mapping

### Session Lifecycle

| Broker-side intent | Local `codexw` route | Mapping class | Notes |
| --- | --- | --- | --- |
| create remote session | `POST /api/v1/session/new` | native | connector can proxy almost directly |
| attach to existing conversation | `POST /api/v1/session/attach` | native | preserves `session_id` + `thread_id` split |
| inspect session | `GET /api/v1/session/{session_id}` | native | response may be passed through with light envelope cleanup |
| publish client event with explicit session id | `POST /api/v1/session/client_event` | native | useful raw-proxy escape hatch when the client already has a concrete session id |

### Turn Lifecycle

| Broker-side intent | Local `codexw` route | Mapping class | Notes |
| --- | --- | --- | --- |
| start a run/turn | `POST /api/v1/turn/start` | adapter | broker “run” vocabulary may need renaming to `turn` or vice versa |
| interrupt active work | `POST /api/v1/turn/interrupt` | native | semantics align well |

### Transcript And Streaming

| Broker-side intent | Local `codexw` route | Mapping class | Notes |
| --- | --- | --- | --- |
| subscribe to event stream | `GET /api/v1/session/{session_id}/events` | adapter | SSE transport is native, but event families remain `codexw`-specific |
| read transcript snapshot | `GET /api/v1/session/{session_id}/transcript` | native | bounded transcript snapshot is directly useful to remote clients |

### Orchestration

| Broker-side intent | Local `codexw` route | Mapping class | Notes |
| --- | --- | --- | --- |
| compact orchestration status | `GET /api/v1/session/{session_id}/orchestration/status` | adapter | useful as a broker-visible capability, but not a standard `agentd` primitive |
| worker graph | `GET /api/v1/session/{session_id}/orchestration/workers` | adapter | explicitly `codexw`-native |
| dependency graph | `GET /api/v1/session/{session_id}/orchestration/dependencies` | adapter | explicitly `codexw`-native |

### Background Shells And Services

| Broker-side intent | Local `codexw` route | Mapping class | Notes |
| --- | --- | --- | --- |
| list shell jobs | `GET /api/v1/session/{session_id}/shells` | adapter | wrapper-owned jobs are `codexw`-specific |
| inspect one shell job | `GET /api/v1/session/{session_id}/shells/{job_ref}` | adapter | useful for remote focused job views and alias/capability-based lookup |
| start shell | `POST /api/v1/session/{session_id}/shells/start` | adapter | should remain available remotely |
| poll shell | `POST /api/v1/session/{session_id}/shells/{job_ref}/poll` | adapter | `job_ref` needs connector-safe path encoding |
| send shell stdin | `POST /api/v1/session/{session_id}/shells/{job_ref}/send` | adapter | explicit side-effect route |
| terminate shell | `POST /api/v1/session/{session_id}/shells/{job_ref}/terminate` | adapter | explicit side-effect route |
| list services | `GET /api/v1/session/{session_id}/services` | adapter | valuable for remote orchestration and WebUI |
| list capabilities | `GET /api/v1/session/{session_id}/capabilities` | adapter | valuable and `codexw`-specific |
| mutate service capabilities | `POST /api/v1/session/{session_id}/services/{job_ref}/provide` | adapter | not expected to map to a generic broker verb |
| mutate dependencies | `POST /api/v1/session/{session_id}/services/{job_ref}/depend` | adapter | same |
| mutate contract | `POST /api/v1/session/{session_id}/services/{job_ref}/contract` | adapter | same |
| relabel service | `POST /api/v1/session/{session_id}/services/{job_ref}/relabel` | adapter | same |

## Event Mapping

The connector should not rewrite `codexw` events into fake `agentd` events when
the semantic meaning differs.

Recommended rule:

- preserve the envelope contract from
  [docs/codexw-broker-event-envelope.md](docs/codexw-broker-event-envelope.md)
- expose `codexw` event `type` values unchanged by default
- optionally add connector metadata outside the `data` object

### Connector Metadata

Suggested connector-added fields:

```json
{
  "type": "orchestration.updated",
  "session_id": "sess_01HX...",
  "thread_id": "thread_abc123",
  "ts_unix_ms": 1760000002000,
  "source": "codexw",
  "broker": {
    "agent_id": "codexw-lab",
    "deployment_id": "mac-mini-01"
  },
  "data": {
    "counts": {
      "waits": 1,
      "exec_services": 1
    }
  }
}
```

This keeps the event payload `codexw`-native while giving remote clients enough
broker context to reason about deployment and routing.

## Reference Encoding Rules

`codexw` service/job routes allow:

- `bg-*`
- alias
- unique `@capability`
- numeric index when meaningful

The connector should avoid embedding raw `@capability` values directly into path
segments when that creates escaping ambiguity.

Recommended rule:

- path parameters use URL-encoded `job_ref`
- clients may still pass `@api.http`, but the connector is responsible for
  percent-encoding and decoding it across both detail routes and mutating
  action routes; malformed encoded path segments are rejected explicitly rather
  than forwarded as ambiguous local refs

## Auth Mapping

### Current Local Boundary

- local `codexw` API:
  - loopback-only
  - optional bearer token

### Connector Boundary

- connector handles remote broker auth:
  - broker-side OIDC/JWT or client token rules
  - optional mTLS for agent/deployment identity
- connector authenticates to local `codexw` using:
  - loopback trust
  - or local bearer token

Recommended rule:

- broker credentials must never be forwarded directly to local `codexw`
- local daemon auth and broker auth are separate trust boundaries

## Minimal Connector Responsibilities

The first useful connector should do only these things:

1. map broker-routed requests to local `codexw` HTTP routes
2. proxy SSE event streams
3. add deployment metadata
4. translate auth contexts
5. map explicit unsupported routes to clear errors

It should not:

- reinterpret orchestration semantics
- synthesize fake service/capability abstractions
- rewrite transcript content
- invent a second session model

It also should not imply broker-visible artifact routes exist before the local
API artifact track is actually implemented. That track is now explicitly
separated in:

- [codexw-broker-artifact-contract-sketch.md](codexw-broker-artifact-contract-sketch.md)
- [codexw-broker-artifact-implementation-plan.md](codexw-broker-artifact-implementation-plan.md)

It also should not imply broker-visible project-assignment or dependency-edge
routes exist before the local API collaboration-metadata track is actually
implemented. That track is now explicitly separated in:

- [codexw-cross-project-dependency-contract-sketch.md](codexw-cross-project-dependency-contract-sketch.md)
- [codexw-cross-project-dependency-implementation-plan.md](codexw-cross-project-dependency-implementation-plan.md)

## Unsupported Or Deferred Areas

The connector should explicitly reject or defer:

- scene/entity APIs
- audio/video parity assumptions from other systems
- any route that requires native `agentd` DB/scene semantics
- any route that assumes a generic run/artifact model where `codexw` has a more
  specific orchestration/service abstraction
- any artifact index/detail/content route until the local API artifact layer
  exists and the adapter contract/policy docs explicitly add it to the
  supported surface
- any project-assignment or dependency-edge route until the local API
  collaboration-metadata layer exists and the adapter contract/policy docs
  explicitly add it to the supported surface

## Initial Adapter Deliverable

The initial connector adapter should ship with a small compatibility table:

| Broker-facing route | Local `codexw` route | Status |
| --- | --- | --- |
| `/v1/agents/{agent_id}/proxy/...` session create | `/api/v1/session/new` | works |
| `/v1/agents/{agent_id}/proxy/...` session list | `/api/v1/session` | works as thin pass-through for canonical local session reads |
| `/v1/agents/{agent_id}/proxy/...` session inspect | `/api/v1/session/{session_id}` | works as thin pass-through for canonical local session reads |
| `/v1/agents/{agent_id}/proxy/...` session attach | `/api/v1/session/attach` | works with the same client/lease header projection policy as the session-scoped attach alias when the caller provides `session_id` in the body |
| `/v1/agents/{agent_id}/proxy/...` top-level client event | `/api/v1/session/client_event` | works with the same client/lease header projection policy as the session-scoped client-event alias when the caller provides `session_id` in the body |
| `/v1/agents/{agent_id}/proxy/...` turn start | `/api/v1/turn/start` | works with the same client/lease header projection policy as the session-scoped turn alias |
| `/v1/agents/{agent_id}/proxy/...` turn interrupt | `/api/v1/turn/interrupt` | works with the same client header projection policy as the session-scoped interrupt alias |
| `/v1/agents/{agent_id}/proxy/...` transcript | `/api/v1/session/{session_id}/transcript` | works as thin pass-through for canonical local transcript reads |
| `/v1/agents/{agent_id}/proxy_sse/...` events | `/api/v1/session/{session_id}/events` | works; non-`GET` methods are rejected as `method_not_allowed`, and `Last-Event-ID` replay stays a thin pass-through to the local API |
| `/v1/agents/{agent_id}/proxy/...` orchestration reads | `/api/v1/session/{session_id}/orchestration/*` | works as thin pass-through for canonical local orchestration status/worker/dependency reads |
| `/v1/agents/{agent_id}/proxy/...` shell reads | `/api/v1/session/{session_id}/shells*` | works as thin pass-through for canonical local shell list/detail reads, including preserving encoded path segments instead of alias-level decoding |
| `/v1/agents/{agent_id}/proxy/...` service reads | `/api/v1/session/{session_id}/services*` | works as thin pass-through for canonical local service list/detail reads |
| `/v1/agents/{agent_id}/proxy/...` capability reads | `/api/v1/session/{session_id}/capabilities*` | works as thin pass-through for canonical local capability list/detail reads, including preserving encoded path segments instead of alias-level decoding |
| `/v1/agents/{agent_id}/sessions` | `/api/v1/session` and `/api/v1/session/new` | works as method-sensitive alias surface |
| `/v1/agents/{agent_id}/sessions/{session_id}/attach` | `/api/v1/session/attach` | works as POST-only alias with `session_id` body injection when missing |
| `/v1/agents/{agent_id}/sessions/{session_id}/attachment/renew` | `/api/v1/session/{session_id}/attachment/renew` | works as POST-only alias surface with client/lease header projection |
| `/v1/agents/{agent_id}/sessions/{session_id}/attachment/release` | `/api/v1/session/{session_id}/attachment/release` | works as POST-only alias surface with client header projection |
| `/v1/agents/{agent_id}/sessions/{session_id}/turns` | `/api/v1/session/{session_id}/turn/start` | works as POST-only alias surface |
| `/v1/agents/{agent_id}/sessions/{session_id}/events` | `/api/v1/session/{session_id}/events` | works as alias SSE surface; non-`GET` methods are rejected as `method_not_allowed` |
| `/v1/agents/{agent_id}/sessions/{session_id}/shells` | `/api/v1/session/{session_id}/shells` and `/shells/start` | works as method-sensitive alias surface |
| `/v1/agents/{agent_id}/sessions/{session_id}/shells/{job_ref}` | `/api/v1/session/{session_id}/shells/{job_ref}` | works as alias surface with percent-decoded path segments on both detail and action routes |
| `/v1/agents/{agent_id}/sessions/{session_id}/services` | `/api/v1/session/{session_id}/services` | works as alias surface |
| `/v1/agents/{agent_id}/sessions/{session_id}/services/{job_ref}` | `/api/v1/session/{session_id}/services/{job_ref}` | works as alias surface |
| `/v1/agents/{agent_id}/sessions/{session_id}/capabilities` | `/api/v1/session/{session_id}/capabilities` | works as alias surface |
| `/v1/agents/{agent_id}/sessions/{session_id}/capabilities/{capability}` | `/api/v1/session/{session_id}/capabilities/{capability}` | works as alias surface with percent-decoded path segment |
| `/v1/agents/{agent_id}/sessions/{session_id}/services/{job_ref}/run` | `/api/v1/session/{session_id}/services/{job_ref}/run` | works as alias surface with percent-decoded action refs |
| orchestration/service extensions | `codexw`-specific routes | connector-extension |

That table should be kept concrete and testable.

The current connector now also keeps the mutating-route header projection policy
and the connector allowlist on one shared local-route classifier, so new POST
alias surfaces and the supported raw proxy turn-control routes do not need to
be added to two separate behavioral lists.
The same thin-pass-through proof now covers canonical raw proxy session list,
orchestration, shell, service, and capability reads too, rather than only the
mutating raw proxy routes and event replay path.
Read-only broker aliases are also method-sensitive at resolution time now, so
wrong-method requests fail as unknown connector routes instead of falling into
the generic raw-proxy allowlist rejection path.
The same is now true for the remaining write aliases and the `/sessions`
collection root, so alias routing no longer treats arbitrary verbs as if they
were implicit GET or POST variants.

The current repo now also includes a minimal consumer-side artifact for this
mapping:

- [scripts/codexw_broker_client.py](../scripts/codexw_broker_client.py)

That script intentionally exercises the broker-facing aliases, not the raw
local API, so it serves as the simplest non-test proof that the mapping is
usable by a real remote client shape.

The repo now also has a process-level smoke path that invokes this fixture
against the real connector binary, so the consumer-side mapping is verified as
well as documented.
