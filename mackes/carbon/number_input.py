"""Carbon NumberInput — labeled spin button with - / + adjusters."""
from __future__ import annotations

from typing import Callable, Optional

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402


class NumberInput(Gtk.Box):
    def __init__(
        self,
        label: str,
        *,
        value: int = 0,
        minimum: int = 0,
        maximum: int = 100,
        step: int = 1,
        on_change: Optional[Callable[[int], None]] = None,
    ) -> None:
        super().__init__(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        self.get_style_context().add_class("cds-number-input")

        if label:
            self._label = Gtk.Label(label=label)
            self._label.set_xalign(0)
            self._label.set_size_request(180, -1)
            self.pack_start(self._label, False, False, 0)

        adj = Gtk.Adjustment(value=value, lower=minimum, upper=maximum,
                             step_increment=step, page_increment=step * 10)
        self._spin = Gtk.SpinButton()
        self._spin.set_adjustment(adj)
        self._spin.set_numeric(True)
        self._spin.set_value(value)
        if on_change is not None:
            self._spin.connect(
                "value-changed",
                lambda b: on_change(int(b.get_value())),
            )
        self.pack_start(self._spin, False, False, 0)

    def get_value(self) -> int:
        return int(self._spin.get_value())

    def set_value(self, value: int) -> None:
        self._spin.set_value(value)
