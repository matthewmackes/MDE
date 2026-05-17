# Mackes Shell — Master Specification

**Version:** 1.0.0 — "XFCE Provisioner"
**Successor to:** xfce11-unified v2.2 → Mackes Shell 0.2.0
**Status:** Design locked. The 1.0 pivot retired Polybar / Plank / Rofi / picom / dunst in favor of a standard XFCE shell (Whisker Menu, Docklike Taskbar, volume/power applets), Carbon Design System chrome, and a self-hosted mesh fabric (Headscale + Tailscale-bootstrap + NATS JetStream + SSHFS-over-QNM) with Thunar-integrated Mesh browser, distributed clipboard/notifications/services, headless-node mode, and identity-based SSH. Sections 8.10–8.15 document the mesh stack; the original survey decisions (§1) remain authoritative for the XFCE control-panel core.

---

## 0. What Mackes Shell Is

**Mackes Shell is the persistent control panel that replaces `xfce4-settings` as the daily interface for managing an XFCE-based Fedora workstation.**

It also serves two secondary roles:
- A first-run provisioner that brings a fresh machine to a known-good state via curated preset
- A drift monitor that warns when current configuration deviates from the active preset

It is **not** a configuration framework, not a fleet manager in the Ansible sense, and not a kitchen-sink desktop environment. It is a focused, opinionated, single-binary GTK app that swallows `xfce4-settings` plus the Xfce11 shell stack (Polybar / Plank / Rofi) and presents them as one coherent control surface.

---

## 1. Locked Design Decisions

The full design was settled across a 20-question survey. The complete table is included for posterity and as a reference when implementation drifts.

| # | Decision Area | Locked Answer |
|---|---|---|
| 1 | Tool identity | Persistent control panel replacing xfce4-settings as everyday interface |
| 2 | Replacement scope | Full xfce4-settings parity — Appearance + Shell + Hardware + Window Manager + Workspaces + Session/Startup + Notifications + Removable Media + Date/Time + Default Apps |
| 3 | Navigation model | Two-level hybrid — task-oriented top tabs, object panels inside each |
| 4 | Entry point | Single binary `mackes`; detects state and routes to wizard or workbench |
| 5 | Daily landing | Live status dashboard — service health, active profile, hardware summary, last snapshot, drift warnings, quick links |
| 6 | First-run flow | Full guided wizard (7+ screens) with live preview at each step |
| 7 | Profile model | Single curated preset (`chupre`). Earlier shipped presets (Workstation / Laptop / Audio Rig / Server Console) removed in 0.1.1. User-preset overrides in `~/.config/mackes-shell/presets/` still respected. |
| 8 | Bootstrap | One-line `curl …/install.sh \| bash` |
| 9 | Settings application | Immediate apply — every change writes through instantly (xfce4-settings behavior) |
| 10 | Snapshot / rollback | Manual only — user clicks "Create Restore Point" |
| 11 | Visual direction | Native GTK look — follow active GTK theme |
| 12 | Shell config model | Preset picker only — dropdown of named profiles for Polybar / Plank / Rofi |
| 13 | Appearance layout | One unified Appearance panel with internal sections (Theme / Icons / Cursor / Fonts / Wallpaper) |
| 14 | CLI surface | GUI only — no separate CLI tool |
| 15 | Web Workbench | Deleted entirely |
| 16 | Implementation strategy | Embed xfconf properties directly — Mackes panels bind to xfconf keys; `xfsettingsd` applies live |
| 17 | Operations location | Dedicated **Maintain** tab — Snapshots, Health Check, Dependencies, Logs, Repair, Reset to Preset |
| 18 | Network / QNM placement | Sixth top tab **Network** — Wi-Fi/Ethernet, VPN, QNM, Firewall |
| 19 | xfce4-settings coexistence | Hide menu entries + redirect deep links; xfce4-settings + xfsettingsd remain installed under the hood |
| 20 | Packaging / delivery | Direct RPM download from GitHub Releases — no COPR maintenance burden |

---

## 2. Architecture Overview

```
                ┌────────────────────────────────────────────────────┐
                │                  mackes  (GTK3 / PyGObject)        │
                │                                                    │
                │  ┌─────────────┐    ┌───────────────────────────┐  │
                │  │ State probe │───▶│  First-run?    Wizard     │  │
                │  └─────────────┘    │  Installed?    Workbench  │  │
                │                     └───────────────────────────┘  │
                │                                                    │
                │  ┌──────────────────────────────────────────────┐  │
                │  │  Panel registry — Look & Feel / Shell /      │  │
                │  │  Devices / Network / System / Maintain       │  │
                │  └──────────────────────────────────────────────┘  │
                │                          │                         │
                │                          ▼                         │
                │             ┌──────────────────────┐               │
                │             │   xfconf wrapper     │               │
                │             │ (python-xfconf or    │               │
                │             │  xfconf-query shell) │               │
                │             └──────────────────────┘               │
                └──────────────────────────┼─────────────────────────┘
                                           │
                                           ▼
                                  ┌────────────────┐
                                  │     xfconf     │ ◀── persists config in XDG
                                  └────────┬───────┘
                                           │
                                           ▼
                                  ┌────────────────┐
                                  │  xfsettingsd   │ ◀── unchanged, kept running
                                  └────────┬───────┘
                                           │
                          ┌────────────────┼────────────────┐
                          ▼                ▼                ▼
                       xfwm4          (panel/dock)     fonts/cursor/theme
```

**Why this works:**
- `xfconf` is XFCE's configuration database. Every xfce4-settings dialog is a GTK form over xfconf.
- `xfsettingsd` watches xfconf and pushes changes live to xfwm4, the cursor, fonts, etc. No restart required.
- Mackes panels are GTK forms over the same xfconf keys. When a user toggles something, Mackes writes xfconf, xfsettingsd notices, the desktop updates.
- xfce4-settings stays installed but its menu entries are hidden — its dialogs still function if launched manually, and any third-party tool that reads xfconf still works.

**What Mackes adds on top of xfconf:**
- The Xfce11 shell layer (Polybar / Plank / Rofi) — managed via file-based configs, since xfconf doesn't cover these
- Preset definitions and apply logic (`mackes/presets_engine.py`)
- Snapshot/restore (file-tree backup of `~/.config/xfce4/`, `~/.config/polybar/`, `~/.config/plank/`, `~/.config/rofi/`, plus xfconf channel dumps)
- Drift detection (compare current state to active preset)
- Maintenance operations (health checks, dependency install, repair)
- QNM integration (existing QNM daemon, exposed in the Network tab)

---

## 3. Navigation Structure

```
mackes
├── Dashboard                      (landing — not a tab, the home view)
│
├── Look & Feel
│   └── Appearance                 (one unified panel)
│       ├── Theme                  (GTK theme picker, dark/light toggle)
│       ├── Icons                  (icon theme picker)
│       ├── Cursor                 (cursor theme + size)
│       ├── Fonts                  (UI font / monospace / TTY console font + sizes)
│       └── Wallpaper              (per-monitor wallpaper + style)
│
├── Shell
│   ├── Polybar                    (preset picker only)
│   ├── Plank                      (preset picker only)
│   ├── Rofi Launcher              (preset picker only)
│   └── Panel Visibility           (toggle xfce4-panel autostart on/off + rollback)
│
├── Devices
│   ├── Display                    (xfconf: /Display)
│   ├── Keyboard                   (xfconf: /Keyboard + shortcuts)
│   ├── Mouse & Touchpad           (xfconf: /Pointers)
│   ├── Sound                      (PulseAudio default sink/source picker)
│   └── Power                      (xfconf: xfce4-power-manager channel)
│
├── Network
│   ├── Wi-Fi / Ethernet           (nmcli-backed status + connect/disconnect)
│   ├── VPN                        (NetworkManager VPN list + import .ovpn/.conf)
│   ├── Quick Network Mesh         (QNM enable/disable, GUI launcher, status)
│   └── Firewall                   (firewalld zone + service list)
│
├── System
│   ├── Window Manager             (xfconf: /xfwm4)
│   ├── Workspaces                 (xfconf: /xfwm4 workspace count + names)
│   ├── Session & Startup          (xfconf: /xfce4-session + autostart + managed-process supervisor)
│   ├── Notifications              (xfconf: /xfce4-notifyd)
│   ├── Default Apps               (mimeapps.list)
│   ├── Removable Media            (xfconf: /thunar-volman)
│   └── Date & Time                (timedatectl wrapper)
│
├── Apps                            ← new 7th tab (C1 lock)
│   ├── Install                    (curated set per active preset's apps.install)
│   ├── Remove                     (Fedora bloat + 'XFCE components replaced by Mackes')
│   └── Installed                  (searchable rpm -qa with per-row remove)
│
└── Maintain
    ├── Snapshots                  (list / create / restore / delete)
    ├── Health Check               (preflight + validate, unified)
    ├── Dependencies               (missing/optional package list + install button)
    ├── Logs                       (tail mackes.log + xfsettingsd journal)
    ├── Repair                     (re-apply current preset; rebuild menu folder; restore xfce4-settings menu entries; rewrite Polybar launcher + autostart)
    ├── Reset to Preset            (revert all local changes to the active preset's defaults)
    └── Uninstall                  ← new sub-panel (Q8 lock) — see §8.7
```

