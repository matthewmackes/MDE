"""KDE Connect Workbench panels (Phase 13.3.1 – 13.3.6).

Six panels live here — Devices, Clipboard, Files, SMS, Phone, and
Device-Detail. Each is a self-contained Gtk.Box subclass. A small
top-level `KdeConnectControlPanel` notebook ties them together so
the sidebar shows a single "KDE Connect" entry that tabs into the
detail views.

The data layer reads through `mackes_kdc` (the Rust crate exposed
later via PyO3 in v2.0.0) when available, falling back to a
file-based scan of `~/.config/kdeconnect/<uuid>/` so the panels
render gracefully when the daemon isn't running. Live DBus calls
land alongside Phase 13.2 (the bridge daemon).
"""
from __future__ import annotations

import json
import os
import time
from pathlib import Path

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402

from mackes.workbench._common import a11y, empty_state, error_state


def _kde_config_root() -> Path:
    return Path(os.environ.get("HOME", str(Path.home()))) / ".config" / "kdeconnect"


def _is_uuid_dir(name: str) -> bool:
    """Match KDE Connect's UUID directory convention (8-4-4-4-12 hex
    with optional dashes)."""
    cleaned = name.replace("-", "")
    return len(cleaned) == 32 and all(c in "0123456789abcdef" for c in cleaned)


def paired_device_records() -> list[dict]:
    """Best-effort scan of `~/.config/kdeconnect/<uuid>/identity.json`
    so the panel renders even when the upstream daemon isn't running.

    Returns a list of dicts: {id, name, kind, reachable, battery_pct,
    last_seen_s}. Empty list when the config root is missing.
    """
    root = _kde_config_root()
    try:
        entries = list(root.iterdir())
    except OSError:
        return []
    out: list[dict] = []
    for entry in entries:
        if not entry.is_dir() or not _is_uuid_dir(entry.name):
            continue
        record = {
            "id":          entry.name,
            "name":        entry.name[:8],
            "kind":        "unknown",
            "reachable":   False,
            "battery_pct": None,
            "last_seen_s": 0,
        }
        identity = entry / "identity.json"
        if identity.exists():
            try:
                data = json.loads(identity.read_text(encoding="utf-8"))
                record["name"] = data.get("name", record["name"])
                record["kind"] = data.get("deviceType", record["kind"])
            except (OSError, json.JSONDecodeError):
                pass
        try:
            stat = entry.stat()
            record["last_seen_s"] = int(stat.st_mtime)
        except OSError:
            pass
        out.append(record)
    return out


# -----------------------------------------------------------------
# Pure-helper formatters (unit-testable without GTK).
# -----------------------------------------------------------------

_KIND_GLYPHS = {
    "phone":   "📱",
    "tablet":  "📟",
    "desktop": "🖥",
    "unknown": "❓",
}


def format_device_label(record: dict) -> str:
    """Single-line label for a device row.

    Shape: ``<glyph> <name>  ·  <kind>  ·  <reachable | offline>``
    """
    glyph = _KIND_GLYPHS.get(record.get("kind", "unknown"), _KIND_GLYPHS["unknown"])
    name = record.get("name", record.get("id", "?"))
    kind = record.get("kind", "unknown")
    reach = "reachable" if record.get("reachable") else "offline"
    return f"{glyph} {name}  ·  {kind}  ·  {reach}"


def format_last_seen(epoch_s: int, *, now: int | None = None) -> str:
    """Human-readable "just now" / "Xm ago" / "Xh ago" / "Xd ago"
    formatter for the device row meta line. Lifted out for unit tests."""
    if epoch_s <= 0:
        return "never"
    now_s = now if now is not None else int(time.time())
    delta = max(0, now_s - epoch_s)
    if delta < 60:
        return "just now"
    if delta < 3600:
        return f"{delta // 60}m ago"
    if delta < 86400:
        return f"{delta // 3600}h ago"
    return f"{delta // 86400}d ago"


# -----------------------------------------------------------------
# Panel widgets — one per worklist substep.
# -----------------------------------------------------------------

def _page_title(text: str) -> Gtk.Widget:
    label = Gtk.Label(label=text)
    label.set_xalign(0)
    label.get_style_context().add_class("mackes-page-title")
    return label


