# codexw Cross-Project Dependency Contract Sketch

## Purpose

This document sketches the first broker-visible contract for project-aware
collaboration across multiple `codexw` deployments.

It sits below:

- [codexw-cross-project-dependency-collaboration.md](codexw-cross-project-dependency-collaboration.md)
- [codexw-cross-deployment-handoff-contract-sketch.md](codexw-cross-deployment-handoff-contract-sketch.md)

This is still a design sketch, not an implemented claim.

## Goal

Deliver the smallest contract that lets broker-facing clients answer these
questions explicitly:

- which project is a session currently working on?
- which projects depend on which other projects?
- which dependency edge is blocking work right now?
- which handoff or review request corresponds to that dependency edge?

Because deployments may run on different hosts, this contract must stay
broker-mediated rather than relying on same-host discovery or shared
filesystem state.

## Non-goals

The first project/dependency contract should **not** try to solve:

- global repo discovery across all hosts
- automatic dependency graph inference from source trees
- artifact replication between deployments
- transparent cross-host filesystem browsing
- direct deployment-to-deployment transport outside the broker
- a universal scheduler for all dependent work

## Core Objects

### Project Record

Recommended first fields:

- `project_id`
- `project_label`
- `repo_hint`
- `workspace_hint`
- `default_branch_hint`

The first contract can keep project records lightweight. It only needs enough
identity to let clients and handoff records talk about the same project
consistently.

### Session Project Assignment

Recommended first fields:

- `assignment_id`
- `deployment_id`
- `session_id`
- `thread_id`
- `project_id`
- `project_role`
- `updated_at`

Suggested first `project_role` values:

- `owner`
- `dependency_provider`
- `dependency_consumer`

### Dependency Edge

Recommended first fields:

- `dependency_id`
- `upstream_project_id`
- `downstream_project_id`
- `kind`
- `state`
- `summary`
- `blocking`
- `updated_at`

Suggested first `state` values:

- `declared`
- `in_progress`
- `blocked`
- `satisfied`

Suggested first `kind` values:

- `api_contract`
- `generated_artifact`
- `verification`
- `review`
- `release_input`

## First Route Family

The first route family should stay narrow and explicit.

Suggested local-API shape:

- `POST /api/v1/session/{session_id}/project`
- `GET /api/v1/session/{session_id}/project`
- `POST /api/v1/projects/{project_id}/dependencies`
- `GET /api/v1/projects/{project_id}/dependencies`
- `GET /api/v1/dependencies/{dependency_id}`

The first contract does not need a full global project catalog before it can
support real cross-project collaboration. Session assignment and dependency-edge
visibility are the higher-leverage first slice.

## Request Sketches

### Bind a session to a project

`POST /api/v1/session/{session_id}/project`

Request body sketch:

```json
{
  "projectId": "compiler-mini",
  "projectLabel": "compiler-mini",
  "repoHint": "github.com/example/compiler-mini",
  "workspaceHint": "/work/compiler-mini",
  "defaultBranchHint": "main",
  "projectRole": "owner"
}
```

### Declare a dependency edge

`POST /api/v1/projects/{project_id}/dependencies`

Request body sketch:

```json
{
  "upstreamProjectId": "compiler-mini",
  "downstreamProjectId": "compiler-mini-stage2-verification",
  "kind": "verification",
  "state": "blocked",
  "blocking": true,
  "summary": "stage-2 verification waits on compiler-mini root-cause isolation"
}
```

### Inspect one dependency edge

`GET /api/v1/dependencies/{dependency_id}`

The response should include enough identity that a broker/WebUI client can link
it back to project assignments and any related handoff record.

## Event Sketch

The event stream should carry replayable project/dependency transitions.

Suggested first event names:

- `session_project_bound`
- `project_dependency_declared`
- `project_dependency_updated`

Each event should carry:

- relevant deployment/session identity
- project identity
- dependency identity when applicable

## Relationship To Handoffs

Project/dependency metadata and handoffs should stay related but distinct.

That means:

- project assignment says what a session is currently working on
- dependency edges explain why one project blocks on another
- handoff records represent a concrete collaboration request between
  deployments/sessions

The first handoff contract should therefore cite `dependency_id` values instead
of embedding an entirely separate dependency model inside each handoff payload.

## Identity Rules

The first contract should preserve existing identity boundaries:

- `session_id` remains deployment-local
- `project_id` is collaboration-local metadata, not a replacement for
  `session_id`
- `dependency_id` identifies a relationship, not a session or deployment

## Error Model

The first project/dependency contract should make these failures explicit:

- session not found
- invalid project assignment mutation
- dependency edge not found
- invalid dependency state transition
- project/dependency references that conflict with the current handoff payload

## Relationship To Artifact Track

Project dependencies may eventually refer to explicit artifacts, but the first
contract should not require that.

That means:

- dependency edges can exist before any artifact list/detail/content API exists
- artifact ids may become one optional supporting reference later
- the dependency model should still work with shell/service/transcript/event
  provenance only
- those missing artifact routes remain outside the current supported experimental adapter
  until the separate artifact track is implemented and proven

## Acceptance Bar

The first project/dependency contract is well-defined when:

1. session-to-project assignment is explicit
2. dependency edges are explicit
3. broker-facing clients can replay the relevant assignment/dependency events
4. the design does not imply same-host deployment coexistence or artifact sync
   it does not actually implement