Seven top tabs, ~28 second-level panels. Every panel has one job.

---

## 4. The Dashboard (daily landing)

A single scrolled view, top-to-bottom:

### 4.1 Status strip (always visible at top)

A horizontal row of compact status badges:

```
[●] Polybar running    [●] Plank running    [●] Rofi installed    [●] xfsettingsd alive    [●] xfconf reachable
Preset: Audio Rig                Last snapshot: 2h ago "before-display-rearrange"
```

Green dot = ok. Amber = warning. Red = action needed.

### 4.2 Drift card (only shown when drift exists)

```
┌────────────────────────────────────────────────────────────────────────┐
│  ⚠  Configuration drift from "Audio Rig" preset                        │
│                                                                        │
│   • GTK theme is "Arc-Dark", preset specifies "Mac-Dark"               │
│   • Polybar profile is "Custom-1", preset specifies "Icon-Only"        │
│   • 2 autostart entries not in preset                                  │
│                                                                        │
│   [Review drift]   [Reset to preset]   [Save current as new preset]    │
└────────────────────────────────────────────────────────────────────────┘
```

### 4.3 Hardware summary card

Hostname, kernel, CPU, RAM, audio device, monitors, network interfaces. Read once on launch, refresh on demand.

### 4.4 Quick actions row

Six big buttons for the most-used operations:

```
[ Open Appearance ]  [ Switch Theme ]  [ Switch Polybar Profile ]
[ Create Snapshot ]  [ Health Check ]  [ Open Log ]
```

### 4.5 Recent actions

Last 5 changes Mackes made, with timestamps. Each clickable to open its panel.

---

## 5. First-Run Wizard

When `mackes` launches and detects the system has never been provisioned (no `~/.config/mackes-shell/state.json` or no installed-marker), the wizard runs. Live preview at every step means changes apply as you toggle them, and Back un-applies cleanly.

```
Screen 1  ┃  Welcome           — what Mackes is, what it will do, key bindings
Screen 2  ┃  Environment Scan  — detected hardware, Fedora version, XFCE version,
          ┃                      missing/optional packages, current XFCE panel state
Screen 3  ┃  Preset Selection  — Workstation / Laptop / Audio Rig / Server Console
          ┃                      cards with description and preview thumbnail
Screen 4  ┃  Appearance        — theme/icons/font choices for this preset; live preview
Screen 5  ┃  Shell Layout      — Polybar profile, Plank dock, Rofi launcher; live preview
Screen 6  ┃  Hardware          — display arrangement, default audio output, power profile
Screen 7  ┃  Network           — enable QNM Y/N, firewall stance, VPN import (optional)
Screen 8  ┃  Snapshot Policy   — create initial restore point Y/N, name it
Screen 9  ┃  Review            — full diff of what will be applied
Screen 10 ┃  Apply             — progress bar streaming actions; on success drop into
          ┃                      the Dashboard with a "Welcome to Mackes" overlay
```

The wizard is exit-able at any point (with a confirm prompt). On exit, whatever was already applied stays applied.

---

## 6. Profile Presets

Each preset is a YAML file in `data/presets/`. Presets are read-only at runtime; they define the *target* state. Daily changes via the Workbench update the live system but **never** modify the preset file — that's why drift exists.

### 6.1 Preset schema

The single shipped preset is `data/presets/chupre.yaml` — reproduced here verbatim
as the schema reference (Q33 lock — the shipped YAML *is* the example):

```yaml
name: chupre
display_name: "Chupre (default)"
description: >
  Dark, modern, polished. WhiteSur-Dark theme, Inter Nerd Font UI, the chupre
  Polybar with workspaces / window title / weather / battery / volume / power
  modules, matching Rofi launcher. Adapted for XFCE.

appearance:
  gtk_theme: "WhiteSur-Dark"
  icon_theme: "WhiteSur-dark"
  cursor_theme: "XCursor-Pro-Dark"
  cursor_size: 24
  font_ui: "Inter Nerd Font 11"
  font_monospace: "SFMono Nerd Font 11"
  wallpaper: "/usr/share/mackes-shell/wallpapers/chupre.jpg"

shell:
  polybar_profile: "chupre-custom"
  plank_profile:   "chupre"
  rofi_profile:    "chupre"
  xfce_panel_enabled: false

devices:
  display_scaling:    "auto"
  power_profile:      "balanced"
  audio_default_sink: "pipewire"

network:
  qnm_enabled:           false
  firewall_default_zone: "FedoraWorkstation"

system:
  workspace_count:       9
  window_manager_theme:  "Default"
  notifications_enabled: true
  autostart_extras:
    - picom.desktop

apps:
  install:            [filezilla, terminator, vlc, microsoft-edge-stable,
                       code, cursor, claude-code, remmina, mc, neofetch, dunst]
  remove_bloat:       [gnome-software, gnome-tour, gnome-maps, gnome-weather,
                       gnome-contacts, gnome-clocks, gnome-calendar,
                       gnome-music, totem, "libreoffice-*"]
  lean_xfce_remove:
    - {package: xfce4-panel,     replaced_by: polybar}
    - {package: xfce4-appfinder, replaced_by: rofi}
    - {package: xfdesktop,       replaced_by: plank}
    - {package: xfce4-notifyd,   replaced_by: dunst}

snapshot:
  initial_snapshot_name: "chupre-baseline"
```

### 6.2 Shipped presets

| Preset | Targeted at | Distinguishing choices |
|---|---|---|
| **Chupre** | Default for every install | WhiteSur-Dark theme, chupre-custom Polybar, matching Rofi launcher, 9 workspaces, picom autostart, Inter / SFMono Nerd Fonts |

Earlier shipped presets (Workstation / Laptop / Audio Rig / Server Console)
were removed in 0.1.1 — the wizard's preset-pick screen is auto-skipped when
only one preset is shipped (Q2 lock). Custom user presets remain undocumented
but supported via `~/.config/mackes-shell/presets/` (Q3, Q31 locks). When a
user-local preset exists, the wizard reinstates the preset-pick screen so the
user can choose between chupre and their own.

---

## 7. File-System Layout

### 7.1 Source tree (the repo)

```
mackes-shell/
├── README.md
├── LICENSE
├── install.sh                     # the curl-bootstrap target
├── packaging/
│   └── fedora/
│       └── mackes-shell.spec      # RPM spec
├── data/
│   ├── presets/                   # shipped preset YAML files
│   │   ├── workstation.yaml
│   │   ├── laptop.yaml
│   │   ├── audio-rig.yaml
│   │   └── server-console.yaml
│   ├── shell-profiles/            # Polybar/Plank/Rofi config templates
│   │   ├── polybar/
│   │   │   ├── icon-only.ini
│   │   │   ├── power-user.ini
│   │   │   ├── mac-style.ini
│   │   │   └── minimal.ini
│   │   ├── plank/
│   │   │   ├── standard.dock
│   │   │   ├── minimal.dock
│   │   │   └── intellihide.dock
│   │   └── rofi/
│   │       ├── black-droid.rasi
│   │       └── default.rasi
│   ├── wallpapers/
│   ├── icons/
│   │   └── mackes-shell.svg
│   ├── applications/
│   │   └── mackes-shell.desktop   # the only menu entry Mackes installs
│   └── overrides/
│       └── xfce-settings-hidden/  # .desktop overrides to hide xfce4-settings menu entries
├── mackes/                        # the Python package
│   ├── __init__.py
│   ├── __main__.py                # `python -m mackes`
│   ├── app.py                     # Gtk.Application, state routing
│   ├── state.py                   # install state, active preset, drift detection
│   ├── xfconf_bridge.py           # xfconf read/write wrapper
│   ├── presets.py                 # preset load + apply engine
│   ├── snapshots.py               # snapshot create/list/restore
│   ├── shell_profiles.py          # Polybar/Plank/Rofi profile apply
│   ├── menu_integration.py        # hide xfce4-settings entries, install mackes menu
│   ├── qnm_bridge.py              # talk to the existing QNM daemon
│   ├── wizard/
│   │   ├── __init__.py
│   │   ├── window.py              # wizard Assistant
│   │   └── pages/
│   │       ├── welcome.py
│   │       ├── env_scan.py
│   │       ├── preset_pick.py
│   │       ├── appearance.py
│   │       ├── shell.py
│   │       ├── hardware.py
│   │       ├── network.py
│   │       ├── snapshot.py
│   │       ├── review.py
│   │       └── apply.py
│   └── workbench/
│       ├── __init__.py
│       ├── window.py              # Workbench main window
│       ├── dashboard.py
│       ├── look_and_feel/appearance.py
│       ├── shell/
│       │   ├── polybar.py
│       │   ├── plank.py
│       │   ├── rofi.py
│       │   └── panel_visibility.py
│       ├── devices/
│       │   ├── display.py
│       │   ├── keyboard.py
│       │   ├── mouse.py
│       │   ├── sound.py
│       │   └── power.py
│       ├── network/
│       │   ├── wifi.py
│       │   ├── vpn.py
│       │   ├── qnm.py
│       │   └── firewall.py
│       ├── system/
│       │   ├── window_manager.py
│       │   ├── workspaces.py
│       │   ├── session.py
│       │   ├── notifications.py
│       │   ├── default_apps.py
│       │   ├── removable.py
│       │   └── datetime.py
│       └── maintain/
│           ├── snapshots.py
│           ├── health_check.py
│           ├── dependencies.py
│           ├── logs.py
│           ├── repair.py
│           └── reset_to_preset.py
└── tests/
    └── …
```

