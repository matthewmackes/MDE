"""Birthright — first-run install steps that turn a stock XFCE box into Mackes.

Each function is idempotent (safe to re-run via Maintain → Reset to Preset)
and returns a `list[str]` of action lines for the wizard's apply page log.

These are the thirteen "birthright" items the v1.5.0 wizard runs in
addition to the v1.0.x xfconf-only apply pipeline:

  1. apply_themes              — deploy PadOS GTK theme + Carbon icon theme files
  2. apply_fonts               — install IBM Plex Sans + Mono via dnf
  3. apply_apps                — install preset.apps.install / remove preset.apps.remove_bloat
  4. apply_panel_layout        — write the Mackes default xfce4-panel layout
  5. apply_plymouth            — install + activate the Mackes Plymouth boot theme
  6. apply_dnf_update          — dnf upgrade -y --refresh (full system update)
  7. apply_third_party_repos   — install fedora-workstation-repositories (Chrome, RPM Fusion, etc.)
  8. apply_flathub             — add the Flathub flatpak remote (per-user)
  9. apply_remote_desktop      — xrdp + x11vnc + guacd + tomcat + Guacamole web app
                                  + mackes-remote-sync (Headscale→Guacamole config)
 10. apply_fleet               — ansible-core + ansible-pull timer + seeded
                                  QNM-Shared playbook tree (v1.3.0 lock)
 11. apply_conky               — Mackes Conky HUD: top-right Carbon-styled
                                  desktop HUD with live mesh/fleet/drift
                                  state (v1.4.0 lock)
 12. apply_maximize_all        — every new top-level window starts maximized,
                                  via the mackes-maximizer user service
                                  (v1.4.1 lock)
 13. apply_clipboard_daemon    — mesh clipboard daemon: bidirectional sync
                                  between XA_CLIPBOARD and QNM-Shared
                                  clipboard bucket (v1.5.0 lock)

All thirteen are wired into mackes/wizard/pages/apply.py between Panel and Mesh.
"""
from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path
from typing import Iterable, List

from mackes.logging import log_action
from mackes.presets import Preset


# ---------------------------------------------------------------------------
# Resolve repo / install paths
# ---------------------------------------------------------------------------


def _data_roots() -> List[Path]:
    """Return ordered list of candidate data roots (installed > source-tree)."""
    return [
        Path("/usr/share/mackes-shell/data"),
        Path(__file__).resolve().parent.parent / "data",
    ]


def _find_data(*rel: str) -> Path | None:
    for root in _data_roots():
        p = root.joinpath(*rel)
        if p.exists():
            return p
    return None


def _branding(*rel: str) -> Path | None:
    """Branding lives at /usr/share/mackes-shell/branding/ (RPM) or repo branding/."""
    for root in (
        Path("/usr/share/mackes-shell/branding"),
        Path(__file__).resolve().parent.parent / "branding",
    ):
        p = root.joinpath(*rel)
        if p.exists():
            return p
    return None


def _run_root(cmd: list[str], *, timeout: int = 300) -> tuple[int, str]:
    """Run a command with root privileges.

    v1.4.0: routes through AdminSession so the user only authenticates
    ONCE per Mackes session (sudo timestamp keepalive). Falls back to
    per-call pkexec when the session is locked or sudo is unavailable.
    """
    from mackes.admin_session import AdminSession
    return AdminSession.instance().run(cmd, timeout=timeout)


def _run(cmd: list[str], *, timeout: int = 60) -> tuple[int, str]:
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
        return proc.returncode, (proc.stdout + proc.stderr)
    except (OSError, subprocess.TimeoutExpired) as e:
        return 1, str(e)


# ---------------------------------------------------------------------------
# 1. Themes — copy PadOS GTK theme + Carbon icon theme into system dirs
# ---------------------------------------------------------------------------


def apply_themes(_preset: Preset) -> List[str]:
    """Deploy PadOS + Carbon to /usr/share/{themes,icons}/ and refresh caches.

    Source: data/themes/PadOS/ and data/icons/Carbon/ (shipped by the RPM).
    Idempotent: skips if the destination is newer than the source.
    """
    actions: List[str] = []

    pad_src = _find_data("themes", "PadOS")
    carbon_src = _find_data("icons", "Carbon")
    pad_dst = Path("/usr/share/themes/PadOS")
    carbon_dst = Path("/usr/share/icons/Carbon")

    # PadOS GTK theme ---------------------------------------------------
    if pad_src is None:
        actions.append("themes: PadOS source missing in data/themes/PadOS — skipping")
    elif _newer_than(pad_dst, pad_src):
        actions.append(f"themes: PadOS already installed at {pad_dst} (up to date)")
    else:
        rc, out = _run_root(
            ["cp", "-rT", str(pad_src), str(pad_dst)],
            timeout=120,
        )
        if rc == 0:
            actions.append(f"themes: installed PadOS to {pad_dst}")
        else:
            actions.append(f"themes: PadOS install failed: {out.strip().splitlines()[-1] if out.strip() else 'rc='+str(rc)}")

    # Carbon icon theme -------------------------------------------------
    if carbon_src is None:
        actions.append("themes: Carbon source missing in data/icons/Carbon — skipping")
    elif _newer_than(carbon_dst, carbon_src):
        actions.append(f"themes: Carbon already installed at {carbon_dst} (up to date)")
    else:
        rc, out = _run_root(
            ["cp", "-rT", str(carbon_src), str(carbon_dst)],
            timeout=300,
        )
        if rc == 0:
            actions.append(f"themes: installed Carbon to {carbon_dst}")
            # Refresh icon cache
            if shutil.which("gtk-update-icon-cache"):
                _run_root(["gtk-update-icon-cache", "-f", "-t", str(carbon_dst)], timeout=60)
                actions.append("themes: rebuilt Carbon icon cache")
        else:
            actions.append(f"themes: Carbon install failed: {out.strip().splitlines()[-1] if out.strip() else 'rc='+str(rc)}")

    for line in actions:
        log_action(line)
    return actions


