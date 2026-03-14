# codexw Broker Client + Host Shell Architecture

## Purpose

This document records the current architecture requirement for `codexw`'s
broker-facing evolution.

The requirement is no longer just "make remote control plausible." `codexw`
must support:

1. broker-exposed client attachment for non-terminal clients such as app and
   WebUI surfaces
2. broker-exposed host shell access so those clients can examine the host and
   generated artifacts without needing direct local terminal access

This document is the source-of-truth design note for that requirement. It sits
above the narrower route, adapter, and proof docs.

For the workflow-level read on what remote clients can already inspect today,
see [codexw-broker-host-examination-matrix.md](codexw-broker-host-examination-matrix.md).
For the short implementer-facing handoff to the sibling `~/work/agent`
workspace, see
[codexw-broker-integration-handoff.md](codexw-broker-integration-handoff.md).
For the next native architecture track beyond single-deployment client
attachment, namely cross-deployment `codexw` collaboration and explicit work
handoff, see
[codexw-cross-deployment-collaboration.md](codexw-cross-deployment-collaboration.md).

## External Baseline

The sibling `~/work/agent` workspace already documents the architecture shape
that `codexw` needs to align with:

- `/Users/zongbaolu/work/agent/DESIGN.md`
  - daemon-first, multi-client layering
  - broker as an optional but first-class control plane
  - full host tool ecosystem including shell and filesystem tools
- `/Users/zongbaolu/work/agent/docs/BROKER.md`
  - broker relay model
  - client-facing proxy and SSE paths
  - authenticated client and deployment routing
- `/Users/zongbaolu/work/agent/docs/CLIENT.md`
  - client identity
  - client events
  - bidirectional collaboration and event replay
- `/Users/zongbaolu/work/agent/docs/WEBUI.md`
  - direct mode and broker mode for WebUI
  - connection profiles that target a broker-managed agent runtime

Those documents establish a fact-based baseline: brokered app/WebUI clients and
broker-mediated host tooling are already part of the broader system
architecture, so `codexw` should not keep treating them as a vague future UX
idea.

They also imply a follow-on native requirement once multiple broker-routed
deployments exist: deployment-to-deployment collaboration should become a
first-class routed surface rather than a manual operator convention.

## Required Product Posture

`codexw` should be understood as one runtime that can serve two modes at once:

- native terminal-first operation for local operator use
- broker-exposed runtime control for remote clients such as app and WebUI

That does **not** mean the broker becomes the source of truth. The source of
truth should remain the `codexw` runtime plus its local HTTP/SSE API. Broker
exposure should layer on top of that runtime, not replace it.

## Required Client Surface

The broker-facing client surface should cover the same practical workflow that a
human operator can already drive locally:

- create, attach, inspect, and list sessions
- start and interrupt turns
- consume transcript snapshots and SSE event streams
- inspect orchestration state
- operate wrapper-owned background shells and reusable services
- inspect the resulting host-side outputs and artifacts

The key point is that "broker client support" is not complete if it only
forwards high-level conversation turns. It must also carry the host-examination
surfaces that make `codexw` useful for real engineering work.

## Host Shell + Artifact Exposure

For `codexw`, host examination should be exposed through the existing wrapper
surfaces that already model durable shell and service state:

- background shell lifecycle and IO
- service/capability inspection and attachment
- transcript/event history
- any host-generated artifact references that already appear in transcript or
  shell output

This design requirement is intentionally **not** a return to the removed
workspace dynamic tools. Those tools were de-advertised because they were too
bug-prone relative to their value. The preferred general-purpose host
examination substrate remains:

- host shell commands
- Python or other host interpreters when appropriate
- wrapper-owned background shell/service control when the work must remain
  structured and observable over time

In other words, remote clients should reach host examination through brokered
shell/service control, transcript/event inspection, and explicit artifact
references, not through a resurrected structured workspace-read tool family.

## Architecture Direction

The required direction remains:

1. `codexw` local HTTP/SSE API as the canonical runtime control surface
2. broker-facing connector/adapter as the first integration path
3. client apps and WebUI attach through the broker-facing layer

This matches the current implementation facts better than direct broker
connectivity inside the main wrapper process:

- `codexw` is still an app-server client, not a standalone daemon
- the loopback local API already exists
- the connector already proves broker-style alias routes and SSE relay
- the terminal runtime and remote-control security boundaries are easier to keep
  separate with a connector boundary

Direct embedded broker connectivity inside `codexw` may still become desirable
later, but it is not required to satisfy the present architecture requirement.

## Mapping To Current `codexw` Surfaces

The current implementation already covers a meaningful subset of the required
broker-facing host surface:

- session lifecycle via the local API and connector aliases
- turn control via the local API and connector aliases
- transcript fetch plus SSE replay/resume
- orchestration inspection
- shell list/start/detail/poll/send/terminate
- reusable service and capability list/detail/attach/wait/run

That means the main architecture gap is no longer "can brokered clients touch
host state at all?" They already can.

The remaining product question is whether the exposed broker-facing surface is
complete enough for app/WebUI clients to examine the host and artifacts without
falling back to direct terminal access.

## Current Gaps To Keep Explicit

The following gaps should remain explicit while this architecture is being
carried forward:

- app/WebUI clients live in the sibling `~/work/agent` workspace rather than in
  this repo
- `codexw` does not yet define a richer broker-visible artifact catalog
  separate from transcript, shell output, and existing file/path references
- auth, identity, and deployment routing still depend on the external broker
  layer rather than being fully modeled inside `codexw`

These are architecture gaps worth tracking, but they do not invalidate the
requirement itself.

## Practical Rule

When broker-facing work is added in this repo, ask this concrete question:

"Would an app or WebUI client attached through the broker be able to inspect
the session, steer the runtime, operate the necessary host shell/service flows,
and understand the resulting artifacts without direct terminal access?"

If the answer is no, the broker/client architecture requirement is not yet met.
