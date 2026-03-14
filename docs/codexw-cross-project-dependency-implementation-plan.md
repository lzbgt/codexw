# codexw Cross-Project Dependency Implementation Plan

## Purpose

This document turns the cross-project dependency collaboration requirement into
an implementation-facing delivery order.

It sits below:

- [codexw-cross-project-dependency-collaboration.md](codexw-cross-project-dependency-collaboration.md)
- [codexw-cross-project-dependency-contract-sketch.md](codexw-cross-project-dependency-contract-sketch.md)

## Goal

Deliver the smallest end-to-end slice that lets broker-facing clients see:

- which project a session is working on
- which dependency edges connect the active projects
- which dependency edge a later handoff refers to

Because the participating deployments may not share a host, this slice must
stay broker-mediated and metadata-oriented rather than trying to share local
filesystems or direct process state.

This plan also stays inside the current supported experimental adapter
boundary. The already-supported broker-facing foundation is the shell-first
host-examination surface; project and dependency metadata should layer on top
of that surface without implying that artifact index/detail/content routes are
already supported.

## First Deliverables

The first implementation slice should include:

- session-scoped project assignment create/detail routes
- dependency-edge create/list/detail routes
- replayable assignment and dependency events
- dependency ids that handoff records can reference
- proof that the same model works across deployments that do not share a host

## Explicitly Deferred

The first slice should defer:

- automatic dependency discovery from repo analysis
- global project search across every deployment
- artifact replication or artifact download semantics
- any broker-visible artifact index/detail/content route family
- cross-host workspace synchronization
- automatic rescheduling of blocked work

## Suggested Delivery Order

### 1. Session project assignment model

Add:

- project record type
- session project assignment type
- assignment routes for bind/detail

This is the smallest way to make project context explicit without inventing a
full global project registry first.

### 2. Dependency-edge model

Add:

- dependency-edge type
- dependency state enum
- create/list/detail routes for dependency edges

The first model only needs enough state to explain blocking relationships and
to support later handoff references.

### 3. Event publication and replay

Add semantic event publication for:

- project bound
- dependency declared
- dependency updated

The event stream should be enough for a broker/WebUI client to restore the
visible project/dependency graph after reconnect.

### 4. Handoff integration

Once dependency ids exist, handoff creation should prefer referencing existing
dependency ids instead of creating one-off inline dependency models per handoff.

### 5. Connector mapping update

Expose the narrow project/dependency route family through the same broker
adapter path used for the existing session/event/orchestration/shell/service
surfaces.

## Likely `codexw` Touch Points

The first implementation likely belongs near:

- `wrapper/src/local_api/routes/*`
- `wrapper/src/local_api/events.rs`
- `wrapper/src/local_api/snapshot.rs`
- `wrapper/src/state/*`
- connector mapping/tests if the route family is exposed immediately

The model should stay lightweight enough that it links deployments and sessions
through broker-visible metadata rather than attempting a hidden distributed
runtime. It should not widen the supported adapter boundary from shell-first
host examination into an implied artifact-browser contract.

## Proof Expectations

The first implementation should prove all of:

- a session can be bound to a project explicitly
- dependency edges can be created and listed explicitly
- dependency edges replay through the event stream
- handoff creation can reference dependency ids cleanly
- the model does not require same-host deployment discovery
- the model does not require artifact routes
- the model does not silently expand the supported experimental adapter into
  artifact index/detail/content routes

## Relationship To The Handoff Lane

The recommended split is:

- project/dependency routes explain structural work relationships
- handoff routes explain concrete collaboration requests

That avoids turning every handoff into a hidden project/dependency registry and
keeps the route families understandable.

## Relationship To The Sibling Workspace

The sibling `~/work/agent` workspace should treat this as the next design lane
after the already-supported single-deployment session/event/shell/service
surface and alongside the explicit handoff lane.

The current expectation is:

- do not assume deployment switching alone captures project/dependency context
- do not scrape transcript prose to reconstruct dependency graphs
- do not assume the participating deployments share a host or workspace

## Completion Bar

The first implementation track is complete when:

1. sessions can declare project identity explicitly
2. dependency edges can be declared and inspected explicitly
3. broker-facing clients can replay project/dependency state
4. handoff records can reference dependency ids instead of inventing their own
   hidden graph model
5. docs and status claims say exactly what is implemented, what remains
   deferred, and that the supported experimental adapter still stops at the
   shell-first host-examination surface
