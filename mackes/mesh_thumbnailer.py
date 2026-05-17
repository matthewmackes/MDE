"""§8.10 Q-MX19 Tumbler thumbnailer for mesh clipboard + notification files.

Tumbler (Thunar's thumbnail daemon) reads a small .thumbnailer file that
specifies the MIME types it handles and a command line to run for each.
We ship `mackes-mesh.thumbnailer` in /usr/share/thumbnailers/ pointing at
this module's main() entry, which dispatches on the input file's MIME.

For .md notification files, we render a Carbon-styled card preview by
generating a PNG via Pango. For raw clipboard items we fall back to the
file's existing thumbnailer (image-magick / xdg-thumbnailers).
"""
from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path


def _render_notification_thumbnail(src: Path, dst: Path, size: int) -> int:
    """Render a notification .md as a Carbon-styled card PNG."""
    try:
        import gi
        gi.require_version("Pango", "1.0")
        gi.require_version("PangoCairo", "1.0")
        from gi.repository import Pango, PangoCairo  # type: ignore
        import cairo  # type: ignore
    except (ImportError, ValueError):
        return 1

    try:
        text = src.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return 1

    # Strip frontmatter
    meta = {}
    body = text
    if text.startswith("---\n"):
        end = text.find("\n---\n", 4)
        if end != -1:
            for ln in text[4:end].splitlines():
                if ":" in ln:
                    k, v = ln.split(":", 1)
                    meta[k.strip()] = v.strip()
            body = text[end + 5:]

    title = body.split("\n", 1)[0].lstrip("# ").strip() or "Mesh notification"
    rest = body.split("\n", 1)[1].strip() if "\n" in body else ""
    peer = meta.get("peer", "")

    # Dimensions — fixed aspect (3:2) scaled to size
    w = size
    h = max(64, size * 2 // 3)
    surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, w, h)
    ctx = cairo.Context(surface)
    # Carbon Gray 100 background
    ctx.set_source_rgb(0x16 / 255, 0x16 / 255, 0x16 / 255)
    ctx.rectangle(0, 0, w, h)
    ctx.fill()
    # Accent left border
    ctx.set_source_rgb(0xfa / 255, 0x4d / 255, 0x56 / 255)
    ctx.rectangle(0, 0, 4, h)
    ctx.fill()
    # Title
    ctx.set_source_rgb(0xf4 / 255, 0xf4 / 255, 0xf4 / 255)
    layout = PangoCairo.create_layout(ctx)
    layout.set_font_description(Pango.FontDescription("IBM Plex Sans Bold 11"))
    layout.set_text(title, -1)
    layout.set_width((w - 24) * Pango.SCALE)
    layout.set_ellipsize(Pango.EllipsizeMode.END)
    ctx.move_to(16, 12)
    PangoCairo.show_layout(ctx, layout)
    # Body (truncated)
    layout2 = PangoCairo.create_layout(ctx)
    layout2.set_font_description(Pango.FontDescription("IBM Plex Sans 9"))
    layout2.set_text(rest[:300], -1)
    layout2.set_width((w - 24) * Pango.SCALE)
    layout2.set_height((h - 60) * Pango.SCALE)
    layout2.set_ellipsize(Pango.EllipsizeMode.END)
    ctx.set_source_rgb(0xc6 / 255, 0xc6 / 255, 0xc6 / 255)
    ctx.move_to(16, 38)
    PangoCairo.show_layout(ctx, layout2)
    # Footer (peer)
    layout3 = PangoCairo.create_layout(ctx)
    layout3.set_font_description(Pango.FontDescription("IBM Plex Sans 8"))
    layout3.set_text(f"from {peer}" if peer else "", -1)
    ctx.set_source_rgb(0x96 / 255, 0x96 / 255, 0x96 / 255)
    ctx.move_to(16, h - 24)
    PangoCairo.show_layout(ctx, layout3)

    surface.write_to_png(str(dst))
    return 0


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(prog="mackes-mesh-thumbnailer")
    p.add_argument("--size", "-s", type=int, default=256)
    p.add_argument("input")
    p.add_argument("output")
    args = p.parse_args(argv)
    src = Path(args.input)
    dst = Path(args.output)
    if not src.exists():
        return 1
    if src.suffix.lower() == ".md":
        return _render_notification_thumbnail(src, dst, args.size)
    # For everything else, defer to whatever thumbnailer normally handles it
    # (we don't claim those MIMEs in our .thumbnailer file).
    return 1


if __name__ == "__main__":
    sys.exit(main())
