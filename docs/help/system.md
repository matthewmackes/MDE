# System

Seven sub-panels covering window-manager and session-level concerns.

## Window Manager

Backed by `xfwm4` xfconf channel. Title-bar layout, focus mode (click /
sloppy / focus-follows-mouse), workspace edges, compositing toggle.

## Workspaces

Workspace count + per-workspace names. Backed by `xfwm4` channel.

## Session & Startup

`xfce4-session` xfconf channel: save-on-exit, auto-save interval,
lock-screen-before-suspend. Plus the **Autostart** section: list/toggle/
add `~/.config/autostart/*.desktop` entries.

The legacy "Managed processes" sub-section (Polybar/Plank/dunst/picom
status) was removed in 1.0 along with `session_manager.py` — standard
XFCE handles its own daemons via xfce4-session.

## Notifications

`xfce4-notifyd` config: position, theme, do-not-disturb toggle,
critical-only filter.

## Default Apps

Read/write `~/.config/mimeapps.list`. Per-MIME-type default app picker.

## Removable Media

`thunar-volman` xfconf channel: automount, autobrowse, autorun policies
for USB / CDs / cameras / phones.

## Date & Time

Wraps `timedatectl`. NTP toggle, timezone picker, manual time set.
