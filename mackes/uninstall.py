"""Uninstall — removes all Mackes changes, files, and previous-version residue.

Locks: Q8 (Maintain panel), Q16 (drop daemon kills + lean-XFCE reinstall).

The uninstall is *destructive by design*. It:

  1. Creates a pre-uninstall snapshot and copies it to ~/Desktop/ as a
     tarball (Q11, Q12) — the only artifact that survives.
  2. Resets xfconf channels to distribution defaults (Q14) and signals
     xfsettingsd (Q40).
  3. Deletes user-owned Mackes files: ~/.config/mackes-shell/ and the
     snapshots/logs trees (Q15).
  4. Removes xfce11-unified v2.2 leftovers from a known path list (Q19–Q21),
     preserving quick-network-mesh (Q20).
  5. Runs install-helpers/restore-xfce-settings.sh explicitly (Q18) to
     un-hide xfce4-settings menu entries.
  6. Removes the Mackes package itself — adapting to RPM / pip / git
     install modes (Q29).
  7. Writes a log of every step to ~/Desktop/mackes-shell-uninstall-<ts>.log
     (Q27) and returns a structured report.

Best-effort (Q26): each step catches and records its own failure. The
caller decides what to do with the report (the GUI shows it inline; the
CLI prints it to stdout).
"""
from __future__ import annotations

import datetime as _dt
import os
import shutil
import subprocess
import tarfile
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable, Optional

from mackes.logging import log_action
from mackes.state import CONFIG_DIR, DATA_DIR, HOME, SNAPSHOT_DIR, LOG_DIR


# ---------------------------------------------------------------------------
# Known v2.2 paths (Q20–Q21 locks). QNM intentionally excluded.
# ---------------------------------------------------------------------------


V22_KNOWN_PATHS = [
    HOME / "xfce11-unified",
    HOME / "Desktop" / "xfce11-unified",
    Path("/opt/xfce11-unified"),
    Path("/usr/local/share/xfce11-unified"),
    # Loose desktop launcher from v2.2
    HOME / "Desktop" / "START-HERE-XFCE11-UNIFIED.desktop",
    Path("/usr/share/applications/START-HERE-XFCE11-UNIFIED.desktop"),
    Path("/usr/local/share/applications/START-HERE-XFCE11-UNIFIED.desktop"),
]


# ---------------------------------------------------------------------------
# Result type
# ---------------------------------------------------------------------------


@dataclass
class UninstallStep:
    name: str
    ok: bool
    detail: str


@dataclass
class UninstallReport:
    steps: list[UninstallStep] = field(default_factory=list)
    log_path: Optional[Path] = None
    desktop_tarball: Optional[Path] = None
    failed_count: int = 0

    def add(self, name: str, ok: bool, detail: str = "") -> None:
        self.steps.append(UninstallStep(name=name, ok=ok, detail=detail))
        if not ok:
            self.failed_count += 1


# ---------------------------------------------------------------------------
# Step implementations (each isolated to keep best-effort semantics easy)
# ---------------------------------------------------------------------------


ProgressCb = Optional[Callable[[str], None]]


def _emit(report: UninstallReport, log_handle, progress: ProgressCb,
          name: str, ok: bool, detail: str = "") -> None:
    report.add(name, ok, detail)
    line = f"[{'OK ' if ok else 'FAIL'}] {name}" + (f" — {detail}" if detail else "")
    log_action(line)
    if log_handle is not None:
        log_handle.write(line + "\n")
        log_handle.flush()
    if progress is not None:
        progress(line)


