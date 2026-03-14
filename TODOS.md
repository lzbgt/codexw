# codexw TODOs

This file is the repo-level backlog for work that is still concretely open.

It is intentionally derived from the current design, proof, and status docs
rather than used as a speculative product wishlist. When this file and the
deeper docs disagree, update both in the same batch.

Primary source docs:

- [docs/codexw-design.md](docs/codexw-design.md)
- [docs/codexw-native-gap-assessment.md](docs/codexw-native-gap-assessment.md)
- [docs/codexw-native-product-recommendation.md](docs/codexw-native-product-recommendation.md)
- [docs/codexw-native-support-policy.md](docs/codexw-native-support-policy.md)
- [docs/codexw-native-support-boundaries.md](docs/codexw-native-support-boundaries.md)
- [docs/codexw-native-product-status.md](docs/codexw-native-product-status.md)
- [docs/codexw-native-proof-matrix.md](docs/codexw-native-proof-matrix.md)
- [docs/codexw-native-hardening-catalog.md](docs/codexw-native-hardening-catalog.md)
- [docs/codexw-workspace-tool-policy.md](docs/codexw-workspace-tool-policy.md)
- [docs/codexw-broker-client-architecture.md](docs/codexw-broker-client-architecture.md)
- [docs/codexw-broker-adapter-contract.md](docs/codexw-broker-adapter-contract.md)
- [docs/codexw-broker-adapter-status.md](docs/codexw-broker-adapter-status.md)
- [docs/codexw-broker-client-policy.md](docs/codexw-broker-client-policy.md)
- [docs/codexw-broker-out-of-scope.md](docs/codexw-broker-out-of-scope.md)
- [docs/codexw-broker-proof-matrix.md](docs/codexw-broker-proof-matrix.md)
- [docs/codexw-broker-promotion-recommendation.md](docs/codexw-broker-promotion-recommendation.md)
- [docs/codexw-broker-support-policy.md](docs/codexw-broker-support-policy.md)
- [docs/codexw-broker-hardening-catalog.md](docs/codexw-broker-hardening-catalog.md)
- [docs/codexw-support-claim-checklist.md](docs/codexw-support-claim-checklist.md)

## Highest-Leverage Active Work

### 1. Brokered Client + Host Surface Alignment

Status:
- the sibling `~/work/agent` workspace already assumes broker-backed app/WebUI
  clients and broker-mediated host tooling
- `codexw` now has a verified broker/local-API adapter, but some top-level docs
  had still described brokered clients as future investigation rather than a
  required architecture direction

Concrete tasks:
- keep broker-exposed app/WebUI attachment as an explicit design requirement in:
  - README
  - design docs
  - broker docs
  - repo backlog
- keep broker-facing host shell examination explicit as a first-class part of
  the client surface, not as an accidental side effect of low-level routes
- keep the current shell-first host-examination posture explicit:
  - brokered host examination should use shell/service control, transcript, and
    artifact references
  - do not reintroduce removed workspace dynamic tools as the preferred remote
    inspection model
- make any remaining artifact-surface gaps explicit whenever broker/client docs
  claim that remote clients can examine host results

Primary source:
- [docs/codexw-broker-client-architecture.md](docs/codexw-broker-client-architecture.md)
- [docs/codexw-broker-connectivity.md](docs/codexw-broker-connectivity.md)
- [docs/codexw-broker-adapter-status.md](docs/codexw-broker-adapter-status.md)
- [docs/codexw-workspace-tool-policy.md](docs/codexw-workspace-tool-policy.md)

### 2. Broker Adapter Support Follow-Through

Status:
- the broker-facing adapter contract is now explicitly documented
- the current recommendation is to treat it as a supported experimental adapter
- the strongest remaining gaps are no longer missing routes or missing policy
  language

Concrete tasks:
- keep unsupported broker boundary enforcement explicit whenever new alias routes or connector features are added
- keep route, error, event, and lease behavior aligned between:
  - local API
  - connector alias layer
  - standalone broker client fixtures
  - broker-facing docs
- keep the current support-level wording aligned across:
  - README
  - broker status docs
  - promotion docs
  - proof docs
- keep README, status docs, and future release notes aligned with the current
  support-level claim
- avoid reintroducing stale wording that describes the adapter as “only a
  prototype” where the current docs now recommend supported experimental status
- if a newly discovered contradiction appears in the proof matrix, update:
  - [docs/codexw-broker-proof-matrix.md](docs/codexw-broker-proof-matrix.md)
  - [docs/codexw-broker-adapter-status.md](docs/codexw-broker-adapter-status.md)
  - [docs/codexw-broker-promotion-recommendation.md](docs/codexw-broker-promotion-recommendation.md)

Why this is still active:
- the broker stack is no longer blocked on missing contract definition
- the active work is now preserving a coherent supported experimental adapter surface,
  not proving that the contract exists at all

### 3. Native Product Gaps Outside The Broker Track

Status:
- command-level and protocol-level wrapper work is largely complete
- the main remaining gaps are architectural or UX-level

Concrete tasks:
- keep the terminal-first recommendation and support boundary explicit across:
  - README
  - native gap docs
  - native status/proof docs
  - native support policy docs
  - design docs
  - repo backlog
- only reopen alternate-screen/native TUI work if a concrete workflow is
  blocked by the current scrollback-first model
- only reopen audio/realtime expansion if a concrete supported target is chosen
- keep the wrapper-owned async shell boundary explicit whenever orchestration or
  local-API work expands
- keep optional native-side polish or parity ideas in
  [docs/codexw-native-hardening-catalog.md](docs/codexw-native-hardening-catalog.md)
  unless they become real support-level blockers

Primary source:
- [docs/codexw-design.md](docs/codexw-design.md)
- [docs/codexw-native-gap-assessment.md](docs/codexw-native-gap-assessment.md)
- [docs/codexw-native-product-recommendation.md](docs/codexw-native-product-recommendation.md)
- [docs/codexw-native-support-boundaries.md](docs/codexw-native-support-boundaries.md)
- [docs/codexw-native-product-status.md](docs/codexw-native-product-status.md)
- [docs/codexw-native-proof-matrix.md](docs/codexw-native-proof-matrix.md)

## Secondary Work

### 4. Optional Broker Hardening Catalog Maintenance

Concrete tasks:
- keep optional churn, replay, and adversarial workflow ideas in
  [docs/codexw-broker-hardening-catalog.md](docs/codexw-broker-hardening-catalog.md)
  instead of treating them as active blockers by default
- move an item from the hardening catalog back into this backlog only if:
  - a regression appears
  - a contradiction appears
  - the supported contract expands

### 5. Documentation Hygiene

Concrete tasks:
- keep the top-level docs synchronized so the same status does not have to be inferred from multiple long design files
- prefer adding or updating source-of-truth docs over copying large overlapping sections into multiple files
- keep this file updated when major broker or local-API milestones land
- use [docs/codexw-support-claim-checklist.md](docs/codexw-support-claim-checklist.md)
  whenever a batch changes support-level wording across broker/native docs

### 6. Structural Cleanup When It Compounds

Concrete tasks:
- continue splitting oversized files only when the split removes a real maintenance hotspot
- prefer coherent namespace/test splits over single-file cosmetic churn
- keep test and production module layouts aligned when behavior is split

## Explicitly Not This File

This file is not:
- a vague roadmap
- a speculative feature wishlist
- a replacement for the detailed broker/local-API design docs

It is the short, visible backlog that answers the practical question:

- what concrete work is still left right now?
