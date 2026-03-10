# codexw Native Gap Assessment

This document is the source-of-truth assessment for the main non-broker work
still left in `codexw`.

The broker/local-API track now has explicit contract, proof, recommendation,
support-policy, and backlog documents. The remaining high-leverage work is no
longer mostly about broker reachability. It is about the gap between the
current `codexw` terminal/runtime model and the native upstream Codex product
experience.

Related docs:

- [codexw-design.md](codexw-design.md)
- [../TODOS.md](../TODOS.md)
- [codexw-broker-prototype-status.md](codexw-broker-prototype-status.md)
- [codexw-native-product-recommendation.md](codexw-native-product-recommendation.md)
- [codexw-native-support-boundaries.md](codexw-native-support-boundaries.md)

## Current State

`codexw` already has:

- broad command-side parity for the app-server-backed terminal workflow
- a scrollback-first inline terminal UI
- unattended automation defaults
- wrapper-owned same-turn background shell workflows
- orchestration visibility for agent threads, background shells, terminals, and
  capability/service state
- a loopback local API plus broker-style connector prototype

The main remaining gaps are therefore not "missing command handlers." They are:

1. alternate-screen / widget-tree parity
2. richer realtime UX, especially audio-oriented flows
3. deeper async execution parity with native Codex process/session behavior

## Gap 1: Alternate-Screen / Widget-Tree Parity

### Current behavior

`codexw` is intentionally scrollback-first:

- transcript output is appended into the normal terminal scrollback
- the prompt is rendered inline
- a transient status line sits above the prompt
- the user relies on the terminal's native scroll and history behavior

This is a deliberate design choice, not an accident.

### Native upstream behavior

The native Codex TUI is alternate-screen and widget-tree oriented:

- fixed viewport layout
- dedicated panels and transient widgets
- popup and picker affordances that do not become scrollback
- tighter control of focus, selection, and layout state

### Why this matters

The scrollback-first model has clear benefits:

- terminal-native behavior
- easier transcript capture
- less rendering complexity
- less risk of "blank screen" failure modes

But it also has clear limits:

- popup-heavy interaction cannot be matched exactly
- layout semantics are weaker than a retained widget tree
- some kinds of transient state become harder to present cleanly
- parity claims against the native TUI will always be qualified

### Decision space

There are three realistic positions:

1. Keep the scrollback-first model as the intended product shape.
   This means `codexw` should optimize the current design instead of chasing
   strict native-TUI parity.

2. Add a partial alternate-screen mode.
   This would preserve the current default while introducing an optional native
   layout mode for users who want stronger parity.

3. Re-architect around alternate-screen by default.
   This would be the highest-parity route, but it would also be the largest
   product and implementation shift.

### Current recommendation

The facts today favor option 1 unless a concrete product need appears:

- the current scrollback-first design is already coherent
- most completed work in the repo reinforces that model rather than fighting it
- the remaining parity gain from a widget-tree rewrite is real, but expensive
- there is not yet evidence that this is the highest-leverage next investment

So the active task is not "build alternate-screen mode now." The active task is
"make the product decision explicit and only reopen it if a strong user/workflow
need appears."

## Gap 2: Realtime UX and Audio

### Current behavior

`codexw` currently supports realtime as text-oriented state:

- realtime transport/status
- textual transcript and status integration
- no audio capture/playback UX
- no media-session model

### Native upstream behavior

The upstream product includes a richer realtime/audio experience.

### Why this matters

This is the largest user-visible behavior difference outside the TUI model.

If `codexw` remains a primarily terminal-first engineering client, it is
reasonable to decide that audio parity is intentionally out of scope. If
`codexw` wants to claim broader native-product parity, audio becomes harder to
ignore.

### Decision space

1. Explicitly keep audio UX out of scope for `codexw`.
2. Add a minimal audio-aware realtime mode.
3. Pursue broader upstream-style audio parity.

### Current recommendation

The facts currently favor option 1 or, at most, a very narrow option 2:

- this repo is terminal-first
- no current broker/local-API surface is designed around media transport
- no existing proof surface suggests audio is the highest-leverage next build

So the near-term work is decision documentation, not implementation. The repo
should be explicit that realtime is text-first today and that audio parity is
not assumed.

## Gap 3: Async Execution Parity

### Current behavior

Because app-server does not expose a public client request for directly writing
to or polling model-owned `item/commandExecution` sessions, `codexw` implements
same-turn async shell behavior through wrapper-owned dynamic tools and
background shell jobs.

That gives the user and the model something practical and useful, but it is not
the same thing as native process/session reuse inside upstream Codex internals.

### Why this matters

This is the main architectural gap beneath several UX differences:

- background shells are wrapper-owned, not backend-owned
- `/ps` and orchestration views are coherent, but not fully native-equivalent
- process reuse semantics differ from upstream unified-exec-style behavior

### Decision space

1. Keep the wrapper-owned background-shell model as the intended solution.
2. Extend the local API/connector around the current model and accept the
   architectural difference explicitly.
3. Wait for app-server to expose stronger process/session control surfaces.
4. Attempt a deeper non-app-server integration route.

### Current recommendation

The facts favor options 1 and 2 for now:

- the current background-shell system is already implemented and well proven
- the broker/local-API surfaces are built around it
- there is no public app-server surface today that closes the gap directly
- a deeper integration route would be a separate architecture project, not an
  incremental cleanup

So the near-term task is to keep describing this as an explicit architectural
boundary, not to imply that native process/session parity is already achievable.

## Relationship To Current Recommendation

This document is the assessment layer for native-side remaining work.

The current source-of-truth recommendation and support boundary are in:

- [codexw-native-product-recommendation.md](codexw-native-product-recommendation.md)
- [codexw-native-support-boundaries.md](codexw-native-support-boundaries.md)

That means:

- this document explains the remaining gaps and why they matter
- the recommendation doc says what `codexw` should optimize for right now
- the support-boundary doc says what is supported versus explicitly
  unsupported today

Future edits should keep those three documents aligned rather than treating
this assessment alone as the whole product decision.

## Highest-Leverage Next Work

The next high-value tasks in this area are:

1. Keep the recommendation and support-boundary docs aligned with the actual
   product and backlog state.
2. Keep realtime/audio scope explicit, so future readers do not assume audio
   parity is merely unfinished implementation work.
3. Keep the wrapper-owned async shell boundary explicit whenever orchestration
   or local-API work expands.
4. Reopen alternate-screen or audio work only if a concrete workflow need, not
   generic parity pressure, makes it the best next investment.

## What Would Reopen These Decisions

The current recommendations should be revisited if any of the following become
true:

- a concrete user workflow is blocked by the scrollback-first model
- broker or local-API consumers require stronger viewport/layout semantics
- upstream app-server exposes public process/session control that materially
  reduces the async execution gap
- a clear terminal-compatible audio UX target appears and is worth supporting

Until then, these areas should be treated as explicit product boundaries and
architecture choices, not as quietly unfinished parity debt.
