# Mackes Desktop Environment (MDE) — Platform Design Brief

**Audience:** AI design partners (Claude sessions) before helping
design or build features in this repo.
**Last refreshed:** 2026-05-25 (post-BUS lock)
**Status:** SUPERSEDED by `docs/AI_GOVERNANCE.md` (locked 2026-05-25
via 100-Q tightening survey) for AI-design guidance going forward.
Kept here as legacy reference for the pre-100-Q-survey state. Read
`AI_GOVERNANCE.md` first.
**Status:** Living document — update when the platform's identity
or methods shift, not for routine feature work.

**Read this BEFORE:**
- Proposing new architecture or naming patterns
- Suggesting features that look adjacent to existing scope
- Recommending tools, frameworks, or services not already in use
- Writing design docs or worklist tasks for new epics

**Companion docs to consult after:**
- `.claude/CLAUDE.md` — operational rules (commits, gates, cuts)
- `docs/PROJECT_WORKLIST.md` — current + historical work (~15k lines)
- `docs/design/*.md` — 21 locked design specs (one per major epic)
- `~/.claude/projects/.../memory/MEMORY.md` — operator preferences +
  cross-session context

---

## 1. Identity

**Mackes Desktop Environment (MDE)** is a Wayland-only, Rust-based,
mesh-native Linux desktop environment for **small-business fleets**
(target: ≤ 8 peers per mesh, tightened from 16 per Q3 of the
100-Q tightening survey 2026-05-25). It runs on Fedora 44+, ships as
RPMs, and is operated by **a single engineer** (Matthew Mackes) in
collaboration with multiple Claude sessions.

It started life as `mackes-shell` — a GTK3 Python control panel
on top of XFCE — and rebranded to MDE at v2.0 (2026-05-19) when
it became a standalone Wayland DE. Both names appear in the
codebase; **MDE** is the v2.0+ canonical name (binaries `mded`,
`mde-*`; package `mde`; D-Bus `dev.mackes.MDE.*`; config
`~/.config/mde/`; CSS `.mde-*`). The package spec still lives at
`packaging/fedora/mackes-shell.spec` for historical-tag reasons.

**Tagline (operator framing):**
> *"A Mackes mesh is one user's many machines, not N independent
> desktops."*

Every architectural decision flows from this premise.

---

## 2. Goals (in priority order)

### 2.1 Single user, many machines
The operator owns 2-16 machines (desktops, laptops, lighthouses,
phone-pinned tablets). They should feel like one logical
workstation that follows the user across hardware. Files,
notifications, DND state, focus modes, app configs, peer trust —
all sync.

### 2.2 Flat trust, open mesh
*Lock:* [[project_open_mesh_directive]] (2026-05-23).
Every enrolled peer **fully trusts** every other peer. One
passcode handles all auth (no per-node ACLs). The mesh is the
trust boundary; once you're in, you're in. This eliminates the
"who can access what" complexity that crushes most mesh
deployments at small scale.

### 2.3 Self-hosted, no SaaS
No cloud accounts, no remote APIs for control plane, no
third-party identity providers. Every dependency is in-repo or
on-peer:
- **Transport:** Nebula (self-hosted PKI, axed Tailscale/Headscale
  in v2.5)
- **Storage:** GlusterFS mesh-home (axed cloud-sync in v5.0)
- **Notifications:** GF-17 unified bus over QNM-Shared (v5.1)
- **Telemetry:** Netdata streaming aggregator (axed Prometheus in
  v2.6)
- **VoIP:** Kamailio + Vitelity trunks (v4.x)
- **Music:** native Airsonic client (v6.x AIR-*)

### 2.4 Local-first, offline-capable
Every peer holds every file (gluster full-mesh replica). Every
peer holds every notification history row (NotificationRelayWorker).
Disconnect a peer for a week → it operates fully → on reconnect,
sync converges in seconds.

### 2.5 Operator visibility + control
Three surfaces, three jobs:
- **Portal** (`mde-portal`) — at-a-glance state + navigation
  (the always-on shell)
- **Workbench** (`mde-workbench` / `mackes/workbench/`) — control
  + configuration (the settings app)
- **Netdata web UI** — drill-down telemetry (the metrics surface)

### 2.6 Aesthetic clarity — Classic ChromeOS
*Lock:* [[project_chromeos_classic_visual_lock]] (2026-05-24, 22-Q).
Flat pre-2022 ChromeOS visual language. `#202124` palette. Roboto
+ Roboto Mono. 28 px rows. 4 px corners. 48 px Shelf. Hover-expand
56→256 px sidebar. Solid indigo selection. This replaced an earlier
Carbon refresh (v1.1.0) which replaced an earlier PatternFly attempt
(v2.0 NF-* draft). **Three design-system pivots in one year —
treat the active lock as authoritative; do not re-introduce Carbon
or PatternFly tokens.**

---

## 3. Mental model

### 3.1 Two-primitive design language (v6.0)
*Lock:* [[project_v6_0_mde_portal]] (2026-05-24/25, 523-Q across
10 rounds).
> *"Every object is a card. Every navigation is a breadcrumb."*

Cards have 6 render modes (segment / cascade-card / list-row /
mini-tree-cell / lock-widget / hero). Apps, files, peers,
contacts, containers, workspaces — everything renders as one of
those modes. Breadcrumbs are the navigation primitive in Portal
+ Workbench. Search is **removed** (Portal-* lock; the breadcrumb
+ tag system replaces it).

