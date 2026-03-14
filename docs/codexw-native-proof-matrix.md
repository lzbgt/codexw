# codexw Native Proof Matrix

This document maps the native-side product recommendation and support boundary
to the current repository evidence.

It is the native analogue to the broker-side
[codexw-broker-proof-matrix.md](codexw-broker-proof-matrix.md).

Related docs:

- [codexw-native-gap-assessment.md](codexw-native-gap-assessment.md)
- [codexw-native-product-recommendation.md](codexw-native-product-recommendation.md)
- [codexw-native-support-policy.md](codexw-native-support-policy.md)
- [codexw-native-support-boundaries.md](codexw-native-support-boundaries.md)
- [codexw-native-product-status.md](codexw-native-product-status.md)
- [codexw-native-hardening-catalog.md](codexw-native-hardening-catalog.md)
- [codexw-design.md](codexw-design.md)
- [codexw-support-claim-checklist.md](codexw-support-claim-checklist.md)

## Reading This Matrix

Each row answers:

- what claim is being made?
- what evidence currently supports it?
- is it already strong enough for current product claims?

The statuses are:

- `strong`: already well supported by implementation plus regression coverage
- `adequate`: enough for the current product claim, but still mostly supported
  by design intent plus indirect evidence
- `boundary`: intentionally unsupported; the proof is explicit wording rather
  than positive implementation

## Matrix

### 1. Terminal-first, scrollback-first product shape

- Status: `strong`
- Claim:
  - `codexw` is a scrollback-first inline terminal client, not an
    alternate-screen widget-tree TUI
- Evidence:
  - [README.md](../README.md)
  - [codexw-design.md](codexw-design.md)
  - prompt and transcript implementation in `wrapper/src/output/`,
    `wrapper/src/render_prompt.rs`, and related renderer/state modules
  - broad runtime/prompt/render regression coverage under `wrapper/src/` test
    modules
- Notes:
  - this is not merely a missing feature; it is the currently supported shape

### 2. Inline prompt plus transient status is the intended UX

- Status: `strong`
- Claim:
  - prompt editing, wrapped inline composition, and transient status are first
    class behavior
- Evidence:
  - [README.md](../README.md)
  - [codexw-design.md](codexw-design.md)
  - `wrapper/src/output/ui.rs`
  - `wrapper/src/render_prompt.rs`
  - prompt/status tests in `wrapper/src/main_test_session_status/` and
    `wrapper/src/render_tests/`

### 3. Text-first realtime is supported

- Status: `strong`
- Claim:
  - realtime in `codexw` is text-oriented, semantic, and observable
- Evidence:
  - [README.md](../README.md)
  - [codexw-design.md](codexw-design.md)
  - [codexw-native-support-boundaries.md](codexw-native-support-boundaries.md)
  - realtime/session status modules in `wrapper/src/`
  - local API semantic event stream and tests
- Notes:
  - the claim is text-first realtime, not audio parity

### 4. Wrapper-owned background shells are the intended async execution model

- Status: `strong`
- Claim:
  - wrapper-owned background shells are the supported solution for same-turn
    async shell execution
- Evidence:
  - [README.md](../README.md)
  - [codexw-design.md](codexw-design.md)
  - orchestration/background shell implementation in `wrapper/src/background_shells/`
  - dynamic-tool, `:ps`, local-API, connector, and fixture coverage already in
    repo
- Notes:
  - the evidence is implementation-heavy, not only design prose

### 5. Alternate-screen parity is intentionally unsupported today

- Status: `boundary`
- Claim:
  - full alternate-screen/native widget-tree parity is not part of the current
    supported product boundary
- Evidence:
  - [codexw-native-product-recommendation.md](codexw-native-product-recommendation.md)
  - [codexw-native-support-boundaries.md](codexw-native-support-boundaries.md)
  - [codexw-native-gap-assessment.md](codexw-native-gap-assessment.md)
  - [codexw-native-product-status.md](codexw-native-product-status.md)
