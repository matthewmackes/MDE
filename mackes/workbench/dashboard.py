"""Dashboard — the daily landing view (Q5 lock: live status dashboard).

Sections, top to bottom:
  1. Status strip       — service health badges + active preset + last snapshot
  2. Drift card         — shown only when current state diverges from active preset
  3. Hardware summary   — hostname / CPU / RAM / OS
  4. Quick actions      — six big buttons for the most-used operations
  5. Recent actions     — last 5 Mackes-applied changes (read from mackes.log)
"""
from __future__ import annotations

from typing import Callable, Optional

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402

from pathlib import Path

from mackes.presets import active_preset_drift
from mackes.snapshots import create_snapshot
from mackes.state import (
    LOG_DIR,
    MackesState,
    hardware_summary,
    last_snapshot,
    service_health,
)


def _hero_logo_path() -> Optional[Path]:
    """Return the Mackes Shell hero logo path if shipped; None otherwise."""
    candidates = [
        Path("/usr/share/mackes-shell/branding/MACKES-XFCE-LOGO.png"),
        Path(__file__).resolve().parents[2] / "branding" / "MACKES-XFCE-LOGO.png",
    ]
    for p in candidates:
        if p.exists():
            return p
    return None


_STATUS_DOTS = {
    "ok": "●",
    "warn": "●",
    "fail": "●",
    "missing": "○",
}
_STATUS_CLASSES = {
    "ok": "success",
    "warn": "warning",
    "fail": "error",
    "missing": "dim-label",
}


def _section(title: str) -> tuple[Gtk.Box, Gtk.Box]:
    """Return (outer, content) — content is where you pack the section body."""
    outer = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
    head = Gtk.Label(label=title.upper())
    head.set_xalign(0)
    ctx = head.get_style_context()
    ctx.add_class("title-4")
    ctx.add_class("mackes-section-header")
    outer.pack_start(head, False, False, 0)

    frame = Gtk.Frame()
    frame.set_shadow_type(Gtk.ShadowType.NONE)
    frame.get_style_context().add_class("view")
    content = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
    content.set_margin_top(12); content.set_margin_bottom(12)
    content.set_margin_start(14); content.set_margin_end(14)
    frame.add(content)
    outer.pack_start(frame, False, False, 0)
    return outer, content


