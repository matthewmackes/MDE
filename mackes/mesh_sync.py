"""Mesh-sync — NATS substrate stand-in (§8.10/§8.13).

The spec calls for NATS JetStream + Object Store as the substrate for
distributed clipboard, notifications, theme bundles, preset sync, and
generic blob-drop. Mackes 1.0 ships a file-based substrate that uses the
SSHFS mesh-fs mounts (§8.10) as the wire — every peer publishes into its
own ~/QNM-Shared/.qnm-sync/ tree; every peer reads from every other peer's
mount at ~/QNM-Mesh/<peer>/.qnm-sync/.

This is API-compatible with a future NATS replacement: callers don't see
the transport. The interface exposes Object-Store-like primitives:

    put(bucket, key, bytes_or_str)        # publish to my own bucket
    list(bucket)                          # list every peer's keys
    get(bucket, peer, key)                # read a specific entry
    delete(bucket, key)                   # remove from my bucket
    versions(bucket, peer, key)           # list prior revisions
    subscribe(bucket, callback)           # called on new entry
"""
from __future__ import annotations

import json
import shutil
import socket
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Iterable, Optional

from mackes.mesh_fs import QNM_MESH, QNM_SHARED


SYNC_ROOT_MINE = QNM_SHARED / ".qnm-sync"
PEERS_ROOT     = QNM_MESH   # each peer's mount

# Standard bucket names ship as constants; callers can register any new
# string and the FS layout auto-creates a directory.
BUCKET_CLIPBOARD     = "clipboard"
BUCKET_NOTIFICATIONS = "notifications"
BUCKET_SNAPSHOTS     = "snapshots"
BUCKET_THEMES        = "themes"
BUCKET_PRESETS       = "presets"
BUCKET_DROP          = "drop"
BUCKET_VPN_STATE     = "vpn-state"
BUCKET_CA_ROOT       = "ca-root"
BUCKET_SSH_KEYS      = "ssh-keys"
BUCKET_SSH_AUDIT     = "ssh-audit"

ALL_BUCKETS = (
    BUCKET_CLIPBOARD, BUCKET_NOTIFICATIONS, BUCKET_SNAPSHOTS, BUCKET_THEMES,
    BUCKET_PRESETS, BUCKET_DROP, BUCKET_VPN_STATE, BUCKET_CA_ROOT,
    BUCKET_SSH_KEYS, BUCKET_SSH_AUDIT,
)


@dataclass
class BucketEntry:
    bucket:    str
    peer:      str
    key:       str
    path:      Path
    revision:  int
    mtime:     float
    size:      int


def _mine_path(bucket: str) -> Path:
    return SYNC_ROOT_MINE / bucket


def _peer_path(peer: str, bucket: str) -> Path:
    return PEERS_ROOT / peer / ".qnm-sync" / bucket


def ensure_buckets() -> list[str]:
    actions: list[str] = []
    for b in ALL_BUCKETS:
        p = _mine_path(b)
        if not p.exists():
            p.mkdir(parents=True, exist_ok=True)
            actions.append(f"created bucket dir: {p}")
    return actions


# ---------------------------------------------------------------------------
# put / get / list / delete
# ---------------------------------------------------------------------------


def put(bucket: str, key: str, value, *, max_versions: int = 100) -> list[str]:
    """Publish to this peer's bucket. value: bytes | str | dict (JSON)."""
    d = _mine_path(bucket)
    d.mkdir(parents=True, exist_ok=True)
    key_dir = d / key
    key_dir.mkdir(exist_ok=True)

    # Pick next revision
    existing = sorted(key_dir.glob("v*.dat"),
                      key=lambda p: int(p.stem.lstrip("v") or "0"))
    next_rev = (int(existing[-1].stem.lstrip("v")) + 1) if existing else 1
    out = key_dir / f"v{next_rev}.dat"

    if isinstance(value, dict):
        out.write_text(json.dumps(value, indent=2), encoding="utf-8")
    elif isinstance(value, str):
        out.write_text(value, encoding="utf-8")
    else:
        out.write_bytes(bytes(value))

    # Rotate to max_versions
    versions = sorted(key_dir.glob("v*.dat"),
                      key=lambda p: int(p.stem.lstrip("v") or "0"))
    for old in versions[:-max_versions]:
        try:
            old.unlink()
        except OSError:
            pass

    # Maintain a "latest" symlink for easy reads
    latest = key_dir / "latest"
    if latest.is_symlink() or latest.exists():
        try:
            latest.unlink()
        except OSError:
            pass
    try:
        latest.symlink_to(out.name)
    except OSError:
        pass

    return [f"put {bucket}/{key} v{next_rev} ({out.stat().st_size} bytes)"]


