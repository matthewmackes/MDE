"""Network → Mesh Services panel (§8.13).

Five sections:
  - Discovered services (Tile grid)
  - Unified gateway (Caddy + CA install)
  - Bundled clients (Jellyfin Media Player, Strawberry)
  - mDNS bridge (per-service-type opt-out)
  - Help cheatsheet (Layer 1 raw URLs)
"""
from __future__ import annotations

import subprocess

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402

from mackes.carbon import (
    Button, ButtonKind, Tile, ClickableTile, MultiSelect,
    Notification, NotificationKind,
)
from mackes.mesh_services import (
    load_catalog, load_registry, probe_all, url_for, launch,
)
from mackes.mdns_relay import DEFAULT_RELAYED_TYPES, DEFAULT_PRIVATE_TYPES
from mackes.workbench._common import (
    info_label, panel_box, section_header, title_label,
)


class MeshServicesPanel(Gtk.Box):
    def __init__(self) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self._build()
        self._refresh()

    def _build(self) -> None:
        box = panel_box()
        box.pack_start(title_label("Mesh Services"), False, False, 0)
        box.pack_start(info_label(
            "Discover HTTP services across every mesh peer (Jellyfin, "
            "Airsonic, Plex, Sonarr, Home Assistant, Grafana, and many "
            "more). Open in browser, launch native clients, or expose "
            "everything under a single https://media.mesh URL."
        ), False, False, 0)

        # ---- Discovered services tile grid ----
        box.pack_start(section_header("Discovered services"), False, False, 0)
        actions_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        actions_box.pack_start(
            Button("Scan now", kind=ButtonKind.PRIMARY,
                   icon_name="view-refresh-symbolic",
                   on_click=self._on_scan_now),
            False, False, 0,
        )
        actions_box.pack_start(
            Button("Refresh tiles", kind=ButtonKind.GHOST,
                   icon_name="view-refresh-symbolic",
                   on_click=self._refresh),
            False, False, 0,
        )
        box.pack_start(actions_box, False, False, 0)

        self._scroll = Gtk.ScrolledWindow()
        self._scroll.set_min_content_height(280)
        self._grid = Gtk.FlowBox()
        self._grid.set_valign(Gtk.Align.START)
        self._grid.set_max_children_per_line(4)
        self._grid.set_min_children_per_line(2)
        self._grid.set_selection_mode(Gtk.SelectionMode.NONE)
        self._grid.set_homogeneous(True)
        self._grid.set_column_spacing(8)
        self._grid.set_row_spacing(8)
        self._scroll.add(self._grid)
        box.pack_start(self._scroll, True, True, 0)

        # ---- Unified gateway ----
        box.pack_start(section_header("Unified gateway (https://media.mesh)"), False, False, 0)
        gw_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        gw_box.pack_start(info_label(
            "Optional Caddy reverse proxy that exposes every mesh service "
            "at https://media.mesh/<service>/<peer>/. Private CA installed "
            "into each peer's trust store via pkexec."
        ), True, True, 0)
        gw_box.pack_start(
            Button("Enable gateway", kind=ButtonKind.TERTIARY,
                   icon_name="emblem-system-symbolic",
                   on_click=self._on_enable_gateway),
            False, False, 0,
        )
        gw_box.pack_start(
            Button("Install CA cert", kind=ButtonKind.SECONDARY,
                   icon_name="security-high-symbolic",
                   on_click=self._on_install_ca),
            False, False, 0,
        )
        box.pack_start(gw_box, False, False, 0)

        # ---- Bundled native clients ----
        box.pack_start(section_header("Bundled native clients"), False, False, 0)
        nc_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        nc_box.pack_start(
            Button("Refresh server lists", kind=ButtonKind.TERTIARY,
                   icon_name="document-send-symbolic",
                   on_click=self._on_refresh_native_clients),
            False, False, 0,
        )
        self._native_status = Gtk.Label(label="(idle)")
        self._native_status.set_xalign(0)
        nc_box.pack_start(self._native_status, True, True, 0)
        box.pack_start(nc_box, False, False, 0)

        # ---- mDNS bridge ----
        box.pack_start(section_header("mDNS bridge"), False, False, 0)
        items = [
            (t, t, t in DEFAULT_RELAYED_TYPES)
            for t in (*DEFAULT_RELAYED_TYPES, *DEFAULT_PRIVATE_TYPES)
        ]
        self._mdns_select = MultiSelect(
            "Service types to relay across the mesh", items,
            on_change=self._on_mdns_changed,
        )
        box.pack_start(self._mdns_select, False, False, 0)

        # ---- Cheatsheet (Layer 1) ----
        box.pack_start(section_header("Raw URL cheatsheet"), False, False, 0)
        self._cheat = Gtk.TextView()
        self._cheat.set_monospace(True)
        self._cheat.set_editable(False)
        self._cheat.set_wrap_mode(Gtk.WrapMode.NONE)
        cheat_scroll = Gtk.ScrolledWindow()
        cheat_scroll.set_min_content_height(120)
        cheat_scroll.add(self._cheat)
        box.pack_start(cheat_scroll, False, False, 0)

        self.add(box)

    # ---- refresh -------------------------------------------------------

    def _refresh(self) -> None:
        for child in list(self._grid.get_children()):
            self._grid.remove(child)
        hits = load_registry()
        if not hits:
            empty = Notification("No services discovered yet",
                                 body='Click "Scan now" to probe each mesh peer.',
                                 kind=NotificationKind.INFO, dismissible=False)
            self._grid.add(empty)
        else:
            catalog = {d.name: d for d in load_catalog()}
            for hit in hits:
                tile = ClickableTile(
                    on_click=(lambda h=hit: launch(h) and None),
                )
                title = Gtk.Label(label=catalog.get(hit.service).display
                                  if hit.service in catalog else hit.service)
                title.set_xalign(0)
                title.get_style_context().add_class("cds-heading-02")
                tile.pack(title)
                peer = Gtk.Label(label=f"on {hit.peer}")
                peer.set_xalign(0)
                peer.get_style_context().add_class("cds-helper-text-01")
                tile.pack(peer)
                url = Gtk.Label(label=url_for(hit))
                url.set_xalign(0)
                url.set_selectable(True)
                url.get_style_context().add_class("cds-code-01")
                tile.pack(url)
                self._grid.add(tile)
        self._grid.show_all()

        # Cheatsheet
        from mackes.mesh_services import cheatsheet_lines
        self._cheat.get_buffer().set_text("\n".join(cheatsheet_lines()))

    # ---- actions -------------------------------------------------------

    def _on_scan_now(self) -> None:
        peers = []
        try:
            peers = [p.name for p in __import__("mackes.mesh_vpn",
                                                fromlist=["headscale_list_peers"]
                                                ).headscale_list_peers()]
        except Exception:  # noqa: BLE001
            pass
        if not peers:
            # Fall back to whatever's mounted in ~/QNM-Mesh/
            from pathlib import Path
            import os
            home = Path(os.path.expanduser("~"))
            mesh_root = home / "QNM-Mesh"
            if mesh_root.exists():
                peers = [d.name for d in mesh_root.iterdir() if d.is_dir()]
        probe_all(peers)
        self._refresh()

    def _on_enable_gateway(self) -> None:
        from mackes.caddy_gateway import enable_gateway
        enable_gateway()
        self._refresh()

    def _on_install_ca(self) -> None:
        from mackes.caddy_gateway import install_ca_into_trust_store
        install_ca_into_trust_store()

    def _on_refresh_native_clients(self) -> None:
        from mackes.native_clients import refresh_all
        results = refresh_all()
        self._native_status.set_text("  ·  ".join(results)[:200])

    def _on_mdns_changed(self, _keys: list[str]) -> None:
        # Persist selected types; daemon picks them up on next loop
        from pathlib import Path
        import json
        from mackes.state import CONFIG_DIR
        path = CONFIG_DIR / "mdns-relay-types.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps({"relay": _keys}), encoding="utf-8")
