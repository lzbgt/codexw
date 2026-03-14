# codexw Workspace Tool Policy

This document defines when `codexw` should expose read-only workspace dynamic
tools to the model, and when shell or Python execution should remain the
preferred path.

It exists because the repo now has both:

- generic execution paths such as shell commands and Python
- wrapper-owned read-only workspace tools such as `workspace_list_dir`,
  `workspace_stat_path`, `workspace_read_file`, `workspace_find_files`, and
  `workspace_search_text`

Those surfaces overlap in capability, but they do not serve the same product
goal.

Related docs:

- [codexw-design.md](codexw-design.md)
- [codexw-native-gap-assessment.md](codexw-native-gap-assessment.md)
- [../TODOS.md](../TODOS.md)

## Core Decision

`codexw` should keep a small read-only workspace tool surface, but it should
not treat dynamic tools as the default answer for all filesystem access.

The intended split is:

- dynamic workspace tools for narrow, high-frequency, low-side-effect
  inspection
- shell or Python for open-ended, repo-specific, or multi-step workflows

This means the product is not "tool-first everywhere" and it is not
"shell-first everywhere." It is a mixed model with an explicit boundary.

## Why Dynamic Workspace Tools Exist

Read-only workspace tools still provide real product value even though shell
and Python can often do similar work.

### Safety and Policy

The workspace tools are intentionally narrower than shell execution:

- they are read-only
- they resolve paths against the current workspace root
- they reject paths outside that workspace
- they return one predictable text response instead of arbitrary subprocess
  output

That makes them safer and easier to reason about than letting the model invent
filesystem shell commands every time it wants a quick answer.

### Structured Model Interface

The workspace tool layer in
[wrapper/src/client_dynamic_tools/specs/workspace.rs](../wrapper/src/client_dynamic_tools/specs/workspace.rs)
gives the model stable parameter shapes such as:

- `path`
- `query`
- `limit`
- `startLine`
- `endLine`

That is materially easier for the model than generating shell syntax, handling
quoting, and then parsing arbitrary stdout.

### Portability and Determinism

The dynamic tools behave the same way regardless of local shell flavor or
installed utilities. A tool call does not care whether `rg`, `find`, Python,
or GNU/BSD flags differ across environments.

That makes them a better fit for simple inspection paths that should behave
consistently across hosts.

### Cost and Latency

For small reads and small searches, direct Rust-side helpers are cheaper than
starting a subprocess and then asking the model to interpret its output.

That matters because these operations are common and repetitive.

## Why Shell or Python Must Still Exist

The workspace tools are intentionally incomplete. They should not expand until
they become a second general-purpose execution system.

Shell or Python remains the right path for:

- complex multi-step repo inspection
- binary-file handling
- non-UTF-8 content
- project-specific build, lint, and test workflows
- custom filtering or aggregation logic
- large-scale scripted transforms
- situations where the model genuinely needs a programming environment, not a
  fixed schema

Trying to encode all of that into more dynamic tools would duplicate shell
execution badly and create an unbounded maintenance surface.

## Current Recommendation By Tool

### Strong Keep

These tools justify their maintenance cost and should remain part of the model
surface:

- `workspace_read_file`
- `workspace_find_files`
- `workspace_search_text`

They cover common inspection needs with clear structure and low ambiguity.

### Convenience Keep, Not Strategic

These tools are useful, but they are not as essential:

- `workspace_list_dir`
- `workspace_stat_path`

They are worth keeping while they remain cheap and reliable, but they should
not be treated as architectural pillars.

If tool-surface simplification becomes necessary, these are the first
deprecation candidates.

## Standard For Adding New Workspace Tools

Add a new read-only workspace tool only if it clearly beats shell execution on
at least two of these dimensions:

- stronger safety boundary
- substantially better determinism for the model
- meaningfully lower token/latency cost
- portable cross-platform behavior that would otherwise be fragile
- repeated high-frequency use that justifies dedicated code

Do not add a tool just because shell could also do it "with more steps."

That standard is intentionally strict. The maintenance burden is real.

## Standard For Removing Workspace Tools

Remove or deprecate a workspace tool if one or more of these becomes true:

- it is rarely used compared with shell alternatives
- its behavior is too trivial to justify dedicated maintenance
- it keeps creating reliability problems disproportionate to its value
- its semantics drift too close to open-ended shell execution

The recent `workspace_list_dir` hang on a large directory is a concrete example
of the maintenance risk. Even a simple read-only convenience tool can become a
real reliability problem if it eagerly processes large trees instead of
respecting bounded output.

That bug does not mean the tool must be removed. It does mean the repo should
be selective about adding more tools of the same class.

## Product Posture

The intended product posture is:

- shell is the general-purpose execution substrate
- dynamic workspace tools are the narrow structured fast path
- wrapper-owned background shell tools exist because async shell control has
  product value and app-server does not expose equivalent public control of
  model-owned command sessions

That keeps the tool surface coherent:

- read-only workspace tools for bounded inspection
- background-shell tools for wrapper-owned async control
- shell for everything open-ended

## Practical Guidance

Use this policy when making any of these decisions:

- whether a new filesystem helper should be a dynamic tool or a shell workflow
- whether an existing workspace tool should remain supported
- whether a bug in a convenience tool should be fixed or should trigger
  deprecation instead
- whether the model should be nudged toward shell-first behavior for a given
  task class

The default answer should be conservative: prefer fewer workspace tools with
clear value over a growing pile of small convenience wrappers.
