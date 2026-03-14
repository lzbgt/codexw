# codexw Broker Event Envelope Sketch

This document captures the event-envelope shape that emerged from the brokered
or remotely accessible `codexw` API work.

The goal is to expose stable semantic state transitions rather than terminal
presentation artifacts.

It should now be read as a historical design record plus a current semantic
reference, not as an unimplemented future proposal.

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
    "async_tool_supervision": {
      "classification": "tool_slow",
      "tool": "background_shell_start",
      "summary": "arguments= command=sleep 5 tool=background_shell_start",
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
    "async_tool_supervision": {
      "classification": "tool_wedged",
      "tool": "background_shell_start",
      "summary": "arguments= command=sleep 5 tool=background_shell_start",
      "elapsed_seconds": 75,
      "active_request_count": 1
    }
  }
}
```

`status.updated` now also carries the first emitted self-supervision audit-trail
slice: async-tool supervision classifications such as `tool_slow` and
`tool_wedged`.

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
