"""Carbon Skeleton loaders — grey placeholders while data is fetching."""
from __future__ import annotations

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402


class SkeletonLine(Gtk.Box):
    """A single skeleton line (placeholder for a text line)."""

    def __init__(self, *, width: int = 200, height: int = 16) -> None:
        super().__init__(orientation=Gtk.Orientation.HORIZONTAL, spacing=0)
        self.get_style_context().add_class("cds-skeleton")
        self.set_size_request(width, height)
        self.set_margin_top(4); self.set_margin_bottom(4)


class Skeleton(Gtk.Box):
    """Vertical stack of skeleton lines, sized per the locked variants."""

    def __init__(self, *, lines: int = 3, line_widths: list[int] | None = None) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        self.get_style_context().add_class("cds-skeleton-container")
        if line_widths is None:
            line_widths = [240, 320, 180][:lines]
            line_widths += [200] * max(0, lines - len(line_widths))
        for w in line_widths:
            self.pack_start(SkeletonLine(width=w), False, False, 0)
