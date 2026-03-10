# codexw Native Support Boundaries

This document describes what the current `codexw` product shape does and does
not support on the non-broker side.

It complements:

- [codexw-native-gap-assessment.md](codexw-native-gap-assessment.md)
- [codexw-native-product-recommendation.md](codexw-native-product-recommendation.md)
- [codexw-native-support-policy.md](codexw-native-support-policy.md)
- [codexw-native-product-status.md](codexw-native-product-status.md)
- [codexw-native-proof-matrix.md](codexw-native-proof-matrix.md)
- [codexw-native-hardening-catalog.md](codexw-native-hardening-catalog.md)
- [codexw-design.md](codexw-design.md)

The purpose of this doc is to stop architectural differences from being treated
as ambiguous “unfinished work.”

## Supported Product Shape

The currently supported non-broker product shape is:

- terminal-first
- scrollback-first
- inline prompt plus transient status
- text-first realtime
- wrapper-owned async shell orchestration

Those are not fallback behaviors. They are the supported shape of the current
product.

## Supported Behaviors

### 1. Scrollback-First Transcript UI

Supported:

- transcript rendered into normal terminal scrollback
- inline prompt/composer
- transient status above the prompt
- terminal-native scroll/history behavior

Not required for support:

- alternate-screen layout parity
- popup-heavy retained widget-tree semantics

### 2. Text-First Realtime

Supported:

- textual realtime state
- semantic event reporting
- text-oriented transcript/status integration

Not currently supported:

- upstream-style audio UX
- media-session semantics
- broad claims of audio parity

### 3. Wrapper-Owned Async Shell Execution

Supported:

- wrapper-owned background shells
- orchestration visibility over those shells
- local API and broker/connector surfaces built around that model

Not currently supported:

- direct public client control of backend-owned model command sessions
- claims of unified-exec parity with native internal Codex behavior

### 4. Explicit Architecture Differences

Supported:

- documenting architectural boundaries plainly
- describing current differences as product choices or backend-surface limits

Not supported:

- describing architectural gaps as though they are already solved
- implying strict native parity where the implementation intentionally differs

## Unsupported Areas

The following are intentionally unsupported today unless the docs say
otherwise:

- full alternate-screen/native widget-tree parity
- upstream audio parity
- backend-owned async execution parity

These may be revisited in the future, but they are not part of the current
supported product boundary.

## What Counts As A Regression

The following are regressions because they violate the current supported shape:

- breaking scrollback-first prompt/transcript behavior
- making textual realtime less coherent or less observable
- breaking wrapper-owned background shell workflows
- making the docs imply stronger native parity than the product actually has

The following are not regressions by themselves:

- absence of alternate-screen parity
- absence of audio parity
- absence of backend-owned process/session reuse parity

Those are known unsupported areas unless the support boundary changes.

## When To Reopen Unsupported Areas

An unsupported area should only move back into active backlog if:

- a concrete user workflow is blocked
- a contradiction appears between support docs and implementation
- a newly exposed backend surface changes the tradeoff materially

Until then, these areas should remain explicit unsupported boundaries, not
quietly reintroduced parity debt.
