"""Maintain → Dependencies.

Lists required and recommended Fedora packages, indicates which are
installed, and offers a one-click install for missing ones via
`pkexec dnf install`. Optional packages (themes, fonts) are listed too —
checked but never auto-installed; the user has to opt in.
"""
from __future__ import annotations

import shutil
import subprocess

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk  # noqa: E402

from mackes.logging import log_action
from mackes.workbench._common import (
    info_label, panel_box, section_header, title_label,
)


# (package_name, friendly_label, required?)
PACKAGES: list[tuple[str, str, bool]] = [
    # Required runtime
    ("xfconf",                  "xfconf-query — XFCE settings DB", True),
    ("xfce4-settings",          "xfce4-settings — stays installed, hidden", True),
    ("python3-gobject",         "PyGObject — GTK3 bindings",        True),
    ("gtk3",                    "GTK 3 runtime",                    True),
    ("python3-pyyaml",          "PyYAML — preset loader",           True),
    # XFCE shell pieces baked into the standard layout (Q19 lock)
    ("xfce4-whiskermenu-plugin",   "Whisker Menu — panel start menu",   True),
    ("xfce4-docklike-plugin",      "Docklike Taskbar — replaces Window Buttons", True),
    ("xfce4-pulseaudio-plugin",    "Volume applet — panel",             True),
    ("xfce4-power-manager-plugin", "Power applet — panel",              True),
    # Network
    ("NetworkManager",          "NetworkManager",                   True),
    ("openssh-server",          "OpenSSH server — enabled by default", True),
    ("firewalld",               "firewalld",                        False),
    # Audio
    ("pulseaudio-utils",        "pactl — Sound panel backend",      False),
    # Typography defaults — Carbon Design System
    ("ibm-plex-sans-fonts",     "IBM Plex Sans (UI)",               False),
    ("ibm-plex-mono-fonts",     "IBM Plex Mono (monospace)",        False),
]


def _is_installed(pkg: str) -> bool:
    if shutil.which("rpm") is None:
        return False
    try:
        subprocess.check_output(["rpm", "-q", pkg], stderr=subprocess.DEVNULL)
        return True
    except subprocess.CalledProcessError:
        return False


def _install(packages: list[str]) -> tuple[int, str]:
    """Install via pkexec dnf. Returns (returncode, combined-output)."""
    if not packages:
        return 0, ""
    cmd = ["pkexec", "dnf", "install", "-y", *packages]
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
        return result.returncode, (result.stdout + result.stderr).strip()
    except FileNotFoundError:
        return 127, "pkexec or dnf not found"
    except subprocess.TimeoutExpired:
        return 124, "dnf timed out after 10 minutes"


class DependenciesPanel(Gtk.Box):
    def __init__(self) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self._build()

    def _build(self) -> None:
        box = panel_box()
        box.pack_start(title_label("Dependencies"), False, False, 0)
        box.pack_start(info_label(
            "Required and recommended Fedora packages for Mackes Shell. "
            "Missing required packages prevent panels from working; missing "
            "recommended ones disable specific features (theme/font defaults, "
            "firewall, audio device picker)."
        ), False, False, 0)

        bar = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        refresh = Gtk.Button(label="Refresh")
        refresh.connect("clicked", lambda *_: self._refresh())
        self._install_btn = Gtk.Button(label="Install missing required")
        self._install_btn.get_style_context().add_class("suggested-action")
        self._install_btn.connect("clicked", lambda *_: self._install(required_only=True))
        self._install_all_btn = Gtk.Button(label="Install all missing")
        self._install_all_btn.connect("clicked", lambda *_: self._install(required_only=False))
        bar.pack_start(refresh, False, False, 0)
        bar.pack_start(self._install_btn, False, False, 0)
        bar.pack_start(self._install_all_btn, False, False, 0)
        box.pack_start(bar, False, False, 0)

        self._status = Gtk.Label(label=""); self._status.set_xalign(0)
        self._status.get_style_context().add_class("dim-label")
        box.pack_start(self._status, False, False, 0)

        box.pack_start(section_header("Required"), False, False, 0)
        self._required = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        box.pack_start(self._required, False, False, 0)

        box.pack_start(section_header("Recommended / Optional"), False, False, 0)
        self._optional = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        box.pack_start(self._optional, False, False, 0)

        self.add(box)
        self._refresh()

    def _refresh(self) -> bool:
        for box in (self._required, self._optional):
            for child in list(box.get_children()):
                box.remove(child)
        for pkg, label, required in PACKAGES:
            installed = _is_installed(pkg)
            row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
            dot = Gtk.Label(label="●" if installed else "○")
            dot.get_style_context().add_class("success" if installed else
                                              ("error" if required else "dim-label"))
            row.pack_start(dot, False, False, 0)
            lbl = Gtk.Label(label=f"{pkg}    {label}")
            lbl.set_xalign(0); lbl.set_line_wrap(True)
            row.pack_start(lbl, True, True, 0)
            (self._required if required else self._optional).pack_start(row, False, False, 0)
        self._required.show_all(); self._optional.show_all()
        return False

    def _install(self, *, required_only: bool) -> None:
        targets = [p for p, _, req in PACKAGES if (req or not required_only) and not _is_installed(p)]
        if not targets:
            self._status.set_text("Nothing to install.")
            return
        self._status.set_text(f"Running: pkexec dnf install {' '.join(targets)} (this may prompt)…")
        # Run synchronously — dnf can take a while, but we want predictable UX
        # and the panel already shows a status line.
        while Gtk.events_pending():
            Gtk.main_iteration_do(False)
        rc, output = _install(targets)
        log_action(f"deps: install rc={rc} pkgs={','.join(targets)}")
        head = "OK" if rc == 0 else f"FAILED (rc={rc})"
        self._status.set_text(f"{head}. {output.splitlines()[-1] if output else ''}")
        self._refresh()
