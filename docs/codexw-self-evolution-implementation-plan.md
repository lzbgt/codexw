# codexw Self-Evolution Implementation Plan

## Purpose

This document turns the self-evolution requirement into an implementation-facing
delivery order.

It sits below:

- [codexw-self-evolution.md](codexw-self-evolution.md)
- [codexw-self-supervision.md](codexw-self-supervision.md)
- [codexw-plugin-system.md](codexw-plugin-system.md)

## Goal

Deliver the smallest safe slice that lets a running older `codexw` instance:

- checkpoint its current session context
- start a newer binary
- have that newer binary resume the same thread and operator intent
- retire the old process only after explicit acknowledgment

## First Deliverables

The first implementation slice should include:

- a self-handoff checkpoint file format
- a startup mode that resumes from that checkpoint
- acknowledgment from the new process back to the old process
- explicit rollback/manual-resume behavior on failure
- operator-visible status text explaining why replacement happened
- explicit policy for whether the replacement binary comes from:
  - the current local checkout
  - or a trusted repo refresh step such as `git pull`
- explicit policy for when plugin installation/update is preferred over full
  binary replacement

## Explicitly Deferred

The first slice should defer:

- broker-coordinated remote self-upgrade
- unrestricted automatic repo fetch/pull/update logic
- cross-host replacement
- replacing the upstream `codex` backend in the same workflow
- silent always-on auto-upgrade behavior
- automatic plugin publication or cross-host plugin distribution

## Suggested Delivery Order

### 1. Checkpoint format

Add a durable self-handoff record type with:

- `thread_id`
- `cwd`
- draft text if present
- auto-continue marker or continuation mode
- self-evolution reason
- candidate replacement binary path
- creation timestamp

The first format can be JSON in a dedicated temp/runtime directory.

### 2. Startup resume mode

Add a dedicated startup path such as:

- `codexw --resume-handoff <path>`

That mode should:

- load the checkpoint
- resume the target thread
- restore draft/continuation context where possible
- emit explicit acknowledgment on success

### 3. Launch and handshake path

Add an old-process launcher path that:

- validates the replacement binary path
- starts the newer binary with the handoff file
- waits for explicit acknowledgment
- times out cleanly if acknowledgment never arrives

### 4. Rollback path

If acknowledgment fails:

- keep the old process alive when possible
- emit an explicit manual resume command
- keep the handoff file for diagnosis

### 5. Upgrade trigger policy

Only after the handoff path is safe should the runtime add smarter triggers for:

- outdated binary generation
- local issue fixed in the repo
- high-value missing feature identified during work, such as host voice reminder
  or live IM progress reporting
- checkpointable wedged runtime

Those triggers should be fed by the separate self-supervision lane rather than
implemented as ad hoc one-off checks.

### 6. Trusted source policy

Only after the handoff path is safe should the runtime add smarter source
selection for the replacement binary:

- use current checked-out repo by default
- optionally allow trusted refresh such as `git pull`
- never treat arbitrary untrusted remote code discovery as equivalent to a safe
  upgrade source

### 7. Plugin versus core decision policy

Before performing full binary replacement, the runtime should ask:

- can the needed capability be satisfied by a trusted plugin install or plugin
  update?
- does the missing capability require new core CLI/runtime/protocol semantics?

If the answer to the first question is yes and the second is no, prefer the
plugin lane documented in
[codexw-plugin-system.md](codexw-plugin-system.md) and
[codexw-plugin-system-implementation-plan.md](codexw-plugin-system-implementation-plan.md).

## Likely `codexw` Touch Points

The first implementation likely belongs near:

- CLI parsing and startup normalization
- app/runtime exit and resume hint helpers
- thread resume handling
- local runtime state serialization helpers
- a new self-handoff or self-evolution module

It should reuse the current thread resume path rather than inventing a second
session restoration protocol.

## Proof Expectations

The first implementation should prove all of:

- checkpoint file is written with the expected thread/cwd context
- new process can resume from checkpoint deterministically
- acknowledgment is explicit and machine-readable
- old process does not exit before acknowledgment
- failure leaves a usable manual resume path
- replacement-source policy is explicit instead of silently pulling arbitrary
  code

## Relationship To Existing Install Flow

`./scripts/install-codexw` already gives the project a deterministic way to
build, sign, and install the newer binary.

The missing piece is not “how to build a new binary.” The missing piece is:

- how the running old process safely hands control to that newer binary

The stronger future lane is:

- optionally refresh the trusted local repo
- build and install the resulting binary
- then perform the same safe handoff/acknowledgment path

So the self-evolution lane should reuse the existing install flow rather than
replace it.

## Completion Bar

The first implementation track is complete when:

1. a running instance can prepare a durable self-handoff checkpoint
2. a newer binary can resume from that checkpoint explicitly
3. acknowledgment gates old-process exit
4. rollback remains explicit and operator-visible
5. docs/status text say exactly what is implemented and what is still deferred
