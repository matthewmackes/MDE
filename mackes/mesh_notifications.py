"""Mesh notifications — distributed `notify-send` over the mesh fabric.

Maps to:
  - `mackes notify <peer> "msg"` CLI subcommand (also `--all` for broadcast)
  - Cron / script integration on headless nodes (Q-HL7)
  - Notifications subtree of mesh:/// (rendered as .md files per §8.10)

Backend: notifications are written to a per-peer outbox under
mesh-fs (~/QNM-Mesh/<target-peer>/.qnm-notifications/) which the target
peer's qnmd watcher reads + dispatches to xfce4-notifyd via notify-send.
"""
from __future__ import annotations

import json
import os
import secrets
import shutil
import socket
import subprocess
import time
from pathlib import Path
from typing import Iterable, Optional

from mackes.logging import log_action
from mackes.state import CONFIG_DIR, DATA_DIR, HOME


MESH_NOTIF_INBOX = HOME / ".qnm-notifications"   # this peer's inbox
MESH_MOUNT_ROOT  = HOME / "QNM-Mesh"             # where remote peers' shares live


def _ensure_inbox() -> None:
    MESH_NOTIF_INBOX.mkdir(parents=True, exist_ok=True)


def send(target_peer: str, title: str, *, body: str = "",
         urgency: str = "normal", icon: str = "dialog-information") -> list[str]:
    """Drop a notification into the target peer's inbox.

    If the peer's mesh-fs mount exists at ~/QNM-Mesh/<peer>/.qnm-notifications/,
    write the .md there directly. Otherwise, queue it locally for the
    mackes-meshd daemon to flush when the mount becomes available.
    """
    actions: list[str] = []
    ts = time.strftime("%Y-%m-%dT%H-%M-%S")
    nid = secrets.token_hex(4)
    filename = f"{ts}_{socket.gethostname()}_{nid}.md"
    md = (
        "---\n"
        f"peer: {socket.gethostname()}\n"
        f"timestamp: {ts}\n"
        f"urgency: {urgency}\n"
        f"icon: {icon}\n"
        f"app: mackes-mesh-notify\n"
        "---\n"
        f"\n# {title}\n\n{body}\n"
    )

    if target_peer == "*":
        targets = _list_known_peers()
    else:
        targets = [target_peer]

    for peer in targets:
        peer_mount = MESH_MOUNT_ROOT / peer / ".qnm-notifications"
        if peer_mount.exists():
            try:
                peer_mount.mkdir(parents=True, exist_ok=True)
                (peer_mount / filename).write_text(md, encoding="utf-8")
                actions.append(f"delivered to {peer}: {filename}")
                continue
            except OSError as e:
                actions.append(f"could not write to {peer_mount}: {e}")
        # Queue locally for the daemon to flush
        queue = CONFIG_DIR / "mesh-notify-queue" / peer
        queue.mkdir(parents=True, exist_ok=True)
        (queue / filename).write_text(md, encoding="utf-8")
        actions.append(f"queued for {peer} -> {queue / filename}")
    for line in actions:
        log_action(line)
    return actions


def _list_known_peers() -> list[str]:
    """Best-effort list of mesh peers (used by --all broadcasts)."""
    try:
        from mackes.mesh_vpn import headscale_list_peers
        return [p.name for p in headscale_list_peers() if p.name]
    except Exception:  # noqa: BLE001
        return [
            d.name for d in MESH_MOUNT_ROOT.iterdir()
            if MESH_MOUNT_ROOT.exists() and d.is_dir()
        ] if MESH_MOUNT_ROOT.exists() else []


def receive_loop_once() -> list[str]:
    """One pass of the inbox watcher — called by mackes-meshd.

    For every .md in MESH_NOTIF_INBOX, parse frontmatter, fire notify-send,
    then move the file into a 'read' archive.
    """
    actions: list[str] = []
    _ensure_inbox()
    archive = MESH_NOTIF_INBOX / "read"
    archive.mkdir(exist_ok=True)
    for md in sorted(MESH_NOTIF_INBOX.glob("*.md")):
        if md.parent != MESH_NOTIF_INBOX:
            continue
        try:
            text = md.read_text(encoding="utf-8")
        except OSError as e:
            actions.append(f"failed to read {md}: {e}")
            continue
        meta, body = _parse(text)
        title = body.split("\n", 1)[0].lstrip("# ").strip() or "Mesh notification"
        rest = body.split("\n", 1)[1].strip() if "\n" in body else ""
        urgency = meta.get("urgency", "normal")
        icon = meta.get("icon", "dialog-information")
        if shutil.which("notify-send"):
            subprocess.call([
                "notify-send",
                "--urgency=" + urgency,
                "--icon=" + icon,
                title, rest,
            ])
        actions.append(f"surfaced notification: {md.name}")
        shutil.move(str(md), str(archive / md.name))
    return actions


def _parse(text: str) -> tuple[dict[str, str], str]:
    meta: dict[str, str] = {}
    body = text
    if text.startswith("---\n"):
        end = text.find("\n---\n", 4)
        if end != -1:
            for ln in text[4:end].splitlines():
                if ":" in ln:
                    k, v = ln.split(":", 1)
                    meta[k.strip()] = v.strip()
            body = text[end + 5:]
    return meta, body


__all__ = ["send", "receive_loop_once"]
