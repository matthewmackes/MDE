# Disable the auto-generated debuginfo / debugsource subpackages. Our C
# panel plugin is tiny (~60 KB stripped) and we don't ship separate
# debug builds. Avoids "Empty %files file debugsourcefiles.list" errors
# from rpmbuild on Fedora 40+.
%global debug_package %{nil}

# INST-1 / operator directive 2026-05-29: the base package is named
# `mde-core` (the headless Fedora-Server substrate you build up from);
# `%{appname}` keeps the on-disk paths at /usr/share/mde, /etc/mde,
# /var/lib/mde regardless of the package Name so runtime code that
# reads those paths is unaffected by the rename.
%global appname mde

Name:           mde-core
Version:        4.0.0
Release:        1%{?dist}
Summary:        Mackes Desktop Environment (MDE) — headless core substrate (Fedora Server build-up base)

License:        GPL-3.0
URL:            https://github.com/matthewmackes/MAP2-RELEASES
# Source tarball still ships under the legacy name so dist/mackes-shell-...
# keeps working; the package itself is renamed via Provides/Obsoletes.
Source0:        mackes-shell-%{version}.tar.gz

# v2.0.0 cut commit — package renamed `mackes-xfce-workstation` →
# `mde`. 2026-05-29 — base renamed `mde` → `mde-core`; `Provides: mde`
# keeps `dnf install mde`, the comps group, and existing scripts
# resolving to the base substrate (no flag day). Back-compat
# Provides/Obsoletes:
Provides:       mde = %{version}-%{release}
Provides:       mackes-shell = %{version}-%{release}
Provides:       mackes-xfce-workstation = %{version}-%{release}
Obsoletes:      mde < 4.0.0-2
Obsoletes:      mackes-shell < 2.0.0
Obsoletes:      mackes-xfce-workstation < 2.0.0

# CB-3.3 Q5 lock — Conflicts: block.  After v2.0.0 cuts, the
# legacy XFCE stack is incompatible with MDE: every component
# below is either replaced (xfce4-panel → mackes-panel,
# xfdesktop → sway+swaybg, xfce4-session → mde-session) or
# in v2.0.0 retires (xfwm4, i3 → sway).  `dnf install mde` on a
# box that already has any of these errors out with a clear
# "would break mde" message rather than silently leaving the
# old desktop pieces running alongside MDE.  `< 999` is the
# rpmlint silence-cap convention we use elsewhere.
Conflicts:      xfce4-panel < 999
Conflicts:      xfdesktop < 999
Conflicts:      xfce4-session < 999
Conflicts:      xfce4-settings < 999
Conflicts:      xfwm4 < 999
Conflicts:      xfce4-whiskermenu-plugin < 999
Conflicts:      xfce4-docklike-plugin < 999
Conflicts:      xfce4-pulseaudio-plugin < 999
Conflicts:      xfce4-power-manager-plugin < 999
Conflicts:      i3 < 999

# v2.0.0 cut: ExclusiveArch retained because we still build the
# Rust mackes-panel + mackesd + mde-workbench workspace binaries.
# The C xfce4-panel plugins are dropped (Conflicts: xfce4-panel
# above) — no libxfce4panel link, no xfce4-panel-devel
# BuildRequires.
ExclusiveArch:  %{ix86} x86_64 aarch64

# Build dependencies. v2.0.0 cut: dropped
# xfce4-panel-devel + libxfce4ui-devel (C plugins retired).
BuildRequires:  python3
BuildRequires:  pkgconfig
# Rust toolchain for crates/mackes-panel + workspace crates
BuildRequires:  rust
BuildRequires:  cargo

# Runtime deps the entry point and all panels need.
# v2.0.0 cut: dropped every XFCE Requires (xfconf, xfce4-settings,
# xfce4-session, xfce4-power-manager, terminator) + i3 stack
# (i3, i3status, dmenu) per CB-3.2. The Wayland stack below
# replaces them.
Requires:       python3
Requires:       python3-pyyaml

# CB-3.2 Wayland stack moved to `mde-desktop` subpackage
# (INST-1, 2026-05-25). A `dnf install mde` lighthouse install
# no longer pulls sway / mako / pipewire / etc. — install
# `mde mde-desktop` together on workstations to get the full
# desktop session.

# python3-gobject + gtk3 also live on `mde-desktop` — the
# surviving mackes/ Python panels are a GTK-only legacy
# surface; the daemon + CLI don't import them.

# v2.1+ KDC2 — native re-implementation supersedes the v13
# Option A wrapper of upstream kdeconnectd. The KDC protocol
# runs in-process inside mackesd via the mde-kdc + mde-kdc-proto
# crates; upstream kdeconnectd is no longer needed and must
# not co-install (it would race on UDP/1716 + D-Bus name
# acquisition).
#
# KDC2-6.2 Obsoletes: forces dnf to drop kdeconnectd and the
# Qt-laden kdeconnect-cli / kdeconnect-indicator on upgrade.
# KDC2-6.3 Conflicts: prevents a re-install from a manual
# `dnf install kdeconnect-cli` after MDE is up — both would
# bind the same D-Bus service name.
Obsoletes:      kdeconnect < 999
Obsoletes:      kdeconnectd < 999
Obsoletes:      kdeconnect-cli < 999
Obsoletes:      kdeconnect-indicator < 999
Conflicts:      kdeconnect
Conflicts:      kdeconnect-cli
Conflicts:      gsconnect

# SSH enabled by default on every Mackes install
Requires:       openssh-server

# v2.0.0 cut: wmctrl (X11-only) dropped. Wayland window
# enumeration goes through swayipc-async (Phase E rewrite)
# or `swaymsg -t get_tree` subprocess (today).

# Plymouth boot splash — MackesDE for Workgroups ships its own theme
# at data/plymouth/mde/ (Material card + stacked-logo + indeterminate
# blue progress bar, per `Mackes DE Bootsplash.html` design lock
# 2026-05-25). The "script" plugin (plymouth-plugin-script) is the
# canonical Plymouth backend on Fedora 44+ and is what `mde.script`
# targets — pulled explicitly so the theme renders rather than
# silently falling back to BGRT.
Requires:       plymouth
Requires:       plymouth-scripts
Requires:       plymouth-plugin-script

# Flatpak — birthright wizard adds the Flathub remote per-user
Recommends:     flatpak

# Remote-desktop birthright stack (xrdp / wayvnc / Guacamole)
# moved to `mde-desktop` subpackage (INST-1, 2026-05-25). The
# birthright step that installs the Guacamole web app via
# `curl` only runs on full-profile installs that already pulled
# `mde-desktop`, so the deps move cleanly with no orphan code.

# Fleet management birthright (v1.3.0) — ansible-pull on every node,
# playbook tree replicated through QNM-Shared. The 7 curated roles ship
# under /usr/share/mackes-shell/data/ansible/playbooks/.
Requires:       ansible-core
Requires:       python3-ansible-runner
Requires:       podman
# Container-runtime-setup playbook uses these too, but they're optional
# (the playbook tolerates failures via the dnf module's state=present).
Recommends:     buildah
Recommends:     skopeo
Recommends:     toolbox

# Textual TUI — headless entry point (v1.4.0). Falls back to argparse
# CLI when textual is missing, so this is a Recommends not a Requires.
Recommends:     python3-textual
Recommends:     python3-rich

# Wizard boot splash (v1.4.0) — plays branding/MACKES-XFCE-LOGO.mp4
# before the first-run wizard. Splash is opt-in: if GStreamer or its
# codecs aren't installed, mackes/wizard/splash.py silently skips it.
# We only Recommend the runtime stack to keep the install footprint
# reasonable for headless nodes.
Recommends:     gstreamer1
Recommends:     gstreamer1-plugins-base
Recommends:     gstreamer1-plugins-good
Recommends:     gstreamer1-plugins-bad-free
# openh264 (Cisco) decodes H.264 — shipped by Fedora via the codeina
# mozilla-openh264 repo, not the main collection. Recommends without
# the repo will silently skip; documented in CHANGELOG so the user
# knows how to enable.
Recommends:     mozilla-openh264
Recommends:     gstreamer1-plugin-openh264

# Conky HUD (v1.4.0 birthright) — right-side Carbon-themed desktop panel
# with live Mackes state (mesh, fleet, drift, storage, services). The
# wizard's apply_conky step installs the user config + XDG autostart.
# The Tweaks panel toggle turns the HUD on/off without uninstalling.
Requires:       conky
# v2.0.0 cut: xorg-x11-server-utils, xdotool, wmctrl, xprop
# all dropped (X11-only tools). Wayland equivalents:
# - swaymsg / swayipc-async for window enumeration (replaces
#   wmctrl + xprop).
# - kanshi for per-output config (replaces xrandr / xorg-utils).

# Mesh fabric (v2.5 NF-6.1): Nebula overlay. The legacy
# Tailscale + Headscale deps retire — Nebula's lighthouse
# replaces Headscale's control plane, and Nebula's native UDP
# overlay replaces the Tailscale dataplane. The covert TCP/443
# tunnel is owned by the mackes-nebula-https-tunnel binary the
# workspace builds.
Requires:       nebula >= 1.9.0

# Mesh storage (GF-1.1, v5.0.0): GlusterFS over the Nebula
# overlay. `glusterfs-server` ships the `glusterd` daemon that
# owns the cluster + volume topology; `glusterfs-fuse` is the
# FUSE client every peer mounts to serve `~/Documents`,
# `~/Pictures`, etc. from the replicated `mesh-home` volume
# (see `docs/PROJECT_WORKLIST.md` GF-2.x / GF-4.x for the
# bootstrap + mount wiring). The %post block below enables
# glusterd at install time so subsequent gluster-worker ticks
# can hand the daemon CLI work without a manual operator step.
Requires:       glusterfs-server
Requires:       glusterfs-fuse

# Monitoring (MON-1, v2.6): Netdata for per-peer metrics +
# alerting. The 25-Q monitoring design lock (2026-05-24) reuses
# `mackesd::leader` for the parent/child streaming election;
# the aggregator publishes its overlay IP via QNM-Shared and
# children stream to it. Fail-soft: when the aggregator is
# unreachable each peer self-parents (7-day local dbengine
# retention). The birthright `apply_netdata_monitor` step
# writes /etc/netdata/netdata.conf with the locked baseline
# params (dbengine memory mode + 7d retention + cloud off +
# bind to 127.0.0.1 only); the dynamic stream-block update on
# leader-flip lands with MON-1.b.
Requires:       netdata

# EPIC-MESH-PROBE (MESH-PROBE-3) — nmap is the sole probe engine
# (design Q3). mackesd's probe worker + `mackesd probe scan` shell
# out to it; the bundled NSE scripts at %{_datadir}/%{appname}/nmap/
# extend `-sV` with the mesh-media + Mackes-service fingerprinters.
Requires:       nmap

# VV-1 + VV-1.5 (v4.1.0) — voice stack. Kamailio is the SIP
# proxy/registrar/router; rtpengine is the SRTP relay it drives
# via the NG unix socket. Both from F44's official repo (no
# third-party COPRs). The 2026-05-24 Asterisk→Kamailio swap
# replaced the previous `Requires: asterisk >= 21.0` plan.
Requires:       kamailio >= 5.8
Requires:       rtpengine

# DM-1 (v2.7, 2026-05-25) — greetd + regreet + cage replace
# LightDM as the display manager per the 10-Q operator survey
# locked 2026-05-24. greetd is the daemon (kiosk-locked,
# socket-IPC to greeters); regreet is the Rust+GTK4 greeter
# UI; cage is the one-window wlroots compositor that hosts
# regreet so pre-auth has no escape keybindings. LightDM
# stays Required for now (rollback path); the
# `apply_display_manager()` birthright step (DM-5) is what
# flips the systemd default on first wizard run. All three
# packages ship in F44's official repo — no Copr / RPM-Fusion
# dep. After greetd is verified on the bench (HW-*),
# `Requires: lightdm` can drop.
Requires:       greetd
Requires:       regreet
Requires:       cage

