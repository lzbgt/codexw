# codexw Broker Client Policy

This document resolves the client-policy and attachment semantics questions that
were previously called out before the broker adapter contract was frozen.

The core question is not whether `codexw` can carry `client_id` and
`lease_seconds` today. It can. The real question is what remote clients should
be allowed to do concurrently, and which operations must remain lease-owned.

Those remote clients now explicitly include broker-backed app/WebUI consumers
that inspect sessions, orchestration state, host shell/service state, and the
resulting references without direct terminal access. This policy therefore needs
to stay aligned with the shell-first host-examination posture rather than
assuming a terminal-only consumer.

For the source docs that define that current shell-first remote
host-examination surface, see
[codexw-local-api-sketch.md](codexw-local-api-sketch.md),
[codexw-local-api-implementation-plan.md](codexw-local-api-implementation-plan.md),
[codexw-local-api-event-sourcing.md](codexw-local-api-event-sourcing.md),
[codexw-local-api-route-matrix.md](codexw-local-api-route-matrix.md), and
[codexw-native-support-boundaries.md](codexw-native-support-boundaries.md).

## Purpose

This document defines the current explicit policy contract for:

- attachment ownership
- observer behavior
- mutation eligibility
- lease renewal and release
- connector responsibilities when multiple clients are active

It is intentionally scoped to the current single-process runtime model.

## Current Runtime Reality

The current implementation already has these concrete facts:

- one `codexw` process exposes one process-scoped remote-control session model
- a session may carry an attachment object with:
  - `client_id`
  - `lease_seconds`
  - `lease_expires_at_ms`
  - `lease_active`
- mutating local-API routes enforce active lease ownership
- read routes are generally available without lease ownership
- the connector can project:
  - `X-Codexw-Client-Id`
  - `X-Codexw-Lease-Seconds`
  into supported local-API request bodies

This document does not replace those facts. It names their intended policy.

## Client Roles

Current remote clients should be understood as one of three roles.

### 1. Owner

Definition:

- the client that currently holds the active attachment lease

Capabilities:

- may perform lease-owned mutations
- may renew or release the lease
- may start and interrupt turns
- may mutate shell/service state
- may drive broker-visible host shell/service examination workflows that require
  mutation rights
- may publish `client_event`

### 2. Observer

Definition:

- a client that reads session state without owning the lease
- an observer may be named or anonymous

Capabilities:

- may read session snapshots
- may read transcript
- may read orchestration views
- may read shell/service/capability detail
- may inspect host-result references already visible through transcript, shell,
  service, or event surfaces
- may consume SSE event streams

Restrictions:

- may not perform lease-owned mutations while another client holds an active
  lease

### 3. Rival

Definition:

- a non-owner client attempting a lease-owned mutation while another client has
  the active lease
- a rival may be a named client or an anonymous caller with no `client_id`

Expected outcome:

- receives structured `attachment_conflict`
- does not implicitly steal the lease

This is not a separate API identity class. It is a runtime policy state.

## Lease-Owned Operations

The following operations are current lease-owned operations and should
continue to require the active owner:

- session mutation:
  - `session/new`
  - `session/attach`
  - `attachment/renew`
  - `attachment/release`
- turn mutation:
  - `turn/start`
  - `turn/interrupt`
- shell mutation:
  - `shells/start`
  - `shells/{job_ref}/poll`
  - `shells/{job_ref}/send`
  - `shells/{job_ref}/terminate`
- service mutation:
  - `provide`
  - `depend`
  - `contract`
  - `relabel`
  - `attach`
  - `wait`
  - `run`
- semantic client publication:
  - `client_event`

## Observer-Allowed Operations

The following operations should remain readable without lease ownership:

- `GET /api/v1/session`
- `GET /api/v1/session/{session_id}`
- transcript fetch
- orchestration status/workers/dependencies
- shell list/detail
- service list/detail
- capability list/detail
- SSE event consumption

If a dedicated artifact index/detail/content layer is later implemented, it
should be added to this section deliberately rather than assumed implicitly from
today's transcript or shell-read semantics.

## Renewal and Expiry Rules

Current rules:

1. only the current owner may renew the active lease
2. only the current owner may explicitly release the active lease
3. once `lease_expires_at_ms` is in the past, the lease is considered inactive
4. after expiry, a new client may acquire ownership through normal lease-bearing
   create/attach behavior

Important limitation:

- the current model is process-scoped, not distributed
- there is no claim of clock-perfect distributed lock semantics

## Conflict Contract

When a rival attempts a lease-owned mutation during an active lease, the
response should continue to be:

- HTTP `409`
- error `code = "attachment_conflict"`
- structured details including:
  - `requested_client_id`
  - current attachment holder
  - lease timing metadata

Notes:

- for anonymous rival calls, `requested_client_id` is expected to be `null`
- anonymous reads remain observer-allowed, but anonymous lease-owned mutation is
  treated as rival behavior while another client owns the lease

## Connector Responsibilities

The connector should continue to do exactly these policy-sensitive things:

- preserve `client_id` and `lease_seconds`
- inject them from headers only when the outgoing JSON body does not already
  provide them
- preserve structured conflict/error payloads
- not invent a second lease model of its own
- not imply that a separate artifact surface exists before the local API and
  support docs actually define one
- not imply that a separate project-assignment or dependency-edge collaboration surface exists before the local API and support docs actually define one

## Explicit Non-Goals

The following are intentionally not guaranteed in this first policy contract:

- multi-owner cooperative steering
- distributed broker-wide lock coordination
- server-mediated queueing of rival mutations
- priority/preemption semantics
- forced takeover semantics
- durable audit log semantics beyond current event and response payloads

## Practical Summary

The current contract is:

- one owner at a time
- many observers allowed
- rival mutation is rejected with structured conflict details
- connector preserves policy inputs and outputs
- read paths remain broadly observable
- broker-backed app/WebUI clients can participate in the same owner/observer/
  rival model for shell-first host examination

For support-level language, that means the current supported experimental adapter
covers shell-first host examination through the current
session/transcript/orchestration/shell/service surfaces. It does not imply that
a separate artifact index/detail/content surface exists before the local API,
connector mapping, proof, and support docs explicitly add one.