### 3.2 Three Portal layers
Portal-full has three layers:
- **Hub** — entry point, the M-button + cards landing surface
- **Library** — every object the mesh knows about (apps, files,
  peers, contacts)
- **Control** — settings, status, system actions

Plus 6 Wayland surfaces: Dock / Portal-compact (Mod+Space mesh-
glance globe) / Portal-full / Lock / Theater / Mesh-Wallpaper.

### 3.3 Roles at birthright
Every peer gets a role at birthright (the first-run wizard):
- `lighthouse` — Nebula coordinator, no GUI, default focus: `off`
- `host` — desktop workstation, default focus: `work`
- `peer` — laptop / tablet / phone-pinned, default focus: `work`

Roles determine: which workers spawn, which D-Bus services
register, which sidebar groups appear, which birthright steps
run.

### 3.4 QNM-Shared coordination substrate
`~/QNM-Shared/` is the cross-peer shared directory replicated
between every peer. Pre-v5.0 this was Syncthing-replicated; now
overlapping with gluster mesh-home (but **QNM-Shared is not
inside the XDG mesh-home volume** — they're parallel sync
mechanisms). Every cross-peer coordination file lands here:
- `<qnm_root>/<peer>/mackesd/nebula-bundle.json` — Nebula CA + keys
- `<qnm_root>/<peer>/mackesd/netdata-aggregator.json` — elected
  leader pointer
- `<qnm_root>/<peer>/mackesd/notifications/<ulid>.json` —
  notification bus events (v5.1)
- `<qnm_root>/<peer>/mackesd/attendance.json` — focus + idle
  state (v5.1)
- `<qnm_root>/<peer>/mackesd/focus-profile.json` — focus mode
  profile (v5.1)
- `<qnm_root>/<peer>/.qnm-notifications/*.json` — notification
  history rows

**Pattern:** every cross-peer fact is a file in QNM-Shared, polled
by interested workers. No event bus, no message queue, no NATS
(NATS was tried and axed in v5.0).

---

## 4. Architecture

### 4.1 Stack summary

| Layer | Technology | Replaces |
|---|---|---|
| Display server | Wayland (sway) | XFCE/X11 (v2.0 cut) |
| Window manager | sway via swayipc-async | i3, xfwm4 (v2.0) |
| Daemon | `mded` (Rust, single binary, tokio) | 9+ python services |
| UI surfaces | Iced + libcosmic + smithay-client-toolkit | GTK3 Python |
| Panel/overlay | wlr-layer-shell | xfce4-panel, mackes-panel |
| IPC | zbus 5 (`dev.mackes.MDE.*`) | dbus-python |
| Storage | GlusterFS + SQLite (per-peer) | SSHFS, cloud-sync |
| Transport | Nebula overlay (10.42.0.0/16) | Tailscale, Headscale, DERP |
| Metrics | Netdata streaming aggregator | Prometheus, mesh_metrics.py |
| Notifications | `org.freedesktop.Notifications` via mded | notify-osd, fork |
| Packaging | RPM (Fedora 44+) | n/a |
| Display manager | greetd + regreet (v2.7) | LightDM |
| First-run | birthright wizard (Python) → mde-wizard (Rust, in flight) | n/a |

### 4.2 Crate layout (27 crates)

**MDE-prefixed (v2.0+, the canonical line):**
- `mded` — Rust services daemon (single binary, supervisor + workers)
- `mde-portal` — at-a-glance shell (Dock + Portal-compact + Portal-full)
- `mde-workbench` — control + configuration UI (Iced rewrite, in
  flight; replacing `mackes/workbench/` Python tree)
- `mde-files` — file manager (forked from pop-os/cosmic-files,
  Iced, mesh-first sidebar)
- `mde-panel` — sway-side panel chrome (status zone, app-switcher)
- `mde-popover` — popover surfaces (start menu, notifications)
- `mde-drawer` — slide-out drawer (replaced by Portal in v6.0)
- `mde-applets` — 17 sub-crates, one per applet (apple-menu,
  app-switcher, audio, bg, brightness-osd, clock, dock, mesh-status,
  network, notification-bell, notifications, recents, start-menu,
  status-cluster, sway-cluster, volume-osd, applet-api)
- `mde-wizard` — first-run wizard (Rust port of `mackes/wizard/`)
- `mde-session` — session boot orchestration
- `mde-kdc` + `mde-kdc-proto` — native KDE Connect (v2.1 KDC2)
- `mde-iced-components` — shared Iced widgets
- `mde-theme` — theme runtime
- `mde-config` — config loader
- `mde-peer-card` — peer card modal (cross-Portal pattern)
- `mde-voice-config` — voice config materializer
- `mde-logout-dialog` — logout dialog
- `mde-mesh-types` — type re-export facade over `mackes-mesh-types`
- `mde-alert-emit` — Netdata alert → JSON ULID emitter (MON-3)

**Mackes-prefixed (v1.x or shared infra):**
- `mackes-panel` — legacy GTK panel (retired in favor of mde-panel,
  allow-listed by lint)
- `mackes-theme` — legacy theme module
- `mackes-config` — legacy config
- `mackes-mesh-types` — mesh resource types (source of truth)
- `mackes-transport` — KDC2-1 Transport trait + capability + scorer
- `mackes-nebula-https-tunnel` — Nebula-over-HTTPS-443 activation

### 4.3 mded worker model

