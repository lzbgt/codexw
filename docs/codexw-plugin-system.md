# codexw Plugin System

## Purpose

This document records a fundamental requirement for `codexw` growth:

- optional new capabilities should prefer landing as plugins
- self-evolution should be able to install, update, enable, disable, or roll
  back plugins safely
- the core binary should only change when runtime, protocol, or safety
  semantics actually need to change

This keeps self-evolution manageable. `codexw` should not need a full binary
replacement every time it discovers a useful extension.

This plugin lane is also separate from the broker artifact track. Plugin
installation and lifecycle are local/runtime capability management, not proof
that a broker-visible artifact index/detail/content API exists.

For the source docs that define the current shell-first remote/workspace
surface and local runtime semantics that this plugin lane builds on, see:

- [codexw-workspace-tool-policy.md](codexw-workspace-tool-policy.md)
- [codexw-local-api-sketch.md](codexw-local-api-sketch.md)
- [codexw-local-api-implementation-plan.md](codexw-local-api-implementation-plan.md)
- [codexw-local-api-event-sourcing.md](codexw-local-api-event-sourcing.md)
- [codexw-local-api-route-matrix.md](codexw-local-api-route-matrix.md)

## Problem Statement

During normal project work, a running `codexw` instance may identify that it
would benefit from capabilities it does not currently have, for example:

- voice reminder over the host speaker
- live IM progress reporting to the user
- notification sinks for long-running build/test jobs
- project- or environment-specific adapters that are useful but not part of the
  core runtime contract

If every one of those capabilities requires a full core patch, build, install,
and restart, the system becomes hard to evolve safely.

## Design Stance

The plugin system should be:

- optional rather than mandatory for core operation
- capability-scoped rather than arbitrary code injection
- trust-aware rather than “install anything from anywhere”
- supervision-aware rather than invisible to runtime monitoring
- compatible with self-evolution rather than separate from it

## What A Plugin Is

For `codexw`, a plugin should be a separately versioned extension package with:

- a stable `plugin_id`
- a semantic version
- a manifest that declares capability type and trust metadata
- a loadable entrypoint
- explicit enable/disable state

The initial plugin kinds should stay narrow, for example:

- notification
- progress relay
- external integration
- optional host capability adapters

The first plugin system should **not** try to support arbitrary in-process code
mutation with unrestricted runtime privileges.

## Manifest Sketch

Minimum manifest fields should include:

- `plugin_id`
- `version`
- `display_name`
- `kind`
- `entrypoint`
- `capabilities`
- `trusted_source`
- `min_codexw_version`
- `enabled_by_default`

Useful optional fields:

- `description`
- `homepage`
- `repo`
- `permissions`
- `healthcheck`
- `rollback_hint`

## Trust Model

The plugin system must stay source-aware.

Safe initial rules:

- trusted local plugins are allowed
- a dedicated plugin repo may be treated as a named trusted source
- ad hoc remote download and execution from arbitrary URLs is not allowed by
  default
- plugin activation should be explicit and inspectable

The dedicated plugin repo for this track is:

- <https://github.com/lzbgt/codexw-plugins>

That repo should hold plugin packages, manifests, and release metadata that are
meant to be consumed by `codexw` self-evolution or operator-driven plugin
installation workflows.

## Relationship To Self-Evolution

The plugin system is part of the self-evolution story, not a competing story.

Preferred decision rule:

- if a missing capability can be delivered through the plugin API safely,
  prefer plugin install or plugin update
- if the requirement needs new core runtime semantics, new supervision logic,
  new protocol routes, or new restart/handoff behavior, use core self-evolution

Examples:

- voice reminder over host speakers: likely plugin-first
- live IM progress reporting: likely plugin-first
- fixing a wedged dynamic-tool execution model: core self-evolution
- adding a new handoff checkpoint mode: core self-evolution

## Relationship To Self-Supervision

Plugins must be visible to supervision.

That means the runtime should be able to observe:

- plugin install/update attempts
- plugin load success or failure
- plugin healthcheck failures
- plugin-caused stalls or crashes
- plugin rollback events

The supervision lane is documented in
[codexw-self-supervision.md](codexw-self-supervision.md).

## Safe Lifecycle

The first plugin lifecycle should be explicit:

1. discover a trusted plugin candidate
2. inspect manifest and compatibility metadata
3. install to a dedicated plugin directory
4. enable or update the plugin explicitly
5. healthcheck the loaded plugin
6. disable or roll back on failure

The first slice should prefer explicit local state over hidden magic.

## Boundaries

The first plugin system should not promise:

- arbitrary remote code execution from untrusted sources
- cross-host plugin federation
- plugin hot-patching of the live core binary image
- broker-dependent installation semantics
- a global plugin marketplace in this repo

The plugin lane must remain useful even for a standalone local `codexw`
instance with no broker involved.

That boundary should stay explicit in docs and status claims: the current
supported experimental adapter still stops at the shell-first host-examination
surface, while plugin delivery is a separate local/runtime lane.
