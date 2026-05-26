# MackesDE for Workgroups — AI Governance Document

**Locked:** 2026-05-25 via 100-Q tightening survey
**Supersedes:** `docs/PLATFORM_DESIGN_BRIEF.md` (kept as legacy reference)
**Authority:** Memory > CLAUDE.md > **this doc** > other design docs >
worklist body (newer locks always win per §0.67)

This is the single source of truth for AI design partners working on
MackesDE for Workgroups. Read this first; consult others on demand.

---

## 0. Master rule

> **"Secure, Simple, Centerless Workgroup."**

When two locks conflict and the survey didn't resolve it, pick the
option that best embodies all four words. (Q1 + Q100)

---

## 1. Identity & scope

| | Lock | Source |
|---|---|---|
| **Product name** | MackesDE for Workgroups | Q71 |
| **Casual short form** | MackesDE | Q73 |
| **Code/internal** | MDE (binaries `mded`, `mde-*`; D-Bus `dev.mackes.MDE.*`) | Q73 |
| **Identity** | Secure, Simple, Centerless Workgroup | Q1 |
| **Workgroup unit** | 1 person, 3-8 of their own devices | Q2 |
| **Fleet cap** | **8 peers** (tightened from 16) | Q3 |
| **Geographic scope** | Mixed LAN+WAN, always-reachable | Q4 |
| **Hardware** | x86_64 desktops/laptops + SBC lighthouses + KDC2-bridged Android | Q5 |
| **iOS** | Not supported | Q40 |
| **Distribution** | Operator + small self-supporting circle | Q6 |
| **Brand permanence** | Permanent; rebrand cut at 1.0 | Q71 |

---

## 2. Bundle policy

**Rule:** Mesh-integrated only. Test = "does this gain value from
cross-peer state?" If no → not bundled. (Q7)

| Component | Decision | Lock |
|---|---|---|
| Gluster mesh-home | Core | v5.0 |
| ntfy + Bus | Core | BUS |
| KDC2 phone bridge | Core | v2.1 |
| Netdata aggregator | Core | v2.6 MON-* |
| Airsonic music client | **Core** (mesh-library aware) | Q9 |
| Voice & Video (PJSIP + Vitelity) | **Ships in 1.0; stays in core forever** | Q8 + Q21 (25-Q) + R11 (v6.0) |
| Caddy gateway | **Retire** | Q10 |

---

## 3. Architecture

### 3.1 Sync substrates — TWO only

| Substrate | Holds | Mount |
|---|---|---|
| Gluster mesh-home (one volume + arbiter brick on lighthouses) | XDG files + MDE-Workgroup coordination | `~/Documents` + `~/Pictures` + `~/Music` + `~/Videos` + `~/Downloads` + `~/.mde-mesh/<peer>/` |
| Bus (ntfy brokers + GFS persistence + per-topic file tree) | Events + clipboard + audit + notifications | `~/.local/share/mde/bus/<topic>/<ulid>.json` |

QNM-Shared **retires** as term + path. Replaced by **MDE-Workgroup**.
(Q14 + Q21 + Q22 + Q77)

### 3.2 Transport

Simplified: **2 TransportKind variants** — `Nebula` (with internal mode
field: Direct / Https443 / LighthouseRelay) + `KdcTls`. (Q11)

### 3.3 IPC default — **Bus for everything**

- Commands → `action/<domain>/<verb>` topics (Q31)
- Responses → `reply/<original-ulid>` topics, publisher subscribes
  before publishing (Q32)
- Events → domain topics (e.g. `mesh/conflict`, `mon/cpu`)
- Slow state (≤1/min) → file polling against gluster mesh-home (Q13)
- D-Bus **fully retires by 1.0** (Q20 + Q96); only `org.freedesktop.*`
  FDO interop survives

### 3.4 Worker pattern (mded)

- **Default `RestartPolicy::Always`** (Q16); exceptions documented inline
- **Pure-fn extraction expanded to ALL IO** — argv + file + DBus +
  network. Not just argv (Q17)
- **No subprocess-supervised Python daemons by 1.0** (Q15 + Q95)

### 3.5 D-Bus services (interim only — retired by 1.0)

Pre-retirement taxonomy: **5 domain services** (Mesh / Shell /
Notifications / Voice / Files). Post-1.0: every MDE-internal D-Bus
surface becomes Bus action/reply topics. (Q18 + Q96)

