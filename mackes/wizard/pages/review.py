"""Wizard screen 9 — Review (full diff of what will be applied)."""
from __future__ import annotations

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402

from mackes.workbench._common import section_header


def _render_section(title, dict_):
    lines = [f"  {k}: {v!r}" for k, v in (dict_ or {}).items()]
    return f"[{title}]\n" + ("\n".join(lines) if lines else "  (no changes)") + "\n"


def build(ctx) -> Gtk.Widget:
    box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
    box.set_margin_top(28); box.set_margin_bottom(28)
    box.set_margin_start(40); box.set_margin_end(40)

    title = Gtk.Label(label="Review")
    title.set_xalign(0); title.get_style_context().add_class("title-1")
    box.pack_start(title, False, False, 0)

    p = ctx.selected_preset
    summary = []
    summary.append(f"Preset: {p.display_name if p else '(none)'}")
    summary.append(f"Description: {p.description if p else ''}")
    summary.append("")
    summary.append(_render_section("appearance (preset)",  p.appearance if p else {}))
    summary.append(_render_section("devices (preset)",     p.devices if p else {}))
    summary.append(_render_section("system (preset)",      p.system if p else {}))
    summary.append(_render_section("network (preset)",     p.network if p else {}))
    summary.append("")
    summary.append("--- your overrides on top ---")
    for k, v in ctx.overrides.items():
        summary.append(_render_section(k, v))
    summary.append("")
    summary.append(f"QNM enabled: {ctx.enable_qnm}")
    summary.append(f"Firewall zone: {ctx.firewall_zone}")
    summary.append(f"VPN to import: {ctx.imported_vpn_path or '(none)'}")
    summary.append(f"Initial snapshot: {ctx.create_initial_snapshot} "
                   f"(label={ctx.snapshot_label!r})")
    summary.append("")
    if ctx.missing_packages:
        summary.append("WARNING: missing required binaries: " + ", ".join(ctx.missing_packages))
    summary.append("Click Apply to commit.")

    box.pack_start(section_header("What will happen"), False, False, 0)
    view = Gtk.TextView(); view.set_editable(False); view.set_monospace(True)
    view.get_buffer().set_text("\n".join(summary))
    scroll = Gtk.ScrolledWindow(); scroll.add(view); scroll.set_size_request(-1, 400)
    box.pack_start(scroll, True, True, 0)
    return box
