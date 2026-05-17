"""Mesh SSH — §8.14 three-layer SSH for the mesh.

Layer 0 — Raw SSH cheatsheet (free baseline; doc-only).
Layer A — Auto-distributed ed25519 keys via NATS Object Store: every peer
          publishes its pubkey; every peer subscribes and appends remote
          pubkeys to a configured user's ~/.ssh/authorized_keys with
          surgical markers.
Layer B — Identity-based SSH via Headscale's Tailscale-SSH support. ACLs
          managed via Headscale policy YAML. Audit log of accepted/denied
          sessions recorded to NATS.

The auto-key distribution uses a simple file-watch / NATS Object Store
adapter; for the Mackes 1.0 codebase we treat NATS as optional and fall
back to a shared file under ~/.config/mackes-shell/mesh-ssh-keys/<peer-id>.pub
that mesh-meshd syncs across peers. The interface is identical.
"""
from __future__ import annotations

import json
import os
import shutil
import socket
import subprocess
import time
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Optional

from mackes.logging import log_action
from mackes.state import CONFIG_DIR, DATA_DIR, HOME


# ---------------------------------------------------------------------------
# Layout
# ---------------------------------------------------------------------------

SSH_DIR             = HOME / ".ssh"
MESH_KEY_PATH       = SSH_DIR / "mackes_mesh_ed25519"
MESH_PUB_PATH       = SSH_DIR / "mackes_mesh_ed25519.pub"
AUTHORIZED_KEYS     = SSH_DIR / "authorized_keys"
MESH_KEYS_DIR       = CONFIG_DIR / "mesh-ssh-keys"   # local cache of all peer pubkeys
MESH_POLICY_PATH    = CONFIG_DIR / "mesh-ssh-policy.yaml"
MESH_POLICY_EXAMPLE = Path("/usr/share/mackes-shell/data/mesh-ssh-policy.example.yaml")
MESH_AUDIT_LOG      = DATA_DIR / "logs" / "mesh-ssh-audit.jsonl"

# Surgical authorized_keys markers — Mackes only touches lines between
# these bracket pairs. This keeps user-managed keys completely intact.
MARKER_BEGIN = "# managed-by-mackes-mesh-{peer_id} begin"
MARKER_END   = "# managed-by-mackes-mesh-{peer_id} end"


@dataclass
class PolicyRule:
    action: str = "accept"        # accept | reject
    src:    list[str] = field(default_factory=lambda: ["*"])
    dst:    list[str] = field(default_factory=lambda: ["*"])
    users:  list[str] = field(default_factory=lambda: ["root"])


@dataclass
class AuditRecord:
    timestamp:   str
    source_peer: str
    source_user: str
    target_peer: str
    target_user: str
    session_id:  str
    exit_status: int


# ---------------------------------------------------------------------------
# Layer A — key generation + distribution
# ---------------------------------------------------------------------------


