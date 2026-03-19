# codexw Broker Shared-Assumptions Matrix

This document records the broker shared-assumptions assessment:

- evaluate whether the user’s C agent framework in `~/work/agent` can share
  key assumptions with `codexw`

This is not a code-reuse claim. It is a design-compatibility assessment.

## Source References

The assessment is based on these project docs:

- `/Users/zongbaolu/work/agent/DESIGN.md`
- `/Users/zongbaolu/work/agent/broker/README.md`
- `/Users/zongbaolu/work/agent/docs/PROTOCOL.md`
- `/Users/zongbaolu/work/agent/docs/CLIENT.md`

For the source docs that define the current shell-first remote
host-examination surface on the `codexw` side of this matrix, see
[codexw-local-api-sketch.md](codexw-local-api-sketch.md),
[codexw-local-api-implementation-plan.md](codexw-local-api-implementation-plan.md),
[codexw-local-api-event-sourcing.md](codexw-local-api-event-sourcing.md),
[codexw-local-api-route-matrix.md](codexw-local-api-route-matrix.md), and
[codexw-native-support-boundaries.md](codexw-native-support-boundaries.md).

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
| broker-backed client surface | broker-backed app/WebUI clients are first-class consumers rather than incidental clients of a private daemon | `codexw` now explicitly treats broker-backed app/WebUI clients as part of the intended architecture | `shared directly` | Strong alignment; the connector is not just a terminal convenience layer |
| broker transport | broker uses HTTP proxying, SSE, and outbound websocket connector | `codexw` now exposes local HTTP routes, semantic SSE, and a connector adapter; websocket transport remains deferred | `shared with adapter` | Transport ideas fit, but the implemented contract is intentionally the supported loopback HTTP/SSE plus connector subset rather than full broker transport parity |
| auth model | broker uses OIDC/JWT for clients and mTLS for agent connections | `codexw` docs recommend optional local bearer token first, broker auth later | `shared with adapter` | Good long-term fit, but the current supported adapter intentionally keeps the local auth boundary simpler |
| deployment routing | broker can route to multiple agent deployments under one logical agent id | `codexw` has not promoted a deployment-routing key beyond `session_id`/`thread_id` into its supported adapter contract | `shared with adapter` | Compatible conceptually, but not yet a native `codexw` runtime concept |
| host examination through broker routes | broker-facing clients should be able to inspect and control host-side execution without direct local terminal access | `codexw` now exposes broker-visible shell/service/transcript/event surfaces for that purpose | `shared with adapter` | The shell/service/transcript approach fits the same product need, and it is the shell-first foundation of the current supported experimental adapter even though the exact route shapes remain adapter-owned |
| dedicated artifact catalog | richer app/WebUI clients benefit from stable artifact list/detail/content routes | `codexw` only has a separate artifact-contract track today, not an implemented broker-visible artifact catalog | `shared with adapter` | The product assumption is shared, but the current repo intentionally keeps artifact routes outside the current supported experimental adapter until they are implemented and proven |
| scene/entity model | `agentd` exposes durable scene/entity APIs | `codexw` has no scene/entity runtime equivalent | `not shared` | Should stay out of current compatibility claims |
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
5. broker-backed clients should be able to inspect host-side execution through
   structured remote surfaces instead of direct terminal access

## Adapter-Required Areas

These areas are compatible in principle but should not be treated as wire-level
equivalence:

- broker route shapes
- auth deployment details
- deployment selection and routing
- event family names
- run/turn lifecycle naming
- any future artifact index/detail/content contract

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
4. as a reminder that broker-backed app/WebUI clients and host-side inspection
   are product requirements, while the dedicated artifact catalog remains a
   separate tracked implementation lane

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
