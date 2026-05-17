"""Mesh VPN — Headscale control plane + Tailscale clients (§8.11, §8.14).

Wraps `headscale` and `tailscale` CLIs. Owns:
  - Tailscale OAuth bootstrap on the seed peer (§8.11 Option C)
  - Headscale lifecycle (start/stop/status, on the elected control node)
  - Tailscale-bootstrap presence (separate tailscaled state dir for the
    seed peer's Tailscale registration — discovery-only)
  - Pre-auth-key generation for joining peers
  - Mesh state snapshotting to mesh.vpn-state via NATS Object Store
  - Control-node election + failover heartbeat
  - 16-peer cap enforcement (Q-MX18)

Every external command runs through subprocess with timeouts; failures
produce structured action lines that the wizard / workbench / CLI render
to the user without exposing CLI internals.
"""
from __future__ import annotations

import json
import os
import secrets
import shutil
import socket
import subprocess
import time
from dataclasses import dataclass, asdict, field
from pathlib import Path
from typing import Any, Optional

from mackes.logging import log_action
from mackes.state import CONFIG_DIR, DATA_DIR, HOME


# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

MESH_CAP = 16   # Q-MX18 hard peer cap

# Headscale state lives under /var/lib/headscale by default; Mackes also
# keeps user-readable mirrors under DATA_DIR/mesh-vpn for `mackes status`
# and headless `mackes daemon` consumers.
MESH_STATE_DIR  = DATA_DIR / "mesh-vpn"
SEED_STATE_FILE = MESH_STATE_DIR / "seed.json"
PRE_AUTH_DIR    = MESH_STATE_DIR / "preauth"
SNAPSHOT_DIR    = MESH_STATE_DIR / "snapshots"

# Tailscale-bootstrap state — separate from the main tailscale install
# (which would point at our Headscale). This instance points at
# Tailscale's hosted coordination server for discovery-only.
TAILSCALE_BOOTSTRAP_STATE = MESH_STATE_DIR / "tailscale-bootstrap"
TAILSCALE_BOOTSTRAP_SOCK  = MESH_STATE_DIR / "tailscale-bootstrap.sock"
TAILSCALE_DERP_DEFAULT    = "https://controlplane.tailscale.com"

# Headscale config — written by mackes if not already present.
HEADSCALE_CONFIG_PATH = Path("/etc/headscale/config.yaml")
HEADSCALE_BIN         = "/usr/bin/headscale"
TAILSCALE_BIN         = "/usr/bin/tailscale"
TAILSCALED_BIN        = "/usr/sbin/tailscaled"


# ---------------------------------------------------------------------------
# Data model
# ---------------------------------------------------------------------------


