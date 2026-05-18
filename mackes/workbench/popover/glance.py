"""Glance tab — live mesh + system summary in the 420×600 popover.

Top-down layout (Q3/Q5/Q10 lock):

  Mesh state pill                   ●  4/8 ok · 1 fail
  Peers (TreeView, top 6)            ● alpha   100.64.0.5  online
                                     ✗ beta    100.64.0.6  offline    [Wake]
  Recent activity (last 5 lines)      mesh: peer alpha came online
  System pulse                        CPU 32%  RAM 41%  drift OK

All values read from mackes.mesh.health() + mackes.state.service_health()
on a daemon thread; UI updates posted via GLib.idle_add. Refreshes
every 5 s while visible.
"""
from __future__ import annotations

import threading

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk  # noqa: E402


class GlanceTab(Gtk.Box):
    def __init__(self) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        self.set_margin_top(12); self.set_margin_bottom(12)
        self.set_margin_start(12); self.set_margin_end(12)

        # Overall mesh pill
        self._mesh_pill = Gtk.Label(label="(loading…)")
        self._mesh_pill.set_xalign(0)
        self._mesh_pill.get_style_context().add_class("mackes-glance-pill")
        self.pack_start(self._mesh_pill, False, False, 0)

        # Peers TreeView
        self.pack_start(self._build_peers_view(), True, True, 0)

        # Recent activity
        self.pack_start(self._heading("Recent activity"), False, False, 0)
        self._activity = Gtk.Label(label="(no activity yet)")
        self._activity.set_xalign(0); self._activity.set_line_wrap(True)
        self._activity.get_style_context().add_class("mackes-glance-meta")
        self.pack_start(self._activity, False, False, 0)

        # System pulse
        self.pack_start(self._heading("System"), False, False, 0)
        self._pulse = Gtk.Label(label="…")
        self._pulse.set_xalign(0)
        self._pulse.get_style_context().add_class("mackes-glance-meta")
        self.pack_start(self._pulse, False, False, 0)

        # Refresh on map; tick while visible
        self._tick_id: int | None = None
        self.connect("map",   lambda *_: self._start_ticking())
        self.connect("unmap", lambda *_: self._stop_ticking())

    # ---- Refresh -------------------------------------------------------

    def _start_ticking(self) -> None:
        if self._tick_id is None:
            self._refresh()
            self._tick_id = GLib.timeout_add_seconds(
                5, lambda: (self._refresh(), True)[1])

    def _stop_ticking(self) -> None:
        if self._tick_id is not None:
            GLib.source_remove(self._tick_id)
            self._tick_id = None

    def _refresh(self) -> None:
        def worker():
            try:
                from mackes.mesh import health, overall_state, summary
                snap = health()
                state = overall_state(snap)
                summary_str = summary(snap)
            except Exception as e:  # noqa: BLE001
                snap, state, summary_str = {}, "fail", f"({e})"
            peers = self._collect_peers(snap)
            activity = self._collect_activity()
            pulse = self._collect_pulse()
            GLib.idle_add(self._apply, state, summary_str, peers, activity, pulse)
        threading.Thread(target=worker, daemon=True).start()

    def _apply(self, state, summary_str, peers, activity, pulse) -> bool:
        glyph = {"ok": "●", "warn": "▲", "fail": "✗", "missing": "○"}.get(
            state, "·")
        pretty = {"ok": "Mesh healthy",  "warn": "Mesh has warnings",
                  "fail": "Mesh has failures",
                  "missing": "Mesh mostly off"}.get(state, state)
        self._mesh_pill.set_text(f"{glyph}  {pretty} · {summary_str}")
        # Refresh peers store
        self._peers_store.clear()
        from mackes.mesh_wol import peer_mac
        for p in peers[:6]:
            online = bool(p.get("online"))
            mac = peer_mac(p.get("mesh_ip", "")) or ""
            self._peers_store.append([
                "" if online else "",
                p.get("name", "?").split(".", 1)[0],
                p.get("mesh_ip", ""),
                "online" if online else "offline",
                mac,
            ])
        self._activity.set_text(activity or "(no activity yet)")
        self._pulse.set_text(pulse or "")
        return False

    # ---- Data sources -------------------------------------------------

    def _collect_peers(self, snap) -> list[dict]:
        try:
            from mackes.mesh_vpn import tailscale_status
            return (tailscale_status().get("peers") or [])
        except Exception:  # noqa: BLE001
            return []

    def _collect_activity(self) -> str:
        from pathlib import Path
        log = Path.home() / ".local/share/mackes-shell/logs/mackes.log"
        if not log.is_file():
            return ""
        try:
            text = log.read_text(encoding="utf-8")
        except OSError:
            return ""
        lines = [
            ln.split(":: ", 1)[-1]
            for ln in text.splitlines()[-5:]
            if ":: " in ln
        ]
        return "\n".join(lines)

    def _collect_pulse(self) -> str:
        try:
            from mackes.state import hardware_summary, service_health
            hw = hardware_summary()
            sv = service_health()
            ok = sum(1 for v in sv.values() if v == "ok")
            return (f"{hw.get('cpu','?').split()[0]} · "
                    f"{hw.get('ram','?')} · "
                    f"services {ok}/{len(sv)} ok")
        except Exception:  # noqa: BLE001
            return ""

    # ---- TreeView builder --------------------------------------------

    def _build_peers_view(self) -> Gtk.Widget:
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        box.pack_start(self._heading("Peers"), False, False, 0)

        # Columns: glyph, name, IP, status, MAC (hidden if absent)
        store = Gtk.ListStore(str, str, str, str, str)
        self._peers_store = store
        view = Gtk.TreeView(model=store)
        view.set_headers_visible(False)
        view.get_style_context().add_class("mackes-glance-table")
        for i, w in enumerate([24, 80, 110, 60]):
            r = Gtk.CellRendererText()
            if i == 0:   # glyph column uses Nerd Font
                r.set_property("family", "Hack Nerd Font Mono")
            elif i == 2 or i == 4:  # IP + MAC monospace
                r.set_property("family", "Hack Nerd Font Mono")
                r.set_property("scale", 0.92)
            col = Gtk.TreeViewColumn("", r, text=i)
            col.set_fixed_width(w)
            col.set_sizing(Gtk.TreeViewColumnSizing.FIXED)
            view.append_column(col)
        scroll = Gtk.ScrolledWindow()
        scroll.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        scroll.set_min_content_height(120)
        scroll.add(view)
        box.pack_start(scroll, True, True, 0)
        return box

    def _heading(self, text: str) -> Gtk.Widget:
        lab = Gtk.Label(label=text); lab.set_xalign(0)
        lab.get_style_context().add_class("mackes-glance-heading")
        return lab


__all__ = ["GlanceTab"]
