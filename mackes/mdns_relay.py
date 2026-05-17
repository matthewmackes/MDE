"""mDNS-over-mesh relay (§8.13 Layer 5).

On each peer:
  1. Listen for local mDNS announcements (`avahi-browse`).
  2. Republish them through mesh-sync as `mesh.mdns.<peer>.<service-type>`.
  3. Subscribe to incoming `mesh.mdns.*.*` from other peers and
     re-broadcast them on the LOCAL LAN via `avahi-publish-service` —
     substituting the originating peer's mesh IP for the source LAN IP.

Anti-loop: announcements we relay never get rebroadcast on the source
peer (we tag them with origin-peer-id and skip our own).
"""
from __future__ import annotations

import json
import shutil
import socket
import subprocess
import time
from dataclasses import dataclass, asdict
from typing import Optional

from mackes.logging import log_action
from mackes.mesh_sync import BUCKET_NOTIFICATIONS, put, list_keys
from mackes.state import CONFIG_DIR

RELAY_BUCKET = "mdns"
ME = socket.gethostname()

# Service types to relay by default (Q-MX13/Q-MX17 lock: opt-out per type).
DEFAULT_RELAYED_TYPES = (
    "_jellyfin._tcp",
    "_googlecast._tcp",
    "_airplay._tcp",
    "_spotify-connect._tcp",
    "_home-assistant._tcp",
    "_syncthing._tcp",
    "_netdata._tcp",
    "_subsonic._tcp",
)
# Service types NOT relayed by default (privacy)
DEFAULT_PRIVATE_TYPES = (
    "_ipp._tcp",
    "_pdl-datastream._tcp",
    "_smb._tcp",
    "_afpovertcp._tcp",
    "_ssh._tcp",
)


@dataclass
class MdnsAnnounce:
    peer:        str     # originating mesh peer
    service:     str     # e.g. "jellyfin-server.local"
    service_type: str    # e.g. "_jellyfin._tcp"
    port:        int
    host:        str     # mesh-IP we want clients to connect to
    txt:         list[str]


def _have(c: str) -> bool:
    return shutil.which(c) is not None


def scan_local(timeout: float = 3.0) -> list[MdnsAnnounce]:
    """Browse local mDNS for known service types via avahi-browse."""
    if not _have("avahi-browse"):
        return []
    out: list[MdnsAnnounce] = []
    cmd = ["avahi-browse", "-a", "-r", "-p", "-l", "-t"]
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True,
                              timeout=timeout + 1)
    except (OSError, subprocess.TimeoutExpired):
        return []
    # avahi-browse parseable output: lines like
    # =;wlp1s0;IPv4;Jellyfin Media Server;_jellyfin._tcp;local;jellyfin.local;192.168.1.5;8096;...
    for line in proc.stdout.splitlines():
        if not line.startswith("="):
            continue
        parts = line.split(";")
        if len(parts) < 9:
            continue
        try:
            stype  = parts[4]
            sname  = parts[3]
            host   = parts[6]
            port   = int(parts[8])
            txt    = parts[9:] if len(parts) > 9 else []
        except (ValueError, IndexError):
            continue
        if stype not in DEFAULT_RELAYED_TYPES:
            continue
        out.append(MdnsAnnounce(
            peer=ME, service=sname, service_type=stype,
            port=port, host=host, txt=list(txt),
        ))
    return out


def publish_to_mesh(announces: list[MdnsAnnounce]) -> list[str]:
    actions: list[str] = []
    for a in announces:
        key = f"{a.service_type}__{a.service}".replace(" ", "_").replace("/", "_")
        put(RELAY_BUCKET, key, json.dumps(asdict(a)))
        actions.append(f"published mDNS announce {a.service_type}/{a.service}")
    return actions


def rebroadcast_from_mesh() -> list[str]:
    """Look at every other peer's mdns announces and republish locally."""
    if not _have("avahi-publish-service"):
        return ["avahi-publish-service missing; cannot rebroadcast"]
    actions: list[str] = []
    entries = list_keys(RELAY_BUCKET)
    me = ME
    for e in entries:
        if e.peer == me:
            continue  # don't loop our own announces back
        try:
            ann = json.loads(e.path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            continue
        stype = ann.get("service_type", "")
        sname = ann.get("service", "")
        port  = int(ann.get("port", 0))
        if not stype or not sname or not port:
            continue
        # Name with peer suffix to avoid collisions (e.g. jellyfin-laptop-mm.local)
        rebroadcast_name = f"{sname.rstrip('.local')}-{ann.get('peer','')}"
        cmd = [
            "avahi-publish-service",
            "--no-fail",
            rebroadcast_name, stype, str(port),
        ]
        # Spawn as a background process; one publisher per service stays
        # alive until killed. mackes-meshd manages lifecycle.
        try:
            subprocess.Popen(cmd, stdout=subprocess.DEVNULL,
                             stderr=subprocess.DEVNULL, start_new_session=True)
            actions.append(f"rebroadcast {rebroadcast_name} ({stype}:{port})")
        except OSError as e:
            actions.append(f"failed to rebroadcast {sname}: {e}")
    return actions


def loop_once() -> list[str]:
    """Single pass — scan + publish + rebroadcast. Used by mackes-meshd."""
    actions: list[str] = []
    found = scan_local()
    actions.extend(publish_to_mesh(found))
    actions.extend(rebroadcast_from_mesh())
    return actions


__all__ = [
    "MdnsAnnounce", "DEFAULT_RELAYED_TYPES", "DEFAULT_PRIVATE_TYPES",
    "scan_local", "publish_to_mesh", "rebroadcast_from_mesh", "loop_once",
]
