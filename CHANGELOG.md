# Changelog

All notable user-facing and architectural changes. The current line is
unreleased; tag versions get a date when they ship.

## 3.0.1 — KDC2 native re-implementation + v3 cut (2026-05-22)

(Tagged v3.0.1 rather than v3.0.0 — the v3.0.0 git tag was
reserved in 2024 against an earlier rename plan and points at
a pre-MDE commit. Functionally this is the v3.0 release.)


The headline change: MDE replaces its v13 wrapper of upstream
`kdeconnectd` with a built-in native KDE Connect host (the
"KDC2" re-implementation). The Workbench grows a Connect peer
card; phone notifications flow through mako via the
`dev.mackes.MDE.Connect` D-Bus surface and the Iced
notifications applet badges them with the phone glyph.

### Breaking changes

* **Phones must be re-paired exactly once.** The pair-store
  format moved from `~/.config/kdeconnect/` to
  `~/.config/mde/connect/` and the handshake keypair is
  generated fresh on first launch. The wizard surfaces a
  one-time card on the v2.0.x → v3.0.0 upgrade explaining
  this; re-pair each phone from **Workbench → Connect** after
  finishing setup.
* **`kdeconnectd` is no longer required.** The native
  re-implementation runs in-process inside `mackesd` and
  binds UDP/1716 + the D-Bus name (`dev.mackes.MDE.Connect`).
  The upstream `kdeconnectd`, `kdeconnect-cli`, and
  `kdeconnect-indicator` packages are **Obsoletes:**d on
  upgrade and **Conflicts:** on co-install. Operators that
  relied on `kdeconnect-cli` should script against the
  `dev.mackes.MDE.Connect1` D-Bus interface instead
  (`RingDevice` / `SendSms` / `SendClipboard` / `SendFile` /
  `PairDevice` / `UnpairDevice`).
* **`org.mackes.*` D-Bus aliases removed.** The v2.0.0
  one-release back-compat window for the legacy bus names
  closed. Only `dev.mackes.MDE.*` ships from v3.0 onward.
* **Qt-free dep closure.** With the wrapper gone, MDE pulls
  zero `libQt*` / `libKF*` / `PyQt*` / `PySide*` deps. A
  spec `%check` guard fails the build if any sneak in.

### KDC2 platform

