# codexw Self-Supervision

## Purpose

This document records a non-optional runtime requirement:

- no `codexw` tool use or shell exec should be allowed to hang the runtime indefinitely without supervision
- the runtime should notice stuck states, classify them, and choose a recovery
  path
- self-evolution and plugin updates should be available as recovery tools, not
  just feature-delivery tools

## Problem Statement

The practical failure mode is straightforward:

- a dynamic tool call or shell workflow wedges
- the current task still matters
- the operator may not be able to or want to restart manually

Without supervision, `codexw` can recognize that something is wrong but still
leave the work trapped in the bad runtime generation.

## Design Stance

Self-supervision should be:

- standalone local-runtime-first
- explicit rather than mystical
- observable rather than silent
- escalation-based rather than immediately destructive
- linked to self-evolution rather than isolated from it

Broker participation may matter later, but the first supervision lane must work
for a standalone local instance.

## What Must Be Supervised

The first supervision lane should watch at least:

- active dynamic tool calls
- background shell starts, polls, waits, and recipe invokes
- turn-level activity that remains active without forward progress
- plugin lifecycle operations
- self-handoff attempts and acknowledgments

## Supervision Outcomes

The runtime should not have only one response.

Ordered escalation should be:

1. warn
2. classify the stalled state
3. interrupt or cancel when safe
4. offer or perform explicit rollback
5. hand off to a newer binary when a core fix is needed
6. prefer plugin install/update when the missing capability is plugin-suitable

## Classification Examples

Useful first classes:

- `tool_slow`
- `tool_wedged`
- `shell_start_stalled`
- `shell_poll_repeated_terminal_retry`
- `plugin_load_failed`
- `self_handoff_ack_timeout`

These classifications should be visible in operator-facing status rather than
hidden inside internal timers.

The first emitted native runtime slice should at least expose `tool_slow` and
`tool_wedged` for long-running async shell-tool work.

The first recommended actions should stay narrow and operator-safe:

- `tool_slow` -> `observe_or_interrupt`
- `tool_wedged` -> `interrupt_or_exit_resume`

The first recovery-policy decisions should also be machine-readable:

- `tool_slow` -> `warn_only`
- `tool_wedged` -> `operator_interrupt_or_exit_resume`
- `automation_ready` should remain `false` for both until autonomous
  interruption or replacement is actually implemented

The first recovery options should also be explicit enough for clients to render
or relay without inventing their own heuristics:

- `tool_slow` should expose `observe_status`, and when a turn is active also
  `interrupt_turn`
- `tool_wedged` should expose `interrupt_turn` and, when a thread is attached,
  `exit_and_resume`
- options should carry concrete local-API method/path or CLI resume command
  fields instead of forcing prompt scraping

The first emitted recovery signal should also be sticky enough to notice:

- raise a structured `supervision_notice` when a class threshold is crossed
- keep that notice active while the stalled condition remains true
- clear the notice explicitly when the tool issue is gone
- enforce a hard runtime limit for async shell-tool calls and fail the overdue
  request locally rather than leaving the active turn hung forever
- retain a machine-readable abandoned async worker backlog after that timeout,
  rather than pretending the detached worker disappeared
- expose that backlog through an `async_tool_backpressure` state slice
- once the abandoned async backlog is saturated, refuse new background-shell
  async requests locally until the backlog drains or the operator exits and
  resumes

## Relationship To Runtime Responsiveness

The first concrete runtime rule should be:

- background-shell dynamic tools must not execute in a way that freezes the input loop indefinitely

That means a wedged background-shell tool should still leave the operator able
to:

- observe what is happening
- interrupt the turn
- exit with a resume hint
- resume later in a newer generation if needed

The current self-heal floor should also treat abandoned async workers as a
first-class safety issue:

- background-shell tool calls belong on dedicated wrapper worker threads, not
  the main runtime loop
- the backend inspection lane should expose `async_tool_workers` with each
  worker's request id, dedicated thread name, and lifecycle state such as
  `running` or `abandoned_after_timeout`
- a timed-out detached worker may still return later, but its late response
  must be ignored for protocol correctness
- the runtime should keep counting that abandoned async worker until the late
  return arrives or the session is reset
- that count should become an admission-control input, not just a debug fact
- this inspection lane is intentionally narrower than a true progress oracle:
  it tells the agent backend which dedicated worker thread still exists and
  whether `codexw` still considers it running, but not yet whether a blocking
  syscall inside that worker is making forward progress

## Relationship To Self-Evolution

Self-supervision is the trigger discipline for self-evolution.

If supervision determines that:

- the runtime is wedged but checkpointable
- the repo contains a known fix
- the problem is in core runtime behavior rather than an optional extension

then self-evolution should be able to launch the safe-handoff path described in
[codexw-self-evolution.md](codexw-self-evolution.md).

## Relationship To Plugins

If supervision determines that:

- the needed capability is optional
- the missing capability fits the plugin API
- a trusted plugin update exists

then plugin installation or update should be preferred over core replacement.

That plugin track is documented in
[codexw-plugin-system.md](codexw-plugin-system.md).

## Boundaries

The first supervision lane should not promise:

- infallible diagnosis
- invisible autonomous behavior with no audit trail
- broker dependence for local recovery
- arbitrary untrusted code upgrades as a recovery method

It should promise something narrower and more useful:

- wedged tool and shell paths are observable
- recovery choices are explicit
- the runtime can escalate from warning to self-heal instead of remaining stuck
