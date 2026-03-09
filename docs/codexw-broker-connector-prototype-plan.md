# codexw Broker Connector Prototype Plan

This document turns the connector mapping design into a concrete prototype plan.

The goal is not full production broker support. The goal is to validate that a
thin connector can expose `codexw` remotely without distorting the local API.

## Objective

Prototype a connector that:

- talks to a local `codexw` loopback API
- exposes selected routes/events through a broker-facing surface
- proves that `codexw` can be remotely driven by browser/mobile/terminal clients
  without making the daemon core broker-native

## Prerequisite

This prototype depends on the local API spike being complete enough to provide:

- session create/attach
- turn start/interrupt
- transcript/event SSE
- orchestration status/workers/dependencies
- shell/service/capability inspection

## Prototype Scope

### In Scope

- one connector process
- connection to one local `codexw` instance
- one broker-facing registration identity
- HTTP proxying for selected routes
- SSE bridging for selected event streams
- deployment metadata injection

### Out Of Scope

- full `agentd` wire compatibility
- scene/entity compatibility
- audio/video parity
- cross-deployment fanout
- multi-connector coordination

## Minimal Route Set

The first useful connector should support:

- session create
- session attach
- attachment renew
- attachment release
- session inspect
- turn start
- turn interrupt
- transcript snapshot
- event SSE
- orchestration status
- orchestration workers
- orchestration dependencies

Nice-to-have in the same prototype if cheap:

- shell list/poll/send/terminate
- services list
- capabilities list

## Minimal Event Set

The connector should be able to forward:

- `session.*`
- `turn.*`
- `transcript.item`
- `status.updated`
- `orchestration.updated`

Nice-to-have:

- `shell.updated`
- `service.updated`
- `capability.updated`

## Candidate Runtime Shape

Recommended prototype:

- standalone connector process
- configured with:
  - local `codexw` base URL
  - broker base URL or connection target
  - deployment identity
  - local auth token if enabled
  - broker auth material

This matches the direction already used by `~/work/agent` and avoids polluting
the first `codexw` local API implementation with broker reconnect logic.

## Adapter Responsibilities

The connector should be responsible for:

1. broker auth
2. deployment registration
3. mapping broker-facing route shapes to local API shapes
4. SSE passthrough or translation
5. enriching events with deployment metadata
6. clear handling of unsupported surfaces

The connector should not be responsible for:

- owning session state
- owning transcript history
- owning orchestration logic
- synthesizing new service/capability semantics

## Success Criteria

The prototype is successful if:

1. a remote browser or terminal can create and attach to a `codexw` session
2. a remote client can start a turn and observe resulting events
3. the connector can relay semantic SSE without dropping identity fields
4. orchestration state can be inspected remotely
5. no connector logic needs to parse terminal-rendered output

## Failure Signals

The connector plan should be reconsidered if any of these happen:

- the local API is too `codexw`-specific to map cleanly even with explicit
  adapter behavior
- the connector must maintain shadow state to function
- auth or deployment routing forces large changes into the daemon core
- event translation becomes lossy enough that remote clients need daemon-native
  broker semantics instead

## Prototype Test Matrix

Minimum scenarios:

1. local session create through connector
2. attach to existing thread through connector
3. start turn through connector
4. interrupt turn through connector
5. consume transcript/status/orchestration SSE through connector
6. inspect orchestration workers/dependencies through connector

Recommended follow-up scenarios:

7. inspect shell jobs and services remotely
8. validate unique `@capability` handling through encoded route parameters
9. verify reconnect behavior for SSE clients

Current automated coverage now includes a real process-level smoke path:

