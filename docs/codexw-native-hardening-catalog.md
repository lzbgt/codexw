# codexw Native Hardening Catalog

This document tracks optional native-side hardening work that is useful but
not currently a blocker for the supported `codexw` product shape.

It is the native-side analogue to
[codexw-broker-hardening-catalog.md](codexw-broker-hardening-catalog.md).

The purpose is to keep optional improvements visible without letting them blur
the current support boundary or backlog.

Related docs:

- [codexw-native-product-recommendation.md](codexw-native-product-recommendation.md)
- [codexw-native-support-policy.md](codexw-native-support-policy.md)
- [codexw-native-support-boundaries.md](codexw-native-support-boundaries.md)
- [codexw-native-product-status.md](codexw-native-product-status.md)
- [../TODOS.md](../TODOS.md)

## How To Read This Catalog

Items here are:

- potentially useful
- sometimes high-quality future work
- but not active blockers for the current supported native product shape

An item should move into [../TODOS.md](../TODOS.md) only if:

- it becomes necessary for the supported product claim
- a real regression appears
- a concrete workflow is blocked without it

## 1. Transcript And Prompt Ergonomics

Potential work:

- additional transcript summarization or folding affordances
- more precise status-line verbosity controls
- stronger prompt editing polish around complex wrapped drafts
- more deliberate rendering choices for long-running shell/service sections

Why this is optional:

- the current scrollback-first prompt/transcript model is already supported and
  coherent
- these are quality improvements inside the current model, not missing product
  prerequisites

## 2. Orchestration UX Depth

Potential work:

- richer summary surfaces for worker/dependency state
- better filtering or grouping across shells/services/capabilities
- stronger operator affordances around long-lived service workflows

Why this is optional:

- the current orchestration model is already implemented and proven enough for
  the supported terminal-first product shape
- further work here is refinement, not missing product legitimacy

## 3. Alternate-Screen Experimentation

Potential work:

- optional alternate-screen preview mode
- popup/picker experiments
- richer retained-layout prototypes

Why this is optional:

- the current recommendation is explicitly scrollback-first
- alternate-screen parity is not part of the supported native product claim
- work in this area should be driven by real workflow pressure, not vague
  parity ambition

## 4. Audio / Realtime Expansion

Potential work:

- defining a narrow supported terminal-compatible audio target
- richer realtime session affordances
- media-session integration experiments

Why this is optional:

- the current supported product is text-first realtime
- audio parity is explicitly unsupported
- there is no chosen supported target yet

## 5. Backend-Owned Async Execution Revisit

Potential work:

- re-evaluating async execution if app-server gains stronger public process or
  session control
- documenting a migration path if wrapper-owned shells ever stop being the
  best supported solution

Why this is optional:

- the wrapper-owned background shell model is already the supported answer
- there is no current public app-server capability that changes that decision

## 6. Stronger Native-Side Proof Expansion

Potential work:

- more explicit route-to-proof mapping for native local runtime surfaces
- stronger direct evidence around prompt/transcript guarantees
- additional adversarial tests around wrapper-owned shell workflows

Why this is optional:

- the native-side proof matrix is already strong enough for the current
  support claim
- this would strengthen confidence, not unlock a missing product decision

## Catalog Maintenance Rule

If a native-side item is:

- useful
- interesting
- but not required for the current support claim

it belongs here, not in [../TODOS.md](../TODOS.md).

That keeps the active backlog honest and prevents optional parity work from
being mistaken for unfulfilled supported product commitments.