# Mesh filesystem (SSHFS-over-QNM, §8.10)
# Fedora packages sshfs as `fuse-sshfs` (deliberately namespaced to
# disambiguate fuse2/fuse3 binaries).
Requires:       fuse-sshfs
Requires:       fuse
Requires:       fuse3

# GVFS-mesh FUSE backend (mesh:// URI scheme handler)
Requires:       python3-fusepy
Requires:       gvfs
Requires:       gvfs-fuse

# mDNS bridge (§8.13 Layer 5)
Recommends:     avahi
Recommends:     avahi-tools
# v1.6.2 — push-based service discovery (mackes.mesh_mdns listener)
Recommends:     python3-zeroconf
# v1.6.2 — mesh perf round (every dep is optional; the modules
# degrade gracefully when absent and report so in the Mesh
# Performance panel)
Recommends:     python3-fusepy
Recommends:     python3-paramiko
Recommends:     python3-diskcache
Recommends:     python3-nats-py
Recommends:     wireguard-tools
# Wake-on-LAN tools are optional — mackes.mesh_wol uses pure Python
# for the magic packet but reads ARP via `ip neigh` (in iproute2,
# already a hard dep) so no extra Recommends needed.

# Caddy gateway retired 2026-05-25 (Q10 of the 100-Q tightening
# survey + EPIC-RETIRE-CADDY): mesh-services catalog gone with
# DEAD-2.9; v6.x Mackes Bus owns webhook ingress on its own
# Nebula-only port. No remaining use case justifies the dep.

# Native media clients with mesh autoconfig (§8.13 Layer 4)
# jellyfin-media-player isn't in Fedora's mainline repos — users
# install it from Flathub or RPM Fusion. Not Recommended here to keep
# the dep resolver clean on stock Fedora. Mackes' Media-Hub discovery
# still surfaces Jellyfin instances on the mesh regardless of local
# client install.
Recommends:     strawberry

# Network / firewall / audio
Requires:       NetworkManager
Recommends:     firewalld
Recommends:     pulseaudio-utils
# CB-3.2 Wayland-alternate Recommends.
Recommends:     cosmic-files
Recommends:     yazi
Recommends:     kanshi
Recommends:     wlogout
Recommends:     wofi

# Typography defaults — PatternFly v6 (Red Hat Display + Text + Mono)
Recommends:     redhat-display-fonts
Recommends:     redhat-text-fonts
Recommends:     redhat-mono-fonts
# v4.0.1 Geologica audit follow-up: visual-identity.md Q12 locks
# IBM Plex Mono for terminal + tabular surfaces. Fedora ships it
# as `ibm-plex-mono-fonts`; Recommends keeps the install graceful
# on systems where the operator deliberately strips fonts. The
# Geologica side of the lock (Q11) needs an upstream tarball
# bundle that's deferred to v4.0.2 because Google Fonts'
# /download endpoint doesn't return a programmatically-fetchable
# ZIP today.
Recommends:     ibm-plex-mono-fonts

# CR-1 (2026-05-25): Roboto + Roboto Mono — desktop-only.
# Moved to `mde-desktop` subpackage by INST-1 the same day.
# A lighthouse-only `dnf install mde` doesn't render any UI;
# Roboto only matters on workstations + laptops that also
# pulled `mde-desktop`.

# QNM is detected at runtime; soft dep so users without QNM still get
# Network → QNM panel with an install prompt (C2/Q38 lock).
Recommends:     qnm

# Force /usr as the base so the package lands in /usr/lib/python3.X/site-packages
# (the Fedora convention for distro-installed Python packages), not /usr/local.
%global py3_ver %(python3 -c "import sys; print('%d.%d' % sys.version_info[:2])")
%global py3_sitelib /usr/lib/python%{py3_ver}/site-packages

%description
Mackes Desktop Environment (MDE) is a Wayland-only Fedora desktop
environment built on sway as the compositor. v2.0.0 retires the XFCE
session host that v1.x ran on top of — sway / swaylock / swayidle /
swaybg replace xfce4-session + xfwm4 + xfdesktop; the Iced MDE
Workbench replaces the GTK3 xfce4-settings; mde-session orchestrates
first-boot config migration off the v1.x xfconf tree. One Workbench
window, a dashboard, nine task groups (Look & Feel, Devices, Network,
System, Apps, Maintain, Fleet, Help, Dashboard), and a first-run
wizard that brings a fresh machine to a known preset in under five
minutes. PatternFly v6 styling (Red Hat Display / Text / Mono).

# ---------------------------------------------------------------------------
# mde-xorg sub-package — XOrg/i3 edition of the MDE session
# ---------------------------------------------------------------------------
%package -n mde-xorg
Summary:        Mackes Desktop Environment — XOrg/i3 session
Requires:       mde-core = %{version}-%{release}
Requires:       mde-desktop = %{version}-%{release}
Requires:       i3
Requires:       i3lock
Requires:       xorg-x11-server-utils
Requires:       xfce4-terminal
Requires:       scrot
Requires:       dmenu
Recommends:     brightnessctl

%description -n mde-xorg
XOrg/i3 session add-on for Mackes Desktop Environment.  Installs
the i3-backed mde-session binary (built --features x11), the
mde-xorg.desktop XSession entry, an i3 config tuned for MDE, and
a systemd user target that gates on DISPLAY instead of
WAYLAND_DISPLAY.  Install alongside the `mde-desktop` package to
get both session choices in the display-manager greeter.

# INST-1 (v2.7, 2026-05-25; base renamed mde→mde-core 2026-05-29) —
# addon RPM that adds every GUI binary + Wayland-stack runtime dep
# to a base `mde-core` install. Lighthouse VPS boxes and headless
# mesh peers `dnf install mde-core` without dragging in sway /
# wlroots / GTK / iced / KDC2 GUI plugins / Roboto fonts. A full
# desktop install runs `dnf install mde-core mde-desktop` (or pulls
# the `mackes-desktop-environment` comps group, which lists both).
%package -n mde-desktop
Summary:        Mackes Desktop Environment — Wayland desktop add-on
Requires:       mde-core = %{version}-%{release}

# Wayland compositor stack — the desktop add-on owns the sway
# session host + every Wayland-native runtime dep.
Requires:       sway
Requires:       swaylock
Requires:       swayidle
Requires:       swaybg
Requires:       foot
Requires:       bemenu
Requires:       brightnessctl
Requires:       pipewire
Requires:       wireplumber
Requires:       grim
Requires:       slurp

# Wayland-native notification daemon. Conflicts: dunst stays
# here so the addon's installation guarantees no X11 daemon
# races for org.freedesktop.Notifications.
Requires:       mako
Conflicts:      dunst

# GTK 3 for the surviving mackes/ Python panels (v1.x legacy
# surface still shipped by the addon; v2.x retires per panel).
Requires:       python3-gobject
Requires:       gtk3

# Remote-desktop birthright stack (wayvnc + xrdp + Guacamole).
# Pure GUI feature — moves out of base.
Requires:       xrdp
Requires:       xrdp-selinux
Requires:       wayvnc
Requires:       guacd
Requires:       tomcat
Requires:       curl

# Classic ChromeOS typography (CR-1) — only matters once a UI
# is on the wire. Lighthouse / headless installs skip Roboto.
Requires:       google-roboto-fonts
Requires:       google-roboto-mono-fonts
# Portal-3 — Intel One Mono for mde-portal Dock/Portal surfaces;
# Symbols Nerd Font Mono for Carbon icon-glyph fallback rendering.
Requires:       intel-one-mono-fonts
Requires:       symbols-nerd-font-mono-fonts

%description -n mde-desktop
Wayland desktop add-on for Mackes Desktop Environment.  Pulls
in the sway compositor, every mde-* / mde-applet-* GUI binary,
the GTK 3 stack for the surviving v1.x panels, the Wayland-
native notification daemon (mako), the remote-desktop birthright
stack (wayvnc + xrdp + Guacamole), and the Classic ChromeOS
Roboto / Roboto Mono typography.  Install alongside `mde`
(`dnf install mde mde-desktop`) on workstations and laptops;
omit on lighthouse VPS boxes and headless mesh peers that only
need the daemon + CLI.

%prep
# sdist generated by `python -m build` unpacks to mackes_shell-<version>/
# (PyPI canonical underscore name), not mackes-shell-<version>/.
%autosetup -n mackes_shell-%{version}

%build
# v2.0.0 cut: C xfce4-panel external plugins retired
# (Conflicts: xfce4-panel above). The data/panel-plugins/ tree
# stays in the source tree as historical reference for the
# v1.x release line but is not built or shipped in v2.0.0.

# Rust workspace — mackes-panel binary + library crates (Phase 0.3).
# Offline=false because we let Cargo resolve crates.io deps; the build
# environment is expected to have network. If/when we vendor, drop in
# `--offline` and a vendored target dir.
cargo build --release --workspace

# XOrg fork — build mde-session with --features x11 into a separate
# target dir so both binaries can be installed from the same spec.
cargo build --release --features x11 -p mde-session \
    --target-dir target/x11

%check
# KDC2-6.4 — Qt-free dep-closure gate.
#
# The v2.1+ KDC2 native re-implementation deliberately drops
# every Qt / KF6 dependency. If any sneaks in through a transitive
# crate or a stray Python import, this stanza fails the build
# loudly rather than letting the RPM ship with hidden Qt deps.
#
# Strategy: scan the built Rust binaries' link map + the Python
# package source tree for any `Qt`/`KF6` markers.  The previous
# Phase 13 wrapper required `kdeconnectd` (a Qt app) as a
# Requires; with KDC2-6.1's drop, nothing in the spec dep
# block references Qt either — this %check is the belt-and-
# suspenders backstop.
set -e
ldd target/release/mackesd 2>/dev/null | \
    grep -iE 'libQt[0-9]|libKF[0-9]' && \
    { echo "FAIL: Qt/KF library linked into mackesd"; exit 1; } || true
ldd target/release/mde-session 2>/dev/null | \
    grep -iE 'libQt[0-9]|libKF[0-9]' && \
    { echo "FAIL: Qt/KF library linked into mde-session"; exit 1; } || true
# Python tree must not import PyQt or PySide either.
if grep -rnE '^[[:space:]]*(import|from)[[:space:]]+(PyQt[0-9]+|PySide[0-9]+|PyKF[0-9]+)' \
        mackes/ 2>/dev/null; then
    echo "FAIL: Python module imports Qt/KF bindings"
    exit 1
fi
echo "%check: Qt-free dep closure confirmed (KDC2-6.4)"

%install
# 1. Install the Python package directly into site-packages. Skip
#    setuptools/pip entirely — Mackes has no C extensions and no
#    generated metadata that matters at runtime.
install -d %{buildroot}%{py3_sitelib}
cp -r mackes %{buildroot}%{py3_sitelib}/

# 2. dist-info so dnf/rpm-py-inspector and `pip list` can see it
install -d %{buildroot}%{py3_sitelib}/mackes_shell-%{version}.dist-info
cat > %{buildroot}%{py3_sitelib}/mackes_shell-%{version}.dist-info/METADATA <<EOF
Metadata-Version: 2.1
Name: mackes-shell
Version: %{version}
Summary: %{summary}
License: %{license}
EOF
cat > %{buildroot}%{py3_sitelib}/mackes_shell-%{version}.dist-info/INSTALLER <<EOF
rpm
EOF