`crates/mackesd/src/workers/` holds 30+ workers, each spawned by
the `Supervisor` with a `RestartPolicy` (`Always` / `OnFailure` /
`Never`). Standard worker shape:

```rust
pub struct FooWorker { /* state */ }

#[async_trait::async_trait]
impl Worker for FooWorker {
    fn name(&self) -> &'static str { "foo" }
    async fn run(&mut self, mut shutdown: ShutdownToken) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                biased;
                _ = shutdown.wait() => return Ok(()),
                _ = tokio::time::sleep(TICK) => {}
            }
            if let Err(e) = self.tick().await {
                tracing::warn!("foo tick failed: {e}");
            }
        }
    }
}
```

**Recurring patterns inside `tick()`:**
- Pure-fn helpers (`bootstrap_argv`, `parse_pool_list`,
  `elect_attended`, `should_coalesce`) extracted for unit-testing
  without subprocess shelling
- Atomic-write tempfile + fsync + rename for every state file
- Polling-based discovery via QNM-Shared (`scan_*_pointers`,
  `peer_probe_targets`) — never event bus
- Builder-pattern config: `with_qnm_peer_discovery(root, node_id)`,
  `with_tick(duration)`, `with_signal_sender(sender)`
- Mutex<rusqlite::Connection> shared state via `Arc<Mutex<...>>`
- `ShutdownToken::from_receiver` for clean SIGTERM
- `tracing::{info, warn, error}` everywhere; structured fields
  via `?` syntax

### 4.4 D-Bus surfaces (`dev.mackes.MDE.*`)

All on the shared `org.mackes.mackesd` connection. Current 6
services:
- `dev.mackes.MDE.Nebula.Status` — mesh fabric state
- `dev.mackes.MDE.Gluster.Status` — file-replication state
- `dev.mackes.MDE.Portal` — Goto/Focus/Lock/ToggleDND/Activity
- `dev.mackes.MDE.Shell.Workers` — worker introspection
- `dev.mackes.MDE.Shell.Healthz` — Prometheus-style health endpoint
- `dev.mackes.MDE.Notifications` — FDO Notifications interface

Method shape: `async fn method() -> zbus::fdo::Result<String>`
returning JSON (lowest-effort cross-language). Signal shape:
`#[interface]` block declares; mpsc channel + `SignalEmitter` for
async dispatch.

### 4.5 The "subprocess legacy layer" pattern

`mded` workers like `media_sync.rs` and `fs_sync.rs` supervise
legacy Python subprocesses (`mackes/media_sync_daemon.py`,
`mackes/mesh_gvfs/daemon.py`). This is the transitional bridge
during the v2.0→v5.x Python-to-Rust migration; DEAD-2.12 retires
the gvfs one in v5.2.

---

## 5. Design patterns

### 5.1 The N-Q survey lock
Before any non-trivial epic, the operator asks Claude to fire an
N-question survey via `AskUserQuestion` (one question at a time
per [[feedback_question_workflow]]). Each answer is a permanent
lock that the design doc + worklist references.

**Scale:** 5-Q (small features), 10-Q (workflows), 22-Q (visual),
25-Q (substrate), 50-Q (design rounds), 523-Q (v6.0 Portal across
10 rounds). The 523-Q lock is the largest formal design exercise
in the codebase.

After the survey, Claude:
1. Writes `docs/design/<epic>.md` capturing every lock in a
   table + the resulting architecture
2. Adds a `<EPIC>-*` section to `docs/PROJECT_WORKLIST.md` with
   user-story tasks (`As/I want/so that` + bench-observable
   acceptance bullets)
3. Implements each task as a separate commit

### 5.2 Worklist task format
*Lock:* `.claude/skills/mackes-worklist-management/SKILL.md`.

```
- [ ] **<PREFIX>-N.M: <release> — <short title>** *(optional carve-out tag)*
  **As** <role>,
  **I want** <capability>,
  **so that** <outcome>.
  **Acceptance** (each bench-observable):
    - [ ] specific bench-observable bullet
    - [ ] specific bench-observable bullet
```

Status legend: `[ ] Open`, `[>] In Progress`, `[✓] Done`,
`[!] Blocked`. `[~] Deferred` was retired 2026-05-19.

Every task carries a release-tag prefix per WF-5 (e.g.,
`v2.0.1:`, `UX-14:`, `GF-2.5:`, `DEAD-2.12: v5.2 —`).

### 5.3 Definition of Done (7 gates)
*Lock:* `.claude/CLAUDE.md` §0.8.
1. Committed to `main`
2. Pushed to BOTH remotes (origin + mde-x)
3. RPM builds clean (`make rpm` exit 0)
4. Tagged + released (for ship versions)
5. All module imports clean (`python3 -c "import mackes.<module>"`
   for every touched module)
6. CHANGELOG updated
7. Runtime-reachability — every new public function invocable
   from a runtime entry point (no `pub mod foo;` with zero
   non-self references)

### 5.4 No stubs / skeletons / staged work
*Lock:* `.claude/CLAUDE.md` §0.12; memory [[feedback_no_stubs]] +
[[feedback_helpers_vs_wired]].

Every commit ships **fully complete** code. No `todo!()`, no
`unimplemented!()`, no `panic!("not yet")`. No "phase 2 lands
later" commit messages. No `pub mod foo;` declarations without
external consumers. If a task can't ship complete in one commit,
**split it at write-time** into smaller tasks each of which CAN
ship complete.

