# 25-Q Tuning Survey — 2026-05-26

**Status:** Locked 2026-05-26.
**Authority:** Below AI_GOVERNANCE.md (which holds the master 100-Q
locks); newer locks here SUPERSEDE the older 100-Q locks per §0.14
authority hierarchy.
**Operator-issued lift:** This survey is an explicit lift of the
§0.16 platform feature lock for the 25-Q scope. After the lift, §0.16
re-engages.
**Goal statement (operator-typed 2026-05-26):**

> Complete releases. No Stubs or Coming Soon work. World Class
> Design. Always Follow Carbon Design or Material Design Design
> Concepts. Build Autonomously whenever possible.

This document captures every lock from the 25-question survey
fired this session via `AskUserQuestion`, the resulting design
implications, and the worklist additions that operationalize each
lock.

---

## Round 1 — Design system clarification

### Q1 + Q2 — Material Design only (supersedes the briefing's Carbon-split hypothesis)

| Lock | Value | Supersedes |
|---|---|---|
| Design system | **Material Design only** (icons + idiom + token vocabulary) | Q1+Q2 briefing hypothesis of Carbon-for-data-dense |
| Carbon status | Fully retired (icons retired Q43; design language now also retired) | Any historical Carbon-anywhere lock |

### Q3 — Reaffirm current hybrid

| Lock | Value |
|---|---|
| Visual baseline | **ChromeOS Classic chrome** (Q41) + **Material indigo accent** (Q45) + Material Symbols icons (Q43) + Roboto/Intel One Mono (Q44) + 4 px radius (Q42) + 150 ms motion (Q47) |
| Pivot scope | **No new pivot.** Current hybrid stays the platform-locked state |

**Implication:** The operator's "always follow Carbon or Material"
instruction is interpreted as **"follow Material; don't roll our
own bespoke design"** — Material is the single locked source.

---

## Round 2 — Design quality floor

### Q4 — Per-crate empty states, doc-enforced

Empty states stay per-crate (no shared `iced_components::empty_state`
widget). `docs/design/visual-identity.md` + `voice-and-tone.md` are
the enforcement layer. No new lint specifically for empty states.

### Q5 — Per-crate error states, doc-enforced

Same shape as Q4: no typed `ErrorSeverity` enum, no shared widget,
no new lint. Per-crate flexibility wins over platform cohesion.

### Q6 — Cite-required gate for visual commits (NEW lint #11)

