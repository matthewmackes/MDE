"""Per-subtree path-classification + virtual-file generation.

The FUSE backend dispatches every getattr/readdir/open/read/write/create/
unlink call to one of four "operation handlers" (peers / clipboard /
notifications / object_store) based on the leading path segment. Each
handler maps virtual filesystem paths to underlying mesh state.
"""
from __future__ import annotations

import errno
import os
import stat as stat_mod
import socket
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Optional

from mackes.mesh_fs import QNM_MESH
from mackes.mesh_sync import (
    BUCKET_CLIPBOARD, BUCKET_NOTIFICATIONS, ALL_BUCKETS,
    BUCKET_DROP, BUCKET_THEMES, BUCKET_PRESETS, BUCKET_SNAPSHOTS,
    BUCKET_VPN_STATE, BUCKET_CA_ROOT, BUCKET_SSH_KEYS, BUCKET_SSH_AUDIT,
    list_keys, get, put, delete,
)


# Object-Store buckets exposed as folders under the mesh:// surface
EXPOSED_BUCKETS = (
    BUCKET_DROP, BUCKET_THEMES, BUCKET_PRESETS, BUCKET_SNAPSHOTS,
    BUCKET_VPN_STATE, BUCKET_CA_ROOT, BUCKET_SSH_KEYS, BUCKET_SSH_AUDIT,
)


@dataclass
class FsAttr:
    mode:  int = 0o755 | stat_mod.S_IFDIR
    nlink: int = 2
    size:  int = 0
    mtime: float = 0.0
    uid:   int = os.getuid()
    gid:   int = os.getgid()

    def asdict(self) -> dict[str, int | float]:
        st = {
            "st_mode":  self.mode,
            "st_nlink": self.nlink,
            "st_size":  self.size,
            "st_mtime": self.mtime,
            "st_atime": self.mtime,
            "st_ctime": self.mtime,
            "st_uid":   self.uid,
            "st_gid":   self.gid,
        }
        return st


# Plain file with read-only permission default
def _dir(mode: int = 0o755) -> FsAttr:
    return FsAttr(mode=mode | stat_mod.S_IFDIR, nlink=2, size=4096,
                  mtime=time.time())


def _file(size: int, mtime: float, mode: int = 0o644) -> FsAttr:
    return FsAttr(mode=mode | stat_mod.S_IFREG, nlink=1,
                  size=size, mtime=mtime)


# ---------------------------------------------------------------------------
# Path classification + virtual operations
# ---------------------------------------------------------------------------


def _parts(path: str) -> list[str]:
    return [p for p in path.split("/") if p]


def attr(path: str) -> Optional[FsAttr]:
    """getattr — return None if the path doesn't exist."""
    parts = _parts(path)
    if not parts:
        return _dir()

    head = parts[0]

    # ---- Peers/<peer>/<sub>... — pass-through to sshfs mount ----------
    if head == "Peers":
        if len(parts) == 1:
            return _dir()
        peer = parts[1]
        if len(parts) == 2:
            # peer dir — check sshfs mount
            target = QNM_MESH / peer
            if target.exists():
                try:
                    s = target.stat()
                    return _file(0, s.st_mtime, mode=0o755) if False else FsAttr(
                        mode=stat_mod.S_IFDIR | 0o755, nlink=2, size=4096,
                        mtime=s.st_mtime,
                    )
                except OSError:
                    return None
            return None
        # Deeper paths within a peer mount — forward to the real fs
        target = QNM_MESH / peer / "/".join(parts[2:])
        if not target.exists():
            return None
        try:
            s = target.stat()
        except OSError:
            return None
        is_dir = stat_mod.S_ISDIR(s.st_mode)
        return FsAttr(
            mode=s.st_mode,
            nlink=s.st_nlink,
            size=s.st_size,
            mtime=s.st_mtime,
        )

    # ---- Clipboard / Notifications / Object Store ---------------------
    if head in ("Clipboard", "Notifications"):
        bucket = BUCKET_CLIPBOARD if head == "Clipboard" else BUCKET_NOTIFICATIONS
        if len(parts) == 1:
            return _dir()
        # Top-level peer name
        peer = parts[1]
        peers = {e.peer for e in list_keys(bucket)}
        peers.add(socket.gethostname())   # always have a 'mine' entry concept
        if peer not in peers and peer != "mine":
            return None
        if len(parts) == 2:
            return _dir()
        # Saved/ subfolder (clipboard only)
        if head == "Clipboard" and parts[2] == "Saved":
            if len(parts) == 3:
                return _dir()
            # Saved/<key>
            entries = list_keys(bucket, peer=peer)
            key = "Saved/" + "/".join(parts[3:])
            for e in entries:
                if e.key == key:
                    return _file(e.size, e.mtime)
            return None
        # Direct entry: Clipboard/<peer>/<key>
        key = "/".join(parts[2:])
        for e in list_keys(bucket, peer=peer):
            if e.key == key:
                return _file(e.size, e.mtime)
        return None

    if head in ("Object Store", "ObjectStore"):
        if len(parts) == 1:
            return _dir()
        bucket = parts[1]
        if bucket not in EXPOSED_BUCKETS:
            return None
        if len(parts) == 2:
            return _dir()
        # bucket/<peer>/<key>?  Object Store flattens peer into the key
        # presentation: a single file per (peer, key) tuple, named
        # "<key>" if there's only one peer or "<peer>/<key>" if multiple.
        # For consistency: bucket/<key>
        rest = "/".join(parts[2:])
        # Look up the key across all peers; return the latest revision.
        for e in list_keys(bucket):
            if e.key == rest:
                return _file(e.size, e.mtime)
        # Sub-key directory case
        for e in list_keys(bucket):
            if e.key.startswith(rest + "/"):
                return _dir()
        return None

    return None


