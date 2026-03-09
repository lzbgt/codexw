# codexw Broker Connectivity Design

## Objective

Define how `codexw` could evolve from a local terminal wrapper into a remotely reachable Codex runtime that can be driven through a broker by multiple client types such as:

- a mobile app
- a browser UI
- a remote terminal
- other automation clients

This document is an investigation and design-planning artifact. It is not a commitment to immediate implementation.

## Why This Matters

`codexw` already has several properties that make remote access plausible:

- a serialized internal state model in `wrapper/src/state.rs`
- structured transcript and status rendering rather than raw log dumping
- explicit orchestration and worker views
- wrapper-owned async shell and service-control surfaces
- clear turn/session lifecycle boundaries

That means the remaining work is not inventing all remote concepts from scratch. The higher-value problem is deciding which existing local concepts should become stable remote APIs.

## Reference Baseline From `~/work/agent`

The sibling `~/work/agent` project already defines a concrete daemon/broker/client model that is relevant here:

- `/Users/zongbaolu/work/agent/DESIGN.md`
  - daemon-first multi-client architecture
  - broker relay flow
  - outbound agent connectivity
- `/Users/zongbaolu/work/agent/broker/README.md`
  - `wss://.../v1/agent/connect`
  - `/v1/agents/{agent_id}/proxy/...`
  - `/v1/agents/{agent_id}/proxy_sse/...`
  - OIDC/token auth and optional mTLS
- `/Users/zongbaolu/work/agent/docs/PROTOCOL.md`
  - run/event envelopes
  - session-safe event flows
  - artifact signaling
- `/Users/zongbaolu/work/agent/docs/CLIENT.md`
  - client identity
  - client events
  - bidirectional UI/agent collaboration model

Those documents make brokered connectivity for `codexw` a fact-based design investigation, not a generic wishlist item.

## Non-Goals For The First Design Slice

- replacing `codex app-server`
- forcing full protocol compatibility with `~/work/agent` before a gap analysis
- redesigning the local inline terminal UX first
- exposing every internal render detail as a public API

## Constraints

Any broker-connected `codexw` design has to respect these realities:

1. `codexw` is currently an app-server client, not a standalone agent daemon.
2. Some async execution behavior is wrapper-owned because app-server does not expose public control of model-owned `item/commandExecution` sessions.
3. `codexw` today is terminal-first and scrollback-first, not a browser-first service runtime.
4. Remote APIs must map to stable state transitions, not terminal presentation artifacts.

## Architectural Options

### Option A: Local API + Separate Connector

Shape:

- `codexw` exposes a local HTTP/SSE API
- a connector process speaks the cloud broker protocol
- remote clients go through the connector

Pros:

- keeps broker logic out of the main wrapper process
- easiest path to partial compatibility with `~/work/agent`
- lower risk to the local interactive terminal workflow

Cons:

- two-process deployment model
- more moving parts for local development
- another compatibility boundary to maintain

### Option B: Direct Broker Connectivity In `codexw`

Shape:

- `codexw` itself maintains an outbound broker connection
- clients proxy into the running wrapper directly

Pros:

- simplest runtime graph once running
- fewer components to deploy
- closest to "one runtime, many clients"

Cons:

- mixes local TTY concerns with remote control-plane concerns
- harder security boundary
- more invasive lifecycle and reconnect logic inside the wrapper

### Option C: Compatibility Layer Only

Shape:

- `codexw` keeps working locally
- it exposes a protocol shaped like the `agent` broker/client surfaces
- another host runtime can translate or embed it

Pros:

- lowest implementation risk
- allows iterative compatibility experiments

Cons:

- does not actually make `codexw` remotely reachable by itself
- lower product value than Options A or B

## Recommended Direction

The current highest-leverage path is:

1. define a local `codexw` HTTP/SSE API first
2. model it against the `~/work/agent` broker/client contract
3. keep direct broker connectivity as a second-phase decision

That keeps the first implementation aligned with the existing wrapper architecture and still preserves a path to broker compatibility.

## Compatibility Matrix

### Areas That Already Map Reasonably Well

- session lifecycle
  - `codexw` has thread start/resume/fork and explicit status surfaces
