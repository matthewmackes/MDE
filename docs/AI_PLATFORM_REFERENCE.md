# MackesDE for Workgroups — AI Design Partner Reference

**Status:** Strategic briefing, locked 2026-05-26 (synthesis of the
100-Q survey + 17 epic design docs + 31 memory files + the live
worklist).
**Authority:** Below `docs/AI_GOVERNANCE.md` (which holds the *locks*);
above `docs/design/<epic>.md` (which holds the *details*). This
document is **synthesis**, not new policy. Conflicting reads default
to AI_GOVERNANCE.md per §0.14 of CLAUDE.md.
**Audience:** AI systems asked to design, extend, audit, or critique
MackesDE for Workgroups. Read this BEFORE proposing platform changes.

This is the briefing your predecessors wished they'd had on session
one. Read it whole; cross-reference the cited locks before acting.

---

## 0. How to read this document

Every claim labeled **(LOCK Qn)** is verbatim from the 100-Q
tightening survey of 2026-05-25 and is authoritative. Every claim
labeled **(INFERRED)** is the author's synthesis from observed
patterns — accurate as of writing but not survey-locked, so treat
as a strong hypothesis rather than canon. Every claim labeled
**(GAP)** is something this document found *missing* from the
canonical record and is worth a future survey question.

---

## 1. Platform Overview

### 1.1 What this is

