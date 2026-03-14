# codexw Broker Proof Matrix

This document maps the broker/local-API adapter claims to concrete repo
evidence.

Use it when checking whether the current supported experimental adapter claim
is still justified under
[codexw-broker-adapter-promotion.md](codexw-broker-adapter-promotion.md).

It does not redefine the broker design. It answers a narrower question:

- which claims are process-level proven
- which claims are only policy or design intent
- which areas still need stronger evidence before promotion

## Companion Docs

- [codexw-broker-adapter-status.md](codexw-broker-adapter-status.md)
- [codexw-broker-adapter-promotion.md](codexw-broker-adapter-promotion.md)
- [codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md)
- [codexw-broker-support-policy.md](codexw-broker-support-policy.md)
- [codexw-broker-client-policy.md](codexw-broker-client-policy.md)
- [codexw-broker-host-examination-matrix.md](codexw-broker-host-examination-matrix.md)
- [codexw-broker-integration-handoff.md](codexw-broker-integration-handoff.md)
- [codexw-cross-project-dependency-contract-sketch.md](codexw-cross-project-dependency-contract-sketch.md)
- [codexw-cross-project-dependency-implementation-plan.md](codexw-cross-project-dependency-implementation-plan.md)
- [codexw-broker-artifact-contract-sketch.md](codexw-broker-artifact-contract-sketch.md)
- [codexw-broker-out-of-scope.md](codexw-broker-out-of-scope.md)
- [codexw-broker-client-fixture.md](codexw-broker-client-fixture.md)
- [codexw-broker-connector-adapter-plan.md](codexw-broker-connector-adapter-plan.md)
- [codexw-support-claim-checklist.md](codexw-support-claim-checklist.md)

## Reading This Matrix

Status labels:

- `strong proof`
  - backed by process-level connector smoke and/or the real standalone broker
    client fixtures
- `partial proof`
  - backed by implementation plus some smoke proof, but not yet at the level
    desirable for promotion
- `doc/policy only`
  - specified clearly, but still primarily a contract statement rather than a
    defended proof surface

For the short implementer-facing summary of what the sibling `~/work/agent`
workspace can rely on today, see
[codexw-broker-integration-handoff.md](codexw-broker-integration-handoff.md).

## Promotion Criteria Matrix

### Route Contract