- event streaming
  - `codexw` already has structured item, turn, status, and orchestration events internally
- orchestration inspection
  - `:status`, `:ps`, dependency views, capability views, and worker summaries are already modeled
- background execution
  - wrapper-owned shell/service control already exists

### Areas That Need Explicit Design

- remote session identity
  - mapping local thread ids, wrapper session state, and remote client sessions
- auth model
  - local-only token, browser-safe auth, broker token auth, or mTLS
- event/public API stability
  - which internal state transitions are safe to make public
- approval semantics
  - remote clients may need visibility into approval posture even if local defaults stay automated
- multi-client concurrency
  - steering, interrupting, and session mutation need ordering rules

### Areas That Are Still Architectural Gaps

- alternate-screen/native-TUI parity
- upstream audio/realtime UX parity
- app-server-owned command session reuse

Those are important, but they are not blockers for a first remote-control API if the scope is state/control/event transport rather than full UI parity.

## API Surface To Design

The first remote API proposal should cover:

### Session Endpoints

- create wrapper session
- list resumable threads
- attach to thread
- expose cwd/objective/current model/personality/collab state

### Turn Endpoints

- submit turn
- steer turn
- interrupt turn
- fetch last assistant reply / last diff / last status

### Stream Endpoints

- transcript/event stream
- orchestration stream
- status stream

### Orchestration Endpoints

- `orchestration_status`
- worker views
- dependency views
- guidance/actions views

### Background Shell Endpoints

- start/poll/send/terminate
- attach/wait/run recipe
- provide/depend/relabel/contract
- list services and capabilities

## Session Model Questions

These questions must be decided before implementation:

1. Is a remote session just a view over one local thread, or a separate wrapper-side collaboration context?
2. Can multiple clients attach to the same active thread concurrently?
3. If yes, which actor is authoritative for interrupt, steer, or shell mutation actions?
4. How are local resume commands and remote session identifiers related?

## Security Questions

These need explicit design before any implementation work:

- whether the first remote API binds only to loopback
- whether browser access is supported in phase 1
- whether auth is bearer-token only or broker-shaped from the start
- whether outbound broker identity should reuse the `agent` project’s mTLS/client-auth assumptions

## Event Model Questions

The remote event stream should prefer stable semantic events such as:

- session attached
- turn started
- turn completed
- transcript item appended
- status updated
- orchestration changed
- background shell changed
- service capability changed

It should not expose terminal-only concerns such as wrapped prompt layout or ANSI block formatting as public protocol.

## Phased Plan

### Phase 0: Compatibility Audit

- compare `codexw` state/events against `~/work/agent` protocol and client expectations
- list exact mismatches instead of assuming compatibility

### Phase 1: Local API Design

- define a local HTTP/SSE API for `codexw`
- decide auth and session identity
- specify event envelopes

### Phase 2: Prototype Connector

- adapt the local API to the broker relay model
- test remote terminal and browser-driven operation

### Phase 3: Decide Native Broker Support

- after real connector experience, choose whether direct broker connectivity belongs in `codexw`

## TODOs

- Audit `codexw` state and event surfaces against:
  - `/Users/zongbaolu/work/agent/DESIGN.md`
  - `/Users/zongbaolu/work/agent/broker/README.md`
  - `/Users/zongbaolu/work/agent/docs/PROTOCOL.md`
  - `/Users/zongbaolu/work/agent/docs/CLIENT.md`
- Write a first `codexw` local API sketch covering:
  - session lifecycle
  - turn lifecycle
  - transcript/event streaming
  - orchestration views
  - background shell/service control
- Decide whether the first implementation target is:
  - loopback-only local API
  - local API plus connector
  - direct broker connectivity
- Define the minimal compatibility target:
  - payload vocabulary reuse
  - partial protocol compatibility
  - or full broker/client compatibility
- Evaluate whether the user’s C agent framework can share:
  - session identity rules
  - event envelopes
  - broker transport assumptions
  - auth and deployment routing concepts

## Current Status

This is now an explicit tracked design area, not an informal idea.

The next high-leverage step is not implementation. It is a compatibility and API audit that turns the broad broker idea into a bounded remote-control contract for `codexw`.
