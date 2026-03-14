# codexw Broker Host Examination Matrix

## Purpose

This document translates the broker/client architecture requirement into
concrete remote-client workflows.

The question is not only whether `codexw` exposes enough individual routes. The
practical question is:

"Can an app or WebUI client attached through the broker actually examine the
host and the resulting outputs without direct terminal access?"

This matrix answers that workflow-level question against the current
implementation.

Companion docs:

- [codexw-broker-client-architecture.md](codexw-broker-client-architecture.md)
- [codexw-broker-artifact-contract-sketch.md](codexw-broker-artifact-contract-sketch.md)
- [codexw-broker-adapter-status.md](codexw-broker-adapter-status.md)
- [codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md)
- [codexw-broker-adapter-contract.md](codexw-broker-adapter-contract.md)
- [codexw-local-api-route-matrix.md](codexw-local-api-route-matrix.md)

## Reading This Matrix

Status labels:

- `strong`
  - current route surface plus process-level proof already support the workflow
- `usable with caveat`
  - the workflow is achievable now, but the client must assemble it from
    lower-level surfaces rather than rely on a dedicated higher-level contract
- `gap`
  - current routes/proof do not yet provide a coherent workflow for this area

## Workflow Matrix

| Workflow | Status | Current surface | Current caveat / gap |
| --- | --- | --- | --- |
| Inspect session identity and attachment state | strong | Session create/list/inspect/attach plus attachment renew/release through local API, connector aliases, and fixture/smoke coverage | No major gap on the currently claimed route surface. |
| Observe live progress and replay recent history | strong | Transcript snapshot, semantic SSE session events, and `Last-Event-ID` replay/resume | Event taxonomy is semantic and replayable, but not yet frozen as a richer app-facing product schema beyond the current adapter contract. |
| Diagnose slow or wedged wrapper-owned async shell work remotely | strong | `status.updated`, session snapshot supervision state, explicit classifications such as `tool_slow` / `tool_wedged`, observation/output facts such as `wrapper_background_shell_started_no_output_yet` and `no_output_observed_yet`, correlated `observed_background_shell_job`, `async_tool_backpressure`, and `async_tool_workers` | The current surface can show ownership, correlated `bg-*` shell facts, silent-started versus streaming versus terminal-without-tool-response states, and abandoned worker backlog, but it is still not proof of true in-worker forward progress; remote clients are observing the wrapper supervision lane rather than controlling a separate app-server background-task API. |
| Inspect orchestration blockers, workers, and dependency state | strong | Orchestration status/workers/dependencies routes plus broker-style aliases and fixture coverage | The current surface is already useful for app/WebUI state views; schema freezing remains a support-level decision. |
| Start and control host shell work remotely | strong | Shell list/start/detail/poll/send/terminate through the local API, connector aliases, and process-level smoke | This is the current primary broker-visible host execution surface. |
| Inspect reusable host services and capabilities | strong | Service list/detail/attach/wait/run plus capability list/detail, with mutation coverage for provide/depend/contract/relabel | Useful for remote service-centric inspection and interaction flows today. |
| Publish client collaboration events back into the session | strong | `client_event` publish plus replay/resume and lease-policy proof | No major gap on the currently claimed collaboration/event-ingest surface. |
| Examine host command output after remote shell execution | usable with caveat | Shell detail/poll snapshots, transcript snapshots, and semantic event stream | There is no separate broker-visible artifact/result catalog yet; clients currently assemble this from shell snapshots and transcript/event history. |
| Examine artifact references produced by the runtime | usable with caveat | Transcript items, semantic events, service attachment metadata, and shell output can all carry paths, endpoints, labels, and other result hints | The runtime exposes result references, but it does not yet define a dedicated broker-facing artifact inventory or artifact fetch/download contract. |
| Build a richer app/WebUI artifact browser over broker routes | gap | Indirectly possible only by scraping or interpreting transcript, shell, and service/result references | `codexw` currently lacks a first-class broker-visible artifact catalog with stable listing/fetch semantics, and artifact list/detail/content routes are not part of the current supported experimental adapter until they are implemented and proven explicitly. |

## Current Reality

For broker-exposed host examination, `codexw` is already meaningfully capable:

- remote clients can inspect session state
- remote clients can watch live events and replay them
- remote clients can inspect orchestration state
- remote clients can start and control host shell work
- remote clients can inspect and operate reusable services

That is enough to support real engineering workflows through broker-facing
clients today.

## Current Artifact Boundary

The remaining workflow gap is not generic host access. It is artifact
normalization.

Today, host results are exposed through:

- transcript snapshots
- supervision/status snapshots for active async shell-tool ownership and
  correlated shell-job facts
- shell snapshots and control responses
- semantic event payloads
- service attachment metadata and recipe outputs
- explicit path/endpoint text carried by those surfaces

What does **not** exist yet is a separate broker-facing artifact contract such
as:

- artifact list/index
- artifact detail metadata
- artifact fetch/download endpoint
- artifact type/schema guarantees for app/WebUI rendering

That means the current remote host-examination story is shell-first and
transcript/event-first rather than artifact-catalog-first.

Those missing routes are outside the current supported experimental adapter
claim. The supported shell-first host-examination foundation is real today, but
the artifact browser lane remains a separate design/implementation track until
route, proof, and policy updates land together.

For the concrete design sketch of that missing contract, see
[codexw-broker-artifact-contract-sketch.md](codexw-broker-artifact-contract-sketch.md).

## Design Consequence

Broker/client work should preserve two truths at once:

1. the current shell/service/transcript/event surface is already a valid remote
   host-examination foundation
2. richer artifact-centric app/WebUI workflows will need an explicit artifact
   contract instead of assuming transcript or shell text alone is sufficient

The repo should avoid describing the current state as if a dedicated artifact
API already exists, but it should also avoid understating the value of the
existing shell/service/transcript broker surface.

## Near-Term Guidance

When evaluating broker-facing changes, use this sequence:

1. Can the workflow already be expressed through session, event, shell, or
   service routes?
2. If yes, keep the current shell/transcript/event-first posture explicit,
   including the supervision/status slice for wrapper-owned async shell work.
3. If the remaining need is stable artifact browsing or download semantics,
   track it as an artifact-contract gap instead of inventing another ad hoc
   shell or transcript route.
4. Do not describe artifact routes as part of the supported broker adapter
   until the contract, proof, and support-policy docs are updated in the same
   batch.