# 3. Shipped read-only data
install -d %{buildroot}%{_datadir}/%{appname}/data
cp -r data/presets        %{buildroot}%{_datadir}/%{appname}/data/
cp -r data/wallpapers     %{buildroot}%{_datadir}/%{appname}/data/
cp -r data/css            %{buildroot}%{_datadir}/%{appname}/data/
cp -r data/dnf            %{buildroot}%{_datadir}/%{appname}/data/
# Fleet management (v1.3.0) — 7 curated Ansible roles
cp -r data/ansible        %{buildroot}%{_datadir}/%{appname}/data/
# v2.2.0 — data/conky/ removed (Conky HUD replaced by Notification Drawer).
# v1.6.2 — canonical xfce4-panel snapshot for apply_panel_layout.
# data/panel/xfce4-panel.snapshot.json is the platform default panel
# (regenerated via tools/snapshot-panel.py on the reference box).
if [ -d data/panel ]; then
    cp -r data/panel      %{buildroot}%{_datadir}/%{appname}/data/
fi
# Mackes Conky autostart .desktop
# v2.2.0 — mackes-conky.desktop removed (Conky HUD replaced by the
# Notification Drawer panel applet).
cp -r data/systemd        %{buildroot}%{_datadir}/%{appname}/data/
# Vendored GTK themes + icon theme — system-wide install
install -d %{buildroot}%{_datadir}/themes
cp -r data/themes/Orchis-Dark   %{buildroot}%{_datadir}/themes/
cp -r data/themes/Shiki-Statler %{buildroot}%{_datadir}/themes/
# v4.0.2 — Mackes-Dark greeter theme (Carbon palette + indigo
# accent). configure-lightdm.sh sets theme-name=Mackes-Dark so a
# fresh `dnf install mde && reboot` lands the in-house brand
# stripe instead of the third-party Orchis fallback.
cp -r data/themes/Mackes-Dark   %{buildroot}%{_datadir}/themes/
# Portal-37 — MDE-Dark GTK4 + GTK3 theme. Matches Portal visual
# identity (Carbon-inspired pastel-on-charcoal + indigo accent +
# Intel One Mono). mde-session's theme_pump writes
# ~/.config/gtk-{3,4}.0/settings.ini → gtk-theme-name=MDE-Dark on
# every login so freshly-launched GTK apps inherit it.
cp -r data/themes/MDE-Dark      %{buildroot}%{_datadir}/themes/
# Portal-37 — Qt6 color scheme. mde-session's theme_pump points
# qt6ct.conf at this file via color_scheme_path so Qt6 apps blend
# with the GTK4 surfaces.
install -d %{buildroot}%{_datadir}/qt6ct/colors
install -m 0644 data/themes/MDE-Dark/qt6ct/colors/MDE-Dark.conf \
    %{buildroot}%{_datadir}/qt6ct/colors/MDE-Dark.conf
install -d %{buildroot}%{_datadir}/icons
cp -r data/icons/Black-Sun     %{buildroot}%{_datadir}/icons/
cp -r data/icons/Mackes-Carbon %{buildroot}%{_datadir}/icons/
# Plymouth MackesDE boot theme — installed but NOT activated at %post; the
# wizard's birthright step (mackes.birthright.apply_plymouth) activates it
# only when the user opts in (initrd rebuild is heavy and disruptive).
# Theme directory renamed mackes/ → mde/ per the 2026-05-25 100-Q rebrand
# (Q71 + Q73: code-internal name is "MDE"); old `mackes` theme dir retired.
install -d %{buildroot}%{_datadir}/plymouth/themes
cp -r data/plymouth/mde %{buildroot}%{_datadir}/plymouth/themes/
# v2.0.0 cut: C panel-plugin install steps retired (see %build).
cp -r data/grub           %{buildroot}%{_datadir}/%{appname}/data/
cp    data/media-services.yaml %{buildroot}%{_datadir}/%{appname}/data/
# Per-preset captured xfconf baselines (§8 lock). Each preset directory
# contains <channel>.txt dumps from `xfconf-query --channel X --list -v`
# plus a manifest.json. Apply via install-helpers/apply-xfce-baseline.sh.
if [ -d data/xfce-baseline ]; then
    cp -r data/xfce-baseline %{buildroot}%{_datadir}/%{appname}/data/
fi

# A1 fix (2026-05-29) — back-compat path symlink. The package
# renamed mackes-shell -> mde at v2.0, and all shipped data moved to
# %{_datadir}/%{appname}/ == /usr/share/mde/. But the path migration
# was only half-done: ~40 consumers (Python in mackes/, several Rust
# crates, install-helpers/*.sh, systemd unit Documentation= lines,
# packaging/iso/mde.ks) still read /usr/share/mackes-shell/. On an
# installed system those paths did not exist, so e.g. birthright
# could not find its shipped data/branding (it silently fell back to
# its source-tree path, masking the bug in dev). This relative
# symlink makes every legacy /usr/share/mackes-shell/ reference
# resolve to the real /usr/share/mde/ tree. Tracked for full consumer
# migration (then drop this) under INST-DATADIR-SWEEP.
ln -sfn %{appname} %{buildroot}%{_datadir}/mackes-shell