class DashboardView(Gtk.Box):
    def __init__(self, state: MackesState,
                 navigate: Optional[Callable[[str], None]] = None) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=18)
        self.state = state
        self.navigate = navigate or (lambda _t: None)
        self.set_margin_top(20); self.set_margin_bottom(20)
        self.set_margin_start(20); self.set_margin_end(20)

        scroller = Gtk.ScrolledWindow()
        scroller.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        inner = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=18)
        scroller.add(inner)
        self.pack_start(scroller, True, True, 0)

        self._inner = inner
        self._render()

    def _render(self) -> None:
        for child in list(self._inner.get_children()):
            self._inner.remove(child)
        hero = self._hero_image()
        if hero is not None:
            self._inner.pack_start(hero, False, False, 0)
        self._inner.pack_start(self._status_strip(), False, False, 0)
        drift = self._drift_card()
        if drift is not None:
            self._inner.pack_start(drift, False, False, 0)
        self._inner.pack_start(self._hardware_card(), False, False, 0)
        self._inner.pack_start(self._quick_actions(), False, False, 0)
        self._inner.pack_start(self._recent_actions(), False, False, 0)
        self._inner.show_all()


    def _hero_image(self) -> Optional[Gtk.Widget]:
        from gi.repository import GdkPixbuf
        logo = _hero_logo_path()
        if logo is None:
            return None
        try:
            pixbuf = GdkPixbuf.Pixbuf.new_from_file_at_scale(
                str(logo), width=420, height=-1, preserve_aspect_ratio=True,
            )
        except Exception:  # noqa: BLE001
            return None
        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        img = Gtk.Image.new_from_pixbuf(pixbuf)
        img.set_halign(Gtk.Align.CENTER)
        box.pack_start(img, True, True, 0)
        return box

    # ---- Section 1: status strip -----------------------------------------

    def _status_strip(self) -> Gtk.Widget:
        outer, content = _section("Status")
        row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=18)
        for name, status in service_health().items():
            cell = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
            dot = Gtk.Label(label=_STATUS_DOTS[status])
            dot.get_style_context().add_class(_STATUS_CLASSES[status])
            cell.pack_start(dot, False, False, 0)
            cell.pack_start(Gtk.Label(label=name), False, False, 0)
            row.pack_start(cell, False, False, 0)
        content.pack_start(row, False, False, 0)

        meta = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=24)
        preset_label = Gtk.Label(label=f"Active preset: {self.state.active_preset or 'none'}")
        preset_label.set_xalign(0)
        meta.pack_start(preset_label, False, False, 0)

        snap = last_snapshot()
        snap_text = "Last snapshot: none yet"
        if snap is not None:
            name, when = snap
            snap_text = f"Last snapshot: {when:%Y-%m-%d %H:%M} — {name}"
        snap_label = Gtk.Label(label=snap_text)
        snap_label.set_xalign(0)
        snap_label.get_style_context().add_class("dim-label")
        meta.pack_start(snap_label, False, False, 0)

        content.pack_start(meta, False, False, 0)
        return outer

    # ---- Section 2: drift card (conditional) -----------------------------

    def _drift_card(self) -> Gtk.Widget | None:
        if not self.state.active_preset:
            return None
        try:
            preset, items = active_preset_drift()
        except Exception:  # noqa: BLE001
            return None
        if preset is None or not items:
            return None

        outer, content = _section(f"Drift from preset \"{preset.display_name}\"")
        head = Gtk.Label(label=(
            f"⚠  {len(items)} item(s) differ from the preset's declared values."
        ))
        head.set_xalign(0); head.set_line_wrap(True)
        head.get_style_context().add_class("warning")
        content.pack_start(head, False, False, 0)

        for it in items[:8]:
            line = Gtk.Label(label=f"  • {it.section}.{it.field}: "
                                   f"preset={it.expected!r}  live={it.actual!r}")
            line.set_xalign(0); line.set_line_wrap(True)
            content.pack_start(line, False, False, 0)
        if len(items) > 8:
            more = Gtk.Label(label=f"  …and {len(items) - 8} more.")
            more.set_xalign(0); more.get_style_context().add_class("dim-label")
            content.pack_start(more, False, False, 0)

        btns = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        review_btn = Gtk.Button(label="Open Maintain → Reset")
        review_btn.connect("clicked", lambda *_: self.navigate("reset"))
        btns.pack_start(review_btn, False, False, 0)
        snap_btn = Gtk.Button(label="Snapshot first")
        snap_btn.connect("clicked", lambda *_: (create_snapshot("pre-drift-review"),
                                                self._render()))
        btns.pack_start(snap_btn, False, False, 0)
        content.pack_start(btns, False, False, 0)
        return outer

    # ---- Section 3: hardware -------------------------------------------

    def _hardware_card(self) -> Gtk.Widget:
        outer, content = _section("This machine")
        grid = Gtk.Grid(column_spacing=24, row_spacing=4)
        info = hardware_summary()
        rows = [
            ("Hostname", info.get("hostname", "")),
            ("OS",       info.get("os", "")),
            ("CPU",      info.get("cpu", "")),
            ("RAM",      info.get("ram", "")),
        ]
        for i, (k, v) in enumerate(rows):
            lk = Gtk.Label(label=k); lk.set_xalign(0); lk.get_style_context().add_class("dim-label")
            lv = Gtk.Label(label=str(v)); lv.set_xalign(0)
            grid.attach(lk, 0, i, 1, 1)
            grid.attach(lv, 1, i, 1, 1)
        content.pack_start(grid, False, False, 0)
        return outer

    # ---- Section 4: quick actions ---------------------------------------

    def _quick_actions(self) -> Gtk.Widget:
        outer, content = _section("Quick actions")
        grid = Gtk.Grid(column_spacing=10, row_spacing=10,
                        column_homogeneous=True)
        actions: list[tuple[str, str, Callable[[], None]]] = [
            ("Open Appearance",        "preferences-desktop",
             lambda: self.navigate("appearance")),
            ("Open Display",           "preferences-desktop-display",
             lambda: self.navigate("display")),
            ("Open Network",           "network-wired",
             lambda: self.navigate("network")),
            ("Create Snapshot",        "document-save",
             self._on_snapshot),
            ("Health Check",           "emblem-system",
             lambda: self.navigate("health")),
            ("Open Log",               "text-x-generic",
             lambda: self.navigate("logs")),
        ]
        for i, (label, icon, fn) in enumerate(actions):
            btn = Gtk.Button()
            btn_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
            btn_box.set_margin_top(4); btn_box.set_margin_bottom(4)
            btn_box.set_margin_start(6); btn_box.set_margin_end(6)
            img = Gtk.Image.new_from_icon_name(icon, Gtk.IconSize.LARGE_TOOLBAR)
            btn_box.pack_start(img, False, False, 0)
            btn_box.pack_start(Gtk.Label(label=label), False, False, 0)
            btn.add(btn_box)
            btn.connect("clicked", lambda _b, f=fn: f())
            grid.attach(btn, i % 3, i // 3, 1, 1)
        content.pack_start(grid, False, False, 0)
        return outer

    def _on_snapshot(self) -> None:
        snap = create_snapshot("dashboard-quick-snapshot",
                               source_preset=self.state.active_preset)
        self._render()

    # ---- Section 5: recent actions --------------------------------------

    def _recent_actions(self) -> Gtk.Widget:
        outer, content = _section("Recent activity")
        log = LOG_DIR / "mackes.log"
        if not log.exists():
            empty = Gtk.Label(label="No activity yet.")
            empty.set_xalign(0)
            empty.get_style_context().add_class("dim-label")
            content.pack_start(empty, False, False, 0)
            return outer

        lines: list[str] = []
        try:
            for line in log.read_text(encoding="utf-8", errors="ignore").splitlines()[-8:]:
                lines.append(line)
        except OSError:
            pass

        for line in lines or ["No activity yet."]:
            lbl = Gtk.Label(label=line)
            lbl.set_xalign(0)
            lbl.set_line_wrap(True)
            content.pack_start(lbl, False, False, 0)
        return outer
