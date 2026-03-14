# codexw Native Product Status

This document is the concise current-status snapshot for the non-broker side of
`codexw`.

It answers a narrower question than the larger design docs:

- what is the currently supported native-side product shape?
- what has already been decided?
- what concrete work is still left?

Related docs:

- [codexw-native-gap-assessment.md](codexw-native-gap-assessment.md)
- [codexw-native-product-recommendation.md](codexw-native-product-recommendation.md)
- [codexw-native-support-policy.md](codexw-native-support-policy.md)
- [codexw-native-support-boundaries.md](codexw-native-support-boundaries.md)
- [codexw-native-proof-matrix.md](codexw-native-proof-matrix.md)
- [codexw-native-hardening-catalog.md](codexw-native-hardening-catalog.md)
- [codexw-self-evolution.md](codexw-self-evolution.md)
- [codexw-self-supervision.md](codexw-self-supervision.md)
- [codexw-plugin-system.md](codexw-plugin-system.md)
- [../TODOS.md](../TODOS.md)
- [codexw-support-claim-checklist.md](codexw-support-claim-checklist.md)

## Current Supported Shape

The current non-broker product shape is:

- terminal-first
- scrollback-first
- inline prompt plus transient status
- text-first realtime
- wrapper-owned async shell orchestration

Those are not fallback behaviors. They are the supported shape of the product
today.

## What Is Already Decided

The repo has already made these native-side decisions explicit:

1. `codexw` is not currently optimizing for full alternate-screen parity with
   the upstream Codex TUI.
2. `codexw` is not currently optimizing for upstream-style audio parity.
3. `codexw` is not currently treating backend-owned async execution parity as a
   short-term product goal.
4. The wrapper-owned background shell model is the intended supported solution
   within current app-server constraints.

These are product and architecture decisions, not merely missing
implementation work.

## What Is Already True In The Product

The implemented native-side product already has:

- broad command-side parity for the app-server-backed terminal workflow
- a coherent scrollback transcript model
- a wrapped inline prompt/editor with transient status
- text-oriented realtime state and semantic event reporting
- wrapper-owned background shells with orchestration visibility
- off-thread async shell-tool execution so wrapper-owned shell calls do not
  freeze the prompt/input loop
- prompt/status visibility for in-flight async shell-tool work until the tool
  response completes
- first self-supervision classifications for async shell-tool stalls, currently
  `tool_slow` and `tool_wedged`, surfaced in prompt/status output with narrow
  recommended actions such as `observe_or_interrupt` and
  `interrupt_or_exit_resume`
- orchestration views over agents, shells, services, capabilities, and
  terminals
- a new self-supervision design lane for stalled tool/runtime recovery
- a new plugin-first expansion lane for optional capabilities

That means most native-side remaining work is no longer “missing command
handlers.” It is about explicit product boundaries and architecture choices.

## Remaining Active Work

The highest-leverage remaining native-side work is:

1. keep the native recommendation, support boundary, and backlog wording
   aligned
2. keep native unsupported areas explicit so they do not drift back into vague
   “unfinished parity” language
3. only reopen alternate-screen or audio work if a concrete workflow need
   appears
4. keep the wrapper-owned async shell boundary explicit as orchestration and
   local-API surfaces evolve
5. turn self-supervision into a real native runtime capability so wedged tool
   paths do not leave the operator trapped in an old client generation
6. prefer plugin-first expansion for optional capabilities such as voice
   reminder or live IM progress reporting, reserving full self-evolution for
   core runtime or protocol changes

## Remaining Gaps, Classified

### Architectural gaps

These remain true, but they are not currently active implementation targets:

- alternate-screen / widget-tree parity
- audio-oriented realtime parity
- backend-owned async execution/session parity

### Documentation/backlog work

This is still active:

- keeping the docs, backlog, and future support claims internally consistent
- preventing stale “prototype” or “missing parity” wording from returning

### Reopen conditions

These areas should only move back into active implementation backlog when:

- a concrete workflow is blocked by the current scrollback-first model
- a concrete terminal-compatible audio target is chosen
- app-server exposes materially stronger public execution/session control
- remote consumers require stronger layout semantics than the current product
  can support coherently

## Current Recommendation In One Sentence

The current native-side recommendation is:

- keep making `codexw` a stronger terminal-first engineering client
- not a partial rewrite in anticipation of upstream native parity

## How To Use This Document

Use this file when you need the short answer to:

- what is the current native-side product shape?
- what is still left?
- what is intentionally unsupported?

Use the deeper docs when you need detail:

- gap analysis: [codexw-native-gap-assessment.md](codexw-native-gap-assessment.md)
- product recommendation: [codexw-native-product-recommendation.md](codexw-native-product-recommendation.md)
- support boundary: [codexw-native-support-boundaries.md](codexw-native-support-boundaries.md)
- evidence mapping: [codexw-native-proof-matrix.md](codexw-native-proof-matrix.md)
