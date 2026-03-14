# codexw Cross-Deployment Handoff Implementation Plan

## Purpose

This document turns the cross-deployment collaboration requirement and handoff
contract sketch into an implementation-facing delivery order.

It sits below:

- [codexw-cross-deployment-collaboration.md](codexw-cross-deployment-collaboration.md)
- [codexw-cross-deployment-handoff-contract-sketch.md](codexw-cross-deployment-handoff-contract-sketch.md)
- [codexw-cross-project-dependency-implementation-plan.md](codexw-cross-project-dependency-implementation-plan.md)

## Goal

Deliver the smallest end-to-end slice that lets one deployment propose work to
another deployment and lets broker-facing clients inspect and drive that
handoff without inventing a global scheduler.

Because the participating deployments may live on different hosts, this slice
must stay broker-mediated rather than relying on same-host deployment
coexistence.

This plan builds on the current supported experimental adapter boundary rather
than widening it silently. The already-supported broker-facing foundation is
the shell-first host-examination surface; this handoff lane adds explicit
collaboration metadata and workflow on top of that surface without implying
that broker-visible artifact index/detail/content routes are already part of
the supported adapter.

## First Deliverables

The first implementation slice should include:

- session-scoped handoff create/list/detail routes
- accept/decline/complete transitions
- replayable handoff events
- source/target deployment identity in the handoff payload
- source/target session correlation when acceptance binds a target session
- source/target project identity in the handoff payload
- dependency references when the handoff is caused by one project blocking on
  another

## Explicitly Deferred

The first slice should defer:

- multi-deployment lease transfer
- automatic task reassignment between deployments
- artifact replication or content fetch
- any broker-visible artifact index/detail/content route family
- cross-session global search over all handoffs
- non-broker direct deployment-to-deployment handoff transport
- any requirement that the collaborating deployments share one host

## Suggested Delivery Order

### 1. Route and model scaffolding

Add:

- handoff record type
- handoff status enum
- provenance ref type
- dependency ref type
- route skeletons for create/list/detail/accept/decline/complete

### 2. In-memory session-scoped persistence

Use the same local-API/session truth model that the current broker-visible
surfaces already follow.

The first implementation does not need durable storage across process restarts
if the rest of the handoff contract is still being proven, but it does need a
stable in-process record model and replayable events.

### 3. Event publication and replay

Add semantic event publication for:

- proposed
- accepted
- declined
- completed

The event stream should be enough for a broker/WebUI client to restore visible
handoff state after reconnect.

### 4. Session binding on accept

Acceptance should bind:

- source deployment/session identity
- source project identity
- target deployment identity
- target session identity
- target project identity

This is the first point where the handoff lane becomes more than a note.

### 5. Broker/connector mapping update

Once the local route family exists, decide how the connector exposes it. The
first connector story should be explicit and narrow, similar to the existing
broker-facing session/orchestration/shell mapping.

### 6. Proof surface

Add coverage for:

- create handoff
- list handoffs for a session
- inspect one handoff
- accept with target session binding
- decline
- completed terminal state
- replay visibility for proposed/accepted/declined/completed events

## Likely `codexw` Touch Points

The first implementation likely belongs near:

- `wrapper/src/local_api/routes/*`
- `wrapper/src/local_api/server.rs`
- `wrapper/src/local_api/events.rs`
- `wrapper/src/local_api/snapshot.rs`
- `wrapper/src/state/*` or a sibling session-scoped runtime store
- connector mapping/tests if the route family is exposed immediately

The model should also leave room for a project/dependency metadata layer that
the broker-facing surfaces can replay without pretending the deployments share
storage or a filesystem. Handoff provenance stays metadata-oriented and
shell/service/transcript/event-linked; it does not require a broker artifact
catalog to become part of the supported adapter first.

## Proof Expectations

The first implementation should prove all of:

- handoff creation preserves source session identity
- handoff creation preserves source project identity and dependency refs when
  present
- acceptance preserves target deployment identity and target session identity
- acceptance preserves target project identity
- invalid status transitions fail cleanly
- handoff events replay in a way broker/WebUI clients can consume
- handoff provenance refs do not require artifact routes
- the route family does not silently expand the supported experimental adapter
  from shell-first host examination into artifact index/detail/content routes
- broker-mediated collaboration does not assume same-host deployment discovery

## Relationship To The Sibling Workspace

The sibling `~/work/agent` workspace should not wait for the entire handoff
implementation to start its design work.

The current expectation is:

- implement today’s single-deployment session/event/shell/service baseline now
- treat cross-deployment handoff as the next collaboration lane
- keep collaboration broker-mediated because deployments may not share a host
- do not fake handoff by only switching deployment filters or by storing plain
  transcript notes where a future handoff object should exist

## Completion Bar

The first implementation track is complete when:

1. a handoff route family exists
2. a session-scoped handoff model exists
3. accept/decline/complete are replayable state transitions
4. broker/WebUI clients can render handoff state without inventing artifact
   routes
5. the proof/docs/status layer says exactly what is implemented, what remains
   deferred, and that the supported experimental adapter still stops at the
   shell-first host-examination surface
