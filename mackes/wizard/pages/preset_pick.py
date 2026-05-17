"""Wizard screen 3 — Preset Pick (Act II of the cb-welcome–style ritual).

CrunchBang-style card grid: each preset is a card with its wallpaper
thumbnail, display name (large), one-line voice, accent stripe. Click a
card to select. Selection is single-radio.
"""
from __future__ import annotations

from pathlib import Path
from typing import Optional

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import GdkPixbuf, Gtk  # noqa: E402

from mackes.presets import list_presets
from mackes.workbench._common import title_label


_WALL_CANDIDATES = (
    Path("/usr/share/mackes-shell/data/wallpapers"),
    Path(__file__).resolve().parents[3] / "data" / "wallpapers",
)


def _wallpaper_for(preset_name: str) -> Optional[Path]:
    for root in _WALL_CANDIDATES:
        p = root / f"{preset_name}.png"
        if p.is_file():
            return p
    return None


def _voice_for(preset) -> str:
    """Pull the *first* sentence of `description` so cards stay one-liner-short."""
    desc = (preset.description or "").strip().replace("\n", " ")
    for sep in (". ", " — ", " - "):
        if sep in desc:
            return desc.split(sep, 1)[0] + (sep.strip().rstrip(".-").strip()
                                            if sep.strip() in (".",) else "")
    return desc[:120]


def _build_card(preset, on_pick, group_radio) -> Gtk.Widget:
    """One preset card. Returns a clickable Frame; the radio inside owns the
    selection state."""
    frame = Gtk.Frame()
    frame.set_shadow_type(Gtk.ShadowType.NONE)
    frame.get_style_context().add_class("view")

    outer = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
    outer.set_margin_top(8); outer.set_margin_bottom(8)
    outer.set_margin_start(8); outer.set_margin_end(8)

    # Thumbnail
    wp = _wallpaper_for(preset.name)
    if wp is not None:
        try:
            pixbuf = GdkPixbuf.Pixbuf.new_from_file_at_scale(
                str(wp), width=280, height=160, preserve_aspect_ratio=False,
            )
            img = Gtk.Image.new_from_pixbuf(pixbuf)
            img.set_halign(Gtk.Align.CENTER)
            outer.pack_start(img, False, False, 0)
        except Exception:  # noqa: BLE001
            pass

    # Display name (large)
    name_lbl = Gtk.Label(label=preset.display_name)
    name_lbl.set_xalign(0); name_lbl.set_margin_top(10); name_lbl.set_margin_start(4)
    name_lbl.get_style_context().add_class("title-2")
    outer.pack_start(name_lbl, False, False, 0)

    # Voice — one line
    voice = Gtk.Label(label=_voice_for(preset) + ".")
    voice.set_xalign(0); voice.set_line_wrap(True)
    voice.set_margin_start(4); voice.set_margin_end(4); voice.set_margin_bottom(8)
    voice.get_style_context().add_class("dim-label")
    outer.pack_start(voice, False, False, 0)

    # Radio (the actual selection control, label hidden)
    radio = Gtk.RadioButton.new_from_widget(group_radio)
    radio.set_label("")
    radio.set_margin_start(4); radio.set_margin_bottom(4)
    radio.connect("toggled", lambda b: on_pick(preset) if b.get_active() else None)
    outer.pack_start(radio, False, False, 0)

    # Clicking anywhere on the card toggles the radio
    ev = Gtk.EventBox()
    ev.add(outer)
    ev.connect("button-press-event", lambda *_: (radio.set_active(True), False)[1])
    frame.add(ev)
    return frame, radio


def build(ctx, on_pick=None) -> Gtk.Widget:
    box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=20)
    box.set_margin_top(48); box.set_margin_bottom(32)
    box.set_margin_start(56); box.set_margin_end(56)

    title = title_label("Pick a preset")
    title.set_halign(Gtk.Align.CENTER)
    box.pack_start(title, False, False, 0)

    sub = Gtk.Label()
    sub.set_markup("<span size='large'>Four moods. Click one. You can change later.</span>")
    sub.set_halign(Gtk.Align.CENTER)
    box.pack_start(sub, False, False, 0)

    # Hide the headless 'node' preset from the GUI picker — it's auto-
    # selected by `mackes init` when no display is present.
    presets = [p for p in list_presets() if p.name != "node"]
    if not presets:
        err = Gtk.Label(label="No presets found in data/presets/.")
        err.get_style_context().add_class("error")
        box.pack_start(err, False, False, 0)
        return box

    if ctx.selected_preset is None or ctx.selected_preset.name == "node":
        ctx.selected_preset = presets[0]

    def _set(preset):
        ctx.selected_preset = preset
        if on_pick:
            on_pick(preset)

    # 2-column grid
    grid = Gtk.FlowBox()
    grid.set_valign(Gtk.Align.START)
    grid.set_max_children_per_line(2)
    grid.set_min_children_per_line(2)
    grid.set_homogeneous(True)
    grid.set_column_spacing(16)
    grid.set_row_spacing(16)
    grid.set_selection_mode(Gtk.SelectionMode.NONE)

    group_radio: Optional[Gtk.RadioButton] = None
    radios: list[tuple] = []
    for p in presets:
        card, radio = _build_card(p, _set, group_radio)
        if group_radio is None:
            group_radio = radio
        radio.set_active(ctx.selected_preset.name == p.name)
        radios.append((p, radio))
        grid.add(card)

    box.pack_start(grid, True, True, 0)
    return box
