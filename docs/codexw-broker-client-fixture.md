# codexw Broker Client Fixture

This document describes the small broker-style client fixtures shipped with
this repo:

- [scripts/codexw_broker_client.py](../scripts/codexw_broker_client.py)
- [scripts/codexw_broker_client_node.mjs](../scripts/codexw_broker_client_node.mjs)

The goal is practical consumption of the current connector adapter surface, not a production
SDK. It is a standard-library-only helper that lets developers drive the
broker-style alias surface without writing one-off `curl` sequences.

## Purpose

The fixture answers one concrete question:

Can a real remote client consume the current connector surface coherently enough
to create a session, drive turns, observe events, inspect orchestration, and
control shells/services?

In architecture terms, these fixtures also answer a narrower host-examination
question: can a broker-facing client inspect the host through session,
transcript, event, shell, and service surfaces without direct terminal access?

These fixtures are the reference clients for that question.
For the short external-consumer summary aimed at the sibling `~/work/agent`
workspace, see
[codexw-broker-integration-handoff.md](codexw-broker-integration-handoff.md).

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
- semantic `status.updated` supervision consumption through the same event
  stream, including owner-lane and correlated shell-job facts
- orchestration status / workers / dependencies
- shell list / start / detail / poll / send / terminate
- service list / detail / attach / wait / run
- service mutation:
  - `provide`
  - `depend`
  - `contract`
  - `relabel`
- capability list / detail

For the workflow-level read on that host-examination question, including the
remaining artifact-contract gap, see
[codexw-broker-host-examination-matrix.md](codexw-broker-host-examination-matrix.md).

What the fixture does **not** currently prove is a dedicated artifact index/detail/content route family,
because that route family does not exist yet.
The current fixture surface proves shell-first host examination through
session, transcript, event, shell, and service routes, while the richer
artifact browser track remains separately documented in:

- [codexw-broker-artifact-contract-sketch.md](codexw-broker-artifact-contract-sketch.md)
- [codexw-broker-artifact-implementation-plan.md](codexw-broker-artifact-implementation-plan.md)

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
- broker-visible async supervision replay with:
  - `tool_slow` / `tool_wedged`
  - owner-lane facts such as `wrapper_background_shell`
  - `source_call_id`
  - correlated `observed_background_shell_job`
  - `async_tool_backpressure`
  - `async_tool_workers`
- one combined leased workflow that mixes:
  - initial event consumption
  - lease-owned service mutation
  - focused service-detail verification
  - resumed event consumption with `Last-Event-ID`
  - through the real standalone broker client fixtures, not only Rust smoke helpers
- one Node-driven event workflow that proves:
  - session create
  - lease-owned `client-event` publish
  - event stream consumption
  - `Last-Event-ID` resume
- one Node-driven attachment lifecycle workflow that proves:
  - session create
  - attachment renew
  - session snapshot verification of the renewed lease
  - attachment release
  - session snapshot verification of the released lease
- one Node-driven lease-conflict workflow that proves:
  - leased session ownership by one client
  - conflicting rival service mutation
  - structured `attachment_conflict` propagation with lease-holder details
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
It is also intentionally not a hidden artifact API placeholder: if a later
batch adds artifact routes, the fixture doc and the process-level fixture proof
should be updated explicitly in the same batch.

## Invocation Shape

Each fixture uses:

- `--base-url` for the connector base URL
- `--agent-id` for the broker-visible agent id
- optional `--client-id`
- optional `--lease-seconds`

Those common flags apply to all subcommands. Both clients project `client_id`
and `lease_seconds` through the connector’s header-based client policy layer,
which is the same path expected by remote connector clients.

That means the fixture is already representative of broker-backed app/WebUI or
automation clients for the currently supported shell-first host examination
surface, even though it is not yet a dedicated artifact-browser client.

For active wrapper-owned background-shell work, the fixture contract should be
read as a semantic supervision client, not a prompt scraper. The expected
broker-visible shape is:

- `status.updated` carries async-tool supervision state
- the sticky `supervision_notice` object carries the same current request/thread,
  owner-lane, source/target correlation, inspection facts, and explicit
  `recommended_action`, `recovery_policy`, and `recovery_options` as the active
  supervision slice
- the owner lane is explicit, currently `wrapper_background_shell`
- the source request is explicit through `source_call_id`
- if the wrapper lane has already started a shell job, the payload carries a
  correlated `observed_background_shell_job` object with the `bg-*` identity,
  status, command, and recent output lines
- worker/backlog inspection comes from `async_tool_workers` and
  `async_tool_backpressure`, not from inferring hidden background-task APIs
- `async_tool_backpressure` carries explicit backlog `recommended_action`,
  `recovery_policy`, and `recovery_options`, plus oldest-worker identity such as
  `oldest_request_id` / `oldest_thread_name` and retained correlation facts such
  as `oldest_source_call_id` and `oldest_target_background_shell_job_id`

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
or other client fixtures.

## Non-Goals

This fixture is not:

- a stable public SDK
- a replacement for the connector smoke tests
- a generic `agentd` client
- a websocket client

It is a concrete companion to the current connector adapter surface, not a compatibility
promise.

## Relationship To Other Broker Docs

- High-level architecture: [codexw-broker-connectivity.md](codexw-broker-connectivity.md)
- Connector mapping: [codexw-broker-connector-mapping.md](codexw-broker-connector-mapping.md)
- Connector adapter plan: [codexw-broker-connector-adapter-plan.md](codexw-broker-connector-adapter-plan.md)
- Local API route ownership: [codexw-local-api-route-matrix.md](codexw-local-api-route-matrix.md)

## Future Direction

The fixture suggests the next likely evolution:

- either promote this client shape into a tiny supported example tool
- or replace it later with a thin real client library once the connector/API
  surface stabilizes enough

Until then, this script is the best fact-based demonstration that the connector
is usable by something that is not an internal test harness.
