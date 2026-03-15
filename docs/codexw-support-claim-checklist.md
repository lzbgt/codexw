# codexw Support Claim Checklist

This document is the operational checklist for keeping `codexw`'s public
support claims internally consistent.

It exists because the repo now has:

- broker-side recommendation / contract / proof / support-policy docs
- native-side recommendation / status / proof / support-policy docs
- a visible repo backlog in [../TODOS.md](../TODOS.md)

Those docs explain *what* the current supported shapes are. This checklist is
the shorter companion that answers:

- what must be reviewed before changing support-level wording
- which docs must stay aligned
- what evidence is expected before a support claim is strengthened
- what wording regressions should be treated as bugs

## When To Use This Checklist

Use this checklist whenever a batch does one or more of:

- changes README support wording
- changes a status document
- changes a recommendation or promotion document
- changes a support policy document
- changes proof-matrix conclusions
- changes route families, error envelopes, event semantics, or lease rules on
  the broker side
- changes the native product boundary or reopens previously unsupported native
  parity work

If the batch changes behavior *and* changes support-level language, this
checklist should be reviewed in the same turn.

## Global Rules

### 1. Status, Recommendation, Policy, And Proof Must Agree

Do not let one doc say:

- "supported experimental adapter"

while another still says:

- "only a prototype"

unless the difference is deliberate and explicitly explained.

Likewise, do not let native docs claim:

- "terminal-first supported product"

while another doc casually implies:

- alternate-screen or audio parity is part of the active support claim

without an explicit policy change.

### 2. Evidence Comes Before Stronger Claims

If support language becomes stronger, the proof surface must also be reviewed.

At minimum:

- route and contract claims should be covered by route/unit tests
- policy-sensitive claims should be covered by process-level smoke or fixture
  proof where practical
- source-of-truth status/policy/proof docs should continue to satisfy the
  automated consistency guard in
  [../wrapper/tests/doc_consistency.rs](../wrapper/tests/doc_consistency.rs)
- proof docs should say clearly whether the claim is:
  - recommendation only
  - supported shape
  - supported experimental adapter
  - optional hardening / future work

### 3. Unsupported Boundaries Must Stay Explicit

The repo should not drift into vague wording such as:

- "future parity work"
- "not done yet"
- "prototype limitations"

when the actual state is:

- intentionally unsupported boundary
- optional hardening
- deferred by product decision

## Broker Checklist

When broker-facing wording changes, review:

- [codexw-broker-client-architecture.md](codexw-broker-client-architecture.md)
- [codexw-broker-compatibility-target.md](codexw-broker-compatibility-target.md)
- [codexw-broker-connector-decision.md](codexw-broker-connector-decision.md)
- [codexw-broker-connector-mapping.md](codexw-broker-connector-mapping.md)
- [codexw-broker-session-identity.md](codexw-broker-session-identity.md)
- [codexw-broker-client-policy.md](codexw-broker-client-policy.md)
- [codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md)
- [codexw-broker-support-policy.md](codexw-broker-support-policy.md)
- [codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md)
- [codexw-broker-adapter-status.md](codexw-broker-adapter-status.md)
- [codexw-broker-promotion-recommendation.md](codexw-broker-promotion-recommendation.md)
- [codexw-broker-adapter-promotion.md](codexw-broker-adapter-promotion.md)
- [codexw-broker-out-of-scope.md](codexw-broker-out-of-scope.md)
- [codexw-broker-hardening-catalog.md](codexw-broker-hardening-catalog.md)
- [codexw-broker-host-examination-matrix.md](codexw-broker-host-examination-matrix.md)
- [codexw-broker-integration-handoff.md](codexw-broker-integration-handoff.md)
- [codexw-cross-project-dependency-collaboration.md](codexw-cross-project-dependency-collaboration.md)
- [codexw-cross-project-dependency-contract-sketch.md](codexw-cross-project-dependency-contract-sketch.md)
- [codexw-cross-project-dependency-implementation-plan.md](codexw-cross-project-dependency-implementation-plan.md)
- [codexw-broker-artifact-contract-sketch.md](codexw-broker-artifact-contract-sketch.md)
- [codexw-broker-artifact-implementation-plan.md](codexw-broker-artifact-implementation-plan.md)

