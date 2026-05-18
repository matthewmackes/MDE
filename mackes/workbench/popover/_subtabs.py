"""Shared sub-tab strip used by Mesh / Tools / Manage tabs.

Each Q10-lock'd tab consolidates multiple existing panels behind a
horizontal sub-tab strip (glyph + short label). The strip is two
rows tall in 420px — enough for 4 sub-tabs across without ellipsis.
"""
from __future__ import annotations

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402


def build_subtab_container(items: list[tuple[str, str, str, str]]) -> Gtk.Widget:
    """items: list of (key, glyph, label, factory) where factory is
    "module:ClassName". Returns a widget with a sub-tab strip on top
    and a stack below. Panels build lazily on first activation."""
    outer = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)

    stack = Gtk.Stack()
    stack.set_transition_type(Gtk.StackTransitionType.CROSSFADE)
    stack.set_transition_duration(100)

    factories = {k: f for k, _, _, f in items}
    placeholders: dict[str, Gtk.Box] = {}
    built: set[str] = set()
    for key, _glyph, label, _factory in items:
        ph = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        placeholders[key] = ph
        stack.add_named(ph, key)

    bar = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=0,
                  homogeneous=True)
    bar.get_style_context().add_class("mackes-subtab-bar")
    buttons: dict[str, Gtk.ToggleButton] = {}

    def _on_toggle(btn, key):
        if btn.get_active() and stack.get_visible_child_name() != key:
            stack.set_visible_child_name(key)

    for key, glyph, label, _factory in items:
        btn = Gtk.ToggleButton()
        btn.set_relief(Gtk.ReliefStyle.NONE)
        inner = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=4)
        if glyph:
            g = Gtk.Label(label=glyph)
            g.get_style_context().add_class("mackes-subtab-glyph")
            inner.pack_start(g, False, False, 0)
        t = Gtk.Label(label=label)
        t.get_style_context().add_class("mackes-subtab-label")
        inner.pack_start(t, False, False, 0)
        btn.add(inner)
        btn.connect("toggled", _on_toggle, key)
        buttons[key] = btn
        bar.pack_start(btn, True, True, 0)

    def _on_visible(stk, _pspec):
        key = stk.get_visible_child_name()
        for k, b in buttons.items():
            want = (k == key)
            if b.get_active() != want:
                b.handler_block_by_func(_on_toggle)
                b.set_active(want)
                b.handler_unblock_by_func(_on_toggle)
        if key in factories and key not in built:
            built.add(key)
            try:
                import importlib
                mod_name, cls = factories[key].rsplit(":", 1)
                mod = importlib.import_module(mod_name)
                widget = getattr(mod, cls)()
            except Exception as e:  # noqa: BLE001
                widget = Gtk.Label(label=f"{key} failed: {e}")
                widget.set_line_wrap(True)
            placeholders[key].pack_start(widget, True, True, 0)
            placeholders[key].show_all()

    stack.connect("notify::visible-child", _on_visible)
    if items:
        stack.set_visible_child_name(items[0][0])

    outer.pack_start(bar,   False, False, 0)
    scroll = Gtk.ScrolledWindow()
    scroll.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
    scroll.add(stack)
    outer.pack_start(scroll, True, True, 0)
    return outer


__all__ = ["build_subtab_container"]
