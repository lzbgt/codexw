# codexw Broker Service + iOS App Implementation Plan

## Purpose

This document turns the Go broker service and iOS app designs into one delivery
sequence that can be verified incrementally on the current Apple Silicon
development machine.

Related docs:

- [codexw-go-broker-service.md](codexw-go-broker-service.md)
- [codexw-ios-app.md](codexw-ios-app.md)
- [codexw-broker-connectivity.md](codexw-broker-connectivity.md)
- [codexw-broker-client-architecture.md](codexw-broker-client-architecture.md)

## Delivery Principle

Do not start with cloud-only deployment concerns.

Start with one local end-to-end path on this M2 Mac:

1. `codexw` with local API enabled
2. connector or broker agent adapter
3. Go broker running locally
4. iOS simulator connected to the broker

If that local loop does not work, cloud deployment will only make debugging
slower.

## Phase 1: Runtime Discovery

Goal:

- make one `codexw` instance discoverable as a broker-routable deployment

Required `codexw` work:

- `GET /api/v1/runtime`
- machine-readable runtime instance metadata
- suggested deployment id
- host OS and arch
- Apple Silicon fact
- preferred broker transport

Proof:

- local-api tests for runtime route contract and auth
- connector can fetch and relay runtime metadata

## Phase 2: Go Broker Registry

Goal:

- accept deployment registration and maintain liveness

Required broker work:

- WebSocket deployment connection
- heartbeat timeout policy
- deployment list/read API
- in-memory registry first
- optional Postgres later

Proof:

- one local `codexw` appears in broker deployment list
- disconnect is detected and reflected

## Phase 3: Routed Session + Turn Control

Goal:

- mobile/web clients can do useful work through the broker

Required broker work:

- deployment routing
- session create/attach/read
- turn start/interrupt
- transcript read

Required app work:

- deployment list screen
- session detail
- transcript screen
- prompt submit and interrupt

Proof:

- iOS simulator starts and interrupts a turn through broker routing
- transcript updates without direct terminal access

## Phase 4: Streaming + Supervision

Goal:

- remote clients can distinguish healthy quiet work from real trouble

Required broker work:

- event stream fanout
- replay cursor support
- supervision event relay

Required app work:

- live event stream
- quiet-turn and self-heal status presentation

Proof:

- a quiet turn, an interrupt-pending turn, and a self-heal event are rendered
  distinctly in the app

## Phase 5: Shell + Service Surfaces

Goal:

- remote clients can inspect and operate host-side work, not just prompts

Required broker work:

- shell list/detail/start/send/terminate
- service list/detail/attach/wait/run
- capability inspection

Required app work:

- shell list/detail screens
- service list/detail screens
- safe write controls for targeted shell actions

Proof:

- iOS simulator can inspect a running shell/service and invoke a service recipe

## Phase 6: Notifications

Goal:

- mobile usage becomes asynchronous and operationally useful

Required broker work:

- notification routing
- APNs integration
- per-user subscription preferences

Required app work:

- push registration
- deep links into deployment/session/turn

Proof:

- completed-turn and intervention-needed pushes open the right screen

## Phase 7: Hardening

Goal:

- make the system safe enough for routine use

Required work:

- deployment auth
- client auth and RBAC
- audit trail
- broker persistence
- reconnect semantics
- conflict handling for multiple clients

Proof:

- mutating actions are attributable by user/client/deployment/session
- deployment reconnect restores listability and streaming continuity

## What Not To Do Early

Do not front-load:

- full PTY terminal emulation on iPhone
- artifact catalog work
- cross-deployment dependency UI
- broker-managed source replication
- raw app-server protocol tunneling

Those are later-value items, not first blockers.

## Repo-Local Next Steps

The next high-leverage changes inside this repo are:

1. finish documenting the runtime discovery contract
2. expose runtime discovery through the connector prototype
3. add a small broker registration fixture against that route
4. keep all proof local-machine reproducible on the M2 Mac
