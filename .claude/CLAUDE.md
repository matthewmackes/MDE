# Mackes Shell — Claude Workspace Instructions

**Last updated:** 2026-05-17
**Worklist:** `docs/PROJECT_WORKLIST.md` (canonical)
**Memory:** `~/.claude/projects/-home-mm-Desktop-files-mackes-shell/memory/MEMORY.md`

Adapted from the MAP2 audio platform's `.claude/CLAUDE.md` §0 rulebook,
ported to a single-repo GTK3 Python application with RPM packaging.

---

## 0. Commit & Push Rulebook (APPLIES TO EVERY CHANGE)

These rules govern every commit and push Claude performs in mackes-shell.
They override any conflicting default behavior.

### 0.1 Branch discipline

- **Always stay on `main`.** Never create feature branches unless the user
  explicitly asks.
- **Never force-push to `main`.** If asked, warn and confirm first.
- **No `--no-verify`, `--no-gpg-sign`, `--amend` of pushed commits**, or
  hook skipping — unless the user explicitly requests it. Always prefer a
  new commit over amending.

### 0.2 Dual-remote push (mackes-shell-specific, updated 2026-05-23)

mackes-shell has **two** remotes:

  * `origin  → github.com:matthewmackes/MAP2-RELEASES.git`
    (releases mirror; protected `main` ref; the bypass message
    `remote: - Cannot update this protected ref.` is expected on
    every push and does not indicate failure — the push still
    completes, see the `<sha>..<sha>` line below it).
  * `mde-x   → github.com:matthewmackes/MDE-X.git`
    (development mirror; unprotected).

The canonical push is **dual** — every push to `main` lands on both:

```bash
git push origin main && git push mde-x main
```

Treat both as required: a push that only lands on one of them leaves
the other mirror behind. If a future cut splits release vs dev
artifacts unevenly, document the divergence here before changing the
pattern. The previous "single remote → origin only" wording (pre
2026-05-23) was stale — the `mde-x` remote was added some time after
the v3.x cuts and the rulebook only caught up here.

### 0.3 Staging hygiene

- Prefer `git add <file>` with explicit paths over `git add -A` / `.`.
- Never commit a file that likely contains secrets. Warn first.
- Never modify `git config`.
- Never touch `mackes/__init__.py:__version__`, `pyproject.toml`,
  `setup.py`, or `packaging/fedora/mackes-shell.spec` versions manually —
  they're bumped via the cut-release flow (see §0.6).

### 0.4 Commit message format

- Follow the existing log style (inspect `git log` before drafting).
- Focus on **why**, not what — the diff already shows what.
- Pass the body via HEREDOC so newlines survive intact:
  ```bash
  git commit -m "$(cat <<'EOF'
  Concise summary

  Optional paragraph explaining motivation and user-visible impact.

  Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
  EOF
  )"
  ```
- "add" = new feature, "update" = enhancement, "fix" = bug fix,
  "refactor" = no behavior change.

### 0.5 Only commit and push when the user asks

- **Never commit unsolicited.** Writing code, running tests, and making
  edits does not license a commit.
- **Never push unsolicited.** Even after the user approves a commit, a
  push is a separate authorization.
- One commit/push approval is not a standing license — ask each time
  unless the user has said "autonomous" / "execute" / "no confirmation
  needed" for the current scope.

### 0.6 `cut release` shorthand

When the user types `cut release X.Y.Z` (or says "cut a release"), treat
it as a single executable command with seven ordered steps — execute all
seven without asking for confirmation between steps unless a step fails:

1. **Bump version** in four files:
   - `mackes/__init__.py:__version__`
   - `pyproject.toml:version`
   - `setup.py:version=`
   - `packaging/fedora/mackes-shell.spec:Version`
2. **CHANGELOG entry** at the top of `CHANGELOG.md` under
   `## X.Y.Z — <one-line summary> (YYYY-MM-DD)`. Describe what shipped.
