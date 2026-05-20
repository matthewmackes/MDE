"""Birthright — first-run install steps that turn a stock XFCE box into Mackes.

Each function is idempotent (safe to re-run via Maintain → Reset to Preset)
and returns a `list[str]` of action lines for the wizard's apply page log.

These are the fourteen "birthright" items the v1.5.2 wizard runs in
addition to the v1.0.x xfconf-only apply pipeline:

  1. apply_themes              — deploy Orchis-Dark + Shiki-Statler GTK + Black-Sun icon
  2. apply_fonts               — install Red Hat Text + Mono via dnf
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
 11. apply_drawer              — Notification Drawer (v2.2.0 lock):
                                  ensures the cache dir exists for the
                                  mackes-drawer xfce4-panel plugin and
                                  sweeps legacy conky / tray autostarts
 12. (retired in 1.0.7)        — apply_maximize_all was the mackes-maximizer enabler,
                                  via the mackes-maximizer user service
                                  (v1.4.1 lock)
 13. apply_clipboard_daemon    — mesh clipboard daemon: bidirectional sync
                                  between XA_CLIPBOARD and QNM-Shared
                                  clipboard bucket (v1.5.0 lock)
 14. apply_qnm                 — Quick Network Mesh: dnf install qnm,
                                  enable qnm.service, run qnmctl init
                                  (v1.5.2 lock)
 15. apply_panel_swap          — Phase 10.6.1-4: start mackes-panel,
                                  retire xfce4-panel + xfdesktop, unbind
                                  the Whisker Super-key (v1.0.7 lock)
 16. apply_panel_archive       — Phase 10.6.7: archive the user's
                                  pre-1.0 xfce4-panel state under
                                  ~/.config/mackes-panel/legacy-xfce-panel/
 17. apply_enforce_i3          — Phase 8.8: replace xfwm4 with i3, retire
                                  the mackes-maximizer service (v1.0.7)
 18. apply_user_dirs           — Phase 1.1.0: remap XDG user-dirs to
                                  ~/QNM-Mesh subdirs + local Downloads
 19. apply_uninstall_legacy_xfce — Phase 10.6.6: single dnf-remove of the
                                  six legacy XFCE packages mackes-panel
                                  has supplanted (gated on 10.6.1-4)

All wired into mackes/wizard/pages/apply.py between Panel and Mesh.
"""
from __future__ import annotations

import os
import re
import shutil
import subprocess
from pathlib import Path
from typing import List, Optional

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
# 1. Themes — deploy the vendored Orchis-Dark + Shiki-Statler GTK themes
#    and the Black-Sun icon theme to /usr/share/{themes,icons}/.
# ---------------------------------------------------------------------------


_VENDORED_THEMES: tuple[tuple[str, str, str, int], ...] = (
    # (subdir, name,             upstream comment for the action log,    cp timeout)
    ("themes", "Orchis-Dark",    "github.com/vinceliuice/Orchis-theme", 120),
    ("themes", "Shiki-Statler",  "sourceforge.net/projects/archbangretro", 60),
    ("icons",  "Black-Sun",      "github.com/SethStormR/Black-Sun",     300),
    ("icons",  "Mackes-Carbon",  "carbon-design-system/carbon (Apache 2.0)", 180),
)


def apply_themes(_preset: Preset) -> List[str]:
    """Deploy every vendored theme to /usr/share/{themes,icons}/ and
    refresh icon caches. Idempotent — skips a theme when the destination
    dir's mtime is at-or-newer than the source's."""
    actions: List[str] = []

    for subdir, name, _upstream, timeout in _VENDORED_THEMES:
        src = _find_data(subdir, name)
        dst = Path(f"/usr/share/{subdir}/{name}")
        if src is None:
            actions.append(f"themes: {name} source missing — skipping")
            continue
        if _newer_than(dst, src):
            actions.append(f"themes: {name} already installed (up to date)")
            continue
        rc, out = _run_root(["cp", "-rT", str(src), str(dst)],
                             timeout=timeout)
        if rc != 0:
            last = (out.strip().splitlines()[-1]
                    if out.strip() else f"rc={rc}")
            actions.append(f"themes: {name} install failed: {last}")
            continue
        actions.append(f"themes: installed {name} to {dst}")
        if subdir == "icons" and shutil.which("gtk-update-icon-cache"):
            _run_root(["gtk-update-icon-cache", "-f", "-t", str(dst)],
                       timeout=60)
            actions.append(f"themes: rebuilt {name} icon cache")

    for line in actions:
        log_action(line)
    return actions


def _newer_than(dst: Path, src: Path) -> bool:
    """Skip-already-installed check: compare top-level directory mtimes
    only. Walking every file is `os.stat()` per icon — on themes like
    Black-Sun (~2.5k SVGs) or Orchis (~3k files) that's seconds of
    stat() per apply. The top-level dir's mtime is updated by `cp -rT`
    when the contents change, so it's a sufficient invalidation signal.
    """
    if not dst.exists():
        return False
    try:
        return dst.stat().st_mtime >= src.stat().st_mtime
    except OSError:
        return False


# ---------------------------------------------------------------------------
# 2. Fonts — dnf install Red Hat Display + Text + Mono (PF v6 stack)
# ---------------------------------------------------------------------------


# v2.0.0 — PatternFly's official font stack is Red Hat Display (headings),
# Red Hat Text (body), Red Hat Mono (code). All three are on Fedora main.
# Red Hat stays as a CSS fallback in data/css/tokens.css for hosts that
# haven't yet run this birthright step (e.g. on-the-fly developer
# workstations) — the apply pipeline replaces them at first-run.
_FONT_PACKAGES = (
    "redhat-display-fonts",
    "redhat-text-fonts",
    "redhat-mono-fonts",
)

# Hack Nerd Font — installed from upstream because Fedora doesn't
# package any nerd-font (only the base hack-fonts without PUA glyphs).
# Conky's HUD uses Hack Nerd Font for the section icons.
_NERD_FONT_VERSION = "3.2.1"
_NERD_FONT_URL = (
    "https://github.com/ryanoasis/nerd-fonts/releases/download/"
    f"v{_NERD_FONT_VERSION}/Hack.tar.xz"
)
_NERD_FONT_DEST = Path("/usr/local/share/fonts/HackNerdFont")


