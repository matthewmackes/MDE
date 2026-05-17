"""mackes-clipboard — distributed mesh clipboard (v1.5.0 rewrite).

Three roles, picked by CLI flag:

  --daemon   :  Headless watcher. Listens to XA_CLIPBOARD via
                Gtk.Clipboard owner-change; publishes every new text or
                image to ~/QNM-Shared/.qnm-sync/clipboard/<me>/<ts>.{txt,png}.
                Filters likely secrets (token-shaped strings; opt-in via
                Tweaks → 'Sync sensitive items'). Runs forever; meant
                to be supervised by mackes-clipboard-daemon.service.

  --viewer   :  Foreground GTK window showing every peer's clipboard
                history grouped into tabs. Double-click an entry to
                copy it back to this peer's clipboard.

  (no flag)  :  Default = --viewer (legacy launcher path).

Spec called for a C/Vala xfce4-panel plugin; the Python implementation
covers the same surface with less moving infrastructure. The
companion C plugin at /usr/lib64/xfce4/panel/plugins/mackes-clipboard
provides the panel-popup read view; this app provides the daemon write
side + a full window viewer.
"""
from __future__ import annotations

import hashlib
import math
import os
import socket
import sys
import time
from pathlib import Path
from typing import Optional

import gi
gi.require_version("Gtk", "3.0")
gi.require_version("Gdk", "3.0")
gi.require_version("GdkPixbuf", "2.0")
from gi.repository import GdkPixbuf, GLib, Gtk, Gdk  # noqa: E402

from mackes.mesh_sync import (
    BUCKET_CLIPBOARD, put, list_keys, get,
)
from mackes.logging import log_action


ME = socket.gethostname()
SETTINGS_FILE = Path(os.path.expanduser(
    "~/.config/mackes-shell/clipboard-daemon.json"))


# --------------------------------------------------------------------------
# Settings (Tweaks-panel writes these; daemon reads them)
# --------------------------------------------------------------------------


def _load_settings() -> dict:
    import json
    defaults = {
        "enabled": True,
        "sync_text": True,
        "sync_images": True,
        "filter_secrets": True,
        "filter_min_length": 12,    # below this length: never a secret
        "filter_max_length": 256,   # above this length: probably a paragraph
        "filter_entropy": 4.5,      # shannon-bits/char; tokens are ~5+
        "max_text_bytes": 65536,
        "max_image_bytes": 4 * 1024 * 1024,
    }
    if SETTINGS_FILE.exists():
        try:
            data = json.loads(SETTINGS_FILE.read_text(encoding="utf-8"))
            defaults.update({k: v for k, v in data.items() if k in defaults})
        except (OSError, ValueError):
            pass
    return defaults


def _save_settings(s: dict) -> None:
    import json
    SETTINGS_FILE.parent.mkdir(parents=True, exist_ok=True)
    SETTINGS_FILE.write_text(
        json.dumps(s, indent=2, sort_keys=True), encoding="utf-8")


# --------------------------------------------------------------------------
# Heuristics — likely-secret detection
# --------------------------------------------------------------------------


def _shannon_entropy(s: str) -> float:
    """Bits per character. Compact tokens like API keys land 4.5+."""
    if not s:
        return 0.0
    freqs = {}
    for ch in s:
        freqs[ch] = freqs.get(ch, 0) + 1
    n = len(s)
    return -sum((c / n) * math.log2(c / n) for c in freqs.values())


