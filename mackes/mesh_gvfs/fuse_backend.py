"""FUSE filesystem implementation for mesh:// via fusepy.

Exposes the mesh as a real filesystem at ~/.local/share/mackes-mesh-fuse/.
GVFS registers a `mesh://` URI handler against this mount-point so Thunar's
location bar and bookmark UI surface the filesystem as if it were a true
GVFS backend.

Operations dispatch to mackes.mesh_gvfs.operations which translates each
virtual path into a mesh_sync / mesh_fs operation.
"""
from __future__ import annotations

import errno
import os
import stat as stat_mod
import sys
from pathlib import Path
from typing import Any

try:
    from fuse import FUSE, FuseOSError, Operations
except ImportError:
    FUSE = None  # type: ignore[misc]
    FuseOSError = OSError  # type: ignore[misc,assignment]

    class Operations:  # type: ignore[no-redef]
        """Fallback stub when fusepy isn't installed.

        Importing this module without fusepy still works; only the
        `MeshFuse.run()` path is gated on the real library.
        """


from mackes.logging import log_action
from mackes.mesh_gvfs import operations as ops
from mackes.mesh_gvfs.uri import MOUNT_POINT


class MeshFuse(Operations):
    """fusepy Operations subclass implementing the mesh:// surface."""

    use_ns = True   # nanosecond timestamps (modern fusepy)

    # ----------------------------------------------------------------- meta
    def __init__(self) -> None:
        self._open_handles: dict[int, dict[str, Any]] = {}
        self._next_fh = 1

    # ----------------------------------------------------------------- stat
    def access(self, path: str, mode: int) -> None:
        a = ops.attr(path)
        if a is None:
            raise FuseOSError(errno.ENOENT)

    def getattr(self, path: str, fh: int | None = None) -> dict:
        a = ops.attr(path)
        if a is None:
            raise FuseOSError(errno.ENOENT)
        return a.asdict()

    def readdir(self, path: str, fh: int | None = None) -> list[str]:
        entries = [".", ".."] + ops.readdir(path)
        # Sanitize: yield only basenames
        return [e for e in entries if "/" not in e]

    def statfs(self, path: str) -> dict:
        # Pretend to have plenty of room — the underlying buckets can grow
        return {
            "f_bsize":  4096,
            "f_blocks": 1 << 30,
            "f_bfree":  1 << 28,
            "f_bavail": 1 << 28,
            "f_namemax": 255,
        }

    # ------------------------------------------------------------- file I/O
    def open(self, path: str, flags: int) -> int:
        a = ops.attr(path)
        if a is None:
            raise FuseOSError(errno.ENOENT)
        fh = self._next_fh
        self._next_fh += 1
        self._open_handles[fh] = {"path": path, "flags": flags}
        return fh

    def release(self, path: str, fh: int) -> None:
        self._open_handles.pop(fh, None)

    def read(self, path: str, size: int, offset: int, fh: int | None = None) -> bytes:
        try:
            return ops.read_bytes(path, offset, size)
        except OSError as e:
            raise FuseOSError(e.errno or errno.EIO) from e

    def write(self, path: str, data: bytes, offset: int, fh: int | None = None) -> int:
        try:
            return ops.write_bytes(path, data, offset)
        except OSError as e:
            raise FuseOSError(e.errno or errno.EIO) from e

    def create(self, path: str, mode: int, fi: Any = None) -> int:
        try:
            ops.create(path, mode)
        except OSError as e:
            raise FuseOSError(e.errno or errno.EIO) from e
        return self.open(path, os.O_RDWR)

    def truncate(self, path: str, length: int, fh: int | None = None) -> None:
        a = ops.attr(path)
        if a is None:
            raise FuseOSError(errno.ENOENT)
        if length == 0:
            # Re-create the underlying key as empty
            try:
                ops.create(path, 0o644)
            except OSError as e:
                raise FuseOSError(e.errno or errno.EIO) from e

    def unlink(self, path: str) -> None:
        try:
            ops.unlink(path)
        except OSError as e:
            raise FuseOSError(e.errno or errno.EIO) from e

    def mkdir(self, path: str, mode: int) -> None:
        # mkdir semantics: creating a subdirectory under Clipboard/peer
        # or under Object Store/bucket is essentially a no-op — the
        # mesh-sync layer auto-creates parent keys as soon as a file is
        # placed. So this just succeeds without doing anything.
        a = ops.attr(path)
        if a is not None and (a.mode & stat_mod.S_IFDIR):
            return
        # The fact that ops.attr returned None is fine — the directory
        # will appear once a key is created underneath. Permit.

    def rmdir(self, path: str) -> None:
        # Likewise a no-op: empty bucket subdirs disappear naturally.
        return

    def utimens(self, path: str, times: tuple[int, int] | None = None) -> None:
        # No-op: mesh_sync versions carry their own mtime.
        return

    # ---------------------------------------------------------------- mount
    def run(self) -> int:
        """Mount the filesystem at MOUNT_POINT (foreground)."""
        if FUSE is None:
            print("ERROR: fusepy (python3-fusepy) not installed; cannot mount.",
                  file=sys.stderr)
            return 127
        Path(MOUNT_POINT).mkdir(parents=True, exist_ok=True)
        log_action(f"gvfsd-mesh: mounting at {MOUNT_POINT}")
        FUSE(self, MOUNT_POINT, foreground=True, nothreads=False,
             allow_other=False, fsname="mesh", subtype="mackes-mesh",
             default_permissions=False)
        log_action("gvfsd-mesh: unmounted")
        return 0


__all__ = ["MeshFuse"]