def apply_fonts(_preset: Preset) -> List[str]:
    """Install Red Hat Display + Text + Mono via dnf, plus Hack Nerd Font
    from upstream. Idempotent."""
    actions: List[str] = []
    if shutil.which("dnf") is None:
        actions.append("fonts: dnf not available — skipping")
        return actions

    # Skip if already installed
    needed = []
    for pkg in _FONT_PACKAGES:
        rc, _ = _run(["rpm", "-q", pkg])
        if rc != 0:
            needed.append(pkg)
    if not needed:
        actions.append("fonts: Red Hat font stack already installed")
    else:
        rc, out = _run_root(["dnf", "install", "-y", *needed], timeout=600)
        if rc == 0:
            actions.append(f"fonts: installed {', '.join(needed)}")
        else:
            last = out.strip().splitlines()[-1] if out.strip() else f"rc={rc}"
            actions.append(f"fonts: Red Hat fonts install failed: {last}")

    actions.extend(_apply_nerd_font())

    if shutil.which("fc-cache"):
        _run_root(["fc-cache", "-fv"], timeout=120)
        actions.append("fonts: rebuilt fontconfig cache")

    for line in actions:
        log_action(line)
    return actions


def _apply_nerd_font() -> List[str]:
    """Install Hack Nerd Font under /usr/local/share/fonts/ if missing."""
    actions: List[str] = []
    # Already installed system-wide (any HackNerdFont*.ttf in /usr/local
    # or /usr/share counts)?
    if _hack_nerd_present():
        return ["fonts: Hack Nerd Font already present"]

    if shutil.which("curl") is None or shutil.which("tar") is None:
        return ["fonts: curl/tar missing — skipping Hack Nerd Font"]

    # Stage in a temp dir, then move to /usr/local via root.
    import tempfile
    with tempfile.TemporaryDirectory(prefix="mackes-nerd-") as td:
        tar = Path(td) / "Hack.tar.xz"
        rc, _ = _run(["curl", "-fsSL", "-o", str(tar), _NERD_FONT_URL],
                     timeout=300)
        if rc != 0 or not tar.is_file():
            return [f"fonts: Hack Nerd Font download failed (curl rc={rc})"]
        extract_dir = Path(td) / "extracted"
        extract_dir.mkdir()
        rc, _ = _run(["tar", "-xJf", str(tar), "-C", str(extract_dir)],
                     timeout=120)
        if rc != 0:
            return [f"fonts: Hack Nerd Font extract failed (tar rc={rc})"]
        # Move just the TTFs to /usr/local/share/fonts/HackNerdFont/.
        # The release tar contains many variants (Mono, Propo) — ship them
        # all; total is ~14 MB.
        _run_root(["mkdir", "-p", str(_NERD_FONT_DEST)], timeout=10)
        rc, _ = _run_root(
            ["sh", "-c",
             f"cp {extract_dir}/*.ttf {_NERD_FONT_DEST}/ 2>/dev/null"],
            timeout=30,
        )
        if rc != 0:
            return [f"fonts: Hack Nerd Font install failed (cp rc={rc})"]
    actions.append(f"fonts: installed Hack Nerd Font v{_NERD_FONT_VERSION}")
    return actions


def _hack_nerd_present() -> bool:
    """True if Hack Nerd Font is registered with fontconfig."""
    if shutil.which("fc-list") is None:
        return _NERD_FONT_DEST.is_dir() and any(_NERD_FONT_DEST.glob("*.ttf"))
    try:
        r = subprocess.run(["fc-list", ":family"], capture_output=True,
                           text=True, timeout=5)
        return "Hack Nerd Font" in r.stdout
    except (OSError, subprocess.TimeoutExpired):
        return False


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


_PANEL_SNAPSHOT = "panel/xfce4-panel.snapshot.json"

# Properties skipped at apply time. These are caches the panel populates
# itself at runtime — shipping them would leak the snapshotting box's
# usage history (Wi-Fi SSIDs in known-legacy-items, app names in
# known-items, etc.) to every fresh install.
_PANEL_TRANSIENT_PATTERNS = (
    re.compile(r"^/plugins/plugin-\d+/known-items$"),
    re.compile(r"^/plugins/plugin-\d+/known-legacy-items$"),
)


def _panel_snapshot_path() -> Optional[Path]:
    p = _find_data("panel", "xfce4-panel.snapshot.json")
    return p if p and p.is_file() else None


def _panel_rc_dir() -> Optional[Path]:
    p = _find_data("panel", "panel-rc")
    return p if p and p.is_dir() else None


def _panel_is_transient(prop: str) -> bool:
    return any(rx.match(prop) for rx in _PANEL_TRANSIENT_PATTERNS)


def _xfconf_set(channel: str, prop: str, ty: str, value, timeout: int = 10
                ) -> tuple[int, str]:
    """Set a single xfconf property by (type, value) — handles arrays.

    `ty` is the type tag from the snapshot:
        bool / int / uint / double / string  → scalar
        array-<elem-type>                     → array of <elem-type>
    """
    if ty.startswith("array-"):
        elem_ty = ty[len("array-"):]
        # Wipe any prior array first so size changes cleanly.
        _run(["xfconf-query", "--channel", channel,
              "--property", prop, "--reset"], timeout=timeout)
        cmd = ["xfconf-query", "--channel", channel,
               "--property", prop, "--create", "--force-array"]
        for v in value or []:
            cmd.extend(["--type", elem_ty, "--set", _xfconf_str(elem_ty, v)])
        return _run(cmd, timeout=timeout)
    return _run(["xfconf-query", "--channel", channel,
                 "--property", prop, "--create",
                 "--type", ty, "--set", _xfconf_str(ty, value)],
                timeout=timeout)


def _xfconf_str(ty: str, value) -> str:
    """Coerce a Python value back to the string form xfconf-query expects."""
    if ty == "bool":
        return "true" if value else "false"
    return str(value)


PANEL_PROFILE_FILE = "panel/xfce4-panel-profile.tar.bz2"