- the tracked connector binary is started as a subprocess
- a fake local-API server is bound behind it
- a remote client drives the connector over TCP/HTTP
- the smoke test currently verifies:
  - broker-style `sessions` alias mapping for session create
  - broker-style `sessions/{session_id}/attach` alias mapping
  - broker-style `sessions/{session_id}/attachment/{renew|release}` alias mapping
  - a realistic broker-style workflow:
    - create session
    - start turn
    - inspect transcript
    - inspect orchestration status/dependencies
  - a realistic broker-style shell/service workflow:
    - create session
    - start shell
    - inspect services
    - attach to service
    - wait for service readiness
    - run service recipe
    - inspect capabilities
  - a realistic broker-style service-control plus SSE-resume workflow:
    - create session
    - consume initial service event stream
    - attach / wait / run through service aliases
    - reconnect the event stream with `Last-Event-ID`
    - observe capability state after the service interaction path
  - a realistic broker-style service-mutation workflow:
    - create session
    - provide capabilities
    - retarget dependencies
    - update contract metadata
    - relabel service
    - inspect services and capabilities
  - a realistic broker-style focused-detail plus resume workflow:
    - create leased session
    - consume initial event stream
    - mutate service capabilities
    - inspect focused service detail
    - inspect focused capability detail
    - reconnect the event stream with `Last-Event-ID`
    - observe the resumed capability-state event after mutation
  - a realistic broker-style conflict workflow:
    - create session with an active lease
    - attempt turn/service mutation from another client identity
    - observe forwarded `attachment_conflict` details with lease-holder context
  - alias-based SSE forwarding with `Last-Event-ID` passthrough on reconnect
  - broker-style `sessions/{session_id}/shells` alias mapping for shell start
  - broker-style `sessions/{session_id}/services/{job_ref}/run` alias mapping
  - `session_id` body projection for attach aliases
  - client/lease header projection into local-API JSON bodies
  - alias-based SSE event forwarding and broker metadata wrapping
  - coherent service interaction behavior under SSE reconnect conditions instead
    of only isolated alias checks
  - local-API structured error envelopes surviving the connector path for
    attachment-lease conflicts on broker-style alias routes

## Deliverables

The prototype should produce:

- a small route mapping table with actual working paths
- one event passthrough example log
- a list of unsupported broker surfaces
- a list of adapter-only `codexw` extensions

## Current Prototype Status

An initial standalone prototype now exists as:

- `cargo run --bin codexw-connector-prototype -- --agent-id <id> --deployment-id <id> --local-api-base <url>`

Current implemented behavior:

- listens on its own loopback HTTP bind address
- requires an exact `{agent_id}` segment in:
  - `/v1/agents/{agent_id}/proxy/...`
  - `/v1/agents/{agent_id}/proxy_sse/...`
- also exposes first-pass broker-style aliases for the common remote-client paths:
  - `/v1/agents/{agent_id}/sessions`
  - `/v1/agents/{agent_id}/sessions/{session_id}`
  - `/v1/agents/{agent_id}/sessions/{session_id}/attach`
  - `/v1/agents/{agent_id}/sessions/{session_id}/attachment/{renew|release}`
  - `/v1/agents/{agent_id}/sessions/{session_id}/turns`
  - `/v1/agents/{agent_id}/sessions/{session_id}/interrupt`
  - `/v1/agents/{agent_id}/sessions/{session_id}/transcript`
  - `/v1/agents/{agent_id}/sessions/{session_id}/events`
  - `/v1/agents/{agent_id}/sessions/{session_id}/orchestration/{status|workers|dependencies}`
  - `/v1/agents/{agent_id}/sessions/{session_id}/shells`
  - `/v1/agents/{agent_id}/sessions/{session_id}/shells/{job_ref}/{poll|send|terminate}`
  - `/v1/agents/{agent_id}/sessions/{session_id}/services`
  - `/v1/agents/{agent_id}/sessions/{session_id}/services/{job_ref}`
  - `/v1/agents/{agent_id}/sessions/{session_id}/capabilities`
  - `/v1/agents/{agent_id}/sessions/{session_id}/capabilities/{capability}`
  - `/v1/agents/{agent_id}/sessions/{session_id}/services/{job_ref}/{provide|depend|contract|relabel|attach|wait|run}`
- forwards only an allowlisted subset of HTTP requests under `proxy/...` to the
  local API path of the same suffix
- forwards SSE requests under `proxy_sse/...` to the local API event stream
- resolves the broker-style aliases onto the same local-API allowlist instead of
  creating a second policy path
- wraps SSE `data:` payloads with:
  - `source: "codexw"`
  - `broker.agent_id`
  - `broker.deployment_id`
- supports:
  - optional incoming connector bearer auth
  - optional outgoing local-API bearer auth
  - connector-side client/lease header injection via:
    - `X-Codexw-Client-Id`
    - `X-Codexw-Lease-Seconds`
    for supported mutating JSON routes

Current explicit limitations:

- no broker registration handshake yet
- no broker-side policy beyond the current static allowlist and agent-id path ownership
- no shadow session/deployment store
- no remote lease policy beyond header-to-local-API projection and what the local API already enforces
- no persistence or reconnect bookkeeping outside SSE `Last-Event-ID` passthrough

## Decision After Prototype

After the prototype, the project should decide one of:

1. connector path is viable, continue
2. connector path is too lossy, revise the local API
3. connector path is too awkward, reconsider direct broker integration

That decision should be based on the prototype results, not on assumptions made
before the local API exists.
