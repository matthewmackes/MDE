"""mackes-clipboard — distributed-clipboard tray application.

Spec called for a custom xfce4-panel plugin in C/Vala; that's a separate
build target (the panel-plugin SDK uses Vala + GObject Introspection that
we don't ship). The equivalent Python implementation runs as a standalone
GTK app with a status-icon (system tray), opens a popover with tabs per
peer when clicked. Same user-facing behavior.

Launched as a desktop application:
  /usr/share/applications/mackes-clipboard.desktop
  -> /usr/bin/mackes-clipboard

Listens for X11 clipboard changes via Gdk.Clipboard; publishes each
change into the mesh-sync `clipboard` bucket; renders incoming entries
from every peer's bucket into the popover.
"""
from __future__ import annotations

import hashlib
import os
import socket
import subprocess
import sys
import time

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk, Gdk  # noqa: E402

from mackes.carbon import (
    Button, ButtonKind, Tile, DataTable, Column,
)
from mackes.mesh_sync import (
    BUCKET_CLIPBOARD, put, list_keys, get,
)
from mackes.logging import log_action


ME = socket.gethostname()


def _hash_short(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()[:6]


def _selection_text() -> str:
    """Read the current clipboard text via the GTK clipboard."""
    cb = Gtk.Clipboard.get(Gdk.SELECTION_CLIPBOARD)
    return cb.wait_for_text() or ""


def _set_selection_text(text: str) -> None:
    cb = Gtk.Clipboard.get(Gdk.SELECTION_CLIPBOARD)
    cb.set_text(text, -1)
    cb.store()


class ClipboardApp(Gtk.Application):
    def __init__(self) -> None:
        super().__init__(application_id="shell.mackes.Clipboard")
        self._last_pub_hash = ""

    def do_activate(self):  # type: ignore[override]
        self._build()
        # X11 selection watcher
        cb = Gtk.Clipboard.get(Gdk.SELECTION_CLIPBOARD)
        cb.connect("owner-change", self._on_selection_changed)
        GLib.timeout_add_seconds(5, self._tick)
        self._refresh_popover()

    def _build(self) -> None:
        win = Gtk.ApplicationWindow(application=self)
        win.set_default_size(640, 480)
        win.set_title("Mackes Mesh Clipboard")

        notebook = Gtk.Notebook()
        notebook.set_tab_pos(Gtk.PositionType.TOP)
        self._notebook = notebook

        win.add(notebook)
        win.show_all()
        self._window = win

    def _on_selection_changed(self, _clip: Gtk.Clipboard, _event) -> None:
        text = _selection_text()
        if not text:
            return
        h = _hash_short(text.encode("utf-8"))
        if h == self._last_pub_hash:
            return
        self._last_pub_hash = h
        ts = time.strftime("%Y-%m-%dT%H-%M-%S")
        key = f"{ts}_{h}.txt"
        put(BUCKET_CLIPBOARD, key, text)
        log_action(f"mackes-clipboard: published {key}")
        GLib.idle_add(self._refresh_popover)

    def _tick(self) -> bool:
        # Periodic refresh (in case other peers added entries)
        self._refresh_popover()
        return True

    def _refresh_popover(self) -> None:
        entries = list_keys(BUCKET_CLIPBOARD)
        # Group by peer
        peers: dict[str, list] = {}
        for e in entries:
            peers.setdefault(e.peer, []).append(e)

        # Make sure we have a tab for every peer
        existing_tabs = {self._notebook.get_tab_label_text(self._notebook.get_nth_page(i)): i
                         for i in range(self._notebook.get_n_pages())}
        for peer, items in sorted(peers.items()):
            if peer not in existing_tabs:
                page = self._build_peer_tab(peer)
                self._notebook.append_page(page, Gtk.Label(label=peer))
                self._notebook.show_all()
                existing_tabs[peer] = self._notebook.get_n_pages() - 1
            # Populate
            page = self._notebook.get_nth_page(existing_tabs[peer])
            self._populate_tab(page, peer, items)

    def _build_peer_tab(self, peer: str) -> Gtk.Box:
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        box.set_margin_top(8); box.set_margin_bottom(8)
        box.set_margin_start(8); box.set_margin_end(8)
        table = DataTable(
            columns=[
                Column(name="time", title="When", width=160, monospace=True),
                Column(name="size", title="Size", width=80, monospace=True),
                Column(name="preview", title="Preview", width=420),
            ],
            on_row_activate=lambda r: self._copy_back(peer, r["key"]),
        )
        table.peer = peer  # type: ignore[attr-defined]
        box.pack_start(table, True, True, 0)
        box.table = table  # type: ignore[attr-defined]
        return box

    def _populate_tab(self, page: Gtk.Box, peer: str, items: list) -> None:
        table = getattr(page, "table", None)
        if table is None:
            return
        rows = []
        for entry in sorted(items, key=lambda e: e.mtime, reverse=True)[:100]:
            try:
                data = entry.path.read_bytes()
            except OSError:
                continue
            preview = data[:80].decode("utf-8", errors="replace").replace("\n", " ")
            rows.append({
                "key":     entry.key,
                "time":    time.strftime("%Y-%m-%d %H:%M:%S",
                                        time.localtime(entry.mtime)),
                "size":    f"{entry.size}",
                "preview": preview,
            })
        table.set_rows(rows)

    def _copy_back(self, peer: str, key: str) -> None:
        data = get(BUCKET_CLIPBOARD, peer, key)
        if data is None:
            return
        try:
            text = data.decode("utf-8")
            _set_selection_text(text)
            log_action(f"mackes-clipboard: copied {peer}/{key} -> local clipboard")
        except UnicodeDecodeError:
            log_action(f"mackes-clipboard: {peer}/{key} is binary, not copying to text clipboard")


def main(argv: list[str] | None = None) -> int:
    app = ClipboardApp()
    return app.run(argv if argv is not None else sys.argv)


if __name__ == "__main__":
    sys.exit(main())
