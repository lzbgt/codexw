# codexw Broker Support Policy

This document defines what "supported experimental adapter" means for the
broker-facing `codexw` adapter surface.

It is the operational-policy companion to:

- [codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md)
- [codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md)
- [codexw-broker-promotion-recommendation.md](codexw-broker-promotion-recommendation.md)
- [codexw-broker-prototype-status.md](codexw-broker-prototype-status.md)

It does not redefine the adapter contract. It answers a narrower question:

- what support level the current adapter claim implies
- which parts are expected to remain stable
- what kinds of changes are still allowed
- what documentation and proof must accompany changes

## Scope

This policy applies to the documented broker-facing adapter surface built from:

- the local API as the canonical runtime contract
- the connector as a thin broker-style adapter
- the Python broker-style fixture as a reference client

This policy covers:

- documented route families
- documented error semantics
- documented SSE/replay semantics
- documented lease and client-role semantics
- the explicit unsupported boundary

This policy does not create a promise for:

- full `agentd` compatibility
- all possible future broker routes
- production deployment, auth, or fleet semantics
- multi-daemon lease coordination
- a general-purpose public SDK

## Support Level

The current intended support level is:

- **supported experimental adapter**

That means:

- the documented surface is real and intentionally supported
- consumers may build against the documented surface
- the surface is still allowed to evolve more aggressively than a fully stable
  public platform API

It does **not** mean:

- "best effort only"
- "prototype with no expectations"
- "frozen forever"

## Stability Expectations

### Stable Enough To Rely On

The following are expected to remain stable enough that a broker-style consumer
can depend on them:

- documented route families and high-level responsibilities
- owner / observer / rival role model
- lease-owned versus observer-readable operation classes
- connector header projection semantics for `client_id` and `lease_seconds`
- structured error envelope fields:
  - `status`
  - `code`
  - `message`
  - `retryable`
  - `details`
- SSE replay behavior through `Last-Event-ID`
- explicit rejection of unsupported broker/client surfaces

### Still Allowed To Evolve

The following may still evolve without invalidating the support claim, provided
the changes are documented and verified:

- additional supported routes
- additional event families
- richer response payload fields
- stronger adversarial and multi-client proof
- internal code organization and implementation structure

### Not Allowed To Drift Silently

The following should not change silently:

- mutation versus observer-read boundaries
- lease conflict semantics
- connector-side validation behavior
- error-code meaning for documented failure classes
- whether a route family is supported or unsupported

If one of those changes, the adapter contract and support docs must be updated
deliberately in the same batch.

## Breaking Change Policy

Breaking changes to the supported experimental surface are still possible, but
they must be explicit.

For this adapter, a change is breaking if it:

- removes a documented route family
- changes a documented route from observer-readable to lease-owned or vice
  versa
- changes the meaning of documented error codes
- removes required error-envelope fields
- removes documented SSE resume behavior
- turns a documented supported route into an unsupported one

When a breaking change is necessary, the batch should do all of the following:

1. update the adapter contract doc
2. update the support policy doc
3. update the prototype-status doc
4. update the proof matrix if the proof story changed
5. update process-level proof and fixture coverage

This repo does not currently promise a long deprecation window, but it does
require explicit change disclosure in docs and tests.

## Deprecation Policy

Deprecation is preferred over silent removal when practical.

For this adapter, deprecation means:

- the route, field, or behavior is still accepted for now
- the docs mark it as deprecated
- the intended replacement is named
- process-level proof is adjusted to prefer the replacement path

The repo does not yet define a time-based deprecation SLA. It does require
that deprecations be documented before removal.

## Proof Maintenance Policy

Because this adapter claim is evidence-driven, doc changes alone are not enough
for contract-affecting behavior changes.

If a batch changes supported adapter behavior, it should update:

- unit or route-level validation coverage where relevant
- connector smoke coverage where relevant
- Python broker-client fixture coverage when the behavior affects an external
  consumer shape

For policy-sensitive changes, process-level proof is preferred over only unit
coverage.

## Unsupported Boundary Policy

The unsupported boundary is part of the support story, not a disclaimer.

That means:

- unsupported broker-style routes should keep failing explicitly
- raw proxy passthrough outside the allowlist should keep failing explicitly
- docs should keep naming the intentionally unsupported areas

Adding new supported surface is fine, but it should happen by explicit policy
change, not by accidental passthrough.

## Documentation Update Policy

Any batch that changes the supported broker-facing adapter surface should review
and update the relevant docs, usually a subset of:

- [codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md)
- [codexw-broker-support-policy.md](codexw-broker-support-policy.md)
- [codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md)
- [codexw-broker-prototype-status.md](codexw-broker-prototype-status.md)
- [codexw-broker-client-fixture.md](codexw-broker-client-fixture.md)
- [codexw-broker-out-of-scope.md](codexw-broker-out-of-scope.md)

The goal is to keep status, contract, support level, and proof claims aligned
instead of letting one doc quietly outrun the others.

## Current Interpretation

Under this policy, the repo's present recommendation is:

- the broker-facing adapter is supported for documented experimental use
- consumers may build against the documented contract
- future changes must remain explicit and evidence-backed

That is stronger than "prototype only," but still intentionally narrower than a
fully frozen production platform API.

## Companion Docs

- [codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md)
- [codexw-broker-adapter-promotion.md](codexw-broker-adapter-promotion.md)
- [codexw-broker-promotion-recommendation.md](codexw-broker-promotion-recommendation.md)
- [codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md)
- [codexw-broker-prototype-status.md](codexw-broker-prototype-status.md)
- [codexw-broker-client-policy.md](codexw-broker-client-policy.md)
- [codexw-broker-out-of-scope.md](codexw-broker-out-of-scope.md)
