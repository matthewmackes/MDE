"""NF-5.3 + NF-13 (v2.5) — thin read/configure wrapper around Nebula.

Python-side counterpart to the Rust `mackes-nebula-https-tunnel`
+ `mackesd::ca` modules. NO privileged operations live here —
enrollment, cert rotation, lighthouse promotion all route
through `mded`'s D-Bus surface (`dev.mackes.MDE.Nebula.Status`).
This module is the consumer side: read overlay state, write
sshd config snippets, emit WoL via the lighthouse relay.

Per the open-mesh / flat-trust directive (2026-05-23), every
service on a paired peer is reachable from every other peer.
The helpers here don't introduce ACLs.
"""
from __future__ import annotations

import os
import shutil
import socket
import subprocess
from pathlib import Path
from typing import Iterable, Optional


# ─────────────────────────────────────────────────────────────────
# Canonical paths
# ─────────────────────────────────────────────────────────────────

CONFIG_DIR = Path("/etc/nebula")
HOST_CERT_PATH = CONFIG_DIR / "host.crt"
LIGHTHOUSE_CONFIG_PATH = CONFIG_DIR / "lighthouse-config.yaml"
SSHD_DROPIN_DIR = Path("/etc/ssh/sshd_config.d")
SSHD_DROPIN_PATH = SSHD_DROPIN_DIR / "mackes-mesh.conf"


# ─────────────────────────────────────────────────────────────────
# Read helpers (no privilege required)
# ─────────────────────────────────────────────────────────────────