def apply_panel_layout(_preset: Preset) -> List[str]:
    """Install the shipped xfce4-panel layout via xfce4-panel-profiles.

    The profile archive (data/panel/xfce4-panel-profile.tar.bz2) is a
    snapshot captured with `xfce4-panel-profiles save`. It bundles:
      * the full xfconf dump with the right GVariant types
        (uint32, GVariant-array, etc.) — these were the source of
        every "Plugin (null) could not be loaded" crash in 1.6.x
        when we tried to write the layout by hand.
      * per-launcher .desktop RC files under launcher-N/

    `xfce4-panel-profiles load` handles --quit + restart of the
    panel internally. If the tool isn't installed, we leave the
    user's existing panel layout untouched — never half-apply.

    Re-snapshot the shipped default with:
        xfce4-panel-profiles save \\
            data/panel/xfce4-panel-profile.tar.bz2
    on a reference machine.
    """
    actions: List[str] = []
    tool = shutil.which("xfce4-panel-profiles")
    if tool is None:
        actions.append(
            "panel layout: xfce4-panel-profiles not installed — "
            "leaving panel layout untouched. "
            "`dnf install xfce4-panel-profiles` for the Mackes default."
        )
        log_action(actions[-1])
        return actions

    profile = _find_data(*PANEL_PROFILE_FILE.split("/"))
    if profile is None:
        actions.append(
            "panel layout: shipped profile archive missing — skipping")
        log_action(actions[-1])
        return actions

    rc, out = _run([tool, "load", str(profile)], timeout=30)
    if rc == 0:
        actions.append(f"panel: applied profile {profile.name}")
    else:
        last = out.strip().splitlines()[-1] if out.strip() else f"rc={rc}"
        actions.append(f"panel: profile load failed: {last}")
    log_action(actions[-1])
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
# 11. Notification Drawer — single applet replacing Conky + tray + popover
#     (v2.2.0 birthright). The C panel plugin (mackes-drawer) ships with the
#     RPM under /usr/lib/xfce4/panel/plugins/mackes-drawer and is registered
#     via /usr/share/xfce4/panel/plugins/mackes-drawer.desktop. The xfce4-
#     panel-profiles archive (data/panel/xfce4-panel-profile.tar.bz2) wires
#     it into the default panel layout — we just need to make sure the
#     cache dir exists so the plugin's state-file reads don't ENOENT, and
#     that any stray legacy autostart (conky / mackes-tray) is removed
#     from the user's session.
# ---------------------------------------------------------------------------


def apply_drawer(_preset: Preset) -> List[str]:
    """Set up the cache dir the Notification Drawer reads/writes, and
    sweep away the autostart entries the v1.x conky + tray surfaces
    left behind so they don't double-render alongside the new pill."""
    actions: List[str] = []

    cache = Path(os.path.expanduser("~/.cache/mackes"))
    try:
        cache.mkdir(parents=True, exist_ok=True)
        actions.append(f"drawer: cache dir ready at {cache}")
    except OSError as e:
        actions.append(f"drawer: could not create cache dir: {e}")

    # Sweep legacy autostarts so we don't run the dead surfaces alongside
    # the new drawer pill.
    for legacy in (
        Path(os.path.expanduser("~/.config/autostart/mackes-conky.desktop")),
        Path(os.path.expanduser("~/.config/autostart/mackes-tray.desktop")),
    ):
        if legacy.exists():
            try:
                legacy.unlink()
                actions.append(f"drawer: removed legacy autostart {legacy}")
            except OSError as e:
                actions.append(f"drawer: could not remove {legacy}: {e}")

    # Stop any orphan conky process from a previous install.
    if shutil.which("pkill"):
        _run(["pkill", "-x", "conky"], timeout=5)

    for line in actions:
        log_action(line)
    return actions


# ---------------------------------------------------------------------------
# 12. (retired) Always-maximize windows — apply_maximize_all
#
#     Removed in Phase 8.8 (1.0.7). The mackes-maximizer.service was an
#     xfwm4 crutch; i3 tiles natively so the auto-maximize behavior is
#     no longer needed. apply_enforce_i3 (step 17 below) handles the
#     stop/disable of the legacy service on existing 1.0.6 installs.
# ---------------------------------------------------------------------------


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


# ---------------------------------------------------------------------------
# 14. Quick Network Mesh (QNM) — install + enable (v1.5.2 birthright)
# ---------------------------------------------------------------------------


def apply_qnm(preset: Preset) -> List[str]:
    """Install QNM via dnf, run qnmctl init, enable qnm.service.

    QNM may not be in stock Fedora repos — when `dnf install qnm` fails
    the step logs a clear "not available in current repos" message and
    returns. Birthright is graceful: installing QNM later via the Apps
    panel finishes the wiring.
    """
    actions: List[str] = []
    if shutil.which("dnf") is None:
        actions.append("qnm: dnf not available — skipping")
        return actions

    # Honor the preset's network.qnm_enabled flag — if the preset
    # explicitly opts out we don't install.
    qnm_pref = (preset.network or {}).get("qnm_enabled", True)
    if not qnm_pref:
        actions.append("qnm: preset has qnm_enabled=false — skipping")
        return actions

    # ---- 1. Install qnm (best-effort, may not be in stock repos) ----
    if shutil.which("qnmctl") is None:
        actions.append("qnm: installing via dnf")
        rc, out = _run_root(["dnf", "install", "-y", "qnm"], timeout=300)
        if rc != 0:
            last = out.strip().splitlines()[-1] if out.strip() else f"rc={rc}"
            actions.append(
                f"qnm: dnf install failed: {last}. "
                "QNM may not be in your repo set — install manually via "
                "Apps panel or `dnf install qnm` once the repo is available."
            )
            for line in actions:
                log_action(line)
            return actions
    else:
        actions.append("qnm: qnmctl already installed")

    # ---- 2. qnmctl init (idempotent — does nothing if already set up) ----
    if shutil.which("qnmctl"):
        rc, out = _run_root(["qnmctl", "init"], timeout=60)
        if rc == 0:
            actions.append("qnm: qnmctl init OK")
        else:
            last = out.strip().splitlines()[-1] if out.strip() else f"rc={rc}"
            actions.append(f"qnm: qnmctl init: {last}")

    # ---- 3. Enable + start the qnm system service --------------------
    rc, out = _run_root(["systemctl", "enable", "--now", "qnm.service"],
                        timeout=30)
    if rc == 0:
        actions.append("qnm: qnm.service enabled + started")
    else:
        last = out.strip().splitlines()[-1] if out.strip() else f"rc={rc}"
        actions.append(f"qnm: service enable: {last}")

    # ---- 4. Set the preset-level qnm flag on so the Mackes UI knows --
    try:
        from mackes.qnm_bridge import set_qnm_enabled
        actions.extend(set_qnm_enabled(True))
    except Exception as e:  # noqa: BLE001
        actions.append(f"qnm: set_qnm_enabled fallback: {e}")

    actions.append(
        "qnm: ready — open Network → Quick Network Mesh (QNM) for the "
        "control panel, or run `qnmctl status` from a terminal."
    )
    for line in actions:
        log_action(line)
    return actions