def _newer_than(dst: Path, src: Path) -> bool:
    """Return True iff dst exists and is at least as new as the newest file in src."""
    if not dst.exists():
        return False
    try:
        dst_mtime = max(_walk_mtimes(dst), default=0.0)
        src_mtime = max(_walk_mtimes(src), default=0.0)
        return dst_mtime >= src_mtime
    except OSError:
        return False


def _walk_mtimes(path: Path) -> Iterable[float]:
    if path.is_file():
        try:
            yield path.stat().st_mtime
        except OSError:
            pass
        return
    if not path.is_dir():
        return
    try:
        for root, _dirs, files in os.walk(path):
            for f in files:
                try:
                    yield os.stat(os.path.join(root, f)).st_mtime
                except OSError:
                    continue
    except OSError:
        return


# ---------------------------------------------------------------------------
# 2. Fonts — dnf install IBM Plex Sans + Mono
# ---------------------------------------------------------------------------


_PLEX_PACKAGES = ("ibm-plex-sans-fonts", "ibm-plex-mono-fonts")


def apply_fonts(_preset: Preset) -> List[str]:
    """Install IBM Plex Sans + Mono via dnf. Idempotent."""
    actions: List[str] = []
    if shutil.which("dnf") is None:
        actions.append("fonts: dnf not available — skipping")
        return actions

    # Skip if already installed
    needed = []
    for pkg in _PLEX_PACKAGES:
        rc, _ = _run(["rpm", "-q", pkg])
        if rc != 0:
            needed.append(pkg)
    if not needed:
        actions.append("fonts: IBM Plex already installed")
        return actions

    rc, out = _run_root(["dnf", "install", "-y", *needed], timeout=600)
    if rc == 0:
        actions.append(f"fonts: installed {', '.join(needed)}")
        if shutil.which("fc-cache"):
            _run_root(["fc-cache", "-fv"], timeout=120)
            actions.append("fonts: rebuilt fontconfig cache")
    else:
        last = out.strip().splitlines()[-1] if out.strip() else f"rc={rc}"
        actions.append(f"fonts: install failed: {last}")
    for line in actions:
        log_action(line)
    return actions


# ---------------------------------------------------------------------------
# 3. Apps — process preset.apps.install + apps.remove_bloat
# ---------------------------------------------------------------------------


def apply_apps(preset: Preset) -> List[str]:
    """Process the preset's apps.install + apps.remove_bloat lists."""
    actions: List[str] = []
    apps_section = getattr(preset, "apps", None) or {}
    install = apps_section.get("install") if isinstance(apps_section, dict) else []
    remove  = apps_section.get("remove_bloat") if isinstance(apps_section, dict) else []
    install = install or []
    remove  = remove  or []

    if not install and not remove:
        actions.append("apps: preset declares neither install nor remove_bloat — nothing to do")
        return actions

    # Install
    if install:
        from mackes.app_mgmt import install_curated_set
        try:
            actions.append(f"apps: installing {len(install)} app(s): {', '.join(install)}")
            actions.extend(install_curated_set(list(install)))
        except Exception as e:  # noqa: BLE001
            actions.append(f"apps: install pipeline error: {e}")

    # Remove bloat
    if remove:
        from mackes.app_mgmt import remove_packages
        try:
            actions.append(f"apps: removing bloat ({len(remove)} pattern(s))")
            actions.extend(remove_packages(list(remove), category="bloat"))
        except Exception as e:  # noqa: BLE001
            actions.append(f"apps: remove pipeline error: {e}")

    for line in actions:
        log_action(line)
    return actions


# ---------------------------------------------------------------------------
# 4. Panel layout — write Mackes default xfce4-panel layout via xfconf
# ---------------------------------------------------------------------------