### 3.6 UI toolkit

**"Create the Leanest Path"** — pick the simplest viable stack per
surface. Iced when sufficient; libcosmic when its widget fits; smithay
raw when Iced is too heavy; GTK4/Qt only with explicit justification.
(Q19)

---

## 4. Sync + storage

| Decision | Lock | Source |
|---|---|---|
| **Gluster volume** | One volume `mesh-home`, arbiter brick on lighthouses | Q22 |
| **Coordination path** | `~/.mde-mesh/<peer>/` via `mde-mesh-mount@.mde-mesh.service` | Q21 |
| **Stub cap** | 5 GB; per-file pin xattr escape | Q24 |
| **Version history / snapshots** | CUT (live state only) | Q25 |
| **App-config sync** | New Rust `mde-app-sync` worker (replaces `media_sync_daemon.py`) | Q26 |
| **Conflict UI** | Bus `mesh/conflict` topic, **high priority** (status-zone strip + sound + persistent until ack) | Q23 |
| **Clipboard blobs** | Unify into `~/.mde-mesh/blobs/<ulid>.<ext>` (gluster-replicated) | Q29 |
| **Backup** | Mesh-replicated + optional off-mesh upload (S3/B2/SSH); operator-configurable | Q30 + Q59 |

---

## 5. Bus + notifications

(Foundation: `docs/design/v6.x-mackes-bus.md`. Refinements below.)

| Decision | Lock | Source |
|---|---|---|
| Topic naming | Slash hierarchy, MQTT wildcards (`+` / `#`), self-serve creation | BUS R3 |
| Action namespace | `action/<domain>/<verb>` (e.g. `action/gluster/resolve-conflict`) | Q31 |
| Action responses | `reply/<original-ulid>` (subscribe-before-publish RPC) | Q32 |
| Priority → surface | min=silent / default=tray+badge / high=strip+sound+persistent / urgent=Theater+wallpaper+phone | BUS R5 |
| First-to-ack | Keep BUS-6.4 as locked (`ack=once`, 500 ms cancel) | Q33 |
| Correlation seed | Ship **5 examples** with BUS-6.5 (power-outage, disk-pressure, mesh-degraded, VPN-flap, GFS-quota) | Q34 |
| **Drop BUS-6.6 DM addressing** (personal-mesh = meaningless) | | Q37 |
| Topic delete | Hard delete with history | Q38 |
| Replay catch-up | Replay-all since last-seen ULID | Q39 |
| Mobile reach | Android only (KDC2 + ntfy app dual-path); iOS not supported | Q40 |
| Federation | Explicit OOB-passcode pairing → symmetric subscribe-only grants | Q35 + Q55 |
| Webhook auth | Nebula source-IP only | Q36 |
| `{{exec}}` templates | Keep wide-open (documented flat-trust amplifier) | Q56 |
| DND model | Single toggle + per-topic mute/snooze (replaces v5.1 3-mode focus catalog) | BUS R6 |
| Audit | `audit/<peer>` Bus topic, **retention forever**, all peers subscribe by default (mesh-wide transparency) | Q28 + Q54 |
| Bus retention | Keep BUS-1.9 locks (7d default / urgent forever / high 30d / min 24h; 500MB warn / 2GB stop) | Q27 |

---

## 6. UI + design

