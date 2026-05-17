"""Carbon Tile — basic and clickable variants.

A Tile is a container with a heading + body + optional footer. Used
heavily on the Mesh Services Hub, Mesh VPN peer list, Dashboard widgets.
"""
from __future__ import annotations

from typing import Callable, Optional

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gdk, Gtk  # noqa: E402


class Tile(Gtk.Box):
    """Static Tile — visual grouping of content with Carbon styling."""

    def __init__(self, *, title: Optional[str] = None) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        self.get_style_context().add_class("cds-tile")
        # spacing-05 = 16px on all four sides
        self.set_margin_top(0); self.set_margin_bottom(0)
        self.set_margin_start(0); self.set_margin_end(0)

        self._body = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        self._body.set_margin_top(16); self._body.set_margin_bottom(16)
        self._body.set_margin_start(16); self._body.set_margin_end(16)

        if title:
            self._title = Gtk.Label(label=title)
            self._title.set_xalign(0)
            self._title.get_style_context().add_class("cds-heading-02")
            self._body.pack_start(self._title, False, False, 0)

        self.add(self._body)

    def pack(self, widget: Gtk.Widget, expand: bool = False, fill: bool = False) -> None:
        self._body.pack_start(widget, expand, fill, 0)

    def set_body_padding(self, px: int) -> None:
        self._body.set_margin_top(px); self._body.set_margin_bottom(px)
        self._body.set_margin_start(px); self._body.set_margin_end(px)


class ClickableTile(Gtk.EventBox):
    """Clickable Tile — wraps a Tile in an EventBox so clicks fire a callback.

    Use for entries in the Media Hub Tile grid, peer Tiles in Mesh SSH /
    Mesh VPN, etc.
    """

    def __init__(
        self,
        *,
        title: Optional[str] = None,
        on_click: Optional[Callable[[], None]] = None,
    ) -> None:
        super().__init__()
        self.get_style_context().add_class("cds-tile")
        self.get_style_context().add_class("cds-tile-clickable")
        self._inner = Tile(title=title)
        self._inner.get_style_context().remove_class("cds-tile")  # we own that class
        self.add(self._inner)
        if on_click is not None:
            self.connect("button-release-event", lambda *_: (on_click(), False)[1])
        # Show hand cursor on enter
        self.connect("realize", self._on_realize)
        self.set_visible_window(True)

    def _on_realize(self, _w: Gtk.Widget) -> None:
        window = self.get_window()
        if window is not None:
            display = Gdk.Display.get_default()
            if display is not None:
                window.set_cursor(Gdk.Cursor.new_from_name(display, "pointer"))

    def pack(self, widget: Gtk.Widget, expand: bool = False, fill: bool = False) -> None:
        self._inner.pack(widget, expand, fill)
