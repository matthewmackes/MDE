"""Snapshot create / list / restore (Q10 lock: manual snapshots only).

A snapshot is a timestamped directory under
`~/.local/share/mackes-shell/snapshots/`:

    snapshots/2026-05-15T142300_pre-display/
    ├── manifest.json       # name, timestamp, hostname, source preset
    ├── xfconf/             # `xfconf-query --channel X --list -v` per channel
    └── xfce4/              # full copy of ~/.config/xfce4

Restore wipes the live config dirs and copies snapshot contents back, then
loads the xfconf dumps. xfsettingsd applies live; no service restart.
"""
from __future__ import annotations

import json
import re
import shutil
import socket
import subprocess
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Optional

from mackes.logging import log_action
from mackes.state import HOME, SNAPSHOT_DIR, ensure_dirs


XFCONF_CHANNELS = [
    "xsettings",
    "xfwm4",
    "xfce4-desktop",
    "xfce4-panel",
    "xfce4-session",
    "xfce4-notifyd",
    "xfce4-power-manager",
    "keyboards",
    "keyboard-layout",
    "pointers",
    "displays",
    "thunar-volman",
]

LIVE_CONFIG_DIRS = {
    "xfce4": HOME / ".config" / "xfce4",
}


# ---------------------------------------------------------------------------
# Model
# ---------------------------------------------------------------------------


@dataclass
class Snapshot:
    path: Path

    @property
    def name(self) -> str:
        return self.path.name

    @property
    def created(self) -> datetime:
        return datetime.fromtimestamp(self.path.stat().st_mtime)

    def manifest(self) -> dict:
        mf = self.path / "manifest.json"
        if mf.exists():
            try:
                return json.loads(mf.read_text(encoding="utf-8"))
            except json.JSONDecodeError:
                pass
        return {}

    def display_label(self) -> str:
        m = self.manifest()
        label = m.get("label") or self.name
        return f"{self.created:%Y-%m-%d %H:%M}  —  {label}"


# ---------------------------------------------------------------------------
# Operations
# ---------------------------------------------------------------------------


def _slug(label: str) -> str:
    s = re.sub(r"[^A-Za-z0-9._-]+", "-", label.strip().lower())
    return s.strip("-") or "snapshot"


def _ts() -> str:
    return datetime.now().strftime("%Y-%m-%dT%H%M%S")


def list_snapshots() -> list[Snapshot]:
    if not SNAPSHOT_DIR.exists():
        return []
    snaps = [Snapshot(p) for p in SNAPSHOT_DIR.iterdir() if p.is_dir()]
    snaps.sort(key=lambda s: s.path.stat().st_mtime, reverse=True)
    return snaps


def create_snapshot(label: str = "snapshot", *, source_preset: Optional[str] = None) -> Snapshot:
    ensure_dirs()
    dest = SNAPSHOT_DIR / f"{_ts()}_{_slug(label)}"
    dest.mkdir(parents=True)

    # 1. xfconf channel dumps
    xf_dir = dest / "xfconf"
    xf_dir.mkdir()
    for channel in XFCONF_CHANNELS:
        try:
            out = subprocess.check_output(
                ["xfconf-query", "--channel", channel, "--list", "--verbose"],
                stderr=subprocess.DEVNULL, text=True,
            )
        except (FileNotFoundError, subprocess.CalledProcessError):
            continue
        if out.strip():
            (xf_dir / f"{channel}.txt").write_text(out, encoding="utf-8")

    # 2. config tree copies
    for name, src in LIVE_CONFIG_DIRS.items():
        if src.exists():
            shutil.copytree(src, dest / name, symlinks=True, dirs_exist_ok=True)

    # 3. manifest
    manifest = {
        "label": label,
        "created": datetime.now().isoformat(timespec="seconds"),
        "hostname": socket.gethostname(),
        "source_preset": source_preset,
        "channels": [c for c in XFCONF_CHANNELS if (xf_dir / f"{c}.txt").exists()],
    }
    (dest / "manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")

    log_action(f"snapshot created: {dest.name} (label={label!r})")
    return Snapshot(dest)


def _xfconf_load_dump(channel: str, dump_path: Path) -> bool:
    """Re-apply a `--list --verbose` dump by parsing it line-by-line.

    `xfconf-query --load` works on `--export` XML output, not `--list -v` text,
    so we do it the manual way: each non-blank line is `<key>  <value>`.
    Booleans round-trip as 'true'/'false'; numbers via int/float; everything
    else as string.
    """
    if not dump_path.exists():
        return False
    bridge_imported = False
    try:
        from mackes.xfconf_bridge import get_bridge
        xf = get_bridge()
        bridge_imported = True
    except Exception:
        return False

    for raw in dump_path.read_text(encoding="utf-8").splitlines():
        line = raw.rstrip()
        if not line or line.startswith("Property"):
            continue
        # Format: <key><whitespace><value>
        parts = line.split(None, 1)
        if len(parts) != 2:
            continue
        key, value = parts
        if not key.startswith("/"):
            continue
        try:
            if value == "true":
                xf.set(channel, key, True)
            elif value == "false":
                xf.set(channel, key, False)
            elif re.fullmatch(r"-?\d+", value):
                xf.set(channel, key, int(value))
            elif re.fullmatch(r"-?\d+\.\d+", value):
                xf.set(channel, key, float(value))
            else:
                xf.set(channel, key, value, type_hint="string")
        except Exception as e:  # noqa: BLE001
            log_action(f"snapshot restore: skip {channel}{key}: {e}")
    return bridge_imported


def restore_snapshot(snap: Snapshot) -> list[str]:
    actions: list[str] = [f"--- restoring snapshot {snap.name} ---"]

    # 1. config trees
    for name, dest in LIVE_CONFIG_DIRS.items():
        src = snap.path / name
        if not src.exists():
            continue
        if dest.exists():
            shutil.rmtree(dest)
        shutil.copytree(src, dest, symlinks=True)
        actions.append(f"restored {name} -> {dest}")

    # 2. xfconf
    xf_dir = snap.path / "xfconf"
    if xf_dir.exists():
        for dump in sorted(xf_dir.glob("*.txt")):
            channel = dump.stem
            if _xfconf_load_dump(channel, dump):
                actions.append(f"restored xfconf channel: {channel}")

    for line in actions:
        log_action(line)
    return actions


def delete_snapshot(snap: Snapshot) -> None:
    if snap.path.exists():
        shutil.rmtree(snap.path)
        log_action(f"snapshot deleted: {snap.name}")
