"""mesh_gvfs — GVFS-backed mesh:// surface.

A real FUSE filesystem (via fusepy) that exposes the mesh as a navigable
tree at a fixed mount-point. GVFS sees the mount through its .mount file
registration and surfaces it under the `mesh:///` URI scheme so Thunar's
sidebar, location bar, and bookmark UI all treat it as a native
filesystem.

Layout (matches §8.10 spec):

    mesh:///
    ├── Peers/<peer>/                      (live SSHFS — pass-through)
    ├── Clipboard/<peer>/                  (mesh_sync clipboard bucket)
    │   └── Saved/                         (pinned items, uncapped)
    ├── Notifications/<peer>/              (mesh_sync notifications bucket)
    └── Object Store/<bucket>/             (themes / snapshots / presets /
                                            drop / ca-root / ssh-keys /
                                            vpn-state / ssh-audit)

Reads enumerate live state; writes flow back to mesh_sync.put() (for the
bucket subtrees) or to the local file system (for the Peers subtree —
which is itself sshfs-mounted by mesh_fs).

All entry points:

  - `python3 -m mackes.mesh_gvfs.daemon` — start the FUSE backend
  - /usr/bin/mackes-gvfsd-mesh           — shell wrapper for systemd
  - data/systemd/mackes-gvfsd-mesh.service (user unit)
  - data/gvfs/mesh.mount                  (GVFS mount manifest)
  - data/applications/mackes-mesh-uri-handler.desktop (mesh:// handler)
"""
from __future__ import annotations

from mackes.mesh_gvfs.uri import parse_mesh_uri, MeshPath


__all__ = ["parse_mesh_uri", "MeshPath"]