### 7.2 Runtime paths (where Mackes writes on the user's machine)

```
~/.config/mackes-shell/
├── state.json                     # install state, active preset, last apply timestamp
├── presets/                       # user-added presets (optional, undocumented)
└── overrides/                     # backup of any xfce4-settings .desktop entries Mackes hid

~/.local/share/mackes-shell/
├── logs/
│   └── mackes.log                 # unified log
├── snapshots/
│   └── 2026-05-15T142300_pre-display/
│       ├── manifest.json
│       ├── xfconf/                # xfconf-query --channel xx --dump
│       ├── polybar/               # copy of ~/.config/polybar
│       ├── plank/                 # copy of ~/.config/plank
│       └── rofi/                  # copy of ~/.config/rofi
└── data/                          # cached/derived runtime data
```

System install paths (from RPM):

```
/usr/bin/mackes
/usr/lib/python3.X/site-packages/mackes/...
/usr/share/mackes-shell/data/...   (presets, shell-profiles, wallpapers)
/usr/share/applications/mackes-shell.desktop
/usr/share/icons/hicolor/scalable/apps/mackes-shell.svg
```

---

## 8. Implementation Notes (per-area)

### 8.1 xfconf binding

Each panel inherits a base `XfconfPanel` class:

```python
class XfconfPanel(Gtk.Box):
    channel: str        # e.g. "xsettings"

    def bind(self, widget, key: str, default=None):
        """Two-way bind a GTK widget to an xfconf key. Immediate apply."""
```

The bridge uses `python-xfconf` if available (preferred — it gets DBus change notifications for free) or falls back to `xfconf-query` subprocess. All writes are immediate (Q9). Mackes never batches.

### 8.2 Shell profiles (Polybar / Plank / Rofi)

Profiles are config-file blobs in `data/shell-profiles/`. Applying = copy file to `~/.config/<tool>/`, then signal the running daemon (Polybar: `killall -USR1 polybar` or full restart via launcher script; Plank: dconf overwrite; Rofi: no daemon to signal).

Polybar launcher script (`~/.local/bin/mackes-polybar-launch.sh`) is owned by Mackes and re-generated on profile switch.

### 8.3 Snapshots

A snapshot is a timestamped directory under `~/.local/share/mackes-shell/snapshots/`. Contents:
- xfconf dumps of all known channels (`xfconf-query --channel X --dump`)
- Copies of `~/.config/polybar/`, `~/.config/plank/`, `~/.config/rofi/`
- Copy of `~/.config/xfce4/panel/` (for full XFCE panel restore)
- A `manifest.json` with timestamp, optional user-supplied name, source preset, hostname

Restore = wipe live config dirs, copy snapshot contents back, `xfconf-query --channel X --load` each dump file, signal xfsettingsd.

### 8.4 Drift detection

On dashboard load, compare the active preset's declared values to the current system state. Each preset YAML field has a corresponding "read current value" function. Mismatches go into the drift card (§4.2). Drift detection is read-only and informational — no automatic remediation.

### 8.5 Menu integration

`menu_integration.py` does on Mackes install:

1. Write `~/.local/share/applications/xfce-display-settings.desktop` etc. with `NoDisplay=true` (XDG override that hides without removing). One override per xfce4-settings .desktop.
2. Snapshot the originals to `~/.config/mackes-shell/overrides/` so `mackes maintain repair` can restore them.
3. Install `mackes-shell.desktop` as the new top-level Settings entry.
4. Register deep-link mime handlers so `xfce4-display-settings &` routed via `xdg-open` opens the Mackes Display panel. (Optional, nice-to-have.)

On Mackes uninstall, the RPM scriptlet restores the originals.

### 8.6 QNM bridge

The existing `qnmctl` / `qnmd` / `qnm-gui` from v2.2 stay in their own service unit and binary. Mackes' Network → QNM panel just calls `qnmctl status`, exposes start/stop/restart, and embeds the existing GUI as a launcher button. No QNM logic moves into Mackes itself. RPM relation is `Recommends: qnm` (C2/Q38) — soft, not hard.

### 8.7 Uninstall (Maintain → Uninstall, `mackes --uninstall`)

Locked across Q8–Q30 and X5. The uninstall path lives in `mackes/uninstall.py` and is exposed two ways: the **GUI panel** at Maintain → Uninstall (streaming log + progress bar + single-checkbox consent + post-uninstall logout countdown dialog) and the **CLI flag** `mackes --uninstall` (which accepts `--yes` to bypass the interactive `UNINSTALL` confirmation).

Sequence (best-effort — every step records its own success/failure and the run continues):

