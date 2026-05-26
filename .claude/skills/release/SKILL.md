---
name: release
description: Execute the `cut release X.Y.Z` 7-step shorthand once operator types the trigger — version bump (4 files), CHANGELOG entry, smoke test, `make rpm`, commit, tag, push (dual-remote), and watch the GitHub workflow. **Will refuse to run if any HW carve-out item targeting the release version is still `[ ]` Open per CLAUDE.md §0.15 (Q69 lock).** Use ONLY when operator types `cut release X.Y.Z` or `cut release` shorthand. New 2026-05-25 per Q87 of the 100-Q tightening survey. Sister skills: `plan` (design first), `ship` (execute the queue first).
---

# Release

The release-cut skill. Executes the canonical `cut release X.Y.Z`
flow per CLAUDE.md §0.6 + the new §0.15 HW bench gate. **Refuses
to run if HW items for the target release are still Open.**

New skill 2026-05-25 per Q87 of the 100-Q tightening survey.

## Triggers

- "Cut release X.Y.Z" / "cut release" (operator-typed only)
- "Tag v1.0.0" / "tag and push v1.0"
- "Ship the 1.0 cut"

**Never auto-trigger.** Per the §0.5 standing rule + memory
[[feedback_push_commit_auth]], `cut release` is operator-typed
only; this skill executes it without per-step confirmation
once invoked.

## Pre-flight (gate §0.15)

**Before executing any cut step**, verify:

1. Every non-HW worklist task targeting the release version is
   `[✓] Done` (per memory [[feedback_no_cut_until_worklist_empty]]).
2. Every HW carve-out task targeting the release version is also
   `[✓] Done` with operator-confirmed bench notes (per CLAUDE.md
   §0.15, Q69 lock).
3. `git status --short` is clean (no pre-staged work that doesn't
   belong in the cut).

If any of (1)/(2)/(3) fails, **stop + report what's missing**.
Do not proceed with the cut.

## The 7-step shorthand (CLAUDE.md §0.6)

Once pre-flight passes, execute without per-step confirmation:

1. **Bump version** in 4 files:
   - `mackes/__init__.py:__version__`
   - `pyproject.toml:version`
   - `setup.py:version=`
   - `packaging/fedora/mackes-shell.spec:Version`

2. **CHANGELOG entry** at the top of `CHANGELOG.md` under
   `## X.Y.Z — <one-line summary> (YYYY-MM-DD)`. Describe what
   shipped. For 1.0, the entry is "MackesDE for Workgroups 1.0
   — rebrand cut" per Q72.

3. **Smoke test:** `python3 -c "import mackes; print(mackes.__version__)"`.

4. **Local RPM build:** `make rpm`. **Always go through `make rpm`**,
   never invoke `rpmbuild` with `--short-circuit` (stamps an
   unsatisfiable `rpmlib(ShortCircuited)` dep). If short-circuit
   guard fires, blow away `rpmbuild/{BUILD,BUILDROOT,RPMS,SRPMS}`
   + rerun `make rpm` without flags.

5. **Commit** via the §0.4 HEREDOC format with the Q85 co-attribution
   trailer.

6. **Push + tag:**
   ```bash
   git push origin main && git push mde-x main && \
       git tag -a vX.Y.Z -m "MackesDE for Workgroups X.Y.Z — …" && \
       git push origin vX.Y.Z && git push mde-x vX.Y.Z
   ```
   Tag annotation uses the operator-canonical product name
   ("MackesDE for Workgroups") per Q71 — not the legacy "Mackes
   Shell" / "Mackes XFCE Workstation" / "Mackes Desktop
   Environment" strings.

7. **Watch the workflow:** `gh run watch <id> --exit-status`, then
   confirm with `gh release view vX.Y.Z`.

## Cut-for-testing variant

If the operator types `build the RPM for testing` instead of
`cut release X.Y.Z`, run steps 1-4 only (no commit, no tag, no
push). The intent is to produce a fresh RPM in `rpmbuild/RPMS/`
for bench validation, not to publish.

## Standing authorization (per Q83)

Per memory [[feedback_push_commit_auth]] (expanded by Q83):
once operator types the trigger, this skill executes the full
shorthand without per-step pauses. Standing auth covers commit
+ push + `make rpm` + tag + watch.

## Companion skills

- `plan` — used before release scope is set
- `ship` — used to drain the queue before release becomes
  eligible

## Failure modes

- **Step 4 `make rpm` fails** → diagnose, fix, re-run; do NOT
  push the half-built tree. Drop back to `ship` to fix the
  underlying issue, then re-attempt the cut.
- **Step 6 push fails (non-fast-forward)** → `git fetch
  origin main`; merge or rebase; resolve conflicts; re-run the
  push.
- **Step 7 workflow fails** → `gh run view <id> --log` to
  diagnose; the release tag is published but the GitHub
  release artifact may not be. Operator decides whether to
  re-trigger or move on.
