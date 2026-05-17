"""Mesh Services — §8.13 catalog loader + port-probe scanner.

Reads `data/media-services.yaml` from the shipped tree + user overrides
from `~/.config/mackes-shell/media-services.yaml`. Scans every reachable
mesh peer for known service ports + their HTTP path. Publishes the
discovery matrix as a local registry (read by the Media Hub panel) and
optionally pushes to NATS `mesh.services` for cross-peer sharing.
"""
from __future__ import annotations

import json
import socket
import subprocess
import time
import urllib.error
import urllib.request
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Iterable, Optional

from mackes.logging import log_action
from mackes.state import CONFIG_DIR, DATA_DIR

try:
    import yaml   # type: ignore
except ImportError:
    yaml = None   # noqa: N816


CATALOG_PATHS = [
    Path("/usr/share/mackes-shell/data/media-services.yaml"),
    Path(__file__).resolve().parent.parent / "data" / "media-services.yaml",
]
USER_CATALOG = CONFIG_DIR / "media-services.yaml"
REGISTRY_PATH = DATA_DIR / "mesh-services.json"


@dataclass
class ServiceDef:
    name:           str
    display:        str
    category:       str
    port:           Optional[int] = None
    https_port:     Optional[int] = None
    path:           str = "/"
    icon:           str = ""
    mdns_type:      str = ""
    native_client:  str = ""
    description:    str = ""


@dataclass
class ServiceHit:
    """A live (peer, service) tuple in the registry."""
    peer:       str
    service:    str
    port:       int
    scheme:     str = "http"
    path:       str = "/"
    online:     bool = True
    last_probe: float = 0.0


def _load_yaml(path: Path) -> list[dict]:
    if yaml is None or not path.exists():
        return []
    try:
        data = yaml.safe_load(path.read_text(encoding="utf-8")) or {}
    except (OSError, yaml.YAMLError):
        return []
    return data.get("services") or []


def load_catalog() -> list[ServiceDef]:
    """Load the shipped catalog and overlay user overrides."""
    shipped: list[dict] = []
    for cand in CATALOG_PATHS:
        if cand.exists():
            shipped = _load_yaml(cand)
            break
    user = _load_yaml(USER_CATALOG)
    merged: dict[str, dict] = {}
    for d in shipped:
        merged[d.get("name", "")] = dict(d)
    for d in user:
        merged[d.get("name", "")] = dict(d)
    defs: list[ServiceDef] = []
    for name, d in merged.items():
        if not name:
            continue
        defs.append(ServiceDef(
            name=name,
            display=d.get("display", name),
            category=d.get("category", "media"),
            port=d.get("port"),
            https_port=d.get("https-port") or d.get("https_port"),
            path=d.get("path", "/"),
            icon=d.get("icon", ""),
            mdns_type=d.get("mdns-type", "") or d.get("mdns_type", ""),
            native_client=d.get("native-client", "") or d.get("native_client", ""),
            description=d.get("description", ""),
        ))
    return defs


# ---------------------------------------------------------------------------
# Probe a single (peer, service)
# ---------------------------------------------------------------------------


def _probe_tcp(host: str, port: int, timeout: float = 1.5) -> bool:
    try:
        with socket.create_connection((host, port), timeout=timeout):
            return True
    except (OSError, socket.timeout):
        return False


def _probe_http(host: str, port: int, path: str, *, scheme: str = "http",
                timeout: float = 2.0) -> bool:
    url = f"{scheme}://{host}:{port}{path}"
    try:
        req = urllib.request.Request(url, method="HEAD")
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return 200 <= resp.status < 500
    except (urllib.error.URLError, OSError, ValueError):
        # HEAD may not be supported; try TCP-only as a sanity check
        return _probe_tcp(host, port, timeout=timeout)


def probe_service(peer_host: str, svc: ServiceDef) -> Optional[ServiceHit]:
    """Probe one (peer, service) pair. Returns a ServiceHit or None."""
    if svc.port is None and not svc.https_port:
        return None
    # Try HTTPS first if available
    if svc.https_port and _probe_tcp(peer_host, svc.https_port, timeout=1.0):
        if _probe_http(peer_host, svc.https_port, svc.path, scheme="https"):
            return ServiceHit(peer=peer_host, service=svc.name,
                              port=svc.https_port, scheme="https", path=svc.path,
                              online=True, last_probe=time.time())
    if svc.port and _probe_tcp(peer_host, svc.port, timeout=1.0):
        if _probe_http(peer_host, svc.port, svc.path, scheme="http"):
            return ServiceHit(peer=peer_host, service=svc.name,
                              port=svc.port, scheme="http", path=svc.path,
                              online=True, last_probe=time.time())
    return None


def probe_all(peers: Iterable[str]) -> list[ServiceHit]:
    """Scan every (peer, service) combination from the catalog."""
    catalog = load_catalog()
    hits: list[ServiceHit] = []
    for peer in peers:
        for svc in catalog:
            hit = probe_service(peer, svc)
            if hit is not None:
                hits.append(hit)
    REGISTRY_PATH.parent.mkdir(parents=True, exist_ok=True)
    REGISTRY_PATH.write_text(
        json.dumps([asdict(h) for h in hits], indent=2),
        encoding="utf-8",
    )
    return hits


def load_registry() -> list[ServiceHit]:
    """Read the last-published registry without doing a fresh probe."""
    if not REGISTRY_PATH.exists():
        return []
    try:
        data = json.loads(REGISTRY_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return []
    return [ServiceHit(**d) for d in data]


def url_for(hit: ServiceHit) -> str:
    """Best-effort URL for a service hit."""
    suffix = ".mesh" if not hit.peer.endswith(".mesh") and "." not in hit.peer else ""
    host = f"{hit.peer}{suffix}"
    return f"{hit.scheme}://{host}:{hit.port}{hit.path}"


def launch(hit: ServiceHit) -> list[str]:
    """xdg-open the service URL."""
    url = url_for(hit)
    if not subprocess.run(["which", "xdg-open"], capture_output=True).returncode == 0:
        return [f"xdg-open missing; visit {url}"]
    subprocess.Popen(["xdg-open", url], stdout=subprocess.DEVNULL,
                     stderr=subprocess.DEVNULL)
    return [f"launched {url}"]


def cheatsheet_lines() -> list[str]:
    """Plain-text cheatsheet of every URL in the registry (Layer 1)."""
    hits = load_registry()
    if not hits:
        return ["(no services discovered yet — run `mackes services list`)"]
    return [url_for(h) for h in hits]


__all__ = [
    "ServiceDef", "ServiceHit",
    "load_catalog", "load_registry", "probe_service", "probe_all",
    "url_for", "launch", "cheatsheet_lines",
]