def _detect_install_mode() -> str:
    """Return one of: 'rpm', 'pip', 'git', 'unknown'.

    Q29 lock: adapt removal to install mode.
    """
    try:
        subprocess.check_call(
            ["rpm", "-q", "mackes-shell"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        return "rpm"
    except (subprocess.CalledProcessError, FileNotFoundError):
        pass
    # Pip — `pip show` if a non-editable install is present.
    try:
        proc = subprocess.run(
            ["python3", "-m", "pip", "show", "mackes-shell"],
            capture_output=True, text=True, timeout=10,
        )
        if proc.returncode == 0 and "Name: mackes-shell" in proc.stdout:
            return "pip"
    except (OSError, subprocess.TimeoutExpired):
        pass
    # Git checkout — running from a working tree
    src_marker = Path(__file__).resolve().parents[1] / ".git"
    if src_marker.exists():
        return "git"
    return "unknown"


def _create_pre_uninstall_snapshot(report, log_handle, progress) -> Optional[Path]:
    """Q11 + Q12 lock — auto-snapshot, tarball to ~/Desktop/."""
    try:
        from mackes.snapshots import create_snapshot
    except Exception as e:  # noqa: BLE001
        _emit(report, log_handle, progress,
              "pre-uninstall snapshot", False, f"snapshot module: {e}")
        return None
    ts = _dt.datetime.now().strftime("%Y%m%dT%H%M%S")
    snap_name = f"pre-uninstall-{ts}"
    try:
        snap_path = create_snapshot(snap_name)
        _emit(report, log_handle, progress, "snapshot created", True, str(snap_path))
    except Exception as e:  # noqa: BLE001
        _emit(report, log_handle, progress, "snapshot create", False, str(e))
        return None
    # Tarball to ~/Desktop/
    desktop = HOME / "Desktop"
    desktop.mkdir(parents=True, exist_ok=True)
    tarball = desktop / f"mackes-shell-final-snapshot-{ts}.tar.gz"
    try:
        with tarfile.open(tarball, "w:gz") as tf:
            tf.add(snap_path, arcname=snap_path.name)
        _emit(report, log_handle, progress,
              "snapshot tarball", True, str(tarball))
        return tarball
    except Exception as e:  # noqa: BLE001
        _emit(report, log_handle, progress, "snapshot tarball", False, str(e))
        return None


def _reset_xfconf_defaults(report, log_handle, progress) -> None:
    """Q14 + Q40: reset known channels to defaults, signal xfsettingsd."""
    if not shutil.which("xfconf-query"):
        _emit(report, log_handle, progress,
              "xfconf reset", False, "xfconf-query missing")
        return
    channels = (
        "xsettings", "xfwm4", "xfce4-desktop", "xfce4-panel",
        "xfce4-notifyd", "xfce4-power-manager", "xfce4-session",
        "thunar-volman", "keyboards", "pointers",
    )
    for chan in channels:
        try:
            subprocess.call(
                ["xfconf-query", "--channel", chan, "--reset", "--root", "-r"],
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=20,
            )
        except (OSError, subprocess.TimeoutExpired) as e:
            _emit(report, log_handle, progress,
                  f"xfconf reset {chan}", False, str(e))
            continue
    _emit(report, log_handle, progress,
          "xfconf channels reset", True, ", ".join(channels))
    # Signal xfsettingsd to re-read.
    if shutil.which("pkill"):
        subprocess.call(["pkill", "-HUP", "-x", "xfsettingsd"])
        _emit(report, log_handle, progress,
              "xfsettingsd signaled", True, "SIGHUP")


def _remove_user_files(report, log_handle, progress) -> None:
    """Q15 lock: wipe Mackes-owned user data."""
    targets = [
        CONFIG_DIR,    # ~/.config/mackes-shell
        DATA_DIR,      # ~/.local/share/mackes-shell
        SNAPSHOT_DIR,  # ~/.local/share/mackes-shell/snapshots
        LOG_DIR,
    ]
    for tgt in targets:
        try:
            if tgt.is_dir():
                shutil.rmtree(tgt)
                _emit(report, log_handle, progress,
                      f"removed dir: {tgt}", True, "")
            elif tgt.exists():
                tgt.unlink()
                _emit(report, log_handle, progress,
                      f"removed file: {tgt}", True, "")
        except OSError as e:
            _emit(report, log_handle, progress,
                  f"remove {tgt}", False, str(e))


def _remove_v22_leftovers(report, log_handle, progress) -> None:
    """Q19–Q21 lock: known-list search for v2.2 paths. QNM preserved."""
    for path in V22_KNOWN_PATHS:
        if not path.exists():
            continue
        try:
            if path.is_dir():
                shutil.rmtree(path)
            else:
                path.unlink()
            _emit(report, log_handle, progress,
                  f"v2.2 leftover removed: {path}", True, "")
        except OSError as e:
            _emit(report, log_handle, progress,
                  f"v2.2 remove {path}", False, str(e))


def _run_restore_xfce_settings(report, log_handle, progress) -> None:
    """Q18 lock — explicit call before dnf remove."""
    helpers = [
        Path("/usr/share/mackes-shell/install-helpers/restore-xfce-settings.sh"),
        Path(__file__).resolve().parents[1] / "install-helpers" / "restore-xfce-settings.sh",
    ]
    for script in helpers:
        if script.exists() and os.access(script, os.X_OK):
            try:
                cmd = ["bash", str(script)]
                # Root-owned writes inside /etc/skel — needs sudo.
                if shutil.which("pkexec"):
                    cmd = ["pkexec", *cmd]
                elif shutil.which("sudo"):
                    cmd = ["sudo", *cmd]
                proc = subprocess.run(cmd, capture_output=True, text=True, timeout=60)
                _emit(report, log_handle, progress,
                      "restore-xfce-settings.sh", proc.returncode == 0,
                      proc.stdout.strip() or proc.stderr.strip() or "ok")
                return
            except (OSError, subprocess.TimeoutExpired) as e:
                _emit(report, log_handle, progress,
                      "restore-xfce-settings.sh", False, str(e))
                return
    _emit(report, log_handle, progress,
          "restore-xfce-settings.sh", False, "helper script not shipped")


def _remove_package(report, log_handle, progress, mode: str) -> None:
    """Q29 lock: adapt removal to install mode."""
    if mode == "rpm":
        cmd = ["pkexec", "dnf", "remove", "-y", "mackes-shell"]
        if not shutil.which("pkexec"):
            cmd = ["sudo", "dnf", "remove", "-y", "mackes-shell"]
        try:
            proc = subprocess.run(cmd, capture_output=True, text=True, timeout=300)
            _emit(report, log_handle, progress,
                  "dnf remove mackes-shell", proc.returncode == 0,
                  proc.stdout.strip().splitlines()[-1] if proc.stdout.strip() else "")
        except (OSError, subprocess.TimeoutExpired) as e:
            _emit(report, log_handle, progress, "dnf remove", False, str(e))
    elif mode == "pip":
        cmd = ["python3", "-m", "pip", "uninstall", "-y", "mackes-shell"]
        try:
            proc = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
            _emit(report, log_handle, progress,
                  "pip uninstall mackes-shell", proc.returncode == 0,
                  proc.stdout.strip().splitlines()[-1] if proc.stdout.strip() else "")
        except (OSError, subprocess.TimeoutExpired) as e:
            _emit(report, log_handle, progress, "pip uninstall", False, str(e))
    elif mode == "git":
        _emit(report, log_handle, progress,
              "package removal skipped", True,
              "running from a git checkout; delete the working tree manually")
    else:
        _emit(report, log_handle, progress,
              "package removal skipped", True,
              "install mode not detected")


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def run_uninstall(*, progress: ProgressCb = None) -> UninstallReport:
    """Run the full uninstall sequence.

    `progress` callback is invoked once per step with a single line string
    suitable for streaming into a GUI text view.
    """
    report = UninstallReport()
    ts = _dt.datetime.now().strftime("%Y%m%dT%H%M%S")
    desktop = HOME / "Desktop"
    desktop.mkdir(parents=True, exist_ok=True)
    log_path = desktop / f"mackes-shell-uninstall-{ts}.log"
    report.log_path = log_path

    try:
        log_handle = log_path.open("w", encoding="utf-8")
    except OSError:
        log_handle = None

    try:
        log_handle and log_handle.write(
            f"# Mackes Shell uninstall — {ts}\n"
            f"# Install mode: {_detect_install_mode()}\n\n"
        )

        report.desktop_tarball = _create_pre_uninstall_snapshot(report, log_handle, progress)
        _reset_xfconf_defaults(report, log_handle, progress)
        _run_restore_xfce_settings(report, log_handle, progress)
        _remove_user_files(report, log_handle, progress)
        _remove_v22_leftovers(report, log_handle, progress)
        _remove_package(report, log_handle, progress, _detect_install_mode())

        # Tail summary line.
        ok = report.failed_count == 0
        summary = (
            f"--- uninstall complete: {len(report.steps)} steps, "
            f"{report.failed_count} failed ---"
        )
        if log_handle is not None:
            log_handle.write("\n" + summary + "\n")
            log_handle.flush()
        if progress is not None:
            progress(summary)
    finally:
        if log_handle is not None:
            log_handle.close()
    return report


def schedule_logout(delay_seconds: int = 10) -> None:
    """Q25 lock — fire `xfce4-session-logout` after a brief delay."""
    if not shutil.which("xfce4-session-logout"):
        return
    def _go() -> None:
        time.sleep(delay_seconds)
        subprocess.Popen(
            ["xfce4-session-logout", "--logout", "--fast"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
    import threading
    threading.Thread(target=_go, daemon=True).start()
