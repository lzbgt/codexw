# codexw Broker Proof Matrix

This document maps the broker/local-API adapter claims to concrete repo
evidence.

Use it when deciding whether the current stack is only a strong prototype or is
close to promotion under
[codexw-broker-adapter-promotion.md](codexw-broker-adapter-promotion.md).

It does not redefine the broker design. It answers a narrower question:

- which claims are process-level proven
- which claims are only policy or design intent
- which areas still need stronger evidence before promotion

## Companion Docs

- [codexw-broker-prototype-status.md](codexw-broker-prototype-status.md)
- [codexw-broker-adapter-promotion.md](codexw-broker-adapter-promotion.md)
- [codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md)
- [codexw-broker-support-policy.md](codexw-broker-support-policy.md)
- [codexw-broker-client-policy.md](codexw-broker-client-policy.md)
- [codexw-broker-out-of-scope.md](codexw-broker-out-of-scope.md)
- [codexw-broker-client-fixture.md](codexw-broker-client-fixture.md)
- [codexw-broker-connector-prototype-plan.md](codexw-broker-connector-prototype-plan.md)

## Reading This Matrix

Status labels:

- `strong proof`
  - backed by process-level connector smoke and/or the real Python broker
    client fixture
- `partial proof`
  - backed by implementation plus some smoke proof, but not yet at the level
    desirable for promotion
- `doc/policy only`
  - specified clearly, but still primarily a contract statement rather than a
    defended proof surface

## Promotion Criteria Matrix

### Route Contract

| Area | Status | Evidence | Remaining gap |
| --- | --- | --- | --- |
| Session lifecycle and inspection | strong proof | [codexw-broker-prototype-status.md](codexw-broker-prototype-status.md), [wrapper/tests/connector_prototype_smoke/workflows/session.rs](../wrapper/tests/connector_prototype_smoke/workflows/session.rs), [wrapper/tests/connector_prototype_smoke/fixture/session.rs](../wrapper/tests/connector_prototype_smoke/fixture/session.rs), [wrapper/src/local_api/tests/session/lifecycle.rs](../wrapper/src/local_api/tests/session/lifecycle.rs), [wrapper/src/local_api/tests/session/read.rs](../wrapper/src/local_api/tests/session/read.rs), [wrapper/src/bin/codexw_connector_prototype/tests/routing.rs](../wrapper/src/bin/codexw_connector_prototype/tests/routing.rs) | No major prototype gap on the currently claimed session route surface. |
| Attachment renew/release | strong proof | [wrapper/tests/connector_prototype_smoke/workflows/session.rs](../wrapper/tests/connector_prototype_smoke/workflows/session.rs), [wrapper/tests/connector_prototype_smoke/fixture/session.rs](../wrapper/tests/connector_prototype_smoke/fixture/session.rs), [wrapper/src/local_api/tests/session/attachment.rs](../wrapper/src/local_api/tests/session/attachment.rs) | No major prototype gap. |
| Turn start/interrupt | strong proof | [wrapper/tests/connector_prototype_smoke/workflows/session.rs](../wrapper/tests/connector_prototype_smoke/workflows/session.rs), [wrapper/tests/connector_prototype_smoke/fixture/session.rs](../wrapper/tests/connector_prototype_smoke/fixture/session.rs) | Steer/resume semantics remain outside the current adapter proof set. |
| Transcript inspection | strong proof | [wrapper/tests/connector_prototype_smoke/workflows/session.rs](../wrapper/tests/connector_prototype_smoke/workflows/session.rs), [wrapper/tests/connector_prototype_smoke/fixture/session.rs](../wrapper/tests/connector_prototype_smoke/fixture/session.rs) | No major prototype gap. |
| Orchestration status/workers/dependencies | strong proof | [wrapper/tests/connector_prototype_smoke/workflows/session.rs](../wrapper/tests/connector_prototype_smoke/workflows/session.rs), [wrapper/tests/connector_prototype_smoke/fixture/session.rs](../wrapper/tests/connector_prototype_smoke/fixture/session.rs) | Schema freezing is still a promotion-time decision. |
| Shell list/detail/control | strong proof | [wrapper/tests/connector_prototype_smoke/aliases/session.rs](../wrapper/tests/connector_prototype_smoke/aliases/session.rs), [wrapper/tests/connector_prototype_smoke/aliases/services.rs](../wrapper/tests/connector_prototype_smoke/aliases/services.rs), [wrapper/tests/connector_prototype_smoke/workflows/services.rs](../wrapper/tests/connector_prototype_smoke/workflows/services.rs), [wrapper/tests/connector_prototype_smoke/fixture/shells.rs](../wrapper/tests/connector_prototype_smoke/fixture/shells.rs) | No major prototype gap. |
| Service list/detail/control | strong proof | [wrapper/tests/connector_prototype_smoke/workflows/services.rs](../wrapper/tests/connector_prototype_smoke/workflows/services.rs), [wrapper/tests/connector_prototype_smoke/fixture/services.rs](../wrapper/tests/connector_prototype_smoke/fixture/services.rs) | No major prototype gap. |
| Capability list/detail | strong proof | [wrapper/tests/connector_prototype_smoke/aliases/services.rs](../wrapper/tests/connector_prototype_smoke/aliases/services.rs), [wrapper/tests/connector_prototype_smoke/workflows/services.rs](../wrapper/tests/connector_prototype_smoke/workflows/services.rs), [wrapper/tests/connector_prototype_smoke/fixture/services.rs](../wrapper/tests/connector_prototype_smoke/fixture/services.rs) | No major prototype gap. |
| `client_event` publish | strong proof | [wrapper/tests/connector_prototype_smoke/fixture/events/client_events/basic.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/client_events/basic.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/client_events/handoff.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/client_events/handoff.rs), [codexw-broker-client-fixture.md](codexw-broker-client-fixture.md) | No major prototype gap on the currently claimed client-event route surface. |
| Unsupported route behavior | strong proof | connector allowlist structure in [wrapper/src/bin/codexw_connector_prototype/routing.rs](../wrapper/src/bin/codexw_connector_prototype/routing.rs), explicit process-level rejection coverage in [wrapper/tests/connector_prototype_smoke/aliases/negative.rs](../wrapper/tests/connector_prototype_smoke/aliases/negative.rs) for unknown broker aliases plus disallowed raw proxy and raw proxy SSE paths | Broader fuzzing or exhaustive negative matrices are still optional hardening, not a missing promotion prerequisite for the currently claimed surface. |

