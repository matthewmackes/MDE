---
name: hyprland-cut
description: Orchestrate the v6.5 Hyprland compositor migration as ONE atomic landing — the HYP-1..HYP-33 cluster cannot drain bundle-by-bundle on main because porting any worker to hyprland-rs regresses the live sway path and deleting swayipc-async breaks every un-ported consumer. Runs the cut in a git worktree (main stays green/sway-based until merge), lands the HYP-* sub-tasks in dependency order, gates on full-workspace cargo green + the HYP-31 operator HW bench, then squash-merges + hands to `release` for the v6.5 tag. Use when the operator types "land the v6.5 cut", "execute the hyprland migration", "do the compositor swap", "ship v6.5". NOT a /ship drain. Sister skills: `plan` (design first), `ship` (isolated bundles), `release` (the tag).
---

# Hyprland Cut

The v6.5 compositor-migration orchestration skill. Lands the
`HYP-1..HYP-33` cluster (`docs/design/v6.5-hyprland-compositor.md`,
30-Q lock 2026-05-27) as **one coordinated cut**, not a `/ship`
drain.

## Why this can't be a `/ship` drain

`/ship` requires every commit to ship complete + green on `main`
(§0.12). The v6.5 cluster violates that at the *individual-task*
level:

- **Porting one worker to hyprland-rs regresses the live path.**
  `main` runs sway today. The moment `workspace_namer` (HYP-9) talks
  to a Hyprland socket that doesn't exist in the running sway
  session, that worker dead-loops. A lone port breaks working
  behavior.
- **Deleting `swayipc-async` (HYP-3) breaks every un-ported
  consumer.** It can only land *after* all of HYP-9..HYP-18 port.
- **Q1 + Q8 lock a hard cut**: "one named version flips the
  compositor", "sway-cluster deleted same commit hyprland-cluster
  lands". The design doc's own rule: *"hyprland-rs ports happen in
  the same bundles as sway-IPC removals; no co-existence."*
- **Q10: the HW bench is the gate**: "broken Hyprland never tags
  v6.5; no field rollback path."

So the cut lands **atomically**: `main` stays green + sway-based
until the entire cluster merges at once, the operator benches it on
real hardware, and only then does the v6.5 tag cut.

## Trigger

Operator-typed only — one of:
- "land the v6.5 cut" / "execute the hyprland migration"
- "do the compositor swap" / "ship v6.5"

**Never auto-trigger.** This is a destructive, hard-to-reverse,
multi-session landing. Treat it like `release`: operator says go,
then run without per-step confirmation until a real blocker.

## Pre-flight

1. **Read the design doc** (`docs/design/v6.5-hyprland-compositor.md`)
   + the live HYP-* state in `docs/PROJECT_WORKLIST.md`. The §11
   roadmap item #19 is the cut definition; the worklist is the live
   task state.
2. **Census the HYP-* board.** Build the done / in-flight / open
   map. As of 2026-05-28 the shipped foundation is: HYP-2 (hyprland-rs
   dep), HYP-5.a/b (baseline conf + birthright seed), HYP-7 (motion),
   HYP-8.5 + .birthright + .watch (tag manifests), HYP-13 (mouse
   resize), HYP-19/20/23/24/26 (baseline visual locks), HYP-25.a
   (VOIP windowrule), HYP-27.a (VRR), HYP-29 (greeter), HYP-30 (docs),
   plus the interim **sway-bridges** (HYP-9/10/11/22.sway-bridge +
   HYP-10.layout-bridge + HYP-AutoMark.sway-bridge) that keep tag
   manifests working on sway TODAY. The sway-bridges are throwaway —
   the real hyprland-rs ports replace them inside the worktree.
3. **Scan for sibling `[>] session=` markers** on HYP-* tasks (Q70 +
   Q86). In-flight at last census: HYP-5 (ship-CG), HYP-25 (ship-CH),
   HYP-27 (ship-CG), HYP-33 (ship-CH). **Do not claim a sub-task
   another session holds** — either wait for it to close or fold its
   shipped pieces into the worktree once merged to main. Re-fetch
   `git log` + `git status` before claiming anything.
4. **Confirm the foundation is buildable.** `cargo check --workspace
   --all-features` green on `main` before you start (the worktree
   forks from a known-green base).

## The worktree

The whole cut lives in an isolated git worktree so `main` never goes
red and sibling `/ship` sessions keep draining unrelated work:

```bash
git worktree add ../mde-hyprland-cut -b hyprland-cut origin/main
cd ../mde-hyprland-cut
```

- Branch name: `hyprland-cut` (deleted after merge).
- WIP commits inside the worktree may be red mid-flight — that's the
  point. The invariant is **green at the END**, then one merge to
  `main`.
- Per the EnterWorktree tooling if available, prefer it; otherwise the
  raw `git worktree` above. Clean up with `git worktree remove` after
  merge (or it auto-cleans if no changes).

## Landing sequence (dependency order)

Land inside the worktree in this order. Each layer must
`cargo check -p <crate>` clean before the next; the **whole
workspace** only needs to be green after the final sweep.

**Layer 1 — make Hyprland available** (nothing to port against
without this):
- **HYP-1**: CI builds Hyprland from a pinned tag into
  `rpmbuild/SOURCES/hyprland-bundle.tar.gz`; the `mde` RPM gains
  `Provides: hyprland`, `hyprctl`, `hyprland-plugin-api`. Edits
  `.github/workflows/release.yml` + the spec.
- **HYP-4**: `crates/mde-hypr-plugin/` skeleton — Cargo crate wrapping
  a cmake-built C++ plugin via `build.rs`; the 3-subsystem stub
  (custom layout / window rules / event-bridge) per the §10.1
  simplification re-lock.

