"""Native client autoconfig (§8.13 Layer 4).

Generates server-list configs for native media clients so every Jellyfin
peer and every Airsonic/Subsonic-compatible peer is pre-discovered.

  Jellyfin Media Player — ~/.local/share/jellyfinmediaplayer/servers.json
  Strawberry            — ~/.config/strawberry/servers.json (custom config)
"""
from __future__ import annotations

import json
import shutil
from pathlib import Path
from typing import Iterable

from mackes.logging import log_action
from mackes.mesh_services import ServiceHit, load_registry, url_for
from mackes.state import HOME


JELLYFIN_SERVERS = HOME / ".local" / "share" / "jellyfinmediaplayer" / "servers.json"
STRAWBERRY_CONFIG = HOME / ".config" / "strawberry" / "mackes-mesh-servers.json"


def _filter(hits: Iterable[ServiceHit], service: str) -> list[ServiceHit]:
    return [h for h in hits if h.service == service and h.online]


def write_jellyfin_servers() -> list[str]:
    actions: list[str] = []
    hits = _filter(load_registry(), "jellyfin")
    if not hits:
        return ["no Jellyfin instances discovered on mesh"]
    JELLYFIN_SERVERS.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "Servers": [
            {
                "Name":          f"Mackes Mesh — {h.peer}",
                "Address":       url_for(h),
                "DateLastAccessed": 0,
                "ManualAddress": url_for(h),
                "UserId":        "",
                "AccessToken":   "",
                "ServerId":      f"mackes-mesh-{h.peer}",
            }
            for h in hits
        ],
    }
    JELLYFIN_SERVERS.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    actions.append(f"wrote {JELLYFIN_SERVERS} ({len(hits)} server(s))")
    return actions


def write_strawberry_servers() -> list[str]:
    actions: list[str] = []
    hits = _filter(load_registry(), "airsonic") + _filter(load_registry(), "navidrome")
    if not hits:
        return ["no Airsonic/Navidrome instances discovered on mesh"]
    STRAWBERRY_CONFIG.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "servers": [
            {
                "name":     f"Mackes Mesh — {h.peer} ({h.service})",
                "url":      url_for(h),
                "service":  "subsonic",
                "username": "",
                "password": "",
            }
            for h in hits
        ],
    }
    STRAWBERRY_CONFIG.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    actions.append(f"wrote {STRAWBERRY_CONFIG} ({len(hits)} server(s))")
    return actions


def refresh_all() -> list[str]:
    """Re-generate every native-client config from the current registry."""
    actions = []
    actions.extend(write_jellyfin_servers())
    actions.extend(write_strawberry_servers())
    for a in actions:
        log_action(a)
    return actions


__all__ = ["write_jellyfin_servers", "write_strawberry_servers", "refresh_all"]
