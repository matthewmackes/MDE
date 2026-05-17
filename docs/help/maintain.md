# Maintain

12 panels covering snapshots, recovery, observability, and uninstall.

## Snapshots

Manual restore points. A snapshot captures:

- xfconf dumps of every known channel (xsettings, xfwm4, xfce4-panel,
  xfce4-desktop, xfce4-notifyd, xfce4-power-manager, xfce4-session,
  thunar-volman, keyboards, pointers, displays, keyboard-layout)
- Copy of `~/.config/xfce4/` (panel layouts, desktop config, plugin .rc
  files)
- `manifest.json` (label, timestamp, hostname, source preset, captured
  channels)

Snapshots live at `~/.local/share/mackes-shell/snapshots/<ts>_<slug>/`.

Operations: **Create** (with a label) · **List** · **Restore** ·
**Delete**. Restore wipes the live config dirs and re-loads the dumps
via xfconf; xfsettingsd applies live, no service restart needed.

## Drift

Live read-only comparison of the active preset against the current
system. Three-way per drifted key: **Revert to preset** (write the preset
value), **Adopt into preset** (update the preset YAML), **Ignore** (this
session only).

## System Update

Wraps `pkexec dnf upgrade`. Live progress, log capture, "remind me later"
defer.

## Fonts

Browser of `fc-list` with preview. Quick-install of common font packages
(`google-noto-fonts`, `jetbrains-mono-fonts`, `ibm-plex-*-fonts`, etc.).

## Power

Runtime power profile switching via `power-profiles-daemon`
(performance / balanced / power-saver). Different from the **Devices →
Power** panel which sets idle/lid policies.

## Resources

Live CPU / RAM / disk cards (1s refresh) for quick health check.

## Health Check

Combined preflight + validate. Each item ok/warn/fail:

- Core binaries: xfconf-query, xfsettingsd, xfce4-panel, xfce4-session,
  xfdesktop, xfce4-appfinder, sshd
- Network: nmcli, firewall-cmd, timedatectl
- Service health: same as Dashboard status strip
- Config paths: log dir, snapshot dir, ~/.config/xfce4
- xfconf reachability

## Dependencies

List of required + recommended Fedora packages with install state. Click
**Install missing required** to fix gaps via `pkexec dnf install`.

## Logs

Tail of `~/.local/share/mackes-shell/logs/mackes.log` + `xfsettingsd`
journal entries. Refresh button, filter by severity.

## Repair

Common recovery operations:

- Re-apply active preset (fixes drift)
- Re-hide xfce4-settings menu entries (rebuild menu folder)
- Restore xfce4-settings menu entries (un-hide; for when Mackes is leaving)
- Re-install the Mackes menu entry

## Reset to Preset

Drops every local change and reapplies the preset clean. Effectively
"factory reset" relative to the preset.

## Uninstall

Removes Mackes entirely. Sequence:

1. Auto-snapshot to `~/Desktop/mackes-shell-final-snapshot-<ts>.tar.gz`
2. Reset xfconf channels to XFCE distribution defaults
3. Restore hidden xfce4-settings menu entries
4. Delete `~/.config/mackes-shell/` and `~/.local/share/mackes-shell/`
5. Remove xfce11-unified v2.2 leftovers (QNM preserved)
6. Remove the mackes-shell package (dnf / pip / git-detected)
7. Write uninstall log to `~/Desktop/mackes-shell-uninstall-<ts>.log`
8. Offer 10-second logout countdown
