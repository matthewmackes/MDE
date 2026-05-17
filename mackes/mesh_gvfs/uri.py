"""Parse + represent mesh:// URIs and their FUSE path translations."""
from __future__ import annotations

import os
import urllib.parse
from dataclasses import dataclass
from typing import Optional


# Where the FUSE filesystem is mounted on disk. GVFS overlays its
# `mesh:///` URI scheme onto this path via the shadow-mount mechanism
# (see data/gvfs/mesh.mount).
MOUNT_POINT = os.path.expanduser("~/.local/share/mackes-mesh-fuse")


@dataclass
class MeshPath:
    """A parsed mesh:// path.

    Either:
      kind=Peers,         peer=<name>, rel=<remainder>
      kind=Clipboard,     peer=<name>, rel=<remainder>
      kind=Notifications, peer=<name>, rel=<remainder>
      kind=ObjectStore,   bucket=<name>, rel=<remainder>
      kind=root           — the mesh:/// root listing
    """
    kind:   str = "root"      # 'root' | 'Peers' | 'Clipboard' | 'Notifications' | 'ObjectStore'
    peer:   Optional[str] = None
    bucket: Optional[str] = None
    rel:    str = ""          # path under the subtree (e.g., 'mine/foo.txt' or 'Saved/abc.png')

    def fs_path(self) -> str:
        """The on-disk path corresponding to this mesh:// path."""
        if self.kind == "root":
            return MOUNT_POINT
        if self.kind == "Peers":
            base = os.path.join(MOUNT_POINT, "Peers", self.peer or "")
            return os.path.join(base, self.rel) if self.rel else base
        if self.kind == "Clipboard":
            base = os.path.join(MOUNT_POINT, "Clipboard", self.peer or "")
            return os.path.join(base, self.rel) if self.rel else base
        if self.kind == "Notifications":
            base = os.path.join(MOUNT_POINT, "Notifications", self.peer or "")
            return os.path.join(base, self.rel) if self.rel else base
        if self.kind == "ObjectStore":
            base = os.path.join(MOUNT_POINT, "Object Store", self.bucket or "")
            return os.path.join(base, self.rel) if self.rel else base
        return MOUNT_POINT


def parse_mesh_uri(uri: str) -> MeshPath:
    """Parse a 'mesh:///<subtree>/<peer-or-bucket>/<rel>' URI."""
    if uri.startswith("mesh://"):
        path = uri[len("mesh://"):]
        if path.startswith("/"):
            path = path[1:]
    else:
        path = uri.lstrip("/")
    path = urllib.parse.unquote(path)

    if not path:
        return MeshPath()

    parts = path.split("/")
    head = parts[0]
    if head == "Peers" and len(parts) >= 2:
        return MeshPath(kind="Peers", peer=parts[1], rel="/".join(parts[2:]))
    if head == "Clipboard" and len(parts) >= 2:
        return MeshPath(kind="Clipboard", peer=parts[1], rel="/".join(parts[2:]))
    if head == "Notifications" and len(parts) >= 2:
        return MeshPath(kind="Notifications", peer=parts[1], rel="/".join(parts[2:]))
    if head in ("Object Store", "ObjectStore") and len(parts) >= 2:
        return MeshPath(kind="ObjectStore", bucket=parts[1], rel="/".join(parts[2:]))
    return MeshPath()


__all__ = ["MeshPath", "parse_mesh_uri", "MOUNT_POINT"]
