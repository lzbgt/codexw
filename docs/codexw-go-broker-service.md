# codexw Go Broker Service Design

## Purpose

This document narrows the broader broker/connectivity design into one concrete
product target:

- a cloud broker service implemented in Go
- routing multiple `codexw` deployments
- serving app, WebUI, terminal, and automation clients
- preserving `codexw` as the runtime authority instead of proxying raw
  `codex app-server` internals

It is intentionally more product-specific than
[codexw-broker-connectivity.md](codexw-broker-connectivity.md) and more
deployment-specific than
[codexw-broker-client-architecture.md](codexw-broker-client-architecture.md).

## Core Rule

The broker should speak to `codexw`, not to `codex app-server`.

`codexw` remains responsible for:

- thread and turn lifecycle
- auto-continue and self-heal behavior
- local shell/service runtime truth
- transcript, supervision, and orchestration state
- local API and any direct connector-facing contracts

The broker is responsible for:

- deployment registration and liveness
- client authentication and routing
- multi-client fanout
- cloud persistence and replay
- notifications and audit trails

This avoids coupling the cloud service to private stdio transport details
between `codexw` and the upstream app-server.

## Why Go

Go is a good fit for the broker because it needs:

- many concurrent long-lived WebSocket connections
- simple deployment as one static service binary
- strong HTTP/WebSocket server tooling
- straightforward integration with Postgres, Redis, APNs, and OIDC
- low operational complexity for a control-plane process

## Runtime Graph

Recommended first graph:

1. `codexw` exposes its local HTTP/SSE API on loopback
2. a connector or embedded broker agent adapter fronts that local API
3. the Go broker accepts outbound deployment connections
4. remote clients attach to the broker
5. the broker relays high-level commands and event streams to deployments

This remains connector-first rather than direct embedded broker transport in
the first slice because the current repo already has a verified local API plus
connector path.

## Broker Responsibilities

### 1. Deployment Registry

Track each connected `codexw` deployment with:

- broker connection id
- deployment id
- runtime instance id
- host metadata
- connection age
- last heartbeat time
- claimed capabilities

The local API should expose enough discovery metadata that the connector can
register the deployment without scraping terminal text. The new
`GET /api/v1/runtime` route is the first native slice for that requirement.

### 2. Command Relay

Support high-level routed operations such as:

- list deployments
- inspect one deployment
- create or attach a session
- start or interrupt a turn
- consume transcript or event streams
- inspect orchestration state
- start, poll, write, and terminate shells
- inspect and mutate services/capabilities

The broker should relay stable local-API semantics, not invent a second runtime
state machine.

### 3. Event Fanout

The broker should preserve semantic SSE-style events from `codexw` and make
them available to multiple clients with:

- replay by event id
- last-seen cursor resume
- deployment/session/thread correlation
- optional push-notification summaries

### 4. Authentication And Policy

The first viable security model should separate:

- deployment authentication
- end-user authentication
- client session attachment and audit attribution

Recommended first cut:

- deployment-to-broker auth: signed instance secret or mTLS later
- client-to-broker auth: OIDC/JWT
- audit on every mutating request: user id, client id, deployment id, session id

### 5. Persistence

Persist only control-plane state and metadata, not a second full runtime truth.

Store:

- deployment registry state
- client sessions and leases
- event replay cursor metadata
- notification state
- audit trail

Do not make the broker the source of truth for:

- local shell buffers
- live Codex thread state
- deployment-local transcript authority

## Protocol Shape

The first broker protocol can stay simple and JSON-based.

Deployment connection message families:

- `deployment.hello`
- `deployment.heartbeat`
- `deployment.snapshot`
- `deployment.event`
- `deployment.command_result`

Broker-to-deployment commands:

- `session.new`
- `session.attach`
- `turn.start`
- `turn.interrupt`
- `shell.start`
- `shell.send`
- `shell.terminate`

Client-facing broker routes can remain HTTP/WebSocket while the internal
deployment connection uses WebSocket frames.

## Heartbeats

There are two different heartbeat problems and they should not be conflated:

1. deployment heartbeat
2. upstream model/worker progress heartbeat

The Go broker can prove that the `codexw` process is alive and connected. It
cannot prove that the cloud LLM worker is making progress unless `codexw`
receives that signal from upstream.

So the first protocol should carry:

- transport liveness heartbeats from deployment to broker
- runtime status heartbeats from deployment to broker:
  - turn running
  - last app-server event age
  - interrupt pending
  - self-heal active
  - active shell/service counts

## Minimal API Needed From `codexw`

The current local API already covers session, turn, transcript, event, shell,
service, and capability flows.

The first additional broker-oriented native requirement is deployment discovery.
That is why `GET /api/v1/runtime` matters. It should expose machine-readable:

- process-scoped instance id
- suggested deployment id
- host OS and arch
- Apple Silicon fact when relevant
- preferred broker transport
- current local-api session id and cwd context

That is enough for the Go broker to list and route deployments without
inventing extra terminal scraping logic.

## Local Verification On This M2 Mac

This host can verify the first broker slice locally:

- Go broker runs natively on Apple Silicon
- `codexw` runs locally with loopback local API
- connector runs locally and bridges to the broker
- iOS app can target the same broker through the iOS simulator

That means the initial broker slice should be designed for single-machine
verification before any cloud-only requirement is introduced.

## Non-Goals

The first Go broker slice should not try to solve:

- global artifact indexing
- deployment-to-deployment filesystem replication
- direct `codexw` to `codexw` peer transport
- a new generic tool runtime independent of `codexw`
- app-server protocol compatibility as a public contract

## Recommended Delivery Order

1. Runtime discovery route in local API
2. Connector projection of runtime discovery and deployment metadata
3. Go broker registry + deployment heartbeat
4. Routed session/turn control through the broker
5. Routed shell/service control through the broker
6. Multi-client event replay and notifications
