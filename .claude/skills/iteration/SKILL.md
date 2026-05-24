---
name: iteration
description: Long-running autonomous loop that grinds the canonical worklist to "only hardware bench testing remains" — but ALWAYS begins with a worklist rescue pass that finds dead modules, re-cues misleading [✓] entries, surfaces mockup-only "shipped" features, scans for "lands in a follow-up" / "wired in Phase N" / "deferred to" markers (both in code comments AND in user-visible `text("…")` strings), audits design notes for incomplete ideas, and adds wiring tasks before any new code lands. New worklist items land as user stories (As/I want/so that + bench-observable acceptance) with Carbon Icon Set as the locked iconography source. Standing authorizations include commit anytime, best-choice decisions, scope/design improvements, new tasks, chrome upgrades. Use when the user says "iterate", "ship the worklist", "complete the build", "keep going until done", "rescue the worklist", "audit worklist for overlooked items", "find unwired modules", "find dead modules", "find mockup-only features", "audit design notes for incomplete ideas", "evaluate the design criteria", or sets a /goal that demands worklist exhaustion. Sister skills: [[autonomous-worker]] (single-task version), [[complete-remaining-work]] (parallelization policy), [[mackes-worklist-management]] (worklist schema).
---

# Iteration

The autonomous-worker pattern, scaled up, **with a mandatory rescue
pass at the front of every invocation**. Where `autonomous-worker`
runs one task at a time and pauses for commit approval, `iteration`
treats the worklist as a single goal, runs commits inline, and
keeps moving until only the Hardware Testing epic remains.

The rescue pass exists because the v3.x runtime-integration audit
on 2026-05-22 surfaced 13 dead panel modules + 6 unspawned daemon
workers all marked `[✓] shipped` — implementations + tests landed,
but the code was never reachable from a runtime entry point. Live
operator hit four user-visible bugs as the direct consequence. If
iteration starts without a rescue pass, the loop would happily mark
new modules `[✓]` against an already-rotten foundation.

## Triggers

Iteration triggers:

- "iterate"
- "ship the worklist"
- "complete the full build"
- "keep moving until done"
- "until the worklist has been fully and completely completed"
- A `/goal` that names worklist exhaustion as the condition

Rescue-only triggers (run Phase 0 then stop, do not enter the loop):

- "audit worklist for overlooked items"
- "rescue the worklist"
- "find unwired modules"
- "find dead modules"
- "find deferred wiring"
- "check for helpers-shipped-but-not-wired"
- "find mockup-only features"
- "find features that look done but use demo data"
- "audit design notes for incomplete ideas"
- "find stub backends" / "find demo backends"
- "evaluate the design criteria"
- "audit the design against the build"
- "find unfinished design ideas" / "compare design to implementation"

When triaging a "feature X doesn't work" bug against a `[✓]` entry,
run Phase 0 first to see whether the feature ever actually wired up.

## Standing authorizations (active for the whole loop)

The user opens this skill by granting a set of authorizations. The
canonical bundle, in order of decreasing default-on'ness:

1. **Commit when needed** — every logical unit becomes a commit, no
   per-commit approval prompts.
2. **Best-choice decisions** — when the worklist describes a design
   choice loosely ("E.1.2 skeleton" — wholesale GTK rip vs.
   side-by-side crate?), pick the one that best serves the spirit of
   the ask and document the call in the commit body.
3. **Move between phases/tasks** — don't fixate on a blocked task;
   advance to the next workable item. Update the worklist as items
   land.
4. **Improve the design** if the result aligns with the "spirit" of
   the ask. Capture deviations from the lock as one-line worklist
   notes ("changed X to Y because Z").
5. **Add new worklist items / epics** when an improvement or
   follow-up emerges. New epics get IDs like `IT-1`, `CHR-1`, etc.
6. **Chrome upgrades** — visual polish to 2026-grade design
   aesthetic. May iterate on previously-locked design ideas; mark
   the supersession in-place per the "newer wins silently" rule.
7. **Re-cue misleading [✓]s during rescue** — flip to `[>] In
   Progress` and add wiring tasks without per-flip approval.

Authorizations NOT granted by default (require explicit lift):

- **Push to remote** — `git push origin main` stays gated per
  `.claude/CLAUDE.md` §0.5 unless the user explicitly opens it.
- **Cut releases** — version bumps + tag + workflow runs go through
  the `cut release X.Y.Z` shorthand per §0.6, not autonomously.
- **Feature branches** — `main`-only per §0.1.
- **Destructive ops** — `rm -rf` outside build dirs, force-push,
  `reset --hard` per §0.9.

## Execution pipeline

Phase 0 runs once per invocation, before any code lands. Phases
1-7 loop until exit.