**Origin of the rule:** v3.0.3 audit (2026-05-22) found 13 of 18
panel modules marked `[✓]` had shipped helpers + tests but were
unreachable at runtime — 4 user-visible bugs followed. The
runtime-reachability gate + this rule are the upstream prevention.

### 5.5 The wholesale-retire pattern
*Lock:* NF-5.1 (2026-05-24). Now the template for the entire
DEAD-2.x retirement queue.

When deleting a module, **don't migrate consumers one-by-one.**
Audit: are the import sites already wrapped in `try/except
ImportError`? Most v1.x mesh modules already are (graceful
degradation pattern from the Tailscale era). If yes, delete the
module wholesale; consumers degrade to empty / not-joined stand-
ins automatically. If not, add the try/except in the same commit
as the deletion.

### 5.6 Pre-commit gates per §0.7
For every commit, applicable gates:
1. **Module-import smoke** (`python3 -c "import mackes.<X>"`)
2. **Tests** (`make test-nodeps`)
3. **Ruff lint** (`make lint` — F401/F541/F811/F841 only)
4. **RPM build** (`make rpm`) when packaging/spec/data touched
5. **CSS lint** (`install-helpers/lint-css.sh`)
6. **Voice-and-tone lint** (`install-helpers/lint-voice.sh`) —
   enforces verb-discipline + forbidden-strings list from
   `docs/design/voice-and-tone.md`
7. **Legacy-mesh-vocabulary lint** (`install-helpers/lint-legacy-mesh.sh`)
   — catches net-new `tailscale|headscale|derper` references

Hook failures → fix + create NEW commit; **never `--amend`** the
failed one.

### 5.7 Hardware Testing carve-out
*Lock:* memory [[feedback_hardware_testing_epic]] +
[[feedback_no_cut_until_worklist_empty]].

Multi-peer bench tests live in the "Hardware Testing" epic at
the bottom of `PROJECT_WORKLIST.md`. They're tagged `[HW carve-
out]` inline on individual tasks. They **never gate a release**
— they're a parallel sign-off pass against an already-feature-
complete cut. The drain rule "don't cut until worklist is empty"
explicitly excludes HW carve-outs.

### 5.8 Cut-release shorthand
*Lock:* `.claude/CLAUDE.md` §0.6. When operator types `cut release
X.Y.Z`:
1. Bump version in 4 files (`mackes/__init__.py`, `pyproject.toml`,
   `setup.py`, `packaging/fedora/mackes-shell.spec`)
2. CHANGELOG entry at top
3. Smoke test (`python3 -c "import mackes; print(mackes.__version__)"`)
4. Local RPM build (`make rpm` — never `rpmbuild --short-circuit`
   directly; that stamps `rpmlib(ShortCircuited)` which makes the
   RPM uninstallable)
5. Commit + push + tag + push-tag (dual remote)
6. Watch the workflow (`gh run watch <id> --exit-status`)
7. Confirm via `gh release view vX.Y.Z`

### 5.9 Dual-remote push discipline
*Lock:* `.claude/CLAUDE.md` §0.2 (updated 2026-05-23).
Every push to `main` lands on BOTH `origin` (MAP2-RELEASES) +
`mde-x` (MDE-X). The protected-ref bypass message from origin
("Cannot update this protected ref") is expected; the push
succeeds anyway.

```bash
git push origin main && git push mde-x main
```

### 5.10 Visual work uses a PR-based branch lane
*Lock:* `.claude/CLAUDE.md` §0.11 (WF-1, 2026-05-21). All code
ships to `main` directly — except UX-* / visual-design work,
which uses `ux/<task-id>` short-lived branches with before/after
screenshots required in PR descriptions. The merge gate is an
explicit operator OK on the PR (CI-green is necessary but not
sufficient).

---

## 6. Methods of operation

### 6.1 Single-engineer + multi-AI development
The operator is one person (Matthew Mackes). Most code is written
in collaboration with Claude (Opus + Sonnet, multiple parallel
sessions). Every commit carries:
```
Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

There are no team PRs (except UX-*), no issue tracker, no code-
review workflow. The worklist is the queue; memory notes are the
durable preferences; design docs are the locks.

### 6.2 Standing commit/push authorization
*Lock:* memory [[feedback_push_commit_auth]] (2026-05-25). The
operator has authorized commit + push without per-operation
confirmation. Claude commits and pushes immediately when work is
complete + gates pass; does NOT ask "should I push now?"

### 6.3 Memory as cross-session continuity
`~/.claude/projects/-home-mm-Desktop-files-mackes-shell/memory/`
holds ~30 typed memory files (user / feedback / project / reference).
Index in `MEMORY.md`. Memory is for facts that should survive
session boundaries:
- Locks (design surveys, role defaults, brand decisions)
- Feedback (don't bundle pre-staged work, don't cut until drained,
  no stubs)
- Project state (active epics, retirements in flight, version
  trajectories)
- References (pointers to external tools, dashboards, projects)

NOT for: code patterns (the code is the source), ephemeral
session state (use tasks), or anything already in CLAUDE.md.

### 6.4 Skills as scoped behaviors
`.claude/skills/` holds slash-command-invokable skills:
- `mackes-worklist-management` — worklist schema enforcement
- `complete-remaining-work` — autonomous queue draining
- `autonomous-worker` — single-task autonomous mode
- `iteration` — open-ended grind with rescue pass
- `batch` — group-and-execute mode
- `frontend-design`, `verify`, `code-review`, `init`, `review`
  — utility skills