# 3a. Mackes Shell branding — hero logo (About / Wizard / Dashboard) +
#     standard wallpaper (desktop + LightDM greeter)
install -d %{buildroot}%{_datadir}/%{appname}/branding
cp -r branding/* %{buildroot}%{_datadir}/%{appname}/branding/

# 3b. In-Mackes help documentation (docs/help/*.md)
install -d %{buildroot}%{_datadir}/%{appname}/help
cp docs/help/*.md %{buildroot}%{_datadir}/%{appname}/help/

# 3c. Apple-menu "About Mackes" credits + license file (1.0.7+).
#     Consumed by mackes/about.py via /usr/share/mackes-shell/ABOUT.txt.
install -D -m 0644 data/ABOUT.txt %{buildroot}%{_datadir}/%{appname}/ABOUT.txt

# 3d. Build-identity files (v2.0.3) — written here so both panel
#     watermarks (legacy mackes-panel + Iced mde-panel) report the
#     same build hash + date without compile-time env var
#     coordination. Both panels read /usr/share/mde/build-{hash,date}.
#     Falls back to "unknown" if SOURCE_DATE_EPOCH isn't set
#     (manual builds) and the git short isn't probeable.
mkdir -p %{buildroot}%{_datadir}/mde
{
  if [ -n "${SOURCE_DATE_EPOCH:-}" ]; then
    date -u -d "@${SOURCE_DATE_EPOCH}" +%%Y-%%m-%%d
  else
    date -u +%%Y-%%m-%%d
  fi
} > %{buildroot}%{_datadir}/mde/build-date
chmod 0644 %{buildroot}%{_datadir}/mde/build-date
{
  if [ -f .git_short ]; then
    cat .git_short
  elif command -v git >/dev/null 2>&1 && git rev-parse --short HEAD >/dev/null 2>&1; then
    git rev-parse --short HEAD
  else
    echo "%{version}"
  fi
} > %{buildroot}%{_datadir}/mde/build-hash
chmod 0644 %{buildroot}%{_datadir}/mde/build-hash

# 4. Install helper scripts (called from %%post / %%preun, plus the
#    capture/apply pair admins use to manage XFCE baselines, plus the
#    theme/icon bootstrap, lightdm config, and mackes-user creation)
install -d %{buildroot}%{_datadir}/%{appname}/install-helpers
for helper in \
    hide-xfce-settings.sh restore-xfce-settings.sh \
    add-mackes-repo.sh install-recovery.sh \
    capture-xfce-baseline.sh apply-xfce-baseline.sh \
    configure-lightdm.sh create-mackes-user.sh \
    mesh-ca-trust.sh register-gvfs-mesh.sh \
    sync-user-sway-exec-lines.sh ; do
    install -m 0755 install-helpers/$helper \
        %{buildroot}%{_datadir}/%{appname}/install-helpers/
done

# 4a. data/mesh-ssh-policy.example.yaml
install -D -m 0644 data/mesh-ssh-policy.example.yaml \
    %{buildroot}%{_datadir}/%{appname}/data/mesh-ssh-policy.example.yaml

# 4b. systemd units (system + user)
install -d %{buildroot}%{_unitdir}
install -m 0644 data/systemd/mackes-node.service             %{buildroot}%{_unitdir}/
install -m 0644 data/systemd/mackes-tailscale-bootstrap.service %{buildroot}%{_unitdir}/
# NF-3.1/3.2/3.3 (v2.5) — Nebula systemd units.
install -m 0644 data/systemd/nebula.service                  %{buildroot}%{_unitdir}/
install -m 0644 data/systemd/nebula-lighthouse.service       %{buildroot}%{_unitdir}/
install -m 0644 data/systemd/mackes-nebula-https-tunnel.service %{buildroot}%{_unitdir}/
# NF-6.2 — sealed CA dir + Nebula config dir + per-peer cert dir.
install -d -m 0700 %{buildroot}/var/lib/mackesd/nebula-ca
install -d -m 0755 %{buildroot}/etc/nebula
install -d -m 0700 %{buildroot}/var/lib/mackesd/nebula-peers
# GF-1.3.a (v5.0.0) — overlay-ip publish dir. nebula_supervisor
# atomic-writes /var/lib/mackesd/nebula/overlay-ip on every
# refresh_config tick. 0755 because non-root downstream services
# (notably the glusterd-nebula-bind helper in GF-1.3.b) need to
# read it; the file itself contains nothing private (just the
# overlay IPv4).
install -d -m 0755 %{buildroot}/var/lib/mackesd/nebula
# VV-1 (v4.1.0) — Kamailio daemon unit + config dir.
install -m 0644 data/systemd/kamailio-mde.service            %{buildroot}%{_unitdir}/
install -d -m 0755 %{buildroot}/etc/kamailio-mde
# VV-1.5 (v4.1.0) — RTPengine daemon unit + config dir.
install -m 0644 data/systemd/rtpengine-mde.service           %{buildroot}%{_unitdir}/
install -d -m 0755 %{buildroot}/etc/rtpengine-mde
# v2.0.0 Phase B.13 — 10 standalone .service/.timer units retired
# (mackes-clipboard-daemon, mackes-gvfsd-mesh, mackes-mdns-relay,
# mackes-remmina-sync.{service,timer}, mackes-media-sync.{service,
# timer}, mackes-ansible-pull.{service,timer}, mackesd-kdc-bridge).
# Each role now runs as an in-process worker inside `mackesd serve`
# (mackesd.service ExecStart points there).
install -d %{buildroot}%{_userunitdir}
# v2.0.0 Phase D.6 — mde-session user unit (Wayland orchestrator).
install -m 0644 data/systemd/mde-session.service             %{buildroot}%{_userunitdir}/
# GF-4.1 (v5.0.0) — per-user mesh-home FUSE mount template.
# Operators enable via `systemctl --user enable
# mde-mesh-mount@<xdg-dir>.service` once GF-2.x has
# bootstrapped the volume; the GF-3.3 birthright step will
# automate the enable when it ships.
install -m 0644 data/systemd/mde-mesh-mount@.service        %{buildroot}%{_userunitdir}/
# MON-2 (v2.6) — Netdata health.d alert configs. Five files:
#   nebula.conf        — overlay process / peer / relay / handshake / latency
#   gluster.conf       — brick / heal-queue / split-brain / quota / quorum
#   mackesd.conf       — Healthz ping / leader flap / no-leader
#   workstation.conf   — boot disk / swap thrash / thermal throttle / dnf pending
#   mde-suppressions.conf — disable stock alerts that don't fit MDE's failure model
# Netdata auto-loads everything in /etc/netdata/health.d/*.conf on daemon
# start + on `netdatacli reload-health`. The MON-1 birthright step enables
# the daemon; the MON-1.b stream-block rewriter will trigger reload-health
# on every CA-leader transition once it ships.
install -d -m 0755 %{buildroot}%{_sysconfdir}/netdata/health.d
for healthconf in nebula gluster mackesd workstation mde-suppressions; do
    install -m 0644 data/netdata/health.d/${healthconf}.conf \
        %{buildroot}%{_sysconfdir}/netdata/health.d/${healthconf}.conf
done
# Sudoers drop-in (v1.4.1) — grants NOPASSWD on Mackes-managed commands
install -D -m 0440 data/sudoers.d/mackes-shell               %{buildroot}/etc/sudoers.d/mackes-shell
install -D -m 0755 bin/mackes-wm                              %{buildroot}%{_bindir}/mackes-wm

# 1.0.7 — default i3 config shipped to /usr/share/mackes-shell/i3/.
# mackes-wm seeds ~/.config/i3/config from this file on first switch.
install -D -m 0644 data/i3/config %{buildroot}%{_datadir}/%{appname}/i3/config
# 1.1.x — Phase 6.4 hotkeys + Phase 3.6 apple-menu shortcut land as a
# config.d drop-in so user overrides at
# ~/.config/i3/config.d/mackes-overrides.conf load alphabetically
# after our defaults. The main config already `include`s the
# directory (added 1.1.0 for the gaps profile picker).
install -D -m 0644 data/i3/config.d/mackes-defaults.conf \
    %{buildroot}%{_datadir}/%{appname}/i3/config.d/mackes-defaults.conf
# v2.0.0 Phase D.5 — sway config shipped under /usr/share/mde/sway/.
# mde-session seeds ~/.config/sway/ from this directory on first
# launch via the mde-shell-migrate-v2 step.
install -D -m 0644 data/sway/config %{buildroot}%{_datadir}/mde/sway/config
install -D -m 0644 data/sway/config.d/mackes-defaults.conf \
    %{buildroot}%{_datadir}/mde/sway/config.d/mackes-defaults.conf

# HYP-8.5 (v6.5) — default tag manifests. Six curated tags
# (voip/dev/hub/web/media/chat) shipped under /usr/share/mde/
# tag-manifests/; the birthright wizard step (HYP-8.5.birthright
# follow-on) copies them to ~/.config/mde/tags/ on first login.
# mded's tag_manifest loader (`crates/mackesd/src/config/
# tag_manifest.rs`) reads from ~/.config/mde/tags/ at startup +
# publishes `event/config/tags/loaded` per tag on the bus.
install -D -m 0644 data/tag-manifests/voip.toml  %{buildroot}%{_datadir}/mde/tag-manifests/voip.toml
install -D -m 0644 data/tag-manifests/dev.toml   %{buildroot}%{_datadir}/mde/tag-manifests/dev.toml
install -D -m 0644 data/tag-manifests/hub.toml   %{buildroot}%{_datadir}/mde/tag-manifests/hub.toml
install -D -m 0644 data/tag-manifests/web.toml   %{buildroot}%{_datadir}/mde/tag-manifests/web.toml
install -D -m 0644 data/tag-manifests/media.toml %{buildroot}%{_datadir}/mde/tag-manifests/media.toml
install -D -m 0644 data/tag-manifests/chat.toml  %{buildroot}%{_datadir}/mde/tag-manifests/chat.toml

# DM-2 (v2.7, 2026-05-25) — greetd substrate config. Wires
# `cage -s -- regreet` as greetd's default_session on vt 1
# with no [initial_session] block (always prompt; no
# auto-login per Q4). Ships to /usr/share/mde/greetd/ rather
# than directly into /etc/greetd/ because Fedora's `greetd`
# RPM already owns /etc/greetd/config.toml — dual-ownership
# would fail at dnf-install time. DM-5's birthright step
# (apply_display_manager) is what installs this over the live
# /etc/greetd/config.toml when the operator commits to the
# DM swap.
install -D -m 0644 data/greetd/config.toml \
    %{buildroot}%{_datadir}/mde/greetd/config.toml

# BUS-1.2 (v6.x Mackes Bus) — ntfy broker config template. The
# mde-bus daemon renders this against the live Nebula overlay IP
# at startup (Tera). Per design doc §line 210 the broker uses
# plain HTTP — Nebula transport encryption + flat-trust mesh
# remove the need for TLS at the broker layer.
install -D -m 0644 data/ntfy/server.yml.tmpl \
    %{buildroot}%{_datadir}/mde/ntfy/server.yml.tmpl

# BUS-1.7 (v6.x Mackes Bus) — subscription manifest template. The
# mde-bus daemon seeds ~/.local/share/mde/bus/subs.yaml from this
# file on first launch when the per-peer file doesn't exist; the
# SubsWatcher then live-reloads on every operator edit.
install -D -m 0644 data/bus/subs.yaml.tmpl \
    %{buildroot}%{_datadir}/mde/bus/subs.yaml.tmpl

# EPIC-BUS-EXT-CORRELATION-5 (Q34) — 5 sample cross-topic
# correlation rules. Reference template (NOT auto-seeded —
# correlation is opt-in); the operator copies it to
# ~/.config/mde/bus-correlate.yaml + trims to the rules their
# fleet actually publishes.
install -D -m 0644 data/bus/correlate.yaml.tmpl \
    %{buildroot}%{_datadir}/mde/bus/correlate.yaml.tmpl

# BUS-3.1 + BUS-3.2 (v6.x Mackes Bus) — webhook ingress.
# hooks.yaml.tmpl is the operator-facing seed that the per-peer
# ~/.config/mde/bus-hooks.yaml starts from; the listener reads the
# per-peer file on every inbound webhook so edits land without a
# daemon restart. hooks/github.yaml ships the full BUS-3.3 rule
# set the operator can include verbatim.
install -D -m 0644 data/bus/hooks.yaml.tmpl \
    %{buildroot}%{_datadir}/mde/bus/hooks.yaml.tmpl
install -D -m 0644 data/bus/hooks/github.yaml \
    %{buildroot}%{_datadir}/mde/bus/hooks/github.yaml
install -D -m 0644 data/bus/hooks/gitea.yaml \
    %{buildroot}%{_datadir}/mde/bus/hooks/gitea.yaml
install -D -m 0644 data/bus/hooks/sonarr.yaml \
    %{buildroot}%{_datadir}/mde/bus/hooks/sonarr.yaml
install -D -m 0644 data/bus/hooks/nut.yaml \
    %{buildroot}%{_datadir}/mde/bus/hooks/nut.yaml
install -D -m 0644 data/bus/hooks/home_assistant.yaml \
    %{buildroot}%{_datadir}/mde/bus/hooks/home_assistant.yaml
install -D -m 0644 data/bus/hooks/generic.yaml \
    %{buildroot}%{_datadir}/mde/bus/hooks/generic.yaml

# EPIC-MESH-PROBE (MESH-PROBE-3) — bundled NSE scripts the deep
# probe profile loads via `nmap --script %{_datadir}/mde/nmap`
# (the probe_nmap deep_argv default nse_dir). The media detector
# names Airsonic/Navidrome/Jellyfin behind generic http banners;
# the service fingerprinter marks Mackes Bus broker peers.
install -D -m 0644 data/nmap/mde-media-detect.nse \
    %{buildroot}%{_datadir}/mde/nmap/mde-media-detect.nse
install -D -m 0644 data/nmap/mde-service-fingerprint.nse \
    %{buildroot}%{_datadir}/mde/nmap/mde-service-fingerprint.nse

# BUS-3.1 (v6.x Mackes Bus) — firewalld service definition for
# the two Bus ports (8443/tcp ntfy + 8444/tcp webhooks). Operator
# applies via `firewall-cmd --add-service=mackes-bus --zone=trusted`
# when explicit firewalld allow-listing is preferred over the
# default bind-scope-only security boundary.
install -D -m 0644 data/firewall/mackes-bus.xml \
    %{buildroot}%{_prefix}/lib/firewalld/services/mackes-bus.xml

# DM-3 + DM-6 (v2.7) — regreet config sourced from the shared-tokens
# theme dir. The .toml installs as %config(noreplace) so operator
# edits survive upgrades. The greeter's CSS lives under
# /usr/share/mde/theme/ alongside tokens.css so a single hex change
# in tokens.css ripples to greeter + panel + Workbench.
install -D -m 0644 data/regreet/regreet.toml \
    %{buildroot}%{_sysconfdir}/regreet/regreet.toml
# DM-6 — shared design-tokens dir. tokens.css + greeter.css live
# here as the single source of truth for cross-process theming.
# Note: tokens.css ALSO ships under /usr/share/mde/data/css/ via
# the broad `cp -r data/css ...` line above for the panel's
# legacy reader path; the duplication is harmless (same bytes,
# `%{_datadir}/%{appname}/` catch-all owns both).
install -D -m 0644 data/css/tokens.css \
    %{buildroot}%{_datadir}/mde/theme/tokens.css
install -D -m 0644 data/css/greeter.css \
    %{buildroot}%{_datadir}/mde/theme/greeter.css

# KDC2-1.10 — Connect routing policy default. Ships as a
# %config(noreplace) so operator edits survive package upgrades.
install -D -m 0644 data/etc/mde/connect/policy.toml \
    %{buildroot}%{_sysconfdir}/mde/connect/policy.toml
# v2.0.0 Phase 0.5 + H.5 — first-boot config migrators.
install -D -m 0755 bin/mde-migrate-from-1x \
    %{buildroot}%{_bindir}/mde-migrate-from-1x
install -D -m 0755 bin/mde-shell-migrate-v2 \
    %{buildroot}%{_bindir}/mde-shell-migrate-v2
# v2.0.3 — per-output default scale picker; sway `exec_always`s this
# at session start to give 4K outputs a readable scale by default.
install -D -m 0755 bin/mde-output-autoscale \
    %{buildroot}%{_bindir}/mde-output-autoscale
# v2.0.0 Phase 0.3 — mde-* binary wrappers alongside the legacy
# mackes-* binaries during the one-release backward-compat window.
install -D -m 0755 bin/mde                  %{buildroot}%{_bindir}/mde
install -D -m 0755 bin/mde-wm               %{buildroot}%{_bindir}/mde-wm
install -D -m 0755 bin/mde-enforce-session  %{buildroot}%{_bindir}/mde-enforce-session
# v2.0.0 Phase 0.3 — man pages for every mde-* entry point.
install -d %{buildroot}%{_mandir}/man1 %{buildroot}%{_mandir}/man8
install -m 0644 data/man/mde.1                  %{buildroot}%{_mandir}/man1/mde.1
install -m 0644 data/man/mde-migrate-from-1x.1  %{buildroot}%{_mandir}/man1/mde-migrate-from-1x.1
install -m 0644 data/man/mde-shell-migrate-v2.1 %{buildroot}%{_mandir}/man1/mde-shell-migrate-v2.1
install -m 0644 data/man/mded.8                 %{buildroot}%{_mandir}/man8/mded.8
# v2.0.0 Phase 0.4 — D-Bus service files (new dev.mackes.MDE.*
# names + legacy org.mackes.* aliases for one-release backward
# compat).
install -d %{buildroot}%{_datadir}/dbus-1/services
install -m 0644 data/dbus-1/services/*.service \
    %{buildroot}%{_datadir}/dbus-1/services/
# XOrg sub-package install steps.
# mde-session-xorg — mde-session built --features x11.
install -D -m 0755 target/x11/release/mde-session \
    %{buildroot}%{_bindir}/mde-session-xorg
# Session entry + startup script.
install -D -m 0755 data/xorg/mde-xorg-session \
    %{buildroot}%{_bindir}/mde-xorg-session
install -D -m 0644 data/xorg/mde-xorg.desktop \
    %{buildroot}%{_datadir}/xsessions/mde-xorg.desktop
# i3 config for the MDE-X session.
install -D -m 0644 data/i3/mde-xorg.config \
    %{buildroot}%{_datadir}/mde-xorg/i3/mde-xorg.config
install -D -m 0644 data/i3/config.d/mde-xorg-defaults.conf \
    %{buildroot}%{_datadir}/mde-xorg/i3/config.d/mde-xorg-defaults.conf
# systemd user target.
install -D -m 0644 data/systemd/user/mde-xorg.target \
    %{buildroot}%{_userunitdir}/mde-xorg.target

# headscale.service is owned by the upstream `headscale` RPM at the same
# path; shipping our copy would cause a file-conflict on dnf install.
# Our data/systemd/headscale.service is kept in the source tree as a
# reference but not installed. To customize (MemoryHigh, etc.), drop a
# systemd drop-in at /etc/systemd/system/headscale.service.d/mackes.conf.
install -d %{buildroot}%{_userunitdir}

# 1.1.0 — systemd user-preset for the Mackes user units. Fedora reads
# files under /usr/lib/systemd/user-preset/ in lexical order to decide
# whether a unit is enabled by default on first encounter for new
# accounts. Without this, the clipboard daemon shipped fine but never
# auto-started (reported as a 1.0.x bug — the mesh clipboard service
# wasn't running after install).
install -D -m 0644 data/systemd/90-mackes.preset \
    %{buildroot}%{_prefix}/lib/systemd/user-preset/90-mackes.preset
# CB-3.6 — v2.0.0 preset (enables mde-session.service for new users).
# Lands alongside the v1.x preset during the back-compat window so
# fresh installs pick up the MDE user-session orchestrator without
# additional configuration.
install -D -m 0644 data/systemd/90-mde.preset \
    %{buildroot}%{_prefix}/lib/systemd/user-preset/90-mde.preset

# CB-2.1 — Wayland-session entry. The canonical static copy lands in
# /usr/share/wayland-sessions/ (FHS home; also read by GDM/SDDM if ever
# present); the mde-desktop %post symlinks it into the regreet-curated
# /var/lib/mde/wayland-sessions/ at install time. Name="MDE", Exec points
# at mde-session which defaults to sway (SWAY-9.a, 2026-05-29 — reverts the
# retired v6.5 HYP-29 mde-hyprland.desktop + MDE_COMPOSITOR=hyprland env).
install -D -m 0644 data/wayland-sessions/mde.desktop \
    %{buildroot}%{_datadir}/wayland-sessions/mde.desktop

# CB-3.4 — comps group definition for
# `dnf groupinstall mackes-desktop-environment`.
install -D -m 0644 data/comps/mackes-desktop-environment.xml \
    %{buildroot}%{_datadir}/mde/comps/mackes-desktop-environment.xml

# CB-2.4 — first-boot orchestration target + the two oneshot
# migrator units it gates on. mde-session.service Wants= the
# target so the migrators run before the first login session
# starts; ConditionPathExists short-circuits subsequent logins.
install -D -m 0644 data/systemd/mde-firstboot.target \
    %{buildroot}%{_userunitdir}/mde-firstboot.target
install -D -m 0644 data/systemd/mde-migrate-from-1x.service \
    %{buildroot}%{_userunitdir}/mde-migrate-from-1x.service
install -D -m 0644 data/systemd/mde-shell-migrate-v2.service \
    %{buildroot}%{_userunitdir}/mde-shell-migrate-v2.service

# 4c. Tumbler thumbnailer + 4d. GVFS mount registration RETIRED in
# DEAD-2.11 (2026-05-26): the `mesh://` URI scheme is obviated by
# gluster mesh-home — XDG dirs (~/Documents etc.) are natively
# FUSE-mounted from the mesh volume per GF-4.1. No URI scheme,
# no Thunar bookmarks, no Tumbler thumbnailer. Operators navigate
# via the regular file manager.

# 5. Top-level launchers + icons
install -D -m 0644 data/applications/mackes-shell.desktop \
    %{buildroot}%{_datadir}/applications/mackes-shell.desktop
install -D -m 0644 data/applications/mackes-clipboard.desktop \
    %{buildroot}%{_datadir}/applications/mackes-clipboard.desktop

# v2.0.0 cut (CB-3.5, H.4): every XDG autostart override is
# dropped. The Wayland session orchestrator (mde-session) and
# sway config own panel/desktop bring-up natively. The
# xfdesktop / mackes-suppress-xfce4-panel / kdeconnect-indicator
# suppressor overrides are no longer needed because:
# - xfce4-panel/xfdesktop are Conflicts (CB-3.3) — they can't
#   be installed alongside MDE.
# - kdeconnect-indicator never starts on Wayland.
# mackes-mesh-uri-handler.desktop RETIRED in DEAD-2.11 (2026-05-26):
# `mesh://` URI scheme obviated by gluster mesh-home. See 4c/4d note above.
# AppStream metainfo — surfaces Mackes in GNOME Software / KDE Discover
# and is the modern standard for desktop app metadata. Reverse-DNS app
# ID matches mackes-shell.desktop's launchable type.
install -D -m 0644 data/applications/mackes-shell.metainfo.xml \
    %{buildroot}%{_metainfodir}/io.github.matthewmackes.MackesShell.metainfo.xml
# Panel binary gets its own metainfo so GNOME Software / KDE Discover
# surface the Rust panel + dock + wallpaper as a distinct component
# from the workbench app (1.0.7+).
install -D -m 0644 data/metainfo/shell.mackes.Panel.metainfo.xml \
    %{buildroot}%{_metainfodir}/shell.mackes.Panel.metainfo.xml
# v2.0.0 Phase 0.9 — MDE-namespaced metainfo + .desktop ship
# alongside the legacy entries for the one-release backward-compat
# window.
install -D -m 0644 data/metainfo/dev.mackes.MDE.metainfo.xml \
    %{buildroot}%{_metainfodir}/dev.mackes.MDE.metainfo.xml
install -D -m 0644 data/applications/mde.desktop \
    %{buildroot}%{_datadir}/applications/mde.desktop
install -D -m 0644 data/icons/mackes-shell.svg \
    %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/mackes-shell.svg

# 6. /usr/bin/* wrappers
#
# `python3 -P` (Python 3.11+) prevents the cwd from being prepended to
# sys.path. Without -P, launching `mackes` from a directory that
# contains a `mackes/` subdirectory (notably the project's own source
# checkout) would import the in-repo copy instead of the installed
# /usr/lib/python3.14/site-packages/mackes/, silently bypassing every
# fix shipped through this RPM. Measured impact in 1.0.7:
#   - without -P from ~/Desktop/files: 17 s cold start (old code path)
#   - with    -P from any cwd:          1.5 s cold start
install -d %{buildroot}%{_bindir}
cat > %{buildroot}%{_bindir}/mackes <<'EOF'
#!/usr/bin/env bash
exec python3 -P -m mackes "$@"
EOF
chmod 0755 %{buildroot}%{_bindir}/mackes

# 6a. mackes-panel — Rust binary from the workspace (Phase 0.3).
#     Placeholder skeleton in 1.0.0-dev; replaces xfce4-panel at M1.
install -D -m 0755 target/release/mackes-panel \
    %{buildroot}%{_bindir}/mackes-panel
# 6a-bis. mackesd — Mesh control plane binary (Phase 12.1, 1.0.7).
#     Today ships only `migrate` + `status` subcommands (Phase 12.2
#     SQLite store + applied-migration counter). The reconcile loop
#     and serve subcommand land in Phase 12.5+. The systemd unit
#     calls `mackesd migrate` on boot so the store stays current as
#     schema versions land.
install -D -m 0755 target/release/mackesd \
    %{buildroot}%{_bindir}/mackesd
install -D -m 0644 data/systemd/mackesd.service \
    %{buildroot}%{_unitdir}/mackesd.service

# INST-3 (v2.7) — installation manager + updater. Universal substrate
# (work without a Python interpreter on lighthouse VPS boxes), so they
# ship in base `mde-core`, NOT `mde-desktop`. `cargo build --release
# --workspace` above already builds them (mde-installer is a workspace
# member).
install -D -m 0755 target/release/mde-install \
    %{buildroot}%{_bindir}/mde-install
install -D -m 0755 target/release/mde-update \
    %{buildroot}%{_bindir}/mde-update

# CB-1 — Iced MDE Workbench preview (1.1.1). Ships alongside the
# v1.x GTK3 panel so users can try the v2.0.0 Workbench surface
# early; CB-1.12 retires the Python `mackes/workbench/` tree once
# every panel has an Iced port.
install -D -m 0755 target/release/mde-workbench \
    %{buildroot}%{_bindir}/mde-workbench
install -D -m 0644 data/applications/mde-workbench.desktop \
    %{buildroot}%{_datadir}/applications/mde-workbench.desktop

# v2.0.0 Phase E.1 — mde-panel (Iced side-by-side rewrite of the
# GTK mackes-panel; ships in parallel, spec primary swaps at
# end-of-Phase-E parity).
install -D -m 0755 target/release/mde-panel \
    %{buildroot}%{_bindir}/mde-panel

# v3.0.2 panel-host wiring — mde-popover (Iced layer-shell overlay
# host). Spawned by mde-panel's click handlers; mounts the
# start-menu overlay today, audio / notifications / clock / network
# stubs scoped for v3.1 follow-ups.
install -D -m 0755 target/release/mde-popover \
    %{buildroot}%{_bindir}/mde-popover

# v6.0 Portal-1 — mde-portal unified shell (Dock + Portal-compact +
# Portal-full + Lock + Theater + Mesh-wallpaper).  Registers
# dev.mackes.MDE.Portal on the session bus; mackesd calls Lock/Goto/
# Focus/ToggleDND from daemon-side events (idle-lock, mesh alerts).
install -D -m 0755 target/release/mde-portal \
    %{buildroot}%{_bindir}/mde-portal
install -m 0644 data/systemd/user/mde-portal.service \
    %{buildroot}%{_userunitdir}/

# Portal-16 — mde-portal-full scratchpad surface (Iced regular window;
# sway scratchpad rules in data/sway/config place it offscreen until
# the Dock's nav clicks raise it via `scratchpad show`).
install -D -m 0755 target/release/mde-portal-full \
    %{buildroot}%{_bindir}/mde-portal-full

# Portal-35 — mde-open URI dispatcher. Registered as the
# `x-scheme-handler/mde` handler so external apps can route deep links
# through `xdg-open mde://...` → `dev.mackes.MDE.Portal.OpenUri`.
install -D -m 0755 target/release/mde-open \
    %{buildroot}%{_bindir}/mde-open
install -D -m 0644 data/applications/mde-open.desktop \
    %{buildroot}%{_datadir}/applications/mde-open.desktop

# v4.0.1 BUG-4 — mde-files (Iced mesh-first Artifact Manager,
# crates/mde-files/, forked from pop-os/cosmic-files). Replaces
# cosmic-files as the default inode/directory handler; the .desktop
# entry's MimeType= + mde-enforce-session's `xdg-mime default` make
# the swap sticky at every login.
install -D -m 0755 target/release/mde-files \
    %{buildroot}%{_bindir}/mde-files
install -D -m 0644 data/applications/mde-files.desktop \
    %{buildroot}%{_datadir}/applications/mde-files.desktop

# v2.0.0 Phase D.1 — mde-session Wayland session orchestrator.
install -D -m 0755 target/release/mde-session \
    %{buildroot}%{_bindir}/mde-session

# v2.0.0 Phase D.2 — mde-logout-dialog Iced confirmation dialog.
install -D -m 0755 target/release/mde-logout-dialog \
    %{buildroot}%{_bindir}/mde-logout-dialog

# v2.0.0 Phase A+B — `mded` daemon CLI surface. The Rust binary
# still builds under the legacy `mackesd` name (the workspace
# directory rename `crates/mackesd → crates/mded` is part of
# the v2.1+ Phase 0 rebrand). For now, ship `mded` as a shell
# shim that execs `mackesd` so every consumer that already
# spells the command `mded` (Python `mackes/workbench/fleet/*`,
# Iced `crates/mde-workbench` + `crates/mde-applets/*` +
# `crates/mde-wizard`, admin scripts via `systemctl restart
# mded`) keeps working through the rename without a gating
# binary rebuild.
install -D -m 0755 bin/mded \
    %{buildroot}%{_bindir}/mded

# v2.0.0 Phase E.8 — mde-applet-drawer (drawer overlay binary).
install -D -m 0755 target/release/mde-applet-drawer \
    %{buildroot}%{_bindir}/mde-applet-drawer

# v2.0.0 CB-1.10 — mde-wizard first-run provisioning UI.
install -D -m 0755 target/release/mde-wizard \
    %{buildroot}%{_bindir}/mde-wizard

# PC-12 (2026-05-21) — mde-peer-card hero modal spawned on
# mesh-peer join by mded's peer-join worker (PC-3). Always
# spawned on demand; no autostart entry needed.
if [ -f target/release/mde-peer-card ]; then
    install -D -m 0755 target/release/mde-peer-card \
        %{buildroot}%{_bindir}/mde-peer-card
fi

# v2.0.0 Phase E1.x — applet binaries (panel-host spawns these per
# pane / per tray slot via mde-panel host.rs).
for applet in mde-applet-app-switcher mde-applet-apple-menu \
              mde-applet-audio mde-applet-bg mde-applet-brightness-osd \
              mde-applet-clock mde-applet-dock mde-applet-mesh-status \
              mde-applet-network mde-applet-notification-bell \
              mde-applet-notifications mde-applet-recents \
              mde-applet-start-menu mde-applet-status-cluster \
              mde-applet-sway-cluster mde-applet-volume-osd; do
    if [ -f target/release/$applet ]; then
        install -D -m 0755 target/release/$applet \
            %{buildroot}%{_bindir}/$applet
    fi
done

# v12.16 — self-hosted DERP relay unit (only active on the Host-role
# peer; gated by ConditionPathExists=/var/lib/mde/derper.enabled).
install -D -m 0644 data/systemd/mde-derper.service \
    %{buildroot}%{_unitdir}/mde-derper.service
install -D -m 0644 data/headscale/derp-map.example.json \
    %{buildroot}%{_datadir}/mde/headscale/derp-map.example.json

# v4.0.1 BUG-13 / visual-identity.md Q11: Geologica font family
# (OFL 1.1, bundled from fonts.gstatic.com 2026-05-23). 5 weights —
# Light / Regular / Medium / Bold / Black — installed to
# /usr/share/fonts/geologica/ where fontconfig picks them up via the
# default font path. fc-cache fires in %post.
install -D -m 0644 data/fonts/Geologica-Light.ttf \
    %{buildroot}%{_datadir}/fonts/geologica/Geologica-Light.ttf
install -D -m 0644 data/fonts/Geologica-Regular.ttf \
    %{buildroot}%{_datadir}/fonts/geologica/Geologica-Regular.ttf
install -D -m 0644 data/fonts/Geologica-Medium.ttf \
    %{buildroot}%{_datadir}/fonts/geologica/Geologica-Medium.ttf
install -D -m 0644 data/fonts/Geologica-Bold.ttf \
    %{buildroot}%{_datadir}/fonts/geologica/Geologica-Bold.ttf
install -D -m 0644 data/fonts/Geologica-Black.ttf \
    %{buildroot}%{_datadir}/fonts/geologica/Geologica-Black.ttf
install -D -m 0644 data/fonts/Geologica-OFL.txt \
    %{buildroot}%{_datadir}/fonts/geologica/OFL.txt

cat > %{buildroot}%{_bindir}/mackes-clipboard <<'EOF'
#!/usr/bin/env bash
exec python3 -m mackes.clipboard_app "$@"
EOF
chmod 0755 %{buildroot}%{_bindir}/mackes-clipboard
# GVFS-mesh wrappers
install -m 0755 bin/mackes-gvfsd-mesh %{buildroot}%{_bindir}/mackes-gvfsd-mesh
install -m 0755 bin/mackes-mesh-open  %{buildroot}%{_bindir}/mackes-mesh-open

# v2.0.0 cut: mackes-enforce-session retired
# (xfce4-session is Conflicts now; no XFCE session means no
# need to enforce the i3 + mackes-panel session). The binary
# still ships at /usr/bin/ for back-compat (callers in v1.x
# upgrade paths can still invoke it; it's now a no-op stub
# that exits 0 on Wayland).
install -m 0755 bin/mackes-enforce-session \
    %{buildroot}%{_bindir}/mackes-enforce-session

# v2.0.0 cut: kdeconnect-indicator + xfce4-panel XDG suppressors
# retired (see the CB-3.5 / H.4 comment in section 5a).

%pre
# Phase 12.1 (1.0.7) — `mackesd` runs as a dedicated system user.
# The unit's StateDirectory=mackesd creates /var/lib/mackesd at first
# start; we just need the user to own it.
getent group mackesd >/dev/null 2>&1 || \
    groupadd --system mackesd
getent passwd mackesd >/dev/null 2>&1 || \
    useradd --system --gid mackesd --home-dir /var/lib/mackesd \
            --shell /sbin/nologin \
            --comment "Mackes Mesh control plane" mackesd

# VV-1 + VV-1.5 (v4.1.0) — voice stack runs as two dedicated
# users for defense-in-depth. Per-component naming locked
# 2026-05-24; the underscore prefix matches the design-doc
# convention.
getent group _kamailio_mde >/dev/null 2>&1 || \
    groupadd --system _kamailio_mde
getent passwd _kamailio_mde >/dev/null 2>&1 || \
    useradd --system --gid _kamailio_mde --home-dir /var/lib/kamailio-mde \
            --shell /sbin/nologin \
            --comment "MDE per-host Kamailio daemon (VV-1)" _kamailio_mde
getent group _rtpengine_mde >/dev/null 2>&1 || \
    groupadd --system _rtpengine_mde
getent passwd _rtpengine_mde >/dev/null 2>&1 || \
    useradd --system --gid _rtpengine_mde --home-dir /var/lib/rtpengine-mde \
            --shell /sbin/nologin \
            --comment "MDE per-host RTPengine relay (VV-1.5)" _rtpengine_mde
# Kamailio needs write access to RTPengine's NG socket. The
# `usermod -aG` is idempotent so this is safe on upgrade.
usermod -aG _rtpengine_mde _kamailio_mde 2>/dev/null || :

%post
/usr/share/%{appname}/install-helpers/create-mackes-user.sh || :
/usr/share/%{appname}/install-helpers/hide-xfce-settings.sh || :
# Enable SSH by default on every Mackes install (per design lock).
systemctl enable --now sshd.service || :
# Refresh systemd unit cache so the new mackes-* units are visible.
systemctl daemon-reload || :
# A8 (2026-05-29) — enable (but do NOT --now start) the mesh
# services here. `mde-install` stops → wipes /var/lib/mde →
# restarts them during convergence, so starting them in %post just
# means they briefly run against config that's about to be wiped.
# `enable` (no --now) means they come up on the next boot / when
# mde-install's start_services step runs, whichever is first.
# Phase 12.1 — mackesd store init is idempotent (migrate on start).
systemctl enable mackesd.service 2>/dev/null || :
# GF-1.2 (v5.0.0) — glusterd for the mesh-home volume (binds
# locally until GF-1.3's Nebula-overlay drop-in lands).
systemctl enable glusterd.service 2>/dev/null || :
# MON-1 (v2.6) — netdata for the apply_netdata_monitor birthright
# step (bound to 127.0.0.1 until birthright writes the overlay bind).
systemctl enable netdata.service 2>/dev/null || :
# VV-1 + VV-1.5 (v4.1.0) — voice stack state + spool + TLS dirs.
# Config trees /etc/kamailio-mde/ + /etc/rtpengine-mde/ are owned
# by the RPM (root:root 0755) so the generated config files end
# up world-readable, letting the daemons read them while running
# as their dedicated users. The state dirs hold per-peer
# secrets / TLS keys / Vitelity creds, so those stay tight
# (0750 owned by the service user). systemd's StateDirectory /
# RuntimeDirectory / LogsDirectory directives create most of
# these at first start, but seeding the TLS dir + the
# /etc/kamailio-mde subdirs keeps operator-side
# `ls /etc/kamailio-mde` predictable before the daemons enable.
install -d -m 0750 -o _kamailio_mde -g _kamailio_mde /etc/kamailio-mde/tls 2>/dev/null || :
install -d -m 0750 -o _kamailio_mde -g _kamailio_mde /var/lib/kamailio-mde 2>/dev/null || :
install -d -m 0750 -o _rtpengine_mde -g _rtpengine_mde /var/lib/rtpengine-mde 2>/dev/null || :
# Don't auto-enable the voice services yet — VV-1 + VV-1.5 ship
# the units + render-config hook, but the policy-driven config
# that routes real mesh + PSTN calls lands in VV-2..VV-4 / VV-14.
# The operator (or a v4.1.0 cut script) flips
# `systemctl enable --now kamailio-mde.service rtpengine-mde.service`
# once VV-4 + VV-14 are green. Until then, refresh systemd's
# cache so `systemctl status kamailio-mde` reports a sane
# "inactive (dead)" rather than "unknown unit."
systemctl daemon-reload 2>/dev/null || :
# Validate the sudoers drop-in we shipped; on failure remove it so we
# never break the host's sudo behavior.
visudo -c -f /etc/sudoers.d/mackes-shell >/dev/null 2>&1 \
    || rm -f /etc/sudoers.d/mackes-shell
# A5 (2026-05-29) — GTK icon-cache + fontconfig refreshes moved to
# `%post -n mde-desktop` below: base `mde-core` is headless (no GTK,
# no rendered icons/fonts), so caching them here was both wasted work
# and a layering violation. The dead `fc-cache .../fonts/geologica`
# line was also dropped — Geologica was retired for Roboto (Q44) and
# fonts now come from dnf-installed packages (mde-desktop Requires),
# not a vendored /usr/share/fonts/geologica tree that no longer ships.
# CB-3.4 — register the comps group so `dnf groupinstall
# mackes-desktop-environment` resolves on this host. Silently no-ops
# on systems where dnf-plugins-core isn't available.
dnf groups mark install mackes-desktop-environment 2>/dev/null || :
# v2.0.1 hotfix — sweep orphan xsession .desktop files installed by
# pre-2.0 shell scripts (xfce11-unified era). RPM never owned these,
# so dnf can't sweep them through file ownership; LightDM otherwise
# shows broken entries or hides the MDE Wayland session entirely.
# Mirrors mackes.birthright.apply_uninstall_legacy_xsessions for the
# install/upgrade path. Idempotent: rm -f silently no-ops on absent
# files.
rm -f /usr/share/xsessions/xfce11-i3-plank.desktop \
      /usr/share/xsessions/xfce11.desktop \
      /usr/share/xsessions/mackes.desktop 2>/dev/null || :

# A5 (2026-05-29) — GUI asset caches belong with the desktop addon,
# not the headless base. gtk-update-icon-cache needs GTK (an
# mde-desktop dep) and only matters once a UI renders. (Mackes-Carbon
# is mid-retirement to Material Symbols per Q43/Q97; cached here until
# EPIC-UI-MATERIAL removes it. Roboto/Material fonts come from
# dnf-installed font packages, which carry their own fontconfig
# triggers — no manual fc-cache needed.)
%post -n mde-desktop
gtk-update-icon-cache -f -t %{_datadir}/icons/Black-Sun     2>/dev/null || :
gtk-update-icon-cache -f -t %{_datadir}/icons/Mackes-Carbon 2>/dev/null || :
# SWAY-9.a (2026-05-29) — provision the regreet-curated sessions dir.
# regreet reads /var/lib/mde/wayland-sessions exclusively (DM-3, to keep
# stray third-party /usr/share/wayland-sessions/*.desktop out of the
# picker). Symlink the MDE session in at install time so the greeter has
# it before first login — a first-login birthright step would deadlock
# (the picker is pre-auth). Idempotent on upgrade via `ln -sf`.
mkdir -p %{_sharedstatedir}/mde/wayland-sessions
ln -sf %{_datadir}/wayland-sessions/mde.desktop \
    %{_sharedstatedir}/mde/wayland-sessions/mde.desktop

%preun
# Only on uninstall, not upgrade ($1 == 0 → final removal)
if [ "$1" = "0" ]; then
    /usr/share/%{appname}/install-helpers/restore-xfce-settings.sh || :
fi

# INST-2 (v2.7) — post-transaction convergence banner. %posttrans
# runs once at the end of every dnf transaction (install AND
# upgrade), so the operator always sees the reminder to run the
# installer. echo-only — no binary is invoked, so unattended dnf,
# image builds, and rpm-ostree complete normally (Fedora packaging
# guideline). Voice-and-tone: factual, no emoji/exclamation.
%posttrans
echo ">>> mde-core installed. Run \`sudo mde-install\` to finish setup."

%posttrans -n mde-desktop
echo ">>> mde-desktop installed. Run \`sudo mde-install --profile=full\` to finish setup."

%files
%license LICENSE
%doc README.md CHANGELOG.md docs/MACKES_SHELL_SPEC.md docs/MIGRATION_FROM_V2.2.md
# INST-1 (v2.7, 2026-05-25) — base RPM ships the CLI entry
# points, the mackesd daemon, the migrators, and the Python
# birthright tree. Every GUI binary moves to `%files desktop`
# below so `dnf install mde` on a lighthouse VPS doesn't pull
# in sway / iced / GTK / Roboto.
%{_bindir}/mackes
%{_bindir}/mackesd
%{_bindir}/mded
# INST-3 (v2.7) — installer + updater, universal substrate (base).
%{_bindir}/mde-install
%{_bindir}/mde-update
# mde-* wrapper + migrators stay in base — they're invocable
# on lighthouses too (the migrators run pre-install on every
# upgrade path, with or without a GUI).
%{_bindir}/mde
%{_bindir}/mde-migrate-from-1x
%{_bindir}/mde-shell-migrate-v2
# v2.0.0 Phase 0.3 — man pages.
%{_mandir}/man1/mde.1*
%{_mandir}/man1/mde-migrate-from-1x.1*
%{_mandir}/man1/mde-shell-migrate-v2.1*
%{_mandir}/man8/mded.8*
# v2.0.3 build-identity files (%{_datadir}/mde/build-{hash,date})
# are covered by the %{_datadir}/%{appname}/ catch-all below.
# v2.0.0 Phase 0.4 — D-Bus service files. mackesd registers
# these at run-time; keeping them in base means a lighthouse
# install of `mde` alone still exposes the daemon's surfaces
# for CLI consumers (`busctl --user call` etc.). Desktop GUI
# clients added by `mde-desktop` consume the same service
# names without owning the .service files.
%{_datadir}/dbus-1/services/dev.mackes.MDE.*.service
%{_datadir}/dbus-1/services/org.mackes.*.service
# v2.0.0 Phase D.5 — sway config now covered by the
# %{_datadir}/%{appname}/ catch-all below (Name: mde, so
# %{_datadir}/%{appname}/ == /usr/share/mde/).
%{py3_sitelib}/mackes/
%{py3_sitelib}/mackes_shell-%{version}.dist-info/
%{_datadir}/%{appname}/
# A1 fix — back-compat symlink: /usr/share/mackes-shell -> mde, so the
# ~40 legacy consumers still reading /usr/share/mackes-shell/ resolve.
%{_datadir}/mackes-shell
# Metainfo for the dev.mackes.MDE upstream identity. Stays in
# base so `appstream-cli` / dnf metadata clients can list MDE
# even on lighthouse installs.
%{_metainfodir}/dev.mackes.MDE.metainfo.xml
%{_metainfodir}/io.github.matthewmackes.MackesShell.metainfo.xml
%{_metainfodir}/shell.mackes.Panel.metainfo.xml
%{_datadir}/icons/hicolor/scalable/apps/mackes-shell.svg
%{_unitdir}/mackes-node.service
%{_unitdir}/mackes-tailscale-bootstrap.service
%{_unitdir}/mackesd.service
# NF-3.1/3.2/3.3 (v2.5) — Nebula systemd units. Activation
# is supervisor-driven (NF-3.4 nebula_supervisor writes the
# role.host marker that gates the lighthouse + tunnel
# units); the regular nebula.service runs on every node.
%{_unitdir}/nebula.service
%{_unitdir}/nebula-lighthouse.service
%{_unitdir}/mackes-nebula-https-tunnel.service
# NF-6.2 — sealed dirs the supervisor writes into.
%dir %attr(0700, root, root) /var/lib/mackesd/nebula-ca
%dir %attr(0755, root, root) /etc/nebula
%dir %attr(0700, root, root) /var/lib/mackesd/nebula-peers
# GF-1.3.a — overlay-ip publish dir. 0755 (world-readable) so
# downstream services like glusterd-nebula-bind can `cat` the
# file without sudo.
%dir %attr(0755, root, root) /var/lib/mackesd/nebula
# GF-4.1 (v5.0.0) — mesh-home FUSE mount template. Stays in
# base so a lighthouse install can mount mesh-home even
# without the desktop session host.
%{_userunitdir}/mde-mesh-mount@.service
# v12.16 Self-hosted DERP relay unit. Inactive on non-Host peers
# (gated by ConditionPathExists=/var/lib/mde/derper.enabled). The
# headscale DERP-map example shipped under
# %{_datadir}/%{appname}/headscale/ is covered by the data catch-all.
%{_unitdir}/mde-derper.service
# BUS-3.1 — firewalld service definition. Installed inert; the
# operator applies via `firewall-cmd --add-service=mackes-bus` if
# explicit firewalld allow-listing is wanted on top of the
# default bind-to-overlay-IP security boundary.
%{_prefix}/lib/firewalld/services/mackes-bus.xml
# MON-2 (v2.6) — Netdata health.d alert configs. %config(noreplace)
# so operator edits to thresholds survive package upgrades; the
# defaults match the v2.6 MON-2 design lock.
%dir %{_sysconfdir}/netdata/health.d
%config(noreplace) %{_sysconfdir}/netdata/health.d/nebula.conf
%config(noreplace) %{_sysconfdir}/netdata/health.d/gluster.conf
%config(noreplace) %{_sysconfdir}/netdata/health.d/mackesd.conf
%config(noreplace) %{_sysconfdir}/netdata/health.d/workstation.conf
%config(noreplace) %{_sysconfdir}/netdata/health.d/mde-suppressions.conf
# DM-3 (v2.7) — regreet config + stylesheet. The .toml is
# %config(noreplace) so operator tweaks survive upgrades; the
# .css under /usr/share/mde/regreet/ is covered by the data
# catch-all below.
%dir %{_sysconfdir}/regreet
%config(noreplace) %{_sysconfdir}/regreet/regreet.toml
# DM-2 (v2.7) — greetd substrate config lives at
# /usr/share/mde/greetd/config.toml (covered by the existing
# %{_datadir}/%{appname}/ catch-all below). Lives there rather
# than directly in /etc/greetd/ because Fedora's greetd RPM
# already owns /etc/greetd/config.toml. DM-5's birthright
# step copies this over the live /etc/greetd/config.toml at
# install-time.
%{_prefix}/lib/systemd/user-preset/90-mackes.preset
%{_prefix}/lib/systemd/user-preset/90-mde.preset
# CB-2.4 — first-boot orchestration target + the two oneshot
# migrator units. Migrators stay in base because they run on
# every upgrade path (GUI or headless) before the desktop
# session host comes up.
%{_userunitdir}/mde-firstboot.target
%{_userunitdir}/mde-migrate-from-1x.service
%{_userunitdir}/mde-shell-migrate-v2.service
%config(noreplace) /etc/sudoers.d/mackes-shell
# KDC2-1.10 — Connect routing policy. Operator edits survive
# package upgrades. Stays in base because mackesd reads it
# at start-up regardless of whether the GUI is installed.
%dir %{_sysconfdir}/mde
%dir %{_sysconfdir}/mde/connect
%config(noreplace) %{_sysconfdir}/mde/connect/policy.toml
# Plymouth MackesDE boot theme. Stays in base because Plymouth
# can render on TTY-only lighthouse boxes (themes are inert
# until activated; activation happens in the wizard's
# birthright step which is gated on the full profile).
%{_datadir}/plymouth/themes/mde/

%files -n mde-desktop
# INST-1 (v2.7, 2026-05-25) — every Wayland-stack GUI binary
# the platform ships. A `dnf install mde` lighthouse install
# omits all of these; `dnf install mde mde-desktop` (or the
# `mackes-desktop-environment` comps group) pulls them back.
%{_datadir}/applications/mackes-shell.desktop
%{_datadir}/applications/mackes-clipboard.desktop
# mackes-mesh-uri-handler.desktop RETIRED in DEAD-2.11 (2026-05-26).
# v2.0.0 Phase 0.9 — MDE-namespaced .desktop launcher.
%{_datadir}/applications/mde.desktop
# Legacy mackes-* GTK panel binaries that the v2.0+ Iced port
# is still co-shipping pending per-panel retirements.
%{_bindir}/mackes-clipboard
%{_bindir}/mackes-enforce-session
%{_bindir}/mackes-gvfsd-mesh
%{_bindir}/mackes-mesh-open
%{_bindir}/mackes-panel
%{_bindir}/mackes-wm
# Iced GUI binaries (mde-* family).
%{_bindir}/mde-workbench
%{_datadir}/applications/mde-workbench.desktop
%{_bindir}/mde-panel
%{_bindir}/mde-popover
%{_bindir}/mde-portal-full
%{_bindir}/mde-open
%{_datadir}/applications/mde-open.desktop
%{_bindir}/mde-files
%{_datadir}/applications/mde-files.desktop
%{_bindir}/mde-session
%{_bindir}/mde-logout-dialog
%{_bindir}/mde-wizard
%{_bindir}/mde-peer-card
# Per-applet binaries (16).
%{_bindir}/mde-applet-drawer
%{_bindir}/mde-applet-app-switcher
%{_bindir}/mde-applet-apple-menu
%{_bindir}/mde-applet-audio
%{_bindir}/mde-applet-bg
%{_bindir}/mde-applet-brightness-osd
%{_bindir}/mde-applet-clock
%{_bindir}/mde-applet-dock
%{_bindir}/mde-applet-mesh-status
%{_bindir}/mde-applet-network
%{_bindir}/mde-applet-notification-bell
%{_bindir}/mde-applet-notifications
%{_bindir}/mde-applet-recents
%{_bindir}/mde-applet-start-menu
%{_bindir}/mde-applet-status-cluster
%{_bindir}/mde-applet-sway-cluster
%{_bindir}/mde-applet-volume-osd
# v2.0.0 Phase 0.3 — mde-* GUI helpers.
%{_bindir}/mde-wm
%{_bindir}/mde-enforce-session
# v2.0.3 — per-output default scale picker (sway-only).
%{_bindir}/mde-output-autoscale
# v4.0.1 visual-identity.md Q11 — Geologica font family (OFL 1.1).
# Grandfathered alongside the CR-1 Roboto swap; both shipped
# under the desktop addon (lighthouse installs skip all UI fonts).
%dir %{_datadir}/fonts/geologica
%{_datadir}/fonts/geologica/Geologica-Light.ttf
%{_datadir}/fonts/geologica/Geologica-Regular.ttf
%{_datadir}/fonts/geologica/Geologica-Medium.ttf
%{_datadir}/fonts/geologica/Geologica-Bold.ttf
%{_datadir}/fonts/geologica/Geologica-Black.ttf
%license %{_datadir}/fonts/geologica/OFL.txt
# Vendored GTK themes — only render in a graphical session.
%{_datadir}/themes/Orchis-Dark/
%{_datadir}/themes/Shiki-Statler/
%{_datadir}/themes/Mackes-Dark/
# Portal-37 — runtime app theme + Qt6 color scheme.
%{_datadir}/themes/MDE-Dark/
%{_datadir}/qt6ct/colors/MDE-Dark.conf
# Vendored icon themes — same reason.
%{_datadir}/icons/Black-Sun/
%{_datadir}/icons/Mackes-Carbon/
# DEAD-2.11 (2026-05-26) retired the gvfs mesh-mount handler +
# mackes-mesh thumbnailer along with mesh_browser.py. The install
# lines were dropped in that commit; these %files entries were
# left dangling and broke the rpm build (file-not-found in
# BUILDROOT). Retired here as a hygiene fix.
# Wayland session entry — only useful when a DM is on the box.
# (mde-desktop %post symlinks this into /var/lib/mde/wayland-sessions/
# — the dir regreet reads — at install time, before first login.)
%{_datadir}/wayland-sessions/mde.desktop
# v6.0 Portal-1 — mde-portal unified shell + user service.
%{_bindir}/mde-portal
%{_userunitdir}/mde-portal.service
# v2.0.0 Phase D.6 mde-session.service — user-session
# orchestrator. Only runs after the operator logs in to the
# Wayland session host.
%{_userunitdir}/mde-session.service
# VV-1 (v4.1.0) — Kamailio daemon. Voice/video stack is a
# desktop feature (the GUI is what calls it); on lighthouses
# the SIP path stays dormant.
%{_unitdir}/kamailio-mde.service
%dir %attr(0755, root, root) /etc/kamailio-mde
%{_unitdir}/rtpengine-mde.service
%dir %attr(0755, root, root) /etc/rtpengine-mde

%files -n mde-xorg
%{_bindir}/mde-session-xorg
%{_bindir}/mde-xorg-session
%{_datadir}/xsessions/mde-xorg.desktop
%{_datadir}/mde-xorg/
%{_userunitdir}/mde-xorg.target

%changelog
* Fri May 22 2026 Matt Mackes <matthewmackes@gmail.com> - 2.0.3-1
- Operator-verification hotfix bundle (2026-05-22 bench install on
  a laptop + 4K-TV dual-monitor rig surfaced 7 defects, fixed at
  source). Full per-defect breakdown in CHANGELOG.md.
- Sway config: bindsym restart → reload (i3-only command was firing
  swaynag every login); removed 5 duplicate bindings overlapping
  mackes-defaults.conf; added arrow-key focus aliases; added
  `exec mde-panel` autostart line.
- mde-panel: fixed Wayland xdg-shell app_id propagation
  (shell.mackes.Panel) — Iced 0.13 doesn't inherit it from
  iced::Settings.id on Linux; needs window::Settings.platform_specific.
- mde-migrate-from-1x: now disables + removes obsolete v1.x systemd
  user units (qnm-daemon.service first), stopping a 290+/min
  restart loop on every fresh v2 boot.
- packaging: Requires: mako + Conflicts: dunst so Wayland-native
  notifications converge on install. Operator helper
  install-helpers/bench-bootstrap.sh ships for in-place upgrades.
- bin/mde-output-autoscale: width-based per-output scale picker
  applied at every session start (4K → 2.0, 2K → 1.5, ≤1080p →
  1.0) so 4K TVs are readable out of the box. Sacrosanct override
  rule: scale != 1.0 is treated as intentional and skipped.
- Right-click admin menu: every sudo call site replaced with
  pkexec sh -c so privileged actions work under Wayland sessions
  where terminator doesn't always inherit a controlling TTY.
- Desktop watermark: rebranded "Mackes XFCE Workstation" →
  "Mackes Desktop Environment". Added build-date stamp synced
  between legacy GTK + new Iced watermarks via
  /usr/share/mde/build-{hash,date}.

* Wed May 20 2026 Matt Mackes <matthewmackes@gmail.com> - 2.0.0-1
- v2.0.0 monolithic cut commit (CB-3.1 lock + CB-2.2 + CB-3.2 +
  CB-3.3 + CB-3.5 + H.1 + H.2 + H.4 + Phase 0.7 + 0.8 landed
  together so dnf upgrade lands on mde-2.0.0 in one transaction).
- Name: mackes-xfce-workstation → mde. Provides + Obsoletes carry
  both mackes-shell and mackes-xfce-workstation so existing dnf
  upgrade paths resolve cleanly.
- CB-3.2: every XFCE Requires dropped (xfconf, xfce4-settings,
  xfce4-session, xfce4-power-manager, terminator, i3, i3status,
  dmenu, wmctrl, xprop, xorg-x11-server-utils, xdotool); the
  Wayland stack lands hard-Requires (sway, swaylock, swayidle,
  swaybg, foot, bemenu, brightnessctl, pipewire, wireplumber,
  grim, slurp). New Recommends: cosmic-files, yazi, kanshi,
  wlogout, wofi.
- CB-3.3: Conflicts block per Q5 lock (xfce4-panel, xfdesktop,
  xfce4-session, xfce4-settings, xfwm4, xfce4-whiskermenu-plugin,
  xfce4-docklike-plugin, xfce4-pulseaudio-plugin,
  xfce4-power-manager-plugin, i3 — all `< 999` cap pattern).
- CB-3.5 / H.4: every XDG autostart override retired
  (mackes-panel.desktop, xfdesktop.desktop, mackes-enforce-session
  .desktop, mackes-suppress-xfce4-panel.desktop,
  kdeconnect-indicator.desktop). Wayland session orchestrator
  handles bring-up natively.
- C panel-plugin trio retired (mackes-clipboard, mackes-launcher,
  mackes-drawer). Their roles move to native mackes-panel applets
  in Phase E.1.x (Iced port). BuildRequires drops
  xfce4-panel-devel + libxfce4ui-devel.
- 21 Iced Workbench panels shipped in v1.1.x partial-progress
  cuts now compose the v2.0.0 Workbench surface (mde-workbench
  binary, /usr/bin/mde-workbench).
- 5 mded subcommands shipped: nodes list, ansible-history list,
  playbooks {list, run}, events list.

* Wed May 20 2026 Matt Mackes <matthewmackes@gmail.com> - 1.1.4-1
- Drop all 5 remaining XFCE Obsoletes (dnf5 implicit_ts_elements
  assertion fix, take 2). RPM now installs cleanly on Fedora 44.

* Sat May 16 2026 Matt Mackes <matthewmackes@gmail.com> - 0.2.0-1
- Identity: stripped "PRIVATE WORK / Sub Testing Release" markers from
  dashboard, wizard, and About dialog.
- Wizard rebuilt as a 3-act ceremony: spare welcome, 4-card preset picker
  with wallpaper thumbnails, narrated apply ("Becoming <preset>" →
  "You are now <preset>").
- Replaced single chupre preset with FOUR: hashbang (default, display
  '#!'), mackes, daylight, vanilla. Each ships its own polybar / plank /
  rofi profile + wallpaper.
- Design system: SF Pro typography stack, mackes-* CSS classes, and
  per-preset accent CSS swapped at startup based on state.active_preset.
- New Polybar Editor (replaces Q12 preset-picker lock): 21 vendored
  adi1090x families, pure-function generator, 3-zone DnD module editor,
  save-as-profile, live debounced apply.
- MaintenanceKit: 4 new panels — System Update (pkexec dnf wrapper),
  Drift (first-class per-key revert/adopt), Fonts (fc-list browser +
  preview + quick installs), Power (power-profiles-daemon), Resources
  (CPU/RAM/disk live cards).
- Recovery shell foundation: mackes/recover.py TTY snapshot picker,
  mackes-recovery.target systemd unit, GRUB submenu source, idempotent
  install-helpers/install-recovery.sh.
- Update mechanism plumbing: data/dnf/mackes-shell.repo pointing at
  matthewmackes.github.io/MAP2-RELEASES, add-mackes-repo.sh helper.
- ISO build scaffolding: packaging/iso/mackes-xfce.ks kickstart + `make
  iso` target.
- Headless apply CLI: `python3 -m mackes.cli_apply --preset NAME`.
- Test runner that works without pytest: tests/_run_without_pytest.py
  (`make test-nodeps`).
- Vendored adi1090x/polybar-themes (GPL-3.0) under
  data/shell-profiles/polybar/upstream/ — 8.7 MB, 21 families.

* Sat May 16 2026 Matt Mackes <matthewmackes@gmail.com> - 0.1.1-1
- MAP2 Sub Testing Release. PRIVATE WORK.
- Drop Workstation/Laptop/Audio Rig/Server Console presets; ship only chupre (Q1).
- Add Apps tab (Install / Remove / Installed) — curated install set with
  third-party repo auto-enablement (Microsoft Edge, VS Code) plus Cursor
  AppImage and Claude CLI via npm (C1–C3).
- Add Lean XFCE removal — xfce4-panel/appfinder/xfdesktop/xfce4-notifyd
  removed when their replacements (polybar/rofi/plank/dunst) are running.
  Reinstalled automatically on uninstall (X1–X5).
- Add Maintain → Uninstall + mackes --uninstall CLI + uninstall.sh artifact.
  Auto-snapshots to ~/Desktop/, wipes user files, resets xfconf, cleans v2.2
  leftovers (Q8–Q30, Q46–Q47).
- Add session-manager extension: managed-process supervisor with status
  dots on Dashboard + Start/Stop/Restart controls in System → Session
  (C6, C11). Also drives chupre dotfiles staging (alacritty + gtk-3.0 +
  gtk-4.0 from chupre/dotfiles tree/v2/.config).
- Fix Polybar autostart: install ~/.config/autostart/mackes-polybar.desktop,
  parse bar names from the active profile (no more hardcoded `polybar
  mackes`), and capture stderr to ~/.local/share/mackes-shell/logs/polybar.log
  (P1 lock).
- Wire MAP2 branding logo into About dialog, Wizard welcome screen and
  Dashboard header. Add 'PRIVATE WORK — Sub Testing Release' banner to
  the Dashboard.

* Fri May 15 2026 Matt Mackes <matthewmackes@gmail.com> - 0.1.0-1
- Initial Mackes Shell RPM.
- Replaces xfce4-settings menu entries via NoDisplay overrides (Q19).
- Ships the erikdubois/plankthemes catalog under data/plank-themes/.
- Single-binary entry: /usr/bin/mackes routes to wizard or workbench.