| Decision | Lock | Source |
|---|---|---|
| **Visual language** | ChromeOS Classic — retire Carbon + PatternFly tokens from all new code | Q41 |
| **Card radius** | 4 px (Object Card 12 px outlier **retired**) | Q42 |
| **Icons** | **Material Symbols** (replaces Carbon) | Q43 |
| **Body font** | Roboto | Q44 |
| **Mono font** | Intel One Mono (Roboto Mono retired) | Q44 |
| **Palette** | Pure Classic ChromeOS (#202124-class) + Material You indigo | Q45 |
| **Density** | Three modes: compact 24 px / regular 28 px / comfortable 32 px | Q46 |
| **Motion** | Functional + subtle decorative (150 ms ease-out) | Q47 |
| **Wallpaper** | Decoration + optional Bus mesh-stripe (urgent only) | Q48 |
| **Workbench tree** | **Retire python `mackes/workbench/` in 1.0**; all panels target Iced `mde-workbench` | Q49 |
| **Portal-compact** | Keep mesh-glance globe as flagship | Q50 |
| **Presets** | Four: ChromeOS Classic Light + ChromeOS Classic Dark + Ableton 12 Light + Ableton 12 Dark | Q79 |

Migration note: existing GF-8.2 + GF-17.10 worklist items must
re-target to the Iced workbench tree, not `mackes/workbench/`.

---

## 7. Trust + security

| Decision | Lock | Source |
|---|---|---|
| Mesh passcode | Single, **never rotates** (operator-master credential) | Q51 |
| Passcode storage | systemd-creds TPM-or-host-key | Q52 |
| Lighthouse compromise | CA revoke + **ban list** (refuses re-join even with correct passcode) | Q53 |
| Audit visibility | All peers subscribe by default (mesh-wide transparency) | Q54 |
| Federation pairing | Out-of-band passcode + Workbench accept-pair UI; symmetric subscribe-only | Q55 |
| `{{exec}}` restrictions | None (open mesh = flat trust amplifier) | Q56 |
| Clipboard exclusion | Super+Shift+C modifier only | Q57 |
| Phone trust model | Beside the mesh + Bus-reach (no Nebula peer-hood) | Q58 |
| Backup destinations | Mesh-replicated + optional off-mesh (S3/B2/SSH) | Q59 |
| Public ports | Document allowed (4242/UDP + 443/TCP); lint-block anything else | Q60 |

**Posture expansion:** [`docs/design/security-posture.md`](design/security-posture.md)
documents the deliberate stance — Fedora targeted SELinux as the
policy base, user-UID isolation as the process model, Nebula's
`CAP_NET_ADMIN` scoped via systemd `CapabilityBoundingSet` (no
custom `nebula_mde_t` SELinux type in 1.0; trigger documented),
flat-trust threat model rationale, and the intra-mesh boundary
list. Closes AI_PLATFORM_REFERENCE.md §11.5.

---

## 8. Process + rules

| Decision | Lock | Source |
|---|---|---|
| Commit cadence | Small commits direct to main; every worklist task = 1+ commits | Q61 |
| PR lane | UX-* visual work only (`ux/<task-id>` branches with screenshots) | Q62 |
| Pre-commit gates (§0.7) | **9 total**: module-smoke, tests, ruff, RPM, CSS, voice, legacy-mesh + **D-Bus shape lint** + **Material Symbols lint** | Q63 |
| Definition of Done (§0.8) | **8 gates**: existing 7 + **security review for new public ports / D-Bus methods / `{{exec}}` templates** | Q64 |
| Retirement cadence | **Inline-per-epic** (every epic names retirements) + **quarterly fallback audit** | Q65 |
| Survey trigger | Any non-trivial design fork (≥3 reasonable options) → `AskUserQuestion` | Q66 |
| Authority hierarchy | **Memory > CLAUDE.md > this doc > other design docs > worklist body** (newest wins) | Q67 |
| Worklist file size | Single file forever (`docs/PROJECT_WORKLIST.md`) | Q68 |
| Bench validation | **Pre-release HW bench required** — each `cut release X.Y.Z` needs HW items green | Q69 |
| AI session coordination | Worklist `[>] session=<id>` is the only primitive | Q70 + Q86 |
| Spec filename | Stays `packaging/fedora/mackes-shell.spec` (historical) | Q75 |
| Birthright term | Kept | Q76 |

---

## 9. Naming + versioning

### 9.1 Naming

- **Product:** MackesDE for Workgroups (marketing, About, release pages)
- **Casual user-visible:** MackesDE (in-app strings, About panel)
- **Code/internal:** MDE (`mded`, `mde-*` crates, `dev.mackes.MDE.*` D-Bus)
- **QNM-Shared → MDE-Workgroup** (term retires; `qnm_root` → `workgroup_root`)
- **Crate prefixes:** `mackes-*` for shared/legacy (mackes-mesh-types,
  mackes-transport, mackes-config, mackes-theme,
  mackes-nebula-https-tunnel, mackes-panel); `mde-*` for new
- **Birthright** retained (distinctive operator term)

### 9.2 Versioning

- **Reset to 1.0 on rebrand** — "MackesDE for Workgroups 1.0" is
  the first official release (Q72)
- Everything pre-rebrand = "mackes-shell pre-history"
- Design docs keep filenames (`v5.0-gluster.md`, `v6.x-bus.md`); release
  SemVer ticks independently (Q80)
- Post-1.0 cadence: **continuous main + annual major tags** (Q92)

### 9.3 Epic numbering

**Move to `EPIC-001..NNN` numbering** with `tag:` field carrying the
old prefix for grep/history. Migration designed under EPIC-PROC-3. (Q78)

---

## 10. AI collaboration model

| Decision | Lock | Source |
|---|---|---|
| Model tiering | **Opus** for design/audit/scaffold; **Sonnet** for implementation; **Haiku** for grunt | Q81 |
| Memory cleanup | No auto-delete; operator-explicit only | Q82 |
| Standing authorization | commit + push + RPM build + cut release (operator-initiated) | Q83 |
| Error handling | Revert + commit; no special memory note | Q84 |
| Co-attribution | Always include trailer + exact model identifier (Opus 4.7 / Sonnet 4.6 / Haiku 4.5) | Q85 |
| Session attribution | `[>] session=<id>` on every claimed task | Q86 |
| Skills | **Consolidate to 3**: `plan` (design/survey/audit), `ship` (worklist drain), `release` (cut/push/tag) | Q87 |
| Operator presence | Self-decide unless explicit fork; survey only when no memory + no design doc covers it | Q88 |
| Background work | Use freely (long cargo test, RPM build, etc.); AI continues + reports | Q89 |
| New session onboarding | **Harness auto-injects** brief + MEMORY.md + last-3-commits | Q90 |

---

## 11. 1.0 Roadmap (MAXIMUM scope per Q91 — NO INCOMPLETE RELEASES per §0.17)

> **NO INCOMPLETE RELEASES** (operator directive 2026-05-26 +
> CLAUDE.md §0.17 + memory [[feedback_no_incomplete_releases]]).
> Every cut release ships EVERY locked item below. No minimal-1.0,
> no scope-cut, no defer-to-1.1. The §11 roadmap below IS the cut
> definition. When asked "what's the fastest path," reframe to
> execution-pace (more parallel sessions, more bundles per session,
> more HW bench passes) — never scope reductions. The only
> legitimate path to revisit Q91 is an operator-typed "lift the
> lock for X" / "amend Q91."

MackesDE for Workgroups 1.0 ships when ALL of these are green:

| # | Scope item | Source |
|---|---|---|
| 1 | **BUS-1..7** fully shipped (foundation + surfaces + webhooks + migration + clipboard + advanced routing + federation/audit + BUS-7.7-FED federation UX) | Q91 + Q24 (25-Q) |
| 2 | **GF-17 retired** (BUS-4.2 hard cut) | Q91 |
| 3 | **DEAD-2 fully drained** (mesh-module retirement queue clean) | Q91 |
| 4 | **CR-*** ChromeOS Classic visual retrofit complete | Q91 |
| 5 | **Python `mackes/workbench/` retired** (Q49); all panels in Iced `mde-workbench` | Q49 |
| 6 | **Every Python daemon ported to Rust** (no subprocess-supervised Python) | Q95 |
| 7 | **D-Bus → Bus migration complete** (only FDO interop survives) | Q96 |
| 8 | **Material Symbols pivot complete** (Carbon icons gone from user-visible code) | Q97 |
| 9 | **4 presets implemented** (ChromeOS Classic L/D + Ableton 12 L/D) | Q79 |
| 10 | **Fleet cap update** — design docs + code reflect 8-peer cap (was 16); birthright hard-limit + `--override-cap` flag | Q3 + Q22 (25-Q) |
| 11 | **INST-*** completed (installation manager) | Q98 |
| 12 | **DM-*** completed (greetd + regreet display manager) | Q98 |
| 13 | **Caddy gateway retired** | Q10 |
| 14 | **QNM-Shared term retired** (renamed to MDE-Workgroup throughout) | Q14 + Q77 |
| 15 | **Operator's full 8-peer fleet HW bench green** (per-bullet acceptance per 25-Q Q13) | Q98 + Q13 (25-Q) |
| 16 | **EPIC-TUNING-25Q completed** — 16 tuning tasks from the 2026-05-26 25-Q survey: 5 new pre-commit gates (#11-#15), pre-cut-check + hard-block, autonomy amendments, cap enforcement, security-posture doc, Carbon final sweep, phone Nebula peer-hood, federation UX | 25-Q (2026-05-26) |
| 17 | **PHONE-NEBULA-PEER** — phone elevated from "beside the mesh" to full Nebula peer (Q23 reopens Q58) | Q23 (25-Q) |
| 18 | **Security posture documented** (`docs/design/security-posture.md`) — Fedora targeted + user-UID stance | Q25 (25-Q) |

**Post-1.0:**
- Continuous main; tag minor versions annually (1.1, 1.2, …)
- **VoIP stays in core forever** (Q21 of 25-Q retired Q92's `mde-voice` spinout plan)
- AIR-* music continues in core
- **Continuous retirement audit on every /ship cycle** (Q20 of 25-Q replaces quarterly cadence)
- Quarterly skill curation
- FUSE-on-Android for phone GFS mount (deferred from PHONE-NEBULA-PEER per Q23 R1 risk note)

---

## 12. Survey-derived worklist work

The 100-Q survey produces a large set of worklist additions. They're
indexed in `docs/PROJECT_WORKLIST.md` under **"EPIC-MASTER + EPIC-RETIRE
+ EPIC-UI + EPIC-SYNC + EPIC-BUS-EXT + EPIC-PROC + EPIC-SEC"** sections
landed 2026-05-25. Each epic carries the Q-references that locked it.

Per Q78, these will renumber to `EPIC-001..NNN` once that migration
ships.

---

## 13. How to use this document

### 13.1 Read order for a new Claude session

1. **This doc** (governance)
2. `.claude/CLAUDE.md` (operational rules)
3. `~/.claude/projects/.../memory/MEMORY.md` (operator preferences)
4. `docs/PROJECT_WORKLIST.md` — start with the `[ ] Open` section that
   matches your task; look for `[>] session=<id>` markers to avoid
   collisions
5. Specific `docs/design/<epic>.md` only if doing epic-specific work
   — including [`security-posture.md`](design/security-posture.md)
   when touching public ports, D-Bus methods, capabilities, or
   `{{exec}}` Tera templates (§7 expansion)

### 13.2 When in doubt

- Apply the §0 master rule
- Check the relevant table above
- Survey via `AskUserQuestion` if you find a genuine design fork
  (≥3 reasonable options, Q66)
- When older docs/memories contradict this doc, **this doc wins**
  (Q67 hierarchy: newer locks supersede)

### 13.3 What this doc is NOT for

- Day-to-day commit procedure → CLAUDE.md
- Operator personal preferences → memory files
- Specific feature designs → `docs/design/<epic>.md`
- Current task queue → `docs/PROJECT_WORKLIST.md`

This doc is the **identity + architectural compass**. Implementation
details live elsewhere; cross-reference but don't duplicate.

---

## 14. Supersessions from older docs

The following older locks are **superseded** by this doc:

| Older lock | Superseded by | Reason |
|---|---|---|
| Carbon icons (iteration skill, memory `project_ux_polish_locks`) | Q43 Material Symbols | Visual pivot |
| Object Card 12 px (memory `project_object_card_pattern`) | Q42 conform to 4 px | Design-system consistency |
| 16-peer fleet (memory `project_v12_connectivity_scope`) | Q3 8-peer cap | Tighter scope |
| QNM-Shared coord substrate (multiple) | Q14 + Q77 fold into gluster + rename to MDE-Workgroup | Substrate consolidation |
| GF-17 notification bus design | BUS-4.2 hard cut + this doc §5 | Bus replaces |
| 3-mode focus catalog (GF-17) | BUS DND single-toggle + per-topic | Bus simplifies |
| `mackes-shell` naming | Q71 MackesDE for Workgroups | Rebrand |
| v12.x / v8.x / v5.x design IDs as release versions | Q72 1.0 reset | Versioning clean slate |
| Hardware Testing carve-out (memory `feedback_no_cut_until_worklist_empty`) | Q69 pre-release HW bench required | Tighter gate |

Memory files for the items above should be updated to reflect
supersession; CLAUDE.md §0.7 and §0.8 need amendment per Q63 + Q64
(new lints + 8th DoD gate).

---

## 15. Living-document policy

Update this doc when:
- A new N-Q survey locks a platform-wide direction
- An entry in §1-11 is superseded by a newer decision
- A new column needed in §11 1.0 roadmap (added scope item)

Do NOT update for:
- Per-feature design changes (use `docs/design/<epic>.md`)
- Routine epic landings (use `docs/PROJECT_WORKLIST.md`)
- Operator preference shifts (use memory files)
- Bug fixes / typo fixes (use commit messages)

When this doc gains an entry, ensure CLAUDE.md + MEMORY.md are
cross-referenced if the change affects them.
