"""Preset load + apply + drift detection (Q7 lock: curated presets only).

A preset is a YAML file in `data/presets/` (system) or
`~/.config/mackes-shell/presets/` (undocumented user override location).

Presets are read-only at runtime — apply pushes preset values into the live
system, but daily Workbench changes never modify the YAML. That's why drift
exists: live state diverges from the preset's declared targets.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Iterable, Optional

from mackes.logging import log_action
from mackes.state import CONFIG_DIR

try:
    import yaml  # type: ignore
except ImportError:  # pragma: no cover - YAML is a hard dep at runtime
    yaml = None  # noqa: N816


# Search order: user-local first (overrides shipped), then shipped data dir.
USER_PRESET_DIR = CONFIG_DIR / "presets"
SHIPPED_PRESET_DIRS = [
    Path("/usr/share/mackes-shell/data/presets"),
    Path(__file__).resolve().parent.parent / "data" / "presets",
]

# Name of the preset that gets pre-selected in the wizard and used as the
# implicit "active" preset when nothing is set in state.json.
DEFAULT_PRESET_NAME = "chromeos-classic-dark"


# ---------------------------------------------------------------------------
# Preset model
# ---------------------------------------------------------------------------


@dataclass
class Preset:
    name: str
    display_name: str
    description: str
    appearance: dict[str, Any] = field(default_factory=dict)
    devices: dict[str, Any] = field(default_factory=dict)
    network: dict[str, Any] = field(default_factory=dict)
    system: dict[str, Any] = field(default_factory=dict)
    panel: dict[str, Any] = field(default_factory=dict)
    apps: dict[str, Any] = field(default_factory=dict)
    snapshot: dict[str, Any] = field(default_factory=dict)
    source_path: Optional[Path] = None
    # INST-8 (v2.7) — install profile gating which birthright steps run.
    # "full" is the default so existing callers (the GTK wizard, which
    # always builds a full desktop) are unchanged; the installer sets
    # this to "lighthouse" / "headless" for non-desktop nodes. Steps
    # like apply_display_manager already branch on `preset.profile`;
    # before this field existed that was a latent AttributeError on any
    # Preset built from_dict.
    profile: str = "full"

    @classmethod
    def from_dict(cls, data: dict[str, Any], source: Optional[Path] = None) -> "Preset":
        return cls(
            name=str(data.get("name", source.stem if source else "unnamed")),
            display_name=str(data.get("display_name", data.get("name", "Unnamed"))),
            description=str(data.get("description", "")).strip(),
            appearance=dict(data.get("appearance") or {}),
            devices=dict(data.get("devices") or {}),
            network=dict(data.get("network") or {}),
            system=dict(data.get("system") or {}),
            panel=dict(data.get("panel") or {}),
            apps=dict(data.get("apps") or {}),
            snapshot=dict(data.get("snapshot") or {}),
            source_path=source,
            profile=str(data.get("profile", "full")),
        )


# ---------------------------------------------------------------------------
# Discovery and load
# ---------------------------------------------------------------------------


def _candidate_dirs() -> Iterable[Path]:
    if USER_PRESET_DIR.is_dir():
        yield USER_PRESET_DIR
    for d in SHIPPED_PRESET_DIRS:
        if d.is_dir():
            yield d


def list_presets() -> list[Preset]:
    """All presets visible to Mackes, with user overrides shadowing shipped ones.

    The default preset (DEFAULT_PRESET_NAME) is sorted to the front so callers
    that index [0] (e.g. the wizard's preset picker) get the right starting
    selection without extra logic.
    """
    if yaml is None:
        return []
    seen: dict[str, Preset] = {}
    for d in _candidate_dirs():
        for path in sorted(d.glob("*.yaml")):
            try:
                data = yaml.safe_load(path.read_text(encoding="utf-8")) or {}
            except (OSError, yaml.YAMLError):
                continue
            preset = Preset.from_dict(data, source=path)
            seen.setdefault(preset.name, preset)
    presets = list(seen.values())
    presets.sort(key=lambda p: (p.name != DEFAULT_PRESET_NAME, p.name))
    return presets


def default_preset() -> Optional[Preset]:
    """The shipped default. Falls back to the first available if missing."""
    preset = load_preset(DEFAULT_PRESET_NAME)
    if preset is not None:
        return preset
    presets = list_presets()
    return presets[0] if presets else None


def load_preset(name: str) -> Optional[Preset]:
    for preset in list_presets():
        if preset.name == name:
            return preset
    return None


# ---------------------------------------------------------------------------
# Apply pipeline
# ---------------------------------------------------------------------------


# Map preset.appearance keys -> (xfconf channel, property)
APPEARANCE_KEYS: dict[str, tuple[str, str]] = {
    "gtk_theme":      ("xsettings", "/Net/ThemeName"),
    "icon_theme":     ("xsettings", "/Net/IconThemeName"),
    "cursor_theme":   ("xsettings", "/Gtk/CursorThemeName"),
    "cursor_size":    ("xsettings", "/Gtk/CursorThemeSize"),
    "font_ui":        ("xsettings", "/Gtk/FontName"),
    "font_monospace": ("xsettings", "/Gtk/MonospaceFontName"),
}

# Wallpaper key — per-monitor in reality; we apply to the canonical default.
WALLPAPER_KEY = ("xfce4-desktop", "/backdrop/screen0/monitor0/workspace0/last-image")


def apply_appearance(preset: Preset) -> list[str]:
    """Push the preset's appearance section into xfconf. Returns a log of writes."""
    from mackes.xfconf_bridge import get_bridge, XfconfError
    actions: list[str] = []
    try:
        xf = get_bridge()
    except XfconfError as e:
        actions.append(f"skip appearance: {e}")
        return actions
    for field_name, value in preset.appearance.items():
        if field_name == "wallpaper":
            continue  # handled below
        binding = APPEARANCE_KEYS.get(field_name)
        if binding is None:
            actions.append(f"skip unknown appearance key: {field_name}")
            continue
        channel, prop = binding
        try:
            xf.set(channel, prop, value)
            actions.append(f"set {channel}{prop} = {value!r}")
        except XfconfError as e:
            actions.append(f"failed {channel}{prop}: {e}")
    wp = preset.appearance.get("wallpaper")
    # v1.4.6: fall back to the bundled standard-wallpaper if the preset's
    # declared path is missing. Stops a fresh install ending with a black
    # desktop because the preset's wallpaper path didn't survive.
    if wp and not Path(str(wp)).exists():
        for fallback in (
            "/usr/share/mackes-shell/branding/standard-wallpaper.png",
            str(Path(__file__).resolve().parent.parent
                / "branding" / "standard-wallpaper.png"),
        ):
            if Path(fallback).exists():
                actions.append(
                    f"wallpaper: {wp} missing, falling back to {fallback}"
                )
                wp = fallback
                break
    if wp and Path(str(wp)).exists():
        try:
            xf.set(*WALLPAPER_KEY, str(wp), type_hint="string")
            actions.append(f"set wallpaper -> {wp}")
            # Also stamp common per-monitor keys — XFCE 4.18+ reads
            # /backdrop/screen0/monitor<NAME>/workspace0/last-image
            # before the canonical key on some builds.
            for alt_prop in (
                "/backdrop/screen0/monitorVGA-1/workspace0/last-image",
                "/backdrop/screen0/monitorHDMI-1/workspace0/last-image",
                "/backdrop/screen0/monitorHDMI-A-1/workspace0/last-image",
                "/backdrop/screen0/monitoreDP-1/workspace0/last-image",
                "/backdrop/screen0/monitorLVDS-1/workspace0/last-image",
            ):
                try:
                    xf.set("xfce4-desktop", alt_prop, str(wp),
                           type_hint="string")
                except XfconfError:
                    pass
        except XfconfError as e:
            actions.append(f"failed wallpaper: {e}")
    elif wp:
        actions.append(f"wallpaper: {wp} not found on disk, skipped")

    # LightDM greeter — mirror the preset's look on the login screen.
    if preset.appearance:
        try:
            from mackes.lightdm import configure_from_preset
            wp_path = Path(str(wp)) if wp else None
            actions.extend(configure_from_preset(preset.name, wallpaper=wp_path))
        except Exception as e:  # noqa: BLE001
            actions.append(f"lightdm config: {e}")
    return actions


def apply_devices(preset: Preset) -> list[str]:
    # v2.0.0 Phase C.13 — preset device-settings now route through
    # mde_settings_bridge instead of xfconf. The Rust applier in
    # crates/mackesd/src/settings/power.rs picks up the sidecar
    # write + invokes powerprofilesctl on the host.
    from mackes import mde_settings_bridge as bridge
    actions: list[str] = []
    if "power_profile" in preset.devices:
        prof = str(preset.devices["power_profile"])
        if bridge.power_profile_set(prof):
            actions.append(f"power profile -> {prof}")
        else:
            actions.append(f"power: powerprofilesctl rejected {prof}")
    if "audio_default_sink" in preset.devices:
        actions.append(f"audio sink (informational only): {preset.devices['audio_default_sink']}")
    return actions


def apply_system(preset: Preset) -> list[str]:
    # v2.0.0 Phase C.13 — preset system-settings now route through
    # mde_settings_bridge. The Rust applier in
    # crates/mackesd/src/settings/ owns the side effect.
    from mackes import mde_settings_bridge as bridge
    actions: list[str] = []
    if "workspace_count" in preset.system:
        # sway uses a static workspace count via the
        # workspace.count key (Phase C.8 keybinds applier).
        count = int(preset.system["workspace_count"])
        if bridge.set_setting("workspace.count", count):
            actions.append(f"workspaces -> {count}")
        else:
            actions.append("workspaces: bridge.set_setting rejected")
    if "window_manager_theme" in preset.system:
        # sway doesn't carry a "WM theme" knob — the panel +
        # libcosmic theme covers what xfwm4 themes used to do.
        # Preset hint becomes informational.
        actions.append(
            f"window manager theme (informational only — sway uses libcosmic): "
            f"{preset.system['window_manager_theme']}"
        )
    if "notifications_enabled" in preset.system:
        enabled = bool(preset.system["notifications_enabled"])
        # Notifications stay on system-wide; the per-user DND toggle
        # lives at notification.do_not_disturb (a flag-file the
        # notifications_server worker honors).
        if bridge.set_setting("notification.do_not_disturb", not enabled):
            actions.append(f"notifications -> {'on' if enabled else 'off'}")
        else:
            actions.append("notifyd: bridge.set_setting rejected")
    extras = preset.system.get("autostart_extras") or []
    if extras:
        actions.append(f"autostart hint (manual): {', '.join(extras)}")
    return actions


def apply_network(preset: Preset) -> list[str]:
    from mackes.qnm_bridge import set_qnm_enabled
    actions: list[str] = []
    if "qnm_enabled" in preset.network:
        actions.extend(set_qnm_enabled(bool(preset.network["qnm_enabled"])))
    if "firewall_default_zone" in preset.network:
        actions.append(f"firewall zone hint: {preset.network['firewall_default_zone']} "
                       "(use Network → Firewall to apply)")
    return actions


def apply_panel(preset: Preset) -> list[str]:
    """Apply xfce4-panel plugin overrides from preset.panel.

    Today this writes the clock plugin's format/font/layout to every
    plugin whose root value is 'clock' in the xfce4-panel xfconf channel.
    Extensible: other plugin types (docklike, whiskermenu) can grow their
    own preset.panel.<name> blocks the same way.
    """
    import subprocess
    from mackes.xfconf_bridge import get_bridge, XfconfError
    actions: list[str] = []
    clock = preset.panel.get("clock") or {}
    if not clock:
        return actions
    try:
        xf = get_bridge()
    except XfconfError as e:
        actions.append(f"skip panel: {e}")
        return actions
    # Find every clock plugin in the live xfce4-panel xfconf channel.
    try:
        out = subprocess.check_output(
            ["xfconf-query", "--channel", "xfce4-panel", "--list", "--verbose"],
            stderr=subprocess.DEVNULL, text=True, timeout=5,
        )
    except (OSError, subprocess.CalledProcessError, subprocess.TimeoutExpired) as e:
        actions.append(f"panel: could not list xfce4-panel channel: {e}")
        return actions
    clock_ids: list[str] = []
    for line in out.splitlines():
        parts = line.split()
        if len(parts) == 2 and parts[1] == "clock":
            key = parts[0]
            if key.startswith("/plugins/plugin-") and "/" not in key[len("/plugins/plugin-"):]:
                clock_ids.append(key[len("/plugins/plugin-"):])
    if not clock_ids:
        actions.append("panel.clock: no clock plugin found in xfce4-panel (skipping)")
        return actions
    for pid in clock_ids:
        for key, value in clock.items():
            prop = f"/plugins/plugin-{pid}/{key}"
            try:
                if isinstance(value, str):
                    xf.set("xfce4-panel", prop, value, type_hint="string")
                else:
                    xf.set("xfce4-panel", prop, value)
                actions.append(f"panel clock-{pid}.{key} = {value!r}")
            except XfconfError as e:
                actions.append(f"panel clock-{pid}.{key} failed: {e}")
    return actions


def apply_mesh(preset: Preset) -> list[str]:
    """Initialize mesh subsystems based on the preset's network block.

    For 'node' preset (or any preset with network.mesh_*_enabled=true):
      - ensure mesh-SSH keypair + publish pubkey
      - ensure mesh-fs directories
      - ensure mesh-sync bucket dirs
      - install Thunar bookmarks pointing at QNM-* directories
    """
    actions: list[str] = []
    net = preset.network or {}
    if net.get("mesh_ssh_auto_keys", True):
        try:
            from mackes.mesh_ssh import ensure_mesh_keypair, publish_my_pubkey
            actions.extend(ensure_mesh_keypair())
            actions.extend(publish_my_pubkey())
        except Exception as e:  # noqa: BLE001
            actions.append(f"mesh-ssh init: {e}")
    if net.get("mesh_fs_enabled", True):
        try:
            from mackes.mesh_fs import ensure_dirs as mfs_ensure
            actions.extend(mfs_ensure())
        except Exception as e:  # noqa: BLE001
            actions.append(f"mesh-fs init: {e}")
    if net.get("mesh_sync_enabled", True):
        try:
            from mackes.mesh_sync import ensure_buckets
            actions.extend(ensure_buckets())
        except Exception as e:  # noqa: BLE001
            actions.append(f"mesh-sync init: {e}")
    try:
        from mackes.mesh_browser import ensure_layout, install_thunar_bookmarks
        actions.extend(ensure_layout())
        actions.extend(install_thunar_bookmarks())
    except Exception as e:  # noqa: BLE001
        actions.append(f"mesh-browser layout: {e}")
    return actions


def apply_preset(preset: Preset, *, sections: Optional[set[str]] = None) -> list[str]:
    """Apply selected (or all) sections of a preset.

    sections: optional subset of
        {appearance, devices, system, network, panel, mesh}.
    Returns a flat list of human-readable action lines, also written to
    mackes.log.
    """
    sections = sections or {"appearance", "devices", "system", "network",
                            "panel", "mesh"}
    actions: list[str] = [f"--- apply preset: {preset.name} ---"]
    if "appearance" in sections:
        actions.extend(apply_appearance(preset))
    if "devices" in sections:
        actions.extend(apply_devices(preset))
    if "system" in sections:
        actions.extend(apply_system(preset))
    if "network" in sections:
        actions.extend(apply_network(preset))
    if "panel" in sections:
        actions.extend(apply_panel(preset))
    if "mesh" in sections:
        actions.extend(apply_mesh(preset))
    for line in actions:
        log_action(line)
    return actions


# ---------------------------------------------------------------------------
# Drift detection (read-only; informational on Dashboard §4.2)
# ---------------------------------------------------------------------------


@dataclass
class DriftItem:
    section: str
    field: str
    expected: Any
    actual: Any

    def describe(self) -> str:
        return f"{self.section}.{self.field}: preset={self.expected!r} live={self.actual!r}"


def _read_appearance_actual() -> dict[str, Any]:
    from mackes.xfconf_bridge import get_bridge, XfconfError
    try:
        xf = get_bridge()
    except XfconfError:
        return {}
    out: dict[str, Any] = {}
    for field_name, (channel, prop) in APPEARANCE_KEYS.items():
        out[field_name] = xf.get(channel, prop, None)
    out["wallpaper"] = xf.get(*WALLPAPER_KEY, "")
    return out


def _read_system_actual() -> dict[str, Any]:
    from mackes.xfconf_bridge import get_bridge, XfconfError
    try:
        xf = get_bridge()
    except XfconfError:
        return {}
    return {
        "workspace_count":      xf.get("xfwm4", "/general/workspace_count", None),
        "window_manager_theme": xf.get("xfwm4", "/general/theme", None),
        "notifications_enabled": (xf.get("xfce4-notifyd", "/notify-location", 1) != 0),
    }


def detect_drift(preset: Preset) -> list[DriftItem]:
    items: list[DriftItem] = []
    sections: list[tuple[str, dict[str, Any], dict[str, Any]]] = [
        ("appearance", preset.appearance, _read_appearance_actual()),
        ("system",     preset.system,     _read_system_actual()),
    ]
    for section_name, expected_dict, actual_dict in sections:
        for field_name, expected in expected_dict.items():
            if field_name == "autostart_extras":
                continue  # informational; no drift check
            actual = actual_dict.get(field_name, None)
            if actual is None:
                continue
            if str(actual).strip() != str(expected).strip():
                items.append(DriftItem(section_name, field_name, expected, actual))
    return items


# Convenience for the Dashboard
def active_preset_drift() -> tuple[Optional[Preset], list[DriftItem]]:
    from mackes.state import MackesState
    state = MackesState.load()
    if not state.active_preset:
        return None, []
    preset = load_preset(state.active_preset)
    if preset is None:
        return None, []
    return preset, detect_drift(preset)