# ---------------------------------------------------------------------------
# 15. LightDM greeter — promoted from apply_appearance (v1.6.0 birthright)
# ---------------------------------------------------------------------------


def apply_lightdm(preset: Preset) -> List[str]:
    """Configure the LightDM GTK greeter to match the active preset.

    This was previously folded into apply_appearance — promoted to its
    own step in v1.6.0 so the wizard rail surfaces it explicitly.
    Writes /etc/lightdm/lightdm-gtk-greeter.conf via AdminSession so
    the sudoers NOPASSWD drop-in covers it without a prompt.
    """
    actions: List[str] = []
    try:
        from mackes.lightdm import configure_from_preset
        from pathlib import Path as _P
        wp = preset.appearance.get("wallpaper") if preset.appearance else None
        wp_path = _P(str(wp)) if wp else None
        actions.extend(configure_from_preset(preset.name, wallpaper=wp_path))
    except Exception as e:
        actions.append(f"lightdm: {e}")
    for line in actions:
        log_action(line)
    return actions


# ---------------------------------------------------------------------------
# 16. Thunar @ QNM-Mesh — open mesh file browser every graphical login
# ---------------------------------------------------------------------------


_THUNAR_AUTOSTART_PATH = Path(os.path.expanduser(
    "~/.config/autostart/mackes-thunar-mesh.desktop"))


def apply_thunar_autostart(_preset: Preset) -> List[str]:
    """Write an XDG autostart entry that opens Thunar at ~/QNM-Mesh."""
    actions: List[str] = []
    if shutil.which("thunar") is None:
        actions.append("thunar: not installed - skipping autostart entry")
        return actions
    home = Path(os.path.expanduser("~"))
    mesh_dir = home / "QNM-Mesh"
    try:
        mesh_dir.mkdir(parents=True, exist_ok=True)
    except OSError as e:
        actions.append(f"thunar: could not create {mesh_dir}: {e}")
    _THUNAR_AUTOSTART_PATH.parent.mkdir(parents=True, exist_ok=True)
    contents = (
        "[Desktop Entry]\n"
        "Type=Application\n"
        "Name=Mackes Mesh Files\n"
        "Comment=Open Thunar at the mesh root every graphical login\n"
        f"Exec=thunar {mesh_dir}\n"
        "Icon=folder-remote\n"
        "Categories=System;FileManager;\n"
        "Terminal=false\n"
        "X-GNOME-Autostart-enabled=true\n"
        "X-Mackes-Managed=1\n"
        "StartupNotify=false\n"
        "NoDisplay=false\n"
    )
    try:
        _THUNAR_AUTOSTART_PATH.write_text(contents, encoding="utf-8")
        actions.append(f"thunar: installed autostart at {_THUNAR_AUTOSTART_PATH}")
    except OSError as e:
        actions.append(f"thunar: autostart write failed: {e}")
    for line in actions:
        log_action(line)
    return actions


def apply_hotkey(_preset: Preset) -> List[str]:
    """Bind <Super>m to `mackes --drawer` via xfconf.

    v2.2.0 — the Notification Drawer is reachable via the panel pill
    AND this keyboard shortcut. Idempotent on re-run.
    """
    actions: List[str] = []
    if shutil.which("xfconf-query") is None:
        actions.append("hotkey: xfconf-query not installed — skipping")
        return actions
    # Find the actual mackes binary so the binding works even if PATH
    # is exotic at session start.
    mackes_bin = shutil.which("mackes") or "/usr/bin/mackes"
    command = f"{mackes_bin} --drawer"
    key = "/commands/custom/<Super>m"
    rc, out = _run(
        ["xfconf-query", "--channel", "xfce4-keyboard-shortcuts",
         "--property", key, "--create", "--type", "string",
         "--set", command],
        timeout=5,
    )
    if rc == 0:
        actions.append(f"hotkey: bound <Super>m → {command}")
    else:
        last = (out.strip().splitlines()[-1] if out.strip() else rc)
        actions.append(f"hotkey: bind failed: {last}")
    for line in actions:
        log_action(line)
    return actions


# ---------------------------------------------------------------------------
# Media clients — Sublime Music (Airsonic) + Delfin (Jellyfin) via Flathub.
# v2.1.0 lock: GTK-native clients auto-configured against discovered
# mesh media servers by mackes-media-sync.service.
# ---------------------------------------------------------------------------


_MEDIA_FLATPAKS: tuple[tuple[str, str], ...] = (
    ("com.sublimemusic.SublimeMusic", "Sublime Music"),
    ("app.drey.Delfin",               "Delfin"),
)