**Layer 2 — mded marks producer** (HYP-2 ✓ unblocks):
- **HYP-14**: `mackesd::workers::marks_state` — owns per-window marks,
  subscribes to `hyprctl sockets2`, exposes `action/marks/*` on Bus,
  publishes `event/marks/<addr>`. Unblocks 15/21/22.

**Layer 3 — worker ports** (HYP-2 ✓ unblocks; each REPLACES its
sway-bridge):
- **HYP-9** workspace_namer, **HYP-10** workspace_router, **HYP-11**
  tag_autostart, **HYP-16** templates `hyprctl --batch`, **HYP-17**
  session_persist — each swaps `swayipc_async` for `hyprland` crate
  event-listener + dispatch. Pure-fn helpers survive; the IPC layer
  is rewritten.

**Layer 4 — plugin features + visual** (need HYP-4 / HYP-14):
- **HYP-12** custom MDE layout (plugin `IHyprLayout`), **HYP-18**
  window rules (plugin), **HYP-15** mark pills (mde-portal layer-shell
  overlay), **HYP-21** per-elevation M3 shadows (mded `elevation`
  worker), **HYP-22** per-tag border (mded `border_colors` worker).

**Layer 5 — the sweep (atomic break)** — only after every consumer
above ports:
- **HYP-3**: delete `swayipc-async` from every `Cargo.toml`; `grep -rl
  swayipc_async crates/ src/` returns zero. This is the point of no
  return inside the worktree.
- **HYP-8**: `git rm -r crates/mde-applets/sway-cluster/`; create
  `hyprland-cluster/`.
- **HYP-32**: voice-tone string sweep — `grep -rn "sway\|i3"
  crates/mde-*/src` returns only retraction comments;
  `install-helpers/lint-voice.sh` green; CHANGELOG records the flip.

**Layer 6 — installer + settings + follow-ons**:
- **HYP-28**: `mde-installer` drops sway, seeds the bundled Hyprland.
- **HYP-6** mde-config writes hyprland.conf; **HYP-8.6** tag-manifest
  Settings UI; **HYP-5.c/d/e** (GFS replication / EDID overlays /
  reload watcher); **HYP-25.b/c**, **HYP-27.b** — fold in if their
  parent sessions have closed, else leave for a follow-up bundle.
- **HYP-33** + **HYP-33.followup**: re-point any remaining Portal-4N
  swayipc acceptance bullets.

## Green gate (before merge)

The worktree must clear ALL of these before it touches `main`:

1. `cargo build --workspace --all-features` — 0 errors.
2. `cargo test --workspace` — green.
3. `grep -rl swayipc_async crates/ src/` — zero hits (HYP-3 done).
4. `grep -rn "sway\|i3" crates/mde-*/src` — only retraction comments
   (HYP-32 done).
5. All §0.7 pre-commit gates: no-stubs, runtime-reachability,
   voice-tone, legacy-mesh, dbus-shape, material-symbols,
   public-ports, design-tokens.
6. `make rpm` — exits 0; the RPM `Provides: hyprland` (HYP-1). Never
   `--short-circuit` (§0.6 ShortCircuit guard).

If any gate fails, fix inside the worktree. The cut does not merge
red.

## Merge

```bash
cd ../mackes-shell           # back to the main checkout
git merge --squash hyprland-cut
git commit -m "<v6.5 cut HEREDOC message>"   # §0.4 format + co-attr
git push origin main && git push mde-x main  # §0.2 dual-remote
git worktree remove ../mde-hyprland-cut
git branch -D hyprland-cut
```

- **One squash commit** so `main`'s history shows the compositor flip
  as a single reviewable landing (sway → Hyprland in one diff).
- Update every landed HYP-* to `[✓]` in the worklist with concrete
  notes (which layer, what shipped) in the same push.
- CHANGELOG: one v6.5 cut block summarizing the flip.

## The bench gate (HYP-31) — operator-typed, blocks the tag

Per Q10 + §0.15, the v6.5 tag CANNOT cut until **HYP-31** is green on
real hardware. Each bullet is operator-typed (§0.15 per-bullet `[✓]`):
Hyprland reaches initial state <2s on the bench peers; plugin loads +
3 subsystems respond; all 7 mackesd workers survive a 30-min stress
run; VOIP fullscreenstate + hangup gesture work; GFS-replicated
hyprland.conf survives reboot + peer-add; per-tag borders correct
across ≥3 tags; no animation regression; battery within 5% of sway
baseline.

**Do not tag v6.5 until the operator marks every HYP-31 bullet
`[✓]`.** Surface the remaining bullets: `grep -A 20 "HYP-31"
docs/PROJECT_WORKLIST.md`.

## The tag

Once the merge is on `main` + HYP-31 is operator-green, hand to the
`release` skill (or the §0.6 `cut release 6.5.0` shorthand). The
release skill's own §0.15 pre-cut-check enforces the HW gate again as
a backstop. **Never auto-cut** — `cut release` stays operator-typed.

## When to stop

- A sub-task is held by a sibling `[>] session=` and isn't closing —
  fold what's shipped, defer the rest, note it; don't fight for the
  lock.
- HYP-1 / HYP-4 (the foundation) aren't landable yet (missing pinned
  Hyprland tag, no cmake toolchain decision) — escalate; the cut
  can't proceed without Hyprland actually building.
- The green gate can't be reached after a soft-escape (§0.10 3×
  same-fix-same-failure) — surface to the operator.
- HYP-31 bench is red — the cut holds; report which bullet failed.

## Companion skills

- `plan` — if a v6.5 design fork emerges mid-cut, switch to `plan`.
- `ship` — for the isolated non-HYP work that keeps draining on `main`
  in parallel sessions while the cut cooks in its worktree.
- `release` — the `cut release 6.5.0` tag, after merge + bench-green.
