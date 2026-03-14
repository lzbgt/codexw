# codexw Native Support Policy

This document defines what "supported" means for the non-broker side of
`codexw`.

It complements the native-side recommendation, boundary, status, and proof
docs by answering a narrower operational question:

- what is the supported product surface right now?
- what stability should users expect from that surface?
- what kinds of native-side changes are allowed without changing the support
  claim?

Related docs:

- [codexw-native-product-recommendation.md](codexw-native-product-recommendation.md)
- [codexw-native-support-boundaries.md](codexw-native-support-boundaries.md)
- [codexw-native-product-status.md](codexw-native-product-status.md)
- [codexw-native-proof-matrix.md](codexw-native-proof-matrix.md)
- [codexw-native-gap-assessment.md](codexw-native-gap-assessment.md)
- [codexw-self-evolution.md](codexw-self-evolution.md)
- [codexw-self-supervision.md](codexw-self-supervision.md)
- [codexw-plugin-system.md](codexw-plugin-system.md)
- [../TODOS.md](../TODOS.md)

## Supported Native Product Surface

The supported non-broker product surface is:

- terminal-first
- scrollback-first
- inline prompt plus transient status
- text-first realtime
- wrapper-owned async shell orchestration
- explicit self-supervision for wedged tool/runtime recovery as an intended
  native capability direction
- plugin-first optional capability expansion rather than forcing every new
  feature into core replacement

That means the native-side support claim is about the product that already
exists, not about hypothetical upstream parity.

## Stability Expectations

### Stable Enough To Rely On

The following are part of the supported native-side contract and should not
drift casually:

- normal terminal scrollback as the primary transcript surface
- inline prompt/editor behavior as the main interaction model
- transient status rendering as the primary live-status surface
- text-first realtime visibility
- wrapper-owned background shell behavior and orchestration views
- async shell-tool execution that remains visible in prompt/status surfaces
  while the request is in flight, rather than freezing the input loop
- operator-visible async-tool supervision classifications such as `tool_slow`
  and `tool_wedged` for long-running shell-tool work
- the direction that stalled tool/runtime paths should be recoverable through
  self-supervision rather than left as indefinite hangs
- the direction that optional capabilities should prefer plugin delivery when
  core runtime semantics do not need to change
- explicit documentation of unsupported native-parity areas

Breaking one of those is not a harmless implementation detail. It is a change
to the supported product shape.

### Allowed To Improve

The following kinds of changes are encouraged and do not change the support
claim by themselves:

- clearer transcript rendering
- stronger prompt ergonomics
- better status wording
- better orchestration visibility
- better wrapper-owned shell tooling
- stronger self-supervision and self-heal behavior inside the current
  terminal-first model
- better plugin-managed optional capability delivery
- stronger local API or broker documentation around existing boundaries

These are improvements inside the current support boundary, not changes to the
boundary itself.

## What Is Not Promised

The native support policy does not promise:

- alternate-screen parity with upstream Codex
- audio parity
- backend-owned async execution parity
- popup-heavy retained-widget interaction semantics

Those remain explicit unsupported areas unless the native recommendation and
support-boundary docs change.

## What Counts As A Native-Side Regression

The following count as native-side regressions because they break the current
supported product surface:

- making the scrollback-first UI incoherent or unreliable
- breaking the inline prompt/editor interaction model
- reducing text-first realtime observability
- breaking wrapper-owned async shell workflows
- regressing into indefinite wedged tool behavior without a coherent
  self-supervision story
- reintroducing vague wording that implies unsupported native parity is already
  expected

The following do not count as regressions on their own:

- not adding alternate-screen mode
- not adding audio UX
- not adding backend-owned async execution parity

Those are unsupported areas, not silently promised work.

## What Would Change This Policy

This policy should be revised only if one of these becomes true:

- the product intentionally adds a supported alternate-screen mode
- the product intentionally chooses a supported audio target
- app-server exposes a public surface that materially changes async execution
  parity
- the native-side recommendation itself changes

Until then, native-side work should optimize the current terminal-first shape
instead of quietly broadening support claims.

## Practical Use

Use this document when a question is really about support-level meaning, for
example:

- "is scrollback-first still the supported UI contract?"
- "does native support imply audio parity?"
- "would changing prompt/status behavior count as a regression?"

Use the other native docs for different questions:

- gap analysis:
  [codexw-native-gap-assessment.md](codexw-native-gap-assessment.md)
- product recommendation:
  [codexw-native-product-recommendation.md](codexw-native-product-recommendation.md)
- unsupported boundaries:
  [codexw-native-support-boundaries.md](codexw-native-support-boundaries.md)
- concise snapshot:
  [codexw-native-product-status.md](codexw-native-product-status.md)
- evidence map:
  [codexw-native-proof-matrix.md](codexw-native-proof-matrix.md)
- support-claim review checklist:
  [codexw-support-claim-checklist.md](codexw-support-claim-checklist.md)

For batches that change native support wording across status, policy, proof, or
README surfaces, use the operational checklist in
[codexw-support-claim-checklist.md](codexw-support-claim-checklist.md).