def current_overlay_ip(host_cert_path: Optional[Path] = None) -> Optional[str]:
    """NF-5.3 / NF-13.1 — return this peer's allocated overlay IP
    (e.g. "10.42.0.5") read from the nebula host cert.

    Implementation: shells out to `nebula-cert print -path <crt>`
    + greps the "Ips:" line. Returns None when nebula-cert isn't
    on PATH (dev boxes without the Fedora `nebula` package) or
    when the cert doesn't exist (pre-enrollment).
    """
    path = host_cert_path or HOST_CERT_PATH
    if not path.exists():
        return None
    if shutil.which("nebula-cert") is None:
        return None
    try:
        out = subprocess.run(
            ["nebula-cert", "print", "-path", str(path)],
            capture_output=True,
            text=True,
            timeout=2,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if out.returncode != 0:
        return None
    for line in out.stdout.splitlines():
        if "Ips:" in line:
            # Lines look like "Ips: [10.42.0.5/16]"
            body = line.split("Ips:", 1)[1].strip()
            body = body.strip("[]")
            ip_with_mask = body.split(",")[0].strip()
            ip = ip_with_mask.split("/")[0]
            if ip:
                return ip
    return None


def lighthouse_addresses(
    lighthouse_config_path: Optional[Path] = None,
) -> list[str]:
    """NF-13.6 — return the list of lighthouse overlay IPs from
    the local nebula config. Empty list when no config exists.
    Pure read.
    """
    path = lighthouse_config_path or LIGHTHOUSE_CONFIG_PATH
    if not path.exists():
        # Try the regular config (peer-role) instead.
        alt = CONFIG_DIR / "config.yaml"
        if not alt.exists():
            return []
        path = alt
    try:
        body = path.read_text()
    except OSError:
        return []
    return _extract_lighthouse_hosts(body)


def _extract_lighthouse_hosts(yaml_body: str) -> list[str]:
    """Pure helper — pulls IPv4 strings from the
    `lighthouse.hosts:` block of a nebula config YAML.

    Not a full YAML parser; tolerates the shape the
    nebula_supervisor's `render_config_yaml` emits:

        lighthouse:
          am_lighthouse: false
          hosts:
            - "10.42.0.1"
            - "10.42.0.2"
    """
    out: list[str] = []
    inside_hosts = False
    for raw in yaml_body.splitlines():
        line = raw.rstrip()
        if line.startswith("lighthouse:"):
            inside_hosts = False
            continue
        if "hosts:" in line and inside_hosts is False:
            inside_hosts = True
            continue
        if inside_hosts:
            stripped = line.strip()
            if stripped.startswith("- "):
                ip = stripped[2:].strip().strip('"').strip("'")
                if ip:
                    out.append(ip)
            elif stripped and not stripped.startswith("-"):
                # Left the hosts list — next key starts.
                inside_hosts = False
    return out


# ─────────────────────────────────────────────────────────────────
# Write helpers (privileged, expect pkexec / root)
# ─────────────────────────────────────────────────────────────────

def write_sshd_overlay_bind(
    overlay_ip: str,
    dropin_path: Optional[Path] = None,
) -> Path:
    """NF-13.1 — write the sshd_config.d drop-in that binds the
    SSH daemon to the nebula overlay IP. Replaces any existing
    file with identical contents (idempotent — the supervisor
    re-runs this on every overlay-IP change, which is rare;
    only on re-enrollment under a new CA epoch).

    Returns the path written.

    Raises OSError on permission failure (caller is expected to
    invoke under pkexec / inside a privileged systemd unit).
    """
    path = dropin_path or SSHD_DROPIN_PATH
    path.parent.mkdir(parents=True, exist_ok=True)
    body = (
        "# Generated by mackes/mesh_nebula.py (NF-13.1)\n"
        "# Do not edit by hand — the supervisor rewrites this\n"
        "# on every overlay-IP change.\n"
        f"ListenAddress {overlay_ip}\n"
        "# Per the open-mesh directive (2026-05-23): every\n"
        "# enrolled peer sees every other on the overlay; no\n"
        "# per-service ACL splits here.\n"
    )
    # Atomic write via temp + rename so a sshd reload mid-write
    # doesn't see a half-formed config.
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(body)
    tmp.replace(path)
    return path


def reload_sshd() -> int:
    """NF-13.1 — best-effort `systemctl reload sshd` after a
    drop-in change. Returns the exit code; 0 on success. Non-
    zero exits get swallowed by the caller (the supervisor) so
    a failed reload doesn't kill the worker.
    """
    if shutil.which("systemctl") is None:
        return 1
    return subprocess.call(["systemctl", "reload", "sshd"])


# ─────────────────────────────────────────────────────────────────
# WoL via lighthouse relay (NF-13.6 — new capability)
# ─────────────────────────────────────────────────────────────────

def wol_via_lighthouse(
    target_mac: str,
    lighthouse_ip: Optional[str] = None,
) -> int:
    """NF-13.6 — wake the peer with `target_mac` by sending the
    magic packet over the nebula overlay to a lighthouse, which
    de-encapsulates + re-broadcasts on the target's LAN segment.

    This is the new "WoL across LANs" capability the v2.5 cut
    enables — pre-Nebula, WoL only worked within a single
    broadcast domain.

    Implementation: shells out to `wakeonlan` (the canonical
    Fedora WoL utility) targeting the lighthouse's overlay IP.
    The lighthouse-side relay re-broadcasts on the target LAN
    via the static_host_map cached MAC address.

    Returns the wakeonlan exit code; 0 on success. Returns 2
    when no lighthouse can be reached (no IPs in
    `lighthouse_addresses()` and no override supplied).
    """
    if lighthouse_ip is None:
        candidates = lighthouse_addresses()
        if not candidates:
            return 2
        lighthouse_ip = candidates[0]
    if shutil.which("wakeonlan") is None:
        return 3
    return subprocess.call(
        ["wakeonlan", "-i", lighthouse_ip, target_mac]
    )


# ─────────────────────────────────────────────────────────────────
# Read-only status query for the panel / workbench
# ─────────────────────────────────────────────────────────────────

CANONICAL_SERVICES: tuple[tuple[str, str, int, str], ...] = (
    # (service-id, display-name, default-port, "tcp"|"udp")
    ("ssh", "SSH", 22, "tcp"),
    ("nats", "NATS broker", 4222, "tcp"),
    ("fs", "Mesh FS (SSHFS)", 22, "tcp"),
    ("media", "Media library", 8080, "tcp"),
    ("sync", "rsync", 873, "tcp"),
    ("wol", "Wake-on-LAN relay", 9, "udp"),
    ("av", "Audio/video transport", 5004, "udp"),
)


def published_services_summary() -> list[dict]:
    """NF-13.8 — return one row per canonical service for the
    new "Service Publishing" workbench panel. Each row carries
    the service id, display name, port + protocol, the overlay
    IP it would bind to (None when not yet enrolled), and a
    bench-observable "is_publishable" flag (true when both an
    overlay IP exists AND the service binary is on PATH).

    Pure read — no side effects.
    """
    overlay = current_overlay_ip()
    out: list[dict] = []
    for service_id, display, port, proto in CANONICAL_SERVICES:
        out.append({
            "id": service_id,
            "name": display,
            "port": port,
            "proto": proto,
            "overlay_ip": overlay,
            "is_publishable": overlay is not None,
        })
    return out


__all__ = [
    "CANONICAL_SERVICES",
    "CONFIG_DIR",
    "HOST_CERT_PATH",
    "LIGHTHOUSE_CONFIG_PATH",
    "SSHD_DROPIN_DIR",
    "SSHD_DROPIN_PATH",
    "current_overlay_ip",
    "lighthouse_addresses",
    "published_services_summary",
    "reload_sshd",
    "wol_via_lighthouse",
    "write_sshd_overlay_bind",
    "_extract_lighthouse_hosts",
]
