# Disable the auto-generated debuginfo / debugsource subpackages. Our C
# panel plugin is tiny (~60 KB stripped) and we don't ship separate
# debug builds. Avoids "Empty %files file debugsourcefiles.list" errors
# from rpmbuild on Fedora 40+.
%global debug_package %{nil}

Name:           mde
Version:        2.0.0
Release:        1%{?dist}
Summary:        Mackes Desktop Environment (MDE) — Wayland-only Fedora DE

License:        GPL-3.0
URL:            https://github.com/matthewmackes/MAP2-RELEASES
# Source tarball still ships under the legacy name so dist/mackes-shell-...
# keeps working; the package itself is renamed via Provides/Obsoletes.
Source0:        mackes-shell-%{version}.tar.gz

# v2.0.0 cut commit — package renamed `mackes-xfce-workstation` →
# `mde`. Back-compat Provides/Obsoletes:
Provides:       mackes-shell = %{version}-%{release}
Provides:       mackes-xfce-workstation = %{version}-%{release}
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
Requires:       python3-gobject
Requires:       gtk3
Requires:       python3-pyyaml

# CB-3.2 Wayland-stack hard-Requires (v2.0.0 lock).
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

# Phase 13.1.1 — KDE Connect Option A integration. Workbench
# Connect talks org.kde.kdeconnect.* DBus + a mesh-mDNS bridge.
# The upstream kdeconnectd daemon stays user-session-autostarted.
Requires:       kdeconnectd

# SSH enabled by default on every Mackes install
Requires:       openssh-server

# v2.0.0 cut: wmctrl (X11-only) dropped. Wayland window
# enumeration goes through swayipc-async (Phase E rewrite)
# or `swaymsg -t get_tree` subprocess (today).

# Plymouth boot splash — Mackes ships its own theme (data/plymouth/mackes/)
Requires:       plymouth
Requires:       plymouth-scripts

# Flatpak — birthright wizard adds the Flathub remote per-user
Recommends:     flatpak

# Remote desktop birthright (v1.2.0) — every node serves xrdp + x11vnc and
# runs a local Tomcat-hosted Guacamole web app behind the Caddy gateway.
# The web .war is fetched from the Apache archive during the wizard's
# Remote desktop step (not vendored to keep the RPM size sane).
Requires:       xrdp
Requires:       xrdp-selinux
Requires:       x11vnc
Requires:       guacd
Requires:       tomcat
Requires:       curl

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

# Mesh fabric (§8.11–§8.14): WireGuard via Tailscale + self-hosted Headscale
Requires:       tailscale
Requires:       headscale

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

# Mesh-services unified gateway (§8.13 Layer 3) — optional, opt-in via UI
Recommends:     caddy

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
%package xorg
Summary:        Mackes Desktop Environment — XOrg/i3 session
Requires:       mde = %{version}-%{release}
Requires:       i3
Requires:       i3lock
Requires:       xorg-x11-server-utils
Requires:       xfce4-terminal
Requires:       scrot
Requires:       dmenu
Recommends:     brightnessctl

%description xorg
XOrg/i3 session add-on for Mackes Desktop Environment.  Installs
the i3-backed mde-session binary (built --features x11), the
mde-xorg.desktop XSession entry, an i3 config tuned for MDE, and
a systemd user target that gates on DISPLAY instead of
WAYLAND_DISPLAY.  Install alongside the main `mde` package to get
both session choices in the display-manager greeter.

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
install -d %{buildroot}%{_datadir}/%{name}/data
cp -r data/presets        %{buildroot}%{_datadir}/%{name}/data/
cp -r data/wallpapers     %{buildroot}%{_datadir}/%{name}/data/
cp -r data/css            %{buildroot}%{_datadir}/%{name}/data/
cp -r data/dnf            %{buildroot}%{_datadir}/%{name}/data/
# Fleet management (v1.3.0) — 7 curated Ansible roles
cp -r data/ansible        %{buildroot}%{_datadir}/%{name}/data/
# v2.2.0 — data/conky/ removed (Conky HUD replaced by Notification Drawer).
# v1.6.2 — canonical xfce4-panel snapshot for apply_panel_layout.
# data/panel/xfce4-panel.snapshot.json is the platform default panel
# (regenerated via tools/snapshot-panel.py on the reference box).
if [ -d data/panel ]; then
    cp -r data/panel      %{buildroot}%{_datadir}/%{name}/data/