def ensure_mesh_keypair() -> list[str]:
    """Generate ~/.ssh/mackes_mesh_ed25519 if absent. Idempotent."""
    actions: list[str] = []
    SSH_DIR.mkdir(parents=True, exist_ok=True)
    SSH_DIR.chmod(0o700)
    if MESH_KEY_PATH.exists() and MESH_PUB_PATH.exists():
        actions.append(f"mesh keypair present at {MESH_KEY_PATH}")
        return actions
    if not shutil.which("ssh-keygen"):
        actions.append("ssh-keygen missing; cannot generate mesh keypair")
        return actions
    hostname = socket.gethostname()
    rc = subprocess.call([
        "ssh-keygen", "-t", "ed25519",
        "-N", "",                    # no passphrase
        "-C", f"mackes-mesh-{hostname}",
        "-f", str(MESH_KEY_PATH),
    ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if rc == 0:
        MESH_KEY_PATH.chmod(0o600)
        MESH_PUB_PATH.chmod(0o644)
        actions.append(f"generated mesh keypair at {MESH_KEY_PATH}")
    else:
        actions.append(f"ssh-keygen failed rc={rc}")
    return actions


def publish_my_pubkey(*, peer_id: Optional[str] = None) -> list[str]:
    """Write my pubkey to MESH_KEYS_DIR for other peers to pick up.

    The mackes-meshd daemon mirrors MESH_KEYS_DIR into NATS Object Store
    (production) or rsyncs it across SSHFS peer mounts (mesh-fs fallback).
    The local effect is the same: a per-peer file readable by every peer.
    """
    actions: list[str] = []
    if not MESH_PUB_PATH.exists():
        actions.append("mesh pubkey missing; run ensure_mesh_keypair() first")
        return actions
    MESH_KEYS_DIR.mkdir(parents=True, exist_ok=True)
    pid = peer_id or socket.gethostname()
    dest = MESH_KEYS_DIR / f"{pid}.pub"
    dest.write_text(MESH_PUB_PATH.read_text(encoding="utf-8"), encoding="utf-8")
    actions.append(f"published pubkey for peer-id={pid} -> {dest}")
    return actions


def _strip_block(content: str, peer_id: str) -> str:
    begin = MARKER_BEGIN.format(peer_id=peer_id)
    end   = MARKER_END.format(peer_id=peer_id)
    lines = content.splitlines()
    out: list[str] = []
    skip = False
    for ln in lines:
        if ln.strip() == begin:
            skip = True
            continue
        if ln.strip() == end:
            skip = False
            continue
        if not skip:
            out.append(ln)
    return "\n".join(out)


def install_peer_pubkey(peer_id: str, pubkey: str) -> list[str]:
    """Append a peer's pubkey to ~/.ssh/authorized_keys with markers."""
    SSH_DIR.mkdir(parents=True, exist_ok=True)
    SSH_DIR.chmod(0o700)
    existing = AUTHORIZED_KEYS.read_text(encoding="utf-8") if AUTHORIZED_KEYS.exists() else ""
    stripped = _strip_block(existing, peer_id)
    begin = MARKER_BEGIN.format(peer_id=peer_id)
    end   = MARKER_END.format(peer_id=peer_id)
    new = stripped.rstrip()
    if new:
        new += "\n\n"
    new += f"{begin}\n{pubkey.strip()}\n{end}\n"
    AUTHORIZED_KEYS.write_text(new, encoding="utf-8")
    AUTHORIZED_KEYS.chmod(0o600)
    return [f"installed pubkey for peer-id={peer_id} into {AUTHORIZED_KEYS}"]


def uninstall_peer_pubkey(peer_id: str) -> list[str]:
    if not AUTHORIZED_KEYS.exists():
        return ["authorized_keys absent; nothing to remove"]
    existing = AUTHORIZED_KEYS.read_text(encoding="utf-8")
    new = _strip_block(existing, peer_id).rstrip() + "\n"
    AUTHORIZED_KEYS.write_text(new, encoding="utf-8")
    AUTHORIZED_KEYS.chmod(0o600)
    return [f"removed pubkey block for peer-id={peer_id} from {AUTHORIZED_KEYS}"]


def sync_authorized_keys() -> list[str]:
    """Make ~/.ssh/authorized_keys reflect every file in MESH_KEYS_DIR.

    Idempotent — re-running has no visible effect once converged.
    Used by mackes-meshd on every peer event.
    """
    actions: list[str] = []
    if not MESH_KEYS_DIR.is_dir():
        return ["no mesh keys cache yet"]
    seen: set[str] = set()
    for pubfile in sorted(MESH_KEYS_DIR.glob("*.pub")):
        peer_id = pubfile.stem
        seen.add(peer_id)
        try:
            pubkey = pubfile.read_text(encoding="utf-8").strip()
        except OSError:
            continue
        actions.extend(install_peer_pubkey(peer_id, pubkey))
    # Drop blocks for peers that no longer have a pubfile (peer left)
    if AUTHORIZED_KEYS.exists():
        existing = AUTHORIZED_KEYS.read_text(encoding="utf-8")
        # Scan for any management markers — peers whose markers exist but
        # whose pubfiles have been removed need their blocks pruned.
        import re
        for m in re.finditer(r"^# managed-by-mackes-mesh-(\S+) begin\s*$",
                             existing, re.MULTILINE):
            pid = m.group(1)
            if pid not in seen:
                actions.extend(uninstall_peer_pubkey(pid))
    return actions


# ---------------------------------------------------------------------------
# Layer B — Headscale Tailscale-SSH policy + audit
# ---------------------------------------------------------------------------


_EXAMPLE_POLICY = """# Mackes Mesh SSH policy (Headscale-format).
# See §8.14 for the spec; edit via Network → Mesh SSH → Access Policy.
groups:
  group:admin:   ["mm"]
  group:user:    ["mm", "guest"]
tagOwners:
  tag:mackes-admin:      ["group:admin"]
  tag:mackes-user:       ["group:user"]
  tag:mackes-fileserver: ["group:admin"]
acls:
  - action: accept
    src:    ["*"]
    dst:    ["*:*"]
ssh:
  - action: accept
    src:    ["tag:mackes-admin"]
    dst:    ["*"]
    users:  ["root", "mm"]
  - action: accept
    src:    ["tag:mackes-user"]
    dst:    ["tag:mackes-fileserver"]
    users:  ["mm"]
"""


def ensure_policy_file() -> list[str]:
    """Drop the example policy file if no policy exists yet."""
    if MESH_POLICY_PATH.exists():
        return [f"policy already at {MESH_POLICY_PATH}"]
    MESH_POLICY_PATH.parent.mkdir(parents=True, exist_ok=True)
    if MESH_POLICY_EXAMPLE.exists():
        MESH_POLICY_PATH.write_text(
            MESH_POLICY_EXAMPLE.read_text(encoding="utf-8"),
            encoding="utf-8",
        )
        return [f"copied example policy -> {MESH_POLICY_PATH}"]
    MESH_POLICY_PATH.write_text(_EXAMPLE_POLICY, encoding="utf-8")
    return [f"seeded default policy at {MESH_POLICY_PATH}"]


def load_policy_yaml() -> str:
    if MESH_POLICY_PATH.exists():
        return MESH_POLICY_PATH.read_text(encoding="utf-8")
    return _EXAMPLE_POLICY


def save_policy_yaml(text: str) -> list[str]:
    MESH_POLICY_PATH.parent.mkdir(parents=True, exist_ok=True)
    MESH_POLICY_PATH.write_text(text, encoding="utf-8")
    actions = [f"wrote {MESH_POLICY_PATH}"]
    # Push to Headscale
    if shutil.which("headscale"):
        from mackes.mesh_vpn import _pkexec_run
        rc, out, err = _pkexec_run(
            ["headscale", "policy", "set", "--file", str(MESH_POLICY_PATH)],
            timeout=10,
        )
        actions.append(f"headscale policy set rc={rc} {(out+err).strip().splitlines()[-1] if (out+err).strip() else ''}")
    return actions


# ---------------------------------------------------------------------------
# Audit
# ---------------------------------------------------------------------------


def record_audit(rec: AuditRecord) -> None:
    MESH_AUDIT_LOG.parent.mkdir(parents=True, exist_ok=True)
    with MESH_AUDIT_LOG.open("a", encoding="utf-8") as f:
        f.write(json.dumps(asdict(rec)) + "\n")


def read_audit(*, last_n: int = 1000) -> list[AuditRecord]:
    if not MESH_AUDIT_LOG.exists():
        return []
    try:
        lines = MESH_AUDIT_LOG.read_text(encoding="utf-8").splitlines()
    except OSError:
        return []
    out: list[AuditRecord] = []
    for ln in lines[-last_n:]:
        try:
            data = json.loads(ln)
            out.append(AuditRecord(**data))
        except (json.JSONDecodeError, TypeError):
            continue
    return out


# ---------------------------------------------------------------------------
# Open a session (CLI: `mackes ssh <peer>`)
# ---------------------------------------------------------------------------


def open_session(peer_name: str, *, layer: str = "auto", user: Optional[str] = None) -> int:
    """Open an interactive SSH session against a mesh peer.

    layer: 'auto' (B if available, A fallback), 'A' (key-based), 'B' (TS-SSH).
    Returns the SSH subprocess exit code.
    """
    target_user = user or os.environ.get("USER") or "mm"
    target_host = f"{peer_name}.mesh"
    cmd: list[str]

    if layer in ("auto", "B"):
        # Layer B: ssh via tailscale ssh (uses Tailscale identity)
        if shutil.which("tailscale"):
            cmd = ["tailscale", "ssh", f"{target_user}@{peer_name}"]
            rc = subprocess.call(cmd)
            record_audit(AuditRecord(
                timestamp=time.strftime("%Y-%m-%dT%H:%M:%S"),
                source_peer=socket.gethostname(),
                source_user=os.environ.get("USER", "?"),
                target_peer=peer_name,
                target_user=target_user,
                session_id=f"ts-ssh-{int(time.time())}",
                exit_status=rc,
            ))
            return rc

    # Layer A: regular ssh + auto-distributed keys
    cmd = ["ssh", "-i", str(MESH_KEY_PATH), f"{target_user}@{target_host}"]
    rc = subprocess.call(cmd)
    record_audit(AuditRecord(
        timestamp=time.strftime("%Y-%m-%dT%H:%M:%S"),
        source_peer=socket.gethostname(),
        source_user=os.environ.get("USER", "?"),
        target_peer=peer_name,
        target_user=target_user,
        session_id=f"ssh-A-{int(time.time())}",
        exit_status=rc,
    ))
    return rc


def cheatsheet() -> list[str]:
    """Return a list of `ssh user@peer.mesh` cheatsheet lines."""
    user = os.environ.get("USER") or "mm"
    out: list[str] = []
    # Try to pull peer list from headscale; fall back to mesh-keys cache
    try:
        from mackes.mesh_vpn import headscale_list_peers
        peers = [p.name for p in headscale_list_peers()]
    except Exception:  # noqa: BLE001
        peers = []
    if not peers and MESH_KEYS_DIR.is_dir():
        peers = [f.stem for f in MESH_KEYS_DIR.glob("*.pub")]
    if not peers:
        return ["(no mesh peers known yet)"]
    for p in peers:
        out.append(f"ssh {user}@{p}.mesh")
    return out


__all__ = [
    "MESH_KEY_PATH", "MESH_PUB_PATH", "AUTHORIZED_KEYS",
    "MESH_KEYS_DIR", "MESH_POLICY_PATH", "MESH_AUDIT_LOG",
    "PolicyRule", "AuditRecord",
    "ensure_mesh_keypair", "publish_my_pubkey",
    "install_peer_pubkey", "uninstall_peer_pubkey", "sync_authorized_keys",
    "ensure_policy_file", "load_policy_yaml", "save_policy_yaml",
    "record_audit", "read_audit",
    "open_session", "cheatsheet",
]
