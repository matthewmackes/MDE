"""Carbon Accordion — collapsible content sections."""
from __future__ import annotations

from typing import Optional

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402


class AccordionItem(Gtk.Box):
    """Single collapsible row inside an Accordion."""

    def __init__(self, title: str, body: Gtk.Widget, *, expanded: bool = False) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.get_style_context().add_class("cds-accordion-item")

        self._header = Gtk.Button()
        self._header.get_style_context().add_class("cds-button-ghost")
        self._header.set_relief(Gtk.ReliefStyle.NONE)
        hbox = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        hbox.set_margin_top(12); hbox.set_margin_bottom(12)
        hbox.set_margin_start(16); hbox.set_margin_end(16)
        self._arrow = Gtk.Label(label="▸")
        self._arrow.set_xalign(0)
        hbox.pack_start(self._arrow, False, False, 0)
        title_lbl = Gtk.Label(label=title)
        title_lbl.set_xalign(0)
        title_lbl.get_style_context().add_class("cds-heading-01")
        hbox.pack_start(title_lbl, True, True, 0)
        self._header.add(hbox)
        self._header.connect("clicked", self._on_toggle)
        self.pack_start(self._header, False, False, 0)

        self._revealer = Gtk.Revealer()
        self._revealer.set_transition_type(Gtk.RevealerTransitionType.SLIDE_DOWN)
        self._revealer.set_transition_duration(150)
        body_wrap = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        body_wrap.set_margin_start(16); body_wrap.set_margin_end(16)
        body_wrap.set_margin_bottom(16)
        body_wrap.pack_start(body, True, True, 0)
        self._revealer.add(body_wrap)
        self.pack_start(self._revealer, False, False, 0)

        sep = Gtk.Separator(orientation=Gtk.Orientation.HORIZONTAL)
        sep.get_style_context().add_class("cds-accordion-divider")
        self.pack_start(sep, False, False, 0)

        self._expanded = expanded
        self._revealer.set_reveal_child(expanded)
        self._arrow.set_label("▾" if expanded else "▸")

    def _on_toggle(self, _btn: Gtk.Button) -> None:
        self._expanded = not self._expanded
        self._revealer.set_reveal_child(self._expanded)
        self._arrow.set_label("▾" if self._expanded else "▸")

    def is_expanded(self) -> bool:
        return self._expanded

    def set_expanded(self, expanded: bool) -> None:
        if expanded != self._expanded:
            self._on_toggle(self._header)


class Accordion(Gtk.Box):
    """Container for AccordionItem instances."""

    def __init__(self) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.get_style_context().add_class("cds-accordion")
        self._items: list[AccordionItem] = []

    def add_item(self, item: AccordionItem) -> None:
        self._items.append(item)
        self.pack_start(item, False, False, 0)

    def collapse_all(self) -> None:
        for item in self._items:
            item.set_expanded(False)
