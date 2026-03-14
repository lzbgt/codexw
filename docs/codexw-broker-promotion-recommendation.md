# codexw Broker Promotion Recommendation

This document answers the current decision question left open by the broader
broker docs:

- should the current supported experimental adapter claim remain in place
- or should the repo retreat to a narrower non-supported/prototype framing

## Recommendation

Recommend promoting the current broker/local-API stack to a **supported
experimental adapter** for the contract frozen in
[codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md).

That recommendation is intentionally narrower than:

- full `agentd` parity
- production deployment infrastructure
- production browser/mobile UX
- multi-daemon lease coordination
- a general-purpose public SDK commitment
- a dedicated broker-visible artifact catalog/detail/content contract
- a project-assignment or dependency-edge route family for cross-project
  collaboration metadata

The recommended claim is:

- the local API is the canonical runtime contract
- the connector is a supported adapter for the documented broker-style subset
- the owner / observer / rival and lease semantics are part of the supported
  contract
- the structured error and SSE replay behavior are part of the supported
  contract
- the unsupported boundary remains explicit and enforced

That supported claim currently covers the verified session/event/orchestration/
shell/service/capability surface. It does **not** yet imply that artifact index,
detail, or content routes are part of the supported adapter. It also does
**not** yet imply that session project-assignment or project dependency-edge
routes are part of the supported adapter.

The operational meaning of that support level now lives in
[codexw-broker-support-policy.md](codexw-broker-support-policy.md), so this
document no longer has to carry implicit support/stability assumptions by
itself.

## Why Promotion Is Now Justified

This recommendation is based on facts already present in the repo.

### 1. Contract Language Is Frozen

The repo now has explicit documents for:

- the adapter contract:
  [codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md)
- the support/stability policy:
  [codexw-broker-support-policy.md](codexw-broker-support-policy.md)
- client policy and role semantics:
  [codexw-broker-client-policy.md](codexw-broker-client-policy.md)
- unsupported boundaries:
  [codexw-broker-out-of-scope.md](codexw-broker-out-of-scope.md)
- promotion criteria:
  [codexw-broker-adapter-promotion.md](codexw-broker-adapter-promotion.md)
- proof mapping:
  [codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md)

That means promotion is no longer blocked on missing design language.

### 2. Proof Coverage Is Broad And Process-Level

The current proof set is not only unit-level. It includes:

- local API route tests
- connector unit tests
- connector process-level smoke tests
- the standalone broker-style client fixtures exercised as real
  subprocess client

The proof matrix now marks all major contract areas as `strong proof`, including:

- route contract
- error contract
- event contract
- client policy contract
- unsupported boundary

### 3. The Remaining Gaps Are Optional Hardening, Not Missing Contract Basics

The remaining gaps still mentioned in the repo are things like:

- longer-lived contention churn
- broader adversarial permutations
- route-by-route proof density beyond representative workflows
- the still-design-only artifact contract track documented in
  [codexw-broker-artifact-contract-sketch.md](codexw-broker-artifact-contract-sketch.md)
  and [codexw-broker-artifact-implementation-plan.md](codexw-broker-artifact-implementation-plan.md)
- the still-design-only project/dependency contract track documented in
  [codexw-cross-project-dependency-contract-sketch.md](codexw-cross-project-dependency-contract-sketch.md)
  and [codexw-cross-project-dependency-implementation-plan.md](codexw-cross-project-dependency-implementation-plan.md)

Those are good hardening tasks, but they are not evidence that the current
adapter claim is false.

### 4. The Connector Still Preserves The Intended Architecture

Promotion is justified partly because the connector has *not* drifted into a
shadow runtime. The code and docs still support the intended authority model:

- local API remains canonical
- connector remains thin
- lease semantics come from the local API contract, not an invented connector
  shadow model

That makes promotion much safer than if the connector had started owning
independent coordination state.

## Why This Should Still Be Experimental

The recommendation is **supported experimental adapter**, not
fully-general production adapter.

That qualifier remains appropriate because:

- the session model is still process-scoped
- there is no multi-daemon coordination claim
- there is no production auth/deployment stack claim
- the route surface is still intentionally selective
- the client fixture is strong evidence, but not the same thing as a mature SDK

So the right move is not to keep calling it “only a prototype,” but also not to
oversell it as a production-complete broker system.

## Recommended Wording For Repo Status

Recommended concise status:

- `codexw` now has a supported experimental broker adapter for its documented
  broker-facing contract.
- The supported claim applies to the documented route, error, event, lease, and
  unsupported-boundary surface.
- Full broker parity and production deployment semantics remain out of scope.

## Recommended Immediate Follow-Ons

Promotion should change the framing of next work.

The next high-leverage tasks should be:

1. keep adding adversarial proof as hardening, not as proof that the contract is
   still undefined
2. keep those optional hardening items centralized in
   [codexw-broker-hardening-catalog.md](codexw-broker-hardening-catalog.md)
   instead of leaving them phrased as active blockers in status docs
3. tighten doc wording anywhere that still says the stack is merely a prototype
   when it is actually a supported experimental adapter
4. preserve the unsupported boundary explicitly so promotion does not silently
   expand the claim surface
5. keep the artifact-contract track explicit as design-only until local routes,
   connector mapping decisions, and process-level proof actually exist
6. keep the project/dependency collaboration-metadata track explicit as
   design-only until local routes, connector mapping decisions, and
   process-level proof actually exist
7. only reopen the promotion decision if the connector starts needing shadow
   state or if the local API authority model changes

## Decision Rule

Unless a newly discovered contradiction appears in the proof matrix, the
working recommendation should now be:

- **promote to supported experimental adapter**

not:

- keep as mere prototype by default

## Companion Docs

- [codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md)
- [codexw-broker-support-policy.md](codexw-broker-support-policy.md)
- [codexw-broker-adapter-promotion.md](codexw-broker-adapter-promotion.md)
- [codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md)
- [codexw-broker-adapter-status.md](codexw-broker-adapter-status.md)
- [codexw-broker-client-policy.md](codexw-broker-client-policy.md)
- [codexw-broker-out-of-scope.md](codexw-broker-out-of-scope.md)
- [codexw-broker-hardening-catalog.md](codexw-broker-hardening-catalog.md)
- [codexw-broker-artifact-contract-sketch.md](codexw-broker-artifact-contract-sketch.md)
- [codexw-broker-artifact-implementation-plan.md](codexw-broker-artifact-implementation-plan.md)
- [codexw-cross-project-dependency-contract-sketch.md](codexw-cross-project-dependency-contract-sketch.md)
- [codexw-cross-project-dependency-implementation-plan.md](codexw-cross-project-dependency-implementation-plan.md)
