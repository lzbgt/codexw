# codexw Self-Supervision Implementation Plan

## Purpose

This document turns self-supervision into an implementation-facing sequence.

It sits below:

- [codexw-self-supervision.md](codexw-self-supervision.md)
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
