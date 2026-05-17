"""Entry point: `python3 -m mackes.mesh_gvfs.daemon` (used by systemd).

Mounts the mesh-FUSE filesystem at MOUNT_POINT and runs in the
foreground until SIGTERM. Registers the mesh:// URI handler with GVFS
on startup if not already registered.
"""
from __future__ import annotations

import signal
import sys

from mackes.logging import log_action
from mackes.mesh_gvfs.fuse_backend import MeshFuse
from mackes.mesh_gvfs.uri import MOUNT_POINT


_RUNNING = True


def _sigterm(_a, _b):
    global _RUNNING
    _RUNNING = False


def main() -> int:
    signal.signal(signal.SIGTERM, _sigterm)
    signal.signal(signal.SIGINT,  _sigterm)
    log_action(f"gvfsd-mesh: starting, mount={MOUNT_POINT}")
    fs = MeshFuse()
    return fs.run()


if __name__ == "__main__":
    sys.exit(main())
