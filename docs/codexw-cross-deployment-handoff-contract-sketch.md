# codexw Cross-Deployment Handoff Contract Sketch

## Purpose

This document sketches the first broker-visible contract for cross-deployment
`codexw` collaboration.

It sits below
[codexw-cross-deployment-collaboration.md](codexw-cross-deployment-collaboration.md)
and turns that architecture direction into an API-facing handoff shape.
For the adjacent project/dependency metadata contract that this handoff lane
should reference, see
[codexw-cross-project-dependency-contract-sketch.md](codexw-cross-project-dependency-contract-sketch.md).

This is still a design sketch, not an implemented claim.

For the source docs that define the current shell-first remote
host-examination surface that this handoff contract would extend, see:

- [codexw-workspace-tool-policy.md](codexw-workspace-tool-policy.md)
- [codexw-local-api-sketch.md](codexw-local-api-sketch.md)
- [codexw-local-api-implementation-plan.md](codexw-local-api-implementation-plan.md)
- [codexw-local-api-event-sourcing.md](codexw-local-api-event-sourcing.md)
- [codexw-local-api-route-matrix.md](codexw-local-api-route-matrix.md)

## Goal

Deliver the smallest collaboration contract that lets one `codexw` deployment
propose work to another deployment through the broker surface while preserving:

- source/target deployment identity
- source/target session identity
- source/target project identity
- dependency context between those projects
- replayable state transitions
- provenance back to source runtime truth

## Non-goals

The first handoff contract should **not** try to solve:

- global work scheduling across every deployment
- generic artifact replication or download
- multi-deployment lease consensus
- filesystem mirroring
- direct deployment-to-deployment transport that bypasses broker mediation
- same-host assumptions about where the participating deployments run

## Contract Object: Handoff Record

The core object should be a handoff record.

Recommended fields:

- `handoff_id`
- `status`
- `created_at`
- `updated_at`
- `source.deployment_id`
- `source.session_id`
- `source.thread_id`
- `source.project_id`
- `target.deployment_id`
- `target.session_id`
- `target.project_id`
- `summary`
- `requested_outcome`
- `next_action`
- `dependency_refs`
- `provenance_refs`

## Status Values

The first contract should support at least:

- `proposed`
- `accepted`
- `declined`
- `in_progress`
- `completed`
- `cancelled`

Those values are enough to model a real proposal/acceptance flow without
claiming a richer scheduler than the system actually has.

## Provenance References

The handoff record should allow structured provenance references back to the
source deployment.

The first reference classes should be:

- transcript item ids
- event ids
- shell ids
- service ids

The handoff should also allow explicit dependency references between projects.
The first dependency reference shape can stay narrow:

- `dependency_id`
- `upstream_project_id`
- `downstream_project_id`
- `kind`

If the future artifact track becomes implemented, artifact ids can become one
additional provenance class. They should not be required for the first
cross-deployment handoff contract.

## First Route Family

The route family should stay session-scoped and explicit.

Suggested local-API shape:

- `POST /api/v1/session/{session_id}/handoffs`
- `GET /api/v1/session/{session_id}/handoffs`
- `GET /api/v1/handoffs/{handoff_id}`
- `POST /api/v1/handoffs/{handoff_id}/accept`
- `POST /api/v1/handoffs/{handoff_id}/decline`
- `POST /api/v1/handoffs/{handoff_id}/complete`

The first contract should prefer a narrow set of state transitions over a wide
command surface.

## Request Sketches

### Create handoff

`POST /api/v1/session/{session_id}/handoffs`

Request body sketch:

```json
{
  "target": {
    "deployment_id": "build-farm-a"
  },
  "sourceProjectId": "compiler-mini",
  "targetProjectId": "compiler-mini-stage2-verification",
  "summary": "Take over stage-2 native verification after local repro failed",
  "requestedOutcome": "Reproduce, diagnose, and report the root cause",
  "nextAction": "Run the stage-2 repro and compare shell failure output",
  "dependencyRefs": [
    {
      "dependencyId": "dep-stage2-native",
      "upstreamProjectId": "compiler-mini",
      "downstreamProjectId": "compiler-mini-stage2-verification",
      "kind": "verification"
    }
  ],
  "provenanceRefs": [
    {"kind": "shell", "id": "bg-14"},
    {"kind": "event", "id": "evt_102"},
    {"kind": "transcript", "id": "msg_57"}
  ]
}
```

Response should include the created handoff record.

### Accept handoff

`POST /api/v1/handoffs/{handoff_id}/accept`

Request body sketch:

```json
{
  "targetSessionId": "sess_target_9"
}
```

If the target session does not exist yet, the first implementation may allow
the broker-facing client to create the target session first and then call
accept. The handoff contract should not hide that session lifecycle.

### Decline handoff

`POST /api/v1/handoffs/{handoff_id}/decline`

Optional body sketch:

```json
{
  "reason": "deployment unavailable"
}
```

### Complete handoff

`POST /api/v1/handoffs/{handoff_id}/complete`

Optional body sketch:

```json
{
  "summary": "Stage-2 repro completed and root cause isolated",
  "provenanceRefs": [
    {"kind": "shell", "id": "bg-3"},
    {"kind": "event", "id": "evt_44"}
  ]
}
```

## Event Sketch

The event stream should carry replayable handoff transitions.

Suggested first event names:

- `session_handoff_proposed`
- `session_handoff_accepted`
- `session_handoff_declined`
- `session_handoff_completed`

Each event should carry:

- `handoff_id`
- `status`
- source deployment/session identity
- source project identity
- target deployment/session identity when known
- target project identity when known
- dependency references when present

## Identity Rules

The handoff contract should preserve the current session identity model:

- do not collapse source and target work into one global session id
- do not replace `session_id` with a synthetic handoff-only execution id
- do use the handoff record as the explicit linking object between deployments
- do keep project identity separate from deployment identity because one broker
  may coordinate deployments working on related but distinct projects

## Error Model

The first handoff contract should make the following failures explicit:

- unknown target deployment
- handoff not found
- invalid handoff status transition
- missing or invalid target session when accepting
- forbidden state mutation because the caller does not hold the required lease

## Relationship To The Artifact Track

This contract should explicitly say that provenance refs are not equivalent to
a full artifact API.

That means:

- project-aware handoff metadata and dependency refs are first-class even
  before artifact routes exist
- provenance refs may include shell/service/transcript/event references
- provenance refs do not imply artifact list/detail/content routes exist
- those missing artifact routes remain outside the current supported experimental adapter
  until the separate artifact track is implemented and proven
- future artifact ids are additive, not required for the first handoff surface

## Acceptance Bar

The first handoff contract is well-defined when:

1. its route family is explicit
2. its status model is explicit
3. its provenance model is explicit
4. its identity rules do not conflict with the existing session model
5. it does not imply artifact sync or global orchestration semantics that are
   not yet implemented
