# codexw Cross-Project Dependency Collaboration

## Purpose

This document narrows the broader cross-deployment collaboration requirement
into the specific multi-project case:

- multiple `codexw` deployments are active at the same time
- each deployment is primarily working on one project
- those projects may depend on one another
- collaboration cannot assume the deployments share one host

That last point matters. The collaboration model therefore has to be
broker-mediated rather than based on direct local IPC, shared filesystem
assumptions, or same-host process discovery.

This is a design note, not an implementation claim.

It also stays inside the current supported experimental adapter boundary. The
already-supported broker-facing surface is the shell-first host-examination
lane; cross-project dependency metadata can build on that surface without
implying that broker-visible artifact index/detail/content routes already
exist.

It sits between:

- [codexw-cross-deployment-collaboration.md](codexw-cross-deployment-collaboration.md)
- [codexw-cross-deployment-handoff-contract-sketch.md](codexw-cross-deployment-handoff-contract-sketch.md)
- [codexw-cross-deployment-handoff-implementation-plan.md](codexw-cross-deployment-handoff-implementation-plan.md)

For the first broker-visible project/dependency contract sketch and delivery
order, see
[codexw-cross-project-dependency-contract-sketch.md](codexw-cross-project-dependency-contract-sketch.md)
and
[codexw-cross-project-dependency-implementation-plan.md](codexw-cross-project-dependency-implementation-plan.md).

For the source docs that define the current shell-first remote
host-examination surface that this project/dependency collaboration track
builds on, see:

- [codexw-workspace-tool-policy.md](codexw-workspace-tool-policy.md)
- [codexw-local-api-sketch.md](codexw-local-api-sketch.md)
- [codexw-local-api-implementation-plan.md](codexw-local-api-implementation-plan.md)
- [codexw-local-api-event-sourcing.md](codexw-local-api-event-sourcing.md)
- [codexw-local-api-route-matrix.md](codexw-local-api-route-matrix.md)

## Why This Requirement Exists

The existing broker/client direction already implies:

- app/WebUI clients can drive more than one deployment through the broker
- sessions remain deployment-local rather than global
- shell/service/transcript/event truth remains deployment-local

Once that is true, a common workflow appears:

- deployment A is working on project `frontend`
- deployment B is working on project `api`
- deployment C is working on project `compiler-mini`
- `frontend` depends on `api`
- `api` depends on generated artifacts or behavior from `compiler-mini`

Those dependencies are real work relationships, not just documentation notes.
If one project blocks on another, the collaboration surface has to express:

- which project currently owns the blocking work
- which project is downstream
- which deployment/session is working each side
- what exact dependency edge motivated the handoff or request

## Design Stance

Cross-project collaboration should be:

- broker-mediated rather than direct deployment-to-deployment transport
- project-aware rather than deployment-aware only
- dependency-explicit rather than hidden in free-form transcript prose
- session-scoped at execution time rather than modeled as one global worker pool
- provenance-preserving rather than summary-only

The broker is the only plane that can safely connect deployments that may not
coexist on the same host.

## Core Model

The first model needs four layers.

### 1. Deployment

Deployment identity stays explicit:

- `deployment_id`
- deployment-local sessions
- deployment-local thread, shell, service, and event truth

### 2. Project

A project is the collaboration unit that the deployment is currently advancing.

Recommended first fields:

- `project_id`
- `project_label`
- `workspace_hint`
- `repo_hint`
- `default_branch_hint`

The first model does not need a globally authoritative repo registry. It does
need a stable enough project identity that broker-visible clients can tell
which work item belongs to which project.

### 3. Project Assignment

Each active collaborating session should make its project context explicit.

Recommended assignment fields:

- `deployment_id`
- `session_id`
- `project_id`
- `project_role`

The `project_role` can stay narrow at first:

- `owner`
- `dependency_provider`
- `dependency_consumer`

### 4. Dependency Edge

The collaboration model should be able to express that one project depends on
another without pretending the broker is a full build graph engine.

Recommended fields:

- `dependency_id`
- `upstream_project_id`
- `downstream_project_id`
- `kind`
- `summary`
- `blocking`

Suggested `kind` examples:

- `api_contract`
- `generated_artifact`
- `verification`
- `review`
- `release_input`

## Relationship To Handoffs

Not every dependency edge is a handoff, but handoffs should be able to cite the
dependency edges that motivated them.

That means a handoff record should be able to include:

- `source.project_id`
- `target.project_id`
- `dependency_refs`

Example:

- project `frontend` blocks on project `api`
- the `frontend` deployment opens a handoff or dependency request toward the
  `api` deployment
- the handoff carries the dependency edge plus transcript/shell/event
  provenance from the blocked session

## Broker Responsibilities

Because deployments may live on separate hosts, the broker-mediated layer is
responsible for:

- project/deployment/session addressing
- storing project-aware handoff records
- replaying dependency-aware handoff events
- letting clients inspect collaboration state across deployments

The broker should not become:

- a filesystem sync layer
- a distributed execution fabric
- a shared shell multiplexer
- a global source-of-truth for deployment-local transcript contents

## Non-goals

The first design should not claim:

- cross-host shared workspaces
- automatic source tree synchronization
- transparent artifact replication across deployments
- broker-visible artifact index/detail/content routes as part of the current
  supported adapter
- globally optimal scheduling of dependent projects
- direct `codexw` to `codexw` transport that bypasses the broker

## First Practical Workflow

The smallest real workflow should look like this:

1. deployment A is bound to project A
2. deployment B is bound to project B
3. a broker-visible dependency edge says project B depends on project A
4. session B proposes a handoff or dependency request toward deployment A
5. the proposal includes:
   - project identity on both sides
   - the dependency edge reference
   - transcript/event/shell/service provenance
   - the requested outcome
6. deployment A accepts, declines, or completes the request through the broker
7. clients can replay that state later without scraping transcript prose

## First Implementation Boundary

The first cross-project dependency collaboration slice is well-defined when:

1. broker-visible handoffs can carry source and target project identity
2. handoffs can reference explicit dependency edges
3. the collaboration model does not assume same-host deployment coexistence
4. deployment-local runtime truth remains local while the broker carries the
   linking metadata
5. docs and client handoff notes say explicitly that broker mediation is
   required for cross-host collaboration, while the supported experimental
   adapter still stops at the shell-first host-examination surface

## Relationship To Artifact Track

Project dependencies may eventually reference explicit artifacts, but this
design should not wait for that.

The first viable collaboration model can work with:

- dependency metadata
- the already-supported shell/service/transcript/event host-examination surface
  instead of a not-yet-implemented artifact API
- summary text
- transcript refs
- event refs
- shell refs
- service refs

Artifact ids can be added later when a real artifact route family exists.
