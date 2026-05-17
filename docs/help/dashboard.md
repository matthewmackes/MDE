# Dashboard

The Dashboard is the daily landing view. Top-to-bottom:

## Status strip

Six dots — green (ok), yellow (warn), red (fail), grey (missing):

- **xfce4-panel** — the standard XFCE panel running
- **xfdesktop** — the desktop background process
- **xfsettingsd** — the XFCE settings daemon that applies xfconf changes live
- **xfconf-query** — the xfconf CLI binary (always present on XFCE installs)
- **NetworkManager** — the networking daemon
- **sshd** — OpenSSH server (enabled by default per Mackes 1.0)

Below the dots: active preset name + last snapshot timestamp.

## Drift card

Shown only when your live state has diverged from the active preset.
Examples: you changed the GTK theme via xfce4-appearance-settings instead
of Mackes; the wallpaper got swapped; the workspace count changed.

Each drifted item shows `section.field: preset=X live=Y`. Two buttons:
- **Open Maintain → Reset** — reapply the preset (overwrites live changes)
- **Snapshot first** — capture the current state before resetting

Mackes detects drift on every Dashboard load; it's never automatic, always
informational.

## Hardware card

Hostname, OS, CPU model, RAM. Read once at launch from `/proc/*` and
`/etc/os-release`.

## Quick actions

Six big buttons for common operations. Click → jumps to the relevant
panel (Appearance, Display, Network, Snapshots, Health, Logs).

## Recent activity

Last 8 lines from `~/.local/share/mackes-shell/logs/mackes.log`. Every
Mackes action (xfconf write, snapshot, preset apply, etc.) is logged
here. Click → open log in default editor.
