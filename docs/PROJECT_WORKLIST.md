# Project Worklist — Mackes XFCE Workstation 1.0.0

**Canonical worklist for the v3.0.0 / 1.0.0 rewrite.**
**Status legend:** `[ ] Open` · `[>] In Progress` · `[✓] Done` · `[!] Blocked` · `[~] Deferred`
**Design source:** `docs/design/v3.0.0-mackes-xfce-workstation.md` (50-question lock, 2026-05-18)

Estimated total effort: ~5–6 months. M1 ships the **full v1 vision** in a
single coherent release (Q47 lock — no partial cuts).

**Performance budget (Q41 revised 2026-05-18):**
- Cold start: **< 200 ms** (Xvfb-measured from systemd `Started mackes-panel.service` to first paint)
- Idle CPU: **< 1%** (averaged over 60 s with no input)
- RSS: **≤ 150 MB** (resident memory after 5 minutes runtime, drawer + dock fully populated)

These are CI gates, not aspirations. A PR that regresses any of them is
blocked until fixed. See Phase 9.4 below.

---

## Phase 0 — Foundations (1–2 weeks)

- [✓] **0.1 Rust toolchain in repo** — `rust-toolchain.toml` pins Fedora 44's Rust 1.95.0. Workspace `Cargo.toml` at repo root. `make rust` / `make rust-check` wired. CI's `rust (Fedora 44)` job green on `cargo fmt --check` + `clippy -D warnings` + `cargo check` + `cargo test`. Landed in `440c190`.
- [✓] **0.2 Cargo workspace skeleton** — three crates now: `mackes-panel` (skeleton bin), `mackes-config` (4 tests, TOML schema for panel.toml top_bar/dock/mesh sections), `mackes-mesh-types` (3 tests, MeshResource Peer/MountedShare/Service). 7 unit tests, fmt/clippy/check clean. Landed in `570146e`.
- [✓] **0.3 Build/packaging plumbing** — `cargo build --release --workspace` runs in `%build`, `target/release/mackes-panel` installs to `/usr/bin/mackes-panel`, MANIFEST.in ships Cargo.{toml,lock} + crates/ in the sdist. Verified: RPM build succeeds, ELF binary present. Landed in `e9cfc35`. `Obsoletes: mackes-shell < 3` deferred to 10.1 (the actual rename commit).
- [✓] **0.4 First boot: empty top bar** — 20 px GTK3 ApplicationWindow with Dock type-hint, PatternFly-dark, clean SIGTERM/SIGINT. Landed in `cc5a122`.
- [✓] **0.5 First boot: empty bottom dock** — second Dock-hint window, 80 px, bottom of primary monitor. `FallbackGeometry` factored out, `apply_placeholder_style` reused across surfaces. Landed in `196cbb6`.
- [✓] **0.6 Wallpaper rendering** — Desktop-hint window with scaled wallpaper from state.json (or branding/ fallback). Pure GTK stacking. Release 558 KB / 433 KB stripped. Landed in `9c51124`. **Phase 0 complete.**
- [✓] **0.7 Repair the latent pytest suite uncovered by ci.yml fix** — ci.yml YAML-bug fixed in `d379914`. Then `f96044e` purged stale `mackes.mesh_*` from sys.modules in `conftest.isolated_xdg`, fixed `test_list_presets_ships_five`, and added cairo/textual to CI deps. `8eb3eb7` added a Typelib/namespace filter to `test_every_non_gui_module_imports`. `32cf2f1` dropped the redundant import-smoke shell step. CI run `26052513245` green: ✓ python (F43) · ✓ python (F44) · ✓ rust (F44). First green CI since 0.2.0.

## Phase 1 — Visual chrome (3–4 weeks)

