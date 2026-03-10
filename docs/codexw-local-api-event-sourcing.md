# codexw Local API Event Sourcing Plan

This document explains how the local API produces semantic events without
scraping terminal output.

The goal is to preserve `codexw`'s existing runtime truth:

- turn lifecycle
- transcript evolution
- orchestration state changes
- wrapper-owned shell/service mutations

and expose those through a stable event bus that can feed:

- loopback SSE clients
- the current broker connector prototype
- local WebUI/mobile clients

## Core Principle

The local API must emit events from runtime mutation points, not from rendered
terminal blocks.

That means:

- do not parse ANSI output
- do not reverse-engineer transcript meaning from scrollback text
- do not derive orchestration changes from prompt suffixes

Instead, emit events when the actual runtime state changes.

## Event Producers Already Present In `codexw`

The codebase already has natural mutation points that act as event producers.

### Session and Turn Lifecycle

Primary sources:

- `wrapper/src/events/notifications/turns.rs`
- `wrapper/src/response_thread_runtime.rs`
- `wrapper/src/response_thread_loaded.rs`
- `wrapper/src/requests/thread_switch_common/*`

These paths already know when:

- a session attaches
- a thread loads
- a turn starts
- a turn completes
- a turn is interrupted

### Transcript Item Evolution

Primary sources:

- `wrapper/src/notification_item_buffers.rs`
- `wrapper/src/notification_item_status/item.rs`
- `wrapper/src/notification_item_completion/item.rs`
- transcript summary/render helpers

These paths already know when a semantic item becomes visible as:

- assistant content
- command execution
- file changes
- tool call / tool result
- plan / reasoning summary

### Orchestration State

Primary sources:

- `wrapper/src/orchestration_registry/tracking.rs`
- `wrapper/src/orchestration_registry/graph/edges.rs`
- `wrapper/src/orchestration_registry/graph/scheduler.rs`
- `wrapper/src/background_shells/services/updates/*`
- `wrapper/src/background_terminals/tracking.rs`

These paths already know when:

- a wait edge appears or clears
- a sidecar agent becomes live
- a background shell starts/stops
- a reusable service changes readiness/capabilities/dependencies
- a server-observed terminal appears or clears

### Background Shell and Service State

Primary sources:

- `wrapper/src/background_shells/execution/manage/lifecycle/start.rs`
- `wrapper/src/background_shells/execution/manage/control.rs`
- `wrapper/src/background_shells/execution/interact/tools/jobs.rs`
- `wrapper/src/background_shells/execution/interact/tools/services.rs`
- `wrapper/src/background_shells/services/updates/*`

These paths already know when:

- a shell starts
- output changes materially
- stdin is written
- a shell terminates
- a service contract changes
- capabilities or dependencies change

## Recommended Event Bus Shape

Introduce one small runtime event bus attached near `AppState` or the app
runtime layer.

Suggested internal shape:

- append-only semantic event push
- best-effort fanout to zero or more SSE subscribers
- session-scoped filtering
- no persistence requirement for the first spike

The current implementation does not need durable replay. It already relies on a
clean semantic publication path.

## Suggested Internal Types

The concrete module graph has evolved, but the recommended structure is still:

- `LocalApiEvent`
  - already normalized, session-scoped semantic event
- `LocalApiEventBus`
  - subscriber registry
  - publish method
  - per-session subscriber fanout
- `LocalApiSubscriber`
  - sender half for SSE connections

The important rule is that producers publish semantic payloads, and the SSE
layer only serializes them.

## Event Emission Points

### Emit `session.*`

At:

- successful session create
- successful thread attach
- successful thread resume/fork/start completion

Emission owners:

- `local_api/routes/session.rs`
- `response_thread_runtime.rs`
- `response_thread_loaded.rs`

### Emit `turn.*`

At:

- turn start notification
- turn completed notification
- local interrupt success

Likely emission owners:

- `events/notifications/turns.rs`
- turn-interrupt response handlers

### Emit `transcript.item`

At:

- item becomes semantically complete enough to expose
- item deltas accumulate into a meaningful new snapshot if streaming items are
  intended

For the current implementation, a conservative model remains acceptable:

- emit transcript items on item completion
- optionally emit coarse updates for long-running items later

Likely emission owners:

- `notification_item_completion/item.rs`
- possibly `notification_item_status/item.rs` for coarse in-progress items

### Emit `status.updated`

At:

- changes to summary-visible working state
- account/config/token/rate status changes
- last status line changes materially

Likely emission owners:

- runtime status update helpers
- response/notification handlers that mutate status-relevant fields

### Emit `orchestration.updated`

At:

- registry graph changes
- wait dependencies change
- shell/service readiness/capabilities/dependencies change
- observed background terminals change

Likely emission owners:

- orchestration registry tracking/graph helpers
- service update helpers
- background-terminal tracking helpers

### Emit `shell.updated`, `service.updated`, `capability.updated`

At:

- shell lifecycle changes
- service readiness or contract changes
- capability-provider/consumer index changes

Likely emission owners:

- background-shell lifecycle helpers
- service update helpers
- capability-index update paths

## Publication Rules

### Coalescing

Do not emit on every internal byte delta.

Preferred strategy:

- emit on semantic state transitions
- allow later addition of coarse throttled progress events

Examples:

- good:
  - shell started
  - shell terminated
  - service became ready
  - dependency moved from missing to satisfied
- bad:
  - every terminal redraw
  - every spinner tick
  - every ANSI block render

### Session Scoping

Every published event must be associated with:

- one `session_id`
- and, when relevant, one `thread_id`

If a local runtime hosts multiple remote sessions later, the bus must be able
to fan out only the relevant session events.

### Serialization Boundary

Semantic event structs should be created before JSON serialization.

That keeps:

- SSE output
- connector forwarding
- route-level tests

all aligned on the same semantic payload.

## Current Implementation Recommendation

For the implemented local API plus connector stack:

1. add a simple in-memory event bus
2. publish only the required initial semantic families:
   - `session.*`
   - `turn.*`
   - `transcript.item`
   - `status.updated`
   - `orchestration.updated`
3. add `shell.updated`, `service.updated`, and `capability.updated` only where
   cheap and already well-structured

This keeps the event model small and semantic while still supporting the
current loopback SSE and connector surfaces.

## Test Strategy

The event bus is not done unless it is tested at the semantic level.

Minimum test set:

- session create emits `session.attached` or equivalent
- turn start emits `turn.started`
- completed assistant item emits `transcript.item`
- shell start/terminate emits `shell.updated`
- service readiness or capability mutation emits `service.updated` or
  `capability.updated`
- orchestration wait edge creation emits `orchestration.updated`

Avoid tests that assert rendered terminal strings as the primary evidence.

## Risks To Avoid

### Risk: Shadow State

If the local API event layer starts caching a second copy of transcript,
orchestration, or shell state just to emit events, the design is drifting.

The event bus should observe existing mutations, not recreate runtime truth.

### Risk: UI-Derived Semantics

If the easiest producer becomes “scrape the line we just rendered,” stop and
move the publication point earlier.

### Risk: Over-Streaming

If the first SSE implementation tries to mirror every internal delta, clients
will get noisy and unstable payloads.

Start with semantic transitions only.

## Exit Criteria

This plan is complete when:

1. the local API spike has named event producers for every required initial
   semantic family
2. event serialization is independent of terminal rendering
3. the SSE implementation can subscribe to one semantic bus instead of many ad
   hoc hooks
4. the broker connector can forward the same envelope without inventing
   its own event semantics
