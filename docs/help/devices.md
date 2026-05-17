# Devices

Five sub-panels, each a thin GTK form over a specific xfconf channel or
hardware API.

## Display

Monitor arrangement, resolution, refresh rate, scaling. Backed by the
`displays` xfconf channel and `xrandr`. For complex multi-monitor layouts,
the panel embeds `xfce4-display-settings` rather than reinventing it.

## Keyboard

Layout, repeat rate, repeat delay. Plus the **Keyboard Shortcuts**
sub-panel showing every system keybinding (xfwm4 + xfce4-settings) with
a search box. Edit a shortcut → writes to `xfce4-keyboard-shortcuts`
xfconf channel.

## Mouse & Touchpad

Pointer speed, double-click threshold, tap-to-click, scroll direction.
Backed by `pointers` xfconf channel + `libinput`.

## Sound

Default sink/source picker (PipeWire-aware). Pull-out for volume curves
per-device. No EQ — that's the application's job (Pulseaudio Volume Control
remains available).

## Power

Lid-close behavior, idle timer, battery thresholds. Backed by
`xfce4-power-manager` xfconf channel.

For runtime power *profile* switching (performance / balanced /
power-save), see **Maintain → Power** which talks to
`power-profiles-daemon`.
