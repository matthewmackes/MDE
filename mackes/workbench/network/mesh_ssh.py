"""Network → Mesh SSH panel (Carbon-styled).

Four sections per §8.14:
  1. Discovered Peers — Tile per peer with Open Terminal button
  2. Key Distribution — Layer A pubkey sync state
  3. Access Policy — Layer B Headscale Tailscale-SSH policy editor
  4. Audit Log — recent SSH sessions DataTable
"""
from __future__ import annotations

import subprocess

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402

from mackes.carbon import (
    Button, ButtonKind, Tile, DataTable, Column,
    Notification, NotificationKind,
)
from mackes.mesh_ssh import (
    MESH_KEYS_DIR, MESH_POLICY_PATH,
    cheatsheet, load_policy_yaml, save_policy_yaml, read_audit,
)
from mackes.mesh_vpn import headscale_list_peers
from mackes.workbench._common import (
    info_label, panel_box, section_header, title_label,
)


class MeshSshPanel(Gtk.Box):
    def __init__(self) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self._build()
        self._refresh()

    def _build(self) -> None:
        box = panel_box()
        box.pack_start(title_label("Mesh SSH"), False, False, 0)
        box.pack_start(info_label(
            "Three layers: cheatsheet baseline · auto-distributed ed25519 "
            "keys via NATS · Tailscale-SSH identity-based access. "
            "Audit log of every accepted session."
        ), False, False, 0)

        # ---- Discovered peers ----
        box.pack_start(section_header("Discovered peers"), False, False, 0)
        self._peers_table = DataTable(
            columns=[
                Column(name="name",      title="Hostname", width=180),
                Column(name="mesh_ip",   title="Mesh IP",  width=140),
                Column(name="layer",     title="Pref.",    width=80),
                Column(name="action",    title="",         width=120),
            ],
            searchable=True,
            on_row_activate=self._on_peer_activated,
        )
        self._peers_table.set_size_request(-1, 220)
        box.pack_start(self._peers_table, False, True, 0)

        # ---- Key Distribution ----
        box.pack_start(section_header("Key distribution"), False, False, 0)
        self._key_status = Gtk.Label(label="(loading…)")
        self._key_status.set_xalign(0)
        box.pack_start(self._key_status, False, False, 0)
        kd_bar = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        kd_bar.pack_start(
            Button("Re-distribute my key", kind=ButtonKind.TERTIARY,
                   icon_name="document-send-symbolic",
                   on_click=self._on_republish),
            False, False, 0,
        )
        kd_bar.pack_start(
            Button("Sync authorized_keys", kind=ButtonKind.TERTIARY,
                   icon_name="view-refresh-symbolic",
                   on_click=self._on_sync_keys),
            False, False, 0,
        )
        box.pack_start(kd_bar, False, False, 0)

        # ---- Access Policy ----
        box.pack_start(section_header("Access policy (Headscale)"), False, False, 0)
        self._policy_view = Gtk.TextView()
        self._policy_view.set_monospace(True)
        self._policy_view.set_wrap_mode(Gtk.WrapMode.WORD_CHAR)
        scroll_policy = Gtk.ScrolledWindow()
        scroll_policy.set_min_content_height(180)
        scroll_policy.add(self._policy_view)
        box.pack_start(scroll_policy, False, True, 0)
        ap_bar = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        ap_bar.pack_start(
            Button("Save policy", kind=ButtonKind.PRIMARY,
                   icon_name="document-save-symbolic",
                   on_click=self._on_save_policy),
            False, False, 0,
        )
        ap_bar.pack_start(
            Button("Reload from disk", kind=ButtonKind.GHOST,
                   icon_name="view-refresh-symbolic",
                   on_click=self._on_reload_policy),
            False, False, 0,
        )
        box.pack_start(ap_bar, False, False, 0)

        # ---- Audit Log ----
        box.pack_start(section_header("Audit log"), False, False, 0)
        self._audit_table = DataTable(
            columns=[
                Column(name="timestamp",   title="When",     width=160, monospace=True),
                Column(name="source_peer", title="From peer", width=140),
                Column(name="source_user", title="From user", width=100),
                Column(name="target_peer", title="To peer",   width=140),
                Column(name="target_user", title="To user",   width=100),
                Column(name="exit_status", title="rc",        width=60, monospace=True),
            ],
            searchable=True,
        )
        self._audit_table.set_size_request(-1, 240)
        box.pack_start(self._audit_table, True, True, 0)

        self.add(box)

    # ---- refresh -------------------------------------------------------

    def _refresh(self) -> None:
        peers = headscale_list_peers()
        rows = []
        for p in peers:
            layer = "B" if p.online else "A"
            rows.append({
                "name":    p.name,
                "mesh_ip": p.mesh_ip,
                "layer":   layer,
                "action":  "Open Terminal" if p.online else "(offline)",
            })
        self._peers_table.set_rows(rows)

        if MESH_KEYS_DIR.is_dir():
            n = sum(1 for _ in MESH_KEYS_DIR.glob("*.pub"))
            self._key_status.set_text(f"Local cache: {n} peer pubkey(s) in {MESH_KEYS_DIR}")
        else:
            self._key_status.set_text("Mesh-ssh key cache not initialized yet.")

        self._policy_view.get_buffer().set_text(load_policy_yaml())

        audit = read_audit(last_n=200)
        self._audit_table.set_rows([
            {
                "timestamp":   a.timestamp,
                "source_peer": a.source_peer,
                "source_user": a.source_user,
                "target_peer": a.target_peer,
                "target_user": a.target_user,
                "exit_status": a.exit_status,
            }
            for a in reversed(audit)
        ])

    # ---- handlers ------------------------------------------------------

    def _on_peer_activated(self, row: dict) -> None:
        name = row.get("name")
        if not name:
            return
        # Open xfce4-terminal (or fallback) running `mackes ssh <peer>`
        import shutil
        term = (shutil.which("xfce4-terminal") or shutil.which("gnome-terminal")
                or shutil.which("xterm"))
        if term is None:
            return
        subprocess.Popen([term, "-e", f"mackes ssh {name}"],
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                         start_new_session=True)

    def _on_republish(self) -> None:
        from mackes.mesh_ssh import publish_my_pubkey
        publish_my_pubkey()
        self._refresh()

    def _on_sync_keys(self) -> None:
        from mackes.mesh_ssh import sync_authorized_keys
        sync_authorized_keys()
        self._refresh()

    def _on_save_policy(self) -> None:
        buf = self._policy_view.get_buffer()
        text = buf.get_text(buf.get_start_iter(), buf.get_end_iter(), False)
        save_policy_yaml(text)
        self._refresh()

    def _on_reload_policy(self) -> None:
        self._policy_view.get_buffer().set_text(load_policy_yaml())