| Area | Status | Evidence | Remaining gap |
| --- | --- | --- | --- |
| Session lifecycle and inspection | strong proof | [codexw-broker-adapter-status.md](codexw-broker-adapter-status.md), [wrapper/tests/connector_prototype_smoke/workflows/session.rs](../wrapper/tests/connector_prototype_smoke/workflows/session.rs), [wrapper/tests/connector_prototype_smoke/fixture/session/lifecycle.rs](../wrapper/tests/connector_prototype_smoke/fixture/session/lifecycle.rs), [wrapper/tests/connector_prototype_smoke/aliases/session.rs](../wrapper/tests/connector_prototype_smoke/aliases/session.rs), [wrapper/src/local_api/tests/session/lifecycle.rs](../wrapper/src/local_api/tests/session/lifecycle.rs), [wrapper/src/local_api/tests/session/read.rs](../wrapper/src/local_api/tests/session/read.rs), [wrapper/src/bin/codexw_connector_prototype/tests/routing.rs](../wrapper/src/bin/codexw_connector_prototype/tests/routing.rs) | No major gap on the currently claimed session route surface; the connector now also asserts that every claimed broker alias still resolves to an allowlisted local target. |
| Attachment renew/release | strong proof | [wrapper/tests/connector_prototype_smoke/workflows/session.rs](../wrapper/tests/connector_prototype_smoke/workflows/session.rs), [wrapper/tests/connector_prototype_smoke/fixture/session/lifecycle.rs](../wrapper/tests/connector_prototype_smoke/fixture/session/lifecycle.rs), [wrapper/src/local_api/tests/session/attachment.rs](../wrapper/src/local_api/tests/session/attachment.rs) | No major gap on the currently claimed attachment surface. |
| Turn start/interrupt | strong proof | [wrapper/tests/connector_prototype_smoke/workflows/session.rs](../wrapper/tests/connector_prototype_smoke/workflows/session.rs), [wrapper/tests/connector_prototype_smoke/fixture/session/turns.rs](../wrapper/tests/connector_prototype_smoke/fixture/session/turns.rs), [wrapper/tests/connector_prototype_smoke/aliases/session.rs](../wrapper/tests/connector_prototype_smoke/aliases/session.rs), [wrapper/src/bin/codexw_connector_prototype/tests/routing.rs](../wrapper/src/bin/codexw_connector_prototype/tests/routing.rs) | Steer/resume semantics remain outside the current adapter proof set. |
| Transcript inspection | strong proof | [wrapper/tests/connector_prototype_smoke/workflows/session.rs](../wrapper/tests/connector_prototype_smoke/workflows/session.rs), [wrapper/tests/connector_prototype_smoke/fixture/session/turns.rs](../wrapper/tests/connector_prototype_smoke/fixture/session/turns.rs), [wrapper/tests/connector_prototype_smoke/aliases/session.rs](../wrapper/tests/connector_prototype_smoke/aliases/session.rs) | No major gap on the currently claimed transcript surface. |
| Orchestration status/workers/dependencies | strong proof | [wrapper/tests/connector_prototype_smoke/workflows/session.rs](../wrapper/tests/connector_prototype_smoke/workflows/session.rs), [wrapper/tests/connector_prototype_smoke/fixture/session/orchestration.rs](../wrapper/tests/connector_prototype_smoke/fixture/session/orchestration.rs), [wrapper/tests/connector_prototype_smoke/aliases/session.rs](../wrapper/tests/connector_prototype_smoke/aliases/session.rs) | Schema freezing is still a promotion-time decision. |
| Shell list/detail/control | strong proof | [wrapper/tests/connector_prototype_smoke/aliases/session.rs](../wrapper/tests/connector_prototype_smoke/aliases/session.rs), [wrapper/tests/connector_prototype_smoke/aliases/services.rs](../wrapper/tests/connector_prototype_smoke/aliases/services.rs), [wrapper/tests/connector_prototype_smoke/workflows/services.rs](../wrapper/tests/connector_prototype_smoke/workflows/services.rs), [wrapper/tests/connector_prototype_smoke/fixture/shells.rs](../wrapper/tests/connector_prototype_smoke/fixture/shells.rs), [wrapper/src/bin/codexw_connector_prototype/tests/routing.rs](../wrapper/src/bin/codexw_connector_prototype/tests/routing.rs) | No major gap on the currently claimed shell surface, including percent-decoded alias refs on detail/control routes and thin raw-proxy canonical reads. |
| Service list/detail/control | strong proof | [wrapper/tests/connector_prototype_smoke/workflows/services.rs](../wrapper/tests/connector_prototype_smoke/workflows/services.rs), [wrapper/tests/connector_prototype_smoke/fixture/services/interaction.rs](../wrapper/tests/connector_prototype_smoke/fixture/services/interaction.rs), [wrapper/tests/connector_prototype_smoke/fixture/services/mutations.rs](../wrapper/tests/connector_prototype_smoke/fixture/services/mutations.rs), [wrapper/tests/connector_prototype_smoke/aliases/services.rs](../wrapper/tests/connector_prototype_smoke/aliases/services.rs), [wrapper/src/bin/codexw_connector_prototype/tests/routing.rs](../wrapper/src/bin/codexw_connector_prototype/tests/routing.rs) | No major gap on the currently claimed service surface, including percent-decoded alias refs on detail/mutating action routes and thin raw-proxy canonical reads. |
| Capability list/detail | strong proof | [wrapper/tests/connector_prototype_smoke/aliases/services.rs](../wrapper/tests/connector_prototype_smoke/aliases/services.rs), [wrapper/tests/connector_prototype_smoke/workflows/services.rs](../wrapper/tests/connector_prototype_smoke/workflows/services.rs), [wrapper/tests/connector_prototype_smoke/fixture/services/mutations.rs](../wrapper/tests/connector_prototype_smoke/fixture/services/mutations.rs), [wrapper/src/bin/codexw_connector_prototype/tests/routing.rs](../wrapper/src/bin/codexw_connector_prototype/tests/routing.rs) | No major gap on the currently claimed capability surface, including explicit rejection of malformed percent-encoded alias refs and thin raw-proxy canonical reads. |
| `client_event` publish | strong proof | [wrapper/tests/connector_prototype_smoke/fixture/events/client_events/basic.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/client_events/basic.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/client_events/handoff.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/client_events/handoff.rs), [wrapper/tests/connector_prototype_smoke/aliases/session.rs](../wrapper/tests/connector_prototype_smoke/aliases/session.rs), [codexw-broker-client-fixture.md](codexw-broker-client-fixture.md) | No major gap on the currently claimed client-event route surface. |
| Unsupported route behavior | strong proof | connector allowlist structure in [wrapper/src/bin/codexw_connector_prototype/routing.rs](../wrapper/src/bin/codexw_connector_prototype/routing.rs), explicit process-level rejection coverage in [wrapper/tests/connector_prototype_smoke/aliases/negative.rs](../wrapper/tests/connector_prototype_smoke/aliases/negative.rs) for unknown broker aliases, wrong-method read-only and write-only aliases, explicit `method_not_allowed` rejection on SSE routes, plus disallowed raw proxy and raw proxy SSE paths | Broader fuzzing or exhaustive negative matrices are still optional hardening, not a missing promotion prerequisite for the currently claimed surface. |
| Status / policy / proof doc consistency | strong proof | automated doc guard in [wrapper/tests/doc_consistency.rs](../wrapper/tests/doc_consistency.rs), plus the operational review flow in [codexw-support-claim-checklist.md](codexw-support-claim-checklist.md) | Future broker support-level wording changes should extend the guard instead of relying only on manual wording review. |

