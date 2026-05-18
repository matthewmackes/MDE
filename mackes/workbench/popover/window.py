"""Popover Window — 420×600 slide-out launched by the panel button.

Behavior:
  * Gtk.Window type=POPUP, fixed 420×600
  * Anchored to the top-right of the screen (panel-button-adjacent)
  * Dismiss on focus-out (Q4 lock)
  * 5 tabs in a horizontal top bar (Q2 / Q3 lock)
  * Nerd Font glyphs on each tab (Q10 lock)
  * Carbon Productive type scale via .mackes-productive class (Q6 lock)
"""
from __future__ import annotations

import gi
gi.require_version("Gtk", "3.0")
gi.require_version("Gdk", "3.0")
from gi.repository import Gdk, Gtk  # noqa: E402

from mackes.workbench._common import versioned_title


POPOVER_WIDTH  = 420
POPOVER_HEIGHT = 600


# Q3 lock: five tabs. Each (key, Nerd-Font-glyph, label, factory).
TABS = (
    ("glance", "",  "Glance",  "mackes.workbench.popover.glance:GlanceTab"),
    ("mesh",   "",  "Mesh",    "mackes.workbench.popover.mesh_tab:MeshTab"),
    ("tools",  "",  "Tools",   "mackes.workbench.popover.tools_tab:ToolsTab"),
    ("manage", "",  "Manage",  "mackes.workbench.popover.manage_tab:ManageTab"),
    ("help",   "",  "Help",    "mackes.workbench.popover.help_tab:HelpTab"),
)


