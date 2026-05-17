# Presets

A preset is a YAML file declaring the *target* state for a Mackes install.
The wizard applies it; the workbench reapplies on demand; drift detection
compares it against the live system.

## Shipped presets

| Name | Display | When | What it looks like |
|---|---|---|---|
| **hashbang** (default) | `#!` | First impression of Mackes | CrunchBang-spirit; black/monochrome; alacritty + neovim + firefox + mpv + conky + menulibre |
| **mackes** | `Mackes` | House style | Warm-dark; curated dev tool set (Edge, VS Code, Cursor, Claude CLI, Terminator, VLC) |
| **daylight** | `Daylight` | Light-mode productivity | Documents-first; LibreOffice, Thunderbird, GIMP, Inkscape, Evince |
| **vanilla** | `Vanilla` | "Don't touch" | No theme override, no apps installed, no bloat removed; mesh on by default |
| **node** | `Node` | Headless servers | Auto-selected by `mackes init` headless; empty appearance/apps; mesh-only |

The `node` preset is hidden from the GUI wizard's picker — it's
auto-selected when no display is detected.

## Common defaults across non-vanilla presets

- GTK theme: **PadOS**
- Icon theme: **Carbon**
- UI font: IBM Plex Sans
- Monospace font: IBM Plex Mono
- Wallpaper: `branding/standard-wallpaper.png` (desktop + LightDM greeter)
- Panel layout: Whisker → Docklike taskbar → systray → volume → power → clock
- Panel clock font: IBM Plex Sans 10 / IBM Plex Sans Bold 12
- Bloat removal: GNOME-on-XFCE + libreoffice-* + asunder + parole +
  pragha + xfburn + transmission-gtk + claws-mail + pidgin
  (`daylight` keeps libreoffice; `vanilla` removes nothing)

## YAML schema

```yaml
name: hashbang
display_name: "#!"
description: >
  One-paragraph description shown in the preset picker.

appearance:
  gtk_theme:      "PadOS"
  icon_theme:     "Carbon"
  cursor_theme:   "Adwaita"
  cursor_size:    24
  font_ui:        "IBM Plex Sans 10"
  font_monospace: "IBM Plex Mono 10"
  wallpaper:      "/usr/share/mackes-shell/branding/standard-wallpaper.png"

devices:
  display_scaling:    "auto"
  power_profile:      "balanced"
  audio_default_sink: "pipewire"

network:
  qnm_enabled:           false
  firewall_default_zone: "FedoraWorkstation"

system:
  workspace_count:       4
  window_manager_theme:  "Default"
  notifications_enabled: true
  autostart_extras:
    - conky.desktop

panel:
  # xfce4-panel plugin overrides
  clock:
    digital-layout:      1
    digital-date-format: "%B %d, %Y"
    digital-date-font:   "IBM Plex Sans 10"
    digital-time-format: "%I:%M %p"
    digital-time-font:   "IBM Plex Sans Bold 12"

apps:
  install:
    - alacritty
    - neovim
    - firefox
    - mpv
    - conky
    - menulibre
  remove_bloat:
    - gnome-software
    - gnome-tour
    - libreoffice-*
    - asunder
    - parole
    - pragha
    - xfburn
    - transmission-gtk
    - claws-mail
    - pidgin

snapshot:
  initial_snapshot_name: "hashbang-baseline"
```

## Custom presets

Drop a YAML file at `~/.config/mackes-shell/presets/my-preset.yaml`. It
shadows any shipped preset with the same `name`, or stands as a new
option in the wizard picker.

The shipped presets live at `/usr/share/mackes-shell/data/presets/`.

## Apply / reapply

- **Wizard** applies the chosen preset as part of first-run setup.
- **Workbench header menu → Run First-Run Wizard…** re-runs the wizard
  against current state.
- **Maintain → Reset to Preset** wipes local changes and applies clean.
- **Maintain → Repair → Re-apply active preset** writes preset values on
  top of current state (preserves anything the preset doesn't set).

Headless equivalents:
- `mackes preset apply <name>` — apply a named preset
- `mackes preset list` — print available presets

## Drift detection

After applying a preset, any change you make via xfce4-appearance-
settings, xfce4-panel preferences, etc. creates drift. The Dashboard
shows a drift card listing every diverged key, with three options per
item (revert / adopt / ignore).

Mackes never auto-resolves drift. It's always informational and
user-driven.
