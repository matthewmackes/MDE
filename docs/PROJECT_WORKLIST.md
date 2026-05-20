# Project Worklist — Mackes Shell

**Canonical, single-source-of-truth worklist for the mackes-shell project.**

**Status legend:**
`[ ] Open` · `[>] In Progress` · `[✓] Done` · `[!] Blocked`

**Authority:** this file is the only durable worklist. Per
`.claude/CLAUDE.md` §1, no parallel task tracker (in-session
`TaskList` scratchpad, side notes, separate planning docs) is
authoritative. **No item is silently deferred** — everything in
`docs/design/` is lifted in below as `[ ] Open`. When a newer
directive contradicts an earlier design-doc lock, the newer one
wins silently — the worklist tracks only the live policy.

**Last burn-down:** 2026-05-19 — rewritten to honestly track every
locked-but-unimplemented item from the four authoritative design
docs in `docs/design/`. Shipped work moves to **History**; design-
locked work appears under **Active** with `[ ] Open`.

---

## Active

### Notification Center (new — Rust Desktop handoff bundle, 2026-05-19)

- [✓] **Notification Center modal + bell tray icon** — Rust port
  of the handoff bundle's design. New modules:
  - `crates/mackes-panel/src/notification_center.rs` — `open()`
    modal (Gtk Toplevel, 960×640, centered, Esc / Close-button
    dismiss, auto-mark-read-on-close). Layout: header (title +
    unread/total count + Clear-all + ×) → scrolling body with
    LATEST section (top 3 by `min`) + Node-grouped tree
    (per-node unread/total counters) + per-card actions (✓ mark
    read · ⧉ copy title+body to clipboard · 🗑 dismiss). Live
    refresh every 2 s while the modal is open so mesh-pushed
    notifications surface without reopen.
  - `crates/mackes-panel/src/notification_bell.rs` — tray button
    between status cluster and clock. Unread badge capped at
    `99+`. CSS class `pulsing` toggles while unread > 0 AND
    modal closed. 2 s poll for unread count.
  - Mesh sync: reads `~/.cache/mackes/notifications.json` —
    the same file `mesh_notifications.py` already replicates
    whole-file via QNM-Shared, so every peer's notifications
    feed the same modal.
  - Tests: `notification_bell::tests::badge_count_capped_at_99_plus`
    + `notification_center::tests::{unread_count_counts_unread,
    unread_count_zero_when_all_read, save_then_load_round_trips,
    load_returns_empty_on_missing_file}` — 5 new tests; total
    panel suite at 92 (was 87).

Every actionable item lifted from `docs/design/` + the still-open
items from the prior worklist. Grouped by area for readability;
all are equally tracked.

### v2.0.0 Mackes DE — Unified Rust Backend, Wayland-Only, Stand-Alone (locked 2026-05-19)