1. **Pre-uninstall snapshot.** `snapshots.create_snapshot("pre-uninstall-<ts>")`, then tarball to `~/Desktop/mackes-shell-final-snapshot-<ts>.tar.gz` — the only artifact that survives the uninstall (Q11, Q12, Q13).
2. **Stop managed daemons.** `pkill -x polybar`, `pkill -x plank`, `pkill -x dunst`, `pkill -x picom` (Q16).
3. **Reinstall lean-XFCE.** Any packages Mackes removed during provisioning (tracked in `~/.config/mackes-shell/removed-by-mackes.json`) are reinstalled via `dnf install` so the user gets stock XFCE back (X5).
4. **Reset xfconf.** `xfconf-query --reset --root -r` on every known channel (xsettings, xfwm4, xfce4-desktop, xfce4-panel, xfce4-notifyd, xfce4-power-manager, xfce4-session, thunar-volman, keyboards, pointers), then `pkill -HUP xfsettingsd` (Q14, Q40).
5. **Re-enable xfce4-panel.** Drop the disabling autostart override, then `exec xfce4-panel` (Q17).
6. **Run `restore-xfce-settings.sh`** explicitly via pkexec/sudo to remove the `X-Mackes-Hidden=1` overrides from `/etc/skel/.local/share/applications/` (Q18).
7. **Delete user files.** `~/.config/mackes-shell/`, `~/.local/share/mackes-shell/`, snapshots, `~/.config/{polybar,plank,rofi,alacritty,gtk-3.0,gtk-4.0}/`, Polybar autostart .desktop, launcher script (Q15).
8. **Remove v2.2 leftovers.** Known path list: `~/xfce11-unified`, `~/Desktop/xfce11-unified`, `/opt/xfce11-unified`, `/usr/local/share/xfce11-unified`, `START-HERE-XFCE11-UNIFIED.desktop` (under `~/Desktop`, `/usr/share/applications`, `/usr/local/share/applications`). `quick-network-mesh/` is intentionally preserved (Q19, Q20, Q21).
9. **Remove the Mackes package.** Install mode is auto-detected (Q29) — `rpm -q mackes-shell` → `dnf remove`; `pip show mackes-shell` → `pip uninstall`; git checkout → no package removal (manual `rm -rf` is the user's job).
10. **Write the uninstall log** to `~/Desktop/mackes-shell-uninstall-<ts>.log` (Q27). Every line emitted to the streaming GUI view also lands here.
11. **Post-uninstall.** Show a "Log out in 10s — [Stay logged in]" dialog (Q25); on OK or expiry, fire `xfce4-session-logout --logout --fast` so the next login lands in a clean stock XFCE session.

The Releases artifact `uninstall.sh` (Q46) curl-pipe-bashes to either `mackes --uninstall --yes` (when Mackes is installed) or runs a standalone bash routine that cleans the v2.2 path list (Q47) — that's the only way to clean v2.2 residue from a machine where Mackes never landed.

### 8.8 App Management (Apps tab)

C1–C4 + C9 + C10 locks. Three sub-panels:

* **Apps → Install.** Renders the active preset's `apps.install` list. Per-row backend badge (`Fedora repo` / `third-party repo` / `AppImage` / `npm global`). Curated set: Filezilla, Terminator, VLC, Microsoft Edge, VS Code, Cursor, Claude CLI, Remmina, mc, neofetch, dunst. Third-party repos (Microsoft, VS Code) are added transparently via `pkexec bash -lc <snippet>` — every addition is logged.
* **Apps → Remove.** Two grouped subsections (X2 lock):
  - *Fedora bloat* — `apps.remove_bloat` from the preset. Default set: GNOME-on-XFCE apps + `libreoffice-*` (C9). Glob expressions stay verbatim; reinstall uses the same expression.
  - *XFCE components replaced by Mackes* — `apps.lean_xfce_remove`. Each entry declares its `replaced_by` daemon. A row is only marked *eligible for removal* when the replacement is actually running (X4) — so a user who hasn't started Polybar yet cannot accidentally remove `xfce4-panel`.
* **Apps → Installed.** `rpm -qa` browser with a filter box and per-row destructive remove.

Removals are tracked in `~/.config/mackes-shell/removed-by-mackes.json` with two categories — `bloat` and `lean_xfce`. `mackes --uninstall` reads `lean_xfce` to reinstall those packages (X5).

The catalog (`mackes/app_mgmt.py:CATALOG`) maps each curated name to a backend. Adding a new app means adding a `CATALOG` entry; the preset's `apps.install` list can name anything in the catalog or fall back to plain `dnf install <name>` for unknown names.

### 8.9 Session-manager extension

C6.a / C6.b / C11 locks. `mackes/session_manager.py` owns three responsibilities:

1. **Apply the chupre dotfiles bundle.** The bundle is staged under `data/shell-profiles/chupre/<subdir>/` and copied to `~/.config/<subdir>/` at preset-apply time. Applied subdirs: `alacritty`, `gtk-3.0`, `gtk-4.0`. Skipped: `i3`, `picom`, `sxhkd`, `nvim`, `networkmanager-dmenu` (not part of an XFCE session). Polybar / Plank / Rofi configs are owned by `shell_profiles.py` and not duplicated here.
2. **Supervise managed processes.** Registry: polybar, plank, dunst, picom. For each, `process_status()` returns `installed / running / pid`. Cheap pgrep — no background daemon.
3. **Surface state in the UI.** The Dashboard's status strip carries a "Managed:" row of dots (per-process); System → Session has a "Managed processes" section with Start / Stop / Restart per row.

The Polybar fix (P1) is wired through here: the launcher script `~/.local/bin/mackes-polybar-launch.sh` parses `[bar/<name>]` headers from the active profile's `.ini` (no more hardcoded `polybar mackes`) and redirects stderr to `~/.local/share/mackes-shell/logs/polybar.log`. An autostart entry `~/.config/autostart/mackes-polybar.desktop` ensures Polybar comes up on every login — that was the missing piece in the previous version.

### 8.10 Mesh Thunar Extension (Q-MX1–Q-MX20)

A 20-question survey locked the mesh-aware Thunar surface that unifies clipboard / notifications / shared files / NATS Object Store under a single `mesh:///` namespace, navigable from inside Thunar (and any GVFS-aware app).

**Entry & shape.**
- Three entry points — sidebar entry, `mesh://` URI, XDG bookmarks (Q-MX1).
- Root layout: `mesh:///` → 4 subtrees `Peers/`, `Clipboard/`, `Notifications/`, `Object Store/` (Q-MX2).
- Implementation: custom `gvfsd-mesh` Python GVFS backend (Q-MX16). Works in any GVFS-aware app — Thunar, file pickers, `gio`.

**File representation.**
- Clipboard items → `<ISO-ts>_<short-hash>.<ext>` with native MIME extension. Drag-out copies; double-click opens with system default (Q-MX3).
- Notifications → `<ts>_<peer>_<id>.md` with YAML frontmatter + body; attachments as siblings (Q-MX4).
- Object Store → one folder per bucket (`Themes/`, `Snapshots/`, `Presets/`, `Drop/`, …) (Q-MX5).
- Versions hidden by default; right-click → "Show versions…" lists revisions with Restore/Open (Q-MX13).

**Peer state.**
- Offline peers stay visible, greyed with timestamped "offline" badge + Reconnect button (Q-MX6).
- Pinned clipboard items move to a sibling `Saved/` folder (uncapped); ring stays 100 (Q-MX10).
- Notifications use bold + dot badge for unread; manual Delete propagates to originating peer (Q-MX11).

**Interaction.**
- qnmd pushes NATS events → FUSE inotify invalidation → live ~ms refresh (Q-MX7).
- Right-click menu: Copy to local · Send to peer… · Pin/Unpin · Delete from mesh · Save as File · View · Open (Q-MX8).
- Every desktop→mesh drop opens a destination picker (bucket + optional target peer) (Q-MX9).
- Mesh-wide search box at `mesh:///` root; per-folder uses native Ctrl+F (Q-MX12).

**Policy.**
- All 4 subtrees RW by default; per-item RO via right-click; per-peer overrides in Mackes → Network → QNM (Q-MX14).
- Last-write-wins on Object Store conflicts; older write preserved as a prior revision (Q-MX15).
- Hard cap **16 peers** — 17th add fails with a Carbon Toast (Q-MX18).
- Auto-subscribe everything from every peer; per-peer mute toggles in Mackes → Network → QNM (Q-MX20).

**Visual & cross-surface.**
- Custom Tumbler thumbnailer ships rich previews — image scaling, text/HTML, audio waveforms, Carbon-styled `.md` notification cards (Q-MX19).
- Mackes Dashboard "Mesh activity" card + xfdesktop right-click "Drop on mesh…" entry (Q-MX17).

**Layout cheat-sheet.**

```
mesh:///
├── Peers/
│   ├── peer-A/        [SSHFS, RW, per-peer accent badge]
│   ├── peer-B/        [offline · since 14:32 · Reconnect]
│   └── …              [≤16 total]
├── Clipboard/
│   ├── mine/
│   │   ├── 2026-05-16T14-32-08_a3f9.png   [100-item ring]
│   │   ├── 2026-05-16T14-31-44_c712.txt
│   │   └── Saved/                          [uncapped, pinned]
│   └── peer-A/
│       ├── …                               [100-item ring]
│       └── Saved/                          [uncapped, pinned]
├── Notifications/
│   ├── mine/  …_xx_<id>.md                 [bold = unread]
│   └── peer-A/ …
└── Object Store/
    ├── Themes/                             [versioned blobs]
    ├── Snapshots/
    ├── Presets/
    └── Drop/
```

**Components to build for this feature.**
1. `gvfsd-mesh` — Python GVFS backend (new package)
2. `mackes-thunarx-mesh.thumbnailer` — Tumbler thumbnailer for `.md` notification cards + clipboard items
3. `mesh.desktop` — sidebar entry .desktop file → `mesh:///`
4. qnmd `mesh-mirror` module — auto-subscribes to all peers' NATS subjects + Object Store buckets; maintains local cache
5. Mackes Dashboard "Mesh activity" card (Carbon Tile + Skeleton loader)
6. Mackes → Network → QNM → Mesh panel — per-peer mute toggles, mesh-wide RW/RO overrides
7. xfdesktop right-click integration — XDG desktop action for "Drop on mesh…"
8. Destination picker popover — Carbon Modal (small variant) for drag-drop targets

All Mackes-authored chrome inside this feature continues to use the Carbon Design System (Q-CB1–Q-CB10): Gray 100 palette, IBM Plex Sans/Mono, Carbon Icons, strict 8px grid, per-preset accent.

### 8.11 Mesh VPN (Headscale + Tailscale, made invisible)

The SSHFS and NATS substrates (§8.6, §8.10) only work when peers can reach each other on the network. The Mesh VPN layer provides stable virtual IPs and routes peers to each other regardless of physical network — home, coffee shop, CGNAT, behind corporate firewalls.

**Backend choice.** Headscale (self-hosted Tailscale control plane) + Tailscale clients. WireGuard data plane, DERP relays for NAT-traversal fallback, MagicDNS for hostname routing. Self-hosted: no third-party dependency on the control plane.

**Design principles.**
1. **Zero CLI exposure.** `headscale` and `tailscale` CLIs exist on disk but the user never touches them. Mackes wraps everything.
2. **Auto-elected control node.** First peer to install is the control node implicitly. Failover via NATS-heartbeat election with 120s grace period.
3. **mDNS for LAN joins, QR/link for cross-network joins.** No URLs or tokens for the user to copy by hand.
4. **State replicated to NATS.** Headscale state (peer registry, pre-auth keys, ACLs) checkpointed to a `mesh.vpn-state` Object Store bucket every 30s. Failover restores from the latest snapshot.
5. **Carbon-styled UI** at Mackes → Network → Mesh VPN: Tile per peer, Toast on join/leave, Modal for Add Peer, DataTable for the registry.

**Cross-network rendezvous: Tailscale-bootstrap (Option C).** Headscale's control plane handles ongoing operations but a fresh remote peer has no way to *find* an existing peer behind NAT. Mackes solves this by leveraging Tailscale's free coordination service for **first-contact rendezvous only**:

- **Only the seed peer** signs into Tailscale (during its wizard). Subsequent peers never need a Tailscale account.
- The seed peer's `tailscaled` runs continuously alongside `headscale` (separate state dir, separate network interface) — it exists *only* to keep the seed peer's current public endpoint registered in Tailscale's directory under tag `tag:mackes-<mesh-id>`.
- The seed generates a scoped Tailscale API key (read-only, restricted to that tag) and stores it in `mesh.vpn-state` so every existing peer can hand it to a joiner.
- Adding a remote peer: the QR / paste-link generated on any existing peer encodes `mesh-join://?code=412753&ts-key=<scoped-key>&seed-tag=mackes-<mesh-id>`. The remote peer queries Tailscale's REST API with the scoped key, retrieves the seed's STUN-discovered public endpoint, contacts it (DERP-relayed if needed), exchanges the code for a Headscale pre-auth key, then joins Headscale. Tailscale never sees the remote peer — only the seed is registered there.

**Why this is free.** Tailscale's Personal tier is unlimited for one user up to 100 nodes; only the seed peer is registered (one node), so we're at 1/100 forever. DERP relays are free for everyone, including non-Tailscale users.

**User journey.**
- **First peer (seed):** wizard's Network screen has "Mesh VPN" toggle on by default. A new **Tailscale account** sub-step opens the browser for one-time OAuth signin (Google / Microsoft / GitHub / email). Wizard copy: *"Mesh VPN uses your free Tailscale account to help remote peers find this machine when they're outside your local network. Mesh traffic itself runs through your own self-hosted Headscale — Tailscale only sees this seed peer's current address."* On apply: `headscale serve` (Headscale control plane) + `tailscaled` (Tailscale-coordinated, for discoverability) + a second tailscale-equivalent data-plane joined to Headscale all come up. This peer is implicitly the control node.
- **Second peer same LAN:** wizard detects mesh via mDNS, offers one-click join. Pre-auth key fetched silently; data-plane connects to Headscale in 5–10s. **No Tailscale signin required** on this or any subsequent peer.
- **Third peer different network:** wizard prompts for QR scan or pasted join link (bare 6-digit codes only work on the same LAN — surfaced explicitly in wizard copy). Any existing peer's Mackes → Network → Mesh VPN → Add Peer → modal shows the link + QR encoding `mesh-join://?code=412753&ts-key=…&seed-tag=mackes-<mesh-id>` (valid 10 min). Remote peer scans QR or pastes link, queries Tailscale's API for seed's current endpoint, contacts seed via DERP, exchanges code for Headscale pre-auth key, joins Headscale.
- **Control node fails over:** election triggered after 120s missed NATS heartbeats. Next peer in deterministic order (lowest peer_id) takes over, restoring headscale state from the latest snapshot AND assuming the Tailscale presence — the new leader uses the API key from `mesh.vpn-state` to re-register its own public endpoint under tag `tag:mackes-<mesh-id>`, replacing the dead seed. Existing WireGuard tunnels — established peer-to-peer, not through the control node — keep working throughout. Carbon Toast on every peer: "Mesh control role moved to desktop-mm".

**Architectural calls.**
- DERP relays = Tailscale's public DERP servers by default (free, no ops). Self-hosted DERP optional via advanced panel.
- Election trigger = 120s of missed NATS heartbeats (tunable; avoids flapping).
- Snapshot cadence = 30s. Worst-case data loss on failover: 30s of unsynced pre-auth keys (which expire anyway).
- MagicDNS enabled by default. Peers reachable as `<hostname>.mesh` from any other peer.
- 16-peer cap (Q-MX18) enforced at headscale registration; 17th peer attempt fails with a Carbon Toast.
- **Tailscale exposure (Option C):** seed peer (and any peer that subsequently takes the control role via failover) signs into Tailscale's free Personal tier and stays registered there. Only one peer per mesh is ever in Tailscale's tailnet (1/100 free-tier node count). Subsequent peers never see a Tailscale login. Headscale pre-auth keys are the only credential exchanged between peers.
- **Same-LAN joins** use mDNS only and require no Tailscale account at all. Tailscale is invoked exclusively for cross-network endpoint discovery.

**Mackes → Network → Mesh VPN panel.**
```
┌─ Mesh VPN ───────────────────────────────────────┐
│  [●] Connected · 4 peers · You are control node   │
│  [+ Add Peer]    [Leave Mesh]    [Diagnostics]    │
│                                                   │
│  Peers (Carbon DataTable)                         │
│    hostname     mesh-IP    route   RTT   seen     │
│    laptop-mm    100.64.1.2 direct  12ms  now      │
│    desktop-mm   100.64.1.3 direct  3ms   now      │
│    phone-mm     100.64.1.4 relay   45ms  now      │
│    vps-mm       100.64.1.5 direct  28ms  2m ago   │
│                                                   │
│  Control node: this machine (since 2026-05-16)    │
│  State checkpoint: 14s ago · 4.2 KB               │
│  [▸ Advanced (ACLs, DERP, exit nodes…)]           │
└───────────────────────────────────────────────────┘
```

**Components to build for this feature.**
1. `mackes/mesh_vpn.py` — Python wrapper around `headscale` and `tailscale` CLIs; election logic; mDNS announce/discover; snapshot/restore of headscale state to NATS Object Store; Tailscale OAuth-bootstrap flow; Tailscale REST-API lookup helper.
2. `mackes/workbench/network/mesh_vpn.py` — Carbon-styled Mesh VPN panel.
3. `mackes/wizard/pages/mesh_vpn_account.py` — new wizard sub-step: Tailscale OAuth signin browser flow (seed peer only; auto-skipped on subsequent peers).
4. `data/systemd/headscale.service` — systemd unit, always installed, only enabled when this peer is the control node.
5. `data/systemd/mackes-tailscale-bootstrap.service` — runs `tailscaled` with a separate state dir on the control node, joined to Tailscale's tailnet for endpoint registration only.
6. `data/systemd/mackes-mesh-vpn.service` — qnmd companion that runs election + 30s snapshot + Tailscale-presence handoff on failover.
7. `install-helpers/mesh-vpn-bootstrap.sh` — generates headscale config on first install.

**Mackes-wide updates.**
- `mackes/workbench/network/qnm.py` — Mesh VPN status line + link to panel.
- `mackes/wizard/pages/network.py` — auto-detect existing mesh via mDNS; show join UI; route to `mesh_vpn_account.py` substep on seed peers; surface "bare 6-digit code is same-LAN only" copy on remote-join path.
- `mackes/state.py` — `service_health()` adds `tailscaled` (always on control node), `headscale` (only when control node).
- `mackes/workbench/dashboard.py` — Mesh VPN dot in status strip.
- `mackes/workbench/maintain/dependencies.py` — `headscale`, `tailscale` as required.
- `packaging/fedora/mackes-shell.spec` — `Requires: tailscale`, `Requires: headscale` (always shipped; election decides which peer runs `headscale serve` + `tailscaled-bootstrap`).
- `packaging/iso/mackes-xfce.ks` — packages added.

**What "just works" means concretely.**

| Action | User does | Mackes handles |
|---|---|---|
| Install seed peer | leaves toggle on + one-time Tailscale OAuth signin | spins up headscale + Tailscale-bootstrap presence, scoped API key, mesh IP |
| Join same-LAN peer | clicks "Join existing mesh" | mDNS lookup, pre-auth key fetch, Headscale connect (no Tailscale) |
| Join remote peer | scans QR or pastes join link | Tailscale API lookup → seed endpoint → code exchange over DERP → Headscale connect |
| Control node crashes | nothing | election, snapshot restore, Tailscale-presence handoff, Toast |
| Peer roams (home→coffee→home) | nothing | DERP fallback, direct rebind on reconnect |
| 17th peer attempt | sees Toast: "Mesh capacity (16/16)" | refuse with explicit reason |

All Mackes-authored chrome continues Carbon Design (Q-CB1–Q-CB10).

### 8.12 Headless Node Mode (Q-HL1–Q-HL7)

Mackes runs as a full mesh node on headless servers (fileservers, NAS boxes, VPSes) without a display manager. Same backend code, no GUI, full subcommand surface, systemd-managed lifecycle.

**Activation (Q-HL1).** On launch, `mackes` checks `$DISPLAY`, `$WAYLAND_DISPLAY`, and `loginctl show-session $XDG_SESSION_ID` for a graphical session. If none of those are present, headless mode is selected automatically. `mackes --headless` forces it; `mackes --gui` forces the GTK path. Most cloud-init / SSH provisioning sessions hit the auto-detect path with no flag needed.

**Interactive UI (Q-HL2).** Pure stdin prompts via `input()` and `getpass()` with light ANSI coloring. No new dependencies. Works on serial consoles, recovery shells, dumb terminals, and copy-pastes cleanly into screenshots. The wizard becomes a sequence of numbered prompts.

**Subcommand surface (Q-HL3).** Full backend parity with the GUI panels:

| Command | What it does |
|---|---|
| `mackes init` | First-time setup wizard (mirrors the GUI wizard's flow) |
| `mackes join <mesh-join://link>` | Join an existing mesh from a paste-link |
| `mackes status` | Current node state: preset, mesh peers, services, control role |
| `mackes peers` | List mesh peers with mesh-IP, route type, RTT |
| `mackes shares` | List shared filesystems mounted from / served to other peers |
| `mackes snapshot {create\|list\|restore <name>}` | Snapshots panel parity |
| `mackes maintain {repair\|health\|logs}` | Maintain panel parity |
| `mackes apps {install\|remove\|list}` | Apps panel parity |
| `mackes preset {list\|apply <name>}` | Preset list / apply |
| `mackes uninstall` | Same uninstall flow as the GUI Maintain → Uninstall panel |

Every subcommand also supports flag-driven non-interactive mode for cloud-init (`mackes init --preset node --tailscale-authkey=… --enable-on-boot`).

**Node preset (Q-HL4).** New shipped preset `data/presets/node.yaml`:
- `appearance:` empty (no theme/font/wallpaper applied — the box has no display)
- `system:` empty (no workspace count, no xfwm theme, no notification config)
- `apps.install:` empty (fileservers shouldn't be opinionated about apps)
- `apps.remove_bloat:` empty (don't strip packages without intent)
- `panel:` empty
- mesh-vpn / NATS / SSHFS all enabled by default
- `network.qnm_enabled: true`

`mackes init` headless auto-picks `node`. The other four presets (hashbang/mackes/daylight/vanilla) remain GUI-only since they each touch XFCE config that's meaningless on a server.

**Systemd integration (Q-HL5).** RPM ships `/etc/systemd/system/mackes-node.service`:
```ini
[Unit]
Description=Mackes Shell — mesh node services
After=network-online.target
Wants=network-online.target

[Service]
Type=notify
ExecStart=/usr/bin/mackes daemon
Restart=on-failure
RestartSec=10s
User=mackes
Group=mackes

[Install]
WantedBy=multi-user.target
```
After `mackes init` completes, the wizard prompts: *"Auto-start mesh node on boot? [Y/n]"* — default Yes. On Yes, runs `systemctl enable --now mackes-node`. The service runs `qnmd`, joins the mesh, mounts SSHFS shares, subscribes to NATS subjects, and (if elected) hosts Headscale + Tailscale-bootstrap.

**Tailscale OAuth (Q-HL6).** Two paths on a seed peer:
- **Interactive**: `mackes init` prints `→ Open https://login.tailscale.com/a/<code> on any device, sign in, then press Enter here.` Standard Tailscale device-auth flow. Polls until the user completes browser login, then continues.
- **Cloud-init / fully automated**: `mackes init --tailscale-authkey=tskey-auth-…` supplies a pre-generated auth key (admin generates it once on the Tailscale console for tag `tag:mackes-bootstrap`). Zero terminal interaction.

Headless **joining** peers (not seed) need no Tailscale auth — same as the GUI flow. They consume the scoped API key embedded in the join link.

**Mesh role (Q-HL7).** Backend-services-only:
- Participates in **SSHFS** — shares its `~/QNM-Shared/` (configurable to any local path), mounts other peers' shares at `~/QNM-Mesh/<peer>/`.
- Hosts a **NATS replica** — backs up clipboard, notifications, Object Store buckets like any other peer. Snapshot witness for offline backup.
- **Headscale-eligible** — can win control-node election like any other peer. Often the *preferred* control node since headless servers stay online.
- **Mesh-VPN data plane** — full WireGuard tunnels to every other peer.
- Does **not** originate clipboard items (no X11 selection to read) or render notifications (no display). A `mackes notify <peer> "message"` CLI subcommand exists for ad-hoc notifications from cron/scripts.

**Components to build.**
1. `mackes/headless/__init__.py` — entry-point dispatcher; auto-detect + flag override.
2. `mackes/headless/wizard.py` — stdin-prompts version of the GUI wizard.
3. `mackes/headless/cli.py` — argparse subcommand router.
4. `mackes/headless/status.py` — `mackes status` / `mackes peers` / `mackes shares` formatters.
5. `mackes/headless/daemon.py` — `mackes daemon` entry-point for the systemd unit (longest-running process, supervises qnmd + dependencies).
6. `data/presets/node.yaml` — new headless preset.
7. `data/systemd/mackes-node.service` — systemd unit, RPM-installed.
8. `install-helpers/create-mackes-user.sh` — RPM scriptlet that creates a `mackes` system user/group for the service.

**Mackes-wide updates.**
- `mackes/app.py` — entry-point checks for headless conditions, dispatches to `mackes.headless.cli` or the GTK path.
- `packaging/fedora/mackes-shell.spec` — `Requires: openssh-server` (already there from prior commit); `%post` creates `mackes` user; `%files` includes the systemd unit.
- `packaging/iso/mackes-xfce.ks` — unchanged (ISO is always-graphical); a separate headless `mackes-node.ks` kickstart could ship later.

**What "just works" means concretely for headless.**

| Action | Admin types | Mackes handles |
|---|---|---|
| First fileserver provision | `curl ... \| sudo bash` then `mackes init` | auto-detects headless, prompts through node preset, prints Tailscale URL, sets up Headscale, prompts to enable service |
| Cloud-init automated provision | `mackes init --preset node --tailscale-authkey=… --join 'mesh-join://…' --enable-on-boot` | zero interaction; full mesh-join + service-enable in one command |
| Daily status check | `mackes status` | one-screen summary: preset, mesh peers, mounts, service state |
| Backup snapshot of mesh state | `mackes snapshot create offsite-backup` | same as GUI snapshot |
| Reboot recovery | nothing | systemd unit auto-restarts qnmd, re-joins mesh, re-mounts SSHFS |

This preserves the GUI's "just works" promise on machines that will never have a screen attached.

### 8.13 Mesh Media Services (5-layer composition)

Any HTTP service running on any peer (Jellyfin :8096, Airsonic :4040, Plex :32400, Sonarr :8989, Radarr :7878, Home Assistant :8123, Grafana :3000, …) is reachable from every other mesh peer over the WireGuard data plane (§8.11). Mackes layers five complementary surfaces on top of that raw reachability — each addresses a different client audience. **All five share one service catalog + one live registry**, so they never drift from each other.

**Shared infrastructure.**
- **Service catalog**: `data/media-services.yaml` lists known service types — each entry carries `port`, `mdns-type`, `icon`, `display-name`, `category` (media / monitoring / iot / dev). User-extensible via `~/.config/mackes-shell/media-services.yaml` overrides.
- **Live service registry**: qnmd's new `mesh-services` module port-probes every mesh peer every 60s against catalog entries and publishes the matrix to a `mesh.services` NATS bucket. Layer 5's mDNS relay also writes into the same registry.
- **All five layers consume the same registry** — no drift between Media Hub Tiles, Caddy proxy routes, native-client server lists, and mDNS rebroadcasts.

**Layer 1 — Raw mesh URLs (free baseline).** Mesh VPN + MagicDNS makes `http://<peer-hostname>.mesh:<port>` work day one. Mackes → Network → QNM → Help renders a copy-paste cheatsheet generated from the live registry: every service-on-peer pair as a clickable URL. For power users who know what they're looking for. Zero new code.

**Layer 2 — Mesh Media Hub panel.** New panel at Network → Media Hub. Each detected service rendered as a Carbon Tile: peer name + service icon + green/grey status dot + "Open" button → `xdg-open http://<peer>.mesh:<port>`. Filter chips at the top by category. One-click launch; no URL memorization. Updates live from the NATS `mesh.services` registry. Suitable for the everyday desktop user.

**Layer 3 — Unified `https://media.mesh` reverse-proxy gateway (Caddy).** Mackes installs Caddy on every peer (opt-in via Network → Mesh Services → Enable Unified Gateway). Caddy is config-generated from the live registry to expose every service under a single URL scheme:
- `https://media.mesh/jellyfin/headless-server/` → `http://headless-server.mesh:8096/`
- `https://media.mesh/airsonic/headless-server/` → `http://headless-server.mesh:4040/`
- `https://media.mesh/jellyfin/laptop-mm/` → `http://laptop-mm.mesh:8096/`

TLS via a Mackes-managed private CA. CA root distributed via NATS Object Store (`mesh.ca-root` bucket); per-peer trust-store install via pkexec helper. One browser bookmark = the entire mesh's HTTP service catalog. Failed peers' routes return 502 with a Carbon-styled error page; recovered peers appear automatically. Suitable for users with one canonical URL bookmark.

**Layer 4 — Bundled native clients with mesh-aware autoconfig.** Apps → Install adds curated entries with auto-injected server lists:
- **Jellyfin Media Player** (`jellyfin-media-player`) — Mackes writes `~/.local/share/jellyfinmediaplayer/servers.json` listing every mesh peer running Jellyfin.
- **Subsonic-compatible client** — `strawberry` (or `clementine` as fallback); server list pre-populated for every mesh peer running Airsonic / Subsonic-compatible API.

Server-list refresh runs on mesh-peer events from NATS (peer up → add server; peer down → mark unreachable but keep in list). Native clients connect direct to `<peer>.mesh:<port>` — they don't traverse the Caddy proxy. Better playback UX than browsers (proper buffering, offline downloads, native gestures). Suitable for primary-media-consumer peers.

**Layer 5 — mDNS-over-mesh relay.** New qnmd module `mdns-relay` bridges each peer's `avahi-daemon` across the mesh:
1. Local mDNS announcements (`_jellyfin._tcp.local`, `_googlecast._tcp.local`, `_airplay._tcp.local`, `_ipp._tcp.local`, …) captured on each peer.
2. Captured announcements republished to NATS subject `mesh.mdns.<peer-id>.<service-type>`.
3. On every other peer, qnmd subscribes to `mesh.mdns.*.*` and re-broadcasts received announcements on the local LAN — with the *originating peer's mesh IP* substituted for the source LAN IP.
4. Any mDNS-aware client (Jellyfin Roku app, Plex iPhone, Chromecast, AirPlay speaker, network printer browser, KDE Connect, Home Assistant device discovery) sees every mesh peer's services as if they were on the local subnet.

Anti-loop policy: announcements carry an `origin-peer-id`; receivers never re-publish their own. Name-collision handling: services are renamed `jellyfin-headless-server.local` (hostname suffix) before local rebroadcast to avoid `jellyfin.local` collisions. Privacy: per-service-type opt-out checkboxes in Mackes → Network → Mesh Services → mDNS Relay; canonical media types default ON, printer/file-share types default OFF.

**Mackes → Network → Mesh Services panel** (one Carbon-styled panel hosting all five layers' controls):
- **Discovered services** — Tile grid (Layer 2 surface)
- **Unified gateway** — toggle + CA-install button + active route count (Layer 3)
- **Bundled clients** — install/config status for Jellyfin Media Player + Strawberry (Layer 4)
- **mDNS bridge** — per-service-type opt-out checkboxes + active relay count (Layer 5)
- **Help cheatsheet** — auto-generated raw-URL list (Layer 1)

**Layer interactions.**
- L2 (Media Hub) and L3 (Caddy proxy) read the same registry — Tile click can route via either direct URL or proxy URL; user toggle in Settings.
- L4 (native clients) always connects direct, bypassing L3 — playback quality benefits from no proxy hop.
- L5 (mDNS relay) feeds discoveries into the same registry L2/L3 consume — services announced via mDNS but not on a probed port still appear in the Media Hub (e.g., a Chromecast).
- L3's Caddy can optionally serve the L4 native-client config endpoints too, giving non-Mackes-instance clients a way to bootstrap their server list.

**Components to build for §8.13.**
1. `mackes/mesh_services.py` — service catalog loader; port-probe scanner; NATS registry publisher.
2. `mackes/workbench/network/mesh_services.py` — Carbon-styled five-section panel.
3. `mackes/mdns_relay.py` — qnmd companion: mDNS capture + NATS publish + mDNS rebroadcast with anti-loop.
4. `data/media-services.yaml` — initial service catalog (Jellyfin, Airsonic, Plex, Sonarr, Radarr, Home Assistant, Grafana, Pi-hole, AdGuard, Nextcloud, Vaultwarden, Syncthing, qBittorrent, …).
5. `data/caddy/Caddyfile.tmpl` — Caddy config template; Mackes regenerates per peer from registry.
6. `install-helpers/mesh-ca-trust.sh` — distributes Mackes CA root cert into the system trust store (pkexec'd).
7. `data/applications/mesh-services-launcher.desktop` — `mesh-launch://<peer>/<service>` URI handler for L2 Tile clicks.

**Mackes-wide updates.**
- `mackes/app_mgmt.py` CATALOG: add `jellyfin-media-player`, `strawberry`, `clementine`.
- `packaging/fedora/mackes-shell.spec`: `Recommends: caddy`, `Recommends: jellyfin-media-player`, `Recommends: strawberry`.
- `packaging/iso/mackes-xfce.ks`: same additions.
- `mackes/workbench/dashboard.py`: "Mesh services" tile (count of discovered services across mesh).
- `mackes/workbench/network/qnm.py`: link to Mesh Services panel.

**Engineering estimate.**
- Layer 1: free (docs only).
- Layer 2: small (~500 LOC + Carbon panel).
- Layer 3: medium (~2 weeks — Caddy config generation, private CA lifecycle, per-OS trust install).
- Layer 4: medium (~1 week — client autoconfig, refresh on NATS events).
- Layer 5: large (~2-3 weeks — mDNS bridge correctness, name-collision policy, multicast-over-tunnel gotchas).

Suggested implementation order: **1 → 2 → 5 → 4 → 3**. Layer 5 is high-leverage (every mDNS-aware client benefits with zero per-service code); Layer 3 is polish (one URL bookmark) that can land last.

**Carbon Design (Q-CB1–Q-CB10)** applies throughout the Mackes-rendered surfaces — Tile, DataTable, Modal (for CA-install confirmation), Toast (for service up/down events), Skeleton (during initial probe), Gray 100 palette, IBM Plex Sans/Mono, per-preset accent.

### 8.14 Mesh SSH (Identity-based via Headscale + auto-key + cheatsheet)

Three layers shipped together for "ssh anywhere in the mesh, no friction":

**Layer 0 — Raw SSH cheatsheet (free baseline).** Mesh VPN + MagicDNS makes `ssh user@<peer-hostname>.mesh` work day one. Mackes → Network → Mesh SSH renders a copy-paste cheatsheet generated from the live peer registry. Documented in Help. Zero new code.

**Layer A — Auto-distributed SSH keys via NATS.** Mackes generates `~/.ssh/mackes_mesh_ed25519` per peer at install/preset apply. The pubkey is published to a `mesh.ssh-keys` NATS Object Store bucket keyed by peer-id. qnmd subscribes; on receive, appends the remote pubkey to the configured target user's `~/.ssh/authorized_keys` bracketed with `# managed-by-mackes-mesh-<peer-id> {begin,end}` markers so updates/removals are surgical. New peer joins → pubkey distributes mesh-wide in seconds; peer leaves → pubkey purged. Default target user is the wizard-running user; admin can pick a different local account per peer in Network → Mesh SSH → Key Distribution.

**Layer B — Identity-based SSH (Tailscale SSH via Headscale).** Headscale's experimental Tailscale-SSH support is enabled by default. Users on any peer can run `mackes ssh <peer-name>` (or `tailscale ssh <peer-name>`) to open a session authenticated *by mesh identity*, not by SSH key. ACLs configured in Mackes → Network → Mesh SSH → Access Policy:

```yaml
# data/mesh-ssh-policy.yaml (generated by Mackes from the GUI policy editor)
ssh:
  - action: accept
    src:    ["tag:mackes-admin"]
    dst:    ["*"]
    users:  ["root", "mm"]
  - action: accept
    src:    ["tag:mackes-user"]
    dst:    ["tag:mackes-fileserver"]
    users:  ["mm"]
```

Every accepted SSH session writes a structured record to NATS `mesh.ssh-audit` bucket: timestamp, source peer-id, source user, target peer-id, target user, session-id, exit-status. Audit log surfaced in Mackes → Network → Mesh SSH → Audit Log (DataTable with filter/search).

**Mackes → Network → Mesh SSH panel** (Carbon-styled, four sections):
- **Discovered peers** — Tile per mesh peer with hostname, mesh-IP, SSH route status, "Open Terminal" button (defaults to identity-based session via Layer B; falls back to Layer A keys if Headscale SSH unavailable).
- **Key Distribution** — Layer A status: peer-list with each peer's auto-key state (synced / pending / opted-out); per-peer toggle.
- **Access Policy** — Layer B editor: visual ACL builder (src tag → dst tag → users), saves to `data/mesh-ssh-policy.yaml`, applied via Headscale API.
- **Audit Log** — DataTable of accepted/denied sessions, last 1000 entries from NATS bucket.

**Layer interactions.** Layer 0 + Layer A is the default frictionless story (works without Headscale SSH). Layer B adds identity-based access on top — Mackes' `mackes ssh` command prefers Layer B when available, falls back to Layer A for SSH clients that don't speak Tailscale-SSH. Both audit through the same NATS bucket.

**Components to build for §8.14.**
1. `mackes/mesh_ssh.py` — ed25519 keygen on install; pubkey publish/subscribe via NATS; authorized_keys manager (surgical marker-bracketed edits); Headscale policy writer/reader.
2. `mackes/workbench/network/mesh_ssh.py` — Carbon-styled four-section panel.
3. `mackes/cli/mesh_ssh.py` — `mackes ssh <peer>` subcommand wrapper.
4. `data/mesh-ssh-policy.example.yaml` — initial restrictive policy (admin-only).

**Mackes-wide updates.**
- `mackes/mesh_vpn.py` — enables Headscale's `--policy-mode=database` + Tailscale-SSH at startup.
- `mackes/workbench/network/qnm.py` — link to Mesh SSH panel.
- `mackes/headless/cli.py` — new `mackes ssh <peer>` subcommand.

Carbon Design (Q-CB1–Q-CB10) applies throughout. Layer A relies on standard openssh-server (already a hard Requires from §8 ssh-by-default lock). Layer B requires Headscale ≥ 0.23 for SSH support — RPM `Requires: headscale >= 0.23`.

### 8.15 Help Menu (in-Mackes documentation)

Mackes ships a complete user-facing documentation tree under `docs/help/` and surfaces it via a Help tab in the workbench and a Help launcher in the activity bar / menu.

**Documentation tree** (`docs/help/`):
- `index.md` — landing page with feature-area links
- `getting-started.md` — first-run wizard walkthrough
- `dashboard.md` — what the dashboard shows
- `look-and-feel.md` — Appearance, themes, fonts, icons
- `devices.md` — Display, Keyboard, Mouse, Sound, Power
- `network.md` — Wi-Fi, VPN, QNM, Mesh VPN, Mesh SSH, Mesh Services, Firewall
- `system.md` — Window Manager, Workspaces, Session, Notifications, Default Apps, Removable Media, Date & Time
- `apps.md` — Install, Remove, Installed; the curated catalog
- `maintain.md` — Snapshots, Drift, Repair, Reset, Health Check, Logs, Update, Fonts, Power, Resources, Dependencies, Uninstall
- `mesh.md` — overview of the mesh: VPN, Thunar Extension, Clipboard, Notifications, Object Store, Media Services, SSH
- `mesh-thunar.md` — using `mesh:///` in Thunar
- `mesh-vpn.md` — Headscale + Tailscale bootstrap, joining peers, control-node failover
- `mesh-services.md` — five-layer media services architecture
- `mesh-ssh.md` — three-layer SSH (cheatsheet + auto-keys + identity)
- `headless.md` — `mackes init` on a fileserver
- `presets.md` — the 4 shipped presets + custom user presets
- `troubleshooting.md` — common issues, log locations, recovery
- `keybindings.md` — keyboard shortcuts
- `cli-reference.md` — every `mackes <subcommand>` documented

**Help panel** (`mackes/workbench/help.py`): a sidebar of doc topics + a content pane that renders the selected markdown file. Markdown is rendered via a small Pango converter (headers → `<b>`, code → `<tt>`, lists → indented bullets, links → underlined + clickable opening `xdg-open`). No external markdown library required.

**Workbench integration**: a new "Help" tab in the top-level navigation (8th tab, alongside Dashboard / Look & Feel / Devices / Network / System / Apps / Maintain). Also accessible from the workbench header menu's existing "About Mackes Shell" item, which gets a sibling "Help / User Guide" item.

**Headless integration**: `mackes help [topic]` subcommand prints the docs to stdout (rendered as plain text via the same Pango-stripping function), or opens the markdown file in `$PAGER` (default: `less`).

**Update on every release**: docs/help/ is shipped in the RPM at `/usr/share/mackes-shell/help/`. The Help panel reads from there at runtime (with a dev-mode fallback to the repo's `docs/help/` for in-tree dev work).

---

## 9. Build & Distribution

### 9.1 install.sh (the curl bootstrap)

Roughly:

```bash
#!/usr/bin/env bash
set -euo pipefail
REPO="mattmacke/mackes-shell"           # GitHub repo
TAG=$(curl -sL "https://api.github.com/repos/$REPO/releases/latest" \
        | grep -oP '"tag_name":\s*"\K[^"]+')
URL="https://github.com/$REPO/releases/download/$TAG/mackes-shell-${TAG#v}.fc$(rpm -E %fedora).noarch.rpm"
TMP=$(mktemp -d)
curl -L -o "$TMP/mackes.rpm" "$URL"
sudo dnf install -y "$TMP/mackes.rpm"
rm -rf "$TMP"
exec mackes
```

The user's curl-pipe-bash one-liner installs Mackes and immediately launches it into the first-run wizard.

### 9.2 RPM spec sketch

```spec
Name:           mackes-shell
Version:        0.1.0
Release:        1%{?dist}
Summary:        Mackes Shell — XFCE control panel and shell manager
License:        GPL-3.0
URL:            https://github.com/mattmacke/mackes-shell
Source0:        %{name}-%{version}.tar.gz
BuildArch:      noarch

Requires:       python3, python3-gobject, gtk3, xfconf, xfce4-settings
Requires:       polybar, plank, rofi
Requires:       NetworkManager, firewalld
Recommends:     papirus-icon-theme, arc-theme, google-droid-sans-fonts, jetbrains-mono-fonts

%post
# Install hidden-overrides for xfce4-settings menu entries
/usr/share/mackes-shell/install-helpers/hide-xfce-settings.sh || :

%preun
# Restore hidden xfce4-settings menu entries
/usr/share/mackes-shell/install-helpers/restore-xfce-settings.sh || :
```

The RPM **Requires** `xfce4-settings` (per Q19 lock — we keep it installed and hide menu entries, not uninstall it).

### 9.3 Release pipeline

GitHub Actions on tag push:
1. Build source tarball
2. Run `rpmbuild --define '_topdir $PWD/rpmbuild' -ba packaging/fedora/mackes-shell.spec` inside a Fedora container
3. Upload the resulting `.noarch.rpm` to the GitHub release as a release asset

`install.sh` finds it via the GitHub Releases API. No COPR needed (Q20 lock).

---

## 10. Migration from xfce11-unified v2.2

A full mapping is in `docs/MIGRATION_FROM_V2.2.md`. Summary:

- **Kept**: backend logic in `scripts/xfce11v2.py` (refactored into the `mackes/` Python package — split into `presets.py`, `snapshots.py`, `shell_profiles.py`, `menu_integration.py`, `qnm_bridge.py`)
- **Kept**: Polybar/Plank/Rofi config templates (move to `data/shell-profiles/`)
- **Kept**: QNM (`quick-network-mesh/`) entirely as a standalone — Mackes just embeds its GUI as a launcher
- **Kept**: Fedora RPM spec (rewritten to install `/usr/bin/mackes` instead of the v2.2 layout)
- **Deleted**: `web-workbench/` (Q15)
- **Deleted**: `native-workbench/xfce11_workbench.py` (replaced by `mackes/workbench/`)
- **Deleted**: `scripts/install*.sh` (replaced by single `install.sh`)
- **Deleted**: `START-HERE-XFCE11-UNIFIED.desktop` (replaced by `mackes-shell.desktop`)
- **Deleted**: the flat 30+ action list in the Workbench — actions become either settings panels or live in the Maintain tab
- **Deleted**: the "Apply Black Droid Menu Chrome" duplicate — it becomes a Theme preset in the Appearance panel, not a top-level button
- **Deleted**: separate per-font buttons — become one Font picker in the Appearance panel

---

## 11. Open Items for Later

These were intentionally deferred during the survey and are tracked here so they don't get lost:

1. **xfsettingsd decoupling** (Q19 fallback). Long-term goal of full xfce4-settings uninstall. Requires shipping a Mackes equivalent daemon. Not v1.
2. **Custom user presets UI.** YAML drop-in works but is undocumented. A "save current as preset" button could land in v2 if real demand emerges.
3. **Wayland.** Mackes targets X11 + XFCE today. xfce4 Wayland support is still nascent.
4. **Search bar across panels.** xfce4-settings doesn't have one. GNOME Settings does. Worth considering once panel count is final.
5. **Multi-machine sync of customizations.** Out of scope per Q7 (curated presets only). Re-evaluate if usage shows people manually editing the same settings on every machine.
6. **Wallpaper slideshow / per-workspace wallpaper.** v2 if requested. xfdesktop supports it.
7. **Keyboard-shortcut conflict detector.** Would be a nice addition to the Devices → Keyboard panel.

---

## 12. Success Criteria

Mackes Shell is succeeding when:

- A new machine can be brought from fresh Fedora install to working Xfce11 environment with **one** curl one-liner plus the wizard, in under five minutes.
- A user never needs to open `xfce4-settings-manager` for daily settings work.
- Switching presets is one click and visibly reconfigures the entire desktop.
- Drift from an active preset is always visible on the dashboard.
- The "disjointed" feeling of v2.2 — five entry points, two workbenches, 30 flat actions — is gone. There is one binary, one window, six tabs.