def readdir(path: str) -> list[str]:
    """readdir — return list of entries; empty list for empty dir."""
    parts = _parts(path)
    if not parts:
        return ["Peers", "Clipboard", "Notifications", "Object Store"]

    head = parts[0]

    if head == "Peers":
        if len(parts) == 1:
            if QNM_MESH.exists():
                return [d.name for d in QNM_MESH.iterdir() if d.is_dir()]
            return []
        # Inside a peer mount
        target = QNM_MESH.joinpath(*parts[1:])
        if not target.is_dir():
            return []
        try:
            return [c.name for c in target.iterdir()]
        except OSError:
            return []

    if head in ("Clipboard", "Notifications"):
        bucket = BUCKET_CLIPBOARD if head == "Clipboard" else BUCKET_NOTIFICATIONS
        if len(parts) == 1:
            peers = {e.peer for e in list_keys(bucket)}
            peers.add(socket.gethostname())
            return sorted(peers)
        peer = parts[1]
        if len(parts) == 2:
            keys = [e.key for e in list_keys(bucket, peer=peer)]
            if head == "Clipboard":
                # Filter out Saved/* entries — they appear under the
                # Saved/ folder, not at the top of the peer dir.
                top = [k for k in keys if not k.startswith("Saved/")]
                if any(k.startswith("Saved/") for k in keys):
                    top.append("Saved")
                return sorted(set(top))
            return sorted(set(keys))
        if head == "Clipboard" and len(parts) == 3 and parts[2] == "Saved":
            return sorted({
                e.key.split("Saved/", 1)[1]
                for e in list_keys(bucket, peer=peer)
                if e.key.startswith("Saved/")
            })
        return []

    if head in ("Object Store", "ObjectStore"):
        if len(parts) == 1:
            return sorted(EXPOSED_BUCKETS)
        bucket = parts[1]
        if bucket not in EXPOSED_BUCKETS:
            return []
        prefix = "/".join(parts[2:])
        seen: set[str] = set()
        for e in list_keys(bucket):
            if prefix and not e.key.startswith(prefix + "/") and e.key != prefix:
                continue
            sub = e.key[len(prefix) + 1:] if prefix else e.key
            # Take only the first segment after prefix (directory-like
            # collapse so callers see nested keys as subfolders)
            first = sub.split("/", 1)[0]
            seen.add(first)
        return sorted(seen)

    return []


def read_bytes(path: str, offset: int, size: int) -> bytes:
    parts = _parts(path)
    if not parts:
        raise OSError(errno.EISDIR, "is a directory")
    head = parts[0]

    if head == "Peers" and len(parts) >= 3:
        target = QNM_MESH.joinpath(*parts[1:])
        try:
            with open(target, "rb") as f:
                f.seek(offset)
                return f.read(size)
        except OSError as e:
            raise OSError(e.errno or errno.EIO, str(e)) from e

    if head in ("Clipboard", "Notifications"):
        bucket = BUCKET_CLIPBOARD if head == "Clipboard" else BUCKET_NOTIFICATIONS
        peer = parts[1] if len(parts) >= 2 else None
        if peer is None:
            raise OSError(errno.EISDIR, "is a directory")
        key = "/".join(parts[2:])
        data = get(bucket, peer, key) or b""
        return data[offset:offset + size]

    if head in ("Object Store", "ObjectStore") and len(parts) >= 3:
        bucket = parts[1]
        rest = "/".join(parts[2:])
        # Find owning peer for this key
        for e in list_keys(bucket):
            if e.key == rest:
                data = get(bucket, e.peer, rest) or b""
                return data[offset:offset + size]
        raise OSError(errno.ENOENT, "no such file")

    raise OSError(errno.ENOENT, "no such file")


