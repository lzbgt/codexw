# codexw Broker Integration Handoff

## Purpose

This document is the short implementation-facing handoff for the sibling `~/work/agent` workspace
and any other broker/WebUI consumer of `codexw`.

The repo already has deeper design, proof, and support-policy docs. This file
answers a narrower question:

"If another workspace is implementing broker, app, or WebUI features against
`codexw`, what should it assume today, and what should it explicitly *not*
assume yet?"

## Intended Audience

Use this document when working in:

- `~/work/agent` broker code
- `~/work/agent` WebUI or app code
- any external client or integration layer that wants to consume `codexw`
  through the broker-facing adapter

This is not the contract source of truth by itself. It is the short handoff
that points implementers at the right contract docs.

## Current Product Direction

`codexw` is now intentionally aligned around this model:

1. `codexw` local HTTP/SSE API is the canonical runtime contract
2. a connector/adapter exposes a broker-facing client surface
3. broker-backed app/WebUI clients are first-class consumers of that surface
4. host examination is currently shell-first, not artifact-catalog-first

That means the external workspace should treat broker-backed session control,
event replay, orchestration inspection, and shell/service control as real
surfaces, not as temporary experiments.

## What External Clients Can Rely On Today

External broker-facing clients can already rely on the documented supported
experimental adapter surface for:

- session create / attach / list / inspect
- attachment renew / release
- turn start / interrupt
- transcript fetch
- SSE event consumption and `Last-Event-ID` replay/resume
- semantic `status.updated` supervision slices for async tool stalls and
  recovery hints
- orchestration status / workers / dependencies
- shell list / start / detail / poll / send / terminate
- service list / detail / attach / wait / run
- capability list / detail
- structured client event publication
- owner / observer / rival lease semantics

Primary contract references:

- [codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md)
- [codexw-broker-adapter-status.md](codexw-broker-adapter-status.md)
- [codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md)
- [codexw-broker-client-policy.md](codexw-broker-client-policy.md)
- [codexw-broker-connector-mapping.md](codexw-broker-connector-mapping.md)

## Current Host-Examination Posture

The current broker-visible host-examination model is:

- inspect session and transcript state
- inspect live event streams and resume them
- inspect semantic supervision state for active async shell-tool work
- inspect orchestration state
- start and control host shell work remotely
- inspect and operate reusable services and capabilities
- interpret resulting host references from transcript, shell, service, and
  event surfaces

This is intentionally **shell-first**. It is not based on the removed workspace
dynamic tools, and it does not yet include a dedicated artifact browser.

For active wrapper-owned background-shell work, external clients can now also
rely on broker-visible supervision facts rather than only a generic tool name:

- classifications such as `tool_slow` and `tool_wedged`
- narrow actions such as `observe_or_interrupt` and
  `interrupt_or_exit_resume`
- explicit owner-lane facts such as `wrapper_background_shell`
- explicit silent-started versus streaming output states such as
  `wrapper_background_shell_started_no_output_yet`,
  `wrapper_background_shell_streaming_output`, and matching output freshness
  values such as `no_output_observed_yet`, `recent_output_observed`, and
  `stale_output_observed`
- source request correlation through `source_call_id`
- resolved reuse-target correlation through
  `target_background_shell_reference` and
  `target_background_shell_job_id`
- matched `observed_background_shell_job` detail with `job_id`, `status`,
  `command`, `total_lines`, `last_output_age_seconds`, and recent output
  preview when the wrapper shell lane has already started a `bg-*` job
- abandoned-backlog visibility through `async_tool_backpressure`
- dedicated worker inspection through `async_tool_workers`

Workflow-level reference:

- [codexw-broker-host-examination-matrix.md](codexw-broker-host-examination-matrix.md)

## What External Clients Must Not Assume Yet

External broker/WebUI code should **not** assume that `codexw` currently has:

- a broker-visible artifact index route
- a broker-visible artifact detail route
- a broker-visible artifact content/download route
- generic filesystem browsing semantics through the broker adapter
- full `agentd` protocol parity
- multi-daemon lease coordination
- production deployment/auth semantics owned by `codexw` itself

Those missing pieces are not vague future work. They are explicit boundaries.
In particular, external clients should not assume a stable artifact
list/detail/content API exists yet.

Boundary references:

