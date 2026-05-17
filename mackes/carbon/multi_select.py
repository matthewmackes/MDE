"""Carbon MultiSelect — checkbox list bundled with a single label.

Used for: bloat-removal selection, mesh-services category filters,
mDNS-relay per-service-type opt-outs, etc.
"""
from __future__ import annotations

from typing import Callable, Iterable, Optional

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402


class MultiSelect(Gtk.Box):
    def __init__(
        self,
        label: str,
        items: Iterable[tuple[str, str, bool]],
        *,
        on_change: Optional[Callable[[list[str]], None]] = None,
    ) -> None:
        """
        items: iterable of (key, display, initial-checked)
        """
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        self.get_style_context().add_class("cds-multi-select")

        if label:
            lbl = Gtk.Label(label=label)
            lbl.set_xalign(0)
            lbl.get_style_context().add_class("cds-heading-01")
            self.pack_start(lbl, False, False, 0)

        self._on_change = on_change
        self._checks: list[tuple[Gtk.CheckButton, str]] = []
        for key, display, checked in items:
            cb = Gtk.CheckButton(label=display)
            cb.set_active(bool(checked))
            cb.connect("toggled", lambda _b, _k=key: self._fire())
            self._checks.append((cb, key))
            self.pack_start(cb, False, False, 0)

    def _fire(self) -> None:
        if self._on_change is not None:
            self._on_change(self.selected_keys())

    def selected_keys(self) -> list[str]:
        return [k for cb, k in self._checks if cb.get_active()]

    def set_selected(self, keys: Iterable[str]) -> None:
        want = set(keys)
        for cb, k in self._checks:
            cb.set_active(k in want)