def write_bytes(path: str, data: bytes, offset: int) -> int:
    """Writes. For Object Store + Clipboard buckets, this calls
    mesh_sync.put() with the merged bytes."""
    parts = _parts(path)
    if not parts:
        raise OSError(errno.EISDIR, "is a directory")
    head = parts[0]

    if head == "Peers" and len(parts) >= 3:
        target = QNM_MESH.joinpath(*parts[1:])
        target.parent.mkdir(parents=True, exist_ok=True)
        try:
            mode = "rb+" if target.exists() else "wb"
            with open(target, mode) as f:
                f.seek(offset)
                f.write(data)
            return len(data)
        except OSError as e:
            raise OSError(e.errno or errno.EIO, str(e)) from e

    if head in ("Clipboard", "Notifications"):
        bucket = BUCKET_CLIPBOARD if head == "Clipboard" else BUCKET_NOTIFICATIONS
        if len(parts) < 3:
            raise OSError(errno.EISDIR, "must specify a key")
        peer = parts[1]
        if peer not in ("mine", socket.gethostname()):
            raise OSError(errno.EACCES, "can only write to your own bucket")
        key = "/".join(parts[2:])
        # Read-modify-write — caller might be appending or middle-writing
        existing = get(bucket, "*self*", key) or b""
        if offset == 0 and not existing:
            new = data
        else:
            buf = bytearray(existing)
            if offset > len(buf):
                buf.extend(b"\x00" * (offset - len(buf)))
            buf[offset:offset + len(data)] = data
            new = bytes(buf)
        put(bucket, key, new)
        return len(data)

    if head in ("Object Store", "ObjectStore") and len(parts) >= 3:
        bucket = parts[1]
        if bucket not in EXPOSED_BUCKETS:
            raise OSError(errno.ENOENT, "no such bucket")
        rest = "/".join(parts[2:])
        existing = get(bucket, "*self*", rest) or b""
        if offset == 0 and not existing:
            new = data
        else:
            buf = bytearray(existing)
            if offset > len(buf):
                buf.extend(b"\x00" * (offset - len(buf)))
            buf[offset:offset + len(data)] = data
            new = bytes(buf)
        put(bucket, rest, new)
        return len(data)

    raise OSError(errno.EACCES, "writes not supported here")


def create(path: str, mode: int) -> None:
    """create — establish an empty key. mode is the POSIX mode bits."""
    parts = _parts(path)
    head = parts[0] if parts else None

    if head == "Peers" and len(parts) >= 3:
        target = QNM_MESH.joinpath(*parts[1:])
        target.parent.mkdir(parents=True, exist_ok=True)
        target.touch()
        return

    if head in ("Clipboard", "Notifications", "Object Store", "ObjectStore"):
        bucket = {
            "Clipboard":     BUCKET_CLIPBOARD,
            "Notifications": BUCKET_NOTIFICATIONS,
        }.get(head)
        if bucket is None:
            if len(parts) < 3:
                raise OSError(errno.EISDIR, "specify a key")
            bucket = parts[1]
            if bucket not in EXPOSED_BUCKETS:
                raise OSError(errno.ENOENT, "no such bucket")
            rest = "/".join(parts[2:])
            put(bucket, rest, b"")
            return
        if len(parts) < 3:
            raise OSError(errno.EISDIR, "specify a key")
        peer = parts[1]
        if peer not in ("mine", socket.gethostname()):
            raise OSError(errno.EACCES, "can only create in your own bucket")
        key = "/".join(parts[2:])
        put(bucket, key, b"")
        return

    raise OSError(errno.EACCES, "cannot create here")


def unlink(path: str) -> None:
    parts = _parts(path)
    if not parts:
        raise OSError(errno.EISDIR, "is the root")
    head = parts[0]
    if head == "Peers" and len(parts) >= 3:
        target = QNM_MESH.joinpath(*parts[1:])
        try:
            target.unlink()
        except OSError as e:
            raise OSError(e.errno or errno.EIO, str(e)) from e
        return
    if head in ("Clipboard", "Notifications"):
        bucket = BUCKET_CLIPBOARD if head == "Clipboard" else BUCKET_NOTIFICATIONS
        if len(parts) < 3:
            raise OSError(errno.EISDIR, "specify a key")
        peer = parts[1]
        if peer not in ("mine", socket.gethostname()):
            raise OSError(errno.EACCES, "can only delete from your own bucket")
        key = "/".join(parts[2:])
        delete(bucket, key)
        return
    if head in ("Object Store", "ObjectStore") and len(parts) >= 3:
        bucket = parts[1]
        if bucket not in EXPOSED_BUCKETS:
            raise OSError(errno.ENOENT, "no such bucket")
        rest = "/".join(parts[2:])
        delete(bucket, rest)
        return
    raise OSError(errno.EACCES, "cannot delete here")


__all__ = [
    "FsAttr", "attr", "readdir", "read_bytes", "write_bytes",
    "create", "unlink", "EXPOSED_BUCKETS",
]
