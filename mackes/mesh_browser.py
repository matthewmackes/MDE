"""§8.10 mesh:// browser — Thunar-friendly mesh surface.

The spec calls for a custom gvfsd-mesh GVFS backend that exposes
`mesh:///` with subtrees for Peers / Clipboard / Notifications /
Object Store. The full GVFS backend is multi-month work (D-Bus, mount
protocol, gvfs-daemon integration); for Mackes 1.0 we achieve the same
user-facing behavior via real directories under `~/QNM-Mesh/` (already
mounted by mesh_fs) augmented with `~/QNM-Clipboard/`,
`~/QNM-Notifications/`, `~/QNM-Drop/` — Thunar treats them as native
folders, supports live updates via inotify, and the user gets identical
right-click drag-drop semantics.

Maintained layout (every peer):

    ~/QNM-Mesh/<peer>/                  (sshfs mount — peer's QNM-Shared)
    ~/QNM-Clipboard/
        mine/                            (local 100-item ring)
        peer-A/                          (mirror of peer A's ring)
        .../Saved/                       (pinned items, uncapped)
    ~/QNM-Notifications/
        mine/                            (this peer's notifications)
        peer-A/                          (peer A's notifications, .md per)
    ~/QNM-Drop/                          (NATS Object-Store-equivalent)
        Themes/  Snapshots/  Presets/  Drop/

The mesh-meshd daemon keeps these directories in sync with the actual
mesh-sync buckets. Thunar bookmarks point at each top-level dir so they
appear in the sidebar.

This module is also what `mackes shares` reads.
"""
from __future__ import annotations

import os
import shutil
import socket
from pathlib import Path
from typing import Iterable

from mackes.logging import log_action
from mackes.mesh_sync import (
    ALL_BUCKETS, BUCKET_CLIPBOARD, BUCKET_DROP, BUCKET_NOTIFICATIONS,
    BUCKET_PRESETS, BUCKET_SNAPSHOTS, BUCKET_THEMES,
    list_keys,
)
from mackes.state import HOME


# Top-level user-facing directories
DIR_MESH         = HOME / "QNM-Mesh"
DIR_CLIPBOARD    = HOME / "QNM-Clipboard"
DIR_NOTIFICATIONS = HOME / "QNM-Notifications"
DIR_DROP         = HOME / "QNM-Drop"

# Map mesh-sync bucket name -> top-level user-facing folder
BUCKET_VIEWS = {
    BUCKET_CLIPBOARD:     DIR_CLIPBOARD,
    BUCKET_NOTIFICATIONS: DIR_NOTIFICATIONS,
    BUCKET_DROP:          DIR_DROP / "Drop",
    BUCKET_THEMES:        DIR_DROP / "Themes",
    BUCKET_SNAPSHOTS:     DIR_DROP / "Snapshots",
    BUCKET_PRESETS:       DIR_DROP / "Presets",
}


def ensure_layout() -> list[str]:
    """Create all the top-level mesh-view directories. Idempotent."""
    actions: list[str] = []
    for d in (DIR_MESH, DIR_CLIPBOARD, DIR_NOTIFICATIONS, DIR_DROP):
        if not d.exists():
            d.mkdir(parents=True, exist_ok=True)
            actions.append(f"created {d}")
    for sub in BUCKET_VIEWS.values():
        if not sub.exists():
            sub.mkdir(parents=True, exist_ok=True)
            actions.append(f"created {sub}")
    # mine/ subdir for clipboard + notifications
    for parent in (DIR_CLIPBOARD, DIR_NOTIFICATIONS):
        mine = parent / "mine"
        if not mine.exists():
            mine.mkdir(parents=True, exist_ok=True)
            actions.append(f"created {mine}")
        saved = mine / "Saved"
        if not saved.exists() and parent == DIR_CLIPBOARD:
            saved.mkdir(parents=True, exist_ok=True)
            actions.append(f"created {saved}")
    for line in actions:
        log_action(line)
    return actions


def install_thunar_bookmarks() -> list[str]:
    """Add Mackes mesh bookmarks to ~/.config/gtk-3.0/bookmarks."""
    bookmarks = HOME / ".config" / "gtk-3.0" / "bookmarks"
    bookmarks.parent.mkdir(parents=True, exist_ok=True)
    want = [
        (DIR_MESH,          "Mesh Peers"),
        (DIR_CLIPBOARD,     "Mesh Clipboard"),
        (DIR_NOTIFICATIONS, "Mesh Notifications"),
        (DIR_DROP,          "Mesh Drop"),
    ]
    existing = []
    if bookmarks.exists():
        try:
            existing = bookmarks.read_text(encoding="utf-8").splitlines()
        except OSError:
            existing = []
    # Drop any existing Mackes mesh bookmarks (we re-add canonically)
    keep = [
        ln for ln in existing
        if "QNM-Mesh" not in ln
        and "QNM-Clipboard" not in ln
        and "QNM-Notifications" not in ln
        and "QNM-Drop" not in ln
    ]
    new_lines = keep + [
        f"file://{d.as_posix()} {label}"
        for d, label in want
    ]
    bookmarks.write_text("\n".join(new_lines) + "\n", encoding="utf-8")
    return [f"installed Thunar bookmarks (4 mesh entries)"]


def sync_clipboard_view() -> list[str]:
    """Materialize NATS clipboard entries into ~/QNM-Clipboard/<peer>/.

    Each bucket entry becomes a file with the entry's filename as the
    last segment of its key. Idempotent — sync_loop calls this on every
    pass.
    """
    actions: list[str] = []
    me = socket.gethostname()
    for entry in list_keys(BUCKET_CLIPBOARD):
        peer_dir = DIR_CLIPBOARD / entry.peer
        peer_dir.mkdir(parents=True, exist_ok=True)
        dest = peer_dir / entry.key
        try:
            data = entry.path.read_bytes()
            if not dest.exists() or dest.stat().st_size != len(data):
                dest.write_bytes(data)
                actions.append(f"synced clipboard {entry.peer}/{entry.key}")
        except OSError as e:
            actions.append(f"clipboard sync error {entry.path}: {e}")
    return actions


def sync_notifications_view() -> list[str]:
    actions: list[str] = []
    for entry in list_keys(BUCKET_NOTIFICATIONS):
        peer_dir = DIR_NOTIFICATIONS / entry.peer
        peer_dir.mkdir(parents=True, exist_ok=True)
        dest = peer_dir / entry.key
        try:
            data = entry.path.read_bytes()
            if not dest.exists() or dest.stat().st_size != len(data):
                dest.write_bytes(data)
                actions.append(f"synced notification {entry.peer}/{entry.key}")
        except OSError as e:
            actions.append(f"notif sync error {entry.path}: {e}")
    return actions


def sync_all() -> list[str]:
    """Single-pass sync; called by mackes-meshd."""
    actions: list[str] = []
    actions.extend(ensure_layout())
    actions.extend(sync_clipboard_view())
    actions.extend(sync_notifications_view())
    return actions


__all__ = [
    "DIR_MESH", "DIR_CLIPBOARD", "DIR_NOTIFICATIONS", "DIR_DROP",
    "ensure_layout", "install_thunar_bookmarks",
    "sync_clipboard_view", "sync_notifications_view", "sync_all",
]
