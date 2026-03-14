# codexw Self-Evolution and Safe Runtime Replacement

## Purpose

This document records a native requirement that follows from how `codexw` is
actually used in practice:

- a running `codexw` instance may identify a bug in itself
- that bug may already be fixed in a newer `codexw` build in the same repo
- the currently running instance is still the old binary until the user
  restarts it

`codexw` therefore needs a safe way to evolve itself without depending on
manual user intervention.

This is not a claim that self-replacement is implemented today. It is a design
note for how it should work.

## Problem Statement

The current reality is:

- `codexw` can rebuild and install a newer binary through
  `./scripts/install-codexw`
- `codexw` can resume an existing thread deterministically
- `codexw` can emit a concrete resume command when exiting

But it cannot yet do the most important combined workflow:

1. detect that the current instance is outdated or wedged
2. prepare safe continuation state
3. launch the newer binary
4. confirm the newer binary resumed the intended session
5. retire the old process without asking the user to do that manually

Without that lane, self-aware diagnosis is incomplete: the runtime may know it
should upgrade itself but still cannot safely do so.

## Design Stance

Self-evolution should be:

- explicit rather than magical
- resumable rather than in-place memory mutation
- thread-centric rather than process-state-centric
- rollback-aware rather than one-way
- standalone local-runtime-first rather than broker-dependent
- plugin-aware rather than forcing every new feature into core replacement

The key rule is:

`codexw` should replace the running process by handing off durable session
context to a newer process, not by trying to hot-patch the live binary image.

Stated more concretely: self-evolution is the lane where a running `codexw`
instance safely hands off to a newer binary or installs a newly trusted plugin
without waiting for a human to restart it.

## What Counts As “Evolution”

The first useful definition is narrow.

Self-evolution means:

- switch from one `codexw` executable generation to another
- preserve enough session context that work can continue
- preserve enough operator context that the transition is explainable
- optionally acquire that newer generation from the known local `codexw` repo
  when policy allows

It does **not** mean:

- self-modifying code in place
- patching the live process image without restart
- silently discarding the old session context
- replacing the underlying `codex` backend model or thread identity

The self-evolution requirement is independent of the cloud broker. A standalone
local `codexw` instance should be able to evolve itself safely even if no
broker, connector, or remote client is involved at all.

## Safety Model

The first self-evolution lane needs five explicit phases.

### 1. Detect

The running instance identifies one of these reasons:

- current binary generation is older than the repo-installed generation
- a known issue is active and the local repo contains a fix worth taking
- the working session identifies high-value missing functionality, such as
  voice reminder over host speakers or live IM progress reporting, and the local
  repo already contains or can safely acquire that capability
- the current process is partially wedged but still responsive enough to
  prepare a handoff
- a high-value missing capability is best delivered as a trusted plugin instead
  of a full core replacement

### 2. Checkpoint

Before replacement, the old instance must capture a durable handoff record for
the new process.

Minimum checkpoint fields:

- current `thread_id`
- resolved `cwd`
- current draft input if any
- current auto-continue intent
- last known session/runtime snapshot summary
- reason for self-evolution
- expected replacement binary path

This checkpoint should be written to a durable local handoff file, not kept
only in memory.

### 3. Launch

The old instance launches a newer `codexw` binary through an explicit relay
entrypoint.

The first slice should prefer:

- a small launcher/reexec helper
- or a dedicated `codexw` startup mode such as `--resume-handoff <file>`

The new process should not have to infer restart intent by scraping terminal
history or reading stale temp files opportunistically.

### 4. Acknowledge

The new process should explicitly confirm:

- it read the handoff file
- it resumed the intended thread
- it restored enough context to continue safely

Only after that acknowledgment should the old process consider the replacement
successful.

### 5. Retire Or Roll Back

If acknowledgment succeeds:

- old process exits cleanly
- handoff file can be marked complete or removed

If acknowledgment fails:

