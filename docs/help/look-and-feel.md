# Look & Feel

One unified Appearance panel covers themes, icons, cursors, fonts, and
wallpaper. Every change writes through xfconf immediately — no apply step,
no settings restart.

## GTK theme

**PadOS** is the locked default GTK theme (Mackes vendors it). Other GTK
themes installed on your system show in the picker but are greyed out —
the v1.0 design favors consistency over choice.

To override, set an alternative via xfconf directly:
```
xfconf-query -c xsettings -p /Net/ThemeName -s 'Arc-Dark'
```
This bypasses Mackes' lock; Mackes won't fight the change but the next
preset apply will reset it.

## Icon theme

**Carbon** (IBM Carbon Design icons) ships as the system-wide GTK icon
theme. Like PadOS, other icon themes are visible but greyed out in the
picker.

## Cursor theme

Adwaita 24px by default; user-pickable from any installed cursor theme.

## Fonts

- **UI font**: IBM Plex Sans (10pt on hashbang, 11pt on mackes/daylight)
- **Monospace font**: IBM Plex Mono
- Both packaged as RPM `Recommends` (`ibm-plex-sans-fonts`,
  `ibm-plex-mono-fonts`) so they're pulled in by default

The Mackes window chrome itself uses IBM Plex unconditionally — that's
part of the Carbon Design System lock (see `mesh.md`).

## Wallpaper

Default: `/usr/share/mackes-shell/branding/standard-wallpaper.png`. Applied
to all monitors, all workspaces. Override per-monitor via the Wallpaper
picker (which writes xfce4-desktop xfconf keys per the standard XFCE
schema).

## Login screen

LightDM greeter is configured to match: PadOS theme, Carbon icons, IBM
Plex Sans, standard wallpaper, default-preset accent. Configured silently
on preset apply by writing `/etc/lightdm/lightdm-gtk-greeter.conf` via
pkexec. No Mackes UI for tweaking — `mackes maintain repair` re-applies
if it drifts.
