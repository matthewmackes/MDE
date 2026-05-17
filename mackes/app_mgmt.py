"""App Management — install / remove / list packages.

C1–C4, C9, C10, Q15 locks.

Four install backends, picked per package:
  • `dnf`            — Fedora-repo packages (the default)
  • `dnf-thirdparty` — adds a Microsoft / VS Code / RPM Fusion repo first
  • `appimage`       — downloads an AppImage to ~/.local/bin (Cursor)
  • `npm`            — `npm i -g <pkg>` (Claude CLI)

Every install action is logged. Third-party repo additions surface in the
log so the user knows what got added.

Curated lists are read from the active preset's `apps:` block:
  • apps.install      — curated install set
  • apps.remove_bloat — single combined Bloat list (GNOME-on-XFCE +
                        LibreOffice + XFCE extras), Q15 lock.
"""
from __future__ import annotations

import json
import shutil
import subprocess
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

from mackes.logging import log_action
from mackes.state import CONFIG_DIR, HOME, DATA_DIR


REMOVED_BY_MACKES_FILE = CONFIG_DIR / "removed-by-mackes.json"


# ---------------------------------------------------------------------------
# Catalog: maps a curated name to its install method.
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class AppDef:
    name: str                        # canonical curated name (matches preset YAMLs)
    display: str                     # human-readable
    backend: str                     # 'dnf' / 'dnf-thirdparty' / 'appimage' / 'npm'
    package: Optional[str] = None    # rpm/npm package name (defaults to name)
    repo_setup: Optional[str] = None # shell snippet to add the repo first
    appimage_url: Optional[str] = None
    description: str = ""


CATALOG: dict[str, AppDef] = {
    "filezilla": AppDef("filezilla", "FileZilla", "dnf",
                        description="SFTP/FTP client."),
    "terminator": AppDef("terminator", "Terminator", "dnf",
                         description="Tiling terminal."),
    "vlc": AppDef("vlc", "VLC", "dnf",
                  description="Media player (requires RPM Fusion)."),
    "remmina": AppDef("remmina", "Remmina", "dnf",
                      description="Remote desktop client."),
    "mc": AppDef("mc", "Midnight Commander", "dnf",
                 description="Two-pane file manager."),
    "neofetch": AppDef("neofetch", "neofetch", "dnf",
                       description="System info in the terminal."),
    "microsoft-edge-stable": AppDef(
        "microsoft-edge-stable", "Microsoft Edge", "dnf-thirdparty",
        repo_setup=(
            "sudo rpm --import https://packages.microsoft.com/keys/microsoft.asc && "
            "sudo dnf config-manager addrepo --from-repofile="
            "https://packages.microsoft.com/yumrepos/edge/config.repo || "
            "sudo dnf config-manager --add-repo "
            "https://packages.microsoft.com/yumrepos/edge/config.repo"
        ),
        description="Browser. Adds packages.microsoft.com.",
    ),
    "code": AppDef(
        "code", "Visual Studio Code", "dnf-thirdparty",
        repo_setup=(
            "sudo rpm --import https://packages.microsoft.com/keys/microsoft.asc && "
            "sudo sh -c 'echo -e \"[code]\\nname=Visual Studio Code\\n"
            "baseurl=https://packages.microsoft.com/yumrepos/vscode\\nenabled=1\\n"
            "gpgcheck=1\\ngpgkey=https://packages.microsoft.com/keys/microsoft.asc\" "
            "> /etc/yum.repos.d/vscode.repo'"
        ),
        description="Code editor. Adds the Microsoft vscode repo.",
    ),
    "cursor": AppDef(
        "cursor", "Cursor", "appimage",
        appimage_url=(
            # Cursor publishes AppImages at downloader.cursor.sh; the API redirects
            # to a CDN URL. Using the stable channel.
            "https://download.cursor.sh/linux/appImage/x64"
        ),
        description="AI code editor. AppImage to ~/.local/bin/cursor.",
    ),
    "claude-code": AppDef(
        "claude-code", "Claude Code CLI", "npm",
        package="@anthropic-ai/claude-code",
        description="Anthropic's Claude CLI. Installed globally via npm.",
    ),
}