The `iteration` skill encodes standing authorizations: commit
anytime, best-choice decisions, scope/design improvements, new
tasks, chrome upgrades. Carbon Icon Set is the locked
iconography source per the skill body.

### 6.5 Worklist as primary planning artifact
`docs/PROJECT_WORKLIST.md` is ~15,000 lines and the **single
canonical task list**. Claude Code's in-session TaskList /
TaskCreate / TaskUpdate are ephemeral scratch only. Design docs
(`docs/design/*.md`) are not parallel worklists; every actionable
item from a design doc must be lifted into the worklist as `[ ]
Open`.

When directives contradict, the newer one wins silently. The
worklist tracks live policy; design docs keep historical
context.

---

## 7. Direction (active scope, 2026-05-25)

### 7.1 In flight — v5.0/v5.1/v5.2 (storage + retirements)
- **v5.0** — GlusterFS mesh-home: 29 of 35 GF-* tasks shipped; 6
  HW carve-outs remain (mesh-files badges, KDC2 drop folder,
  applet status line, Workbench panel, quota banner, split-brain
  bench tests)
- **v5.1** — Gluster control surface: 10 GF-16.* tasks queued
  (pause/throttle, action notifications, coalescing, class policy,
  DND pierce, origin xattr, decommission notifications, etc.).
  **Note:** notification-routing portions are SUPERSEDED by BUS-*
  (see §7.5); residual GF-16 work is the gluster operator-control
  layer only
- **v5.1** — ~~Mesh notification bus + focus modes (GF-17)~~
  **SUPERSEDED 2026-05-25 by v6.x Mackes Bus (BUS-4.2 hard cut)**.
  GF-17 tasks retain their state for historical context until
  BUS-4.2 lands; on that commit they convert to `[~]` Retired
- **v5.1/v5.2** — DEAD-2.* mesh module retirement queue: 15 tasks
  across 10 dependency waves

### 7.1.b In flight — v6.x Mackes Bus (BIG NEW LOCK 2026-05-25)
**Status:** Design-locked via 104-Q poll across 26 rounds (largest
single-platform lock in the repo). Awaiting operator "execute" /
"iterate" / "ship the Bus" before implementation. Bound by the cut
drain rule.

**What it is:** the **single bus** for every event the mesh
produces — notifications, alerts, clipboard sync, FDO desktop
notifications, webhook ingress, mesh-internal pub/sub. Built on
self-hosted **ntfy** brokers running on every peer over Nebula,
with persistence on GlusterFS mesh-home + per-topic file tree at
`~/.local/share/mde/bus/<topic-path>/<ulid>.json`.

**What it replaces in one v6.x cut:**
- v5.1 GF-17 mesh notification bus → **hard cut**, delete + rewrite
  callers
- v2.6 MON Netdata alert routing → **parallel-write window**;
  `~/.local/share/mde/alerts/` JSONL preserved for external
  consumers
- Standalone FDO `org.freedesktop.Notifications` → **bridged to
  `fdo/<app>` topics** (every desktop notification auto-captures
  into Bus audit + replay)
- Any per-app clipboard sync → **`clipboard/sync` topic** + new
  `mde-clipd` daemon (`wlr-data-control-unstable-v1`)

**Topic model:** slash hierarchy (`fleet/sec`, `peer/$host/alerts`,
`mon/cpu`); MQTT wildcards (`+` / `#`); self-serve creation; 12
curated defaults seeded on first run.

**Priority → surface map:**
- `min` → silent log only (no Breadcrumb segment)
- `default` → Portal notification tray + dock badge
- `high` → status-zone slide-up strip + sound + persistent until ack
- `urgent` → Theater takeover (full-screen) + sound + Wallpaper
  banner stripe + phone push

**Sub-epics (parallel, no enforced order):**
- **BUS-1** Foundation — `mde-bus` crate, broker, persistence, CLI,
  templating (Tera + `{{exec}}` + `{{include}}` + curated mesh vars)
- **BUS-2** Surfaces — Breadcrumb / tray / strip / Theater /
  wallpaper / `mde://` URL handler / single DND toggle
- **BUS-3** Webhooks — ntfy publisher + YAML rules + 6 built-in
  adapters (GitHub, Gitea, Sonarr/Radarr, UPS/NUT, Home Assistant,
  generic JSON)
- **BUS-4** Migration — GF-17 hard cut + MON parallel-write + FDO
  bridge
- **BUS-5** Clipboard — `mde-clipd` + Super+V centered popover +
  KDC2 round-trip + tag pinning
- **BUS-6** Advanced routing — rooms + first-to-ack + correlation
  engine + DM-by-active-peer + broadcast snooze + phone dedup
- **BUS-7** Federation + audit + Workbench Mesh > Bus subpage

**Key resolutions baked into the lock:**
- Single DND toggle + per-topic mute/snooze (REPLACES the v5.1
  3-mode focus catalog work/quiet/off)
- `override=dnd` tag bypasses everything
- Fleet-wide DND sync (DND on any peer mutes all peers)
- Phone reach via dual KDC2 + ntfy mobile app, deduplicated by ULID
- Topic ACL via passcode tier 2 → **CUT** (violates flat-trust)
- E2E body encryption per topic → **CUT** (Nebula transport is
  enough)