3. **Smoke test:** `python3 -c "import mackes; print(mackes.__version__)"`.
4. **Local RPM build:** `make rpm` (clean dist + rpmbuild first).
   **Always go through `make rpm`.** Never invoke `rpmbuild` directly
   with `--short-circuit` (or any subset flag like `-bi` / `-bb`
   without a clean source tree). Short-circuit builds stamp the
   `rpmlib(ShortCircuited) <= 4.9.0-1` dep on the output, which
   makes the RPM uninstallable (`dnf install` rejects it with
   `is needed by mde-X.Y.Z`). `make rpm` now verifies this and
   fails loudly — if you see that guard fire, blow away
   `rpmbuild/{BUILD,BUILDROOT,RPMS,SRPMS}` and rerun `make rpm`
   without flags. v2.0.1's `rpmbuild/RPMS/x86_64/mde-2.0.1-…rpm`
   was the first one this caught (2026-05-21).
5. **Commit:** `git commit` with the rulebook's HEREDOC format.
6. **Push + tag:**
   ```bash
   git push origin main && \
       git tag -a vX.Y.Z -m "Mackes Desktop Environment X.Y.Z — …" && \
       git push origin vX.Y.Z
   ```
   (Tag annotation uses the v2.0.0 rebrand-locked name "Mackes
   Desktop Environment", not the legacy "Mackes Shell" / "Mackes
   XFCE Workstation" strings. Operator-verification 2026-05-22
   on the v2.0.3 cut surfaced the legacy string still in
   `release.yml`'s release-name field; both the cut-release
   shorthand and the workflow are now aligned.)
7. **Watch the workflow:** `gh run watch <id> --exit-status`, then
   confirm with `gh release view vX.Y.Z`.

If just `build the RPM for testing` is requested instead, run only steps
1–4 (no commit, no tag, no push).

### 0.7 Pre-commit gates

Before every commit, when applicable:

1. **Module import smoke:** `python3 -c "<every module touched>"` —
   must run without error.
2. **Tests:** `make test-nodeps` (if `tests/` or `mackes/` touched).
3. **Ruff lint:** `make lint` (if `mackes/` or `tests/` touched).
   Mirrors the exact `ruff check --select F401,F541,F811,F841 mackes/
   tests/` ci runs, so a local pass means ci will pass too.
4. **RPM build:** `make rpm` (if `packaging/`, `setup.py`,
   `pyproject.toml`, `data/`, or `mackes/birthright.py` touched).
5. **CSS lint:** `install-helpers/lint-css.sh` (if `data/css/` touched).
6. **Voice-and-tone lint** (v4.0.1, added 2026-05-23):
   `install-helpers/lint-voice.sh` (if any user-visible string
   touched — typically `crates/mde-*/src/`, `mackes/workbench/`,
   `mackes/wizard/`, `data/applications/*.desktop`). Enforces the
   verb-discipline table + forbidden-strings list locked in
   `docs/design/voice-and-tone.md`. Mirrors the same script the
   CI runs.
7. **Legacy-mesh-vocabulary lint** (v2.5 NF-20.6, added 2026-05-24):
   `install-helpers/lint-legacy-mesh.sh` (if any `.rs` or `.py`
   file outside the v1.x legacy tree is touched). Catches
   net-new `tailscale` / `headscale` / `derper` references in
   v2.5+ Nebula-native source. The v1.x Python tree
   (`mackes/*`), the legacy GTK panel (`crates/mackes-panel/`),
   the legacy mackesd workers + transport modules that NF-4.5
   will retire, and `tests/*` are allow-listed by directory
   prefix; retraction-comment lines (NF-N.M / GF-N.M / RD-N /
   KDC2-N tags, or "retired"/"legacy"/"superseded" verbs) and
   pure `//` / `#` comment lines are also allow-listed.