- [✓] **1.1 PatternFly tokens loaded** — panel reads `/usr/share/mackes-shell/data/css/tokens.css` + `mackes.css` at startup via screen-wide `Gtk.CssProvider`, plus an inline backup so the chrome renders on uninstalled trees. Per-window `#mackes-*` IDs reserved.
- [✓] **1.2 Top bar layout slots** — left/center/right slots via `gtk::Box::set_center_widget`. 1 px hairline border at bottom via inline CSS. Slots named `#mackes-top-{left,center,right}` ready for Phase 1.5–1.7 widgets.
- [✓] **1.3 Dock layout slots** — single centered slot `#mackes-dock-strip`. Hairline border at top. Phase 5.1+ populates it.
- [✓] **1.4 Mackes-Carbon icon loader** — `icons::load(name, size_px)` resolves freedesktop names + `-symbolic` variants under `/usr/share/icons/Mackes-Carbon/scalable/{actions,status,devices,places,emblems,categories,mimetypes,apps}/`. Thread-local `HashMap<(name,size), Pixbuf>` cache. 3 unit tests for the name-candidate logic.
- [✓] **1.5 Clock widget (center)** — `top_bar::clock()` returns a `gtk::Label` showing `HH:MM`. First tick is scheduled at the next-minute boundary (wall-clock synced), then every 60 s. Calendar dropdown is deferred to a follow-up sub-step.
- [✓] **1.6 Status cluster (right)** — 6-item horizontal box with Carbon-loaded glyphs for mesh/clipboard/volume/battery/notifications/user. Per-item click handlers stubbed; Phase 4.2 replaces them with the Drawer-open signal.
- [✓] **1.7 Apple-menu button (left)** — Mackes-mark button (`applications-system-symbolic` placeholder) with stub click handler. Phase 3 wires the dropdown.
- [✓] **1.8 Dock module dispatch** — `DockModule` trait with `id / icon_name / tooltip / state / on_click`. `DockState` enum: Idle / Running / Focused / Urgent{unread}. `render_module()` builds the widget tree.
- [✓] **1.9 State indicators on dock icons** — `state_dot()` (1 px under-icon with class muted/accent/alert) + `unread_badge()` (top-right corner number, 99+ cap) per Q16. 2 unit tests cover the state→class + unread-skip-zero mapping.

## Phase 2 — Configuration & mesh sync (2–3 weeks)

- [✓] **2.1 panel.toml schema** — shipped early in Phase 0.2 (commit `570146e`). `crates/mackes-config/` holds `PanelConfig` / `TopBarConfig` / `DockConfig` / `MeshConfig` / `DockItem` with serde + 4 unit tests including unknown-section tolerance.
- [✓] **2.2 Default panel.toml** — `config_store::load_or_default()` reads `XDG_CONFIG_HOME/mackes-panel/panel.toml` (with `$HOME/.config` fallback) and writes the default via `mackes_config::default_config()` on first launch. Malformed TOML is logged + falls back to defaults so the panel always starts. 2 new unit tests (default round-trip via TOML, six-item status cluster).
- [✓] **2.3 inotify-driven hot reload** — `config_store::watch(callback)` attaches a `gio::FileMonitor` (inotify-backed on Linux) and re-parses on `ChangesDoneHint`. Atomic-save patterns (delete + create + done-hint) reload once, not three times. Diff-and-apply against the UI lands in Phase 2.5 once the live `PanelConfig` is held in a stable place.
- [✓] **2.4 QNM-Shared mirror** — `mesh_sync::mirror(src)` copies `panel.toml` to `~/.qnm-sync/mackes-panel/panel.toml`. Content-aware: skips the write when bytes already match, so QNM-Shared inotify doesn't echo. Callers wire it after every save (callable from Phase 2.5+ when the watcher triggers a save).
- [✓] **2.5 Drift detection** — `mesh_sync::compute_drift()` SHA-256-hashes the local mirror and each `peers/<peer>/panel.toml` under the same root. Returns a `DriftSummary` with per-peer `InSync` / `Drifted` / `Missing` / `Unreadable`. Empty mesh → vacuously in-sync. 3 unit tests.
- [ ] **2.6 Look & Feel → Panel → Sync status row** — extend `mackes/workbench/look_and_feel/` to show in-sync / drifted / N keys differ. Click → opens drift inspector (Q22).

## Phase 3 — Apple menu + app discovery (2 weeks)

- [ ] **3.1 .desktop scanner** — enumerate `/usr/share/applications/*.desktop` and `~/.local/share/applications/*.desktop`, parse Name / Exec / Icon / Categories.
- [ ] **3.2 Applications submenu builder** — group .desktop entries by Categories (AudioVideo / Development / Game / Graphics / Internet / Office / Settings / System / Utility), build a fan-out submenu structure.
- [ ] **3.3 Apple-menu chrome** — narrow dropdown that drops down from the Mackes button, themed to match top bar. Renders the static items (About, Settings, etc.) plus the dynamic Recent Items and Applications submenus.
- [ ] **3.4 Recent Items source** — read GTK's `recently-used.xbel` + Mackes-shell-tracked recents; show last 10.
- [ ] **3.5 System action wiring** — Sleep / Restart / Shut Down via `loginctl`, Lock via `loginctl lock-session`, Sign Out via `xfce4-session-logout --logout`. All routed through `mackes.admin_session.AdminSession` for consent.
- [ ] **3.6 Super+Space global hotkey** — XGrabKey on Super+Space → toggles the Apple menu.

## Phase 4 — Notification Drawer integration (2 weeks)