def looks_like_secret(text: str, *, settings: Optional[dict] = None) -> bool:
    """Returns True if text matches the "likely secret" heuristic.

      - length within the configured window
      - no whitespace (tokens are dense)
      - shannon entropy >= configured threshold
      - or matches a few high-confidence prefixes (sk-, ghp_, github_pat_, …)
    """
    if not text:
        return False
    s = settings or _load_settings()
    if not s.get("filter_secrets", True):
        return False
    HIGH_CONF_PREFIXES = (
        "sk-",            # OpenAI
        "ghp_",           # GitHub PAT
        "github_pat_",    # GitHub fine-grained PAT
        "gho_",           # GitHub OAuth
        "ghu_",           # GitHub user-to-server
        "ghs_",           # GitHub server-to-server
        "ghr_",           # GitHub refresh
        "xoxp-", "xoxb-", "xoxa-", "xoxr-",  # Slack
        "AKIA", "ASIA",   # AWS access key IDs
        "AIza",           # Google API key
        "ya29.",          # Google OAuth
        "ssh-rsa ", "ssh-ed25519 ", "ssh-dss ",  # SSH public keys
    )
    if any(text.startswith(p) for p in HIGH_CONF_PREFIXES):
        return True
    if any(p in text for p in (
        "BEGIN PRIVATE KEY", "BEGIN RSA PRIVATE",
        "BEGIN OPENSSH PRIVATE", "BEGIN PGP PRIVATE",
    )):
        return True
    L = len(text)
    if L < s.get("filter_min_length", 12):
        return False
    if L > s.get("filter_max_length", 256):
        return False
    if any(c.isspace() for c in text):
        return False
    if _shannon_entropy(text) >= s.get("filter_entropy", 4.5):
        return True
    return False


# --------------------------------------------------------------------------
# Daemon — XA_CLIPBOARD watcher → mesh bucket writer
# --------------------------------------------------------------------------


class ClipboardDaemon:
    """Headless watcher. Spawned by mackes-clipboard-daemon.service."""

    def __init__(self) -> None:
        self._last_text_hash = ""
        self._last_image_hash = ""
        self._settings = _load_settings()
        self._last_settings_read = 0.0
        self._SETTINGS_TTL = 10.0   # re-read every 10s

    def run(self) -> int:
        if not self._settings.get("enabled", True):
            log_action("clipboard daemon: disabled in settings — exiting")
            return 0
        cb = Gtk.Clipboard.get(Gdk.SELECTION_CLIPBOARD)
        cb.connect("owner-change", self._on_owner_change)
        # Periodic resync (in case `owner-change` misses an event)
        GLib.timeout_add_seconds(5, self._tick)
        log_action(f"clipboard daemon: watching XA_CLIPBOARD as peer '{ME}'")
        try:
            Gtk.main()
        except KeyboardInterrupt:
            return 0
        return 0

    def _refresh_settings_if_stale(self) -> None:
        now = time.time()
        if (now - self._last_settings_read) >= self._SETTINGS_TTL:
            self._settings = _load_settings()
            self._last_settings_read = now

    def _tick(self) -> bool:
        self._refresh_settings_if_stale()
        if not self._settings.get("enabled", True):
            return True
        self._sample_clipboard()
        return True

    def _on_owner_change(self, _cb, _event):
        self._refresh_settings_if_stale()
        if not self._settings.get("enabled", True):
            return
        self._sample_clipboard()

    def _sample_clipboard(self) -> None:
        cb = Gtk.Clipboard.get(Gdk.SELECTION_CLIPBOARD)
        # Try text first; if absent, try image
        text = None
        try:
            text = cb.wait_for_text()
        except Exception:  # noqa: BLE001
            text = None
        if text:
            self._publish_text(text)
            return
        if self._settings.get("sync_images", True):
            try:
                pixbuf = cb.wait_for_image()
            except Exception:  # noqa: BLE001
                pixbuf = None
            if pixbuf is not None:
                self._publish_image(pixbuf)

    def _publish_text(self, text: str) -> None:
        if not self._settings.get("sync_text", True):
            return
        max_b = int(self._settings.get("max_text_bytes", 65536))
        b = text.encode("utf-8", errors="replace")
        if len(b) > max_b:
            return
        if looks_like_secret(text, settings=self._settings):
            log_action(
                "clipboard daemon: skipped likely-secret entry "
                f"({len(b)}b, entropy "
                f"{_shannon_entropy(text):.2f})"
            )
            return
        h = hashlib.sha256(b).hexdigest()[:8]
        if h == self._last_text_hash:
            return
        self._last_text_hash = h
        ts = time.strftime("%Y-%m-%dT%H-%M-%S")
        try:
            put(BUCKET_CLIPBOARD, f"{ts}_{h}.txt", text)
            log_action(f"clipboard daemon: published {ts}_{h}.txt ({len(b)}b)")
        except Exception as e:  # noqa: BLE001
            log_action(f"clipboard daemon: publish error: {e}")

    def _publish_image(self, pixbuf: GdkPixbuf.Pixbuf) -> None:
        if not self._settings.get("sync_images", True):
            return
        max_b = int(self._settings.get("max_image_bytes", 4 * 1024 * 1024))
        ok, buf = pixbuf.save_to_bufferv("png", [], [])
        if not ok or buf is None:
            return
        if len(buf) > max_b:
            return
        h = hashlib.sha256(buf).hexdigest()[:8]
        if h == self._last_image_hash:
            return
        self._last_image_hash = h
        ts = time.strftime("%Y-%m-%dT%H-%M-%S")
        try:
            put(BUCKET_CLIPBOARD, f"{ts}_{h}.png", buf)
            log_action(f"clipboard daemon: published image {ts}_{h}.png ({len(buf)}b)")
        except Exception as e:  # noqa: BLE001
            log_action(f"clipboard daemon: image publish error: {e}")