MackesDE for Workgroups (informal: MackesDE; code-internal: MDE)
is a **Linux desktop environment + mesh-orchestration platform**
distributed as a Fedora RPM (`mde-X.Y.Z.fc44.x86_64.rpm`). It
runs on x86_64 Fedora 44+ workstations, x86/SBC lighthouses (the
mesh's tie-break + relay layer), and bridges to one Android
phone per operator via KDC2. **(LOCK Q5)**

It ships with: a Wayland-only session host (Sway substrate +
Iced custom shell), a custom panel + dock (`crates/mde-panel`), a
control-panel application (`crates/mde-workbench`), a file
manager (`crates/mde-files`, forked from `pop-os/cosmic-files`),
a phone-bridge daemon (`crates/mde-kdc-proto`), a notification +
event Bus (`crates/mde-bus`, locked in the BUS-1..7 epic
2026-05-25), a peer daemon (`crates/mackesd`), a session host
(`crates/mded`), and ~30 supporting crates.

The product replaces a v1.x GTK3/Python predecessor called
"mackes-shell" — that codebase is being actively retired (see
`EPIC-RETIRE-PY-WORKBENCH` in the worklist + the `mackes/`
directory tree). The first MackesDE-branded release is **1.0**
(LOCK Q72 — the rebrand resets versioning).

### 1.2 The master rule

> **"Secure, Simple, Centerless Workgroup."** (LOCK Q1 + Q100)

These four words are the tiebreaker for every conflicting design
fork. They appear in this order because the order is the
priority. **Secure** wins over **Simple** when they conflict;
**Simple** wins over **Centerless**; **Centerless** wins over
generic "workgroup" framings that imply a central authority.

### 1.3 What "Centerless" means here

Every locked architectural decision pushes toward eliminating
*designated central authorities* inside the mesh:

- **No cloud control plane.** The platform has no SaaS backend,
  no shared accounts, no remote DB. Every peer is a peer.
- **No fixed leader.** Lighthouses are tie-break + relay
  infrastructure (NAT-traversal hints + HTTPS-fallback tunnel,
  arbiter brick for Gluster), not authority nodes. The leader-
  election lock (QNM-Shared mtime race, locked v12.0) was
  retired by the GFS fold-in (LOCK Q14 + Q21 + Q22 + Q77).
- **No D-Bus hub by 1.0.** D-Bus is retiring entirely except
  for `org.freedesktop.*` interop (LOCK Q20 + Q96); IPC moves
  to the Bus, which has no central broker (per-peer ntfy +
  GFS-replicated message tree).
- **Single shared passcode.** No per-peer ACLs, no role-based
  access (per [[project_open_mesh_directive]]). One credential
  authenticates everything inside the mesh. **(LOCK Q51 + Q56)**
- **Audit-by-subscription.** Every peer subscribes to the
  audit topic by default — there is no privileged "auditor"
  node. **(LOCK Q28 + Q54)**

---

## 2. Primary Goals

### 2.1 What the platform optimizes for

| Goal | Mechanism | Locked at |
|---|---|---|
| Cross-device file continuity | GlusterFS mesh-home (full-mesh replicated XDG dirs) | v5.0 + Q22 |
| Cross-device event surfacing | Mackes Bus (ntfy + GFS file tree + audit JSONL) | BUS-1..7 + Q91 |
| Phone reach | KDC2 bridge + ntfy mobile app (dual-path, dedup by ULID) | v2.1 + BUS R12 |
| Self-supporting operation | Birthright wizard + lighthouse bootstrap + Nebula CA | v2.5 NF-* |
| Calm operation | Apple-System-Settings UX (visual identity) | UX-21 + visual-identity.md |
| Single-operator simplicity | Open mesh, flat trust, one passcode | [[project_open_mesh_directive]] |

### 2.2 What the platform explicitly does NOT optimize for

- **Multi-tenant collaboration.** This is a single-operator
  product. The "Workgroup" framing means one person's 3-8
  devices acting as a logical unit, NOT a team of users
  sharing infrastructure. (LOCK Q2 + Q3 — 8-peer cap,
  tightened from 16 in the 100-Q survey)
- **Cloud / SaaS.** No backend service, no remote storage by
  default (off-mesh backup is an opt-in operator-configured
  destination per LOCK Q59).
- **Cross-platform GUI.** Linux + Wayland + Sway only. macOS
  + Windows are not supported substrates. The KDC2 bridge to
  Android is a tethered protocol, not a port. (LOCK Q5 + Q40)
- **Enterprise IT integration.** No LDAP/SSO/AD/SAML. No
  device-management API. No SCIM. The single-passcode model
  is intentionally incompatible with enterprise IAM.
- **Discovery / virality.** No app store, no public peer
  directory, no inbound discovery surface. Federation between
  meshes requires explicit out-of-band passcode pairing
  (LOCK Q35 + Q55).

### 2.3 Why these tradeoffs

(INFERRED) The platform is the operator's personal computing
substrate. It is built TO BE owned, not to scale to multiple
owners. Every "we don't do that" is a deliberate refusal to
pay the complexity tax that multi-tenancy / SaaS / enterprise
imposes — those costs would compromise "Simple" without
producing value for the locked use case.

---

## 3. Target Users and Use Cases

### 3.1 The operator (singular)

| Attribute | Value |
|---|---|
| Number | One human, one mesh |
| Devices | 3-8 of *their* own machines + 1 Android phone via KDC2 |
| Skill profile | Technically literate; comfortable with Fedora + systemd + journalctl; not a Linux beginner |
| Use case | Personal computing fabric: file continuity, notification fan-out, mesh-aware applications (Airsonic, Voice/Video PBX, monitoring) |
| Distribution | Self-supporting circle — the operator may distribute the RPM to a small group of similar users, but each forms their own one-person workgroup |

(LOCK Q2 + Q5 + Q6)

### 3.2 What the operator does in a typical week

(INFERRED, from worklist + memory analysis)

1. **Boot a workstation.** greetd shows the regreet greeter
   (DM-* epic, Q98). After login, sway launches with mded +
   mackesd + mde-bus + mde-panel auto-started by systemd
   user units.
2. **Files appear continuous.** Documents / Pictures / Music
   / Videos / Downloads are GlusterFS-mounted from the mesh
   volume. Edits on peer A appear on peer B within seconds
   (LWW conflict resolution by mtime + `.conflict-<host>-<ts>`
   siblings — LOCK GF-9).
3. **Notifications appear coherent.** FDO Notify on any peer
   publishes to `fdo/<app>` on the Bus; every peer's tray +
   dock badge updates. Phone gets push via KDC2 + ntfy mobile
   dual-path (LOCK BUS R12, deduplicated by ULID).
4. **Music plays mesh-aware.** Airsonic ships in 1.0 as a
   core bundled application (LOCK Q9) — mesh-library aware
   means scrobbles + playlists + last-played sync.
5. **Voice calls work.** v4.1 Voice & Video PBX (R11 Vitelity
   direct, Kamailio retired) makes outbound calls + receives
   inbound via the SIP Vitelity sub-account. SimRing rings
   every device.
6. **System health is visible.** Netdata aggregator on the
   leader-elected peer monitors the fleet; alerts fire through
   `mde-alert-emit` to the alerts JSONL AND (BUS-4.3) to the
   `mon/*` Bus topics for surface dispatch.
7. **Backups happen quietly.** Mesh-replicated by default
   (every file lives on every peer); optional off-mesh upload
   to S3/B2/SSH if the operator configures it (LOCK Q59).

### 3.3 Use cases explicitly OUT of scope

- Headless server farms (lighthouses can run headless, but
  the platform is desktop-first)
- Containerized multi-app deployment (Podman is OPT-IN per
  user; **Birthright deploys ZERO containers** per
  [[project_v6_0_mde_portal]] R12)
- Sharing files with non-mesh humans (mesh-home is
  intentionally inaccessible to non-peers; the `/Public`
  shared dir was an early design candidate and is NOT in
  current locks)

---

## 4. Core Methods and Workflows

### 4.1 The two-substrate architecture

(LOCK §3.1 of AI_GOVERNANCE)

Every piece of state in the platform lives in exactly ONE of
two substrates:

| Substrate | Holds | Mount / path | Lives where |
|---|---|---|---|
| **Gluster mesh-home** | XDG files + MDE-Workgroup coordination | `~/Documents`, `~/Pictures`, `~/Music`, `~/Videos`, `~/Downloads`, `~/.mde-mesh/<peer>/` | Full-mesh replicated across every peer |
| **Mackes Bus** | Events + clipboard + audit + notifications | `~/.local/share/mde/bus/<topic>/<ulid>.json` + per-peer SQLite index | File tree GFS-replicated; SQLite per-peer |

There is no third substrate. Caddy + the gateway model retired
in this survey (LOCK Q10); QNM-Shared as a separate term
retired in favor of Gluster (LOCK Q14 + Q77). Any future
design proposal that wants to introduce a third sync
mechanism must justify it against this two-substrate lock.

### 4.2 The Bus-for-everything IPC model

(LOCK §3.3 of AI_GOVERNANCE)

Every internal communication uses the Bus by 1.0:

- **Commands:** `action/<domain>/<verb>` topics
  (e.g. `action/gluster/resolve-conflict`) — LOCK Q31
- **Responses:** `reply/<original-ulid>` topics —
  publisher subscribes-before-publishes for RPC-like
  semantics. LOCK Q32
- **Events:** domain topics (e.g. `mesh/conflict`,
  `mon/cpu`, `fdo/<app>`)
- **Slow state (≤1/min):** file polling against gluster
  mesh-home rather than Bus events — LOCK Q13
- **D-Bus:** **retires entirely by 1.0** except for
  `org.freedesktop.*` (FDO Notifications, FreeDesktop
  Secret Service, etc.) — LOCK Q20 + Q96

Pre-1.0 transition state: `dev.mackes.MDE.*` D-Bus services
co-exist with Bus but are slated for migration in the
`EPIC-RETIRE-DBUS` epic (one service at a time).

### 4.3 Priority → surface dispatch

(LOCK BUS R5 + BUS-2.1 design)

Every Bus message carries a priority field. The priority
determines which UI surfaces light up:

| Priority | Surfaces | Use |
|---|---|---|
| `min` | Silent log only; no UI | Background telemetry; clipboard adds; `clipboard/sync` |
| `default` | Tray icon + Dock breadcrumb badge | Routine notifications |
| `high` | Status-zone slide-up strip + sound + persistent until ack | Disk pressure; mesh-degraded; UPS on-battery |
| `urgent` | Theater takeover (full-screen) + wallpaper stripe + phone push | LOWBATT shutdown imminent; mesh quorum lost |

The `LogOnlySurfaces` default impl in `crates/mde-bus/src/surface.rs`
ships as the pre-Iced placeholder; BUS-2.2..2.8 will land the
real Iced surfaces during the v1.0 cut.

### 4.4 Birthright — the onboarding workflow

(LOCK §0.6 + Q76 — birthright term retained)

"Birthright" is the platform's name for the per-peer
onboarding wizard. It runs once on first boot of a fresh
install and:

1. Asks the operator's name + sets up the primary user (uid
   1000:1000 pinned per GF-11).
2. Lays down theme + fonts + curated apps + panel layout.
3. Pairs the peer to the mesh (passcode entry → Nebula CA
   sign-off via lighthouse).
4. Mounts the GFS mesh-home volume.
5. Configures display manager (greetd + regreet).
6. Lights up the dock + dashboard.