- [ ] **4.1 Drawer IPC** — define a `mackes-drawer` D-Bus interface so the new Rust panel can open/close the existing Python drawer window. (Or: port the drawer to Rust — decide in 4.1a planning task.)
- [ ] **4.2 Status-cluster click → Drawer open** — clicking anywhere in the right-side status cluster fires `Drawer::open` over D-Bus / direct call (Q28).
- [ ] **4.3 Drawer port to mackes-panel module** *(if 4.1a == port)* — bring `mackes/drawer.py` into `crates/mackes-panel/src/modules/drawer/` as Rust, using gtk-rs.
- [ ] **4.4 Quick-toggle behaviors** — Mesh on/off, Bluetooth, Do-Not-Disturb, Caffeine all driven from the drawer's existing Python wiring (or ported in 4.3).

## Phase 5 — Dock behaviors (3–4 weeks)

- [ ] **5.1 Pinned-app launchers** — clicking a pinned launcher launches the `Exec=` line of the underlying `.desktop`. Tracks PID, status changes.
- [ ] **5.2 Running-app detection** — talk to `libwnck` (via wnck-rs binding) to enumerate top-level windows; map back to `.desktop` Icon names.
- [ ] **5.3 Window switching** — clicking a running-app dock entry brings its window to focus; second click hides it (macOS-style toggle).
- [ ] **5.4 Mesh-resource enumeration** — periodically query QNM-Mesh + headscale_list_peers + service catalog; produce list of `MeshResource` items for the dock.
- [ ] **5.5 Mesh-resource interleaving** — `panel.toml`-configured order mixes pinned apps + mesh peers + services into one strip (Q10).
- [ ] **5.6 Peer-click action popover** — Q34's popover: Files / SSH / RDP / VNC / Services / Send file. Wired to existing mesh helpers in `mackes.mesh_vpn` / `mackes.mesh_ssh`.
- [ ] **5.7 Drag-to-pin / drag-to-reorder** — accept .desktop drops from the Apple-menu Applications submenu (Q38). Update `panel.toml` on commit.

## Phase 6 — Window management (2 weeks)

- [ ] **6.1 Super+Tab app switcher** — modal overlay strip with live window thumbnails. Hold Super, tap Tab to cycle. Release Super to switch.
- [ ] **6.2 Exposé grid (F3 / hot-corner)** — fullscreen overlay that arranges every visible window in a non-overlapping tile grid. Click to focus.
- [ ] **6.3 Workspaces disabled** — set xfwm4 to 1 workspace via xfconf at first-launch (Q29).
- [ ] **6.4 Other 6 default hotkeys** — Super+Q quit · Super+W close · Super+L lock · Super+V clipboard · Super+E Thunar · F3 Exposé. All via XGrabKey + backup-on-conflict.

## Phase 7 — Iconography + theming (1–2 weeks)

- [ ] **7.1 App → Carbon icon mapping table** — extend `install-helpers/mackes-carbon.map` (or sibling file) with `.desktop Name → carbon-basename` rows. Curate top ~50 common apps (firefox, thunderbird, code, terminal, …). Generic fallback = `application.svg`.
- [ ] **7.2 Inline Nerd Font glyphs** — in Apple-menu status-line items and Drawer mini-indicators, use Nerd Font (Red Hat Mono Nerd?) where Carbon SVG would be too small (Q32).
- [ ] **7.3 Force monochrome on all dock icons** — even when an app ships a colorful PNG, dock loader maps it via 7.1 table or applies a monochrome-Carbon fallback (Q14).

## Phase 8 — Continuity surfaces (1–2 weeks)

- [ ] **8.1 LightDM greeter look** — write `lightdm-gtk-greeter.conf` overlay + CSS that mirrors the 20 px top bar (Q36).
- [ ] **8.2 Plymouth rebuild** — black background, centered Mackes logo, 20 px progress line at the bottom matching dock position (Q37). Replace `data/plymouth/mackes/`.
- [ ] **8.3 xfdesktop removal** — drop xfdesktop from Recommends, kill its autostart, ensure mackes-panel's wallpaper + root-menu cover everything users used it for (Q39/Q40).
- [ ] **8.4 Root right-click menu** — XGrabButton on the root window, right-click opens a Mackes-themed menu (Change wallpaper / Open mesh share / Send file to peer / Display settings).

## Phase 9 — Test pyramid (continuous; ratchet to green before M1)

- [ ] **9.1 Unit tests** — every pure-logic module (config parsing, mesh-resource scoring, icon lookup, hotkey parser). Target: 80% line coverage.
- [ ] **9.2 GTK widget tests** — gtk-test harness around dock, status cluster, Apple menu, calendar dropdown. Headless via Xvfb in CI.
- [ ] **9.3 E2E tests** — xdotool-driven smoke: launch panel, click Mackes button, navigate Applications submenu, launch Firefox via dock, verify running indicator appears. Runs nightly.
- [ ] **9.4 Performance benchmarks** — measure RSS / cold-start / idle CPU on every CI run; gate at **< 200 ms start, < 1% idle, ≤ 150 MB RSS** (Q41 revised). PRs that regress any metric fail CI.

