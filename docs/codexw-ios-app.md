# codexw iOS App Design

## Purpose

This document defines the first iOS client for broker-managed `codexw`
deployments.

The app is not a direct `codex app-server` client. It is a broker client for
`codexw` runtimes.

That distinction matters because the app needs:

- deployment selection
- session and thread inspection
- remote turn control
- transcript and event streaming
- remote shell/service inspection
- supervision and self-heal visibility

Those are `codexw` runtime concepts, not raw app-server stdio concepts.

## Product Goal

The first iOS app should let an operator do the highest-value remote tasks
without opening a desktop terminal:

- see available `codexw` deployments
- inspect whether a deployment is idle, running, quiet, or self-healing
- attach to a session or create one
- resume a thread
- submit a prompt
- interrupt a turn
- read transcript and semantic events
- inspect shell and service state
- receive push notifications for blocked or completed work

## First Release Scope

The first iOS release should be control-first and observation-first.

Ship:

- sign in to broker
- deployment list
- deployment detail
- session list and current session snapshot
- transcript view
- turn submit and interrupt
- orchestration status summary
- shell list and shell detail
- push notifications

Defer:

- full interactive PTY terminal UI
- artifact browser beyond explicit transcript/shell references
- multi-window collaborative editing
- complex project/dependency graph visualizations

## Why Not Start With A Full Terminal

An iPhone terminal emulator is possible, but it is not the first bottleneck.

The first bottleneck is safe, low-friction remote control and visibility. A
structured mobile client is more valuable earlier than a perfect PTY clone
because most remote actions are:

- checking status
- reading recent output
- approving or interrupting work
- submitting one short follow-up prompt

PTY fidelity can come later.

## Architecture

The iOS app should talk only to the broker.

Flow:

1. iOS app authenticates to the broker
2. broker returns available deployments
3. app selects one deployment and session
4. broker relays requests to the connector or deployment agent
5. deployment events stream back through the broker

The app should never need direct access to:

- local `codexw` loopback HTTP
- local filesystem
- raw app-server stdio

## Data Model

The app should understand these core entities:

- deployment
- runtime instance
- session
- thread
- turn
- transcript item
- shell job
- service
- capability
- supervision notice

The app should treat deployment identity and session identity as separate.

## Deployment Discovery

The broker should expose deployment discovery using runtime facts published by
`codexw`, including:

- runtime instance id
- suggested deployment id
- host OS
- host architecture
- Apple Silicon flag
- process age
- current session id
- cwd context

That is why the `codexw` local API now needs a runtime-discovery route instead
of forcing the mobile client to infer deployment identity from session payloads.

## UX Surfaces

### Deployment List

Show:

- deployment id
- current status
- host label
- last heartbeat age
- active turn indicator
- active shell/service counts

### Session View

Show:

- session id
- attached thread id
- objective
- working state
- started/completed turn counts
- current supervision or quiet-turn notice

### Transcript View

Show:

- user and assistant transcript entries
- system or supervision notices
- shell/service result summaries
- token usage summary when present

### Remote Actions

Support:

- submit prompt
- interrupt turn
- refresh snapshot
- open shell detail
- open service detail

## Notifications

Push notifications are a first-class feature for iOS because mobile value is
mostly asynchronous.

Initial notification families:

- turn completed
- turn needs operator intervention
- deployment disconnected
- self-heal triggered
- shell/service became ready

The app should deep-link back into the affected deployment/session/turn.

## SwiftUI App Structure

Recommended first split:

- `Auth`
- `Deployments`
- `Sessions`
- `Transcript`
- `Shells`
- `Services`
- `Notifications`

Transport:

- HTTPS for one-shot requests
- WebSocket or SSE-over-broker for streaming updates

## M2 Mac Verification Plan

This repo is being developed on an Apple Silicon Mac, which is useful for the
mobile lane:

- the Go broker can run locally
- `codexw` can run locally
- iOS simulator can connect to the local broker
- the host already has Apple developer enrollment, so real signing is possible

That means the first iOS slice should be simulator-verifiable on this machine
before any remote-device-only assumptions are made.

## Non-Goals

The first iOS app should not try to be:

- a replacement for the desktop terminal UI
- a full filesystem browser
- a broker administration console
- a local-only direct-mode client that bypasses the broker

## Recommended Delivery Order

1. Deployment discovery and health screen
2. Session snapshot and transcript screen
3. Turn submit and interrupt
4. Live event stream and supervision states
5. Shell and service inspection
6. Push notifications
7. Optional PTY UI later