def apply_panel_layout(_preset: Preset) -> List[str]:
    """Write the Mackes default xfce4-panel layout.

    The layout is a single horizontal panel along the top with:
      - Whisker Menu (replaces XFCE Applications menu)
      - Docklike Taskbar (replaces Window Buttons)
      - Spacer
      - Status Tray
      - Clock (IBM Plex digital)

    This function uses xfconf-query and is per-user. It only writes keys
    that aren't already set to the expected value (idempotent).
    """
    actions: List[str] = []
    if shutil.which("xfconf-query") is None:
        actions.append("panel layout: xfconf-query not installed — skipping")
        return actions

    # Helper: set a single-value key
    def _set(channel: str, prop: str, type_hint: str, value: str) -> None:
        rc, out = _run(["xfconf-query", "--channel", channel, "--property", prop,
                        "--create", "--type", type_hint, "--set", value], timeout=10)
        if rc == 0:
            actions.append(f"panel: set {channel}{prop} = {value}")
        else:
            actions.append(f"panel: failed to set {prop}: "
                           f"{out.strip().splitlines()[-1] if out.strip() else rc}")

    # Helper: set an array key. xfconf-query needs every value flagged
    # individually as `-t TYPE -s VALUE`, plus `-n -a` to declare the
    # array. We always reset+create the array because mixing existing
    # plugin-ids with ours causes overlap (v1.4.6 bug).
    def _set_array(channel: str, prop: str, type_hint: str,
                   values: list[str]) -> None:
        # Wipe any prior array first so the size shrinks/expands cleanly.
        _run(["xfconf-query", "--channel", channel, "--property", prop,
              "--reset"], timeout=10)
        cmd = ["xfconf-query", "--channel", channel, "--property", prop,
               "--create", "--force-array"]
        for v in values:
            cmd.extend(["--type", type_hint, "--set", v])
        rc, out = _run(cmd, timeout=10)
        if rc == 0:
            actions.append(f"panel: set {prop}[] = {values}")
        else:
            actions.append(f"panel: failed array {prop}: "
                           f"{out.strip().splitlines()[-1] if out.strip() else rc}")

    # Plugin IDs we own (chosen to avoid collisions with default 1/2/3)
    # plugin-101 = whiskermenu, 102 = docklike, 103 = separator,
    # 104 = systray, 105 = clock
    plugin_ids = ["101", "102", "103", "104", "105"]

    # /panels = [0] — the array of panel IDs
    _set_array("xfce4-panel", "/panels", "int", ["0"])
    # /panels/panel-0/plugin-ids = [101, 102, 103, 104, 105]
    _set_array("xfce4-panel", "/panels/panel-0/plugin-ids", "uint", plugin_ids)

    # Always write panel-0 metadata + plugin types (idempotent)
    _set("xfce4-panel", "/panels/panel-0/position", "string", "p=8;x=0;y=0")
    _set("xfce4-panel", "/panels/panel-0/length",   "uint",   "100")
    _set("xfce4-panel", "/panels/panel-0/size",     "uint",   "32")
    _set("xfce4-panel", "/panels/panel-0/icon-size", "uint",  "22")
    _set("xfce4-panel", "/panels/panel-0/position-locked", "bool", "true")
    _set("xfce4-panel", "/panels/panel-0/autohide-behavior", "uint", "0")

    # Plugins
    _set("xfce4-panel", "/plugins/plugin-101", "string", "whiskermenu")
    # v1.4.6: Mackes-branded Whisker menu config (button title, icon,
    # search prompt, layout). xfce4-whiskermenu-plugin reads these from
    # the xfce4-panel xfconf channel automatically.
    _set("xfce4-panel", "/plugins/plugin-101/button-title",
         "string", "Mackes")
    _set("xfce4-panel", "/plugins/plugin-101/button-icon",
         "string", "mackes-shell")
    _set("xfce4-panel", "/plugins/plugin-101/show-button-title",
         "bool",   "true")
    _set("xfce4-panel", "/plugins/plugin-101/show-button-icon",
         "bool",   "true")
    _set("xfce4-panel", "/plugins/plugin-101/launcher-show-name",
         "bool",   "true")
    _set("xfce4-panel", "/plugins/plugin-101/launcher-show-description",
         "bool",   "true")
    _set("xfce4-panel", "/plugins/plugin-101/category-icon-size",
         "int",    "1")
    _set("xfce4-panel", "/plugins/plugin-101/item-icon-size",
         "int",    "2")
    _set("xfce4-panel", "/plugins/plugin-101/menu-width",
         "int",    "440")
    _set("xfce4-panel", "/plugins/plugin-101/menu-height",
         "int",    "560")
    _set("xfce4-panel", "/plugins/plugin-101/menu-opacity",
         "int",    "100")
    _set("xfce4-panel", "/plugins/plugin-101/position-search-alternate",
         "bool",   "true")
    _set("xfce4-panel", "/plugins/plugin-101/position-categories-alternate",
         "bool",   "true")
    _set("xfce4-panel", "/plugins/plugin-101/search-actions-enabled",
         "bool",   "true")
    _set("xfce4-panel", "/plugins/plugin-101/recent-items-max",
         "int",    "10")
    _set("xfce4-panel", "/plugins/plugin-101/favorites",
         "string", "mackes-shell.desktop")
    _set("xfce4-panel", "/plugins/plugin-102", "string", "docklike")
    _set("xfce4-panel", "/plugins/plugin-103", "string", "separator")
    _set("xfce4-panel", "/plugins/plugin-103/expand", "bool", "true")
    _set("xfce4-panel", "/plugins/plugin-103/style", "uint", "0")
    _set("xfce4-panel", "/plugins/plugin-104", "string", "systray")
    _set("xfce4-panel", "/plugins/plugin-105", "string", "clock")
    _set("xfce4-panel", "/plugins/plugin-105/digital-time-font", "string", "IBM Plex Sans Bold 12")
    _set("xfce4-panel", "/plugins/plugin-105/digital-time-format", "string", "%I:%M %p")
    _set("xfce4-panel", "/plugins/plugin-105/digital-date-font", "string", "IBM Plex Sans 10")
    _set("xfce4-panel", "/plugins/plugin-105/digital-date-format", "string", "%B %d, %Y")
    _set("xfce4-panel", "/plugins/plugin-105/mode", "uint", "2")

    # Restart panel to pick up changes
    if shutil.which("xfce4-panel"):
        try:
            subprocess.Popen(["xfce4-panel", "--restart"],
                             stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            actions.append("panel: xfce4-panel --restart issued")
        except OSError as e:
            actions.append(f"panel: could not restart xfce4-panel: {e}")

    for line in actions:
        log_action(line)
    return actions


# ---------------------------------------------------------------------------
# 5. Plymouth — install Mackes boot theme and set default
# ---------------------------------------------------------------------------


_PLYMOUTH_DEST = Path("/usr/share/plymouth/themes/mackes")


def apply_plymouth(_preset: Preset) -> List[str]:
    """Install + activate the Mackes Plymouth boot theme.

    Theme source: data/plymouth/mackes/ (shipped by the RPM).
    Activation: plymouth-set-default-theme mackes -R (regenerates initrd).
    """
    actions: List[str] = []
    if shutil.which("plymouth-set-default-theme") is None:
        actions.append("plymouth: plymouth not installed — skipping")
        return actions

    src = _find_data("plymouth", "mackes")
    if src is None:
        actions.append("plymouth: source missing in data/plymouth/mackes — skipping")
        return actions

    # If logo file is missing in source, copy from branding/
    logo_src = src / "logo.png"
    if not logo_src.exists():
        b = _branding("MACKES-XFCE-LOGO.png")
        if b is not None:
            rc, out = _run_root(["cp", str(b), str(logo_src)], timeout=15)
            if rc == 0:
                actions.append("plymouth: copied logo from branding/")

    # Copy theme to /usr/share/plymouth/themes/mackes (root)
    if _PLYMOUTH_DEST.exists() and _newer_than(_PLYMOUTH_DEST, src):
        actions.append(f"plymouth: theme already installed at {_PLYMOUTH_DEST} (up to date)")
    else:
        _PLYMOUTH_DEST.parent.mkdir(parents=True, exist_ok=True)
        rc, out = _run_root(["cp", "-rT", str(src), str(_PLYMOUTH_DEST)], timeout=60)
        if rc == 0:
            actions.append(f"plymouth: installed theme to {_PLYMOUTH_DEST}")
        else:
            last = out.strip().splitlines()[-1] if out.strip() else f"rc={rc}"
            actions.append(f"plymouth: theme copy failed: {last}")
            return actions

    # Activate theme + rebuild initrd
    actions.append("plymouth: activating theme + regenerating initrd (this may take ~30s)…")
    rc, out = _run_root(
        ["plymouth-set-default-theme", "mackes", "-R"],
        timeout=300,
    )
    if rc == 0:
        actions.append("plymouth: theme set to 'mackes'; initrd regenerated")
    else:
        last = out.strip().splitlines()[-1] if out.strip() else f"rc={rc}"
        actions.append(f"plymouth: theme activation failed: {last}")

    for line in actions:
        log_action(line)
    return actions


# ---------------------------------------------------------------------------
# 6. dnf update — full system upgrade (heaviest birthright step)
# ---------------------------------------------------------------------------


def apply_dnf_update(_preset: Preset) -> List[str]:
    """Run a full `dnf upgrade -y --refresh`. May take many minutes."""
    actions: List[str] = []
    if shutil.which("dnf") is None:
        actions.append("system update: dnf not available — skipping")
        return actions

    actions.append("system update: dnf upgrade -y --refresh (this can take several minutes)…")
    rc, out = _run_root(
        ["dnf", "upgrade", "-y", "--refresh"],
        timeout=3600,   # up to 1h for large mirror catches
    )
    # Surface just the last few summary lines (full output goes to log).
    summary = [ln for ln in (out or "").splitlines() if ln.strip()][-5:]
    if rc == 0:
        actions.append("system update: complete")
    else:
        actions.append(f"system update: failed (rc={rc})")
    actions.extend(f"  {ln}" for ln in summary)
    for line in actions:
        log_action(line)
    return actions


# ---------------------------------------------------------------------------
# 7. Third-party repos — Fedora Workstation Repositories meta-package
# ---------------------------------------------------------------------------


def apply_third_party_repos(_preset: Preset) -> List[str]:
    """Install `fedora-workstation-repositories`.

    The package ships repo files for Google Chrome, RPM Fusion, Steam,
    NVIDIA, etc. The repos stay disabled until the user opts in
    (`dnf config-manager --set-enabled <repo>` or the GNOME Software UI).
    Installing the package is enough to surface them in Apps → Install.
    """
    actions: List[str] = []
    if shutil.which("dnf") is None:
        actions.append("third-party repos: dnf not available — skipping")
        return actions

    pkg = "fedora-workstation-repositories"
    rc, _ = _run(["rpm", "-q", pkg])
    if rc == 0:
        actions.append(f"third-party repos: {pkg} already installed")
        # Still enable RPM Fusion + Google Chrome on the user's behalf — those
        # are the universally-useful ones.
    else:
        rc, out = _run_root(["dnf", "install", "-y", pkg], timeout=600)
        if rc == 0:
            actions.append(f"third-party repos: installed {pkg}")
        else:
            last = out.strip().splitlines()[-1] if out.strip() else f"rc={rc}"
            actions.append(f"third-party repos: install failed: {last}")
            for line in actions:
                log_action(line)
            return actions

    # Install RPM Fusion free + nonfree (the most useful third-party repos).
    fedora_ver = _detect_fedora_version()
    if fedora_ver:
        rpmfusion_pkgs = [
            f"https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-{fedora_ver}.noarch.rpm",
            f"https://mirrors.rpmfusion.org/nonfree/fedora/rpmfusion-nonfree-release-{fedora_ver}.noarch.rpm",
        ]
        # Skip if already installed
        free_rc, _ = _run(["rpm", "-q", "rpmfusion-free-release"])
        nonfree_rc, _ = _run(["rpm", "-q", "rpmfusion-nonfree-release"])
        to_install = []
        if free_rc != 0:
            to_install.append(rpmfusion_pkgs[0])
        if nonfree_rc != 0:
            to_install.append(rpmfusion_pkgs[1])
        if to_install:
            rc, out = _run_root(["dnf", "install", "-y", *to_install], timeout=300)
            if rc == 0:
                actions.append(f"third-party repos: enabled RPM Fusion (free + nonfree) for Fedora {fedora_ver}")
            else:
                last = out.strip().splitlines()[-1] if out.strip() else f"rc={rc}"
                actions.append(f"third-party repos: RPM Fusion enable failed: {last}")
        else:
            actions.append("third-party repos: RPM Fusion already enabled")
    else:
        actions.append("third-party repos: could not detect Fedora version — skipping RPM Fusion")

    for line in actions:
        log_action(line)
    return actions


def _detect_fedora_version() -> str | None:
    """Return Fedora major version as a string (e.g. '44'), or None."""
    try:
        for line in Path("/etc/os-release").read_text().splitlines():
            if line.startswith("VERSION_ID="):
                return line.split("=", 1)[1].strip().strip('"')
    except OSError:
        return None
    return None


# ---------------------------------------------------------------------------
# 8. Flathub — add the per-user remote
# ---------------------------------------------------------------------------


# ---------------------------------------------------------------------------
# 9. Remote desktop — xrdp + x11vnc + guacd + tomcat + Guacamole web
# ---------------------------------------------------------------------------


_GUACAMOLE_WAR_VERSION = "1.6.0"
_GUACAMOLE_WAR_URL = (
    f"https://archive.apache.org/dist/guacamole/{_GUACAMOLE_WAR_VERSION}/"
    f"binary/guacamole-{_GUACAMOLE_WAR_VERSION}.war"
)
_TOMCAT_WEBAPPS  = Path("/var/lib/tomcat/webapps")
_GUAC_ETC        = Path("/etc/guacamole")
_NOAUTH_EXT_URL  = (
    f"https://archive.apache.org/dist/guacamole/{_GUACAMOLE_WAR_VERSION}/"
    f"binary/guacamole-auth-noauth-{_GUACAMOLE_WAR_VERSION}.tar.gz"
)


def apply_remote_desktop(_preset: Preset) -> List[str]:
    """Install xrdp + x11vnc + guacd + tomcat + Guacamole web app.

    Locks the noauth design path (Q3 v1.2.0): no Guacamole login screen,
    mesh-firewall trust only. Connections are populated by mackes-remote-sync.
    """
    actions: List[str] = []
    if shutil.which("dnf") is None:
        actions.append("remote-desktop: dnf not available — skipping")
        return actions

    # ---- 1. Install Fedora packages ----------------------------------
    fedora_pkgs = ["xrdp", "xrdp-selinux", "x11vnc", "guacd", "tomcat", "curl"]
    missing = [p for p in fedora_pkgs if _run(["rpm", "-q", p])[0] != 0]
    if missing:
        actions.append(f"remote-desktop: installing {', '.join(missing)} via dnf")
        rc, out = _run_root(["dnf", "install", "-y", *missing], timeout=900)
        if rc != 0:
            last = out.strip().splitlines()[-1] if out.strip() else f"rc={rc}"
            actions.append(f"remote-desktop: dnf install failed: {last}")
            return actions
    else:
        actions.append("remote-desktop: Fedora packages already installed")

    # ---- 2. Download Guacamole web app (.war) ------------------------
    war_target = _TOMCAT_WEBAPPS / "guacamole.war"
    if war_target.exists() and war_target.stat().st_size > 1_000_000:
        actions.append(f"remote-desktop: guacamole.war already at {war_target}")
    else:
        actions.append(f"remote-desktop: downloading guacamole-{_GUACAMOLE_WAR_VERSION}.war")
        rc, out = _run_root(
            ["curl", "-fsSL", _GUACAMOLE_WAR_URL, "-o", str(war_target)],
            timeout=300,
        )
        if rc != 0:
            actions.append(f"remote-desktop: download failed: rc={rc}")
            return actions
        _run_root(["chown", "tomcat:tomcat", str(war_target)])

    # ---- 3. Download + install noauth extension ----------------------
    ext_dir = _GUAC_ETC / "extensions"
    noauth_jar = ext_dir / f"guacamole-auth-noauth-{_GUACAMOLE_WAR_VERSION}.jar"
    if noauth_jar.exists():
        actions.append("remote-desktop: noauth extension already installed")
    else:
        actions.append("remote-desktop: installing noauth extension")
        _run_root(["mkdir", "-p", str(ext_dir)])
        import tempfile
        with tempfile.TemporaryDirectory() as td:
            tar_path = Path(td) / "noauth.tar.gz"
            rc, _ = _run(["curl", "-fsSL", _NOAUTH_EXT_URL,
                          "-o", str(tar_path)], timeout=120)
            if rc != 0:
                actions.append("remote-desktop: noauth extension download failed")
                return actions
            _run(["tar", "xzf", str(tar_path), "-C", td])
            # The tarball contains a jar at guacamole-auth-noauth-<ver>/*.jar
            for jar in Path(td).rglob("*.jar"):
                _run_root(["cp", str(jar), str(noauth_jar)])
                break

    # ---- 4. /etc/guacamole config ------------------------------------
    actions.append("remote-desktop: writing /etc/guacamole config")
    _run_root(["mkdir", "-p", str(_GUAC_ETC), str(ext_dir)])
    props = (
        "# Mackes Shell — Guacamole config (v1.2.0 birthright)\n"
        "# noauth: no Guacamole login; mesh firewall + private CA are the trust.\n"
        "guacd-hostname: 127.0.0.1\n"
        "guacd-port:     4822\n"
        "noauth-config:  /etc/guacamole/noauth-config.xml\n"
    )
    _write_root_file(_GUAC_ETC / "guacamole.properties", props)

    # Seed the connection list before the sync daemon takes over
    try:
        from mackes.remote_desktop import render_noauth_xml, active_connections
        seed_xml = render_noauth_xml(active_connections())
    except Exception:  # noqa: BLE001
        seed_xml = ('<?xml version="1.0" encoding="UTF-8"?>\n'
                    '<user-mapping>\n  <authorize username="" password=""/>\n'
                    '</user-mapping>\n')
    _write_root_file(_GUAC_ETC / "noauth-config.xml", seed_xml)

    # ---- 5. systemd services + x11vnc@:0 template --------------------
    actions.append("remote-desktop: writing x11vnc@.service template")
    x11vnc_unit = (
        "[Unit]\n"
        "Description=x11vnc mirror of X display %i (mesh-only bind)\n"
        "After=display-manager.service\n"
        "Wants=display-manager.service\n"
        "\n"
        "[Service]\n"
        # Bind to the mesh IP only — falls back to localhost if mesh not up.
        # The active display owner (DISPLAY :0) is read via X11 cookie.
        "ExecStart=/usr/bin/x11vnc -display %i -auth guess -forever "
        "-shared -rfbport 5900 -noxdamage -nopw "
        "-listen ${MESH_BIND:-127.0.0.1}\n"
        "Environment=MESH_BIND=127.0.0.1\n"
        "Restart=on-failure\n"
        "RestartSec=5\n"
        "\n"
        "[Install]\n"
        "WantedBy=graphical.target\n"
    )
    _write_root_file(Path("/etc/systemd/system/x11vnc@.service"), x11vnc_unit)

    # mackes-remote-sync.service — regenerate Guacamole config every 30s
    sync_unit = (
        "[Unit]\n"
        "Description=Mackes Shell — sync Headscale peers to Guacamole config\n"
        "After=network-online.target headscale.service\n"
        "\n"
        "[Service]\n"
        "Type=simple\n"
        "ExecStart=/usr/bin/python3 -m mackes.remote_desktop --daemon\n"
        "Restart=on-failure\n"
        "RestartSec=10\n"
        "\n"
        "[Install]\n"
        "WantedBy=multi-user.target\n"
    )
    _write_root_file(Path("/etc/systemd/system/mackes-remote-sync.service"),
                     sync_unit)

    _run_root(["systemctl", "daemon-reload"])

    # ---- 6. Firewall — open ports on the mesh-trusted zone only ------
    if shutil.which("firewall-cmd"):
        actions.append("remote-desktop: opening firewall ports on mesh-trusted zone")
        for port in ("3389/tcp", "5900/tcp", "8080/tcp"):
            _run_root([
                "firewall-cmd", "--permanent",
                "--zone=trusted", f"--add-port={port}",
            ])
        _run_root(["firewall-cmd", "--reload"])

    # ---- 7. Enable + start ------------------------------------------
    actions.append("remote-desktop: enabling + starting daemons")
    for unit in ("xrdp.service", "xrdp-sesman.service",
                 "x11vnc@:0.service", "guacd.service",
                 "tomcat.service", "mackes-remote-sync.service"):
        rc, _ = _run_root(["systemctl", "enable", "--now", unit], timeout=60)
        if rc == 0:
            actions.append(f"  {unit}: enabled + started")
        else:
            actions.append(f"  {unit}: enable failed (rc={rc})")

    actions.append(
        "remote-desktop: ready — open https://media.mesh/desktop/ "
        "on any peer to access the connection picker"
    )
    for line in actions:
        log_action(line)
    return actions


def _write_root_file(path: Path, content: str) -> None:
    """Write `content` to `path` with root privileges."""
    import tempfile
    with tempfile.NamedTemporaryFile("w", suffix=".tmp", delete=False) as tf:
        tf.write(content)
        tmp_path = tf.name
    try:
        _run_root(["mkdir", "-p", str(path.parent)])
        _run_root(["install", "-m", "0644", tmp_path, str(path)])
    finally:
        try:
            os.unlink(tmp_path)
        except OSError:
            pass


# ---------------------------------------------------------------------------
# 10. Fleet management — ansible-pull on every peer (v1.3.0 birthright)
# ---------------------------------------------------------------------------


def apply_fleet(_preset: Preset) -> List[str]:
    """Install ansible-core + python3-ansible-runner; seed the QNM-Shared
    playbook tree from the Mackes-shipped curated set; install + enable
    the mackes-ansible-pull systemd timer.

    Locks v1.3.0 design decisions:
      - Transport: ansible-pull (no central controller)
      - Playbook store: ~/QNM-Shared/.qnm-sync/playbooks/
      - Schedule: 30-min timer with 5-min jitter
      - Run history: 30-day retention
    """
    actions: List[str] = []
    if shutil.which("dnf") is None:
        actions.append("fleet: dnf not available — skipping")
        return actions

    # ---- 1. dnf install ----------------------------------------------
    needed_pkgs = ["ansible-core", "python3-ansible-runner", "podman"]
    to_install = [p for p in needed_pkgs if _run(["rpm", "-q", p])[0] != 0]
    if to_install:
        actions.append(f"fleet: installing {', '.join(to_install)} via dnf")
        rc, out = _run_root(["dnf", "install", "-y", *to_install], timeout=900)
        if rc != 0:
            last = out.strip().splitlines()[-1] if out.strip() else f"rc={rc}"
            actions.append(f"fleet: dnf install failed: {last}")
            return actions
    else:
        actions.append("fleet: ansible-core + ansible-runner + podman already installed")

    # ---- 2. Seed QNM-Shared/.qnm-sync/playbooks/ ---------------------
    src = _find_data("ansible", "playbooks")
    if src is None:
        actions.append("fleet: curated playbook source missing — skipping seed")
    else:
        home = Path(os.path.expanduser("~"))
        dst = home / "QNM-Shared" / ".qnm-sync" / "playbooks"
        dst.parent.mkdir(parents=True, exist_ok=True)
        if dst.exists():
            actions.append(f"fleet: playbook tree already present at {dst}")
        else:
            rc, out = _run(["cp", "-rT", str(src), str(dst)])
            if rc == 0:
                actions.append(f"fleet: seeded curated playbooks → {dst}")
            else:
                actions.append(f"fleet: seed failed: {out.strip().splitlines()[-1] if out.strip() else rc}")

    # ---- 3. systemd units -------------------------------------------
    service_src = _find_data("systemd", "mackes-ansible-pull.service")
    timer_src   = _find_data("systemd", "mackes-ansible-pull.timer")
    if service_src and timer_src:
        actions.append("fleet: installing systemd service + timer")
        _run_root(["install", "-m", "0644", str(service_src),
                   "/etc/systemd/system/mackes-ansible-pull.service"])
        _run_root(["install", "-m", "0644", str(timer_src),
                   "/etc/systemd/system/mackes-ansible-pull.timer"])
        _run_root(["systemctl", "daemon-reload"])
    else:
        actions.append("fleet: systemd unit source missing — skipping install")

    # ---- 4. Enable + start timer ------------------------------------
    rc, _ = _run_root(["systemctl", "enable", "--now",
                       "mackes-ansible-pull.timer"], timeout=30)
    if rc == 0:
        actions.append("fleet: mackes-ansible-pull.timer enabled + started")
    else:
        actions.append(f"fleet: timer enable failed (rc={rc})")

    # ---- 5. Initial pull (background, non-blocking) -----------------
    # Kick off the first pull right away so the wizard exit lands with
    # the fleet already converged. We don't wait for it.
    if shutil.which("systemctl"):
        _run_root(["systemctl", "start", "--no-block",
                   "mackes-ansible-pull.service"])
        actions.append("fleet: initial pull queued (runs in background)")

    actions.append(
        "fleet: ready — view runs in Mackes → Fleet → Run history, "
        "or trigger ad-hoc with Fleet → Inventory → Run on selection"
    )
    for line in actions:
        log_action(line)
    return actions


# ---------------------------------------------------------------------------
# 11. Conky HUD — Mackes-themed right-side desktop panel (v1.4.0 birthright)
# ---------------------------------------------------------------------------


def apply_conky(_preset: Preset) -> List[str]:
    """Install conky + write the Mackes config + register XDG autostart.

    Q2 lock: birthright autostart + Tweaks toggle. The Tweaks panel can
    later flip it off without uninstalling conky.
    """
    actions: List[str] = []

    # ---- 1. Install Fedora package ------------------------------------
    if shutil.which("dnf") is None:
        actions.append("conky: dnf not available — skipping")
        return actions
    if shutil.which("conky") is None:
        actions.append("conky: installing via dnf")
        rc, out = _run_root(["dnf", "install", "-y", "conky"], timeout=300)
        if rc != 0:
            last = out.strip().splitlines()[-1] if out.strip() else f"rc={rc}"
            actions.append(f"conky: install failed: {last}")
            return actions
    else:
        actions.append("conky: already installed")

    # ---- 2. Write user config + autostart ----------------------------
    try:
        from mackes.conky_hud import (
            write_config, install_autostart, start, USER_CONFIG, AUTOSTART_FILE,
        )
    except Exception as e:  # noqa: BLE001
        actions.append(f"conky: orchestrator import failed: {e}")
        return actions

    try:
        cfg = write_config()
        actions.append(f"conky: wrote user config to {cfg}")
    except Exception as e:  # noqa: BLE001
        actions.append(f"conky: config write failed: {e}")
        return actions

    try:
        a = install_autostart()
        actions.append(f"conky: installed autostart at {a}")
    except Exception as e:  # noqa: BLE001
        actions.append(f"conky: autostart install failed: {e}")

    # ---- 3. Start it now so the user sees it immediately -------------
    if start(force=True):
        actions.append("conky: HUD started")
    else:
        actions.append("conky: HUD will start at next login (autostart)")

    for line in actions:
        log_action(line)
    return actions


# ---------------------------------------------------------------------------
# 12. Always-maximize windows — wmctrl-based maximizer (v1.4.1 birthright)
# ---------------------------------------------------------------------------


def apply_maximize_all(_preset: Preset) -> List[str]:
    """Install + enable the mackes-maximizer user service.

    Every new top-level window will start maximized. Toggleable via
    Tweaks → 'Always maximize windows' or by creating the file
    ~/.config/mackes-shell/maximizer.disabled.
    """
    actions: List[str] = []
    if shutil.which("dnf") is None:
        actions.append("maximize-all: dnf not available — skipping")
        return actions

    # ---- 1. Install wmctrl + xprop ----------------------------------
    needed = []
    if shutil.which("wmctrl") is None:
        needed.append("wmctrl")
    if shutil.which("xprop") is None:
        needed.append("xprop")
    if needed:
        actions.append(f"maximize-all: installing {', '.join(needed)} via dnf")
        rc, out = _run_root(["dnf", "install", "-y", *needed], timeout=300)
        if rc != 0:
            last = out.strip().splitlines()[-1] if out.strip() else f"rc={rc}"
            actions.append(f"maximize-all: install failed: {last}")
            return actions
    else:
        actions.append("maximize-all: wmctrl + xprop already installed")

    # ---- 2. Ensure systemd user unit + autostart are reachable -------
    # The RPM installs mackes-maximizer.service + mackes-maximizer.desktop
    # to system paths; the wizard just has to enable the user unit and
    # let the autostart .desktop fire on graphical login.
    if shutil.which("systemctl"):
        rc, _ = _run(["systemctl", "--user", "enable", "--now",
                       "mackes-maximizer.service"], timeout=10)
        if rc == 0:
            actions.append("maximize-all: mackes-maximizer.service enabled + started")
        else:
            actions.append(
                "maximize-all: user-systemctl enable failed; will rely on "
                "XDG autostart at next graphical login"
            )

    # ---- 3. Clear the disable-flag file in case it's leftover from
    # a previous opt-out --------------------------------------------------
    disabled_flag = Path(os.path.expanduser(
        "~/.config/mackes-shell/maximizer.disabled"))
    if disabled_flag.exists():
        try:
            disabled_flag.unlink()
            actions.append("maximize-all: cleared disable flag")
        except OSError:
            pass

    actions.append(
        "maximize-all: ready — toggle via Tweaks → 'Always maximize windows'"
    )
    for line in actions:
        log_action(line)
    return actions


# ---------------------------------------------------------------------------
# 13. Mesh clipboard — XA_CLIPBOARD ↔ QNM-Shared sync (v1.5.0 birthright)
# ---------------------------------------------------------------------------


def apply_clipboard_daemon(_preset: Preset) -> List[str]:
    """Install + enable mackes-clipboard-daemon.service (user unit).

    The daemon watches the X11 clipboard and publishes every new text /
    image item to ~/QNM-Shared/.qnm-sync/clipboard/<me>/<ts>.{txt,png}.
    Other peers' subdirs are surfaced by the existing mackes-clipboard
    GUI + the C panel plugin. Heuristic secret filter on by default;
    toggleable via Tweaks → 'Sync sensitive items'.
    """
    actions: List[str] = []

    # The service file is installed by the RPM; just enable it.
    if shutil.which("systemctl") is None:
        actions.append("clipboard: systemctl not available — skipping")
        return actions
    rc, _ = _run(["systemctl", "--user", "enable", "--now",
                   "mackes-clipboard-daemon.service"], timeout=10)
    if rc == 0:
        actions.append("clipboard: mackes-clipboard-daemon.service enabled + started")
    else:
        actions.append(
            "clipboard: user-systemctl enable failed; will rely on "
            "XDG autostart at next graphical login"
        )

    # Clear any leftover disable flag from a previous opt-out.
    disabled_flag = Path(os.path.expanduser(
        "~/.config/mackes-shell/clipboard.disabled"))
    if disabled_flag.exists():
        try:
            disabled_flag.unlink()
            actions.append("clipboard: cleared disable flag")
        except OSError:
            pass

    actions.append(
        "clipboard: ready — open Mesh Clipboard from the panel or "
        "`mackes-clipboard` from a terminal to see peer history"
    )
    for line in actions:
        log_action(line)
    return actions


def apply_flathub(_preset: Preset) -> List[str]:
    """Add the Flathub remote so flatpak apps are discoverable.

    Per-user (`--user`) so we don't need root. Flatpak is shipped on
    Fedora Workstation by default; we no-op if it's missing.
    """
    actions: List[str] = []
    if shutil.which("flatpak") is None:
        actions.append("flathub: flatpak not installed — skipping")
        return actions

    # Check whether the remote already exists for the current user.
    rc, out = _run(["flatpak", "remotes", "--user", "--columns=name"])
    if rc == 0 and any(line.strip() == "flathub" for line in (out or "").splitlines()):
        actions.append("flathub: per-user remote already configured")
        return actions

    rc, out = _run(
        ["flatpak", "remote-add", "--user", "--if-not-exists", "flathub",
         "https://dl.flathub.org/repo/flathub.flatpakrepo"],
        timeout=60,
    )
    if rc == 0:
        actions.append("flathub: added per-user remote")
    else:
        last = out.strip().splitlines()[-1] if out.strip() else f"rc={rc}"
        actions.append(f"flathub: remote-add failed: {last}")
    for line in actions:
        log_action(line)
    return actions