### Error Contract

| Area | Status | Evidence | Remaining gap |
| --- | --- | --- | --- |
| Structured local-API error envelope | strong proof | [docs/codexw-local-api-route-matrix.md](codexw-local-api-route-matrix.md), local-API route tests under `wrapper/src/local_api/tests/*` | Promotion should freeze this more explicitly as adapter contract language. |
| Connector preserves structured local-API errors | strong proof | [wrapper/tests/connector_prototype_smoke/aliases/validation.rs](../wrapper/tests/connector_prototype_smoke/aliases/validation.rs), [wrapper/tests/connector_prototype_smoke/fixture/services/conflicts.rs](../wrapper/tests/connector_prototype_smoke/fixture/services/conflicts.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/leases/contention.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/contention.rs), plus shared route-shape classifier coverage in [wrapper/src/bin/codexw_connector_prototype/tests/routing.rs](../wrapper/src/bin/codexw_connector_prototype/tests/routing.rs) and [wrapper/src/bin/codexw_connector_prototype/tests/upstream.rs](../wrapper/src/bin/codexw_connector_prototype/tests/upstream.rs) | No major gap on current lease/conflict behavior or on the current header-injection eligibility surface. |
| Field-accurate validation failures | strong proof | local-API validation tests under `wrapper/src/local_api/tests/*`, connector-local validation unit coverage in [wrapper/src/bin/codexw_connector_prototype/tests/upstream.rs](../wrapper/src/bin/codexw_connector_prototype/tests/upstream.rs) and [wrapper/src/bin/codexw_connector_prototype/tests/http.rs](../wrapper/src/bin/codexw_connector_prototype/tests/http.rs), and process-level connector smoke in [wrapper/tests/connector_prototype_smoke/aliases/validation.rs](../wrapper/tests/connector_prototype_smoke/aliases/validation.rs) for malformed lease headers, malformed injected JSON bodies, and preserved local field validation errors | No major gap on the currently claimed validation surface; promotion should freeze the connector/local-API validation envelope as contract language rather than add more ad hoc route cases. |
| Timeout-tolerant request parsing under fragmented reads | strong proof | shared parser in [wrapper/src/http_request_reader.rs](../wrapper/src/http_request_reader.rs), connector parser coverage in [wrapper/src/bin/codexw_connector_prototype/tests/http.rs](../wrapper/src/bin/codexw_connector_prototype/tests/http.rs), local-API parser coverage in [wrapper/src/local_api/server.rs](../wrapper/src/local_api/server.rs), plus targeted reruns of the Node attachment-lifecycle smoke and SSE replay tests during hardening batches | No major gap on the current request-ingest reliability surface; future work is sustained stress rather than basic parser correctness. |

### Event Contract

