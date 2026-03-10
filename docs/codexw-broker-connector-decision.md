# codexw Broker Connector Decision

This document captures the original architecture decision for how broker
connectivity should be layered into `codexw`.

It is now primarily a historical decision record, not a statement that the
connector and local API are still hypothetical. The current implemented state
and support recommendation live in:

- [docs/codexw-broker-adapter-status.md](codexw-broker-adapter-status.md)
- [docs/codexw-broker-promotion-recommendation.md](codexw-broker-promotion-recommendation.md)
- [docs/codexw-broker-support-policy.md](codexw-broker-support-policy.md)
- [docs/codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md)

The decision question is:

- should `codexw` expose only a local API and let some other process bridge it
  to a broker?
- should `codexw` expose a local API and also ship a broker connector?
- should `codexw` speak to a broker directly?

## Options

### Option A: Local API First

Shape:

- `codexw` exposes a local HTTP/SSE or similar control API
- WebUI/mobile/remote terminal clients talk to a local-side adapter or direct
  local endpoint
- broker integration is deferred

Strengths:

- smallest initial implementation surface
- lowest security exposure in the first release
- easiest way to validate session model, event model, and capability routing
- keeps `codexw` focused on daemon/control-plane behavior instead of remote auth
  and relay behavior

Weaknesses:

- does not satisfy universal connectivity by itself
- on its own, it still needs a later bridge or connector for cross-network
  access
- risks creating a local API that is awkward to broker later if not designed
  carefully

Best use:

- the initial implementation vehicle

### Option B: Local API Plus Connector

Shape:

- `codexw` exposes the canonical local control API
- a connector process or connector mode translates that local API to a remote
  broker protocol
- broker-facing clients attach through the connector, not directly to the
  daemon core

Strengths:

- preserves a clean local API that can also power desktop WebUI, local
  automation, and tests
- isolates broker auth, relay, reconnect, and deployment concerns from the core
  `codexw` runtime
- creates the best path to compatibility with `~/work/agent` without forcing
  `codexw` internals to look exactly like that system
- supports staged rollout:
  - local API first
  - connector second
  - broker compatibility third

Weaknesses:

- more moving parts than local-only
- requires one more protocol boundary to specify and test
- can drift if the local API and connector mapping are not kept versioned

Best use:

- likely long-term architecture if universal connectivity is a real project goal

### Option C: Direct Broker Connectivity

Shape:

- `codexw` itself authenticates to a broker and exposes remote sessions directly
- local API is optional or omitted

Strengths:

- fewest runtime layers for remote access once fully built
- no extra connector process

Weaknesses:

- highest complexity inside the `codexw` daemon core
- mixes local agent runtime logic with broker auth, reconnect, relay, and
  deployment concerns
- makes local-only testing and embedding less clean
- creates the strongest pressure to conform internal architecture to the broker
  protocol early, before `codexw` local API semantics have stabilized

Best use:

- only if broker compatibility becomes the dominant product requirement

## Decision Matrix

| Criterion | Local API First | Local API + Connector | Direct Broker |
| --- | --- | --- | --- |
| Fastest first implementation | High | Medium | Low |
| Lowest core complexity | High | Medium | Low |
| Best local testing story | High | High | Medium |
| Best long-term universal connectivity | Low | High | High |
| Best fit for `~/work/agent` compatibility | Medium | High | Medium |
| Lowest security risk in the earliest adapter stage | High | Medium | Low |
| Lowest risk of architectural lock-in | Medium | High | Low |

## Recommended Decision

Recommended path:

1. local API first
2. local API plus connector as the target architecture
3. no direct broker connectivity in the first serious implementation pass

That means:

- the local daemon-facing API should be the canonical contract
- a connector should map that API to a broker-facing protocol
- direct broker mode should remain explicitly deferred unless later evidence
  shows the connector layer is an unacceptable burden

That sequencing has now largely happened:

- the loopback local API exists
- the connector adapter exists
- broker-style alias routes and process-level broker client fixtures exist

What remains is support-level judgment, contract discipline, and optional
hardening rather than the original architecture decision itself.

## Why This Is The Best Fit For codexw

`codexw` already has a strong local runtime model:

- local terminal UI
- app-server-backed thread/session control
- wrapper-owned background shell orchestration
- local filesystem and workspace assumptions

That makes it much safer to stabilize a local control API first.

The `~/work/agent` project is still highly valuable here, but primarily as:

- a broker/protocol reference
- a relay/auth/session model reference
- a compatibility target for the connector

not as proof that `codexw` should collapse its daemon core directly into a
broker-facing agent runtime.

## Implementation Consequences

If this recommendation is accepted, the immediate next design/implementation
sequence should be:

1. finalize local session identity and event envelope
2. define the first local API routes and error model
3. implement a minimal local daemon mode inside `codexw`
4. prototype a thin connector that maps local API calls/events to the broker
   vocabulary from `~/work/agent`
5. evaluate incompatibilities before considering any direct broker mode

## Explicit Non-Goals For The Initial Slice

The original first slice was intentionally not supposed to do all of the
following at once:

- direct broker auth inside `codexw`
- mobile/web/terminal client support
- connector compatibility layer
- deployment routing and multi-device coordination

Trying to do all of them at once would increase the risk of locking in a poor
session model.

## Revisit Conditions

This decision should be revisited only if one of these becomes true:

- the connector mapping to the `~/work/agent` protocol proves too lossy
- the local API becomes only a thin pass-through and provides no meaningful
  abstraction
- broker deployment/auth constraints require tighter integration than a
  connector can reasonably provide
