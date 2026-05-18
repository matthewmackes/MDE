"""Mesh tab — merges Get Online / Health / Performance / VPN / SSH /
Services / Remote behind a sub-tab strip (Q10 lock: merge close-cousin
panels)."""
from __future__ import annotations

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402

from mackes.workbench.popover._subtabs import build_subtab_container


class MeshTab(Gtk.Box):
    def __init__(self) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        items = [
            ("online",   "", "Get Online",
             "mackes.workbench.network.mesh_join:MeshJoinPanel"),
            ("health",   "", "Health",
             "mackes.workbench.network.mesh_health:MeshHealthPanel"),
            ("perf",     "", "Perf",
             "mackes.workbench.network.mesh_performance:MeshPerformancePanel"),
            ("ssh",      "", "SSH",
             "mackes.workbench.network.mesh_ssh:MeshSshPanel"),
        ]
        self.pack_start(build_subtab_container(items), True, True, 0)


__all__ = ["MeshTab"]
