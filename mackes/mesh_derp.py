"""Private DERP relay for the mesh (#1).

DERP (Designated Encrypted Relay for Packets) is Tailscale's relay
protocol — when two peers can't NAT-traverse to each other (CGNAT,
double-NAT, restrictive firewalls), packets flow through a DERP
server instead. By default tailscale uses Tailscale's public DERP
network. We can stand up our OWN DERP on the control peer for:

  * Lower latency (LAN-local vs cross-continent)
  * No third-party dependency
  * Tighter QoS / no shared-relay starvation

Upstream open-source code:
  github.com/tailscale/tailscale/cmd/derper (BSD-3-Clause)

We don't redistribute the binary — `apply_derper` builds it from
source (requires Go toolchain) OR pulls a pre-built static binary
from our release artifacts on first install.

Public API:

  is_installed()      → bool
  is_running()        → bool
  install()           → list[str]   (compiles/downloads + systemd unit)
  start() / stop()
  status()            → dict
  patch_headscale_derp_map(domain: str, public_ip: str)
"""
from __future__ import annotations

import json
import shutil
import socket
import subprocess
from pathlib import Path
from typing import Optional


DERPER_BIN = Path("/usr/local/bin/derper")
DERPER_UNIT = Path("/etc/systemd/system/mackes-derper.service")
DERPER_DATA = Path("/var/lib/mackes-derper")
DERPER_PORT = 3478           # standard DERP port (HTTPS)
DERPER_STUN_PORT = 3478      # STUN co-resides on the same UDP port
DERPER_HTTP_PORT = 80        # ACME challenge port


# ---------------------------------------------------------------------------
# Capability probes
# ---------------------------------------------------------------------------


def is_installed() -> bool:
    return DERPER_BIN.is_file() and DERPER_BIN.stat().st_mode & 0o100


def is_running() -> bool:
    if shutil.which("systemctl") is None:
        return False
    try:
        r = subprocess.run(
            ["systemctl", "is-active", "mackes-derper"],
            capture_output=True, text=True, timeout=4,
        )
        return r.returncode == 0 and r.stdout.strip() == "active"
    except (OSError, subprocess.TimeoutExpired):
        return False


# ---------------------------------------------------------------------------
# Install (compile-from-source, requires `go`)
# ---------------------------------------------------------------------------


def install(*, hostname: str, public_ip: str = "") -> list[str]:
    """Install derper and start the systemd unit.

    Strategy:
      1. If `go` is available → `go install
         tailscale.com/cmd/derper@latest`, then move the binary.
      2. Otherwise advise the user to install golang or fall back to
         Tailscale's public DERP.

    `hostname` should be a DNS name we can prove ownership of (Let's
    Encrypt). For local-only mesh, derper accepts `-c manual` mode
    with a self-signed cert — set hostname to the LAN IP and
    accept the SSL warning.
    """
    from mackes.admin_session import AdminSession
    actions: list[str] = []

    if not is_installed():
        if shutil.which("go") is None:
            return ["derper: go toolchain not installed — "
                    "`dnf install golang` then re-run"]
        # Build into a tmp GOPATH so we don't pollute the user's
        import tempfile
        with tempfile.TemporaryDirectory(prefix="mackes-derper-") as td:
            env = {"GOPATH": td, "GOBIN": td + "/bin",
                   "PATH": "/usr/bin:/usr/local/bin",
                   "HOME": str(Path.home())}
            r = subprocess.run(
                ["go", "install", "tailscale.com/cmd/derper@latest"],
                capture_output=True, text=True, timeout=600, env=env,
            )
            if r.returncode != 0:
                return [f"derper build failed: {r.stderr.strip()[:200]}"]
            built = Path(td) / "bin" / "derper"
            if not built.is_file():
                return ["derper build produced no binary"]
            rc, out = AdminSession.instance().run(
                ["install", "-D", "-m", "0755", str(built),
                 str(DERPER_BIN)], timeout=10,
            )
            if rc != 0:
                return [f"derper install failed: {out}"]
            actions.append(f"derper: built + installed → {DERPER_BIN}")
    else:
        actions.append(f"derper: {DERPER_BIN} already present")

    # systemd unit
    unit_text = _unit_payload(hostname=hostname)
    import tempfile
    with tempfile.NamedTemporaryFile(mode="w", delete=False,
                                      suffix=".service",
                                      encoding="utf-8") as tmp:
        tmp.write(unit_text)
        tmp_path = tmp.name
    rc, _ = AdminSession.instance().run(
        ["install", "-D", "-m", "0644", tmp_path, str(DERPER_UNIT)],
        timeout=10,
    )
    Path(tmp_path).unlink(missing_ok=True)
    if rc != 0:
        return actions + [f"derper: unit install failed (rc={rc})"]
    actions.append(f"derper: wrote {DERPER_UNIT}")
    rc, _ = AdminSession.instance().run(
        ["systemctl", "daemon-reload"], timeout=5)
    rc, out = AdminSession.instance().run(
        ["systemctl", "enable", "--now", "mackes-derper.service"],
        timeout=10,
    )
    if rc == 0:
        actions.append("derper: enabled + started")
    else:
        actions.append(f"derper: enable failed: {out.strip()}")
    return actions


