"""Carbon Inline Notification — info/success/warning/error/highlight bars."""
from __future__ import annotations

import enum
from typing import Callable, Optional

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402


class NotificationKind(enum.Enum):
    INFO      = "cds-notification-info"
    SUCCESS   = "cds-notification-success"
    WARNING   = "cds-notification-warning"
    ERROR     = "cds-notification-error"
    HIGHLIGHT = "cds-notification-highlight"


_ICON = {
    NotificationKind.INFO:      "dialog-information-symbolic",
    NotificationKind.SUCCESS:   "emblem-default-symbolic",
    NotificationKind.WARNING:   "dialog-warning-symbolic",
    NotificationKind.ERROR:     "dialog-error-symbolic",
    NotificationKind.HIGHLIGHT: "emblem-important-symbolic",
}


class Notification(Gtk.Box):
    """Inline notification — colored left border + icon + title/body + close X."""

    def __init__(
        self,
        title: str,
        *,
        body: str = "",
        kind: NotificationKind = NotificationKind.INFO,
        dismissible: bool = True,
        on_dismiss: Optional[Callable[[], None]] = None,
    ) -> None:
        super().__init__(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        self.get_style_context().add_class("cds-notification")
        self.get_style_context().add_class(kind.value)
        self.set_margin_top(8); self.set_margin_bottom(8)
        self.set_margin_start(0); self.set_margin_end(0)

        icon = Gtk.Image.new_from_icon_name(_ICON[kind], Gtk.IconSize.MENU)
        icon.set_margin_top(12); icon.set_margin_bottom(12)
        icon.set_margin_start(12); icon.set_margin_end(0)
        self.pack_start(icon, False, False, 0)

        text_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
        text_box.set_margin_top(8); text_box.set_margin_bottom(8)
        text_box.set_margin_start(0); text_box.set_margin_end(0)
        title_lbl = Gtk.Label(label=title); title_lbl.set_xalign(0)
        title_lbl.get_style_context().add_class("cds-heading-01")
        title_lbl.set_line_wrap(True)
        text_box.pack_start(title_lbl, False, False, 0)
        if body:
            body_lbl = Gtk.Label(label=body); body_lbl.set_xalign(0)
            body_lbl.set_line_wrap(True)
            body_lbl.get_style_context().add_class("cds-body-compact-01")
            text_box.pack_start(body_lbl, False, False, 0)
        self.pack_start(text_box, True, True, 0)

        if dismissible:
            close_btn = Gtk.Button()
            close_btn.set_relief(Gtk.ReliefStyle.NONE)
            close_btn.set_image(Gtk.Image.new_from_icon_name(
                "window-close-symbolic", Gtk.IconSize.MENU))
            close_btn.set_margin_top(8); close_btn.set_margin_bottom(8)
            close_btn.set_margin_end(8)
            close_btn.connect("clicked", lambda *_: (
                on_dismiss and on_dismiss(),
                self.get_parent() and self.get_parent().remove(self),
            ))
            self.pack_end(close_btn, False, False, 0)
