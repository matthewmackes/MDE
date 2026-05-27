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

- **Use `git add -- <file>` with explicit pathspecs** (Q15 of 25-Q
  tuning survey, 2026-05-26). The `--` separator removes pathspec
  ambiguity when a filename collides with a flag-like string;
  explicit paths prevent the same-session-stages-sibling-work
  failure mode that [[feedback_check_pre_staged]] documents.
  Never use `git add -A` / `git add .` / `git add -u` —
  pre-staged work from a parallel /ship session gets silently
  bundled. Even single-file edits use the explicit form.
- **`git commit <file...>` over `git add` + `git commit`** when
  publishing only your bundle's files in an environment where
  other paths may be staged by sibling sessions. The pathspec
  form ignores the index for unnamed paths.
- Never commit a file that likely contains secrets. Warn first.
- Never modify `git config`.
- Never touch `mackes/__init__.py:__version__`, `pyproject.toml`,
  `setup.py`, or `packaging/fedora/mackes-shell.spec` versions manually —
  they're bumped via the cut-release flow (see §0.6).

**Push-conflict auto-resolution** (Q16 of 25-Q, 2026-05-26):
when `git push` rejects on non-fast-forward, automatically run
`git fetch origin main` + `git rebase origin/main` (or `git pull
--rebase origin main`) + re-push. The /ship loop must not stop
on a routine non-fast-forward. Two-file classes get
auto-resolved on rebase conflict:

- **`docs/PROJECT_WORKLIST.md`** — accept both sides
  (worklist additions never structurally conflict; sibling marks
  + your marks are independent line edits). On rebase conflict,
  read both versions + manually splice the new additions; never
  drop a sibling's `[>] session=...` marker or `[✓]` close-out.
- **`CHANGELOG.md`** — same treatment for the top entry block.
  Sibling's entries + your entries are independent line additions;
  splice both into the next-version block.

For any other conflicted file: stop the loop, surface the
conflict to the operator. Don't guess.

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
it as a single executable command with eight ordered steps — execute all
eight without asking for confirmation between steps unless a step fails:

0. **Pre-cut check** (TUNE-7, 2026-05-26 per Q11 + Q12 of 25-Q
   tuning survey): `make pre-cut-check`. This script
   (`install-helpers/pre-cut-check.sh`) refuses if any §11
   roadmap-epic prefix from `docs/AI_GOVERNANCE.md` still has
   open or in-progress tasks in the worklist's Active section.
   **Hard block per Q12 — no operator override flag, no env-var
   bypass, no `--force`.** The legitimate path past this gate
   is the operator typing "amend Q91 to drop <epic>" (which
   removes the line from §11 + the script's `ROADMAP_PREFIXES`
   list). Per §0.15: also verify every HW-* acceptance bullet
   for this release is `[✓]` with operator-confirmed bench
   results — `make pre-cut-check` checks task-level marks; the
   per-bullet check is operator-typed.

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
1–4 (no commit, no tag, no push). Step 0 still applies if the request
is `cut release` shorthand — `build the RPM` standalone skips it.

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
10. **Public-port-bind lint** (added 2026-05-26 per Q60 +
    EPIC-SEC-PUBLIC-PORT-LINT): `install-helpers/lint-public-ports.sh`
    (if any source/config file is touched). Catches net-new
    listeners binding `0.0.0.0` / `[::]` / `EXPOSE` outside the
    locked allow-list. Per Q60 of the 100-Q survey, the only
    public ports MDE peers expose are **UDP/4242** (Nebula
    overlay) + **TCP/443** (Nebula HTTPS-tunnel fallback on
    lighthouses); every other listener must bind on the Nebula
    overlay interface. Pre-existing public binds (WoL broadcast,
    voice config defaults, port-availability probes, Nebula
    listeners themselves) are snapshot-allow-listed with
    inline rationale comments in the script.
