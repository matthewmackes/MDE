"""`mackes daemon` — long-running mesh-node supervisor.

This is the process that systemd's `mackes-node.service` starts. It owns:

  - Periodic mesh-VPN heartbeat + election (every 30s)
  - Mesh-FS mount/unmount sync (every 60s)
  - Mesh-services port-probe (every 60s)
  - Mesh-SSH authorized_keys sync (every 60s)
  - Mesh-notifications inbox watch (every 5s)
  - mDNS relay loop (every 30s)
  - Mesh-VPN snapshot (every 30s)

All loops run in a single Python thread on simple time slices — no
threading needed for a workstation-scale mesh (≤16 peers, modest event
rate).
"""
from __future__ import annotations

import signal
import sys
import time
from typing import Optional

from mackes.logging import log_action
from mackes.state import ensure_dirs


_RUNNING = True


def _sigterm(_signum, _frame) -> None:
    global _RUNNING
    _RUNNING = False


def _safe(label: str, fn) -> None:
    try:
        result = fn()
    except Exception as e:  # noqa: BLE001
        log_action(f"daemon: {label} crashed: {e}")
        return
    if isinstance(result, list):
        for line in result:
            log_action(f"daemon: {label}: {line}")


def run() -> int:
    ensure_dirs()
    signal.signal(signal.SIGTERM, _sigterm)
    signal.signal(signal.SIGINT,  _sigterm)
    log_action("mackes daemon: starting")

    # Notify systemd we're up
    try:
        import socket
        notify = (
            None
            if "NOTIFY_SOCKET" not in __import__("os").environ
            else __import__("os").environ["NOTIFY_SOCKET"]
        )
        if notify:
            with socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM) as s:
                s.sendto(b"READY=1", notify)
    except Exception:  # noqa: BLE001
        pass

    last_30s = 0.0
    last_60s = 0.0
    last_5s = 0.0

    while _RUNNING:
        now = time.monotonic()

        # 5s: notification inbox
        if now - last_5s >= 5:
            from mackes.mesh_notifications import receive_loop_once
            _safe("notif_inbox", receive_loop_once)
            last_5s = now

        # 30s: vpn election + snapshot + mdns relay
        if now - last_30s >= 30:
            from mackes.mesh_vpn import maybe_take_control, snapshot_state
            from mackes.mdns_relay import loop_once as mdns_loop_once
            _safe("election", maybe_take_control)
            _safe("snapshot", snapshot_state)
            _safe("mdns_relay", mdns_loop_once)
            last_30s = now

        # 60s: mesh-fs mount sync + service probe + ssh-keys sync
        if now - last_60s >= 60:
            try:
                from mackes.mesh_vpn import headscale_list_peers
                peer_names = [p.name for p in headscale_list_peers() if p.online]
            except Exception:  # noqa: BLE001
                peer_names = []
            from mackes.mesh_fs import sync_mounts
            from mackes.mesh_services import probe_all
            from mackes.mesh_ssh import sync_authorized_keys
            from mackes.native_clients import refresh_all
            _safe("mount_sync",  lambda: sync_mounts(peer_names))
            _safe("probe_svc",   lambda: probe_all(peer_names))
            _safe("ssh_sync",    sync_authorized_keys)
            _safe("native_cli",  refresh_all)
            last_60s = now

        time.sleep(1)

    log_action("mackes daemon: stopping")
    return 0


if __name__ == "__main__":
    sys.exit(run())
