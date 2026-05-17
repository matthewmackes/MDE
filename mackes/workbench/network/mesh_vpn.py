"""Network → Mesh VPN panel (Carbon-styled).

Renders the live mesh state from mackes.mesh_vpn — peer list, control
status, add-peer modal, diagnostics, advanced (ACLs, DERP, exit nodes).
"""
from __future__ import annotations

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk  # noqa: E402

from mackes.carbon import (
    Button, ButtonKind, Tile, DataTable, Column,
    Notification, NotificationKind, Modal, ModalSize,
)
from mackes.mesh_vpn import (
    MeshState, MESH_CAP, headscale_list_peers,
    generate_join_link, tailscale_status,
)
from mackes.workbench._common import (
    info_label, panel_box, section_header, title_label,
)


class MeshVpnPanel(Gtk.Box):
    def __init__(self) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self._build()
        self._refresh()

    def _build(self) -> None:
        box = panel_box()
        box.pack_start(title_label("Mesh VPN"), False, False, 0)
        box.pack_start(info_label(
            "Self-hosted Headscale control plane + Tailscale clients. "
            "Cross-network peers find the mesh via the Tailscale-bootstrap "
            "presence (only the seed peer is registered there). Up to 16 "
            "peers per mesh."
        ), False, False, 0)

        # ---- Status tile ----
        self._status_tile = Tile(title="Status")
        self._status_label = Gtk.Label(label="(loading…)")
        self._status_label.set_xalign(0)
        self._status_tile.pack(self._status_label)
        box.pack_start(self._status_tile, False, False, 0)

        # ---- Action bar ----
        actions_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        add_btn = Button("Add Peer", kind=ButtonKind.PRIMARY,
                         icon_name="list-add-symbolic",
                         on_click=self._on_add_peer)
        leave_btn = Button("Leave Mesh", kind=ButtonKind.DANGER,
                           icon_name="window-close-symbolic",
                           on_click=self._on_leave_mesh)
        diag_btn = Button("Diagnostics", kind=ButtonKind.SECONDARY,
                          icon_name="emblem-system-symbolic",
                          on_click=self._on_diagnostics)
        refresh_btn = Button("Refresh", kind=ButtonKind.GHOST,
                             icon_name="view-refresh-symbolic",
                             on_click=self._refresh)
        for b in (add_btn, leave_btn, diag_btn, refresh_btn):
            actions_box.pack_start(b, False, False, 0)
        box.pack_start(actions_box, False, False, 0)

        # ---- Peer DataTable ----
        box.pack_start(section_header("Peers"), False, False, 0)
        self._peers_table = DataTable(
            columns=[
                Column(name="name",      title="Hostname", width=160),
                Column(name="mesh_ip",   title="Mesh IP",  width=140),
                Column(name="route",     title="Route",    width=100),
                Column(name="rtt",       title="RTT",      width=80),
                Column(name="last_seen", title="Last seen", width=160),
                Column(name="status",    title="Status",   width=80),
            ],
            searchable=True,
        )
        self._peers_table.set_size_request(-1, 280)
        box.pack_start(self._peers_table, True, True, 0)

        # ---- Control-node info ----
        box.pack_start(section_header("Control node"), False, False, 0)
        self._control_label = Gtk.Label(label="(loading…)")
        self._control_label.set_xalign(0)
        box.pack_start(self._control_label, False, False, 0)

        self.add(box)

    # ---- refresh -------------------------------------------------------

    def _refresh(self) -> None:
        state = MeshState.load()
        ts = tailscale_status()
        peers = headscale_list_peers()
        n = len(peers)

        # Status
        if state.is_control:
            line = f"You are the control node · {n}/{MESH_CAP} peers"
            klass = NotificationKind.SUCCESS
        elif state.mesh_id:
            line = f"Connected · {n}/{MESH_CAP} peers · Control: {state.control_peer_id}"
            klass = NotificationKind.INFO
        else:
            line = "Not joined to any mesh. Run the wizard's Network screen."
            klass = NotificationKind.WARNING
        # Clear previous status content
        for c in list(self._status_tile._body.get_children()):
            self._status_tile._body.remove(c)
        self._status_tile._body.pack_start(Notification(line, kind=klass,
                                                        dismissible=False),
                                           False, False, 0)
        self._status_tile.show_all()

        # Peer table rows
        rows = []
        for p in peers:
            rows.append({
                "name":      p.name,
                "mesh_ip":   p.mesh_ip,
                "route":     p.route,
                "rtt":       "—" if p.rtt_ms is None else f"{p.rtt_ms}ms",
                "last_seen": p.last_seen or ("now" if p.online else "—"),
                "status":    "online" if p.online else "offline",
            })
        self._peers_table.set_rows(rows)

        # Control-node label
        if state.is_control:
            self._control_label.set_text(
                f"This machine ({state.control_peer_id})  ·  "
                f"snapshot age: {self._fmt_age(state.last_snapshot)}"
            )
        else:
            self._control_label.set_text(
                f"Held by: {state.control_peer_id or '(unknown)'}  ·  "
                "This peer is eligible for failover if it disappears > 120s."
            )

    @staticmethod
    def _fmt_age(epoch: float) -> str:
        import time
        if epoch <= 0:
            return "(never)"
        age = time.time() - epoch
        if age < 60:
            return f"{int(age)}s ago"
        if age < 3600:
            return f"{int(age/60)}m ago"
        return f"{int(age/3600)}h ago"

    # ---- actions -------------------------------------------------------

    def _on_add_peer(self) -> None:
        from mackes.mesh_vpn import at_capacity
        if at_capacity():
            Notification("Mesh capacity reached",
                         body=f"Maximum of {MESH_CAP} peers. Remove one first.",
                         kind=NotificationKind.ERROR,
                         dismissible=True).show()
            return
        link, actions = generate_join_link(expiration="10m")
        body = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        if not link:
            body.pack_start(Gtk.Label(
                label="Could not generate a join link. Is the control node up?"
            ), False, False, 0)
            for a in actions:
                lbl = Gtk.Label(label=a); lbl.set_xalign(0)
                lbl.get_style_context().add_class("cds-helper-text-01")
                body.pack_start(lbl, False, False, 0)
        else:
            body.pack_start(Gtk.Label(
                label="Share this link with the joining peer.\n"
                      "Paste into their Mackes wizard's Network screen "
                      "(or scan the QR if displayed).\n"
                      "Valid for 10 minutes."
            ), False, False, 0)
            link_entry = Gtk.Entry()
            link_entry.set_text(link)
            link_entry.set_editable(False)
            link_entry.set_can_focus(True)
            body.pack_start(link_entry, False, False, 0)
            copy_btn = Button("Copy", kind=ButtonKind.TERTIARY,
                              on_click=lambda: self._copy_to_clip(link))
            body.pack_start(copy_btn, False, False, 0)

        modal = Modal(self.get_toplevel(), "Add Peer", body, size=ModalSize.MEDIUM)
        modal.add_action("Close", kind=ButtonKind.SECONDARY)
        modal.run_then_destroy()

    def _copy_to_clip(self, text: str) -> None:
        from gi.repository import Gdk
        clip = Gtk.Clipboard.get(Gdk.SELECTION_CLIPBOARD)
        clip.set_text(text, -1)

    def _on_leave_mesh(self) -> None:
        body = Gtk.Label(label=(
            "This will disconnect this peer from the mesh, stop the local\n"
            "Tailscale client, and (if this peer is the control node) hand\n"
            "the role to the next eligible peer via election.\n\n"
            "Existing snapshots and SSH keys are preserved."
        ))
        body.set_xalign(0); body.set_line_wrap(True)
        modal = Modal(self.get_toplevel(), "Leave the mesh?", body, size=ModalSize.SMALL)
        modal.add_action("Cancel", kind=ButtonKind.SECONDARY,
                         response_id=Gtk.ResponseType.CANCEL)
        modal.add_action("Leave Mesh", kind=ButtonKind.DANGER,
                         on_click=self._do_leave,
                         response_id=Gtk.ResponseType.OK)
        modal.run_then_destroy()

    def _do_leave(self) -> None:
        import subprocess
        subprocess.call(["tailscale", "down"])
        GLib.idle_add(self._refresh)

    def _on_diagnostics(self) -> None:
        # Open the diagnostics text in a modal (read-only)
        ts = tailscale_status()
        text = (
            f"=== tailscale (mesh data plane) ===\n"
            f"installed: {ts.get('installed')}\n"
            f"online:    {ts.get('online')}\n"
            f"mesh IP:   {ts.get('mesh_ip')}\n"
            f"peers:     {len(ts.get('peers', []))}\n"
            f"\n"
            f"=== headscale ===\n"
            f"binary: /usr/bin/headscale\n"
            f"\n"
            f"For deeper inspection see:\n"
            f"  mackes maintain logs --follow\n"
            f"  systemctl status headscale\n"
            f"  tailscale status\n"
        )
        body = Gtk.ScrolledWindow()
        tv = Gtk.TextView(); tv.set_editable(False); tv.set_monospace(True)
        tv.get_buffer().set_text(text)
        body.add(tv); body.set_size_request(-1, 320)
        modal = Modal(self.get_toplevel(), "Mesh VPN diagnostics", body, size=ModalSize.LARGE)
        modal.add_action("Close", kind=ButtonKind.SECONDARY)
        modal.run_then_destroy()