8. **D-Bus shape lint** (added 2026-05-25 per Q12 + Q20 + Q96 +
   EPIC-PROC-LINT): `install-helpers/lint-dbus-shape.sh` (if
   any `crates/*/src/*.rs` is touched). Catches net-new
   `#[interface]` blocks in MDE-internal services. Per Q20 + Q96
   the canonical IPC for MDE-internal control is Bus
   (`action/<domain>/<verb>` for commands, `reply/<ulid>` for
   responses, domain topics for events); D-Bus retires entirely
   by 1.0 except for FDO interop (`org.freedesktop.*`). Pre-
   existing services are snapshot-allow-listed at gate-install
   time; the allow-list shrinks as EPIC-RETIRE-DBUS migrates
   each service.
9. **Material Symbols icon lint** (added 2026-05-25 per Q43 +
   Q97 + EPIC-PROC-LINT): `install-helpers/lint-material-symbols.sh`
   (if any `crates/mde-*/src/`, `data/css/`, or
   `data/applications/*.desktop` is touched). Catches net-new
   Carbon icon references (`carbon-<name>`, `@carbon/icons`,
   `bx--`, `cds--`). Per Q43, Material Symbols replaces
   Carbon; per Q97 migration must finish before 1.0. The
   icon set asset directory (`data/icons/Mackes-Carbon/`), the
   v1.x Python workbench (retiring per Q49), the legacy GTK
   panel, and the panel's icon_mapper.rs (target of
   EPIC-UI-MATERIAL) are allow-listed.

If a pre-commit hook fails, the commit did **not** happen — fix the
issue, re-stage, and create a **new** commit. Never `--amend` in that
scenario.

### 0.8 Definition of Done

A worklist task is not `[✓] Done` until **every** gate passes:

1. **Code committed to `main`** — in git history, not only in working tree.
2. **Pushed to origin** — `git push origin main` completed without error.
3. **RPM builds** — `make rpm` exited 0 (for spec / data / module
   changes).
4. **Tagged + released** — for shipping versions, `vX.Y.Z` tag pushed and
   the release workflow lands on GitHub.
5. **All module imports clean** — `python3 -c "import mackes.<module>"`
   passes for everything touched.
6. **CHANGELOG updated** — user-visible change documented.
7. **Runtime reachability** (added 2026-05-22) — every public function
   the task introduces must be invocable from a runtime entry point
   (a user gesture, scheduled tick, subscription, or daemon `run_serve`
   spawn). For Rust crates, the test is: `grep -rln "${mod}::" --include='*.rs'
   crates/${crate}` returns at least one file other than `${mod}.rs`
   itself. For Python modules, the test is: at least one `import` or
   `from … import` references the module's name from outside its own
   file. This gate is what the v3.x audit on 2026-05-22 would have
   caught — 22 misleading `[✓]` entries shipped helpers + tests but
   left the wiring dead, costing 4 user-visible bugs and an emergency
   integration sweep. See [[V3_RUNTIME_INTEGRATION_AUDIT]].
8. **Security review on new surface** (added 2026-05-25 per Q64 of the
   100-Q tightening survey + EPIC-PROC-DOD) — when a task introduces
   a new public port, a new D-Bus method, or a new Tera template that
   uses `{{exec}}`, leave a one-paragraph "Security review notes" line
   on the task acceptance covering: (a) the surface (port number /
   D-Bus path / template scope), (b) what reaches it (Nebula-only?
   FDO-only? Mesh passcode required?), (c) whether the surface fits
   the open-mesh / flat-trust directive or violates it. No code review
   process — Claude self-flags. The gate fails if the task body
   introduces new surface area without the notes line.

Writing code alone never satisfies Done. If gates 1–8 aren't confirmed,
the task stays `[>] In Progress` with a note on which gate is incomplete.

### 0.9 Destructive operations require explicit authorization

Before running any of these, pause and confirm:

- `git push --force` / `--force-with-lease` to `main`
- `git reset --hard`, `git checkout -- .`, `git restore .`, `git clean -f`
- `git branch -D`
- Amending an already-pushed commit
- `rm -rf` on anything outside `dist/` or `rpmbuild/`
- Any action that modifies GitHub state (closing PRs, deleting remote
  branches)

### 0.10 When a commit/push fails

- **Pre-commit hook failed** → fix the issue; re-stage; make a **new**
  commit. Don't `--amend`.
