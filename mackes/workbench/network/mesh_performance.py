"""Network → Mesh Performance — every perf knob in one panel.

Sections (filled in as each of the 10 perf-ideas lands):

  Datapath     kernel WG / userspace WG status + toggle
  MTU + GSO    current MTU, LAN-MTU toggle, GSO state, sysctl drop-in
  Probes       concurrent-probe speedup vs serial (informational)
  Discovery    mDNS-SD service announcer state               (#4)
  Relay        private DERP server status on the control     (#1)
  Metrics      Prometheus exporter + Grafana URL             (#8)
  Storage      Headscale DB backend (sqlite vs postgres)     (#7)
  Stream       NATS JetStream connection                     (#5)
  Mount        mesh-fs FUSE backend                          (#6)
  Peers        per-peer table with Wake / RTT / throughput   (#10)

Reads from mackes.mesh_perf + mackes.mesh.health() + mackes.mesh_wol.
"""
from __future__ import annotations

import threading

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk  # noqa: E402

from mackes.workbench._common import (
    info_label,
    panel_box,
    section_description,
    section_header,
    title_label,
)


def _breadcrumb(parts: list[str]) -> Gtk.Widget:
    bc = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=4)
    bc.get_style_context().add_class("mackes-breadcrumb")
    for i, p in enumerate(parts):
        lab = Gtk.Label(label=p); lab.set_xalign(0)
        bc.pack_start(lab, False, False, 0)
        if i != len(parts) - 1:
            sep = Gtk.Label(label="/"); sep.set_xalign(0)
            sep.get_style_context().add_class("mackes-dot")
            bc.pack_start(sep, False, False, 0)
    return bc


def _page_subtitle(text: str) -> Gtk.Widget:
    lab = Gtk.Label(label=text)
    lab.set_xalign(0); lab.set_line_wrap(True)
    lab.get_style_context().add_class("mackes-page-subtitle")
    return lab


def _switch_row(label: str, *, initial: bool, on_change) -> Gtk.Widget:
    row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
    row.set_margin_top(4); row.set_margin_bottom(4)
    lab = Gtk.Label(label=label); lab.set_xalign(0)
    row.pack_start(lab, True, True, 0)
    sw = Gtk.Switch(); sw.set_active(initial)
    sw.connect("notify::active",
               lambda s, _gp: on_change(s.get_active()))
    row.pack_start(sw, False, False, 0)
    return row


