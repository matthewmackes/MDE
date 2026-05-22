# Console Installer + Baseline Purge — design (v2.1.0)

**Status:** locked 2026-05-22. The post-RPM, TTY-mode installer that runs
on first boot after `dnf install mde` and the Stage-2 baseline purge that
removes competing desktop environments before MDE first runs.

This directory holds the **canonical design source** for the installer
stack. The Rust + systemd + shell implementation must match the lockset
below; when the implementation drifts, the design wins unless a follow-up
worklist task records the override and the design is amended.

## What the design says, in one paragraph

After the RPM lands, the host's default systemd target flips to
`mde-installer.target` so the **next boot** drops into a console-only
installer on tty1. The installer is the same nine-page `mde-wizard`
binary that runs in graphical mode — a `--frontend={gui,tui,headless}`
flag and a launcher (`/usr/libexec/mde/mde-installer-launch`) collapse
all three start states (headless / Wayland / X) onto one TUI render
path. The framebuffer is clamped to **≤ 1920×1080** at three layers
(kernel `video=` cmdline, userspace `fbset`, console font bump). On
boot, the wizard's **Stage 2: Baseline Purge** page scans the system
for installed DEs / display managers / alt compositors / Flatpak +
Snap apps, shows a per-package list with a per-app whitelist, and
gates the rest of the install on explicit y/n confirmation. Decline
aborts the install entirely.

---

## 1. Five candidate architectures (proposed 2026-05-22)

Recorded for posterity — the locked design is **#5 + #1**, with #4's
kernel-cmdline clamp added as belt-and-suspenders for the resolution
requirement.

1. **`mde-installer.target` (systemd default-flip)** — `%post` runs
   `systemctl set-default mde-installer.target`; next boot lands in
   isolated target with the templated TTY service.
2. **agetty autologin + masked DM** — `getty@tty1.service.d` override
   autologs into an installer user whose shell is the TUI binary.
3. **Compositor-aware launcher** — Tiny script in `%post` detects
   live compositor and either `loginctl terminate-session`s it
   immediately or queues for next boot.
4. **Kernel cmdline boot-into-installer** — `grub-reboot` transient
   entry with `systemd.unit=mde-installer.target video=1920x1080@60`.
5. **Single-binary three-frontend `mde-wizard`** — Shared core,
   three thin frontends (Iced GUI / ratatui TUI / headless answers
   file). Whichever option above launches it, the same binary runs
   the same steps; only the rendering differs.

### Locked: #5 + #1

The wizard is one binary with three rendering paths
(`--frontend={gui,tui,headless,auto}`); the launcher is a systemd
target that takes effect on the next reboot. Why this combo:

- **#5 is the foundation** — without a shared engine, the three
  start states (headless / Wayland / X) would each need separate
  installer flows. With it, only the launcher differs across start
  states; the user-facing experience is identical.
- **#1 takes effect on a reboot, not mid-`dnf install`** — `%post`
  isolating the live session would kill the user's desktop mid-
  install, which is hostile UX. Boot-time switch is the user's
  schedule.
- **#4's `video=` cmdline append** comes along for free — the
  spec's grub-mkconfig step (also in `%post`) caps the framebuffer
  at HD on next boot so userspace `fbset` failures don't leak a
  4K console at the operator.
- **#3 is not picked** because killing a live compositor from
  `%post` surprises users running `dnf install` over SSH. Worse,
  it would never run for kickstart / unattended installs (no live
  compositor to detect).
- **#2 (agetty)** is too clever — the override-then-undo dance
  during the install flow is fragile, and a left-behind autologin
  user is a foot-gun.

---

## 2. Stage-2 Baseline Purge (locked decisions, 2026-05-22)

The wizard's third page (after Welcome and Scan, before LegacyImport)
is a multi-step sub-flow that removes competing desktop environments
before MDE first runs. The four design decisions locked with the user
on 2026-05-22:

| # | Question | Decision |
|---|---|---|
| 1 | Display managers other than LightDM | **Remove** (not just disable) |
| 2 | Flatpak / Snap-installed desktop apps | **Remove** (in scope) |
| 3 | Selection granularity | **Per-app whitelist** (default = remove every scanned package; user toggles individual entries back to keep) |
| 4 | Behavior when user declines | **Abort the entire install** (non-zero exit, `/etc/motd` banner explains how to resume) |

### Sub-step flow

```text
Warning ─→ Scanning ─→ ListView ─→ Confirm ─→ Executing ─→ Done
                            ↓                         ↑
                         (q anywhere)              (real subprocess
                            ↓                       pump in INST-3)
                        Aborted
```

### Catalog-driven, not regex

Removable packages are an explicit allowlist (`pages::purge::CANDIDATES`,
~70 entries across 13 families: GNOME, KDE, XFCE, MATE, Cinnamon, LXQt,
LXDE, Budgie, Pantheon, Deepin, Enlightenment, AltDisplayManager,
AltCompositor). Globbing on `gnome-*` would nuke `gnome-keyring` (sway
uses it) and `gnome-themes-extra` (GTK fallback). The PROTECTED list
(`mackes-shell`, `mde-*`, `sway`, `lightdm`, `NetworkManager`,
`pipewire`, `wireplumber`, `systemd`, `gnome-keyring`) is a second-line
defense even if a name accidentally drifts into the catalog.