Confirm all of the following:

- the support level still says `supported experimental adapter` if that remains
  true
- broker-backed app/WebUI clients still read as part of the intended
  architecture, not as a hypothetical future
- broker-visible host shell examination still reads as a first-class supported
  workflow for the current shell/service/transcript/event surface
- no doc regresses to "prototype only" wording for the documented supported
  broker surface
- current-state broker docs do not regress to stale wording like
  `current prototype`, `prototype proof set`, or `prototype behavior note`
- the current broker fixture diversity claim stays explicit:
  - standalone broker-style client fixtures in Python and Node
  - `scripts/codexw_broker_client.py`
  - `scripts/codexw_broker_client_node.mjs`
- if the documented broker supervision contract changes, the standalone Python
  and Node fixtures should still prove enriched `status.updated` replay for
  `supervision_notice` and `async_tool_backpressure`, not just generic event
  streaming
- the automated doc guard in
  [../wrapper/tests/doc_consistency.rs](../wrapper/tests/doc_consistency.rs)
  still passes for the broker status/policy/proof set
- unsupported broker routes are still described as intentionally unsupported,
  not merely missing
- hardening ideas remain in the hardening catalog unless they became active
  blockers
- the proof matrix still matches the strongest verified route / error / event /
  lease claims
- the current supported claim still clearly excludes artifact index/detail/
  content routes unless those routes were explicitly implemented and proven in
  the same batch
- README and TODO/backlog docs still summarize the same support boundary:
  supported experimental adapter means the current shell-first
  host-examination surface, while artifact routes remain outside that boundary
  until the separate artifact track is implemented and proven
- artifact sketch/plan/host-examination docs still say the supported
  experimental adapter ends at the current shell-first host-examination
  surface, with artifact routes remaining design-only until contract, proof,
  and policy updates land together
- the current supported claim also clearly excludes the design-only
  project-assignment and dependency-edge route family unless those routes were
  explicitly implemented and proven in the same batch
- host-examination docs still distinguish:
  - already-supported shell/service/transcript/event remote inspection
  - design-only artifact contract work
- collaboration docs still distinguish:
  - already-supported single-deployment broker control
  - design-only project/dependency collaboration metadata
  - design-only cross-deployment handoff transport above that metadata layer
- the implementer-facing handoff doc for the sibling `~/work/agent` workspace
  still matches the current support, proof, and boundary docs
- the older broker decision / compatibility / session-identity / client-policy
  docs still describe the same client surface as the status / support / proof
  docs instead of lagging behind with narrower historical wording

If the broker contract changes, also confirm:

- the connector allowlist docs still match the implemented route surface
- local API docs and connector docs describe the same broker-facing behavior
- fixture docs still match the currently verified reference clients in Python
  and Node
- fixture docs still describe shell-first host examination as current reality
  and artifact browsing as a separate design-only lane until explicit artifact
  routes exist

## Native Checklist

When native-side wording changes, review:

- [codexw-native-gap-assessment.md](codexw-native-gap-assessment.md)
- [codexw-native-product-recommendation.md](codexw-native-product-recommendation.md)
- [codexw-native-support-policy.md](codexw-native-support-policy.md)
- [codexw-native-support-boundaries.md](codexw-native-support-boundaries.md)
- [codexw-native-product-status.md](codexw-native-product-status.md)
- [codexw-native-proof-matrix.md](codexw-native-proof-matrix.md)
- [codexw-native-gap-assessment.md](codexw-native-gap-assessment.md)
- [codexw-native-hardening-catalog.md](codexw-native-hardening-catalog.md)
- [codexw-workspace-tool-policy.md](codexw-workspace-tool-policy.md)
- [codexw-local-api-sketch.md](codexw-local-api-sketch.md)
- [codexw-local-api-implementation-plan.md](codexw-local-api-implementation-plan.md)
- [codexw-local-api-event-sourcing.md](codexw-local-api-event-sourcing.md)
- [codexw-local-api-route-matrix.md](codexw-local-api-route-matrix.md)