### Error Contract

| Area | Status | Evidence | Remaining gap |
| --- | --- | --- | --- |
| Structured local-API error envelope | strong proof | [docs/codexw-local-api-route-matrix.md](codexw-local-api-route-matrix.md), local-API route tests under `wrapper/src/local_api/tests/*` | Promotion should freeze this more explicitly as adapter contract language. |
| Connector preserves structured local-API errors | strong proof | [wrapper/tests/connector_prototype_smoke/aliases/validation.rs](../wrapper/tests/connector_prototype_smoke/aliases/validation.rs), [wrapper/tests/connector_prototype_smoke/fixture/services.rs](../wrapper/tests/connector_prototype_smoke/fixture/services.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/leases/contention.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/contention.rs) | No major prototype gap on current lease/conflict behavior. |
| Field-accurate validation failures | strong proof | local-API validation tests under `wrapper/src/local_api/tests/*`, connector-local validation unit coverage in [wrapper/src/bin/codexw_connector_prototype/tests/upstream.rs](../wrapper/src/bin/codexw_connector_prototype/tests/upstream.rs) and [wrapper/src/bin/codexw_connector_prototype/tests/http.rs](../wrapper/src/bin/codexw_connector_prototype/tests/http.rs), and process-level connector smoke in [wrapper/tests/connector_prototype_smoke/aliases/validation.rs](../wrapper/tests/connector_prototype_smoke/aliases/validation.rs) for malformed lease headers, malformed injected JSON bodies, and preserved local field validation errors | No major prototype gap on the currently claimed validation surface; promotion should freeze the connector/local-API validation envelope as contract language rather than add more ad hoc route cases. |

### Event Contract