# ---------------------------------------------------------------------------
# State tracking — record what Mackes removed (informational).
# ---------------------------------------------------------------------------


def _load_removed_record() -> dict[str, list[str]]:
    if not REMOVED_BY_MACKES_FILE.exists():
        return {"bloat": []}
    try:
        data = json.loads(REMOVED_BY_MACKES_FILE.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {"bloat": []}
    data.setdefault("bloat", [])
    return data


def _save_removed_record(record: dict[str, list[str]]) -> None:
    CONFIG_DIR.mkdir(parents=True, exist_ok=True)
    REMOVED_BY_MACKES_FILE.write_text(
        json.dumps(record, indent=2, sort_keys=True), encoding="utf-8",
    )


def record_removed(category: str, packages: list[str]) -> None:
    rec = _load_removed_record()
    existing = set(rec.get(category, []))
    existing.update(packages)
    rec[category] = sorted(existing)
    _save_removed_record(rec)


# ---------------------------------------------------------------------------
# dnf / npm wrappers
# ---------------------------------------------------------------------------


def is_dnf_installed(package: str) -> bool:
    if not shutil.which("rpm"):
        return False
    try:
        subprocess.check_call(
            ["rpm", "-q", package],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        return True
    except subprocess.CalledProcessError:
        return False


def dnf_install(packages: list[str]) -> tuple[int, str]:
    """Install via dnf. Returns (rc, combined output)."""
    if not packages:
        return 0, ""
    cmd = ["pkexec", "dnf", "install", "-y", *packages]
    if not shutil.which("pkexec"):
        cmd = ["sudo", "dnf", "install", "-y", *packages]
    try:
        proc = subprocess.run(
            cmd, capture_output=True, text=True, timeout=600,
        )
        return proc.returncode, (proc.stdout + proc.stderr)
    except (OSError, subprocess.TimeoutExpired) as e:
        return 1, str(e)


def dnf_remove(packages: list[str]) -> tuple[int, str]:
    if not packages:
        return 0, ""
    cmd = ["pkexec", "dnf", "remove", "-y", *packages]
    if not shutil.which("pkexec"):
        cmd = ["sudo", "dnf", "remove", "-y", *packages]
    try:
        proc = subprocess.run(
            cmd, capture_output=True, text=True, timeout=600,
        )
        return proc.returncode, (proc.stdout + proc.stderr)
    except (OSError, subprocess.TimeoutExpired) as e:
        return 1, str(e)


def run_repo_setup(snippet: str) -> tuple[int, str]:
    """Add a third-party repo via a shell snippet. Always runs through sudo
    because repo files live in /etc/yum.repos.d/."""
    cmd = ["pkexec", "bash", "-lc", snippet]
    if not shutil.which("pkexec"):
        cmd = ["sudo", "bash", "-lc", snippet]
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
        return proc.returncode, (proc.stdout + proc.stderr)
    except (OSError, subprocess.TimeoutExpired) as e:
        return 1, str(e)


def install_appimage(app: AppDef) -> tuple[int, str]:
    """Download the AppImage to ~/.local/bin/<name>, chmod +x, write a .desktop."""
    if app.appimage_url is None:
        return 1, "no appimage URL configured"
    dst_bin = HOME / ".local" / "bin"
    dst_bin.mkdir(parents=True, exist_ok=True)
    target = dst_bin / app.name
    try:
        with urllib.request.urlopen(app.appimage_url, timeout=120) as resp:
            target.write_bytes(resp.read())
        target.chmod(0o755)
    except Exception as e:  # noqa: BLE001
        return 1, str(e)
    apps_dir = HOME / ".local" / "share" / "applications"
    apps_dir.mkdir(parents=True, exist_ok=True)
    (apps_dir / f"{app.name}.desktop").write_text(
        "[Desktop Entry]\n"
        "Type=Application\n"
        f"Name={app.display}\n"
        f"Exec={target}\n"
        "Terminal=false\n"
        "X-Mackes-Managed=1\n",
        encoding="utf-8",
    )
    return 0, f"installed AppImage to {target}"


def install_npm_global(package: str) -> tuple[int, str]:
    if not shutil.which("npm"):
        # Try to install npm first via dnf.
        rc, out = dnf_install(["nodejs", "npm"])
        if rc != 0:
            return rc, f"npm not available and `dnf install npm` failed:\n{out}"
    cmd = ["pkexec", "npm", "install", "-g", package]
    if not shutil.which("pkexec"):
        cmd = ["sudo", "npm", "install", "-g", package]
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
        return proc.returncode, (proc.stdout + proc.stderr)
    except (OSError, subprocess.TimeoutExpired) as e:
        return 1, str(e)


# ---------------------------------------------------------------------------
# Curated bulk operations
# ---------------------------------------------------------------------------


def install_app(name: str) -> list[str]:
    """Install one curated app by canonical name."""
    actions: list[str] = []
    app = CATALOG.get(name)
    if app is None:
        # Treat unknown names as a plain dnf install request.
        rc, out = dnf_install([name])
        actions.append(f"dnf install {name}: rc={rc}")
        if out.strip():
            actions.append(out.strip().splitlines()[-1])
        for line in actions:
            log_action(line)
        return actions
    if app.backend == "dnf":
        rc, out = dnf_install([app.package or app.name])
        actions.append(f"{app.display}: dnf install rc={rc}")
    elif app.backend == "dnf-thirdparty":
        if app.repo_setup:
            rc, out = run_repo_setup(app.repo_setup)
            actions.append(f"{app.display}: repo setup rc={rc}")
            if rc != 0:
                actions.append(out.strip().splitlines()[-1] if out.strip() else "repo setup failed")
                for line in actions:
                    log_action(line)
                return actions
        rc, out = dnf_install([app.package or app.name])
        actions.append(f"{app.display}: dnf install rc={rc}")
    elif app.backend == "appimage":
        rc, out = install_appimage(app)
        actions.append(f"{app.display}: appimage rc={rc}")
    elif app.backend == "npm":
        rc, out = install_npm_global(app.package or app.name)
        actions.append(f"{app.display}: npm install rc={rc}")
    else:
        actions.append(f"{app.display}: unknown backend {app.backend}")
    if out and out.strip():
        actions.append(out.strip().splitlines()[-1])
    for line in actions:
        log_action(line)
    return actions


def install_curated_set(names: list[str]) -> list[str]:
    actions: list[str] = []
    for name in names:
        actions.extend(install_app(name))
    return actions


def remove_packages(packages: list[str], *, category: str = "bloat") -> list[str]:
    actions: list[str] = []
    # Filter to only installed packages — dnf is happy to fail on unknown ones,
    # which clutters the log.
    installed = [p for p in packages if "*" in p or is_dnf_installed(p)]
    if not installed:
        actions.append(f"remove: nothing to do ({len(packages)} not installed)")
        log_action(actions[-1])
        return actions
    rc, out = dnf_remove(installed)
    actions.append(f"dnf remove ({category}) rc={rc}: {', '.join(installed)}")
    if out.strip():
        actions.append(out.strip().splitlines()[-1])
    if rc == 0:
        # Glob-expanded names (libreoffice-*) get stored verbatim — reinstall
        # uses the same expression.
        record_removed(category, installed)
    for line in actions:
        log_action(line)
    return actions


# ---------------------------------------------------------------------------
# Browse installed packages (Apps → Installed)
# ---------------------------------------------------------------------------


def list_installed_packages() -> list[tuple[str, str]]:
    """Returns [(name, version)] for every installed RPM."""
    if not shutil.which("rpm"):
        return []
    try:
        out = subprocess.check_output(
            ["rpm", "-qa", "--qf", "%{NAME}\\t%{VERSION}-%{RELEASE}\\n"],
            text=True, stderr=subprocess.DEVNULL, timeout=15,
        )
    except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
        return []
    pairs: list[tuple[str, str]] = []
    for line in out.splitlines():
        if "\t" in line:
            name, version = line.split("\t", 1)
            pairs.append((name.strip(), version.strip()))
    return sorted(pairs)