class MeshPerformancePanel(Gtk.Box):
    """Network → Mesh Performance full-page panel."""

    def __init__(self) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        outer = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        # Tight margins so the page fits a 1366×768 laptop without scroll
        outer.set_margin_top(16); outer.set_margin_bottom(16)
        outer.set_margin_start(24); outer.set_margin_end(24)

        outer.pack_start(_breadcrumb(["Mackes Shell", "Network",
                                      "Mesh Performance"]),
                         False, False, 0)
        t = Gtk.Label(label="Mesh Performance")
        t.set_xalign(0); t.get_style_context().add_class("mackes-page-title")
        outer.pack_start(t, False, False, 0)
        outer.pack_start(_page_subtitle(
            "Every speed dial for the mesh in one place. Green is on; "
            "grey is off. Most settings take effect next time you join "
            "the mesh — Re-apply Mesh from the wizard to pick them up."
        ), False, False, 0)

        outer.pack_start(self._build_datapath_section(), False, False, 0)
        outer.pack_start(self._build_mtu_section(),       False, False, 0)
        outer.pack_start(self._build_sysctl_section(),    False, False, 0)
        outer.pack_start(self._build_discovery_section(), False, False, 0)
        outer.pack_start(self._build_relay_section(),     False, False, 0)
        outer.pack_start(self._build_metrics_section(),   False, False, 0)
        outer.pack_start(self._build_storage_section(),   False, False, 0)
        outer.pack_start(self._build_stream_section(),    False, False, 0)
        outer.pack_start(self._build_mount_section(),     False, False, 0)
        outer.pack_start(self._build_peers_section(),     False, False, 0)

        self.pack_start(outer, True, True, 0)
        # Refresh status lines on a thread after construct
        threading.Thread(target=self._refresh_all, daemon=True).start()

    # ---- Datapath (kernel WG) -------------------------------------------

    def _build_datapath_section(self) -> Gtk.Widget:
        box = panel_box()
        box.pack_start(section_header("Datapath"), False, False, 0)
        box.pack_start(section_description(
            "Kernel WireGuard is faster than the userspace fallback by "
            "30–50%. Mackes turns it on automatically when your "
            "kernel has the wireguard module."
        ), False, False, 0)
        self._datapath_status = Gtk.Label(label="(checking…)")
        self._datapath_status.set_xalign(0)
        box.pack_start(self._datapath_status, False, False, 0)

        from mackes.mesh_perf import use_kernel_mode_preference, set_use_kernel_mode
        row = _switch_row(
            "Use kernel WireGuard when available",
            initial=use_kernel_mode_preference(),
            on_change=lambda v: set_use_kernel_mode(v),
        )
        box.pack_start(row, False, False, 0)
        return box

    # ---- MTU + GSO -------------------------------------------------------

    def _build_mtu_section(self) -> Gtk.Widget:
        box = panel_box()
        box.pack_start(section_header("MTU + offload"), False, False, 0)
        box.pack_start(section_description(
            "Bigger packets = less overhead per byte. The LAN-MTU "
            "setting bumps WireGuard from the default 1280 to 1380, "
            "which is safe on any 1500-MTU LAN."
        ), False, False, 0)
        self._mtu_status = Gtk.Label(label="(checking…)")
        self._mtu_status.set_xalign(0)
        box.pack_start(self._mtu_status, False, False, 0)

        from mackes.mesh_perf import preferred_mtu, set_preferred_mtu, LAN_MTU
        row = _switch_row(
            f"Use LAN-optimised MTU ({LAN_MTU})",
            initial=preferred_mtu() > 0,
            on_change=lambda v: set_preferred_mtu(LAN_MTU if v else 0),
        )
        box.pack_start(row, False, False, 0)
        return box

    # ---- sysctl drop-in --------------------------------------------------

    def _build_sysctl_section(self) -> Gtk.Widget:
        box = panel_box()
        box.pack_start(section_header("Socket buffers (sysctl)"), False, False, 0)
        box.pack_start(section_description(
            "Lets the kernel hold bigger bursts of UDP packets before "
            "dropping them. Helps when many peers send data at once. "
            "Writes /etc/sysctl.d/90-mackes-mesh.conf — you'll be "
            "asked for your password."
        ), False, False, 0)
        self._sysctl_status = Gtk.Label(label="(checking…)")
        self._sysctl_status.set_xalign(0)
        box.pack_start(self._sysctl_status, False, False, 0)

        bar = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        apply_btn = Gtk.Button(label="Apply tuning")
        apply_btn.get_style_context().add_class("suggested-action")
        apply_btn.connect("clicked", lambda *_: self._apply_sysctl())
        bar.pack_start(apply_btn, False, False, 0)
        remove_btn = Gtk.Button(label="Remove")
        remove_btn.connect("clicked", lambda *_: self._remove_sysctl())
        bar.pack_start(remove_btn, False, False, 0)
        box.pack_start(bar, False, False, 0)
        return box

    def _apply_sysctl(self) -> None:
        def worker():
            from mackes.mesh_perf import apply_sysctl_tuning
            for line in apply_sysctl_tuning():
                pass
            self._refresh_all()
        threading.Thread(target=worker, daemon=True).start()

    def _remove_sysctl(self) -> None:
        def worker():
            from mackes.mesh_perf import remove_sysctl_tuning
            remove_sysctl_tuning()
            self._refresh_all()
        threading.Thread(target=worker, daemon=True).start()

    # ---- Stub sections (filled in by later #-tasks) ---------------------

    def _build_discovery_section(self) -> Gtk.Widget:
        box = panel_box()
        box.pack_start(section_header("Service discovery (mDNS)"), False, False, 0)
        self._discovery_status = Gtk.Label(label="(checking…)")
        self._discovery_status.set_xalign(0)
        box.pack_start(self._discovery_status, False, False, 0)
        return box

    def _build_relay_section(self) -> Gtk.Widget:
        box = panel_box()
        box.pack_start(section_header("Private relay (DERP)"), False, False, 0)
        self._relay_status = Gtk.Label(label="(checking…)")
        self._relay_status.set_xalign(0)
        box.pack_start(self._relay_status, False, False, 0)
        return box

    def _build_metrics_section(self) -> Gtk.Widget:
        box = panel_box()
        box.pack_start(section_header("Metrics (Prometheus)"), False, False, 0)
        box.pack_start(section_description(
            "A small Rust exporter publishes per-peer transfer + "
            "handshake stats on port 9586. The control peer scrapes "
            "every machine so you can graph the whole fleet at once."
        ), False, False, 0)
        self._metrics_status = Gtk.Label(label="(checking…)")
        self._metrics_status.set_xalign(0)
        box.pack_start(self._metrics_status, False, False, 0)

        bar = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        install_btn = Gtk.Button(label="Install exporter")
        install_btn.get_style_context().add_class("suggested-action")
        install_btn.connect("clicked", lambda *_: self._install_metrics())
        bar.pack_start(install_btn, False, False, 0)
        remove_btn = Gtk.Button(label="Remove")
        remove_btn.connect("clicked", lambda *_: self._remove_metrics())
        bar.pack_start(remove_btn, False, False, 0)
        box.pack_start(bar, False, False, 0)
        return box

    def _install_metrics(self) -> None:
        def worker():
            from mackes.mesh_metrics import install_exporter
            install_exporter()
            self._refresh_all()
        threading.Thread(target=worker, daemon=True).start()

    def _remove_metrics(self) -> None:
        def worker():
            from mackes.mesh_metrics import uninstall_exporter
            uninstall_exporter()
            self._refresh_all()
        threading.Thread(target=worker, daemon=True).start()

    def _build_storage_section(self) -> Gtk.Widget:
        box = panel_box()
        box.pack_start(section_header("Control-node storage"), False, False, 0)
        self._storage_status = Gtk.Label(label="(checking…)")
        self._storage_status.set_xalign(0)
        box.pack_start(self._storage_status, False, False, 0)
        return box

    def _build_stream_section(self) -> Gtk.Widget:
        box = panel_box()
        box.pack_start(section_header("Sync streams (NATS)"), False, False, 0)
        self._stream_status = Gtk.Label(label="(checking…)")
        self._stream_status.set_xalign(0)
        box.pack_start(self._stream_status, False, False, 0)
        return box

    def _build_mount_section(self) -> Gtk.Widget:
        box = panel_box()
        box.pack_start(section_header("Peer file shares (mesh-fs)"), False, False, 0)
        self._mount_status = Gtk.Label(label="(checking…)")
        self._mount_status.set_xalign(0)
        box.pack_start(self._mount_status, False, False, 0)
        return box

    # ---- Peers (with Wake button) ---------------------------------------

    def _build_peers_section(self) -> Gtk.Widget:
        box = panel_box()
        box.pack_start(section_header("Peers"), False, False, 0)
        box.pack_start(section_description(
            "Each row shows one machine on your mesh. Click Wake to "
            "send a wake-on-LAN packet to a sleeping peer (only "
            "works on the same physical network)."
        ), False, False, 0)
        self._peers_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        box.pack_start(self._peers_box, False, False, 0)
        return box

    # ---- Refresh ---------------------------------------------------------

    def _refresh_all(self) -> None:
        from mackes import mesh_perf, mesh_wol
        try:
            from mackes.mesh_vpn import tailscale_status
            ts = tailscale_status()
        except Exception:  # noqa: BLE001
            ts = {"online": False, "peers": []}
        snap = mesh_perf.summary()

        # Cache MACs while we have the live ARP table
        try:
            mesh_wol.cache_peer_macs()
        except Exception:  # noqa: BLE001
            pass

        def apply():
            # Datapath
            if snap["kernel_module_loaded"]:
                self._datapath_status.set_text(
                    "✓ Kernel WireGuard module loaded — fastest path.")
            elif snap["kernel_mode_available"]:
                self._datapath_status.set_text(
                    "Kernel module is installed but not yet loaded. "
                    "Mackes will use it on next mesh-join.")
            else:
                self._datapath_status.set_text(
                    "Kernel module not available — using userspace "
                    "wireguard-go (still works, just slower).")
            # MTU
            mtu = snap["current_mtu"] or "(no mesh iface)"
            pref = snap["preferred_mtu"]
            gso = "GSO on" if snap["gso_enabled"] else "GSO off"
            self._mtu_status.set_text(
                f"Current MTU: {mtu} · Preferred: "
                f"{pref if pref else 'auto'} · {gso}"
            )
            # sysctl
            self._sysctl_status.set_text(
                "✓ Tuning active" if snap["sysctl_tuning_active"]
                else "Default kernel buffers — apply to enable bursts."
            )
            # Stub statuses
            from mackes import mesh_mdns, mesh_derp
            if mesh_mdns.is_available():
                self._discovery_status.set_text(
                    "✓ mDNS-SD ready (python-zeroconf + avahi-publish "
                    "installed). Services announce on the mesh "
                    "interface in real-time.")
            elif mesh_mdns.has_avahi_publish():
                self._discovery_status.set_text(
                    "avahi-publish installed but python-zeroconf is "
                    "missing — listener path unavailable. "
                    "`pip install zeroconf` (or `dnf install "
                    "python3-zeroconf`).")
            else:
                self._discovery_status.set_text(
                    "Polling-based discovery (active). Install avahi "
                    "+ python3-zeroconf for push-based mDNS.")
            ds = mesh_derp.status()
            if ds["running"]:
                self._relay_status.set_text(
                    f"✓ Private DERP running on :{ds['ports']['derp']}")
            elif ds["installed"]:
                self._relay_status.set_text(
                    "DERP binary installed but service not running.")
            else:
                self._relay_status.set_text(
                    "Using Tailscale public DERP. Install a private "
                    "relay on the control peer for LAN-local latency.")
            from mackes.mesh_metrics import prometheus_status
            ps = prometheus_status()
            if ps["exporter_running"]:
                self._metrics_status.set_text(
                    f"✓ Exporter running on :{ps['exporter_url'].rsplit(':',1)[-1].split('/')[0]}")
            elif ps["exporter_installed"]:
                self._metrics_status.set_text(
                    "Exporter binary installed but service not running.")
            else:
                self._metrics_status.set_text(
                    "Not installed. Click Install to download the "
                    "exporter (~5 MB) and start it.")
            from mackes import headscale_postgres as hp
            hs = hp.status()
            if hs["backend"] == "postgres":
                pg_state = ("✓ running" if hs["pg_running"]
                            else "configured but cluster not running")
                self._storage_status.set_text(
                    f"Headscale on Postgres ({pg_state}, port "
                    f"{hs['pg_port']}).")
            elif hs["backend"] == "sqlite":
                self._storage_status.set_text(
                    "Headscale on SQLite (default). On a fleet >20 "
                    "peers, migrate to Postgres for parallel writes.")
            else:
                self._storage_status.set_text(
                    "Headscale config not detected — install via "
                    "Mesh Setup Wizard.")
            self._stream_status.set_text(
                "Filesystem-based sync (active). "
                "NATS JetStream upgrade ships in a follow-up.")
            self._mount_status.set_text(
                "SSHFS per peer (active). "
                "mesh-fs FUSE backend ships in a follow-up.")

            # Peers table
            for c in self._peers_box.get_children():
                self._peers_box.remove(c)
            peers = ts.get("peers") or []
            if not peers:
                self._peers_box.pack_start(info_label(
                    "No peers online. Join the mesh from Network → "
                    "Get Online to populate this list."
                ), False, False, 0)
            else:
                for p in peers:
                    self._peers_box.pack_start(
                        self._build_peer_row(p), False, False, 0)
            self._peers_box.show_all()
        GLib.idle_add(apply)

    def _build_peer_row(self, p: dict) -> Gtk.Widget:
        from mackes.mesh_wol import peer_mac
        row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        row.get_style_context().add_class("mackes-data-row")
        row.set_margin_top(4); row.set_margin_bottom(4)
        name = p.get("name", "(unknown)").split(".", 1)[0]
        ip = p.get("mesh_ip", "")
        online = bool(p.get("online"))

        nlabel = Gtk.Label(label=name); nlabel.set_xalign(0)
        nlabel.get_style_context().add_class("mackes-section-title")
        row.pack_start(nlabel, True, True, 0)

        ilab = Gtk.Label(label=ip or "(no IP)")
        ilab.get_style_context().add_class("mackes-code")
        row.pack_start(ilab, False, False, 0)

        pill = Gtk.Label(label="ONLINE" if online else "OFFLINE")
        pill.get_style_context().add_class("mackes-tag")
        pill.get_style_context().add_class(
            "mackes-pill-ok" if online else "mackes-pill-neutral")
        row.pack_start(pill, False, False, 0)

        if not online:
            wake_btn = Gtk.Button(label="Wake")
            wake_btn.connect("clicked",
                             lambda *_: self._wake_peer(name, p))
            mac = peer_mac(ip) or peer_mac(name)
            if mac is None:
                wake_btn.set_sensitive(False)
                wake_btn.set_tooltip_text(
                    "No cached MAC — connect once while online so "
                    "the ARP table learns it")
            row.pack_start(wake_btn, False, False, 0)
        return row

    def _wake_peer(self, name: str, p: dict) -> None:
        def worker():
            from mackes.mesh_wol import wake_peer
            ok, msg = wake_peer(p.get("mesh_ip") or name)
            GLib.idle_add(self._show_wake_result, name, ok, msg)
        threading.Thread(target=worker, daemon=True).start()

    def _show_wake_result(self, name: str, ok: bool, msg: str) -> bool:
        # Update the peer's row label briefly to show the result
        try:
            dlg = Gtk.MessageDialog(
                parent=self.get_toplevel(),
                flags=0,
                message_type=(Gtk.MessageType.INFO if ok
                              else Gtk.MessageType.WARNING),
                buttons=Gtk.ButtonsType.OK,
                text=f"Wake {name}",
            )
            dlg.format_secondary_text(msg)
            dlg.run()
            dlg.destroy()
        except Exception:  # noqa: BLE001
            pass
        return False


__all__ = ["MeshPerformancePanel"]
