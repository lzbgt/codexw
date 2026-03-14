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
  and `tool_wedged` for long-running shell-tool work, with narrow recommended
  actions such as `observe_or_interrupt` and `interrupt_or_exit_resume`
- a sticky supervision-notice alert state for active async shell-tool stalls
- machine-readable recovery-policy decisions for those alerts, currently
  `warn_only` versus `operator_interrupt_or_exit_resume`
- explicit recovery options such as `observe_status`, `interrupt_turn`, and
  `exit_and_resume`
- explicit terminal-facing supervision output for those same options, so the
  operator can see concrete `:status`, `:interrupt`, or resume-next-step
  guidance without opening the local API payloads
- compact prompt-line supervision options for those same next steps, so the
  active spinner itself can point at `:status`, `:interrupt`, or `resume`
  without waiting for a separate status command
- compact backlog-only prompt/status guidance for abandoned async work, so the
  operator still sees those same next steps after the active worker has timed
  out into `async_tool_backpressure`
- a runtime-enforced local failure path for overdue async shell-tool calls, so
  supported behavior does not include waiting forever for a wedged tool worker
- dedicated wrapper worker threads for background-shell dynamic tools
- explicit abandoned async backlog visibility through `async_tool_backpressure`
- explicit dedicated worker inspection visibility through `async_tool_workers`,
  including lifecycle states such as `running` and
  `abandoned_after_timeout`
- explicit owner-lane visibility for that async work, currently
  `wrapper_background_shell`
- explicit top-level active-worker identity on `async_tool_supervision` and
  `supervision_notice` through `request_id` and `thread_name`
- explicit owner/correlation/inspection visibility on `supervision_notice`
  itself through fields such as `owner`, `source_call_id`,
  `target_background_shell_reference`, `observation_state`, and
  `observed_background_shell_job`
- a human-readable `:status` supervision block that surfaces that same sticky
  alert identity and inspection context instead of leaving it only in
  structured local-API payloads or live stderr notices
- orchestrator-owned periodic async-worker inspection notices that keep the
  concrete tool summary or shell command visible and say when no
  completion/output has been observed yet
- structured async-worker inspection fields that expose the current
  observation state and the orchestrator's next planned health check horizon
- explicit async-worker output freshness through `output_state` and
  `last_output_age_seconds` when a correlated wrapper shell job has emitted
  output
- explicit oldest-backlog observation/output/job visibility through
  `async_tool_backpressure`, including `oldest_observation_state`,
  `oldest_output_state`, and `oldest_observed_background_shell_job`
- explicit backlog `recovery_options` on `async_tool_backpressure`, so
  timeout/backpressure status keeps the same `observe_status`,
  `interrupt_turn`, and `exit_and_resume` next steps as active supervision
- explicit backlog `recommended_action` and `recovery_policy`, so supported
  clients can tell when backlog state has crossed from warn-only monitoring
  into operator-action-required saturation
- live self-supervision inspection notices that include observation/output
  state, source call id when present, and next-check timing in the terminal
  stream
- correlated wrapper-shell inspection facts, so a supervised
  `background_shell_start` can surface the matched `bg-*` job id, job status,
  command, and recent output preview when those facts exist
- the same correlated wrapper-shell inspection for async shell tools that
  reuse an existing shell by `jobId|alias|@capability`, so
  `background_shell_wait_ready`, `background_shell_poll`,
  `background_shell_send`, `background_shell_attach`, and
  `background_shell_invoke_recipe` can surface the matched `bg-*` job instead
  of remaining in generic unresolved-worker state
- that reuse-target correlation is part of the machine-readable support
  surface through `target_background_shell_reference` and
  `target_background_shell_job_id`
- abandoned async backlog summaries also retain that same correlation through
  `oldest_source_call_id`, `oldest_target_background_shell_reference`, and
  `oldest_target_background_shell_job_id`
- abandoned async worker rows also retain `observation_state`,
  `output_state`, and correlated `bg-*` job facts when the shell is still
  visible after timeout
- local refusal of new background-shell async requests when that backlog is
  saturated
- machine-readable saturation refusal results through
  `failure_kind=async_tool_backpressure` and a structured `backpressure`
  object, including explicit backlog `recommended_action`, `recovery_policy`,
  and `recovery_options`, so callers do not have to parse prose to understand
  the blocked oldest worker or next step
- single-pass resume-history hydration for resumed-thread state seeding and
  latest-message preview extraction
- a local recent-thread cache that can render last-known numbered sessions
  before live `thread/list` refresh completes
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