Confirm all of the following:

- the product is still described as terminal-first / scrollback-first if that
  remains the active recommendation
- the automated doc guard in
  [../wrapper/tests/doc_consistency.rs](../wrapper/tests/doc_consistency.rs)
  still passes for the native status/policy/proof set
- alternate-screen, audio, and backend-owned async parity are still explicit
  unsupported areas unless there is an intentional policy change
- optional polish or parity ideas remain in the native hardening catalog unless
  they became active blockers
- status docs do not casually imply that unsupported parity work is an active
  shipping commitment
- native source docs still describe the same shell-first host-examination
  support boundary as the downstream status/policy/proof docs, rather than
  leaving recommendation, boundaries, or gap-assessment text behind
- native recommendation/boundary/status/policy/proof docs still point to the
  native source docs and workspace/local-API source docs that define that
  shell-first remote/workspace surface, rather than only pointing at summary
  or downstream policy docs
- workspace and local-API source docs still describe shell-first remote host
  examination as the current supported surface and do not imply that a
  broker-visible artifact list/detail/content API already exists

## README / Backlog Checklist

Whenever support wording changes, review:

- [../README.md](../README.md)
- [../TODOS.md](../TODOS.md)
- [codexw-design.md](codexw-design.md)

Confirm all of the following:

- README points to the current source-of-truth docs instead of stale summary
  wording
- README still points to the native source docs and workspace/local-API source
  docs that define the shell-first host-examination support boundary behind the
  current support claim
- `TODOS.md` distinguishes active support-level work from optional hardening
- `TODOS.md` still lists the native source docs and workspace/local-API source
  docs as primary source material for the active support-boundary work, rather
  than only listing downstream status or policy summaries
- design docs do not contradict the current recommendation/status docs
- design docs still point to the same native source docs and workspace/local-API
  source docs that define the shell-first remote/workspace support boundary,
  instead of only pointing at downstream status/policy summaries

## Release Note / Change Summary Checklist

If you write release notes, milestone notes, or a status summary outside the
repo, the wording should pass this check:

### Broker-side allowed wording

- "supported experimental adapter"
- "documented broker-facing adapter contract"
- "explicit unsupported broker boundary"
- "proof-backed connector/local-API surface"

### Broker-side wording to avoid unless truly accurate

- "only a prototype"
- "best effort only"
- "fully stable public broker platform"

### Native-side allowed wording

- "terminal-first supported product shape"
- "scrollback-first interactive client"
- "wrapper-owned async shell model"
- "explicitly unsupported alternate-screen/audio/backend-owned parity"

### Native-side wording to avoid unless the policy changed

- "full native Codex parity is an active commitment"
- "audio support is part of the current product promise"
- "alternate-screen parity is expected by default"

## If A Contradiction Is Found

If you find a contradiction, do not fix only one file.

Update the smallest coherent set together:

- recommendation or promotion doc
- support policy
- status snapshot
- proof matrix
- README / TODO if the contradiction is user-visible

The goal is to keep support claims synchronized, not merely locally corrected.

## Companion Docs

- [../README.md](../README.md)
- [../TODOS.md](../TODOS.md)
- [codexw-broker-support-policy.md](codexw-broker-support-policy.md)
- [codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md)
- [codexw-broker-adapter-status.md](codexw-broker-adapter-status.md)
- [codexw-native-support-policy.md](codexw-native-support-policy.md)
- [codexw-native-product-status.md](codexw-native-product-status.md)
- [codexw-native-proof-matrix.md](codexw-native-proof-matrix.md)
