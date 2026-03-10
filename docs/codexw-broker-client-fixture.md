# codexw Broker Client Fixture

This document describes the small broker-style client fixture shipped with this
repo:

- [scripts/codexw_broker_client.py](../scripts/codexw_broker_client.py)

The goal is practical consumption of the connector prototype, not a production
SDK. It is a standard-library-only helper that lets developers drive the
broker-style alias surface without writing one-off `curl` sequences.

## Purpose

The fixture answers one concrete question:

Can a real remote client consume the current connector surface coherently enough
to create a session, drive turns, observe events, inspect orchestration, and
control shells/services?

This script is the reference fixture for that question.

## Scope

The fixture intentionally targets the current connector alias surface rather
than inventing a new abstraction layer.

Supported operations include:

- session create / attach / list / inspect
- attachment renew / release
- client event publish
- turn start / interrupt
- transcript fetch
- event stream consumption with optional `Last-Event-ID`
- orchestration status / workers / dependencies
- shell list / start / detail / poll / send / terminate
- service list / detail / attach / wait / run
- service mutation:
  - `provide`
  - `depend`
  - `contract`
  - `relabel`
- capability list / detail

## Why It Exists

Before this fixture, the repo already had:

- connector implementation
- connector unit tests
- connector subprocess smoke tests

What it did not have was a reusable client-side artifact outside the test
suite. That gap is now closed: the connector is both internally verified and
manually consumable through this standalone fixture.

This script closes that gap.

The repo now also includes process-level smoke coverage that invokes this
fixture against the real connector binary for:

- session create / turn / transcript
- session list
- client event publish plus event-stream replay/resume
- client-event ownership conflict plus explicit lease handoff and resumed
  observation
- turn interrupt
- session attach plus orchestration status / workers / dependencies inspection
- attachment renew / release plus session snapshot verification
- shell list / detail / send / poll / terminate
- shell start plus service attach / wait / run
- service capability/contract/label mutation (`provide` / `depend` / `contract` / `relabel`)
- lease-conflict propagation through broker-style alias routes
- focused service-detail and capability-detail inspection
- event stream consumption and `Last-Event-ID` resume
- one combined leased workflow that mixes:
  - initial event consumption
  - lease-owned service mutation
  - focused service-detail verification
  - resumed event consumption with `Last-Event-ID`
  - through the real standalone Python fixture, not only Rust smoke helpers
- one adversarial multi-client workflow that mixes:
  - owner-created leased session
  - observer event consumption
  - conflicting rival mutation with structured lease conflict details
  - owner mutation recovery
  - observer `Last-Event-ID` resume
- one observer-readable contention workflow that mixes:
  - owner-created leased session
  - observer session/orchestration/shell/service/capability reads
  - conflicting rival mutation with structured lease conflict details
  - observer reads remaining available after the conflict
- one anonymous observer/rival workflow that mixes:
  - owner-created leased session
  - anonymous event/session/orchestration/service/capability reads
  - conflicting anonymous mutation with `requested_client_id = null`
  - anonymous reads remaining available after the conflict
- one lease-handoff workflow that mixes:
  - owner-created leased session
  - two independent observers consuming the same initial event state
  - conflicting rival mutation before release
  - owner attachment release
  - rival lease acquisition and successful mutation
  - dual-observer `Last-Event-ID` resume after the handoff
- one repeated role-reversal workflow that mixes:
  - owner-created leased session
  - observer event consumption
  - rival conflict before release
  - owner release
  - rival takeover and successful mutation
  - former-owner conflict while rival holds the lease
  - rival release
  - owner retake and successful mutation
  - observer `Last-Event-ID` resume after the second role change
- one client-event lease-handoff workflow that mixes:
  - owner-created leased session
  - observer initial event consumption
  - rival `client-event` conflict before release
  - explicit owner release
  - rival lease acquisition
  - successful rival `client-event` publish
  - observer `Last-Event-ID` resume after the handoff

So the fixture is not just a documentation example.

## Invocation Shape

The fixture uses:

- `--base-url` for the connector base URL
- `--agent-id` for the broker-visible agent id
- optional `--client-id`
- optional `--lease-seconds`