Flatpak + Snap items are dynamic — gathered at scan time by shelling
out to `flatpak list` and `snap list`. `snapd` is auto-appended to the
removal set whenever any snap is found (otherwise removing the snaps
without removing snapd leaves an orphan).

### Why per-app whitelist (not per-group)

The user can plausibly want to keep individual apps (e.g., GNOME
Calculator, KDE Krita) while removing the surrounding session
infrastructure. Per-group toggles would force the all-or-nothing
choice. The per-app whitelist UX cost is one ratatui virtualized list
with a `/` filter — cheap.

### Why abort on decline (not warn-and-continue)

MDE has hard requirements on the D-Bus name space and the LightDM
session entries; a "soft fail" mode would land the user in a desktop
that randomly breaks because GNOME's keyring daemon also claims
`org.freedesktop.secrets`. Forcing the choice up-front keeps the
support surface narrow.

### Rollback manifest

The executor writes `/var/lib/mde/installer/purge-manifest.json` before
the first removal. `mde install --restore-purged` reads the manifest
and reinstalls everything via `dnf install -y <pkg>...`. Keeps the
purge from being a one-way door.

---

## 3. Resolution clamp (locked decision, 2026-05-22)

User requirement: **"at least HD or lower, nothing higher."** Reading:
**cap at 1920×1080**. Anything ≤ 1080p is fine.

Implemented at three layers (defense in depth — KMS drivers have wildly
inconsistent behavior on runtime mode changes):

1. **Kernel cmdline** — `%post` appends `video=1920x1080@60` to
   `GRUB_CMDLINE_LINUX` in `/etc/default/grub` and reruns
   `grub2-mkconfig`. Most robust; even before userspace starts, the
   framebuffer comes up clamped.
2. **Userspace `fbset`** — `mde-installer-launch` reads current
   geometry; if `> 1920x1080`, calls `fbset -g 1920 1080 1920 1080 32`.
   No-op when already ≤ HD.
3. **Console font** — `setfont ter-132n` (or fallback ter-v32n) so
   text is legible at 1080p on a 24" monitor.

GUI-mode clamp (for the same wizard running under Wayland / X — not
the post-RPM path, but the `mde-wizard --rerun` recovery path):

- Wayland → `wlr-randr --output <out> --mode 1920x1080@60`.
- X → `xrandr --output <out> --mode 1920x1080 --rate 60`.

These are not yet wired (INST-3 follow-up); the post-RPM TTY path is
what currently ships.

---

## 4. Component map

| Artifact | Path | Owns |
|---|---|---|
| Purge data layer | `crates/mde-wizard/src/pages/purge.rs` | Catalog, PROTECTED list, scanner, plan builder, manifest, real-rpm/flatpak/snap shellouts |
| Frontend dispatcher | `crates/mde-wizard/src/frontend.rs` | `--frontend={gui,tui,headless,auto}` flag, auto-detect from `$DISPLAY` / `$WAYLAND_DISPLAY` |
| TUI render loop | `crates/mde-wizard/src/tui.rs` | crossterm setup/teardown, page dispatch, breadcrumb, key hints |
| Purge sub-UI | `crates/mde-wizard/src/purge_ui.rs` | Stage-2 state machine, per-app whitelist list, filter, confirmation gate |
| Iced GUI (unchanged) | `crates/mde-wizard/src/main.rs` | Existing graphical wizard path |
| Launcher | `install-helpers/mde-installer-launch.sh` | Stop DM, terminate live sessions, chvt 1, clamp framebuffer, setfont, exec wizard |
| Completion handler | `install-helpers/mde-installer-complete.sh` | Restore graphical default, clear flag, reboot |
| Systemd target | `data/systemd/mde-installer.target` | Isolatable system-level target, Conflicts=graphical.target |
| Systemd templated unit | `data/systemd/mde-installer@.service` | Owns the TTY, runs the launcher |
| Completion unit | `data/systemd/mde-installer-complete.service` | Wraps the completion handler |
| Spec wiring | `packaging/fedora/mackes-shell.spec` | `%post` flag-create + default-target-flip + grub cmdline append |

---

## 5. Open follow-ups

These are tracked in `docs/PROJECT_WORKLIST.md` under the
"Console Installer Stack (v2.1 scope)" section:

- **INST-1.d** — Headless answers-file driver (`--frontend=headless`).
  Currently stubbed; needs a JSON / YAML schema for kickstart +
  unattended installs.
- **INST-3** — Live executor pump. Replace the Executing-step argv
  preview with a real subprocess pump that streams `dnf remove -y`
  output into a ratatui progress widget. Includes the manifest
  write.
- **INST-3.b** — Abort-path `/etc/motd` banner. PurgeDeclined exit
  should drop a banner at `/etc/motd.d/00-mde-installer` explaining
  how to resume (`systemctl start mde-installer@tty1.service` after
  resolving the conflict).
- **INST-3.c** — `mde install --restore-purged` subcommand. Read
  the manifest, run the `dnf install` argv. Wire into the `mde`
  binary's existing subcommand dispatcher.
- **INST-5** — Bench validation on real Fedora hardware. Boot a
  clean F44 VM with GNOME pre-installed, `dnf install mde-2.1.x`,
  reboot, walk the installer, confirm GNOME is gone and MDE works.
  Belongs in the HW-* epic.