class PopoverWindow(Gtk.Window):
    def __init__(self, *, application=None, anchor: str = "top-right",
                 initial_tab: str = "glance") -> None:
        super().__init__(type=Gtk.WindowType.POPUP)
        if application is not None:
            application.add_window(self)
        self.set_title(versioned_title("Mackes Shell"))
        self.set_default_size(POPOVER_WIDTH, POPOVER_HEIGHT)
        self.set_resizable(False)
        self.set_decorated(False)
        self.set_keep_above(True)
        self.set_skip_taskbar_hint(True)
        self.set_skip_pager_hint(True)
        self.get_style_context().add_class("mackes-popover")
        self.get_style_context().add_class("mackes-productive")

        self._anchor_position(anchor)

        # ESC closes; focus-out closes (Q4)
        self.connect("focus-out-event", lambda *_: self._on_focus_out())
        self.connect("key-press-event", self._on_key)

        # Build shell
        shell = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        shell.pack_start(self._build_header(), False, False, 0)
        shell.pack_start(self._build_tabbar(),  False, False, 0)
        self._stack = Gtk.Stack()
        self._stack.set_transition_type(Gtk.StackTransitionType.CROSSFADE)
        self._stack.set_transition_duration(120)
        shell.pack_start(self._stack, True, True, 0)
        self.add(shell)

        # Lazy-build tab content on first navigation
        self._tab_factories: dict[str, str] = {k: f for k, _, _, f in TABS}
        self._tab_built: set[str] = set()
        for key, _glyph, label, _factory in TABS:
            placeholder = Gtk.Box(orientation=Gtk.Orientation.VERTICAL,
                                  spacing=0)
            self._stack.add_named(placeholder, key)

        self._stack.connect("notify::visible-child", self._on_tab_changed)
        self.set_visible_tab(initial_tab)

    # ---- Layout ----------------------------------------------------------

    def _build_header(self) -> Gtk.Widget:
        bar = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        bar.set_margin_top(8); bar.set_margin_bottom(4)
        bar.set_margin_start(12); bar.set_margin_end(8)
        bar.get_style_context().add_class("mackes-popover-header")

        # Logo + version
        title = Gtk.Label(label="Mackes")
        title.get_style_context().add_class("mackes-popover-title")
        bar.pack_start(title, False, False, 0)

        version_lab = Gtk.Label(label="")
        try:
            from mackes import __version__
            version_lab.set_text(__version__)
        except Exception:  # noqa: BLE001
            pass
        version_lab.get_style_context().add_class("mackes-popover-version")
        bar.pack_start(version_lab, False, False, 0)

        bar.pack_start(Gtk.Box(), True, True, 0)   # spacer

        # Open-in-window button (escape to full size)
        open_btn = Gtk.Button(label="")
        open_btn.get_style_context().add_class("mackes-popover-iconbtn")
        open_btn.set_tooltip_text("Open in full window")
        open_btn.connect("clicked", lambda *_: self._open_full_window())
        bar.pack_start(open_btn, False, False, 0)

        close_btn = Gtk.Button(label="")
        close_btn.get_style_context().add_class("mackes-popover-iconbtn")
        close_btn.set_tooltip_text("Close")
        close_btn.connect("clicked", lambda *_: self.close())
        bar.pack_start(close_btn, False, False, 0)
        return bar

    def _build_tabbar(self) -> Gtk.Widget:
        # Five tab buttons in a horizontal box. Each is a toggle button
        # but we manage activation manually so only one is ever active.
        bar = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=0,
                      homogeneous=True)
        bar.get_style_context().add_class("mackes-popover-tabbar")
        self._tab_buttons: dict[str, Gtk.ToggleButton] = {}
        for key, glyph, label, _ in TABS:
            btn = Gtk.ToggleButton()
            btn.set_relief(Gtk.ReliefStyle.NONE)
            inner = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
            g = Gtk.Label(label=glyph)
            g.get_style_context().add_class("mackes-popover-tabglyph")
            inner.pack_start(g, False, False, 0)
            t = Gtk.Label(label=label)
            t.get_style_context().add_class("mackes-popover-tablabel")
            inner.pack_start(t, False, False, 0)
            btn.add(inner)
            btn.connect("toggled", self._on_tab_button_toggled, key)
            self._tab_buttons[key] = btn
            bar.pack_start(btn, True, True, 0)
        return bar

    # ---- Tab activation --------------------------------------------------

    def set_visible_tab(self, key: str) -> None:
        if key not in self._tab_factories:
            return
        self._stack.set_visible_child_name(key)

    def _on_tab_button_toggled(self, btn: Gtk.ToggleButton, key: str) -> None:
        # Only react on activation; deactivation is driven by the
        # cross-fade logic below.
        if btn.get_active() and self._stack.get_visible_child_name() != key:
            self._stack.set_visible_child_name(key)

    def _on_tab_changed(self, stk, _pspec) -> None:
        key = stk.get_visible_child_name()
        # Sync button states
        for k, b in self._tab_buttons.items():
            want = (k == key)
            if b.get_active() != want:
                b.handler_block_by_func(self._on_tab_button_toggled)
                b.set_active(want)
                b.handler_unblock_by_func(self._on_tab_button_toggled)
        # Lazy-build the tab body
        if key in self._tab_factories and key not in self._tab_built:
            self._build_tab_body(key)

    def _build_tab_body(self, key: str) -> None:
        factory = self._tab_factories[key]
        mod, cls = factory.rsplit(":", 1)
        try:
            import importlib
            m = importlib.import_module(mod)
            widget = getattr(m, cls)()
        except Exception as e:  # noqa: BLE001
            widget = Gtk.Label(label=f"Tab {key} failed to load: {e}")
            widget.set_line_wrap(True)
        placeholder = self._stack.get_child_by_name(key)
        if placeholder is not None and isinstance(placeholder, Gtk.Box):
            placeholder.pack_start(widget, True, True, 0)
            placeholder.show_all()
        self._tab_built.add(key)

    # ---- Anchor + behavior ----------------------------------------------

    def _anchor_position(self, anchor: str) -> None:
        screen = self.get_screen()
        if screen is None:
            return
        display = Gdk.Display.get_default()
        monitor = display.get_primary_monitor() if display else None
        geo = monitor.get_geometry() if monitor else None
        if geo is None:
            return
        margin = 12
        # Default: top-right (panel-button-adjacent)
        x = geo.x + geo.width - POPOVER_WIDTH - margin
        y = geo.y + margin + 32   # below the xfce4-panel
        if anchor == "bottom-right":
            y = geo.y + geo.height - POPOVER_HEIGHT - margin - 32
        elif anchor == "top-left":
            x = geo.x + margin
        elif anchor == "bottom-left":
            x = geo.x + margin
            y = geo.y + geo.height - POPOVER_HEIGHT - margin - 32
        self.move(x, y)

    def _on_focus_out(self) -> bool:
        # Q4 lock: close on focus-out. But only if the popover already
        # was visible — otherwise we'd close immediately on first paint.
        if self.get_visible():
            self.close()
        return False

    def _on_key(self, _w, event) -> bool:
        if event.keyval == Gdk.KEY_Escape:
            self.close()
            return True
        return False

    def _open_full_window(self) -> None:
        """Q9 lock — escape to the full-window shell."""
        try:
            from mackes.workbench.shell.sidebar_window import WorkbenchWindow
            from mackes.state import MackesState
            app = self.get_application()
            w = WorkbenchWindow(application=app, state=MackesState.load())
            w.show_all()
            self.close()
        except Exception:  # noqa: BLE001
            pass


__all__ = ["PopoverWindow", "POPOVER_WIDTH", "POPOVER_HEIGHT"]