- [codexw-broker-out-of-scope.md](codexw-broker-out-of-scope.md)
- [codexw-broker-support-policy.md](codexw-broker-support-policy.md)
- [codexw-broker-promotion-recommendation.md](codexw-broker-promotion-recommendation.md)

## Artifact Track Rule

If the sibling workspace needs richer artifact-centric UX, route it through the
explicit artifact-contract track rather than stretching transcript or shell
surfaces ad hoc.

That means:

- do not pretend current transcript or shell references are already a stable
  artifact API
- do not infer artifact list/detail/content routes from unsupported connector
  passthrough behavior
- do not reintroduce workspace dynamic tools as the answer for broker-visible
  artifact browsing

Artifact-track references:

- [codexw-broker-artifact-contract-sketch.md](codexw-broker-artifact-contract-sketch.md)
- [codexw-broker-artifact-implementation-plan.md](codexw-broker-artifact-implementation-plan.md)

## Cross-Deployment Collaboration Boundary

The current supported surface is still centered on one `codexw` deployment at a
time. However, the architecture now also recognizes a next native requirement:
cross-deployment `codexw` collaboration and explicit work handoff.

External workspaces should treat that as a separate design track from both:

- the already-supported single-deployment session/event/shell/service surface
- the still-separate artifact-contract track

Reference:

- [codexw-cross-deployment-collaboration.md](codexw-cross-deployment-collaboration.md)
- [codexw-cross-project-dependency-collaboration.md](codexw-cross-project-dependency-collaboration.md)
- [codexw-cross-project-dependency-contract-sketch.md](codexw-cross-project-dependency-contract-sketch.md)
- [codexw-cross-project-dependency-implementation-plan.md](codexw-cross-project-dependency-implementation-plan.md)
- [codexw-cross-deployment-handoff-contract-sketch.md](codexw-cross-deployment-handoff-contract-sketch.md)
- [codexw-cross-deployment-handoff-implementation-plan.md](codexw-cross-deployment-handoff-implementation-plan.md)

## Recommended Consumption Order For `~/work/agent`

For the sibling workspace, the lowest-risk implementation order is:

1. consume the already-supported session / event / transcript / orchestration
   surfaces
2. consume shell / service / capability control as the first-class host
   examination path
3. assemble artifact understanding from transcript, shell, service, and event
   references only where needed
4. if stable artifact browsing becomes necessary, treat it as an explicit
   artifact-contract requirement and sync with `codexw` on that separate track
5. if one deployment needs to hand work to another deployment, treat that as an
   explicit cross-deployment collaboration/handoff requirement rather than as a
   hidden operator convention
6. if multiple deployments are each working on related projects, model that
   through broker-visible project identity and dependency-aware handoffs rather
   than assuming those deployments share one host or one workspace

This keeps the external client aligned with what `codexw` actually proves
today.

## Implementation Checklist For External Clients

When building against the current `codexw` broker adapter, confirm:

- the client uses `session_id` as the primary remote-control handle
- the client understands owner / observer / rival behavior for lease-owned
  mutations
- SSE consumers persist and reuse `Last-Event-ID` when reconnecting
- status consumers understand the current supervision slice:
  `tool_slow` / `tool_wedged`, owner lane, `source_call_id`, correlated
  `observed_background_shell_job`, `async_tool_backpressure`, and
  `async_tool_workers`
- host examination flows are designed around shell/service/transcript/event
  surfaces
- UI copy does not imply a first-class artifact browser unless artifact routes
  are actually implemented
- UI copy does not imply resurrected workspace dynamic tools

## External Baseline References

Relevant sibling-workspace references:

- `/Users/zongbaolu/work/agent/docs/BROKER.md`
- `/Users/zongbaolu/work/agent/docs/CLIENT.md`
- `/Users/zongbaolu/work/agent/docs/WEBUI.md`

These are the important external documents to compare against when deciding
whether a new broker/WebUI feature belongs in:

- the already-supported adapter surface
- optional hardening
- the separate artifact-contract track

## Practical Rule

If `~/work/agent` implementers ask:

"Can we build this against today's `codexw` broker surface?"

use this rule:

- if the workflow can be satisfied through session, event, orchestration,
  shell, service, capability, and transcript surfaces, the answer is probably
  yes
- if the workflow requires a stable artifact list/detail/content API, the
  answer is not yet, and the request belongs in the artifact-contract track