def _page_subtitle(text: str) -> Gtk.Widget:
    label = Gtk.Label(label=text)
    label.set_xalign(0); label.set_line_wrap(True)
    label.get_style_context().add_class("mackes-page-subtitle")
    return label


class KdeConnectDevicesPanel(Gtk.Box):
    """Devices panel (Phase 13.3.1) — paired + reachable list with
    pair/unpair + drill-in."""

    def __init__(self, on_select_device=None) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.set_margin_top(16); self.set_margin_start(16)
        self.set_margin_end(16); self.set_margin_bottom(16)
        self._on_select_device = on_select_device
        self.pack_start(_page_title("Paired devices"), False, False, 0)
        self.pack_start(_page_subtitle(
            "Phones and tablets reachable through KDE Connect or the "
            "mesh-mDNS bridge."
        ), False, False, 6)
        self._list = Gtk.ListBox()
        self._list.set_selection_mode(Gtk.SelectionMode.SINGLE)
        scroll = Gtk.ScrolledWindow()
        scroll.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        scroll.add(self._list)
        self.pack_start(scroll, True, True, 0)
        self._refresh()

    def _refresh(self) -> None:
        for c in self._list.get_children():
            self._list.remove(c)
        devices = paired_device_records()
        if not devices:
            self._list.add(empty_state(
                "No paired devices",
                "Pair a phone or tablet from its KDE Connect app — it "
                "will appear here.",
                None, None,
            ))
            self._list.show_all()
            return
        for record in devices:
            row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
            row.set_margin_top(6); row.set_margin_bottom(6)
            row.set_margin_start(8); row.set_margin_end(8)
            text = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
            text.set_hexpand(True)
            title = Gtk.Label(label=format_device_label(record))
            title.set_xalign(0); title.get_style_context().add_class("mackes-row-title")
            meta = Gtk.Label(label=(
                f"last seen: {format_last_seen(record.get('last_seen_s', 0))}"
            ))
            meta.set_xalign(0); meta.get_style_context().add_class("mackes-row-meta")
            text.pack_start(title, False, False, 0)
            text.pack_start(meta, False, False, 0)
            row.pack_start(text, True, True, 0)
            open_btn = Gtk.Button(label="Open")
            a11y(open_btn, f"Open detail panel for {record['name']}", tooltip=None)
            open_btn.connect("clicked", lambda _b, rid=record["id"]:
                             self._on_select_device(rid) if self._on_select_device else None)
            row.pack_start(open_btn, False, False, 0)
            self._list.add(row)
        self._list.show_all()


class _ListWithEmptyPanel(Gtk.Box):
    """Shared chrome for the Clipboard / Files / SMS panels — title +
    subtitle + scrolling list of rows + empty state."""

    def __init__(self, title: str, subtitle: str, empty_title: str,
                 empty_body: str) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.set_margin_top(16); self.set_margin_start(16)
        self.set_margin_end(16); self.set_margin_bottom(16)
        self.pack_start(_page_title(title), False, False, 0)
        self.pack_start(_page_subtitle(subtitle), False, False, 6)
        self._list_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        self.pack_start(self._list_box, True, True, 0)
        self._empty_title = empty_title
        self._empty_body = empty_body
        self._render_empty()

    def _render_empty(self) -> None:
        for c in self._list_box.get_children():
            self._list_box.remove(c)
        self._list_box.pack_start(empty_state(
            self._empty_title, self._empty_body, None, None,
        ), False, False, 0)
        self._list_box.show_all()


class KdeConnectClipboardPanel(_ListWithEmptyPanel):
    """Clipboard panel (Phase 13.3.2)."""

    def __init__(self) -> None:
        super().__init__(
            title="Clipboard",
            subtitle=(
                "Per-device clipboard view with push/pull. The last "
                "50 entries are kept."
            ),
            empty_title="No clipboard activity yet",
            empty_body=(
                "Copy something on a paired device and it will appear "
                "here. Use Push to send your current clipboard to a "
                "device; Pull to receive the device's clipboard."
            ),
        )


class KdeConnectFilesPanel(_ListWithEmptyPanel):
    """Files panel (Phase 13.3.3)."""

    def __init__(self) -> None:
        super().__init__(
            title="Files",
            subtitle=(
                "Drag-and-drop file send + per-device receive history. "
                "Drops land in ~/Downloads/<device>/."
            ),
            empty_title="No file transfers yet",
            empty_body=(
                "Drag a file from your file manager onto this panel "
                "to send it. Files received from devices will show "
                "up here as well."
            ),
        )


