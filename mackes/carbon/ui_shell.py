"""Carbon UIShell — header + side nav + content + status bar.

Used as the top-level scaffold for the Mackes workbench (Q-CB2 lock).
GTK doesn't have a single 'shell' widget, so this composes Gtk.HeaderBar
+ side-nav Gtk.ListBox + main-content Gtk.Stack + bottom status Gtk.Box.
"""
from __future__ import annotations

from dataclasses import dataclass
from typing import Callable, Optional

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402


@dataclass
class SideNavItem:
    key: str
    label: str
    icon_name: Optional[str] = None
    badge: Optional[str] = None  # small text badge (e.g. peer count)


class UIShell(Gtk.Box):
    """Top-level Carbon shell: header / side-nav / content / status bar."""

    def __init__(
        self,
        *,
        title: str = "Mackes Shell",
        side_nav_items: Optional[list[SideNavItem]] = None,
        on_select: Optional[Callable[[str], None]] = None,
    ) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.get_style_context().add_class("cds-ui-shell")
        self._on_select = on_select

        # ----- Top header (48px) -----
        self._header = Gtk.HeaderBar()
        self._header.set_show_close_button(True)
        self._header.set_title(title)
        self._header.get_style_context().add_class("cds-ui-shell-header")
        self.pack_start(self._header, False, False, 0)

        # ----- Body: side nav + content -----
        body = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=0)

        # Side nav
        side_wrap = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        side_wrap.set_size_request(256, -1)
        side_wrap.get_style_context().add_class("cds-ui-shell-side-nav")

        self._side_list = Gtk.ListBox()
        self._side_list.set_selection_mode(Gtk.SelectionMode.SINGLE)
        self._side_list.connect("row-selected", self._on_row_selected)
        scroll_side = Gtk.ScrolledWindow()
        scroll_side.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        scroll_side.add(self._side_list)
        side_wrap.pack_start(scroll_side, True, True, 0)

        if side_nav_items:
            for item in side_nav_items:
                self.add_side_nav_item(item)

        body.pack_start(side_wrap, False, False, 0)
        body.pack_start(Gtk.Separator(orientation=Gtk.Orientation.VERTICAL),
                        False, False, 0)

        # Content stack
        self._content = Gtk.Stack()
        self._content.set_transition_type(Gtk.StackTransitionType.CROSSFADE)
        self._content.set_transition_duration(150)
        body.pack_start(self._content, True, True, 0)

        self.pack_start(body, True, True, 0)

        # ----- Status bar (24px) -----
        self._status = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=16)
        self._status.get_style_context().add_class("cds-status-bar")
        self._status.set_size_request(-1, 24)
        self._status_left = Gtk.Label(label="")
        self._status_left.set_xalign(0)
        self._status_left.set_margin_start(16)
        self._status.pack_start(self._status_left, True, True, 0)
        self._status_right = Gtk.Label(label="")
        self._status_right.set_xalign(1)
        self._status_right.set_margin_end(16)
        self._status.pack_end(self._status_right, False, False, 0)
        self.pack_end(self._status, False, False, 0)

    # ---- public API -----------------------------------------------------

    def add_side_nav_item(self, item: SideNavItem) -> None:
        row = Gtk.ListBoxRow()
        row.nav_key = item.key  # type: ignore[attr-defined]
        inner = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        inner.set_margin_top(12); inner.set_margin_bottom(12)
        inner.set_margin_start(16); inner.set_margin_end(16)
        if item.icon_name:
            img = Gtk.Image.new_from_icon_name(item.icon_name, Gtk.IconSize.MENU)
            inner.pack_start(img, False, False, 0)
        lbl = Gtk.Label(label=item.label)
        lbl.set_xalign(0)
        inner.pack_start(lbl, True, True, 0)
        if item.badge:
            badge = Gtk.Label(label=item.badge)
            badge.get_style_context().add_class("cds-side-nav-badge")
            inner.pack_end(badge, False, False, 0)
        row.add(inner)
        self._side_list.add(row)
        row.show_all()

    def add_content_panel(self, key: str, widget: Gtk.Widget) -> None:
        self._content.add_named(widget, key)

    def show_panel(self, key: str) -> None:
        self._content.set_visible_child_name(key)

    def header_pack_end(self, widget: Gtk.Widget) -> None:
        self._header.pack_end(widget)

    def header_pack_start(self, widget: Gtk.Widget) -> None:
        self._header.pack_start(widget)

    def set_status_left(self, text: str) -> None:
        self._status_left.set_text(text)

    def set_status_right(self, text: str) -> None:
        self._status_right.set_text(text)

    # ---- internal -------------------------------------------------------

    def _on_row_selected(self, _box: Gtk.ListBox, row: Optional[Gtk.ListBoxRow]) -> None:
        if row is None:
            return
        key = getattr(row, "nav_key", None)
        if key is None:
            return
        self.show_panel(key)
        if self._on_select is not None:
            self._on_select(key)