Birthright is **idempotent** — running it again is a no-op on
already-configured slots. This is a critical platform
invariant: every install workflow tolerates partial
completion + resumption.

### 4.5 The AI collaboration workflow

(LOCK §10 of AI_GOVERNANCE)

AI design partners use exactly three skills:

- **`plan`** — design forks + N-Q surveys + worklist audits +
  drafting design docs. Runs BEFORE code.
- **`ship`** — drain the worklist autonomously. The standing
  exception §0.16 authorizes /ship to drain BUS-1..7 without
  per-bundle confirmation.
- **`release`** — execute `cut release X.Y.Z` (the 7-step
  shorthand from §0.6 of CLAUDE.md).

Every other historical skill (`mackes-worklist-management`,
`autonomous-worker`, `complete-remaining-work`, `iteration`,
`batch`) is retired but kept for slash-name back-compat (LOCK
Q87). Surveys fire via `AskUserQuestion` one question at a
time per [[feedback_question_workflow]]. Model tiering: Opus
for design/audit, Sonnet for implementation, Haiku for grunt
(LOCK Q81).

---

## 5. Design Patterns

These patterns repeat across the codebase. Recognizing them
helps an AI propose changes that fit rather than fight.

### 5.1 Pure-function extraction (LOCK Q17)

Every worker, every adapter, every server handler is built
as: pure helpers + thin async glue. Tests cover the pure
helpers deterministically; the glue gets bench-tested.

The pure-fn discipline expanded in the 100-Q survey from "argv
only" to "ALL IO" — file reads, D-Bus calls, network calls all
extract into pure-fn rendering + a thin IO wrapper. Example:
`crates/mde-bus/src/broker.rs` separates `render_config`,
`evaluate_prereqs`, `materialize_config`, `spawn_ntfy` (all
pure-or-IO-as-data) from `start_if_ready` (the async
orchestrator).

### 5.2 Graceful degradation via `SkipReason` enums

Every component that depends on external state (overlay-IP
publish file, gluster mount, ntfy binary on PATH, X11 env,
…) returns a typed `*SkipReason` enum from its prereq check.
The outer supervisor logs the reason once + retries on next
tick. This is THE pattern for handling pre-enrollment +
mid-flight environment changes; ~12 components use it.

Example: `BrokerSkipReason` enum has four variants
(`NoOverlayIp`, `EmptyOverlayIp`, `NtfyMissing`,
`TemplateMissing`). Each variant has a `Display` impl with an
operator-facing message naming the next step.

### 5.3 Bind-scope-as-security

Every internal listener binds on the Nebula overlay IP, NOT
on `0.0.0.0`. The kernel socket layer is the security
boundary — underlay traffic is dropped before the application
layer ever sees it. The `lint-public-ports.sh` pre-commit gate
(LOCK Q60) catches net-new public binds outside a small
documented allow-list (UDP/4242 Nebula + TCP/443 HTTPS-tunnel
fallback). 

This means no application-level auth tokens, no API keys, no
TLS-in-mesh certs — Nebula transport encryption + bind-scope
makes them redundant.

### 5.4 Atomic write via temp + rename

Every persistent write goes temp + fsync + rename. The Bus
file tree, the SQLite WAL, the regreet config, the firewalld
service definition — everything. This guarantees readers
either see the old version or the new version, never a partial
write.

### 5.5 Idempotent seeding

Every component that initializes per-peer state checks if it's
already initialized + no-ops if so. `Persist::open` runs
`CREATE TABLE IF NOT EXISTS`; `seed_defaults_with_hostname`
returns `Ok(0)` on second call; the Birthright steps tolerate
re-run; the subs.yaml `load_or_seed` only writes if the
per-peer file is missing.

### 5.6 ULIDs as causal cursors

Every Bus message + every alert + every persisted event
carries a 26-char Crockford-base32 ULID. ULIDs are:

- **Sortable** by timestamp prefix → `(topic, ulid)` is the
  natural index for `list_since(topic, since)` queries.
- **Globally unique** → cross-peer dedup is trivial (KDC2
  bridge + ntfy dual-path use ULID for dedup).
- **Deterministic on input** (in `mde-alert-emit`) → repeat
  invocations of the same Netdata alarm produce the same
  ULID and don't duplicate-write.

### 5.7 Snapshot-allow-list lints

The pre-commit gates (10 of them, LOCK Q63) include three
"net-new violations only" lints (legacy-mesh, dbus-shape,
material-symbols, public-ports). Each carries a snapshot
allow-list of pre-existing violations captured at lint
introduction time. New code can't add new violations; old
code is grandfathered until the corresponding retirement
epic catches up.

### 5.8 Best-effort writes around the durable path

Many components write to multiple stores. The convention:

1. The *authoritative* store goes first + must succeed.
2. *Secondary* stores (audit log, Bus publish, surface
   dispatch) are best-effort — failures log but don't propagate.

This appears in `Persist::write` (file tree FIRST, then
SQLite index, then audit log, then surface dispatch); in the
webhook server (file tree FIRST, then ntfy POST); in
`mde-alert-emit` (JSONL FIRST, then `mde-bus publish`); in
`NotificationsService::notify` (SQLite FIRST, then shell-out
to mde-bus).

### 5.9 Carve-outs documented inline

Every exception to a platform-wide rule lives next to the
rule, not in a hidden override file. `§0.16` lists the three
standing exceptions to the feature lock; the `lint-*.sh`
scripts inline their allow-list rationale; the
`PROJECT_WORKLIST.md` "Standing exceptions" block names every
authorized carve-out by epic. There is no separate
`exceptions.toml` — exceptions live where the rule lives.

### 5.10 Newest-lock-wins

(LOCK Q67, mirrored in CLAUDE.md §0.14)

When two locks contradict, the newer one wins silently. The
older lock keeps its text for historical context but the
worklist reflects the live policy. The authority hierarchy is:

```
Memory (operator's live preferences)
    ↓
.claude/CLAUDE.md (operational rulebook)
    ↓
docs/AI_GOVERNANCE.md (platform identity + compass)
    ↓
docs/design/<epic>.md (per-epic locks)
    ↓
docs/PROJECT_WORKLIST.md body (actionable state)
```

---

## 6. Rules, Constraints, and Guardrails

### 6.1 Pre-commit gates (LOCK Q63 — 10 total)

Every commit runs:

