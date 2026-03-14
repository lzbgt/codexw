# codexw Broker Artifact Implementation Plan

## Purpose

This document turns the artifact contract sketch into an implementation-facing
delivery plan.

It is intentionally narrower than the broader design note in
[codexw-broker-artifact-contract-sketch.md](codexw-broker-artifact-contract-sketch.md).
The sketch answers "what should this contract mean?" This plan answers
"what should be implemented first, from which runtime truth sources, and with
what proof?"

Companion docs:

- [codexw-broker-artifact-contract-sketch.md](codexw-broker-artifact-contract-sketch.md)
- [codexw-broker-host-examination-matrix.md](codexw-broker-host-examination-matrix.md)
- [codexw-local-api-event-sourcing.md](codexw-local-api-event-sourcing.md)
- [codexw-local-api-route-matrix.md](codexw-local-api-route-matrix.md)
- [codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md)

## Implementation Goal

Deliver the smallest artifact surface that materially improves broker-backed
app/WebUI clients without diluting the existing shell-first host-examination
model.

That first surface should provide:

- session-scoped artifact indexing
- artifact detail/provenance
- optional content fetch for artifact classes with clear backing content

It should not try to become a general filesystem browser.

## Proposed First Delivery Slice

### In Scope

- artifact index route
- artifact detail route
- normalization of artifact candidates from existing semantic runtime truth
- content fetch only for artifact classes with clear backing content
- broker-facing mapping/proof updates for any new local routes that actually land

### Explicitly Out Of Scope

- arbitrary path browsing
- write/upload routes
- global artifact search across sessions
- non-session-scoped artifact registry
- binary/media protocol expansion
- browser-specific preview rendering contracts

## Recommended Route Shape

The first likely route set is:

- `GET /api/v1/session/{session_id}/artifacts`
- `GET /api/v1/session/{session_id}/artifacts/{artifact_id}`
- `GET /api/v1/session/{session_id}/artifacts/{artifact_id}/content`

Recommended rule:

- do not implement the content route until at least one artifact class has a
  clear, bounded, and non-ambiguous content backing

That means the index/detail routes can land before content fetch if needed.

## Derivation Sources

The first implementation should derive artifacts from runtime truth already
owned elsewhere in the repo.

### 1. Transcript Items

Likely producers:

- assistant items that mention or summarize generated outputs
- file-change items that already have structured path lists
- tool results that already expose concrete path-oriented payloads

Why this is a good source:

- transcript items already have stable session and item identity
- they already appear in the semantic local API/event model

### 2. Shell Snapshots

Likely producers:

- background shell jobs whose structured snapshots reference explicit paths or
  outputs
- shell lifecycle/control results that can carry stable job refs and output
  metadata

Why this is a good source:

- shell jobs already have durable ids and session scoping
- they are already broker-visible and process-level proven

### 3. Service Interaction Results

Likely producers:

- service attach metadata
- recipe invocation results
- service endpoints and named interaction contracts

Why this is a good source:

- service jobs already have stable refs, labels, and capability relationships
- app/WebUI clients often need to render these as reusable runtime outputs

### 4. Client Events

Likely producers:

- explicitly published client-side acknowledgements or structured references

Why this is a useful secondary source:

- it provides a path for client-confirmed artifact visibility without scraping
  transcript lines later

## Normalization Rules

The first artifact layer should normalize only clearly identifiable results.

Good candidates:

- explicit filesystem paths
- structured file-change path sets
- stable service endpoints
- stable recipe result records
- structured client-published references

Bad candidates:

- arbitrary prose mentioning a filename
- raw terminal blocks with no stable path/result identity
- generic assistant summaries with no concrete result reference

## Suggested Ownership

The likely code ownership split should be:

- `local_api/snapshot.rs`
  - artifact candidate extraction helpers from current session state
- `local_api/routes/*`
  - artifact index/detail/content handlers once implemented
- `local_api/events.rs`
  - any event-side provenance hooks if artifact ids or references need replayable
    event alignment
- transcript and shell/service summary helpers
  - only as input sources, not as new transport authorities

The artifact layer should remain a thin view over existing state rather than a
second subsystem with its own truth model.

## Verification Plan

The first artifact implementation should require all of:

### Route-Level Tests

- artifact index on an empty session
- artifact index after transcript/file-change activity
- artifact index after shell/service activity
- artifact detail for a known artifact id
- stable error for unknown artifact id

### Provenance Tests

- artifact entries preserve session scoping
- artifact entries preserve source/provenance links such as item id, event id,
  or job ref
- artifact extraction does not require rendered terminal text parsing

### Broker/Connector Tests

- any new artifact route added to the local API must be explicitly mapped or
  explicitly rejected in the connector
- if mapped, add process-level smoke proof through the broker-style fixture path
- if not mapped yet, keep the unsupported boundary explicit in the docs

### Doc Consistency

- update source-of-truth docs and the support-doc guard together when the
  artifact contract shifts from sketch to implemented surface

## Delivery Order

The most coherent delivery order is:

1. finalize artifact normalization rules
2. land index route
3. land detail route
4. prove route/error/provenance behavior locally
5. decide whether the connector should expose artifact routes in the currently
   supported experimental adapter
6. only then consider content fetch

That sequence avoids promising download semantics before the repo even agrees on
artifact identity.

## Exit Criteria For The First Artifact Track

The first artifact track is complete when:

1. a session-scoped artifact index exists
2. each artifact entry has stable provenance back to transcript/event/shell/service truth
3. remote clients no longer have to scrape transcript or shell output just to
   enumerate artifact-like results
4. the broker proof/status docs explicitly state whether artifact routes are:
   - implemented and supported
   - implemented but out of the current supported adapter
   - still design-only
