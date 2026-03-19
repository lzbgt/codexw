# codexw Cross-Deployment Collaboration

## Purpose

This document records the next native requirement implied by the current
broker/client architecture:

`codexw` should not stop at broker-backed single-deployment client control. It
should also support collaboration between distinct `codexw` deployments,
including explicit work handoff.

This is a design note, not an implementation claim. It explains how
cross-deployment collaboration should fit the current `codexw` session,
identity, broker, and artifact boundaries.

For the first broker-visible handoff contract sketch and implementation-facing
delivery order, see
[codexw-cross-deployment-handoff-contract-sketch.md](codexw-cross-deployment-handoff-contract-sketch.md)
and
[codexw-cross-deployment-handoff-implementation-plan.md](codexw-cross-deployment-handoff-implementation-plan.md).
For the more specific case where multiple deployments each work on different
projects with dependency edges between them, see
[codexw-cross-project-dependency-collaboration.md](codexw-cross-project-dependency-collaboration.md).

For the source docs that define the current shell-first remote
host-examination surface that this cross-deployment collaboration track builds
on, see:

- [codexw-native-support-boundaries.md](codexw-native-support-boundaries.md)
- [codexw-local-api-sketch.md](codexw-local-api-sketch.md)
- [codexw-local-api-implementation-plan.md](codexw-local-api-implementation-plan.md)
- [codexw-local-api-event-sourcing.md](codexw-local-api-event-sourcing.md)
- [codexw-local-api-route-matrix.md](codexw-local-api-route-matrix.md)

## Why This Requirement Exists

The repo already treats these statements as required architecture:

- broker-backed app/WebUI clients are first-class consumers of `codexw`
- host examination is broker-visible and shell-first
- `session_id` is the stable remote-control handle

Once multiple broker-routed deployments exist, a new native requirement
appears:

- one deployment must be able to hand work to another deployment without
  collapsing into direct terminal access or vague out-of-band operator notes

Typical examples:

- a laptop-scoped `codexw` session hands build/test work to a stronger remote
  deployment
- one deployment hands a review or verification step to another deployment with
  a different trust boundary or tool posture
- an app/WebUI operator needs to move a long-running investigation from one
  deployment to another while preserving provenance

## Design Stance

Cross-deployment collaboration should be:

- broker-mediated rather than ad hoc deployment-to-deployment RPC
- session-scoped rather than thread-agnostic
- project-aware rather than deployment-aware only
- explicit rather than inferred from transcript text
- replayable through events rather than hidden in one-off side channels
- provenance-preserving rather than summary-only

The broker should remain the routing/control plane. The source of truth should
still be the per-deployment `codexw` runtime plus its local session state.
That is especially important because the collaborating deployments may not coexist on one host, so same-host assumptions cannot be part of the contract.

## Core Objects

The minimum design needs these concepts:

### 1. Source Deployment

The `codexw` deployment that currently owns the active work context.

Identity:

- `source_deployment_id`
- `source_session_id`
- `source_thread_id` when attached

### 2. Target Deployment

The intended receiving `codexw` deployment.

Identity:

- `target_deployment_id`
- optionally `target_session_id` once accepted or created

### 3. Handoff Record

A durable, replayable record that one deployment proposed work for another.

The handoff record should be its own object, not just a transcript note.

Minimum fields:

- `handoff_id`
- `source_deployment_id`
- `source_session_id`
- `target_deployment_id`
- `target_session_id` once bound
- `status`
- `created_at`
- `updated_at`
- `summary`
- `requested_outcome`
- `next_action`
- `provenance_refs`

### 4. Provenance References

Structured references back to the runtime truth that motivated the handoff.

Examples:

- transcript item ids
- event ids
- shell ids
- service ids
- artifact refs if and only if a future artifact surface exists

## Required Handoff Semantics

### Explicit proposal and acceptance

Work handoff should not silently mutate ownership across deployments.

The minimum flow is:

1. source deployment proposes handoff
2. target deployment accepts or declines
3. if accepted, the broker binds the handoff to a target session context
4. both sides emit replayable state transitions

### Status model

The handoff record should at least support:

- `proposed`
- `accepted`
- `declined`
- `in_progress`
- `completed`
- `cancelled`

### Session correlation

The design should preserve the current session identity model:

- `session_id` remains the remote-control handle inside one deployment
- handoff does not replace that with one global synthetic identity
- cross-deployment collaboration links sessions; it does not erase them

### Provenance over transcription

The receiving deployment should be able to understand:

- what work was being done
- why the handoff occurred
- what evidence or outputs matter

That means a handoff payload should contain both a human summary and structured
references back to transcript/event/shell/service truth where possible.

## Broker Role

The broker is the natural mediation plane for cross-deployment collaboration.

Responsibilities that belong in the broker-facing layer:

- deployment addressing
- handoff record persistence
- accept/decline state transitions
- replayable handoff event delivery
- routing the accepted handoff toward the target deployment

Responsibilities that should remain deployment-local:

- local session creation/attachment
- local transcript/event/shell/service truth
- actual turn execution
- actual host-side work

## Minimum API Shape

The exact route naming can change, but the first design should assume a route
family shaped roughly like this:

- `POST /api/v1/session/{session_id}/handoffs`
- `GET /api/v1/session/{session_id}/handoffs`
- `GET /api/v1/handoffs/{handoff_id}`
- `POST /api/v1/handoffs/{handoff_id}/accept`
- `POST /api/v1/handoffs/{handoff_id}/decline`
- `POST /api/v1/handoffs/{handoff_id}/complete`

And replayable events such as:

- `session_handoff_proposed`
- `session_handoff_accepted`
- `session_handoff_declined`
- `session_handoff_completed`

The first route family should stay narrow. It should not try to solve:

- global task scheduling across all deployments
- generic artifact sync
- multi-deployment lease consensus
- arbitrary filesystem replication

## Relationship To Artifact Track

Cross-deployment handoff is not the same problem as the artifact-contract gap.

The two should stay separate:

- handoff needs durable structured provenance
- a future artifact API may become one provenance source
- but handoff should not wait for a complete artifact index/detail/content
  surface to exist, because that artifact lane is still outside the current
  supported experimental adapter while the shell/service/transcript/event
  host-examination surface is already supported

The first handoff implementation should therefore rely on:

- summary text
- transcript refs
- event refs
- shell/service refs

and only include artifact ids when a real artifact surface exists.

## Relationship To Lease Policy

Cross-deployment handoff should not pretend lease policy disappears.

Important constraints:

- a source deployment may still have an active owner attachment on its local
  session
- a target deployment accepting a handoff does not automatically gain authority
  over the source session
- the collaboration object links work context; it does not imply shared mutable
  ownership of one local session

If a true control transfer is needed later, it should be modeled explicitly as
a separate lease/ownership policy decision.

## First Implementation Boundary

The first cross-deployment collaboration track is complete when all of these
are true:

1. a source deployment can create a replayable handoff record addressed to a
   target deployment
2. the target deployment can accept or decline it
3. acceptance binds the collaboration object to a target session context
4. app/WebUI clients can inspect handoff state through the broker surface
5. the handoff record preserves provenance back to source transcript/event/shell
   or service truth
6. the design does not claim artifact sync or global work orchestration it does
   not actually implement

## Sibling Workspace Implication

For the sibling `~/work/agent` workspace, this requirement means the broker and
WebUI should eventually grow a dedicated collaboration/handoff lane for
multiple `codexw` deployments rather than treating deployment switching as a
manual operator convention.

That work belongs after the already-supported single-deployment session/event/
shell/service baseline, but it is now an explicit architecture track rather
than an implied future possibility.