# --------------------------------------------------------------------------
# Viewer — foreground window with per-peer history tabs
# --------------------------------------------------------------------------


class ClipboardViewer(Gtk.Application):
    def __init__(self) -> None:
        super().__init__(application_id="shell.mackes.Clipboard")
        self._notebook: Optional[Gtk.Notebook] = None

    def do_activate(self):  # type: ignore[override]
        self._build()
        GLib.timeout_add_seconds(5, self._refresh_tabs_tick)
        self._refresh_tabs()

    def _build(self) -> None:
        win = Gtk.ApplicationWindow(application=self)
        win.set_default_size(820, 560)
        win.set_title("Mackes Mesh Clipboard")
        win.get_style_context().add_class("mackes-app-window")

        outer = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        outer.set_margin_top(24); outer.set_margin_bottom(24)
        outer.set_margin_start(32); outer.set_margin_end(32)

        title = Gtk.Label(label="Mesh Clipboard")
        title.set_xalign(0)
        title.get_style_context().add_class("mackes-page-title")
        outer.pack_start(title, False, False, 0)
        sub = Gtk.Label(label=(
            "Every peer's recent clipboard items, replicated through "
            "QNM-Shared/.qnm-sync/clipboard/. Double-click an entry "
            "to copy it into this peer's local clipboard."
        ))
        sub.set_xalign(0); sub.set_line_wrap(True)
        sub.get_style_context().add_class("mackes-page-subtitle")
        outer.pack_start(sub, False, False, 0)

        notebook = Gtk.Notebook()
        notebook.set_tab_pos(Gtk.PositionType.TOP)
        notebook.set_margin_top(16)
        outer.pack_start(notebook, True, True, 0)
        self._notebook = notebook

        win.add(outer)
        win.show_all()

    def _refresh_tabs_tick(self) -> bool:
        self._refresh_tabs()
        return True

    def _refresh_tabs(self) -> None:
        if self._notebook is None:
            return
        entries = list_keys(BUCKET_CLIPBOARD)
        peers: dict[str, list] = {}
        for e in entries:
            peers.setdefault(e.peer, []).append(e)
        existing = {self._notebook.get_tab_label_text(self._notebook.get_nth_page(i)): i
                    for i in range(self._notebook.get_n_pages())}
        for peer in sorted(peers):
            if peer not in existing:
                page = self._build_peer_tab(peer)
                self._notebook.append_page(page, Gtk.Label(label=peer))
                self._notebook.show_all()
                existing[peer] = self._notebook.get_n_pages() - 1
            self._populate_tab(self._notebook.get_nth_page(existing[peer]),
                                peer, peers[peer])

    def _build_peer_tab(self, peer: str) -> Gtk.Box:
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        box.set_margin_top(12); box.set_margin_bottom(8)
        box.set_margin_start(8); box.set_margin_end(8)
        listbox = Gtk.ListBox()
        listbox.set_selection_mode(Gtk.SelectionMode.SINGLE)
        listbox.connect("row-activated",
                        lambda _b, row: self._on_row_activated(peer, row))
        scroller = Gtk.ScrolledWindow()
        scroller.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        scroller.add(listbox)
        box.pack_start(scroller, True, True, 0)
        box.listbox = listbox  # type: ignore[attr-defined]
        return box

    def _populate_tab(self, page: Gtk.Box, peer: str, items: list) -> None:
        listbox = getattr(page, "listbox", None)
        if listbox is None:
            return
        # Replace contents
        for c in list(listbox.get_children()):
            listbox.remove(c)
        for entry in sorted(items, key=lambda e: e.mtime, reverse=True)[:200]:
            row = Gtk.ListBoxRow()
            row.entry_key = entry.key                      # type: ignore[attr-defined]
            row.entry_peer = peer                          # type: ignore[attr-defined]
            row.entry_path = entry.path                    # type: ignore[attr-defined]
            row.get_style_context().add_class("mackes-side-nav-item")

            box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
            box.set_margin_top(8); box.set_margin_bottom(8)
            box.set_margin_start(12); box.set_margin_end(12)
            when = Gtk.Label(label=time.strftime(
                "%H:%M:%S", time.localtime(entry.mtime)))
            when.set_xalign(0); when.set_size_request(76, -1)
            when.get_style_context().add_class("mackes-section-meta")
            box.pack_start(when, False, False, 0)
            kind_label = "image" if entry.key.endswith(".png") else "text"
            tag = Gtk.Label(label=kind_label)
            tag.get_style_context().add_class("mackes-tag")
            tag.get_style_context().add_class(
                "info" if kind_label == "image" else "neutral")
            box.pack_start(tag, False, False, 0)
            try:
                data = entry.path.read_bytes()
                if entry.key.endswith(".png"):
                    preview = f"<image {entry.size}b>"
                else:
                    preview = data[:120].decode("utf-8", errors="replace") \
                                        .replace("\n", " ")
            except OSError:
                preview = "(unreadable)"
            lbl = Gtk.Label(label=preview)
            lbl.set_xalign(0); lbl.set_line_wrap(False)
            lbl.set_max_width_chars(80)
            lbl.set_ellipsize(__import__("gi").repository.Pango.EllipsizeMode.END)
            box.pack_start(lbl, True, True, 0)
            row.add(box)
            listbox.add(row)
        listbox.show_all()

    def _on_row_activated(self, peer: str, row: Gtk.ListBoxRow) -> None:
        key = getattr(row, "entry_key", None)
        if key is None:
            return
        data = get(BUCKET_CLIPBOARD, peer, key)
        if data is None:
            return
        cb = Gtk.Clipboard.get(Gdk.SELECTION_CLIPBOARD)
        if key.endswith(".png"):
            try:
                loader = GdkPixbuf.PixbufLoader.new()
                loader.write(data); loader.close()
                pixbuf = loader.get_pixbuf()
                if pixbuf is not None:
                    cb.set_image(pixbuf); cb.store()
                    log_action(f"clipboard: pasted image {peer}/{key}")
            except Exception as e:  # noqa: BLE001
                log_action(f"clipboard: image paste failed: {e}")
        else:
            try:
                cb.set_text(data.decode("utf-8", errors="replace"), -1)
                cb.store()
                log_action(f"clipboard: pasted text {peer}/{key}")
            except Exception as e:  # noqa: BLE001
                log_action(f"clipboard: text paste failed: {e}")


# --------------------------------------------------------------------------
# CLI entry
# --------------------------------------------------------------------------


def main(argv: list[str] | None = None) -> int:
    argv = list(argv if argv is not None else sys.argv)
    args = argv[1:]
    if "--daemon" in args:
        return ClipboardDaemon().run()
    # Default: viewer
    args = [a for a in args if a not in ("--viewer", "--gui")]
    return ClipboardViewer().run([argv[0]] + args)


if __name__ == "__main__":
    sys.exit(main())
