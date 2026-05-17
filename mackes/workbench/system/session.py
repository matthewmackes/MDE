"""System → Session & Startup.

Two responsibilities:
  - xfce4-session save-on-exit / logout prompt behavior (xfconf channel
    `xfce4-session`).
  - Autostart `.desktop` files under ~/.config/autostart/ — list, toggle the
    Hidden field, add a new entry by picking an executable.
"""
from __future__ import annotations

import os
from pathlib import Path

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk  # noqa: E402

from mackes.logging import log_action
from mackes.state import HOME
from mackes.xfconf_bridge import XfconfError, get_bridge
from mackes.workbench._common import (
    error_label, info_label, labeled_row, panel_box, section_header, title_label,
)


CHANNEL = "xfce4-session"
AUTOSTART_DIR = HOME / ".config" / "autostart"
SYSTEM_AUTOSTART_DIRS = [
    Path("/etc/xdg/autostart"),
    Path("/usr/share/xdg/autostart"),
]


def _parse_desktop(path: Path) -> dict[str, str]:
    out: dict[str, str] = {}
    try:
        in_entry = False
        for line in path.read_text(encoding="utf-8", errors="ignore").splitlines():
            line = line.strip()
            if line.startswith("["):
                in_entry = line == "[Desktop Entry]"
                continue
            if not in_entry or "=" not in line or line.startswith("#"):
                continue
            key, _, value = line.partition("=")
            out[key.strip()] = value.strip()
    except OSError:
        pass
    return out


def _autostart_entries() -> list[tuple[Path, dict[str, str], bool]]:
    """Return (path, parsed_entry, is_user_override). User entries shadow system ones."""
    seen: dict[str, tuple[Path, dict[str, str], bool]] = {}
    for d in SYSTEM_AUTOSTART_DIRS:
        if d.is_dir():
            for p in sorted(d.glob("*.desktop")):
                seen[p.name] = (p, _parse_desktop(p), False)
    if AUTOSTART_DIR.is_dir():
        for p in sorted(AUTOSTART_DIR.glob("*.desktop")):
            seen[p.name] = (p, _parse_desktop(p), True)
    return list(seen.values())


def _is_hidden(entry: dict[str, str]) -> bool:
    return entry.get("Hidden", "").lower() == "true" or \
        entry.get("X-GNOME-Autostart-enabled", "").lower() == "false"


def _set_autostart_hidden(name: str, hidden: bool) -> None:
    """Write/update a user-local override at ~/.config/autostart/<name>."""
    AUTOSTART_DIR.mkdir(parents=True, exist_ok=True)
    user_path = AUTOSTART_DIR / name
    # If a user override already exists, edit it in place; otherwise compose
    # one over the system file's contents.
    if user_path.exists():
        text = user_path.read_text(encoding="utf-8", errors="ignore")
    else:
        system: Path | None = None
        for d in SYSTEM_AUTOSTART_DIRS:
            if (d / name).exists():
                system = d / name
                break
        text = system.read_text(encoding="utf-8", errors="ignore") if system else "[Desktop Entry]\n"

    lines = text.splitlines()
    out: list[str] = []
    set_hidden = False
    set_gnome = False
    for line in lines:
        if line.startswith("Hidden="):
            out.append(f"Hidden={'true' if hidden else 'false'}")
            set_hidden = True
        elif line.startswith("X-GNOME-Autostart-enabled="):
            out.append(f"X-GNOME-Autostart-enabled={'false' if hidden else 'true'}")
            set_gnome = True
        else:
            out.append(line)
    if not set_hidden:
        out.append(f"Hidden={'true' if hidden else 'false'}")
    if not set_gnome:
        out.append(f"X-GNOME-Autostart-enabled={'false' if hidden else 'true'}")
    user_path.write_text("\n".join(out) + "\n", encoding="utf-8")
    log_action(f"autostart: {name} hidden={hidden}")


def _add_autostart(exec_cmd: str, display_name: str) -> Path:
    AUTOSTART_DIR.mkdir(parents=True, exist_ok=True)
    safe = "".join(c if c.isalnum() or c in "-_" else "-" for c in display_name.lower()) or "mackes-entry"
    target = AUTOSTART_DIR / f"{safe}.desktop"
    target.write_text(
        "[Desktop Entry]\n"
        "Type=Application\n"
        f"Name={display_name}\n"
        f"Exec={exec_cmd}\n"
        "X-GNOME-Autostart-enabled=true\n"
        "X-Mackes-Added=1\n",
        encoding="utf-8",
    )
    os.chmod(target, 0o644)
    log_action(f"autostart: added {target.name} -> {exec_cmd}")
    return target


