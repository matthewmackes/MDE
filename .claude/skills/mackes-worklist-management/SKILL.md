---
name: mackes-worklist-management
description: Enforce a single canonical project worklist with strict task schema, status tracking, and restart-safe handoffs. Use when planning, executing, or reporting project work; when adding new ideas, bugs, or features; when coordinating parallel subtasks across AI threads/humans; and when the user asks to apply the Cortex AL worklist rule. Adapted from map2-audio for the mackes-shell project.
---

# Mackes Shell — Worklist Management

Use this skill as the default execution protocol for project task
management unless the user explicitly says `DISABLE WORKLIST RULE`.

## Single Source of Truth

Maintain exactly one authoritative worklist for the project.

- Keep all tasks and subtasks only in the canonical worklist.
- Do not keep side lists, private notes, or alternate trackers for
  project tasks.
- Check and update the canonical worklist before starting substantive
  work.
- If another list exists, merge it into the canonical list and stop
  updating the duplicate.

**The in-session `TaskList` / `TaskCreate` / `TaskUpdate` tools are an
ephemeral conversation-scoped scratchpad — not a worklist.** Use them
freely during execution if they help, but never treat them as a
parallel authoritative tracker. Anything that should survive the
session is written to the canonical file before the session ends.
The file wins on every divergence, and the in-session list is allowed
to be empty.

**Design docs are not a parallel worklist.** Every actionable item
from a locked design under `docs/design/*.md` gets lifted into
`docs/PROJECT_WORKLIST.md` as `[ ] Open`. The design doc keeps
rationale + locks; the worklist keeps the actionable list.

**No silent deferrals (user directive 2026-05-19).** Status is
exactly one of `[ ] Open` · `[>] In Progress` · `[✓] Done` ·
`[!] Blocked`. The `[~] Deferred` status is retired.

**Newer directive wins silently (user directive 2026-05-19).** When
a newer directive contradicts an earlier design-doc lock, update the
affected worklist items in place and move on. Don't maintain a
separate "Conflicts" section, don't ask the user to adjudicate the
older lock. The original design doc keeps its old text as historical
context; the worklist tracks live policy only.

## Canonical Location

Resolve the canonical list location in this order:

1. User-designated location in the current conversation.
2. Pinned/shared project location if explicitly defined.
3. Repository default: `docs/PROJECT_WORKLIST.md`.

Create the default file if missing.

## Required Task Schema

Write every task and subtask with this exact field set:

- `ID`: unique short code (`T001`, `T002-subA`)
- `Status`: exactly one of `[ ] Open`, `[>] In Progress`,
  `[✓] Done`, `[!] Blocked`. The `[~] Deferred` /
  `[~] Cancelled` statuses are retired by the 2026-05-19
  directive. When a newer directive contradicts an earlier lock,
  update the affected items in place — don't track the
  contradiction separately.
- `Title`: one-line outcome statement
- `Description`: include all of:
  - exact goal and acceptance criteria
  - why it matters (brief context)
  - dependencies (task IDs or `None`)
  - estimated effort (`Low`, `Medium`, `High`, or time range)
  - required outputs/deliverables
- `Subtasks` (optional): nested entries using the same schema
- `Assigned to` (optional): AI thread/role/human
- `Last updated`: `YYYY-MM-DD HH:MM - actor`

Use `references/worklist-template.md` as the canonical template.

## Core Workflow Principles

Follow these operating rules for every execution cycle:

1. Decompose aggressively into restartable units (target 15-60 minutes
   each when possible).
2. Prioritize and parallelize independent tasks.
3. Add every new idea/bug/improvement directly to the canonical list.
4. Make tasks atomic and handoff-ready for future AI/humans.
5. After completion, update status, add completion notes, and propose
   next 1-3 logical tasks.

## Required Response Structure

When this rule is active, structure each response as follows:

1. Start by showing the top 5-10 tasks (or the relevant section) with
   statuses.
2. Propose updates/completions/new tasks.
3. Execute or plan the highest-priority feasible work.
4. End by showing the updated list state.

If no canonical list exists yet, create one first, then continue.

## Mackes-Shell-Specific Additions

- **Definition of Done** for a task includes the gates in
  `.claude/CLAUDE.md` §0.8 — code committed to main, pushed to origin,
  RPM builds (when applicable), CHANGELOG updated, all module imports
  clean.
- **Bundle naming**: prefix tasks with the target version `vX.Y.Z:` so
  the worklist self-organizes by release. Example:
  `v1.4.0: session unlock — hold root for the whole session`.
- **Defer the right things**: legacy-panel rewrites and wizard-chrome
  rebuilds are big and should be tracked as separate tasks at the
  v1.4.1 / v1.5.0 layer rather than blocking a release that ships
  other features.

## Strict Prohibitions

- Do not maintain duplicate lists or hidden project-memory notes.
- Do not assume context that is not in the canonical list or recent
  messages.
- Do not execute substantive work before checking/updating the
  canonical list.

## Disable Condition

Treat this rule as permanent for project work unless the user
explicitly says:

`DISABLE WORKLIST RULE`

## Output Quality Bar

- Keep entries concise but explicit enough for cold-start handoff.
- Use deterministic IDs and preserve history when statuses change.
- Favor concrete file paths, commands, and acceptance criteria over
  abstract notes.