## Phase 10 — Migration + cutover (2 weeks)

- [ ] **10.1 RPM rename** — change package name to `mackes-xfce-workstation`, add `Obsoletes: mackes-shell < 3.0` so 2.x dnf upgrades replace cleanly (Q49).
- [ ] **10.2 First-launch wizard** — detect `~/.config/mackes-shell/` leftovers from 2.x; import preset + active wallpaper + pinned apps into `~/.config/mackes-panel/panel.toml`. Show what's being migrated.
- [ ] **10.3 Brand surfacing** — About dialog text, `.desktop` Name field, greeter banner, Plymouth header all say "Mackes XFCE Workstation."
- [ ] **10.4 CHANGELOG 1.0.0 section** — write the user-visible summary referencing the design doc.
- [ ] **10.5 Cut release 1.0.0** — follow the standard cut-release flow (CLAUDE.md §0.6) but with renamed RPM and version reset.

### 10.6 — Birthright removal sequence (replaces incumbent panel + desktop)

Per Q2 / Q5 / Q29 / Q39 we replace xfce4-panel, xfdesktop, the
Whisker-menu plugin, and the legacy mackes-shell Python entry points
with the unified mackes-panel binary. Order matters — a peer can't
lose its panel before the replacement is running. Each substep is a
new birthright step in `mackes.birthright` (placed after the existing
14 v1.x steps so legacy installs still wash through them cleanly):

- [ ] **10.6.1 Install + start mackes-panel** (first new birthright step, runs *before* any removal). `systemctl --user enable --now mackes-panel.service`. **Gate:** `loginctl show-session -p Type` returns x11 AND `pidof mackes-panel` non-empty AND `xprop -root _NET_WORKAREA` reflects the strut. If the gate fails, abort the whole removal sequence with a recovery hint.
- [ ] **10.6.2 Stop + disable xfce4-panel** (only after 10.6.1 gate passes). `xfce4-panel --quit` then `systemctl --user mask xfce4-panel.service` (XFCE doesn't ship one by default — also drop `~/.config/autostart/xfce4-panel.desktop` if present, and any per-session autostart). **Why before uninstall:** stopping cleanly preserves the user's panel layout snapshot for the migration wizard to read in 10.2.
- [ ] **10.6.3 Stop + disable xfdesktop** (after 10.6.2). `xfdesktop --quit`; remove autostart entry. mackes-panel already owns wallpaper rendering by this point (Phase 0.6) so the wallpaper survives the swap.
- [ ] **10.6.4 Unregister the Whisker-menu launcher binding** (after 10.6.3). The Super-key xfconf binding gets swapped from `xfce4-popup-whiskermenu` to `mackes-panel --apple-menu`. Backup any conflicting bindings to `~/.config/mackes-panel/keybindings.backup.toml` (Q35).
- [ ] **10.6.5 Remove xfwm4 workspaces** (after 10.6.4). `xfconf-query --channel xfwm4 --property /general/workspace_count --set 1` (Q29). Quiet, no UX change required.
- [ ] **10.6.6 Uninstall the now-orphaned packages** (final removal step, only after 10.6.1–10.6.5 succeed). Single dnf call: `dnf remove -y xfce4-panel xfdesktop xfce4-whiskermenu-plugin xfce4-docklike-plugin xfce4-pulseaudio-plugin xfce4-power-manager-plugin`. Side effect: the legacy mackes-launcher / mackes-clipboard / mackes-drawer C plugin RPMs (which BuildRequire xfce4-panel-devel) are obsoleted by the renamed mackes-xfce-workstation RPM in 10.1.
- [ ] **10.6.7 Clean leftover xfce4-panel-profiles snapshots** (after 10.6.6). Remove `/usr/share/xfce4-panel-profiles/layouts/` we shipped in 2.x; archive the user's `~/.config/xfce4/panel/` to `~/.config/mackes-panel/legacy-xfce-panel/` for diagnostics. Surface in the first-launch wizard's "what was migrated" summary.
- [ ] **10.6.8 Rollback path** — every removal step writes a `~/.config/mackes-panel/rollback/<step>.json` with the previous state. If `mackes-panel` segfaults or the daemon-stop wedges, `mackes-panel --recover` reads the most-recent rollback and reverses everything in 10.6.1–10.6.6 (re-install xfce4-panel + xfdesktop, restore layout snapshot, re-enable Whisker hotkey). Rollback paths land alongside each forward step, not as one big final task.

---

## Tracking

This worklist is the canonical source for v3.0.0 / 1.0.0 work, per
[mackes-worklist-management](.claude/skills/mackes-worklist-management/SKILL.md).
Mark items `[>] In Progress` before starting; `[✓] Done` only when every
gate in CLAUDE.md §0.8 (committed · pushed · RPM builds · imports clean ·
CHANGELOG updated) is satisfied.
