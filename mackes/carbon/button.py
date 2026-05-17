"""Carbon Button — 5-tier hierarchy (Q-CB5).

  ButtonKind.PRIMARY    — primary action (preset-accent fill)
  ButtonKind.SECONDARY  — secondary action (cds-bg-layer-02 fill)
  ButtonKind.TERTIARY   — tertiary (accent-colored outline)
  ButtonKind.GHOST      — ghost (transparent, accent text)
  ButtonKind.DANGER     — destructive action (red fill)
"""
from __future__ import annotations

import enum
from typing import Callable, Optional

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402


class ButtonKind(enum.Enum):
    PRIMARY   = "cds-button-primary"
    SECONDARY = "cds-button-secondary"
    TERTIARY  = "cds-button-tertiary"
    GHOST     = "cds-button-ghost"
    DANGER    = "cds-button-danger"


class Button(Gtk.Button):
    """Carbon-styled Gtk.Button.

    Adds the cds-button-<kind> CSS class so tokens.css can style it. Falls
    back gracefully if tokens.css isn't loaded (just looks like a stock
    GTK button).
    """

    def __init__(
        self,
        label: str = "",
        *,
        kind: ButtonKind = ButtonKind.SECONDARY,
        icon_name: Optional[str] = None,
        on_click: Optional[Callable[[], None]] = None,
        tooltip: Optional[str] = None,
    ) -> None:
        super().__init__()
        self._kind = kind
        self.get_style_context().add_class(kind.value)
        # Cross-compat with existing GTK conventions used by mackes code
        if kind is ButtonKind.PRIMARY:
            self.get_style_context().add_class("suggested-action")
        elif kind is ButtonKind.DANGER:
            self.get_style_context().add_class("destructive-action")

        if icon_name or label:
            box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
            if icon_name:
                box.pack_start(
                    Gtk.Image.new_from_icon_name(icon_name, Gtk.IconSize.BUTTON),
                    False, False, 0,
                )
            if label:
                lbl = Gtk.Label(label=label)
                lbl.set_xalign(0.5)
                box.pack_start(lbl, True, True, 0)
            self.add(box)
        else:
            self.set_label("")

        if tooltip:
            self.set_tooltip_text(tooltip)
        if on_click is not None:
            self.connect("clicked", lambda *_: on_click())

    def set_kind(self, kind: ButtonKind) -> None:
        ctx = self.get_style_context()
        for k in ButtonKind:
            ctx.remove_class(k.value)
        ctx.add_class(kind.value)
        self._kind = kind