1. Module import smoke (Python: `python3 -c "import mackes.<x>"`)
2. Tests (`make test-nodeps`)
3. Ruff lint
4. RPM build (`make rpm`)
5. CSS lint (`install-helpers/lint-css.sh`)
6. Voice-and-tone lint (`lint-voice.sh`)
7. Legacy-mesh-vocabulary lint (`lint-legacy-mesh.sh`)
8. D-Bus shape lint (`lint-dbus-shape.sh`)
9. Material Symbols icon lint (`lint-material-symbols.sh`)
10. Public-port-bind lint (`lint-public-ports.sh`)

### 6.2 Definition of Done (LOCK Q64 — 8 gates)

A worklist task is `[✓] Done` only when:

1. Committed to `main` (in git history, not just working tree)
2. Pushed to origin
3. RPM builds clean
4. Tagged + released (for shipping versions)
5. Module imports clean
6. CHANGELOG updated
7. **Runtime reachability** — the new code is invocable from
   a real entry point (a user gesture, a scheduled tick, a
   subscription, a daemon spawn). This gate prevents the
   "helpers shipped, wiring deferred" failure mode that the
   v3.x audit on 2026-05-22 caught (13 dead panel modules,
   4 user-visible bugs as the consequence).
8. **Security review notes** on new public ports / new D-Bus
   methods / new `{{exec}}`-using templates (LOCK Q64).

### 6.3 No stubs (§0.12 — codified after the v3.x audit)

Hard refusals — never commit:

- `todo!()`, `unimplemented!()`, `panic!("not yet")`
- Match arms that log "not yet implemented" and exit 0
- `pub mod foo;` with zero external references
- Commit messages saying "wiring lands in a follow-up,"
  "phase 2 implements," "stub for now"
- Worklist `[✓]` marks for tasks where only helpers + tests
  landed (the wiring must be real)

Splitting rule: if a task can't ship complete, split it at
write-time into smaller tasks each of which CAN ship complete
— NOT into "helpers + wiring."

### 6.4 No incomplete releases (§0.17 — operator directive 2026-05-26)

> Every cut release ships every locked §11 roadmap item from
> `AI_GOVERNANCE.md`. No minimal releases. No scope-cut releases.
> No defer-to-1.1.

Trigger phrases AI partners must REFUSE silently or surface
this rule first:

- "What's the fastest path to ship?"
- "Can we defer X to 1.1?"
- "Let's cut a minimal 1.0"
- "Should we shrink scope?"
- "Lock a smaller 1.0"
- "Pick the critical-path items only"

For any of these, reframe the question as execution-pace
(parallel sessions, more bundles per session, HW bench
acceleration) — never scope reduction. The only legitimate
path to revisit Q91 is operator-typed "lift the lock for X" /
"amend Q91."

### 6.5 Platform feature lock (§0.16 — through next cut)

> Effective 2026-05-26 the platform is FEATURE LOCKED until
> the next named release cuts.

The 324 active items in `docs/PROJECT_WORKLIST.md` are the
complete release backlog. AI partners must REFUSE new feature
scoping. Allowed without question:

- Bug fixes against existing tasks
- Polish / refactor / simplification / scope reduction on
  locked work
- Worklist hygiene; retirement audits
- Hardware bench + release-prep + cut flow
- Completing already-locked tasks end-to-end

Standing exceptions:
1. BUS-1..BUS-7 build authorized (drain autonomously)
2. R11 stale-section re-locks completed
3. HW bench gate deferred to RC-completion

### 6.6 Branch discipline

- **Always stay on `main`.** Never create feature branches
  unless the operator explicitly asks.
- **Never force-push to `main`.** Warn + confirm if asked.
- **No `--no-verify`, `--no-gpg-sign`, `--amend` of pushed
  commits** unless explicit.
- **Dual-remote push** (`origin` + `mde-x`) is the canonical
  push pattern; both remotes get every commit.
- **UX-* visual work uses `ux/<task-id>` branches** with
  before/after screenshots in the PR (§0.11 — the ONE
  exception to main-only discipline).

### 6.7 Cut-release shorthand (§0.6)

When the operator types `cut release X.Y.Z`:

1. Bump version in 4 files
2. CHANGELOG entry
3. Smoke test (`python3 -c "import mackes; print(mackes.__version__)"`)
4. Local RPM build (`make rpm` — never `--short-circuit`)
5. Commit
6. Push + tag (dual-remote; tag uses "Mackes Desktop Environment"
   not legacy "Mackes Shell" naming)
7. Watch the workflow (`gh run watch`)

Per §0.15 (Q69 lock), HW carve-out items targeting that
release version must be `[✓]` with operator-confirmed bench
results BEFORE step 1.

### 6.8 Authority hierarchy + newest-lock-wins (LOCK Q67)

See §5.10 above. Operationally: when proposing a change,
check memory first (highest), then CLAUDE.md, then
AI_GOVERNANCE.md, then the relevant `docs/design/<epic>.md`,
then the worklist body. If the newest lock contradicts an
older one, the newer wins silently — update the worklist in
place; leave the older design doc as historical context.

---

## 7. Product Direction and Strategic Implications

### 7.1 The 1.0 cut — MAXIMUM scope (LOCK Q91 + §0.17)

The 1.0 cut ships ALL 15 of these items:

1. BUS-1..7 fully shipped (foundation + surfaces + webhooks +
   migration + clipboard + advanced routing + federation/audit)
2. GF-17 retired (BUS-4.2 hard cut) — **shipped 2026-05-26**
3. DEAD-2 fully drained (mesh-module retirement queue clean)
4. CR-* ChromeOS Classic visual retrofit complete
5. Python `mackes/workbench/` retired (Q49); all panels in
   Iced `mde-workbench`
6. Every Python daemon ported to Rust (no subprocess-supervised
   Python by 1.0)
7. D-Bus → Bus migration complete (only FDO interop survives)
8. Material Symbols pivot complete (Carbon gone from
   user-visible code)
9. 4 presets implemented (ChromeOS Classic L/D + Ableton 12 L/D)
10. Fleet cap update (design docs + code reflect 8-peer cap,
    was 16)
11. INST-* completed (installation manager)
12. DM-* completed (greetd + regreet display manager)
13. Caddy gateway retired
14. QNM-Shared term retired (renamed to MDE-Workgroup)
15. Operator's full 8-peer fleet HW bench green

### 7.2 What 1.0 implies strategically

(INFERRED)