* **Pure protocol crate `mde-kdc-proto`** — wire (Packet,
  codec, framing), discovery (Announce + UDP/1716 encoder +
  mDNS TXT encoder + DiscoveryRegistry with source-address
  cache), crypto (KeyStore trait + RingKeyStore with
  RSA-2048 PKCS#1v15/SHA-256 + AES-256-GCM session AEAD),
  plugins (ten typed packet bodies: Battery / Clipboard /
  Notification / Share / SMS / Ping / FindMyPhone / MPRIS /
  Telephony / RunCommand). 120+ unit tests including
  libFuzzer corpus seed on the codec.
* **Host crate `mde-kdc`** — first-launch identity
  generation (PKCS#8 RSA-2048), file-backed pairing store
  with interior mutability, live TLS connect via
  `tokio_rustls` + cert-fingerprint pinning, D-Bus host on
  the user session bus with `ListDevices` / `GetDevice` /
  `PairDevice` / `UnpairDevice` / `RingDevice` / `SendSms`
  / `SendClipboard` / `SendFile` methods + `DeviceAdded` /
  `DeviceRemoved` / `DeviceUpdated` signals + dispatch-time
  policy enforcement (per-device gating via
  `[plugins.<name>] allow_devices`), and host-side
  `UdpBroadcastRunner` + `MdnsRunner` discovery I/O.
* **Transport trait crate `mackes-transport`** —
  `Transport` trait + `MessageClass` + capability model,
  `PeerPath` per-peer router state, scorer
  (`select_best_transport` with flap-penalty cooldown),
  conformance harness.
* **Mesh router worker** in `mackesd` — ticks every 10 s,
  records decision time into the
  `kdc2_router_decision_us` Prometheus histogram
  (100µs..50ms bucket schedule); SLO check asserts
  p50 < 5 ms / p99 < 25 ms. Mesh-shunt fan-out reads
  every neighbor's `phones.json` from QNM-Shared and
  injects synthetic announces so a phone paired with one
  peer becomes reachable from every peer. `PathSwitch`
  audit events distinguish direct-LAN ↔ mesh-shunt ↔
  flap-penalty transitions.
* **Metrics-flush worker** writes
  `/var/lib/node_exporter/textfile_collector/mackesd.prom`
  every 10 s; the `kdc2_router_decision_us` snapshot
  reflects live router observations.

### Packaging

* `Obsoletes: kdeconnect kdeconnectd kdeconnect-cli
  kdeconnect-indicator`,
  `Conflicts: kdeconnect kdeconnect-cli gsconnect`.
* Legacy `crates/mackes-kdc/` (296 LOC) + `kdc_bridge`
  worker (154 LOC) + `data/dbus-1/services/org.mackes.*`
  aliases deleted.
* Spec `%check` stanza scans every Rust binary with `ldd`
  + the Python tree with grep for `PyQt[0-9]+` /
  `PySide[0-9]+` — fails the build on any hit.

### Other

* **mde-output-autoscale grows EDID-aware diagonal split.**
  4K outputs with a known physical size now pick 1.5
  (desk-distance monitor) or 2.0 (sofa-distance TV) by
  diagonal — a 27" 4K Acer XB272 and a 40" 4K Vizio V405
  on the same machine pick different scales without
  operator override.
* **mackesd `peer-card --peer <id>`** CLI subcommand wires
  the existing `peer_join::handle_peer_joined` helper for
  operator-driven peer-card spawns.
* **Wizard re-pair card** activates on v2.0.x → v3.0.0
  first boot when `~/.config/kdeconnect/` exists and
  `~/.config/mde/connect/identity.pem` doesn't.
* **`%check` Qt-free dep-closure guard** (KDC2-6.4).

### Worklist closeout

64 KDC2 sub-tasks shipped between 2026-05-19 and
2026-05-22. The remaining open items in
`docs/PROJECT_WORKLIST.md` are:

* 10 KDC2 hardware/operator-verification gates (KDC2-4.4
  Tailscale Transport blocked on infra, KDC2-4.6 Docker
  3-peer integration test, KDC2-7.1-7.7 acceptance gates).
* 4 HW-* Hardware Testing epic items (fresh-install
  bench, upgrade bench, Wayland smoke, Docker peer
  fan-out).
* The 26 v2.2 / v2.1+ scope items (CB-1.x retirements,
  UX-13-23, BR-*, UX-*.a, 0.7, C.11, 12.18) are tracked
  as post-v3.0 follow-ups with explicit "needs X before
  it can ship" notes; the v3.0 cut intentionally does not
  block on them.

## 2.0.3 — Operator-verification hotfix bundle (2026-05-22)

Hotfix release driven by a fresh v2.0.2 bench install on a
laptop + 4K-TV dual-monitor rig. Every fix landed at source after
direct operator surfacing of the symptom; the seven discrete
defects + their root causes are captured below.

* **Sway config parse errors + duplicate bindings** —
  `data/sway/config` shipped with `bindsym $mod+Shift+r restart`,
  which is an i3-only command. Sway has no `restart`, only
  `reload`. Sway fired swaynag on every login. Five additional
  bindings (`$mod+q/w/e/l/space`) were defined in both the main
  config and `config.d/mackes-defaults.conf`, generating duplicate-
  binding warnings. Removed the conflicting main-config bindings
  (mackes-defaults wins) and added arrow-key navigation aliases
  (`$mod+arrows`) to replace the focus-right binding that mackes-
  defaults repurposes for `loginctl lock-session`. Also wired the
  panel into the session — added an `exec mde-panel` autostart
  line so the panel comes up on login.

* **mde-panel `app_id` empty under Wayland** — The Iced 0.13
  application builder does NOT inherit `app_id` from
  `iced::Settings.id` on Linux — only from
  `window::Settings.platform_specific.application_id`. mde-panel
  called `.window_size(...)` instead of `.window(window::Settings
  { platform_specific: ... })`, so the xdg-shell `app_id` field
  was never written. Sway's `for_window [app_id="..."]` rule
  therefore never matched and the panel landed as a tiled grey
  strip across the middle of the screen. Fixed by introducing
  `APP_ID = "shell.mackes.Panel"` (freedesktop-style reverse-DNS,
  matches the planned `.desktop` basename) and wiring it into
  `window::settings::PlatformSpecific::application_id` via the
  full `window::Settings` literal. New unit test locks the
  constant against the sway config rule so future renames touch
  both files. Title-match fallback rule kept around for v2.0.2
  binaries on bench machines that haven't upgraded yet.

* **Obsolete v1.x qnm-daemon.service in crash-loop** —
  `mde-migrate-from-1x` only migrated config/cache/state
  directories. It left obsolete v1.x systemd user units
  (`qnm-daemon.service`) in place, which then crash-looped every
  3 seconds because the binary they reference
  (`~/.local/bin/qnm-daemon`) was removed from v2.0.0. The bench
  machine recorded 290+ restarts within minutes of login. Added
  `OBSOLETE_USER_UNITS` list + `disable_obsolete_unit()` helper
  that disables --now via systemctl and deletes the unit file.
  `qnm-daemon.service` is the first entry; more legacy units can
  be added as they're discovered.

* **dunst → mako (Wayland-native notifications)** —
  `dunst.service` ships as a D-Bus activated unit owning
  `org.freedesktop.Notifications` but dunst is X11-only and
  crashed on every Wayland login. Phase 1:
  `install-helpers/bench-bootstrap.sh` lands as a reversible
  operator-run helper that `dnf install`s mako (+ Wayland debug
  tools), masks `dunst.service`, and enables `mako.service` so it
  owns the notification name on next login. Phase 2:
  `packaging/fedora/mackes-shell.spec` gains `Requires: mako` +
  `Conflicts: dunst` so fresh installs auto-converge.

* **Dual-monitor default scaling** — Bench rig was eDP-1 1366×768
  + DP-2 3840×2160 both at `scale=1.0`. UI elements on the 4K TV
  were unreadable across a living-room viewing distance. Added
  `bin/mde-output-autoscale`: width-based heuristic (4K → 2.0,
  2K → 1.5, ≤1080p → 1.0) applied via `swaymsg output ... scale
  ...` at every session start. `exec_always` in `data/sway/config`
  so display hotplug triggers a re-pick. Operator overrides
  (current scale ≠ 1.0) are sacred — the helper skips them.

* **Right-click admin menu pkexec migration** — `mackes-panel/
  src/admin_menu.rs` spawned `terminator -x bash -c 'sudo ...'`
  for every privileged action. Under Wayland sessions terminator
  doesn't always inherit a controlling TTY (sway, lightdm,
  mde-session all spawn it without one), so sudo's password
  prompt failed with "a terminal is required to read the
  password" — the user saw most of the right-click items surface
  a sudo error. Switched every elevation call site to
  `pkexec sh -c '<cmd>'` so the polkit GUI auth agent owns the
  prompt. Drive-by cleanups: `systemctl status` + `dnf history
  list` no longer escalate (they don't need root); `sudo -i`
  became `pkexec bash -l`; `sudoedit` became `pkexec nano`. Hard
  regression guard: a test fails CI if any future SECTIONS edit
  reintroduces raw `sudo`. Same `sudo dnf upgrade` → `pkexec
  dnf upgrade` change applied to the watermark's left-click
  handler.

* **Watermark branding refresh + synced build date** — The
  legacy GTK watermark still said "Mackes XFCE Workstation"
  (v1.x project name). Updated to "Mackes Desktop Environment".
  Version line now reads "MDE 2.0.3 (build <hash>) · Built
  <YYYY-MM-DD>". The build-hash and build-date stamps are
  written by the RPM `%install` step to `/usr/share/mde/
  build-{hash,date}` (with SOURCE_DATE_EPOCH support for
  reproducible builds) and read by BOTH watermarks (legacy GTK
  in `mackes-panel` + Iced in `mde-panel`) so they cannot drift
  on which build is reported.

## 2.0.2 — 2.0 Bug Fixes (2026-05-22)

Bug-fix release on the v2.0.x line. Hardens the v2.0.0 monolithic
cut with the UX-3..UX-9 chrome polish work, the PC-2/3/4.a/10/12
peer-card foundations, the BR-0 brand asset pack, the
short-circuit RPM guard, and the first green ci pytest run since
pre-1.1.0. Larger feature work (UX-7.a sweep, UX-8.a SVG bundle,
UX-9.a animation wiring, PC-3.a event-source integration,
PC-5/6/7 online enrichment, CB-1.8 follow-up, 12.18 HTTPS-tunnel)
ships in a later minor.

### Chrome + design system (UX round 1)

- **UX-3 — palette + iced::Theme coherence.** `iced::Theme::Custom("MDE")`
  is derived from `mde_theme::Palette`, so every widget that defers
  to the framework palette renders against the Q-locked indigo
  (`#5b6af5`) accent + Q-locked Apple-charcoal (`#1d1d1f`) background
  instead of Iced's stock dark navy.
- **UX-4 — custom MDE window header bar.** sway tiles Iced apps
  without server-side decorations; the workbench now draws a 48 px
  `palette.surface` header bar with the "MDE" wordmark at
  `TypeRole::Subheading` (14 sp medium) and min / max / close
  controls that hover-tint with the accent (close hover-tints
  danger). `decorations: false` requested so GNOME-shell /
  KDE-on-Wayland sessions also fall back to client-side chrome.
- **UX-5 — sidebar nav polish.** 240 px fixed sidebar, 40 px nav
  rows with a reserved 20 px icon slot, accent stripe + tint +
  accent text on the selected row, surface-2 hover, 1 px hairline
  section dividers with all-caps 11 sp muted labels, focus ring
  on the active row when the sidebar pane holds keyboard focus.
- **UX-6 — panel chrome primitives (Phase 1+2 + 6.a + 6.b).** New
  `crates/mde-workbench/src/panel_chrome.rs` exposes
  `panel_container`, `section_header`, `section_block`,
  `data_row`, `status_badge` (5 severities), `card`, and
  `empty_state` (plus `dialog`, `dialog_backdrop`, `tooltip` from
  UX-9). SPACE_24 outer padding now applies at `App::view()`
  centrally, so every panel inherits it; `Padding::new(0.0)`
  no-op wrappers swept from 32 panels. Six data panels
  (snapshots / inventory / mesh_history / fleet_revisions /
  playbooks / run_history) carry a polished zero-data view.
- **UX-7 — control state primitives (Phase 1).** `controls.rs`
  ships `variant_button(Primary | Secondary | Ghost)`,
  `styled_text_input`, `toggle` pill, `skeleton`, and `spinner`
  with the locked dimensions (36 px button height, RADIUS_MD,
  SPACE_12 padding, 40 % disabled opacity, 2 px focus ring).
  Snapshots panel migrates as the canonical example. The 27-site
  sweep across remaining panels + focus-ring render are
  follow-ups (UX-7.a; focus-ring chains on the UX-PRE Iced
  upgrade).
- **UX-8 — Carbon icon API (v1).** `mde_theme::icons::Icon`
  enum (48 semantic variants) + `mde_icon(icon, size) ->
  ResolvedIcon` resolver paired with Q37-locked size tiers
  (16 / 20 / 24 / 32 / 48 px). Window controls, empty-state
  hero icons, and `icon_for_device_type` peer-card router swap
  in. Real Carbon SVG bundling is UX-8.a follow-up.
- **UX-9 — motion + dialog chrome (Phase 1).** `mde_theme::motion`
  centralizes the locked durations (180 ms panel mount, 2 s
  notification pulse, 120 ms tooltip fade) + dialog tokens
  (480 px max-width, 50 % backdrop). `panel_chrome::dialog`,
  `dialog_backdrop`, `tooltip` widgets ship. Snapshots-restore
  confirm dialog migrates to the new chrome. UX-9.a wires the
  actual animation subscriptions (needs Iced 0.14).

### Peer Connection Card foundations

- **PC-2** — `PeerProbe` schema moved from the PC-1 placeholder
  to its canonical home `crates/mackes-mesh-types/src/peer_probe.rs`
  (re-exported as `mde_mesh_types::peer_probe::*`). Cross-crate
  consumers (mded peer-join worker, future workbench Fleet
  panels) now share one definition.
- **PC-3** — stand-alone peer-join handler
  (`crates/mackesd/src/peer_join.rs`): writes
  `~/.cache/mde/peers/<id>/probe.json`, spawns
  `mde-peer-card --peer <id>` as a detached child, debounces
  re-spawn within a 30 s per-peer window. Event-source
  integration into the mesh / topology layer chains as PC-3.a.
- **PC-4.a** — production `Hwdb` index over
  `/usr/share/hwdata/usb.ids`; `Hwdb::shared()` caches the
  parsed index process-wide via `OnceLock`. `HwdbInfo::from_lookup`
  returns resolved vendor / product names with hex-string
  fallbacks for unknown IDs.
- **PC-10** — `mackes_config::PeerCardConfig.online_enrichment`
  toggle (default `true`); when `false`, PC-5/6/7 enrichment
  short-circuits and the card renders hwdb-only.
- **PC-12** — RPM spec ships `mde-peer-card` at `%{_bindir}/`;
  always spawned on demand by PC-3, no autostart entry.

### Brand assets

- **BR-0** — `assets/brand/` ships wordmark / wordmark-hero /
  monogram / app-icon / greeter-wordmark in both PNG (ChatGPT-
  generated raw originals + flat slots) and baked SVG forms;
  `crates/mde-theme/src/brand.rs` runtime loader resolves
  `BrandSlot` to bytes with `$MDE_BRAND_DIR` override and
  `include_bytes!` fallback. BR-1..BR-5 consumer-side wiring
  scheduled for v2.2.

### Infrastructure

- **Short-circuit RPM guard** — `make rpm` rejects any artifact
  carrying `rpmlib(ShortCircuited) <= 4.9.0-1`. v2.0.1 shipped
  two RPMs with that dep (manual `rpmbuild --short-circuit`
  during iteration); the guard catches it the next time. See
  `.claude/CLAUDE.md §0.6` for the canonical-path note.
- **ci pytest green** — pytest had been red since pre-1.1.0
  (10 failures from `empty_state(... None, None)` keyword-only
  arg mismatches in 4 legacy GTK panels + an order-sensitive
  facade test). Fixed the call sites + switched the facade
  test to `sys.modules` lookup (order-independent). The
  Python ci job can finally land green: 412 passed · 11
  skipped · 0 failed.

### Test counts (all green)

- mde-workbench lib: 479 (was 452 pre-cut)
- mde-theme lib: 83 (was 59)
- mde-peer-card lib: 30 (was 21)
- mackes-config lib: 20 (was 18)
- mackesd peer_join: 8 (new)
- pytest: 412 (was 402, 10 failures gone)

### Known follow-ups (Open in `docs/PROJECT_WORKLIST.md`)

UX-7.a control sweep + focus-ring render · UX-8.a Carbon SVG
bundle · UX-9.a animation wiring · PC-3.a event-source
integration · PC-5/6/7 online enrichment · CB-1.8 follow-up
(10 Network admin Iced panels) · 12.18 HTTPS-tunnel. The
**UX-PRE** Iced 0.13 → 0.14 workspace bump is BLOCKED on
upstream softbuffer 0.4.8 / Rust 1.95 incompatibility; it
gates Phase E.2 layer-shell + the UX-7.a focus-ring render.

## 2.0.1 — Wayland session hotfix (2026-05-21)

The v2.0.0 RPM declared every `mde-*` Rust binary in `%files` but the
`%install` section never copied them out of `target/release/`. The
result was a working RPM-metadata Wayland desktop with **none** of
the binaries that implement it — `/usr/bin/mde-session`,
`/usr/bin/mde-panel`, `/usr/bin/mded`, `/usr/bin/mde-drawer`,
`/usr/bin/mde-wizard`, and 16 applets were all missing. LightDM
silently filtered the MDE session out of its dropdown (TryExec
pointed at the missing `mde-session`), leaving the user with only
the upstream `sway.desktop` entry. v2.0.0 boxes booted into vanilla
sway — visually i3-like but with no MDE panel, no MDE workbench, no
mesh integration.

**Fixes:**
- Spec now copies every workspace binary built by
  `cargo build --release` into `%{buildroot}%{_bindir}/`:
  `mde-panel`, `mde-session`, `mde-logout-dialog`, `mded`,
  `mde-applet-drawer`, `mde-wizard`, plus all 16
  `mde-applet-*` binaries (clipboard / clock / dock / network /
  status-cluster / sway-cluster / volume-osd / brightness-osd / …).
  Each applet install wraps in `[ -f target/release/$applet ]` so a
  partial workspace build doesn't break the spec.
- `mackes/birthright.py` gains step 20 —
  `apply_uninstall_legacy_xsessions()` — that sweeps three known
  orphan xsession `.desktop` files from `/usr/share/xsessions/`
  (`xfce11-i3-plank`, `xfce11`, `mackes`). These were installed by
  pre-2.0 shell scripts and not tracked by RPM, so package
  upgrades couldn't remove them; their broken `TryExec` paths
  confused the LightDM dropdown.
- Wired the new step into `mackes/wizard/pages/apply.py` between
  the existing legacy-XFCE uninstall and the Mesh step.

Verification: 266 unit tests pass / 93 skipped / 0 failed (4 new
tests cover the legacy-xsession sweep — idempotency, partial-set
removal, rm-failure reporting, allow-list audit).

## 2.0.0 — Mackes Desktop Environment (MDE) monolithic cut (2026-05-20)

The v2.0.0 cut commit. Package rename + Wayland-only stack flip
in one coordinated commit so `dnf upgrade` lands every 1.x user
on `mde-2.0.0` in a single transaction.

**Package identity:**
- `Name: mde` (was `mackes-xfce-workstation`).
- `Provides: mackes-shell` + `Provides: mackes-xfce-workstation`
  + `Obsoletes: mackes-shell < 2.0.0` +
  `Obsoletes: mackes-xfce-workstation < 2.0.0` so every 1.x
  install path resolves to `mde-2.0.0`.

**Dependency swap (CB-3.2):**
- Dropped every XFCE Requires: `xfconf`, `xfce4-settings`,
  `xfce4-session`, `xfce4-power-manager`, `terminator`, `i3`,
  `i3status`, `dmenu`, `wmctrl`, `xprop`,
  `xorg-x11-server-utils`, `xdotool`.
- Added hard Wayland Requires: `sway`, `swaylock`, `swayidle`,
  `swaybg`, `foot`, `bemenu`, `brightnessctl`, `pipewire`,
  `wireplumber`, `grim`, `slurp`.
- New Recommends: `cosmic-files`, `yazi`, `kanshi`,
  `wlogout`, `wofi`.
- Dropped BuildRequires: `xfce4-panel-devel`, `libxfce4ui-devel`
  (C panel plugins retired).

**Conflicts block (CB-3.3, Q5 lock):**
- `Conflicts: xfce4-panel < 999`, `xfdesktop < 999`,
  `xfce4-session < 999`, `xfce4-settings < 999`,
  `xfwm4 < 999`, `xfce4-whiskermenu-plugin < 999`,
  `xfce4-docklike-plugin < 999`,
  `xfce4-pulseaudio-plugin < 999`,
  `xfce4-power-manager-plugin < 999`, `i3 < 999`.
- After `dnf install mde`, trying to install any of those
  errors out cleanly ("would break mde") instead of leaving
  the old desktop pieces running alongside MDE.

**XDG autostart cleanup (CB-3.5, H.4):**
- Retired every override: `mackes-panel.desktop`,
  `xfdesktop.desktop`, `mackes-enforce-session.desktop`,
  `mackes-suppress-xfce4-panel.desktop`,
  `kdeconnect-indicator.desktop`. The Wayland session
  orchestrator (`mde-session`) and sway config own
  panel + desktop bring-up natively.

**C panel-plugin trio retired:**
- `mackes-clipboard`, `mackes-launcher`, `mackes-drawer` no
  longer build or ship. Their roles move to native
  `mackes-panel` applets in Phase E.1.x (Iced port).

**Workbench:**
- 21 Iced Workbench panels shipped across the v1.1.x
  partial-progress cuts now compose the v2.0.0 Workbench
  surface (`/usr/bin/mde-workbench`).
- 5 `mded` subcommands shipped: `nodes list`,
  `ansible-history list`, `playbooks {list, run}`,
  `events list`.
- Tests: 164 → 444 unit tests across mde-workbench (+170 %).

**Upgrade path:**
`dnf upgrade` on a 1.x box lands on `mde-2.0.0` via the
Provides/Obsoletes graph. The Conflicts: block evicts any
xfce4-* / xfwm4 / i3 packages the user previously installed
manually; if dnf reports a refusal, the user removes the
named package and retries (the install-helpers/uninstall
flow already supports this).

The repair panel's "Restart mded" + "Re-install MDE launcher"
buttons handle most of the runtime first-boot smoothing.
`mde-firstboot.target` orchestrates the
`mde-migrate-from-1x` + `mde-shell-migrate-v2` oneshots on
first login so xfconf state migrates to MDE settings + sway
config seeds from the shipped template.

## 1.1.4 — Drop all XFCE Obsoletes (dnf5 install fix, take 2) (2026-05-20)

`dnf install mackes-xfce-workstation-1.1.3-1.fc44.x86_64.rpm`
still crashed with:

> terminate called after throwing an instance of 'libdnf5::AssertionError'
> 'implicit_ts_elements.empty()' failed: The rpm transaction
> contains more elements than requested

Even on a small upgrade (1.0.7 → 1.1.3) where dnf5 only had
one explicit upgrade + 5 implicit erases via Obsoletes, libdnf5
≤ 5.2.x trips an internal accounting assertion when the
implicit element set is non-empty during fill().

Fix: dropped all 5 remaining XFCE Obsoletes from the spec
(xfdesktop, xfce4-whiskermenu-plugin, xfce4-docklike-plugin,
xfce4-pulseaudio-plugin, xfce4-power-manager-plugin). The
runtime birthright step `apply_uninstall_legacy_xfce` already
walks the same list with `dnf remove` after install — it was
the real cleanup all along, the spec Obsoletes were
belt-and-suspenders. Now they're actively harmful.

Spec audit test (`test_spec_does_not_obsolete_legacy_xfce_packages`)
flipped to assert the inverse: any of those 6 names in the
Obsoletes block fails.

v2.0.0's monolithic cut uses `Conflicts:` + `Provides:` /
`Obsoletes:` at the package-rename moment, with a different
test surface; the Obsoletes question is revisited there.

## 1.1.3 — Install fix + Iced MDE Workbench preview (2026-05-20)

Install regression — 1.1.0/1.1.1/1.1.2 RPMs failed to install
on a fresh Fedora 44 box with:

> conflicting requests
> mackes-xfce-workstation requires libxfce4panel-2.0.so.4 but
> obsoletes xfce4-panel < 999 (the only provider).

Root cause: the C panel-plugin under
`data/panel-plugins/mackes-clipboard/` links
`libxfce4panel-2.0.so.4`, which only the `xfce4-panel` package
provides. The spec was Obsoleting xfce4-panel at the same time —
which made the package un-installable end-to-end.

Fix: removed `Obsoletes: xfce4-panel < 999` from the spec and
dropped `xfce4-panel` from the `_LEGACY_XFCE_PACKAGES` tuple in
`apply_uninstall_legacy_xfce`. The xfce4-panel binary stays
suppressed via the existing
`/etc/xdg/autostart/mackes-suppress-xfce4-panel.desktop`
override (the runtime behaviour the Obsoletes was reaching
for); only its on-disk library + .desktop files remain so the
linked C plugin keeps its library provider. v2.0.0's monolithic
cut retires the C plugin entirely and can restore the
Obsoletes then.

The other 5 Obsoletes (xfdesktop, xfce4-whiskermenu-plugin,
xfce4-docklike-plugin, xfce4-pulseaudio-plugin,
xfce4-power-manager-plugin) stay — none of those provide
shared libraries we link.

## 1.1.2 — Iced MDE Workbench preview (2026-05-20)

First v2.0.0-line preview shipped inside a v1.x point release.
`mde-workbench` is a new Iced binary that ports an early slice
of the CB-1 Workbench rewrite (`crates/mde-workbench/`,
164 unit tests). The v1.x Python+GTK3 Workbench remains the
default — `mde-workbench` ships alongside as an opt-in
preview so users can exercise the v2.0.0 surfaces before the
monolithic cut.

**What's shipping**

- **Scaffold:** 9-group collapsible sidebar (Dashboard / Apps /
  Devices / Fleet / Look & Feel / Maintain / Network / System
  / Help), breadcrumb + page-title chrome, keyboard nav (Tab
  cycles sidebar↔main, Ctrl+1..9 jumps to group, Escape
  closes detail), `--focus <slug>` deep-link CLI arg.
- **Single-instance D-Bus contract** —
  `dev.mackes.MDE.Shell.Workbench.Focus(slug)` interface on
  the workbench's own bus name (`dev.mackes.MDE.Workbench`);
  a second `mde-workbench --focus <slug>` call routes
  through the live instance instead of opening a duplicate
  window. Replaces the v1.x WM_CLASS-based hack.
- **9 working panels** wired to the unified Backend trait
  (live `dev.mackes.MDE.Settings.Get/Set` via zbus, with
  a DemoBackend swap-in for tests):
  - Look & Feel: themes, fonts, wallpaper.
  - System: session (3 booleans), notifications (DND
    checkbox + placement combo + numeric expire-ms).
  - Devices: power (5 keys: profile combo, lid_action combo,
    two idle-suspend integers, presentation_mode checkbox),
    removable (3 automount booleans).
  - Fleet: settings (key + value_json + peers Push subprocess
    to `mded fleet push-setting`), revisions (list + Rollback
    button per row).
- **Launch surface:** new `mde-workbench.desktop` entry under
  Settings + System categories.

**Out of scope for 1.1.1 (tracked as `[ ] Open` follow-ups in
the worklist):** the remaining ~36 panels across Apps,
Devices (displays / sound / printers), Fleet (inventory /
playbooks / run_history), Look & Feel preview, Maintain,
Network, System (datetime / default_apps / window_manager /
snapshots), plus the Wizard port. Each follow-up names the
backend it needs.

**Other**

- `cargo test -p mde-workbench`: 164 pass.
- Workspace gains `crates/mde-workbench/` with iced 0.13 +
  zbus 5 (tokio) + tokio process + clap deps. The CB-1
  panel modules share `panels/json_helpers.rs` for the
  Settings JSON wire-format encode/decode helpers.

## 2.0.0 — Rebrand to Mackes Desktop Environment (MDE) + Wayland-only Rust DE

**Rebrand:** "Mackes Shell" becomes "Mackes Desktop Environment (MDE)" on
first reference, "MDE" thereafter. RPM package `mackes-shell` →  `mde`;
binaries `mackesd` → `mded`, `mackes-panel` → `mde-panel`, `mackes` →
`mde`. D-Bus surfaces `org.mackes.*` → `dev.mackes.MDE.*`. Config paths
`~/.config/mackes-shell/` → `~/.config/mde/`. Full identifier mapping
ships in `docs/design/v2.0.0-mde-rebrand/identifiers.md`.

**Upgrade path (Phase H):** `dnf upgrade` from any v1.x lands on `mde-2.0.0`
automatically via `Obsoletes: mackes-shell < 2.0.0` + `Provides:
mackes-shell = 2.0.0` in the new spec. `mde-migrate-from-1x` (runs from
mde-session.service the first time it starts) atomically moves
`~/.config/mackes-shell/` → `~/.config/mde/` (and cache + state trees);
`mde-shell-migrate-v2` does the first-boot heavy lift (xfconf channels →
settings table, drop XDG autostart overrides, back up `~/.config/xfce4/`,
seed `~/.config/sway/`). Env-var shim reads `MDE_*` first, falls back to
`MACKES_*` with a one-shot deprecation warning (drops in v2.1). D-Bus
service-file aliases for the v1.x `org.mackes.*` names ship one release
for backward compatibility.

**Architectural shifts:**

- **Unified Rust meta-daemon.** Every long-running v1.x Python daemon
  folds into `mded` as a `Worker` registered with the Phase A.2
  supervisor: `clipboard`, `mdns`, `fs_sync`, `media_sync`,
  `remmina_sync`, `ansible_pull`, `kdc_bridge`, `heartbeat`,
  `notification_relay`, `notifications_server`. `mded serve` is the new
  systemd ExecStart (replaces the v1.x `migrate && status`); the 10
  retired standalone `.service`/`.timer` units leave the spec.
- **Wayland-only (sway).** XFCE + X11 + i3 retired. Layer-shell + Iced +
  libcosmic + smithay-client-toolkit + swayipc-async for the panel +
  applets; new `mde-session` crate orchestrates login + the
  `dev.mackes.MDE.Session` DBus surface. `data/sway/config` ships as a
  drop-in replacement for `data/i3/config` with matching binding names.
- **Native settings layer (`mded_core::settings`).** 29 dot-notated keys
  cover theme / font / display / power / notification / automount /
  wallpaper / keybinds / autostart. Each value routes through GSettings
  or a JSON sidecar under `$XDG_CACHE_HOME/mde/`; the matching applier
  in `crates/mackesd/src/settings/` handles the side effect. The
  `dev.mackes.MDE.Settings` interface exposes `Get / Set / Snapshot /
  Restore / ListKeys + Changed` signal.
- **Fleet config layer.** `DesiredSnapshot.settings_keys` carries
  per-revision (key, value_json) pairs that every peer's reconcile loop
  applies via `settings::apply_all`. Workbench panels Fleet → Push and
  Fleet → Revisions surface the push + rollback paths.
- **Notifications.** `mded` implements `org.freedesktop.Notifications`
  per spec — every libnotify / notify-send / GTK app reaches `mded`
  transparently, retiring `mako` / `fnott` / `xfce4-notifyd`. Cross-peer
  notification relay reads `~/QNM-Shared/<peer>/.qnm-notifications/`
  and persists to the `notifications` table.

**Workbench panels migrated to MDE settings bridge:** Devices→Power,
System→Removable Media, System→Notifications, System→Session,
System→Window Manager. New: Fleet→Push, Fleet→Revisions. Drawer DND +
Caffeine toggles flip the same flag files the notifications_server +
mde-session honor. `mackes/menu_integration.py` retired (XFCE settings
panels no longer installed).

**Spec changes (Phase H):** drops `i3`, `i3-gaps`, `xfwm4`,
`xfce4-session`, `xfce4-power-manager`, `xfce4-notifyd`,
`xfce4-clipman`, `xfsettingsd`, `xfconfd`, `xfconf`, `xfce4-settings`,
`thunar-volman`, `xdotool`, `xprop`, `wmctrl`, `xrandr`, `xclip`. Adds
`sway`, `swayidle`, `swaylock`, `swaynag`, `swaybg`, `foot`,
`wl-clipboard`, `brightnessctl`, `wlr-randr`, `udisks2`,
`power-profiles-daemon`, `upower`, `pipewire`, `wireplumber`.
Recommends: `cosmic-files`, `yazi`, `kanshi`. Drops thunar.

**Testing:** workspace test count crosses 400 (was 230). Phase 12.11.3
failure-scenario suite (7 named cases) green; Phase 12.11.2
testcontainers integration tests gated under `--features docker-tests`;
Cairo rendering smoke under headless `ImageSurface`. New Phase 9.3
xdotool E2E gates run in CI under Xvfb.

**Installer (CB-5.x rebrand):** `install.sh` banner now reads
"Mackes Desktop Environment (MDE) · installer" with the
"PatternFly 6 · Wayland · Fedora" subtitle (was "Carbon Design System
chrome · XFCE · Fedora"). Hand-off `exec mackes` → `exec mde` (the
bin shim covers the back-compat window per CB-3.7). Wizard / TUI
hints rewritten to `mde --wizard` / `mde --tui`. Headless fallback
(no DISPLAY + no WAYLAND_DISPLAY) now nudges the user toward
picking "Mackes Desktop Environment" from the greeter session menu
on next login — no GPU probing (Q2 hard-switch lock — no detect-
and-pick; the user picks the session entry once and stays there).
Smoke: `bash -n install.sh` green; 7 rebrand assertion tests under
`tests/test_install_sh_rebrand.py`.

### BREAKING CHANGES (Phase H + CB-3.x)

- **XFCE 4 desktop fully removed.** Every `xfce4-*` Requires line
  drops (xfwm4, xfce4-session, xfce4-power-manager, xfce4-notifyd,
  xfce4-clipman, xfsettingsd, xfconfd, xfconf, xfce4-settings) and
  the supporting X11 tooling (xdotool, xprop, wmctrl, xrandr,
  xclip, i3, i3-gaps, thunar, thunar-volman) goes with it. v1.x
  panels that wrote `xfconf` keys now route through
  `mackes.mde_settings_bridge` instead — the bridge maps onto
  gsettings keys + JSON sidecars under `$XDG_CACHE_HOME/mde/`.
- **Wayland-only (hard switch, Q2 lock).** sway is the only
  supported compositor. No "detect-and-pick" between Wayland and
  X11 — the installer informs, the greeter offers the session, the
  user picks once. X11 sessions from v1.x stop launching after
  upgrade (the spec drops the `.desktop` entries).
- **Binary rename `mackes` → `mde`** (and `mackesd` → `mded`,
  `mackes-panel` → `mde-panel`, etc). v1.x names ship as bin-
  shims for one release window (per CB-3.7) so existing scripts
  + bookmarks keep working; the shims will land their deprecation
  warning at v2.1 cut and the names disappear at v2.2.
- **DBus surface rename `org.mackes.*` → `dev.mackes.MDE.*`.** One
  release of alias `.service` files keeps clients of the v1.x
  names working transparently.
- **Config path move `~/.config/mackes-shell/` → `~/.config/mde/`.**
  Atomic migration runs on first launch of `mde-session.service`
  via the new `mde-migrate-from-1x` helper (cache + state trees
  move too).
- **Env-var rename `MACKES_*` → `MDE_*`.** New names take
  precedence; old names still read with a one-shot deprecation
  warning + retire at v2.1.
- **DNF upgrade UX (hard switch).** `dnf upgrade` from any v1.x
  ships `mde-2.0.0` automatically via `Obsoletes: mackes-shell
  < 2.0.0`. The transition is one-way — the v1.x package is no
  longer in the repo. Reverting requires a snapshot rollback
  (via `mde recover --latest` if a snapshot was taken
  pre-upgrade).

## 1.1.0 — Win10 layout (2026-05-19)

Visual reskin of the panel chrome from a 20 px top bar + 80 px
Plank-parity dock into a single 40 px Win10-style taskbar at the
bottom. Same content sources (panel.toml, status_cluster probes,
desktop_files scan, weather popover, recents catalog) — the actual
behavior changes are right-click admin menu, focused-app hero,
desktop watermark, and the new XDG / clipboard / update plumbing.

### Panel chrome

- **Single 40 px bottom taskbar.** Layout left → center → right:
  Start (`apple_menu_button`, lit-amber when open) + pinned-apps
  strip; centered i3 cluster (SPLIT / LAYOUT / WINDOW chips, no
  workspace switcher); status cluster + two-line clock. The prior
  top bar + Plank dock builders stay in-tree as `#[allow(dead_code)]`
  for one release cycle.
- **Right-click Start: 9-item Fedora admin menu (Q15/Q16).** Sections
  — Shells / Packages / Services / Security / Storage. Each item
  launches `terminator -x bash -c '<cmd>; bash'` (keeps the shell
  open after the command finishes per the terminator deprecation of
  `--hold`). Tooltips show the literal command + a sudo-cache hint
  (`sudo -nv` exit code) so the user knows whether the action will
  prompt.
- **`window_buttons.rs` retired (Q11/Q12).** i3 keybindings (Mod+q
  close / Mod+f fullscreen / Mod+space float) + each app's own CSD
  buttons carry the UX. Apps without CSD use the keybindings.

### Desktop layer

- **Win10-style watermark (Q19–Q21, suggestions #2/#10).** Three
  lines in the lower-right corner (name + `Version 1.1.0 (build
  <git-hash>)` + Fedora release · hostname). Hidden by default;
  becomes visible when `dnf check-update` reports pending updates
  (poll every 4 h). Version line gains `— N updates available`
  while the count is known and >0. Left-click opens
  `terminator -x bash -c 'sudo dnf upgrade --refresh; bash'`;
  right-click drops a context menu — *Check for updates now* /
  *Hide for this session*.

### Workbench integration

- **`mackes --focus <slug>` second-click toggles closed (suggestion
  #5).** A repeated tray click on the same status-cluster slug
  destroys the workbench rather than opening a second window.
  Implementation in `app.py` + `_active_panel_key` exposed by
  `sidebar_window.py:go_to`.
- **First-time wizard critically reviewed (10 pages → 3).** Welcome
  / Scan / Appearance / Hardware / Network / Summary demoted to
  Workbench panels or dropped. Wizard retains Preset (conditional)
  / Review / Apply (with silent snapshot). Analysis only this
  release — implementation lands in 1.1.1.

### Fedora-native plumbing

- **XFCE menu hides expanded (18 → 32 entries).** Now covers
  xfce4-panel preferences, Whisker, docklike-plugin, xfdesktop,
  xfce4-screensaver, appfinder, xfce4-settings-editor, xfconf-query.
  Propagated to existing users on every login via the
  `mackes-enforce-session` autostart (the 1.0.8 enforcer also gains
  a 5a step that enables Mackes user systemd units idempotently).
- **`90-mackes.preset`.** Fedora systemd-preset that enables the
  Mackes user units (clipboard daemon, gvfsd-mesh, remmina-sync,
  media-sync) for new accounts. Closes the gap that left the mesh
  clipboard daemon never auto-starting on 1.0.x installs.
- **`apply_user_dirs` birthright step.** Rewrites
  `~/.config/user-dirs.dirs` so XDG well-known folders point at
  `~/QNM-Mesh/{Documents,Music,Pictures,Videos}`; Downloads stays
  local at `~/Downloads`; Desktop / Templates / Public Share collapse
  to `$HOME`. Idempotent; backs up the legacy file once on first
  rewrite to `user-dirs.dirs.legacy`.
- **`.repo` file at Fedora best practice.** `repo_gpgcheck=1`,
  `metadata_expire=4h` (matches the watermark poll cadence),
  `clean_requirements_on_remove=True`.
- **`mackes update` CLI subcommand.** Single unified update path
  shared with the watermark + admin menu (`sudo dnf upgrade
  mackes-xfce-workstation --refresh`). Flags: `--check-only` /
  `--refresh|--no-refresh` / `--yes`.
- **AppStream releases.** Both `mackes-shell.metainfo.xml` and
  `shell.mackes.Panel.metainfo.xml` carry `<release>` entries for
  1.0.8 + 1.1.0; both validate clean via `appstreamcli validate`.

### Notes

- **PNG screenshots in metainfo are deferred** — must be captured on
  a real Mackes host (workbench / taskbar / mesh topology) and
  dropped into `branding/screenshots/` before the next release that
  surfaces them in GNOME Software / KDE Discover.
- **Hero animation (i3-msg subscribe + 280 ms slide), Carbon Icon
  Mapper, multi-monitor wallpaper, PulseAudio compliance, and the
  full clipboard manager popover land in subsequent 1.1.x point
  releases** — the design is locked (memory:
  `project_1_1_0_win10_layout`), the implementation is sequenced
  but not in this tag.

## 1.0.8 — First-boot hotfix: i3 + mackes-panel takeover on every login, Workbench geometry, status-cluster opens Workbench (2026-05-19)

Three bugs reported after installing 1.0.7 + rebooting on a stock Fedora
44 XFCE session: xfwm4 and xfce4-panel still started (Failsafe template
hadn't been overridden), the Workbench window was being tiled
full-screen by i3 (no `for_window` rule matched it), and the top-right
status-cluster icons opened the drawer instead of the Workbench.

### Window manager / panel takeover

- **`mackes-enforce-session` (new XDG autostart).** A small shell
  script installed at `/usr/bin/mackes-enforce-session` and wired to
  `/etc/xdg/autostart/mackes-enforce-session.desktop`. On every
  login it idempotently runs `i3 --replace` (no-op when i3 is
  already the active WM), kills any `xfce4-panel` / `xfdesktop`
  that `xfce4-session` spawned from its Failsafe client list, and
  re-launches `mackes-panel` if it died. Closes the gap between
  install-and-reboot and the `apply_enforce_i3` /
  `apply_panel_swap` birthright steps, which previously only ran
  when the user opened the setup wizard manually.
- **`mackes-suppress-xfce4-panel.desktop` (new XDG autostart).**
  Belt-and-braces Hidden=true override for the XDG autostart spawn
  path (mirrors the existing `xfdesktop.desktop` override). Doesn't
  conflict with the `xfce4-panel` RPM because it lives at a
  Mackes-prefixed filename.

### Workbench geometry on i3

- **`Mackes-shell` WM_CLASS + i3 float rule.** `WorkbenchWindow`
  now calls `set_wmclass("mackes-shell", "Mackes-shell")` so the
  res_class is stable + predictable. `data/i3/config` grows a
  matching `for_window [class="^Mackes-shell$"] floating enable`
  rule alongside the existing `Mackes-panel` rule. Result: the
  workbench respects `set_default_size(1280x720)` +
  `WindowPosition.CENTER` again. Existing users with a stale
  `~/.config/i3/config` from 1.0.7 should run `mackes-wm reset`
  (or delete the file and re-login) to pick up the new rule.

### Top-bar status cluster click-target

- **`mackes --focus <slug>` opens Workbench focused on a panel
  (Q-lock 2026-05-19).** Every status-cluster icon (mesh,
  clipboard, volume, battery, notifications, user) now spawns
  `mackes --focus <slug>` instead of `mackes --drawer
  --drawer-focus <slug>`. The Python side owns the slug → panel
  mapping (mesh → mesh_join, volume → devices, battery / user →
  system, clipboard / notifications → dashboard); unknown slugs
  fall through to the dashboard. The drawer is no longer
  reachable from this cluster — it stays bound to Super+M and the
  drawer applet.

## 1.0.7 — Plank-parity dock, i3, About, drawer wiring, window buttons, xfwm4 retirement, mackesd scaffold (2026-05-19)

Second polish wave on the Mackes XFCE Workstation line. Brings the dock
to feature parity with Plank, adds optional i3 as a tiling alternative
to xfwm4, replaces the popover-only status cluster with live read-only
numeric indicators, and wires every probe in the Python drawer to a
real data source.

### Window management

- **i3 fully replaces xfwm4 (Phase 8.8).** xfwm4 is no longer
  installed by the RPM. The XFCE session host (xfsettingsd,
  xfce4-power-manager, thunar, xfconf) stays unchanged — only
  the WM swaps. `bin/mackes-wm` simplifies to `status` + `reset`;
  legacy `i3` / `xfwm4` verbs print a deprecation note. The
  `apply_enforce_i3` birthright step auto-migrates existing
  1.0.6 installs on first launch (stops + disables
  mackes-maximizer.service, runs `i3 --replace`, seeds
  `~/.config/i3/config` if missing). `mackes-maximizer` (the
  binary, the user-systemd unit, the autostart .desktop) is
  retired — i3 tiles natively.
- **Workbench → System → Window Manager simplified.** Drops
  the WM-toggle row; renders only the i3 layout-preset grid.
- **Top-bar window-management buttons (Phase 8.7).** Three
  Carbon-symbolic glyphs at the far-right corner of the top bar:
  minimize / maximize / close. Operate the i3 focused window via
  `i3-msg`. 45% greyed-out + no-op click when no window is
  focused. Maximize uses `floating toggle + resize 100 ppt` so
  the panel chrome stays visible (NOT `fullscreen toggle`).
  AT-SPI accessible names + tooltips. 4 unit tests cover the
  JSON scan for i3's focused leaf container.

### Top bar

- **Status cluster shows live numbers.** The six right-side icons
  (mesh / clipboard / volume / battery / notifications / user) now
  render an icon + numeric pair (`🌐 3`, `🔊 75`, `🔋 87`, …) refreshed
  every 2 s. Clicking an item opens the Notification Drawer scrolled
  to the matching section. Probe failure renders `—` with a dimmed
  icon and a tooltip naming the cause (`Mesh: tailscale not running`).
  New module `crates/mackes-panel/src/status_cluster.rs`; replaces the
  1.0.6 review popovers.
- **Top-bar strut tracks realized height.** A 500 ms timer republishes
  `_NET_WM_STRUT_PARTIAL` once the bar's actual height settles past
  the requested 20 px; fixes the few-px occlusion delta on first
  paint under xfwm4 / i3.

### Dock

- **Plank-parity rebuild.** Pinned launchers on the left, a live
  tasklist on the right for every running window that doesn't already
  belong to a pinned launcher. Multi-window launchers show a 1 / 2 /
  3+ tick indicator under the icon. Left-click activates (or launches);
  right-click opens a context menu (Open New / Bring to Front: «title»
  / Close All Windows / Pin to Dock).
- **Polling refresh.** The dock rebuilds both segments every 2 s from
  a single `DockSnapshot` of open windows + `WM_CLASS`. Re-reads
  `panel.toml` per tick so Pin/Unpin actions land in ~2 s without a
  separate file-watch path.

### Window managers

- **i3 as an optional tiling WM.** New `/usr/bin/mackes-wm` shell
  switcher: `mackes-wm i3` runs `i3 --replace`, stops the
  mackes-maximizer service, and seeds `~/.config/i3/config` from the
  shipped `/usr/share/mackes-shell/i3/config` default. `mackes-wm
  xfwm4` swaps back. Workbench → System → Window Manager surfaces an
  active-WM toggle row plus (under i3) an 8-cell layout-preset grid
  (Maximized / Side by Side / Split-in-4 / Master+Stack / Tabbed /
  Stacking / Focus / Floating) driven by `i3-msg`. RPM gains
  `Requires: i3 i3status dmenu`.

### About + drawer

- **About Mackes window.** New `mackes/about.py` opens a scrollable
  window over the bundled `data/ABOUT.txt` (credits + license +
  upstream attributions). Wired to the apple-menu's "About Mackes"
  item and the `mackes --about` CLI flag.
- **Drawer live-data wiring pass.** Replaced every mocked data source
  in `mackes/drawer.py` with real probes: `pactl` (volume),
  `bluetoothctl` (Bluetooth), `xfconf-query notifyd` (do-not-disturb),
  `xfce4-power-manager presentation-mode` (caffeine), `tailscale
  status --json` (mesh + fleet), `who -u` (remote sessions), MPRIS
  DBus (playing media), `/sys/class/power_supply` (battery),
  `/proc/{stat,meminfo,loadavg}` (hardware). Sections that depended
  on subsystems not yet implemented (Drift / Shared storage / Daemons
  grid) were removed rather than left as placeholders.

### Mesh control plane scaffold (Phase 12)

- **`crates/mackesd/` workspace member (Phase 12.1.1).** New Rust
  crate ships two artifacts: the `mackesd` binary (CLI for the
  mesh control plane — currently `migrate` + `status` subcommands)
  and the `mackesd_core` library (in-process read API for the
  panel — no IPC, no networked API per Phase 12.A.3 lock). Builds
  clean against cargo 1.95.0; 4 unit tests cover the SQLite store
  and migration application.
- **SQLite store with WAL + 8-table schema (Phase 12.2).** New
  `crates/mackesd/migrations/0001_init.sql` defines `nodes`,
  `desired_config`, `runtime_state`, `observed_telemetry`,
  `topology_link_health`, `events`, `policies`, `leader_lease`
  with CHECK constraints on the deployment-state machine and
  node roles. `mackesd_core::store::migrate` is idempotent.
- **RPM packaging for `mackesd` (Phase 12.1).** Spec gains the
  binary install line, a hardened `data/systemd/mackesd.service`
  unit, `%pre` creation of the `mackesd` system user/group,
  `%post` `systemctl enable --now`. State directory
  `/var/lib/mackesd` (0750, owned by mackesd:mackesd) created
  automatically by systemd's `StateDirectory=`. The reconcile
  loop subcommand (`mackesd serve`) lands in 12.5; today's unit
  runs `mackesd migrate` on every boot so the store stays current.
- **Connectivity scope (Phase 12.14–12.23, 25-Q survey 2026-05-19).**
  Locked: 16-peer small-business fleet, ~50% LAN/~50% WAN,
  throughput-first routing (NOT LAN-first), self-hosted DERP
  default with public Tailscale DERP as fallback, IPv6 descoped
  to a future phase, < 3 s first-packet SLO, < 10 s roaming
  handoff, no new security or monitoring requirements. Full Q&A
  + per-item evaluation in `docs/design/v12-connectivity-scope.md`.
- **Phase 12.17 + 12.21 + 12.23 — connectivity layer extends.**
  `crates/mackesd/src/stun.rs` ships a real RFC 5389/8489 STUN
  client: pure-fn binding-request encoder, attribute-walking
  binding-response parser that extracts XOR-MAPPED-ADDRESS for
  IPv4 + IPv6, and a tokio `gather_endpoint(server, timeout)`
  that validates the transaction-id echo before trusting the
  reflexive address (13 tests). `lan_discovery` gains
  `should_eager_bootstrap` (Phase 12.21 predicate — fresh + low-
  RTT prewarm decision) and the multicast surface (Phase 12.23 —
  locked group 239.42.7.16, wired-only Q16 guard,
  `open_multicast_listener(iface)` that joins the group via
  tokio). 4 new lan_discovery tests, taking the worker's unit
  count from 16 → 20.
- **Phase 12.14 + 12.15 + 12.22 — connectivity primitives shipped.**
  New worker `crates/mackesd/src/workers/lan_discovery.rs`
  announces `_mackes-peer._udp.local` via `mdns-sd` 0.11 and runs
  a tokio UDP probe loop (9-byte MPRB ping/pong, LE seq). RTT
  samples land in a shared `Registry`. Pure-fn ranking ships:
  `lan_direct_wins(lan_rtt, derp_rtt)` (Q23 throughput-first
  proxy), `ipv6_direct_wins(ipv6_rtt, ipv4_derp_rtt)` (Q12.15
  IPv6-first promotion), and
  `higher_throughput_wins(a_bps, b_bps)` (Q23 bandwidth-wins
  override). 16 unit tests cover encode/decode, registry
  semantics, and the full 4-quadrant truth table for every
  ranker.

### Phase 13 — KDE Connect integration (design lock)

- **Option A locked 2026-05-19 (5-option survey).** Wrap upstream
  `kdeconnectd` + Mackes-themed Workbench GUI over the
  `org.kde.kdeconnect.*` DBus interface + mesh-mDNS bridge as the
  shunt that re-announces remote phones on every peer's local
  LAN. Full 6-section worklist in `PROJECT_WORKLIST.md § Phase 13`.
  Implementation lands in 1.1+.

### Documentation + tests

- **AppStream metainfo refreshed.** `data/applications/mackes-shell.metainfo.xml`
  carries the 1.0.x branding ("Mackes XFCE Workstation"), the panel +
  dock + i3 feature list, and explicit release entries for 1.0.0,
  1.0.6, and 1.0.7. `appstreamcli validate` exits clean.
- **README rebuilt.** Drops the legacy 2.x framing. Adds a "Build from
  source" section listing every dev loop (`make rust`, `cargo run -p
  mackes-panel`, `make test`, `make test-nodeps`, `python3 -P -m
  mackes [--drawer|--about|status]`), with explicit toolchain
  dependencies for Fedora 44+.
- **Keyboard shortcut catalog.** `docs/help/keyboard-shortcuts.md`
  documents every panel-owned, WM-owned, drawer, and CLI mirror
  binding plus the `panel.toml:[keybindings]` override syntax.
- **Wayland readiness audit.** `docs/design/wayland-readiness.md`
  inventories every X11-specific surface (strut, wmctrl, xprop,
  xdotool, `XGrabKey`) with per-feature Wayland replacements
  (layer-shell, foreign-toplevel, idle-notify, global-shortcuts
  portal) and a sequenced port plan.
- **Panel-instantiation smoke test.** New `tests/test_panel_instantiation_smoke.py`
  walks `mackes.workbench.**`, finds every `*Panel(Gtk.Box)` subclass
  (49 discovered), and instantiates each headless under Xvfb with a
  5 s hard timeout per panel. Failures surface main-thread blocking
  bugs as "slow constructor" informational output. Full pytest run
  under Xvfb: 118 passed, 5 skipped.
- **Accessibility names + tooltips.** Apple-menu button, clock
  button, and all 6 status-cluster items expose AT-SPI
  `set_name` + `set_description`. Status cluster announces
  context-aware phrases ("Mesh: 3 peers online", "Notifications: 1
  unread") rather than the generic "button".

### Reliability + performance

- **`async_probe` helper (Phase 11.9).** New
  `mackes.workbench._async.async_probe(probe, on_result, on_error=None)`
  runs a probe function on a daemon thread and marshals the result
  back to the GTK main thread via `GLib.idle_add`. Swallows
  exceptions on both sides so a buggy panel can't corrupt GLib's
  main context. Canonical pattern for the Phase 11.9 reliability
  sweep — every blocking probe in `__init__` now has an idiomatic
  replacement.
- **Four panels stopped blocking the main thread.** FirewallPanel
  used to hang ≥ 5 s waiting on `firewall-cmd --list-all` when
  firewalld was down; MeshVpnPanel blocked 15 s on
  `tailscale_status` + `headscale_list_peers`; MeshSshPanel blocked
  7 s on `headscale_list_peers`; DependenciesPanel blocked on the
  initial `rpm -qa` walk. All four now render a skeleton on
  construct, then fill in via `async_probe`. The Workbench sidebar
  click → first paint is now < 50 ms for every converted panel.
- **`firewall-cmd` timeouts reduced 8 s → 2 s.** Long enough to
  succeed when firewalld is alive, short enough to give up before
  the user notices.
- **Panel-instantiation smoke test refactored** to surface remaining
  slow constructors as informational test output rather than
  failures — keeps the gate green while pointing at the next
  candidates for conversion.

- **Drawer process hold/release.** The GApplication `hold()`s before
  `toggle()` so the process survives past `do_activate`, and
  `release()`s when the drawer hides. Was a hot bug: drawer closed on
  first click because the GApp exited.
- **Sidebar status refresh non-blocking.** First `_refresh_status_bar`
  call now runs on a background thread; previously blocked
  `WorkbenchWindow.__init__` for ~7 s while headscale + fleet + drift
  probes ran synchronously.
- **`python3 -P` mackes wrapper.** RPM-installed `/usr/bin/mackes` now
  invokes `python3 -P -m mackes` so the cwd's `mackes/` subdirectory
  never shadows the installed package. Cold start from
  `~/Desktop/files`: 17 s → 1.5 s.

## 1.0.6 — First-boot panel polish (2026-05-18)

(Patch numbers 1.0.1–1.0.5 were already taken by the legacy Mackes
Shell 2.x train; this is the direct successor to 1.0.0 on the Mackes
XFCE Workstation line.)


User-feedback bundle on the freshly-installed Mackes XFCE Workstation
panel. Five bugs, fixed together because they all surface on first
launch and share build/test gates:

- **Top-bar icons are now visible.** `icons::load()` no longer feeds
  raw `fill="currentColor"` SVGs to gdk-pixbuf — that produced black
  glyphs on a black panel and made the left Mackes button + the right
  status cluster look unwired. The loader now substitutes
  `currentColor` for Carbon text-primary (`#f0f0f0`) before
  rasterizing, so every cached `Pixbuf` is already drawn in the
  panel's foreground color. A panel-scoped block in `data/css/mackes.css`
  forces `window#mackes-top-bar` / `window#mackes-dock` and their
  descendants to the same color so any label/button text follows.
- **Bottom dock auto-sizes and hides when empty.** Fixed
  `DOCK_HEIGHT_PX = 80` reserved a thick strip even with zero items.
  Now the dock strip is built first; if it has no children, the dock
  window never shows. When populated it sizes to `DOCK_ICON_PX + 8 px`
  padding (~30 % slimmer than the prior 80 px — full 50 % reduction
  would require shrinking the locked-by-Q12 48 px icon size).
- **Clock switches to 12-hour and opens a weather panel.** Top-bar
  clock is now `h:MM AM/PM` (`%l:%M %p`, leading space trimmed),
  wrapped in a frameless `gtk::Button`. Click opens a `gtk::Popover`
  rendering current temperature and symbol code fetched from
  `api.met.no/weatherapi/locationforecast/2.0/complete` — the same
  endpoint xfce4-weather-plugin uses. New
  `crates/mackes-panel/src/weather.rs` module; HTTP via the system
  `curl` (no new crate dep) with the descriptive User-Agent met.no
  requires. Default coords are London-as-sentinel until `panel.toml`
  grows a `[weather]` section. 3 unit tests cover the JSON parser
  shape.
- **Status-cluster review popovers.** Each of the 6 right-side
  status buttons (mesh / clipboard / volume / battery /
  notifications / user) now opens an in-process `gtk::Popover` with
  the cluster title + a one-line summary + an "Open in Drawer →"
  button that delegates to `mackes --drawer --drawer-focus <slug>`.
  The user gets immediate visual feedback whether or not the Python
  drawer subprocess is up — addressing the "Unable to open the
  dropdown to review" feedback.
- **Panel + dock publish `_NET_WM_STRUT_PARTIAL`.** New
  `crates/mackes-panel/src/strut.rs` looks up each panel window's
  XID via `xdotool search --name` (already a hard dep from Phase
  5.3's window-switching path) and publishes both
  `_NET_WM_STRUT_PARTIAL` (12-cardinal) and `_NET_WM_STRUT` (legacy
  4-cardinal) via `xprop -id`. Any EWMH-compliant window manager —
  xfwm4, i3, bspwm, awesome, LeftWM — now leaves the panel and dock
  space alone when windows maximize. Workspace-manager swap (five
  alternatives surveyed in the feedback thread) deferred to a future
  phase; this strut fix unblocks the occlusion bug under the current
  xfwm4.

## 1.0.0 — Mackes XFCE Workstation (2026-05-18)

The Mackes Shell line graduates from the 2.x XFCE-control-panel
framing to a unified product: **Mackes XFCE Workstation**. The RPM
renames from `mackes-shell` to `mackes-xfce-workstation` (with
`Obsoletes: mackes-shell < 3.0` so `dnf upgrade` is automatic), and
the desktop ships its own panel, dock, and wallpaper layer written
fresh in Rust. Filesystem paths (`~/.config/mackes-shell/`,
`/usr/share/mackes-shell/`, the `mackes/` Python package) stay
unchanged so 2.x installations carry forward.

Full design lock: `docs/design/v3.0.0-mackes-xfce-workstation.md`
(50-question survey, locked 2026-05-18).

### What's new

**Mackes-Carbon icon theme** — symbolic, single-color icon set
derived from IBM Carbon Design System (Apache 2.0). 2,617 SVGs
across the freedesktop categories with `fill="currentColor"`
injected so GTK and the Mackes panel CSS recolor uniformly. New
default for every preset. App-icon mapping table covers ~45 common
apps (Firefox → earth, Thunar → folder--open, vim → terminal …);
fallthrough is `applications-other-symbolic`.

**mackes-panel** — `/usr/bin/mackes-panel`, a new Rust binary that
renders the top status bar + bottom dock + wallpaper. Replaces
xfdesktop (and via Phase 8.3 autostart, takes over from
xfce4-panel on Mackes sessions).

  - **Top bar (20 px)** — Apple-menu button on the left, HH:MM
    clock in the center (wall-clock synced), 6-glyph status cluster
    on the right (mesh / clipboard / volume / battery / notifications
    / user). Each cluster click opens the v2.2.0 Notification Drawer
    with section focus. PatternFly dark surface, monochrome
    Mackes-Carbon glyphs.
  - **Apple menu** — real `gtk::Menu` dropdown: About / Settings /
    Software Update / Recent Items → / Applications → (categorized
    by `.desktop` Categories into Internet / Multimedia / Graphics
    / Office / Development / Games / System / Utilities / Other) /
    Force Quit / Sleep / Restart / Shut Down / Lock / Sign Out.
    System actions go through `loginctl` and `xfce4-session-logout`.
  - **Bottom dock (80 px)** — primary monitor only. Reads
    `~/.config/mackes-panel/panel.toml`, renders pinned apps + mesh
    resources interleaved per Q10. Clicking a running app raises
    its window via `wmctrl -i -a`; second click minimises with
    `xdotool windowminimize`. Mesh peers expose a six-button action
    popover: Files / SSH / RDP / VNC / Services / Send file.
  - **Wallpaper layer** — third Desktop-hint window owns the root
    background, sourced from `~/.config/mackes-shell/state.json`
    or the branded fallback.

**Config + mesh sync** — `panel.toml` lives in TOML at
`~/.config/mackes-panel/panel.toml`, mesh-replicated whole-file to
`~/.qnm-sync/mackes-panel/panel.toml`, hot-reloaded via inotify
(`gio::FileMonitor`), drift-detected against peers via SHA-256.
Look & Feel → Panel surfaces the sync status.

**Boot-to-desktop continuity** — Plymouth rebuilt (centered logo +
20 px progress strip pinned to the bottom edge), LightDM greeter
mirrors the panel's top bar (`panel-position = top`, `clock-format
= %H:%M`, slim indicator cluster), mackes-panel takes over after
login. Single visual language from power-on through running session.

**Performance** — measured under Xvfb (commit 99e2680):

      cold start  5 ms      (target < 200 ms, 40× under)
      RSS         85 MB     (target ≤ 150 MB,  43% under)
      idle CPU    0.0 %     (target < 1 %,     far under)

    `install-helpers/bench-panel.sh` is the perf gate — runs it,
    returns non-zero on regression.

**Workspaces dropped** — every preset ships `workspace_count: 1`.
Single desktop, Cmd+Tab app-switch model.

### Post-1.0 roadmap

The 50-question lock anticipated more than 1.0 lands in one cut.
The following items are tracked in `docs/PROJECT_WORKLIST.md` and
ship in follow-up minor releases:

- Global hotkey grabs (Super+Space / Super+Tab / Super+L / Super+V
  / Super+E / F3 etc.) via x11rb — the panel currently relies on
  xfconf-bound xfwm4 actions, so most macOS-style shortcuts work
  via xfwm4's own keybinding system; full Mackes-side grabs land
  in 1.1.
- Cmd+Tab app-switcher overlay and Exposé grid (need a window-
  thumbnails overlay layer).
- Notification Drawer port from Python to Rust (currently invoked
  via `mackes --drawer`).
- Full GTK widget + xdotool E2E test pyramid (workspace currently
  has 58 unit tests).
- First-launch migration wizard for 2.x → 1.0 user data.
- Root right-click menu (`Change wallpaper / Open mesh share /
  Send file to peer / Display settings`) — Phase 8.4.

### Migration from 2.x

`dnf upgrade` does the work. The new RPM `Obsoletes: mackes-shell
< 3.0` so the old package is replaced. Existing config in
`~/.config/mackes-shell/` is untouched. `~/.config/xfce4/panel/` is
archived to `~/.config/mackes-panel/legacy-xfce-panel/` on first
run for safekeeping. The birthright apply sequence brings up
mackes-panel, then quits xfce4-panel and xfdesktop, then rebinds
Super-key shortcuts to `mackes-panel --apple-menu`.

Foundation for the v3.0.0 / 1.0.0 rebrand per
`docs/design/v3.0.0-mackes-xfce-workstation.md`. Tracked in
`docs/PROJECT_WORKLIST.md`; currently 29 of 67 worklist items complete.

* **`mackes-panel`** — new Rust binary (`/usr/bin/mackes-panel`) that
  renders the top status bar + bottom dock + wallpaper. Three crates
  in the workspace: `mackes-mesh-types`, `mackes-config`, `mackes-panel`.
  ~2,290 lines of Rust, 38 unit tests, no `unsafe` (forbidden at the
  module level), clippy pedantic+nursery clean.

* **Performance gate measured.** `install-helpers/bench-panel.sh`
  runs the binary under Xvfb and samples `/proc/<pid>/`. First
  measurement (commit `99e2680`):

      cold start  5 ms       (target < 200 ms)
      RSS         85 MB      (target ≤ 150 MB)
      idle CPU    0.0 %      (target < 1 %)

  All three Q41-revised gates pass with significant margin.

* **What runs today.** Wallpaper layer (replaces xfdesktop). Top
  bar with Apple-menu button → real `gtk::Menu` dropdown with
  categorized Applications submenu + working system actions
  (`loginctl suspend|reboot|poweroff|lock-session`). HH:MM clock
  (wall-clock synced). Status cluster opens the existing Python
  Notification Drawer with section focus. Bottom dock reads
  `~/.config/mackes-panel/panel.toml` and renders pinned apps as
  monochrome Carbon glyphs via the new app→Carbon icon mapping.

* **Config persistence.** Panel config lives in TOML, mesh-replicated
  to `~/.qnm-sync/mackes-panel/panel.toml`, hot-reloaded via
  `gio::FileMonitor`, drift-detected per peer via SHA-256.

* **Packaging.** Mackes installs now ship
  `/etc/xdg/autostart/mackes-panel.desktop` (brings up the Rust panel)
  and `/etc/xdg/autostart/xfdesktop.desktop` (overrides upstream
  xfdesktop with `Hidden=true` so it never starts on Mackes).

* **Still gating actual 1.0.0 release** (see worklist Phase 5.2-5.3,
  Phase 6, Phase 4.3, Phase 9.1-9.3, Phase 10): libwnck-driven
  running-app / window switching, global hotkeys, Rust port of the
  Notification Drawer, GTK widget + xdotool E2E test pyramid,
  RPM rename to `mackes-xfce-workstation`, first-launch migration
  wizard.

## 2.3.0 — Mackes-Carbon icon theme (2026-05-18)

* **New default icon theme: `Mackes-Carbon`.** A symbolic, single-color
  icon set derived from the IBM Carbon Design System (Apache 2.0).
  Replaces `Black-Sun` as the default `xsettings/IconThemeName` for
  every preset (#!, mackes, daylight). Black-Sun is still installed —
  switch back in Look & Feel → Appearance.

* **Coverage:** 264 freedesktop standard icon names mapped explicitly
  across actions / apps / categories / devices / emblems / mimetypes /
  places / status — every name mackes-shell's own UI references plus
  the broader freedesktop spec (mail-*, format-*, go-*, view-*,
  weather-*, etc.). 2,526 native Carbon SVGs are also dumped under
  `scalable/apps/` so any Carbon basename works directly as an icon
  name (e.g. `Gtk.Image(icon_name="chart-bar")`).

* **Theming:** every SVG gets `fill="currentColor"` injected on the
  root `<svg>` so GTK's symbolic-icon recoloring and the panel CSS's
  `-gtk-icon-foreground-color` both work uniformly. Dark and light
  desktops both render correctly without separate variant files.

* **Reproducibility:** `install-helpers/build-mackes-carbon.sh` is
  idempotent — fetches Carbon SVGs from `/tmp/carbon-icons` (override
  via `CARBON_SVG_DIR=`), reads the freedesktop → Carbon name map from
  `install-helpers/mackes-carbon.map`, writes the theme tree, the
  `index.theme`, and a NOTICE + LICENSE attributing IBM Carbon. Re-run
  it after editing the map to refresh.

* **Packaging:** `packaging/fedora/mackes-shell.spec` installs the
  theme under `/usr/share/icons/Mackes-Carbon/` and runs
  `gtk-update-icon-cache` in `%post`. `mackes.birthright._VENDORED_THEMES`
  copies the tree alongside Orchis-Dark, Shiki-Statler, and Black-Sun
  during birthright apply. `mackes.birthright_check._check_themes`
  verifies it's installed.

### Also in this cut

* **Fix:** `Fleet → Run history` panel locked the entire app on open.
  `_reset_combo` triggered `changed` on the peer/playbook combos,
  which re-entered `_refresh()`, which re-rebuilt the combos — infinite
  recursion. Reentrancy guard added; the noop `handler_block_by_func(None)`
  stub is removed.

## 2.2.0 — Notification Drawer (2026-05-18)

**Breaking visual change.** Three surfaces are deleted and replaced by a
single XFCE panel applet:

  | Removed                              | Replacement                |
  |--------------------------------------|----------------------------|
  | Conky HUD (mackes/conky_hud.py)      | Notification Drawer        |
  | Tray icon (mackes/tray.py)           | Notification Drawer        |
  | Mini popover (mackes.workbench.popover/) | Notification Drawer    |

### What ships

* **C panel plugin** `mackes-drawer` (data/panel-plugins/mackes-drawer/) —
  external xfce4-panel plugin built against libxfce4panel-2.0. Renders
  a single pill on the panel:

  ```
    ▤ Mon May 18  10:34  ·  ◐ 3  ·  ⚡ 77% ▾
  ```

  Reads display state from `~/.cache/mackes/drawer-state.json`,
  refreshes every 5s. On click → spawns `mackes-shell --drawer`.

* **Python drawer window** (`mackes/drawer.py`) — right-anchored
  POPUP window, 420 px wide, full screen-minus-panel height. Slides
  in from the right with a 3 px accent stripe down the left edge.
  Sections (top to bottom): Header · Quick toggles (Mesh · Bluetooth
  · DND · Caffeine) · Volume + Brightness sliders · Mesh (hub + peer
  list) · Fleet (2×2 node grid) · Services (unread / playing /
  remote counts) · Notifications (list with clear-all) · Battery
  (bar + state) · Hardware (CPU/RAM/load/clock). Closes on Esc,
  focus-out, or re-clicking the panel pill.

* **Live data wiring** — every section reads from the existing
  Mackes modules: `mesh_vpn.tailscale_status()` for the mesh peer
  list, `/proc/stat` + `/proc/meminfo` for CPU/RAM,
  `/sys/class/power_supply` for battery, `~/.cache/mackes/notifications.json`
  for the notification queue, `~/.cache/mackes/fleet.json` for the
  fleet grid.

### Removals

* `mackes/conky_hud.py` — DELETED
* `mackes/tray.py` — DELETED
* `mackes/workbench/popover/` — DELETED (the entire 5-tab popover)
* `data/conky/` — DELETED (config template + cairo Lua stripe)
* `data/applications/mackes-conky.desktop` — DELETED
* `data/applications/mackes-tray.desktop` — DELETED
* `apply_conky()` birthright step — REPLACED with `apply_drawer()`
  (creates `~/.cache/mackes/`, sweeps legacy autostart entries,
  kills any orphan conky process)
* `--popover` CLI flag → `--drawer`
* Super+M hotkey → `mackes --drawer`
* Spec `%files` no longer carries conky / tray .desktop entries
* Spec `%build` adds the new mackes-drawer plugin

### Design source

`docs/design/v2.2.0-notification-drawer/` — Carbon Gray 90 (#262626)
base · 3 px accent stripe · Red Hat Display headings + Red Hat Text
body + JetBrains Mono numerics. Mirrors the prototype in
"Mackes Notification Drawer.html" generated via claude.ai/design.

XFCE conventions honored:

* External panel plugin, not an internal one — runs in its own
  process, can't crash the panel.
* `X-XFCE-API=2.0` in the `.desktop` (the lesson the mackes-clipboard
  plugin learned the hard way in 1.6.2).
* GtkPlug socket protocol (argv[2] = socket id) so xfce4-panel can
  embed the pill. Standalone invocation still works for development.

### Deferred to v2.3.x

* Drift / Shared storage / Daemons sections (the drawer's section
  bodies are stubs that read from cache files; the writers come
  online as the mackes-drift / mackes-stated daemons land).
* Density Tweak (compact / standard / full) — design surface
  exists; implementation lands when the Tweaks panel comes back
  in the v2.3 PF6 rewrite track.
* Accent picker — surfaces through the existing per-preset accent;
  no in-drawer picker until v2.3 Tweaks.

## 2.1.0 — Mesh Media (2026-05-18)

Two GTK-native media clients ship at birthright and auto-configure
against discovered mesh media servers. The user opens Thunar, clicks
**Mackes Media**, and sees one launcher per Airsonic or Jellyfin
server on the mesh — no copy-paste of URLs, no per-machine setup.

### Shipped

* **Clients**: Sublime Music (`com.sublimemusic.SublimeMusic`) for
  Airsonic / Subsonic, and Delfin (`app.drey.Delfin`) for Jellyfin.
  Both installed per-user from Flathub by the new `apply_media_clients()`
  birthright step. Both are GTK-native, MPRIS-aware, and theme cleanly
  with the v2.0 PatternFly tokens.
* **Discovery**: new `mackes/mesh_media.py` exposes `discover()`
  returning a deduped union of:
    - mDNS push (`_subsonic._tcp` / `_jellyfin._tcp`) — sub-second
    - TCP port-probe fallback over every tailscale peer (:4040 / :8096)
      with a 250ms connect timeout per port. Catches stock Airsonic /
      Jellyfin installs that don't publish mDNS.
* **Sync daemon**: new `mackes-media-sync.service` + 60s timer
  (user-level systemd). One cycle:
    1. Run `mesh_media.discover()`
    2. Pull QNM-Shared creds from
       `~/.local/share/mackes/qnm-shared/mackes/media-credentials.json`
       if present (no creds → client surfaces its own login)
    3. Atomically rewrite `~/.config/sublime-music/config.json`
    4. Atomically rewrite `~/.local/share/Delfin/servers.json`
    5. Rebuild the Thunar view + bookmark (next item)
* **Thunar view**: `~/Mackes Media/` directory contains one
  `.desktop` launcher per discovered server. A bookmark line
  `file://~/Mackes Media/  Mackes Media` is appended to
  `~/.config/gtk-3.0/bookmarks`. Stale entries (servers that have
  left the mesh) are reaped on every cycle.
* **Credentials**: locked to the QNM-Shared bucket — one set of
  creds per server, replicated to every mesh peer. New peers get
  access automatically when they join. v1.8.0 onboarding wizards
  will surface "claim this new server" inline; until then it's
  manual via QNM.

### Why "GTK clients via dnf where available" landed as "Flathub-only"

Neither Sublime Music nor Delfin is in Fedora's main repos. The user's
locked decision was "Native dnf packages where available" — which
degrades to Flathub for both (the only practical source). The
birthright step prefers Flatpak over a bespoke RPM build, with
`flatpak install --user` so no root is needed and updates ride the
normal flatpak update cycle.

### Files

| Path | Status |
|---|---|
| `mackes/mesh_media.py` | new |
| `mackes/media_sync_daemon.py` | new |
| `data/systemd/mackes-media-sync.service` | new |
| `data/systemd/mackes-media-sync.timer` | new |
| `mackes/birthright.py` | `apply_media_clients()` step added |
| `mackes/wizard/pages/apply.py` | "Media clients" step wired in |
| `packaging/fedora/mackes-shell.spec` | systemd units + `%files` updated |

### Deferred to v1.8.0 onboarding wizards

The "claim a new mesh media server" flow lives in the onboarding
wizards package (queued for v1.8.0). Until that ships, a newly-
discovered server with no credentials just appears in the Thunar
view; opening it surfaces the client's own login prompt. The user
adds the credential to QNM-Shared via `qnmctl share set
mackes/media-credentials.json` and the next sync cycle picks it up.

## 2.0.0 — PatternFly v6 design system (2026-05-18)

Mackes Shell's visual identity moves from IBM Carbon to PatternFly v6
(Red Hat's design system). This release lands the **design-system
swap** — tokens, typography, surfaces, accents, border radii — across
every panel by re-pointing the existing `.cds-*` selectors at PF
values. The class-name rename to `.pf-*` and module rename
`mackes/carbon/` → `mackes/patternfly/` are deferred to v2.1 so panels
can migrate piecewise without a single landing blast.

### What changed

* **Design tokens** (`data/css/tokens.css`) rewritten against PF v6's
  dark scale: `--pf-t--global--background--color--*` values mapped onto
  the existing `cds_bg_default / cds_bg_layer_0[1-3] / cds_bg_hover /
  cds_bg_active / cds_bg_selected / cds_bg_inverse` tokens. Text,
  border, focus, link, support, and field tokens follow the same map.
* **Accent** default flips from Carbon blue `#0f62fe` to PF6 blue
  `#2b9af3`; per-preset accent overrides still ride on top.
* **Typography** is **Red Hat Display + Red Hat Text + Red Hat Mono**
  (PF v6's official stack). Birthright `apply_fonts()` installs
  `redhat-display-fonts redhat-text-fonts redhat-mono-fonts` instead
  of `ibm-plex-*-fonts`. Spec `Recommends:` updated. Presets and
  LightDM defaults follow. IBM Plex remains a CSS fallback so the UI
  still draws cleanly on hosts that haven't yet run the v2.0
  birthright step.
* **Surface radii** shift from Carbon's flat `border-radius: 0` to
  PF6's `4px`. Buttons, tiles, frames, scrollbar sliders.
* **Type scale** rebalanced for PF v6 (heading-03 = 18px, heading-04 =
  24px, heading-05 = 28px). The `cds-heading-*` selector names stay
  for continuity; only the values shifted.

### Why the locked "full rewrite of every panel" landed as a
### design-system swap

The locked v2.0.0 design called for "Top-to-bottom rebuild of
mackes/workbench/*." In practice, a design-system migration with PF
parity at the token layer achieves the visible outcome (PatternFly
look, Red Hat fonts, PF radii + spacing) without breaking the 153
existing `.cds-*` references across the codebase mid-flight. Each
panel rewrite is now a focused, low-risk v2.x point release rather
than a single 30-panel blast. The v2.0.0 cut delivers the PF identity;
v2.1.0 onward delivers the namespace + per-panel layout refinements.

### Deferred to v2.1+

* Rename `.cds-*` selectors to `.pf-*` across panels (mechanical sed
  cleanup, one panel group at a time).
* Rename `mackes/carbon/` module to `mackes/patternfly/`. The widget
  files are GTK code that doesn't care about the design system; this
  is naming hygiene, not functional change.
* Adaptive light/dark token swap (`data/css/pf-light.css`). PF6 dark
  is the default; light surfaces follow once a real user signal
  asks for it.
* Per-panel layout rewrites against PF6 Page / Sidebar / Toolbar /
  Card patterns. Tracked as v2.1.x panel-by-panel.

## 1.7.0 — Outcome-driven mesh join (2026-05-18)

User-facing focus: the Setup / Join Node workflow was confusing. This
release reshapes it around the outcome ("get me on the mesh") rather
than the role taxonomy ("seed / join / reconfig"). The mesh nav is
collapsed; auto-heal makes most join failures recover transparently;
mDNS makes peers on the same LAN findable without copy-pasting links.

### Mesh join

* The Headscale setup wizard's three-card Seed/Join/Reconfig chooser
  folds into two outcome cards: *Join an existing mesh* / *Host a new
  mesh*. Reconfig was redundant — host_run is idempotent on already-
  provisioned peers.
* New `mackes/mesh_discovery.py` exposes a discovery fallback chain
  (`scan_clipboard` → `scan_mdns` → manual) so the join page can pre-
  fill credentials when possible.
* Joining via clipboard `mackes://` link is auto-detected on entry to
  the Join screen; the entry field is pre-filled and the Continue
  button is the default action so Ctrl+V → Enter just works.
* New `mackes.mesh_vpn.join_with_retry()` wraps the join with a
  progressive 3-attempt auto-heal chain: retry → restart tailscaled
  → flush state file → fail. Ground-truth is `tailscale status`'s
  `Self.Online`, not the rc of `tailscale up`.
* Control nodes publish `_mackes-mesh._tcp` over Avahi when Headscale
  comes up; the file is retracted when Headscale stops. Peers on the
  same LAN can mDNS-discover the control endpoint without sharing a
  link.

### Network nav

* The Network sidebar group collapses from 11 flat items to four:
  Wi-Fi & Ethernet · Mesh · Mesh Remote · Advanced. Mesh Health,
  Mesh Performance, Mesh VPN, Mesh SSH, Mesh Services, Firewall, VPN,
  and QNM all move under the Advanced sub-nav (same lazy-build
  pattern Devices and System use).

### Wizard

* The Appearance step is now read-only — theme / icon / font /
  wallpaper are platform-fixed and apply automatically from the
  preset's declared defaults. Renamed "Appearance & Desktop" so the
  scope is explicit.
* Fixed the Next-button click-through where a user could click
  "Continue ›" mid-install and skip into the summary while the
  installer was still running. The Apply page's worker thread now
  drives an `on_complete` callback that gates Next from
  "Working… / disabled" to "Continue › / enabled."
* Standard Linux dialog keyboard idioms: Escape closes the wizard
  (except mid-install where the page's own Cancel button owns it);
  Next is the default action so Enter advances; tooltips and
  accessible names on Back / Cancel / Next.

### Desktop integration

* New AppStream metainfo (`io.github.matthewmackes.MackesShell`)
  installed to `/usr/share/metainfo/` so Mackes Shell surfaces in
  GNOME Software and KDE Discover.
* `Actions=Wizard;Popover;` on the main `.desktop` exposes the
  existing `--wizard` and `--popover` flags as right-click jump-list
  entries.

### Hygiene

* Dropped the orphan PadOS GTK theme (8 files) and Carbon icon theme
  (2522 SVGs) plus `install-carbon-icons.sh` — all unreferenced after
  the 1.6.6 Orchis-Dark + Black-Sun lock-in. apply_themes() collapses
  to a data-driven `_VENDORED_THEMES` tuple over only the three themes
  we actually ship.

### Deferred to v1.8.0

* Onboarding wizards for third-party services that need operator
  config (Headscale public hostname for WAN-reach, Guacamole admin
  password). The package scaffold is staged.
* QR-scan discovery (needs `zbar-tools` dep + webcam handling).
* DERP rotation between join attempts (Tailscale's own auto-failover
  handles the common case; manual rotation only worth adding once we
  see a confirmed map-update failure).
* Always-on every role on every node — Headscale binary, Tailscale,
  NATS, Avahi tools are all installed at birthright today; the
  remaining locked promise is auto-elect-on-demand (which lands as
  v1.8.0 work, not v1.7.0).

* **Drop orphan PadOS GTK theme and Carbon icon theme.** Slim
  `apply_themes()` to a data-driven `_VENDORED_THEMES` tuple over only
  the three themes we actually ship (Orchis-Dark / Shiki-Statler /
  Black-Sun). Removes 2522 stale SVGs plus
  `install-carbon-icons.sh`.
* **Fix wizard Next button click-through during install.** The Apply
  page's worker thread now drives an `on_complete` callback that gates
  the Next button, so the user can't skip into the summary while
  installation is still running.
* **Make wizard Appearance step static, no user options.** Theme /
  icon / font / wallpaper are platform-fixed and apply automatically.
  The step renders a read-only summary of what's about to install;
  later tuning lives in Look & Feel → Appearance.
* **Simplify Network nav and mesh setup chooser.** Network sidebar
  collapses from 11 flat items to four: Wi-Fi · Mesh · Mesh Remote ·
  Advanced (sub-nav over Mesh Health / Performance / VPN / SSH /
  Services / Firewall / VPN / QNM). Headscale setup wizard's three-card
  Seed/Join/Reconfig chooser folds into two outcome cards: Join an
  existing mesh / Host a new mesh. Pre-fills the join link from the
  clipboard if a `mackes://` URL is already there.
* **AppStream metainfo + Desktop Actions.** Ship
  `io.github.matthewmackes.MackesShell.metainfo.xml` to
  `/usr/share/metainfo/` so Mackes surfaces in GNOME Software / KDE
  Discover. Add `Actions=Wizard;Popover;` to the main `.desktop`,
  exposing the existing `--wizard` and `--popover` flags as jump-list
  entries.
* **Wizard keyboard + accessibility wires.** Escape closes the wizard
  (except mid-install). Next button is the default action so Enter
  advances. Tooltips + accessible names on Back / Cancel / Next.

## 1.6.7 — apply_panel_layout uses xfce4-panel-profiles (2026-05-18)

Every "Plugin (null) could not be loaded" + `g_value_get_int`
assertion crash in the 1.6.x line traced back to writing
`/panels/...` xfconf keys with the wrong GVariant typing. We've
swung between two failed approaches (data-driven snapshot loader,
hand-rolled hardcoded sequence) and both hit the same wall.

This release stops fighting xfce4-panel's xfconf shape and uses
the upstream tool that knows it natively:

* New shipped artifact: `data/panel/xfce4-panel-profile.tar.bz2` —
  a 1.7 KB archive captured via `xfce4-panel-profiles save`. Contains
  the full xfconf dump with the right `uint32`/`GVariant`-array
  typing plus per-launcher `.desktop` RC files.
* `apply_panel_layout` is now a single shell call:
  `xfce4-panel-profiles load <archive>` — handles panel `--quit`,
  xfconf write, RC-file copy, and panel restart internally.
* Spec adds `Requires: xfce4-panel-profiles` (in Fedora main repo).

Re-snapshot the shipped default at any time with
`xfce4-panel-profiles save data/panel/xfce4-panel-profile.tar.bz2`
on a reference machine; commit; ship.

Upstream tool: https://gitlab.xfce.org/apps/xfce4-panel-profiles

## 1.6.6 — Orchis Dark GTK + classic Win2K-style panel layout (2026-05-17)

**Orchis Dark replaces Shiki-Statler as the default GTK theme.**
Vendored from https://github.com/vinceliuice/Orchis-theme (GPL-3.0)
at `data/themes/Orchis-Dark/`. Material-design dark theme covering
gtk-2.0 + gtk-3.0 + gtk-4.0 + xfwm4 + cinnamon + metacity — every
modern GTK app picks it up natively, no GTK3 port needed unlike
Shiki-Statler. Shiki stays bundled as an alternative.

Every preset (hashbang / mackes / daylight) now defaults to:
```
gtk_theme:             "Orchis-Dark"   (was Shiki-Statler)
window_manager_theme:  "Shiki-Statler" (kept — Orchis xfwm is
                                        too rounded for the
                                        classic feel)
icon_theme:            "Black-Sun"     (unchanged)
```

LightDM greeter default `gtk_theme` flips to `Orchis-Dark` too.

**Classic Windows 2000-style xfce4-panel layout.** All Mackes-specific
plugins removed from the wizard's panel-apply step. `apply_panel_layout`
now writes a single bottom panel with the standard XFCE plugin set:

```
plugin-1  applicationsmenu      ("Start" button, left)
plugin-2  separator             (small gap)
plugin-3  tasklist              (window buttons, raised style)
plugin-4  separator-expand      (push systray + clock right)
plugin-5  systray
plugin-6  clock                 (digital %I:%M %p)
```

Position `p=10` (bottom-fixed), size 30 px, icon-size 16 px,
enable-struts true — the classic Win2K bottom-strip feel. No
whiskermenu rebrand, no `mackes-launcher` in the panel, no
`mackes-clipboard` in the panel.

The `mackes-launcher` + `mackes-clipboard` panel plugins are still
installed by the RPM; users who want them on the panel can
right-click → Panel → Add New Items.

## 1.6.5 — GUI refine pass: compact, professional, functional (2026-05-17)

Three-round refinement of the GUI surface:

**Compactness**
- WorkbenchWindow drops the maximize-on-realize default. Opens at
  1280×720 (capped to fit small laptops), centered on the primary
  monitor; the user can still maximize themselves.
- Every workbench panel's outer margins trimmed:
  `set_margin_top(32) → 12`, `set_margin_start(40) → 16` across 25
  files. `_common.section_header()` from 28/8 to 12/4. Net ≈ 10–15
  extra content rows visible at the same window size.
- Left sidebar rail from 256 → 220 px (still room for the longest
  group title at 8pt).

**Professional**
- New high-specificity CSS overrides at the top of
  `carbon-productive.css`:
  - Tighter page-title / page-subtitle / section-title / section-
    description / breadcrumb spacing
  - 7pt breadcrumb (was 8pt) — denser hierarchy
  - Standardised `.mackes-pill-{ok,warn,fail,neutral}` ruleset so
    every panel renders status pills identically: alpha-tinted
    background, 7pt bold, 2px radius, semantic color tokens.

**Functional**
- Removed both Tweaks UIs (full-page System → Tweaks + floating
  gear-drawer overlay; −664 LOC). Per-feature toggles still
  read+written via tweaks.json + the per-module CLIs.
- Preset chip in the header was a no-op after the Tweaks-drawer
  removal. Rewired to `_on_open_wizard` so chip → Setup Wizard
  (preset_pick page) is the canonical preset-swap surface now.
- Dropped the dead `_on_preset_chip` method and the `tweaks`
  legacy-key-map entry. Popover Manage tab loses its Tweaks
  sub-tab; now Fleet / Screens / Boot only.

## 1.6.4 — Tweaks drawer close button + 8pt density (2026-05-17)

**Tweaks drawer close ✕.** The right-side sliding Tweaks overlay had
no obvious dismissal — only re-clicking the gear button (often hidden
underneath the drawer's content on small screens). Added an explicit
✕ button in the drawer's header row that calls `TweaksOverlay.close()`
directly.

**Global 8pt text density.** User-requested compact preset for
high-data-density viewing. New high-specificity override at the top
of `data/css/carbon-productive.css` forces `font-size: 8pt` on every
descendant of `.mackes-app-window`, `.mackes-productive`,
`.mackes-popover`, and `.mackes-tweaks-drawer`. The Carbon Productive
14/12/10 scale rules below still apply for spacing/weight/family —
only the size is forced down.

## 1.6.3 — Hotfix: xfce4-panel crash on wizard apply (2026-05-17)

**`apply_panel_layout` REVERTED to the v1.5.x-style hardcoded
xfconf-query sequence.** The v1.6.2 data-driven snapshot loader was
writing types xfce4-panel 4.20 doesn't accept:

* `/panels` as `array-uint` — xfce4-panel expects `array-int`
* whiskermenu `menu-width` / `menu-height` as `uint` — whisker reads
  them as `int`, triggering `g_value_get_int: assertion
  'G_VALUE_HOLDS_INT (value)' failed` storms
* Snapshot writes ran while xfce4-panel was watching the channel,
  racing on partial state and showing "Plugin "(null)" could not be
  loaded — do you want to remove" dialogs

The reverted function uses the proven v1.5.x sequence (stable for
months) with `mackes-launcher` and `mackes-clipboard` added. Plugin
IDs 101–107: whiskermenu (Mackes-branded) · mackes-launcher (Super+M
popover) · docklike · separator-expand · mackes-clipboard · systray ·
clock (IBM Plex digital). v1.5.1 race-fix preserved: `xfce4-panel
--quit` before any writes; `/panels` + `plugin-ids` written LAST after
every plugin's type has landed.

The snapshot file (`data/panel/xfce4-panel.snapshot.json`) stays in
the tree as a reference / diagnostic artifact; `tools/snapshot-panel.py`
still works for re-capture. `apply_panel_layout` no longer reads from
it.

**Black-Sun `index.theme` fix.** Upstream's `Directories=` line
listed `places/symbolic` but the matching `[places/symbolic]` section
header was missing. Added a minimal stanza
(Context=Places, Size=16, MinSize=8, MaxSize=512, Type=Scalable).
Stops the "Theme directory places/symbolic of theme Black-Sun has no
size field" warning from every GTK app on the desktop.

## 1.6.2 — Slide-out popover + mesh perf overhaul + new default themes (2026-05-17)

**Final-pass tasklist clear-out — panel button + tray + GTK3 port.**

* **Shiki-Statler GTK3 port.** `data/themes/Shiki-Statler/gtk-3.0/
  gtk.css` ports the canonical Shiki-Colors palette (`fg #101010`,
  `bg #D8D8D8`, `base #F5F5F5`, `selected_bg #808080`, dark
  `headerbar #2A2A2A`) to GTK3+ semantic tokens. GTK3 apps —
  including Mackes Shell itself — now pick up the same look the
  GTK2 + xfwm4 paths already did. ~200 LOC of CSS covering buttons,
  entries, lists, headerbars, menus, tooltips, notebooks,
  scrollbars, progress, switches, checks.

* **`data/panel-plugins/mackes-launcher/`** — new external xfce4-panel
  plugin (mirrors the mackes-clipboard layout). Click → spawns
  `mackes --popover`. Built C source, .desktop with `X-XFCE-API=2.0`,
  Makefile that compiles with the same pkg-config + CFLAGS the
  existing clipboard plugin uses. RPM builds + installs it under
  `/usr/lib64/xfce4/panel/plugins/mackes-launcher`.

* **`mackes/tray.py`** — Gtk.StatusIcon tray + context menu (Open
  popover, Open full window, Mesh Health, Quit). Tooltip + state
  refresh every 30 s from `mesh.health()`. Autostart shim at
  `data/applications/mackes-tray.desktop` so it launches on login.



**New default themes — Black-Sun (icons) + Shiki-Statler (GTK/xfwm).**

* `data/icons/Black-Sun/` — vendored from
  https://github.com/SethStormR/Black-Sun (GPL-3.0). 2,524 SVGs.
  Inherits from Papirus-Dark / breeze-dark / Cosmic / Adwaita /
  hicolor. Set as `icon_theme` in every preset (hashbang / mackes /
  daylight) and as the LightDM greeter `icon-theme-name` default.
* `data/themes/Shiki-Statler/` — vendored from
  https://sourceforge.net/projects/archbangretro/files/Shiki-Statler.tar.xz
  (GPL, md5 verified). Set as `gtk_theme` + `window_manager_theme`
  in every preset and as the LightDM greeter default. **Limitation:**
  the upstream ships only `gtk-2.0/` + `xfwm4/` + `openbox-3/`;
  GTK3+ apps (Mackes Shell itself, modern XFCE apps) fall back to
  their inherited theme. A GTK3 port of Shiki-Statler is captured
  as a follow-up.
* `apply_themes` extended to deploy both vendored themes to
  `/usr/share/{themes,icons}/`. RPM ships both. `gtk-update-icon-cache`
  refreshes the Black-Sun cache at `%post`.



**GUI redesign v1 — slide-out popover.** Locked via 10-question
survey 2026-05-17. Mackes Shell now ships in two shapes:

* Full window (current behaviour, unchanged) — `mackes`
* New 420×600 slide-out popover — `mackes --popover` (and the new
  Super+M hotkey + panel-plugin button + tray icon ship in a
  follow-up commit)

Popover (`mackes.workbench.popover.*`):
  * `window.PopoverWindow` — Gtk.Window type=POPUP, undecorated,
    keep-above, anchored top-right, dismiss on focus-out or Esc.
  * Tab bar: **Glance · Mesh · Tools · Manage · Help** with Hack
    Nerd Font glyphs above each label.
  * `glance.GlanceTab` — live mesh state pill, top-6 peers
    GtkTreeView (with Wake action on offline rows), last 5 mackes.log
    lines, system pulse (CPU/RAM/services). 5-second refresh while
    visible.
  * `mesh_tab.MeshTab` — sub-tabs for Get Online / Health / Perf /
    SSH (Q10 lock: merge close-cousin panels).
  * `tools_tab.ToolsTab` — Apps / Sources / Update / Fonts.
  * `manage_tab.ManageTab` — Fleet / Tweaks / Screens / Boot.
  * `help_tab.HelpTab` — quick-link buttons to full-window views
    (Wizard, Logs, full Mackes Shell, Help docs).

`data/css/carbon-productive.css` — Carbon's Productive type scale
(14/18 body, 12/16 helper, 10/12 caption) applied via the
`.mackes-productive` root class. Replaces the heavier Expressive
scale on the popover. Glance/Mesh/Tools/Manage all opt in.

**xfce4-panel snapshot rebrand.** `data/panel/xfce4-panel.snapshot.json`
rebuilt as a clean Mackes-branded default: single panel along the
top, 7 plugins (whiskermenu / mackes-launcher / docklike /
separator-expand / mackes-clipboard / systray / clock). Whisker
button-title="Mackes", button-icon=mackes-shell, favorites curated
around the Mackes-essentials set. Drops the dual-panel + orphan
101-105 entries the original capture inherited.



**Mesh perf round verification — tests + spec wiring.**

* `tests/test_mesh_perf.py`, `test_mesh_wol.py`, `test_mesh_metrics.py`
  cover the new module surfaces — tweak round-trip, MAC parsing, WoL
  magic-packet bytes (102-byte packet to UDP/9 + UDP/7), Prometheus
  metric parsing.
* Spec `Recommends:` adds `python3-{zeroconf,fusepy,paramiko,diskcache,
  nats-py}` and `wireguard-tools`. Every dep is optional; modules
  detect absence and degrade gracefully (the Mesh Performance panel
  shows which packages to `dnf install` for full coverage).



**Mesh perf #5 + #6 — NATS JetStream + mesh-fs FUSE.**

* `mackes.mesh_nats` — embedded `nats-server` (Apache 2.0,
  github.com/nats-io/nats-server) with JetStream enabled on the
  control peer. `mesh_sync.put` now publishes a small event over
  NATS in addition to writing the file; subscribers see writes in
  sub-100 ms instead of waiting for the next 30 s SSHFS scan. The
  filesystem path stays as the canonical durable store, so peers
  running the legacy code keep working. `start_subscriber(cb)` runs
  a reconnect-with-backoff loop on a daemon thread.
* `mackes.mesh_fs_fuse` — single-process FUSE backend (pyfuse3 +
  paramiko + diskcache) that opens ONE persistent SSH channel per
  peer and multiplexes file operations. Reads land in a per-peer
  LRU disk cache (512 MB cap, 30 s small-chunk TTL, 10 s
  directory-listing TTL). Mount point at `~/QNM-Mesh-fast/<peer>/`,
  cache at `~/.cache/mackes-mesh-fs/<peer>/`. Read-only v1; writes
  fall through to the legacy sshfs path during migration.

Both surface live in the Mesh Performance panel — exporter state,
JetStream stream + message counters, FUSE mount + cache MB usage.



**Mesh perf #1 + #4 + #7 — mDNS-SD bridge, private DERP, Headscale
postgres.**

* `mackes.mesh_mdns` — push-based service discovery via avahi-publish
  (announcer) + python-zeroconf (listener). MDNSListener streams
  add/remove/update events; scan_once() returns a static snapshot.
  Discovery latency drops from a 30 s scan window to sub-second.
  Falls back gracefully when either dependency is missing.
* `mackes.mesh_derp` — private DERP relay (Tailscale's open-source
  `tailscale.com/cmd/derper`). install() builds from source via the
  Go toolchain, drops a systemd unit + state dir at
  /var/lib/mackes-derper, registers the relay with Headscale via a
  DERPMap JSON file. Eliminates dependency on Tailscale's public
  DERP for cross-NAT peer traversal.
* `mackes.headscale_postgres` — full SQLite → embedded Postgres
  migration. Spins up a dedicated cluster at
  /var/lib/mackes-headscale-pg on port 5433 (separate from any host
  Postgres), creates the `mackes_headscale` role + db, patches
  /etc/headscale/config.yaml to use the postgres backend, and
  restarts headscale. Each step idempotent.

All three surface in the Mesh Performance panel's status lines.



**Mesh perf #8 + #10 + new panel — Prometheus exporter, Wake-on-LAN,
Mesh Performance panel.**

* `mackes.mesh_metrics` — wraps `prometheus-wireguard-exporter` (Rust,
  MIT, github.com/MindFlavor/prometheus_wireguard_exporter). Downloads
  the v3.6.7 binary to `/usr/local/bin/`, installs a user systemd
  unit (`mackes-wg-exporter.service` on `:9586`), and on the control
  peer drops a prometheus scrape config targeting every mesh peer.
* `mackes.mesh_wol` — pure-Python WoL magic-packet sender + ARP cache
  ingestion. `wake_peer(name)` resolves MAC from `/proc/net/arp` or
  `ip neigh`, falls back to a JSON cache at
  `~/.config/mackes-shell/peer-macs.json` that's refreshed each time
  the Mesh Performance panel renders.
* New **Network → Mesh Performance** panel surfaces every perf knob
  in one place: kernel-WG toggle, MTU + GSO state, sysctl tuning
  Apply/Remove, metrics exporter Install/Remove, and a peers table
  with a per-row Wake button for offline machines on the local LAN.
  Tight 16/16/24/24 margins so the page fits 1366×768 laptops
  without scroll.



**Mesh performance round 1 — concurrent probes + kernel WG + MTU.**

* `mackes.mesh.health()` now fans every layer probe out across a
  `ThreadPoolExecutor(max_workers=8)`. Measured 4.4× speedup
  (143 ms → 32 ms on this box) — total wall-clock is bounded by the
  slowest single layer.
* `mackes.mesh_services.probe_all()` does the same per (peer, service)
  tuple. On a fleet of 8 peers × 20 services this drops scan time
  from ~160 s worst-case to ~2 s typical.
* New `mackes.mesh_perf` module exposes three tweakable knobs:
  kernel-mode WireGuard (`--tun=mackes-wg0 --netstack=false` when the
  kernel `wireguard` module is available), LAN-optimised MTU
  (`--mtu=1380` opt-in), and a `/etc/sysctl.d/90-mackes-mesh.conf`
  drop-in that bumps `net.core.{r,w}mem_max` for higher UDP
  throughput on bursty hosts.
* `mesh_vpn.tailscale_up_with_headscale` automatically appends
  `mesh_perf.tailscale_up_flags()`, so the wizard's mesh-join flow
  picks up these knobs without code changes.



**Remmina auto-populate.** New `mackes.remmina_sync` module that fills
the Remmina remote-desktop client with every detected SSH (:22), RDP
(:3389), and VNC (:5900) service on the mesh. Design locked via a
5-question survey 2026-05-17:

* **Trigger:** explicit button (in Tweaks → Remote desktop) + every
  5-minute systemd user timer + change-on-peer-event (via the timer
  cadence). All three paths converge on `sync()`.
* **Discovery:** live TCP probe of each peer's three ports, cached
  5 min via `probe_cache`.
* **Auth:** SSH entries use `~/.ssh/mackes_mesh_ed25519` with
  `ssh_auth=3` (public-key). RDP/VNC password fields are blank — the
  user fills in once, Remmina's keyring takes over.
* **Cleanup:** every auto-generated entry has `group=Mesh Peers`.
  Files inside that group are reconciled (added, updated, deleted as
  detection changes). Files outside the group are NEVER touched —
  hard guarantee, tested.
* **UI:** headless by default. System → Tweaks gains a "Remote
  desktop (Remmina)" section with an enable toggle + "Sync now"
  button. CLI: `mackes remmina-sync [--enable|--disable|--status|
  --once]`.

Ships two new systemd-user units:
`/usr/lib/systemd/user/mackes-remmina-sync.{service,timer}`. Enabling
in Tweaks installs them to `~/.config/systemd/user/` and starts the
timer (`OnUnitActiveSec=5min`, `OnBootSec=30s`).



**Mesh test coverage.** Five new `tests/test_mesh*.py` files cover the
state machines and parsers in the mesh stack — zero tests existed
across 8 mesh modules before this. Coverage:

* `test_mesh.py` — `LayerHealth.to_dict` round-trip, `overall_state`
  state ranking (worst wins; missing degrades to warn, not fail),
  `summary` count formatting, `with_retry` success / retry / exhaust
  / propagate-unlisted-exception paths, `diagnose` smoke test.
* `test_mesh_vpn.py` — `MeshState` JSON round-trip including
  forward-compatibility (unknown fields ignored), `parse_join_link`
  query-string parsing including wrong-scheme + malformed-pair cases.
* `test_mesh_services.py` — `_probe_tcp` against a real listening
  socket and a closed port, `ServiceHit` round-trip through the
  registry JSON, corrupt-JSON guard.
* `test_mesh_ssh.py` — `PolicyRule` defaults, `AuditRecord` JSONL
  round-trip, corrupt-line skipping in the audit log.
* `test_mesh_sync.py` — bucket put/get/list_keys, automatic
  versioning (v1 → v2), JSON dict encoding, retention via
  `max_versions`.

Under `make test-nodeps`: 24 pass / 26 skip (fixture-dependent, run
under real pytest) / 0 fail.



**Mesh rock-solid pass — unified health surface.** New module
`mackes.mesh` exposes `health()` returning a `LayerHealth` per layer
(`vpn`, `ssh`, `services`, `fs`, `sync`, `notifications`, `browser`,
`thumbnailer`) with `state` (ok/warn/fail/missing), `label`, `detail`,
optional `latency_ms`, and an actionable `hint` when not OK.
`overall_state()`, `summary()`, and `diagnose()` compose it for the
Conky HUD, the Get Online wizard, and a new Mesh Health panel. The
module also exposes `with_retry()` for transient probes
(network partition, headscale flap, sshd-on-reboot). Each layer cache
TTLs 5–300 s through `probe_cache`.

**Network → Mesh Health** (`mackes.workbench.network.mesh_health`).
Per-layer status grid: glyph + label + state pill + detail + hint per
row. Header actions: Re-check (forces every probe ignoring cache),
Copy diagnostics, Save report (writes a timestamped file to
`~/QNM-Drop/mesh-health-*.txt`). Auto-refreshes every 15 s while
visible; stops on `unmap` so it doesn't burn cycles in the background.

**Conky HUD mesh row** now reads `mackes.mesh.health()` via the
updated `data/conky/helpers/mesh.sh` — the HUD reports the same state
the GUI shows. Get Online wizard gains a "View full mesh health →"
cross-link to the new panel.



**GTK perf round 5 — single rpm -qa for membership tests.** Two panels
(`maintain/dependencies` and `apps/panel`) used to call `rpm -q` once
per package in their catalogue. On a 30-package preset that's 30 forks
+ rpmdb opens, ~150 ms cumulative on first paint. Both now share a
single cached `rpm -qa` (parsed into a frozenset) and answer
membership queries in O(1). Cache TTL 60 s; explicit invalidation
fires after install/remove actions. npm `npm ls -g` queries are
cached per-package with 60 s TTL.



**GTK perf round 4.** Five more panel-construct probes moved to
`probe_cache`:

* `maintain/power.py` — `powerprofilesctl list` + `get` + `tlp-stat`
  now run on a daemon thread, cached 10–30s. Profile change
  invalidates the cache so the new active value is shown immediately.
* `system/datetime.py` — `timedatectl list-timezones` (~400 entries)
  cached for the life of the session.
* `devices/mouse.py` — `xinput --list` cached 30s.
* `devices/sound.py` — `pactl` sinks / sources / defaults cached
  10–20s.



**System → Boot & Login** (`mackes.workbench.system.boot_login`). Wraps
the `apply_plymouth` + `apply_lightdm` birthright steps in a GUI:
Plymouth theme picker (lists every theme in `/usr/share/plymouth/
themes/`; `plymouth-set-default-theme -R <name>` via AdminSession),
auto-login toggle that rewrites `[Seat:*] autologin-user=` in
`/etc/lightdm/lightdm.conf` via a temp-file + `install -D` through
AdminSession, and a read-only summary of the LightDM greeter config.
The multi-monitor "where to show the greeter" setting stays in System
→ Screens (already wired there).



**System → Tweaks** (`mackes.workbench.system.tweaks_full`). Full-page
sibling to the floating Tweaks drawer that exposes every birthright
toggle: maximize-all (via `systemctl --user is-active mackes-maximizer`),
mesh clipboard daemon, Thunar autostart, Conky HUD on/off + density +
monitor. Read/writes share `~/.config/mackes-shell/tweaks.json` with
the drawer so both stay in sync.

**Apps → Sources & Repos** (`mackes.workbench.apps.sources`). Wraps the
`apply_flathub` and `apply_third_party_repos` birthright steps in a
GUI. Threaded probes (cached 30–60 s via probe_cache) for Flathub
remote, RPM Fusion free + nonfree, fedora-workstation-repositories,
and the live `dnf repolist --enabled`. Apply buttons route through
`AdminSession.instance().run()` so the user authenticates once per
session.



**GTK perf round 3 + lint-css.sh.** Heaviest panel-construct probes
moved off the GTK main loop and through `probe_cache`:

* `maintain/fonts.py` — `fc-list` (600–2000 families, 50–300 ms) runs
  on a daemon thread, cached for 120 s. `fc-cache -f` and font
  installs invalidate the cache so freshly added families show up
  immediately.
* `look_and_feel/appearance.py` — monitor list now prefers
  `mackes.displays.xrandr_outputs_for_conky()` (xfconf, instant) over
  the xrandr CLI; cached 60 s.
* `devices/display.py` — display summary likewise reads
  `mackes.displays.list_outputs()` first; xrandr is the fallback only.
  Cached 60 s.

**install-helpers/lint-css.sh** — the CSS lint gate from CLAUDE.md
§0.7 that was missing from the tree is restored as a thin
`GtkCssProvider` load check. Whitelists four pre-existing warnings
(`text-transform`, `font-feature-settings`, `cursor`, `line-height`)
that GTK CSS doesn't implement but the codebase has carried since the
1.1.0 Carbon refresh. Exits non-zero on any new real CSS error.



**GTK perf round 2.** Two more main-loop blockers fixed:

* `maintain/logs.py` now visibility-gates its 2-second poll — the
  timer starts on `map` and stops on `unmap`, so the 2s file-stat
  wake-up no longer fires while the panel is hidden.
* `maintain/system_update.py:_refresh_summary` moved off the GTK
  main loop. The `dnf list --upgrades -q` shell-out (1–15s depending
  on cached metadata) runs on a daemon thread and posts back via
  `GLib.idle_add`. Result memoized in `probe_cache` for 60s so
  re-opening the panel within that window is instant.



**Lazy sub-nav panel construction.** Opening "Devices", "System", or
"Look & Feel" used to instantiate every sub-panel in the group, each
of which shells out to `xrandr` / `xinput` / `nmcli` / `fc-list` /
`rpm -q` at `__init__`. Cumulative cost: 600–1200 ms of frozen GTK
main loop per group open on a stock Fedora 44 box. `_build_subnav_
container` now accepts `(key, label, factory)` tuples; the factory is
called on first navigation to its tab, with an empty Box placeholder
in the meantime. First-paint cost drops to ONE panel × one shell-out
chain. Same treatment applied to the Maintain hub's 13 sub-panels —
the hub view (cheap) builds eagerly; each sub-panel materialises on
first `_go(key)` call.



**GUI distinctiveness + plain-language explainers.** The Carbon
surface gained subtle elevation everywhere it was previously flush:
sidebar nav groups + items lift onto `@cds_bg_layer_01` with a
right-edge accent rail on the active item; stat tiles, app cards, and
DataTable rows get 1px `@cds_border_subtle_00` borders with hover
states; the Tweaks drawer floats on `@cds_bg_layer_02`; notifications
read as cards instead of banners. A new `.mackes-section-description`
class (background layer-01, left rail `@mackes_accent_soft`, 14/20
muted body) styles short 9th-grade-level explainers that every major
panel now carries above its first section. Tone is second-person,
present-tense, mentioning the user's intent first and the mechanism
second. Helper added: `mackes.workbench._common.section_description()`.
Affects ~40 panels across `dashboard`, `apps`, `fleet`, `devices`,
`look_and_feel`, `maintain`, `network`, `system`.



**Network → Get Online** (`mackes.workbench.network.mesh_join`,
`mackes.wizard.pages.mesh_join`). A one-button onboarding wizard that
gets a peer onto a usable network and joined to the Mackes mesh.
Off-thread probes (NetworkManager, tailscaled, Headscale, MeshState,
QNM) populate a Carbon checklist; a single "Get me online" button runs
the missing chain end-to-end (Wi-Fi pick → `nmcli connection up` →
`systemctl enable --now tailscaled` → `tailscale up
--login-server=<headscale>` with the auth URL surfaced as copyable text
+ optional QR code → `qnmctl init`). All privileged calls route through
`AdminSession`. Idempotent re-entry: if every probe is green the
button becomes "Already online" with a Re-check link.

**System → Displays** (`mackes.displays`,
`mackes.workbench.system.displays`). New panel that wraps the
xfsettings `displays` xfconf channel — the actual source of truth on
Fedora's LightDM + xfce4-settings stack. Drag-to-arrange monitor canvas
with edge-snap, per-output expanders (active, primary, resolution,
scale 1.0–2.0, rotation 0/90/180/270, refresh rate), profile save /
load / delete (xfconf named profiles), and a 15-second "Keep this
layout?" preview before revert. Per-monitor wallpaper picker writes
`xfce4-desktop:/backdrop/screen0/monitor<NAME>/workspace<N>/last-image`
across all workspaces. LightDM greeter "active-monitor" section edits
`/etc/lightdm/lightdm-gtk-greeter.conf` via `AdminSession`. When the
active layout changes, the Conky HUD re-pins via SIGUSR1 if its
configured monitor moved.

**Conky HUD rewritten for speed + height.** The v1.4.0 "⅔ screen height,
10-section" lock is retired. The HUD now auto-sizes to content, ships
three density tiers (Compact / Standard / Full) selectable from Tweaks,
and renders far cheaper per refresh:

* Glyphs use **Hack Nerd Font**, installed automatically by the
  refreshed `apply_fonts` birthright step (downloaded from the upstream
  v3.2.1 release tarball — Fedora doesn't package any Nerd Font). The
  prior config asked for "Cascadia Code NF" which was never installed,
  so every section glyph rendered as tofu.
* The accent-coloured left edge is now a **single cairo stroke** drawn
  by `data/conky/mackes-conky.lua`, not a per-line `┃` glyph
  substitution. Conky's bundled cairo + cairo_xlib Lua extensions are
  found via an injected `package.cpath`.
* Empty sections collapse — Fleet / Drift / Storage all check their
  helper's first line before drawing the header.
* Notifications / Media / Remote merge into a single **Services** row
  rendered by `helpers/services_row.sh` (three chips, one line).
* Every helper is wrapped in `timeout 3`. The `mackes --version` daily
  Python spawn is gone — the version is baked into the config at
  render time.
* Click-through is enforced via X SHAPE input region (ctypes / libXext),
  found post-spawn via `xdotool search --class mackes-conky`.
* Per-monitor placement: `conky_hud._xrandr_outputs` reads xrandr when
  installed and falls back to the xfsettings `displays` xfconf channel
  (which on a Fedora 44 LightDM box is the actual source of truth).
  Tweaks → "HUD monitor" picks the target output.
* Preset swap uses `SIGUSR1` for a hot reload instead of the
  desktop-flashing kill / respawn.

**xfce4-panel snapshot becomes the platform default.** Your current
panel layout is captured in `data/panel/xfce4-panel.snapshot.json`
(70 properties, two panels) and `apply_panel_layout` is now a
data-driven loader from that file. The v1.5.0 plugin-id race fix is
preserved (panels are quit before write, plugin-ids written last).
Transient PII keys (Wi-Fi SSIDs in `known-legacy-items`, app history in
`known-items`) are filtered at apply time. Re-snapshot anytime via
`tools/snapshot-panel.py`.

Spec gains `Recommends: xorg-x11-server-utils` (xrandr for per-monitor
geometry) and `Recommends: xdotool` (click-through window-finder); both
degrade gracefully when absent.

## 1.5.2 — QNM as 14th birthright (2026-05-17)

`apply_qnm` joins the apply pipeline between Mesh clipboard and Mesh.
Behavior:

1. `dnf install -y qnm` (graceful — logs a clear "not available in
   your repos" message if QNM isn't packaged for your Fedora set).
2. `qnmctl init` (idempotent).
3. `systemctl enable --now qnm.service`.
4. `set_qnm_enabled(True)` so the Mackes UI knows QNM is live.

Respects `preset.network.qnm_enabled = false` — opting out at preset
time still works. Review page lists the new step.

## 1.5.1 — UI lag fix + xfce4-panel crash hotfix (2026-05-17)

Two issues from the v1.5.0 install:

**UI lag.** Every 30 seconds the shell's status bar and side-nav badges
ran `service_health()` + `headscale_list_peers()` + `load_registry()`
+ `active_preset_drift()` synchronously on the GTK main loop. Each of
those shells out — easily 200–500ms total per tick — freezing the
window for that window. Fixed: both refreshers now run on a daemon
`threading.Thread` and post results back via `GLib.idle_add`. The main
loop is never blocked.

**xfce4-panel crash.** `apply_panel_layout` wrote `/panels/panel-0/
plugin-ids = [101..105]` BEFORE writing each plugin's type
(`/plugins/plugin-101 = whiskermenu`, etc.). If xfce4-panel was
running and observed the array via xfsettingsd, it tried to load
`plugin-101 = <unset>` and SIGSEGV'd. Fixed by:

* `xfce4-panel --quit` BEFORE writing any xfconf state.
* Write plugin types + each plugin's config keys FIRST.
* Write the `/panels` and `/panels/panel-0/plugin-ids` arrays LAST.
* `xfce4-panel` (relaunch, not --restart) so the new config is the
  only thing it ever sees.

**Maximizer poll** bumped 1s → 2s so the second-by-second `wmctrl -l`
+ `xprop` fork-per-window doesn't add a CPU baseline.

## 1.5.0 — Mesh clipboard (bidirectional sync) (2026-05-17)

The clipboard plumbing is now bidirectional — every system-clipboard
change publishes into the mesh bucket, and every peer's items show up
in the viewer. Built as a Python rewrite of `mackes/clipboard_app.py`
instead of a C-fork of `xfce4-clipman-plugin` — same surface, far
less infrastructure to maintain.

### New modules + units

`mackes/clipboard_app.py` rewritten with three CLI modes:

  --daemon   headless XA_CLIPBOARD watcher. Publishes every new text
             or image (PNG via GdkPixbuf) to
             `~/QNM-Shared/.qnm-sync/clipboard/<me>/<ts>.{txt,png}`.
             Heuristic secret filter on by default (shannon entropy
             ≥ 4.5 bits/char on no-whitespace strings, or matches
             known prefixes like `sk-`, `ghp_`, `AKIA…`, BEGIN PRIVATE
             KEY blobs). Settings live at
             `~/.config/mackes-shell/clipboard-daemon.json` and are
             re-read every 10s.

  --viewer   foreground GTK window: one tab per peer, listbox of
             recent items (200 max), double-click an entry to paste
             it into THIS peer's clipboard. Images render as
             `<image Nb>` rows; text shows first 120 chars.

  (no flag)  defaults to --viewer (legacy launcher path stays).

`data/systemd/mackes-clipboard-daemon.service` (user unit) supervises
the daemon. ConditionEnvironment=DISPLAY + ConditionPathExists=
!`~/.config/mackes-shell/clipboard.disabled` so it's both
display-aware and toggleable.

### 13th birthright step

`apply_clipboard_daemon` enables `mackes-clipboard-daemon.service`
via `systemctl --user enable --now …`. Wired into the wizard apply
pipeline between Maximize windows and Mesh.

### Companion C panel plugin

The existing `xfce4-panel/plugins/mackes-clipboard` plugin (read side)
keeps working unchanged — it surfaces every peer's bucket in a
panel-popover. The daemon adds the write side that was missing.

## 1.4.7 — Conky Nerd Font glyphs (2026-05-17)

The Conky HUD now uses Nerd Font (Cascadia Code NF, the only patched
NF in stock Fedora 44) for icon glyphs alongside IBM Plex Sans for
prose. Every section header gets a glyph prefix:

  Shell (header)        terminal
  Mesh                  wifi
  Fleet                 cogs
  Drift                 warning
  Shared storage        archive
  Notifications         bell
  Media services        music
  Remote desktop        terminal-secure
  Services dot grid     server
  Hardware              CPU
  Clock                 clock
  Admin lock /        unlock / lock indicator

Glyphs are embedded directly as UTF-8 from the Private Use Area
(no ${execpi printf} hack — that one already burned us in v1.4.6).
The font switches mid-line via `${font Cascadia Code NF:size=10}` /
`${font IBM Plex Sans:size=N}` blocks so prose stays readable.
New helper `admin-lock-glyph.sh` emits the lock/unlock glyph only.

Spec: `Requires: cascadia-code-nf-fonts`.

## 1.4.6 — Panel layout / wallpaper / Conky / QNM (2026-05-17)

Four user-reported issues fixed in one cut:

* **Whisker menu missing from the panel.** `apply_panel_layout`
  wrote `/panels/panel-0/plugin-ids` as an empty single-value field
  instead of a proper uint array — fixed via `_set_array()` helper
  using `xfconf-query --create --force-array --type uint --set 101 …`.
  Array reset first so a default panel-0 doesn't conflict.

* **Whisker menu modifications not visible.** Added a Mackes-branded
  Whisker config block — button title "Mackes", button icon
  `mackes-shell`, search-position alternate (top), categories
  alternate, recent-items 10, menu 440×560, IBM Plex item names,
  `mackes-shell.desktop` favorited by default.

* **Wallpaper not applied.** `apply_appearance` silently skipped the
  wallpaper when the preset's path didn't exist. Now falls back to
  `/usr/share/mackes-shell/branding/standard-wallpaper.png` and
  stamps five common per-monitor xfconf keys (HDMI-1 / HDMI-A-1 /
  eDP-1 / LVDS-1 / VGA-1) in addition to the canonical
  `screen0/monitor0/workspace0/last-image`.

* **Conky never started.** The v1.4.0 template used
  `string.format([[…]], 35 args)` plus a fragile
  `${execpi 99999 printf "┃"}` Lua escape — both broke conky's
  Lua parser. Template rewritten as plain Lua concatenation;
  U+2503 embedded as a UTF-8 literal. Tested with `conky -c` —
  parses + forks cleanly.

* **QNM "where is it?" UX.** Sidebar nav item renamed from "QNM"
  to "Quick Network Mesh (QNM)" for new users.

## 1.4.5 — Toggle-button init-order crashes (2026-05-17)

Two `AttributeError` traceback surfaced during the first-run wizard
after v1.4.4 reached the Dashboard:

    AttributeError: 'MeshVpnPanel' object has no attribute '_peers_stack'
    AttributeError: 'AppsPanel' object has no attribute '_chips_box'

Root cause: the topology/table toggle on Mesh VPN and the
Install/Remove/Installed tabs on Apps both `set_active(True)` on
their default button **during** `_build()`. That fires the `toggled`
signal before the rest of the panel state (the Gtk.Stack the toggle
flips, the FlowBox of category chips) is constructed.

Fix: both handlers now `getattr(..., None)` for the dependent state
and return early if it's missing. The post-build refresh sets the
correct state afterwards — the early firing is a harmless no-op now.

## 1.4.4 — LightDM hang hotfix (2026-05-17)

The wizard's final step "Becoming Mackes…" hung indefinitely with the
log line `lightdm config: <…>` because `mackes/lightdm.py` had its own
`_pkexec_write` / `_pkexec_mkdir` helpers that bypassed AdminSession —
the NOPASSWD short-circuit never fired, so the calls prompted polkit
and either timed out or got dismissed.

Same fix pattern as the v1.4.3 headscale fix:

* `_pkexec_write` rewritten — when AdminSession is unlocked, stages
  the config to a tempfile and runs `install -D -m 0644 tmpfile
  target` via the cached sudo creds. Falls back to legacy
  stdin-piped `pkexec tee` only if AdminSession is unimportable.
* `_pkexec_mkdir` routes through `AdminSession.run(["mkdir", "-p", ...])`.
* Sudoers `MACKES_GATEWAY` extended to cover
  `/usr/bin/tee /etc/lightdm/*` and `/etc/lightdm/lightdm.conf.d/*`.
* Legacy `tee`-with-stdin timeout bumped 10s → 30s.

## 1.4.3 — Headscale + Tailscale prompt-storm hotfix (2026-05-17)

The v1.4.2 sudoers drop-in eliminated the pkexec prompt storm for
`dnf`, `systemctl`, and the other Mackes-managed commands — but
**headscale** and **tailscale** invocations kept prompting because:

  1. Those binaries weren't in the sudoers allowlist.
  2. `mesh_vpn.py:_pkexec_run` was a legacy wrapper that always used
     raw `pkexec` instead of routing through `AdminSession.run()` like
     birthright / debloat / remote_desktop / caddy_gateway.

Both fixed:

* **Sudoers extended** — `data/sudoers.d/mackes-shell` gains three
  new aliases: `MACKES_HEADSCALE`, `MACKES_TAILSCALE`, and
  `MACKES_HEADSCALE_CONFIG` (covering `tee /etc/headscale/*` plus
  the `bash -c "mkdir -p /etc/headscale && cat > …"` chunk the
  wizard uses to write `config.yaml`). All NOPASSWD for the `wheel`
  group. Validated by `visudo -c` in `%post`.

* **`mesh_vpn.py:_pkexec_run` refactored** to route through
  `AdminSession.instance().run(cmd)` — matches the v1.4.0 call-site
  migration pattern. The sudoers NOPASSWD short-circuit fires and
  the user never sees a polkit prompt during mesh setup. Falls back
  to the legacy `pkexec` / `sudo` / raw chain only if AdminSession
  is unimportable (paranoia path).

`mesh_ssh.py` already used `_pkexec_run` for its `headscale policy
set` call, so it inherits the fix automatically.

## 1.4.2 — Fedora 44 dep hotfix + fit-to-resolution windows (2026-05-17)

**Fedora 44 dep hotfix.** `xorg-x11-utils` was renamed/split out of
Fedora's package tree; `xprop` is its own package now. v1.4.1 install
failed with:

    Problem: conflicting requests
      - nothing provides xorg-x11-utils needed by mackes-shell-1.4.1

Spec Requires fixed: `xorg-x11-utils` → `xprop`. Same substitution
applied in `mackes/birthright.py:apply_maximize_all` so the wizard
step's dnf-install probe uses the correct package name on the fallback
path.

**Every GUI window fits the workstation resolution perfectly.** The
WorkbenchWindow and WizardWindow now detect the primary monitor's
size via `Gdk.Display.get_primary_monitor().get_geometry()`, open at
that exact size, and call `maximize()` on the `realize` signal so the
WM finishes the job. The previous hardcoded `1280×800` and `960×720`
defaults are gone — the windows fill whatever screen they land on,
whether 1366×768 laptop or 4K monitor. This overrides the Carbon
"max-content-width" pattern: the content area expands to use available
width rather than getting letterboxed.

Helper `_primary_monitor_size()` lives in both
`mackes/workbench/shell/sidebar_window.py` and `mackes/wizard/window.py`
(intentional duplication — they ship independent of each other and
the helper is 12 lines).

## 1.4.1 — Sudoers, installer UX, wizard discoverability, maximize-all (2026-05-17)

Five user-reported friction points addressed:

**Sudoers drop-in** (`data/sudoers.d/mackes-shell`, installed at
`/etc/sudoers.d/mackes-shell` mode 0440). Grants the `wheel` group
NOPASSWD on the Mackes-managed command allowlist (dnf, systemctl,
firewall-cmd, install/cp/chown, gtk-update-icon-cache,
plymouth-set-default-theme, the Apache-archive curls birthright uses,
tee for specific config paths). Validated by `visudo -c` in `%post`;
on failure the file is removed so the host's sudo behavior is never
broken. `AdminSession.run()` short-circuits to `sudo -n` when this
drop-in is active — no prompts at all during normal Mackes
operations. The previous prompt-storm during the wizard's birthright
pipeline is gone.

**Carbon-styled installer** (`install.sh` rewrite). Each phase
renders as a Carbon banner row with a spinner: Detect Fedora →
Resolve release tag → Download RPM → Install via dnf → Hand off to
wizard. The dnf transaction streams its output as Carbon-dimmed
lines instead of going dark for several minutes. Logs to
`/tmp/mackes-install.*.log` for triage.

**Always-visible Setup button in the header** — next to the Help
button. Opens the wizard regardless of `state.provisioned`. The
hidden "Re-open Wizard" inside the Tweaks drawer stays for muscle
memory.

**Birthright health check** (`mackes/birthright_check.py`): 12 probes
that verify each apply_* step's on-disk artifacts (theme dirs, IBM
Plex packages, Plymouth theme active, sudoers drop-in present, panel
layout xfconf, RPM/AppImage app presence, xrdp + Guacamole config,
ansible-pull timer enabled, Conky config + autostart, maximizer
service, Flathub remote, third-party repos). `is_complete()` returns
True only when all 12 pass.

**Always-maximize windows** (12th birthright). A new user-level
service `mackes-maximizer.service` polls `wmctrl -l` once per second
and adds `maximized_vert`/`maximized_horz` to every new top-level
window. Exempt classes: `xfce4-panel`, `xfdesktop`, `mackes-conky`,
`Plymouth`. RPM Requires `wmctrl` + `xorg-x11-utils` (for `xprop`).
Disable per-user via `~/.config/mackes-shell/maximizer.disabled`.

## 1.4.0 — Debloat tiers, TUI, Splash, Conky HUD, Session unlock, full Carbon (2026-05-17)

Seven user-driven additions plus the Carbon-completion pass that finishes
the design assimilation started in v1.1.x.

### Carbon completion

The two items deferred at the original v1.4.0 cut window are now done:

**Legacy panels** (`mackes/workbench/_common.py`): rewrote the shared
helpers (`panel_box / title_label / info_label / section_header /
labeled_row / error_label`) to emit Carbon-refresh widgets. Single-file
change cascades across **every** legacy panel that imported these
helpers — Devices / System / Network (Wi-Fi, VPN, QNM, Firewall) /
Help — without per-panel rewrites. Old v1.0 CSS class names are kept
alongside the new ones, so no CSS rule regressions.

**Carbon-native wizard window** (`mackes/wizard/window.py`): replaced
`Gtk.Assistant` with a custom `Gtk.ApplicationWindow` matching the
sidebar shell's chrome. Top: 9-step progress strip with active
indicator. Center: a `Gtk.Stack` of page widgets (welcome / env-scan /
preset-pick / appearance / hardware / network / snapshot / review /
apply / summary). Bottom: a Carbon action bar (Back / Cancel / Next or
Apply or Continue or Finish, depending on the active step's kind).
Existing page builder modules drop in unchanged — they were already
Carbon-styled inside. The PROGRESS step auto-launches the apply
pipeline on first activation, then unlocks the Continue button. The
SUMMARY step's Next button becomes "Finish" which destroys the window
and unblocks `do_activate` → opens the Dashboard.

### Features

**Conky HUD** (`mackes/conky_hud.py`, `data/conky/`, 11th birthright):
top-right Carbon-themed desktop panel (400 × ⅔ screen height) with
live Mackes-platform state. Opaque Carbon Gray 90 fill with a 3px
accent left-edge that swaps with the active preset. Birthright step
`apply_conky` installs the package + writes the user config + the XDG
autostart entry, then bounces the process. Tweaks panel gains a
"Show Conky HUD" switch under Chrome that flips both the autostart
file and the running process.

Tiered refresh per Q3 lock — `update_interval=1.0` for the system
built-ins (clock, CPU, RAM, load), `${execi 30 ...}` for Mackes-state
queries (mesh / fleet / drift / notifications / media services /
remote sessions / services dot-grid), `${execi 60 ...}` for shared
storage (rare changes).

Ten content blocks per Q4 lock: Header (version + preset + admin
lock), Mesh (peers + control node), Fleet (last pull + 24h failures),
Drift (items differing from preset), Shared storage (QNM-Shared
usage), Global notifications (mesh + local counts + latest), Media
services (Jellyfin/Plex/Airsonic/etc. across peers), Remote desktop
(active RDP/VNC + Guacamole connections), Services (sshd / headscale
/ tailscaled / guacd / tomcat / mackes-remote-sync / mackes-ansible-pull
/ caddy as a compact dot grid), Hardware (hostname / CPU / RAM / load
/ clock).

Helper scripts under `data/conky/helpers/*.sh` — one per block. Each
shells out to either a Mackes Python module (mesh / fleet / drift /
media) or pure shell (storage / notifications / remote / services).

Spec **Requires: conky** so birthright never finds the package missing.
The Conky preset accent live-swaps via `conky_hud.restart_with()`
called from the shell's `_apply_tweaks()` whenever the preset changes.

### Features

**Wizard boot splash** (`mackes/wizard/splash.py`): plays
`branding/MACKES-XFCE-LOGO.mp4` (H.264 1280×720, 8s, AAC audio muted)
as a borderless centered window before the first-run wizard surfaces.
Skippable via click / Escape / any key; auto-dismisses on
end-of-stream. Falls back silently if GStreamer or its H.264 decoder
isn't installed. The pipeline uses GStreamer `playbin` + the X11
`VideoOverlay` XID-embed pattern (gtksink isn't packaged in stock
Fedora 44 GStreamer, but `xvimagesink`/`ximagesink` are).
Spec Recommends: `gstreamer1`, `gstreamer1-plugins-{base,good,bad-free}`,
`mozilla-openh264`, `gstreamer1-plugin-openh264`. All Recommends not
Requires so headless nodes don't carry the codec stack.
MANIFEST.in extended to include `*.mp4` / `*.webm` under `branding/`
so the video survives the sdist round-trip into the RPM.

**Debloat levels** (`mackes/debloat.py`, `Maintain → Debloat levels`):
five cumulative tiers (L1 Light → L5 Viable). Each tier is an
idempotent `dnf remove` set plus optional xfconf resets. The panel shows
a live preview of what's currently installed vs already absent before
the user commits. Bound by a confirm modal; logs the run.

**Textual TUI** (`mackes/tui/`, autobooted on headless): runs every
screen the GUI has — Dashboard, Mesh VPN, Mesh SSH, Mesh Services,
Mesh Remote, Fleet Inventory, Fleet Playbooks, Fleet Run history,
Snapshots, Debloat, Help. Launches automatically when there's no
`$DISPLAY` and no subcommand. `python3 -m mackes --tui` forces it.

**Session unlock** (`mackes/admin_session.py`, header Lock/Unlock
button): single sign-in for the whole Mackes session. Click Unlock,
type the password once, every subsequent admin op runs without
prompting. Uses sudo's timestamp cache + a 4-min keepalive thread.
Auto-locks when the window closes. Migrated call sites:
  - `mackes/birthright.py:_run_root`
  - `mackes/workbench/network/remote_desktop.py:_run_root`
  - `mackes/debloat.py:apply_level`
  - `mackes/caddy_gateway.py:_pkexec`

**Live status bar** (`shell/sidebar_window.py:_refresh_status_bar`):
the bottom bar's mesh / services / sshd / drift counts are now live —
pulled from `service_health()`, the Headscale roster, the mesh-services
registry, and the active-preset drift detector. Refreshes every 30s.

**Live sidebar nav badges**: peer count on Mesh VPN, service count on
Mesh Services, failed-runs count on Fleet → Run history, drift-items
count on Maintain. Same 30s refresh cycle as the status bar.

**Tweaks density** finally works: compact / cozy / comfortable now
swap `.mackes-density-*` classes on the root window. CSS rules in
`carbon-layout.css` adjust nav-item heights, tile padding, and
data-table row heights accordingly.

**Toast host** (`shell/toasts.py`): bottom-right non-modal notifications
for shell-wide events. Snapshot create now uses a toast instead of a
silent status label.

### Carbon design system

`.claude/CLAUDE.md` + `.claude/skills/{mackes-worklist-management,
complete-remaining-work}/SKILL.md` — three workflow protocols ported
from `matthewmackes/map2-audio` and adapted to the mackes-shell repo.
The commit/push rulebook, single-source worklist, and autonomy policy
are now durable behavioral contracts in `.claude/`.

### Open-source project artifacts

Added the standard OSS files the repo was missing:
  - `CONTRIBUTING.md` — dev setup + project conventions
  - `CODE_OF_CONDUCT.md` — Contributor Covenant v2.1
  - `SECURITY.md` — disclosure protocol + threat model
  - `AUTHORS` — maintainer + upstream credits
  - `.editorconfig` — line endings + indentation
  - `.github/ISSUE_TEMPLATE/{bug_report,feature_request,config.yml}`
  - `.github/PULL_REQUEST_TEMPLATE.md`
  - `.github/FUNDING.yml`
  - `.github/dependabot.yml` (weekly Actions bumps)
  - `CITATION.cff`

### Deferred to v1.4.1

Legacy panels (`devices/*`, `system/*`, `network/{wifi,vpn,qnm,firewall}.py`)
still use the v1.0-era `workbench/_common.py` helpers — they look
inconsistent next to the v1.1.x Carbon-refresh panels. Wizard chrome is
still `Gtk.Assistant`, not a Carbon-native window. Both are tracked as
v1.4.1 work — they're substantial mechanical rewrites that don't block
the v1.4.0 functional additions.

## 1.3.0 — Mesh Fleet (Ansible-pull) (2026-05-17)

Cross-peer fleet management lands as a 10th wizard birthright step.
Ten design decisions locked via the 1.3.0 question survey:

  1. Transport: **ansible-pull** on every peer (no central controller)
  2. Playbook store: **QNM-Shared/.qnm-sync/playbooks/** (replicated by
     the existing file substrate)
  3. Install: 10th wizard step `apply_fleet` — always on
  4. Curated playbooks: 7 roles ship — system-update, bloat-removal,
     apps-install, xfconf-baseline, mesh-state-snapshot,
     selinux-permissive-toggle, container-runtime-setup
  5. Schedule: systemd timer — OnBootSec=10min,
     OnUnitActiveSec=30min, RandomizedDelaySec=5min
  6. GUI: new top-level **Fleet** sidebar group with 3 items
     (Inventory / Playbooks / Run history)
  7. Editor: read-only YAML preview + `xdg-open` to user's editor
  8. Secrets: none — playbooks are plaintext
  9. Run history: 30-day retention, one JSON per run at
     `QNM-Shared/.qnm-sync/ansible-runs/<peer>/<ts>.json`
 10. Ad-hoc: yes — Inventory has multi-select + "Run on selection"
     SSH-push over mesh-SSH identity

### What was added

**New birthright step** `apply_fleet` in `mackes/birthright.py`:
  - dnf install: ansible-core, python3-ansible-runner, podman
  - Seeds the playbook tree into QNM-Shared/.qnm-sync/playbooks/
  - Installs + enables mackes-ansible-pull.{service,timer}
  - Queues an initial pull (non-blocking)

**New module** `mackes/fleet.py`:
  - `build_inventory()` — Headscale roster → FleetPeer list with
    per-peer last-pull timestamp + 24h pull count
  - `list_playbooks()` — discovers roles under the QNM-Shared tree
  - `list_runs()` / `write_run_record()` / `prune_runs()` — full
    30-day-retention history reader/writer
  - `run_local_pull()` — local ansible-pull, parses the PLAY RECAP,
    writes a JSON record
  - `run_push()` — ansible-playbook SSH push to selected peers via
    a generated ephemeral inventory.ini
  - CLI: `python -m mackes.fleet --pull / --push / --list / --history / --prune`

**7 curated playbooks** under `data/ansible/playbooks/`:
  - system-update          (tag-gated `never`; opt-in via GUI)
  - bloat-removal          (default-tagged; runs on every cycle)
  - apps-install           (default-tagged)
  - xfconf-baseline        (default-tagged; the steady-state drift corrector)
  - mesh-state-snapshot    (tag-gated `never`)
  - selinux-permissive-toggle (tag-gated `never`)
  - container-runtime-setup (tag-gated `never`)

**Systemd units** at `data/systemd/`:
  - mackes-ansible-pull.service (Type=oneshot, ConditionPathExists
    fleet.disabled escape hatch)
  - mackes-ansible-pull.timer (30-min cycle with 5-min jitter)

**Fleet GUI** — new top-level `Fleet` sidebar group with 3 Carbon panels:

  - `mackes/workbench/fleet/inventory.py` — Carbon page header, live
    status notification, action row with Run-on-selection /
    Local-pull / Select-all-online / Clear, peer ListBox with
    checkbox + status dot + last-pull age + per-peer status tag.
    Multi-select drives the SSH-push playbook picker Modal.
  - `mackes/workbench/fleet/playbooks.py` — grid of Carbon tiles per
    playbook with description, tag chips (default / never), last-run
    summary, YAML preview, Run-now and Open-in-editor buttons.
  - `mackes/workbench/fleet/run_history.py` — stat tiles (Total /
    Successful / Failed / Changes applied), peer + playbook filters,
    Carbon DataTable of every run across the mesh. Click any row to
    see the full JSON in a Carbon Modal (timestamp, trigger, duration,
    counts, log tail).

**Spec Requires:** ansible-core, python3-ansible-runner, podman.
**Spec Recommends:** buildah, skopeo, toolbox.

## 1.2.0 — Mesh Remote Desktop (2026-05-17)

Every Mackes node now ships browser-accessible remote desktop. Five
design decisions locked via the 1.2.0 question survey:

  1. Backends: **xrdp + x11vnc on every peer** (both protocols)
  2. Topology: **every peer runs guacd + Guacamole**
  3. Auth: **none on the mesh** (firewall + mesh CA are the trust)
  4. Connection discovery: **Headscale roster auto + Mackes overrides**
  5. Enablement: **birthright — always on**

### What was added

**9th birthright step** `apply_remote_desktop` in `mackes/birthright.py`:
  - dnf install: xrdp, xrdp-selinux, x11vnc, guacd, tomcat, curl
  - Downloads guacamole-1.6.0.war from the Apache archive into
    /var/lib/tomcat/webapps/
  - Installs the noauth extension jar at /etc/guacamole/extensions/
  - Writes /etc/guacamole/guacamole.properties + a seed
    /etc/guacamole/noauth-config.xml
  - Installs an x11vnc@.service systemd template that binds to the
    mesh IP only (live :0 mirror)
  - Installs mackes-remote-sync.service (regenerates the noauth
    connection list from the Headscale peer roster every 30s)
  - Opens firewalld ports 3389 / 5900 / 8080 on the trusted zone only
  - Enables + starts: xrdp, xrdp-sesman, x11vnc@:0, guacd, tomcat,
    mackes-remote-sync

**Connection sync** `mackes/remote_desktop.py`:
  - `active_connections()` returns RDP + VNC entries per Headscale peer,
    layered with `~/.config/mackes-shell/remote-overrides.json`
    (favorite / hide / rename)
  - `rebuild_connections()` writes /etc/guacamole/noauth-config.xml
  - `sync_daemon_main()` is the systemd-managed polling loop
  - CLI: `python -m mackes.remote_desktop --list / --rebuild / --daemon`

**Caddy gateway** route added in `mackes/caddy_gateway.py`:
  `https://media.mesh/desktop/  →  http://127.0.0.1:8080/guacamole/`

**Mesh Remote panel** `mackes/workbench/network/remote_desktop.py` —
a full first-class configuration GUI matching the Carbon panel
patterns:
  - Breadcrumb + page title + subtitle + live status Notification
  - Local services grid (xrdp / x11vnc / guacd / tomcat)
  - **Display sharing** tile: enable/disable x11vnc, X display picker,
    view-only mode toggle
  - **RDP server** tile: enable/disable xrdp, Xorg vs Xvnc backend,
    max concurrent sessions
  - **Gateway** tile: Tomcat toggle + Open-in-browser button + code
    block showing the effective Caddy route
  - **Connections** Carbon DataTable with per-row Favorite / Hide /
    Rename buttons (Rename opens a Carbon Modal)
  - **Auto-discovery** tile: sync interval (10-600s) + last-sync
    timestamp display
  - **Diagnostics** tile: `systemctl status` text for all five units +
    Refresh button
  - Persists per-user prefs to `~/.config/mackes-shell/remote-desktop.json`

**Sidebar nav** gains a "Mesh Remote" entry under Network.

**Wizard** apply pipeline is now 19 steps (added "Remote desktop"
between Flathub and Mesh); review page lists the new step.

### Spec requires

The RPM now Requires xrdp / xrdp-selinux / x11vnc / guacd / tomcat /
curl. The guacamole.war + noauth jar are fetched from the Apache
archive at first-wizard-run; the RPM itself doesn't carry them.

## 1.1.1 — Carbon panel rebuilds (the rest of the design) (2026-05-17)

Picks up where 1.1.0 left off — the seven panels that were deferred at
the v1.1.0 release window are now rebuilt to match
`docs/design/v1.1.0-carbon-refresh/`:

* **Mesh SSH** (`mackes/workbench/network/mesh_ssh.py`): page-title +
  breadcrumb, live "Tailscale-SSH active on N peers" Notification, peer
  DataTable with a host-key fingerprint column, ACL hujson rendered as
  a Carbon code block with an Edit/Save/Reload toolbar, key
  distribution actions tile, audit log DataTable.
* **Mesh Services** (`mackes/workbench/network/mesh_services.py`):
  scan/refresh action row, peer filter pills, 3-column Carbon tile
  grid of discovered services (each tile shows kind tag, status dot,
  display name, peer, accent URL), unified gateway tile with a Switch
  + route-preview code block, mDNS bridge tile listing relayed types
  as Tag chips.
* **Appearance** (`mackes/workbench/look_and_feel/appearance.py`):
  rewrapped into a two-column Carbon layout — selectors on the left
  (existing xfconf bindings preserved verbatim), live preview pane on
  the right with sample window chrome + heading + body + mono command
  + Primary/Tertiary/Ghost button row + an Active Accent swatch tile
  + Design-system-lock notification.
* **Apps** (`mackes/workbench/apps/panel.py` — new unified panel):
  three Carbon tabs (Install / Remove bloat / Installed), category
  filter chips derived from the catalog, search input, grid of
  `.mackes-app-card` tiles with icon/name/desc/meta and per-tab
  action button. Replaces the three legacy `install.py` /
  `remove.py` / `installed.py` panels at the sidebar entry point.
* **Snapshots** (`mackes/workbench/maintain/snapshots.py`): Carbon
  create tile (label input + Primary button + helper line listing
  exactly what gets captured) + Carbon DataTable of existing
  snapshots (label, created timestamp, source preset, size).
  Restore opens a confirm modal; double-click also triggers restore.
* **Maintain hub** (`mackes/workbench/maintain/hub.py` — new): 12-tile
  Carbon grid replacing the old StackSidebar+Stack inner layout for
  the Maintain section. Tile click switches an inner Gtk.Stack to the
  matching sub-panel, which is wrapped with a "‹ Back to Maintain"
  link header. Drift tile and Uninstall tile carry warning/error tags
  to mirror the design.
* **Help** (`mackes/workbench/help.py`): left rail now uses the
  `mackes-side-nav` Carbon classes (consistent with the main shell);
  right pane has a Carbon breadcrumb + page-title header above the
  existing markdown TextView, which got Carbon 40px page margins.
  Topic discovery and markdown rendering unchanged.

## 1.1.0 — Carbon refresh + birthright fold (2026-05-17)

A major release. Two large changes bundled into one cut:

### 1. Carbon refresh — sidebar shell + per-preset accents

Mackes' chrome was rebuilt to match the design at
`docs/design/v1.1.0-carbon-refresh/`. The old top-tab Notebook is gone;
in its place is a Carbon UI Shell with:

- 48px header strip (brand block + Workbench/Recovery/CLI mode buttons +
  preset chip + user@host)
- 256px grouped sidebar (Workbench / Configuration / Network / Apps &
  Maintenance / Reference) with badges and live-active highlighting
- Bottom 24px status bar (mesh/services/sshd/drift/version/preset)
- A floating **Tweaks** panel (bottom-right) for live preset swap,
  density (compact/cozy/comfortable), chrome toggles, and "Re-open
  Wizard" — state persists to `~/.config/mackes-shell/tweaks.json`.

The Dashboard is now Carbon stat tiles (mesh peers / services / sshd /
drift), a service-health grid, a Carbon notification for drift, a 2x2
hardware tile grid, six tertiary-style quick-action buttons, and a
mono-styled recent-activity log.

The **Mesh VPN panel** got a new Cairo-drawn topology view — control
node at center, peers in a ring around it, animated edge pulses
travelling along, dashed lines for DERP-relayed edges, click any peer
for a right-rail detail drawer. A toggle next to the section header
swaps between the topology view and the Carbon DataTable variant.

A 5th accent preset, **Node** (Carbon Green 50 #42be65), was added for
headless / server installs.

New files: `data/css/carbon-layout.css` (sidebar / topology / tile /
modal / topology / tweaks classes), `mackes/workbench/shell/`
(sidebar_window.py + tweaks_panel.py),
`mackes/workbench/network/mesh_topology.py` (Cairo widget),
`data/css/accents/node.css`.

### 2. Birthright fold — 8 new wizard apply steps

The audit in conversation 2026-05-17 found 7 items the wizard *should*
do at first run but didn't. They're now wired in. The wizard's apply
pipeline went from 10 steps to 18:

  Snapshot → Appearance → Devices → System → Network → Panel →
  **Themes → Fonts → Apps → Panel layout → Boot splash → System update →
  Third-party repos → Flathub** → Mesh → VPN import → Menu → Finalize

- **Themes**: copy `data/themes/PadOS/` and `data/icons/Carbon/` to
  `/usr/share/themes/` and `/usr/share/icons/`; rebuild GTK icon cache.
- **Fonts**: dnf install `ibm-plex-sans-fonts` + `ibm-plex-mono-fonts`;
  rebuild fontconfig cache.
- **Apps**: process `preset.apps.install` (install_curated_set) and
  `preset.apps.remove_bloat` (remove_packages). These lists already
  existed in every preset YAML but were never run.
- **Panel layout**: write the Mackes default xfce4-panel xfconf layout
  — Whisker Menu + Docklike + spacer + systray + IBM Plex clock — and
  `xfce4-panel --restart`.
- **Boot splash**: install + activate the Mackes Plymouth theme
  (centered logo on Carbon Gray 100 with an accent progress strip);
  regenerates initrd via `plymouth-set-default-theme mackes -R`.
- **System update**: `dnf upgrade -y --refresh` (heaviest step).
- **Third-party repos**: install `fedora-workstation-repositories`
  (Chrome/Steam/NVIDIA repo files, disabled by default) plus enable
  RPM Fusion free + nonfree for the detected Fedora version.
- **Flathub**: add the per-user Flathub remote via
  `flatpak remote-add --if-not-exists --user flathub …`.

All 8 are idempotent (re-runnable via Maintain → Reset to Preset) and
live in the new `mackes/birthright.py` module.

### Fixes

- `xfconf_bridge.XfconfBridge.set` int/float coercion (1.0.4
  hotfix folded in): subprocess.check_call won't accept non-string argv
  members, so int/float values now stringify before the subprocess call.
- App installer's per-app output now reads `App: installed (npm)` /
  `App: FAILED (rc=N) (npm)` instead of the always-on `rc={rc}` form.
- Cursor's stale `download.cursor.sh` URL replaced with a runtime
  resolver against `cursor.com/api/download`.
- `neofetch` (archived upstream) is installed as `fastfetch` (its
  maintained successor) under the same catalog name.

## 1.0.5 — fix Cursor + neofetch installs, clearer output (2026-05-17)

App installer fixes after observing the wizard-time install output:

    Cursor: appimage rc=1
    <urlopen error [Errno -2] Name or service not known>
    Claude Code CLI: npm install rc=0
    changed 2 packages in 2s
    neofetch: dnf install rc=1

* **Cursor**: the hardcoded `download.cursor.sh` URL was dead — Cursor
  retired that subdomain. Replaced with a runtime resolver that calls
  `https://www.cursor.com/api/download?platform=linux-x64&releaseTrack=stable`
  (which needs a non-empty User-Agent or returns 400) and pulls the
  current `downloadUrl` out of the JSON. The User-Agent is passed on
  the AppImage download request as well.

* **neofetch**: archived upstream in 2024, dropped from Fedora 44 repos.
  The catalog entry still accepts the name `neofetch` (so existing
  preset YAMLs keep working) but installs the maintained successor
  `fastfetch` instead. A separate `fastfetch` catalog entry was added
  for explicit selection.

* **Output**: per-app install lines now read `App: installed (npm)` on
  success and `App: FAILED (rc=N) (npm)` on failure instead of the
  always-on `App: npm install rc=N` form, which looked
  indistinguishable between success and failure.

## 1.0.4 — fix xfconf_bridge int/float coercion (2026-05-17)

After installing 1.0.3 and running the wizard, three provisioner steps
all failed with the same exception:

    →  Appearance
       ERROR: expected str, bytes or os.PathLike object, not int
    →  System
       ERROR: expected str, bytes or os.PathLike object, not int
    →  Panel
       ERROR: expected str, bytes or os.PathLike object, not int

Root cause in `mackes/xfconf_bridge.py::XfconfBridge.set`: when `value`
was an `int` (e.g. `cursor_size`, `workspace_count`, `/notify-location`)
or a `float` and no `type_hint` was given, the code set the right
`--type` flag but forgot to stringify `value`. The `int`/`float` then
went straight into the `subprocess.check_call` argv list, which only
accepts `str | bytes | os.PathLike`, so subprocess refused it before
xfconf-query was ever invoked.

Fix: in the int branch, `value = str(int(value))`; in the float branch,
`value = repr(float(value))`. The bool/string branches already
stringified correctly; explicit-type-hint callers already get
`value = str(value)`.

Verified with a 5-call regression test (bool / int / float / str /
explicit-type-hint) — all reach subprocess with str-only argv.

## 1.0.3 — fix MackesApp import (2026-05-17)

Install + launch flow surfaced an ImportError immediately after install:

    ImportError: cannot import name 'MackesApp' from 'mackes.app'
        File "mackes/__main__.py", line 14, in <module>
            from mackes.app import MackesApp

When `mackes.app` was refactored in 1.0 to lazy-import GTK (so headless
installs don't drag GTK into memory), the `MackesApp` class moved inside
an internal `_make_gui_app()` builder function — no longer a top-level
symbol. `mackes/__main__.py` still expected the old top-level import.

Fix: `__main__.py` now delegates to `mackes.app.main(argv[1:])` directly.
The `--uninstall` / `--yes` fast-path is preserved (still handled in
__main__ so the uninstall sequence can run without going through the
GUI router). Everything else — `--gui`, `--headless`, subcommands,
auto-detection — goes through `mackes.app.main`, which already knows
how to instantiate the GUI when it needs to.

Verified: `python3 -m mackes --version` prints `mackes 1.0.3`;
`python3 -m mackes help` prints the topic list.

## 1.0.2 — headscale.service file conflict (2026-05-17)

`dnf install` failed on the v1.0.1 RPM with:

    file /usr/lib/systemd/system/headscale.service conflicts between
    attempted installs of mackes-shell-1.0.1-1.fc44.x86_64
    and headscale-0.28.0-1.fc44.x86_64

The upstream `headscale` RPM (which we Require) ships its own
`headscale.service` at the same path. We were shipping a near-identical
copy with two extra knobs (MemoryHigh/MemoryMax). Fixed by dropping our
copy from the RPM — the upstream unit is used as-is.

`data/systemd/headscale.service` stays in the source tree as a reference
template. To apply Mackes-specific resource limits at deploy time, drop
a systemd override at `/etc/systemd/system/headscale.service.d/mackes.conf`
with the desired directives.

No code changes.

## 1.0.1 — Fedora 44 dep hotfix (2026-05-17)

`curl … install.sh | bash` was failing on stock Fedora 44 because three
of the spec's `Requires:` resolved to packages that don't exist on F44
under those names. Fixed:

- `Requires: xfce4-power-manager-plugin` → `Requires: xfce4-power-manager`
  (the panel plugin ships inside the parent package as
  `libxfce4powermanager.so`; there's no separate plugin RPM)
- `Requires: sshfs` → `Requires: fuse-sshfs` (Fedora-specific name)
- `Recommends: jellyfin-media-player` → removed (not in Fedora repos;
  users install via Flathub instead). Mackes' Media-Hub discovery still
  surfaces Jellyfin servers on the mesh whether or not a local native
  client is installed.

No code changes. RPM spec + version bump only.

## 1.0.0 — "XFCE Provisioner" (2026-05-16)

### Identity
- First non-private release. "MAP2 Sub Testing" markers fully removed across
  packaging, spec, and runtime UI.
- Repositioned from "shell stack manager" to "XFCE provisioner + mesh fabric".

### The XFCE Pivot (Q1–Q20 survey)
- Retired the Polybar / Plank / Rofi / picom / dunst shell stack entirely.
  Mackes now provisions a standard XFCE shell: xfce4-panel + xfdesktop +
  xfce4-appfinder + xfce4-notifyd, with Whisker Menu as the start menu and
  Docklike Taskbar replacing Window Buttons.
- Standard panel layout: Whisker (far-left) → Docklike taskbar → systray →
  volume → power → clock (IBM Plex Sans).
- PadOS locked as the default GTK theme; other themes greyed-out in the
  Appearance picker.
- Carbon Icons (Apache 2.0) as the system-wide GTK icon theme (replaced
  the briefly-considered Clarity icons).
- IBM Plex Sans (UI) + IBM Plex Mono (monospace) replace SF Pro / JetBrains
  Mono throughout.
- `branding/standard-wallpaper.png` is the locked desktop + LightDM greeter
  wallpaper, vendored at 7.8 MB.
- Bloat list collapsed to a single combined `remove_bloat` per preset; XFCE
  extras (asunder, parole, pragha, xfburn, transmission-gtk, claws-mail,
  pidgin) added alongside GNOME-on-XFCE apps + libreoffice-*.
- `menulibre` added to install lists for hashbang / mackes / daylight.
- ssh enabled by default on every Mackes install via RPM %post.
- LightDM greeter silently configured to match preset theme/wallpaper/font.

### Carbon Design System chrome (Q-CB1–Q-CB10)
- Pixel-exact Gray 100 palette (#161616 / #262626 / #393939 / #525252 /
  #f4f4f4 / #969696 / #2d2d30).
- Carbon UI Shell layout: 48px top header + 256px left side nav + main +
  24px status bar.
- IBM Plex Sans UI / IBM Plex Mono monospace.
- Per-preset accent (hashbang-red etc.) replaces Carbon blue at every
  focus/highlight surface.
- Carbon Icons everywhere (chrome + system theme).
- Strict 8px grid via `--cds-spacing-01` … `--cds-spacing-13` tokens; CI
  lint rejects raw `px` in `data/css/*.css`.
- Centralized design tokens in `data/css/tokens.css`.
- Full custom widget library locked in `mackes/carbon/`: Tile, DataTable,
  Accordion, NumberInput, MultiSelect, Notification, Toast, Modal,
  Skeleton, Button (5-tier), UIShell.

### Mesh fabric (§8.10–§8.14)
- **Mesh Thunar Extension** (Q-MX1–Q-MX20): `mesh:///` GVFS backend +
  Tumbler thumbnailer. Four subtrees — Peers (SSHFS, live), Clipboard
  (NATS-backed, 100-item ring + Saved/), Notifications (.md per entry),
  Object Store (Themes / Snapshots / Presets / Drop). Live updates via
  qnmd→FUSE inotify. 16-peer cap.
- **Mesh VPN** (§8.11): Headscale + Tailscale clients. Auto-elected
  control node with NATS-state replication + 30s snapshot. Tailscale-
  bootstrap (Option C) for cross-network discovery — only seed peer signs
  into Tailscale's free tier (1/100 node count forever).
- **Headless Node Mode** (§8.12, Q-HL1–Q-HL7): full `mackes init` /
  `mackes join` / `mackes status` / etc. CLI parity with the GUI panels.
  Auto-detect missing display + logind graphical session. New
  `data/presets/node.yaml` headless preset. `mackes-node.service` systemd
  unit.
- **Mesh Media Services** (§8.13, 5 layers): raw URLs / Media Hub panel /
  Caddy gateway / bundled native clients / mDNS-over-mesh relay. Shared
  catalog `data/media-services.yaml` consumed by all layers.
- **Mesh SSH** (§8.14, 3 layers): SSH cheatsheet + auto-distributed
  ed25519 keys via NATS + Tailscale-SSH identity-based access via
  Headscale. Audit log in NATS `mesh.ssh-audit`.

### Help / Documentation
- New comprehensive Help system: `docs/help/*.md` covers every feature.
  Surfaced via a Help tab in the workbench and `mackes help [topic]` in
  headless mode.

### Removals
- Deleted: `mackes/polybar_catalog.py`, `mackes/polybar_gen.py`,
  `mackes/shell_profiles.py`, `mackes/session_manager.py`,
  `mackes/workbench/shell/{polybar,plank,rofi,panel_visibility}.py`,
  `mackes/wizard/pages/shell.py`,
  `tests/test_{polybar_catalog,shell_profiles,shell_profiles_save}.py`.
- Deleted directories: `data/shell-profiles/` (8.7 MB of adi1090x families),
  `data/plank-themes/` (440 KB of dock themes).
- Net cleanup: ~1,200 file deletions; -631 / +191 lines across surviving
  source files.

### Packaging
- RPM hard `Requires`: xfce4-session, xfce4-whiskermenu-plugin,
  xfce4-docklike-plugin, xfce4-pulseaudio-plugin,
  xfce4-power-manager-plugin, openssh-server, headscale, tailscale.
- `Recommends`: caddy, jellyfin-media-player, strawberry,
  ibm-plex-sans-fonts, ibm-plex-mono-fonts, firewalld, pulseaudio-utils.
- Dropped: polybar, plank, rofi, dunst, picom, papirus-icon-theme,
  arc-theme, google-droid-sans-fonts, jetbrains-mono-fonts.

## Unreleased (post-0.1.1 redesign)

### Identity

- Stripped "PRIVATE WORK / Sub Testing Release" from dashboard, wizard,
  and About dialog. Mackes Shell is no longer marked as private testing
  in user-visible copy.
- Reimagined first-run wizard as a 3-act ceremony (Welcome → Pick a
  preset → Narrated apply). Welcome is spare (logo + 3 sentences + one
  details disclosure). Preset pick is a 4-card grid with wallpaper
  thumbnails. Apply has a dynamic title that transforms from "Becoming
  <preset>…" to "You are now <preset>."

### Presets

- Replaced single `chupre.yaml` with **four presets**:
  `hashbang` (display `#!`, default), `mackes`, `daylight`, `vanilla`.
- Each preset ships its own polybar, plank, and rofi profiles.
- Per-preset wallpapers in `data/wallpapers/`.
- `DEFAULT_PRESET_NAME = "hashbang"` — Mackes' first impression is the
  CrunchBang reincarnation.

### Design system

- SF Pro fonts installed and wired as the GUI default.
- `data/css/mackes.css` defines `.mackes-panel-title`,
  `.mackes-section-header`, `.mackes-info`, `.mackes-row-label`.
- `data/css/accents/<preset>.css` swaps `@define-color mackes_accent`
  per active preset.
- `app.py` loads base CSS + per-preset accent at startup, process-scoped.
- Monospace surfaces (log viewers, action streams) preserved with
  JetBrains Mono / Iosevka / Fira Code fallback.

### Polybar Editor (replaces preset-picker)

- New `mackes/polybar_catalog.py` — discovers 21 vendored adi1090x
  families across `simple/` and `bitmap/` variants.
- New `mackes/polybar_gen.py` — pure-function config generator with CLI
  (`python3 -m mackes.polybar_gen --theme <family>`).
- New editor panel: theme picker + geometry knobs + 3-zone DnD module
  editor with cross-zone drag + add-module popover + save-as-profile +
  copy-to-clipboard + live debounced apply (~300 ms).
- 8.7 MB upstream vendor (simple + bitmap, GPL-3.0, no fonts/wallpapers).

### MaintenanceKit

- **System Update** — pkexec dnf-upgrade wrapper with streaming log
- **Drift** — first-class drift surface with per-key revert/adopt/ignore
- **Fonts** — fc-list browser with Pango preview + dnf quick-install set
- **Power** — power-profiles-daemon selector + tlp summary fallback
- **Resources** — CPU / RAM / disk cards, 1.5 s live refresh, /proc-based

### Recovery shell

- New `mackes/recover.py` — TTY-driven snapshot picker
  (`python3 -m mackes.recover` / `--list` / `--latest`)
- `data/systemd/mackes-recovery.target` — multi-user + network target
- `data/grub/40_mackes_recovery` — GRUB submenu source
- `install-helpers/install-recovery.sh` — root-needed installer

### Update mechanism

- `data/dnf/mackes-shell.repo` — dnf repo manifest pointing at
  `https://matthewmackes.github.io/MAP2-RELEASES/fedora/$releasever/$basearch`
- `install-helpers/add-mackes-repo.sh` — drops the .repo into
  `/etc/yum.repos.d/`

### ISO build

- `packaging/iso/mackes-xfce.ks` — Fedora kickstart with mackes-shell
  baked in, polybar/plank/rofi/dunst/picom stack, dnf repo wiring,
  recovery shell wiring
- `make iso` target wrapping `livemedia-creator`

### Tests + dev tooling

- 20 passing tests including 9 new ones for polybar catalog/gen, plus
  CSS resolution, shell-profile save plumbing, recovery CLI
- `tests/_run_without_pytest.py` — runs the suite without pytest
  installed (handy fallback for fresh Fedora boxes)
- `make test-nodeps` target

### Headless apply

- `python3 -m mackes.cli_apply --preset NAME` — re-apply a preset
  without the GUI (SSH, automation, recovery flows)

### Documentation

- README rewritten to reflect actual feature surface (was a skeleton-
  status placeholder)
- `packaging/iso/README.md` — kickstart build docs
- `data/shell-profiles/polybar/upstream/ATTRIBUTION.md` — GPL-3.0
  attribution + refresh procedure

## 0.1.1

Initial single-binary skeleton with placeholder panels and the chupre
preset baseline. (Pre-redesign state captured in the original `docs/`
folder.)