Those common flags apply to all subcommands. The client projects `client_id`
and `lease_seconds` through the connector’s header-based client policy layer,
which is the same path expected by remote connector clients.

## Example Workflows

Create a session:

```bash
python3 scripts/codexw_broker_client.py \
  --base-url http://127.0.0.1:4317 \
  --agent-id codexw-lab \
  --client-id remote-web \
  --lease-seconds 45 \
  session-create \
  --thread-id thread_1
```

Start a turn:

```bash
python3 scripts/codexw_broker_client.py \
  --base-url http://127.0.0.1:4317 \
  --agent-id codexw-lab \
  --client-id remote-web \
  turn-start \
  --session-id sess_1 \
  --prompt "Summarize the repository status"
```

Renew an attachment lease:

```bash
python3 scripts/codexw_broker_client.py \
  --base-url http://127.0.0.1:4317 \
  --agent-id codexw-lab \
  --client-id remote-web \
  attachment-renew \
  --session-id sess_1 \
  --lease-seconds 90
```

Release an attachment:

```bash
python3 scripts/codexw_broker_client.py \
  --base-url http://127.0.0.1:4317 \
  --agent-id codexw-lab \
  --client-id remote-web \
  attachment-release \
  --session-id sess_1
```

Read a few events:

```bash
python3 scripts/codexw_broker_client.py \
  --base-url http://127.0.0.1:4317 \
  --agent-id codexw-lab \
  events \
  --session-id sess_1 \
  --limit 5
```

Publish a client event:

```bash
python3 scripts/codexw_broker_client.py \
  --base-url http://127.0.0.1:4317 \
  --agent-id codexw-lab \
  --client-id remote-web \
  --lease-seconds 45 \
  client-event \
  --session-id sess_1 \
  --event selection.changed \
  --data-json '{"selection":"services"}'
```

Resume from a known event id:

```bash
python3 scripts/codexw_broker_client.py \
  --base-url http://127.0.0.1:4317 \
  --agent-id codexw-lab \
  events \
  --session-id sess_1 \
  --last-event-id 42 \
  --limit 5
```

Run a service recipe:

```bash
python3 scripts/codexw_broker_client.py \
  --base-url http://127.0.0.1:4317 \
  --agent-id codexw-lab \
  --client-id remote-web \
  service-run \
  --session-id sess_1 \
  --job-ref dev.api \
  --recipe health
```

Inspect one capability:

```bash
python3 scripts/codexw_broker_client.py \
  --base-url http://127.0.0.1:4317 \
  --agent-id codexw-lab \
  capability-detail \
  --session-id sess_1 \
  --capability @frontend.dev
```

## Output Contract

Non-streaming operations print a simple JSON envelope:

```json
{
  "status": 200,
  "body": {
    "ok": true
  }
}
```

Streaming `events` output prints a JSON array of decoded SSE items:

```json
[
  {
    "id": "31",
    "event": "capabilities.updated",
    "data": {
      "session_id": "sess_1"
    }
  }
]
```

This is intentionally easy to pipe into `jq`, shell scripts, browser harnesses,
or other prototype clients.

## Non-Goals

This fixture is not:

- a stable public SDK
- a replacement for the connector smoke tests
- a generic `agentd` client
- a websocket client

It is a concrete companion to the connector prototype, not a compatibility
promise.

## Relationship To Other Broker Docs

- High-level architecture: [codexw-broker-connectivity.md](codexw-broker-connectivity.md)
- Connector mapping: [codexw-broker-connector-mapping.md](codexw-broker-connector-mapping.md)
- Connector prototype scope: [codexw-broker-connector-prototype-plan.md](codexw-broker-connector-prototype-plan.md)
- Local API route ownership: [codexw-local-api-route-matrix.md](codexw-local-api-route-matrix.md)

## Future Direction

The fixture suggests the next likely evolution:

- either promote this client shape into a tiny supported example tool
- or replace it later with a thin real client library once the connector/API
  surface stabilizes enough

Until then, this script is the best fact-based demonstration that the connector
is usable by something that is not an internal test harness.
