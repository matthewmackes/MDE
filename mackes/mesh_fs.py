"""Mesh filesystem — SSHFS-over-QNM mount/unmount supervisor (§8.10).

Each mesh peer designates `~/QNM-Shared/` as its shared directory; every
other peer mounts it at `~/QNM-Mesh/<peer>/` via sshfs. This module is
the supervisor that:

  - Watches peer up/down events
  - Mounts new peers at ~/QNM-Mesh/<peer>/
  - Unmounts departed peers
  - Re-mounts on reconnect after a network blip

Backend: standard sshfs CLI from the `sshfs` Fedora package. We use the
Layer-A mesh ssh key (~/.ssh/mackes_mesh_ed25519) for auth.
"""
from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path
from typing import Iterable

from mackes.logging import log_action
from mackes.state import HOME

QNM_SHARED = HOME / "QNM-Shared"
QNM_MESH   = HOME / "QNM-Mesh"
SSH_KEY    = HOME / ".ssh" / "mackes_mesh_ed25519"
SSHFS_OPTS = [
    "-o", "reconnect",
    "-o", "ServerAliveInterval=15",
    "-o", "ServerAliveCountMax=3",
    "-o", "allow_other",
    "-o", "follow_symlinks",
    "-o", "default_permissions",
]


def ensure_dirs() -> list[str]:
    actions: list[str] = []
    for p in (QNM_SHARED, QNM_MESH):
        if not p.exists():
            p.mkdir(parents=True, exist_ok=True)
            actions.append(f"created {p}")
    return actions


def is_mounted(peer: str) -> bool:
    mp = QNM_MESH / peer
    if not mp.is_dir():
        return False
    # `mountpoint -q` is cheap and accurate
    rc = subprocess.call(["mountpoint", "-q", str(mp)],
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return rc == 0


def mount_peer(peer: str, *, user: str = None, remote_dir: str = None) -> list[str]:
    """Mount peer.mesh:~/QNM-Shared/ at ~/QNM-Mesh/<peer>/."""
    actions: list[str] = []
    user = user or os.environ.get("USER") or "mm"
    remote = remote_dir or "/home/" + user + "/QNM-Shared"
    if not shutil.which("sshfs"):
        actions.append("sshfs binary missing — install with: dnf install sshfs")
        return actions
    if is_mounted(peer):
        actions.append(f"{peer} already mounted at {QNM_MESH / peer}")
        return actions
    target = QNM_MESH / peer
    target.mkdir(parents=True, exist_ok=True)
    cmd = [
        "sshfs",
        f"{user}@{peer}.mesh:{remote}",
        str(target),
        "-o", f"IdentityFile={SSH_KEY}",
        *SSHFS_OPTS,
    ]
    rc = subprocess.call(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    if rc == 0:
        actions.append(f"mounted {peer}.mesh:{remote} -> {target}")
    else:
        actions.append(f"sshfs failed for {peer} (rc={rc})")
    return actions


def unmount_peer(peer: str) -> list[str]:
    mp = QNM_MESH / peer
    if not is_mounted(peer):
        return [f"{peer} not mounted"]
    rc = subprocess.call(["fusermount", "-u", str(mp)],
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if rc != 0:
        # Try lazy unmount
        rc = subprocess.call(["fusermount", "-uz", str(mp)],
                             stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return [f"unmount {peer} rc={rc}"]


def sync_mounts(peers: Iterable[str]) -> list[str]:
    """Ensure exactly the given set of peers are mounted; unmount others."""
    actions: list[str] = []
    actions.extend(ensure_dirs())
    want = set(peers)
    have = set()
    for d in QNM_MESH.iterdir():
        if d.is_dir():
            have.add(d.name)
    for peer in want - have:
        actions.extend(mount_peer(peer))
    for peer in have:
        if peer not in want:
            actions.extend(unmount_peer(peer))
            try:
                (QNM_MESH / peer).rmdir()
            except OSError:
                pass
    for line in actions:
        log_action(line)
    return actions


__all__ = [
    "QNM_SHARED", "QNM_MESH",
    "ensure_dirs", "is_mounted",
    "mount_peer", "unmount_peer", "sync_mounts",
]