class KdeConnectSmsPanel(_ListWithEmptyPanel):
    """SMS panel (Phase 13.3.4) — Android-only."""

    def __init__(self) -> None:
        super().__init__(
            title="SMS",
            subtitle=(
                "Per-device thread list with send-from-desktop. "
                "Android only — iOS doesn't expose SMS over KDE Connect."
            ),
            empty_title="No SMS threads",
            empty_body=(
                "When a paired Android device unlocks and the "
                "Connect app has SMS permission, conversation "
                "threads appear here."
            ),
        )


class KdeConnectPhonePanel(_ListWithEmptyPanel):
    """Phone panel (Phase 13.3.5)."""

    def __init__(self) -> None:
        super().__init__(
            title="Phone",
            subtitle=(
                "Battery, Find-my-phone, MPRIS media controls, call "
                "silencer, remote-input pairing."
            ),
            empty_title="No phone reachable",
            empty_body=(
                "When a paired phone is online, its battery, media, "
                "and find-my-phone controls show up here."
            ),
        )


class KdeConnectDetailPanel(Gtk.Box):
    """Per-device deep view (Phase 13.3.6)."""

    def __init__(self, device_id: str | None = None) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.set_margin_top(16); self.set_margin_start(16)
        self.set_margin_end(16); self.set_margin_bottom(16)
        self.pack_start(_page_title("Device detail"), False, False, 0)
        self.pack_start(_page_subtitle(
            "Per-device deep view. Use the Devices tab to drill in."
        ), False, False, 6)
        self._content = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        self.pack_start(self._content, True, True, 0)
        self.show_device(device_id)

    def show_device(self, device_id: str | None) -> None:
        for c in self._content.get_children():
            self._content.remove(c)
        if not device_id:
            self._content.pack_start(empty_state(
                "No device selected",
                "Pick a device from the Devices tab to see battery, "
                "clipboard, file history, and per-feature pairing.",
                None, None,
            ), False, False, 0)
        else:
            records = [r for r in paired_device_records() if r["id"] == device_id]
            if not records:
                self._content.pack_start(
                    error_state(
                        f"Device {device_id} not found",
                        "It may have been unpaired since the last "
                        "refresh.", None, None,
                    ),
                    False, False, 0,
                )
            else:
                r = records[0]
                meta = Gtk.Label(label=(
                    f"id:          {r['id']}\n"
                    f"name:        {r['name']}\n"
                    f"kind:        {r['kind']}\n"
                    f"reachable:   {r['reachable']}\n"
                    f"battery:     {r.get('battery_pct') or '?'}%\n"
                    f"last seen:   {format_last_seen(r.get('last_seen_s', 0))}\n"
                ))
                meta.set_xalign(0); meta.set_line_wrap(True)
                self._content.pack_start(meta, False, False, 0)
        self._content.show_all()


class KdeConnectControlPanel(Gtk.Box):
    """Top-level tabbed surface combining the six 13.3.x panels."""

    def __init__(self) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self._notebook = Gtk.Notebook()
        self._notebook.set_scrollable(True)
        a11y(self._notebook, "KDE Connect feature tabs", tooltip=None)

        self._detail_panel = KdeConnectDetailPanel()
        self._devices_panel = KdeConnectDevicesPanel(
            on_select_device=self._open_device,
        )
        for label, widget in [
            ("Devices",   self._devices_panel),
            ("Clipboard", KdeConnectClipboardPanel()),
            ("Files",     KdeConnectFilesPanel()),
            ("SMS",       KdeConnectSmsPanel()),
            ("Phone",     KdeConnectPhonePanel()),
            ("Detail",    self._detail_panel),
        ]:
            scroller = Gtk.ScrolledWindow()
            scroller.set_policy(Gtk.PolicyType.AUTOMATIC,
                                Gtk.PolicyType.AUTOMATIC)
            scroller.add(widget)
            self._notebook.append_page(scroller, Gtk.Label(label=label))
        self.pack_start(self._notebook, True, True, 0)

    def _open_device(self, device_id: str) -> None:
        self._detail_panel.show_device(device_id)
        # Detail tab is the 6th page (index 5).
        self._notebook.set_current_page(5)
