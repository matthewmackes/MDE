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


# ─────────────────────────────────────────────────────────────────
# NF-16 notification emitters (toasts → ~/.cache/mde/toasts.jsonl)
# ─────────────────────────────────────────────────────────────────
#
# The Iced toast applet at crates/mde-popover/src/toasts.rs tails
# ~/.cache/mde/toasts.jsonl + stacks up to STACK_LIMIT=3 toasts
# above the panel. These emitters write JSON-line events; the
# applet's `Kind` field maps to ToastKind { Info, Success, Warn,
# Error }. Failures are best-effort — a missing cache dir or
# disk-full doesn't crash the caller.

import json


TOASTS_PATH = Path(
    os.environ.get("XDG_CACHE_HOME") or os.path.expanduser("~/.cache")
) / "mde" / "toasts.jsonl"


def _emit_toast(kind: str, title: str, body: str = "",
                visible_ms: int = 5000) -> bool:
    """Append one JSON line to the toast stream. Returns True on
    success, False on filesystem error (caller is expected to
    treat the toast as best-effort).
    """
    try:
        TOASTS_PATH.parent.mkdir(parents=True, exist_ok=True)
        with TOASTS_PATH.open("a", encoding="utf-8") as fh:
            fh.write(json.dumps({
                "kind": kind,
                "title": title,
                "body": body,
                "visible_ms": visible_ms,
                "created_at": int(__import__("time").time()),
            }) + "\n")
        return True
    except OSError:
        return False


def emit_lighthouse_event(promoted: bool) -> bool:
    """NF-16.1 — subtle informational toast on
    promotion/demotion to/from the lighthouse role.
    """
    if promoted:
        return _emit_toast(
            "info",
            "Lighthouse active",
            "This peer is now serving as a lighthouse for the mesh.",
        )
    return _emit_toast(
        "info",
        "Lighthouse stepped down",
        "This peer is no longer serving as a lighthouse.",
    )


def emit_ca_rotation(success: bool, error_detail: str = "") -> bool:
    """NF-16.2 — per-peer toast when the mesh CA rotates.
    Success path: info toast confirming the new cert
    propagated. Failure path: error toast pointing to the
    recovery doc.
    """
    if success:
        return _emit_toast(
            "info",
            "Mesh CA rotated",
            "Your peer cert was re-issued under the new CA epoch.",
        )
    body = "See docs/help/mesh-recovery.md for recovery steps."
    if error_detail:
        body = f"{error_detail}\n\n{body}"
    return _emit_toast("error", "CA rotation failed", body)


def emit_https_fallback_state(active: bool) -> bool:
    """NF-16.3 — transition-only event when the TCP/443
    fallback flips Active / Inactive. Honors the Q12 lock:
    transition event, not a persistent banner.
    """
    if active:
        return _emit_toast(
            "warn",
            "Mesh in firewall mode",
            "UDP path lost — falling over to TCP/443 (covert tunnel).",
        )
    return _emit_toast(
        "info",
        "Direct UDP mesh restored",
        "Covert TCP/443 fallback stood down.",
    )


def emit_cert_expiry_warning(peer_name: str, days_remaining: int) -> bool:
    """NF-16.4 — early warning when a peer's cert is approaching
    expiry. < 24 h escalates to error severity (the consumer
    paints it as a persistent banner); 1-7 days is a warn.
    """
    if days_remaining < 1:
        return _emit_toast(
            "error",
            f"{peer_name} cert expired",
            "Re-enroll the peer or rotate the CA to restore reachability.",
            visible_ms=0,  # 0 = persistent in the applet's convention
        )
    if days_remaining <= 7:
        return _emit_toast(
            "warn",
            f"{peer_name} cert expires in {days_remaining}d",
            "Plan a CA rotation or peer re-enrollment soon.",
        )
    # Already > 7 days — don't toast.
    return False


# ─────────────────────────────────────────────────────────────────
# NF-17 firewall + system surface
# ─────────────────────────────────────────────────────────────────

NEBULA_FIREWALL_PORTS: tuple[tuple[int, str], ...] = (
    (4242, "udp"),  # native Nebula
    (443, "tcp"),   # NF-1 covert TCP/443 fallback
)


def apply_nebula_firewall_preset() -> int:
    """NF-17.1 — one-click "Allow Nebula" firewalld preset.
    Opens UDP/4242 inbound + outbound and TCP/443 outbound
    on the default zone. Returns 0 on success; non-zero on
    firewall-cmd failure (caller surfaces a toast).

    Tailscale's UDP/41641 preset (the v1.x default) is NOT
    cleaned up here — leave existing rules alone so a peer
    migrating from Tailscale doesn't lose connectivity
    mid-flight. The mackesd cleanup pass retires the
    Tailscale preset in NF-6.x once the operator confirms
    the migration succeeded.
    """
    if shutil.which("firewall-cmd") is None:
        return 1
    rc = 0
    for port, proto in NEBULA_FIREWALL_PORTS:
        spec = f"{port}/{proto}"
        rc |= subprocess.call(
            ["firewall-cmd", "--permanent", "--add-port", spec],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    rc |= subprocess.call(
        ["firewall-cmd", "--reload"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    return rc


__all__ = [
    "CANONICAL_SERVICES",
    "CONFIG_DIR",
    "HOST_CERT_PATH",
    "LIGHTHOUSE_CONFIG_PATH",
    "SSHD_DROPIN_DIR",
    "SSHD_DROPIN_PATH",
    "TOASTS_PATH",
    "current_overlay_ip",
    "emit_ca_rotation",
    "emit_cert_expiry_warning",
    "emit_https_fallback_state",
    "emit_lighthouse_event",
    "lighthouse_addresses",
    "published_services_summary",
    "reload_sshd",
    "wol_via_lighthouse",
    "write_sshd_overlay_bind",
    "_extract_lighthouse_hosts",
    "NEBULA_FIREWALL_PORTS",
    "apply_nebula_firewall_preset",
]
