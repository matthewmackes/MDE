# Changelog

All notable user-facing and architectural changes. The current line is
unreleased; tag versions get a date when they ship.

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