- **It's a brand reset.** Versioning resets to 1.0; the "mackes-
  shell pre-history" becomes a deprecated codebase. AI partners
  proposing changes that depend on v1.x conventions should expect
  rejection.
- **It's a forcing function on scope.** The MAXIMUM-scope lock
  prevents the "ship what we have + defer the rest" failure mode.
  Every item above must be green simultaneously, which forces
  parallelization + sequence discipline.
- **It's the end of the porting era.** Items 5-8 (Python →
  Iced/Rust port, Material Symbols pivot, D-Bus → Bus migration)
  are heavyweight refactors. Post-1.0 development can spend more
  time on NEW capability rather than removing old.

### 7.3 Post-1.0 direction

(LOCK §11 + Q92)

- **Continuous main** + annual major tags (1.1, 1.2, …)
- **VoIP spinout to `mde-voice` repo** (deferred from Q8 —
  revisit at 1.1; v4.1 voice-video is BUNDLED at 1.0)
- **Airsonic music continues in core** (LOCK Q9 — mesh-library
  awareness is the differentiator)
- **Quarterly DEAD-N retirement audits** (LOCK §0.13 — the
  inline-per-epic retirement loop plus a 3-monthly fallback)
- **Quarterly skill curation** (the 3-skill model is itself
  subject to review)

### 7.4 What this means for AI design partners

(INFERRED)

- Proposals that add new SaaS dependencies, multi-tenant
  features, or cloud control-plane components will be rejected
  on first principles.
- Proposals that move state OUT of the two-substrate model
  (gluster + Bus) require a survey to lift the lock — never
  silent additions.
- Proposals that touch the visual identity (ChromeOS Classic +
  Material Symbols + Roboto + indigo) need design-doc-cited
  justification + UX-* branch-lane review.
- The "Workgroup" framing is permanent — proposals that
  reframe the platform as a team-collaboration product reject
  the master rule.

---

## 8. Normative Platform Model — What "normal" looks like

A *normal* change in this platform:

- **Touches one or two crates** (`mde-bus` + `mackesd`, or
  `mde-workbench` + `mde-panel`, etc.) — never the whole tree.
- **Lands as a single commit on `main`** with a Conventional-
  Commit-style first line (`BUS-N.M: subject`).
- **Updates exactly three files outside the crate**: the
  worklist (status flip + completion note), the CHANGELOG
  (user-visible entry under "Unreleased"), and sometimes the
  spec (if installing data files).
- **Passes 10 pre-commit gates + 8 DoD gates.**
- **Adds tests** — usually pure-fn unit tests, occasionally
  integration tests with a stub TCP server / tempdir
  fixture / synthesized clock.
- **Ships END-TO-END** — runtime-reachable from a user
  gesture or scheduled tick.
- **Dual-remote pushes** to `origin` + `mde-x`.

A normal *task* in the worklist:

- Carries a `<EPIC>-N.M:` prefix tied to a release tag.
- Follows the user-story shape (As/I want/so that +
  bench-observable acceptance bullets).
- Lives in a section that names its epic + design-doc lock.
- Picks up `[>] session=<id>` when an AI claims it; flips to
  `[✓]` with a multi-paragraph completion note on commit.

A normal *design doc*:

- Lives at `docs/design/<epic>.md`.
- Cites the survey Q-IDs that locked each row of its tables.
- Names every cross-epic dependency + every retirement it
  triggers.
- Is supplemented (not replaced) by every later survey.

A normal *AI session*:

- Reads AI_GOVERNANCE + CLAUDE.md + MEMORY.md + the last 3
  commits at start (LOCK Q90 — harness auto-injects these).
- Marks claimed tasks `[>] session=<id>` before substantive
  edits.
- Surfaces conflicting locks immediately + checks memory + asks
  rather than guessing.
- Commits + pushes + updates worklist + updates CHANGELOG
  per task — never batched.

---

## 9. Outlier Features and Behaviors

These are platform decisions or implementations that DON'T fit
the surrounding norms. Each one is intentional but worth
flagging because future AI partners will trip over them.

### 9.1 The Object Card 12 px → 4 px reversal (LOCK Q42 — outlier RETIRED)

Background: Object Cards (Start menu, mde-files grid, peer
cards, recents) were locked to 12 px corner radius in
[[project_object_card_pattern]] 2026-05-24 as an "intentional
break from the 4 px platform rule." The 100-Q survey on
2026-05-25 **reversed** this — Q42 retires the 12 px outlier
to conform to 4 px universally.

Implication: any code still using the 12 px constant is a
hangover that needs cleanup. The cleanup epic
(`EPIC-UI-CARDS`) is partially done (`CARD_CORNER_RADIUS`
const exists). AI partners should NOT propose Object Card
exceptions to the platform-wide 4 px radius.

### 9.2 VoIP / Voice & Video bundled at 1.0 (LOCK Q8 + Q94)

The v4.1 / VOIP-* epics bundle a full SIP PBX (PJSIP direct +
Vitelity + SimRing + ephemeral mediasoup bridge for >2-party
calls) in the 1.0 cut. R11 retired Kamailio in favor of
direct PJSIP-to-Vitelity per peer.

Outlier because: post-1.0 Q92 + Q8 spin VoIP OUT into a
separate `mde-voice` repo at 1.1. So the core platform
INCLUDES a complex subsystem it plans to UN-INCLUDE. Implication:
internal coupling between core MDE and VOIP-* tasks must stay
loose enough that the spinout is feasible — AI partners
should not deepen the coupling.

### 9.3 KDC2 phone trust model (LOCK Q58)

(BUS R12 + [[project_v2_1_kdc2_native]])

The Android phone is paired to the mesh but does NOT become a
Nebula peer. The phone gets:

- KDC2 bidirectional protocol (clipboard, SMS, battery,
  notifications, mpris, ping, find-my-phone)
- ntfy mobile app subscription (dual-path with KDC2,
  deduplicated by ULID)
- Bus reach (notifications surfaced via push)

The phone CANNOT:
- Mount the GFS volume (no FUSE-mount on stock Android)
- Initiate publishes to arbitrary Bus topics
- Hold the mesh CA

Outlier because: every other "peer" is a full Nebula node
with the master passcode. The phone is **beside** the mesh,
not in it. Phone file-sharing (originally BUS-5.9 scope) is
limited to a one-direction "drop folder" pattern at
`~/Documents/From-<phone>/` rather than full file-tree
mounting — per [[project_kdc2_file_transfer_removed]].

### 9.4 `{{exec}}` template execution (LOCK Q56)

