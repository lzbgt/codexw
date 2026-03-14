# codexw Self-Supervision Implementation Plan

## Purpose

This document turns self-supervision into an implementation-facing sequence.

It sits below:

- [codexw-self-supervision.md](codexw-self-supervision.md)
- [codexw-background-execution-boundary.md](codexw-background-execution-boundary.md)
- [codexw-self-evolution.md](codexw-self-evolution.md)
- [codexw-plugin-system.md](codexw-plugin-system.md)

## Goal

Deliver the smallest supervision slice that ensures:

- wedged tool paths do not freeze the input loop indefinitely
- stalled states are visible and classified
- recovery can escalate from warning to interrupt to self-heal

## First Deliverables

The first implementation slice should include:

- non-blocking execution for potentially wedging dynamic-tool paths
- elapsed-time tracking for active tool calls
- operator-visible stall warnings
- supervision classifications for common wedged states
- hooks that can invoke self-evolution or plugin update policy later

## Suggested Delivery Order

### 1. Keep the input loop alive

Move the highest-risk tool families off the direct input-loop execution path so
the operator can still interrupt or exit while they run.

The first obvious target is:

- background-shell dynamic tools

### 2. Supervision timers

Track tool-call and shell-operation elapsed time so the runtime can detect:

- unusually slow operations
- repeated terminal retries
- likely wedged calls

### 3. Classification and status

Emit machine- and operator-readable classifications such as:

- `tool_slow`
- `tool_wedged`
- `shell_start_stalled`
- `self_handoff_ack_timeout`

The first concrete emitted slice should classify long-running async shell-tool
work at least as `tool_slow` and `tool_wedged` in prompt/runtime status.

The inspection cadence for that slice should remain an orchestrator policy
decision, not a single hardcoded interval. The runtime should choose the next
inspection point from the scale signals it actually has locally, such as tool
kind, timeout budget, elapsed runtime, and whether completion or output has
been observed.

That slice should also expose a small machine-readable recommendation field so
clients do not need to invent recovery guidance from the class label alone:

- `tool_slow` -> `observe_or_interrupt`
- `tool_wedged` -> `interrupt_or_exit_resume`

It should also raise a sticky `supervision_notice` record when the class
crosses into a supervised state, so the runtime and external clients can react
to an alert lifecycle instead of polling only the raw classification field.

### 4. Recovery policy hooks

When a stalled state is classified, the runtime should decide whether to:

- warn only
- interrupt the active turn
- preserve a manual resume path
- invoke self-evolution for a core fix
- prefer plugin update for an optional capability gap

The first delivered policy hook should stay narrow and non-autonomous:

- `tool_slow` -> `warn_only`
- `tool_wedged` -> `operator_interrupt_or_exit_resume`
- `automation_ready=false` for both, so the emitted policy is explicit without
  pretending the runtime already performs those recovery steps by itself

The same slice should emit explicit recovery options:

- `observe_status` via `GET /api/v1/session/{session_id}`
- `interrupt_turn` via `POST /api/v1/session/{session_id}/turn/interrupt`
- `exit_and_resume` via the concrete `codexw --cwd ... resume ...` command when
  a thread id is available
- a runtime-enforced async-tool deadline that fails an overdue request locally
  instead of waiting forever for the worker thread to return
- explicit abandoned async backlog tracking after that local timeout
- `async_tool_backpressure` in the local snapshot/SSE status slice
- admission control that refuses new background-shell async requests once the
  abandoned async backlog is saturated

### 5. Audit trail

Keep supervision actions visible through status text or event logs so recovery
is inspectable rather than mysterious.

The first audit-trail slice should expose supervision classifications through
the local API snapshot and `status.updated` SSE events, so WebUI or broker
clients can observe `tool_slow` and `tool_wedged` plus their recommended next
operator action without scraping prompt text.

That same slice should carry `supervision_notice` in the snapshot and
`status.updated` payload so alert raise/escalate/clear state is semantic rather
than inferred from prompt wording.

It should also carry the recovery-policy decision object, so clients can
distinguish a warning-only state from an operator-interrupt/exit-resume state
without reverse-engineering the recommended-action string.

It should also carry explicit `recovery_options`, so clients can present or log
the actual next-step affordances without inventing their own route mapping.

It should also carry an `async_tool_workers` inspection slice so an agent
backend can inspect the dedicated worker-thread lane directly:

- request id
- dedicated worker thread name
- lifecycle state such as `running` or `abandoned_after_timeout`
- runtime/state elapsed seconds and hard timeout
- current per-worker supervision classification when one exists
- current observation state such as
  `no_job_or_output_observed_yet` or
  `wrapper_background_shell_streaming_output`
- current output freshness state such as `no_output_observed_yet`,
  `recent_output_observed`, or `stale_output_observed`
- next planned orchestrator health check horizon in seconds
- explicit owner lane such as `wrapper_background_shell`
- source call id when available
- if a worker times out, the abandoned backlog should retain that same source
  call id plus any resolved target fields such as
  `target_background_shell_reference` and
  `target_background_shell_job_id`
- if the correlated shell is still visible, that abandoned worker entry should
  still carry current `observation_state`, `output_state`, and matched `bg-*`
  job facts instead of falling back to null inspection fields
- when the owner is the wrapper background-shell lane, the matched `bg-*` job
  snapshot facts needed for operator inspection:
  job id, status, command, line count, output age, and recent output preview

The same audit trail should keep the concrete tool summary or shell command
visible in periodic inspection notices, so the operator is not left with only
a generic background-tool label while the worker remains unresolved.

Those live inspection notices should also keep the structured supervision facts
visible in-line:

- observation state
- output state
- source call id when available
- next planned health check horizon
- matched `bg-*` job id, status, command, line count, output age, and latest
  output preview when available

That slice should stay explicit about its limit: it is an inspection-ready
worker-lifecycle report, not yet a proof that a blocking call inside the worker
thread is still making forward progress.

The current wrapper lane should also stay explicit in the implementation:

- `background_shell_*` async workers are `codexw`-owned
- app-server `command/exec` and server-observed background terminals are a
  separate ownership model
- recent hang fixes in this track are wrapper-runtime fixes, not claims that
  both layers were co-owning one background task

The current self-heal floor should be explicit: if an async shell-tool worker
does not return before its bounded runtime limit, `codexw` should emit a failed
tool response itself and let the turn continue, even if the detached worker
thread later returns and must be ignored.

That worker model should stay explicit in the implementation:

- background-shell tool calls belong on dedicated wrapper worker threads, not
  the main runtime loop
- the bug being fixed here is wrapper-side supervision/admission behavior in
  `codexw`, not the upstream app-server transport model

## Explicitly Deferred

The first slice should defer:

- fully autonomous always-on self-replacement
- broker-coordinated supervision across hosts
- arbitrary remote update discovery
- plugin marketplace semantics

## Proof Expectations

The first implementation should prove:

- a wedged background-shell tool path no longer freezes input handling
- repeated interrupts can still break out of a stalled session
- supervision classifications are emitted for representative stalled cases
- the runtime can choose between plugin-first and core-replacement recovery
  policy without conflating the two