- Notes:
  - this is an explicit product boundary, not a missing proof item

### 6. Audio parity is intentionally unsupported today

- Status: `boundary`
- Claim:
  - upstream-style audio UX is out of scope until a concrete supported target
    exists
- Evidence:
  - [codexw-native-product-recommendation.md](codexw-native-product-recommendation.md)
  - [codexw-native-support-boundaries.md](codexw-native-support-boundaries.md)
  - [codexw-native-gap-assessment.md](codexw-native-gap-assessment.md)
- Notes:
  - the correct proof here is explicit policy language, not missing code

### 7. Backend-owned async execution parity is intentionally unsupported today

- Status: `boundary`
- Claim:
  - `codexw` is not claiming native backend-owned command-session parity under
    current app-server constraints
- Evidence:
  - [codexw-native-gap-assessment.md](codexw-native-gap-assessment.md)
  - [codexw-native-product-recommendation.md](codexw-native-product-recommendation.md)
  - [codexw-native-support-boundaries.md](codexw-native-support-boundaries.md)
  - current implemented alternative in `wrapper/src/background_shells/`
- Notes:
  - the repo proves the alternative model strongly and names the unsupported
    native parity explicitly

### 8. Native-side remaining work is mostly documentation and boundary hygiene

- Status: `adequate`
- Claim:
  - the highest-leverage remaining native-side work is keeping the product
    recommendation and support boundary coherent, not implementing large hidden
    feature gaps
- Evidence:
  - [../TODOS.md](../TODOS.md)
  - [codexw-native-gap-assessment.md](codexw-native-gap-assessment.md)
  - [codexw-native-product-status.md](codexw-native-product-status.md)
- Notes:
  - this is backed by both the implementation state and the explicit backlog,
    but it is still more of a product-governance claim than a runtime behavior
    claim

### 9. Native support-claim docs are kept in sync automatically

- Status: `strong`
- Claim:
  - the native-side status/policy/proof source docs are guarded against the
    most important wording drift
- Evidence:
  - [wrapper/tests/doc_consistency.rs](../wrapper/tests/doc_consistency.rs)
  - [codexw-support-claim-checklist.md](codexw-support-claim-checklist.md)
- Notes:
  - this does not replace human review, but it does turn the highest-signal
    support-claim invariants into a regression-proof surface

### 10. Async shell-tool supervision classifications are now operator-visible

- Status: `strong`
- Claim:
  - long-running async shell-tool work is not only non-blocking, but also
    classified in operator-visible prompt/runtime surfaces with labels such as
    `tool_slow` and `tool_wedged`, plus narrow recommended actions such as
    `observe_or_interrupt` and `interrupt_or_exit_resume`, plus a sticky
    `supervision_notice` alert state while the issue remains active
- Evidence:
  - [codexw-self-supervision.md](codexw-self-supervision.md)
  - [codexw-native-product-status.md](codexw-native-product-status.md)
  - [codexw-native-support-policy.md](codexw-native-support-policy.md)
  - `wrapper/src/state/types.rs`
  - `wrapper/src/session_prompt_status_active.rs`
  - `wrapper/src/session_snapshot_runtime.rs`
  - regression coverage in `wrapper/src/main_test_session_status/`
- Notes:
  - this is the first concrete emitted classification slice, not yet the full
    recovery-policy system described by the self-supervision plan

## What Would Upgrade Or Change This Matrix

The matrix should change if any of these happen:

- `codexw` adds an optional or default alternate-screen mode
- `codexw` chooses and implements a supported audio target
- app-server exposes a public execution/session control surface that materially
  changes async execution parity
- the repo backlog intentionally promotes one of the current unsupported areas
  into active implementation work

## Practical Use

Use this file when someone asks:

- what native-side product claims are already well supported?
- what is explicitly unsupported versus merely unbuilt?
- is there evidence for the current native recommendation, or only prose?

Use the linked recommendation/support docs for the product decision itself, and
this matrix for the evidence map.
