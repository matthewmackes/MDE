"""Devices → Display.

Most display arrangement (mirrored / extended / per-monitor resolution) is
done via xrandr in xfce4-display-settings. Mackes exposes the xfconf-bound
preferences (default scaling, primary monitor name) and provides a button
that launches `xrandr -q` output for visibility, plus a launcher to the
shipped xfce4 dialog as the fallback for live arrangement.
"""
from __future__ import annotations

import subprocess

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402

from mackes.xfconf_bridge import XfconfError, get_bridge
from mackes.workbench._common import (
    error_label, info_label, labeled_row, panel_box, section_header, title_label,
)


def _xrandr_summary() -> str:
    """One-line-per-output summary. Reads xfsettings `displays` channel
    when xrandr isn't installed (the common minimal-Fedora case)."""
    from mackes.probe_cache import cached

    def _probe() -> str:
        # Prefer the xfsettings channel — no shell-out, instant.
        try:
            from mackes.displays import list_outputs
            outs = list_outputs()
            if outs:
                lines = []
                for o in outs:
                    state = "primary" if o.primary else (
                        "active" if o.active else "off")
                    res = f"{o.width}x{o.height}" if o.active else "(off)"
                    lines.append(f"{o.name:10s} {state:8s} {res}")
                return "\n".join(lines)
        except Exception:  # noqa: BLE001
            pass
        # Fallback: xrandr CLI
        try:
            out = subprocess.check_output(
                ["xrandr", "--query"], text=True, stderr=subprocess.DEVNULL)
        except (FileNotFoundError, subprocess.CalledProcessError):
            return "xrandr not available."
        lines = []
        for line in out.splitlines():
            if " connected" in line or " disconnected" in line:
                lines.append(line.split(" (")[0])
        return "\n".join(lines) or "No displays detected."

    return cached("devices.display_summary", factory=_probe, ttl_s=60)


class DisplayPanel(Gtk.Box):
    def __init__(self) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.add(self._build())

    def _build(self) -> Gtk.Widget:
        box = panel_box()
        box.pack_start(title_label("Display"), False, False, 0)
        box.pack_start(info_label(
            "See which monitors are plugged in, and pick how much "
            "windows and text should scale up on a high-resolution "
            "screen."
        ), False, False, 0)

        try:
            xf = get_bridge()
        except XfconfError as e:
            box.pack_start(error_label(str(e)), False, False, 0)
            return box

        box.pack_start(section_header("Layout"), False, False, 0)
        view = Gtk.TextView()
        view.set_editable(False); view.set_monospace(True)
        view.get_buffer().set_text(_xrandr_summary())
        view.set_size_request(-1, 100)
        box.pack_start(view, False, False, 0)

        launch = Gtk.Button(label="Open xfce4-display-settings (live arrange)")
        def on_launch(_):
            subprocess.Popen(["xfce4-display-settings"], stdout=subprocess.DEVNULL,
                             stderr=subprocess.DEVNULL)
        launch.connect("clicked", on_launch)
        box.pack_start(launch, False, False, 0)

        box.pack_start(section_header("Defaults"), False, False, 0)

        scale = Gtk.SpinButton.new_with_range(0.5, 3.0, 0.05)
        scale.set_digits(2)
        scale.set_value(float(xf.get("xsettings", "/Gdk/WindowScalingFactor", 1.0) or 1.0))
        def on_scale(s):
            xf.set("xsettings", "/Gdk/WindowScalingFactor", float(s.get_value()), type_hint="double")
        scale.connect("value-changed", on_scale)
        box.pack_start(labeled_row("Window scaling factor", scale), False, False, 0)

        return box
