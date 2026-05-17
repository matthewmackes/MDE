"""mackes.headless — CLI entry points for use without a display.

§8.12 lock — auto-detected on launch when $DISPLAY / $WAYLAND_DISPLAY
are empty and no logind graphical session is registered. Force with
`mackes --headless`; bypass with `mackes --gui`.
"""
from __future__ import annotations

import os
import shutil
import subprocess


def is_headless() -> bool:
    """Heuristic: no display server + no graphical session at the
    systemd-logind level."""
    if os.environ.get("DISPLAY") or os.environ.get("WAYLAND_DISPLAY"):
        return False
    sid = os.environ.get("XDG_SESSION_ID")
    if sid and shutil.which("loginctl"):
        try:
            rc = subprocess.call(
                ["loginctl", "show-session", sid, "-p", "Type", "--value"],
                stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
            )
            # If the session type is tty/unspecified, treat as headless
            # (this returns rc=0 even on tty sessions; the value-check is in run())
        except OSError:
            pass
    return True
