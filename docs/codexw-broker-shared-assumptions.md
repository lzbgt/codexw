# codexw Broker Shared-Assumptions Matrix

This document resolves the remaining broker-design TODO:

- evaluate whether the user’s C agent framework in `~/work/agent` can share
  key assumptions with `codexw`

This is not a code-reuse claim. It is a design-compatibility assessment.

## Source References

The assessment is based on these project docs:

- `/Users/zongbaolu/work/agent/DESIGN.md`
- `/Users/zongbaolu/work/agent/broker/README.md`
- `/Users/zongbaolu/work/agent/docs/PROTOCOL.md`
- `/Users/zongbaolu/work/agent/docs/CLIENT.md`

## Classification Meanings

- `shared directly`
  - the assumption is already compatible with `codexw`'s current direction
- `shared with adapter`
  - the concept is compatible, but a mapping layer or wrapper-specific field set
    is still required
- `not shared`
  - the assumption should not be treated as a common contract

## Matrix

| Area | `~/work/agent` assumption | `codexw` status | Classification | Notes |
| --- | --- | --- | --- | --- |
| session identity | daemon owns session ids; sessions are explicit client-facing objects | `codexw` now proposes wrapper-owned `session_id` distinct from local `thread_id` | `shared directly` | Strong alignment; both systems want a client-facing session handle independent from low-level provider state |
| underlying conversation identity | provider/server state is not the canonical session identity | `codexw` keeps `thread_id` as underlying Codex conversation identity, not the remote-control id | `shared directly` | Same conceptual split: remote session handle vs backend conversation identity |
| client identity | clients attach with distinct client ids / instance ids | `codexw` broker design keeps optional `client_id` or attachment id separate from `session_id` | `shared directly` | Good fit for audit and multi-client concurrency rules |
| event envelopes | typed, machine-readable events preferred over UI heuristics | `codexw` broker event envelope follows the same principle | `shared directly` | This is one of the strongest compatibility points |
| transcript/log model | append-only friendly structured events | `codexw` event sketch is append-only friendly and SSE-ready | `shared directly` | Exact event types differ, but the persistence and replay shape aligns |
| local API first | `agentd` already exposes daemon HTTP APIs used by clients | `codexw` recommends a local daemon-facing API first | `shared directly` | Strong architectural convergence |
| broker connector model | connector bridges local daemon to broker over websocket | `codexw` recommends local API plus connector as target architecture | `shared directly` | This is the clearest reusable deployment pattern |
| broker transport | broker uses HTTP proxying, SSE, and outbound websocket connector | `codexw` can support SSE and proxyable local routes, but does not yet expose them | `shared with adapter` | Transport ideas fit; exact route and envelope mapping still needs a connector layer |
| auth model | broker uses OIDC/JWT for clients and mTLS for agent connections | `codexw` docs recommend optional local bearer token first, broker auth later | `shared with adapter` | Good long-term fit, but phase-1 `codexw` is intentionally simpler |
| deployment routing | broker can route to multiple agent deployments under one logical agent id | `codexw` design has not yet fixed a deployment routing key beyond `session_id`/`thread_id` | `shared with adapter` | Compatible conceptually, but not yet a native `codexw` runtime concept |
| scene/entity model | `agentd` exposes durable scene/entity APIs | `codexw` has no scene/entity runtime equivalent | `not shared` | Should stay out of first-phase compatibility claims |
| tool/runtime abstraction | `agentd` is a generic run/tool daemon | `codexw` has wrapper-owned orchestration, background shells, and service capabilities | `shared with adapter` | The control-plane idea fits, but `codexw` has richer wrapper-native orchestration semantics |
| background shell/service registry | no 1:1 native concept in `agentd` docs | `codexw` exposes wrapper-owned jobs, services, capabilities, and live mutation controls | `not shared` | These should remain `codexw`-specific API surfaces even if broker-compatible transport is adopted |

## High-Value Shared Assumptions

The following assumptions are strong enough that `codexw` should intentionally
reuse them when practical:

1. client-facing session ids should be explicit and daemon-owned
2. typed event envelopes are better than terminal/UI heuristic scraping
3. a local daemon API is the right first abstraction boundary
4. broker connectivity should be added through a connector before direct daemon
   coupling unless proven otherwise

## Adapter-Required Areas

These areas are compatible in principle but should not be treated as wire-level
equivalence:

- broker route shapes
- auth deployment details
- deployment selection and routing
- event family names
- run/turn lifecycle naming

These need an explicit connector mapping or compatibility layer.

## Areas That Should Remain codexw-Native

The following should remain clearly `codexw`-native:

- orchestration worker/dependency views
- background shell lifecycle and reuse
- service capability registry
- live service mutation controls (`provide`, `depend`, `contract`, `relabel`)
- `bg-*` / alias / `@capability` reference semantics

Trying to force these into the `agentd` abstraction too early would make the
system less clear, not more compatible.

## Practical Recommendation

The user’s C agent framework should be leveraged in three ways:

1. as a daemon/broker architectural reference
2. as a connector/proxy compatibility target
3. as a vocabulary source for event/session/auth concepts where they already fit

It should not be treated as a requirement that `codexw` collapse into the same
runtime model.

## Decision Summary

Recommended conclusion:

- session identity rules: share directly
- event envelope philosophy: share directly
- broker transport assumptions: share with adapter
- auth and deployment routing concepts: share with adapter
- scene/entity and generic daemon semantics: do not force as shared

This keeps the compatibility target strong without erasing `codexw`'s
higher-level orchestration and service-control model.