- Cron / scheduled publish → **CUT** (use systemd timers)
- Calendar / SMS / Piper TTS / geofence → all **CUT**

### 7.2 In flight — v2.6 (visual + monitoring)
- **CR-*** — ChromeOS Classic visual retrofit (26-Q lock)
- **MON-*** — Netdata monitoring + alert routing (in progress)
- **RD-*** — Wayland VNC server gap

### 7.3 In flight — v2.7 (install + display manager)
- **INST-1..15** — `mde-installer` crate + paired updater
- **DM-1..8** — greetd + regreet replacing LightDM

### 7.4 In flight — v4.x (voice/video)
- **VV-** + Voice & Video epic (Kamailio + Vitelity + Opus)
- **v4.2** Voice PBX epic

### 7.5 In flight — v6.0 (unified shell)
- **Portal-*** — mde-portal Dock + Portal-compact + Portal-full
- **VOIP-*** / **MESH-*** / **CONTAINER-*** — Portal-side wiring
- **AIR-*** (v6.x) — native Airsonic music player

### 7.6 Constants
- Fedora 44+ only (no other distros, no other release lines)
- Wayland-only (sway), no X11 fallback
- Rust for new daemon + UI work
- Python for legacy + transitional layers being retired
- RPM packaging only

### 7.7 The pattern in epic naming
Different epic prefixes signal different domains:
- `GF-*` — GlusterFS file sync
- `NF-*` — Nebula fabric
- `MON-*` — Netdata monitoring
- `DEAD-*` — retirements
- `UX-*` / `BR-*` — visual / brand
- `WF-*` — workflow infrastructure
- `Portal-*` / `VOIP-*` / `MESH-*` / `CONTAINER-*` — v6.0 portal
- `WM-*` / `WB-*` — window management / workbench
- `CB-*` — cut blockers
- `RD-*` — Wayland VNC
- `OV-*` — Workbench Overview
- `CR-*` — ChromeOS visual retrofit
- `INST-*` / `DM-*` — installer / display manager
- `KDC2-*` — native KDE Connect
- `AIR-*` — Airsonic music
- `MIGRATION-*` — schema migrations

---

## 8. Things that are "outside the norm" — for discussion

This section lists features, patterns, and decisions that are
either (a) outside conventional desktop-environment scope, (b)
outside the platform's own stated patterns, or (c) deserve
discussion before being mimicked. Each item is a discussion
prompt, not a criticism — many are deliberate, but a fresh AI
should flag them rather than silently extend them.

### 8.1 Scope outliers — "is this really a DE?"

| # | Feature | Why it's outside DE norm | Status |
|---|---|---|---|
| 1 | **GlusterFS** as XDG file backend | Gluster is enterprise distributed storage. Using it for `~/Documents` replication is creative but unusual; most DEs leave file-sync to Syncthing/Dropbox/cloud | Locked v5.0; central to platform |
| 2 | **Kamailio + Vitelity** PBX | Telephony-server stack in a DE; most DEs treat calling as an app concern | Locked v4.x; integrated into Portal |
| 3 | **Caddy reverse proxy** on every peer | HTTPS gateway for cross-peer service exposure (`https://media.mesh/<service>/<peer>/`); most DEs don't ship a web server | `mackes/caddy_gateway.py` live; tied to mesh-services (being retired) |
| 4 | **Ansible-pull** on every peer | Config-mgmt infrastructure for an 8-peer fleet — most fleets that small use SSH scripts | Locked v1.3 Fleet; 7 curated playbooks in QNM-Shared |
| 5 | **Native Airsonic music client** | Bundling a music app in the DE; could be a separate app | Locked v6.x AIR-*; 30-Q survey |
| 6 | **Headless mode** (`mackes/headless/`) | A "desktop environment" with full CLI-driven no-GUI mode for lighthouses + servers | Live; tied to `role: lighthouse` |
| 7 | **Birthright wizard does UID renames** | `usermod -u 1000` + recursive chown of `$HOME` is unusually invasive for a first-run wizard | Locked GF-3.1; required for gluster UID stability |
| 8 | **mesh:// URI scheme** | Custom URI scheme registered with Thunar for browsing peer dirs | Being retired (DEAD-2.11) since gluster makes XDG dirs natively mesh-mounted |

### 8.2 Architectural outliers — pattern conflicts

| # | Pattern | Tension | Status |
|---|---|---|---|
| 9 | **mesh_router + mackes-transport** abstraction with Nebula-only world | 4 TransportKind variants + scorer + capability model, but only Nebula has 3 modes + KdcTls for phones. Abstraction may be over-engineered | Live; justified by KDC2 |
| 10 | **Polling-instead-of-event-bus** (shipped) | Original GF-2.5 design called for `nebula_supervisor::EnrollmentCompleted` event subscription; shipped as filesystem polling of QNM-Shared. Pattern repeats: GF-2.6 (peer detach), GF-17 (attendance) | Established convention; **partially superseded** by BUS pub/sub once it ships |
| 11 | **mded `Notify` writes to BOTH local DB AND fires `notify-send`** | Two side effects from one call; consumers may double-process. With BUS the pattern collapses into one publish (BUS-4.4 bridge) | **Resolved by BUS-4.4** once it ships |
| 12 | **Three parallel sync substrates**: QNM-Shared (Syncthing-era, all coordination), gluster mesh-home (v5.0, XDG dirs), ntfy/Bus (v6.x, events+clipboard) | Three different replication mechanisms on the same peers, each with different latency/durability/scope guarantees | Active; no plan to unify (Bus uses GFS for index + file tree, ntfy for transport) |
| 13 | **Subprocess legacy layer** (`media_sync.rs` supervises `media_sync_daemon.py`) | Rust worker drives Python subprocess; transitional pattern | Live during the v2.0→v5.x migration |
| 14 | **D-Bus methods return JSON strings** (not strongly typed) | Cross-language ease but loses type safety; consumers parse JSON every call | Established convention |
| 14b | **`{{exec 'cmd'}}` in Bus templates** (Tera) | Any peer with the mesh passcode can publish a template that shells on every render-target peer. Designed flat-trust amplifier | Locked + documented in `docs/design/v6.x-mackes-bus.md` §10 |
| 14c | **GF-17 superseded same-day as locked** | 5-Q lock + design doc + 11 worklist tasks shipped 2026-05-25; SAME-DAY 104-Q BUS lock retires it. Hard-cut, no migration window | Process pattern — design churn is normal here; treat any "just locked" epic as potentially superseded if a larger system-level lock follows |

