# codexw Plugin System Implementation Plan

## Purpose

This document turns the plugin-system requirement into a delivery order that can
work with the self-evolution and self-supervision lanes.

It sits below:

- [codexw-plugin-system.md](codexw-plugin-system.md)
- [codexw-self-evolution.md](codexw-self-evolution.md)
- [codexw-self-supervision.md](codexw-self-supervision.md)

## Goal

Deliver the smallest useful plugin slice that lets `codexw`:

- install trusted optional capability packages
- load and monitor them safely
- prefer plugin update over full core replacement when appropriate
- roll plugins back cleanly when they fail

This plan is about local/runtime capability delivery, not a broker artifact
API. Plugin payload management should not be described as if it proves
broker-visible artifact index/detail/content support.

## First Deliverables

The first implementation slice should include:

- a plugin manifest format
- a dedicated plugin directory under `codexw`-managed state
- enable/disable/install/update metadata
- trusted-source checks
- compatibility checks against the running `codexw` version
- supervision-visible plugin lifecycle events

## Suggested Delivery Order

### 1. Manifest parser

Add a manifest type that validates:

- `plugin_id`
- `version`
- `kind`
- `entrypoint`
- `trusted_source`
- `min_codexw_version`

### 2. Plugin state directory

Create a dedicated plugin state/install location that separates:

- installed plugin payloads
- plugin manifests
- enabled/disabled state
- supervision or health metadata

### 3. Trusted source policy

Add an initial trusted-source rule set that recognizes:

- local explicit plugin installs
- the dedicated plugin repo:
  - <https://github.com/lzbgt/codexw-plugins>

The first slice should not allow arbitrary remote plugin execution by default.

### 4. Loader and healthcheck

Add a narrow loader that:

- resolves enabled plugins
- loads entrypoints in a bounded way
- records healthcheck success/failure
- exposes failures as supervision-visible events

### 5. Self-evolution decision hook

Before full binary replacement, the runtime should ask whether:

- the missing feature is plugin-satisfiable
- a trusted plugin update is available

If yes, prefer plugin install/update over full core replacement.

### 6. Rollback path

If a plugin install or activation fails:

- keep the previous plugin state intact when possible
- disable the broken plugin explicitly
- preserve enough metadata for diagnosis

## Explicitly Deferred

The first slice should defer:

- cross-host plugin distribution
- broker-coordinated plugin install/update
- any broker-visible artifact index/detail/content route family
- arbitrary third-party marketplace semantics
- unrestricted in-process code loading
- plugin hot-patching of the core binary

## Proof Expectations

The first implementation should prove:

- manifest validation rejects malformed plugins
- trusted-source policy blocks untrusted plugin sources
- plugin enable/disable state is durable
- plugin load failures are visible to supervision
- rollback leaves the runtime usable
- plugin-first routing is preferred when the feature does not require new core
  semantics