fi
# Mackes Conky autostart .desktop
# v2.2.0 — mackes-conky.desktop removed (Conky HUD replaced by the
# Notification Drawer panel applet).
cp -r data/systemd        %{buildroot}%{_datadir}/%{name}/data/
# Vendored GTK themes + icon theme — system-wide install
install -d %{buildroot}%{_datadir}/themes
cp -r data/themes/Orchis-Dark   %{buildroot}%{_datadir}/themes/
cp -r data/themes/Shiki-Statler %{buildroot}%{_datadir}/themes/
install -d %{buildroot}%{_datadir}/icons
cp -r data/icons/Black-Sun     %{buildroot}%{_datadir}/icons/
cp -r data/icons/Mackes-Carbon %{buildroot}%{_datadir}/icons/
# Plymouth Mackes boot theme — installed but NOT activated at %post; the
# wizard's birthright step (mackes.birthright.apply_plymouth) activates it
# only when the user opts in (initrd rebuild is heavy and disruptive).
install -d %{buildroot}%{_datadir}/plymouth/themes
cp -r data/plymouth/mackes %{buildroot}%{_datadir}/plymouth/themes/
# v2.0.0 cut: C panel-plugin install steps retired (see %build).
cp -r data/grub           %{buildroot}%{_datadir}/%{name}/data/
cp    data/media-services.yaml %{buildroot}%{_datadir}/%{name}/data/
# Per-preset captured xfconf baselines (§8 lock). Each preset directory
# contains <channel>.txt dumps from `xfconf-query --channel X --list -v`
# plus a manifest.json. Apply via install-helpers/apply-xfce-baseline.sh.
if [ -d data/xfce-baseline ]; then
    cp -r data/xfce-baseline %{buildroot}%{_datadir}/%{name}/data/
fi