Every visual commit must cite (a) a specific section of
`visual-identity.md` / `motion-language.md` / `chromeos-classic-spec.md`
AND (b) name a Material 3 reference target (Apple System Settings /
Linear / Raycast / Arc / Vercel dashboard) the change conforms to.
New `install-helpers/lint-visual-citation.sh` (pre-commit gate #11)
scans commit messages on commits touching `crates/mde-*/src/*.rs` +
`data/css/*.css` for the citation lines.

### Q7 — Broad design-token lint (NEW lint #12)

New `install-helpers/lint-design-tokens.sh` (pre-commit gate #12)
flags every hardcoded design token in code:

- Hex literals (`#1d1d1f`, `Color::from_rgb(...)`, `rgb(...)`)
  outside `data/css/tokens.css` + `crates/mde-theme/`
- Duration literals (`300ms`, `0.5s`, `Duration::from_millis(X)`
  for X != 150) outside `data/css/motion-vocabulary.css`
- Font names (`Roboto`, `Intel One Mono`) outside `crates/mde-theme/`
- Row-height literals (`28px`, `24px`, `32px`) outside the density
  constants in `crates/mde-theme/`

Snapshot-allow-listed for pre-existing violations at lint-introduction.

---

## Round 3 — No-stubs enforcement automation

### Q8 — Code-only no-stubs lint (NEW lint #13)

New `install-helpers/lint-no-stubs.sh` (pre-commit gate #13) blocks
commits containing:

- `todo!()`, `unimplemented!()`, `panic!("not yet")`,
  `panic!("todo")`, `panic!("unimplemented")`
- Commit-message strings: `wired later`, `phase 2`, `follow-up
  ships`, `stub for now`, `lands in N`, `deferred to`

User-visible string side is voice-tone territory (Q9).

### Q9 — Broad voice-tone coming-soon forbidden list

Extends `install-helpers/lint-voice.sh` (gate #6) with:

- `coming soon` / `coming-soon`
- `TBD` / `tbd`
- `WIP` / `work in progress`
- `not yet implemented`
- `placeholder`
- `experimental` (except documented Wayland-protocol contexts)
- `beta` / `alpha` (same exception)
- `preview` (same exception)
- `early access`
- `soon™` / `soon`

Blocks commit on any net-new hit.

### Q10 — Standalone runtime-reachability lint (NEW lint #14)

New `install-helpers/lint-runtime-reachability.sh` (pre-commit gate
#14) walks every `pub mod foo;` declaration in `crates/*/src/lib.rs`
+ `crates/*/src/*/mod.rs` and greps for at least one external
`foo::` reference. Blocks commit on zero hits. Runs only when the
commit touches `mod.rs` or `lib.rs`. Closes the §0.8 gate-7 manual
check by automation.

---

## Round 4 — Complete-releases enforcement

### Q11 — Auto-verify §11 roadmap before cut

New `make pre-cut-check` (Makefile target + `install-helpers/pre-cut-check.sh`)
greps the worklist for each §11 roadmap item's epic prefix
(BUS-1..7, GF-17 retired, DEAD-2, CR-*, INST-*, DM-*, EPIC-RETIRE-PY-WORKBENCH,
EPIC-RETIRE-DBUS, EPIC-UI-MATERIAL, EPIC-UI-PRESETS, EPIC-CAP-UPDATE,
DEAD-CADDY, EPIC-RENAME-QNM) and refuses to proceed if any non-HW
item is still `[ ] Open` or `[>] In Progress`. HW carve-outs check
per-bullet acceptance per Q13. Runs as step 0 of `cut release X.Y.Z`.

### Q12 — Hard block, no exception path

`make pre-cut-check` refuses with no operator override. The cut
release shorthand fails immediately when any §11 item is incomplete.
AI must drain the gap autonomously per /ship discipline before the
cut can proceed. The only legitimate path to revisit §11 scope is
operator-typed "amend Q91" via /plan.

### Q13 — Per-bullet HW acceptance checklist

Worklist schema extension: every HW-* task acceptance bullet gets
its own `[ ]`/`[✓]` toggle (already the current format; this lock
formalizes it). `make pre-cut-check` verifies every acceptance bullet
on every HW item is `[✓]` before allowing the cut. Operator marks
bullets as bench passes happen; no separate artifact file required.

---

## Round 5 — AI autonomy expansion

### Q14 — /ship standing auth for ALL §11 roadmap epics

§0.16 amendment: /ship gains standing autonomous-drain authority
across every §11 roadmap epic — BUS-1..7, DEAD-2, CR-*, INST-*,
DM-*, EPIC-RETIRE-PY-WORKBENCH, EPIC-RETIRE-DBUS, EPIC-UI-MATERIAL,
EPIC-UI-PRESETS, EPIC-CAP-UPDATE, DEAD-CADDY, EPIC-RENAME-QNM,
BUS-7.7-FED (new from Q24). HW-* stays operator-only. Visual changes
still pass the Q6 cite-required gate per commit.

### Q15 — Strict per-file staging + auto-rebase merger

CLAUDE.md §0.3 amendment:

- AI MUST stage via `git add -- <explicit-file>` (use the `--`
  separator) instead of multi-file `git add` (which auto-picks up
  unrelated working-tree state).
- On push reject → `git fetch` + `git rebase origin/main` + retry.
- When rebase reports conflicts on `CHANGELOG.md` or
  `docs/PROJECT_WORKLIST.md` only, AI auto-resolves via a
  section-aware merger (both modifications land in date-order).
- Non-doc conflicts escalate to operator.

### Q16 — No session budget; drain until done

§0.16 amendment: /ship runs autonomously until the targeted epic /
queue is fully drained OR /ship hits a blocker (failing gate after
auto-fix retries, missing fact, destructive op outside §0.9). No
periodic checkpoint. No time budget. No commit-count budget.

### Q17 — Auto-fix anything, no retry cap

§0.10 amendment: AI auto-fixes pre-commit gate failures iteratively
with no retry cap until the gate is green. The risk of infinite-
loop attempts on hard failures is accepted in service of the
autonomy goal. Operator can interrupt at any time.

---

## Round 6 — Cross-cutting process

### Q18 — Pre-commit lint enforces design-doc→worklist sync (NEW lint #15)

New `install-helpers/lint-design-doc-sync.sh` (pre-commit gate #15):
every commit touching `docs/design/<epic>.md` must also touch
`docs/PROJECT_WORKLIST.md` (assumed lift of new actionable items).
Otherwise refuse commit. Forces the design-doc → worklist sync to
happen at write-time, preventing design-doc actions from sitting
un-lifted.

### Q19 — Move superseded memory files to memory/archive/

Memory hygiene: on supersession (per AI_GOVERNANCE.md §14), AI
moves the superseded file to
`~/.claude/projects/.../memory/archive/<filename>.md` + adds a
`SUPERSEDED YYYY-MM-DD by [[new-file]]` banner at the top + drops
the entry from `MEMORY.md` index. Live load path stays clean;
context-on-recall via the archive dir.

### Q20 — Continuous retirement audit on every /ship cycle

§0.13 amendment: drop the quarterly cadence. Replace with: /ship
scans for retirement candidates on EVERY task completion — dead
`pub mod`, deferred markers in commit messages or worklist text,
mockup-only `[✓]` marks where the runtime-reachability gate
doesn't actually hold, design-doc actions never lifted. New
findings auto-file as worklist tasks under a rolling
`EPIC-DEAD-<YYYY-MM-DD>` section. Eliminates quarterly catch-up.

---

## Round 7 — Outlier resolution

### Q21 — Keep VoIP in core forever (Q92 spinout retires)

AI_GOVERNANCE.md §11 + Q92 amendment: revoke the v4.1 voice-video
spinout to `mde-voice` repo. VoIP stays bundled in mackes-shell
indefinitely. Aligns with the "Simple" master rule (fewer repos,
fewer release pipelines). Post-1.0 epic for voice-video evolution
stays in-tree.

### Q22 — 8-peer cap: hard limit + --override-cap flag

Q3 amendment: cap becomes a runtime-enforced hard limit in
birthright. Pairing the 9th peer fails by default with the error:

> MackesDE for Workgroups is sized for up to 8 peers (Q3 lock).
> Run `mackes-cli pair --override-cap` to bypass; document the
> exception in `docs/design/cap-overrides.md`.

The `--override-cap` flag bypasses the check. Each override gets
logged to the audit topic.

### Q23 — Phone gets full Nebula peer-hood (Q58 reopens — MAJOR EXPANSION)

Q58 reopens. Phone elevates from KDC2-bridged "beside the mesh"
to full Nebula peer:

- Phone holds the mesh passcode (counts as a peer for Q22 cap)
- Phone joins the Nebula overlay (Android Nebula client bundled)
- Phone gets GFS mount via FUSE-on-Android workaround (or SAF
  shim if FUSE is unrootable)
- Phone can publish to arbitrary Bus topics
- Phone subscribes to `fdo/#` for cross-peer notifications
- KDC2 stays as the optimization layer (battery-aware, push-
  efficient) — phone keeps using KDC2 for clipboard + SMS +
  battery + mpris while also being a Nebula peer

New epic: **PHONE-NEBULA-PEER** (~5-8 tasks):
1. Bundle Nebula Android client into the KDC2 app
2. Birthright pairing extension to mint phone certs
3. FUSE-on-Android (rooted) OR SAF shim (unrooted) for GFS
4. Update cap accounting + worklist tasks
5. Update audit log to expect phone publishes
6. Update voice-and-tone strings ("Mesh peer (phone)")

### Q24 — Federation ships complete in 1.0 (NEW EPIC)

Q35 + Q55 + §0.17 implication: federation must ship complete or
not at all. The OOB pairing UX was sketched but not designed; new
**BUS-7.7-FED** epic designs + ships the full federation flow
before 1.0:

1. Workbench accept-pair UI (text-typed mesh-A passcode → mesh-B
   passcode handshake → symmetric subscribe-only grant)
2. Nebula CA cross-sign protocol
3. Bus federation grant config at
   `~/.local/share/mde/bus/federation.yaml`
4. Federation audit log entries
5. Cap accounting (federated mesh-B peers DON'T count against
   mesh-A's 8-peer cap; they're external)

### Q25 — Document explicit "Fedora targeted + user-UID" MAC stance

New `docs/design/security-posture.md` locks the deliberate choice:

> MackesDE for Workgroups relies on Fedora's targeted SELinux
> policy + standard user-UID process isolation. Custom per-
> component SELinux policies are intentionally out of scope —
> the platform's daemons (`mackesd`, `mded`, `mde-bus`,
> `mde-portal`) run under the operator's user UID; the only
> capability scope is Nebula's `CAP_NET_ADMIN` which falls
> under Fedora's existing `nebula_t` domain.
>
> The flat-trust mesh model + bind-scope security boundary
> + Nebula transport encryption together carry the platform's
> intra-mesh threat model. SELinux policy authoring is an
> expert skill and custom policies are easy to get wrong;
> deferring to Fedora's targeted policy is the deliberate
> choice.

This closes §11.5 of the briefing document.

---

## Aggregated impact

### New pre-commit gates (now 15 total)

| # | Name | Lock | Catches |
|---|---|---|---|
| 1-5 | Existing | — | module / tests / ruff / RPM / CSS |
| 6 | voice-tone (extended) | Q9 | + coming-soon strings |
| 7 | legacy-mesh | — | tailscale/headscale/derper |
| 8 | dbus-shape | — | net-new D-Bus surfaces |
| 9 | material-symbols | Q43 | net-new Carbon icons |
| 10 | public-port | Q60 | net-new 0.0.0.0 binds |
| **11** | **visual-citation** | **Q6** | **missing design-doc cite on visual commits** |
| **12** | **design-tokens** | **Q7** | **hardcoded colors/durations/fonts/sizes** |
| **13** | **no-stubs** | **Q8** | **todo!() / unimplemented!() / deferral commit msgs** |
| **14** | **runtime-reachability** | **Q10** | **dead pub mod declarations** |
| **15** | **design-doc-sync** | **Q18** | **design-doc edits without worklist lift** |

### Amended CLAUDE.md sections

- **§0.3** — strict per-file staging discipline (Q15)
- **§0.7** — gate list grows to 15 (Q6/Q7/Q8/Q10/Q18)
- **§0.10** — auto-fix anything, no retry cap (Q17)
- **§0.13** — continuous retirement audit (Q20)
- **§0.15** — per-bullet HW acceptance checklist (Q13)
- **§0.16** — standing auth for all §11 epics (Q14); no session
  budget (Q16); operator-issued lift recorded for the 25-Q survey
- **§0.17** — `make pre-cut-check` hard block (Q11/Q12)

### Amended AI_GOVERNANCE.md sections

- **§3.1** — phone added as 4th-class peer with limited Nebula
  membership (Q23)
- **§6 Visual** — Q6 cite + Q7 token lints documented (already
  partial under Q63 update)
- **§7 Trust** — Q22 cap enforcement + Q23 phone trust evolution
- **§11 1.0 roadmap** — gains BUS-7.7-FED (Q24); item 6 expands
  (Phone Nebula peer scope); removes Q92 VoIP spinout note
- **§13 Read order** — gains the security-posture.md reference (Q25)

### New design documents

| Path | Lock | Source |
|---|---|---|
| `docs/design/security-posture.md` | Q25 | This survey |
| `docs/design/cap-overrides.md` | Q22 | Override audit format |
| (already exists, gets updates) `docs/design/v2.1-kdc2-native.md` | Q23 | Phone peer evolution |

### New epics in PROJECT_WORKLIST.md

Lifted into a new **`### EPIC-TUNING-25Q-2026-05-26`** section:

1. **TUNE-LINT-11** — New `lint-visual-citation.sh` (gate #11)
2. **TUNE-LINT-12** — New `lint-design-tokens.sh` (gate #12)
3. **TUNE-LINT-13** — New `lint-no-stubs.sh` (gate #13)
4. **TUNE-LINT-14** — New `lint-runtime-reachability.sh` (gate #14)
5. **TUNE-LINT-15** — New `lint-design-doc-sync.sh` (gate #15)
6. **TUNE-LINT-6E** — Extend `lint-voice.sh` with coming-soon list
7. **TUNE-CUT-PRECHECK** — `make pre-cut-check` + hard-block flow
8. **TUNE-HW-CHECKLIST** — Per-bullet HW acceptance checklist
9. **TUNE-AUTONOMY** — CLAUDE.md §0.3/0.10/0.13/0.15/0.16/0.17 amendments
10. **TUNE-CAP-ENFORCE** — Birthright cap check + `--override-cap`
11. **TUNE-MEMORY-ARCHIVE** — Memory archive workflow + MEMORY.md sweep
12. **PHONE-NEBULA-1..N** — Phone gets full Nebula peer-hood (new epic)
13. **BUS-7.7-FED-1..N** — Federation pairing UX (new epic)
14. **TUNE-SECURITY-POSTURE** — Write `security-posture.md`
15. **TUNE-RETIRE-Q92** — Drop the v4.1 spinout-to-mde-voice plan
16. **TUNE-RETIRE-CARBON** — Final Carbon-anywhere sweep

### Retirements triggered

| Retires | Reason |
|---|---|
| Briefing §11.5 (MAC posture gap) | Q25 closes the gap |
| Q92 VoIP spinout plan | Q21 keeps voice in core |
| Carbon-anywhere-in-platform hypothesis | Q1+Q2 Material-only lock |
| Quarterly DEAD-N audit cadence | Q20 continuous replaces |
| §0.16 BUS-only-exception scope | Q14 expands to all §11 epics |
| Q58 "phone beside the mesh" lock | Q23 phone peer-hood |

---

## Risks + open questions raised by this survey

### R1. Phone Nebula peer-hood (Q23) is technically aggressive

FUSE-on-Android requires root, which most operators don't have.
The fallback (Storage Access Framework shim) is NOT a real mount;
file access goes through Android's content-provider model. This
may require revising the GFS-mount scope on phone:

- Option A: phone gets full Nebula peer-hood but NOT GFS mount —
  Bus pub/sub + Nebula service access only. Files via a
  KDC2-style drop-folder pattern.
- Option B: require operator's phone to be rooted (small target
  audience).
- Option C: ship our own FUSE bridge as part of the KDC2 app
  (technically possible on rooted devices; needs design).

**Recommended:** Option A as a pragmatic 1.0 ship; revisit FUSE
in a 1.1 follow-up. Document in `phone-nebula-peer.md` design doc.

### R2. Federation cap accounting (Q24)

Federated mesh-B peers don't count against mesh-A's 8-peer cap.
But the OOB pairing UI needs a way to display "this peer is from
external mesh X." Surfaces a UX question: when viewing the peer
list, are external-mesh peers visually distinguished? Where?

**Recommended:** federated peers get a distinct icon variant in
the peer list + a "external mesh" badge in mde-portal Peer Card.
Lift into BUS-7.7-FED's UX scope.

### R3. Auto-fix anything (Q17) creates infinite-loop risk

The §0.7 gates include some that can't be auto-fixed (RPM build
failures on missing deps; test failures on real-world flakes).
The "no retry cap" lock means AI could spin indefinitely.

**Recommended:** add an EXPLICIT escape: AI emits a single
escalation message ("auto-fix attempted N times; cannot make
forward progress on gate X") once detection heuristics fire
(same fix produces same failure 3 times). This is a soft cap,
not a hard one — the loop continues if a NEW fix attempt is
plausible.

### R4. Cite-required gate (Q6) overhead

Every visual commit now needs a citation line. AI partners will
need to internalize the visual-identity.md section structure
deeply. There's a learning-curve cost; the first ~20 visual
commits will probably fail the lint until the discipline lands.

**Recommended:** add the citation format as a template in the
ship skill: "When committing visual changes, format:
`Cite: visual-identity.md §X.Y; ref: Apple System Settings`"

### R5. Continuous retirement audit (Q20) noise

Running the audit on every /ship cycle could surface a stream
of low-value findings (e.g., a `pub mod` that just landed and
hasn't been referenced YET because the consumer is in a parallel
session's pending commit).

**Recommended:** the audit skips findings where the `pub mod`
was added in the LAST commit (typical lag between landing the
module and wiring its consumer). If still unreferenced after
N commits, escalate.

---

## Sequencing

These additions should land in this order to minimize cascading
breakage:

1. **TUNE-MEMORY-ARCHIVE** — clean up memory state first
2. **TUNE-LINT-13** (no-stubs) — catches the discipline first
3. **TUNE-LINT-14** (runtime-reachability) — pairs with #13
4. **TUNE-LINT-15** (design-doc-sync) — enforces this survey's
   own lifts going forward
5. **TUNE-LINT-6E** (voice-tone extension) — extends existing
   lint, low risk
6. **TUNE-AUTONOMY** — CLAUDE.md amendments capture the rest
7. **TUNE-CUT-PRECHECK** + **TUNE-HW-CHECKLIST** — release-flow
   gates
8. **TUNE-LINT-11** (visual citation) — depends on operators +
   AI internalizing the format
9. **TUNE-LINT-12** (design tokens) — biggest snapshot allow-list
10. **TUNE-CAP-ENFORCE** + **TUNE-SECURITY-POSTURE** + **TUNE-RETIRE-Q92** + **TUNE-RETIRE-CARBON** — parallel
11. **PHONE-NEBULA-1..N** + **BUS-7.7-FED-1..N** — large new epics,
    parallel to the rest

Total estimated session count for all 16 new tasks: ~15-25
sessions of /ship at current cadence.

---

## How this survey was conducted

- 25 questions fired via `AskUserQuestion` one at a time per
  `[[feedback_question_workflow]]`
- Each question offered 4 options; operator answered all 25
- Mid-survey reversal on Q1+Q2 captured (initial answer "per-
  surface Carbon" was revised to "Material only")
- Questions structured in 7 rounds for cognitive grouping
- This document captures every answer verbatim with the
  resulting implementation note

The questions were generated specifically to advance the
operator's stated goals (complete releases / no stubs / world-
class design / autonomous building) by finding gaps where the
existing 100-Q locks were silent or ambiguous.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
