"""Wizard screen 2 — Environment Scan."""
from __future__ import annotations

import shutil
import subprocess

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402

from mackes.state import hardware_summary


REQUIRED_BINS = ["xfconf-query", "xfsettingsd", "xfce4-panel", "xfdesktop", "xfce4-appfinder"]
RECOMMENDED_BINS = ["nmcli", "firewall-cmd", "pactl", "timedatectl", "gsettings"]


def _xfce_version() -> str:
    try:
        out = subprocess.check_output(["xfce4-about", "--version"], text=True,
                                      stderr=subprocess.DEVNULL, timeout=3)
        return out.strip().splitlines()[0]
    except (FileNotFoundError, subprocess.CalledProcessError, subprocess.TimeoutExpired):
        return "unknown"


def build(ctx) -> Gtk.Widget:
    box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
    box.set_margin_top(28); box.set_margin_bottom(28)
    box.set_margin_start(40); box.set_margin_end(40)

    title = Gtk.Label(label="Environment Scan")
    title.set_xalign(0); title.get_style_context().add_class("title-1")
    box.pack_start(title, False, False, 0)

    hw = hardware_summary()
    ctx.detected = dict(hw)
    ctx.detected["xfce"] = _xfce_version()

    grid = Gtk.Grid(column_spacing=24, row_spacing=4)
    rows = [
        ("Hostname", hw.get("hostname", "")),
        ("OS",       hw.get("os", "")),
        ("CPU",      hw.get("cpu", "")),
        ("RAM",      hw.get("ram", "")),
        ("XFCE",     ctx.detected["xfce"]),
    ]
    for i, (k, v) in enumerate(rows):
        lk = Gtk.Label(label=k); lk.set_xalign(0); lk.get_style_context().add_class("dim-label")
        lv = Gtk.Label(label=str(v)); lv.set_xalign(0)
        grid.attach(lk, 0, i, 1, 1); grid.attach(lv, 1, i, 1, 1)
    box.pack_start(grid, False, False, 0)

    box.pack_start(Gtk.Separator(orientation=Gtk.Orientation.HORIZONTAL), False, False, 8)

    pkg_label = Gtk.Label(label="Required binaries")
    pkg_label.set_xalign(0); pkg_label.get_style_context().add_class("title-4")
    box.pack_start(pkg_label, False, False, 0)

    missing: list[str] = []
    for b in REQUIRED_BINS:
        present = shutil.which(b) is not None
        line = Gtk.Label(label=f"{'●' if present else '○'}  {b}{'' if present else '  — MISSING'}")
        line.set_xalign(0)
        line.get_style_context().add_class("success" if present else "error")
        box.pack_start(line, False, False, 0)
        if not present:
            missing.append(b)

    rec_label = Gtk.Label(label="Recommended binaries")
    rec_label.set_xalign(0); rec_label.get_style_context().add_class("title-4")
    box.pack_start(rec_label, False, False, 0)
    for b in RECOMMENDED_BINS:
        present = shutil.which(b) is not None
        line = Gtk.Label(label=f"{'●' if present else '○'}  {b}")
        line.set_xalign(0)
        line.get_style_context().add_class("success" if present else "dim-label")
        box.pack_start(line, False, False, 0)

    ctx.missing_packages = missing
    if missing:
        warn = Gtk.Label(label=(
            "Missing required binaries: " + ", ".join(missing) +
            "\nMackes will still let you continue. After the wizard, open "
            "Maintain → Dependencies to install them."
        ))
        warn.set_xalign(0); warn.set_line_wrap(True)
        warn.get_style_context().add_class("warning")
        box.pack_start(warn, False, False, 0)

    return box
