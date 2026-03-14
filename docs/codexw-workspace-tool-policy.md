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

`codexw` should not advertise read-only workspace tools on new threads by
default, and it should not treat dynamic tools as the default answer for
filesystem access.

The intended split is:

- shell or Python for open-ended, repo-specific, or multi-step inspection
- wrapper-owned background-shell tools for async control with real product
  value
- any retained workspace helpers treated as optional fallback code, not as a
  primary model-facing surface

This means the product is now intentionally shell-first for workspace
inspection, while staying tool-first only for the wrapper-owned orchestration
and background-shell surfaces.

## Why Dynamic Workspace Tools Exist

The repo still contains read-only workspace helper code, but the model no
longer needs that surface advertised by default.

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

The older workspace tool layer was able to give the model stable parameter
shapes such as:

- `path`
- `query`
- `limit`
- `startLine`
- `endLine`

That is materially easier for the model than generating shell syntax, handling
quoting, and then parsing arbitrary stdout.

The advertised tool descriptions should keep that same bounded read-only
framing. They should not describe the workspace tools like a second
general-purpose automation surface.

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

## Current Recommendation

The current recommendation is simpler than the earlier mixed model:

- do not advertise the workspace dynamic tools on new threads
- let the model use host shell and Python for repo inspection
- only keep the workspace helper implementation around as transitional or
  fallback code until the repo decides whether to delete it entirely

## Standard For Adding New Workspace Tools

Add a new read-only workspace tool only if it clearly beats shell execution on
at least two of these dimensions and the repo is willing to advertise it by
default:

- stronger safety boundary
- substantially better determinism for the model
- meaningfully lower token/latency cost
- portable cross-platform behavior that would otherwise be fragile
- repeated high-frequency use that justifies dedicated code

Do not add a tool just because shell could also do it "with more steps."

That standard is intentionally strict. The maintenance burden is real.

## Standard For Removing Workspace Tools

Remove, de-advertise, or deprecate a workspace tool if one or more of these
becomes true:

- it is rarely used compared with shell alternatives
- its behavior is too trivial to justify dedicated maintenance
- it keeps creating reliability problems disproportionate to its value
- its semantics drift too close to open-ended shell execution

The recent `workspace_list_dir` hang on a large directory is a concrete example
of the maintenance risk. Even a simple read-only convenience tool can become a
real reliability problem if it eagerly processes large trees instead of
respecting bounded output.

That is why `workspace_list_dir` should be treated as a quick directory peek,
not as an exact large-directory enumeration primitive. If the model needs an
exact full listing, exact omitted counts, or project-specific filtering over a
huge directory, shell tools are the better fit.

That bug was enough to justify de-advertising the workspace tool surface. It
also means the repo should be highly selective about adding more tools of the
same class.

## Product Posture

The intended product posture is:

- shell is the general-purpose execution substrate for workspace inspection
- workspace dynamic tools are no longer advertised by default
- already-running older threads may still trigger retained workspace helpers,
  and `codexw` should label that as a legacy compatibility path in operator
  stderr instead of making it look like normal current-state tool usage
- wrapper-owned background shell tools exist because async shell control has
  product value and app-server does not expose equivalent public control of
  model-owned command sessions

That keeps the tool surface coherent:

- any retained workspace helpers as fallback code rather than the primary
  model-facing path
- background-shell tools for wrapper-owned async control
- shell for everything open-ended

## Practical Guidance

Use this policy when making any of these decisions:

- whether a new filesystem helper should be a dynamic tool or a shell workflow
- whether an existing workspace tool should remain supported
- whether a retained workspace helper should stay hidden or be deleted
- whether a bug in a convenience tool should be fixed or should trigger
  deprecation instead
- whether the model should be nudged toward shell-first behavior for a given
  task class

The default answer should be conservative: prefer fewer workspace tools with
clear value over a growing pile of small convenience wrappers.
