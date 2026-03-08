---
name: session-autopilot
description: Continue a coding or repo-work session across turns until the concrete project goal is finished. Use when Codex should keep working without waiting for fresh user prompts, re-scan TODO or plan documents, merge newly discovered tasks with existing work, reweight by leverage and risk, and end each turn with an explicit AUTO_MODE_NEXT stop or continue marker for an external runtime such as codexw.
---

# Session Autopilot

Continue the current project deliberately, not blindly. This skill is for cooperative autopilot: another runtime detects turn boundaries and decides whether to submit the next prompt, while this skill decides how to evaluate remaining work and how to end each turn.

## Core Rules

Follow this priority order every turn:

1. Execute the most recent explicit user request first.
2. Otherwise inspect concrete remaining work from:
   - TODO or plan documents in the repo
   - failing verification, build, or test output
   - incomplete tasks already identified in the session
   - newly discovered tasks implied by the current work
3. Reweight tasks by leverage, dependency order, and risk.
4. Execute the single best next task or a tight batch of related tasks.

Do not invent speculative work. Continue only from explicit requirements, concrete failures, or directly implied follow-up tasks.

## Execution Style

- Prefer fundamental fixes over ad-hoc patches.
- Keep documentation and implementation in sync.
- When behavior or workflows change, update the relevant docs in the same turn.
- Avoid repeatedly partial work on the same theme across many turns. If several adjacent cleanup or refactor steps are obvious and safe, batch them into one meaningful slice.
- Do not stop mid-refactor or mid-cleanup just because one local change landed. If the same turn can safely finish the surrounding rewires, deletions, docs sync, and verification, do that before ending.
- Run appropriate verification before ending the turn.
- If code or docs changed, summarize `git diff --stat`, then commit and push when permitted.
- Ask a clarifying question only when the choice materially affects correctness, security, destructive actions, or long-term architecture.

## End-Of-Turn Workflow

Before ending a turn:

1. Re-scan the workspace for concrete remaining work.
2. Decide whether the project goal is actually complete.
3. If work remains, identify the highest-leverage next task.
4. End the final response with exactly one line:

`AUTO_MODE_NEXT=continue`

or

`AUTO_MODE_NEXT=stop`

Use `AUTO_MODE_NEXT=stop` only when both are true:

- the session objective is achieved
- no concrete task remains

If there is uncertainty, unfinished verification, an open TODO, a documented next step, or a concrete follow-up that increases leverage, default to `continue`.

## Continuation Prompt Expectations

When a runtime resumes the session, expect the next prompt to include:

- the session objective
- the latest assistant response
- instructions to continue without waiting for user input

Treat that latest assistant response as state to build from. Do not repeat the same summary unless it is necessary to ground the next action.

## Example Trigger Phrases

This skill should trigger for requests such as:

- "continue this session until no task remains"
- "autopilot the repo"
- "keep working turn by turn"
- "reweight TODOs and continue"
- "auto continue after each turn"
