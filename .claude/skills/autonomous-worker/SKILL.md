---
name: autonomous-worker
description: "[RETIRED 2026-05-25 per Q87] Use `ship` instead — it absorbs this skill's single-task autonomy loop. This skill body is retained for slash-name back-compat."
---

> **RETIRED 2026-05-25 by Q87 of the 100-Q tightening survey.**
> Skill catalog consolidated to 3 (`plan` / `ship` / `release`).
> Use **`ship`** instead. This body is retained for slash-name
> back-compat.

# Autonomous Worker

Adapted from a generic autonomous-loop pattern to respect this
project's `.claude/CLAUDE.md` rulebook. Where the generic pattern says
"auto-commit and push every step," this skill defers commits/pushes to
the user per §0.5.

## Triggers

- "start autonomous mode"
- "run worklist"
- "continue autonomously"
- "execute" / "ship it" / "complete remaining work"

When invoked, also consult [[complete-remaining-work]] for the
parallelization + completeness policy and
[[mackes-worklist-management]] for the worklist schema.

## Execution Pipeline

1. **Read the canonical worklist.** Open `docs/PROJECT_WORKLIST.md` and
   identify the highest-priority `[ ] Open` item (respecting any
   declared `Depends:` chains). Never invent a side worklist; never
   read `.claude/worklist.md` — it does not exist in this project.
2. **Claim the task.** Edit the worklist in place to flip the task from
   `[ ] Open` to `[>] In Progress` before substantive edits begin. This
   is the restart-safe handoff signal.
3. **Implement.** Write the code, follow the project's code-style locks
   (`.claude/CLAUDE.md` §3), and run focused validation:
   - Module import smoke for every Python module touched.
   - `make test-nodeps` if `tests/` touched.
   - `make rpm` if `packaging/`, `setup.py`, `pyproject.toml`, `data/`,
     or `mackes/birthright.py` touched.
   - `install-helpers/lint-css.sh` if `data/css/` touched.
4. **Capture follow-ups.** If implementation introduces debt or a
   deferred decision, add a new `[ ] Open` task to the worklist with
   acceptance criteria — do not silently defer.
5. **Mark done.** Flip the task to `[✓] Done` (project's status legend
   — never `[x]`). If a gate from §0.8 Definition of Done is
   incomplete (e.g. not yet committed/pushed), leave the task at
   `[>] In Progress` with a one-line note on which gate is pending.
6. **Pause for commit authorization.** Stage the relevant files
   (explicit paths, never `git add -A`), draft a HEREDOC commit
   message in the project's style, and surface it to the user. **Do
   not run `git commit` or `git push` without explicit user
   authorization** — `.claude/CLAUDE.md` §0.5 is a hard lock and a
   single approval is never a standing license.
7. **Loop.** Once the user authorizes the commit/push (or explicitly
   says "batch the commits, keep going"), return to step 1 and pick
   the next open task. Stop when no `[ ] Open` items remain at the
   current priority or a blocker per §Constraints fires.

## Constraints & Safety

- **Self-correct on failures.** If a test, lint, build, or shell
  command fails, read the error, fix the underlying issue, and retry.
  Don't ask the user about routine, fixable failures.
- **Hard stops that require human authorization.** Pause and surface
  the issue when:
  - A commit or push is ready (§0.5).
  - A destructive operation is needed (§0.9 — force-push, `reset
    --hard`, `rm -rf` outside build dirs, GitHub state changes).
  - A pre-commit hook fails repeatedly with no clear fix.
  - The next task contradicts a locked design decision and the
    newer-wins rule (§1) can't resolve it alone.
  - A `cut release` step fails (§0.6).
- **No silent deferrals.** The `[~] Deferred` status is retired
  (user directive 2026-05-19). Items are `[ ]`, `[>]`, `[✓]`, or
  `[!] Blocked`.
- **Worklist file wins on every conflict.** If the in-session
  `TaskList` and `docs/PROJECT_WORKLIST.md` diverge, sync the file
  first.

## End of Turn

Return control to the user when:

- Every `[ ] Open` task at the current priority is `[✓] Done`, **or**
- A commit/push is queued and awaiting authorization, **or**
- A hard stop above has fired.

End-of-turn report: one or two sentences. What shipped, what's queued,
what's blocked.
