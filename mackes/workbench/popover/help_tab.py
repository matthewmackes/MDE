"""Help tab — quick links + about + version."""
from __future__ import annotations

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402


class HelpTab(Gtk.Box):
    def __init__(self) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        self.set_margin_top(12); self.set_margin_bottom(12)
        self.set_margin_start(12); self.set_margin_end(12)

        title = Gtk.Label(label="Mackes Shell")
        title.set_xalign(0)
        title.get_style_context().add_class("mackes-popover-title")
        self.pack_start(title, False, False, 0)

        try:
            from mackes import __version__
            v = __version__
        except Exception:  # noqa: BLE001
            v = "?"
        version = Gtk.Label(label=f"Version {v}")
        version.set_xalign(0)
        version.get_style_context().add_class("mackes-glance-meta")
        self.pack_start(version, False, False, 0)

        # Quick links
        for label, target in [
            (" Open full window",  "_open_full_window"),
            ("  Re-run setup wizard", "_open_wizard"),
            (" View logs",          "_open_logs"),
            (" Help docs",         "_open_help"),
        ]:
            btn = Gtk.Button(label=label)
            btn.set_relief(Gtk.ReliefStyle.NONE)
            btn.set_halign(Gtk.Align.START)
            btn.get_style_context().add_class("mackes-help-link")
            btn.connect("clicked",
                        lambda _b, t=target: getattr(self, t)())
            self.pack_start(btn, False, False, 0)

        spacer = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        self.pack_start(spacer, True, True, 0)

        footer = Gtk.Label(label=
            "Mackes Shell is the XFCE control panel + mesh fabric for "
            "small fleets. Hit Super+M anywhere to open this popover."
        )
        footer.set_xalign(0); footer.set_line_wrap(True)
        footer.get_style_context().add_class("mackes-glance-meta")
        self.pack_start(footer, False, False, 0)

    # ---- Actions -------------------------------------------------------

    def _open_full_window(self) -> None:
        try:
            from mackes.workbench.shell.sidebar_window import WorkbenchWindow
            from mackes.state import MackesState
            top = self.get_toplevel()
            app = top.get_application() if top else None
            w = WorkbenchWindow(application=app, state=MackesState.load())
            w.show_all()
            if top is not None:
                top.close()
        except Exception:  # noqa: BLE001
            pass

    def _open_wizard(self) -> None:
        try:
            from mackes.wizard.window import WizardWindow
            from mackes.state import MackesState
            top = self.get_toplevel()
            app = top.get_application() if top else None
            w = WizardWindow(application=app, state=MackesState.load())
            w.show_all()
            if top is not None:
                top.close()
        except Exception:  # noqa: BLE001
            pass

    def _open_logs(self) -> None:
        import subprocess
        try:
            subprocess.Popen(
                ["xdg-open",
                 str((__import__("pathlib").Path.home()
                      / ".local/share/mackes-shell/logs"))],
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            )
        except OSError:
            pass

    def _open_help(self) -> None:
        import subprocess
        try:
            subprocess.Popen(["xdg-open", "/usr/share/mackes-shell/help/"],
                             stdout=subprocess.DEVNULL,
                             stderr=subprocess.DEVNULL)
        except OSError:
            pass


__all__ = ["HelpTab"]
