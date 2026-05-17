"""Carbon Toast — transient floating notifications.

Use the ToastHost overlay-style container at the top of the Mackes
window; Toast.show(host, …) presents the notification and auto-removes
it after a timeout.
"""
from __future__ import annotations

from typing import Optional

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk  # noqa: E402

from mackes.carbon.notification import NotificationKind


class Toast(Gtk.Box):
    """Single toast — typically created and shown via Toast.show()."""

    def __init__(
        self,
        title: str,
        *,
        body: str = "",
        kind: NotificationKind = NotificationKind.INFO,
    ) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=2)
        self.get_style_context().add_class("cds-toast")
        self.get_style_context().add_class(kind.value)
        self.set_margin_top(0); self.set_margin_bottom(8)
        self.set_margin_start(0); self.set_margin_end(0)

        inner = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
        inner.set_margin_top(12); inner.set_margin_bottom(12)
        inner.set_margin_start(16); inner.set_margin_end(16)
        t = Gtk.Label(label=title); t.set_xalign(0)
        t.get_style_context().add_class("cds-heading-01")
        inner.pack_start(t, False, False, 0)
        if body:
            b = Gtk.Label(label=body); b.set_xalign(0); b.set_line_wrap(True)
            b.get_style_context().add_class("cds-body-compact-01")
            inner.pack_start(b, False, False, 0)
        self.pack_start(inner, False, False, 0)

    @staticmethod
    def show(
        host: "ToastHost",
        title: str,
        *,
        body: str = "",
        kind: NotificationKind = NotificationKind.INFO,
        duration_ms: int = 6000,
    ) -> "Toast":
        toast = Toast(title, body=body, kind=kind)
        host.add_toast(toast, duration_ms=duration_ms)
        return toast


class ToastHost(Gtk.Box):
    """Top-right container that stacks active toasts.

    Embed this in the main workbench window. Toasts are appended at the
    top and auto-dismiss after duration_ms milliseconds.
    """

    def __init__(self) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        self.get_style_context().add_class("cds-toast-host")
        self.set_halign(Gtk.Align.END)
        self.set_valign(Gtk.Align.START)
        self.set_margin_top(16); self.set_margin_end(16)

    def add_toast(self, toast: Toast, *, duration_ms: int = 6000) -> None:
        self.pack_start(toast, False, False, 0)
        toast.show_all()

        def _dismiss() -> bool:
            if toast.get_parent() is self:
                self.remove(toast)
            return False

        if duration_ms > 0:
            GLib.timeout_add(duration_ms, _dismiss)
