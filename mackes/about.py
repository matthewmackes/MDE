"""About MDE — a small scrollable window over the bundled ABOUT.txt.

Opened from the apple menu's "About" item (which runs
`mackes --about`) and from the Workbench's footer when present.

v2.0.0 Phase 0.11 — user-visible strings switched to "Mackes Desktop
Environment (MDE)" on first reference, "MDE" thereafter.
"""
from __future__ import annotations

from pathlib import Path
from typing import Optional

import os

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import GdkPixbuf, Gtk  # noqa: E402

from mackes import __version__


# Brand-asset resolution mirrors the Rust `mde_theme::Brand` loader's
# probe order so the About window shows the same wordmark the shell
# does: $MDE_BRAND_DIR override → system install → source tree. SVG is
# preferred over PNG (scales cleanly); the first existing file wins.
_BRAND_DIRS = [
    os.environ.get("MDE_BRAND_DIR"),
    "/usr/share/mde/brand",
    str(Path(__file__).resolve().parents[1] / "assets" / "brand"),
]


def _resolve_brand_asset(basename: str) -> Optional[str]:
    """First-hit path for a brand slot, probing svg then png across the
    override / system / source-tree layers. Returns None if absent."""
    for directory in _BRAND_DIRS:
        if not directory:
            continue
        for ext in ("svg", "png"):
            candidate = Path(directory) / f"{basename}.{ext}"
            if candidate.is_file():
                return str(candidate)
    return None


def _brand_wordmark(height: int = 56) -> Optional[Gtk.Image]:
    """Load the MDE wordmark as a height-scaled Gtk.Image, or None if no
    asset resolves (caller falls back to a text wordmark)."""
    path = _resolve_brand_asset("wordmark")
    if path is None:
        return None
    try:
        # Scale by height, preserving aspect (width = -1). librsvg's
        # GdkPixbuf loader handles the SVG; PNGs scale the same way.
        pixbuf = GdkPixbuf.Pixbuf.new_from_file_at_scale(path, -1, height, True)
    except Exception:  # noqa: BLE001
        return None
    image = Gtk.Image.new_from_pixbuf(pixbuf)
    image.set_halign(Gtk.Align.START)
    return image


# Where the credits file lives once the RPM is installed. We also fall
# back to the source-tree copy so `python -m mackes --about` works in a
# checked-out repo without installing. Phase 0.8 will swap the
# `/usr/share/mackes-shell/` prefix to `/usr/share/mde/`; we probe
# both so the About window works during the transition.
_INSTALLED_PATHS = [
    Path("/usr/share/mde/ABOUT.txt"),
    Path("/usr/share/mackes-shell/ABOUT.txt"),
]
_REPO_PATH = Path(__file__).resolve().parents[1] / "data" / "ABOUT.txt"


def _resolve_about_text() -> str:
    for path in [*_INSTALLED_PATHS, _REPO_PATH]:
        try:
            return path.read_text(encoding="utf-8")
        except OSError:
            continue
    return (
        "Mackes Desktop Environment (MDE)\n"
        "=================================\n\n"
        "ABOUT.txt could not be located. The RPM ships it at\n"
        "/usr/share/mde/ABOUT.txt (or, for v1.x boxes,\n"
        "/usr/share/mackes-shell/ABOUT.txt) — if you see this\n"
        "message, the package install is incomplete.\n"
    )


def build_about_window(application: Optional[Gtk.Application] = None) -> Gtk.Window:
    """Construct the About MDE window. Caller is responsible for
    show_all() and (typically) connecting "destroy" to quit the app."""
    win = Gtk.ApplicationWindow(application=application) if application \
        else Gtk.Window()
    win.set_title(f"About MDE — v{__version__}")
    win.set_default_size(640, 600)
    # Gtk.WindowPosition.CENTER picks the wrong head on multi-monitor
    # setups; compute the center of the *primary* monitor explicitly.
    try:
        from gi.repository import Gdk
        display = Gdk.Display.get_default()
        mon = display.get_primary_monitor() or display.get_monitor(0) if display else None
        if mon is not None:
            g = mon.get_geometry()
            win.move(
                g.x + max(0, (g.width  - 640) // 2),
                g.y + max(0, (g.height - 600) // 2),
            )
    except Exception:  # noqa: BLE001
        win.set_position(Gtk.WindowPosition.CENTER)
    win.get_style_context().add_class("mackes-app-window")
    win.get_style_context().add_class("mackes-about")
    # Phase 11.2 a11y: Escape dismisses the About window.
    from mackes.gtk_common import close_on_escape
    close_on_escape(win)

    outer = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)

    # Header — brand block. Mirrors the Workbench header's visual.
    header = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
    header.set_margin_top(24); header.set_margin_bottom(8)
    header.set_margin_start(24); header.set_margin_end(24)
    # Prefer the resolved brand wordmark (mesh-penguin + "MDE / Mackes
    # Desktop Environment"); fall back to a text wordmark if no asset
    # resolves on this box.
    brand = _brand_wordmark()
    if brand is None:
        brand = Gtk.Label()
        brand.set_markup(
            '<span size="x-large" weight="bold">MDE</span> '
            '<span size="x-large" foreground="#a8a8a8">'
            'Mackes Desktop Environment</span>'
        )
        brand.set_xalign(0)
    header.pack_start(brand, False, False, 0)

    sub = Gtk.Label()
    sub.set_markup(
        f'<span foreground="#a8a8a8">'
        f'v{__version__} · Private project · '
        f'Buffalo, NY'
        f'</span>'
    )
    sub.set_xalign(0)
    header.pack_start(sub, False, False, 0)
    outer.pack_start(header, False, False, 0)

    sep = Gtk.Separator(orientation=Gtk.Orientation.HORIZONTAL)
    outer.pack_start(sep, False, False, 0)

    # Body — scrollable monospaced credits.
    scroller = Gtk.ScrolledWindow()
    scroller.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
    scroller.set_hexpand(True); scroller.set_vexpand(True)
    text = Gtk.TextView()
    text.set_editable(False)
    text.set_cursor_visible(False)
    text.set_wrap_mode(Gtk.WrapMode.WORD_CHAR)
    text.set_left_margin(24); text.set_right_margin(24)
    text.set_top_margin(16); text.set_bottom_margin(16)
    text.set_monospace(True)
    text.get_buffer().set_text(_resolve_about_text())
    scroller.add(text)
    outer.pack_start(scroller, True, True, 0)

    # Footer — close button + contact.
    sep2 = Gtk.Separator(orientation=Gtk.Orientation.HORIZONTAL)
    outer.pack_start(sep2, False, False, 0)

    footer = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
    footer.set_margin_top(12); footer.set_margin_bottom(12)
    footer.set_margin_start(24); footer.set_margin_end(24)
    contact = Gtk.Label()
    contact.set_markup(
        '<span foreground="#a8a8a8">'
        'matthewmackes@outlook.com'
        '</span>'
    )
    contact.set_xalign(0)
    footer.pack_start(contact, True, True, 0)
    close = Gtk.Button(label="Close")
    close.connect("clicked", lambda *_: win.close())
    close.set_tooltip_text("Close the About Mackes window (Esc)")
    _ax = close.get_accessible()
    if _ax is not None:
        _ax.set_name("Close the About Mackes window")
    footer.pack_end(close, False, False, 0)
    outer.pack_start(footer, False, False, 0)

    win.add(outer)
    return win