### Phase 0 — Rescue pass (mandatory, runs once)

The audit that catches "shipped helpers, deferred wiring" before
iteration adds more dead code on top.

0.1. **Inventory dead modules.** For every `pub mod` / `mod`
     declared in a crate root (`lib.rs` / `main.rs`), test whether
     any other file in the workspace references the module. Zero
     workspace references = dead at runtime. Paste-ready:

```bash
cd <workspace-root>
for root in crates/*/src/lib.rs crates/*/src/main.rs; do
  [ -f "$root" ] || continue
  crate=$(echo "$root" | cut -d/ -f2)
  grep -h '^pub mod \|^mod ' "$root" 2>/dev/null \
    | grep -v '^mod tests' \
    | awk '{print $NF}' | tr -d ';' \
    | while read -r mod; do
      modfile="crates/$crate/src/$mod.rs"
      [ -f "$modfile" ] || continue
      refs=$(grep -rln "${mod}::\|crate::${mod}\|self::${mod}" \
        --include='*.rs' crates/ 2>/dev/null \
        | grep -v "^${modfile}$" | wc -l)
      [ "$refs" = "0" ] && echo "$crate :: $mod (declared in $root)"
    done
done
```

Each line is a candidate dead module. Manually triage to
distinguish "intentional helper library" (legit) from "shipped but
never wired" (broken).

For non-Rust projects, adapt the inventory step to the language's
module convention (Python: `import X` / `from X import`, JS/TS:
`import ... from`).

0.2. **Cross-reference with the worklist.** For each confirmed
     dead module, grep `docs/PROJECT_WORKLIST.md` for entries that
     name the module or its phase ID. Surface `[✓] Done` entries
     whose body claims the module "shipped," "renders,"
     "subscribes," or "is wired" — these are the misleading [✓]s.

```bash
grep -n "${mod}\b" docs/PROJECT_WORKLIST.md | head -10
```

0.3. **Re-cue the misleading [✓]s.** Flip each to
     `[>] In Progress`:

- `- [✓] **Phase X.y title (shipped YYYY-MM-DD)** —` becomes
  `- [>] **Phase X.y title (helpers shipped YYYY-MM-DD, <gap>
  deferred — audit YYYY-MM-DD)** —`.
- Append `**Re-opened YYYY-MM-DD:**` note to the existing entry
  body stating exactly what's missing. Link to the audit doc.
- Do NOT delete the original body — the "shipped" date is
  historically true; the audit appends, never overwrites.

0.4. **Add wiring follow-ups.** For each rescue:

- If 1-2 rescues: add `[ ] Open v<next>:` task immediately under
  the re-cued entry.
- If 3+ rescues: create a new section header
  `### <version> runtime integration pass (audit YYYY-MM-DD)`
  below the current hotfix bundle. Cluster the new tasks there,
  in dependency order (subscriptions first → widgets they feed
  second → polish third).

Each new task must include: release-tag prefix per
[[mackes-worklist-management]] §1.1, Tier label (Tier 1 = live
bug, Tier 2 = dead module, Tier 3 = daemon worker), explicit
acceptance criterion (bench-observable behavior, not a file
landing).

0.5. **Write an audit doc** if 3+ modules need rescue at once,
     at `docs/V<version>_RUNTIME_INTEGRATION_AUDIT.md`. Structure:
     TL;DR · Tier 1 live bugs · Tier 2 dead modules table (module
     / phase / LOC / tests / wired? / worklist fine print) · Tier 3
     daemon · dependency-ordered plan · process retro. The v3.x
     example at `docs/V3_RUNTIME_INTEGRATION_AUDIT.md` is the
     reference shape.

0.6. **Report Phase 0 output** as four blocks: counts (`N dead ·
     M misleading [✓]s · K new tasks · 1 audit doc`), misleading
     [✓]s rewritten (compact list, one line each), rescue tasks
     added (compact, dep order), what's next (highest-priority
     rescue + unblocked-by chain).

