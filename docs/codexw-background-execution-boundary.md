# codexw Background Execution Boundary

## Purpose

This document makes one runtime boundary explicit:

- `codexw` has a wrapper-owned background execution lane
- the upstream `codex app-server` has its own exec/background-terminal lane
- those lanes may both exist in one session, but they do not co-own the same
  job

That distinction matters because recent hang bugs were wrapper-runtime defects
in `codexw`'s async dynamic-tool lane, not a protocol race where both sides
were trying to drive one shell process.

## Ownership Model

There are two different background-execution surfaces.

### 1. App-server-owned execution

This is the upstream lane:

- `command/exec`
- server-observed background terminals
- `item/commandExecution` and related app-server notifications

In this lane:

- the app-server owns the child process lifecycle
- `codexw` is an observer and renderer
- `codexw` may show process output and status, but does not pretend it owns the
  process model

### 2. Wrapper-owned background shell execution

This is the `codexw` lane:

- `background_shell_start`
- `background_shell_poll`
- `background_shell_send`
- `background_shell_wait_ready`
- `background_shell_invoke_recipe`
- related wrapper service/capability helpers

In this lane:

- `codexw` owns the async worker thread
- `codexw` owns the `BackgroundShellManager`
- `codexw` owns the background shell job ids such as `bg-1`
- `codexw` owns the supervision, timeout, and local failure behavior

## Non-Conflict Rule

The important rule is narrow:

- one background shell job should have one owner

So:

- wrapper-owned `background_shell_*` work is not also app-server `command/exec`
- app-server `command/exec` work is not also a wrapper `BackgroundShellManager`
  job

That means there should not be dual mutation authority over one running shell
job.

## Why Recent Hang Bugs Landed In codexw

The recent failures fit the wrapper-owned lane:

- a dynamic tool call was moved onto a dedicated wrapper worker thread
- the worker could still block forever
- the main runtime needed supervision, timeout, and admission control around
  that worker

Those are `codexw` architecture issues:

- async worker ownership
- local timeout policy
- abandoned-worker backlog handling
- inspection and operator visibility

They are not proof that the app-server background-terminal model is wrong.

## Correlation Model

For wrapper-owned `background_shell_start`, the runtime now has an explicit
correlation path:

1. the async dynamic-tool request has a `callId`
2. the started background shell job stores that origin `callId`
3. the main orchestrator can match the active async tool request to the
   wrapper-owned shell job
4. supervision can report the real job facts instead of a meaningless spinner

For wrapper-owned shell tools that target an existing shell, the runtime also
has a direct target-correlation path:

1. the async dynamic-tool request carries `jobId`, which may be a concrete
   `bg-*` id, alias, or `@capability`
2. `codexw` resolves that selector to the concrete wrapper-owned shell job at
   request start when possible
3. the main orchestrator can inspect that exact `bg-*` job while the async
   tool worker is still unresolved
4. supervision can report the same job/output facts for
   `background_shell_wait_ready`, `background_shell_poll`,
   `background_shell_send`, `background_shell_attach`, and
   `background_shell_invoke_recipe`, not only for `background_shell_start`

That resolved-target lane should remain explicit in machine-readable
supervision/event surfaces through fields such as
`target_background_shell_reference` and `target_background_shell_job_id`.

That gives the orchestrator concrete evidence such as:

- owner kind: `wrapper_background_shell`
- source call id
- wrapper job id
- job status
- command
- observed line count
- recent output preview

## Supervision Implications

Because of that correlation, the orchestrator should not treat every async
worker as equally opaque.

For wrapper-owned background shell work, it can distinguish at least:

- no job or output observed yet
- background shell job started but still silent, currently
  `wrapper_background_shell_started_no_output_yet`
- background shell job streaming output
- background shell job reached a terminal state but the dynamic tool response
  still has not returned

And it can report output freshness separately:

- `no_output_observed_yet`
- `recent_output_observed`
- `stale_output_observed`

That still is not a true forward-progress oracle for the worker thread itself.
It is a stronger inspection lane:

- the backend can see whether the worker is still unresolved
- the backend can see whether a wrapper-owned shell job exists
- the backend can see whether output is being observed
- the backend can decide whether to wait, poll, interrupt, or exit/resume

## Operator-Facing Rule

If `codexw` is going to keep an async background task alive, it should surface
concrete facts instead of only:

- tool name
- elapsed time

The minimum useful visible facts are:

- who owns the work
- what concrete command/tool summary is running
- the source call id when one exists
- whether a wrapper shell job exists yet
- the matched `bg-*` job id and status when one exists
- the latest observed output preview when one exists
- the next planned orchestrator health check horizon

## Design Consequence

Future background execution changes should preserve this split:

- do not blur wrapper-owned background shell jobs into app-server exec jobs
- do not claim progress certainty when only lifecycle/output evidence exists
- do keep correlation and ownership explicit across prompt, `:status`, local
  API, broker eventing, and self-supervision docs
