# codexw Native Product Recommendation

This document is the current recommendation for the non-broker side of
`codexw`'s product shape.

It is intentionally narrower than
[codexw-native-gap-assessment.md](codexw-native-gap-assessment.md).
That assessment explains the remaining gaps and tradeoffs. This document says
what `codexw` should actually optimize for right now.

Related docs:

- [codexw-native-gap-assessment.md](codexw-native-gap-assessment.md)
- [codexw-design.md](codexw-design.md)
- [../TODOS.md](../TODOS.md)

## Recommendation Summary

`codexw` should continue to optimize for a:

- terminal-first product shape
- scrollback-first UI model
- text-first realtime model
- wrapper-owned async shell/orchestration model

It should not currently optimize for:

- full alternate-screen parity with the native upstream Codex TUI
- upstream-style audio parity
- backend-owned async execution/session parity that app-server does not expose

## Recommended Product Position

The current facts favor describing `codexw` as:

- an app-server-backed terminal client
- with strong automation defaults
- strong orchestration visibility
- strong wrapper-owned async execution support
- and explicit architectural differences from the native upstream product

That means `codexw` should be improved as its own coherent terminal client,
not described as if it is one large rewrite away from being the upstream TUI.

## Recommendation 1: Keep The Scrollback-First UI Model

`codexw` should continue to treat the normal terminal scrollback model as the
intended default product shape.

Why:

- the current UI is already coherent around inline prompt + transient status +
  scrollback transcript
- terminal-native scrolling and capture are real advantages, not accidental
- a retained alternate-screen widget tree would be a large product shift, not a
  small parity patch
- no current proof suggests alternate-screen parity is the highest-leverage
  next investment

Practical implication:

- improve rendering, prompt behavior, and transcript clarity inside the current
  model first
- only reopen alternate-screen work when a specific workflow is blocked by the
  scrollback model

## Recommendation 2: Keep Realtime Text-First

`codexw` should continue to treat realtime as text-first by default.

Why:

- the repo's existing local API, connector, and proof surfaces are already
  built around textual state and semantic events
- the product identity is still terminal-first
- no concrete terminal-compatible audio UX target is currently documented

Practical implication:

- continue improving textual realtime visibility and state semantics
- do not describe audio as “obviously coming later” unless a concrete target
  and scope are chosen

## Recommendation 3: Keep Audio Out Of Scope For Now

The current recommendation is to keep upstream-style audio parity out of scope
for `codexw` until a concrete supported target exists.

That does not mean audio is impossible forever. It means:

- it is not part of the current recommended product shape
- it should not be treated as silent parity debt
- reopening it should require a concrete target and explicit support boundary

## Recommendation 4: Keep Wrapper-Owned Async Shells As The Intended Solution

`codexw` should continue to present wrapper-owned background shells as the
supported answer to same-turn async shell execution.

Why:

- that model is implemented
- it is visible in the orchestration state and local API
- it is covered by the connector/prototype proof surface
- app-server still does not expose a public client surface that closes the gap
  directly

Practical implication:

- do not describe the current async shell model as a temporary hack
- describe it as the intended supported solution within current app-server
  constraints
- keep the architectural difference from native backend-owned execution
  explicit

## Recommendation 5: Prefer Explicit Boundaries Over Parity Ambiguity

When documentation mentions native upstream differences, `codexw` should prefer
explicit product boundaries over vague “still missing” phrasing.

That means:

- if something is intentionally unsupported, say so
- if something is architectural rather than accidental, say so
- if something should only be revisited under specific conditions, name those
  conditions

## What Would Change This Recommendation

This recommendation should be revisited only if one of these becomes true:

- a concrete user workflow is blocked by the scrollback-first model
- a concrete terminal-compatible audio target is defined
- app-server exposes stronger public execution/session control
- remote clients require layout semantics that the current model cannot support
  coherently

Until then, the correct product posture is:

- optimize the current terminal-first model
- keep the unsupported boundaries explicit
- avoid implying that native parity is merely waiting on implementation time
