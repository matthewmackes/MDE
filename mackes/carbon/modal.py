"""Carbon Modal — three sizes for confirmations and forms."""
from __future__ import annotations

import enum
from typing import Callable, Optional

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402

from mackes.carbon.button import Button, ButtonKind


class ModalSize(enum.Enum):
    SMALL  = (400, 200)
    MEDIUM = (640, 360)
    LARGE = (840, 520)


class Modal(Gtk.Dialog):
    """Carbon-styled modal dialog.

    Usage:
        modal = Modal(parent, "Title", body, size=ModalSize.MEDIUM)
        modal.add_action("Cancel", kind=ButtonKind.SECONDARY)
        modal.add_action("Delete", kind=ButtonKind.DANGER, on_click=do_delete)
        modal.run_then_destroy()
    """

    def __init__(
        self,
        parent: Optional[Gtk.Window],
        title: str,
        body: Gtk.Widget,
        *,
        size: ModalSize = ModalSize.MEDIUM,
    ) -> None:
        super().__init__(title=title, transient_for=parent, modal=True)
        self.get_style_context().add_class("cds-modal")
        w, h = size.value
        self.set_default_size(w, h)

        content = self.get_content_area()
        content.set_margin_top(16); content.set_margin_bottom(16)
        content.set_margin_start(24); content.set_margin_end(24)
        content.set_spacing(16)

        title_lbl = Gtk.Label(label=title); title_lbl.set_xalign(0)
        title_lbl.get_style_context().add_class("cds-heading-03")
        content.pack_start(title_lbl, False, False, 0)
        content.pack_start(body, True, True, 0)

        self._action_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        self._action_box.set_halign(Gtk.Align.END)
        action_area = self.get_action_area()
        if isinstance(action_area, Gtk.Box):
            action_area.set_spacing(8)

    def add_action(
        self,
        label: str,
        *,
        kind: ButtonKind = ButtonKind.SECONDARY,
        on_click: Optional[Callable[[], None]] = None,
        response_id: int = Gtk.ResponseType.OK,
    ) -> Button:
        btn = Button(label, kind=kind)
        # Routing: clicking the button triggers both on_click (if any) and
        # dialog response so .run() returns.
        def _on_btn_clicked(*_a):
            if on_click is not None:
                on_click()
            self.response(response_id)
        btn.connect("clicked", _on_btn_clicked)
        self.get_action_area().pack_end(btn, False, False, 0)
        btn.show()
        return btn

    def run_then_destroy(self) -> int:
        rc = self.run()
        self.destroy()
        return rc
