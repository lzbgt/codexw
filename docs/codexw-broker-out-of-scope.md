# codexw Broker Out-of-Scope Matrix

This document records the broker out-of-scope decision:

- which broker/client surfaces are intentionally out of scope for `codexw`

The goal is to stop forcing readers to infer scope from scattered caveats in
the connectivity, compatibility, and endpoint-audit docs.

The currently named out-of-scope broker surfaces are also backed by explicit
connector rejection proof in the process-level smoke suite, not only by prose.

## Scope Classes

This document uses three scope labels.

### In Scope

- part of the intended remote-control story for `codexw`
- reasonable to support within the current adapter scope

### Deferred

- plausible later
- not required for current remote-control value

### Out of Scope

- not part of the intended `codexw` broker/connector objective
- would distort the project if added only for “completeness”

## In-Scope Remote-Control Surface

These remain clearly in scope:

- session create / attach / inspect
- attachment lease renew / release
- turn start / interrupt
- transcript fetch
- orchestration status / workers / dependencies
- shell list / detail / start / poll / send / terminate
- service list / detail / attach / wait / run
- service mutation:
  - `provide`
  - `depend`
  - `contract`
  - `relabel`
- capability list / detail
- event streaming and `Last-Event-ID` resume
- structured `client_event` publish
- connector alias mapping for those surfaces
- broker-style fixture/client examples that exercise those surfaces

## Deferred Areas

These are plausible future extensions, but not required now:

- stronger adapter hardening beyond the current connector
- more formal SDKs or reusable client libraries
- browser/mobile UI layers in this repo
- a dedicated broker-visible artifact catalog/detail/content contract
- richer client presence or collaboration state above the current lease model
- additional long-lived multi-client coordination semantics
- multi-runtime or multi-daemon coordination
- fuller broker deployment/auth packaging

## Explicitly Out-of-Scope Areas

### 1. Full `agentd` Surface Parity

Out of scope:

- claiming `codexw` should implement every `agentd` endpoint or broker contract
- flattening `codexw` semantics to mimic foreign runtime internals

### 2. Scene / Entity / World Models

Out of scope:

- scene graphs
- entity application routes
- persistent world-model APIs

### 3. Full Audio / Media Broker Parity

Out of scope:

- streaming audio parity
- media-session parity
- transport promises for rich audio workflows

### 4. Alternate-Screen UI Remoting

Out of scope:

- remoting the terminal UI itself
- reproducing alternate-screen widget behavior over the broker path

### 5. Connector-Owned State Machines

Out of scope:

- a second authoritative lease model in the connector
- connector-owned mutation queueing
- connector-owned session truth that diverges from local runtime state

### 6. Production Security Claims

Out of scope for the current supported-experimental adapter:

- claiming production-grade auth, secret handling, or tenant isolation
- claiming deployment-ready zero-trust broker support

## Decision Rule For Future Additions

When a new broker-facing idea comes up, evaluate it in this order:

1. Does it expose real existing `codexw` semantics remotely?
2. Does it improve remote control of the existing runtime?
3. Does it preserve the local API as the authority?
4. Does it avoid forcing `codexw` to mimic unrelated runtime models?

If the answer to 1-3 is “yes” and 4 is “yes”, it is likely in scope or
deferred.

If the feature exists mainly to chase foreign parity without local runtime
value, it is probably out of scope.
