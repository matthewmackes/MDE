"""Wizard screen 1 — Welcome (Act I of the cb-welcome–style ritual).

A deliberate first moment. Logo, three sentences of voice, one CTA. Detail
about what the wizard will do is collapsed under a 'show details' disclosure
so the welcome page itself stays spare.
"""
from __future__ import annotations

from pathlib import Path

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import GdkPixbuf, Gtk  # noqa: E402

from mackes.workbench._common import title_label, info_label


def _hero_logo_path() -> Path | None:
    candidates = [
        Path("/usr/share/mackes-shell/branding/MACKES-XFCE-LOGO.png"),
        Path(__file__).resolve().parents[3] / "branding" / "MACKES-XFCE-LOGO.png",
    ]
    for p in candidates:
        if p.exists():
            return p
    return None


def build(ctx) -> Gtk.Widget:
    box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=18)
    box.set_margin_top(64); box.set_margin_bottom(48)
    box.set_margin_start(56); box.set_margin_end(56)

    # ---- Hero -----------------------------------------------------------
    logo_path = _hero_logo_path()
    if logo_path is not None:
        try:
            pixbuf = GdkPixbuf.Pixbuf.new_from_file_at_scale(
                str(logo_path), width=420, height=-1, preserve_aspect_ratio=True,
            )
            img = Gtk.Image.new_from_pixbuf(pixbuf)
            img.set_halign(Gtk.Align.CENTER)
            img.set_margin_bottom(12)
            box.pack_start(img, False, False, 0)
        except Exception:  # noqa: BLE001
            pass

    # ---- Voice — three sentences, no more ------------------------------
    title = title_label("Mackes Shell")
    title.set_halign(Gtk.Align.CENTER)
    box.pack_start(title, False, False, 0)

    voice = Gtk.Label()
    voice.set_markup(
        "<span size='large'>"
        "<b>A control panel for XFCE on Fedora.</b>\n\n"
        "Pick a preset. Watch it apply.\n"
        "Change anything later — nothing is locked."
        "</span>"
    )
    voice.set_halign(Gtk.Align.CENTER)
    voice.set_justify(Gtk.Justification.CENTER)
    voice.set_line_wrap(True)
    box.pack_start(voice, False, False, 12)

    # ---- Details disclosure --------------------------------------------
    disclosure = Gtk.Expander(label="What the wizard will do")
    disclosure.set_halign(Gtk.Align.CENTER)
    inner = info_label(
        "  •  Detect hardware and installed packages.\n"
        "  •  Apply your chosen preset (theme, shell, device defaults).\n"
        "  •  Install the preset's curated apps; remove XFCE components it replaces.\n"
        "  •  Create a restore point you can roll back to at any time.\n\n"
        "Cancel at any step — anything already applied stays applied."
    )
    inner.set_margin_top(12)
    disclosure.add(inner)
    box.pack_start(disclosure, False, False, 0)

    return box