- **Remote rejected push (non-fast-forward)** →
  `git fetch origin main`; merge or rebase; resolve conflicts; re-run
  the push.
- **`make rpm` failed** → diagnose, fix, re-run; don't push the
  half-built tree.

### 0.11 Visual / design work uses a PR-based branch lane (WF-1, 2026-05-21)

§0.1's "always stay on `main`" default is wrong for visual / design
work, where a human-eye review of before/after screenshots is the
load-bearing gate. UX-* tasks (and any work whose primary diff is
visual rather than logical) follow this branch protocol:

- **Branch name:** `ux/<task-id>` (e.g., `ux/UX-14-command-palette`,
  `ux/UX-5-sidebar`). Short-lived; deleted after merge.
- **PR description requirements:**
  1. Before/after screenshots **in dark + light + all density modes
     touched**, embedded inline.
  2. Cite the design-lock IDs (Q-*, FU-*, NFU-*, UX-*) the change
     implements.
  3. Snapshot-diff output from UX-23 (once that gate lands), or
     manual `make snapshots-local` artifact during the HW-3 deferral
     window.
- **Merge gate:** explicit user OK on the PR. CI-green is necessary
  but not sufficient — a code-passing-tests change can still look
  wrong.
- **Code-only tasks (no visible UI diff)** retain the main-only
  default from §0.1. The branch lane is for visual diffs only.

This rule supersedes §0.1 for UX-* and visual work; §0.1 continues
to apply everywhere else.

### 0.12 No stubs, skeletons, or staged work (audit 2026-05-22)

Every commit ships **fully complete** code. No stubs, no skeletons,
no "data layer now, wiring later" splits, no "phase X-helpers
shipped, phase X-wiring deferred." Code that lands in `main` must
be reachable from a runtime entry point and the user-visible
behavior it implements must work end-to-end.

This rule is the upstream prevention for the failure mode the v3.x
runtime-integration audit on 2026-05-22 surfaced: 13 of 18 panel
modules (`crates/mde-panel/src/*.rs`, ~3,057 LOC, 139 tests) had
been marked `[✓] shipped 2026-05-21` while sitting entirely dead at
runtime — never referenced from `update()`/`view()`. Live operator
hit four user-visible bugs as the direct consequence (start menu
won't close, notifications panel won't close, missing window mgmt
buttons, right-click M button dead). Full inventory at
[`docs/V3_RUNTIME_INTEGRATION_AUDIT.md`](../docs/V3_RUNTIME_INTEGRATION_AUDIT.md).

**Concrete refusals:**

- Never write `todo!()`, `unimplemented!()`, or `panic!("not yet")`
  in committed code.
- Never write a match arm like `Kind::Network => { tracing::info!
  ("network popover not yet implemented; exit 0"); Ok(()) }`.
  Either build the arm or remove the variant until you do. The
  existing `Kind::Network` stub in `crates/mde-popover/src/main.rs`
  is grandfathered until the v3.0.3 network-popover task closes —
  no new instances allowed.
- Never write a commit body that says "wiring lands in a follow-
  up," "phase 2 implements," or "scaffolds the X for later."
- Never commit a `pub mod foo;` declaration unless at least one
  other file in the workspace references `foo::` or `crate::foo::`.
  This is the [[worklist-rescue]] runtime-reachability check and
  should run mentally pre-commit on every module-introducing
  diff. (Tests within `foo.rs` itself don't count — they reference
  the module from inside.)
- Never mark a worklist item `[✓]` until a user gesture or
  scheduled tick can actually invoke the new code. §0.8 Definition
  of Done covers this with a proposed 7th gate (v3.0.3 task
  pending operator authorization).

**Splitting rule:** if a task can't ship complete in one commit,
split it at write-time into smaller tasks each of which CAN ship
complete — NOT into "helpers + wiring." Each subtask's acceptance
criterion must name a bench-observable behavior, not a file that
landed. If a subsystem the task depends on doesn't exist yet,
build that subsystem first.

**Trigger phrases to refuse (or surface this rule before
complying):**