| Area | Status | Evidence | Remaining gap |
| --- | --- | --- | --- |
| Semantic SSE event stream exists | strong proof | [wrapper/src/local_api/events.rs](../wrapper/src/local_api/events.rs), [wrapper/src/bin/codexw_connector_prototype/tests/sse.rs](../wrapper/src/bin/codexw_connector_prototype/tests/sse.rs), [wrapper/tests/connector_prototype_smoke/workflows/events.rs](../wrapper/tests/connector_prototype_smoke/workflows/events.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/basic.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/basic.rs) | No major gap on the currently claimed event surface, including the header/body seam case where the first upstream `data:` line is fragmented before the connector wraps it and still preserves structured `supervision_notice` / `async_tool_backpressure` backlog semantics such as `recommended_action`, `recovery_policy`, `recovery_options`, and `oldest_request_id`. |
| `Last-Event-ID` replay/resume | strong proof | [wrapper/tests/connector_prototype_smoke/workflows/events.rs](../wrapper/tests/connector_prototype_smoke/workflows/events.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/basic.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/basic.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/client_events/handoff.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/client_events/handoff.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/client_events/reversal.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/client_events/reversal.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/leases/handoff.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/handoff.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/leases/reversal.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/reversal.rs), [wrapper/tests/connector_prototype_smoke/aliases/session.rs](../wrapper/tests/connector_prototype_smoke/aliases/session.rs) | Promotion may still want a stricter explicit event-order statement. |
| Event surface is semantic, not terminal-derived | strong proof | [wrapper/src/local_api/events.rs](../wrapper/src/local_api/events.rs), [wrapper/src/local_api/snapshot.rs](../wrapper/src/local_api/snapshot.rs), [docs/codexw-local-api-event-sourcing.md](codexw-local-api-event-sourcing.md) | No major gap on the currently claimed semantic event surface. |
| Async-tool supervision status is remotely observable | strong proof | [wrapper/src/local_api/events.rs](../wrapper/src/local_api/events.rs), [wrapper/src/local_api/snapshot.rs](../wrapper/src/local_api/snapshot.rs), [wrapper/tests/connector_prototype_smoke/workflows/events.rs](../wrapper/tests/connector_prototype_smoke/workflows/events.rs), [docs/codexw-broker-event-envelope.md](codexw-broker-event-envelope.md), [docs/codexw-self-supervision.md](codexw-self-supervision.md) | The current proof covers semantic exposure and replay of `tool_slow` / `tool_wedged`, their narrow recommended actions such as `observe_or_interrupt` and `interrupt_or_exit_resume`, the active `supervision_notice` alert object, recovery-policy decisions such as `warn_only` and `operator_interrupt_or_exit_resume` with `automation_ready=false`, explicit recovery options such as `observe_status`, `interrupt_turn`, and `exit_and_resume`, explicit active-worker identity facts such as `request_id` and `thread_name`, explicit owner-lane/correlation facts such as `owner=wrapper_background_shell`, `source_call_id`, `target_background_shell_reference`, `target_background_shell_job_id`, and `observed_background_shell_job`, including those same owner/correlation/inspection facts on `supervision_notice`, explicit observation-state facts such as `wrapper_background_shell_started_no_output_yet`, explicit output-freshness facts such as `output_state` and `last_output_age_seconds`, continued observation/output/job visibility on `abandoned_after_timeout` worker rows when the correlated shell remains observable, `async_tool_backpressure` for abandoned async worker backlog/saturation visibility plus retained timeout-correlation facts such as `oldest_request_id`, `oldest_thread_name`, `oldest_source_call_id`, `oldest_target_background_shell_reference`, and `oldest_target_background_shell_job_id`, plus oldest abandoned-worker inspection facts such as `oldest_observation_state`, `oldest_output_state`, and `oldest_observed_background_shell_job`, plus backlog `recommended_action`, `recovery_policy`, and `recovery_options` such as `observe_status`, `interrupt_turn`, and `exit_and_resume`, and `async_tool_workers` for dedicated worker thread names plus lifecycle states such as `running` and `abandoned_after_timeout`, not yet broker-driven automated recovery execution or true in-worker progress probes. |
| Multi-client event behavior under lease churn | strong proof | [wrapper/tests/connector_prototype_smoke/fixture/events/leases/contention.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/contention.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/leases/handoff.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/handoff.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/leases/reversal.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/reversal.rs) | Sustained or probabilistic churn is still optional future hardening, not a current missing route. |

### Client Policy Contract

| Area | Status | Evidence | Remaining gap |
| --- | --- | --- | --- |
| Owner / observer / rival roles are explicit | strong proof | [codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md), [codexw-broker-client-policy.md](codexw-broker-client-policy.md), [wrapper/tests/connector_prototype_smoke/fixture/events/leases/basic.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/basic.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/leases/contention.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/contention.rs) | The remaining question is promotion confidence, not whether the role model exists or is process-level defended. |
| Lease-owned versus observer-readable operations | strong proof | [wrapper/tests/connector_prototype_smoke/fixture/events/leases/basic.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/basic.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/leases/contention.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/contention.rs), [codexw-broker-client-fixture.md](codexw-broker-client-fixture.md) | No major gap on the currently claimed named and anonymous observer/rival surface. |
| Renew/release/takeover semantics | strong proof | [wrapper/tests/connector_prototype_smoke/fixture/session/lifecycle.rs](../wrapper/tests/connector_prototype_smoke/fixture/session/lifecycle.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/leases/handoff.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/handoff.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/leases/reversal.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/reversal.rs) | No major gap within the current process-scoped model. |
| Repeated role reversal | strong proof | [wrapper/tests/connector_prototype_smoke/fixture/events/leases/reversal.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/leases/reversal.rs) | More sustained multi-client churn is still a possible follow-up, not a route deficiency. |
| Client event behavior under lease rules | strong proof | [wrapper/tests/connector_prototype_smoke/fixture/events/client_events/policy.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/client_events/policy.rs), [wrapper/tests/connector_prototype_smoke/fixture/events/client_events/reversal.rs](../wrapper/tests/connector_prototype_smoke/fixture/events/client_events/reversal.rs) | No major gap on the currently claimed client-event surface. |