11. **Visual-citation lint** (planned 2026-05-26 per Q6 + TUNE-9
    of the 25-Q tuning survey): `install-helpers/lint-visual-citation.sh`
    (if any `crates/mde-*/src/*.rs` or `data/css/*.css` is
    touched). Every visual commit must cite a `docs/design/<spec>.md`
    section + name a Material 3 reference target (Apple System
    Settings / Linear / Raycast / Arc / Vercel dashboard / Cursor)
    via a `Cite: <doc>.md §X.Y; ref: <target>` line in the commit
    body. Land via TUNE-9.
12. **Design-tokens lint** (added 2026-05-26 per Q7 of the 25-Q
    tuning survey + TUNE-10): `install-helpers/lint-design-tokens.sh`
    (if any `crates/mde-*/src/*.rs` or `data/css/*.css` is touched).
    Catches net-new hardcoded hex literals (`#1d1d1f`,
    `Color::from_rgb(...)`) outside the canonical token files
    (`data/css/tokens.css`, `data/css/motion-vocabulary.css`,
    `data/css/greeter.css`, `crates/mde-theme/`). Snapshot
    allow-list at `install-helpers/lint-design-tokens.allowlist`
    captures 139 pre-existing token-drift entries at lint
    introduction (file:line refs across mde-applets, mde-panel,
    mde-workbench, mde-portal, etc.). Cleanup tracked under
    EPIC-UI-MATERIAL.token-sweep; the allow-list shrinks as each
    entry ports to the canonical token files. Filters out Rust
    attribute syntax (`#[derive]` / `#[cfg]` / `#[allow]`) so
    they don't false-positive on the hex regex. Duration + font
    enforcement deferred to a follow-on iteration — the
    Color::from_rgb + hex literals are the highest-leverage
    catches first.
