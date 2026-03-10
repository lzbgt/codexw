# codexw Broker Adapter Promotion

This document defines what must be true before the current broker/local-API
stack should be treated as a supported adapter layer under the current
supported experimental adapter claim. For the current recommendation based on the
present proof surface, see
[codexw-broker-promotion-recommendation.md](codexw-broker-promotion-recommendation.md).
For the operational meaning of that support level after promotion, see
[codexw-broker-support-policy.md](codexw-broker-support-policy.md).

It does not redefine the whole broker architecture. It answers a narrower
question:

- what contract areas must be stable
- what proof must exist
- what policy choices must be explicit
- what unsupported boundary must remain clear

## Purpose

Use this document when checking whether the current supported experimental
adapter claim should remain in place, be strengthened, or be reconsidered.

## Promotion Does Not Mean Full Broker Parity

Promotion does **not** mean:

- full `agentd` protocol compatibility
- implementing every broker/client surface from `~/work/agent`
- distributed lease coordination
- multi-daemon session replication
- production browser/mobile UX in this repo
- production deployment registration and auth infrastructure

Promotion means a smaller, explicit claim:

- the local API is the canonical runtime contract
- the connector is a supported adapter for a defined subset of broker-style
  routes and SSE behavior
- the client/lease model is explicit enough that independent consumers can act
  correctly without reverse-engineering implementation details

## Required Contract Areas

### 1. Route Contract

The supported route families must be explicit and stable enough to document and
test:

- session lifecycle and inspection
- attachment renew/release
- turn start/interrupt
- transcript fetch
- orchestration status/workers/dependencies
- shell list/detail/control
- service list/detail/control
- capability list/detail
- `client_event`
- session SSE event stream

Promotion requires:

- route ownership is documented
- request and response shapes are documented
- connector alias mapping is documented
- unsupported routes fail explicitly rather than implicitly

### 2. Error Contract

The current structured error envelope must remain a deliberate public contract:

- `status`
- `code`
- `message`
- `retryable`
- `details`

Promotion requires:

- lease conflicts such as `attachment_conflict` remain structured
- validation failures remain field-accurate
- the connector preserves structured local-API errors instead of collapsing
  them into generic text

The current adapter stack already has process-level proof for malformed
injected JSON request bodies, malformed lease-header projection, and preserved
local field-validation errors through the connector path. The remaining
promotion work in this area is contract freezing and wording, not basic
implementation coverage.

### 3. Event Contract

The event surface must be stable enough for remote consumers:

- session and turn updates
- transcript updates
- orchestration updates
- capability/service-related updates
- client-published semantic events

Promotion requires:

- explicit event-envelope fields
- explicit SSE replay/resume semantics with `Last-Event-ID`
- no dependency on terminal-rendered output as the remote contract

### 4. Client Policy Contract

The lease/ownership rules must be clear enough to function as an adapter
contract instead of an implementation-detail note:

- owner
- observer
- rival

Promotion requires:

- lease-owned versus observer-readable operations are documented
- renew/release rules are explicit
- conflict semantics are explicit
- connector-side client/lease projection behavior is explicit

This policy is now frozen directly in
[codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md), with
the longer rationale and role discussion still in
[codexw-broker-client-policy.md](codexw-broker-client-policy.md). Promotion is
therefore no longer blocked on writing down the policy; it is blocked only on
deciding whether the current proof is sufficient.

### 5. Explicit Unsupported Boundary

Promotion requires a stable statement of what is intentionally unsupported:

- unsupported `agentd` surfaces
- distributed deployment behaviors
- scene/entity models
- audio/video expectations
- parity assumptions `codexw` does not intend to honor

That boundary should remain aligned with
[codexw-broker-out-of-scope.md](codexw-broker-out-of-scope.md).

## Proof Requirements

Promotion should require more than route availability.

The repo-side evidence for the areas below is summarized in
[codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md).

### Required Automated Proof

At minimum, the repo should continue to have process-level proof for:

- session create/attach/list/inspect
- attachment renew/release
- turn start/interrupt
- transcript and orchestration inspection
- shell and service control
- focused service/capability detail
- SSE consumption and `Last-Event-ID` resume
- structured conflict propagation
- `client_event` publish plus replay/resume
- the real standalone broker-style fixtures as external consumer shapes

### Required Multi-Client Proof

Promotion should require stable process-level proof for:

- owner and observer coexistence
- rival mutation rejection
- explicit release and takeover
- repeated role reversal
- event replay/resume after ownership changes
- client-event behavior under lease rules

### Recommended Additional Proof Before Promotion

Useful but not strictly required beyond the already-landed proof set:

- a small compatibility matrix beyond the standalone broker fixtures
- broader sustained churn or longevity coverage beyond the current named,
  anonymous, handoff, reversal, and observer-readable contention workflows

## Operational Requirements

Promotion should not happen unless these are still true:

1. the local API remains the canonical runtime surface
2. the connector remains thin and does not own shadow session state
3. the connector does not invent independent lease semantics
4. status docs can describe the supported surface without major caveats on every
   route family

If promotion requires the connector to own session state, event history, or
independent coordination logic, that is a signal to revisit the architecture
instead of promoting it.

## Decision Matrix

With the current contract and proof set in place, the project should be able to
choose one of:

### Promote To Supported Adapter

Choose this when:

- route, error, event, and policy contracts are explicit
- the support/stability expectations for "supported experimental" are explicit
- multi-client proof is strong enough for intended consumers
- the connector remains a thin adapter
- the unsupported boundary is explicit and acceptable

### Keep As Prototype

Choose this when:

- the surface is useful but still moving
- event or policy behavior is still changing
- consumers still need implementation knowledge to behave correctly

### Rework The Adapter Model

Choose this when:

- the connector needs shadow state
- the local API is too awkward for remote consumption
- broker-facing behavior forces distortions that should instead be solved in the
  local API or a different adapter design

## Current Best Reading Of Repo State

Based on the repo as it exists now:

- the route surface is already broad enough
- the connector alias surface is already broad enough
- process-level proof already supports the current supported experimental
  adapter recommendation
- the biggest remaining work is promotion judgment, support follow-through, and
  optional adversarial hardening, not route invention or missing contract text

That means the next meaningful work is:

1. use
   [codexw-broker-hardening-catalog.md](codexw-broker-hardening-catalog.md)
   as the home for optional churn, replay, and adversarial expansions instead
   of treating them as active blockers by default
2. use the explicit adapter contract plus the proof matrix to validate or
   challenge the current recommendation in
   [codexw-broker-promotion-recommendation.md](codexw-broker-promotion-recommendation.md)
3. reopen the promotion question only if new evidence materially changes that
   recommendation

## Companion Docs

- [codexw-broker-adapter-status.md](codexw-broker-adapter-status.md)
- [codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md)
- [codexw-broker-promotion-recommendation.md](codexw-broker-promotion-recommendation.md)
- [codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md)
- [codexw-broker-support-policy.md](codexw-broker-support-policy.md)
- [codexw-broker-client-policy.md](codexw-broker-client-policy.md)
- [codexw-broker-out-of-scope.md](codexw-broker-out-of-scope.md)
- [codexw-broker-hardening-catalog.md](codexw-broker-hardening-catalog.md)
- [codexw-broker-connector-prototype-plan.md](codexw-broker-connector-prototype-plan.md)
- [codexw-broker-connector-mapping.md](codexw-broker-connector-mapping.md)