- old process should emit a concrete rollback or manual resume path
- handoff file should remain inspectable for diagnosis
- the system should prefer “old process remains in control” over silent loss of
  session continuity

## First Trigger Classes

The first implementation should keep trigger classes explicit.

Reasonable first triggers:

- `version_outdated`
- `self_fix_available`
- `runtime_wedged_but_checkpointable`

The first slice should **not** auto-upgrade on every new commit or every build.
It should only evolve when:

- there is a concrete identified value
- the transition can be checkpointed safely

## Update Source Policy

The stronger version of this requirement is:

- a running instance should know where its `codexw` git repo is
- it should be able to decide whether to use the current checked-out tree or
  refresh that tree
- it should be able to build, install, and hand off to the resulting binary

But that only stays safe if update sources are explicit.

The first safe rule should be:

- using the current local checked-out repo is allowed
- `git pull` or similar source refresh is a separate policy-gated action
- untrusted arbitrary update discovery is not allowed

So yes, the long-term self-evolution requirement includes “know repo path,
update, build, install, and replace the running instance with handoff,” but
the `update` step must remain trust-aware rather than automatic by default.

## Relationship To Plugins

Not every missing feature should force a full core rebuild.

Some needs are better treated as plugin-delivered capabilities, for example:

- host voice reminder output
- live IM progress reporting
- optional notification sinks
- project-specific integrations that are useful but not core runtime semantics

The preferred rule should be:

- if the missing capability can be added through a trusted plugin API without
  changing the core runtime contract, prefer plugin installation or plugin
  update
- if the missing capability requires new core runtime behavior, a new startup
  mode, or a new safety/ownership contract, use full binary self-evolution

That keeps self-evolution manageable: `codexw` should not need a full binary
replacement every time it discovers an optional extension.

The plugin architecture for that lane is documented separately in
[codexw-plugin-system.md](codexw-plugin-system.md) and
[codexw-plugin-system-implementation-plan.md](codexw-plugin-system-implementation-plan.md).

## First Boundary

The first self-evolution lane should be local-runtime-focused.

That means:

- one `codexw` process replaces itself on the same host
- replacement uses the existing thread/resume semantics
- no broker coordination is required for the first slice

Later, broker-visible coordination may matter for remote sessions, but the
standalone local process-handoff lane is the high-leverage first step.

## Relationship To Existing Thread Semantics

The existing `thread_id` model is the most important enabling fact.

Because `codexw` already treats thread resume as durable and explicit:

- self-evolution can be modeled as “resume the same thread in a newer binary”
- the handoff artifact does not need to invent a second session model
- the operator-visible rollback path can remain a plain resume command

This is much safer than trying to migrate transient in-memory editor or event
state wholesale.

## Relationship To Automation

Self-evolution should respect the current automation posture:

- preserve whether auto-continue should keep running
- preserve the active continuation objective
- preserve any user draft text that has not been submitted yet

The new instance should therefore resume not just the thread, but also the
local operator intent around the thread.

## Relationship To Hangs And Recovery

This design is directly relevant to issues like:

- stuck interrupt handling
- wedged dynamic-tool polling
- UI/runtime bugs fixed by a newly built binary

The important distinction is:

- interrupt/recovery handles one broken turn
- self-evolution handles one broken client generation

If the runtime can still checkpoint, a safe self-replacement path is often
higher value than forcing the user to discover and perform a manual restart.

## First Implementation Boundary

The first self-evolution slice is well-defined when:

1. a running instance can write a durable self-handoff checkpoint
2. a newer binary can start in a “resume from self-handoff” mode
3. the new instance explicitly acknowledges successful thread recovery
4. the old instance exits only after acknowledgment
5. failure leaves an explicit rollback/manual resume path

## Non-goals

The first slice should **not** try to solve:

- cross-host binary replacement
- broker-coordinated fleet upgrades
- hot patching the live process image
- automatic source fetching or untrusted update discovery
- replacing the upstream `codex` backend as part of the same flow