13. **No-stubs lint** (added 2026-05-26 per Q8 of the 25-Q
    tuning survey + TUNE-2): `install-helpers/lint-no-stubs.sh`
    (if any `crates/*.rs` is touched). Catches net-new `todo!()`,
    `unimplemented!()`, `panic!("not yet …")`, `panic!("todo …")`,
    `panic!("not implemented …")` in committed Rust code. The
    voice-tone lint (#6, extended in TUNE-5) catches the user-
    visible string side ("coming soon" / "TBD" / "WIP" / …).
    Together they enforce §0.12 + 25-Q Q8 + Q9 lock by automation
    rather than self-policing. Test-path heuristic (`/tests/` and
    `*_tests.rs`) excluded so test fixtures can use placeholders.
    Snapshot allow-list empty at lint introduction — no pre-
    existing violations.
14. **Runtime-reachability lint** (added 2026-05-26 per Q10 of
    the 25-Q + TUNE-3): `install-helpers/lint-runtime-reachability.sh`
    (if any `mod.rs` or `lib.rs` is touched). Walks every `pub mod
    foo;` declaration in `crates/*/src/lib.rs` + `crates/*/src/*/mod.rs`;
    requires at least one external `foo::` reference OR a
    `pub use foo::*` re-export. Catches the dead-module failure
    mode the v3.x audit (2026-05-22) surfaced by hand. Snapshot
    allow-list at `install-helpers/lint-runtime-reachability.allowlist`
    captures 53 pre-existing dead modules at lint introduction
    (TUNE-3.b cleanup epic tracks each entry; file shrinks over
    time). Any net-new dead module is a regression. Wall time
    ~60-120 s on the full repo.
15. **Design-doc → worklist sync lint** (added 2026-05-26 per
    Q18 of the 25-Q + TUNE-4): `install-helpers/lint-design-doc-sync.sh`
    (if any `docs/design/<epic>.md` is touched). Refuses the
    commit if `docs/PROJECT_WORKLIST.md` is not ALSO touched —
    forcing design-doc edits to lift actionable items into
    the worklist at write-time. Closes the failure mode where
    design-doc actions sit un-lifted for sessions. Override
    via `git commit --no-verify` only with operator approval +
    commit-body record (e.g., "design-doc typo fix, no new
    actionable items").

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
  `git fetch origin main`; merge or rebase; resolve conflicts per
  §0.3 auto-resolve rules for worklist + CHANGELOG; re-run the
  push. The loop continues — don't stop for routine pull-rebase
  cycles.
- **`make rpm` failed** → diagnose, fix, re-run; don't push the
  half-built tree.

**Auto-fix policy** (Q17 of 25-Q tuning survey, 2026-05-26):
when any pre-commit gate fails (lint, test, ruff, cargo check,
RPM build, voice-tone, public-port, etc.), **auto-fix without
asking** as long as the fix is in your bundle's scope. There is
no retry cap on the auto-fix loop itself; the loop continues
until the gate passes or you detect a same-fix-same-failure
pattern.

**Soft-escape on 3× same-fix-same-failure** (Q17 same lock):
track each (failing-gate, attempted-fix) pair within a single
bundle's pre-commit cycle. If the same fix produces the same
failure 3 times in a row, stop the auto-fix loop + surface to
the operator: "TUNE-6 SOFT-ESCAPE: <gate> failed 3× with the
same fix; need operator review." This prevents infinite loops
on architecturally-incompatible failures (e.g., a test asserts
something my fix can't satisfy).

The soft-escape counter resets per-bundle — don't carry across
bundles. Different bundles are independent escape contexts.

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

### 0.13 Continuous retirement audit (Q20 of 25-Q, 2026-05-26)

**REPLACES** the quarterly cadence locked at Q65 (100-Q,
2026-05-25). The retirement queue now operates in three layers:

1. **Inline-per-epic** — every new epic explicitly names what it
   makes obsolete and retires it in the same cut (NF-5.1 + BUS-4
   precedent; BUS replaced GF-17 in one cut, gluster replaced the
   SSHFS-of-peer-dirs model alongside its own GF-* worklist).
2. **Continuous per-/ship-cycle audit** (NEW per Q20, 2026-05-26)
   — every /ship invocation runs a lightweight retirement scan
   over the worklist + the modules touched in this session. Look
   for:
   - misleading `[✓]` marks where the wiring didn't land (the
     v3.x dead-module failure mode that §0.12 + DoD gate #7 are
     upstream prevention for)
   - mockup-only "shipped" features
   - deferred markers (`lands in a follow-up`, `wired in Phase
     N`, `deferred to`) that have aged past their stated trigger
   - `pub mod foo;` declarations with zero external refs
     (TUNE-3's lint-runtime-reachability.sh enforces this
     automatically once landed; until then, a manual mental
     check on every module-introducing diff)
   - SUPERSEDED memory files still in the live load path
     (TUNE-1's hygiene pattern: archive + banner)
   If the scan surfaces an actionable retirement, add a DEAD-N
   sub-task to the worklist + ship the retirement in the same
   /ship cycle if it's small + non-colliding.
3. **Quarterly fallback audit** (RETAINED as backstop) — every
   three months, a fresh DEAD-N epic captures what the
   continuous pass missed. Smaller scope now that the
   per-cycle audit catches the obvious cases. First audit
   landed before the 1.0 cut; subsequent every-3-months.

This rule supersedes Q65's "quarterly only" framing — Q20
moves the primary forcing function from calendar-cadence to
every-/ship-cycle continuous-pass, with the quarterly audit
staying as a backstop. The continuous pass catches drift
between epics; the quarterly catches drift between sessions.

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

**Per-bullet HW acceptance** (Q13 of 25-Q tuning survey,
2026-05-26): every HW-* task body uses per-bullet `[ ]` / `[✓]`
toggles on its acceptance criteria — not a single task-level
`[ ]` / `[✓]`. This is already the platform's standard worklist
schema for multi-bullet tasks; Q13 formalizes it for HW-* so
TUNE-7's `make pre-cut-check` can verify granular coverage
mechanically rather than guessing from a coarse task-level
mark.

The cut-release flow's step 0 (per §0.6) — once TUNE-7 ships
the `make pre-cut-check` script — will refuse to cut if any
single HW-* acceptance bullet for the target release is still
`[ ]`. The check is binary per-bullet, no partial credit.
Operators harvest the remaining bullets via `grep -A 20
"^### Hardware Testing" docs/PROJECT_WORKLIST.md` to see the
last-mile checklist.

**HW-* schema example** (formalized 2026-05-26 per Q13):

```markdown
- [ ] **HW-3: v1.0 — bench install on operator's primary peer**
  **Acceptance** (each bench-observable):
    - [ ] `mde-installer` boots from USB without errors on Lenovo X1 G10
    - [ ] First-login wizard completes without operator intervention
    - [ ] Nebula enrollment + mesh-home FUSE mount green within 90s
    - [ ] Bus broker visible to second peer within 30s
    - [ ] All four presets render correctly
```

Each `[ ]` bullet gets the operator's `[✓]` mark independently;
the task-level `[ ]` flips to `[✓]` only when every sub-bullet
is checked.

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

**Standing exceptions (2026-05-26):**

Four carve-outs from the lock are in effect through the next cut:

1. **BUS-1..BUS-7 build authorized** — /ship may drain BUS-*
   autonomously. See [[project_v6_x_mackes_bus]].
2. **R11 stale-section re-locks completed 2026-05-26** —
   v4.1.0 Voice & Video, v4.2.0 Voice PBX, and GF-17 sections
   retired in place (folded into VOIP-* + BUS-*).
3. **HW bench gate deferred to RC-completion** — §0.15 still
   requires operator-confirmed bench, but the operator's stated
   plan is to run HW-1..HW-4 once the queue is fully drained
   (right before the cut), not per-bundle. The cut-release
   shorthand step 0 still gates the actual cut on bench-green.
4. **All §11 1.0-roadmap epics build authorized** (Q14 of 25-Q,
   2026-05-26) — /ship has standing auth to drain every locked
   §11 epic (TUNE-*, BUS-*, GF-*, DEAD-*, INST-*, DM-*, CR-*,
   AIR-*, MON-*, Portal-*, VOIP-*, EPIC-RETIRE-*,
   EPIC-MASTER-*, EPIC-UI-*, NF-*, MESH-*, CONTAINER-*) without
   per-epic confirmation. **The single exclusion is HW-***
   (hardware bench items remain operator-typed per §0.15).
   Visual commits still keep the Q6 cite-required gate per
   commit (cite the design-doc + Material 3 reference target;
   TUNE-9 will enforce mechanically).

**No session budget** (Q18 of 25-Q, 2026-05-26): /ship runs
until blocked by a real obstacle (missing operator facts,
required-approval destructive action outside §0.9, worklist
drained, operator interrupt). There is no "stop after N
bundles" / "stop after N commits" cap. The loop continues per
the /ship skill's "continue without confirmation unless
blocked" workflow.

**Mid-flight survey-lift recording** (Q19 of 25-Q, 2026-05-26):
when the operator lifts the lock mid-/ship-cycle for a specific
scope (e.g., "lift the lock for VOIP-21"), record the lift
inline below in the "Net-new additions during the lock
window" list with this template:

```
- **YYYY-MM-DD — <TASK-ID>** (<one-line scope description>).
  Operator-issued lift, recorded in
  [[feedback_platform_feature_locked]]. Lock re-engaged
  immediately after.
```

Re-engage the lock the moment the lifted scope's worklist
entry closes. Don't let a lift cascade into "well, while
we're at it, ..." scope creep.

**Net-new additions during the lock window:**

- **2026-05-26 — VOIP-21** (Mesh-side Vitelity sub-account
  administration UI under VoIP Settings). Operator-issued lift,
  recorded in [[feedback_platform_feature_locked]]. Lock re-
  engaged immediately after.
- **2026-05-26 — Portal-41..Portal-59** (Round 12 i3/sway
  integration tightening, 25-Q survey + 19 new worklist tasks).
  Operator-issued lift for "review i3 documentation and our
  design" survey. Locks captured in
  `docs/design/v6.0-mde-portal.md` §16 and
  [[project_v6_0_mde_portal]] Round 12 section. Amends R3-Q44
  i3 contract (Mod+r) + supersedes R5-Q23 (scratchpad badge) +
  reframes R4-Q67 (5th micro-button). Lock re-engaged after
  Portal-59 lifts close 2026-05-26.
- **2026-05-27 — HYP-1..HYP-33** (v6.5 Hyprland compositor
  migration epic, 30-Q survey across 3 rounds: Strategy &
  scope / Portal R12 redesign + WM mechanics / Visual+motion+
  build+rollout). Operator-issued lift in /plan for full
  Hyprland epic. Locks captured in
  `docs/design/v6.5-hyprland-compositor.md` and
  [[project_v6_5_hyprland]]. Adds §11 1.0-roadmap item #19;
  supersedes the v2.0.0 sway lock + Portal-41..Portal-59 sway-
  IPC contracts + R12 Q44 keyboard-resize submap + Classic
  ChromeOS universal-flat element. Lock re-engaged after HYP-33
  lifts close.

**Why this rule:** the active worklist already represents 324
items across 15+ epics. Continuing to add scope while draining is
the failure mode that produced the v3.x dead-module audit. The
lock is the forcing function that says "ship what's planned, then
plan again."

**Authority placement (§0.14 hierarchy):** this rule lives in
CLAUDE.md tier 2 and is mirrored in memory tier 1
([[feedback_platform_feature_locked]]). Memory wins if they
contradict. Operator-issued lock-lifts override both.

### 0.17 NO INCOMPLETE RELEASES (operator directive 2026-05-26)

**Every cut release ships every locked §11 roadmap item from
`docs/AI_GOVERNANCE.md`. There are no minimal releases, no
scope-cut releases, no defer-to-next-minor releases.**

This rule strengthens Q91 (1.0 = maximum scope) + §0.15 (HW bench
required pre-release) + §0.16 (platform feature lock). It
eliminates the "scope cut" escape hatch from the release pipeline:

- **Never propose "lock a smaller 1.0"** when asked for the fastest
  path to ship. The locked Q91 scope IS the cut definition; the
  release plan is the whole §11 roadmap.
- **Never recommend "defer X to 1.1"** as a way to accelerate.
  Even if a remaining item is multi-bundle, multi-session, or
  needs HW bench — the answer is "grind the locked scope," not
  "shrink it."
- **When the queue is long** and the operator asks "what's the
  most direct path to completion": the answer reframes the
  question as execution-pace (more parallel sessions, more
  bundles per session, more HW bench passes) — NOT scope.
- **On contradiction with the §0 master rule** ("Secure, Simple,
  Centerless Workgroup"), this rule WINS. "Simple" means fewer
  concepts in the design, not a smaller cut at release time.

**Trigger phrases that should surface this rule before
complying** (do NOT silently propose scope cuts):

- "What's the fastest path to ship?"
- "Can we defer X to 1.1?"
- "Let's cut a minimal 1.0"
- "Should we shrink scope?"
- "Some of these are too big — what should we drop?"
- "Lock a smaller 1.0"
- "Pick the critical-path items only"

For any of these, surface this rule first. Then offer execution-
pace options (parallel-session coordination, multi-bundle work
plans, HW bench acceleration) — never scope reductions.

**The only legitimate path to revisit Q91** is the operator
explicitly typing "lift the lock for X" or "amend Q91." Claude
never proposes the amendment.

See [[feedback_no_incomplete_releases]] for the operator
directive that locked this rule.

**Hard-block via `make pre-cut-check`** (Q11 + Q12 of 25-Q
tuning survey, 2026-05-26): when TUNE-7 lands the
`install-helpers/pre-cut-check.sh` script + the matching
Makefile target, §0.6 cut-release shorthand step 0 invokes
`make pre-cut-check` and refuses to proceed if the script
exits non-zero. The script greps the worklist for each §11
roadmap-item epic prefix + verifies every task under those
prefixes is `[✓]` (or carries the per-bullet `[✓]` schema
per §0.15 for HW-* tasks). Per Q12: **no operator override
path**. The lock is mechanical, not advisory. The legitimate
path to cut past an open §11 item is to amend Q91 (operator
types "amend Q91 to drop <item>"); the script never bypasses.

Until TUNE-7 ships, §0.17 is enforced by manual cross-check
against AI_GOVERNANCE.md §11 at cut time — `cut release` will
refuse manually if any §11 item is open. The TUNE-7 script
just mechanizes what's already operator-policy.

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