- "Just stub it out for now"
- "Ship the data layer; we'll wire it later"
- "Phase A is helpers; Phase B is the runtime"
- "Add a stub branch that exits 0"
- "Skeleton the crate; we'll fill it in"
- "Scaffold the module"

If the user confirms they really do want a scaffold despite the
rule, do it — but flag the worklist entry as
`[ ] Open scaffold only — no runtime reachability` so the false-
done signal never appears.

This rule supersedes "looks done if the tests pass" — passing
tests on an unreachable module is exactly the failure mode v3.x
exposed.

### 0.13 Quarterly retirement audit (Q65, 2026-05-25)

The retirement queue (DEAD-N epics) operates in two layers:

1. **Inline-per-epic** — every new epic explicitly names what it
   makes obsolete and retires it in the same cut (NF-5.1 + BUS-4
   precedent; BUS replaced GF-17 in one cut, gluster replaced the
   SSHFS-of-peer-dirs model alongside its own GF-* worklist).
2. **Quarterly fallback audit** — every three months, a fresh
   DEAD-N epic captures what the inline layer missed. The audit
   pass is the [[iteration]] skill's "worklist rescue" mode
   applied to retirement: scan for dead modules (`pub mod foo;`
   with zero external refs), misleading `[✓]` marks where the
   wiring didn't land, mockup-only "shipped" features, and
   deferred markers (`lands in a follow-up`, `wired in Phase N`,
   `deferred to`).

This rule replaces "retirement happens when we notice" — the
quarterly cadence is the forcing function. Calendar trigger:
first audit lands before the MackesDE for Workgroups 1.0 cut;
subsequent every-3-months.

### 0.14 Authority hierarchy (Q67, 2026-05-25)

When two locks contradict, the newer one wins silently. The
hierarchy for "which doc is canonical" on a given topic:

1. **Memory** (`~/.claude/projects/.../memory/*.md`) — operator's
   live preferences + cross-session continuity. Highest authority.
2. **`.claude/CLAUDE.md`** — operational rulebook (this file).
3. **`docs/AI_GOVERNANCE.md`** — platform identity + architectural
   compass (locked 2026-05-25 via the 100-Q tightening survey).
4. **`docs/design/*.md`** — per-epic design locks.
5. **`docs/PROJECT_WORKLIST.md`** body — actionable task state.

When in doubt: §0 master rule from `docs/AI_GOVERNANCE.md`
("Secure, Simple, Centerless Workgroup"). When AI_GOVERNANCE.md
and an older design doc contradict, AI_GOVERNANCE.md wins
(newer). When memory and AI_GOVERNANCE.md contradict, memory
wins (highest tier + likely newer). When CLAUDE.md is silent
on something, fall through to AI_GOVERNANCE.md.

### 0.15 Pre-release HW bench gate (Q69, 2026-05-25)

**REVERSES** the previous "HW carve-out items never gate a cut"
rule from [[feedback_no_cut_until_worklist_empty]] +
[[feedback_hardware_testing_epic]]. Per Q69 of the 100-Q
tightening survey:

> Each `cut release X.Y.Z` requires the HW bench items targeting
> that release to be run + green on operator hardware before the
> cut proceeds.

§0.6 cut-release shorthand step 0 (new): "Verify all HW carve-
out items tagged for this release in `docs/PROJECT_WORKLIST.md`
are `[✓]` with operator-confirmed bench results."

HW items still don't block individual feature commits (commits
ship as the worklist task completes); they block the **release**
itself. The HW backlog becomes the operator's last-mile
checklist before tagging.

This rule supersedes the previous "HW carve-out never blocks
cuts" framing in `.claude/CLAUDE.md` §0.7 + the two memory
files; both should be amended to reference §0.15 as the new
governing rule.

### 0.16 Platform feature lock (2026-05-26)

**Effective 2026-05-26 the platform is FEATURE LOCKED until the
next named release cuts.** No new features, no new epics, no new
survey-locked scope, no new design docs that grow the surface
area. The 324 active items already in `docs/PROJECT_WORKLIST.md`
are the complete release backlog.