def _unit_payload(*, hostname: str) -> str:
    return f"""[Unit]
Description=Mackes private DERP relay
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
# Local-development mode: -c /var/lib/mackes-derper/derper.key generates
# a self-signed cert on first run. For a public DERP set hostname to
# the real DNS name and add -certmode letsencrypt.
ExecStart={DERPER_BIN} \\
    -hostname {hostname} \\
    -a :{DERPER_PORT} \\
    -stun \\
    -stun-port {DERPER_STUN_PORT} \\
    -http-port {DERPER_HTTP_PORT} \\
    -c {DERPER_DATA}/derper.key
WorkingDirectory={DERPER_DATA}
Restart=on-failure
RestartSec=5
StateDirectory=mackes-derper
ProtectSystem=strict
NoNewPrivileges=true

[Install]
WantedBy=multi-user.target
"""


def stop() -> list[str]:
    from mackes.admin_session import AdminSession
    rc, out = AdminSession.instance().run(
        ["systemctl", "disable", "--now", "mackes-derper.service"],
        timeout=10,
    )
    return [f"derper: stopped (rc={rc}) {out.strip().splitlines()[-1] if out else ''}"]


def start() -> list[str]:
    from mackes.admin_session import AdminSession
    rc, out = AdminSession.instance().run(
        ["systemctl", "enable", "--now", "mackes-derper.service"],
        timeout=10,
    )
    return [f"derper: started (rc={rc})"]


def status() -> dict:
    return {
        "installed": is_installed(),
        "running":   is_running(),
        "bin":       str(DERPER_BIN),
        "data":      str(DERPER_DATA),
        "ports":     {"derp": DERPER_PORT, "stun": DERPER_STUN_PORT,
                      "http": DERPER_HTTP_PORT},
    }


# ---------------------------------------------------------------------------
# Headscale DERPMap patching
# ---------------------------------------------------------------------------


def render_derp_map(*, region_id: int = 901, region_name: str = "Mackes",
                    hostname: str, ipv4: str = "",
                    port: int = DERPER_PORT) -> dict:
    """Build a JSON object suitable for /etc/headscale/derp-mackes.json.
    Headscale's config.yaml references it via:
      derp:
        paths:
          - /etc/headscale/derp-mackes.json
        update_frequency: 1h
    """
    region = {
        "RegionID":   region_id,
        "RegionCode": "mackes",
        "RegionName": region_name,
        "Nodes": [{
            "Name":     "mackes-derper",
            "RegionID": region_id,
            "HostName": hostname,
            "IPv4":     ipv4 or "",
            "DERPPort": port,
            "STUNPort": DERPER_STUN_PORT,
            # Self-signed certs on a LAN-only deploy:
            "InsecureForTests": not ipv4.startswith(("100.", "10.", "192.168.")),
        }],
    }
    return {"Regions": {str(region_id): region}}


def write_derp_map(path: Path, derp_map: dict) -> list[str]:
    from mackes.admin_session import AdminSession
    import tempfile
    with tempfile.NamedTemporaryFile(mode="w", delete=False,
                                      suffix=".json",
                                      encoding="utf-8") as tmp:
        tmp.write(json.dumps(derp_map, indent=2))
        tmp_path = tmp.name
    rc, out = AdminSession.instance().run(
        ["install", "-D", "-m", "0644", tmp_path, str(path)], timeout=10,
    )
    Path(tmp_path).unlink(missing_ok=True)
    if rc == 0:
        return [f"derp-map written to {path}"]
    return [f"derp-map write failed: {out}"]


__all__ = [
    "is_installed", "is_running", "install", "stop", "start",
    "status", "render_derp_map", "write_derp_map",
    "DERPER_BIN", "DERPER_UNIT", "DERPER_PORT",
]
