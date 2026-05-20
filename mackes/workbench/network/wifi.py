"""Network → Wi-Fi & Ethernet (NetworkManager / nmcli backed)."""
from __future__ import annotations

import subprocess
from typing import Iterable

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk, GLib  # noqa: E402

from mackes.logging import log_action
from mackes.workbench._common import (
    a11y, error_state, format_probe_error,
    info_label, panel_box, section_description, section_header, title_label,
)


class _NmcliError(RuntimeError):
    """nmcli ran but exited non-zero — wrapped so the panel can distinguish
    a probe failure from a legitimately empty result. Phase 11.5."""


def _nmcli(*args: str, timeout: int = 8) -> str:
    try:
        return subprocess.check_output(["nmcli", *args], text=True, stderr=subprocess.STDOUT,
                                       timeout=timeout).strip()
    except (FileNotFoundError, subprocess.CalledProcessError,
            subprocess.TimeoutExpired) as exc:
        raise _NmcliError(format_probe_error(exc)) from exc


def _nmcli_or_blank(*args: str, timeout: int = 8) -> str:
    try:
        return _nmcli(*args, timeout=timeout)
    except _NmcliError:
        return ""


def _connections() -> list[dict[str, str]]:
    raw = _nmcli("-t", "-f", "NAME,TYPE,DEVICE,STATE", "connection", "show")
    out = []
    for line in raw.splitlines():
        parts = line.split(":")
        if len(parts) >= 4:
            out.append({"name": parts[0], "type": parts[1], "device": parts[2], "state": parts[3]})
    return out


def _wifi_scan() -> list[dict[str, str]]:
    raw = _nmcli("-t", "-f", "IN-USE,SSID,SIGNAL,SECURITY", "device", "wifi", "list")
    out = []
    for line in raw.splitlines():
        parts = line.split(":")
        if len(parts) >= 4 and parts[1]:
            out.append({"in_use": parts[0], "ssid": parts[1], "signal": parts[2],
                        "security": parts[3]})
    return out