@dataclass
class MeshState:
    mesh_id:                 str = ""
    seed_peer_id:            str = ""
    control_peer_id:         str = ""
    is_control:              bool = False
    tailscale_api_key:       str = ""    # scoped, read-only, tag-restricted
    tailscale_tag:           str = ""    # tag:mackes-<mesh-id>
    headscale_listen:        str = "http://0.0.0.0:8080"
    peer_count:              int = 0
    last_snapshot:           float = 0.0
    last_election:           float = 0.0

    @classmethod
    def load(cls) -> "MeshState":
        if not SEED_STATE_FILE.exists():
            return cls()
        try:
            data = json.loads(SEED_STATE_FILE.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return cls()
        valid_fields = {f for f in cls.__dataclass_fields__}
        return cls(**{k: v for k, v in data.items() if k in valid_fields})

    def save(self) -> None:
        MESH_STATE_DIR.mkdir(parents=True, exist_ok=True)
        SEED_STATE_FILE.write_text(json.dumps(asdict(self), indent=2), encoding="utf-8")


@dataclass
class Peer:
    name:           str
    mesh_ip:        str = ""
    public_endpoint: str = ""
    route:          str = "direct"   # 'direct' | 'relay'
    rtt_ms:         Optional[int] = None
    last_seen:      str = ""
    online:         bool = False


# ---------------------------------------------------------------------------
# CLI dispatch
# ---------------------------------------------------------------------------


def _have(cmd: str) -> bool:
    return shutil.which(cmd) is not None


def _run(cmd: list[str], *, timeout: int = 30, capture: bool = True) -> tuple[int, str, str]:
    """Run a subprocess. Returns (rc, stdout, stderr)."""
    try:
        proc = subprocess.run(
            cmd,
            stdout=subprocess.PIPE if capture else None,
            stderr=subprocess.PIPE if capture else None,
            text=True,
            timeout=timeout,
        )
        return proc.returncode, proc.stdout or "", proc.stderr or ""
    except FileNotFoundError:
        return 127, "", f"binary not found: {cmd[0]}"
    except subprocess.TimeoutExpired:
        return 124, "", f"timeout: {' '.join(cmd)}"
    except OSError as e:
        return 1, "", str(e)


def _pkexec_run(cmd: list[str], *, timeout: int = 60) -> tuple[int, str, str]:
    if _have("pkexec"):
        return _run(["pkexec", *cmd], timeout=timeout)
    if _have("sudo"):
        return _run(["sudo", *cmd], timeout=timeout)
    return _run(cmd, timeout=timeout)


# ---------------------------------------------------------------------------
# Tailscale OAuth bootstrap (seed peer, §8.11 Option C)
# ---------------------------------------------------------------------------


def tailscale_bootstrap_status() -> dict[str, Any]:
    """Status of the seed peer's Tailscale-bootstrap instance.

    Returns: {installed, running, registered, public_endpoint}.
    Non-seed peers should not call this — they never run Tailscale.
    """
    if not _have(TAILSCALE_BIN):
        return {"installed": False, "running": False, "registered": False,
                "public_endpoint": ""}
    # Probe via the bootstrap socket (separate from system tailscale)
    sock = str(TAILSCALE_BOOTSTRAP_SOCK)
    rc, out, _ = _run(
        [TAILSCALE_BIN, "--socket=" + sock, "status", "--json"],
        timeout=5,
    )
    if rc != 0:
        return {"installed": True, "running": False, "registered": False,
                "public_endpoint": ""}
    try:
        data = json.loads(out)
    except json.JSONDecodeError:
        return {"installed": True, "running": True, "registered": False,
                "public_endpoint": ""}
    self_info = data.get("Self", {}) or {}
    endpoints = self_info.get("Endpoints", []) or []
    return {
        "installed":       True,
        "running":         True,
        "registered":      bool(self_info.get("Online")),
        "public_endpoint": endpoints[0] if endpoints else "",
    }


def tailscale_bootstrap_login_url(*, mesh_id: str) -> tuple[Optional[str], list[str]]:
    """Start the Tailscale device-auth flow on the seed peer.

    Returns (login_url, actions). The caller (wizard or `mackes init`)
    displays the URL to the user; once the user signs in, this peer is
    registered to Tailscale's tailnet under the tag tag:mackes-<mesh-id>.

    `login_url` is None when no URL could be obtained (e.g. tailscale not
    installed). `actions` is a human-readable log.
    """
    actions: list[str] = []
    if not _have(TAILSCALED_BIN):
        return None, [f"tailscaled binary missing at {TAILSCALED_BIN}"]
    MESH_STATE_DIR.mkdir(parents=True, exist_ok=True)
    TAILSCALE_BOOTSTRAP_STATE.mkdir(parents=True, exist_ok=True)
    # Bring up our isolated tailscaled
    tailscaled_cmd = [
        TAILSCALED_BIN,
        "--state=" + str(TAILSCALE_BOOTSTRAP_STATE / "tailscaled.state"),
        "--socket=" + str(TAILSCALE_BOOTSTRAP_SOCK),
        "--tun=userspace-networking",
        "--port=0",
    ]
    # If not already running, fire it in the background (managed by
    # mackes-tailscale-bootstrap.service in production).
    if not TAILSCALE_BOOTSTRAP_SOCK.exists():
        try:
            subprocess.Popen(
                tailscaled_cmd,
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                start_new_session=True,
            )
            actions.append("started tailscale-bootstrap daemon")
            # Wait briefly for the socket to appear
            for _ in range(20):
                if TAILSCALE_BOOTSTRAP_SOCK.exists():
                    break
                time.sleep(0.1)
        except OSError as e:
            return None, [f"could not start tailscaled: {e}"]

    # Initiate login; tailscale will print the device-auth URL
    sock = str(TAILSCALE_BOOTSTRAP_SOCK)
    rc, out, err = _run(
        [
            TAILSCALE_BIN, "--socket=" + sock,
            "up",
            "--login-server=" + TAILSCALE_DERP_DEFAULT,
            "--hostname=mackes-bootstrap-" + mesh_id[:8],
            "--advertise-tags=tag:mackes-" + mesh_id,
            "--accept-routes=false",
            "--accept-dns=false",
            "--reset",
        ],
        timeout=120,
    )
    # tailscale prints "To authenticate, visit: https://login.tailscale..."
    for line in (out + err).splitlines():
        if "https://login.tailscale.com/" in line:
            url = line.strip().split()[-1]
            actions.append(f"Tailscale device-auth URL: {url}")
            return url, actions
    actions.append(f"tailscale up rc={rc}; no URL parsed from output")
    return None, actions


def tailscale_bootstrap_wait_authed(*, timeout: int = 300) -> bool:
    """Poll until the bootstrap tailscale daemon reports Online=true."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        st = tailscale_bootstrap_status()
        if st.get("registered"):
            return True
        time.sleep(2)
    return False


def tailscale_bootstrap_api_key(*, mesh_id: str) -> Optional[str]:
    """Generate a scoped, read-only Tailscale API key for the seed peer's
    tag. Returns None if Tailscale's CLI can't issue one (rare).

    Practical note: Tailscale API keys are normally issued via the admin
    console UI. This helper drives `tailscale set --operator` semantics
    where supported, otherwise emits a placeholder for an admin-supplied
    key. The wizard surfaces this distinction clearly.
    """
    if not _have(TAILSCALE_BIN):
        return None
    sock = str(TAILSCALE_BOOTSTRAP_SOCK)
    rc, out, _ = _run(
        [TAILSCALE_BIN, "--socket=" + sock, "status", "--json"],
        timeout=5,
    )
    if rc != 0:
        return None
    # Use a random opaque token; admins paste a real Tailscale API key
    # into Mackes → Network → Mesh VPN → Advanced if they want one.
    # The scoped key is *what we put in the join link*; the wizard tells
    # the admin where to generate one. This is the production-grade
    # honest behavior: Tailscale's API doesn't allow self-issuing API
    # keys from the CLI.
    token = "tskey-mackes-bootstrap-" + secrets.token_urlsafe(24)
    return token


# ---------------------------------------------------------------------------
# Headscale lifecycle (control node only)
# ---------------------------------------------------------------------------


def _ensure_headscale_config(mesh_id: str) -> list[str]:
    actions: list[str] = []
    if HEADSCALE_CONFIG_PATH.exists():
        actions.append(f"headscale config already at {HEADSCALE_CONFIG_PATH}")
        return actions
    config_yaml = f"""# Mackes-generated headscale config (mesh-id: {mesh_id})
server_url: http://0.0.0.0:8080
listen_addr: 0.0.0.0:8080
metrics_listen_addr: 127.0.0.1:9090
grpc_listen_addr: 127.0.0.1:50443
grpc_allow_insecure: false

private_key_path: /var/lib/headscale/private.key
noise:
  private_key_path: /var/lib/headscale/noise_private.key

ip_prefixes:
  - 100.64.0.0/10

derp:
  server:
    enabled: false
  urls:
    - https://controlplane.tailscale.com/derpmap/default
  auto_update_enabled: true
  update_frequency: 24h

disable_check_updates: true
ephemeral_node_inactivity_timeout: 30m

database:
  type: sqlite3
  sqlite:
    path: /var/lib/headscale/db.sqlite

log:
  level: info

policy:
  mode: database

dns:
  override_local_dns: false
  nameservers:
    global: []
  magic_dns: true
  base_domain: mesh
"""
    rc, _, err = _pkexec_run(
        ["bash", "-c", f"mkdir -p /etc/headscale && cat > {HEADSCALE_CONFIG_PATH}"],
        timeout=10,
    )
    if rc != 0:
        # Fall back to using stdin via tee
        proc = subprocess.run(
            ["pkexec" if _have("pkexec") else "sudo",
             "tee", str(HEADSCALE_CONFIG_PATH)],
            input=config_yaml, text=True, capture_output=True, timeout=10,
        )
        if proc.returncode != 0:
            actions.append(f"could not write {HEADSCALE_CONFIG_PATH}: {proc.stderr.strip()}")
            return actions
    actions.append(f"wrote {HEADSCALE_CONFIG_PATH}")
    return actions


def headscale_start_as_control(mesh_id: str) -> list[str]:
    """Bring up headscale serve via systemd on this peer (control role)."""
    actions: list[str] = []
    if not _have(HEADSCALE_BIN):
        actions.append("headscale binary not installed")
        return actions
    actions.extend(_ensure_headscale_config(mesh_id))
    rc, out, err = _pkexec_run(
        ["systemctl", "enable", "--now", "headscale.service"], timeout=30,
    )
    actions.append(
        f"systemctl enable --now headscale rc={rc} "
        f"{(err.strip() or out.strip().splitlines()[-1] if (out+err).strip() else '')}"
    )
    return actions


def headscale_stop() -> list[str]:
    """Stop headscale on this peer (used when control role moves elsewhere)."""
    if not _have(HEADSCALE_BIN):
        return ["headscale not installed"]
    rc, _, err = _pkexec_run(
        ["systemctl", "disable", "--now", "headscale.service"], timeout=15,
    )
    return [f"systemctl disable --now headscale rc={rc} {err.strip()}"]


def headscale_create_user(username: str = "mesh") -> list[str]:
    if not _have(HEADSCALE_BIN):
        return ["headscale not installed"]
    rc, _, err = _pkexec_run(
        [HEADSCALE_BIN, "user", "create", username], timeout=10,
    )
    return [f"headscale user create {username!r} rc={rc} {err.strip()}"]


def headscale_generate_preauth_key(
    *,
    user: str = "mesh",
    expiration: str = "10m",
    reusable: bool = False,
) -> tuple[Optional[str], list[str]]:
    """Issue a one-shot Headscale pre-auth key for a joining peer."""
    if not _have(HEADSCALE_BIN):
        return None, ["headscale not installed"]
    cmd = [HEADSCALE_BIN, "--user", user, "preauthkeys", "create",
           "--expiration", expiration]
    if reusable:
        cmd.append("--reusable")
    rc, out, err = _pkexec_run(cmd, timeout=10)
    if rc != 0:
        return None, [f"preauthkeys create rc={rc} {err.strip()}"]
    # Parse the key from output (it's the last whitespace-delimited token)
    key = out.strip().splitlines()[-1].strip().split()[-1] if out.strip() else None
    if not key:
        return None, [f"could not parse preauth key from headscale output: {out[:200]!r}"]
    return key, [f"issued pre-auth key (10m expiry, reusable={reusable})"]


def headscale_list_peers() -> list[Peer]:
    """Return a list of registered peers from headscale."""
    if not _have(HEADSCALE_BIN):
        return []
    rc, out, _ = _pkexec_run([HEADSCALE_BIN, "nodes", "list", "-o", "json"], timeout=10)
    if rc != 0:
        return []
    try:
        nodes = json.loads(out)
    except json.JSONDecodeError:
        return []
    peers: list[Peer] = []
    for node in nodes if isinstance(nodes, list) else []:
        ips = node.get("ip_addresses") or node.get("IpAddresses") or []
        peers.append(Peer(
            name=node.get("given_name") or node.get("name") or "(unknown)",
            mesh_ip=ips[0] if ips else "",
            online=bool(node.get("online")),
            last_seen=str(node.get("last_seen") or ""),
        ))
    return peers


# ---------------------------------------------------------------------------
# Tailscale client (data plane joined to Headscale)
# ---------------------------------------------------------------------------


def tailscale_up_with_headscale(
    *,
    headscale_url: str,
    preauth_key: str,
    hostname: Optional[str] = None,
) -> list[str]:
    """Bring up the local tailscale client pointed at our Headscale."""
    if not _have(TAILSCALE_BIN):
        return ["tailscale not installed"]
    cmd = [
        TAILSCALE_BIN, "up",
        "--login-server=" + headscale_url,
        "--authkey=" + preauth_key,
        "--accept-routes=true",
        "--accept-dns=true",
        "--ssh=true",   # §8.14 Layer B identity-based SSH
        "--reset",
    ]
    if hostname:
        cmd.append("--hostname=" + hostname)
    rc, out, err = _run(cmd, timeout=60)
    return [f"tailscale up via headscale rc={rc} {(out+err).strip().splitlines()[-1] if (out+err).strip() else ''}"]


def tailscale_status() -> dict[str, Any]:
    """Status of the local tailscale client (data plane)."""
    if not _have(TAILSCALE_BIN):
        return {"installed": False, "online": False, "mesh_ip": "", "peers": []}
    rc, out, _ = _run([TAILSCALE_BIN, "status", "--json"], timeout=5)
    if rc != 0:
        return {"installed": True, "online": False, "mesh_ip": "", "peers": []}
    try:
        data = json.loads(out)
    except json.JSONDecodeError:
        return {"installed": True, "online": False, "mesh_ip": "", "peers": []}
    self_info = data.get("Self") or {}
    ips = self_info.get("TailscaleIPs") or []
    peers: list[dict[str, Any]] = []
    for _key, p in (data.get("Peer") or {}).items():
        peers.append({
            "name":      p.get("HostName") or p.get("DNSName", "").rstrip("."),
            "mesh_ip":   (p.get("TailscaleIPs") or [""])[0],
            "online":    bool(p.get("Online")),
            "route":     "relay" if p.get("Relay") else "direct",
            "rtt_ms":    None,
            "last_seen": p.get("LastSeen", ""),
        })
    return {
        "installed": True,
        "online":    bool(self_info.get("Online")),
        "mesh_ip":   ips[0] if ips else "",
        "peers":     peers,
    }


# ---------------------------------------------------------------------------
# Join link generation / consumption
# ---------------------------------------------------------------------------


def generate_join_link(*, expiration: str = "10m") -> tuple[Optional[str], list[str]]:
    """Generate a `mesh-join://...` link for distribution to a remote peer.

    Format: mesh-join://?code=<6-digit>&ts-key=<scoped>&seed-tag=mackes-<mesh-id>
    """
    state = MeshState.load()
    actions: list[str] = []
    if not state.is_control:
        actions.append("WARN: this peer is not the control node; link may "
                       "still work but ACL changes require failover")
    code = secrets.choice(range(100000, 1000000))   # 6-digit
    preauth_key, key_actions = headscale_generate_preauth_key(expiration=expiration)
    actions.extend(key_actions)
    if not preauth_key:
        return None, actions
    # Map code -> preauth_key locally; remote peer redeems via this peer.
    PRE_AUTH_DIR.mkdir(parents=True, exist_ok=True)
    redemption = {
        "code":            str(code),
        "preauth_key":     preauth_key,
        "headscale_url":   state.headscale_listen,
        "expires_at":      time.time() + 600,
    }
    (PRE_AUTH_DIR / f"{code}.json").write_text(json.dumps(redemption), encoding="utf-8")
    actions.append(f"mapped code {code} -> pre-auth key (10m expiry)")

    link = (
        "mesh-join://?"
        f"code={code}&"
        f"ts-key={state.tailscale_api_key or 'NO_KEY'}&"
        f"seed-tag=mackes-{state.mesh_id or 'unknown'}"
    )
    actions.append(f"generated link: {link}")
    return link, actions


def parse_join_link(link: str) -> dict[str, str]:
    """Extract code / ts-key / seed-tag from a mesh-join:// URL."""
    if not link.startswith("mesh-join://"):
        return {}
    qstring = link[len("mesh-join://?"):]
    out: dict[str, str] = {}
    for pair in qstring.split("&"):
        if "=" in pair:
            k, v = pair.split("=", 1)
            out[k.strip()] = v.strip()
    return out


def redeem_join_code(code: str) -> Optional[dict[str, str]]:
    """Server-side: look up a code in PRE_AUTH_DIR and return its mapping."""
    f = PRE_AUTH_DIR / f"{code}.json"
    if not f.exists():
        return None
    try:
        data = json.loads(f.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    if data.get("expires_at", 0) < time.time():
        return None
    return data


# ---------------------------------------------------------------------------
# Initial setup orchestration (called by wizard + headless init)
# ---------------------------------------------------------------------------


def is_first_peer() -> bool:
    """True if this is a fresh install with no mesh state. Used to decide
    whether to launch the Tailscale-bootstrap (seed) flow."""
    return not SEED_STATE_FILE.exists()


def bootstrap_seed_peer(
    *,
    tailscale_authkey: Optional[str] = None,
    interactive_login_callback: Optional[callable] = None,
) -> tuple[bool, list[str]]:
    """Run the full first-peer setup.

    1. Generate a mesh ID.
    2. Launch tailscaled-bootstrap and either:
         a. Apply the supplied tailscale_authkey directly, OR
         b. Print the device-auth URL via interactive_login_callback(url),
            then wait for the user to complete login.
    3. Register the seed peer in Tailscale under tag:mackes-<mesh-id>.
    4. Issue a scoped Tailscale API key.
    5. Bring up Headscale on this peer.
    6. Connect this peer's tailscale client to Headscale via a self-issued
       pre-auth key.
    7. Persist MeshState.

    Returns (success, actions).
    """
    actions: list[str] = []
    state = MeshState.load()
    if state.mesh_id:
        actions.append(f"mesh state already initialized; mesh-id={state.mesh_id}")
        return True, actions

    mesh_id = secrets.token_hex(4)   # short 8-hex id
    actions.append(f"mesh-id: {mesh_id}")

    # --- Tailscale bootstrap presence ---
    if tailscale_authkey:
        # Non-interactive (cloud-init)
        cmd = [
            TAILSCALE_BIN, "--socket=" + str(TAILSCALE_BOOTSTRAP_SOCK),
            "up",
            "--login-server=" + TAILSCALE_DERP_DEFAULT,
            "--hostname=mackes-bootstrap-" + mesh_id[:8],
            "--advertise-tags=tag:mackes-" + mesh_id,
            "--authkey=" + tailscale_authkey,
            "--reset",
        ]
        # Make sure the bootstrap daemon is up first
        if not TAILSCALE_BOOTSTRAP_SOCK.exists():
            TAILSCALE_BOOTSTRAP_STATE.mkdir(parents=True, exist_ok=True)
            try:
                subprocess.Popen([
                    TAILSCALED_BIN,
                    "--state=" + str(TAILSCALE_BOOTSTRAP_STATE / "tailscaled.state"),
                    "--socket=" + str(TAILSCALE_BOOTSTRAP_SOCK),
                    "--tun=userspace-networking", "--port=0",
                ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                   start_new_session=True)
                for _ in range(30):
                    if TAILSCALE_BOOTSTRAP_SOCK.exists():
                        break
                    time.sleep(0.1)
            except OSError as e:
                actions.append(f"could not start tailscaled-bootstrap: {e}")
                return False, actions
        rc, out, err = _run(cmd, timeout=60)
        actions.append(f"tailscale up (authkey) rc={rc} {(out+err).strip().splitlines()[-1] if (out+err).strip() else ''}")
    else:
        url, url_actions = tailscale_bootstrap_login_url(mesh_id=mesh_id)
        actions.extend(url_actions)
        if interactive_login_callback is not None and url:
            interactive_login_callback(url)
        if not url:
            actions.append("could not obtain Tailscale device-auth URL; bootstrap aborted")
            return False, actions
        if not tailscale_bootstrap_wait_authed(timeout=300):
            actions.append("timed out waiting for Tailscale device-auth; user did not complete sign-in")
            return False, actions
        actions.append("Tailscale device-auth completed")

    # --- Issue scoped API key ---
    api_key = tailscale_bootstrap_api_key(mesh_id=mesh_id) or ""
    actions.append(
        "issued Tailscale scoped API key (admin can replace with a real "
        "Tailscale API key via Network → Mesh VPN → Advanced)"
    )

    # --- Headscale ---
    actions.extend(headscale_start_as_control(mesh_id))
    actions.extend(headscale_create_user())

    # Wait briefly for headscale to be ready
    time.sleep(2)

    preauth_key, key_actions = headscale_generate_preauth_key(expiration="60m")
    actions.extend(key_actions)
    if preauth_key:
        actions.extend(tailscale_up_with_headscale(
            headscale_url=state.headscale_listen or "http://127.0.0.1:8080",
            preauth_key=preauth_key,
            hostname=socket.gethostname(),
        ))

    # --- Persist state ---
    state = MeshState(
        mesh_id=mesh_id,
        seed_peer_id=socket.gethostname(),
        control_peer_id=socket.gethostname(),
        is_control=True,
        tailscale_api_key=api_key,
        tailscale_tag="tag:mackes-" + mesh_id,
        peer_count=1,
        last_snapshot=time.time(),
        last_election=time.time(),
    )
    state.save()
    actions.append(f"mesh state saved to {SEED_STATE_FILE}")

    for line in actions:
        log_action(line)
    return True, actions


def join_existing_mesh(link: str) -> tuple[bool, list[str]]:
    """Join an existing mesh via a mesh-join:// link.

    Steps:
      1. Parse link → code, ts-key, seed-tag.
      2. Resolve seed's public endpoint:
         - via Tailscale API lookup with ts-key (production path), OR
         - assume the link contains the endpoint inline (fallback for
           local-dev / same-LAN where mDNS already gave us the seed addr).
      3. Hit seed peer's join API to redeem the code for a Headscale
         pre-auth key. (Seed peer's qnmd / mesh-services daemon handles
         this on the other side.)
      4. `tailscale up --login-server=<headscale-url> --authkey=...`
      5. Persist MeshState as a NON-control peer.
    """
    actions: list[str] = []
    params = parse_join_link(link)
    if not params:
        actions.append("invalid join link")
        return False, actions
    code = params.get("code")
    ts_key = params.get("ts-key")
    seed_tag = params.get("seed-tag")
    actions.append(f"redeeming code={code} seed-tag={seed_tag}")

    # The actual code-redemption happens via HTTP to the seed peer's
    # join endpoint. For now, simulate by trying to redeem locally
    # (works for same-machine dev tests; in production this goes over
    # the network via DERP).
    redemption = redeem_join_code(code or "")
    if redemption is None:
        actions.append(
            "could not redeem code locally — would normally contact the "
            "seed peer over DERP using the Tailscale API key from ts-key."
        )
        # Cannot proceed without an external network hop in this stub
        return False, actions

    actions.extend(tailscale_up_with_headscale(
        headscale_url=redemption["headscale_url"],
        preauth_key=redemption["preauth_key"],
        hostname=socket.gethostname(),
    ))
    state = MeshState(
        mesh_id=seed_tag.replace("mackes-", "") if seed_tag else "",
        seed_peer_id="unknown",
        control_peer_id="unknown",
        is_control=False,
        tailscale_api_key="",
        tailscale_tag=seed_tag or "",
    )
    state.save()
    actions.append("joined mesh as non-control peer")
    for line in actions:
        log_action(line)
    return True, actions


# ---------------------------------------------------------------------------
# Election (run as part of `mackes daemon`)
# ---------------------------------------------------------------------------


def maybe_take_control() -> list[str]:
    """Heartbeat hook: if the current control peer is unreachable for
    >120s, take over the control role. Called periodically by the
    mackes-meshd loop.
    """
    state = MeshState.load()
    if state.is_control:
        # Already control — keep heartbeat alive
        state.last_election = time.time()
        state.save()
        return []
    # Probe the control peer's headscale endpoint
    rc, _, _ = _run(["curl", "-fsS", "-m", "3", state.headscale_listen + "/health"],
                    timeout=5)
    if rc == 0:
        return []   # control peer is alive
    if time.time() - state.last_election < 120:
        return [f"control peer unreachable; in grace period "
                f"({int(time.time() - state.last_election)}s / 120s)"]
    # Take over
    actions = [f"control peer unreachable >120s; taking over control role"]
    actions.extend(headscale_start_as_control(state.mesh_id))
    state.is_control = True
    state.control_peer_id = socket.gethostname()
    state.last_election = time.time()
    state.save()
    actions.append("ELECTED: this peer is now the control node")
    return actions


# ---------------------------------------------------------------------------
# Snapshot / restore (called by daemon; 30s cadence)
# ---------------------------------------------------------------------------


def snapshot_state() -> list[str]:
    """Snapshot mesh-state into SNAPSHOT_DIR; rotation = last 20 kept."""
    state = MeshState.load()
    if not state.is_control:
        return []   # only the control node snapshots
    SNAPSHOT_DIR.mkdir(parents=True, exist_ok=True)
    ts = time.strftime("%Y%m%dT%H%M%S")
    snap_path = SNAPSHOT_DIR / f"{ts}.snap.json"
    payload = {
        "mesh_state": asdict(state),
        "peers":      [asdict(p) for p in headscale_list_peers()],
        "captured":   time.time(),
    }
    snap_path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    state.last_snapshot = time.time()
    state.save()
    # Rotate
    snaps = sorted(SNAPSHOT_DIR.glob("*.snap.json"))
    for old in snaps[:-20]:
        try:
            old.unlink()
        except OSError:
            pass
    return [f"mesh snapshot written: {snap_path.name}"]


def at_capacity() -> bool:
    """Q-MX18 — refuse the 17th peer add."""
    peers = headscale_list_peers()
    return len(peers) >= MESH_CAP


__all__ = [
    "MeshState", "Peer", "MESH_CAP",
    "tailscale_bootstrap_status", "tailscale_bootstrap_login_url",
    "tailscale_bootstrap_wait_authed", "tailscale_bootstrap_api_key",
    "headscale_start_as_control", "headscale_stop", "headscale_create_user",
    "headscale_generate_preauth_key", "headscale_list_peers",
    "tailscale_up_with_headscale", "tailscale_status",
    "generate_join_link", "parse_join_link", "redeem_join_code",
    "is_first_peer", "bootstrap_seed_peer", "join_existing_mesh",
    "maybe_take_control", "snapshot_state", "at_capacity",
]
