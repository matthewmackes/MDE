"""Maintain → Repair.

Common recovery operations:
  - Re-apply the active preset (fixes drift)
  - Rebuild the menu folder (re-hide xfce4-settings entries)
  - Restore xfce4-settings menu entries (un-hide; for when Mackes is leaving)
  - Re-install the mackes-shell .desktop entry
"""
from __future__ import annotations

from pathlib import Path

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk  # noqa: E402

from mackes.menu_integration import (
    hide_xfce_settings_entries, install_mackes_menu_entry, restore_xfce_settings_entries,
)
from mackes.presets import apply_preset, load_preset
from mackes.state import MackesState
from mackes.workbench._common import (
    info_label, panel_box, section_header, title_label,
)


SHIPPED_DESKTOP_CANDIDATES = [
    Path("/usr/share/applications/mackes-shell.desktop"),
    Path(__file__).resolve().parent.parent.parent.parent / "data" / "applications" / "mackes-shell.desktop",
]


def _ship_desktop() -> Path | None:
    for c in SHIPPED_DESKTOP_CANDIDATES:
        if c.exists():
            return c
    return None


class RepairPanel(Gtk.Box):
    def __init__(self, state: MackesState) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.state = state
        self._build()

    def _build(self) -> None:
        box = panel_box()
        box.pack_start(title_label("Repair"), False, False, 0)
        box.pack_start(info_label(
            "Recovery operations. Each is non-destructive on its own — but "
            "Reset to Preset is the one that overwrites local changes, so use "
            "this panel first if you're not sure."
        ), False, False, 0)

        box.pack_start(section_header("Output"), False, False, 0)
        self._output = Gtk.TextView()
        self._output.set_editable(False); self._output.set_monospace(True)
        self._output.set_size_request(-1, 220)
        scroll = Gtk.ScrolledWindow(); scroll.add(self._output)
        scroll.set_size_request(-1, 220)
        box.pack_start(scroll, True, True, 0)

        box.pack_start(section_header("Actions"), False, False, 0)
        grid = Gtk.Grid(column_spacing=8, row_spacing=8, column_homogeneous=True)

        ops = [
            ("Re-apply active preset",        self._reapply_preset),
            ("Re-hide xfce4-settings menu",   self._rehide_menus),
            ("Restore xfce4-settings menu",   self._restore_menus),
            ("Re-install Mackes menu entry",  self._reinstall_entry),
        ]
        for i, (label, fn) in enumerate(ops):
            btn = Gtk.Button(label=label)
            btn.connect("clicked", lambda _b, f=fn: self._run(f))
            grid.attach(btn, i % 2, i // 2, 1, 1)
        box.pack_start(grid, False, False, 0)

        self.add(box)
        self._append("Ready. Click an action above.\n")

    def _append(self, text: str) -> None:
        buf = self._output.get_buffer()
        end = buf.get_end_iter()
        buf.insert(end, text)
        end = buf.get_end_iter()
        self._output.scroll_to_iter(end, 0, False, 0, 1)

    def _run(self, fn) -> None:
        self._append(f"\n--- {fn.__doc__ or fn.__name__} ---\n")
        try:
            for line in fn():
                self._append(line + "\n")
        except Exception as e:  # noqa: BLE001
            self._append(f"ERROR: {e}\n")

    # ----- operations -----------------------------------------------------

    def _reapply_preset(self) -> list[str]:
        """Re-apply the active preset"""
        if not self.state.active_preset:
            return ["No active preset set in state.json."]
        preset = load_preset(self.state.active_preset)
        if preset is None:
            return [f"Preset {self.state.active_preset!r} not found."]
        return apply_preset(preset)

    def _rehide_menus(self) -> list[str]:
        """Re-hide xfce4-settings menu entries"""
        return hide_xfce_settings_entries()

    def _restore_menus(self) -> list[str]:
        """Restore xfce4-settings menu entries"""
        return restore_xfce_settings_entries()

    def _reinstall_entry(self) -> list[str]:
        """Re-install the Mackes menu entry"""
        src = _ship_desktop()
        if src is None:
            return ["mackes-shell.desktop not found in shipped data."]
        return install_mackes_menu_entry(src)