**What this means for Claude:**

- **REFUSE new feature scoping** even if the operator asks for it.
  Surface this rule first, then ask whether the request belongs in
  a *post-release* backlog file or genuinely warrants lifting the
  lock. Don't silently extend the worklist.
- **Allowed without question:**
  - Bug fixes against any existing `[ ] Open` / `[>] In Progress` /
    `[✓] Done` task.
  - Polish, refactor, simplification, or scope **reduction** on
    locked work (R11-style "retire Kamailio" is the canonical
    example — it removes a container layer, doesn't add).
  - Worklist hygiene: retiring SUPERSEDED sections, marking
    obsolete tasks, fixing untagged titles, [[mackes-worklist-management]]
    audit passes.
  - Quarterly retirement audits per §0.13.
  - Hardware bench work + release-prep + the `cut release` flow.
  - Completing already-locked tasks end-to-end (this is the
    point of the lock — drain the queue).
- **Allowed with explicit operator override:**
  - Re-locking sections that became stale because of an
    architectural simplification (e.g., the v4.1.0 Voice & Video +
    v4.2.0 Voice PBX sections need a post-R11 re-lock to align
    with the direct PJSIP-to-Vitelity model). This is technically
    "new scope" because it rewrites task bodies, but its purpose
    is to bring the worklist *into sync* with already-locked
    architecture, not to expand it. Surface the rule, get one-line
    confirmation, proceed.
  - Critical security fixes that warrant scope creep (none
    anticipated; would be vanishingly rare).

**Trigger phrases to refuse (or surface this rule before
complying):**

- "Let's lock in feature X with an N-question survey"
- "Add an epic for Y"
- "Scope a new <surface>" / "design a new <subsystem>"
- "What if we also added Z?"
- "Survey N questions about <net-new feature>"
- "Brainstorm features for <new domain>"
- "Lift design-doc actions for <new doc> into the worklist"

If the operator confirms they really do want to lift the lock for
a specific request, do it — but surface that the lock is being
*temporarily lifted* for that scope and re-engage afterward. Don't
silently let the lock slide.

**When the lock lifts:**

The lock lifts the moment the next named release cuts (any version
tag pushed via the `cut release` flow per §0.6 — most likely 1.0,
but applies to any intermediate cut). Post-release the operator
may re-lock or open a new scoping window explicitly.

**Why this rule:** the active worklist already represents 324
items across 15+ epics. Continuing to add scope while draining is
the failure mode that produced the v3.x dead-module audit. The
lock is the forcing function that says "ship what's planned, then
plan again."

**Authority placement (§0.14 hierarchy):** this rule lives in
CLAUDE.md tier 2 and is mirrored in memory tier 1
([[feedback_platform_feature_locked]]). Memory wins if they
contradict. Operator-issued lock-lifts override both.

---

## 1. Worklist Rule

`.claude/skills/mackes-worklist-management/SKILL.md` is the canonical
project worklist protocol. Apply on every substantive change unless the
user explicitly says `DISABLE WORKLIST RULE`.

**Single worklist — the only one.** `docs/PROJECT_WORKLIST.md` is the
single durable worklist for the project. There is no parallel
tracker:

- The Claude Code in-session `TaskList` / `TaskCreate` / `TaskUpdate`
  tools are an **ephemeral scratchpad** for the active conversation
  only. Anything that needs to survive the session gets written to
  `docs/PROJECT_WORKLIST.md` directly. Do not let the in-session
  task list and the file diverge — the file wins on every conflict.
- No side trackers in CHANGELOG drafts, memory notes, or comments.
- **Design docs (`docs/design/*.md`) are not a parallel worklist.**
  Every actionable item from a locked design doc gets lifted into
  `docs/PROJECT_WORKLIST.md` as `[ ] Open`. The design doc keeps the
  rationale + locks; the worklist keeps the actionable list.

**No silent deferrals (user directive 2026-05-19).** Items are
either `[ ] Open`, `[>] In Progress`, `[✓] Done`, or `[!] Blocked`.
The `[~] Deferred` status is retired.

**When directives contradict, the newer one wins silently
(user directive 2026-05-19).** Don't maintain a separate
"Conflicts" section, don't ask the user to adjudicate older locks
— just update the affected worklist items in place and reflect
the latest decision. The original design doc can keep its old
text as historical context; the worklist tracks live policy only.

The status legend, task schema, and disable phrase are documented in
the skill file.

### 1.1 Release-tag schema on every worklist task (WF-5, 2026-05-21)

Every task title in `docs/PROJECT_WORKLIST.md` must declare its
target release via an explicit prefix. Two acceptable forms:

- **Version prefix:** `v2.0.1: <short title>` or
  `v2.1: <short title>` — fastest for one-off bug-fix / hotfix
  tasks pinned to a specific release.
- **Workstream prefix:** `UX-14: <short title>`,
  `CB-1.5.a: <short title>`, `XOrg-1.2: <short title>`,
  `HW-3: <short title>`, `WF-2: <short title>` — for tasks that
  belong to a named workstream whose target release is declared in
  the section header (e.g., the **UX-10..UX-23: Round 2 (v2.2
  scope)** header sets v2.2 as the target for every UX-N task
  inside).

**Active section filtering:** `target_release >= current_release`.
**History section filtering:** `target_release < current_release`.
Tasks without a recognizable prefix are a worklist-hygiene defect;
the WF-5.a pre-commit hook (when it lands) will block commits that
introduce an untagged task. Until WF-5.a ships, this is enforced by
manual review on every worklist edit.

Rationale: prefixless tasks accumulate as "is this in the next
release or three releases away?" ambiguity — the prefix kills that
ambiguity at write-time, not at scheduling-time.

## 2. Autonomy Rule (`complete-remaining-work`)

`.claude/skills/complete-remaining-work/SKILL.md` defines the autonomy
policy. When the user says "execute" / "continue" / "complete remaining
work" / "ship it", default behavior is:

- Highest-priority unfinished tasks first.
- Independent bundles in parallel.
- Mark `[>] In Progress` before substantive edits.
- Implement fully — no stubs, no placeholders.
- Add follow-up tasks for any tech debt or deferrals introduced.
- Escalate only when blocked by missing facts, required approvals, or
  contradictory requirements.

## 3. Code-style locks (project-specific)

Mirror the existing patterns rather than introducing new ones:

- **Module imports:** `from __future__ import annotations` at top.
  Standard library before third-party; third-party before mackes.*.
- **GTK panels:** must follow the Carbon refresh pattern — breadcrumb +
  `_page_title` + `_page_subtitle` + `_section_title` helpers. See
  `mackes/workbench/network/mesh_ssh.py` for the canonical layout.
- **Privileged ops:** route through `mackes.admin_session.AdminSession`,
  not raw `pkexec`. The session-unlock pattern (v1.4.0) holds creds for
  the whole session.
- **Memory writes:** when the user shares preferences, lock-in
  decisions, or project state worth carrying across conversations,
  follow the auto-memory protocol (every memory in
  `~/.claude/projects/-home-mm-Desktop-files-mackes-shell/memory/`).

## 4. Index — frequently-visited files

| Topic | Path |
|---|---|
| Carbon design tokens | `data/css/tokens.css` |
| Layout classes | `data/css/carbon-layout.css` |
| Shell window | `mackes/workbench/shell/sidebar_window.py` |
| Sidebar nav model | `mackes/workbench/shell/sidebar_window.py:_build_nav` |
| Wizard pipeline | `mackes/wizard/pages/apply.py` |
| Birthright steps | `mackes/birthright.py` |
| Admin session | `mackes/admin_session.py` |
| Mesh fabric core | `mackes/mesh_vpn.py`, `mesh_ssh.py`, `mesh_services.py` |
| RPM spec | `packaging/fedora/mackes-shell.spec` |
| Release workflow | `.github/workflows/release.yml` |
| Help docs | `docs/help/*.md` |
| Design source (v1.1.0 refresh) | `docs/design/v1.1.0-carbon-refresh/` |
