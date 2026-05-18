"""Gtk.Application — the single binary entry point.

Implements Q4 lock (single binary) + Q-HL1 lock (auto-detect headless +
explicit --gui/--headless overrides). The router runs *before* GTK
initialization so headless launches don't drag GTK into memory, and
module import works in pure-Python contexts (tests, CI, headless-only
images) where GTK isn't available.
"""
from __future__ import annotations

import os
import sys
from pathlib import Path
from typing import Optional

from mackes import __version__
from mackes.state import MackesState, ensure_dirs


APP_ID = "shell.mackes.Mackes"

_CSS_ROOTS = (
    Path("/usr/share/mackes-shell/data/css"),
    Path(__file__).resolve().parent.parent / "data" / "css",
)


def _resolve_css(*rel_parts: str) -> Optional[Path]:
    for root in _CSS_ROOTS:
        p = root.joinpath(*rel_parts)
        if p.is_file():
            return p
    return None


def _install_css(active_preset: Optional[str]) -> None:
    """Load tokens.css → mackes.css → per-preset accent into GTK."""
    import gi
    gi.require_version("Gtk", "3.0")
    from gi.repository import Gdk, GLib, Gtk

    def _load(path: Path, priority: int) -> None:
        provider = Gtk.CssProvider()
        try:
            provider.load_from_path(str(path))
        except GLib.Error:
            return
        screen = Gdk.Screen.get_default()
        if screen is None:
            return
        Gtk.StyleContext.add_provider_for_screen(screen, provider, priority)

    tokens = _resolve_css("tokens.css")
    if tokens is not None:
        _load(tokens, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)
    base = _resolve_css("mackes.css")
    if base is not None:
        _load(base, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION + 1)
    layout = _resolve_css("carbon-layout.css")
    if layout is not None:
        _load(layout, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION + 2)
    # v1.6.2 — Carbon Productive type scale + popover styles
    productive = _resolve_css("carbon-productive.css")
    if productive is not None:
        _load(productive, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION + 3)
    if active_preset:
        accent = _resolve_css("accents", f"{active_preset}.css")
        if accent is not None:
            _load(accent, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION + 4)


def _make_gui_app():
    """Build the Gtk.Application subclass dynamically.

    Defining the class inside this function means GTK is only imported
    when actually launching the GUI — headless installs (no GTK) can
    still `import mackes.app` for CLI dispatch.
    """
    import gi
    gi.require_version("Gtk", "3.0")
    from gi.repository import Gtk, Gio, GLib

    class MackesApp(Gtk.Application):  # type: ignore[misc]
        def __init__(self) -> None:
            super().__init__(
                application_id=APP_ID,
                flags=Gio.ApplicationFlags.HANDLES_COMMAND_LINE,
            )
            self.add_main_option(
                "wizard", ord("w"), GLib.OptionFlags.NONE, GLib.OptionArg.NONE,
                "Force the first-run wizard regardless of state", None,
            )
            self.add_main_option(
                "version", ord("V"), GLib.OptionFlags.NONE, GLib.OptionArg.NONE,
                "Print version and exit", None,
            )
            self.add_main_option(
                "popover", ord("p"), GLib.OptionFlags.NONE, GLib.OptionArg.NONE,
                "Open the 420×600 slide-out popover instead of the "
                "full window", None,
            )
            self._force_wizard = False
            self._popover_mode = False

        def do_command_line(self, command_line):  # type: ignore[override]
            opts = command_line.get_options_dict().end().unpack()
            if opts.get("version"):
                print(f"mackes {__version__}")
                return 0
            self._force_wizard = bool(opts.get("wizard"))
            self._popover_mode = bool(opts.get("popover"))
            self.activate()
            return 0

        def do_activate(self):  # type: ignore[override]
            ensure_dirs()
            state = MackesState.load()
            _install_css(state.active_preset)
            if self._force_wizard or not state.provisioned:
                # v1.4.0: play the bundled MP4 splash before the wizard
                # surfaces. If the splash can't open (missing video,
                # GStreamer not installed, etc.) we fall through to the
                # wizard immediately.
                def _open_wizard() -> None:
                    from mackes.wizard.window import WizardWindow
                    win = WizardWindow(application=self, state=state)
                    win.connect("destroy", lambda *_: self.quit())
                    win.show_all()

                try:
                    from mackes.wizard.splash import show_splash
                    if not show_splash(self, on_done=_open_wizard):
                        _open_wizard()
                except Exception:  # noqa: BLE001
                    _open_wizard()
            elif self._popover_mode:
                # v1.6.2 — slide-out popover (Q1 lock).
                from mackes.workbench.popover.window import PopoverWindow
                win = PopoverWindow(application=self)
                win.show_all()
                # Don't connect to destroy → quit; the popover may be
                # one of many windows the app owns (panel plugin etc.)
                return
            else:
                from mackes.workbench.shell.sidebar_window import WorkbenchWindow
                win = WorkbenchWindow(application=self, state=state)
                win.connect("destroy", lambda *_: self.quit())
                win.show_all()

    return MackesApp


def _is_headless_environment() -> bool:
    """Q-HL1 auto-detect: no display + no logind graphical session."""
    if os.environ.get("DISPLAY") or os.environ.get("WAYLAND_DISPLAY"):
        return False
    return True


def main(argv: list[str] | None = None) -> int:
    args = list(argv if argv is not None else sys.argv[1:])
    force_headless = "--headless" in args
    force_gui      = "--gui" in args
    args = [a for a in args if a not in ("--gui", "--headless")]

    has_subcmd = bool(args) and not args[0].startswith("-") and args[0] not in (
        "-V", "--version",
    )

    headless = force_headless or (has_subcmd and not force_gui) or (
        not force_gui and _is_headless_environment()
    )

    if headless:
        from mackes.headless.cli import main as headless_main
        return headless_main(args)

    # GUI path
    MackesApp = _make_gui_app()
    full_argv = [sys.argv[0]] + args
    return MackesApp().run(full_argv)
