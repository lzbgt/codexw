# codexw Broker Event Envelope Sketch

This document captures the event-envelope shape that emerged from the brokered
or remotely accessible `codexw` API work.

The goal is to expose stable semantic state transitions rather than terminal
presentation artifacts.

It should now be read as a historical design record plus a current semantic
reference, not as an unimplemented future proposal.

It should also be read as the event contract for the current supported experimental adapter's
shell-first host-examination surface. It does not imply that `codexw` already
exposes a separate artifact index/detail/content event family before the
dedicated artifact-contract track is implemented and proven.

For the source docs that define that current shell-first remote
host-examination surface, see:

- [codexw-workspace-tool-policy.md](codexw-workspace-tool-policy.md)
- [codexw-local-api-sketch.md](codexw-local-api-sketch.md)
- [codexw-local-api-implementation-plan.md](codexw-local-api-implementation-plan.md)
- [codexw-local-api-event-sourcing.md](codexw-local-api-event-sourcing.md)
- [codexw-local-api-route-matrix.md](codexw-local-api-route-matrix.md)

## Core Envelope

Candidate envelope:

```json
{
  "type": "status.updated",
  "session_id": "sess_01HX...",
  "thread_id": "thread_abc123",
  "turn_id": "turn_def456",
  "item_id": "item_ghi789",
  "ts_unix_ms": 1760000000000,
  "source": "codexw",
  "data": {
    "working": true,
    "supervision_notice": {
      "classification": "tool_slow",
      "recommended_action": "observe_or_interrupt",
      "recovery_policy": {
        "kind": "warn_only",
        "automation_ready": false
      },
      "recovery_options": [
        {
          "kind": "observe_status",
          "label": "Observe current session status",
          "automation_ready": false,
          "cli_command": null,
          "local_api_method": "GET",
          "local_api_path": "/api/v1/session/sess_01HX..."
        },
        {
          "kind": "interrupt_turn",
          "label": "Interrupt the active turn",
          "automation_ready": false,
          "cli_command": null,
          "local_api_method": "POST",
          "local_api_path": "/api/v1/session/sess_01HX.../turn/interrupt"
        }
      ],
      "tool": "background_shell_start",
      "summary": "arguments= command=sleep 5 tool=background_shell_start"
    },
    "async_tool_supervision": {
      "classification": "tool_slow",
      "recommended_action": "observe_or_interrupt",
      "recovery_policy": {
        "kind": "warn_only",
        "automation_ready": false
      },
      "recovery_options": [
        {
          "kind": "observe_status",
          "label": "Observe current session status",
          "automation_ready": false,
          "cli_command": null,
          "local_api_method": "GET",
          "local_api_path": "/api/v1/session/sess_01HX..."
        },
        {
          "kind": "interrupt_turn",
          "label": "Interrupt the active turn",
          "automation_ready": false,
          "cli_command": null,
          "local_api_method": "POST",
          "local_api_path": "/api/v1/session/sess_01HX.../turn/interrupt"
        }
      ],
      "owner": "wrapper_background_shell",
      "source_call_id": "call_123",
      "target_background_shell_reference": "dev.api",
      "target_background_shell_job_id": "bg-1",
      "tool": "background_shell_start",
      "summary": "arguments= command=sleep 5 tool=background_shell_start",
      "observation_state": "wrapper_background_shell_streaming_output",
      "output_state": "recent_output_observed",
      "observed_background_shell_job": {
        "job_id": "bg-1",
        "status": "running",
        "command": "npm run dev",
        "total_lines": 1,
        "last_output_age_seconds": 2,
        "recent_lines": ["READY"]
      },
      "next_check_in_seconds": 9,
      "elapsed_seconds": 21,
      "active_request_count": 1
    }
  }
}
```

## Required Fields

- `type`
- `session_id`
- `ts_unix_ms`
- `data`

## Optional Fields

- `thread_id`
- `turn_id`
- `item_id`
- `source`

Optional means “present only when semantically relevant,” not “randomly omitted.”

## Current Event Families

### Session

- `session.attached`
- `session.updated`

### Turn

- `turn.started`
- `turn.completed`
- `turn.interrupted`

### Transcript

- `transcript.item`

### Status

- `status.updated`

### Orchestration

- `orchestration.updated`

### Background Shells And Services

- `shell.updated`
- `service.updated`
- `capability.updated`

## Example Events

### `turn.started`

```json
{
  "type": "turn.started",
  "session_id": "sess_01HX...",
  "thread_id": "thread_abc123",
  "turn_id": "turn_def456",
  "ts_unix_ms": 1760000000000,
  "data": {
    "working": true
  }
}
```

### `transcript.item`

```json
{
  "type": "transcript.item",
  "session_id": "sess_01HX...",
  "thread_id": "thread_abc123",
  "turn_id": "turn_def456",
  "item_id": "item_ghi789",
  "ts_unix_ms": 1760000001000,
  "data": {
    "kind": "assistant",
    "text": "Reviewing the changed files now."
  }
}
```

### `orchestration.updated`

```json
{
  "type": "orchestration.updated",
  "session_id": "sess_01HX...",
  "thread_id": "thread_abc123",
  "ts_unix_ms": 1760000002000,
  "data": {
    "counts": {
      "waits": 1,
      "sidecar_agents": 2,
      "exec_prereqs": 1,
      "exec_services": 1
    },
    "next_action": {
      "kind": "tool_call",
      "tool": "background_shell_poll",
      "arguments": {
        "jobId": "bg-1"
      }
    }
  }
}
```

### `status.updated`

