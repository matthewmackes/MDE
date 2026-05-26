---
name: complete-remaining-work
description: "[RETIRED 2026-05-25 per Q87] Use `ship` instead — it absorbs this skill's parallelization + completeness rules verbatim. This skill body is retained for slash-name back-compat."
---

> **RETIRED 2026-05-25 by Q87 of the 100-Q tightening survey.**
> Skill catalog consolidated to 3 (`plan` / `ship` / `release`).
> Use **`ship`** instead — it absorbs this skill's parallelization
> + completeness rules verbatim. This body is retained for
> slash-name back-compat.

# Complete Remaining Work

## Core Execution Rule

Treat the canonical project worklist as the only source of truth. Do
not maintain side plans.

## Workflow

1. Read the canonical worklist and identify the highest-priority
   unfinished tasks.
2. Split the next slice into independent bundles that can be executed
   in parallel.
3. Mark selected tasks/subtasks as `[>] In Progress` before substantive
   edits.
4. Implement fully. Do not leave stubs or placeholder logic.
5. Run focused validation for each bundle (typecheck/tests/build
   checks relevant to touched code).
6. Update task/subtask statuses to `[✓] Done` or `[✗] Blocked` with
   concrete notes.
7. If implementation introduces debt, risk, or deferred follow-up, add
   a new worklist task immediately with dependencies and acceptance
   criteria.
8. Continue to the next bundle without asking for confirmation unless
   blocked by missing inputs, permissions, or destructive actions.

## Parallelization Standard

- Prefer running independent file reads, searches, and validations in
  parallel.
- Prefer batching related edits that share context and test surfaces.
- Keep dependency chains sequential only where required by correctness.

## Completeness Standard

- Finish module + spec + CHANGELOG + import-smoke together when a
  change crosses boundaries.
- Add or update tests for new logic paths and edge cases.
- Keep behavior deterministic and rollback-safe.

## Decision Policy

- Assume yes for reasonable implementation choices that preserve safety
  and project conventions.
- Escalate only when blocked by missing facts, required approvals, or
  contradictory requirements.

## Mackes-Shell Notes

- The "build" step is `make rpm` (not `npm build`). It produces
  `rpmbuild/RPMS/x86_64/mackes-shell-X.Y.Z-1.fc44.x86_64.rpm`.
- The "release" step pushes a `vX.Y.Z` tag, which triggers
  `.github/workflows/release.yml` to publish to GitHub Releases.
- Privileged actions route through `mackes.admin_session.AdminSession`,
  never raw `pkexec`.
- New GTK panels MUST follow the Carbon refresh layout pattern (see
  `.claude/CLAUDE.md` §3).
