"""Mackes runtime state — install status, active preset, drift detection.

A single JSON file at `~/.config/mackes-shell/state.json` is the source of truth
for whether Mackes has been provisioned on this machine, which preset is active,
and when the last apply happened. Everything else is read live from the system.
"""
from __future__ import annotations

import json
import os
import shutil
import socket
from dataclasses import asdict, dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Optional

HOME = Path.home()
CONFIG_DIR = Path(os.environ.get("XDG_CONFIG_HOME", HOME / ".config")) / "mackes-shell"
DATA_DIR = Path(os.environ.get("XDG_DATA_HOME", HOME / ".local/share")) / "mackes-shell"
STATE_FILE = CONFIG_DIR / "state.json"
SNAPSHOT_DIR = DATA_DIR / "snapshots"
LOG_DIR = DATA_DIR / "logs"


@dataclass
class MackesState:
    provisioned: bool = False
    active_preset: Optional[str] = None
    last_apply: Optional[str] = None  # ISO timestamp
    schema_version: int = 1
    notes: dict = field(default_factory=dict)

    @classmethod
    def load(cls) -> "MackesState":
        if not STATE_FILE.exists():
            return cls()
        try:
            data = json.loads(STATE_FILE.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return cls()
        return cls(**{k: v for k, v in data.items() if k in cls.__dataclass_fields__})

    def save(self) -> None:
        CONFIG_DIR.mkdir(parents=True, exist_ok=True)
        STATE_FILE.write_text(json.dumps(asdict(self), indent=2), encoding="utf-8")

    def mark_provisioned(self, preset: str) -> None:
        self.provisioned = True
        self.active_preset = preset
        self.last_apply = datetime.now().isoformat(timespec="seconds")
        self.save()


def ensure_dirs() -> None:
    for d in (CONFIG_DIR, DATA_DIR, SNAPSHOT_DIR, LOG_DIR):
        d.mkdir(parents=True, exist_ok=True)


# ----- Service / dependency probes ------------------------------------------


def have(cmd: str) -> bool:
    return shutil.which(cmd) is not None


def is_running(name: str) -> bool:
    """Return True if a process by name appears in `pgrep -x` results."""
    if not have("pgrep"):
        return False
    import subprocess
    try:
        subprocess.check_output(["pgrep", "-x", name], stderr=subprocess.DEVNULL)
        return True
    except subprocess.CalledProcessError:
        return False


def service_health() -> dict[str, str]:
    """Return a dict mapping service name -> 'ok' | 'warn' | 'fail' | 'missing'.

    These drive the dashboard status strip.
    """
    return {
        "xfce4-panel": "ok" if is_running("xfce4-panel") else ("warn" if have("xfce4-panel") else "fail"),
        "xfdesktop": "ok" if is_running("xfdesktop") else ("warn" if have("xfdesktop") else "fail"),
        "xfsettingsd": "ok" if is_running("xfsettingsd") else "fail",
        "xfconf-query": "ok" if have("xfconf-query") else "fail",
        "NetworkManager": "ok" if is_running("NetworkManager") else "warn",
        "sshd": "ok" if is_running("sshd") else ("warn" if have("sshd") else "missing"),
    }


def hardware_summary() -> dict[str, str]:
    """Lightweight hardware fingerprint for the dashboard card.

    Pure stdlib, no third-party deps. Values are best-effort and never raise.
    """
    summary: dict[str, str] = {}
    summary["hostname"] = socket.gethostname()
    try:
        with open("/proc/cpuinfo", encoding="utf-8") as f:
            for line in f:
                if line.startswith("model name"):
                    summary["cpu"] = line.split(":", 1)[1].strip()
                    break
    except OSError:
        summary["cpu"] = "unknown"
    try:
        with open("/proc/meminfo", encoding="utf-8") as f:
            for line in f:
                if line.startswith("MemTotal:"):
                    kb = int(line.split()[1])
                    summary["ram"] = f"{kb // 1024 // 1024} GB"
                    break
    except OSError:
        summary["ram"] = "unknown"
    try:
        with open("/etc/os-release", encoding="utf-8") as f:
            for line in f:
                if line.startswith("PRETTY_NAME="):
                    summary["os"] = line.split("=", 1)[1].strip().strip('"')
                    break
    except OSError:
        summary["os"] = "unknown"
    return summary


def last_snapshot() -> Optional[tuple[str, datetime]]:
    """Return (name, timestamp) of the most recent snapshot, or None."""
    if not SNAPSHOT_DIR.exists():
        return None
    snaps = sorted(SNAPSHOT_DIR.iterdir(), key=lambda p: p.stat().st_mtime, reverse=True)
    if not snaps:
        return None
    latest = snaps[0]
    return (latest.name, datetime.fromtimestamp(latest.stat().st_mtime))