def apply_media_clients(_preset: Preset) -> List[str]:
    """Install Sublime Music + Delfin from Flathub and enable the
    media-sync user timer. Idempotent."""
    actions: List[str] = []

    if shutil.which("flatpak") is None:
        actions.append("media-clients: flatpak not installed — skipping")
        return actions

    for app_id, name in _MEDIA_FLATPAKS:
        # Check if already installed for the user.
        rc, out = _run(["flatpak", "list", "--user", "--columns=application"])
        if rc == 0 and any(line.strip() == app_id for line in (out or "").splitlines()):
            actions.append(f"media-clients: {name} already installed")
            continue
        rc, out = _run(
            ["flatpak", "install", "--user", "--noninteractive",
             "--assumeyes", "flathub", app_id],
            timeout=600,
        )
        if rc == 0:
            actions.append(f"media-clients: installed {name} ({app_id})")
        else:
            last = out.strip().splitlines()[-1] if out.strip() else f"rc={rc}"
            actions.append(f"media-clients: {name} install failed: {last}")

    # Enable the media-sync user timer so configs refresh every 60s.
    if shutil.which("systemctl"):
        rc, _ = _run(["systemctl", "--user", "enable", "--now",
                      "mackes-media-sync.timer"], timeout=10)
        if rc == 0:
            actions.append("media-clients: enabled mackes-media-sync.timer")
        else:
            actions.append(
                "media-clients: user-systemctl enable failed; will rely on "
                "next login to start the timer"
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


# ---------------------------------------------------------------------------
# 15. apply_panel_swap — Phase 10.6.1-4 of the v1.0.0 work.
# ---------------------------------------------------------------------------

def apply_panel_swap(_preset: Preset) -> List[str]:
    """Start mackes-panel, then quit + disable xfce4-panel + xfdesktop,
    then unbind the Whisker Super-key.

    Idempotent. Each step is gated on the previous succeeding; failure
    aborts the remaining steps and leaves the user in a recoverable
    state (mackes-panel + xfce4-panel can coexist briefly until the
    user re-runs the wizard).
    """
    actions: List[str] = []
    home = Path(os.path.expanduser("~"))

    # 10.6.1 — Start mackes-panel.
    if shutil.which("mackes-panel") is None:
        actions.append("panel-swap: mackes-panel not installed — aborting")
        for line in actions:
            log_action(line)
        return actions

    # Phase 10.6.8 — capture rollback state BEFORE we mutate anything.
    # The record stays on disk regardless of how far the step gets, so a
    # partial-failure can still be reversed cleanly.
    try:
        from mackes import birthright_rollback as _rb
        prior, restore_actions = _rb.capture_panel_swap_state()
        _rb.record("apply_panel_swap", prior, restore_actions)
        actions.append("panel-swap: recorded rollback state")
    except (OSError, ImportError) as e:
        # Rollback ledger is best-effort — we never block the real step
        # on it. The action log notes the lapse so the user knows
        # `mackes recover` won't reverse this run.
        actions.append(f"panel-swap: rollback record failed: {e}")
    try:
        subprocess.Popen(
            ["mackes-panel"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )
        actions.append("panel-swap: started mackes-panel")
    except OSError as e:
        actions.append(f"panel-swap: mackes-panel start failed: {e}")
        for line in actions:
            log_action(line)
        return actions

    # 10.6.2 — Quit xfce4-panel + override its autostart.
    if shutil.which("xfce4-panel"):
        subprocess.run(
            ["xfce4-panel", "--quit"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=10,
            check=False,
        )
        actions.append("panel-swap: stopped xfce4-panel")

        autostart = home / ".config" / "autostart" / "xfce4-panel.desktop"
        autostart.parent.mkdir(parents=True, exist_ok=True)
        autostart.write_text(
            "[Desktop Entry]\nType=Application\nHidden=true\n"
            "X-XFCE-Autostart-enabled=false\n",
            encoding="utf-8",
        )
        actions.append(f"panel-swap: disabled {autostart}")

    # 10.6.3 — Quit xfdesktop. The RPM already drops the system-side
    # autostart override (Phase 8.3); the user-side belt-and-braces is
    # here.
    if shutil.which("xfdesktop"):
        subprocess.run(
            ["xfdesktop", "--quit"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=10,
            check=False,
        )
        actions.append("panel-swap: stopped xfdesktop")

    # 10.6.4 — Unbind Whisker Super-key. xfce4 binds <Super>l (lower
    # L; the standard installer ships it bound to popup the whisker
    # menu). Swap to running mackes-panel's apple menu.
    backup_path = home / ".config" / "mackes-panel" / "keybindings.backup.toml"
    backup_path.parent.mkdir(parents=True, exist_ok=True)
    bindings_to_swap = {
        "<Super>l": "mackes-panel --apple-menu",
        "<Super>Space": "mackes-panel --apple-menu",
    }
    backup_lines = ["# Auto-saved by mackes.birthright.apply_panel_swap"]
    for combo, _new in bindings_to_swap.items():
        rc, current = _run(
            ["xfconf-query", "--channel", "xfce4-keyboard-shortcuts",
             "--property", f"/commands/custom/{combo}"],
            timeout=5,
        )
        if rc == 0 and current.strip():
            backup_lines.append(f'"{combo}" = "{current.strip()}"')
        _run(
            ["xfconf-query", "--channel", "xfce4-keyboard-shortcuts",
             "--property", f"/commands/custom/{combo}",
             "--type", "string",
             "--set", "mackes-panel --apple-menu", "--create"],
            timeout=5,
        )
    backup_path.write_text("\n".join(backup_lines) + "\n", encoding="utf-8")
    actions.append(
        f"panel-swap: rebound Super-keys to mackes-panel "
        f"(prior bindings backed up to {backup_path})"
    )

    for line in actions:
        log_action(line)
    return actions


# ---------------------------------------------------------------------------
# 16. apply_panel_archive — Phase 10.6.7 of the v1.0.0 work.
# ---------------------------------------------------------------------------

def apply_panel_archive(_preset: Preset) -> List[str]:
    """Archive the user's pre-1.0 xfce4-panel state under
    ~/.config/mackes-panel/legacy-xfce-panel/ before the rename pass.

    Idempotent — second runs detect the existing archive dir and skip.
    """
    actions: List[str] = []
    home = Path(os.path.expanduser("~"))
    src = home / ".config" / "xfce4" / "panel"
    dst = home / ".config" / "mackes-panel" / "legacy-xfce-panel"

    # Phase 10.6.8 — write rollback ledger BEFORE we mutate the archive
    # directory. Idempotent: if the archive already existed, the
    # restore_actions list is empty (rollback should not delete a dir
    # the user had before they ever ran the swap).
    try:
        from mackes import birthright_rollback as _rb
        prior, restore_actions = _rb.capture_panel_archive_state()
        _rb.record("apply_panel_archive", prior, restore_actions)
    except (OSError, ImportError) as e:
        actions.append(f"panel-archive: rollback record failed: {e}")

    if not src.is_dir():
        actions.append("panel-archive: no legacy xfce4 panel state to archive")
        for line in actions:
            log_action(line)
        return actions
    if dst.exists():
        actions.append(f"panel-archive: already archived to {dst}")
        for line in actions:
            log_action(line)
        return actions

    dst.parent.mkdir(parents=True, exist_ok=True)
    try:
        shutil.copytree(src, dst)
        actions.append(f"panel-archive: copied {src} → {dst}")
    except OSError as e:
        actions.append(f"panel-archive: copytree failed: {e}")

    for line in actions:
        log_action(line)
    return actions


# 17. apply_enforce_i3 — Phase 8.8 of the v1.0.7 work.
#
#     1.0.7 fully replaces xfwm4 with i3. This step migrates an
#     upgraded 1.0.6 install that still has xfwm4 running. Idempotent:
#     reruns on systems where i3 is already the active WM are a no-op.
def apply_enforce_i3(_preset: Preset) -> List[str]:
    """Make i3 the active window manager and retire xfwm4-era cruft.

    Steps (all best-effort; failures land in `actions` but don't abort
    the rest):

    1. Detect the running WM via `wmctrl -m`. If already i3, skip the
       process-swap; only stale-state cleanup runs.
    2. If xfwm4 is running, start `i3 --replace` to take over and let
       xfwm4 exit via the WM-replace protocol.
    3. Disable + stop the mackes-maximizer.service user unit (1.0.6
       era — i3 tiles natively, the maximizer is dead weight).
    4. Hide any user-installed mackes-maximizer.desktop autostart
       entry by writing a `Hidden=true` override.
    5. Make sure `~/.config/i3/config` exists; if not, seed it from
       the shipped /usr/share/mackes-shell/i3/config (matches what
       the legacy mackes-wm switch flow did).
    """
    actions: List[str] = []
    home = Path(os.path.expanduser("~"))

    # ---- 1. detect current WM ----
    current_wm = ""
    try:
        out = subprocess.run(
            ["wmctrl", "-m"], capture_output=True, text=True,
            timeout=2, check=False,
        ).stdout
        for line in out.splitlines():
            if line.startswith("Name:"):
                current_wm = line.split(":", 1)[1].strip()
                break
    except (OSError, subprocess.TimeoutExpired) as e:
        actions.append(f"enforce-i3: wmctrl probe failed: {e}")

    actions.append(f"enforce-i3: current WM = {current_wm or 'unknown'}")

    # ---- 2. swap to i3 if needed ----
    if current_wm.lower() == "i3":
        actions.append("enforce-i3: already on i3, skip --replace")
    elif shutil.which("i3") is None:
        actions.append("enforce-i3: i3 binary missing; reinstall mackes-xfce-workstation")
    else:
        try:
            subprocess.Popen(
                ["i3", "--replace"],
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                start_new_session=True,
            )
            actions.append("enforce-i3: started i3 --replace (taking over from xfwm4)")
        except OSError as e:
            actions.append(f"enforce-i3: i3 --replace failed: {e}")

    # ---- 3. disable mackes-maximizer.service (1.0.6 era) ----
    if shutil.which("systemctl"):
        for verb in ("stop", "disable"):
            try:
                subprocess.run(
                    ["systemctl", "--user", verb, "mackes-maximizer.service"],
                    capture_output=True, timeout=5, check=False,
                )
            except (OSError, subprocess.TimeoutExpired):
                pass
        actions.append("enforce-i3: stopped + disabled mackes-maximizer.service (if it existed)")

    # ---- 4. hide stale autostart entry ----
    autostart_user = home / ".config" / "autostart" / "mackes-maximizer.desktop"
    if autostart_user.exists():
        try:
            content = autostart_user.read_text(encoding="utf-8")
            if "Hidden=true" not in content:
                if content.endswith("\n"):
                    content += "Hidden=true\nX-XFCE-Autostart-enabled=false\n"
                else:
                    content += "\nHidden=true\nX-XFCE-Autostart-enabled=false\n"
                autostart_user.write_text(content, encoding="utf-8")
                actions.append(f"enforce-i3: appended Hidden=true to {autostart_user}")
            else:
                actions.append(f"enforce-i3: {autostart_user} already hidden")
        except OSError as e:
            actions.append(f"enforce-i3: autostart override failed: {e}")

    # ---- 5. seed ~/.config/i3/config if missing ----
    user_i3_config = home / ".config" / "i3" / "config"
    default_i3_config = Path("/usr/share/mackes-shell/i3/config")
    if not user_i3_config.exists():
        if default_i3_config.is_file():
            try:
                user_i3_config.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy(default_i3_config, user_i3_config)
                actions.append(f"enforce-i3: seeded {user_i3_config} from {default_i3_config}")
            except OSError as e:
                actions.append(f"enforce-i3: seed failed: {e}")
        else:
            actions.append(f"enforce-i3: default config {default_i3_config} missing — skipping seed")
    else:
        actions.append(f"enforce-i3: {user_i3_config} already exists")

    for line in actions:
        log_action(line)
    return actions


# 18. apply_user_dirs — Phase 1.1.0 of the v1.1.0 work.
#
#     User lock 2026-05-19: the freedesktop user-dirs default
#     (Music/Pictures/Videos/Documents/Templates/Public/Desktop) is
#     replaced with mesh-sync mount targets plus a local Downloads. This
#     reflects the Mackes platform's model — content lives on the mesh,
#     not in per-machine subdirectories.
def apply_user_dirs(_preset: Preset) -> List[str]:
    """Rewrite ~/.config/user-dirs.dirs to point at QNM-Mesh + Downloads.

    The freedesktop xdg-user-dirs spec says apps consult
    `$XDG_CONFIG_HOME/user-dirs.dirs` for the canonical home of each
    well-known media type. Thunar's sidebar, GTK file pickers, and
    every freedesktop app honor these — pointing them at the mesh
    mount makes mesh-sync the default storage tier for everything but
    transient downloads.

    Idempotent: re-running is a no-op when the file already matches
    the target shape. The previous content is backed up to
    `user-dirs.dirs.legacy` on the first rewrite.
    """
    actions: List[str] = []
    home = Path(os.path.expanduser("~"))
    config_dir = home / ".config"
    target = config_dir / "user-dirs.dirs"
    backup = config_dir / "user-dirs.dirs.legacy"

    mesh_root = home / "QNM-Mesh"
    downloads = home / "Downloads"

    # Make sure the local Downloads dir exists. The mesh subdirs are
    # owned by the mesh-sync layer; create the parent only — actual
    # children land when peers come online.
    downloads.mkdir(parents=True, exist_ok=True)
    mesh_root.mkdir(parents=True, exist_ok=True)

    # The target file content — every XDG well-known dir mapped to
    # either a mesh subdir or $HOME (the spec's "I don't want a
    # dedicated folder for this" idiom).
    target_lines = [
        "# Mackes Shell 1.1.0 — XDG user-dirs remapped to mesh-sync.",
        "# Edit Workbench → Look & Feel → User Folders to override.",
        'XDG_DESKTOP_DIR="$HOME"',
        f'XDG_DOWNLOAD_DIR="{downloads}"',
        'XDG_TEMPLATES_DIR="$HOME"',
        'XDG_PUBLICSHARE_DIR="$HOME"',
        f'XDG_DOCUMENTS_DIR="{mesh_root / "Documents"}"',
        f'XDG_MUSIC_DIR="{mesh_root / "Music"}"',
        f'XDG_PICTURES_DIR="{mesh_root / "Pictures"}"',
        f'XDG_VIDEOS_DIR="{mesh_root / "Videos"}"',
        "",
    ]
    target_content = "\n".join(target_lines)

    if target.is_file():
        try:
            current = target.read_text(encoding="utf-8")
        except OSError as e:
            actions.append(f"user-dirs: could not read {target}: {e}")
            current = ""
        if current.strip() == target_content.strip():
            actions.append(f"user-dirs: {target} already at Mackes target")
            for line in actions:
                log_action(line)
            return actions
        # Back up the existing file once (don't clobber an older backup
        # — that would lose the original freedesktop defaults forever).
        if not backup.exists():
            try:
                shutil.copy(target, backup)
                actions.append(f"user-dirs: backed up legacy file to {backup}")
            except OSError as e:
                actions.append(f"user-dirs: backup failed ({e}); aborting rewrite")
                for line in actions:
                    log_action(line)
                return actions

    try:
        config_dir.mkdir(parents=True, exist_ok=True)
        target.write_text(target_content, encoding="utf-8")
        actions.append(f"user-dirs: wrote Mackes remap to {target}")
    except OSError as e:
        actions.append(f"user-dirs: write failed: {e}")

    # `xdg-user-dirs-update` re-creates the underlying directories when
    # any are missing. Run it after the rewrite so Thunar's sidebar
    # picks up the new targets immediately on next mount.
    if shutil.which("xdg-user-dirs-update"):
        try:
            subprocess.run(
                ["xdg-user-dirs-update", "--force"],
                capture_output=True, timeout=10, check=False,
            )
            actions.append("user-dirs: ran xdg-user-dirs-update --force")
        except (OSError, subprocess.TimeoutExpired) as e:
            actions.append(f"user-dirs: xdg-user-dirs-update failed: {e}")

    for line in actions:
        log_action(line)
    return actions


# ---------------------------------------------------------------------------
# 19. apply_uninstall_legacy_xfce — Phase 10.6.6 of the v1.0.0 work.
#
#     1.0.7+ ships mackes-panel (Rust, Phase 0.3) as a complete drop-in
#     replacement for xfce4-panel + xfdesktop. The legacy stack is held
#     inert at runtime by:
#
#       * mackes-enforce-session (1.0.8) — kills xfce4-panel / xfdesktop
#         on every XDG-autostart spawn,
#       * /etc/xdg/autostart/xfdesktop.desktop override (Hidden=true,
#         shipped by mackes-xfce-workstation),
#       * /etc/xdg/autostart/mackes-suppress-xfce4-panel.desktop,
#       * apply_panel_swap's user-side autostart overrides (10.6.1-4).
#
#     This step is the disk-cleanup follow-up — once the user has
#     successfully run the panel swap, the legacy RPMs are dead weight
#     and removing them frees ~14 MB plus a handful of /etc/xdg
#     autostart entries that mackes-enforce-session would otherwise
#     keep neutralizing on every login.
#
#     Hard prerequisite: apply_panel_swap (10.6.1-4) must have already
#     succeeded. We detect that by:
#
#       (a) mackes-panel is running (pgrep -x mackes-panel), AND
#       (b) the user-side xfce4-panel autostart override exists with
#           Hidden=true (apply_panel_swap writes this).
#
#     If either signal is missing, the step is a clean no-op with a
#     "panel-swap prerequisite not met" message — never attempt the
#     removal on a box where mackes-panel hasn't taken over yet (would
#     leave the user with no panel at all).
# ---------------------------------------------------------------------------

# The six packages mackes-xfce-workstation has supplanted. Order matches
# the worklist lock (Phase 10.6.6) so the dnf invocation in the log is
# easy to grep for. xfce4-power-manager-plugin is included even though
# Fedora 44+ folded it into the parent xfce4-power-manager package — dnf
# treats a remove of a missing package as a no-op rather than an error,
# so listing it is harmless on newer Fedoras and correct on older ones.
# 1.1.3 fix — xfce4-panel intentionally omitted: the C panel-plugin
# under data/panel-plugins/mackes-clipboard/ still links
# libxfce4panel-2.0.so.4, which only the xfce4-panel package
# provides. Removing it would break the linked binary. The spec
# Obsoletes for xfce4-panel was dropped for the same reason. The
# panel's process is already suppressed via the autostart override
# at /etc/xdg/autostart/mackes-suppress-xfce4-panel.desktop, so
# leaving the on-disk files in place is harmless. v2.0.0's
# monolithic cut retires the C plugin entirely; xfce4-panel can
# be re-added to this tuple at that point.
_LEGACY_XFCE_PACKAGES: tuple[str, ...] = (
    "xfdesktop",
    "xfce4-whiskermenu-plugin",
    "xfce4-docklike-plugin",
    "xfce4-pulseaudio-plugin",
    "xfce4-power-manager-plugin",
)


def _panel_swap_succeeded() -> tuple[bool, str]:
    """Return (ok, reason). The reason is a human-readable string the
    caller logs when ok=False."""
    # (a) mackes-panel must be running. pgrep -x requires an exact match
    #     on the basename — the panel binary lives at /usr/bin/mackes-panel
    #     and runs under its own name when launched by apply_panel_swap.
    if shutil.which("pgrep") is None:
        return False, "pgrep unavailable — cannot verify mackes-panel is running"
    try:
        rc = subprocess.run(
            ["pgrep", "-x", "mackes-panel"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=5,
        ).returncode
    except (OSError, subprocess.TimeoutExpired) as e:
        return False, f"pgrep probe failed: {e}"
    if rc != 0:
        return False, "mackes-panel is not running (panel-swap not applied)"

    # (b) user-side autostart override must be in place. apply_panel_swap
    #     writes this whenever an xfce4-panel binary is detected at swap
    #     time — its presence is a positive signal that the swap step ran
    #     to completion at least once.
    home = Path(os.path.expanduser("~"))
    autostart = home / ".config" / "autostart" / "xfce4-panel.desktop"
    if not autostart.is_file():
        # Edge case: a box that never had xfce4-panel installed at swap
        # time (rare — only happens on a minimal Fedora WS install
        # without the xfce4 spin) won't get the autostart override.
        # Accept that as a pass — there's nothing to keep neutralized.
        if shutil.which("xfce4-panel") is None:
            return True, "no xfce4-panel ever installed"
        return False, (f"{autostart} missing — apply_panel_swap "
                       "hasn't been run for this user")
    try:
        content = autostart.read_text(encoding="utf-8")
    except OSError as e:
        return False, f"could not read {autostart}: {e}"
    if "Hidden=true" not in content:
        return False, (f"{autostart} exists but does not have Hidden=true — "
                       "apply_panel_swap did not finish")
    return True, "ok"


def _installed_legacy_packages() -> List[str]:
    """Return the subset of _LEGACY_XFCE_PACKAGES that rpm reports as
    installed. Used both for idempotency (skip the dnf call when nothing
    is left to remove) and for the action log so the user sees exactly
    which packages were dropped."""
    installed: List[str] = []
    for pkg in _LEGACY_XFCE_PACKAGES:
        rc, _ = _run(["rpm", "-q", pkg], timeout=10)
        if rc == 0:
            installed.append(pkg)
    return installed


def apply_uninstall_legacy_xfce(_preset: Preset) -> List[str]:
    """Remove the legacy XFCE packages mackes-xfce-workstation supersedes.

    Idempotent: when none of the six packages are installed (already
    removed, or never were), the step is a clean no-op. Gated on
    apply_panel_swap having completed — refuses to run when
    mackes-panel isn't the active panel, since removing xfce4-panel
    out from under a still-running xfce4-panel process leaves the
    user with no panel at all.

    Side effect: the C panel-plugin sub-RPMs (mackes-launcher /
    mackes-clipboard / mackes-drawer) that BuildRequire
    xfce4-panel-devel are obsoleted by mackes-xfce-workstation's
    Obsoletes: lines in the spec, so a `dnf install
    mackes-xfce-workstation` already handles the rename. This step
    closes the gap for boxes that upgrade via package replacement
    rather than fresh install.
    """
    actions: List[str] = []

    # --- 0. dnf must be present. On a non-Fedora box (somehow) this is a
    #         skip, not an error. ---
    if shutil.which("dnf") is None:
        actions.append("uninstall-legacy-xfce: dnf not available — skipping")
        for line in actions:
            log_action(line)
        return actions

    # --- 1. gate on apply_panel_swap completing successfully. ---
    ok, reason = _panel_swap_succeeded()
    if not ok:
        actions.append(
            f"uninstall-legacy-xfce: panel-swap prerequisite not met — {reason}"
        )
        for line in actions:
            log_action(line)
        return actions

    # --- 2. idempotency probe — if nothing is installed, skip the call. ---
    installed = _installed_legacy_packages()
    if not installed:
        actions.append(
            "uninstall-legacy-xfce: no legacy XFCE packages installed — "
            "nothing to remove"
        )
        for line in actions:
            log_action(line)
        return actions

    # Phase 10.6.8 — record the rollback ledger BEFORE the dnf remove
    # fires. Rolling back this step means `dnf install -y <prior set>`,
    # routed through AdminSession by mackes/headless/cli.py:recover.
    try:
        from mackes import birthright_rollback as _rb
        prior, restore_actions = _rb.capture_uninstall_legacy_state(installed)
        _rb.record("apply_uninstall_legacy_xfce", prior, restore_actions)
        actions.append("uninstall-legacy-xfce: recorded rollback state "
                       f"({len(installed)} packages)")
    except (OSError, ImportError) as e:
        actions.append(f"uninstall-legacy-xfce: rollback record failed: {e}")

    # --- 3. fire the single dnf call. We pass the full canonical list
    #         (not just `installed`) so the log shows the locked package
    #         set — dnf treats already-removed packages as a no-op. ---
    rc, out = _run_root(
        ["dnf", "remove", "-y", *_LEGACY_XFCE_PACKAGES],
        timeout=600,
    )
    if rc == 0:
        actions.append(
            "uninstall-legacy-xfce: removed " + ", ".join(installed)
        )
    else:
        last = (out.strip().splitlines()[-1]
                if out.strip() else f"rc={rc}")
        actions.append(
            f"uninstall-legacy-xfce: dnf remove failed: {last}"
        )

    for line in actions:
        log_action(line)
    return actions
