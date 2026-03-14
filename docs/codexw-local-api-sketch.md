# codexw Local API Sketch

This document is the original local daemon-facing API sketch for `codexw`.
It now serves as a compact conceptual companion to the implemented surface,
not as the sole current source of truth for what exists today.

For current implementation scope and route ownership, use:

- [docs/codexw-local-api-route-matrix.md](codexw-local-api-route-matrix.md)
- [docs/codexw-local-api-implementation-plan.md](codexw-local-api-implementation-plan.md)
- [docs/codexw-broker-adapter-status.md](codexw-broker-adapter-status.md)

This sketch remains intentionally narrower than a full protocol spec. Its job
is to preserve the design intent and resource model behind the local API,
rather than to duplicate the full implemented route inventory.

## Design Goals

- preserve existing `codexw` identities:
  - `session_id`
  - `thread_id`
  - `turn_id`
  - `item_id`
- expose machine-readable state instead of terminal layout
- support local WebUI/mobile/remote-terminal attachment through one canonical
  control API
- support broker-exposed app/WebUI clients that need to inspect host shell
  activity and resulting artifacts without direct terminal access
- remain simple enough to implement before any broker transport work

## Transport Assumption

First implementation target:

- loopback-only HTTP API
- loopback-only SSE event streams
- optional local bearer token

This is consistent with the current broker design recommendation:

- local API first
- connector second

## Resource Model

### Session

A wrapper-owned remote-control context.

Properties:

- may be unattached
- may attach to one local `thread_id`
- owns remote control state for one connected client or client group

### Thread

The underlying Codex conversation identity already used by `codexw`.

### Turn

A submitted unit of user/model work within a thread.

### Orchestration

Derived runtime state summarizing:

- blockers
- sidecars
- dependencies
- services
- background shells

### Background Shell

A wrapper-owned long-lived shell job.

This is part of the intended broker-visible host examination surface, not just a
local operator convenience.

### Service Capability

A reusable role advertised by one or more running service shells.

## Initial Route Sketch

The route groups below preserve the original initial API sketch. They are
useful as a compact conceptual map, but they are no longer the authoritative
"what is implemented today" list. For the current route surface, use:

- [docs/codexw-local-api-route-matrix.md](codexw-local-api-route-matrix.md)
- [docs/codexw-local-api-implementation-plan.md](codexw-local-api-implementation-plan.md)

### Session Lifecycle

#### `POST /api/v1/session/new`

Creates a wrapper session.

Request:

```json
{
  "attach": {
    "thread_id": "thread_abc123"
  }
}
```

`attach` is optional.

Response:

```json
{
  "session_id": "sess_01HX...",
  "thread_id": "thread_abc123"
}
```

#### `POST /api/v1/session/attach`

Binds an existing wrapper session to an existing local thread.

Request:

```json
{
  "session_id": "sess_01HX...",
  "thread_id": "thread_abc123"
}
```

Response:

```json
{
  "ok": true,
  "session_id": "sess_01HX...",
  "thread_id": "thread_abc123"
}
```

#### `GET /api/v1/session/{session_id}`

Returns compact session status.

Response:

```json
{
  "session_id": "sess_01HX...",
  "thread_id": "thread_abc123",
  "working": true,
  "last_turn_id": "turn_def456"
}
```

### Turn Lifecycle

#### `POST /api/v1/turn/start`

Starts a new turn on the attached thread.

Request:

```json
{
  "session_id": "sess_01HX...",
  "input": {
    "text": "Review the changed files and summarize the blockers."
  }
}
```

Response:

```json
{
  "ok": true,
  "session_id": "sess_01HX...",
  "thread_id": "thread_abc123",
  "turn_id": "turn_def456"
}
```

#### `POST /api/v1/turn/interrupt`

Interrupts the currently active turn for that wrapper session.

Request:

```json
{
  "session_id": "sess_01HX..."
}
```

### Transcript And Event Streaming

#### `GET /api/v1/session/{session_id}/events`

Server-sent event stream using the envelope from
[docs/codexw-broker-event-envelope.md](docs/codexw-broker-event-envelope.md).

Original initial event families:

- `session.*`
- `turn.*`
- `transcript.item`
- `status.updated`
- `orchestration.updated`
- `shell.updated`
- `service.updated`
- `capability.updated`

#### `GET /api/v1/session/{session_id}/transcript`

Returns a bounded transcript snapshot.

Query parameters:

- `limit`
- `before_item_id`

### Orchestration Views

#### `GET /api/v1/session/{session_id}/orchestration/status`

Returns compact orchestration summary.

#### `GET /api/v1/session/{session_id}/orchestration/workers`

Optional query:

- `filter=all|blockers|agents|shells|services|capabilities|terminals|guidance|actions`
- `capability=@api.http`

#### `GET /api/v1/session/{session_id}/orchestration/dependencies`

Optional query:

- `filter=all|blocking|sidecars|missing|booting|ambiguous|satisfied`
- `capability=@api.http`

### Background Shells

#### `GET /api/v1/session/{session_id}/shells`

Lists wrapper-owned shell jobs.

#### `POST /api/v1/session/{session_id}/shells/start`

Starts a new shell job.

#### `POST /api/v1/session/{session_id}/shells/{job_ref}/poll`

Returns current shell snapshot.

#### `POST /api/v1/session/{session_id}/shells/{job_ref}/send`

Writes stdin to a running shell.

#### `POST /api/v1/session/{session_id}/shells/{job_ref}/terminate`

Stops a running shell.

### Services And Capabilities

#### `GET /api/v1/session/{session_id}/services`

Optional query:

- `status=all|ready|booting|untracked|conflicts`
- `capability=@api.http`

#### `GET /api/v1/session/{session_id}/capabilities`

Optional query:

- `status=all|healthy|missing|booting|ambiguous|untracked`
- `capability=@api.http`

#### `POST /api/v1/session/{session_id}/services/{job_ref}/provide`

Updates provided capabilities.

#### `POST /api/v1/session/{session_id}/services/{job_ref}/depend`

Updates dependency capabilities.

#### `POST /api/v1/session/{session_id}/services/{job_ref}/contract`

Updates live service contract metadata.

#### `POST /api/v1/session/{session_id}/services/{job_ref}/relabel`

Updates service label.

## Reference Rules

Wherever a route accepts `job_ref`, the first implementation should allow the
same reference contract as the CLI:

- `bg-*`
- alias
- unique `@capability`
- numeric index when meaningful in session scope

## Error Model

The local API should not expose raw terminal-oriented errors.

First-pass error shape:

```json
{
  "error": {
    "code": "capability_ambiguous",
    "message": "Capability @api.http is provided by multiple running service jobs.",
    "details": {
      "capability": "@api.http"
    }
  }
}
```

Preferred error-code families:

- `session_*`
- `thread_*`
- `turn_*`
- `job_*`
- `service_*`
- `capability_*`
- `validation_*`

## Explicit Initial-Slice Non-Goals

Even though much of the local API is now implemented, this sketch still does
not define the following broader areas because they remain intentionally out
of scope for the first local-API/connector phase:

- websocket transport
- broker auth
- cross-device deployment routing
- artifact upload/download protocols
- binary audio streaming
- general scene/entity APIs

## Initial-Slice Success Criteria

The local API is sufficient when it can support:

- a local WebUI attaching to a session
- a mobile/web adapter reading transcript and status streams
- inspection of orchestration blockers and services
- mutation of wrapper-owned shell/service state without scraping terminal text
