# codexw Broker Event Envelope Sketch

This document captures the first concrete event-envelope candidate for brokered
or remotely accessible `codexw` APIs.

The goal is to expose stable semantic state transitions rather than terminal
presentation artifacts.

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
    "elapsed_ms": 12450
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

## First-Phase Event Families

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

## Open Questions

- Should event replay be explicit via cursor or event id?
- Which events should be snapshotted versus emitted only as deltas?
- Does `session_id` represent a wrapper session, a remote client attachment, or a stable alias over a local thread?
- Should `source` remain a flat string or become a richer producer object?