Tera templates in Bus messages can run shell commands via
`{{exec(cmd="uptime -p")}}`. This is a flat-trust amplifier:
any peer that has the mesh passcode can publish a template
that shells on every render-target peer.

Outlier because: every other security boundary in the
platform is defense-in-depth (bind-scope + Nebula encryption +
passcode + ban-list). `{{exec}}` is intentionally wide-open —
the open-mesh directive ([[project_open_mesh_directive]])
says flat trust means flat trust. Documented in
`docs/design/v6.x-mackes-bus.md` §10 as "documented flat-trust
amplifier."

AI implication: do NOT propose locking down `{{exec}}` —
that's been surveyed and the lock holds. DO surface security
review notes per Q64 when any new template that USES `{{exec}}`
ships.

### 9.5 The Bus is BOTH ntfy AND a custom file tree (LOCK §3.1)

Most pub/sub systems are either broker-mediated (Kafka,
Redis, MQTT) or pure file-system-replicated (rsync, syncthing).
The Bus is BOTH:

- **ntfy broker** per peer (broker discovery via mDNS-on-Nebula)
- **Per-topic GFS file tree** (`<bus_root>/<topic>/<ulid>.json`)
- **Per-peer SQLite index** (NOT GFS — see §9.6)
- **Per-peer JSONL audit log** (per-day rotation)

Outlier because: this is unusual architecture; most readers
will assume a Bus is one of the above categories, not all four
simultaneously. The design rationale: file tree wins on inotify
+ GFS replication; SQLite wins on query speed; ntfy wins on
phone push; JSONL wins on audit append-only-ness. The locked
design uses each for its strength.

### 9.6 SQLite index is per-peer, NOT on GFS

The file tree is mesh-replicated. The SQLite index that points
at it is NOT — each peer maintains its own `index.sqlite`.

Outlier because: most "queryable index over replicated state"
designs would put both on the replicated store. The reason it's
local: **SQLite + networked-FS is a known footgun** — lock-
stealing, WAL-replay edge cases, journal corruption. The
per-peer index sacrifices cross-peer query unity for local
correctness. Cross-peer aggregation is BUS-7 federation
territory.

### 9.7 HW carve-out → pre-release HW gate (LOCK Q69 — reversal)

For most of 2026 the lock was: **HW-* tasks never gate a
release.** Then Q69 (2026-05-25) reversed it: HW bench items
targeting a release version must be `[✓]` BEFORE the cut.

Outlier because: this is a recent policy reversal. Memory files
[[feedback_no_cut_until_worklist_empty]] and
[[feedback_hardware_testing_epic]] still reflect the older
language; the AUTHORITATIVE rule is now §0.15 in CLAUDE.md.
AI partners reading older memory must apply the newer rule
silently.

### 9.8 Single-passcode authentication (LOCK Q51 + Q56)

Most security-conscious systems push for per-component
credentials. MDE locks to **one passcode** that authenticates
everything inside the mesh: pairing, SSH, NATS, mesh-fs,
Bus webhooks (via Nebula source-IP, which the passcode
implicitly authorized), `{{exec}}` templates.

Outlier because: it inverts the standard threat model.
Justification: the operator is one person; every node is
their own; per-component auth is operational overhead for
zero security benefit. Documented in
[[project_open_mesh_directive]].

### 9.9 Bus has the `min` priority "silent log" (BUS R5)

The four-level priority enum includes a class explicitly
defined as "no UI surface — log only." 

Outlier because: most priority schemes default the lowest
level to "background tray with badge." MDE's `min` produces
NO UI at all. Clipboard sync events are seeded as `min`
priority — they're plumbing, not events the operator should
see.

### 9.10 The 100-Q tightening survey IS the platform's compass (LOCK 2026-05-25)

Most platforms have many small design decisions made
piecemeal. MDE has ONE 100-question survey from 2026-05-25
that locks platform-wide direction — and is the canonical
source for `AI_GOVERNANCE.md` §1-11.

Outlier because: the density of locked decisions in one survey
is unusually high. Implication: AI partners should treat
Q1..Q100 as the platform's load-bearing decision set. When
proposing changes, cite the Q-ID that locks the relevant
direction; when a question's lock seems wrong, the path is to
*amend* that Q (operator-typed) — not to silently re-survey.

### 9.11 The platform feature lock IS a lock (§0.16)

Most platforms allow rolling feature additions during
development. MDE froze the feature surface 2026-05-26 with
the explicit §0.16 lock. This is unusual mid-development
discipline — every commit must justify itself against an
already-locked backlog.

Outlier because: AI partners coming from generic SaaS
contexts will reflexively propose feature ideas that violate
the lock. The lock holds; AI partners must refuse new
scoping (or surface the rule first) until the operator types
"lift the lock for X."

---

## 10. Implications for Future Design

### 10.1 What an AI partner MUST do before proposing changes

1. **Read AI_GOVERNANCE.md.** It's 372 lines. Read it.
2. **Read CLAUDE.md §0.** It's the operational rulebook.
3. **Scan MEMORY.md for relevant memories.** Operator
   preferences override governance per §0.14.
4. **Check the relevant design doc** at `docs/design/<epic>.md`
   for per-epic locks.
5. **Search the worklist** for `[>] session=<id>` markers to
   avoid collision with parallel AI sessions.

### 10.2 What an AI partner MUST NOT do

- Propose new features without surveying or invoking the
  `lift the lock` exception.
- Propose adding a third sync substrate (the two-substrate
  lock §3.1 holds).
- Propose moving state OFF the mesh into a cloud component.
- Propose multi-tenant features (8-peer cap is a single-
  operator cap).
- Propose stubs, scaffolds, or "phase 2" delays. The §0.12
  no-stubs rule is hard.
- Propose Carbon icons, Geologica fonts, or PatternFly tokens
  in new code. They're retired.
- Propose D-Bus interfaces for MDE-internal control (use Bus
  `action/<domain>/<verb>` instead — D-Bus is reserved for
  FDO interop).
- Propose `--no-verify` / `--amend pushed` / force-push to
  main without explicit operator authorization.

### 10.3 What an AI partner SHOULD do when they find a problem

1. **Surface the problem first** — name the rule that's
   violated, cite the Q-ID + the design-doc location.
2. **Propose the smallest correction** that brings the code
   back to lock.
3. **If the lock itself looks wrong**, name what would need to
   change (Q-amendment + which downstream tasks) but DO NOT
   silently amend.