# 3a. Mackes Shell branding — hero logo (About / Wizard / Dashboard) +
#     standard wallpaper (desktop + LightDM greeter)
install -d %{buildroot}%{_datadir}/%{name}/branding
cp -r branding/* %{buildroot}%{_datadir}/%{name}/branding/

# 3b. In-Mackes help documentation (docs/help/*.md)
install -d %{buildroot}%{_datadir}/%{name}/help
cp docs/help/*.md %{buildroot}%{_datadir}/%{name}/help/

# 3c. Apple-menu "About Mackes" credits + license file (1.0.7+).
#     Consumed by mackes/about.py via /usr/share/mackes-shell/ABOUT.txt.
install -D -m 0644 data/ABOUT.txt %{buildroot}%{_datadir}/%{name}/ABOUT.txt

# 4. Install helper scripts (called from %%post / %%preun, plus the
#    capture/apply pair admins use to manage XFCE baselines, plus the
#    theme/icon bootstrap, lightdm config, and mackes-user creation)
install -d %{buildroot}%{_datadir}/%{name}/install-helpers
for helper in \
    hide-xfce-settings.sh restore-xfce-settings.sh \
    add-mackes-repo.sh install-recovery.sh \
    capture-xfce-baseline.sh apply-xfce-baseline.sh \
    configure-lightdm.sh create-mackes-user.sh \
    mesh-ca-trust.sh register-gvfs-mesh.sh ; do
    install -m 0755 install-helpers/$helper \
        %{buildroot}%{_datadir}/%{name}/install-helpers/
done

# 4a. data/mesh-ssh-policy.example.yaml
install -D -m 0644 data/mesh-ssh-policy.example.yaml \
    %{buildroot}%{_datadir}/%{name}/data/mesh-ssh-policy.example.yaml

# 4b. systemd units (system + user)
install -d %{buildroot}%{_unitdir}
install -m 0644 data/systemd/mackes-node.service             %{buildroot}%{_unitdir}/
install -m 0644 data/systemd/mackes-tailscale-bootstrap.service %{buildroot}%{_unitdir}/
# v2.0.0 Phase B.13 — 10 standalone .service/.timer units retired
# (mackes-clipboard-daemon, mackes-gvfsd-mesh, mackes-mdns-relay,
# mackes-remmina-sync.{service,timer}, mackes-media-sync.{service,
# timer}, mackes-ansible-pull.{service,timer}, mackesd-kdc-bridge).
# Each role now runs as an in-process worker inside `mackesd serve`
# (mackesd.service ExecStart points there).
install -d %{buildroot}%{_userunitdir}
# v2.0.0 Phase D.6 — mde-session user unit (Wayland orchestrator).
install -m 0644 data/systemd/mde-session.service             %{buildroot}%{_userunitdir}/
# Sudoers drop-in (v1.4.1) — grants NOPASSWD on Mackes-managed commands
install -D -m 0440 data/sudoers.d/mackes-shell               %{buildroot}/etc/sudoers.d/mackes-shell
install -D -m 0755 bin/mackes-wm                              %{buildroot}%{_bindir}/mackes-wm

# 1.0.7 — default i3 config shipped to /usr/share/mackes-shell/i3/.
# mackes-wm seeds ~/.config/i3/config from this file on first switch.
install -D -m 0644 data/i3/config %{buildroot}%{_datadir}/%{name}/i3/config
# 1.1.x — Phase 6.4 hotkeys + Phase 3.6 apple-menu shortcut land as a
# config.d drop-in so user overrides at
# ~/.config/i3/config.d/mackes-overrides.conf load alphabetically
# after our defaults. The main config already `include`s the
# directory (added 1.1.0 for the gaps profile picker).
install -D -m 0644 data/i3/config.d/mackes-defaults.conf \
    %{buildroot}%{_datadir}/%{name}/i3/config.d/mackes-defaults.conf
# v2.0.0 Phase D.5 — sway config shipped under /usr/share/mde/sway/.
# mde-session seeds ~/.config/sway/ from this directory on first
# launch via the mde-shell-migrate-v2 step.
install -D -m 0644 data/sway/config %{buildroot}%{_datadir}/mde/sway/config
install -D -m 0644 data/sway/config.d/mackes-defaults.conf \
    %{buildroot}%{_datadir}/mde/sway/config.d/mackes-defaults.conf
# v2.0.0 Phase 0.5 + H.5 — first-boot config migrators.
install -D -m 0755 bin/mde-migrate-from-1x \
    %{buildroot}%{_bindir}/mde-migrate-from-1x
install -D -m 0755 bin/mde-shell-migrate-v2 \
    %{buildroot}%{_bindir}/mde-shell-migrate-v2
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

# CB-2.1 — Wayland-session entry. LightDM / GDM / SDDM all read
# /usr/share/wayland-sessions/ for available sessions.
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

# 4c. Tumbler thumbnailer
install -D -m 0644 data/thumbnailers/mackes-mesh.thumbnailer \
    %{buildroot}%{_datadir}/thumbnailers/mackes-mesh.thumbnailer

# 4d. GVFS mount registration (mesh:// URI scheme)
install -D -m 0644 data/gvfs/mesh.mount \
    %{buildroot}%{_datadir}/gvfs/mounts/mesh.mount

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
install -D -m 0644 data/applications/mackes-mesh-uri-handler.desktop \
    %{buildroot}%{_datadir}/applications/mackes-mesh-uri-handler.desktop
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

# v2.0.0 Phase D.1 — mde-session Wayland session orchestrator.
install -D -m 0755 target/release/mde-session \
    %{buildroot}%{_bindir}/mde-session

# v2.0.0 Phase D.2 — mde-logout-dialog Iced confirmation dialog.
install -D -m 0755 target/release/mde-logout-dialog \
    %{buildroot}%{_bindir}/mde-logout-dialog

# v2.0.0 Phase A+B — mded daemon binary (renamed mackesd surface).
install -D -m 0755 target/release/mded \
    %{buildroot}%{_bindir}/mded

# v2.0.0 Phase E.8 — mde-applet-drawer (drawer overlay binary).
install -D -m 0755 target/release/mde-applet-drawer \
    %{buildroot}%{_bindir}/mde-applet-drawer

# v2.0.0 CB-1.10 — mde-wizard first-run provisioning UI.
install -D -m 0755 target/release/mde-wizard \
    %{buildroot}%{_bindir}/mde-wizard

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

%post
/usr/share/%{name}/install-helpers/create-mackes-user.sh || :
/usr/share/%{name}/install-helpers/hide-xfce-settings.sh || :
# Enable SSH by default on every Mackes install (per design lock).
systemctl enable --now sshd.service || :
# Refresh systemd unit cache so the new mackes-* units are visible.
systemctl daemon-reload || :
# Phase 12.1 — initialize the mackesd store on install/upgrade. The
# migrate subcommand is idempotent (no-op if schema is current).
systemctl enable --now mackesd.service 2>/dev/null || :
# Validate the sudoers drop-in we shipped; on failure remove it so we
# never break the host's sudo behavior.
visudo -c -f /etc/sudoers.d/mackes-shell >/dev/null 2>&1 \
    || rm -f /etc/sudoers.d/mackes-shell
# Rebuild the GTK icon caches for the vendored icon themes
gtk-update-icon-cache -f -t %{_datadir}/icons/Black-Sun     2>/dev/null || :
gtk-update-icon-cache -f -t %{_datadir}/icons/Mackes-Carbon 2>/dev/null || :
# CB-3.4 — register the comps group so `dnf groupinstall
# mackes-desktop-environment` resolves on this host. Silently no-ops
# on systems where dnf-plugins-core isn't available.
dnf groups mark install mackes-desktop-environment 2>/dev/null || :

%preun
# Only on uninstall, not upgrade ($1 == 0 → final removal)
if [ "$1" = "0" ]; then
    /usr/share/%{name}/install-helpers/restore-xfce-settings.sh || :
fi

%files
%license LICENSE
%doc README.md CHANGELOG.md docs/MACKES_SHELL_SPEC.md docs/MIGRATION_FROM_V2.2.md
%{_bindir}/mackes
%{_bindir}/mackes-clipboard
%{_bindir}/mackes-enforce-session
%{_bindir}/mackes-gvfsd-mesh
%{_bindir}/mackes-mesh-open
%{_bindir}/mackes-panel
%{_bindir}/mackes-wm
%{_bindir}/mackesd
# CB-1 — Iced MDE Workbench preview (1.1.1+).
%{_bindir}/mde-workbench
%{_datadir}/applications/mde-workbench.desktop
# v2.0.0 Phase E.1 — Iced panel + Phase D.1 session orchestrator +
# Phase D.2 logout dialog + Phase A daemon binary + Phase E.8
# drawer applet + CB-1.10 wizard + 16 applet binaries.
%{_bindir}/mde-panel
%{_bindir}/mde-session
%{_bindir}/mde-logout-dialog
%{_bindir}/mded
%{_bindir}/mde-applet-drawer
%{_bindir}/mde-wizard
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
# v2.0.0 Phase 0.3 — mde-* binary wrappers + migrators.
%{_bindir}/mde
%{_bindir}/mde-wm
%{_bindir}/mde-enforce-session
%{_bindir}/mde-migrate-from-1x
%{_bindir}/mde-shell-migrate-v2
# v2.0.0 Phase 0.3 — man pages.
%{_mandir}/man1/mde.1*
%{_mandir}/man1/mde-migrate-from-1x.1*
%{_mandir}/man1/mde-shell-migrate-v2.1*
%{_mandir}/man8/mded.8*
# v2.0.0 Phase 0.4 — D-Bus service files (dev.mackes.MDE.* +
# legacy org.mackes.* aliases for one-release back-compat).
%{_datadir}/dbus-1/services/dev.mackes.MDE.*.service
%{_datadir}/dbus-1/services/org.mackes.*.service
# v2.0.0 Phase D.5 — sway config now covered by the
# %{_datadir}/%{name}/ catch-all below (Name: mde, so
# %{_datadir}/%{name}/ == /usr/share/mde/).
%{py3_sitelib}/mackes/
%{py3_sitelib}/mackes_shell-%{version}.dist-info/
%{_datadir}/%{name}/
%{_datadir}/applications/mackes-shell.desktop
%{_datadir}/applications/mackes-clipboard.desktop
%{_datadir}/applications/mackes-mesh-uri-handler.desktop
# v2.0.0 Phase 0.9 — MDE-namespaced .desktop + metainfo aliases.
%{_datadir}/applications/mde.desktop
%{_metainfodir}/dev.mackes.MDE.metainfo.xml
# v2.0.0 cut: XDG autostart overrides retired
# (mackes-panel.desktop, xfdesktop.desktop, mackes-enforce-session
# .desktop, mackes-suppress-xfce4-panel.desktop, kdeconnect-indicator
# .desktop). The Wayland session orchestrator (mde-session) and
# sway config handle panel + desktop bring-up natively.
%{_metainfodir}/io.github.matthewmackes.MackesShell.metainfo.xml
%{_metainfodir}/shell.mackes.Panel.metainfo.xml
%{_datadir}/gvfs/mounts/mesh.mount
%{_datadir}/icons/hicolor/scalable/apps/mackes-shell.svg
%{_datadir}/thumbnailers/mackes-mesh.thumbnailer
%{_unitdir}/mackes-node.service
%{_unitdir}/mackes-tailscale-bootstrap.service
%{_unitdir}/mackesd.service
# v12.16 Self-hosted DERP relay unit. Inactive on non-Host peers
# (gated by ConditionPathExists=/var/lib/mde/derper.enabled). The
# headscale DERP-map example shipped under
# %{_datadir}/%{name}/headscale/ is covered by the data catch-all.
%{_unitdir}/mde-derper.service
# v2.0.0 Phase B.13 retired 10 standalone systemd units (the 8
# named services + 3 paired .timer files); their roles now run
# inside `mackesd serve` as workers (Phase A.2 supervisor).
# v2.0.0 Phase D.6 mde-session.service ships as the user-session
# orchestrator (replaces mackes-enforce-session on the v2.0.0 line).
%{_userunitdir}/mde-session.service
%{_prefix}/lib/systemd/user-preset/90-mackes.preset
# CB-3.6 + CB-2.1 — v2.0.0 user preset + Wayland-session entry.
%{_prefix}/lib/systemd/user-preset/90-mde.preset
%{_datadir}/wayland-sessions/mde.desktop
# CB-2.4 — first-boot orchestration target + the two oneshot
# migrator units (mde-migrate-from-1x.service +
# mde-shell-migrate-v2.service).
%{_userunitdir}/mde-firstboot.target
%{_userunitdir}/mde-migrate-from-1x.service
%{_userunitdir}/mde-shell-migrate-v2.service
# CB-3.4 — comps group definition is under
# %{_datadir}/%{name}/comps/ which is now covered by the
# %{_datadir}/%{name}/ catch-all below.
%config(noreplace) /etc/sudoers.d/mackes-shell
# v2.0.0 cut: C panel plugins (mackes-clipboard, mackes-launcher,
# mackes-drawer) retired alongside xfce4-panel.  Their roles
# move to native mackes-panel applets in Phase E.1.x (Iced
# port). Until then, the desktop-applet feature set is reduced
# to the panel binary's built-in surfaces.
# Vendored GTK themes: Orchis-Dark (gtk-2/3/4 + xfwm) is the default;
# Shiki-Statler provides the classic xfwm4 window borders.
%{_datadir}/themes/Orchis-Dark/
%{_datadir}/themes/Shiki-Statler/
# Vendored Black-Sun icon theme (GPL-3.0, github.com/SethStormR/Black-Sun)
%{_datadir}/icons/Black-Sun/
# Mackes-Carbon — IBM Carbon Design System symbolic icons (Apache 2.0,
# github.com/carbon-design-system/carbon). Built from /tmp/carbon-icons
# via install-helpers/build-mackes-carbon.sh; freedesktop names mapped to
# Carbon basenames in install-helpers/mackes-carbon.map.
%{_datadir}/icons/Mackes-Carbon/
# Plymouth Mackes boot theme (theme files only; not activated at install
# time — activation happens in the wizard's birthright step)
%{_datadir}/plymouth/themes/mackes/

%files xorg
%{_bindir}/mde-session-xorg
%{_bindir}/mde-xorg-session
%{_datadir}/xsessions/mde-xorg.desktop
%{_datadir}/mde-xorg/
%{_userunitdir}/mde-xorg.target

%changelog
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