### 8.3 Design-system drift

| # | Issue | Detail | Status |
|---|---|---|---|
| 15 | **Three design-system pivots in one year** | PatternFly (v2.0 draft) → Carbon (v1.1.0) → ChromeOS Classic (v2.6+, locked 2026-05-24) | Active lock: ChromeOS Classic |
| 16 | **Carbon refresh helpers still being used** in new code | GF-8.2, GF-17.10 worklist items target `mackes/workbench/` using Carbon helpers (`_breadcrumb`, `_page_title`, etc.) even though Carbon is replaced | Inertia; new panels should use ChromeOS Classic tokens |
| 17 | **Object Card pattern breaks the 4 px platform rule** | Memory [[project_object_card_pattern]]: 12 px rounded corners on cards inside a 4 px-rounded UI. Intentional but anomalous | Locked 2026-05-24 |
| 18 | **Three icon-set decisions overlapping** | Carbon (initial v1.1.0), Lucide/Phosphor (UX rounds), Carbon-locked-again (per iteration skill body). Memory [[project_ux_polish_locks]]: "Carbon icons (overrides Lucide/Phosphor)" | Carbon wins; verify on new icon additions |

### 8.4 Legacy carry-overs

| # | Carryover | What it implies | Status |
|---|---|---|---|
| 19 | **Spec file path** `packaging/fedora/mackes-shell.spec` (not `mde.spec`) | Tag history forces this path | No plan to rename; tags reference it |
| 20 | **Python `mackes/` tree** still ~44 k LOC vs Rust ~138 k LOC | Half the codebase is legacy Python being incrementally retired | DEAD-2.* + future epics targeting this |
| 21 | **`mackes-panel` crate** still in repo | v1.x GTK panel; allow-listed in NF-20.6 lint | Slow retirement; allow-listed by directory prefix |
| 22 | **`mackes/workbench/` GTK Python** + `crates/mde-workbench/` Iced Rust | Two workbench implementations; v6.0 hard-cut merges mde-files + mde-workbench (per memory). New GF-8.2 panel still targets Python tree | Tension; coordinate panel additions with the v2.0+ Iced tree |
| 23 | **`mackes-mesh-types` + `mde-mesh-types` dual-naming** | mde-mesh-types is a "Phase 0.2 transitional re-export facade" over mackes-mesh-types. New code uses either | Transitional; v6.0 may unify |
| 24 | **Version sprawl** — v1.x, v2.x, v3.x, v4.x, v5.x, v6.x, v8.x, v12.x in worklist | Some are real releases (v2.0.0, v2.0.3, v3.0.2); others are design-doc identifiers (v8.7, v12.0). Mixed semantics | Confusing; current line is v4.0 (CHANGELOG) but new epics target v5.x+ |

### 8.5 Process patterns unusual for a single-developer project

| # | Pattern | Why unusual | Discussion |
|---|---|---|---|
| 25 | **14,983-line worklist** | Most teams use Jira / Linear / GitHub Issues. The single-file pattern survives only because the operator + Claude pair can fit it in context | Works for now; may need section archival as it grows |
| 26 | **523-Q + 104-Q design-survey locks** | v6.0 Portal has 573 decisions across 10 rounds; Mackes Bus has 104 across 26 rounds (largest per-platform lock). Most projects use 5-10 design decisions per epic | Effective for pre-empting design churn; expensive to reverse |
| 27 | **Voice-and-tone lint script** | Enforces verb discipline + forbidden-strings list. Most projects rely on review | Catches drift; mandatory for any user-visible string touch |
| 28 | **Legacy-mesh-vocabulary lint** | Catches net-new `tailscale\|headscale\|derper` references. Tribal-knowledge-as-lint | Pragmatic regression detection; pattern worth imitating for future deprecations |
| 29 | **§0.8 runtime-reachability gate** | "Every public function the task introduces must be invocable from a runtime entry point." Most projects don't formalize this | Direct response to v3.x dead-modules audit; should be respected on every commit |
| 30 | **AI co-attribution on every commit** | `Co-Authored-By: Claude Opus 4.7 (1M context)` | Established convention; respect it |
| 31 | **Memory notes are partially-authoritative** | Cut policy ([[feedback_no_cut_until_worklist_empty]]) lives in operator memory, not in repo docs. New AI session must read both | Acknowledged trade-off; lifting critical memories into CLAUDE.md may help |

