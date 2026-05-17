# Changelog

All notable user-facing and architectural changes. The current line is
unreleased; tag versions get a date when they ship.

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