class SessionPanel(Gtk.Box):
    def __init__(self) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self._build()

    def _build(self) -> None:
        box = panel_box()
        box.pack_start(title_label("Session & Startup"), False, False, 0)
        box.pack_start(info_label(
            "Logout behavior and autostart entries. Backed by the xfce4-session "
            "xfconf channel and ~/.config/autostart/."
        ), False, False, 0)

        # Session behavior
        try:
            xf = get_bridge()
        except XfconfError as e:
            box.pack_start(error_label(str(e)), False, False, 0)
        else:
            box.pack_start(section_header("Session"), False, False, 0)

            save_on_exit = Gtk.Switch()
            save_on_exit.set_active(bool(xf.get(CHANNEL, "/general/SaveOnExit", True)))
            save_on_exit.connect("notify::active",
                                 lambda s, _g: xf.set(CHANNEL, "/general/SaveOnExit", s.get_active()))
            box.pack_start(labeled_row("Save session on logout", save_on_exit), False, False, 0)

            prompt = Gtk.Switch()
            prompt.set_active(bool(xf.get(CHANNEL, "/shutdown/LockScreen", False)))
            prompt.connect("notify::active",
                           lambda s, _g: xf.set(CHANNEL, "/shutdown/LockScreen", s.get_active()))
            box.pack_start(labeled_row("Lock screen before suspend", prompt), False, False, 0)

            auto_save = Gtk.Switch()
            auto_save.set_active(bool(xf.get(CHANNEL, "/general/AutoSave", False)))
            auto_save.connect("notify::active",
                              lambda s, _g: xf.set(CHANNEL, "/general/AutoSave", s.get_active()))
            box.pack_start(labeled_row("Auto-save session periodically", auto_save), False, False, 0)

        # Autostart entries
        box.pack_start(section_header("Autostart"), False, False, 0)
        self._list_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        box.pack_start(self._list_box, False, False, 0)

        actions = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        add = Gtk.Button(label="Add command…")
        add.connect("clicked", lambda *_: self._add_dialog())
        refresh = Gtk.Button(label="Refresh")
        refresh.connect("clicked", lambda *_: self._refresh())
        actions.pack_start(add, False, False, 0)
        actions.pack_start(refresh, False, False, 0)
        box.pack_start(actions, False, False, 0)

        self.add(box)
        self._refresh()

    def _refresh(self) -> bool:
        for child in list(self._list_box.get_children()):
            self._list_box.remove(child)
        for path, entry, is_user in _autostart_entries():
            row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
            name = entry.get("Name", path.stem)
            exec_cmd = entry.get("Exec", "")
            badge = "  (user)" if is_user else "  (system)"
            lbl = Gtk.Label(label=f"{name}{badge}\n{exec_cmd}")
            lbl.set_xalign(0); lbl.set_line_wrap(True)
            row.pack_start(lbl, True, True, 0)

            switch = Gtk.Switch(); switch.set_active(not _is_hidden(entry))
            def _on_toggle(s, _g, fname=path.name):
                _set_autostart_hidden(fname, not s.get_active())
                GLib.idle_add(self._refresh)
            switch.connect("notify::active", _on_toggle)
            row.pack_end(switch, False, False, 0)
            self._list_box.pack_start(row, False, False, 0)
        if not self._list_box.get_children():
            self._list_box.pack_start(info_label("No autostart entries."), False, False, 0)
        self._list_box.show_all()
        return False

    def _add_dialog(self) -> None:
        dialog = Gtk.Dialog(title="Add autostart entry", transient_for=self.get_toplevel(),
                            modal=True)
        dialog.add_button("Cancel", Gtk.ResponseType.CANCEL)
        dialog.add_button("Add", Gtk.ResponseType.OK)
        content = dialog.get_content_area()
        content.set_margin_top(12); content.set_margin_bottom(12)
        content.set_margin_start(16); content.set_margin_end(16)

        grid = Gtk.Grid(row_spacing=8, column_spacing=8)
        name_entry = Gtk.Entry(); name_entry.set_placeholder_text("Display name")
        exec_entry = Gtk.Entry(); exec_entry.set_placeholder_text("/path/to/program --args")
        grid.attach(Gtk.Label(label="Name:"), 0, 0, 1, 1)
        grid.attach(name_entry, 1, 0, 1, 1)
        grid.attach(Gtk.Label(label="Command:"), 0, 1, 1, 1)
        grid.attach(exec_entry, 1, 1, 1, 1)
        content.add(grid)
        dialog.show_all()

        if dialog.run() == Gtk.ResponseType.OK:
            name = name_entry.get_text().strip()
            cmd = exec_entry.get_text().strip()
            if name and cmd:
                _add_autostart(cmd, name)
                GLib.idle_add(self._refresh)
        dialog.destroy()
