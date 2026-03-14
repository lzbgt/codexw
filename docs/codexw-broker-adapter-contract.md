# codexw Broker Adapter Contract

This document freezes the current broker-facing adapter contract for `codexw`.

It is narrower than the full broker/connectivity design and narrower than a
future production support policy. Its purpose is to state, in one place, what a
remote broker-style client may rely on today if it builds against the
`codexw` local API plus connector.

This document is the contract-oriented companion to:

- [codexw-broker-connectivity.md](codexw-broker-connectivity.md)
- [codexw-broker-client-policy.md](codexw-broker-client-policy.md)
- [codexw-broker-host-examination-matrix.md](codexw-broker-host-examination-matrix.md)
- [codexw-broker-artifact-contract-sketch.md](codexw-broker-artifact-contract-sketch.md)
- [codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md)
- [codexw-broker-adapter-promotion.md](codexw-broker-adapter-promotion.md)

## Contract Scope

This contract covers:

- the authority boundary between local API and connector
- the role and lease model for broker-style clients
- the mutation-versus-read rules that clients may rely on
- the connector projection behavior for `client_id` and `lease_seconds`
- the connector and local-API error expectations relevant to client policy
- the event/replay guarantees the adapter currently exposes
- the current remote host-examination foundation built from session, event,
  shell, and service surfaces

This contract does not claim:

- full `agentd` compatibility
- multi-daemon lease coordination
- production deployment/auth semantics
- compatibility with every future broker route family
- a dedicated artifact catalog or artifact download API

The future artifact-specific track is now sketched separately in
[codexw-broker-artifact-contract-sketch.md](codexw-broker-artifact-contract-sketch.md)
rather than being left as an unnamed caveat.

Those boundaries remain explicit in
[codexw-broker-out-of-scope.md](codexw-broker-out-of-scope.md).

## Authority Model

The current adapter stack has one canonical authority:

- the local API is the canonical runtime contract

The connector is a thin adapter. It may:

- expose broker-style alias routes
- inject `client_id` and `lease_seconds` from headers into supported JSON
  bodies when those fields are absent
- preserve and forward structured local-API responses and errors
- forward SSE with broker metadata wrapping

The connector must not:

- invent independent session state
- invent independent lease state
- reinterpret successful versus conflicting mutation semantics
- expose non-allowlisted route families by passthrough accident

This authority model is part of the adapter contract, not only an architectural
preference.

## Client Roles

Clients interacting through the adapter are interpreted in one of three roles.

### Owner

The owner is the client currently holding the active attachment lease.

Contractually, the owner may:

- renew and release the current lease
- perform lease-owned turn, shell, service, and `client_event` mutations
- continue performing those mutations until the lease is released or expires

### Observer

An observer is a client reading the remote session without owning the lease.
An observer may be named or anonymous.

Contractually, an observer may:

- fetch session snapshots
- fetch transcript state
- fetch orchestration views
- fetch shell list/detail
- fetch service list/detail
- fetch capability list/detail
- consume SSE events and replay with `Last-Event-ID`

An observer may not successfully perform lease-owned mutations while another
client holds the active lease.

### Rival

A rival is a non-owner attempting a lease-owned mutation while another client
holds the active lease. A rival may be named or anonymous.

Contractually, a rival mutation attempt:

- must not implicitly steal the lease
- must fail with structured `attachment_conflict`
- must preserve enough detail for the caller to identify the current holder and
  lease state

`rival` is a runtime policy state, not a separate identity namespace.

## Lease Contract

The adapter contract for lease state is:

- a session attachment may expose:
  - `client_id`
  - `lease_seconds`
  - `lease_expires_at_ms`
  - `lease_active`
- only the active owner may renew the current lease
- only the active owner may explicitly release the current lease
- once `lease_expires_at_ms` is in the past, the lease is no longer active
- after expiry or release, another client may acquire ownership through normal
  create/attach/renew flows supported by the current runtime

This contract is process-scoped. It does not claim distributed lock-perfect
semantics across multiple runtimes or daemons.

## Operation Classes

### Lease-Owned Mutations

The following operation classes are lease-owned under the current adapter
contract:

- session mutation:
  - session creation
  - session attach
  - attachment renew
  - attachment release
- turn mutation:
  - turn start
  - turn interrupt
- shell mutation:
  - shell start
  - shell poll
  - shell send
  - shell terminate
- service mutation:
  - provide
  - depend
  - contract
  - relabel
  - attach
  - wait
  - run
- semantic publication:
  - `client_event`

### Observer-Readable Operations

The following operation classes are observer-readable without lease ownership:

- session inspection
- transcript inspection
- orchestration inspection
- shell list/detail
- service list/detail
- capability list/detail
- SSE subscription and replay with `Last-Event-ID`

The adapter contract relies on this distinction. Remote clients should not
infer broader observer mutation privileges than the ones stated here.

## Connector Projection Contract

For supported mutating JSON routes, the connector may project:

- `X-Codexw-Client-Id`
- `X-Codexw-Lease-Seconds`

into the outgoing local-API JSON body.

The projection contract is:

- projection happens only when the outgoing body does not already provide
  `client_id` or `lease_seconds`
- malformed projected values fail with structured validation errors
- connector-side validation occurs before any upstream connection attempt
- explicit request-body fields always take precedence over connector header
  projection

This behavior is part of the adapter contract because remote clients may rely
on header-based participation in lease ownership without hand-constructing
every local-API JSON shape.

## Error Contract

The current adapter contract requires a structured error envelope with:

- `status`
- `code`
- `message`
- `retryable`
- `details`

For client-policy-sensitive operations:

- lease conflicts must return `attachment_conflict`
- malformed client/lease projection must return structured validation failure
- preserved local field validation errors must not be collapsed into generic
  transport errors by the connector

For conflicts specifically, callers may rely on `details` carrying:

- `requested_client_id`
- current attachment holder details
- lease timing information such as `lease_seconds`, `lease_expires_at_ms`, and
  `lease_active`

## Event Contract

The adapter contract for event delivery is:

- the event stream is semantic rather than terminal-render-derived
- event delivery is available through SSE
- replay/resume with `Last-Event-ID` is supported on the documented session
  event stream
- the connector preserves resume behavior while adding broker metadata wrapping

The contract does not currently guarantee an exhaustive formal event taxonomy
beyond the documented session event stream families, but it does guarantee that
the existing resume/replay model is part of the adapter surface.

## Unsupported Boundary Contract

The adapter contract explicitly excludes unsupported broker/client surfaces.
Remote clients may rely on the following:

- named out-of-scope broker-style routes fail explicitly
- unknown broker-style aliases fail explicitly
- raw `/proxy/...` and `/proxy_sse/...` paths outside the allowlist fail
  explicitly

This negative behavior is part of the contract because it prevents accidental
scope expansion by passthrough.

See [codexw-broker-out-of-scope.md](codexw-broker-out-of-scope.md) for the
named unsupported categories.

## What This Contract Does And Does Not Settle

This contract settles:

- who owns lease-sensitive mutations
- what observers may still read
- how rivals fail
- which layer is authoritative
- how connector header projection behaves
- which unsupported surfaces must fail explicitly

This contract does not settle:

- production service-level guarantees
- compatibility with all future broker/client protocols
- distributed or replicated coordination semantics
- promotion timing by itself

Promotion still depends on the proof and decision criteria in:

- [codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md)
- [codexw-broker-adapter-promotion.md](codexw-broker-adapter-promotion.md)
