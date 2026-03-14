# codexw Broker Compatibility Target

This document records the broker compatibility target decision:

- what level of compatibility should `codexw` target relative to the user’s
  `~/work/agent` broker/client framework?

The answer should be explicit, because “compatibility” can mean very different
things.

## Candidate Levels

### Level 1: Payload Vocabulary Reuse

Meaning:

- keep `codexw` local API semantics independent
- reuse selected field names, envelope fields, and conceptual vocabulary where
  that improves future interoperability

Examples:

- `session_id`
- structured event envelopes
- machine-readable status payloads

Strengths:

- low implementation risk
- little architectural lock-in

Weaknesses:

- not enough for drop-in broker/client reuse

### Level 2: Partial Protocol Compatibility

Meaning:

- `codexw` keeps its own canonical local API
- a connector or adapter maps that API to selected `~/work/agent` broker/client
  surfaces
- compatibility is explicit, scoped, and lossy where necessary

Examples:

- selected session lifecycle routes
- selected event-stream routes
- selected attach/proxy semantics

Strengths:

- strongest leverage-to-risk ratio
- supports real interoperability without forcing `codexw` to mimic foreign
  runtime internals

Weaknesses:

- requires a maintained compatibility matrix
- adapter behavior must be documented carefully

### Level 3: Full Broker/Client Compatibility

Meaning:

- `codexw` aims to behave as if it were a native implementation of the
  `~/work/agent` broker/client contract

Strengths:

- maximum interoperability in theory

Weaknesses:

- high risk of distorting `codexw` internals
- likely to create awkward fits around:
  - app-server thread identity
  - wrapper-owned background shells
  - orchestration graph views
  - service capability registry

## Recommended Target

Recommended compatibility target:

- Level 2: partial protocol compatibility

With this interpretation:

- `codexw` local API is the canonical runtime contract
- `~/work/agent` compatibility is pursued through a connector/adapter layer
- payload vocabulary reuse is encouraged where it helps
- broker-backed app/WebUI clients and broker-visible host shell examination are
  part of the intended compatibility target
- full drop-in protocol equivalence is explicitly not required for the current
  supported adapter scope
- a dedicated artifact index/detail/content contract remains a separate tracked
  design lane rather than part of the current supported adapter claim:
  - [codexw-broker-artifact-contract-sketch.md](codexw-broker-artifact-contract-sketch.md)
  - [codexw-broker-artifact-implementation-plan.md](codexw-broker-artifact-implementation-plan.md)

## Why Level 2 Is Correct

`codexw` already has several domain-specific concepts that are worth preserving:

- wrapper session vs local thread identity
- orchestration worker/dependency views
- wrapper-owned background shell jobs
- reusable service capabilities and live mutation controls

These are valuable features, not accidental implementation details.

Forcing full protocol compatibility too early would create pressure to flatten
or hide those semantics.

## Direct-Fit Compatibility Scope

The following areas are good candidates for direct or near-direct compatibility:

- session creation and attachment concepts
- machine-readable transcript/event streaming
- compact status summaries
- interrupt/stop semantics
- remote broker-backed client attachment and control semantics

## Adapter-Only Compatibility Scope

The following areas should be treated as adapter surfaces, not strict protocol
contracts:

- orchestration graph views
- background shell/service controls
- broker-visible host shell examination flows for app/WebUI clients
- capability registry views
- service mutation operations
- any route that depends on `bg-*`, alias, or `@capability` reference rules

## Explicitly Deferred Compatibility

The following should remain out of scope for the current supported adapter
compatibility claim:

- audio/media streaming parity
- general scene/entity APIs
- full broker deployment management
- perfect transport parity for every websocket/broker lifecycle detail
- any artifact index/detail/content route until it is explicitly implemented,
  mapped, and proven through the separate artifact-contract track; the current
  supported experimental adapter should still be read as shell-first host
  examination rather than as an implicit artifact browser

## Implementation Rule

Every future compatibility claim should be labeled as one of:

- `native`
- `adapter`
- `unsupported`

That label should appear in:

- the connector design
- the endpoint audit
- implementation notes where relevant

## Initial Compatibility Deliverable

The initial compatibility pass should produce a short matrix with three
columns:

1. local `codexw` route or event
2. mapped `~/work/agent` surface, if any
3. compatibility class:
   - native
   - adapter
   - unsupported

That is the minimal rigor needed to avoid vague compatibility claims.
