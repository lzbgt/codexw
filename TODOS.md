# codexw TODOs

This file is the repo-level backlog for work that is still concretely open.

It is intentionally derived from the current design, proof, and status docs
rather than used as a speculative product wishlist. When this file and the
deeper docs disagree, update both in the same batch.

Primary source docs:

- [docs/codexw-design.md](docs/codexw-design.md)
- [docs/codexw-native-gap-assessment.md](docs/codexw-native-gap-assessment.md)
- [docs/codexw-native-product-recommendation.md](docs/codexw-native-product-recommendation.md)
- [docs/codexw-native-support-boundaries.md](docs/codexw-native-support-boundaries.md)
- [docs/codexw-native-product-status.md](docs/codexw-native-product-status.md)
- [docs/codexw-native-proof-matrix.md](docs/codexw-native-proof-matrix.md)
- [docs/codexw-broker-prototype-status.md](docs/codexw-broker-prototype-status.md)
- [docs/codexw-broker-proof-matrix.md](docs/codexw-broker-proof-matrix.md)
- [docs/codexw-broker-promotion-recommendation.md](docs/codexw-broker-promotion-recommendation.md)
- [docs/codexw-broker-support-policy.md](docs/codexw-broker-support-policy.md)
- [docs/codexw-broker-hardening-catalog.md](docs/codexw-broker-hardening-catalog.md)

## Highest-Leverage Active Work

### 1. Broker Adapter Support Follow-Through

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

Why this is still active:
- the broker stack is no longer blocked on missing contract definition
- the active work is now preserving a coherent supported-experimental surface,
  not proving that the contract exists at all

### 2. Native Product Gaps Outside The Broker Track

Status:
- command-level and protocol-level wrapper work is largely complete
- the main remaining gaps are architectural or UX-level

Concrete tasks:
- keep the terminal-first recommendation and support boundary explicit across:
  - README
  - native gap docs
  - native status/proof docs
  - design docs
  - repo backlog
- only reopen alternate-screen/native TUI work if a concrete workflow is
  blocked by the current scrollback-first model
- only reopen audio/realtime expansion if a concrete supported target is chosen
- keep the wrapper-owned async shell boundary explicit whenever orchestration or
  local-API work expands

Primary source:
- [docs/codexw-design.md](docs/codexw-design.md)
- [docs/codexw-native-gap-assessment.md](docs/codexw-native-gap-assessment.md)
- [docs/codexw-native-product-recommendation.md](docs/codexw-native-product-recommendation.md)
- [docs/codexw-native-support-boundaries.md](docs/codexw-native-support-boundaries.md)
- [docs/codexw-native-product-status.md](docs/codexw-native-product-status.md)
- [docs/codexw-native-proof-matrix.md](docs/codexw-native-proof-matrix.md)

### 3. Promotion Follow-Through

Status:
- the docs now recommend promotion to a supported experimental adapter
- the repo still needs to keep that recommendation coherent everywhere

Concrete tasks:
- keep README, status docs, and future release notes aligned with the current support-level claim
- avoid reintroducing stale wording that describes the adapter as “only a prototype” where the current docs now recommend supported experimental status
- if a newly discovered contradiction appears in the proof matrix, update:
  - [docs/codexw-broker-proof-matrix.md](docs/codexw-broker-proof-matrix.md)
  - [docs/codexw-broker-prototype-status.md](docs/codexw-broker-prototype-status.md)
  - [docs/codexw-broker-promotion-recommendation.md](docs/codexw-broker-promotion-recommendation.md)

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
