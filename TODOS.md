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
- [docs/codexw-cross-deployment-collaboration.md](docs/codexw-cross-deployment-collaboration.md)
- [docs/codexw-cross-project-dependency-collaboration.md](docs/codexw-cross-project-dependency-collaboration.md)
- [docs/codexw-cross-project-dependency-contract-sketch.md](docs/codexw-cross-project-dependency-contract-sketch.md)
- [docs/codexw-cross-project-dependency-implementation-plan.md](docs/codexw-cross-project-dependency-implementation-plan.md)
- [docs/codexw-self-evolution.md](docs/codexw-self-evolution.md)
- [docs/codexw-self-evolution-implementation-plan.md](docs/codexw-self-evolution-implementation-plan.md)
- [docs/codexw-self-supervision.md](docs/codexw-self-supervision.md)
- [docs/codexw-self-supervision-implementation-plan.md](docs/codexw-self-supervision-implementation-plan.md)
- [docs/codexw-background-execution-boundary.md](docs/codexw-background-execution-boundary.md)
- [docs/codexw-plugin-system.md](docs/codexw-plugin-system.md)
- [docs/codexw-plugin-system-implementation-plan.md](docs/codexw-plugin-system-implementation-plan.md)
- [docs/codexw-cross-deployment-handoff-contract-sketch.md](docs/codexw-cross-deployment-handoff-contract-sketch.md)
- [docs/codexw-cross-deployment-handoff-implementation-plan.md](docs/codexw-cross-deployment-handoff-implementation-plan.md)
- [docs/codexw-broker-host-examination-matrix.md](docs/codexw-broker-host-examination-matrix.md)
- [docs/codexw-broker-integration-handoff.md](docs/codexw-broker-integration-handoff.md)
- [docs/codexw-broker-artifact-contract-sketch.md](docs/codexw-broker-artifact-contract-sketch.md)
- [docs/codexw-broker-artifact-implementation-plan.md](docs/codexw-broker-artifact-implementation-plan.md)
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
- keep the new cross-deployment collaboration requirement explicit:
  - cross-deployment `codexw` collaboration should be broker-mediated
  - the broker mediation is required because deployments may not share a host
  - collaboration should stay project-aware when each deployment is advancing a
    different dependent project
  - dependency edges between projects should be explicit collaboration context
  - project assignment and dependency-edge metadata should have their own narrow
    broker-visible route family instead of being buried only inside handoff
    payloads
  - work handoff should be session-scoped and replayable
  - work handoff should preserve provenance rather than relying on transcript
    prose alone
- keep the first handoff route/event track narrow:
  - explicit handoff record
  - explicit proposed/accepted/declined/completed states
  - no implied artifact replication or global scheduler
- make any remaining artifact-surface gaps explicit whenever broker/client docs
  claim that remote clients can examine host results
- keep the current workflow-level host-examination read aligned across:
  - architecture docs
  - broker status/proof docs
  - README
- treat a future dedicated artifact inventory/fetch contract as a separate gap
  from the already-supported shell/service/transcript remote-control surface
- when artifact-centric app/WebUI requirements get concrete, drive them through
  the artifact-contract track instead of expanding shell or transcript routes
  ad hoc

Primary source:
- [docs/codexw-broker-client-architecture.md](docs/codexw-broker-client-architecture.md)
- [docs/codexw-cross-deployment-collaboration.md](docs/codexw-cross-deployment-collaboration.md)
- [docs/codexw-cross-project-dependency-collaboration.md](docs/codexw-cross-project-dependency-collaboration.md)
- [docs/codexw-cross-project-dependency-contract-sketch.md](docs/codexw-cross-project-dependency-contract-sketch.md)
- [docs/codexw-cross-project-dependency-implementation-plan.md](docs/codexw-cross-project-dependency-implementation-plan.md)
- [docs/codexw-cross-deployment-handoff-contract-sketch.md](docs/codexw-cross-deployment-handoff-contract-sketch.md)
- [docs/codexw-cross-deployment-handoff-implementation-plan.md](docs/codexw-cross-deployment-handoff-implementation-plan.md)
- [docs/codexw-broker-host-examination-matrix.md](docs/codexw-broker-host-examination-matrix.md)
- [docs/codexw-broker-integration-handoff.md](docs/codexw-broker-integration-handoff.md)
- [docs/codexw-broker-artifact-contract-sketch.md](docs/codexw-broker-artifact-contract-sketch.md)
- [docs/codexw-broker-artifact-implementation-plan.md](docs/codexw-broker-artifact-implementation-plan.md)
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
- turn the new self-evolution requirement into a narrow implementation lane:
  - checkpoint current thread/cwd/draft/continuation state
  - launch a newer binary in an explicit self-handoff resume mode
  - gate old-process exit on explicit acknowledgment from the new process
  - keep rollback/manual resume explicit on failure
- keep the new self-supervision lane explicit:
  - dynamic tools and shell workflows should execute without wedging the input
    loop
  - long-running or stalled tool calls should be detectable as supervision
    events, not just operator anecdotes
  - the agent backend should be able to inspect dedicated async worker thread
    names and lifecycle states through `async_tool_workers`, while keeping it
    explicit that this is not yet a proof of in-worker forward progress
  - keep the main orchestrator responsible for deciding when to inspect an
    active async worker again based on the task scale it can actually observe
    locally, rather than treating health checks as a single fixed interval
  - keep the visible async-worker status explicit about whether completion or
    output has been observed yet, instead of leaving the operator with only a
    generic tool-name spinner
  - keep the ownership boundary explicit:
    - wrapper-owned `background_shell_*` async work is a `codexw` lane
    - app-server-owned `command/exec` and server-observed background terminals
      are a different lane
    - supervision and operator status should say which lane owns the current
      work instead of collapsing them into one generic “background task”
  - the runtime should decide whether to warn, interrupt, hand off, or replace
    itself rather than staying stuck indefinitely
- keep resume list/load latency bounded:
  - resume-history hydration should stay single-pass over turns for latest
    preview/state seeding work
  - thread-list rendering should avoid repeated sort/clone work once a
    response is already in memory
  - startup `resume`, plain `/resume`, and `/threads` should keep showing the
    last local recent-thread cache immediately while live `thread/list`
    refreshes remain in flight
- keep the plugin-first expansion rule explicit:
  - optional capabilities such as voice reminder or live IM reporting should
    land through the plugin system when core runtime contracts do not need to
    change
  - full binary self-evolution should remain for core fixes, protocol changes,
    or safety/runtime changes
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
- [docs/codexw-self-evolution.md](docs/codexw-self-evolution.md)
- [docs/codexw-self-evolution-implementation-plan.md](docs/codexw-self-evolution-implementation-plan.md)
- [docs/codexw-self-supervision.md](docs/codexw-self-supervision.md)
- [docs/codexw-self-supervision-implementation-plan.md](docs/codexw-self-supervision-implementation-plan.md)
- [docs/codexw-background-execution-boundary.md](docs/codexw-background-execution-boundary.md)
- [docs/codexw-plugin-system.md](docs/codexw-plugin-system.md)
- [docs/codexw-plugin-system-implementation-plan.md](docs/codexw-plugin-system-implementation-plan.md)

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