4. **Open a new worklist task** for the correction (user-story
   shape; epic-prefix; bench-observable acceptance).
5. **Mark it `[>] session=<id>` + ship** per the ship skill.

### 10.4 Design moves that fit the platform

These will land cleanly:

- New Bus adapters (BUS-3-style: header dispatch + Rust
  extractor + YAML rule set)
- New Bus surfaces (BUS-2.x-style: trait impl in mde-portal
  or mde-popover)
- New CLI subcommands for `mde-bus` (one verb per file, pure
  helpers, tempdir tests)
- New audit / retention / replay tooling that READS the file
  tree + SQLite (per-peer index reads are cheap)
- New pre-commit lints (the `lint-*.sh` family is open-ended)
- Material Symbols icon adoptions in still-Carbon code
- ChromeOS Classic styling pulls in still-PatternFly code

### 10.5 Design moves that will be rejected

- Anything that adds a network listener on `0.0.0.0` (LOCK Q60
  + lint-public-ports gate)
- Anything that re-introduces Tailscale / Headscale / Derp
  vocabulary in v2.5+ Nebula-native source (LOCK + lint)
- Anything that introduces a Python daemon to the supervisor
  by 1.0 (LOCK Q95)
- Anything that takes a hard dep on `axum` / `reqwest` in
  `mackesd` (keeps the daemon lean; shell-out + library-
  binary split is the pattern)
- Visual proposals without UX-* branch + before/after
  screenshots (LOCK §0.11)

---

## 11. Open Questions for Discussion

These are gaps the author identified while writing this
briefing. None block the 1.0 cut; all are worth surveying
post-1.0.

### 11.1 The post-1.0 retirement of single-passcode auth (GAP)

(INFERRED gap) The single-passcode model assumes the operator
runs all 3-8 devices. If the use case ever broadens — a
spouse's machine, a small family, a couple-of-friends sharing
a media library — the threat model changes. There's no
locked guidance on what triggers a re-survey.

### 11.2 What happens when an 8-peer cap is exceeded (GAP)

The 8-peer cap (LOCK Q3) is a *target sizing*, not a *hard
limit* in code. (INFERRED — searched for cap-enforcement code
and found none.) An operator could pair a 9th peer; the
behavior is unspecified. Worth a survey: do we want a hard
limit, a warning, or silent acceptance?

### 11.3 Federation between meshes (LOCK Q35 + Q55 — sketched, not built)

Federation is "subscribe-only Nebula-to-Nebula bridge"
(BUS-7.7) — but the OOB passcode pairing UX is only
sketched. The Workbench accept-pair UI doesn't exist yet,
and the symmetric grant model has no acceptance criteria
beyond "external mesh peer subscribes to fleet/announce."

### 11.4 What "Hard Drive Bench" actually tests (GAP)

HW-1..HW-4 are referenced but the actual test plan is not in
a central doc. Each HW task has its own acceptance bullets,
but a unified "what passes the bench" checklist would help
AI partners reason about gate readiness.

### 11.5 What replaces the in-process AppArmor / SELinux
posture (CLOSED 2026-05-26 by TUNE-12)

