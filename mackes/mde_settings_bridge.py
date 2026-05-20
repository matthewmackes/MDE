"""Python-side bridge to the MDE settings layer (v2.0.0 Phase C/F).

Replaces `mackes.xfconf_bridge` for every Workbench panel that the
Phase F rewrites switch over. Reads + writes flow through the same
JSON sidecars and gsettings keys that the Rust appliers in
`crates/mackesd/src/settings/` maintain, so the panel and the
canonical store stay in lock-step without needing a Python DBus
client.

Why not pydbus / dasbus?

  We deliberately avoid adding a Python DBus client to the runtime
  dep tree during the v1.x → v2.0.0 transition. Every value the
  Workbench panels need is already addressable via:

  * `gsettings get|set org.gnome.desktop.interface <key>` — for
    theme.* + font.* keys (Phase C.1 + C.2).
  * The JSON sidecars under `$XDG_CACHE_HOME/mde/` — for power.*,
    display.*, automount.*, wallpaper.*, notification.* keys
    (Phase C.3 / C.4 / C.5 / C.6 / C.7).
  * `~/.config/sway/config.d/mackes-bindings.conf` — for keybinds.*
    (Phase C.8).

  This bridge wraps those reads + writes in one Python module so the
  panels don't grow N copies of "shell out to gsettings + parse the
  output."

The Phase E.x Iced rewrite of the panel surfaces moves these calls
over to a real zbus client once libcosmic + pyo3 land; for now this
shim is the load-bearing path.
"""
from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path
from typing import Any, Optional


# --- path helpers -----------------------------------------------------------

def _cache_root() -> Path:
    """Resolve `$XDG_CACHE_HOME` with a `~/.cache` fallback."""
    env = os.environ.get("XDG_CACHE_HOME")
    if env:
        return Path(env)
    return Path.home() / ".cache"


def _config_root() -> Path:
    """Resolve `$XDG_CONFIG_HOME` with a `~/.config` fallback."""
    env = os.environ.get("XDG_CONFIG_HOME")
    if env:
        return Path(env)
    return Path.home() / ".config"


def sidecar_path(name: str) -> Path:
    """Path of the `$XDG_CACHE_HOME/mde/<name>` sidecar file."""
    return _cache_root() / "mde" / name


# --- sidecar JSON helpers ---------------------------------------------------

def read_sidecar(name: str, default: Optional[dict] = None) -> dict:
    """Read one of the MDE settings sidecars by basename.

    Returns the parsed JSON dict, or `default` (or `{}` when None)
    when the file is missing / malformed.
    """
    path = sidecar_path(name)
    try:
        text = path.read_text(encoding="utf-8")
    except OSError:
        return dict(default) if default is not None else {}
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        return dict(default) if default is not None else {}
    if not isinstance(data, dict):
        return dict(default) if default is not None else {}
    return data


def write_sidecar(name: str, data: dict) -> None:
    """Write `data` as pretty-printed JSON to the named sidecar.
    Creates the parent directory as needed."""
    path = sidecar_path(name)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(data, indent=2, sort_keys=True),
        encoding="utf-8",
    )


def update_sidecar(name: str, **kwargs) -> None:
    """Read-modify-write convenience. Reads the sidecar, overlays
    every `kwargs` pair, writes back. Keys not in kwargs are
    preserved unchanged."""
    data = read_sidecar(name)
    data.update(kwargs)
    write_sidecar(name, data)


# --- gsettings helpers ------------------------------------------------------

GSETTINGS_SCHEMA = "org.gnome.desktop.interface"