0.7. **Mockup audit** (added 2026-05-23 after the operator hit
     mde-files showing demo_data peers + a DemoBackend). Phase
     0.1's "dead module" check only catches modules nobody
     references — but a module CAN be reachable from a runtime
     entry point AND still be a mockup, if its production data
     comes from demo constants or a stub backend that pretends
     to work. The DBusBackend parser shipped + the App boots +
     views render — but every list reads from
     `crates/mde-files/src/demo_data.rs` constants. Tests pass.
     The `[✓]` looks honest. The user-visible behavior is fake.

     **Detection greps** (run each, then triage):

     ```bash
     # 1. demo_data:: references in production view/render code.
     #    Any hit OUTSIDE #[cfg(test)] blocks is a mockup signal.
     grep -rn 'demo_data::\|::demo_data' crates/ \
         --include='*.rs' 2>/dev/null \
         | grep -v '#\[cfg(test)\]' \
         | head -40

     # 2. Stub/mock/demo backend types referenced from main launch
     #    paths (not just from #[cfg(test)] modules).
     grep -rn 'DemoBackend\|MockBackend\|StubBackend\|FakeBackend' \
         crates/ --include='*.rs' 2>/dev/null \
         | grep -v 'tests/\|#\[cfg(test)\]' \
         | head -40

     # 3. Iced match arms that exit-0 instead of running. The
     #    grandfathered Kind::Network in mde-popover/src/main.rs
     #    is the reference pattern; new instances are §0.12
     #    violations. Look for `tracing::info!.*not yet
     #    implemented.*exit 0` and friends.
     grep -rn 'not yet implemented\|not implemented yet\|exit 0' \
         crates/ --include='*.rs' 2>/dev/null \
         | head -40

     # 4. Comment + string markers that telegraph an incomplete
     #    impl. The "lands" family targets ONLY temporal-future
     #    "X happens later" shapes — `lands in a follow-up`,
     #    `lands in a later`, `lands when <phase>`, `wiring
     #    lands` — NOT generic data-flow descriptions like
     #    "the entry lands at index 0" or "the line lands in
     #    the journal" which use the same verb in a present-
     #    tense sense.
     grep -rnE 'TODO wire|FIXME wire|\bstub\b|placeholder|mockup|lands in a follow|lands in a later|lands when |wiring lands|wired in (Phase|v[0-9])|follow-up commit|deferred to|deferred from' \
         crates/ --include='*.rs' 2>/dev/null \
         | grep -v 'tests/\|//!' \
         | head -40
     # Also scan user-visible strings — when the operator
     # actually SEES "lands in a later" / "coming soon" / "not
     # yet implemented" copy in the running binary, that's a
     # surfaced mockup the operator ALWAYS reads as a defect.
     # This is the highest-precision sub-pass because the
     # promise has escaped the code into the UI.
     grep -rnE 'text\("[^"]*(lands in|not yet|coming soon|under construction|wired in|substep|follow-up)' \
         crates/ --include='*.rs' 2>/dev/null \
         | head -40

     # 5. Functions whose body is just `Vec::new()` or
     #    `unimplemented!()` or `todo!()` in production code.
     grep -rn 'unimplemented!\|todo!\|panic!("not yet")' \
         crates/ --include='*.rs' 2>/dev/null \
         | grep -v '#\[cfg(test)\]' \
         | head -40

     # 6. Design-doc claims vs. runtime reality. For each design
     #    doc under docs/design/, scan its "Acceptance" /
     #    "Locked behavior" / "Shipped" sections for feature
     #    names; spot-check the worklist + runtime reachability
     #    of each.
     find docs/design -name '*.md' -exec grep -lE \
         '## (Acceptance|Locked|Ships?|Shipped)' {} \; \
         | head -10
     ```

     **Triage rule.** A hit is mockup-only IF the production code
     path (the one invoked by a real `mde-*` binary at runtime,
     no test gate) reads/uses the demo source AND the worklist
     has a corresponding `[✓]` that doesn't already say "demo
     backend" / "mockup data" / "Phase G blocked" in its body.

     **Re-cue rule.** Same pattern as 0.3 but with different
     annotation:

     - `- [✓] **Phase X title (shipped YYYY-MM-DD)** —` becomes
       `- [>] **Phase X title (UI shipped YYYY-MM-DD; backend is
       mockup — audit YYYY-MM-DD)** —`.
     - Append `**Mockup-only:**` paragraph naming the demo
       source and the real-data path it needs (Phase G migration,
       missing DBus surface, hardcoded constants, etc.).
     - Add a follow-up `[ ] Open` task with the exact wire-up
       scope and stacked blockers (DemoBackend → Phase G →
       DBusBackend → mackesd Files server, as a worked example).

     **Design-doc cross-reference.** For each `docs/design/*.md`
     file the project ships, grep its "Locked" / "Acceptance" /
     "Ships" headings for feature claims. For each claim:

     - Is there a `[✓]` worklist entry referencing it?
     - Does the runtime code path use real data or demo data?
     - If the design doc claims a behavior the code can't
       deliver yet (e.g. "peer-card click opens the peer's
       inbox" but DBusBackend is a stub), that's a
       design-vs-reality drift — add a `[ ] Open` task to
       close the gap OR retire the design claim per
       newer-wins-silently.

     Mockup rescues fold into the same audit doc as dead-module
     rescues at 0.5 — same `### <version> runtime integration
     pass` section header, distinguished by a `Tier 2-mockup`
     label so the table differentiates them from `Tier 2-dead`.

0.8. **Design-criteria audit** (added 2026-05-23). Compare the
     platform's locked visual + UX criteria against what's
     actually built. Phase 0.7 catches "the data is fake"; this
     catches "the visual / interaction polish is missing", "the
     design doc evolved past the implementation", and "the
     screenshot in the design doc doesn't match the running
     binary."

     **Influence locks (read these BEFORE auditing chrome):**

     - **Microsoft Windows 11.** Fluent Design 2.0 chrome —
       Mica acrylic surfaces, **8 px rounded corners** on
       elevated surfaces, **per-window controls at top-right
       of each window's title bar** (NOT centered on the
       panel), Segoe Fluent Icon outlines (1 px stroke), Snap
       Layouts for tiling (visual layout templates: single,
       vsplit, grid-4, main+sidebar, tabbed), centered Start
       Menu, taskbar icons centered, 140 ms reveal animations.
       When MDE's UX overlaps Win11 surface (window controls,
       start menu shape, snap-layout idiom), default to the
       Win11 vocabulary so end-user muscle memory transfers.
     - **Ableton Live's default theme.** Dark slate background
       (#181818), **compact dense layouts** (minimal
       whitespace, parameter-display feel), **single accent
       color** across a panel/zone (NOT one per element),
       tabular-numeric readouts in monospace (IBM Plex Mono
       per Q12), **subtle 1 px dividers** between zones, high
       text contrast against the dark surface, slight 4 % bg
       lighten on hover (no big gradient ramp). When MDE's UX
       overlaps Ableton surface (Workbench parameter tables,
       file-list density, status-readout chips), default to
       Ableton's dense + monospaced + single-accent rules.

     The two influences resolve cleanly when split by surface
     role — **chrome** (window decorations, panel buttons,
     start menu, snap layouts) → Win11; **content** (parameter
     editors, file lists, status readouts, Workbench tables)
     → Ableton. Both honor the locked indigo Q2 accent + the
     Geologica/IBM-Plex-Mono typography pair (visual-identity.
     md Q11/Q12).

     **Audit greps + reads (run each, then triage):**

     ```bash
     # 1. Surface every design doc that locks visual / UX criteria.
     find docs/design -name '*.md' | head -40

     # 2. For each locked criterion in those docs, grep the
     #    matching crate for the implementation token. Examples
     #    of criteria → token mappings:
     #      "rounded corners" → grep -n 'radius:\|border_radius:' crates/...
     #      "accent colour"   → grep -n 'ACCENT\b\|accent_color' crates/...
     #      "monospace"       → grep -n 'IBM Plex\|monospace\|MONO\b' crates/...
     #      "Snap Layouts"    → grep -n 'snap_layout\|layout_button' crates/...
     #      "title bar"       → grep -n 'titlebar\|title_bar\|window controls' crates/...
     #    Hit-or-miss is the audit signal.

     # 3. Diff design-doc screenshots vs running binary. If
     #    docs/design/<area>/screenshots/ exists, render the
     #    current binary's equivalent surface (run; screenshot
     #    via `grim`) and visually compare. Any drift = audit
     #    finding.

     # 4. Read the visual-identity.md table of locked tokens
     #    (Geologica, IBM Plex Mono, indigo #5b6af5 Q2, charcoal
     #    #1d1d1f Q3, etc.) and grep for usage. Wrong-color
     #    accent in any chrome surface = audit finding.
     grep -rn '#5b6af5\|#1d1d1f' crates/ --include='*.rs' | head
     ```

     **Triage + capture rule.** For each finding:

     - If the design doc is the live policy and the code lags:
       add a `[ ] Open` worklist task with the precise
       implementation gap. Include the influence reference
       ("per Win11 Snap Layouts", "per Ableton's monospace
       readout convention") so the future implementer knows the
       visual vocabulary to draw from.
     - If the code shipped past the design doc and the operator
       has signed off on the new direction in conversation,
       update the design doc + leave the code (newer-wins-
       silently from
       [[mackes-worklist-management]] §1).
     - If neither is clearly authoritative, surface to the
       operator with a one-line "design vs build mismatch:
       <area> — doc says X, code does Y. Which wins?" — that's
       a product-direction decision, not a best-choice call.

     **Design elaboration rule (for vague worklist entries).**
     When a worklist entry names a feature without specifying
     the visual treatment ("add a layout button", "show the
     update count"), the implementer SHOULD draw from the two
     influence locks above to fill in the spec — rounded
     corners + Carbon SVG icon + indigo accent for chrome
     surfaces; dense + monospace + 4 % hover for content
     surfaces. Mark such expansions in the commit body
     ("Win11-flavored Snap Layout chrome per iteration skill's
     design influence section") so the design lineage stays
     auditable.

If a rescue-only trigger fired (not a full iteration trigger),
**stop here**. Otherwise proceed to Phase 1.

### Phase 1-7 — Iteration loop

Loop until the only `[ ] Open` / `[!] Blocked` items are in the
Hardware Testing epic (or whatever the user-named carve-out is).
Rescue-created tasks count like any other — work them in
dependency order.

1. **Snapshot the worklist.** Count `[✓] Done`, `[!] Blocked`,
   `[>] In Progress`, `[ ] Open`. Identify the Hardware Testing
   epic boundary (top of "Epic: Hardware Testing" section if
   present).
2. **Pick the highest-impact next move.** Prefer items that
   unblock the most downstream tasks (e.g., Phase E.1 unblocks
   ~30 Phase E ports; a toplevels subscription unblocks hero +
   window buttons + expose). Use the bundle/parallelization
   policy from [[complete-remaining-work]] when independent items
   can run side-by-side.
3. **Decide the path** when a worklist item names a design
   choice loosely. Record the call in the commit body so the
   newer-wins-silently rule (§1) keeps history auditable.
4. **Implement — fully, no stubs.** Code, tests, smoke. Per
   `.claude/CLAUDE.md` §0.12 (no stubs / skeletons / staged
   work), the new code MUST be reachable from a runtime entry
   point before this step exits. No `todo!()`, no
   `unimplemented!()`, no `Kind::Foo => Ok(())` stub branches,
   no `pub mod foo;` with zero workspace references. If the
   task can't ship complete in one commit, stop and re-split it
   at write-time — never commit a half-implementation.
   Follow the project's code-style locks (`.claude/CLAUDE.md`
   §3). Run focused validation:
   - Module-import smoke for every Python module touched.
   - `make test-nodeps` if `tests/` touched.
   - `cargo check -p <crate>` if a Rust crate touched
     (`cargo test` if the change is non-trivial).
   - `make rpm` if `packaging/`, `setup.py`, `pyproject.toml`,
     `data/`, or `mackes/birthright.py` touched.
   - `install-helpers/lint-css.sh` if `data/css/` touched.
   - **Reachability check:** confirm the new module's public
     surface is invoked from at least one runtime entry point
     (panel `update()`/`view()`/`subscription()`, daemon
     `run_serve()`, popover `main()`). Same grep as Phase 0.1.
5. **Update the worklist.** Flip `[!]` → `[ ] Open` (unblocked)
   or `[ ] Open` → `[>] In Progress` → `[✓] Done`. Per §0.8 DoD,
   `[✓]` requires runtime reachability — never mark done when
   only helpers + tests landed. Add follow-ups as new `[ ] Open`
   items with crisp acceptance criteria.
6. **Commit.** HEREDOC message in project style, explicit file
   paths, never `git add -A`. Co-author the active model. Commit
   bodies that say "wiring lands in a follow-up" or "phase 2
   implements" are forbidden per §0.12.
7. **In-loop spot-check.** Every ~5 commits, re-run Phase 0.1
   (dead modules), Phase 0.7 (mockup grep), AND Phase 0.8
   (design-criteria) scoped to the modules touched in the
   recent batch. Three failure modes to catch:
   (a) iteration itself produces dead modules despite §0.12 —
       new `pub mod foo;` with zero workspace references → flip
       the related `[✓]` and add a wiring task.
   (b) iteration introduces a new `demo_data::` reference or a
       `DemoBackend`-flavored shortcut into a production code
       path — usually because the real wire-up was harder than
       the deadline allowed and a fixture got committed. Flip
       the related `[✓]` to `[>]` with a **Mockup-only:** note
       per 0.7's re-cue rule.
   (c) iteration ships chrome that drifts from the Win11 or
       Ableton influence locks — sharp corners on an elevated
       surface, per-element accents instead of one-per-zone,
       sans-serif numerics instead of IBM Plex Mono, etc.
       Capture as a follow-up `[ ] Open` design-polish task
       per 0.8's elaboration rule; don't block the current
       commit landing.
   For all three, skip the audit-doc step on single rescues —
   just flip the entry + add the follow-up. The audit doc gets
   written when the next Phase 0 sweep finds ≥ 3 rescues at
   once.
8. **Loop.** Return to step 1.

## Hardware Testing carve-out

Hardware-only validation (clean-install benches, upgrade benches,
VM CI, sway-in-CI smokes, Docker peer fan-out) is its own epic.
It is **not** a blocker on iteration. The loop's exit condition
is: every non-hardware item is `[✓] Done`. Hardware items stay
`[ ] Open` indefinitely — they run on bench/CI cadence against
already-feature-complete cuts.

## When to stop autonomously

Pause and surface to the user when:

- Every non-hardware worklist item is `[✓] Done` (success exit).
- A destructive op is needed (§0.9 lock).
- A cut-release decision arises (§0.6 needs explicit invocation).
- A push-to-remote moment arrives (separate auth per §0.5).
- A single decision would change product direction in a way the
  user couldn't reconstruct from the commit body alone (e.g.,
  retiring a whole locked epic, swapping a major dependency).
- Two consecutive build/test attempts fail with no clear root
  cause (don't sleep-retry; ask).
- Phase 0 surfaces 10+ rescues at once — the size is a signal
  that a sustained sweep needs operator coordination before
  iteration plows ahead. Report Phase 0 output, then pause.

## Reporting cadence

Stay heads-down unless something interesting happens:

- Phase 0 output: always reported (see 0.6).
- Per-commit: skip the report. The commit body is the record.
- Per-phase / per-epic boundary: a one-paragraph status note.
- Per in-loop rescue: one-line callout
  (`rescue: Phase X.y → [>] (was [✓]) — <gap one-liner>`).
- On stop: a short summary (what shipped, what's queued, what's
  blocked, what was rescued) — no marketing copy, just facts.

## Project-specific anchors (Phase 0 detection targets)

- **Worklist:** `docs/PROJECT_WORKLIST.md`
- **Crates root:** `crates/`
- **Panel runtime entry points:** `crates/mde-panel/src/lib.rs::App::{update,view,subscription}`
- **Daemon runtime entry points:** `crates/mackesd/src/bin/mackesd.rs::run_serve`
- **Popover runtime entry points:** `crates/mde-popover/src/main.rs::main`
- **CLAUDE.md no-stubs rule:** §0.12
- **CLAUDE.md DoD gates:** §0.8

## When NOT to rescue a dead module

- The dead module is a documented helper library that's
  legitimately imported by external tooling (e.g. a `mackes-config`
  crate that ships pure types for other binaries). Run the
  inventory with `--include='*.toml'` and check for path-dep
  declarations before re-cuing.
- The module is brand-new in the current branch (not yet
  committed) and hasn't had its wiring step yet — that's just
  work-in-progress, not a rescue case.
- The user has explicitly retired a module (e.g. `layer_shell.rs`
  superseded by `iced_layershell`) — rescue would re-open a
  decision the user already closed. Confirm by grepping the
  worklist for `retired` / `superseded` notes on the module.

## Worklist hygiene during iteration

- Never write `[~] Deferred` (retired 2026-05-19).
- Never call hardware items "blocked" — they're a separate epic.
- When a newer directive contradicts an earlier lock, update the
  worklist in place and move on (§1 newer-wins-silently).
- New items get crisp acceptance criteria; vague items get
  rewritten before being claimed.
- When a worklist item turns out wrong/redundant/contradicted,
  retire it with a one-line `[✓] retired:` note instead of
  forcing it through.
- Per §0.12, never mark `[✓]` on a module that isn't reachable
  from a runtime entry point. The Phase 0 grep is the test.

## Sister skills

- [[autonomous-worker]] — single-task version. Use when the user
  hasn't granted the full Iteration authorization bundle.
- [[complete-remaining-work]] — parallelization + completeness
  policy. Iteration inherits its bundle-naming + "no stubs"
  rules.
- [[mackes-worklist-management]] — worklist schema + status
  legend + canonical-file rules.
- [[frontend-design:frontend-design]] — invoke before shipping
  a chrome upgrade so the design choice is concrete (show,
  then ship).
- [[verify]] — invoke after non-trivial UI changes to confirm
  the feature actually works end-to-end. The runtime-
  reachability check in Phase 4 + 7 is a structural variant;
  `verify` is the user-gesture variant.

## Story format for new worklist items (added 2026-05-23)

Every new worklist task (operator-reported bug, design follow-up,
elaboration of a vague spec, rescue-from-audit) **lands as a
user story**, not a one-line summary. The shape:

```
- [ ] **<release-tag>: <ID> <short title> (Tier <N>)**
  **As** an <operator | maintainer | end user | mesh peer>,
  **I want** <the change in user-visible behavior>,
  **so that** <the user value the change delivers>.

  **Acceptance** (every bullet bench-observable, not a file
  landing):
  - [ ] <criterion 1 — a thing the operator can verify by
        looking at the running system>
  - [ ] <criterion 2>
  - [ ] <criterion 3>

  **Implementation notes** (constraints, references, tradeoffs):
  - <influence reference per Phase 0.8 — "per Win11 Snap
    Layouts" / "per Ableton's monospace readout convention">
  - <stacked blockers if any>
  - <design-doc cross-reference>
  - <iconography source must be Carbon per the lock below>
```

Why stories beat one-liners:

- **"As / I want / so that"** forces the contributor to name
  the user value before implementing. Bug fixes that don't
  improve user-visible behavior get caught at write-time — if
  you can't fill in "so that", the task probably shouldn't ship.
- **Bench-observable acceptance** means "the binary does X
  visibly to the operator" — not "file Y has function Z".
  Per §0.8's runtime-reachability gate, file landings don't
  count as done; observable behavior does.
- **Influence references in implementation notes** keep the
  Phase 0.8 design-criteria pass auditable: the next reviewer
  knows where the visual decisions came from.

The existing one-line tasks created before this rule are
grandfathered — no retroactive rewrite. New `[ ] Open` /
`[!] Blocked` items use the story shape going forward. The
worklist-rescue protocol at 0.3 / 0.7 already requires precise
re-cue annotations, which fit naturally into the story body.

### Iconography lock (added 2026-05-23)

**Every icon in MDE ships from the Carbon Icon Set.** No
exceptions for chrome, content, or content-of-content.

- **Source asset:** `/usr/share/icons/Mackes-Carbon/scalable/
  apps/*.svg` (the system-installed Carbon theme) OR the
  baked subset at `assets/icons/carbon/*.svg` (committed to
  this repo, consumed via `include_bytes!` per BUG-13).
- **Semantic resolver:** `mde_theme::Icon` enum →
  `Icon::carbon_name()` returns the Carbon symbolic name →
  `ResolvedIcon::svg_bytes()` returns the baked bytes (Some
  for every variant after BUG-13.c).
- **Renderer:** `iced::widget::svg(svg::Handle::from_memory(
  bytes))` for Iced surfaces; `gtk_image_new_from_file()`
  with a Carbon-themed path for any residual GTK consumer.
- **Forbidden:** Lucide, Phosphor, Material, Bootstrap, Font
  Awesome, fdo hicolor / Black-Sun / Orchis icon themes for
  any new icon slot. Emoji + Unicode fallbacks are allowed
  ONLY as safety-net text-renders when `svg_bytes()` returns
  None for an unbaked variant; the variant getting an SVG
  added to `assets/icons/carbon/` is the proper closure.
- **When the worklist names a feature involving an icon**,
  the story's Implementation notes section MUST cite which
  Carbon glyph to use (by carbon_name). Example: "icon:
  `network--3` per the existing `Icon::Network` resolver
  arm". An unspecified icon is an incomplete story — kick
  it back for elaboration.

The Phase 0.8 design-criteria audit treats any non-Carbon icon
asset reachable from production code as a finding. The
`assets/icons/carbon/` directory is the bake-source; new
variants land there + get a matching arm in `mde_theme::
ResolvedIcon::svg_bytes()` before the consuming feature can
flip to `[✓]`.

## Worklist hygiene rule for mockups (added 2026-05-23)

Two new rules layer onto the existing §0.12 no-stubs lock + the
runtime-reachability §0.8 gate 7:

- **Mockup-only [✓]s are forbidden.** If the runtime code path
  uses `demo_data::`, a `DemoBackend`-flavored stand-in, or any
  hardcoded fixture in production code (no `#[cfg(test)]` gate),
  the worklist status is at most `[>] In Progress` — never
  `[✓] Done`. The Phase 0.7 grep is the test.
- **Design-doc claims are worklist items.** If a `docs/design/
  *.md` file locks a behavior, the iteration loop treats it as a
  `[ ] Open` worklist task even if the worklist itself doesn't
  carry the entry yet. The Phase 0.7 design-doc cross-reference
  surfaces missing entries; iteration lifts them into the
  worklist (per [[mackes-worklist-management]] §1) before
  treating them as work.

## Design influence locks (Phase 0.8, added 2026-05-23)

Two reference designs anchor the platform's visual vocabulary
and are invoked by the design-criteria audit (Phase 0.8) AND
the per-loop spot-check at step 7. When the implementation
needs visual / interaction polish past what a design doc spells
out, the implementer draws from the appropriate reference:

| Surface kind | Reference | Key tokens |
|---|---|---|
| Chrome (window controls, panel, start menu, snap layouts, popovers) | **Windows 11** | 8 px rounded corners, Mica/acrylic surfaces, Segoe Fluent Icon outlines (1 px stroke), per-window controls top-right, centered start, 140 ms reveals |
| Content (parameter editors, file lists, status chips, Workbench tables) | **Ableton Live default theme** | #181818 slate background, compact dense layouts, single accent per zone (Q2 indigo), IBM Plex Mono numerics, 1 px subtle dividers, 4 % bg lighten on hover |

These are **influence locks**, not pixel-perfect copies — MDE's
own brand tokens (Q2 indigo `#5b6af5`, Q3 charcoal `#1d1d1f`,
Geologica/IBM Plex Mono per Q11/Q12, Carbon SVG icons per Q24
/ BUG-13) always trump the reference where they conflict. The
references define the IDIOM (where do window controls live,
what counts as "elevated", how dense should a parameter list
be), not the exact colors.

**Why this split.** Operators come to MDE with muscle memory
from both worlds: from Windows for chrome interaction (close
button top-right, snap layouts on maximize, centered start),
from creative tools like Ableton for content interaction
(dense tables, monospace numerics, single accent). Splitting
the influence by surface role keeps each muscle-memory pattern
intact in the surface where it applies.

**Forbidden hybrids** (caught by Phase 0.8):

- Rounded corners on a content-row in a Workbench parameter
  table (rounded = elevated chrome; rows should be flat).
- Per-row accent colors (multiple accents in one zone = audit
  finding; one zone gets one accent).
- Sans-serif numerics in a parameter readout (use IBM Plex
  Mono per Q12).
- Per-window controls centered on the panel (Win11 has them
  per-window top-right — BUG-6 was a deviation that the
  operator reversed in BUG-16).
- **Non-Carbon icon assets in any production surface.** The
  iconography lock below makes Carbon Icon Set the only
  source for chrome + content icons. Lucide / Phosphor /
  Material / Font Awesome / hicolor / Black-Sun / Orchis
  icons appearing in production code = audit finding.
  Unicode fallback glyphs are tolerated only as the
  `svg_bytes() → None` safety net (BUG-13).

## History

- 2026-05-22 — merged the standalone `worklist-rescue` skill into
  this one as Phase 0. The standalone was deleted; rescue is now
  inseparable from iteration so the loop can't add stubs on top
  of a rotten foundation.
- 2026-05-23 — added Phase 0.7 (mockup audit) + the in-loop
  mockup spot-check at step 7. Triggered by the operator
  observing that mde-files at v4.0.0 "is mostly mockup" despite
  a [✓] history — the binary boots and renders, but every list
  reads from `demo_data` constants and Send-To runs through a
  synthetic DemoBackend. The dead-module grep at 0.1 didn't
  catch it because the modules ARE reachable; the missing piece
  was the runtime data source. 0.7 closes that gap by grepping
  for `demo_data::` + `DemoBackend` + "not yet implemented" +
  comment markers (TODO wire, stub, placeholder) AND
  cross-referencing the design docs' Acceptance / Locked /
  Shipped sections against actual runtime behavior.
- 2026-05-23 — added Phase 0.8 (design-criteria audit) + the
  Win11 / Ableton influence locks. Triggered by the operator
  reversing BUG-6 (centered window controls) for BUG-16
  (per-window controls top-right, Win11 standard; panel center
  gets Snap-Layout-style Desktop Layout buttons). 0.8
  formalizes the visual evaluation: it compares each
  docs/design/*.md "Locked" / "Acceptance" claim against the
  current implementation, surfaces drift, and gives the
  implementer concrete reference vocabulary (Win11 chrome,
  Ableton content surfaces) for filling in vague design specs.
  The in-loop spot-check at step 7 also re-runs 0.8 against
  modules touched in the recent batch, catching chrome that
  drifts from the influence locks before the [✓] lands.
- 2026-05-23 — added the **story-format** rule for new
  worklist items + the **Carbon iconography lock**. Both are
  operator-locked: every new task body uses an "As / I want /
  so that" narrative + bench-observable acceptance bullets +
  Implementation notes that cite the design-influence
  reference and the Carbon glyph name. Every production icon
  asset ships from `assets/icons/carbon/` (baked from the
  system Mackes-Carbon theme); non-Carbon icons in production
  code are an audit finding. Replaces the prior one-line task
  shape going forward; existing one-liners are grandfathered.
- 2026-05-23 — expanded Phase 0.7's comment-marker grep with
  the **"lands" verb family** (`lands in`, `lands when`,
  `lands later`, `lands in a follow`, `lands at`, plus
  `wired in (Phase|vN)`, `follow-up commit`, `deferred to`,
  `deferred from`). Triggered by the WB-2 audit on the
  Workbench: 12 nav-listed panels rendered the catch-all
  `text("Panel view lands in a later CB-1.x substep.")` —
  a user-visible string that telegraphed incompleteness
  to the operator and was the proximate cause of the
  "many panels in the workbench are incomplete" report.
  The audit now also greps user-visible
  `text("…lands…")` strings, since "lands in a follow-up"
  in a comment is internal tech-debt but the same phrase
  inside a `text()` ships the promise to the screen.