### 8.6 Naming / branding patterns

| # | Pattern | Note |
|---|---|---|
| 32 | **"Mackes"** brand based on operator's last name | Personal-brand product naming; intentional |
| 33 | **"Birthright" metaphor for first-run wizard** | Unusual word choice; locked terminology |
| 34 | **"QNM-Shared"** acronym not expanded in any doc | Likely "Quincy Network Mesh"; historical; survives |
| 35 | **"hashbang" preset** as the default | CrunchBang nod; per memory [[project_default_preset]] |
| 36 | **"DEAD-2" epic numbering** | Suggests "DEAD-1" preceded; correct (DEAD-1 = `metrics_flush` retirement, 2026-05-25). Pattern is reusable |

### 8.7 Decisions that are explicitly contrary to convention

| # | Decision | Convention | Lock |
|---|---|---|---|
| 37 | **Plaintext glusterd inside Nebula tunnel** | Most distributed FS deployments use TLS even inside an overlay | Q3 lock: "Nebula overlay only — no second TLS layer" |
| 38 | **No version history / snapshots** for mesh-home | Most file-sync systems offer version history | Q17 lock: "live state only" |
| 39 | **No per-user gluster quotas** | Per-user limits are standard | Q16 lock: "fleet-wide cap = 0.8 × min(free brick)" |
| 40 | **Hard-cut migrations** (DEAD-2 + v5.1 mesh-alerts path) | Most projects do read-both compat shims | Operator preference per §0.12 "no half-shipped" |
| 41 | **Search REMOVED from Portal** | Every modern shell has search | v6.0 R5-Q lock; tags + breadcrumb replace it |
| 42 | **Single shared 16-char passcode for all mesh auth** | Most enterprises use per-user creds + ACLs | [[project_open_mesh_directive]]; flat trust is the point |

### 8.8 Possible AI-assistance pitfalls

| # | Pitfall | Mitigation |
|---|---|---|
| 43 | **Wave-skipping retirement work** | DEAD-2.x has explicit dependency waves; jumping to a later wave without earlier ones causes consumer breakage | Respect the `*(depends on …)*` tags |
| 44 | **Introducing PatternFly or Carbon tokens** | They're retired; ChromeOS Classic is live | Reference `docs/design/chromeos-classic-spec.md` |
| 45 | **Reintroducing Tailscale/Headscale/DERP/NATS references** | All axed in v2.5 + v5.1 | Legacy-mesh lint catches this |
| 46 | **Treating `mackes/workbench/` as the canonical workbench** | Being retired in favor of `crates/mde-workbench/` | New panels should target Iced tree where possible |
| 47 | **Writing helpers + tests without runtime wiring** | §0.12 forbids this; v3.x audit cost 4 user bugs | Every `pub mod foo;` needs at least one external consumer |
| 48 | **Bundling pre-staged work into a commit** | Memory [[feedback_check_pre_staged]] | `git status --short` before every commit; un-stage other peers' work |
| 49 | **Single-remote push** | Memory + §0.2: dual-remote push always | Always `git push origin main && git push mde-x main` |
| 50 | **Treating an HW-tagged item as a cut blocker** | HW carve-out is explicit | Read inline `[HW carve-out]` tags + memory [[feedback_hardware_testing_epic]] |

---

## 9. The minimum-viable AI primer (TL;DR)

If you only have time for the elevator-pitch version:

1. **MDE is a Wayland-only Rust desktop environment for small
   mesh fleets** (≤ 8 peers per Q3). Single user, many machines.
2. **Stack:** sway + mded + Iced/libcosmic + Nebula + GlusterFS +
   Netdata + zbus. RPM-only, Fedora 44+.
3. **Two design primitives:** every object is a card, every
   navigation is a breadcrumb (v6.0).
4. **Trust model:** flat — every peer fully trusts every other,
   one passcode.
5. **Sync model:** every peer holds every file (gluster); every
   peer sees every notification history row (NotificationRelay);
   notification toasts route to the attended peer (v5.1, in
   flight).
6. **Operator:** one engineer + multiple Claude sessions.
   Standing commit/push authorization.
7. **Worklist:** `docs/PROJECT_WORKLIST.md` (~15 k lines, ~45
   sections) is the canonical queue. Tasks are user-story
   shaped with bench-observable acceptance.
8. **Rules:** §0 commit rulebook in CLAUDE.md. Dual-remote push.
   §0.7 pre-commit gates. §0.8 7-gate DoD. §0.12 no stubs.
9. **Decisions:** lock everything via N-Q surveys (5/10/22/25/50
   etc.) → write `docs/design/<epic>.md` → add `<EPIC>-*`
   worklist section → ship in dependency order.
10. **Things that look weird** (Caddy, Kamailio, Ansible-pull,
    GlusterFS for `~/Documents`, mesh:// URI scheme, headless
    mode in a "DE") — see §8 above; they're either deliberate
    or in retirement.

---

## 10. Maintenance

This document should be updated when:
- A new design-system lock supersedes the current one (e.g.,
  ChromeOS Classic gets replaced)
- A new architecture pattern enters wide use
- A "norm" outlier from §8 gets resolved or normalized
- A new major epic prefix enters circulation
- The trust / sync / role model changes

Out of scope to update for:
- Routine feature work (use the worklist)
- Individual epic locks (use `docs/design/`)
- Operator preferences (use memory)
- Commit / push procedure (use CLAUDE.md)