def gsettings_get(key: str) -> Optional[str]:
    """Run `gsettings get <SCHEMA> <key>` and return the unquoted
    string value (None on failure)."""
    try:
        r = subprocess.run(
            ["gsettings", "get", GSETTINGS_SCHEMA, key],
            capture_output=True, text=True, timeout=5,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if r.returncode != 0:
        return None
    return r.stdout.strip().strip("'")


def gsettings_set(key: str, value: str) -> bool:
    """Run `gsettings set <SCHEMA> <key> <value>` and return success."""
    try:
        r = subprocess.run(
            ["gsettings", "set", GSETTINGS_SCHEMA, key, value],
            capture_output=True, text=True, timeout=5,
        )
    except (OSError, subprocess.SubprocessError):
        return False
    return r.returncode == 0


# --- dot-notated key dispatch ----------------------------------------------

# Map MDE settings key → ("gsettings", gsettings_key)
#                       or ("sidecar", sidecar_filename, json_key)
# Mirrors the Rust applier dispatch in crates/mackesd/src/settings/.
_KEY_MAP: dict[str, tuple] = {
    # theme.* — Phase C.1
    "theme.name":      ("gsettings", "gtk-theme"),
    "theme.icon_set":  ("gsettings", "icon-theme"),
    "theme.accent":    ("gsettings", "accent-color"),
    "theme.mode":      ("gsettings", "color-scheme"),
    # font.* — Phase C.2
    "font.name":       ("gsettings", "font-name"),
    "font.monospace":  ("gsettings", "monospace-font-name"),
    "font.hinting":    ("gsettings", "font-hinting"),
    "font.antialias":  ("gsettings", "font-antialiasing"),
    # power.* — Phase C.4 (power.profile via powerprofilesctl, not here)
    "power.lid_action":              ("sidecar", "power-prefs.json", "lid_action"),
    "power.suspend_idle_battery_s":  ("sidecar", "power-prefs.json", "suspend_idle_battery_s"),
    "power.suspend_idle_ac_s":       ("sidecar", "power-prefs.json", "suspend_idle_ac_s"),
    # display.* — Phase C.3 (display.brightness via brightnessctl, not here)
    "display.primary":            ("sidecar", "display.json", "primary"),
    "display.scale":              ("sidecar", "display.json", "scale"),
    "display.night_light":        ("sidecar", "display.json", "night_light"),
    "display.night_light_temp":   ("sidecar", "display.json", "night_light_temp"),
    # automount.* — Phase C.6
    "automount.on_insert":     ("sidecar", "automount.json", "on_insert"),
    "automount.open_on_mount": ("sidecar", "automount.json", "open_on_mount"),
    "automount.autorun":       ("sidecar", "automount.json", "autorun"),
    # wallpaper.* — Phase C.7
    "wallpaper.path":          ("sidecar", "wallpaper.json", "path"),
    "wallpaper.mode":          ("sidecar", "wallpaper.json", "mode"),
    # notification.* — Phase C.5 (notification.do_not_disturb is a flag file, not a sidecar)
    "notification.location":            ("sidecar", "notifications-prefs.json", "location"),
    "notification.default_expire_ms":   ("sidecar", "notifications-prefs.json", "default_expire_ms"),
    # session.* — Phase F.6 (read by mde-session at login)
    "session.save_on_exit":             ("sidecar", "session-prefs.json", "save_on_exit"),
    "session.lock_on_suspend":          ("sidecar", "session-prefs.json", "lock_on_suspend"),
    "session.auto_save":                ("sidecar", "session-prefs.json", "auto_save"),
    # snapshots.* — Phase F.7 (sidecar replaces xfconf-channel snapshots)
    "snapshots.retention_days":         ("sidecar", "snapshots-prefs.json", "retention_days"),
    "snapshots.compress":               ("sidecar", "snapshots-prefs.json", "compress"),
}


def get_setting(key: str) -> Any:
    """Generic getter: dispatches to gsettings or the sidecar based
    on `key`. Returns `None` when the value isn't set."""
    spec = _KEY_MAP.get(key)
    if spec is None:
        return None
    if spec[0] == "gsettings":
        return gsettings_get(spec[1])
    if spec[0] == "sidecar":
        _, file_name, json_key = spec
        return read_sidecar(file_name).get(json_key)
    return None


def set_setting(key: str, value: Any) -> bool:
    """Generic setter: dispatches to gsettings or the sidecar."""
    spec = _KEY_MAP.get(key)
    if spec is None:
        return False
    if spec[0] == "gsettings":
        return gsettings_set(spec[1], str(value))
    if spec[0] == "sidecar":
        _, file_name, json_key = spec
        update_sidecar(file_name, **{json_key: value})
        return True
    return False


# --- power-specific helpers (for F.1 PowerPanel) ---------------------------

def power_profile_get() -> Optional[str]:
    """Read the current power profile via `powerprofilesctl get`."""
    try:
        r = subprocess.run(
            ["powerprofilesctl", "get"],
            capture_output=True, text=True, timeout=5,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    return r.stdout.strip() if r.returncode == 0 else None


def power_profile_set(profile: str) -> bool:
    """Set the power profile via `powerprofilesctl set <profile>`."""
    try:
        r = subprocess.run(
            ["powerprofilesctl", "set", profile],
            capture_output=True, text=True, timeout=5,
        )
    except (OSError, subprocess.SubprocessError):
        return False
    return r.returncode == 0


__all__ = [
    "sidecar_path", "read_sidecar", "write_sidecar", "update_sidecar",
    "gsettings_get", "gsettings_set", "GSETTINGS_SCHEMA",
    "get_setting", "set_setting",
    "power_profile_get", "power_profile_set",
]
