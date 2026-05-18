"""Tools tab — apps + sources + system update + fonts."""
from __future__ import annotations

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402

from mackes.workbench.popover._subtabs import build_subtab_container


class ToolsTab(Gtk.Box):
    def __init__(self) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        items = [
            ("apps",    "", "Apps",
             "mackes.workbench.apps.panel:AppsPanel"),
            ("sources", "", "Sources",
             "mackes.workbench.apps.sources:SourcesPanel"),
            ("update",  "", "Update",
             "mackes.workbench.maintain.system_update:SystemUpdatePanel"),
            ("fonts",   "", "Fonts",
             "mackes.workbench.maintain.fonts:FontsPanel"),
        ]
        self.pack_start(build_subtab_container(items), True, True, 0)


__all__ = ["ToolsTab"]
