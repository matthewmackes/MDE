---
name: batch
description: "[RETIRED 2026-05-25 per Q87] Use `ship \"<scope>\"` instead — `ship` absorbs the batch-by-scope filter. This skill body is retained for slash-name back-compat."
---

> **RETIRED 2026-05-25 by Q87 of the 100-Q tightening survey.**
> Skill catalog consolidated to 3 (`plan` / `ship` / `release`).
> Use **`ship "<scope>"`** instead — `ship` absorbs the
> batch-by-scope filter (`ship "all UX work"`, `ship "NF-2.x"`).
> This body is retained for slash-name back-compat.

# Batch

A focused-fan variant of [[iteration]]. Where `iteration` grinds
the whole worklist until only the Hardware Testing epic remains,
`batch` packages a *category* of related tasks into the largest
sensible commit-sized bundles + ships them all in one push.

The skill exists because the worklist often clusters by area —
front-end (panels + applets + popovers), Python helpers
(mesh_*.py + birthright + presets), CA / cert lifecycle, wizard
pages, docs + tests — and the operator wants to point at a
category rather than name each task individually.

## Triggers

- `/batch` — no scope. Batches every remaining non-hardware,
  non-release-cut, non-upstream-blocked task into bundles
  sized by dep clusters.
- `/batch "<scope>"` — scope-filtered. Common scope strings:
  - `"all front end work"` → NF-10.x..NF-18.x (panels,
    applets, peer-card, file manager, notifications, firewall,
    backup runbook)
  - `"all CA work"` → NF-2.x (mint + sign + seal + bundle +
    epoch rotation + CLI)
  - `"all wizard work"` → NF-7.x + NF-14.x (mesh-init + enroll +
    legacy wizard-page retirement)
  - `"all docs work"` → NF-15.x + the docs/help/ tree
  - `"all service publishing"` → NF-13.x
- Free-form phrases like "batch up the front-end stuff" / "do
  all the wizard tasks in one shot" / "knock out the doc rewrite
  bundle" are also valid triggers.

## How `batch` differs from `iteration`

| Dimension | `iteration` | `batch` |
|-----------|-------------|---------|
| Scope     | whole worklist | one category, one push |
| Rescue pass | mandatory at start | only when the scope mentions audit |
| Pacing    | infinite loop until exit cond | exits after the scope's bundles ship |
| Commit cadence | one per logical unit | bundles by dep cluster — fewer, larger commits |
| Subagents | parallel for independent bundles | encouraged when the scope splits cleanly by crate / directory |

## Execution pipeline

### Phase A — resolve scope

If `/batch` was called bare, scope = every non-skip item.
If a scope string was supplied:

1. Tokenize the string. Common synonyms:
   - "front end" / "frontend" / "GUI" / "UI" → NF-10..NF-18 +
     any Workbench panel or Iced popover tasks
   - "wizard" → NF-7.x + NF-14.x
   - "CA" / "cert" / "PKI" → NF-2.x + cert-renewal hooks
   - "docs" / "help" / "documentation" → NF-15.x + the
     docs/help/ tree
   - "transport" / "router" → NF-4.x + NF-8.x
   - "panel" / "applet" → NF-10.x + crates/mde-applets/
   - "service" / "publishing" → NF-13.x
   - "notification" / "toast" → NF-16.x
   - "firewall" / "dbus" → NF-17.x
   - "backup" / "recovery" / "runbook" → NF-18.x
2. Grep `docs/PROJECT_WORKLIST.md` for matching entries
   (section headers + task titles + body lines). Build the
   match set.
