# codexw Broker Hardening Catalog

This document is the home for optional broker-adapter hardening work that is
useful, evidence-increasing, and worth tracking, but is **not** a blocker for
the current supported experimental adapter recommendation.

Use this document to keep future stress ideas visible without overstating them
as missing contract basics in the backlog or adapter-status docs.

## Purpose

This catalog exists to separate two different kinds of remaining work:

- active support and consistency work needed to preserve the currently claimed
  adapter contract
- optional hardening that may strengthen confidence further but does not by
  itself mean the adapter contract is still undefined

The current broker recommendation remains:

- promote the current stack to a **supported experimental adapter** for the
  documented contract

That means items in this catalog are not promotion blockers by default.

For the source docs that define the current shell-first remote
host-examination surface that this hardening catalog is strengthening, see:

- [codexw-workspace-tool-policy.md](codexw-workspace-tool-policy.md)
- [codexw-local-api-sketch.md](codexw-local-api-sketch.md)
- [codexw-local-api-implementation-plan.md](codexw-local-api-implementation-plan.md)
- [codexw-local-api-event-sourcing.md](codexw-local-api-event-sourcing.md)
- [codexw-local-api-route-matrix.md](codexw-local-api-route-matrix.md)

## How To Use This Catalog

An item belongs here when it is:

- a stronger stress or adversarial scenario
- a broader repetition of already-proven behavior
- an evidence-strengthening extension above the currently documented contract
- useful for regression defense, but not required to explain what the adapter
  surface currently is

An item should move back into the active repo backlog only if:

- a regression appears
- a contradiction appears between current docs and current proof
- the supported contract expands
- a newly discovered consumer requirement changes the promotion judgment

## Current Catalog

### 1. Sustained Multi-Client Churn

Examples:

- longer lease churn with more than two role transitions in one workflow
- repeated renew/release/takeover cycles mixed with transcript and
  orchestration reads
- mixed shell, service, and `client_event` mutation churn under one long-lived
  session

Why useful:

- strengthens confidence that current role and lease behavior remains stable
  under repetition

Why not a blocker:

- named and anonymous contention, explicit handoff, repeated role reversal, and
  observer-readable contention are already process-level proven

### 2. Event Resume Stress Beyond Current Replay Workflows

Examples:

- more repeated `Last-Event-ID` reconnect cycles in one workflow
- reconnect after multiple semantic event families, not only capability updates
- reconnect after client-event publication mixed with service mutation

Why useful:

- hardens confidence in replay semantics for remote clients

Why not a blocker:

- SSE plus `Last-Event-ID` replay is already process-level proven across core
  session, service, lease, and client-event workflows
- the connector now also has direct unit coverage for the header/body seam
  case where the first upstream `data:` line is fragmented before wrapping

### 3. Additional Unsupported-Boundary Defense

Examples:

- broader negative matrices for unsupported route families
- fuzzier malformed broker alias shapes
- explicit negative coverage for newly invented alias families when the
  connector surface expands

Why useful:

- protects the “thin adapter with explicit unsupported boundary” claim

Why not a blocker:

- the current unsupported boundary is already process-level defended for
  unknown aliases, out-of-allowlist raw proxy routes, unsupported global
  broker routes, and named out-of-scope route families like `scene`

### 4. Version-Evolution Readiness

Examples:

- compatibility tests across future adapter-version changes
- explicit downgrade/upgrade handling expectations for remote clients
- stronger assertions around version headers and body enrichment in fixture
  workflows

Why useful:

- makes future contract evolution safer once external consumers appear

Why not a blocker:

- the current version contract is already documented and exercised for the
  presently claimed adapter surface

### 5. External Consumer Diversity

Examples:

- an additional consumer shape beyond the current Python and Node fixtures
- a tiny connector consumer that simulates a longer-lived remote dashboard
- a narrowly scoped browser-side or Node-side consumer proof
- once artifact routes exist, a focused app/WebUI-style consumer proof that
  verifies artifact browsing without falling back to transcript scraping

Why useful:

- increases confidence that the adapter contract is not accidentally
  overfitted to one fixture

Why not a blocker:

- the current standalone broker-style fixtures already give the repo real
  external consumer shape with process-level proof
- the richer artifact-centric consumer story is still design-only until the
  artifact-contract track becomes implemented surface and joins the supported
  experimental adapter

## Relationship To `TODOS.md`

`TODOS.md` should track:

- active support follow-through
- consistency maintenance
- genuine remaining product or architecture gaps

This catalog should track:

- optional hardening ideas that are worthwhile but not urgent blockers

If an item here becomes urgent because of a regression or contradiction, move
it into `TODOS.md` in the same batch that documents the reason.

## Companion Docs

- [codexw-broker-adapter-status.md](codexw-broker-adapter-status.md)
- [codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md)
- [codexw-broker-promotion-recommendation.md](codexw-broker-promotion-recommendation.md)
- [codexw-broker-adapter-promotion.md](codexw-broker-adapter-promotion.md)
- [../TODOS.md](../TODOS.md)