| Area | Status | Evidence | Remaining gap |
| --- | --- | --- | --- |
| Semantic SSE event stream exists | strong proof | [wrapper/src/local_api/events.rs](../wrapper/src/local_api/events.rs), [wrapper/tests/connector_prototype_smoke/workflows/events.rs](../wrapper/tests/connector_prototype_smoke/workflows/events.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/basic.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/basic.rs) | No major prototype gap. |
| `Last-Event-ID` replay/resume | strong proof | [wrapper/tests/connector_prototype_smoke/workflows/events.rs](../wrapper/tests/connector_prototype_smoke/workflows/events.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/basic.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/basic.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/client_events/handoff.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/client_events/handoff.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/client_events/reversal.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/client_events/reversal.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/leases/handoff.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/handoff.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/leases/reversal.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/reversal.rs) | Promotion may still want a stricter explicit event-order statement. |
| Event surface is semantic, not terminal-derived | strong proof | [wrapper/src/local_api/events.rs](../wrapper/src/local_api/events.rs), [wrapper/src/local_api/snapshot.rs](../wrapper/src/local_api/snapshot.rs), [docs/codexw-local-api-event-sourcing.md](codexw-local-api-event-sourcing.md) | No major prototype gap. |
| Multi-client event behavior under lease churn | strong proof | [wrapper/tests/connector_prototype_smoke/fixture/events/leases/contention.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/contention.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/leases/handoff.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/handoff.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/leases/reversal.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/reversal.rs) | Sustained or probabilistic churn is still optional future hardening, not a current missing route. |

### Client Policy Contract

| Area | Status | Evidence | Remaining gap |
| --- | --- | --- | --- |
| Owner / observer / rival roles are explicit | strong proof | [codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md), [codexw-broker-client-policy.md](codexw-broker-client-policy.md), [wrapper/tests/connector_prototype_smoke/fixture/events/leases/basic.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/basic.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/leases/contention.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/contention.rs) | The remaining question is promotion confidence, not whether the role model exists or is process-level defended. |
| Lease-owned versus observer-readable operations | strong proof | [wrapper/tests/connector_prototype_smoke/fixture/events/leases/basic.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/basic.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/leases/contention.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/contention.rs), [codexw-broker-client-fixture.md](codexw-broker-client-fixture.md) | No major prototype gap on the currently claimed named and anonymous observer/rival surface. |
| Renew/release/takeover semantics | strong proof | [wrapper/tests/connector_prototype_smoke/fixture/session.rs](../wrapper/tests/connector_prototype_smoke/fixture/session.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/leases/handoff.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/handoff.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/leases/reversal.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/reversal.rs) | No major prototype gap within the current process-scoped model. |
| Repeated role reversal | strong proof | [wrapper/tests/connector_prototype_smoke/fixture/events/leases/reversal.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/reversal.rs) | More sustained multi-client churn is still a possible follow-up, not a route deficiency. |
| Client event behavior under lease rules | strong proof | [wrapper/tests/connector_prototype_smoke/fixture/events/client_events/policy.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/client_events/policy.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/client_events/reversal.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/client_events/reversal.rs) | No major prototype gap on the currently claimed client-event surface. |

### Unsupported Boundary

| Area | Status | Evidence | Remaining gap |
| --- | --- | --- | --- |
| Unsupported broker/client surfaces are explicitly named | strong proof | [codexw-broker-out-of-scope.md](codexw-broker-out-of-scope.md), explicit process-level rejection coverage in [wrapper/tests/connector_prototype_smoke/aliases/negative.rs](../wrapper/tests/connector_prototype_smoke/aliases/negative.rs) for out-of-scope broker-style `scene` routes, unsupported global broker routes, unknown broker aliases, and disallowed raw proxy / raw proxy SSE paths | No major prototype gap on the currently claimed unsupported surface. |
| Connector remains thin and local API remains canonical | strong proof | [codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md), code organization under `wrapper/src/local_api/*` and `wrapper/src/bin/codexw_connector_prototype/*`, [codexw-broker-connector-mapping.md](codexw-broker-connector-mapping.md), and the negative-proof connector tests in [wrapper/tests/connector_prototype_smoke/aliases/negative.rs](../wrapper/tests/connector_prototype_smoke/aliases/negative.rs) | The remaining work is promotion judgment and long-term architecture choice, not a missing proof surface for the current thin-adapter claim. |

## Current Read

The current broker/local-API stack is already stronger than a normal prototype
in route coverage and process-level workflow proof.

The weakest remaining areas are no longer missing routes or missing contract
text. The adapter contract now exists explicitly in
[codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md), and
the operational support-level semantics now exist explicitly in
[codexw-broker-support-policy.md](codexw-broker-support-policy.md), and the
process-level proof surface is broad.

That means the next high-leverage work is not more route invention or policy
freezing. The repo now has enough evidence to support a concrete recommendation
in [codexw-broker-promotion-recommendation.md](codexw-broker-promotion-recommendation.md).
Further work is mostly about either reinforcing that recommendation with more
adversarial stress coverage cataloged in
[codexw-broker-hardening-catalog.md](codexw-broker-hardening-catalog.md), or
revising it if contradictory evidence appears.
