---
name: ship
description: Autonomous worklist drain — picks the highest-priority `[ ] Open` task, marks `[>] session=<id>`, implements fully (no stubs per §0.12), runs pre-commit gates (§0.7), commits + dual-remote pushes, then moves to the next bundle without confirmation. Absorbs the retired `autonomous-worker` + `complete-remaining-work` + `iteration` (loop part) + `batch` skills per Q87 (2026-05-25). Use when the user says "ship", "execute", "complete remaining work", "iterate", "continue autonomously", "drain the worklist", "keep going until done", "ship the [epic]". Sister skills: `plan` (design + survey first), `release` (cut/push/tag when 1.0 ready).
---

# Ship

The execution skill. Treats the canonical worklist as the only
source of truth, drains it in priority order with parallel
bundles, dual-remote pushes after each bundle, and continues
without confirmation unless blocked.

Consolidates the retired `autonomous-worker`, `complete-remaining-
work`, `batch`, and the loop-portion of `iteration` per Q87 of the
100-Q tightening survey 2026-05-25.

## Triggers

- "Ship", "execute", "continue", "iterate"
- "Complete remaining work", "drain the worklist"
- "Keep going until done", "autonomous mode", "ship the worklist"
- "Ship BUS-1", "ship DEAD-2", "ship [epic name]" — scoped to
  one epic instead of fleet-wide

## Workflow

1. **Read canonical worklist** (`docs/PROJECT_WORKLIST.md`).
   Identify the highest-priority `[ ] Open` tasks under the
   targeted scope (the whole worklist, or the epic the user
   named).
2. **Split into bundles** that can ship in parallel: independent
   files, no overlapping pre-commit gate runs, no cross-bundle
   blocking deps.
3. **Mark `[>] session=<id>`** on the selected tasks before any
   substantive edit (Q86 lock).
4. **Implement fully** per §0.12 — no stubs, no `todo!()`, no
   "phase 2 lands later", no `pub mod foo;` with zero external
   refs. Every commit ships complete; if it can't, split at
   write-time.
5. **Pre-commit gates** per §0.7 (9 gates total: module-smoke,
   tests, ruff, RPM, CSS lint, voice lint, legacy-mesh lint,
   D-Bus shape lint, Material Symbols lint — only the
   applicable ones for the bundle's touched files).
6. **Pre-staged check** per [[feedback_check_pre_staged]] —
   `git status --short`; if files modified outside the bundle
   scope, **don't bundle them**. Stage only the bundle's files.
7. **Commit + dual-remote push** per §0.2 (HEREDOC message
   format + co-attribution per Q85 with exact model identifier;
   `git push origin main && git push mde-x main`).
8. **Update statuses** to `[✓] Done` or `[!] Blocked` with
   concrete notes.
9. **Add new tasks** for debt, deferrals, or follow-ons
   surfaced during implementation.
10. **Continue to next bundle** without confirmation unless
    blocked (missing facts, required approvals, destructive
    actions outside §0.9 standing auth).

## Parallel-bundle standard

Per the retired `complete-remaining-work` skill:
- Prefer independent file reads + searches + validations in
  parallel
- Batch related edits that share context + test surfaces
- Keep dependency chains sequential only where required by
  correctness

Per the retired `batch` skill:
- A user-named scope ("ship all UX work", "ship NF-2.x") filters
  the queue to matching tasks; ship them as commit-sized bundles

## Coordination with parallel sessions

Per Q70 + Q86: the worklist `[>] session=<id>` marker is the only
coordination primitive. Before claiming a task, scan for existing
`[>]` markers + skip those. If `git status` shows untracked
work in a crate, another session is likely active there; pick
non-colliding bundles.

## Standing authorizations (per Q83)

- Commit + push (no per-op confirmation)
- `make rpm` builds
- The §0.6 `cut release X.Y.Z` 7-step shorthand once operator
  types the trigger (but `cut release` itself stays operator-
  typed — never autonomously decide to cut)
- Best-choice decisions when no design lock covers the case
  + the option fits the §0 master rule from `AI_GOVERNANCE.md`
- Scope/design improvements that align with locked direction
- Adding new worklist tasks for debt or follow-ons

## When to stop the loop

- Blocked on missing facts the operator must provide
- Blocked on required approval (destructive ops outside §0.9
  standing auth)
- Worklist drained to "only HW carve-outs remain" — call
  `release` if 1.0 is ready, else report status
- Operator interrupts

## Companion skills

- `plan` — when a fork emerges, switch to `plan` for survey
- `release` — when 1.0 scope green + HW bench done, switch to
  `release` for the cut

## Retired ancestors

- `autonomous-worker` (single-task version — absorbed)
- `complete-remaining-work` (parallelization policy — absorbed)
- `iteration` (loop part — absorbed; rescue-pass part moved to
  `plan`)
- `batch` (named-scope filtering — absorbed)