def get(bucket: str, peer: str, key: str, *, revision: Optional[int] = None) -> Optional[bytes]:
    """Fetch a value. peer='*self*' means our own bucket."""
    base = _mine_path(bucket) if peer in ("*self*", socket.gethostname()) else _peer_path(peer, bucket)
    key_dir = base / key
    if not key_dir.exists():
        return None
    target: Path
    if revision is None:
        target = key_dir / "latest"
        if not target.exists():
            versions = sorted(key_dir.glob("v*.dat"),
                              key=lambda p: int(p.stem.lstrip("v") or "0"))
            if not versions:
                return None
            target = versions[-1]
    else:
        target = key_dir / f"v{revision}.dat"
    try:
        return target.read_bytes()
    except OSError:
        return None


def list_keys(bucket: str, *, peer: Optional[str] = None) -> list[BucketEntry]:
    """List every key in a bucket across peers (or one peer)."""
    out: list[BucketEntry] = []
    if peer is None:
        peers = ["*self*"]
        if PEERS_ROOT.exists():
            peers += [d.name for d in PEERS_ROOT.iterdir() if d.is_dir()]
    else:
        peers = [peer]
    for p in peers:
        base = _mine_path(bucket) if p == "*self*" else _peer_path(p, bucket)
        if not base.exists():
            continue
        for kd in base.iterdir():
            if not kd.is_dir():
                continue
            versions = sorted(kd.glob("v*.dat"),
                              key=lambda x: int(x.stem.lstrip("v") or "0"))
            if not versions:
                continue
            latest = versions[-1]
            try:
                st = latest.stat()
            except OSError:
                continue
            out.append(BucketEntry(
                bucket=bucket,
                peer=p if p != "*self*" else socket.gethostname(),
                key=kd.name,
                path=latest,
                revision=int(latest.stem.lstrip("v") or "0"),
                mtime=st.st_mtime,
                size=st.st_size,
            ))
    return out


def delete(bucket: str, key: str) -> list[str]:
    key_dir = _mine_path(bucket) / key
    if not key_dir.exists():
        return [f"no such key: {bucket}/{key}"]
    shutil.rmtree(key_dir)
    return [f"deleted {bucket}/{key}"]


def versions(bucket: str, peer: str, key: str) -> list[int]:
    base = _mine_path(bucket) if peer in ("*self*", socket.gethostname()) else _peer_path(peer, bucket)
    key_dir = base / key
    if not key_dir.exists():
        return []
    return sorted(
        int(p.stem.lstrip("v") or "0") for p in key_dir.glob("v*.dat")
    )


# ---------------------------------------------------------------------------
# Subscribe — file-watch style (used by mackes-meshd)
# ---------------------------------------------------------------------------


def poll_new(bucket: str, since: float) -> list[BucketEntry]:
    """Return entries with mtime > since across all peers."""
    return [e for e in list_keys(bucket) if e.mtime > since]


__all__ = [
    "BUCKET_CLIPBOARD", "BUCKET_NOTIFICATIONS", "BUCKET_SNAPSHOTS",
    "BUCKET_THEMES", "BUCKET_PRESETS", "BUCKET_DROP",
    "BUCKET_VPN_STATE", "BUCKET_CA_ROOT", "BUCKET_SSH_KEYS", "BUCKET_SSH_AUDIT",
    "ALL_BUCKETS", "BucketEntry",
    "ensure_buckets", "put", "get", "list_keys", "delete", "versions", "poll_new",
]
