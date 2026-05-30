# Tag Manifests

**HYP-8.5 (v6.5)** — the per-tag policy file format that drives
every tag-aware feature in Mackes Desktop Environment: which
windows belong to which tag, which output a tag claims, what
default layout the compositor uses, what border color
distinguishes the focused window, and whether the tag's apps
autostart on first login.

## File location

Each tag is one TOML file at:

    ~/.config/mde/tags/<tag-name>.toml

The `<tag-name>` portion is by convention the same as the
`name` field inside the file (the loader uses the file stem as
the default `name`; an explicit `name = "..."` in the body
overrides). Mesh peers all see the same tags because
`~/.config/` is replicated by LizardFS per the v5.0.0 mesh-storage
design.

## Schema

```toml
# Required: display name. Defaults to the file stem when absent.
name = "voip"

# Optional: Wayland output assignment. Hyprland desc:* form is
# supported (e.g. "desc:Dell U2715H ABCDE12345") as well as the
# bare output name. Omit for "any output."
output = "HDMI-A-1"

# Optional: app_ids that belong to this tag. Used by the auto-
# mark daemon and the tag-driven workspace router. Empty list =
# no automatic membership; operators can still drag windows
# into the tag manually via Hub.
apps = ["org.mde.voice.hud", "org.mde.voice.dial"]

# Optional: preferred container layout. One of:
#   - "mde"     — compositor's own algorithm (default for v6.5)
#   - "splith"  — horizontal split
#   - "splitv"  — vertical split
#   - "tabbed"  — tabbed container (browsers / media)
#   - "stacked" — stacked container
layout = "splith"

# Optional: comma-delimited default marks the auto-mark daemon
# applies to windows joining this tag. Empty = no defaults.
marks_default = "primary,call"

# Optional: per-tag border color in CSS hex form. HYP-22 reads
# this for the focused-window border tint. Omit for the platform
# default (neutral charcoal).
border_color = "#5b6af5"

# Optional: when true, mded's autostart worker spawns each
# `apps[]` entry on first login if not already running.
autostart = true
```

## Defaults shipped with MDE

Six curated default tags ship under `/usr/share/mde/tag-manifests/`
and are copied to `~/.config/mde/tags/` on first login by the
Birthright wizard:

| Tag    | Apps                                         | Layout | Border  |
|--------|----------------------------------------------|--------|---------|
| voip   | `org.mde.voice.{hud,dial}`                   | splith | indigo  |
| dev    | `foot`, `code`, `firefox`                    | splith | green   |
| hub    | (none — catch-all)                            | mde    | charcoal|
| web    | `firefox`, `chromium`, `google-chrome`, `qutebrowser` | tabbed | blue    |
| media  | `mpv`, `vlc`, `celluloid`, `Amberol`, `gnome-Music` | tabbed | orange  |
| chat   | `org.mde.voice.sms`, `Element`, `telegramdesktop`, `signal-desktop` | splitv | purple  |

## Edit workflow

The recommended editor is MDE Settings → Tags (HYP-8.6) — the
panel surfaces all fields with appropriate widgets (output
dropdown from the live monitor enumeration, color picker
preselected from the Material 3 palette, app multi-select from
the `.desktop` registry).

Direct text-editing the TOML files is supported but unwarranted
unless you're scripting bulk changes. Saved edits propagate via
the `action/config/tags/reload` topic on the mesh bus; mded's
tag_manifest worker re-validates and re-emits
`event/config/tags/loaded` events for downstream consumers.

## Fail-open contract

If a manifest file fails to parse (malformed TOML, unknown
type for a field) mded logs a warning and skips that single
tag. The daemon never crashes on bad manifest content — a
mistyped tag means that tag is missing, not the whole tag
system unavailable.

Missing `~/.config/mde/tags/` directory entirely (pre-first-
login state) is treated as "zero tags loaded" — no error.

## Bus events

On startup mded publishes one event per loaded manifest:

    Topic:    event/config/tags/loaded
    Body:     {"name":"<name>","apps":<count>,"layout":"<layout>","autostart":<bool>}

Downstream consumers (Portal-* surfaces, auto-mark, tag-driven
workspace router) subscribe to this topic to receive tag
membership updates without re-reading the files themselves.

## Relationship to TagStore

The pre-existing `~/.local/share/mde/tags.json` `TagStore`
(Portal-18.a, in `mackes-mesh-types`) carries Hub UI-side tag
metadata (manual / smart / preset flavors, member sets across
9 TagMember kinds, group color for Hub card tinting).

The HYP-8.5 tag-manifest format is the **compositor-side**
counterpart — separate concern, separate file format, separate
loader. The two are linked by tag name: a manifest named
"voip" applies its compositor policy to whatever windows the
TagStore's "voip" tag claims. Future work may merge the two
formats; for v6.5 they ship side-by-side.