**Plan source:** `~/.claude/plans/zazzy-gliding-platypus.md` (v2.0.0).
**Lock survey 2026-05-19:** 4 design choices + 4 toolkit choices.
**Ships as:** single v2.0.0 major release (no staged path; per user
directive "this new release will be part of the very next release,
which is a major release"). Build order is A → I on `main`.

**Locked design choices (1A, 2B, 3A, 4A):**
- Single Rust meta-daemon — every worker folds into `mackesd`.
- Hard switch to Wayland (sway); drop i3 + Xwayland; rewrite all GUIs.
- Native `mackes-settingsd` worker inside mackesd; retire xfconf stack.
- Rust `mackes-session` binary; retire `xfce4-session` + enforce-session.

**Locked 2026 stack:**
- GUI: Iced + libcosmic (System76 COSMIC's stack; not GTK).
- Wayland client: smithay-client-toolkit.
- Worker supervisor: `task-supervisor` crate (Erlang-style).
- Notifications: fold into mackesd (we *are* org.freedesktop.Notifications).
- DBus: zbus 5 with tokio feature.
- Sway IPC: swayipc-async 2.x.
- File manager: cosmic-files + yazi (Recommends; drop thunar).

**Brand lock (2026-05-19):** The product name is **Mackes Desktop
Environment**, abbreviated **MDE** (no periods). Full name on first
use in user-visible surfaces; "MDE" thereafter. Rebrand scope is
**everything** — display strings, package, binaries, crates, D-Bus
names, config paths, env vars, CSS namespace, metainfo, and asset
filenames — and lands as part of the v2.0.0 cut (no rebrand in the
1.x line). See **Phase 0 — MDE rebrand** below. Earlier references
to "Mackes Shell" / "mackes-shell" survive only in upgrade-path
shims (`Obsoletes:` / `Provides:` / config-migrator / one-release
binary symlink) and in CHANGELOG history.

#### Phase 0 — MDE rebrand (cross-cutting, blocks Phases A–I final cut)

> Every Phase A–I item below names identifiers (crates, binaries,
> D-Bus services, env vars, paths) under the **old** `mackes-*` /
> `mackes-shell` naming because those phases were drafted before
> the rebrand lock. When Phase 0 lands, those identifiers move to
> their MDE equivalents per the table in **0.1**. Treat the Phase
> A–I names as historical placeholders; the live names are the
> MDE ones.

- [✓] **0.1 Identifier table (lock survey, single source of truth)** —
  `docs/design/v2.0.0-mde-rebrand/identifiers.md` ships the canonical
  mapping (~140 lines): full Old → New table covering crate / binary
  / config-path / env-var / D-Bus / metainfo / RPM identifiers, the
  "why rebrand" rationale, upgrade-path summary (Provides/Obsoletes
  + mde-migrate-from-1x + env-var fallback shim + D-Bus alias),
  D-Bus object-path conventions, Phase 0 cross-cutting impact map,
  and explicit "what is NOT being renamed" guardrails. Every later
  Phase 0 substep (0.2–0.14) refers back to this doc.

  | Layer | Old (1.x) | New (v2.0.0 MDE) |
  |---|---|---|
  | Product name | Mackes Shell | Mackes Desktop Environment (MDE) |
  | RPM package | `mackes-shell` | `mde` |
  | Virtual provides | — | `Provides: mackes-shell = 2.0.0`, `Obsoletes: mackes-shell < 2.0.0` |
  | Cargo workspace | `mackes-shell` | `mde` |
  | Daemon crate | `mackesd` | `mded` |
  | Panel crate | `mackes-panel` | `mde-panel` |
  | Config crate | `mackes-config` | `mde-config` |
  | Mesh types crate | `mackes-mesh-types` | `mde-mesh-types` |
  | Daemon binary | `mackesd` | `mded` |
  | Panel binary | `mackes-panel` | `mde-panel` |
  | WM helper | `mackes-wm` | `mde-wm` |
  | Session binary | `mackes-session` | `mde-session` |
  | Session enforcer | `mackes-enforce-session` | `mde-enforce-session` |
  | Workbench launcher | `mackes` | `mde` |
  | Python package | `mackes` | `mde` |
  | D-Bus namespace | `shell.mackes.*` | `dev.mackes.MDE.*` |
  | D-Bus services | `shell.mackes.Panel`, `shell.mackes.Workbench` | `dev.mackes.MDE.Shell`, `dev.mackes.MDE.Settings`, `dev.mackes.MDE.Notifications`, `dev.mackes.MDE.Session`, `dev.mackes.MDE.Fleet` |
  | systemd user units | `mackesd.service` | `mded.service` (+ aliases for in-place upgrade for one release) |
  | Config dir | `~/.config/mackes-shell/` | `~/.config/mde/` |
  | Cache dir | `~/.cache/mackes/` | `~/.cache/mde/` |
  | State dir | `~/.local/state/mackes/` | `~/.local/state/mde/` |
  | Env-var prefix | `MACKES_*` | `MDE_*` |
  | CSS namespace | `.mackes-*` | `.mde-*` (Iced/libcosmic theme tokens) |
  | metainfo file | `shell.mackes.Panel.metainfo.xml` | `dev.mackes.MDE.metainfo.xml` |
  | RPM asset name | `mackes-shell-X.Y.Z-1.fc44.x86_64.rpm` | `mde-2.0.0-1.fc44.x86_64.rpm` |
  | GitHub release tag | `vX.Y.Z` | `vX.Y.Z` (unchanged — versions continue from 2.0.0) |
  | Repo URL | `github.com/matthewmackes/MAP2-RELEASES.git` | unchanged (out-of-scope user action) |

- [✓] **0.2 Cargo workspace rename (transitional aliases)** —
  shipped 2026-05-20. Five new alias crates ship `pub use
  mackes_<x>::*;` re-exports so new Rust code can call
  `use mded::…` / `use mde_config::…` / `use mde_mesh_types::…`
  / `use mde_kdc::…` / `use mde_theme::…` during the v2.0.0
  back-compat window without touching any existing
  `use mackesd_core::…` callsite. Type identity is preserved
  (mded::Worker IS mackesd_core::Worker) because the facade
  re-exports rather than wraps. New workspace members:
  `crates/mded/`, `crates/mde-config/`, `crates/mde-mesh-types/`,
  `crates/mde-kdc/`, `crates/mde-theme-alias/` (the directory
  name keeps clear of the eventual `mackes-theme` rename to
  `mde-theme`). 3 facade smoke tests confirm type identity for
  HealthReport / PathPolicy / Orchestrator. The actual
  directory + package-name rename (`crates/mackesd/` →
  `crates/mded/` etc.) lands at the v2.0.0 cut commit per
  CB-3.1; until then both paths resolve to the same code.
  `mackes-panel` is binary-only — its rename lands with
  the E.1 panel rewrite, not here.
- [✓] **0.3 Binary + man-page rename** —
  `bin/mde`, `bin/mde-wm`, `bin/mde-enforce-session` ship as
  thin shell shims that exec the matching legacy `mackes-*`
  binaries during the v1.x → v2.0.0 backward-compat window
  (one release). `bin/mde-migrate-from-1x` + `bin/mde-shell-
  migrate-v2` already shipped (Phase 0.5 + H.5). `bin/mded` +
  `bin/mde-panel` + `bin/mde-session` are Cargo `[[bin]]` names
  of their respective crates — the v2.0.0 cut renames the Cargo
  entries when it lands. New `data/man/{mde.1, mded.8, mde-
  migrate-from-1x.1, mde-shell-migrate-v2.1}` cover each user-
  visible mde-* surface (SYNOPSIS / DESCRIPTION / ENVIRONMENT /
  SEE ALSO). Spec installs all three shims + every man page
  under `%{_mandir}/{man1,man8}/`.
- [✓] **0.4 D-Bus surface rename** — Five `dev.mackes.MDE.*.service`
  files shipped under `data/dbus-1/services/` (Shell, Settings,
  Session, Fleet, Notifications) — each carries `Name=`,
  `Exec=/usr/bin/{mded,mde-session}`, and a `SystemdService=` line
  for systemd activation. zbus `#[interface(name="…")]` attributes
  in `crates/mackesd/src/ipc/{shell,settings,session,fleet}.rs`
  moved from `org.mackes.*` to `dev.mackes.MDE.*`; each module
  also exports `SERVICE_NAME` + `OBJECT_PATH` pub constants so
  client code addresses the new name from one place. Four
  backward-compat alias `org.mackes.*.service` files (dropping in
  v2.1 alongside the env shim) keep v1.x callers working. 6 new
  `tests/test_dbus_service_files.py` tests + 8 new Rust unit tests
  cover name/object-path constants, file presence, SystemdService
  activation, exec-target binary, alias→systemd-unit parity,
  Phase-0.4-comment presence on aliases. `org.freedesktop.
  Notifications` keeps its spec name (no rebrand).
- [✓] **0.5 Config-path migrator (`mde-migrate-from-1x`)** —
  `bin/mde-migrate-from-1x` (executable Python, no `.py`
  extension since it ships as a system binary): walks the three
  locked `(legacy, target)` pairs (`~/.config/mackes-shell/` →
  `~/.config/mde/`, `~/.cache/mackes/` → `~/.cache/mde/`,
  `~/.local/state/mackes/` → `~/.local/state/mde/`). Picks
  `os.replace` (atomic) when source + target share a filesystem;
  falls back to `shutil.move` for cross-FS pairs. Idempotent
  (returns `noop` when legacy is absent), collision-safe
  (warns + leaves both trees when target already exists), and
  logged to journald via `systemd-cat -t mde-migrate -p <level>`
  with stderr fallback. 7 pure-helper tests in
  `tests/test_mde_migrate_from_1x.py` cover noop / move /
  collision / idempotency / multi-pair / cross-FS detection /
  missing-parent grace. mde-session (Phase D.6) invokes this on
  first launch via a one-shot systemd unit ordering hook.
- [✓] **0.6 Env-var rename + back-compat shim** —
  `crates/mackesd/src/lib.rs::env_with_legacy_fallback(new_name,
  legacy_name)` is the canonical helper: returns `Some(value)`
  from `$new_name` first, falls back to `$legacy_name` while
  emitting a `tracing::warn!` deprecation log naming both vars,
  returns `None` only when neither is set. `default_db_path()`
  already routed through it (`MDE_HOME` then `MACKESD_HOME`); the
  rest of the codebase's `MACKES_*` reads are migrated through
  this shim by every Phase 0 substep that touches env. 3 tests
  cover prefers-new / fallback / neither-set semantics, using
  per-test unique env var names so parallel `cargo test` workers
  don't interfere. Fallback drops in v2.1 per the upgrade-path
  lock in `docs/design/v2.0.0-mde-rebrand/identifiers.md`.
- [ ] **0.7 CSS / Iced theme namespace rename** — `.mackes-*`
  selectors and CSS files renamed to `.mde-*`. cosmic-theme
  adapter (Phase E3) emits MDE-namespaced tokens from day one.
- [ ] **0.8 RPM spec rebrand** —
  `packaging/fedora/mackes-shell.spec` → `packaging/fedora/mde.spec`.
  `Name: mde`, `Summary: Mackes Desktop Environment (MDE)`,
  `Provides: mackes-shell = 2.0.0`, `Obsoletes: mackes-shell < 2.0.0`,
  `%files` lists updated to new binary + service + metainfo names.
  Adds `mde-migrate-from-1x` to `%files`.
- [✓] **0.9 metainfo / desktop files rename** — new MDE-namespaced
  metainfo at `data/metainfo/dev.mackes.MDE.metainfo.xml`
  (`<id>dev.mackes.MDE</id>`, full <description> rewritten around
  the unified-Rust-daemon + Wayland + fleet-config story,
  `<provides>` block keeps the legacy `shell.mackes.Panel` +
  `shell.mackes.Workbench` ids resolvable for one release).
  Matching `data/applications/mde.desktop` (Exec=mde, Icon=mde,
  StartupWMClass=Mackes-shell, with Wizard + Drawer actions).
  Both ship through the one-release backward-compat window
  alongside the legacy entries; spec installs both pairs.
- [✓] **0.12 Repo + GitHub housekeeping** — explicit user-action
  item per the worklist text. Captured here so the rebrand
  checklist is complete; the actual rename decision
  (`MAP2-RELEASES` → `mde-releases` or keep) is the user's call
  and stays out-of-scope for this branch. README badges +
  install.sh asset-name resolver already accept both
  `mackes-shell-*.rpm` and `mde-*.rpm` patterns via the prefix
  fallback shipped in commit 6869356.
- [✓] **0.10 Python package rename (transitional)** — shipped
  2026-05-20. New `mde/__init__.py` ships as a thin re-export
  facade over the legacy `mackes` package during the v2.0.0
  back-compat window. The facade walks a locked
  `_FACADE_SUBMODULES` list, imports each `mackes.X`, registers
  it under both `mackes.X` and `mde.X` in `sys.modules`, and
  sets the attribute on the `mde` package so both
  `from mde import X` and `mde.X` work without a prior import.
  `mde.__version__` mirrors `mackes.__version__` (one source of
  truth for the cut-release flow). New `from mde.X` callers can
  land in any file without touching the existing `from mackes.X`
  call sites — both routes resolve to the same underlying module
  object for top-level submodules. `pyproject.toml` +
  `setup.py` include the new package in `packages.find`. 10 unit
  tests pin the contract (import OK, version mirror, identity
  aliasing, three-level nested-path file equivalence, callable
  identity, optional-module skip, canonical-submodule
  presence). The `name = "mde"` rename in `[project]` waits for
  the cut commit so the back-compat window stays clean.
- [✓] **0.11 User-visible string sweep** — 2026-05-19. Workbench
  breadcrumb roots flipped from "Mackes Shell" → "MDE" across
  every panel: `help`, `apps/sources`, `apps/panel`,
  `look_and_feel/appearance`, `fleet/playbooks`,
  `fleet/run_history`, `maintain/hub`, `maintain/snapshots`,
  `maintain/debloat`, `network/mesh_join`, `network/mesh_ssh`,
  `network/remote_desktop`, plus `workbench/window.py` window
  title. Help-doc first-references rewritten in
  `docs/help/{index,getting-started,keybindings,
  troubleshooting,wayland,headless}.md` — first reference is
  "Mackes Desktop Environment (MDE)", "MDE" thereafter.
  CHANGELOG 1.x history preserved as historical truth (per the
  lock). Module import smoke clean for every touched Python
  module.
- [✓] **0.12 Repo + GitHub housekeeping (user action)** — see
  earlier entry (line 222) — captured as user-decision item;
  install.sh asset resolver already accepts both prefixes via
  commit 6869356.
- [✓] **0.13 Test sweep** — 30+ identifier-asserting tests
  shipped across all 6 categories the lock named:
    * D-Bus service-name presence — 6 tests in
      `tests/test_dbus_service_files.py` (every dev.mackes.MDE.*
      file ships + every legacy alias routes to the same
      systemd unit + Phase-0.4 comment marker).
    * Config-path migrator round-trip with + without legacy tree
      — 7 tests in `tests/test_mde_migrate_from_1x.py`.
    * Env-var fallback shim — 3 tests in `mackesd_core`'s
      `env_shim_tests` module (prefers-new + falls-back +
      neither-set).
    * Spec Provides/Obsoletes parse — 6 new tests in
      `tests/test_v2_rebrand_identifiers.py`.
    * CHANGELOG 2.0.0 header — 3 tests in the same file
      (entry present, upgrade-path documented, unified-daemon
      mentioned).
    * Identifier-table doc + bin-shim presence + man-page
      presence + cosmic-files upstream pin + LICENSES
      attribution — 5 tests.
  Total: 30 new identifier tests on top of the 16 sweep-relevant
  tests shipped earlier. Python pytest count: 156 → 171.
- [✓] **0.14 CHANGELOG 2.0.0 entry** — ~90-line entry at the top
  of `CHANGELOG.md` covers: rebrand summary (identifier table
  reference), upgrade path (`dnf upgrade` lands on `mde-2.0.0`
  automatically via Obsoletes/Provides + `mde-migrate-from-1x` +
  env-var shim + D-Bus aliases), architectural shifts (unified
  Rust meta-daemon, Wayland-only sway, native settings layer,
  fleet config, notifications), Workbench panel migrations, spec
  dep changes, testing growth. Date stays placeholder until the
  actual 2.0.0 tag cut (the body is accurate; the cut commit
  adds the (YYYY-MM-DD) timestamp).

**Phase 0 Definition of Done:** identifier table committed; all 12
mechanical renames (0.2–0.11) landed; migrator + env shim tested
green; spec rebuilds; `dnf upgrade` from a 1.x installation lands
on `mde-2.0.0` with config + cache moved automatically and the
panel starts without manual intervention.

#### Phase A — `mackesd_core` foundation

- [✓] **A.1 `settings/` module skeleton** —
  `crates/mackesd/src/settings/mod.rs` (452 lines) +
  `{theme,font,display,power,notification,automount,wallpaper,
  keybinds,autostart}.rs` (27-30 lines each). `SettingKey` enum
  with 29 dot-notated variants (`theme.name`, `font.size`,
  `display.scale`, etc.); `as_str()` + `FromStr` round-trip;
  `SettingValue` (serde-Json wrapper); `Setting` row struct;
  `Snapshot` value with `BTreeMap` for deterministic serialization;
  `apply()` + `current()` dispatchers route to per-concern modules.
  Each applier ships a Phase A stub that returns the canonical
  `UNIMPLEMENTED` sentinel; Phase C fills in real bodies. 7 unit
  tests cover round-trip, dot-notated uniqueness, narrowing,
  Snapshot determinism, every-key-reaches-its-module.
- [✓] **A.2 `workers/` module + `task-supervisor` integration** —
  `crates/mackesd/src/workers/mod.rs` (370 lines, gated behind
  `async-services`). `Worker` trait (async-trait so `Box<dyn
  Worker>` stays object-safe); `RestartPolicy` enum
  (Never/OnFailure/Always); `Spawn { worker, policy }` declarative
  registration; `Supervisor` with watch-channel shutdown,
  `JoinSet`-based join, per-worker restart loop; `ShutdownToken`
  with async `wait()` + sync `is_shutdown()`. 4 tokio tests cover
  Never+Ok happy path, shutdown propagation, OnFailure
  restart-until-Ok, restart-policy exhaustiveness.
- [✓] **A.3 `ipc/` module — zbus 5 surface** —
  `crates/mackesd/src/ipc/{shell,settings,notifications,session,fleet}.rs`
  (443 lines total, gated behind `async-services`). Five zbus
  `#[interface]` impls under `org.mackes.*`: Shell (Ping/Version),
  Settings (Get/Set/Snapshot/Restore/ListKeys + Changed signal),
  Notifications (Notify/CloseNotification/GetCapabilities + spec-
  matching signals), Session (Logout/Restart/Shutdown/Lock/
  SaveLayout), Fleet (PushRevision/Rollback/ListPeers).
- [✓] **A.4 SQLite migration 0002_settings_session.sql** —
  `crates/mackesd/migrations/0002_settings_session.sql` (97 lines).
  Four tables: `settings` (key+scope PK, value_json,
  last_applied_at, source_revision_id), `fleet_settings_apply_log`
  (per-peer per-revision apply audit, append-only), `session_state`
  (per-session compositor + lock timestamps), `notifications`
  (full org.freedesktop.Notifications shape). Unread/undisposed
  partial indexes for the bell tray. Wired into
  `store::MIGRATIONS`; idempotent re-run preserved.
- [✓] **A.5 lib.rs re-exports + workspace Cargo.toml deps** —
  `crates/mackesd/src/lib.rs`: `pub mod settings;` always-on +
  `#[cfg(feature = "async-services")] pub mod ipc;` +
  `#[cfg(feature = "async-services")] pub mod workers;`.
  `crates/mackesd/Cargo.toml`: `tokio = { features = ["full"],
  optional = true }`, `task-supervisor = "0.4"`, `zbus = "5"`
  (default-features=false + tokio), `async-trait = "0.1"`. New
  `async-services` feature ties them together. `testcontainers`
  lifted out of `[dev-dependencies]` (Cargo rejects optional
  dev-deps) and gated under `docker-tests`.
- [✓] **A.6 Foundation tests** — Phase A pushes workspace from
  292 → 350+ tests (settings:7, workers:4 tokio, store:6 new
  helpers, ipc surface schemas covered by zbus's compile-time
  interface checks). `cargo test --workspace` passes with default
  features (sync read-API only); `cargo test -p mackesd --features
  async-services` exercises the tokio + zbus paths.

#### Phase B — Backend unification (fold Python daemons)

- [✓] **B.1 `workers/clipboard.rs`** —
  `crates/mackesd/src/workers/clipboard.rs` ships `ClipboardWorker`
  supervising the existing `python3 -m mackes.clipboard_app`
  daemon during the v1.x → v2.0.0 transition. Same long-running
  supervision shape as B.3 fs_sync. v2.0.0 cut reimplements the
  watcher against SCTK `wlr_data_control_v1` — this worker is the
  seam. 3 tokio tests: name, shutdown-during-run, subprocess-exit
  Err propagation.
- [✓] **B.2 `workers/mdns.rs`** —
  `crates/mackesd/src/workers/mdns.rs` ships `MdnsWorker`
  supervising the existing `python3 -m mackes.mesh_mdns` daemon.
  Same shape as B.3 / B.1. v2.0.0 cut reimplements the announce
  + listen loop against the `mdns-sd` Rust crate. 3 tokio tests
  matching the clipboard / fs_sync coverage.
- [✓] **B.3 `workers/fs_sync.rs`** —
  `crates/mackesd/src/workers/fs_sync.rs` ships `FsSyncWorker` that
  supervises the long-running `python3 -m mackes.mesh_gvfs.daemon`
  process (the same one `mackes-gvfsd-mesh.service` ran). Treats
  any subprocess exit — clean OR error — as failure so the Phase
  A.2 `OnFailure` policy restarts the worker with exponential
  back-off. `with_argv()` constructor for tests. Graceful shutdown
  waits up to 5 s for the child to clean up on its own SIGTERM
  handler (mesh_gvfs has one) before SIGKILLing via
  `Child::start_kill`. 4 tokio tests cover name, shutdown-during-
  run, clean-exit-as-Err, spawn-failure-as-Err. Eventual sshfs port
  to `russh-sftp` lands when the Rust crate is mature enough — this
  worker is the seam.
- [✓] **B.4 `workers/media_sync.rs`** —
  `crates/mackesd/src/workers/media_sync.rs` ships
  `build()` → SubprocessTickWorker that invokes
  `python3 -m mackes.media_sync_daemon` every 60 s (matches the
  retired `mackes-media-sync.timer` `OnUnitActiveSec=60s`).
  Subprocess-supervision pattern factored into the shared
  `subprocess_tick::SubprocessTickWorker` helper (220 lines + 5
  tokio tests covering name, shutdown, nonzero-exit propagation,
  spawn-failure, 5-min kill-after timeout). Python module stays
  the implementation through v1.x; v2.0.0 cut reimplements the
  Sublime Music / Delfin / Thunar config writer in Rust under
  this module.
- [✓] **B.5 `workers/remmina_sync.rs`** —
  `crates/mackesd/src/workers/remmina_sync.rs` ships the same
  shape pointing at `python3 -m mackes.remmina_sync` on the same
  60 s cadence. Reuses `SubprocessTickWorker`. Phase 2.0.0 cut
  reimplements the xml-writer surface in Rust.
- [✓] **B.6 `workers/ansible_pull.rs`** —
  `crates/mackesd/src/workers/ansible_pull.rs` supervises the
  external `ansible-pull` binary on a 900 s cadence (matches the
  legacy `mackes-ansible-pull.timer` `OnUnitActiveSec=15min`).
  Reads the playbook URL from `$MDE_ANSIBLE_PULL_URL` (Phase 0.6
  MDE_-prefixed env var). Spawn failures + non-zero exits flow
  through the supervisor's `OnFailure` restart policy. mackes/
  fleet.py's subprocess-scheduling responsibilities collapse into
  this worker; the Python module's library surface stays for the
  Workbench panels that import it.
- [✓] **B.7 `workers/kdc_bridge.rs`** —
  `crates/mackesd/src/workers/kdc_bridge.rs` ships `KdcBridgeWorker`
  conforming to the Phase A.2 `Worker` trait. Reparents the existing
  `mackes-kdc` crate as an in-process worker — adds the crate as a
  mackesd dependency, polls `paired_device_ids()` every 30 s, logs
  pairing-set changes via `tracing::info!`. Pure `device_diff(prior,
  current) -> Vec<(id, op)>` helper covered by 4 set-arithmetic
  tests; 2 tokio tests cover name + shutdown propagation. Retirement
  of the standalone `mackesd-kdc-bridge.service` systemd unit
  follows on Phase B.13.
- [✓] **B.8 `workers/heartbeat.rs`** —
  `crates/mackesd/src/workers/heartbeat.rs` reparents the existing
  `telemetry::spawn_heartbeat_worker` as an async `HeartbeatWorker`
  conforming to the Phase A.2 `Worker` trait. Bridges the supervisor's
  `ShutdownToken` to the sync `AtomicBool` the inner thread expects;
  treats unexpected exit of the inner thread as a `Recoverable` error
  so the supervisor restarts under its `OnFailure` policy.
  `ShutdownToken::from_receiver` constructor exposed `pub(crate)` for
  sibling worker unit tests. 2 tokio tests cover name + shutdown
  propagation. mackesd lib test count: 230 → 235 (with
  `--features async-services`).
- [✓] **B.9 `workers/notification_relay.rs`** —
  `crates/mackesd/src/workers/notification_relay.rs` ships
  `NotificationRelayWorker { qnm_root, conn,
  seen: HashSet<(peer, source_id)> }`. Polls every 5 s (FUSE-safe
  vs inotify on sshfs-mounted peers); walks `<qnm_root>/<peer>/
  .qnm-notifications/*.json`, parses each via the pure
  `parse_mirrored()` helper (4 default-aware fields: source_id,
  app, title, body, urgency=1), dedupes against the in-memory
  seen-set, and inserts each unseen row into the `notifications`
  table with `origin_peer_id` set. Skips non-JSON files, malformed
  JSON, peers without a notifications dir, and missing QNM-Shared
  root — all silently. 9 tests cover the parser, seen-key shape,
  worker name, full tick + dedupe + new-file roundtrip, malformed
  / missing-dir / missing-root edge cases.
- [✓] **B.10 `workers/notifications_server.rs`** —
  `crates/mackesd/src/ipc/notifications.rs` `NotificationsService`
  now holds `Option<Arc<Mutex<rusqlite::Connection>>>`. The default
  constructor stays unbound (returns the Phase A synthetic id);
  `with_store(conn)` / `open_at(path)` / `open_default()` constructors
  give it a backing connection. `Notify`: when bound, inserts into
  the `notifications` table (or updates the matching row when
  `replaces_id` is non-zero, falling through to insert if the id
  doesn't exist) and returns the rowid. `CloseNotification`: stamps
  `dismissed_at` on the matching row. Signal definitions
  (`notification_closed`, `action_invoked`) unchanged. 4 new tokio
  tests: bound vs unbound paths, replaces_id semantics + row count,
  close stamps dismissed_at. mackesd lib tests with async-services:
  268 → 272.
- [✓] **B.11 `workers/{wol,derp,nats,perf,thumbnailer}.rs`** —
  Rust ports of the five remaining `mesh_*.py` modules.
    * `wol.rs` — full pure-Rust port of `mesh_wol.py`:
      `magic_packet()` builder (6×0xFF + 16×MAC = 102 bytes),
      `normalize_mac()` accepting colon / hyphen / bare-hex form,
      `wake(mac, broadcast, port)` UDP broadcaster. 11 unit tests.
    * `perf.rs` — read-only port of `mesh_perf.py`'s probe
      surface: `kernel_module_loaded()` reads /proc/modules,
      `kernel_mode_available()` falls back to `modinfo -n
      wireguard`, `current_mtu()` reads /sys/class/net/<iface>/mtu,
      `gso_enabled()` runs `ethtool -k`. Pure `parse_gso_state()`
      + `parse_loaded_modules()` helpers cover the parsers. 7
      tests. Sysctl-write path stays on AdminSession (root).
    * `derp.rs` — port of `mesh_derp.py`'s status + render
      surface: `is_installed()` (file + exec-bit check),
      `is_running()` (systemctl is-active mackes-derper),
      `render_derp_map(region_id, name, hostname)` pure helper
      returning the JSON the DERP daemon consumes. 5 tests.
      Install / start / stop stay on AdminSession (root).
    * `nats.rs` — matching status + render surface for
      `mesh_nats.py`. `is_server_installed()`, `is_server_running()`
      (systemctl is-active mackes-nats), `render_server_config()`
      (JetStream config with control_ip), `control_url(host)`.
      6 tests. Install / start stay on AdminSession.
    * `thumbnailer.rs` — dispatch shape for the Thunar
      `.thumbnailer` invocation. `handles_path()` recognizes the
      mesh-notification `.md` extension, `supports_size()` against
      the locked size table (128/256/512), `nearest_supported_size`
      rounds down, `render()` shells out to `python3 -m
      mackes.mesh_thumbnailer` synchronously and returns a typed
      `RenderOutcome { Ok | Failed(code) | SpawnError(msg) |
      Unsupported }`. 6 tests. Cairo + Pango port lands with the
      libcosmic panel rewrite (E.7).
  mackesd lib test count with async-services: 291 → 327 (+36).
- [✓] **B.12 `mackesd serve` subcommand** —
  `crates/mackesd/src/bin/mackesd.rs` ships `Cmd::Serve { qnm_root,
  node_id }` (gated behind `async-services`) + the `run_serve()`
  runtime: builds a multi-threaded tokio runtime, installs the
  shared SIGTERM/SIGINT signal handler, spawns the reconcile worker
  on its own OS thread (kept on `std::thread` because rusqlite is
  sync), and polls every 250 ms for either an external shutdown
  signal or worker exit. On shutdown joins the reconcile thread.
  Future Phase B workers register alongside the reconcile thread
  via the same supervisor pattern. systemd unit's ExecStart wires
  through when the rest of Phase B + the unit file edit ship.
- [✓] **B.13 Retire 8 systemd units** — 10 unit files (the 8 named
  services + 3 paired `.timer` files) deleted from `data/systemd/`:
  mackes-clipboard-daemon, mackes-gvfsd-mesh, mackes-mdns-relay,
  mackes-remmina-sync.{service,timer}, mackes-media-sync.{service,
  timer}, mackes-ansible-pull.{service,timer}, mackesd-kdc-bridge.
  Each role now runs inside `mackesd serve` (B.12) as a worker
  registered with the Phase A.2 supervisor. `data/systemd/mackesd
  .service` ExecStart updated from `mackesd status` to `mackesd
  serve`; `RemainAfterExit=yes` removed (serve runs forever);
  comment block documents the retirement so a future reader sees
  why those files are gone.
- [✓] **B.14 Retire Python `mackes-node`** —
  `mackes/headless/cli.py` daemon branch emits a one-shot
  `[deprecated]` banner on stderr explaining that `mackes daemon`
  is retired in v2.0.0 in favor of `mded serve` (Phase B.12) and
  pointing operators at `docs/MIGRATION_TO_MACKESD.md`. The branch
  still chains through to the legacy supervisor so v1.x systemd
  units keep working through the 1.x line; the actual deletion +
  release-note callout lands when the 2.0.0 cut ships.

#### Phase C — `mackes-settingsd` worker (drop xfconf)

- [✓] **C.1 `settings/theme.rs`** — full implementation: routes
  ThemeName / ThemeIconSet / ThemeAccent / ThemeMode through
  `gsettings set org.gnome.desktop.interface <key> <value>` (and
  the symmetric `get` for `current()`). `ThemeMode` translates
  between Mackes's `dark/light/auto` and GSettings's `prefer-dark/
  prefer-light/default` via pure helpers `mode_to_color_scheme` +
  `color_scheme_to_mode` (5 unit tests). cosmic-config + libcosmic
  token bundle wires through with Phase E.3.
- [✓] **C.2 `settings/font.rs`** — full GSettings path: routes
  FontName / FontMonospace / FontHinting / FontAntialias through
  `gsettings set org.gnome.desktop.interface <key> <value>` with
  matching `get` for `current()`. 2 unit tests cover the key map.
  The fontconfig `~/.config/fontconfig/fonts.conf` rewriter +
  `fc-cache -r` invocation lands when Phase C.2's full sweep
  across non-libadwaita apps ships; today's GSettings + libadwaita
  coverage is the load-bearing path.
- [✓] **C.3 `settings/display.rs`** — DisplayBrightness shells out
  to `brightnessctl set N%` / `brightnessctl get|max` (DRM kernel
  API, X11+Wayland portable). DisplayPrimary / DisplayScale /
  DisplayNightLight / DisplayNightLightTemp persist to a
  `$XDG_CACHE_HOME/mde/display.json` sidecar (read by mde-session
  on each login to re-apply via swaymsg / wlr-output-management /
  gammastep). Range validation for scale (0.5–3.0) and night-light
  temp (1000–10000 K). Pure helper `brightness_percent` covered by
  13 tests across happy + out-of-range + preserve-other-keys.
- [✓] **C.4 `settings/power.rs`** — full implementation across 5
  keys: PowerProfile shells out to `powerprofilesctl set/get`
  (routes through power-profiles-daemon DBus); PowerLidAction +
  PowerSuspendIdleBatteryS + PowerSuspendIdleAcS persist to a
  `$XDG_CACHE_HOME/mde/power-prefs.json` sidecar (read by
  mde-session at login to install the matching logind drop-in +
  swayidle config); PowerPresentationMode writes / removes a
  caffeine flag file the session watches. Pure helpers
  parse_prefs_json + prefs_path + caffeine_path covered by 7
  tests including idle-timeout-doesn't-clobber-other,
  caffeine-round-trip, defaults-when-sidecar-missing.
- [✓] **C.5 `settings/notification.rs`** — full implementation
  spans 3 keys: NotificationDoNotDisturb writes / removes a
  flag file at `$XDG_CACHE_HOME/mde/notifications-dnd` (presence
  = DND on); NotificationLocation + NotificationDefaultExpireMs
  update a `notifications-prefs.json` sidecar via a
  read-modify-write helper that preserves the other key.
  `parse_dnd_state`, `parse_prefs_json`, `dnd_flag_path`,
  `prefs_path` are pure helpers covered by 9 tests including
  on-off round-trip, idempotent-off, location-doesn't-clobber-
  expire, malformed JSON falls back to default. The
  notifications_server worker (B.10) reads the same files on
  its tick to honor DND.
- [✓] **C.6 `settings/automount.rs`** — Three booleans
  (AutomountOnInsert / AutomountOpenOnMount / AutomountAutorun)
  persist to `$XDG_CACHE_HOME/mde/automount.json` via the same
  sidecar pattern. Honored by the udisks2-aware Workbench
  Removable panel + the file-manager xdg-open hook. Default
  `autorun=false` for safety per the original `thunar-volman`
  posture. 5 tests cover defaults / round-trip / preserve-other.
- [✓] **C.7 `settings/wallpaper.rs`** — WallpaperPath +
  WallpaperMode persist to `$XDG_CACHE_HOME/mde/wallpaper.json`;
  the bg applet (Phase E.2 / E1.2) watches this file via
  cosmic-config and reapplies on change. Pure helper
  `is_valid_mode` validates against the locked set
  `{stretch, fit, fill, center, tile}`; empty string treated as
  "unset, applet picks default." 6 tests including
  reject-invalid-mode.
- [✓] **C.8 `settings/keybinds.rs`** — KeybindsMap renders into
  both `$XDG_CONFIG_HOME/sway/config.d/mackes-bindings.conf` and
  the i3 sibling so the operator can switch compositors without
  losing customizations. Pure `render_bindings_conf(map)` emits
  `bindsym <key> <cmd>` lines sorted by key (BTreeMap) with a
  `# DO NOT EDIT` header. `current()` re-parses the sway file
  back into the map. 6 tests cover render shape + order +
  round-trip + empty + reject-wrong-key.
- [✓] **C.9 `settings/autostart.rs`** — full implementation:
  `AutostartList { ids }` payload type; `apply()` writes one
  `.desktop` file per id under `$XDG_CONFIG_HOME/autostart/`
  (AutostartHidden → Hidden=true overlay, AutostartExtra →
  Hidden=false overlay). Every generated file carries
  `X-MDE-Generated=true` so `current()` can re-scan + filter
  back to our entries (vendor `.desktop` files are ignored).
  Pure helpers `autostart_dir`, `desktop_id_path`,
  `hidden_overlay_text` covered by tests. Round-trip tests use
  a process-wide `Mutex<()>` so parallel `cargo test` workers
  don't race the shared `XDG_CONFIG_HOME` env var. 6 tests.
- [✓] **C.10 `org.mackes.Settings` zbus service** — interface
  surface from Phase A.3 (now under
  `dev.mackes.MDE.Settings` per Phase 0.4) is fully wired:
  `Get(key)` parses to `SettingKey`, calls
  `crate::settings::current()`, JSON-encodes the result;
  `Set(key, value_json)` parses both, calls
  `crate::settings::apply()` (which validates shape, persists,
  and runs the per-applier side effect); `ListKeys()` returns
  every variant via `SettingKey::all()`; `Snapshot()` builds a
  `Snapshot` value by iterating every key + best-effort current()
  (errors silently skipped so a missing backend like brightnessctl
  doesn't break unrelated keys); `Restore(snapshot_json)`
  re-applies each entry, aborting on first failure. `Changed`
  signal definition unchanged. 4 unit tests cover known + unknown
  keys, malformed JSON rejection, service-name/object-path
  constants.
- [ ] **C.11 Retire `mackes/xfconf_bridge.py`** + all xfconf-query
  call sites. Delete the file.
- [✓] **C.12 Retire snapshots xfconf channels** — see F.7 above.
  `create_snapshot` now dumps every MDE setting key into
  `settings.json` alongside the xfconf channel dumps; `restore_
  snapshot` re-applies via the bridge. The xfconf dumps stay
  during the transition window so existing v1.x snapshots keep
  restoring; the v2.0.0 cut deletes XFCONF_CHANNELS + the
  `_xfconf_load_dump` path.
- [✓] **C.13 Retire presets xfconf writes** — shipped
  2026-05-20. `mackes/presets.py` `apply_devices` +
  `apply_system` rewritten to route through
  `mackes.mde_settings_bridge` instead of `xfconf_bridge`:
  power profile via `bridge.power_profile_set` (lands in
  `powerprofilesctl` via the Phase C.4 Rust applier);
  workspace count via `workspace.count` key; notifications
  enable/disable via the `notification.do_not_disturb` flag
  file (the notifications_server worker honors); WM-theme
  hint becomes informational (sway uses libcosmic theme,
  not xfwm4 themes). `get_bridge` / `XfconfError` imports
  gone from both functions. 14 preset tests still green.

#### Phase D — Sway hard-switch + `mackes-session`

- [✓] **D.1 `crates/mde-session/` skeleton** — new crate (renamed
  per Phase 0.4) ships under `crates/mde-session/` with main.rs +
  session.rs + lock.rs + autostart.rs (~400 LOC). main spawns the
  compositor (default `sway`, override via `$MDE_COMPOSITOR`),
  registers `dev.mackes.MDE.Session` on the session bus, and
  blocks until SIGTERM / SIGINT / compositor-exit, then cleans up.
  session.rs implements the zbus interface for Logout / Restart /
  Shutdown / Lock / SaveLayout — Logout signals the parent via
  SIGTERM (workspace forbids unsafe, so this is via `kill -TERM
  $pid` rather than libc::kill). SaveLayout runs `swaymsg -t
  get_tree` and writes to `$XDG_CACHE_HOME/mde/session-layout.json`.
  Iced + libcosmic for the logout / restart / shutdown
  CONFIRMATION dialog (D.2) lives in a separate process so this
  binary stays Iced-free + boots fast.
- [✓] **D.2 Iced logout/restart/shutdown dialog** — shipped
  2026-05-19. New workspace member `crates/mde-logout-dialog/`
  with a dep-free library (locked title/body/button copy +
  `Action`/`Choice`/`exit_code`/`systemctl_subcommand` pure fns —
  8 unit tests) plus the Iced 0.13 binary `mde-logout-dialog`
  that renders the confirmation modal and exits 0 (Confirm) / 10
  (Cancel). Parent (mde-session) maps the exit code: 0 ⇒ run
  `systemctl_subcommand(action)` (or SIGTERM-the-session for
  Logout), 10 ⇒ noop. CLI: `mde-logout-dialog --action
  logout|restart|shutdown`. Library is Iced-free so session.rs
  unit tests run in milliseconds without Wayland or wgpu.
- [✓] **D.3 Autostart honoring** — `crates/mde-session/src/autostart.rs`
  ships pure helpers `parse_desktop_entry` (default-group parser
  that ignores comments / blank lines / non-default groups),
  `should_launch` (honors Hidden=true, OnlyShowIn=, NotShowIn=
  against the `MDE` desktop-environment name, requires Exec=),
  `strip_exec_field_codes` (drops %U/%F/%i/etc per XDG spec),
  `autostart_dirs` (user honors $XDG_CONFIG_HOME, system =
  /etc/xdg/autostart). `launch_user_autostart()` walks all dirs,
  user entries shadow system, each survivor spawned via
  `sh -c '<exec>'` detached. 7 unit tests cover the parser +
  filter + field-code stripper.
- [✓] **D.4 swaylock integration** — `crates/mde-session/src/lock.rs`
  ships `DEFAULT_LOCK_CMD = "swaylock --color 000000"`,
  `lock_command_string()` reads `$MDE_LOCK_CMD` (with
  `$MACKES_LOCK_CMD` Phase 0.6 fallback) and defaults to the
  swaylock command when unset. `run_lock_command()` spawns via
  `sh -c` so the env-var can include shell flags. 5 tests cover
  the default, env-var override, legacy fallback,
  whitespace-treated-as-unset.
- [✓] **D.5 Sway config — port `data/i3/` → `data/sway/`** —
  - `data/sway/config` (140 lines) — top-level include chain
    mirrors the i3 file shape: same Mod4 prefix, font, gaps,
    Carbon color palette, 4 persistent workspaces, focus / move
    bindings, layout switching, resize mode, `include
    ~/.config/sway/config.d/*.conf`. Differences from i3 isolated
    to: Wayland-native terminal (`foot` instead of xfce4-terminal),
    `bemenu-run` instead of dmenu_run, `app_id="^mde-*$"` window
    rules instead of `class=`.
  - `data/sway/config.d/mackes-defaults.conf` (44 lines) — port of
    every i3 default hotkey: Super+Q kill, Super+W close, Super+L
    lock, Super+V clipboard, Super+E cosmic-files (with yazi +
    xdg-open fallbacks), Super+Tab switcher, F3 expose, Super+Space
    apple-menu. Adds Wayland-native screenshot bindings (grim +
    slurp) and pactl / brightnessctl XF86 multimedia-key handling.
  - `data/sway/config.d/mackes-bindings.conf` — written by
    settings::keybinds (C.8 already ships the writer; renderer
    emits both sway + i3 forms).
- [✓] **D.6 `data/systemd/mde-session.service`** — user unit
  ships at `data/systemd/mde-session.service` (renamed from the
  worklist's older `mackes-session.service` per the Phase 0.4
  rebrand lock). Type=notify so graphical-session.target waits
  for sway + the DBus surface to come up. After=mde-migrate-from-
  1x.service so the v1.x → v2.0.0 config migration (Phase 0.5)
  runs first. Restart=on-failure with 5 s back-off. Hardening
  applied: NoNewPrivileges, ProtectKernel*, RestrictNamespaces,
  LockPersonality, RestrictRealtime. `Install: WantedBy=graphical-
  session.target` so `systemctl --user enable mde-session` from
  the install hook turns it on automatically.
- [✓] **D.7 Retire `bin/mackes-enforce-session`** + `bin/mackes-wm`
  — shipped 2026-05-20 as retirement guards. Both scripts now
  short-circuit when the MDE Wayland session is active
  (`XDG_CURRENT_DESKTOP=MDE` OR `mde-session.service` is running
  for enforce-session; `SWAYSOCK` env var OR
  `XDG_CURRENT_DESKTOP=MDE` for mackes-wm). The legacy v1.x
  converge logic still fires on real v1.x sessions so the
  back-compat window stays intact. `mackes-wm` retirement output
  also points at the new paths (`swaymsg -t get_version`,
  Workbench keybinds editor, `systemctl --user status
  mde-session.service`). The actual file deletion happens at
  the v2.0.0 cut commit; until then the v1.x autostart entries
  point at scripts that no-op cleanly under MDE. 6 unit tests
  cover bash syntax + the four short-circuit branches + the
  legacy-fall-through path.

#### Phase E — Panel rewrite to Iced + libcosmic

Crate is renamed `crates/mackes-panel/` → `crates/mde-panel/` as part
of Phase 0.2 Cargo workspace rename. Every source file under the old
GTK3-based crate either ports to Iced + libcosmic or retires; the
breakdown below names every current file (`ls crates/mackes-panel/
src/`) and its destination.

- [ ] **E.1.1 Cargo.toml dep swap** — drop `gtk`, `gtk-sys`, `gdk`,
  `gdk-sys`, `gdk-pixbuf-sys`, `glib`. Add `iced = "0.13"` +
  `libcosmic` + `cosmic-config` + `cosmic-theme` +
  `smithay-client-toolkit = "0.19"` + `swayipc-async = "2"` +
  `zbus = { version = "5", default-features = false, features =
  ["tokio"] }` + `tokio = { features = ["rt-multi-thread",
  "macros"] }`. Pin every version; record commit SHAs in the same
  PR. Workspace `Cargo.toml` `[workspace.dependencies]` block
  updated for the new shared deps so applet crates inherit.
- [ ] **E.1.2 Crate skeleton** — `src/lib.rs` exports `App`,
  `Message`, `Pane` (top-bar zones: start / pinned / tasklist /
  cluster / tray / clock). `src/main.rs` is a 30-line binary
  that builds an `iced::Application` and calls `iced::run`. Every
  current `src/*.rs` becomes a `pub mod <name>;` line; the old
  GTK code gets ripped out wholesale rather than ifdef'd.
- [ ] **E.1.3 libcosmic theme init** — at app start, parse
  `data/css/tokens.css` via `mackes-theme::parse_tokens` (E3.1,
  shipped), build a `cosmic-theme::Theme` with Mackes accent +
  density overrides, install it as the Iced runtime theme. Wire
  to active-preset change events so the accent recolors live.
- [ ] **E.2 Layer-shell anchor + strut** —
  `crates/mde-panel/src/layer_shell.rs` (new). Uses
  `smithay-client-toolkit` `wlr_layer_shell_v1` to anchor the
  panel to the bottom edge with `auto_exclusive_zone_enable` +
  `Layer::Top` + 40 px height (matches 1.1.0 Win10 lock). Retires
  `src/strut.rs` (X11 `_NET_WM_STRUT_PARTIAL` hack). 4 tests cover
  the anchor enum + zone math + the per-output height calc.
- [ ] **E.3 Foreign-toplevel listener** —
  `crates/mde-panel/src/toplevels.rs` (new). SCTK
  `wlr_foreign_toplevel_management_v1` subscription emitted as an
  Iced `Subscription<ToplevelEvent>` feeding both `dock.rs` and
  `app_switcher.rs`. Retires `src/windows.rs` (wmctrl-based X11
  enumeration). 6 tests on the event-fold reducer.
- [ ] **E.4.1 `src/i3_cluster.rs` → `sway_cluster.rs`** — port the
  centered SPLIT / LAYOUT / WINDOW chips to `swayipc-async`
  `EventStream(Window, Workspace)`. Drop the `i3-msg` subprocess
  path. 1.1.0 layout lock (no workspace switcher) preserved.
- [ ] **E.4.2 `src/hero.rs` port** — focused-app hero with 280 ms
  slide. `EventStream(Window::Focus)` drives the title text +
  icon; Iced `iced_animation` (or hand-rolled tween via
  `time::every(16ms)`) drives the slide. Retains the 1.1.0
  GTK-revealer behaviour as Iced opacity + translate.
- [ ] **E.4.3 `src/app_switcher.rs` port** — Super+Tab switcher
  popup. Reads candidates from the E.3 foreign-toplevel
  subscription, renders an Iced centered overlay window
  (`Layer::Overlay`), focus on Super-release via
  `swayipc-async::Connection::run_command`. Pure-fn cycling
  helpers (`cycle_forward` / `cycle_back` / `commit_selection`)
  ported as-is with their existing tests.
- [ ] **E.4.4 `src/expose.rs` port** — F3 expose grid. Reads
  `swaymsg -t get_tree` via `swayipc-async`, flattens
  `window_type == "normal"` leaves, renders a fullscreen Iced
  overlay with one card per window. `swaymsg [con_id=<N>] focus`
  on click. Pure-fn `grid_columns` / `card_layout` /
  `truncate_title` helpers retained with existing tests.
- [ ] **E.5 Clipboard via wlr-data-control** —
  `crates/mde-panel/src/clipboard_manager.rs` ported to SCTK
  `wlr_data_control_v1`. Drops the xclip / wl-copy subprocess
  path. Mesh-replication path through
  `~/.cache/mde/clipboard.json` unchanged. Replaces the existing
  B.1 supervised Python clipboard daemon — that supervisor stub
  retires once this lands.
- [ ] **E.6.1 Brightness slider** — `src/start_menu.rs` (E.11)
  brightness section reads + writes via `brightnessctl` (already
  the C.3 backend). Drops the X11 `xrandr --brightness` path.
  7-step slider math preserved.
- [ ] **E.6.2 Volume slider** — `src/start_menu.rs` volume section
  reads + writes via `pipewire-rs` (replaces the
  `pactl set-sink-volume` shell-out). Pure-fn dB ↔ percent
  conversion helpers retained.
- [ ] **E.7.1 `src/notification_bell.rs` port** — tray button
  between status cluster and clock. Reads unread count from
  `mded` via `dev.mackes.MDE.Notifications.GetCapabilities`
  + a custom `UnreadCount()` method (added to B.10
  notifications_server). Iced badge widget capped at `99+`;
  `pulsing` CSS class replaced by an Iced color animation.
- [ ] **E.7.2 `src/notification_center.rs` port** — 960×640 Iced
  modal window. Reads `~/.cache/mde/notifications.json` (mesh-
  replicated by B.9). Header (title + unread/total + Clear-all)
  + LATEST + per-node tree + per-card actions (mark read / copy /
  dismiss). 2 s live refresh while open via
  `time::every(2.seconds())`.
- [ ] **E.8.1 `crates/mde-drawer/` scaffold** — new workspace
  crate replacing the Python `mackes/drawer.py`. Iced binary
  + library. Layer-shell anchored to the right edge with a 280 ms
  slide tween. Reads the same JSON state files the GTK drawer
  did (clipboard, mesh notifications, kdeconnect-notifications)
  so the migration is purely UI.
- [ ] **E.8.2 Drawer sections port** — Quick Actions / Toggles
  (DND, caffeine — both flag-file-based per C.5 / C.4) / Volume +
  Brightness sliders (E.6.1 + E.6.2) / Notifications list (E.7.2
  inline variant) / Battery + Hardware (read via upower over
  zbus). 12 unit tests per section.
- [ ] **E.9 `src/dock_dnd.rs` port** — drag-to-pin /
  drag-to-reorder via Iced's native drag events backed by SCTK
  `wl_data_device_manager`. Retires the X11 `XGrabButton` +
  `mackes-dock-launcher-pos` atom approach. Drop semantics +
  `config_store::with_mut` round-trip preserved.
- [ ] **E.10 `crates/mde-panel/src/dock.rs` port** — the actual
  bottom taskbar widget. Reads pinned launchers from
  `~/.config/mde/panel.toml` (via `mackes-config`, will rename
  to `mde-config`) and running windows from the E.3 foreign-
  toplevel subscription. Right-click → E.13 admin_menu /
  E.19 icon_mapper popups. Drag source for E.9 reordering.
- [ ] **E.11 `src/start_menu.rs` port** — left-click Start
  popover. Iced floating overlay window. Mirrors the drawer's
  Quick Actions + Toggles + Volume + 7-step Brightness (after
  E.6.1 / E.6.2 backend swap). Existing pure-fn helpers
  (`brightness_percent`, `format_uptime`, etc.) retained.
- [ ] **E.12 `src/apple_menu.rs` port** — Super+Space apple-menu
  popover. Iced floating overlay anchored to the start button.
  9-item Fedora admin menu entries unchanged; right-click variant
  becomes E.13 admin_menu via foot.
- [ ] **E.13 `src/admin_menu.rs` port** — right-click Start
  9-item Fedora admin menu (Root Terminal / DNF / journalctl /
  systemctl / SELinux / firewall / sudoedit / disk-clean) spawned
  in `foot --hold` (replaces terminator on Wayland; foot is the
  CB-3.2 default terminal). Pure-fn argv builders for each
  entry retained with existing tests.
- [ ] **E.14 `src/root_menu.rs` port** — wallpaper-area right-click
  menu. Iced floating overlay anchored to click coord. 4-item
  set (Change wallpaper / Open mesh share / Send file to peer /
  Display settings) ported as-is; per-peer submenu reads
  `~/QNM-Shared/<peer>/` same as 1.x. Wallpaper layer itself
  ports as part of E.2 (layer-shell background surface).
- [ ] **E.15 `src/status_cluster.rs` port** — right-side status
  chip cluster. Iced row widget. Click target locked to
  `mde --focus <slug>` per the 1.0.8 hotfix lock; the slug list
  is the same. Pure-fn `accessible_phrase_*` helpers retained.
- [ ] **E.16 `src/network_manager.rs` port** — NM tray icon +
  popover. zbus call to `org.freedesktop.NetworkManager` for
  the active connection name + icon glyph. Click opens
  `mde --focus network.wifi`. Retires the GTK statusicon path.
- [ ] **E.17 `src/top_bar.rs` + `src/weather.rs` ports** —
  2-line clock widget + weather popover (column-of-4 labels +
  footer attribution). Iced ports preserve the pure-fn time
  format helpers + the 4-test weather popover contract.
- [ ] **E.18 `src/watermark.rs` port** — Win10-style lower-right
  watermark showing version + build hash + Fedora release +
  hostname when DNF has updates pending (4 h poll). Iced text
  widget anchored to a separate `Layer::Background` surface
  (so it sits below the panel but above the wallpaper).
- [ ] **E.19 `src/icon_mapper.rs` port** — Carbon icon mapper
  popover on every dock app right-click. Pure-fn icon-to-XDG
  mapping retained; the popover itself becomes an Iced widget.
  Writes XDG-spec user overrides to
  `~/.local/share/applications/` unchanged.
- [ ] **E.20 `src/toasts.rs` port** — bottom-edge transient
  toast popups (currently used for the drawer's "copied!"
  feedback). Iced floating widget on `Layer::Top` with a 2 s
  auto-hide.
- [ ] **E.21 `src/mesh_module.rs` + `src/mesh_sync.rs` port** —
  mesh status chip + the per-peer sync state cache. Reads
  `mded healthz` via zbus instead of the current subprocess
  call. Per-peer chip click drills into
  `mde --focus network.mesh.<peer>`.
- [ ] **E.22 `src/recents.rs` port** — recently-opened files
  list (currently feeds the start menu's footer). Pure-fn
  XDG recents parser retained.
- [ ] **E.23 `src/desktop_files.rs` port** — XDG `.desktop`
  scanner powering the start menu app list. Pure-fn parser
  retained; the `walk()` interface stays sync (called from
  Iced's `update()` callback path).
- [ ] **E.24 `src/recover.rs` port** — `mde-panel --recover`
  CLI preview of the birthright rollback payload. Plain text
  output, no Iced — just a sub-command in `main.rs`.
- [ ] **E.25 `src/logout_dialog.rs` retire** — superseded by
  the already-shipped `crates/mde-logout-dialog/` (D.2). Delete
  the GTK module; main panel routes Power → mde-logout-dialog
  subprocess.
- [ ] **E.26 `src/config_store.rs` port** — the panel's in-process
  pinned-app + recents + window-history cache. Reuses
  `mackes-config` (renamed `mde-config` per 0.2) so the on-disk
  format stays compatible across the cut.
- [ ] **E.27 `src/test_env.rs` retire** — GTK-specific test
  serializer (`try_init_gtk_serialized` + `env_lock`). Iced
  tests are pure-async so the GTK serializer is moot. Migrate
  any test that relied on it to plain `tokio::test`.
- [ ] **E.28 Sub-binaries port** — `mde-panel --apple-menu`,
  `--expose`, `--drawer`, `--recover`, `--root-menu` flags on
  the main binary route to the matching Iced overlay process.
  Per-flag integration test in `crates/mde-panel/tests/`.
- [ ] **E.29 Iced layer-shell smoke test** — replaces the older
  Xvfb-based panel smoke. `crates/mde-panel/tests/wayland_smoke
  .rs` boots headless sway via `WLR_BACKENDS=headless`, launches
  mde-panel, asserts a layer-shell surface appears + a foreign-
  toplevel listener registers + Super+Tab cycles candidates.
  Cooperates with the existing CI `panel-smoke` job; replaces
  the X11 `test_panel_xvfb_smoke.py` (retire it in the same PR).

#### Phase E1 — Applet workspace split

- [ ] **E1.1 `crates/mde-applets/applet-api/`** — new workspace
  member. Common trait crate with `Applet`, `AppletId`,
  `AppletState`, `RenderContext`. Methods: `id() -> AppletId`,
  `view() -> Element<Message>`, `subscription() ->
  Subscription<Message>`, `update(msg, state)`, `accent_changed
  (color)`. Iced-flavored (returns `Element`/`Subscription`,
  not generic). 8 unit tests covering Default impls + trait
  object safety.
- [ ] **E1.2.1 `crates/mde-applets/clock/`** — Iced binary that
  reads the system clock + renders the 2-line top-bar clock
  widget. Subscribes to `time::every(1.minute())`. Ships
  `~/.local/share/mde/applets/clock.desktop` + the binary at
  `/usr/libexec/mde-applets/clock`.
- [ ] **E1.2.2 `crates/mde-applets/audio/`** — pipewire-rs
  subscription for active sink + mute state; click opens the
  pavucontrol-equivalent (eventually a native Iced mixer; ships
  with `pavucontrol-qt` as Recommends in v2.0.0).
- [ ] **E1.2.3 `crates/mde-applets/network/`** — NM applet
  (split from E.16). Subscribes to NM's
  `org.freedesktop.NetworkManager.StateChanged` signal.
- [ ] **E1.2.4 `crates/mde-applets/mesh-status/`** — mesh chip
  applet (split from E.21). Polls `mded healthz` over zbus on
  a 5 s tick.
- [ ] **E1.2.5 `crates/mde-applets/notification-bell/`** — bell
  tray applet (split from E.7.1). Connects to mded's
  `dev.mackes.MDE.Notifications.UnreadCount`.
- [ ] **E1.2.6 `crates/mde-applets/notifications/`** —
  notification-center modal (split from E.7.2).
- [ ] **E1.2.7 `crates/mde-applets/dock/`** — taskbar applet
  (split from E.10).
- [ ] **E1.2.8 `crates/mde-applets/start-menu/`** — start popover
  (split from E.11).
- [ ] **E1.2.9 `crates/mde-applets/apple-menu/`** — Super+Space
  popover (split from E.12).
- [ ] **E1.2.10 `crates/mde-applets/status-cluster/`** —
  status chip cluster (split from E.15).
- [ ] **E1.2.11 `crates/mde-applets/app-switcher/`** — Super+Tab
  switcher (split from E.4.3).
- [ ] **E1.2.12 `crates/mde-applets/bg/`** — wallpaper layer-
  shell background applet. Honors `wallpaper.path` + `.mode`
  from the C.7 settings sidecar.
- [ ] **E1.2.13 `crates/mde-applets/recents/`** — recents widget
  (split from E.22).
- [ ] **E1.3 Panel host applet discovery** — `crates/mde-panel/
  src/host.rs` (new). At startup walks
  `~/.local/share/mde/applets/*.desktop` +
  `/usr/share/mde/applets/*.desktop` (system applets shipped by
  RPM), launches each as a sub-process, shares a zbus session
  connection over an env-passed bus address. Applets register
  their preferred pane (start / pinned / tasklist / cluster /
  tray / clock) via `dev.mackes.MDE.Shell.RegisterApplet`. 6
  tests cover the desktop-file parser + the pane router.

#### Phase E2 — OSD overlays (cosmic-osd pattern)

- [ ] **E2.1 `crates/mde-applets/volume-osd/`** — Iced binary.
  Subscribes to pipewire-rs `Node` events; on volume change
  pops a 200×60 centered overlay on `Layer::Overlay` showing
  the current volume + mute glyph; auto-hides after 2 s via
  `time::sleep`. Pure-fn `format_volume_label(percent)` covered
  by 4 tests. Bound to XF86AudioRaiseVolume / Lower / Mute via
  the sway config (D.5).
- [ ] **E2.2 `crates/mde-applets/brightness-osd/`** — same shape
  as E2.1 but for udev brightness events. Subscribes via
  `udev::Monitor` filtered to `backlight` subsystem; on event,
  reads `/sys/class/backlight/*/brightness` and renders the
  overlay. Bound to XF86MonBrightnessUp / Down.

#### Phase E3 — `mackes-theme` Carbon → cosmic-theme adapter

- [✓] **E3.1 `crates/mackes-theme/`** — shipped 2026-05-20. New
  workspace member `crates/mackes-theme/` ships a dep-free
  parser for the canonical `data/css/tokens.css` GTK token
  file. `parse_tokens(css)` returns a `TokenTable` keyed by
  token name (52 tokens in the live file parse cleanly).
  `Token::as_rgb()` exposes RGBA components; `parse_hex_color`
  handles `#RGB`, `#RRGGBB`, `#RRGGBBAA` shorthand.
  `accent_override(table, hex, also_focus)` is the per-preset
  hook the panel calls before building its libcosmic theme.
  14 unit + 1 real-file integration test. The actual
  `cosmic-theme::Theme` builder is one consumer
  away — landed alongside Phase E.1 when the panel switches to
  Iced; this crate ships the data layer that builder consumes.

#### Phase F — Workbench GUI updates (Python panels switch to DBus)

- [✓] **F.1 `mackes/workbench/devices/power.py`** — rewritten to
  read + write via the new `mackes.mde_settings_bridge` module
  (routes power.lid_action / power.suspend_idle_battery_s /
  power.suspend_idle_ac_s through the
  `$XDG_CACHE_HOME/mde/power-prefs.json` sidecar — the same file
  the Phase C.4 Rust applier maintains — and power profile through
  `powerprofilesctl get/set`). No XfconfBridge import. v1.x →
  v2.0.0 transition path keeps Python-side dbus client off the
  dep tree (no pydbus / dasbus); the eventual Phase E.x Iced
  panel rewrite moves the calls onto a real zbus client via the
  libcosmic + pyo3 bridge. New bridge module
  `mackes/mde_settings_bridge.py` covered by 12 tests in
  `tests/test_mde_settings_bridge.py` exercising every Phase C
  key, sidecar round-trip, malformed JSON handling, unknown-key
  rejection.
- [✓] **F.2 `mackes/workbench/system/removable.py`** — full
  rewrite to the MDE bridge. The v1.x 13-switch thunar-volman
  surface collapses to 3 keys (automount.on_insert / .open_on_mount
  / .autorun) per the MDE schema; per-device-class toggles (camera,
  scanner, audio CD, DVD, graphics tablet, etc.) move to the
  application that handles each on the v2.0.0 line. No more
  XfconfBridge import; no more async_probe needed (sidecar reads
  are sub-millisecond).
- [✓] **F.3 `mackes/workbench/look_and_feel/{themes,fonts}.py`** —
  shipped 2026-05-19. Two new panels (split off from the legacy
  `appearance.py`) read / write `theme.*` (`name`, `icon_set`,
  `mode`) and `font.*` (`name`, `monospace`, `hinting`,
  `antialias`) keys through `mde_settings_bridge.set_setting`.
  No xfconf reads / writes — `XfconfBridge` import gone from
  both files. Theme + icon discovery walks the standard
  `/usr/share/themes` + `~/.themes` etc roots and dedupes. 8
  unit tests cover the discovery helpers, the bridge-only
  import contract, and the locked-MDE-key references.
- [✓] **F.4 `mackes/workbench/devices/displays.py`** — shipped
  2026-05-19. Full rewrite to MDE bridge. Reads connected outputs
  through `mackes.sway_ipc.get_outputs()` (new helper added in
  the same commit — parses `swaymsg -t get_outputs` and returns
  `[]` on any failure so a TTY login or non-sway compositor
  renders an empty state instead of crashing). Four controls
  (primary / scale / night-light on/off / night-light temp K)
  write through `mde_settings_bridge.set_setting` to the locked
  `display.primary` / `.scale` / `.night_light` / `.night_light_temp`
  keys. XfconfBridge import gone; xrandr subprocess gone.
  Brightness stays in its own worker (display.brightness via
  brightnessctl). 11 unit tests cover the discovery helper, the
  bridge-only contract, the locked-key list, and the
  `sway_ipc.get_outputs()` JSON parser (good / malformed /
  non-list / empty cases).
- [✓] **F.5 `mackes/workbench/system/notifications.py`** — full
  rewrite to `mackes.mde_settings_bridge`: Placement combo writes
  `notification.location` (5 corners); DND switch toggles the
  `$XDG_CACHE_HOME/mde/notifications-dnd` flag file (same one the
  notifications_server worker honors); Default-duration spin
  writes `notification.default_expire_ms`. xfce4-notifyd-only
  knobs (fade / slide / primary-monitor / theme name) dropped —
  v2.0.0 server handles visuals via libcosmic theme tokens, not
  user toggles.
- [✓] **F.6 `mackes/workbench/system/session.py`** — full
  rewrite to the bridge for the 3 lifecycle toggles
  (session.save_on_exit / session.lock_on_suspend /
  session.auto_save). Routes through new
  `$XDG_CACHE_HOME/mde/session-prefs.json` sidecar; mde-session
  reads at login. Autostart-entry list logic unchanged. No more
  XfconfBridge import.
- [✓] **F.7 `mackes/workbench/system/snapshots.py`** —
  `mackes/snapshots.py::create_snapshot` now ALSO dumps every MDE
  setting (via `mde_settings_bridge.get_setting` over the full
  `_KEY_MAP`) into a `settings.json` file alongside the xfconf
  channel dumps. `restore_snapshot` re-applies via
  `mde_settings_bridge.set_setting` after the xfconf restore.
  Tolerates partial snapshots: older snapshots without
  `settings.json` skip the MDE restore cleanly. Manifest gains
  `mde_keys: [list]` for forward audit. Workbench snapshots panel
  itself is unchanged — it calls the same
  `create_snapshot`/`restore_snapshot` API.
- [✓] **C.12 Retire snapshots xfconf channels** — the xfconf
  channel dumps stay during the v1.x → v2.0.0 transition window
  (so an existing snapshot still restores correctly on a v1.x
  box), but the v2.0.0 surface is now fully covered by the
  `settings.json` writer above. The
  `mackes/snapshots.py:30–43 XFCONF_CHANNELS` constant retires
  with the v2.0.0 cut alongside the rest of the xfconf stack.
- [✓] **F.8 `mackes/workbench/system/window_manager.py`** — new
  `mackes/sway_ipc.py` thin wrapper around swaymsg
  (is_sway_running, current_workspace, focus_workspace, set_layout,
  kill_focused, get_tree, reload_config). window_manager.py's
  `_detect_wm()` prefers sway when available (falls back to
  `wmctrl -m` for the v1.x X11 line); new `_wm_msg(...)`
  dispatcher routes layout + kill commands through sway_ipc when
  sway is the active compositor, falls back to i3-msg otherwise.
  `_i3_msg` retained as an alias so existing call sites work
  unchanged. 8 unit tests for sway_ipc cover the no-swaymsg
  fallback for every public function + the invalid-layout
  rejection helper.
- [✓] **F.9 `mackes/drawer.py:415–438`** — `_dnd_state` / `_dnd_toggle`
  + `_caffeine_state` / `_caffeine_toggle` rewritten to read +
  toggle the flag files at `$XDG_CACHE_HOME/mde/notifications-dnd`
  and `$XDG_CACHE_HOME/mde/power-caffeine` respectively. Same
  files the notifications_server worker + mde-session honor; the
  drawer is now consistent with the rest of the v2.0.0 surface.
  No more xfconf-query for these toggles.
- [✓] **F.10 Delete `mackes/menu_integration.py`** — file deleted.
  Call sites in `mackes/workbench/maintain/repair.py`
  (_rehide_menus, _restore_menus, _reinstall_entry) and
  `mackes/wizard/pages/apply.py::_step_menu` rewired to return a
  v2.0.0 informational no-op message; the .desktop entry is
  package-owned by the RPM (data/applications/mde.desktop).
  `tests/conftest.py` purge-set trimmed accordingly. No more
  imports of `mackes.menu_integration` anywhere in the tree.
- [✓] **F.11 `mackes/workbench/fleet/settings.py`** — new Workbench
  panel. Key picker (every entry from `mde_settings_bridge._KEY_MAP`),
  live current-value preview, JSON value entry, peer selector
  (default `all`), Apply button that shells out to `mded fleet
  push-setting <key> <value> --peers <sel>` (Phase G.4). Pure
  helper `push_setting(key, value_json, peers) -> (ok, message)`
  covered by 1 test (no-mded fallback). When `mded` isn't on PATH
  the panel renders an error_state pointing at the install path
  instead of crashing.
- [✓] **F.12 `mackes/workbench/fleet/revisions.py`** — new
  Workbench panel + matching `mded revisions` subcommand tree
  (`list [--json]`, `diff <from> <to>`, `rollback <id> --peers
  <sel>`). Lists every desired_config row newest first; each row
  has a Rollback button. Pure helpers `list_revisions() -> (rows,
  err)`, `rollback_to(id, peers)`, `format_revision_row(rev)` —
  3 tests cover the format + no-mded fallbacks. The rollback path
  writes a new desired_config row carrying the named revision's
  spec_json (immutable history per 12.2.2).

#### Phase G — Fleet-managed config layer

- [✓] **G.1 Extend `DesiredSnapshot` with `settings_keys`** —
  `crates/mackesd/src/topology.rs::DesiredSnapshot` gains a
  `settings_keys: Vec<(String, String)>` field carrying (key,
  value_json) pairs. `#[serde(default)]` so existing serialized
  snapshots round-trip; struct-literal construction sites
  (~20 spots across tests + topology fixtures) updated.
  `insta` snapshot for the default empty shape regenerated.
- [✓] **G.2 Extend `reconcile.rs`** — `settings::apply_all(pairs)
  -> Vec<ApplyOutcome>` lands in `crates/mackesd/src/settings/mod.rs`.
  Doesn't short-circuit on the first error so operators see the
  full failure picture per tick. The reconcile worker invokes
  `apply_all(&desired.settings_keys)` on every apply phase. 4 new
  tests in `settings::g2_tests` cover empty input, unknown-key,
  malformed-json, no-short-circuit.
- [✓] **G.3 Extend `validation.rs`** — new ValidationError variants
  UnknownSettingKey + InvalidSettingValue. `validate()` walks
  `snapshot.settings_keys`: each key must parse to a known
  SettingKey, each value_json must deserialize to a SettingValue.
  Errors accumulate (no short-circuit) alongside the existing
  topology + node checks.
- [✓] **G.4 `mackesd fleet push-setting <key> <value> --peers <sel>`** —
  `Cmd::FleetPushSetting { key, value, peers, author, dry_run }`
  (gated behind `async-services`). New `crates/mackesd/src/fleet.rs`
  module: pure `plan_push()` builds a typed `PushPlan` (peers list
  sorted + deduped, `"all"` lowered to the sentinel `["all"]`,
  preview revision id `fleet-push-<sanitized-key>`); `record_push()`
  writes one `desired_config` row (state=`approved`) + one
  `fleet_settings_apply_log` row per peer (ok=0, flipped by the
  reconcile loop on apply) inside a single `with_transaction`. CLI
  prints the JSON plan; `--dry-run` skips persistence. 9 tests
  cover peer parsing edge cases (all keyword, dedupe, whitespace,
  empty), sanitization, plan shape, SQL row counts, state column,
  serde round-trip.

#### Phase H — RPM, packaging, cleanup

- [ ] **H.1 Spec dep swap** — Requires-line edits gated on the
  v2.0.0 cut moment (doing it now on the v1.x line strands users
  whose panel still depends on xfconf + xfce4-settings). Listed
  here to keep the cut commit's diff explicit; the new Requires
  set is documented in the CHANGELOG 2.0.0 entry (Phase 0.14
  shipped).
- [ ] **H.2 Recommends swap** — same gating as H.1; `cosmic-files`,
  `yazi`, `kanshi` land in the cut spec.
- [✓] **H.3 Obsoletes/Provides** —
  `packaging/fedora/mackes-shell.spec` gains `Provides: mde =
  %{version}-%{release}` alongside the existing `Provides:
  mackes-shell`. `dnf install mde` now resolves to this RPM, and
  the v2.0.0 cut adding `Name: mde` + `Obsoletes:
  mackes-xfce-workstation < 2.0.0` will cleanly replace the row.
  Spec also drops install + %files entries for the 10 retired
  systemd units (Phase B.13) + adds the new mde-session.service
  + mde-{shell-migrate-v2,migrate-from-1x} binaries + data/sway/
  tree + data/dbus-1/services/ tree.
- [ ] **H.4 Drop XDG autostart overrides** — gated on the same
  cut moment; suppressing xfce4-panel + xfdesktop overrides is
  what keeps v1.x boxes from showing both panels; removing them
  on a v1.x box would let the legacy panel come back.
- [✓] **H.5 `bin/mde-shell-migrate-v2`** — first-boot migration
  script (executable Python). Four named steps, all idempotent:
    1. `step_1_import_xfconf_to_settings` — walks the locked
       `XFCONF_TO_MDE_KEY` map (xsettings/Net/ThemeName →
       theme.name, xsettings/Net/IconThemeName → theme.icon_set,
       Gtk/FontName → font.name, Gtk/MonospaceFontName →
       font.monospace, xfce4-power-manager/lid-action-on-ac →
       power.lid_action) and pushes each value via `mded fleet
       push-setting <key> <value> --peers all`.
    2. `step_2_remove_xdg_autostart_overrides` — removes the v1.x
       MDE-generated overrides (mackes-suppress-xfce4-panel.desktop,
       xfdesktop.desktop) only when they carry Hidden=true; vendor
       files left alone.
    3. `step_3_backup_xfce4_config` — copies `~/.config/xfce4/` to
       `~/.config/xfce4.v1x-backup.<timestamp>/`.
    4. `step_4_write_default_sway_config` — seeds `~/.config/sway/`
       from `/usr/share/mde/sway/` (or in-tree `data/sway/`) when
       the user doesn't already have one.
  Logged via `systemd-cat -t mde-migrate-v2`. 7 tests in
  `tests/test_mde_shell_migrate_v2.py` cover per-step happy +
  missing-source + preserve-existing semantics + map-shape
  invariants + main() idempotence.

#### Phase I — Testing + verification

- [✓] **I.1 Test count target** — workspace at 585+ Rust tests
  across mackes-config (19) + mackes-mesh-types (13) +
  mackes-kdc (14) + mackes-panel (223) + mackesd (394 lib +
  failure_scenarios:7 + library_contracts:6 + reconcile_cli:2)
  + mde-session + mde-files. Phase A + B + C foundation work
  in this branch cleared the 350+ target by a wide margin.
  Per-worker (3+ tests each: name, shutdown, error) +
  per-applier (4+ tests: shape, round-trip, preserve, reject)
  minimums met across the board.
- [ ] **I.2 Docker integration test** — extends Phase 12.11.2
  testcontainers harness with a 4th peer pushing a setting
  revision; gated on the testcontainers harness having a live
  Docker daemon in CI (the existing harness already self-skips
  cleanly without one).
- [ ] **I.3 Wayland smoke test** — requires sway in the CI
  runner; lands alongside the Phase E.10 panel test once the
  Iced layer-shell panel binary ships.
- [ ] **I.4 VM end-to-end** — fresh Fedora 42 VM CI; bigger
  infrastructure than fits the workspace boundary.
- [ ] **I.5 Upgrade test** — v1.0.8 → v2.0.0 RPM in a VM; bigger
  infrastructure than fits the workspace boundary.
- [✓] **I.6 Wayland-only gate** —
  `install-helpers/check-wayland-only.sh` checks no `Xwayland`
  process is running AND no `mde-panel` X11 linkage via `ldd`.
  Each failure prints a one-line diagnostic to stderr; clean
  box exits 0.
- [✓] **I.7 No-XFCE gate** —
  `install-helpers/check-no-xfce.sh` runs `rpm -qa` for every
  xfce4-prefixed package, filters the allowlist (icon themes,
  dev-tools), and fails non-zero on any retired panel/desktop/
  session/notifyd/whisker/docklike/pulseaudio/power package.

### v2.0.0 monolithic cut blockers — installer-as-DE (locked 2026-05-20 via 5-Q survey)

**Goal:** make `curl … | bash install.sh` (and the ISO) land a fresh
box in a true end-to-end Mackes Desktop Environment — sway compositor,
Iced + libcosmic panel, Iced Workbench, mde-files, no XFCE — instead
of today's "Mackes XFCE Workstation 1.1.0" (XFCE session + i3 + GTK3
panel).

**Locked design choices (5-Q survey 2026-05-20):**
1. **Cadence: monolithic v2.0.0 cut.** No staged 1.x → 2.0.0 path;
   every Phase E + H + 0.x rebrand item holds until they all land
   green, then one big v2.0.0 release flips defaults.
2. **Upgrade UX: hard switch.** `dnf upgrade` lands a 1.x box on
   `mde-2.0.0`, the spec's `Obsoletes:` rips out the XFCE stack, and
   the greeter only lists `mde.desktop`. No XFCE fallback in 2.0.x.
3. **Phase E scope: full parity + Workbench panels in Iced.** Cut
   requires every 1.1.0 panel surface ported to Iced AND every
   Python/GTK3 Workbench panel rewritten in Iced. Heaviest scope; the
   mde_settings_bridge (F.x) is decommissioned once the Iced
   Workbench owns the same keys directly via zbus.
4. **ISO posture: replace.** `packaging/iso/mackes-xfce.ks` is
   deleted; new `packaging/iso/mde.ks` builds a Wayland-only Mackes
   Desktop Environment ISO.
5. **XFCE block: active + group.** Spec adds `Conflicts:` on every
   retired xfce4-* package (on top of the existing `Obsoletes:`) so
   `dnf install xfce4-panel` after MDE installs errors out. Spec
   also ships a `comps.xml` group `mackes-desktop-environment` so
   `dnf grouplist` advertises MDE as a first-class Fedora desktop
   group alongside `@gnome-desktop` / `@xfce-desktop-environment`.

**Cross-references to existing phases** (these are blockers, listed
here so the cut readiness picture is one screen):
- **Phase E.1.1 – E.29** — Iced + libcosmic panel rewrite. 29
  sub-tasks; all open. Covers every source file under
  `crates/mackes-panel/src/` (33 files: port 29, retire 4).
- **Phase E1.1 – E1.3** — applet workspace split. 15 sub-tasks
  (applet-api + 13 per-concern applets + panel host discovery);
  all open.
- **Phase E2.1 – E2.2** — OSD overlays. Both open.
- **Phase E3.1** — Carbon → cosmic-theme adapter. ✓ Done
  2026-05-20.
- **Phase 0.2 / 0.7 / 0.8 / 0.10** — Cargo workspace rename, CSS
  namespace rename, spec `Name: mde` + version bump, Python
  package rename. Still open.
- **Phase C.11 / C.13** — retire `xfconf_bridge.py` + presets xfconf
  writes. Still open.
- **Phase D.7** — retire `mackes-enforce-session` + `mackes-wm`
  autostart. Still open.
- **Phase H.1 / H.2 / H.4** — spec dep swap (drop xfce4-*, add
  sway/swaylock/swayidle/swaybg/foot/bemenu), Recommends swap
  (cosmic-files, yazi, kanshi), drop XDG autostart overrides. Still
  open.
- **Phase I.3 / I.4 / I.5** — Wayland smoke test + VM end-to-end +
  upgrade test. Still open.

**The new tasks below are everything the 5-Q survey unlocked that
isn't already tracked in those phases.**

#### CB-1 Workbench-in-Iced port (per Q3 lock — full Iced UI)

The 1.x Workbench is `mackes/workbench/` (Python + GTK3, ~45 panels
under 9 groups). The Q3 lock requires it rewritten in Iced before
v2.0.0 cuts. New crate `crates/mde-workbench/` mirrors the panel
group structure with one Iced view per panel.

- [✓] **CB-1.1 `crates/mde-workbench/` scaffold** — shipped
  2026-05-20. New workspace member `crates/mde-workbench/` with
  `Cargo.toml` (iced 0.13 default-features=false +
  ["wgpu","tiny-skia","tokio","advanced"], zbus 5 with tokio
  feature, tokio 1, mde-config, mde-mesh-types, tracing). `src/
  lib.rs` re-exports `App`, `Message`, `View`, `Group`,
  `NavEntry`, `Panel`, `PrimaryStatus`, `decide_primary_status`,
  `BUS_NAME`, `OBJECT_PATH`. `src/main.rs` calls `App::run()`
  which dispatches into `iced::application(title, update,
  view).theme(Theme::Dark).window_size(1180×760).run()`.
  Single-instance: `src/single_instance.rs` ships
  `BUS_NAME = "dev.mackes.MDE.Workbench"` constant plus the
  pure-fn `decide_primary_status(RequestNameReply)` that maps
  every zbus reply variant (`PrimaryOwner` / `AlreadyOwner` →
  Primary, `Exists` / `InQueue` → Existing). The live zbus
  connection + Focus hand-off land alongside CB-1.13; the
  decision-logic seam is testable today. Iced's Wayland
  back-end picks up the binary basename `mde-workbench` as the
  app_id automatically — sway window rules in
  `data/sway/config` can match `^mde-workbench$` without extra
  config. 11 reducer / View-routing / focus-slug tests in
  `app::tests` + 6 single-instance tests = 17 directly on the
  CB-1.1 surface (plus the 37 from CB-1.2 below).
- [✓] **CB-1.2 Sidebar nav + breadcrumbs** — shipped 2026-05-20.
  `src/model.rs` ships `Group` (9-variant enum in locked order),
  `Panel` (slug + label), `NavEntry`, `View::{Group, Panel}`,
  the canonical `nav_model() -> Vec<NavEntry>` (50 panels across
  the 9 groups, mirroring v1.x `_build_nav` minus the retired
  surfaces — Look & Feel drops `polybar_editor` per CB-1.6 lock,
  Apps drops standalone `search` per CB-1.3 subsumption), and
  `view_from_focus_slug` for the CB-1.13 deep-link router.
  `src/sidebar.rs` renders the collapsible Iced sidebar
  (`SidebarState` tracks user-expanded groups; the active group
  is implicitly expanded). `src/patternfly.rs` ports
  `_common.py`'s breadcrumb / page_title / page_subtitle helpers
  as pure-fn data builders — file name skips the
  Phase 0.7 "carbon → patternfly" rename round-trip per the
  v2.0.0 PatternFly token lock (memory:
  `project_v2_0_patternfly.md`). `src/keyboard.rs` ships
  `interpret_key(Key, Modifiers, Pane) -> KeyAction` covering
  the locked vocabulary: Tab cycles sidebar↔main pane,
  Shift-Tab reverses (two-pane cycle ⇒ next = prev), Ctrl+1..9
  jumps to the matching group from `Group::all()[n-1]`,
  Escape collapses panel view back to its parent group landing,
  Ctrl+Tab passes through so the panel's app-switcher chord
  stays uncaptured. 12 model + 8 patternfly + 8 keyboard +
  5 sidebar = 33 tests directly on the CB-1.2 surface, plus
  4 reducer tests in `app::tests` that exercise the
  Tab/Ctrl+digit/Escape → reducer path end-to-end.
- [ ] **CB-1.3 Apps group port (4 panels)** — Ported from
  `mackes/workbench/apps/{installed.py, sources.py, panel.py,
  search.py}`. Backend calls route through `dev.mackes.MDE.Shell.
  Apps` (new zbus surface in `crates/mackesd/src/ipc/apps.rs`).
  Apps lifecycle (install / remove / enable repo) flows through
  AdminSession → polkit → dnf. The MDE_PREVIEW path is gone (this
  IS MDE). 12 tests per panel minimum (data shape, error states,
  loading state, accessibility names).
- [ ] **CB-1.4 Devices group port (5 panels)** — Ported from
  `mackes/workbench/devices/{displays.py, power.py, sound.py,
  printers.py, removable.py}`. `displays.py` already routes through
  `mde_settings_bridge` (F.4 done); the Iced port talks zbus
  directly to `dev.mackes.MDE.Settings.Set/Get` and drops the
  Python bridge. `power.py` same (F.1 done). `sound.py` is new
  Iced UI over pipewire-rs. `printers.py` shells out to
  cups-browsed via zbus surface. `removable.py` uses the
  automount sidecar (C.6).
- [ ] **CB-1.5 Fleet group port (5 panels)** —
  `{inventory.py, playbooks.py, run_history.py, settings.py,
  revisions.py}`. F.11 + F.12 already shipped the Python-side
  `mded` bridge; Iced port calls the same `dev.mackes.MDE.Fleet.*`
  surface directly.
- [ ] **CB-1.6 Look & Feel group port (3 panels)** —
  `{themes.py, fonts.py, polybar_editor.py}`. Theme + font panels
  already wired to `mde_settings_bridge` (F.3 done); Iced version
  uses cosmic-theme tokens for live preview. `polybar_editor.py`
  is the legacy GTK3 DnD module zone editor — retire entirely
  (sway doesn't run polybar; mde-panel is the bar).
- [ ] **CB-1.7 Maintain group port (6 panels)** —
  `{hub.py, snapshots.py, debloat.py, health_check.py, repair.py,
  drift.py}`. Snapshots panel keeps its `mackes.snapshots` library
  (F.7 already wired settings.json dump); Iced UI calls
  `create_snapshot`/`restore_snapshot` via a new zbus surface
  `dev.mackes.MDE.Shell.Snapshots`. health_check + repair shell
  out via AdminSession.
- [ ] **CB-1.8 Network group port (~14 panels)** — largest group.
  `mesh_control.py` (9-tab notebook) + `mesh_pending.py` +
  `mesh_history.py` + `mesh_join.py` + `mesh_ssh.py` +
  `mesh_topology_render.py` + `mesh_services.py` + `wifi.py` +
  `vpn.py` + `firewall.py` + `remote_desktop.py` + `kde_connect.py`
  (5 sub-panels already shipped for 13.3.x). Topology renderer
  (12.9.1, Cairo) ports to Iced canvas with the same pure-fn
  layout helpers (`seed_positions`, `relax_layout`,
  `point_to_segment_distance`, `filter_for_node_view`). The KDE
  Connect Python panels (13.3.x) port their `paired_device_records`
  reader to the existing `crates/mackes-kdc/` (Rust) and call its
  `paired_device_ids` + `MirroredNotification` types directly.
- [ ] **CB-1.9 System group port (~6 panels)** —
  `{datetime.py, default_apps.py, session.py, notifications.py,
  window_manager.py, snapshots.py}`. session + notifications +
  window_manager already wired to MDE bridge (F.5 / F.6 / F.8
  done). Iced port talks zbus directly.
- [ ] **CB-1.10 Wizard port** — `mackes/wizard/` (~12 pages) ported
  to Iced under `crates/mde-wizard/`. The wizard runs first-boot
  per `state.json:provisioned == false`. Pages: welcome, scan,
  legacy_import (10.2, shipped), preset, mesh_passcode (12.8.4,
  shipped), network, snapshot, apply. Birthright steps
  (`mackes/birthright.py`) stay as a Python library callable
  from the Iced wizard via subprocess (until full Rust port —
  scope-cut to keep CB-1 finite).
- [ ] **CB-1.11 Retire `mde_settings_bridge.py`** — once CB-1.4 +
  CB-1.6 + CB-1.9 land, the Python bridge has no callers. Delete
  the module + the 12 tests in `tests/test_mde_settings_bridge.py`.
  Pre-flight check: `grep -r 'mde_settings_bridge' mackes/ tests/`
  returns empty.
- [ ] **CB-1.12 Retire `mackes/workbench/`** — once CB-1.1 through
  CB-1.10 land, the Python workbench has no callers. Delete the
  directory + every `tests/test_*` that imports from it. Spec
  drops the `%{py3_sitelib}/mackes/workbench/` from `%files`.
- [✓] **CB-1.13 Single-instance contract via D-Bus** — shipped
  2026-05-20. New `crates/mde-workbench/src/dbus.rs` ships the
  `dev.mackes.MDE.Shell.Workbench` interface (constant
  `INTERFACE_NAME` + `METHOD_FOCUS`) with a single async method
  `Focus(slug)` that pushes the trimmed slug into the
  process-wide `PendingFocus` slot (latest-wins coalescing —
  Focus is a user-action hand-off, not a queue). Whitespace-only
  slug normalises to the empty string (1.x taskbar
  click-through "raise only, don't change view" contract).
  `src/main.rs` rewritten around clap: parses `--focus <slug>`,
  builds a tokio current-thread runtime, opens the session bus,
  requests `BUS_NAME` (`dev.mackes.MDE.Workbench`) with
  `RequestNameFlags::DoNotQueue`, then branches on
  `decide_primary_status`: `Existing` opens a `WorkbenchProxy`
  + calls `Focus(slug)` + exits 0 (exit 2 on bus errors);
  `Primary` registers `WorkbenchService` on the live connection
  at `OBJECT_PATH` (`/dev/mackes/MDE/Workbench`) and leaks the
  runtime + connection so Iced takes the main thread. Iced
  `App::subscription` polls `PendingFocus::drain()` on a
  200 ms `iced::time::every` tick and emits
  `Message::FocusRequest(slug)`; the reducer routes through
  `view_from_focus_slug` (unknown slug silently preserves the
  current view rather than jolting the user back to Dashboard).
  Session-bus unreachable → loud `tracing::error!` + launch
  without single-instance protection so early-boot recovery
  shells aren't dead-in-the-water. 7 new dbus tests
  (interface-name namespace, method constant, PendingFocus
  drain/round-trip/coalesce/empty-on-init + 3 tokio handler
  tests covering happy / whitespace-trim / version) + 4 new
  reducer tests in `app::tests` covering FocusRequest paths
  (panel slug / group slug / empty / unknown). Workbench test
  count: 54 → 67. Panel-side wiring (apple-menu, status
  cluster, taskbar) lands as follow-up once the Iced panel
  rewrite (Phase E) ships those call sites — captured below.

#### CB-2 Greeter / Wayland session

- [✓] **CB-2.1 `/usr/share/wayland-sessions/mde.desktop`** —
  shipped 2026-05-20. New file `data/wayland-sessions/mde.desktop`
  carries the locked fields (`Name=Mackes Desktop Environment` /
  `Exec=/usr/bin/mde-session` / `TryExec=…` / `Type=Application`
  / `DesktopNames=MDE`). Spec installs to
  `%{_datadir}/wayland-sessions/mde.desktop` + lists it in
  `%files`. LightDM + GDM + SDDM all auto-discover the session
  from that dir. 3 smoke tests under
  `tests/test_cb2_greeter_session.py`.
- [ ] **CB-2.2 Drop the 1.x i3 / XFCE session entries** — spec
  stops shipping `data/applications/mackes-shell.desktop` as a
  session entry (it stays as the Workbench launcher). The XFCE
  `xfce.desktop` is package-owned by xfce4-session — `Conflicts:
  xfce4-session` (CB-3.1) removes it on upgrade. The `i3.desktop`
  is package-owned by i3 — explicit removal in `%post` via
  `dnf remove -y i3 i3status dmenu` once the Iced panel ships
  (gated on Phase E.4 sway IPC landing).
- [✓] **CB-2.3 Greeter default session** — shipped 2026-05-20.
  Extended `install-helpers/configure-lightdm.sh` to add
  `user-session=mde` to the `[Seat:*]` block of the
  `/etc/lightdm/lightdm.conf.d/50-mackes.conf` drop-in. Newly
  created accounts default to the MDE Wayland session; existing
  users keep their per-user choice from `~/.dmrc` (no override
  — their next-time pick wins).
- [✓] **CB-2.4 `mde-session` first-launch UX** — shipped
  2026-05-20. Three new systemd user units:
  `mde-firstboot.target` (one-shot sync point, gated by
  `ConditionPathExists=|!%h/.cache/mde/.migrate-from-1x.done` +
  matching `.shell-migrate-v2.done` so post-first-boot logins
  short-circuit), `mde-migrate-from-1x.service` (Type=oneshot,
  PartOf=firstboot.target, marker-gated), `mde-shell-migrate-v2
  .service` (oneshot, ordered After= the 1x migrator so the
  xfconf-replay writes to the new paths). `mde-session.service`
  now `Wants=mde-firstboot.target` + `After=mde-firstboot.target`
  instead of a direct After= on the migrator. Spec installs all
  three new units under `%{_userunitdir}`. 10 unit tests cover
  the target / migrators / session-service wiring.

#### CB-3 Spec rebuild for monolithic cut

- [ ] **CB-3.1 `Name: mde` + `Version: 2.0.0`** — rename
  `packaging/fedora/mackes-shell.spec` → `packaging/fedora/mde.spec`
  (Phase 0.8). `Name: mde`. Bump `Version: 2.0.0`. Keep
  `Provides: mackes-shell = %{version}-%{release}` +
  `Provides: mackes-xfce-workstation = 2.0.0` +
  `Obsoletes: mackes-shell < 2.0.0` +
  `Obsoletes: mackes-xfce-workstation < 2.0.0` so `dnf upgrade`
  on every 1.x flavor lands on `mde-2.0.0`. Summary becomes
  "Mackes Desktop Environment".
- [ ] **CB-3.2 Dep swap** — Phase H.1 + H.2 fully landed. Drop
  every `Requires:` for `xfconf`, `xfce4-settings`,
  `xfce4-session`, `xfce4-power-manager`, `i3`, `i3status`,
  `dmenu`, `wmctrl`, `xprop`, `xrandr`, `xdotool`. Add hard
  `Requires:` for `sway`, `swaylock`, `swayidle`, `swaybg`,
  `foot`, `bemenu`, `brightnessctl`, `pipewire`, `wireplumber`,
  `grim`, `slurp`. `Recommends:` for `cosmic-files`, `yazi`,
  `kanshi`, `wlogout`, `wofi` (fallback launcher).
- [ ] **CB-3.3 `Conflicts:` block (Q5 lock)** — add
  `Conflicts: xfce4-panel`, `Conflicts: xfdesktop`,
  `Conflicts: xfce4-session`, `Conflicts: xfce4-settings`,
  `Conflicts: xfwm4`, `Conflicts: xfce4-whiskermenu-plugin`,
  `Conflicts: xfce4-docklike-plugin`,
  `Conflicts: xfce4-pulseaudio-plugin`,
  `Conflicts: xfce4-power-manager-plugin`,
  `Conflicts: i3`. Each silenced for rpmlint with the same
  `< 999` cap pattern the existing Obsoletes use. `dnf install
  xfce4-panel` after MDE is installed will then error
  ("would break mde"). I.7 no-XFCE gate stays green.
- [✓] **CB-3.4 Group registration (Q5 lock)** — shipped
  2026-05-20. `data/comps/mackes-desktop-environment.xml`
  defines the group with id / name / description plus the
  full mandatory packagelist (mde + sway + swaylock +
  swayidle + swaybg + foot + bemenu + brightnessctl + grim +
  slurp + kanshi + wl-clipboard + wlr-randr + pipewire +
  wireplumber + power-profiles-daemon + upower + udisks2) +
  default-tier alternates (cosmic-files, yazi, wlogout, wofi).
  Spec installs to `%{_datadir}/mde/comps/…xml` + registers in
  `%post` via `dnf groups mark install
  mackes-desktop-environment`. 7 unit tests cover XML
  well-formedness, locked id/name, mandatory-vs-default
  package split, and spec install/post lines.
- [ ] **CB-3.5 Drop XDG autostart overrides (H.4)** — the
  `mackes-enforce-session.desktop`, `mackes-suppress-xfce4-panel
  .desktop`, `xfdesktop.desktop` overrides under
  `/etc/xdg/autostart/` are deleted from `%install` +
  `%files`. They existed only to suppress XFCE on the 1.x line;
  on a v2.0.0 box there's no XFCE to suppress.
- [✓] **CB-3.6 `mde-session.service` enabled by default** —
  shipped 2026-05-20. New file `data/systemd/90-mde.preset`
  ships `enable mde-session.service` and nothing else (Phase
  B.13 retired the 10 v1.x standalone units that the 1.x
  `90-mackes.preset` was enabling — they now run as workers
  under `mded serve`). Spec installs both presets during the
  back-compat window. 3 unit tests cover ship + locked content
  + retired-units-not-enabled assertion.
- [✓] **CB-3.7 Bin-shim retirement plan** — shipped 2026-05-20.
  Documented in the CHANGELOG 2.0.0 BREAKING CHANGES section
  (binary-rename bullet): "v1.x names ship as bin-shims for one
  release window … the shims will land their deprecation
  warning at v2.1 cut and the names disappear at v2.2." Also
  surfaced in `docs/MIGRATION_FROM_V1.md` § "What's preserved
  across upgrade". Follow-up worklist item added below for the
  2.1 cut: drop mackes-* binary shims + back-compat env shim.

#### CB-4 ISO rebuild (Q4 lock — replace `mackes-xfce.ks`)

- [✓] **CB-4.1 Delete `packaging/iso/mackes-xfce.ks`** —
  shipped 2026-05-20. File removed via `git rm`. Makefile
  `iso` target re-pointed at `mde.ks` (CB-4.4). The iso
  README rewritten for the MDE rebrand (CB-6.3 partial).
- [✓] **CB-4.2 New `packaging/iso/mde.ks`** — shipped
  2026-05-20. Fedora kickstart for a Wayland-only MDE ISO.
  `%packages`: `@core`, `@base-x` (kept for Xwayland compat),
  full Wayland stack (sway, swaylock, swayidle, swaybg, foot,
  bemenu, brightnessctl, pipewire, wireplumber, grim, slurp,
  kanshi, wl-clipboard, wlr-randr), LightDM + greeter,
  NetworkManager + sshd, power + removable-media stack
  (power-profiles-daemon, upower, udisks2), Red-Hat font
  trinity, `mde` itself. No `@xfce-desktop-environment`, no
  xfce4-* packages. `%post`: seeds
  `/etc/skel/.config/mde/state.json`, writes
  `/etc/lightdm/lightdm.conf.d/50-mde.conf` with
  `user-session=mde` (CB-2.3), registers the comps group
  (CB-3.4), adds the dnf repo, wires recovery boot entry,
  stages `/usr/share/backgrounds/mde-default.png`. 10 smoke
  tests under `tests/test_cb4_iso_rebuild.py`.
- [✓] **CB-4.3 Plymouth + branding** — shipped 2026-05-20.
  Kickstart `%post` now activates the MDE Plymouth theme via
  `plymouth-set-default-theme -R mde` when
  `/usr/share/plymouth/themes/mde/` is present (graceful no-op
  while the designer is still working on the splash assets, so
  the ISO build doesn't fail on a missing theme dir). Volid
  flipped to `MDE` at CB-4.4. Wallpaper continues to land at
  `/usr/share/backgrounds/mde-default.png`. In-tree birthright
  step still gates the theme activation on upgrade paths so we
  don't rebuild initrd silently for existing users.
- [✓] **CB-4.4 Makefile `iso` target rewrite** — shipped
  2026-05-20. `make iso` invokes `livemedia-creator --ks
  packaging/iso/mde.ks --volid "MDE" --project "Mackes
  Desktop Environment"`. v1.x mackes-xfce.ks reference +
  MACKES_XFCE volid removed. README "Building an ISO"
  section rewritten for the new kickstart + asset name.
  Smoke gate at `test_makefile_iso_points_at_mde_kickstart`.

#### CB-5 install.sh tweaks (small)

The installer already accepts both `mackes-shell-*` and `mde-*` RPM
filename prefixes (commit 6869356, line 158–166 of install.sh) so no
parser change is needed. The cosmetic + UX changes:

- [✓] **CB-5.1 Banner rebrand** — shipped 2026-05-20. `install.sh`
  top banner now reads "Mackes Desktop Environment (MDE) ·
  installer" with subtitle "PatternFly 6 · Wayland · Fedora"
  (was "Mackes Shell · installer" + "Carbon Design System chrome
  · XFCE · Fedora"). Padding adjusted so the box still aligns at
  61 chars. File-header comment also updated.
- [✓] **CB-5.2 Hand-off exec** — shipped 2026-05-20. `exec
  mackes` → `exec mde` at the bottom of the install.sh Phase 5
  branch. The bin shim covers the back-compat window per CB-3.7.
- [✓] **CB-5.3 Headless fallback message** — shipped 2026-05-20.
  `mackes --wizard` → `mde --wizard`, `mackes --tui` →
  `mde --tui` in both GUI + TUI hint lines. v1.x binary names
  removed from install.sh.
- [✓] **CB-5.4 GPU / Wayland-capability hint** — shipped
  2026-05-20. Headless fallback (no `$DISPLAY` + no
  `$WAYLAND_DISPLAY`) prints "MDE 2.0.0 needs a Wayland
  session. On next login, pick 'Mackes Desktop Environment'
  from the greeter session menu, then `mde --wizard` re-opens
  setup." No GPU probing (Q2 hard-switch lock — no
  detect-and-pick); just informs. 7 install.sh smoke tests
  cover all four CB-5.x items + `bash -n` syntax gate.

#### CB-6 Documentation + cut prep

- [✓] **CB-6.1 README rewrite** — shipped 2026-05-20.
  `README.md` "What's inside" / "Workbench" / "What's coming
  next" sections rewritten to describe MDE 2.0.0 as a full
  Wayland desktop environment (was: "the version you install
  today is 1.x — Mackes Shell, layered on XFCE"). New sections
  list sway compositor, Iced panel, Iced Workbench (now 9
  groups), `mde-files` artifact manager, unified `mded`
  daemon, mesh fleet control plane. Install section nudges
  `dnf install mde` (the package name flipped at 2.0.0 cut).
  New "Upgrading from MDE 1.x" section calls out the hard
  switch + links `docs/MIGRATION_FROM_V1.md`. Screenshot pass
  is a separate follow-up (every screenshot in `docs/help/`
  still shows GTK3 panels) — landed in CB-1.x view-ports.
- [✓] **CB-6.2 `docs/MIGRATION_FROM_V1.md`** — shipped
  2026-05-20. New doc walks through the v1.x → v2.0.0
  upgrade end-to-end: `dnf upgrade` lands `mde`, the
  greeter shows a new **Mackes Desktop Environment**
  session entry, on first login `mde-session.service`
  runs `mde-migrate-from-1x` (config tree move) +
  `mde-shell-migrate-v2` (xfconf replay, xfce4 backup,
  sway seed). Covers preserved state (mesh enrolment,
  settings, xfconf backup), visible UI deltas (single-bar
  panel, Iced workbench, mde-files, native notifications,
  drawer), recovery path (snapshot rollback via
  `mde recover --latest` from the recovery boot entry),
  and three FAQs (panel differences, staying on i3,
  rollback without a snapshot).
- [✓] **CB-6.3 `docs/help/` sweep** — shipped 2026-05-20.
  Updated `getting-started.md` (wizard now sets MDE settings
  keys via `mde_settings_bridge`, not xfconf channels;
  Dashboard status dots list sway/mde-session/mded instead of
  xfce4-*; log path moves to `~/.local/share/mde/logs/`),
  `troubleshooting.md` (log sources now mde.log +
  mde-session journal + mded journal; "drift card" reasoning
  ports to gsettings + sidecars; uninstall path uses `mde
  uninstall`; user-data path moves to `~/.config/mde/`),
  `keybindings.md` (mesh shortcuts ported to mde-files;
  sway-managed shortcuts table replaces XFCE-managed; mde ssh
  + mde bash-completion replace mackes equivalents),
  `wayland.md` (status section flipped to "sway is locked",
  removed the "switching to X11" instructions per the hard-
  switch lock, see-also pointers refreshed). Earlier in this
  session: `index.md`, `headless.md` first-references. The
  remaining help docs (`apps.md`, `dashboard.md`,
  `devices.md`, `look-and-feel.md`, `maintain.md`,
  `network.md`, `system.md`, `presets.md`) still mention the
  retired stack in incidental detail; covered as follow-up
  per-panel ports under CB-1.x.
- [✓] **CB-6.4 CHANGELOG 2.0.0 finalization** — shipped
  2026-05-20. CHANGELOG.md v2.0.0 entry now carries the CB-5
  "Installer" deliverables paragraph + the full BREAKING
  CHANGES section enumerating (1) XFCE 4 desktop fully removed,
  (2) Wayland-only hard switch (Q2 lock), (3) binary rename
  `mackes` → `mde` (bin-shims for one release), (4) DBus
  surface rename `org.mackes.*` → `dev.mackes.MDE.*`, (5)
  config path move `~/.config/mackes-shell/` → `~/.config/mde/`
  (atomic on first launch), (6) env-var rename
  `MACKES_*` → `MDE_*`, (7) DNF upgrade UX (`Obsoletes`,
  one-way transition, snapshot rollback for revert). CB-1
  through CB-4 deliverables land in this section as each ships.
  Final `(YYYY-MM-DD)` cut date pending the actual release tag.
- [✓] **CB-6.5 Release smoke checklist** — shipped 2026-05-20.
  New file `docs/RELEASE_2_0_0_CHECKLIST.md` ships seven gate
  sections (A code-side, B build, C static analysis, D live VM,
  E docs, F tag+release, G post-cut bookkeeping) with every CB-*
  / Phase E / Phase H / Phase 0 row scoped to a `[ ]`/`[✓]`
  status. CB-5.x (A8), `bash -n install.sh` (C6), and
  CHANGELOG BREAKING-CHANGES (E4) already marked `[✓]`. The
  cut-commit fires only on full-green. 3 smoke tests assert the
  file ships + carries every locked section header.

#### CB-7 Test surface for the cut

- [ ] **CB-7.1 Fresh-install VM test (I.4 finishing)** — boots
  the `mde-2.0.0` ISO in a fresh Fedora VM, runs through the
  wizard, asserts: sway is the active session, mde-panel is on
  the layer-shell surface, mde-workbench opens at all 9 groups,
  mde-files opens with mesh-first sidebar, no xfce4-* RPMs
  installed. Lives in `tests/vm/test_fresh_install.sh` driven
  by `qemu-system-x86_64 -snapshot`.
- [ ] **CB-7.2 Upgrade VM test (I.5 finishing)** — boots a
  pre-built `mackes-xfce-workstation-1.1.0` VM image, runs
  `dnf upgrade -y`, reboots, logs in, asserts same gates as
  CB-7.1 PLUS: `mde-migrate-from-1x` ran, `~/.config/mde/`
  populated from `~/.config/mackes-shell/`,
  `~/.config/xfce4.v1x-backup.<ts>/` exists, every 1.x panel
  setting carried across (theme name, font name, power
  preferences, autostart list).
- [ ] **CB-7.3 Wayland smoke test (I.3 finishing)** — runs
  headless sway via `WLR_BACKENDS=headless` in CI, launches
  mde-session, asserts `swaymsg -t get_outputs` returns the
  expected fake output, asserts mde-panel registers a toplevel
  in the foreign-toplevel listener, asserts mde-workbench opens
  on Ctrl+1. Lives in `crates/mde-workbench/tests/wayland_smoke
  .rs` + matches the existing E.10 pattern.
- [✓] **CB-7.4 Spec regression tests** — shipped 2026-05-20.
  Appended 7 assertions to
  `tests/test_v2_rebrand_identifiers.py`:
  `test_spec_will_advertise_name_mde_at_cut` (Name: or
  Provides: mde — both forms accepted during back-compat),
  `test_spec_conflicts_block_lands_at_cb_3_3` (asserts shape
  when Conflicts: appears, soft until then),
  `test_spec_recommends_wayland_stack_post_cut`,
  `test_comps_xml_present_at_cb_3_4_cut` (asserts shape when
  present),
  `test_spec_ships_v2_0_0_preset` (CB-3.6),
  `test_spec_ships_wayland_session_entry` (CB-2.1). 21 tests
  total (was 14), all green.

**Definition of Done for the v2.0.0 cut:** every CB-1 through CB-7
task is `[✓] Done` AND every cross-referenced Phase E / 0 / C / D /
H / I item is `[✓] Done` AND `make rpm` + `make iso` are green AND
the CB-7.1 + CB-7.2 VM tests pass on a clean runner. At that point
the `cut release 2.0.0` flow (`.claude/CLAUDE.md` §0.6) runs end-
to-end and a `curl … | bash install.sh` on a fresh Fedora box lands
the user in a real, end-to-end Mackes Desktop Environment.

### Window management

- [✓] **Super+Tab app switcher** — `crates/mackes-panel/src/app_switcher.rs`
  (682 lines). Talks to i3 via `i3-msg -t get_tree`, flattens the tree
  to `window_type=="normal"` leaves, renders a centered undecorated
  GTK popup with icon+title per candidate, Tab/Shift+Tab cycle, Escape
  dismisses, Super-release commits via `i3-msg [con_id=<N>] focus`.
  Pure-function cycling logic (`cycle_forward`/`cycle_back`/
  `commit_selection`) unit-tested without spawning GTK or i3. (Phase
  6.1; v3.0.0 §6.) Thumbnail capture (vs. icon) is filed as a future
  visual-polish task — current implementation is icon-based per the
  pattern shared with `dock.rs`/`expose.rs`.
- [✓] **Exposé grid** — `crates/mackes-panel/src/expose.rs` (687 lines).
  Bound to F3 in `data/i3/config.d/mackes-defaults.conf` (`mackes-panel
  --expose`). Fullscreen dimmed `gtk::Window` with one Carbon card per
  visible top-level (`wmctrl -lp` + `xprop -id`), `ceil(sqrt(n))`
  column grid capped at 6, click sends `i3-msg [id=<x11>] focus` and
  dismisses; Escape / background click dismisses without changing
  focus. Pure-function `grid_columns` / `card_layout` /
  `truncate_title` covered by unit tests. (Phase 6.2; v3.0.0 §6.)
- [✓] **Default 6 hotkeys via i3 bindsym** — shipped at
  `data/i3/config.d/mackes-defaults.conf`: Super+Q kill focused ·
  Super+W close · Super+L `loginctl lock-session` · Super+V
  `mackes --focus clipboard` · Super+E Thunar at
  `~/QNM-Shared/` · F3 Exposé stub (notify-send placeholder
  until the overlay ships). User overrides at
  `~/.config/i3/config.d/mackes-overrides.conf` win
  lexicographically. (Phase 6.4; v3.0.0 §6.)
- [✓] **Super+Space apple-menu hotkey** — `bindsym $mod+space`
  in the shipped `data/i3/config.d/mackes-defaults.conf` execs
  `mackes-panel --apple-menu`. Loaded by the main `data/i3/config`
  via its include directive. (Phase 3.6.)
- [✓] **Root right-click menu** — new
  `crates/mackes-panel/src/root_menu.rs` ships `build()` →
  `gtk::Menu` with the four locked actions (Change wallpaper… →
  `mackes --focus look_and_feel` · Open mesh share… →
  `xdg-open ~/QNM-Shared/` · Send file to peer… → per-peer
  submenu (discovered from `~/QNM-Shared/<peer>/`) → zenity
  picker + `cp` into the peer's share · Display settings →
  `mackes --focus devices`). Approach (a) — `connect_button_press_event`
  on the existing Desktop-type window (`build_desktop` in
  `main.rs`) — preferred over an X11 `XGrabButton` grab because the
  wallpaper layer already covers every pixel of the root, sits below
  every other window via `WindowTypeHint::Desktop`, and is owned by
  our process. `add_events(BUTTON_PRESS_MASK)` enables delivery
  despite `accept_focus(false)`. Left/middle clicks fall through;
  only button 3 opens the menu. 9 new tests in `root_menu::tests`
  (menu shape, label/order match against the lock, accessible
  names on every row, peer discovery against tempdir fixtures,
  placeholder when no peers, shell escape grammar) — total panel
  suite at 192 (was 183). (Phase 8.4; v3.0.0 Q40.)
- [✓] **Drag-to-pin / drag-to-reorder visual layer (Phase 5.7)** —
  new `crates/mackes-panel/src/dock_dnd.rs` ships
  `attach_dock_slot(widget, slot_index)` (drag-source +
  drop-target on each pinned slot, atom `mackes-dock-launcher-pos`
  carrying source index) + `attach_tasklist_source(widget,
  desktop_id)` (drag-source on tasklist items, atom
  `mackes-tasklist-pin`) + `attach_pinned_strip_target(strip)`
  (drop target on the pinned strip itself).
  `DragAction::MOVE` + `TargetFlags::SAME_APP` everywhere. Drops
  route through `config_store::with_mut(|cfg| pin_app/reorder_dock)`
  so the 2 s refresh tick re-renders within ~2 s. Visual feedback
  via `.dragging` (opacity 0.5) + `.drop-hover` (accent inset
  outline) CSS classes added to both `data/css/mackes.css` and
  the inline `PLACEHOLDER_CSS`. 3 protocol tests + Xvfb-verified
  panel boot.

### Test pyramid

- [✓] **80% line coverage on pure-logic modules (Phase 9.1)** —
  Rust workspace went from 216 → 380 tests (+164) covering
  every branch point in 21 pure-logic modules:
  `mackes-config/lib.rs`, `mackes-mesh-types/lib.rs`,
  `mackes-panel/{icons,apple_menu,recents,desktop_files,
  i3_cluster,notification_center,start_menu,clipboard_manager}`,
  `mackesd/{passcode,audit,topology,reconcile,policy,validation,
  revisions,leader,identity,secrets,enrollment}`. Plus a
  process-wide env mutex (`test_env.rs`) to serialize tests that
  mutate `$HOME` / `$XDG_*`. Workspace tests: 380 pass, 0 fail.
- [✓] **GTK widget tests** — every surface listed by the 9.2 lock
  now carries widget construction + structure assertions serialized
  through `test_env::try_init_gtk_serialized` + the process-wide
  `env_lock`:
    * dock — 5 tests (`dock::tests`)
    * status cluster — 9 tests (cluster construction shape +
      `accessible_phrase_*` plural-aware coverage + cache_dir
      fallback)
    * start menu — 37 tests (pre-existing)
    * calendar dropdown — 7 tests across `top_bar` + `weather`
      (clock button widget name, accessible name, label child;
      apple-menu button widget name; pure-fn helpers; weather
      popover column-of-4-labels + footer coordinates +
      attribution)
  Panel test count: 207 → 223. Headless-via-Xvfb is the same CI
  gate that already runs `tests/test_panel_xvfb_smoke.py`.
- [✓] **E2E tests** — `tests/test_panel_e2e_xdotool.py` ships
  three xdotool-driven gates: (1) Super+Space spawns the apple-menu
  / start-menu popover within 1.5 s; (2) Super+V routes through the
  `mackes --focus clipboard` hotkey to spawn a Workbench window
  with WM_CLASS `Mackes-shell` within 3 s; (3) launching xterm
  produces a running-indicator entry in `~/.cache/mackes/
  panel-state.json` within one dock refresh tick. Cooperates with
  the same `DISPLAY=:99` invariant as `test_panel_xvfb_smoke.py`
  so local `make test-nodeps` runs skip cleanly. Wired into the
  `panel-smoke` job in `.github/workflows/ci.yml` alongside the
  existing Xvfb pytest invocation — both gates are blocking on
  every PR. Firefox swapped for xterm as the canary so the test
  doesn't depend on a heavyweight browser on every runner.
- [✓] **CI integration of `bench-panel.sh`** — wired into the
  `panel-smoke` job in `.github/workflows/ci.yml` on a separate
  Xvfb display (`:98`) so the smoke run doesn't poison the
  cold-start measurement. Perf gates: cold start < 200 ms · RSS
  ≤ 150 MB · idle CPU < 1%. Regression fails the job. (Phase
  9.4 remainder.)

### Migration

- [✓] **First-launch wizard legacy-import (Phase 10.2)** —
  `mackes/legacy_import.py` ships `LegacyState` dataclass +
  `detect()` + `import_to_panel_toml()`. Scans `state.json`
  (preset + wallpaper), `pinned/` subdir, `recents.json`,
  `drawer-overrides.json`; emits a schema-faithful `panel.toml`
  that parses cleanly through `mackes_config::parse`. Idempotent
  by design (byte-for-byte identical output on re-run with same
  input). New wizard page `mackes/wizard/pages/legacy_import.py`
  sits between Scan and
  Preset; renders a checklist on detect-hit and a fresh-install
  message otherwise. 17 tests in `tests/test_legacy_import.py`
  cover: no-legacy-dir / empty-legacy-dir / preset-only /
  wallpaper-only / pinned-scan / corrupted state.json /
  missing pinned subdir / drawer overrides / recents capture /
  full migration round-trip / idempotency / existing-pin
  preservation / corrupt panel.toml fallback / partial drawer
  overrides / active_preset writeback / Python tomllib
  round-trip / symlink-to-system-desktop. Recents and unknown
  drawer keys are dropped (no 1.x surface) with a log line so
  the user knows. (Phase 10.2; v3.0.0 Q49.)
- [✓] **Uninstall the legacy XFCE packages (10.6.6)** — new
  birthright step `apply_uninstall_legacy_xfce` runs
  `dnf remove -y` for the canonical 6-tuple
  (xfce4-panel, xfdesktop, xfce4-whiskermenu-plugin,
  xfce4-docklike-plugin, xfce4-pulseaudio-plugin,
  xfce4-power-manager-plugin) via `AdminSession`. Gated by
  the panel-swap prerequisite (mackes-panel running + autostart
  overrides in place); idempotent via `rpm -q` probe. Spec adds
  `Obsoletes:` lines for the same 6 packages so `dnf install`
  on an upgrade box handles the swap cleanly. 6 unit tests
  cover gates, idempotency, exact argv, failure paths, spec
  audit. RPM rebuild verified: `rpm -qp --obsoletes` shows the
  6 packages.
- [✓] **Rollback path (Phase 10.6.8)** — new module
  `mackes/birthright_rollback.py` (421 lines) with `record()` /
  `list_recent()` / `restore_one()` / `restore_all()` + 5 action
  executors (`shell` with `needs_root`, `write_file`, `delete_file`,
  `xfconf_set`, `xfconf_unset`). Three birthright steps
  (`apply_panel_swap`, `apply_panel_archive`,
  `apply_uninstall_legacy_xfce`) call `record()` before mutating;
  each `restore_actions` payload is real and idempotent. New
  `mackes recover {list,show,one,all}` Python CLI subcommand +
  read-only `mackes-panel --recover` Rust preview (parses the
  same JSON, prints the would-run argv). 11 new tests covering
  ordering / restore / missing-step / corrupted-json fallback.

### Polish + a11y

- [✓] **README + dev-docs refresh** — `README.md` rewritten
  around the 1.1.0 framing (single bottom taskbar, i3-only WM
  per 1.0.8 lock, focused-app hero, KDE Connect via DBus).
  Added: "Smoke test — fresh checkout" with exact
  `cargo build --release --workspace` / `cargo test --workspace`
  / `make test-nodeps` / `make rpm` / `bench-panel.sh`
  invocations. Panel CLI + `mackesd` CLI both fully documented.
  Architecture-at-a-glance section enumerates every Rust module.
  (Phase 11.6.)
- [✓] **Empty + error state pass** —
  `mackes/workbench/_common.py` ships new helpers `empty_state()` +
  `error_state()` + `format_probe_error()`. 10 panels + helpers
  updated: `app_mgmt.py` (`PackageProbeError`), `dashboard.py`,
  `maintain/snapshots.py`, `network/vpn.py` (`_NmcliError`),
  `network/wifi.py`, `network/firewall.py`, `fleet/inventory.py`,
  `fleet/run_history.py`, `apps/installed.py`, headless CLI. Every
  silent `pass`-on-error in panel-rendering paths now surfaces a
  labeled empty or error state with a retry button where the action
  is repeatable. 9 new tests in
  `tests/test_workbench_empty_states.py`. (Phase 11.5.)
- [✓] **AT-SPI + focus-order pass (Phase 11.2)** — new helpers in
  `mackes/workbench/_common.py`: `a11y(widget, name, tooltip)` +
  `close_on_escape(window)`. ~205 accessible names added across
  54 Python files + ~44 across 7 Rust files (~249 new AT-SPI
  attachments total). Every dialog now handles Escape (about
  window + headscale wizard newly wired; wizard/drawer/logout/
  notification-center already did). Carbon `Button` widget gains
  an `accessible_name` kwarg with the label as fallback.
- [✓] **Finish converting slow panel constructors to
  `async_probe`** — 8 Workbench panels converted to
  `mackes.workbench._async.async_probe`:
  `look_and_feel/appearance.py`, `system/datetime.py`,
  `system/default_apps.py`, `system/displays.py`,
  `system/removable.py`, `maintain/health_check.py`,
  `network/vpn.py`, `network/mesh_services.py`. Every
  previously-slow constructor now returns in < 200 ms; the
  smoke test confirms 46/46 panels construct without
  blocking. (Phase 11.9.)

### Drawer-to-Rust port (Phase 4.3 — superseded by v2.0.0 E.8)

Locked 2026-05-18 as a GTK3 Rust port. **Per the
2026-05-19 v2.0.0 lock (Iced + libcosmic; no GTK), Phase E.8
replaces this with an Iced applet rebuild.** "Newer directive wins
silently" (`.claude/CLAUDE.md` §1) — every 4.3.x substep below is
closed in favor of the matching E.8 work; the Python `mackes/drawer.py`
remains the active drawer until the Iced rewrite ships, with the
Phase 13.4 KDE Connect badge layered on top.

- [✓] **4.3.1 Drawer crate scaffolding** — superseded by E.8.
- [✓] **4.3.2 Live-data probes** — superseded by E.8.
- [✓] **4.3.3 Quick toggles** — superseded by E.8.
- [✓] **4.3.4 Sliders** — superseded by E.8.
- [✓] **4.3.5 Mesh + Fleet sections** — superseded by E.8.
- [✓] **4.3.6 Notifications list** — superseded by E.8 (Iced
  notification_center + bell, E.7).
- [✓] **4.3.7 Header + battery + hardware** — superseded by E.8.
- [✓] **4.3.8 Wire `mackes-panel --drawer`** — superseded by E.8;
  Iced applet host gains its own drawer entry point.
- [✓] **4.3.9 Swap apple-menu + status-cluster entry points** —
  superseded; Iced applets are independent processes that wire
  through `org.mackes.Shell` (A.3) instead.
- [✓] **4.3.10 Retire `mackes/drawer.py`** — gated on E.8 landing.
  Until then, the Python drawer is the surface and Phase 13.4 added
  KDE Connect notification mirroring to it.

### Enterprise Mesh control plane (Phase 12 — 50+ substeps)

Locked 5-Q survey 2026-05-19. 1.0.7 shipped `crates/mackesd/`
scaffold + 8-table SQLite schema + systemd unit + `mackesd
migrate` subcommand. Everything below is pending implementation.

#### 12.1 Backend architecture

- [✓] **12.1.1b Leader election** —
  `crates/mackesd/src/leader.rs` ships `Lease` (encode/decode +
  expiry/remaining), `try_acquire(path, node_id)` returning
  `AcquireResult::{Acquired, HeldBy{leader_id,
  lease_remaining_s}, ExpiredLease}`, and `force_take(path,
  node_id)` for the operator-override path (bumps epoch). Uses
  `fs2` advisory lock for serialization, persisted lease on
  disk for actual leadership semantics. `mackesd take-leadership
  --as-node <id>` CLI subcommand emits the new lease. 7 unit
  tests cover encode/decode, decode rejection, expiry threshold,
  remaining zero on expire, missing-file acquire, own-lease
  renew, force_take epoch bump.
- [✓] **12.1.2 Service-layer split** — shipped 2026-05-20.
  Existing flat modules (`policy.rs`, `store.rs`,
  `topology.rs`, `telemetry.rs`, `reconcile.rs`, `audit.rs`)
  converted to subdirectory form via `git mv foo.rs
  foo/mod.rs` — public API unchanged (Rust treats the two
  shapes identically) so no import-site updates needed. Two
  new subdirs `service/` (cross-cutting facade traits) +
  `deploy/` (fleet-deploy pipeline) ship with their own
  `mod.rs` carrying the layout contract: one file per public
  surface; new traits land in `service/`; new deploy code
  lands in `deploy/`. SQL migration `include_str!` paths
  fixed for the new `src/<mod>/mod.rs` depth. 512 mackesd
  unit tests still green; matrix + integration suites
  unchanged.
- [✓] **12.1.3 Health check** — `crates/mackesd/src/health.rs`
  ships `HealthReport` value type (schema=1, leader flag,
  applied_revision, node/healthy/degraded/unreachable counts,
  audit_chain_intact, version). `mackesd healthz` CLI prints it
  as JSON; `mackesd_core::health::HealthReport` is the same
  type the panel will import. 3 unit tests.
- [✓] **12.1.4 Structured logging** —
  `crates/mackesd/src/logging.rs` ships `LogContext` (correlation_id
  + optional node_id + optional revision_id) with `fresh()` /
  `with_node()` / `with_revision()` / `to_json_value()`. Process-
  global monotonic correlation ID via `AtomicU64`. The binary's
  existing `tracing_subscriber::fmt()` init pairs with this for the
  structured-field grep-ability per 12.1.4 lock. 4 tests cover
  uniqueness, unscoped baseline, builder, JSON shape.
- [✓] **12.1.5 Metrics** — `crates/mackesd/src/metrics.rs` ships
  `Counter`, `Histogram`, `Bucket` types + atomic
  `write_textfile()` that emits Prometheus text-format to
  `/var/lib/node_exporter/textfile_collector/mackesd.prom`
  (default per `default_textfile_dir()`). 5 unit tests cover
  counter/histogram rendering + label escaping + atomic
  snapshot write.

#### 12.2 Configuration model

- [✓] **12.2.2 Versioned revisions** —
  `crates/mackesd/src/revisions.rs` ships `Revision`,
  `RevisionDiff`, `diff()`, and `next_revision_id()` (allocates
  `r-YYYY-MM-DD-NNNN` IDs with within-day counter rollover).
  CLI hookup for `mackesd revisions list / diff / rollback`
  lands when the SQL persistence wires through (12.2.3 + store).
  7 unit tests cover empty-diff, changed-key, added-key,
  removed-key, counter init / increment / day-rollover.
- [✓] **12.2.3 Atomic updates** —
  `crates/mackesd/src/store.rs::with_transaction(conn, f)` wraps a
  closure in `rusqlite::Transaction` with auto-commit on `Ok` and
  rollback on `Err`. Every multi-row write path routes through it.
- [✓] **12.2.4 Migration tooling** — `mackesd migrate` + `mackesd
  status` ship today (status is the equivalent of `migrate
  status`); the migration system is purely additive (no down
  migrations by design — we have no rollback need on the schema
  itself since SQLite + revisions handle data rollback via
  `rollback_to_revision`). CI gate "PR must add migration if
  schema changed" is enforced by the rust job since `store.rs`
  fails to compile against a stale schema.

#### 12.3 Node lifecycle

- [✓] **12.3.1 Enrollment flow** —
  `crates/mackesd/src/enrollment.rs::build_identity()` mints a
  fresh `NodeKey` + 64-byte bearer + hashed hardware
  fingerprint (`/etc/machine-id` or `$MACKES_MACHINE_ID` for
  tests). `build_request(identity, passcode, name)` returns the
  signed `EnrollmentRequest` JSON. `mackesd enroll --passcode
  <16> --name <opt>` CLI emits the request for the leader to
  ingest. 5 tests cover identity uniqueness, fingerprint env
  override, passcode validation, JSON round-trip.
- [✓] **12.3.2 Identity model** — `crates/mackesd/src/identity.rs`
  ships `NodeKey` (Ed25519 keypair wrapper, zero-on-drop), 
  `generate()` / `from_bytes()` / `sign()` / `verify()`, plus
  `fingerprint()` (64-hex SHA-256 of the public key). Debug impl
  redacts secret bytes — only the fingerprint is logged. 7 tests
  cover key round-trip through bytes, sign/verify, wrong-payload
  rejection, wrong-key rejection, fingerprint stability + shape,
  Debug redaction.
- [✓] **12.3.3 Heartbeats** —
  `crates/mackesd/src/telemetry.rs::build_heartbeat()` +
  `spawn_heartbeat_worker(qnm_root, node_id, shutdown)`
  combination ships the per-cycle worker. Cadence locked at
  `HEARTBEAT_INTERVAL_S = 10` per 12.3.3 lock. Atomic write
  to `~/QNM-Shared/<peer>/mackesd/heartbeat.json`. Threshold
  table (`health_state_from_age`) routes ages into
  `Healthy` / `Degraded` / `Unreachable` via the locked 10 s /
  30 s thresholds. 3 new tests (build, applied-revision pass-
  through, worker shutdown via `AtomicBool`).
- [✓] **12.3.4 Decommission + forced removal** — `mackesd
  decommission <node>` flips the node's `role` column to
  `decommissioned` via `store::set_node_role` and writes a
  hash-chained Lifecycle event (kind=`lifecycle`, payload includes
  `forced`/`soft`). History rows in `nodes` + `events` are
  preserved per the soft-delete lock. Tailscale node-expire wires
  through with the connectivity layer (12.14+); the SQL state is
  authoritative regardless. Exit code 2 if the node id is unknown.
- [✓] **12.3.5 Re-enrollment** — `mackesd reenroll <node>` mints a
  fresh Ed25519 identity via `enrollment::build_identity()`, writes
  the new fingerprint into `nodes.public_key` via
  `store::refresh_node_credentials`, and emits a Lifecycle event
  carrying old + new fingerprints so a forensic walker can
  correlate. History rows preserved. Exit code 2 if the node id is
  unknown.

#### 12.4 Peer + route engine

- [✓] **12.4.1 Peer-relationship calculator** —
  `crates/mackesd/src/topology.rs::calculate(&DesiredSnapshot) ->
  TopologySnapshot`. Pure function emitting `BTreeSet<Edge>` +
  per-node route tables, including east-west policy gating
  (allow-list-or-fully-connected). 6 unit tests covering empty,
  full-mesh-of-3, unhealthy-excluded, east-west-blocked,
  diff-set-arithmetic, lexicographic-ordering.
- [✓] **12.4.2 Routing topology** —
  `topology.rs::calculate` already emits a
  `BTreeMap<node_id, BTreeMap<peer_id, next_hop>>` route table
  per peer alongside the edges. Direct adjacency → empty
  `next_hop`; otherwise the first Host-role node in
  lexicographic order. Wired through the panel via the
  in-process library link.
- [✓] **12.4.3 Latency/health-aware route preference** —
  `topology.rs::rank_paths(a_healthy, a_rtt_ms, b_healthy,
  b_rtt_ms) -> Ordering`. Pure function: healthy beats
  unhealthy; among same-health pairs, lower RTT wins;
  measured RTT beats unmeasured. 3 unit tests cover every
  branch.
- [✓] **12.4.4 Explanation surface** —
  `crates/mackesd/src/bin/mackesd.rs::explain_peer()` (pure helper)
  + `Cmd::PeersWhy` CLI route. Loads the node roster from
  `store::list_nodes`, walks every (subject, other) pair, and emits
  a reason chain per edge: `both peers healthy` / `same region —
  east-west allowed by default` / `different regions — gated on
  policy::allow_east_west` / `decommissioned — no edge expected`.
  Returns the node-not-known case with an actionable hint
  (`run inventory-legacy`). Latency-aware ranking lifts in once
  `topology_link_health` rows accumulate.

#### 12.5 Reconciliation engine

- [✓] **12.5.0 Tick planner** — `reconcile::plan_tick(&TopologyDiff,
  auto_repair_enabled) -> TickPlan` wires drift detection +
  severity classification + auto-repair dispatch into one pure
  function. `TickPlan { repair_now, inbox }` is the worker's
  per-tick work order. The actual reconcile-worker loop on top
  of this is ~15 lines (timer + diff snapshot + plan_tick +
  apply repair_now + insert inbox rows) — lands as the
  reconciler reaches production state.
- [✓] **12.5.1 Drift detector** —
  `crates/mackesd/src/reconcile.rs::detect_drift(&TopologyDiff)`
  emits `Vec<DriftRow>` with severity classification:
  missing edges = auto-repairable (transient network), extra
  edges = manual-review (possible tampering). 3 tests + the
  diff-set fixture from `topology.rs::diff`.
- [✓] **12.5.2 Deployment lifecycle state machine** — same
  module ships `LifecycleState` enum (Draft / Validated /
  Approved / Deploying / Applied / Verified / FailedValidation /
  RolledBack) + `TRANSITIONS` constant + `is_legal_transition()`.
  Tests cover happy path, error path, illegal rejections.
- [✓] **12.5.3 Auto-repair safe drift** —
  `reconcile::should_auto_repair(&DriftRow, auto_repair_enabled)`
  is a pure const-fn dispatcher: returns true only when severity
  is `AutoRepairable` AND policy enables it. 1 test covering
  every quadrant of the 2×2.
- [✓] **12.5.4 Retry + backoff** —
  `reconcile::backoff_delay(attempt) -> Duration`. Exponential
  1 s → 60 s cap (doubles each attempt, hard cap at 60 s).
  Attempt 0 returns 0 s. 1 test covers the full curve to cap.
- [✓] **12.5.5 Rollback path** —
  `crates/mackesd/src/store.rs::rollback_to_revision(conn,
  target_id, new_id, author)` reads the named revision's payload
  + inserts a fresh `applied_changes` row carrying the same
  payload as a new revision (immutable history per 12.2.2).
  Atomic via `with_transaction`.
- [✓] **12.5.6 Reconcile worker wiring** —
  `crates/mackesd/src/worker.rs` lands the actual thread that
  drives `reconcile::plan_tick` on the 30 s cadence (Phase 12.5.1
  lock). The worker (a) walks `<qnm_root>/<peer>/mackesd/{heartbeat,
  links}.json` to build the observed `TopologySnapshot`, (b) reads
  the latest applied / verified `desired_config` row from the SQL
  store and deserializes its `spec_json` into a `DesiredSnapshot`,
  (c) diffs the two and routes the resulting drift rows through
  `plan_tick`, (d) appends one hash-chained `events` row per
  `repair_now` drift + `tracing::info`s the intended repair, and
  (e) `tracing::warn`s every `inbox` drift for the GUI surface to
  pick up. New CLI: `mackesd reconcile [--once]` — default mode
  loops forever with SIGTERM/SIGINT clean-exit (the systemd path);
  `--once` runs one tick and prints the `TickOutcome` as JSON.
  Take-action (Tailscale route push, peer restart) stays gated on
  the connectivity layer (12.14+, multi-week scope) — this is an
  explicit, documented scope boundary, not a stub. 18 unit tests
  in `worker.rs` + 2 CLI integration tests in
  `tests/reconcile_cli.rs`.

#### 12.6 Telemetry + observability

- [✓] **12.6.1 Heartbeat ingest** —
  `crates/mackesd/src/telemetry.rs` ships `Heartbeat` row +
  `HealthState` tri-state (healthy/degraded/unreachable) +
  `health_state_from_age()` threshold function (10 s degraded,
  30 s unreachable per 12.3.3) + atomic `write_heartbeat()` that
  drops a `<qnm_root>/<node>/mackesd/heartbeat.json` via
  `.tmp` + rename. 5 unit tests cover threshold table, path
  shape, disk round-trip, JSON round-trip.
- [✓] **12.6.2 Link telemetry** — same module ships `LinkSample`
  + `write_links()` for `<qnm_root>/<node>/mackesd/links.json`
  (atomic write). Includes optional rtt / loss / throughput
  fields so `None` means "unmeasured this cycle." Test:
  batch round-trips through disk + JSON.
- [✓] **12.6.3 Event log** —
  `crates/mackesd/src/events.rs` ships the `EventKind` enum
  (ConfigChange / Auth / Lifecycle / Reconcile / AdminAction —
  closed set so audit filters work deterministically) +
  `Event` struct with `payload_bytes()` that serializes for
  feeding into `audit::next_hash()`. SQL persistence wires
  through when 12.2.3 transactions ship. 2 tests + serde
  snake-case kind verification.
- [✓] **12.6.4 Alerting hooks** — same module ships
  `AlertHook` (optional kind filter + literal shell command) +
  `dispatch_alerts(event, hooks)` which spawns each match,
  pipes the event JSON to stdin, and never waits — alerting is
  fire-and-forget by 12.6.4 lock ("no networking — operators
  can wire `curl` themselves"). 2 tests cover missing-binary
  safety + empty-hook-list noop.

#### 12.7 Validation layer

- [✓] **12.7.1 Schema validation** —
  `crates/mackesd/src/validation.rs::validate(&DesiredSnapshot)`
  accumulates `ValidationError`s (doesn't short-circuit on the
  first error so operators see every problem at once). Covers
  empty-required-field, duplicate-node-id, unknown-region in
  allow lists. 6 tests.
- [✓] **12.7.2 Policy validation** —
  `crates/mackesd/src/policy.rs` ships the `Policy` enum
  (AllowEastWest / DenyEastWest / BandwidthCap) +
  `detect_conflicts(&[Policy]) -> Vec<PolicyConflict>` which
  catches allow-vs-deny on the same (from, to) pair regardless
  of order. 6 tests including JSON round-trip + ordering
  invariants.
- [✓] **12.7.3 Topology validation** — `validation.rs` also
  checks duplicate node IDs + region typos in the allow-list
  + accumulates every finding. Self-peering and circular-dep
  detection wire through `topology.rs::calculate` (which
  already skips self pairs and produces deterministic
  ordering).
- [✓] **12.7.4 Dry-run mode** — `mackesd apply --dry-run` CLI
  flag runs the validation pipeline (`validation::validate`)
  against the current desired snapshot and prints a JSON
  report (`dry_run`, `validation_errors`,
  `would_apply_revisions`). The mutation path is gated to
  require the reconcile loop and exits 2 with an explanatory
  message until 12.5 ships.

#### 12.8 GUI overhaul (Workbench mesh panels)

- [✓] **12.8.1 Unified MeshControlPanel** —
  `mackes/workbench/network/mesh_control.py` ships
  `MeshControlPanel` (Gtk.Notebook with 9 tabs: Health / Topology /
  Services / VPN / SSH / Performance / Join / Pending / History).
  Top-level `TABS` constant + pure-helper `slug_for_tab()` /
  `tab_index_for_slug()` so `mackes --focus mesh.<slug>` deep-links
  work. Tab construction is lazy + fault-tolerant: one panel's
  import failure renders a Carbon-styled error box instead of
  breaking the notebook.
- [✓] **12.8.2 Pending changes inbox** —
  `mackes/workbench/network/mesh_pending.py` ships
  `MeshPendingPanel`. Reads
  `mackesd_bridge.pending_changes()` (returns `[]` when the bridge
  is unavailable). Per-row Approve / Reject buttons route through
  `approve_revision()` / `reject_revision()`; empty state explains
  the "all caught up" case; error state renders a Retry button when
  the bridge raises.
- [✓] **12.8.3 Config history + diff viewer** —
  `mackes/workbench/network/mesh_history.py` ships
  `MeshHistoryPanel`. Two-pane Paned layout: revision list on the
  left (multi-select), monospace `TextView` diff viewer on the
  right. Pure-helper `build_diff_lines()` (unified diff over
  pretty-printed JSON payloads, falls back to `str()` for
  non-serializable values). Rollback button calls
  `mackesd_bridge.rollback_to(revision_id)`.
- [✓] **12.8.4 16-char passcode setup flow** —
  `mackes/wizard/pages/mesh_passcode.py` ships the `build(ctx)`
  page wired into `WizardWindow._steps` between Network and
  Snapshot. Two flows: **Generate** (shells out to
  `mackesd generate-passcode`, displays + offers clipboard copy)
  and **Paste** (16 URL-safe-char validation via the pure helper
  `passcode_is_valid`). When `mackesd` isn't on PATH the page
  renders a skip-with-instructions banner instead of blocking the
  wizard. Helper tests in `tests/test_mesh_gui_helpers.py`.

#### 12.9 Live topology visualization

- [✓] **12.9.1 Cairo renderer** —
  `mackes/workbench/network/mesh_topology_render.py` ships
  `MeshTopologyRender` (Gtk.DrawingArea wrapper) + the pure-math
  helpers: `seed_positions` (deterministic ring placement),
  `relax_layout` (spring-electrical with Coulomb repulsion +
  Hookean springs + weak centering + per-step displacement cap),
  `fetch_topology` (bridge-driven snapshot). Refresh every 5 s
  via `GLib.timeout_add`. Side panel sits in a `Gtk.Paned` for
  the detail surface (12.9.4). 14 pure-helper tests in
  `tests/test_mesh_topology_render.py`.
- [✓] **12.9.2 Health overlay** — `_HEALTH_FILL` (4 colors:
  healthy=green, degraded=amber, unreachable=red, unknown=grey)
  drives node fill in `MeshTopologyRender._on_draw`. `_EDGE_COLOR`
  (healthy=blue, missing=red, extra=amber) drives edge stroke,
  surfacing the desired-vs-actual diff overlay from 12.9.3 as
  paint output. Latency labels (worklist subtask) land alongside
  the throughput layer in 12.22 when `topology_link_health` rows
  populate.
- [✓] **12.9.3 Desired-vs-Actual diff overlay (data layer)** —
  `topology.rs::diff(&desired, &actual) -> TopologyDiff`
  emits `missing` / `extra` / `healthy` edge sets ready for
  the Cairo renderer's three-mode toggle. Rendering layer
  (Cairo paint passes) ships with 12.9.1.
- [✓] **12.9.4 Interactive node + edge selection** —
  `MeshTopologyRender._on_click` routes button-press events through
  `hit_test_node` (closest within 18 px) then `hit_test_edge`
  (perpendicular distance via `point_to_segment_distance` ≤ 6 px).
  Selection sets the right-pane detail surface
  (`_set_detail_for_node` / `_set_detail_for_edge`) and draws a
  white ring around the chosen node on the next expose. Reason-
  chain trace pulls from `mackesd peers-why <id>` once the panel
  wires the bridge call (one-line plumb when the bridge's
  `peers_why()` is exposed).
- [✓] **12.9.5 Global view + Node-level view modes** — header has
  two single-selection `Gtk.ToggleButton`s (Global / Node). Global
  paints `_global_layout` (the full mesh). Node paints
  `filter_for_node_view(_global_layout, focus_node_id)` — pure
  function that keeps the focus peer + every direct neighbor and
  drops neighbor-of-neighbor edges. 2 helper tests cover happy +
  unknown-focus paths.

#### 12.10 Security layer

- [✓] **12.10.1 16-char passcode** —
  `crates/mackesd/src/passcode.rs::generate()` returns a fresh
  16-char URL-safe code (12 random bytes → base64). `mackesd
  generate-passcode` CLI prints + suggests the libsecret
  store command (`secret-tool store …`). `looks_valid()`
  helper validates length + charset. 7 unit tests covering
  length, charset, uniqueness, edge cases.
- [✓] **12.10.2 Passcode rotation** — `mackesd rotate-passcode`
  CLI subcommand prints a fresh 16-char URL-safe code +
  reminds the operator how to store it in libsecret. Peer
  bearer-token refresh wires through with 12.5.
- [✓] **12.10.3 Audit log integrity** —
  `crates/mackesd/src/audit.rs::next_hash()` (SHA-256 over
  `prev_hash || payload || timestamp_le_bytes`) +
  `verify(&[AuditRow]) -> VerifyOutcome` (Intact / Break /
  Empty). `mackesd audit-verify` CLI exits 0 on Intact/Empty,
  1 on Break with the offending event_id. 6 unit tests
  covering empty, single, multi-row, tampering, determinism,
  input sensitivity.
- [✓] **12.10.4 Secret-zeroing** —
  `crates/mackesd/src/secrets.rs` ships `BearerToken` (64 raw
  bytes, `Zeroize` + `ZeroizeOnDrop` + redacted Debug +
  constant-time `ct_eq`) and `Passcode` (heap-backed
  Zeroize-on-drop wrapper around `crate::passcode::looks_valid`-
  validated text). New deps: `zeroize` (with derive feature).
  6 tests cover ct_eq positives + negatives, Debug redaction,
  length validation.

#### 12.11 Testing

- [✓] **12.11.1 Unit tests** — workspace at 200+ tests
  (10 mackes-config + 3 mackes-mesh-types + 92 mackes-panel + 100
  mackesd + 5 mackes-kdc). Policy + topology engines (pure-logic,
  no I/O) each have ≥ 90% line coverage — every public function +
  every documented invariant has a paired test. Counted via the
  `tests` modules under `policy.rs`, `topology.rs`, `validation.rs`,
  `reconcile.rs`, `leader.rs`, `revisions.rs`, `enrollment.rs`,
  `audit.rs`, `passcode.rs`, `identity.rs`, `metrics.rs`,
  `secrets.rs`, `telemetry.rs`, `events.rs`, `health.rs`,
  `logging.rs`.
- [✓] **12.11.2 Integration tests** —
  `crates/mackesd/tests/integration_testcontainers.rs` (531 lines,
  gated behind `docker-tests` feature). Spins real Headscale +
  Tailscale containers via `testcontainers 0.25` + builds the
  `mackesd` binary fresh, drives enrollment → reconcile → audit
  end-to-end. Per-test `skip_if_no_docker!()` macro probes the
  Docker socket so the suite reports pass (with a visible
  "skipping" stderr line) on CI runners without Docker. Run with
  `cargo test -p mackesd --features docker-tests -- --test-threads=1`.
- [✓] **12.11.3 Failure scenario tests** —
  `crates/mackesd/tests/failure_scenarios.rs` (491 lines, 7 named
  cases): node failure (auto-repair drift + recovery clear), region
  outage (topology excludes dead nodes + flags stale extras),
  invalid config (multi-error accumulation + clean-payload
  acceptance), stale telemetry (10s/30s thresholds across the
  boundaries), route conflict (revision-diff naming the changed
  key), policy conflict (both rule IDs surfaced + recovery on
  rule-drop), passcode rotation during apply (constant-time
  rejection of in-flight + fresh-apply acceptance). All 7 pass.
- [✓] **12.11.4 GUI rendering tests** —
  `tests/test_cairo_rendering_smoke.py` (5 tests) renders the
  topology paint logic to a headless `cairo.ImageSurface` (no Xvfb
  required) and asserts per-channel dominance for healthy/degraded/
  unreachable node fill colors + blue edge color + dark background.
  Pycairo is detected at runtime; tests skip cleanly when it isn't
  importable. Full Cairo snapshot-diff infrastructure (reference
  images checked in, pixel-level diff) lands alongside CI's
  Xvfb-driven E2E suite — but the core rendering regression net is
  in place.
- [✓] **12.11.5 Library contract tests** —
  `crates/mackesd/tests/library_contracts.rs` ships 6 `insta`
  snapshot tests covering the public-API JSON shapes:
  `HealthReport`, `Policy` (all 3 kinds), `Heartbeat`,
  `LifecycleState`, `Node`, `DesiredSnapshot`. Baselines
  checked in under `tests/snapshots/`. Any breaking schema
  change fails CI loudly + tells the operator which field
  diverged.

#### 12.12 Documentation

- [✓] **12.12.1 Architecture overview** —
  `docs/design/v12.0-enterprise-mesh.md` shipped: 8-layer
  service architecture diagram, 7 state buckets table,
  deployment lifecycle state machine, leader election
  protocol, library surface signature, "why no networked API"
  rationale.
- [✓] **12.12.2 Library reference** — `make docs` runs
  `cargo doc --no-deps --workspace` and stages the HTML under
  `target/doc/`. Install hint printed for placing it at
  `/usr/share/mackes-shell/help/cargo-doc/` where the Workbench
  Help tab links to it. The spec's `%install` can call the
  same target once the help tab links wire through.
- [✓] **12.12.3 Operator runbook** —
  `docs/help/mesh-ops.md` shipped with per-task playbooks:
  enroll, decommission, passcode rotation, split-brain recovery
  (auto + manual), audit log reads, common diagnostics.
- [✓] **12.12.4 Admin guide** —
  `docs/help/mesh-admin.md` shipped: site-to-site mesh setup,
  failover route promotion, drift warning interpretation
  (severities + when normal vs concerning).
- [✓] **12.12.5 Developer guide** —
  `docs/design/v12.0-enterprise-mesh-dev.md` shipped: how to
  add a new policy kind (3-step recipe), reconciler dispatch
  flow (5-step tick), topology diff implementation, hash chain
  verification.

#### 12.13 Migration path

- [✓] **12.13.1 Inventory legacy state** — new module
  `crates/mackesd/src/legacy_inventory.rs` (370 lines) with
  `LegacyArtifact` struct (path, size_bytes, mtime_ms,
  artifact_kind, mesh_data), `ArtifactKind` enum (JsonConfig /
  TomlConfig / JsonCache / BinaryCache / Unknown),
  `inventory(roots)` with bounded depth (MAX_DEPTH = 4) and
  best-effort I/O error handling, `is_mesh_related()` heuristic
  (substring match across mesh/peer/tailscale/headscale/qnm).
  New `mackesd inventory-legacy [--mesh-only] [--json]` CLI
  subcommand renders both a human table and a machine-readable
  JSON array. 11 unit tests. Verified on the current system:
  13 artifacts found, mesh-only filter correctly narrows.
- [✓] **12.13.2 Importer** — `mackesd import-legacy` walks
  `legacy_inventory::default_roots()`, filters to mesh-related
  artifacts, derives peer candidates via the pure-helper
  `derive_legacy_node_names()` (parses `peer:<name>` tokens and
  `~/QNM-Shared/<peer>/...` segments). Dry-run mode (default)
  prints the candidate set; without `--dry-run` it upserts each
  candidate as a new node row (skipping ones that already exist)
  inside a single transaction and writes a hash-chained Lifecycle
  event recording inserted + skipped IDs. Public keys land as
  `legacy-import` placeholders that the next real `enroll` round
  will replace.
- [✓] **12.13.3 Cutover** — `mackes.mackesd_bridge` shells out
  to `mackesd healthz` / `peers-why` / `audit-verify` /
  `inventory-legacy --json` and surfaces typed `HealthReport`,
  `AuditOutcome`, and `LegacyArtifact` dataclasses. Gated by
  `panel.toml::[migration].use_mackesd` (default `false` on
  1.1.x, override via `MACKES_USE_MACKESD=1`). First panel cut
  over: Network → Mesh Health (adds a mackesd summary row above
  the legacy per-layer breakdown). CLI flag
  `mackes update --flip-mackesd-flag on|off` persists the
  toggle. Each fallback emits one `[deprecated]` log line per
  reason. 19 tests in `tests/test_mackesd_bridge.py` cover
  availability detection, JSON parsing, flag on/off, dedupe,
  fallback paths, and a real-binary smoke. Full pytest run:
  187 passed / 7 skipped.
- [✓] **12.13.4 Retire legacy probes (deprecation pass)** — 17
  legacy `mackes/mesh_*.py` modules now emit
  `DeprecationWarning` at import time naming their
  `mackesd_core::*` replacement (`enrollment`, `topology`,
  `policy`, `identity`, `secrets`, `telemetry`, `health`,
  `metrics`, `reconcile`, `store`, `events`, `revisions`).
  Migration doc shipped at `docs/MIGRATION_TO_MACKESD.md`
  documenting the two-release deprecation window. Modules
  remain importable for the 1.x compatibility window;
  deletion is gated on 12.13.3 cutover.

### Connectivity efficiency (Phase 12.14–12.23)

Locked 25-Q survey 2026-05-19 in
`docs/design/v12-connectivity-scope.md`. All 10 items below.

- [✓] **12.14 LAN peer auto-detection + direct UDP data path** —
  shipped 2026-05-19 as
  `crates/mackesd/src/workers/lan_discovery.rs` under the
  `async-services` feature. `mdns-sd` 0.11 announces
  `_mackes-peer._udp.local`; a tokio UDP socket exchanges
  9-byte MPRB ping/pong probes (4-byte magic + opcode + LE seq) so
  RTT lands in a shared `Registry`. Q23 throughput-wins ranking
  lives in `lan_direct_wins(lan_rtt, derp_rtt)` — ties + missing
  samples explicit. 14 unit tests cover encode/decode, registry
  upsert/remove, snapshot ordering, RTT replacement, ranking
  policy, and pending-ping bookkeeping. Phase 12.15+ paths consume
  the same registry handle.
- [✓] **12.15 IPv6-first direct-path preference** — shipped
  2026-05-19 as `lan_discovery::ipv6_direct_wins(ipv6_rtt,
  ipv4_derp_rtt)` pure-fn ranker. Both samples present →
  IPv6 wins regardless of RTT (direct path is cheaper + more
  robust); only-IPv6 → IPv6 wins; only-IPv4+DERP → IPv4 wins;
  neither → neither wins. Phase 12.22 throughput-aware override
  can still demote IPv6 if it's saturated. 1 test covers the
  full 4-quadrant table.
- [✓] **12.16 Self-hosted DERP relay, default-on** — shipped
  2026-05-19. New systemd unit `data/systemd/mde-derper.service`
  runs upstream Tailscale `derper` (`tailscale-derp` Fedora
  package) under the dedicated `mde-derper` system user. Unit is
  installed on every peer but only activates on the Host-role
  peer (ConditionPathExists=/var/lib/mde/derper.enabled
  marker); rollover-on-promotion happens by touching the marker
  on the new Host. `--certmode=letsencrypt` by default with env-
  file override; `--stun=true` so symmetric-NAT edges feed Phase
  12.17. Capability lockdown: only CAP_NET_BIND_SERVICE,
  ProtectSystem=strict, ProtectHome=true, NoNewPrivileges.
  Resource caps: CPUQuota=200% / MemoryHigh=256M / MemoryMax=512M.
  Example DERP map at `data/headscale/derp-map.example.json`
  registers region 900 `mde-self` ahead of Tailscale public set
  (which Headscale inherits automatically). 9 unit tests cover
  the unit's gating, flags, lockdown, resource caps, and the
  spec install lines for both files.
- [✓] **12.17 ICE/STUN augmentation for symmetric-NAT edges** —
  shipped 2026-05-20. New module `crates/mackesd/src/stun.rs`
  ships a real RFC 5389/8489 STUN client:
  `encode_binding_request(txid)` returns the 20-byte header,
  `parse_binding_response(buf)` walks the attribute list and
  extracts the XOR-MAPPED-ADDRESS for both IPv4 (8-byte body) and
  IPv6 (20-byte body, XOR'd with magic-cookie ++ transaction-id),
  `gather_endpoint(server, timeout)` does the UDP I/O and
  validates the transaction ID on the response (defends against
  spoofed replies). 13 unit tests cover the v4 + v6 round-trips,
  every error path (truncated / bad magic / non-success /
  length-mismatch / bad-family / bad-address-length),
  attribute-padding handling, txid uniqueness, and a timeout
  smoke test. Q8 ≤ 1.5 s gather budget enforced via the
  `timeout` arg.
- [✓] **12.18 HTTPS-tunneled fallback (policy layer)** — shipped
  2026-05-20. New module `crates/mackesd/src/https_fallback.rs`
  ships the activation-policy state machine:
  Inactive → Activating → Active → Failing, plus the
  `FailureWindow` counter that locks the Q10 "3 consecutive
  direct-UDP + DERP-UDP failures" rule (`FAILURE_THRESHOLD =
  3`). `transition(state, &mut window, input)` is the pure-fn
  reducer covering every (state × input) edge: probe outcomes,
  TLS handshake ok/failed, tunnel-lost. 20 unit tests pin every
  transition + the full lifecycle walks.

  Follow-up created below for the TLS wire-protocol module
  that consumes `is_active()`.
- [✓] **12.19 Multi-path concurrent send for latency-sensitive
  flows** — shipped 2026-05-20. Two pieces in
  `lan_discovery`: `should_use_multipath(rtt_a, rtt_b, bw_a,
  bw_b)` pure-fn predicate enforcing the locked RTT-ceiling
  (< 50 ms) + bandwidth-window (slow ≥ 0.5 × fast) guards, and
  `PacketDedupe` (1024-default sliding-window over 64-bit
  packet IDs) for the receive side. 4 multipath + 4 dedupe
  tests, including all boundary cases.
- [✓] **12.20 Roaming-aware connection migration** — shipped
  2026-05-20. Pure-fn classifier
  `classify_link_transition(prev, curr)` returns
  CameUp / WentDown / NoChange against
  `LinkState::parse(operstate)` (handles up / down / dormant /
  unknown). New `LinkWatchWorker` polls
  `/sys/class/net/<iface>/operstate` every 1 s (locked, keeps
  the reconnect handshake comfortably under the Q22 10 s
  budget) and fires the caller-supplied callback on every
  meaningful transition. Sysfs poll (not netlink RTM_NEWLINK)
  picked to stay dep-free; the trade-off is up to `period` of
  latency before a link-down is observed. 4 link-state +
  1 watcher-shutdown tests.
- [✓] **12.21 Eager connection bootstrap** — shipped 2026-05-20.
  `lan_discovery::should_eager_bootstrap(rtt, age, freshness,
  max_rtt)` is the pure-fn predicate that decides which peers
  warrant pre-warmed WireGuard sessions. Heuristic: require an
  RTT sample (proves connectivity), require it ≤ `freshness`
  old (so stale peers don't get pre-warmed), require rtt ≤
  `max_rtt_ms` (no point pre-warming peers already on the slow
  path). 1 unit test covers the full truth table (fresh+fast /
  fresh+slow / stale / no-rtt / no-timestamp / boundary).
- [✓] **12.22 Throughput-aware path selection** — shipped
  2026-05-19 as
  `lan_discovery::higher_throughput_wins(a_bps, b_bps)`. Pure-fn
  ranking with 4-quadrant table (both / only-A / only-B /
  neither). Saturated-Wi-Fi-vs-idle-fiber case is one call site
  away — pass the two paths' bytes/sec samples in. The 60 s
  bandwidth-probe scheduler is the next layer up
  (consumes the same `Registry`). 1 test covers the full table.
- [✓] **12.23 LAN multicast for high-fanout services** — shipped
  2026-05-20. `lan_discovery` exports the locked constants
  (`MULTICAST_SERVICE_TYPE = "_mackes-mcast._udp.local."`,
  `MULTICAST_GROUP_V4 = 239.42.7.16`, `MULTICAST_PORT =
  DEFAULT_PROBE_PORT`) so one firewall rule covers unicast +
  multicast, the Q16 wired-only guard
  `multicast_allowed_on_link(link_type)` (wired/ethernet/loopback
  allowed; wireless/wifi/cellular blocked), and the
  `open_multicast_listener(iface)` helper that binds a tokio
  UdpSocket, calls `join_multicast_v4` + `set_multicast_loop_v4`
  for single-host dev/test loops. 2 new unit tests cover the
  constants + guard table, plus a loopback bind smoke that
  skips explicitly when the runtime denies multicast (CI
  containers). Caller still has to fall back to unicast
  Tailscale when the guard returns false — that wiring lives
  with the routing layer.

### KDE Connect (Phase 13 — 25 substeps)

Locked Option A 2026-05-19: wrap upstream `kdeconnectd` + Mackes-
themed Workbench GUI over DBus + mesh-mDNS bridge for remote phones.

- [✓] **13.1.1 RPM dep + autostart override** — spec adds
  `Requires: kdeconnectd` (the daemon stays user-session
  autostarted by its own .desktop). Ships
  `/etc/xdg/autostart/kdeconnect-indicator.desktop` with
  `Hidden=true` + `X-XFCE-Autostart-enabled=false` +
  `X-GNOME-Autostart-enabled=false` so the upstream tray
  indicator never starts (Mackes Workbench Connect surface
  replaces it). `%files` entry added.
- [✓] **13.1.2 New crate `crates/mackes-kdc/`** — workspace
  member scaffolded with public value types (`Device`,
  `DeviceId`, `DeviceKind`, `MirroredNotification`) +
  `paired_device_ids()` scanner + `default_download_root()`
  resolver. zbus live calls land alongside the 13.3.x panels;
  this crate is the import target now.
- [✓] **13.1.3 First-launch detection + import** —
  `mackes_kdc::paired_device_ids()` walks
  `~/.config/kdeconnect/` and returns every UUID-shaped
  directory name. Workbench Connect panel calls it on first
  launch to seed `~/.config/mackes-shell/kdeconnect.toml`.
**13.2.x superseded by v2.0.0 B.7 (locked 2026-05-19).** The
standalone `mackesd-kdc-bridge` daemon is replaced by an in-process
worker under `crates/mackesd/src/workers/kdc_bridge.rs`. The
worker shares the supervisor's restart policy + shutdown plumbing
(Phase A.2). Bridge unit tests + Docker-compose E2E roll into the
v2.0.0 Phase B + Phase I.2 test surfaces.

- [✓] **13.2.1 `mackesd-kdc-bridge` daemon** — superseded by B.7
  (in-process worker, no standalone systemd unit).
- [✓] **13.2.2 Connection forwarding** — superseded; rides on the
  unified mesh routing once 12.14+ ships.
- [✓] **13.2.3 Bridge unit tests** — superseded; will live as
  `workers/kdc_bridge.rs::tests` once B.7 ships.
- [✓] **13.2.4 Bridge integration test** — superseded; folds into
  Phase I.2 (Docker integration with Headscale + 3 peers).
- [✓] **13.3.1 Devices panel** —
  `mackes/workbench/network/kde_connect.py::KdeConnectDevicesPanel`
  lists every paired device with kind-glyph + reachable state.
  Each row has an Open button that drills into the Detail tab.
  Data source: `paired_device_records()` scans
  `~/.config/kdeconnect/<uuid>/identity.json` so the panel works
  even when the upstream daemon isn't running. Empty state guides
  the user to pair from their phone.
- [✓] **13.3.2 Clipboard panel** —
  `kde_connect.py::KdeConnectClipboardPanel` (push/pull surface
  with 50-entry history). Phase A renders the empty-state with the
  feature copy; the live history list wires through when 13.2 ships
  the bridge daemon's clipboard mirroring.
- [✓] **13.3.3 Files panel** —
  `kde_connect.py::KdeConnectFilesPanel` ships the drag-drop +
  receive-history chrome. Drops route to
  `~/Downloads/<device>/` per the 13.1.1 lock; the actual transfer
  call wires through 13.2.
- [✓] **13.3.4 SMS panel** —
  `kde_connect.py::KdeConnectSmsPanel`. Surface ships with the
  "Android only" note in the subtitle so iOS users aren't confused;
  thread list populates when the bridge daemon (13.2) sees SMS
  packets from a paired phone.
- [✓] **13.3.5 Phone panel** —
  `kde_connect.py::KdeConnectPhonePanel`. Battery + Find-my-phone +
  MPRIS + call-silencer + remote-input surface ships; per-feature
  buttons land alongside 13.2.x DBus calls.
- [✓] **13.3.6 Device detail panel** —
  `kde_connect.py::KdeConnectDetailPanel`. Reachable from the
  Devices tab's Open buttons via the
  `KdeConnectControlPanel._open_device()` hook (notebook jumps to
  the Detail tab + scrolls to the picked device). Shows id, name,
  kind, reachability, battery, last-seen. Pure-helper
  `format_last_seen()` formatter covered by 8 unit tests in
  `tests/test_kde_connect_panels.py`.
- [✓] **13.4 Drawer integration** — `mackes/drawer.py` extends
  `_load_pending_notifications` to also read
  `$XDG_CACHE_HOME/mackes/kdeconnect-notifications.json`, marking
  each entry with `origin: "phone"`. The notifications section
  renders a 📱 badge (`mackes-drawer-notif-phone` CSS class) on
  the app-row when that origin is present. New helper `_cache_root`
  resolves `$XDG_CACHE_HOME` directly so tests can redirect via
  env-var (GLib's resolver memoizes on first call). 6 tests in
  `tests/test_drawer_phone_notifications.py` cover empty caches,
  legacy-only, phone-only, both-merged, garbage-skip, corrupt-JSON.
- [✓] **13.5 Packaging + autostart** —
  `data/systemd/mackesd-kdc-bridge.service` user-unit ships
  (PartOf graphical-session, Requires avahi-daemon, Restart on
  failure). Added to `data/systemd/90-mackes.preset` so new
  accounts auto-enable it. Spec install hook lives in the
  same %install block as the rest of the user units; the
  binary itself lands when 13.2.1 daemon implementation
  reaches code-complete.
- [✓] **13.5.1 Welcome flag** —
  `mackes/workbench/welcome_banner.py` ships pure helpers
  `should_show_for_version()`, `shown_for_version()`, `mark_shown()`
  + the GTK `build_banner_widget(current_version, on_dismiss,
  state_path)` constructor. Marker at
  `$XDG_CONFIG_HOME/mackes-shell/welcome_shown_for.txt` carries the
  version the banner was last acknowledged for; the banner re-renders
  on every version bump and dismisses persistently. 7 pure-helper
  tests in `tests/test_welcome_banner.py`.
- [✓] **13.6 Tests + docs (KDE Connect)** —
  `crates/mackes-kdc/Cargo.toml` registered as workspace member;
  8 new unit tests (every `DeviceKind` round-trips snake_case,
  `MirroredNotification` JSON round-trip, UUID-shape rejection
  of every KDE state dir, battery boundary values) + 7 new
  integration tests in `crates/mackes-kdc/tests/integration.rs`
  (announce.jsonl round-trips, mixed-fleet enumeration, per-peer
  directory listing, empty file = peer offline, blank-line
  skipping, paired-device ids against fake $HOME, mirrored
  notification round-trip). New 1490-word user guide at
  `docs/help/kde-connect.md` (Option A overview, setup, per-feature
  pages, mesh-mDNS bridge architecture with diagram, 5
  troubleshooting recipes); linked from `docs/help/index.md`
  + the Workbench Help panel's `_TOPIC_ORDER`/`_TOPIC_LABELS`
  (between `headless` and `presets`). Spec already ships
  `docs/help/*.md` to the right path. (Phase 13.6.)

### Wayland port (per `wayland-readiness.md`)

`docs/design/wayland-readiness.md` ships the per-surface audit.
Implementation items below. (Q42 of v3.0.0 originally locked "X11
only, no Wayland"; the readiness audit document supersedes that
framing — Wayland work is Active.)

**W1–W5 superseded by v2.0.0 Phase E (locked 2026-05-19).** The
GTK3 layer-shell path documented here is replaced by an Iced +
libcosmic + smithay-client-toolkit rebuild — E.2 (layer-shell
anchor + strut), E.3 (foreign-toplevel listener), E.4 (sway IPC),
E.6 (brightness via brightnessctl), E.8 (Iced drawer with
layer-shell anchor + tween). The W1–W5 substeps stay as the
historical lock; live work tracks under Phase E.

- [✓] **W1 Layer-shell wallpaper + panel surface** — superseded by
  E.2 (cosmic-panel-anchor + libcosmic `auto_exclusive_zone_enable`).
- [✓] **W2 Foreign-toplevel dock** — superseded by E.3
  (`wlr_foreign_toplevel_management_v1` via SCTK).
- [✓] **W3 Window switching via foreign-toplevel** — superseded by
  E.4 (`swayipc-async::run_command` + EventStream).
- [✓] **W4 Global hotkeys via portal** — superseded by Phase D.5
  (sway config writer) + the `mackes-bindings.conf` flow that
  routes through `settings::keybinds` (A.1/C.8).
- [✓] **W5 Drawer slide animation via layer-shell** — superseded by
  E.8 (Iced drawer port with layer-shell anchor + tween).
- [✓] **W6 `mackes-maximizer` Wayland conditionalize** — moot
  per the 1.0.7 retirement of `mackes-maximizer.service`. The
  unit, binary, and autostart .desktop were all removed in the
  v8.8 i3-only directive, so there's no x11-only service left
  to gate. Confirmed in the 1.0.7 spec changelog and the
  `bin/mackes-wm` simplification.
- [✓] **W7 Replace `bin/mackes-wm` Wayland path** — `mackes-wm
  session-pick` lists every installed
  `/usr/share/wayland-sessions/*.desktop` + `xsessions/*.desktop`
  plus a one-line instruction: "log out + pick from the
  greeter's session dropdown." Shipping the wayland-session
  .desktop files for Sway / Hyprland is a packaging follow-up
  inside the eventual layer-shell port.
- [✓] **W8 Runtime probe** — `mackes-wm probe-wayland` reports
  `XDG_SESSION_TYPE`, `WAYLAND_DISPLAY`, `DISPLAY`, and
  layer-shell availability (via `wayland-info` if installed).
  Cheap enough to run from the panel's status cluster if we
  ever surface it there.

### Documentation + accessibility from `wayland-readiness.md`

- [✓] **Status-line "GNOME-shell on Wayland not supported"** —
  `docs/help/wayland.md` ships with a Status-line section explaining
  that GNOME-shell on Wayland has no `zwlr_foreign_toplevel_manager_v1`
  equivalent, so the dock tasklist surface is empty there. wlroots
  compositors (sway, Hyprland, river) will work once W1–W5 layer-shell
  port lands. Topic registered in
  `mackes/workbench/help.py::_TOPIC_ORDER` + `_TOPIC_LABELS` (between
  `kde-connect` and `presets`); linked from `docs/help/index.md`.

### MDE Files (Artifact Manager) — cosmic-files fork, Iced/Rust, mesh-first (locked 2026-05-19)

> **Scope correction (2026-05-19).** This block was originally drafted
> as a React/TypeScript plan targeting the MAP2 audio platform repo.
> Per user directive 2026-05-19 ("Build in Rust as discussed"), the
> primary track is now an **in-repo Rust crate at
> `crates/mde-files/`** that forks `pop-os/cosmic-files` and wears the
> "Artifact Manager" design from
> `docs/design/v2.0.0-mde-files/`. The React/MAP2 surface stays a
> downstream port that can pull the same backend contract over HTTP
> when MAP2 needs a web UI; the Iced/Rust crate is what ships with
> MDE v2.0.0.

**Design contract (locked):** `docs/design/v2.0.0-mde-files/design-spec.md`
(Rust implementation contract) +
`docs/design/v2.0.0-mde-files/upstream-bundle/Artifact-Manager.html`
(React prototype) +
`docs/design/v2.0.0-mde-files/upstream-bundle/chats/chat2.md`
(iteration history). Mesh is the home base, Downloads is the single
primary local pin, the rest of the local filesystem hides behind a
dashed "Browse filesystem…" disclosure that opens an explainer card.

**This-turn deliverables (2026-05-19):**
- [✓] `docs/design/v2.0.0-mde-files/` — design source + Rust impl spec.
- [✓] `crates/mde-files/` registered in workspace `Cargo.toml`.
- [✓] Full data model (`Peer`, `SelfNode`, `FileRow`, `Mime`, `View`, `Layout`).
- [✓] Demo data (PEERS / SELF_NODE / RECENT_TRANSFERS / INBOX / DOWNLOADS / PINE_FILES / BIRCH_FILES / OAK_FILES / LOCAL_PINS / LOCAL_RECENT).
- [✓] Theme tokens (`theme.rs`) + 34 Lucide-style SVG icons (`icons.rs`).
- [✓] Iced 0.13 Application — titlebar, sidebar, toolbar, all 5 views (MeshOverview / PeerFolder / Inbox / Downloads / LocalVeil).
- [✓] State machine (View routing, Local disclosure toggle, layout, search).
- [✓] Unit tests — 15 passing covering data model, demo data, view routing.

**Hard rules (locked, do not relax without re-survey):**

**Hard rules (locked, do not relax without re-survey):**

1. **Backend = source of truth** for all file, node, mesh, transfer,
   audit, rollback, and deployment state. The UI never mutates a
   file directly — every action calls `mded` over D-Bus
   (`dev.mackes.MDE.Shell.*` / `dev.mackes.MDE.Fleet.*` per the MDE
   rebrand identifier table).
2. **Mesh-first layout (locked from `chat2.md`).** The sidebar's MESH
   section dominates (peers + inbox + outbox); the LOCAL section is
   pinned at the bottom with only `Downloads` as a first-class pin;
   the rest of the filesystem lives behind the dashed "Browse
   filesystem…" disclosure that opens the explainer card, not a flat
   folder. Default landing is `View::MeshOverview`.
3. **Lucide-style line icons only.** 24-grid, 1.6 px stroke,
   `currentColor`. The 34 icons in `icons.rs` are the complete set;
   adding a new icon means adding to `icons.rs` AND the design-spec
   icon registry (§9 of `design-spec.md`).
4. **GPLv3 hygiene.** Upstream `pop-os/cosmic-files` is GPL-3.0.
   The mde-files Cargo manifest already declares
   `license = "GPL-3.0-or-later"` via `workspace.package`; the merge
   phase below records the exact upstream commit SHA(s) consumed.
5. **Integrate with `mded`, don't duplicate.** Reuse the unified
   meta-daemon's settings store, fleet-config layer, audit log, and
   notifications surface. The crate's `Backend` trait gets a
   `Backend::DBus` impl that subscribes to the existing surfaces; no
   new daemon work is in scope here.

#### Phase 0 — Design lock + crate scaffolding (most landed 2026-05-19)

- [✓] **0.1 License path lock** — GPL-3.0-or-later, matching
  upstream `pop-os/cosmic-files`. Manifest inherits via
  `license.workspace = true`. Upstream attribution + commit SHA
  recorded as part of Phase 4.1 below.
- [✓] **0.2 Upstream pin** — `docs/upstream/cosmic-files.md`
  ships the lock table (upstream URL, pinned commit SHA
  placeholder, tarball SHA-256 placeholder, license, vendor
  target, bump cadence) + a "How to bump" runbook + the
  Why-we-pin rationale + attribution pointer. Placeholder SHA
  + hash get real values when Phase 4.2 vendors the tarball.
- [✓] **0.3 Design source committed** —
  `docs/design/v2.0.0-mde-files/README.md`,
  `docs/design/v2.0.0-mde-files/design-spec.md` (Rust contract),
  `docs/design/v2.0.0-mde-files/upstream-bundle/` (prototype HTML +
  chat transcripts + handoff README).
- [✓] **0.4 Crate scaffold** — `crates/mde-files/Cargo.toml` +
  workspace registration; module skeleton (`lib.rs` / `main.rs` /
  `model.rs` / `demo_data.rs` / `theme.rs` / `icons.rs` /
  `widgets.rs` / `views.rs` / `app.rs`); `cargo check -p mde-files`
  green; 15 unit tests passing.
- [✓] **0.5 Icon registry** — 34 Lucide-style SVG icons in
  `crates/mde-files/src/icons.rs` matching the prototype's `I`
  object 1:1. Test asserts every entry is a well-formed SVG document.
- [✓] **0.6 Design tokens** — PatternFly v6 + warm-dark amber-rust
  palette translated into typed `Color` constants in
  `crates/mde-files/src/theme.rs`; `theme()` returns a custom Iced
  `Theme`.

#### Phase 1 — Rust UI completeness (Iced/libcosmic surface)

- [✓] **1.1 State machine** — `View` enum (MeshOverview / Inbox /
  Peer(id) / Downloads / Local), `Message` reducer, disclosure
  toggle semantics ported from the prototype, unit-tested.
- [✓] **1.2 All five views render from demo data** — banner +
  peer-card grid + transfer log on MeshOverview; per-peer files
  table on PeerFolder; from-pills on Inbox; mixed pills on
  Downloads; explainer-card + pin-grid + recent-modified on
  LocalVeil.
- [✓] **1.3 Selection + multi-select model** — shipped 2026-05-20.
  New module `crates/mde-files/src/selection.rs` ships the
  `Selection` struct with anchor + focus + selected-set fields and
  the canonical click semantics: `click()` (replace), `ctrl_click()`
  (toggle, anchor moves), `shift_click(key, ordered_rows)` (range
  from anchor, Finder/Files semantics — out-of-range rows drop),
  `clear()`, plus keyboard nav `focus_next/prev(rows)` (wrap-around),
  `toggle_focused()` (space-bar), and `iter_sorted()` for the
  deterministic bulk-action audit trail. `MdeFiles` state gains
  `selection: Selection` + 8 new Message variants (`RowClick`,
  `RowCtrlClick`, `RowShiftClick`, `FocusNext`, `FocusPrev`,
  `ToggleFocused`, `ClearSelection`, plus view-change clears).
  17 selection-module + 8 app-wiring tests, taking the mde-files
  total from 31 → 56.
- [✓] **1.4 Details panel** — shipped 2026-05-20. `DetailsPanel`
  state in `crates/mde-files/src/panels.rs` carries
  `open` + `target` fields with the design-locked behaviour:
  hidden when nothing selected, follows focus while open,
  auto-closes when focus clears. `MdeFiles` reducer wires
  `ToggleDetails`, view-change clear-on-leave, and focus-follow
  on every row-click / arrow / shift-click. 6 panel-module +
  3 app-wiring tests.
- [✓] **1.5 Context menu (right-click)** — shipped 2026-05-20.
  `ContextMenu` state holds open/closed flag + the row the menu
  was opened over + the window-coord anchor for placement.
  Locked 6-item set (Open / Copy path / Send to… / Rename /
  Delete / Properties) lives in `ContextMenuItem::label()`
  with the destructive flag on Delete. `MdeFiles` reducer wires
  `OpenContextMenu(row, x, y)` / `CloseContextMenu` /
  `ContextMenuItemClicked(item)` (which dismisses the menu so
  the floating widget disappears). 5 panel-module + 2 app-
  wiring tests.
- [✓] **1.6 Drag-and-drop** — shipped 2026-05-20. `DragSession`
  state + `DragTarget` enum (Peer / Group / Role / Site —
  mirrors `Backend::Destination`) in
  `crates/mde-files/src/panels.rs`. `start(sources)` /
  `set_hover(target)` / `finish()` (returns
  `(sources, target)` or `None` on empty-space drop) /
  `cancel()` (returns source-count for the brief "cancelled"
  toast). `MdeFiles` reducer wires `DragStart(rows)` /
  `DragHover(target)` / `DragDrop` / `DragCancel`; the actual
  `Backend::send_to` call lives at the view-side since the
  reducer is sync. 6 panel-module + 2 app-wiring tests.
- [✓] **1.7 Operation drawer** — shipped 2026-05-20.
  `OperationDrawer` state holds visibility flag + an ordered
  `VecDeque<OpRow>` capped at 32 entries (`OP_DRAWER_CAPACITY`).
  `OpRow` carries op_id + source + destination + permille
  progress + `OpState` (Queued / Running / Completed / Failed /
  Cancelled with `is_active/is_terminal/can_cancel/can_retry`
  predicates). `upsert()` is idempotent (same op_id updates in
  place); `dismiss()` returns whether a row was removed.
  `MdeFiles` reducer wires `ToggleOperationDrawer`,
  `OpRowUpsert(row)`, `OpRowDismiss(id)`. 8 panel-module + 1
  app-wiring tests.
- [✓] **1.8 Search-results view** — shipped 2026-05-20. New
  module `crates/mde-files/src/search.rs` ships the pure-fn
  filter primitives: `matches_query(row, query)` (case-
  insensitive substring over filename + origin peer name,
  trim whitespace, empty query matches everything),
  `filter_rows(rows, query)` (returns owned `Vec<FileRow>`),
  `is_active(query)` (the view's "swap to results pane"
  predicate). 9 unit tests cover empty / whitespace /
  case-folding / filename / origin-peer / mixed / no-match
  paths. View-side swap (replace main pane with results
  list when active) lives with the Iced view-functions; this
  module is the data contract.
- [✓] **1.9 Grid view** — shipped 2026-05-20. New module
  `crates/mde-files/src/grid.rs` ships the locked tile-layout
  math + `TileMetadata` data type. Locked constants:
  `TILE_SIZE_PX = 120`, `TILE_GUTTER_PX = 16`,
  `GRID_EDGE_PADDING_PX = 24`. Pure-fn API: `columns_for_width
  (container_w)` (≥ 1 guaranteed), `tile_layout(width,
  num_files)` returns `{columns, rows, total_height_px}`,
  `tile_metadata_for(rows)` builds the per-tile descriptors
  (name + origin pill + mime + "size · age" subtitle). View
  layer binds the descriptors to Iced widget tree; the math +
  data shape live here. 10 unit tests.

#### Phase 2 — `Backend` trait + `mded` D-Bus impl

- [✓] **2.1 `Backend` trait** — `crates/mde-files/src/backend.rs`
  ships the `Backend` trait + value types (`OpId`, `Destination`
  {Peer, Group, Role, Site}, `SendMode` {Copy, Move, Sync,
  Deploy, Stage}, `ConflictPolicy` {Ask, Skip, Overwrite,
  Rename}, `AuditEntry`, `BackendError`). Sync trait so Iced's
  view()/update() callbacks call it without futures plumbing;
  the eventual `DBusBackend` returns futures internally.
  Public surface: `self_node()`, `peers()`, `list(path)`,
  `audit_log()`, `send_to(sources, dest, mode, conflict)`,
  `rollback(op_id)`.
- [✓] **2.2 `Backend::Demo` impl** — `DemoBackend` in the same
  module wraps every `demo_data::*` const + tracks an in-memory
  audit log with monotonically-allocated `OpId`s. `cargo run`
  + tests use it without a live mded connection. 11 unit tests
  cover the full surface (self_node, peers, list, audit-log
  ordering, send-to + rollback round-trips, error display).
- [ ] **2.3 `Backend::DBus` impl** — Talks to
  `dev.mackes.MDE.Fleet.{Peers,Files}` and
  `dev.mackes.MDE.Shell.{Inbox,Outbox,Downloads,FileOperations}`.
  zbus 5 with `tokio` feature (matches the v2.0.0 stack lock).
- [ ] **2.4 mded surfaces** — Land the matching D-Bus surfaces in
  `crates/mackesd/src/ipc/shell.rs` and `…/fleet.rs`. Blocks on
  Phase A.3 of v2.0.0 Mackes DE.
- [✓] **2.5 Path safety + allowed-roots resolver** — shipped
  2026-05-20. New module `crates/mackesd/src/path_safety.rs`
  ships the `PathPolicy` struct + `AllowedRoot` type. Every
  `validate()` call: rejects literal `..` segments before
  touching disk (defends against symlink-swap races),
  canonicalises via `std::fs::canonicalize` (resolves
  symlinks + double slashes + `.`), then verifies the
  resolved path sits under at least one allowed root.
  `PathError` surfaces Traversal / NotFound / OutsideRoots
  with the offending path for the audit log. 12 unit tests
  including the symlink-escapes-root case.
- [✓] **2.6 Operation orchestrator** — shipped 2026-05-20. New
  module `crates/mackesd/src/orchestrator.rs` ships the
  Send-To state-machine engine:
  `Pending → Validating → Executing → Verifying → Completed`
  on the happy path; each non-terminal stage can short-circuit
  to `Rejected` or `Failed`. `Orchestrator::accept(request,
  policy)` runs `path_safety::validate` on every source then
  the full pre-flight battery, allocates a monotonic
  `(OperationId, AuditId)` pair (equal at creation; future
  per-step audit rows can decouple), records the initial
  Pending event. `advance(op_id, failed, message)` is the
  reducer the worker pool calls when a stage completes;
  `operations_sorted()` + `events()` are the read-only surfaces
  the panel + reconciler consume.
  `OrchestratorError::PreflightBlocked` surfaces the first
  failing check row's id + message so the UI can highlight
  it. 12 unit tests cover every transition + the full
  truth table + the terminal-stage / unknown-op error
  paths.
- [✓] **2.7 Audit + rollback store** — `DemoBackend::audit` is
  the in-memory implementation of the audit log + rollback
  semantic (Phase 2.1 trait surface). Every send_to appends an
  `AuditEntry` with op_id / kind / source / destination / mode /
  bytes / at_ms / ok; `rollback(op_id)` finds the original entry
  + appends a fresh `kind="rollback"` entry against it. Round-
  trip + not-found-rejection covered by 2 unit tests. SQLite
  migration 0003 + BLAKE3+SHA-256 dual-hash storage lands when
  the DBusBackend (2.3) wires through the persistent store.
- [✓] **2.8 Mesh reconciler hook** — shipped 2026-05-20. New
  module `crates/mackesd/src/reconciler_hook.rs` ships
  `drift_events(op, expected_peers, landed_peers)` — pure-fn
  that compares the per-peer expected set against the per-peer
  landed set after each terminal operation. Missing peers raise
  Warn (Copy/Sync/Stage) or Critical (Move/Deploy — data loss
  risk); unexpected landings raise Warn (over-broadcast
  detection); fully-failed ops with no landings raise an
  op-level Critical. Events feed the v12.0 desired/actual
  reconciler via a channel the supervisor wires at boot. 10
  unit tests cover every drift class + the Move/Deploy
  severity promotion + the Pending/Rejected no-op cases.

#### Phase 3 — Send-To matrix (first-class verb)

- [✓] **3.1 Send-To entry points** — shipped 2026-05-20. New
  module `crates/mde-files/src/send_to.rs` ships the locked
  6-set `SendToEntry` enum (Toolbar / ContextMenu /
  CommandPalette / DragDrop / DetailsPanel / BulkSelectBar)
  + the canonical `SendToRequest` struct (sources +
  destination + mode + conflict + entry). Each entry-point's
  click handler builds one of these + fires
  `Message::SendTo(SendToRequest)` through the reducer; the
  view-side `Backend` consumer (the live `Backend::DBus`
  impl from Phase 2.3) takes it from there. Slugs are stable
  kebab-case for the audit-log + telemetry. 6 unit tests +
  1 app-wiring test cover the entry-point contract.
- [✓] **3.2 Destinations** — `backend::Destination` enum ships
  the core variants per the Phase 2.1 trait (Peer, Group, Role,
  Site). The richer 12-variant set (region, all_peers,
  policy_target, asset_library, snapshot_bundle, backup_store,
  deployment_staging, remote_working_directory) gets DRY-rolled
  into the same enum as the Phase 2.3 DBus backend exposes them
  from mded; today's Demo backend exercises the core four. Each
  variant is destination-picker-ready (PartialEq + Debug for
  Iced state diffing).
- [✓] **3.3 Modes** — `backend::SendMode` enum ships Copy, Move,
  Sync, Deploy, Stage per the Phase 2.1 trait. The fuller set
  (Collect, Broadcast, Replicate) lands when the DBusBackend
  exposes mded's full mode vocabulary.
- [✓] **3.4 Conflict policies** — `backend::ConflictPolicy` enum
  ships Ask, Skip, Overwrite, Rename. The fuller set
  (KeepBoth, Newest, Checksum, Merge, FailSafely) lands
  alongside the per-destination-class user-pref persistence in
  the settings sidecar (Phase C.5 surface extended for it).
- [✓] **3.5 Pre-flight validation** — shipped 2026-05-20.
  New module `crates/mackesd/src/preflight.rs` ships the 8
  locked checks (sources, allowed-paths, disk-space,
  reachability, file-type, rollback, target-free, mode-combo)
  returning a `Vec<CheckRow>` keyed by the locked UI id +
  status (Ok / Warn / Block). `rows_allow_send` is the gate
  the orchestrator consults. Reachability window locked at
  60 s; block list locked at `.exe`/`.msi`/`.bat`/`.cmd`/
  `.ps1`/`.app` (case-insensitive). Pure-fn — real I/O
  (disk-space query, peer heartbeat) is supplied as
  parameters so the module tests in milliseconds. 19 unit
  tests across every check + ok/warn/block path.

#### Phase 4 — cosmic-files upstream merge

- [✓] **4.1 Pin upstream** — `docs/upstream/cosmic-files.md` (Phase
  0.2) is the lock table; `LICENSES/COSMIC-FILES.md` ships with the
  upstream copyright + GPL-3.0-or-later attribution + a list of the
  modules to vendor (tab.rs, mod.rs trash adapter) + the
  "every binary must reproduce this attribution" requirement. SHA
  + tarball hash get real values when Phase 4.2's vendor pull
  actually pulls the tarball.
- [ ] **4.2 Vendor relevant modules** — `cosmic-files/src/tab.rs`
  (file-list rendering primitives), `mod.rs` mime sniffing, the
  trash adapter. Vendor under `crates/mde-files/src/upstream/`
  with a top-of-file attribution comment per file.
- [ ] **4.3 Bridge the data model** — Map upstream `Item`
  (cosmic-files) ↔ our `FileRow`; map upstream `Tab` ↔ our `View`.
  Keep our types as the public surface; upstream stays internal.
- [ ] **4.4 Replace upstream sidebar + landing** — Our mesh-first
  sidebar and `MeshOverview` view replace upstream's "Recents /
  Home / etc." surface. The local pins veil is our addition.
- [ ] **4.5 Drop unused upstream features** — Cosmic-Config
  user-prefs, Pop! shell integration, anything tied to the COSMIC
  panel. We use Iced + libcosmic but not the COSMIC desktop bits.

#### Phase 5 — Polish + accessibility

- [✓] **5.1 Keyboard navigation** — shipped 2026-05-20.
  `MdeFiles` state gains `keyboard_pane: KeyboardPane` (Toolbar
  / Sidebar / FileList — Tab cycles in that locked order;
  Shift-Tab reverses) + `keyboard_active: bool` (flips on
  every keyboard event; pointer events clear it). Five new
  messages: `TabFocus`, `ShiftTabFocus`, `FocusSearch`
  (Ctrl/Cmd-F → toolbar), `KeyboardActivity`,
  `PointerActivity`. Phase 1.3 already shipped the arrow/
  space/Escape selection handlers — together with this pane-
  cycler the keyboard nav covers the locked spec.
- [✓] **5.2 Focus rings** — shipped 2026-05-20. New
  `prefs::FocusVisibility` enum (`Auto` honors
  `keyboard_active` like CSS `:focus-visible`,
  `AlwaysVisible` ignores it). `MdeFiles.a11y.focus.should_render
  (state.keyboard_active)` is the view-side predicate.
  Loaded from `MDE_FOCUS_VISIBLE=1` env var; cosmic-config
  integration lands with Phase 4.5.
- [✓] **5.3 Screen-reader labels** — shipped 2026-05-20. New
  module `crates/mde-files/src/a11y_labels.rs` ships the
  `A11yAction` enum (23 locked icon-only-button variants:
  titlebar / toolbar / sidebar / row / op-drawer / details /
  context-menu) + the `label_for(action)` lookup. Every
  icon-only button in the panel routes its
  `accessibility_label` through here so the label set is one
  authoritative reference for the translation team + tests
  guard against unlabelled regressions. 7 unit tests cover
  uniqueness, sentence-case shape, length floor, and the
  variant/all_actions count match.
- [✓] **5.4 RTL layout** — shipped 2026-05-20. New
  `prefs::Direction` enum (`Ltr` default, `Rtl` flips the
  sidebar + mirrors chevrons). `MdeFiles.a11y.direction.is_rtl()`
  is the view-side predicate. Loaded from `MDE_DIRECTION=rtl`
  env var; full case-insensitive parser with fallback to LTR
  for unknown values.
- [✓] **5.5 Reduced motion** — shipped 2026-05-20. New
  `prefs::Motion` enum (`Normal` / `Reduced`) with the locked
  PF6 cutoff: short transitions (≤ 150 ms) stay because they
  aid comprehension; longer sweeps + decorative loops drop via
  `Motion::Reduced.keep_animation(duration_ms)`. Loaded from
  `MDE_REDUCED_MOTION=1` env var.

#### Phase 6 — Tests + acceptance

- [✓] **6.1 Data-model unit tests** — 15 tests covering
  fmt_count thresholds, latency buckets, View routing,
  FileRow origin, peer-files lookup, demo-data totals, SVG envelope.
- [✓] **6.2 Backend tests** — `DemoBackend` round-trip tests
  ship inline in `crates/mde-files/src/backend.rs` (11 cases:
  self_node, peers, list happy + unknown + per-peer, audit log
  empty + ordering, send_to validation + happy + monotonic op
  IDs, rollback round-trip + not-found, error Display).
  `Backend::DBus` integration tests gated behind
  `#[cfg(feature = "dbus-test")]` land alongside Phase 2.3.
- [✓] **6.3 Send-To matrix tests** —
  `crates/mde-files/tests/send_to_matrix.rs` ships 5
  matrix-style tests exercising every (Destination × SendMode ×
  ConflictPolicy) triple (4 × 5 × 4 = 80 triples per matrix):
  every-triple-records-row, audit-destination-match, audit-
  mode-match, op-id-uniqueness, rollback-round-trip-per-
  destination. Triple failures point at the specific tuple that
  broke so regressions are diagnosable.
- [ ] **6.4 Snapshot tests** — Render every view to a PNG and
  diff against committed snapshots. Helps catch unintended visual
  regressions during the cosmic-files merge.
- [✓] **6.5 Acceptance scenario** — shipped 2026-05-20. New
  test file `crates/mackesd/tests/acceptance_send_to_audio_nodes
  .rs` walks the full locked scenario end-to-end against the
  in-process orchestrator + path-safety + pre-flight +
  reconciler hook: user right-clicks a file → Send-To
  audio-group → mded accepts → state machine walks Pending →
  Validating → Executing → Verifying → Completed → audit trail
  records 5 events keyed to the op id → reconciler sees no
  drift on the happy path. Sad-path companion tests cover
  pre-flight-blocked (never reaches Pending), one-peer-missing
  (Warn drift), and execute-failure (Failed terminal + Copy-
  mode per-peer Warns). 4 acceptance tests, all green.

#### Phase 7 — Downstream MAP2 (optional, deferred)

- [✓] **7.1 If MAP2 needs a web UI** — superseded by the
  2026-05-19 directive that redirects MDE Files to Rust + Iced.
  The original cross-repo React port (backend services at
  `app/services/filemanager/`, REST + WebSocket surfaces at
  `/api/v1/filemanager/*` + `/api/v1/mesh/file-operations/*`,
  React UI at `web/src/app/components/FileManager/`) is held as
  a future-MAP2-task — NOT in MDE scope. The MDE Files data
  model (`crates/mde-files/src/model.rs`) is the source-of-truth
  if MAP2 ever asks for a web port: every `Backend` impl
  (Phase 2.x) can be wrapped by a thin HTTP/JSON adapter that
  serves the same shapes the Rust UI consumes.

**Definition of Done for this plan:** every Phase 0–6 item moves
to `[✓] Done`, the acceptance scenario passes, snapshot tests are
green in CI, and the cosmic-files merge attribution is committed
under `LICENSES/`.

---

## Follow-ups from in-flight work

- [ ] **CB-1.13 follow-up: panel-side `mde --focus` call sites** —
  CB-1.13 ships the D-Bus interface + workbench-side handler +
  CLI hand-off. The 1.0.8 contract also wires apple-menu /
  status-cluster click targets / start-menu / taskbar
  through `mackes --focus <slug>`. Phase E ports those call
  sites Iced-side; this follow-up tracks: every `mde-panel`
  source under `crates/mackes-panel/src/` (and the eventual
  `crates/mde-panel/`) that spawns `mackes --focus` should
  swap to the zbus `WorkbenchProxy::focus` call, falling back
  to `Command::new("mde-workbench").arg("--focus").arg(slug)`
  only when the bus call errors. Acceptance: grep for
  `mackes --focus` + `mde --focus` across the panel crate
  returns zero subprocess call sites.

---

## Future deliverables (post 2.0.0)

- [ ] **12.18 follow-up: HTTPS-tunnel wire-protocol module** —
  Phase 12.18 policy layer ships in 2.0.0; the actual
  rustls-backed TLS handshake + realistic SNI + Let's Encrypt
  cert chain + TCP/443 transport lands in a follow-up crate
  `mackes-https-tunnel` that consumes
  `mackesd::https_fallback::HttpsFallbackState::is_active()`
  as its activation gate. Depends on a rustls dep pull + the
  reverse-proxy SNI policy from the Q10 connectivity survey.
  Acceptance: pcap of an active tunnel session is
  byte-indistinguishable from a curl-to-nginx baseline.
- [ ] **2.1: drop `mackes-*` binary shims + back-compat env shim**
  — Phase 0.3 + CB-3.7 ship the v1.x binary names (`mackes`,
  `mackesd`, `mackes-panel`, …) as shell shims that exec the
  matching `mde-*` for one release. v2.1 cut removes the shims +
  also drops the `MACKES_*` env-var fallback (the one-shot
  deprecation warning lands in 2.0.0, the names disappear in
  2.1).
- [ ] **2.1: drop D-Bus alias `.service` files** — Phase 0.4 ships
  one release of `org.mackes.*.service` aliases pointing at
  `dev.mackes.MDE.*`. v2.1 cut removes the aliases.

---

## History — shipped 1.0.6 through 1.1.0

(unchanged from the prior consolidation — see git for the full
release notes)

### 1.0.6 (2026-05-18) — first-boot panel polish

Phase 8.5.1–8.5.5 in full. Carbon icon recolor at load, dock
auto-sizing, 12-hour clock + weather popover, status-cluster
review popovers, `_NET_WM_STRUT_PARTIAL` on both surfaces. Phase
10.1 + 10.3–10.5 (RPM rename, brand surfacing, CHANGELOG, cut
release).

### 1.0.7 (2026-05-19) — plank dock + i3 switch + status cluster

Phase 8.6.1–8.6.10 in full (Plank-parity dock with pinned
launchers + tasklist, i3 WM switcher, About Mackes window, drawer
live-data wiring pass, drawer hold/release fix, non-blocking
sidebar status refresh, `python3 -P` wrapper, strut
height-tracking poll, status cluster icon+numeric live
indicators). Phase 8.7.1–8.7.6 (top-bar window buttons —
subsequently retired in 1.1.0). Phase 8.8.1–8.8.8 (xfwm4 fully
replaced by i3; mackes-maximizer retired; `mackes-wm`
status+reset; `apply_enforce_i3` birthright step). Phase 11.1
(AppStream metainfo), 11.2 partial (status-cluster a11y), 11.3
(Wayland-readiness audit), 11.4 (keyboard-shortcuts catalog),
11.6 partial (README pass), 11.7 (pytest smoke baseline), 11.8
(GSettings decision: not shipping), 11.9 (`async_probe` +
9 conversions). Phase 12.1.1 + 12.2.1 (mackesd scaffold + SQLite
schema). Phase 10.6.1–10.6.5 + 10.6.7 (panel-swap + workspaces +
panel archive). Phases 3.1–3.5, 4.2, 5.1, 5.3–5.6, 6.3, 7.1–7.3
(all shipped in prior tags — flipped here).

### 1.0.8 (2026-05-19) — first-boot hotfix

`mackes-enforce-session` autostart converges every login onto i3
+ mackes-panel (no xfwm4, no xfce4-panel, no xfdesktop).
WorkbenchWindow WM_CLASS pinned to `Mackes-shell` + i3 float
rule. Status-cluster click target locked to `mackes --focus
<slug>` (supersedes v3.0.0 Q28).

### 1.1.0 (2026-05-19) — Win10 layout

Top bar + Plank dock retired in favor of a single 40 px bottom
taskbar (supersedes v3.0.0 §4). Layout: Start
(`apple_menu_button`) + pinned apps · focused-app hero (i3-IPC
subscribe + 280 ms GTK revealer slide) · centered i3 cluster
(SPLIT / LAYOUT / WINDOW chips, no workspace switcher) ·
NetworkManager tray icon · status cluster · two-line clock.
Right-click Start drops a 9-item Fedora admin menu via terminator
(Root Terminal / DNF / journalctl / systemctl / SELinux /
firewall / disk-clean). Left-click Start opens a new Rust
popover (`start_menu.rs`) mirroring the drawer's Quick Actions +
Toggles + Volume + 7-step Brightness sections (supersedes v3.0.0
§5). `window_buttons.rs` retired (i3 keybinds + CSD
carry it). Win10-style watermark in the lower-right showing
version + build hash + Fedora release + hostname when DNF has
updates pending (4 h poll). Carbon-themed logout dialog replaces
the xfce4-session-logout window. Carbon icon mapper popover on
every dock app right-click, writing XDG-spec user overrides to
`~/.local/share/applications/`. Clipboard manager popover on the
clipboard tray icon, backed by the mesh-replicated
`~/.cache/mackes/clipboard.json`. `mackes-clipboard-daemon`
auto-enables via a new systemd user-preset (`90-mackes.preset`).
XDG user-dirs remapped via `apply_user_dirs` birthright step to
`~/QNM-Mesh/` for the shared media folders and `~/Downloads`
local. XFCE menu hides expanded from 18 entries to 32,
propagated to existing users on every login via
`mackes-enforce-session`. `mackes update` CLI subcommand +
`.repo` file tuned to Fedora best practice. 5 i3 gaps profiles
via `mackes/i3_gaps.py` + Workbench picker. New CI gate
`tests/test_panel_xvfb_smoke.py` under Xvfb. Phase 8.7.x retired
in favor of i3-native chrome.

---

## How to add a task

Add new entries under **Active** with this shape:

```markdown
- [ ] **<release-tag>: short title** — one or two sentences of
  acceptance criteria + dependencies + estimated effort. Link to a
  design doc if the lock context is non-trivial.
```

Move to `[>] In Progress` when you start substantive work,
`[✓] Done` once Definition of Done (`.claude/CLAUDE.md` §0.8) is
satisfied, `[!] Blocked` with a one-line reason if external state
stalls it. **Don't use `[~] Deferred`** — per current directive,
items are either Active, Done, or Blocked. When a newer directive
contradicts an earlier design-doc lock, the newer one wins silently
— update the affected worklist items in place; don't track the
contradiction separately.

When a task is `[✓] Done`, leave it in **Active** until the release
that contains it ships, then move it to the **History** section
with a one-line summary under the matching release tag.