```json
{
  "type": "status.updated",
  "session_id": "sess_01HX...",
  "thread_id": "thread_abc123",
  "ts_unix_ms": 1760000001500,
  "data": {
    "turn_running": true,
    "supervision_notice": {
      "classification": "tool_wedged",
      "recommended_action": "interrupt_or_exit_resume",
      "recovery_policy": {
        "kind": "operator_interrupt_or_exit_resume",
        "automation_ready": false
      },
      "recovery_options": [
        {
          "kind": "interrupt_turn",
          "label": "Interrupt the active turn",
          "automation_ready": false,
          "cli_command": null,
          "local_api_method": "POST",
          "local_api_path": "/api/v1/session/sess_01HX.../turn/interrupt"
        },
        {
          "kind": "exit_and_resume",
          "label": "Exit and resume the thread in a newer client",
          "automation_ready": false,
          "cli_command": "codexw --cwd /repo resume thread_abc123",
          "local_api_method": null,
          "local_api_path": null
        }
      ],
      "tool": "background_shell_start",
      "summary": "arguments= command=sleep 5 tool=background_shell_start"
    },
    "async_tool_supervision": {
      "classification": "tool_wedged",
      "recommended_action": "interrupt_or_exit_resume",
      "recovery_policy": {
        "kind": "operator_interrupt_or_exit_resume",
        "automation_ready": false
      },
      "recovery_options": [
        {
          "kind": "interrupt_turn",
          "label": "Interrupt the active turn",
          "automation_ready": false,
          "cli_command": null,
          "local_api_method": "POST",
          "local_api_path": "/api/v1/session/sess_01HX.../turn/interrupt"
        },
        {
          "kind": "exit_and_resume",
          "label": "Exit and resume the thread in a newer client",
          "automation_ready": false,
          "cli_command": "codexw --cwd /repo resume thread_abc123",
          "local_api_method": null,
          "local_api_path": null
        }
      ],
      "owner": "wrapper_background_shell",
      "source_call_id": "call_123",
      "target_background_shell_reference": "dev.api",
      "target_background_shell_job_id": "bg-1",
      "tool": "background_shell_start",
      "summary": "arguments= command=sleep 5 tool=background_shell_start",
      "observation_state": "wrapper_background_shell_terminal_without_tool_response",
      "output_state": "stale_output_observed",
      "observed_background_shell_job": {
        "job_id": "bg-1",
        "status": "failed",
        "command": "npm run dev",
        "total_lines": 3,
        "last_output_age_seconds": 75,
        "recent_lines": ["boom"]
      },
      "next_check_in_seconds": 30,
      "elapsed_seconds": 75,
      "active_request_count": 1
    }
  }
}
```

`status.updated` now also carries the first emitted self-supervision audit-trail
slice: async-tool supervision classifications such as `tool_slow` and
`tool_wedged`, plus a narrow recommended-action field such as
`observe_or_interrupt` or `interrupt_or_exit_resume`, and a sticky
`supervision_notice` object for the active alert lifecycle, plus a
machine-readable recovery-policy object such as `warn_only` or
`operator_interrupt_or_exit_resume` with `automation_ready=false`, plus
explicit `recovery_options` such as `observe_status`, `interrupt_turn`, and
`exit_and_resume`, plus `async_tool_backpressure` so remote clients can see the
abandoned async worker backlog and whether new background-shell async requests
should currently be refused, including retained timeout-correlation facts such
as `oldest_request_id`, `oldest_thread_name`, `oldest_source_call_id`,
`oldest_target_background_shell_reference`, and
`oldest_target_background_shell_job_id`, plus oldest abandoned-worker
inspection facts such as `oldest_observation_state`,
`oldest_output_state`, and `oldest_observed_background_shell_job`, plus
backlog-level `recovery_options` so remote clients can render the same
`observe_status` / `interrupt_turn` / `exit_and_resume` next steps even after
the active worker has timed out, plus backlog-level `recommended_action` and
`recovery_policy` so clients do not have to infer severity from saturation
alone, plus
`async_tool_workers` so a remote agent
backend can inspect dedicated worker thread names and lifecycle states such as
`running` and `abandoned_after_timeout` without scraping prompt text, plus
explicit active-worker identity such as `request_id` and `thread_name`, plus
explicit owner-lane state such as `wrapper_background_shell`, source `callId`,
resolved target facts such as `target_background_shell_reference` and
`target_background_shell_job_id`, and correlated `bg-*` job facts when a
wrapper-owned `background_shell_start`
request has already produced a shell job, including abandoned worker rows when
that shell is still observable after timeout, plus output freshness through
`output_state` and `last_output_age_seconds`. Those same owner/correlation /
inspection facts now also travel on the sticky `supervision_notice` object, so
alert-only consumers do not have to recover them by joining a second slice.

## Exclusions

The first broker/local API event model should not expose:

- wrapped prompt layout
- ANSI formatting
- scrollback-specific block titles
- purely visual spinner frames

Those are local presentation concerns, not durable protocol.

## Compatibility Direction With `~/work/agent`

The envelope is intentionally close to the `agent` direction in the ways that
matter most:

- machine-readable
- append-only friendly
- session-scoped
- reconnect/replay friendly

But it should stay `codexw`-native in content:

- thread/turn/item ids should preserve existing `codexw` concepts
- orchestration and shell/service events should expose wrapper-owned semantics directly

## Remaining Design Questions

- Should event replay be explicit via cursor or event id?
- Which events should be snapshotted versus emitted only as deltas?
- Does `session_id` represent a wrapper session, a remote client attachment, or a stable alias over a local thread?
- Should `source` remain a flat string or become a richer producer object?