3. If the operator wrote a free-form phrase ("all the python
   stuff", "everything that touches the dock"), fall back to a
   liberal grep + present a one-line summary + count
   ("Resolved 14 tasks; proceed?"). If the count is > 30,
   ask before fanning out.

### Phase B — plan bundles

Group the match set into dep-ordered bundles. A bundle is
*one commit* — the unit at which `cargo test` / `pytest` run
+ git commit fires. Aim for bundles of 3-10 tasks, ordered so
each bundle leaves the workspace green.

For NF-10..NF-18 specifically the canonical bundling is:

- **Bundle 0 (foundation, if missing):** the D-Bus / data
  contract every consumer chains on. NF-10..18 chain on
  `mded.Nebula.Status`; ship that first if it doesn't exist
  yet.
- **Bundle 1:** NF-10 panel applets (mesh-status / status-
  cluster / network).
- **Bundle 2:** NF-11 workbench panels (peer-card / topology
  / control / history).
- **Bundle 3:** NF-12 mde-files mesh:// URI + GVFS.
- **Bundle 4:** NF-13 service publishing (Python helpers).
- **Bundle 5:** NF-14 wizard rebuild (Python + Rust mirror).
- **Bundle 6:** NF-15 docs + tests rewrites.
- **Bundle 7:** NF-16 notifications.
- **Bundle 8:** NF-17 firewall + D-Bus adjustments.
- **Bundle 9:** NF-18 backup + recovery + admin runbook.

Bundles within a phase (e.g. 1-3, all independent crates)
are parallelizable via subagents when scope size justifies it.

### Phase C — execute

For each bundle:

1. Read every file the bundle touches (batched).
2. Implement all tasks in the bundle.
3. Run targeted tests: `cargo test -p <crate>` per Rust
   crate touched; `pytest tests/<file>` per Python module
   touched.
4. Commit with HEREDOC body listing every task closed +
   acceptance bench-observable behavior.
5. Push to origin (per standing push authorization, when
   granted).
6. Update worklist entries in place ([ ] → [✓]) inside the
   commit.

Per the §0.12 lock: bundles ship complete. No "wiring lands
in a follow-up" half-bundles. If a task can't fit the
bundle, split it at write-time.

### Phase D — exit

When every task in the resolved scope is `[✓]`:

1. Run the broader workspace test (`cargo test --workspace
   --features async-services`) + `pytest tests/` once at
   the end to catch cross-bundle regressions.
2. Report: status-led summary listing every commit + the
   verification each owned. No marketing copy.
3. Suggest the next-up category if there's an obvious follow-on
   (e.g. "front end shipped → backend pass next?"). The
   operator decides.

If a bundle's verification fails:

1. Diagnose, don't flail. Read the actual error.
2. Fix in the same bundle scope (don't drive-by).
3. Re-run the verification.
4. Only mark `[✓]` when the test is observed-green.

## Standing authorizations (inherited from iteration)

When the operator invokes `/batch` they implicitly grant the
same authorization bundle as `iteration`:

1. Commit when needed (every logical unit becomes a commit).
2. Best-choice decisions on loose spec details, documented in
   the commit body.
3. Add new worklist items / follow-ups when emergent work
   surfaces. Tag follow-ups with the bundle's NF prefix.
4. Chrome upgrades when the result aligns with the design lock.
5. Re-cue misleading [✓]s without per-flip approval (rescue
   semantics from `iteration` Phase 0 carry over when a scope
   names an audit).

Authorizations NOT granted by default (require explicit lift):

- Push to remote (gated on the user's standing
  push-authorization).
- Cut releases (separate `cut release X.Y.Z` invocation per
  CLAUDE.md §0.6).
- Feature branches (main-only per §0.1; UX-* visual work
  uses the branch lane per §0.11).
- Destructive ops per §0.9.

## When NOT to use `batch`

- The work is one task, not a category. Use `autonomous-worker`
  or just do it inline.
- The scope crosses every category. Use `iteration` instead.
- The category needs design lock first (5-question survey).
  Run the survey, then `/batch`.
- Hardware-bench testing tasks. Skip per the standing
  "skip Hardware Testing epic" carve-out.

## Reporting cadence

- Per scope resolution: one line ("Resolved N tasks across M
  bundles").
- Per bundle: skip the report. The commit body is the record.
- Per bundle's verification: one line if green, full output
  if red.
- On exit: short summary of bundles shipped + verification
  status + any follow-ups added to the worklist.

## Sister skills

- [[iteration]] — open-ended loop; covers the whole worklist
  instead of one category.
- [[autonomous-worker]] — single-task version.
- [[complete-remaining-work]] — parallelization + completeness
  policy. `batch` inherits its bundle-naming + "no stubs" rules.
- [[mackes-worklist-management]] — worklist schema + status
  legend + canonical-file rules.

## History

- 2026-05-23 — skill created after the operator used `/batch
  "all front end work"` to focus the v2.5 Nebula Fabric NF-10..
  NF-18 desktop-surface push. The pattern was implicit in the
  iteration loop's "scope-filtered run"; capturing it here
  makes the trigger explicit + documents the bundle-sizing
  heuristics.