### Unsupported Boundary

| Area | Status | Evidence | Remaining gap |
| --- | --- | --- | --- |
| Unsupported broker/client surfaces are explicitly named | strong proof | [codexw-broker-out-of-scope.md](codexw-broker-out-of-scope.md), explicit process-level rejection coverage in [wrapper/tests/connector_prototype_smoke/aliases/negative.rs](../wrapper/tests/connector_prototype_smoke/aliases/negative.rs) for out-of-scope broker-style `scene` routes, unsupported global broker routes, unknown broker aliases, and disallowed raw proxy / raw proxy SSE paths | No major gap on the currently claimed unsupported surface. |
| Connector remains thin and local API remains canonical | strong proof | [codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md), code organization under `wrapper/src/local_api/*` and `wrapper/src/bin/codexw_connector_prototype/*`, [codexw-broker-connector-mapping.md](codexw-broker-connector-mapping.md), and the negative-proof connector tests in [wrapper/tests/connector_prototype_smoke/aliases/negative.rs](../wrapper/tests/connector_prototype_smoke/aliases/negative.rs) | The remaining work is promotion judgment and long-term architecture choice, not a missing proof surface for the current thin-adapter claim. |

### Client Workflow Coverage

| Workflow | Status | Evidence | Remaining gap |
| --- | --- | --- | --- |
| Remote host examination through session/event/orchestration/shell/service surfaces | strong proof | Route and workflow proof across [codexw-broker-adapter-status.md](codexw-broker-adapter-status.md), [wrapper/tests/connector_prototype_smoke/aliases/session.rs](../wrapper/tests/connector_prototype_smoke/aliases/session.rs), [wrapper/tests/connector_prototype_smoke/aliases/services.rs](../wrapper/tests/connector_prototype_smoke/aliases/services.rs), [wrapper/tests/connector_prototype_smoke/workflows/session.rs](../wrapper/tests/connector_prototype_smoke/workflows/session.rs), [wrapper/tests/connector_prototype_smoke/workflows/services.rs](../wrapper/tests/connector_prototype_smoke/workflows/services.rs), and [codexw-broker-client-fixture.md](codexw-broker-client-fixture.md) | The current broker-visible shell/service/transcript/event surface is already a valid remote host-examination foundation. |
| Artifact browsing via transcript/event/shell/service references | partial proof | Transcript, SSE, shell detail/poll, service attach/run, and fixture-driven workflows exercised in the same evidence set above, plus the workflow framing in [codexw-broker-host-examination-matrix.md](codexw-broker-host-examination-matrix.md) | `codexw` still lacks a dedicated broker-visible artifact catalog or fetch/download contract; clients must currently assemble artifact understanding from transcript, shell, service, and event references. The intended next design slice is now sketched in [codexw-broker-artifact-contract-sketch.md](codexw-broker-artifact-contract-sketch.md). |
| Project-aware cross-deployment collaboration metadata | doc/policy only | [codexw-cross-project-dependency-collaboration.md](codexw-cross-project-dependency-collaboration.md), [codexw-cross-project-dependency-contract-sketch.md](codexw-cross-project-dependency-contract-sketch.md), [codexw-cross-project-dependency-implementation-plan.md](codexw-cross-project-dependency-implementation-plan.md), and the linked handoff docs | This lane is intentionally designed, but there is not yet route-level or process-level proof for session project assignment or dependency-edge routes. |

## Current Read

The current broker/local-API stack already has broad route coverage and
process-level workflow proof for the current supported experimental adapter
recommendation.

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
revising it if contradictory evidence appears. For the current workflow-level
host-examination read, including the remaining artifact-contract gap, see
[codexw-broker-host-examination-matrix.md](codexw-broker-host-examination-matrix.md).
For the separate design-only cross-project dependency lane, see
[codexw-cross-project-dependency-contract-sketch.md](codexw-cross-project-dependency-contract-sketch.md)
and
[codexw-cross-project-dependency-implementation-plan.md](codexw-cross-project-dependency-implementation-plan.md).