**Closed by [`docs/design/security-posture.md`](design/security-posture.md)**
(2026-05-26, TUNE-12 of the 25-Q tuning survey). The doc states
the deliberate posture explicitly: Fedora targeted SELinux as the
policy base; user-UID isolation as the process model (every
component runs under the operator's `$UID`); Nebula's
`CAP_NET_ADMIN` scoped via systemd `CapabilityBoundingSet` on the
unit file rather than a custom `nebula_mde_t` SELinux type
(deferral trigger documented); flat-trust threat model rationale
(Q51 + Q54 + Q56); a 10-row intra-mesh boundary list mapping
mechanism + lint gate per boundary; and a defense-in-depth-gap
table that names each control NOT implemented with its
deliberate-choice rationale. Cross-referenced from AI_GOVERNANCE.md
§7 + §13.1.

The original gap text is retained below for historical context.

> The platform documents no SELinux policy or AppArmor profile
> strategy. Fedora ships SELinux enforcing by default; MDE
> components don't ship per-component policies. This is fine for
> v1.0 if every component runs as the user's UID, but it's
> under-documented for security reviewers asking "what's your
> MAC posture?"

### 11.6 The "Calm enterprise" framing vs. the single-operator
sizing (INFERRED tension)

The visual identity (`docs/design/visual-identity.md`) brands
MDE as "calm enterprise" with reference targets like Linear,
Raycast, Arc, Apple System Settings. But the platform sizes
to 8 peers / one operator. Is the "enterprise" framing about
visual sophistication only, or does it imply latent ambitions
beyond the locked sizing? Worth resolving in a post-1.0
survey.

### 11.7 Backup destinations (LOCK Q59 — operator-configurable, scope unclear) (GAP)

Q59 locks "mesh-replicated + optional off-mesh upload
(S3/B2/SSH); operator-configurable." But there's no design
doc for the off-mesh upload module. Which crate owns it? What
encryption-at-rest scheme? When does it run? Worth a survey.

### 11.8 Conflict resolution UX (LOCK Q23 — high priority + ack required) (GAP)

Gluster LWW + `.conflict-<host>-<ts>` siblings is the file-
level resolution. The Bus `mesh/conflict` topic surfaces them.
But the *operator UX* for resolving conflicts (accept-version-A
vs accept-version-B vs keep-both) isn't designed. Falls into
the BUS-2.x surface design.

### 11.9 The post-cut migration of dropped memory files (INFERRED)

Per §14 of AI_GOVERNANCE.md, some memory files are
superseded (Carbon icons, Object Card 12 px, 16-peer fleet,
QNM-Shared, etc.). The supersession is documented but the
memory files themselves haven't all been edited to
cross-reference. This is mechanical hygiene + a useful first
post-1.0 task.

---

## 12. AI Design Guidance

### 12.1 Bottom line

> Make changes that the master rule ("Secure, Simple,
> Centerless Workgroup") would endorse. When in doubt,
> survey via `AskUserQuestion`. When even-MORE in doubt,
> ask the operator directly.

### 12.2 Five questions to ask before every change

1. **Does this change touch state that lives in the two
   substrates (Gluster mesh-home or Bus)?** If no, ask whether
   it should — most state belongs in one of the two.
2. **Does this change open a new network surface or D-Bus
   interface?** If yes, name the Q-lock that authorizes it +
   add the security review notes per §0.8 gate #8.
3. **Does this change run before the §0.8 runtime-reachability
   gate would catch a dead module?** If yes, name the entry
   point that invokes it.
4. **Does this change break the operator's existing memory
   files?** If yes, name which memory files need updates.
5. **Could this change ship as one commit?** If no, name the
   atomic split.

### 12.3 Mental model

Imagine the platform as **one operator's networked
workstation that happens to span 8 devices**. Every design
decision should make sense for that mental model:

- File continuity → "I edited this on my laptop; I expect to
  see it on my desktop."
- Notification coherence → "I got Slack on my desktop; my
  phone should know I've seen it."
- Voice → "I called Mom; either device's headset should pick
  up depending on which I'm wearing."
- Backup → "If my house burns down, I want my files in S3
  (operator-configured, optional)."

A proposal that doesn't fit this mental model probably needs
re-framing (or it's a genuine feature gap worth a survey).

### 12.4 Last-mile checklist before proposing

Before opening a worklist task or writing a design doc:

- [ ] Have I read AI_GOVERNANCE.md §1-11?
- [ ] Have I checked MEMORY.md for operator preferences on
      this surface?
- [ ] Have I searched the worklist for `[>] session=<id>`
      claims overlapping my proposal?
- [ ] Have I cited the Q-IDs that lock the relevant direction?
- [ ] Have I named what RETIRES if this proposal lands?
- [ ] Have I named the bench-observable acceptance criterion?
- [ ] Have I structured the task as a user story (As / I want
      / so that)?
- [ ] Have I picked the smallest viable scope (one commit, if
      possible)?

### 12.5 Failure modes to avoid

(From the actual incident history of the platform.)

- **Helpers without wiring** (v3.x audit on 2026-05-22 — 13
  dead panel modules). Always close the runtime-reachability
  gate.
- **Misleading `[✓]` marks** (same audit). Don't mark `[✓]`
  until the user-visible behavior actually works.
- **Silent scope deferral** (operator directive 2026-05-19 +
  §0.17 reinforcement). No `[~] Deferred` status; tasks are
  Open / In Progress / Done / Blocked.
- **Scaffold + stub mindset** (§0.12, codified 2026-05-22).
  No `todo!()`. No "phase 2 lands later." Every commit ships
  END-TO-END.
- **Bundling pre-staged work from parallel sessions into your
  commit** ([[feedback_check_pre_staged]]). Check `git status`
  + only stage YOUR files. Use `git add -- <explicit-path>`
  with the `--` separator.

---

## 13. Appendices

### 13.1 Glossary of platform-specific terms

| Term | Meaning |
|---|---|
| Birthright | Per-peer first-boot wizard that lays down theme, panel, mesh pairing |
| MDE-Workgroup | The coordinated set of 3-8 devices belonging to one operator (replaces QNM-Shared) |
| Mesh-home | The Gluster volume that holds XDG dirs + coordination paths |
| Lighthouse | A Nebula tie-break + relay node (arbiter brick for Gluster) |
| Peer | An enrolled Nebula node holding the mesh passcode |
| Phone | KDC2-bridged Android device — peers but not Nebula nodes |
| Bus | The pub/sub event + clipboard + audit substrate (BUS-1..7) |
| KDC2 | The native Mackes KDE-Connect-compatible phone bridge |
| Voice / Video | The 1.0-bundled SIP PBX (PJSIP + Vitelity + SimRing) |
| Operator | The one human running the workgroup |
| Survey | An N-Q `AskUserQuestion` design fork (≥3 options per Q66) |

### 13.2 Document map

| Document | Purpose | Authority order |
|---|---|---|
| `~/.claude/projects/.../memory/MEMORY.md` | Operator preferences | 1 (highest) |
| `.claude/CLAUDE.md` | Operational rulebook | 2 |
| `docs/AI_GOVERNANCE.md` | Platform identity + locks | 3 |
| `docs/AI_PLATFORM_REFERENCE.md` | This briefing (synthesis, not policy) | 4 |
| `docs/design/<epic>.md` | Per-epic locks | 5 |
| `docs/PROJECT_WORKLIST.md` body | Actionable task state | 6 |

### 13.3 First-session boot order for a new AI partner

1. Read AI_GOVERNANCE.md (§1-11 in full)
2. Read CLAUDE.md §0 (rulebook)
3. Read this document (§1-12)
4. Read MEMORY.md (index of memory files; load specific ones
   when relevant)
5. `git log --oneline -10` (recent commits)
6. `git status` (pre-staged work from parallel sessions?)
7. Open `docs/PROJECT_WORKLIST.md`; find next `[ ] Open` task
   under the targeted epic; check for `[>]` session markers
8. Claim with `[>] session=<id>` + start work

### 13.4 Maintenance policy for this document

Update when:

- A platform-wide pattern emerges that crosses ≥3 crates
- An outlier in §9 is retired (newer lock supersedes)
- A gap in §11 is closed (survey-resolved)
- A new release ships (refresh §7 + the 1.0 roadmap section)

Do NOT update for:

- Per-epic design changes (use the epic design doc)
- Operator preference shifts (use memory files)
- Routine task landings (use the worklist + CHANGELOG)

This document is the **strategic briefing layer** — kept thin,
synthetic, and durable. The Q-references stay even when
specific implementations evolve.

---

## 14. Author's note

This document was synthesized 2026-05-26 by Claude Opus 4.7
during the same session that drained the BUS-3 + BUS-4 + most
of BUS-1.x epics (17 commits, ~5000 LOC). The synthesis is
based on:

- Full read of `docs/AI_GOVERNANCE.md` (the 100-Q survey
  source of truth)
- Full read of `.claude/CLAUDE.md` §0 rulebook
- Index pass over `docs/design/` (17 epic design docs)
- Full read of the memory directory (31 files)
- Live grep over `crates/` to cross-check architectural claims
- Cross-reference against the just-shipped BUS-1..4 + BUS-7.1
  + BUS-2.1 implementations

The author's main observation: **this platform's strength
comes from how thoroughly it has resisted scope creep.** The
100-Q tightening survey reads as the operator deliberately
RETIRING options rather than adding them — Carbon icons,
Geologica fonts, Object Card 12 px, focus-mode catalogs,
mon_aggregator vs alert_relay duplication, D-Bus surfaces,
the 16-peer cap, Caddy gateway. Every "no" makes the next
"yes" cleaner. AI partners who understand that pattern will
propose well; AI partners who don't will keep proposing the
options the survey already retired.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
