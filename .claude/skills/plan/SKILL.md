---
name: plan
description: Design-thinking + survey + worklist-management skill. Use when the user asks to scope a new epic, run an N-Q AskUserQuestion survey, audit the worklist for drift, rescue dead modules, lift design-doc actions into worklist tasks, draft a design document, or otherwise PLAN work before any code lands. Absorbs the prior `mackes-worklist-management` + design-thinking portions of `iteration` (worklist rescue pass). Sister skills (Q87 lock 2026-05-25): `ship` (drains the queue), `release` (cut/push/tag flow).
---

# Plan

The design + worklist-management skill. **Everything before code
lands runs through plan.** Surveys design forks via
`AskUserQuestion`, lifts design-doc actions into worklist tasks,
audits the worklist for drift, drafts new design documents.

Consolidates the planning portions of the retired
`mackes-worklist-management` + `iteration` (rescue-pass) skills per
Q87 of the 100-Q tightening survey 2026-05-25.

## Triggers

- "Design [X]" / "Survey [X]" / "Lock [X]"
- "Audit the worklist" / "Rescue the worklist" / "Find dead modules"
- "Lift the design doc actions into the worklist"
- "Run an N-Q survey on [X]" / "Fire 25 questions about [X]"
- "Plan the next [epic]"

## Method

### Survey pattern (≥3-option design forks per Q66)

When the user asks to lock a non-trivial design decision (≥3
plausible options), fire an `AskUserQuestion` survey **one
question at a time** per [[feedback_question_workflow]]. Group
into rounds (e.g., 10 questions per round). After every round,
recap the locks before proceeding.

After all questions:

1. Write `docs/design/<epic>.md` capturing every lock in a table
   + the resulting architecture + acceptance criteria + risks +
   out-of-scope items.
2. Lift every actionable item into `docs/PROJECT_WORKLIST.md` as
   a new `### EPIC-NAME` section with user-story tasks
   (As/I want/so that + bench-observable acceptance bullets per
   [[feedback_no_stubs]] + §0.12).
3. Update `docs/AI_GOVERNANCE.md` if the survey locks
   platform-wide direction (not just per-epic).
4. Commit + dual-remote push per §0.2 + standing auth.

### Worklist rescue pass

Before any large ship effort, scan for:

- **Dead modules** — `pub mod foo;` in Rust with zero external
  references; Python `mackes/<x>.py` with zero external imports.
- **Misleading `[✓]` marks** — tasks marked done where the
  runtime-reachability gate (§0.8 #7) doesn't actually hold.
- **Mockup-only features** — UI that renders but the underlying
  state never updates.
- **Deferred markers** — code or worklist text saying "lands in
  a follow-up", "wired in Phase N", "deferred to", "stub for now",
  "todo!()", "unimplemented!()".
- **Design-doc actions never lifted** — items in
  `docs/design/*.md` that don't have matching worklist entries.

Each finding becomes a new worklist task (user-story shape) BEFORE
any new code lands. The §0.12 no-stubs rule + the §0.8 runtime-
reachability gate are upstream prevention; the rescue pass is the
downstream catch.

### Authority

When two locks contradict (per CLAUDE.md §0.14):
1. **Memory** (`~/.claude/projects/.../memory/*.md`) — operator
   live preferences, highest.
2. **`.claude/CLAUDE.md`** — operational rulebook.
3. **`docs/AI_GOVERNANCE.md`** — platform identity + compass.
4. **`docs/design/<epic>.md`** — per-epic locks.
5. **`docs/PROJECT_WORKLIST.md`** body — actionable state.

Newest wins. When in doubt: §0 master rule from `AI_GOVERNANCE.md`
("Secure, Simple, Centerless Workgroup").

## Worklist schema (inherited from retired mackes-worklist-management)

```
- [ ] **<PREFIX>-N.M: <release> — <short title>** *(optional carve-out tag)*
  **As** <role>,
  **I want** <capability>,
  **so that** <outcome>.
  **Acceptance** (each bench-observable):
    - [ ] specific bench-observable bullet
    - [ ] specific bench-observable bullet
```

Status legend (per Q86): `[ ] Open`, `[>] session=<id>`,
`[✓] Done`, `[!] Blocked`. `[~] Deferred` is RETIRED — no silent
deferrals (operator directive 2026-05-19).

Every task carries the WF-5 release-tag prefix; per Q78, prefixes
migrate to `EPIC-001..NNN` numbering with `tag:` field.

## Companion skills

- `ship` — when planning is done, switch to ship to drain the
  queue
- `release` — when ship is done + HW bench green, switch to
  release to cut

## Retired ancestors

- `mackes-worklist-management` (the worklist schema part — now
  embedded here)
- `iteration` (the rescue-pass + N-Q survey parts — now embedded
  here; the LOOP part moved to `ship`)
- `autonomous-worker` (the planning part — embedded here)
- `complete-remaining-work` (the prioritization rules — moved
  to `ship`)
