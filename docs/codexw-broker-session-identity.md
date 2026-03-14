# codexw Broker Session Identity Note

This document resolves the placeholder “session identity” question from
`docs/codexw-broker-connectivity.md` into the concrete adapter model now used
throughout the repo.

The goal is to avoid collapsing distinct concepts into one id too early.

It also needs to support the current broker-facing architecture direction:

- broker-backed app/WebUI clients should act through stable session-scoped
  handles
- host shell/service control should stay correlated to that same session handle
- any future artifact index/detail/content surface should derive from that
  session-scoped runtime truth rather than inventing an unrelated identity layer

## Identity Layers

There are four different identity concepts in play:

1. local thread id
2. wrapper session id
3. remote client attachment id
4. resume command / attach target

They should not be treated as interchangeable.

## 1. Local Thread Id

Definition:

- the app-server-backed Codex thread identifier already used by `codexw`

Properties:

- long-lived
- backend-originated
- the canonical identity for resume/fork/rename/review/compact flows

Role in broker design:

- remains the canonical identity for the underlying Codex conversation
- should be exposed in remote APIs
- should not be replaced by a synthetic wrapper-only id

## 2. Wrapper Session Id

Definition:

- a `codexw`-owned remote-control session handle

Properties:

- created by the current local API
- identifies one active remote control context
- may or may not already be attached to a concrete local thread

Role in broker design:

- should be the primary remote API handle
- lets remote clients create a control context before choosing a thread
- isolates remote clients from needing to know local thread ids immediately

Recommended current rule:

- one wrapper session may be unattached or attached to exactly one local thread at a time
- host shell/service/event/result activity should stay attributable to that same
  wrapper session even before a richer artifact catalog exists

## 3. Remote Client Attachment Id

Definition:

- an optional client-specific identity for one browser tab, mobile device, terminal, or automation client

Properties:

- ephemeral
- useful for auditing and concurrency rules
- not a substitute for wrapper session id

Role in broker design:

- used to attribute actions such as:
  - attach
  - steer
  - interrupt
  - shell mutation
- useful if multi-client attachment is supported later

Recommended current rule:

- clients may send a `client_id` or `attachment_id`, but the server-side control plane is still keyed by wrapper session id

## 4. Resume Command / Attach Target

Definition:

- the operator-facing or API-facing thing used to reattach to an existing conversation

Examples:

- local CLI resume command with a `thread_id`
- remote API `session/attach` call
- current broker connector attach flow

Role in broker design:

- this is an operation, not a durable identity type
- it should resolve to:
  - a wrapper session attachment
  - and possibly a local thread attachment

## Recommended Mapping

### Current Session Split

- wrapper API creates `session_id`
- `session_id` may begin unattached
- attach operation binds `session_id -> thread_id`
- remote clients act through `session_id`
- responses and events include both:
  - `session_id`
  - `thread_id` when attached

This gives the API a stable wrapper-scoped handle without hiding the underlying Codex thread identity.
That is especially important for broker-backed app/WebUI clients, which need a
stable session-scoped control and inspection handle even when the underlying
thread identity is not yet known or is only one part of the visible state.

### Deferred Multi-Client Extension

- broker/connector introduces client identity and deployment routing
- one `session_id` may be observed by more than one client attachment
- audit logs should record:
  - `session_id`
  - `thread_id`
  - `client_id`
  - `deployment_id` when brokered

## Why This Split Matters

If `session_id == thread_id` too early:

- unattached remote control contexts become awkward
- local resume semantics leak into every remote client
- future multi-client and broker routing become harder

If `session_id` hides `thread_id` completely:

- existing `codexw` resume, review, fork, and status semantics become harder to preserve
- users lose a stable way to reason about the actual underlying Codex conversation

The right balance is:

- wrapper session id for remote control
- local thread id for underlying Codex conversation identity

## First-Pass API Implications

### `POST /api/v1/session/new`

Returns:

- `session_id`
- `thread_id: null` initially unless immediately attached

### `POST /api/v1/session/attach`

Request:

- `session_id`
- `thread_id`

Effect:

- binds wrapper session to the existing local thread

### Event Streams

All current session-scoped events should include:

- `session_id`
- `thread_id` when attached

That preserves both the remote-control identity and the underlying conversation identity.
It also keeps future shell/service/result references and any later artifact entries
attributable to the same session-scoped control context.

## Current Status

This identity model is no longer only a design proposal. The current local API
already implements the session split described above:

- `POST /api/v1/session/new`
- `POST /api/v1/session/attach`
- `GET /api/v1/session/{session_id}`
- `POST /api/v1/session/{session_id}/attachment/renew`
- `POST /api/v1/session/{session_id}/attachment/release`

The broker-style connector and fixture coverage also already consume that
process-scoped `session_id` plus `thread_id` contract. The remaining open
questions are therefore about future multi-client or multi-daemon policy, not
whether the wrapper/local-thread identity split exists at all.
Likewise, the remaining artifact gap is not an identity-model gap; it is a
separate route and provenance-surface gap tracked in the artifact-contract docs.

## Open Questions

- Should one wrapper session ever be allowed to switch across multiple threads during its lifetime, or should that require a new wrapper session?
- Should multi-client attachment require a lock/lease model for steer/interrupt operations?
- Should broker routing keys be based on wrapper session id, thread id, or both?
