"""Manage tab — fleet + remote + tweaks + boot/login."""
from __future__ import annotations

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402

from mackes.workbench.popover._subtabs import build_subtab_container


class ManageTab(Gtk.Box):
    def __init__(self) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        items = [
            ("fleet",   "", "Fleet",
             "mackes.workbench.fleet.inventory:FleetInventoryPanel"),
            ("tweaks",  "", "Tweaks",
             "mackes.workbench.system.tweaks_full:TweaksPanel"),
            ("screens", "", "Screens",
             "mackes.workbench.system.displays:DisplaysPanel"),
            ("boot",    "", "Boot",
             "mackes.workbench.system.boot_login:BootLoginPanel"),
        ]
        self.pack_start(build_subtab_container(items), True, True, 0)


__all__ = ["ManageTab"]
