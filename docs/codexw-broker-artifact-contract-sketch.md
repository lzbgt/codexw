# codexw Broker Artifact Contract Sketch

## Purpose

This document defines the next design slice after the broker host-examination
matrix:

- `codexw` already has a broker-visible host-examination foundation through
  transcript, event, shell, and service surfaces
- `codexw` does **not** yet have a dedicated broker-visible artifact catalog or
  artifact fetch contract

This document sketches what that future artifact contract should mean, without
claiming it already exists.

Companion docs:

- [codexw-broker-client-architecture.md](codexw-broker-client-architecture.md)
- [codexw-broker-host-examination-matrix.md](codexw-broker-host-examination-matrix.md)
- [codexw-broker-artifact-implementation-plan.md](codexw-broker-artifact-implementation-plan.md)
- [codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md)
- [codexw-local-api-event-sourcing.md](codexw-local-api-event-sourcing.md)
- [codexw-local-api-route-matrix.md](codexw-local-api-route-matrix.md)

## Problem Statement

Remote clients can already learn a lot from the current broker-facing surface:

- transcript snapshots
- semantic SSE events
- shell snapshots and control replies
- service attachment metadata and recipe results

That is sufficient for many engineering workflows, but it is not the same as a
stable artifact contract. Richer app/WebUI clients eventually need something
more structured than "parse transcript and shell references."

## Core Principle

If `codexw` grows a broker-visible artifact surface, it should be:

- derived from existing runtime truth
- session-scoped
- semantic rather than terminal-render-derived
- thin enough that the local API remains canonical

It should **not** introduce a second independent artifact database just to make
broker clients easier.

## Current Artifact Sources

Today, artifact-like information appears through:

- `transcript.item`
  - assistant output
  - command execution
  - file changes
  - tool results
- `shell.updated`
  - running host command output and job state
- `service.updated`
  - reusable service readiness, metadata, and interaction contracts
- `client.event`
  - client-side acknowledgements or published structured state
- explicit path, endpoint, label, or recipe-result text carried by those
  surfaces

Any future artifact contract should normalize information that already exists in
those producers instead of scraping rendered terminal text later.

## What Counts As An Artifact Here

For `codexw`, an artifact should mean one of these:

- a concrete filesystem path produced or referenced by the session
- a structured file-change result already represented in transcript state
- a shell-produced result with stable semantic identity
- a service interaction result with stable semantic identity
- a future explicitly declared result object emitted by the runtime

This is intentionally broader than only "downloadable files," but narrower than
"any arbitrary transcript line."

## Proposed Contract Shape

The likely artifact contract should split into three concerns.

### 1. Artifact Index

Purpose:

- let a remote client list session-scoped artifacts without scraping the whole
  transcript

Likely route family:

- `GET /api/v1/session/{session_id}/artifacts`

Candidate query filters:

- `kind=path|file_change|shell_result|service_result|client_event`
- `source=transcript|shell|service|client`
- `limit`
- `before`

Candidate response fields:

- `artifact_id`
- `session_id`
- `kind`
- `source`
- `created_at_ms`
- `title`
- `summary`
- `path`
- `mime`
- `size_bytes`
- `producer`
- `event_id`
- `item_id`
- `job_ref`

### 2. Artifact Detail

Purpose:

- let a remote client inspect one artifact’s metadata without downloading its
  contents

Likely route:

- `GET /api/v1/session/{session_id}/artifacts/{artifact_id}`

Candidate fields:

- everything from the index entry
- richer provenance
- stable references back to transcript item / event / shell job / service job
- whether content fetch is currently supported

### 3. Artifact Content Fetch

Purpose:

- let a remote client fetch content when the artifact represents something
  concrete and retrievable

Likely route:

- `GET /api/v1/session/{session_id}/artifacts/{artifact_id}/content`

This route should be optional by artifact type. Some artifacts may be metadata
only and should expose detail without content fetch.

## What The First Artifact Contract Should Not Try To Solve

The first artifact contract should not claim:

- generic filesystem browsing
- arbitrary host-path read access
- upload semantics
- permanent content-addressed storage
- cross-session artifact deduplication
- full `agentd` artifact parity
- browser-grade preview schemas for every file type

Those would expand the problem too early and risk rebuilding the removed
workspace tooling under a different name.

## Relationship To Shell-First Host Examination

Even with an artifact contract, shell/service/transcript/event surfaces should
remain primary for open-ended investigation.

The artifact contract should help with:

- stable indexing
- richer app/WebUI rendering
- cleaner download/fetch semantics

It should not replace host shell access as the general-purpose examination
substrate.

## Likely Derivation Strategy

The cleanest derivation path is:

1. publish semantic runtime events from existing mutation points
2. normalize artifact candidates from those event/transcript/shell/service
   payloads
3. expose a session-scoped artifact index/detail view
4. add content fetch only for artifact classes with clear backing content

That keeps the artifact layer aligned with the same truth sources already named
in [codexw-local-api-event-sourcing.md](codexw-local-api-event-sourcing.md).

## Near-Term Decision Rule

When a broker/client request mentions artifacts, ask:

1. Is the need already satisfied by transcript/event/shell/service inspection?
2. If not, is the missing piece stable indexing/metadata/fetch semantics?
3. If yes, it belongs in the artifact-contract track.
4. If the request is really open-ended host inspection, keep it on the
   shell-first track instead.

## Status

This document is a design sketch only.

For the implementation-facing delivery order behind this sketch, see
[codexw-broker-artifact-implementation-plan.md](codexw-broker-artifact-implementation-plan.md).

Current fact:

- `codexw` does not yet implement the routes or proof required for a supported
  broker-visible artifact contract

Current value:

- it makes the remaining gap concrete enough to implement without conflating it
  with generic shell access or with the removed workspace tool family