class WifiPanel(Gtk.Box):
    def __init__(self) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self._build()

    def _build(self) -> None:
        box = panel_box()
        box.pack_start(title_label("Wi-Fi & Ethernet"), False, False, 0)
        box.pack_start(info_label(
            "Pick a Wi-Fi network to join, or see which wired or "
            "wireless connections are active right now."
        ), False, False, 0)
        box.pack_start(section_description(
            "Mackes uses your system's network manager under the hood. "
            "Changes here apply the moment you click."
        ), False, False, 0)

        if not _nmcli_or_blank("--version"):
            box.pack_start(error_state(
                "NetworkManager not available",
                "`nmcli` isn't installed or isn't on $PATH. Install "
                "NetworkManager (Maintain → Dependencies) and reopen "
                "this panel.",
                retry_label=None,
            ), True, True, 0)
            self.add(box); return

        # Active connections section
        box.pack_start(section_header("Active connections"), False, False, 0)
        self._conn_list = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        box.pack_start(self._conn_list, False, False, 0)

        # Wi-Fi networks section
        box.pack_start(section_header("Wi-Fi networks in range"), False, False, 0)
        self._scan_list = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        box.pack_start(self._scan_list, False, False, 0)

        refresh = Gtk.Button(label="Rescan")
        refresh.connect("clicked", lambda *_: self._async_refresh())
        a11y(refresh, name="Rescan for Wi-Fi networks",
             tooltip="Re-run nmcli device wifi list to refresh the network list")
        box.pack_start(refresh, False, False, 0)

        self._async_refresh()
        self.add(box)

    def _clear(self, container: Gtk.Box) -> None:
        for child in list(container.get_children()):
            container.remove(child)

    def _populate_connections(self, items: Iterable[dict[str, str]]) -> None:
        self._clear(self._conn_list)
        any_row = False
        for c in items:
            if not c["device"]:
                continue
            any_row = True
            row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
            label = Gtk.Label(label=f"{c['name']}  [{c['type']}]  on {c['device']}  ({c['state']})")
            label.set_xalign(0)
            row.pack_start(label, True, True, 0)
            disc = Gtk.Button(label="Disconnect")
            def _on_disc(_b, name=c["name"]):
                _nmcli("connection", "down", name)
                log_action(f"network: disconnected {name}")
                GLib.idle_add(self._refresh)
            disc.connect("clicked", _on_disc)
            a11y(disc, name=f"Disconnect from {c['name']}",
                 tooltip=f"Tear down the {c['name']} connection on {c['device']}")
            row.pack_end(disc, False, False, 0)
            self._conn_list.pack_start(row, False, False, 0)
        if not any_row:
            self._conn_list.pack_start(info_label("No active connections."), False, False, 0)
        self._conn_list.show_all()

    def _populate_scan(self, items: Iterable[dict[str, str]]) -> None:
        self._clear(self._scan_list)
        any_row = False
        for net in items:
            any_row = True
            row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
            star = "★ " if net["in_use"] == "*" else ""
            sec = net["security"] or "open"
            lbl = Gtk.Label(label=f"{star}{net['ssid']}   ({sec})   {net['signal']}%")
            lbl.set_xalign(0)
            row.pack_start(lbl, True, True, 0)
            connect = Gtk.Button(label="Connect")
            def _on_connect(_b, ssid=net["ssid"], secured=bool(net["security"])):
                self._connect_dialog(ssid, secured)
            connect.connect("clicked", _on_connect)
            sec_label = net["security"] or "open"
            a11y(connect, name=f"Connect to Wi-Fi network {net['ssid']} ({sec_label})",
                 tooltip=f"Join the {net['ssid']} network — {sec_label}, signal {net['signal']}%")
            row.pack_end(connect, False, False, 0)
            self._scan_list.pack_start(row, False, False, 0)
        if not any_row:
            self._scan_list.pack_start(info_label("No Wi-Fi networks visible."), False, False, 0)
        self._scan_list.show_all()

    def _connect_dialog(self, ssid: str, secured: bool) -> None:
        dialog = Gtk.Dialog(title=f"Connect to {ssid}", transient_for=self.get_toplevel(),
                            modal=True)
        dialog.add_button("Cancel", Gtk.ResponseType.CANCEL)
        dialog.add_button("Connect", Gtk.ResponseType.OK)
        content = dialog.get_content_area()
        content.set_margin_top(12); content.set_margin_bottom(12)
        content.set_margin_start(16); content.set_margin_end(16)
        pwd = Gtk.Entry()
        pwd.set_visibility(False)
        pwd.set_placeholder_text("Password")
        a11y(pwd, name=f"Password for Wi-Fi network {ssid}",
             tooltip="Password is hidden as you type")
        if secured:
            content.add(Gtk.Label(label="Password:"))
            content.add(pwd)
        else:
            content.add(Gtk.Label(label="This network is open."))
        dialog.show_all()
        if dialog.run() == Gtk.ResponseType.OK:
            args = ["device", "wifi", "connect", ssid]
            if secured:
                args += ["password", pwd.get_text()]
            result = _nmcli(*args)
            log_action(f"network: connect {ssid}: {result[:80]}")
            GLib.idle_add(self._refresh)
        dialog.destroy()

    def _async_refresh(self) -> None:
        """11.9: _connections + _wifi_scan each shell out to nmcli;
        together they exceed the 1 s budget. Off-main-thread."""
        from mackes.workbench._async import async_probe
        async_probe(
            lambda: (_connections(), _wifi_scan()),
            self._apply_refresh,
        )

    def _apply_refresh(self, gathered) -> None:
        connections, scan = gathered
        self._populate_connections(connections)
        self._populate_scan(scan)

    def _refresh(self) -> bool:
        # Back-compat shim: legacy callers (a GLib.idle_add closure
        # in the connect flow above) still call `self._refresh`. Route
        # through the async path.
        self._async_refresh()
        return False
