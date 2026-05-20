# Disable the auto-generated debuginfo / debugsource subpackages. Our C
# panel plugin is tiny (~60 KB stripped) and we don't ship separate
# debug builds. Avoids "Empty %files file debugsourcefiles.list" errors
# from rpmbuild on Fedora 40+.
%global debug_package %{nil}

Name:           mackes-xfce-workstation
Version:        1.1.0
Release:        1%{?dist}
Summary:        Mackes XFCE Workstation — unified shell, panel, dock, and mesh for Fedora

License:        GPL-3.0
URL:            https://github.com/matthewmackes/MAP2-RELEASES
# Source tarball still ships under the legacy name so dist/mackes-shell-...
# keeps working; the package itself is renamed via Provides/Obsoletes.
Source0:        mackes-shell-%{version}.tar.gz

# Phase 10.1 rename: this package supersedes the legacy mackes-shell
# package. dnf upgrade auto-replaces installations on the 2.x train.
Provides:       mackes-shell = %{version}-%{release}
Obsoletes:      mackes-shell < 3.0

# v2.0.0 Phase 0.8 + H.3 — MDE rebrand. The package now also
# advertises itself as `mde` so a new `dnf install mde` resolves
# to this RPM, and `dnf upgrade` on a future explicit `mde` cut
# (when the spec rename to `Name: mde` lands) replaces the
# mackes-xfce-workstation row cleanly.
Provides:       mde = %{version}-%{release}

# Phase 10.6.6 — mackes-panel (Rust) fully replaces the legacy XFCE
# panel + desktop + plugin stack. Listing them as Obsoletes means
# `dnf install mackes-xfce-workstation` removes them cleanly on
# upgrade boxes, paralleling the apply_uninstall_legacy_xfce
# birthright step (which handles the runtime/on-disk cleanup for
# already-installed nodes). The `< 999` upper-bound silences the
# rpmlint unversioned-Obsoletes warning while still covering every
# real-world version (current xfce4-panel is 4.20.x).
Obsoletes:      xfce4-panel < 999
Obsoletes:      xfdesktop < 999
Obsoletes:      xfce4-whiskermenu-plugin < 999
Obsoletes:      xfce4-docklike-plugin < 999
Obsoletes:      xfce4-pulseaudio-plugin < 999
Obsoletes:      xfce4-power-manager-plugin < 999

# Arch-specific (was BuildArch:noarch in 0.x): the package now carries a
# compiled C xfce4-panel external plugin under %{_libdir}/xfce4/panel/
# plugins/mackes-clipboard. The rest is pure-Python + data files.
ExclusiveArch:  %{ix86} x86_64 aarch64

# Build dependencies — python3 for site-packages discovery + C toolchain
# for the panel plugin + Rust toolchain for the mackes-panel binary
# (Mackes XFCE Workstation 1.0.0 rewrite, Phase 0.3).
BuildRequires:  python3
# C panel-plugin compile-time deps
BuildRequires:  gcc
BuildRequires:  make
BuildRequires:  pkgconfig
BuildRequires:  gtk3-devel
BuildRequires:  xfce4-panel-devel
BuildRequires:  libxfce4ui-devel
# Rust toolchain for crates/mackes-panel + workspace crates
BuildRequires:  rust
BuildRequires:  cargo

# Runtime deps the entry point and all panels need
Requires:       python3
Requires:       python3-gobject
Requires:       gtk3
Requires:       python3-pyyaml
Requires:       xfconf
Requires:       xfce4-settings
Requires:       xfce4-session

# 1.0.7 (Phase 8.8) — i3 is the only window manager. xfwm4 is
# fully replaced. The XFCE session host stays (xfsettingsd,
# xfce4-power-manager, thunar, xfconf above) — i3 just owns the
# WM role inside the XFCE session. mackes-maximizer is retired
# (it existed only as an xfwm4 crutch; i3 tiles natively).
Requires:       i3
Requires:       i3status
Requires:       dmenu

# Phase 10.6.6 (1.1.0) — xfce4-panel-profiles + the three xfce4-panel
# plugins are NOT required any more: mackes-panel replaces xfce4-panel
# entirely, and the four Obsoletes above mean dnf will refuse to pull
# them back in. apply_panel_layout (birthright) skips itself gracefully
# when xfce4-panel-profiles isn't installed; the wizard's legacy
# panel-archive step preserves any pre-1.0 layout for reference.
#
# 1.1.0 — Right-click Start menu spawns the comprehensive Fedora admin
# action set (Root Terminal / DNF / journalctl / systemctl / SELinux /
# firewall / sudoedit / disk-clean) inside a terminator window with
# the shell kept open after the command finishes (Q15/Q16 lock).
Requires:       terminator

# Phase 13.1.1 — KDE Connect Option A integration. Mackes ships its
# own Workbench GUI talking `org.kde.kdeconnect.*` DBus + a mesh-mDNS
# bridge so remote phones feel local. The upstream `kdeconnectd`
# daemon stays user-session-autostarted (handled by upstream's own
# `.desktop`); only its tray indicator is suppressed below.
Requires:       kdeconnectd
# Note: on Fedora 44+ the power-manager panel plugin ships inside the
# parent xfce4-power-manager package (libxfce4powermanager.so); there
# is no separate xfce4-power-manager-plugin RPM.
Requires:       xfce4-power-manager

# SSH enabled by default on every Mackes install
Requires:       openssh-server

# Phase 5.2 — mackes-panel shells out to `wmctrl -l` for open-window
# enumeration (running-app dock indicators). wmctrl is a 50 KB tool
# and ships in every Fedora repo.
Requires:       wmctrl

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
# v1.6.2 — per-monitor HUD placement uses xrandr; click-through uses
# xdotool to find the conky window before clearing its SHAPE input.
# Both degrade gracefully when absent.
Recommends:     xorg-x11-server-utils
Recommends:     xdotool

# wmctrl + xprop still needed for the panel's window enumeration
# (dock tasklist, EWMH strut publishing). The mackes-maximizer
# service that previously consumed them is retired in Phase 8.8.
Requires:       wmctrl
Requires:       xprop

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
Mackes Shell is the single GTK control panel that replaces xfce4-settings
as the daily interface for managing an XFCE-based Fedora workstation. One
window, a dashboard, task tabs (Look & Feel, Devices, Network, System,
Apps, Maintain), and a first-run wizard that brings a fresh machine to a
known preset in under five minutes. Standard XFCE shell underneath (Whisker
Menu + xfce4-panel + xfdesktop), styled with the Carbon Design System.

%prep
# sdist generated by `python -m build` unpacks to mackes_shell-<version>/
# (PyPI canonical underscore name), not mackes-shell-<version>/.
%autosetup -n mackes_shell-%{version}

%build
# Pure Python — except for the xfce4-panel external clipboard plugin (C).
make -C data/panel-plugins/mackes-clipboard CFLAGS="%{optflags}"
make -C data/panel-plugins/mackes-launcher  CFLAGS="%{optflags}"
make -C data/panel-plugins/mackes-drawer    CFLAGS="%{optflags}"

# Rust workspace — mackes-panel binary + library crates (Phase 0.3).
# Offline=false because we let Cargo resolve crates.io deps; the build
# environment is expected to have network. If/when we vendor, drop in
# `--offline` and a vendored target dir.
cargo build --release --workspace

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
# C plugin install (compiled in %%build above). Pass libdir explicitly
# so on 64-bit the binary lands at %{_libdir}=/usr/lib64, not /usr/lib.
make -C data/panel-plugins/mackes-drawer install \
    DESTDIR=%{buildroot} prefix=%{_prefix} libdir=%{_libdir} datadir=%{_datadir}
make -C data/panel-plugins/mackes-clipboard install \
    DESTDIR=%{buildroot} prefix=%{_prefix} libdir=%{_libdir}
make -C data/panel-plugins/mackes-launcher install \
    DESTDIR=%{buildroot} prefix=%{_prefix} libdir=%{_libdir}
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

# 5a. mackes-panel autostart (Phase 8.3). Drops into /etc/xdg/autostart so
# every XFCE session brings up the new Rust panel. NoDisplay=true keeps
# it out of menus — the panel runs implicitly.
install -D -m 0644 data/applications/mackes-panel.desktop \
    %{buildroot}%{_sysconfdir}/xdg/autostart/mackes-panel.desktop

# 5b. Disable xfdesktop autostart on Mackes installs — mackes-panel owns
# the wallpaper + root-window roles (Q39/Q40). We don't uninstall the
# xfdesktop package (still a recommends from xfce4-session), but we
# override its autostart with Hidden=true.
install -d %{buildroot}%{_sysconfdir}/xdg/autostart
cat > %{buildroot}%{_sysconfdir}/xdg/autostart/xfdesktop.desktop <<'XFDESKTOP_EOF'
[Desktop Entry]
Type=Application
Name=Desktop Manager (disabled by Mackes)
Comment=mackes-panel owns the wallpaper and root-window roles on Mackes installs.
Exec=true
Hidden=true
NoDisplay=true
X-XFCE-Autostart-enabled=false
X-GNOME-Autostart-enabled=false
XFDESKTOP_EOF
chmod 0644 %{buildroot}%{_sysconfdir}/xdg/autostart/xfdesktop.desktop
# v1.6.2 — tray icon autostart (Q8 lock: panel + tray + hotkey)
# v2.2.0 — mackes-tray.desktop removed (tray replaced by the Notification
# Drawer panel applet).
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
cat > %{buildroot}%{_bindir}/mackes-clipboard <<'EOF'
#!/usr/bin/env bash
exec python3 -m mackes.clipboard_app "$@"
EOF
chmod 0755 %{buildroot}%{_bindir}/mackes-clipboard
# GVFS-mesh wrappers
install -m 0755 bin/mackes-gvfsd-mesh %{buildroot}%{_bindir}/mackes-gvfsd-mesh
install -m 0755 bin/mackes-mesh-open  %{buildroot}%{_bindir}/mackes-mesh-open

# 6b. mackes-enforce-session — 1.0.8 hotfix. Runs from
# /etc/xdg/autostart at every login and idempotently converges the
# session onto i3 + mackes-panel (no xfwm4, no xfce4-panel, no
# xfdesktop). Needed because xfce4-session's Failsafe template still
# starts the legacy stack, and we don't ship a system xfce4-session.xml
# override (RPM file conflict with the xfce4-session package).
install -m 0755 bin/mackes-enforce-session \
    %{buildroot}%{_bindir}/mackes-enforce-session
install -D -m 0644 data/applications/mackes-enforce-session.desktop \
    %{buildroot}%{_sysconfdir}/xdg/autostart/mackes-enforce-session.desktop

# 6c-bis. Phase 13.1.1 — kdeconnect-indicator autostart override.
# Mackes ships its own Workbench Connect GUI, so the upstream tray
# indicator would double up next to it. The daemon (`kdeconnectd`)
# stays autostarted by its own .desktop; only the indicator is
# suppressed.
install -d %{buildroot}%{_sysconfdir}/xdg/autostart
cat > %{buildroot}%{_sysconfdir}/xdg/autostart/kdeconnect-indicator.desktop <<'KDC_EOF'
[Desktop Entry]
Type=Application
Name=KDE Connect indicator (disabled by Mackes)
Comment=Mackes Workbench Connect provides the native UI.
Exec=true
Hidden=true
NoDisplay=true
X-XFCE-Autostart-enabled=false
X-GNOME-Autostart-enabled=false
KDC_EOF
chmod 0644 %{buildroot}%{_sysconfdir}/xdg/autostart/kdeconnect-indicator.desktop

# 6c. xfce4-panel autostart override — Hidden=true, mirrors the
# xfdesktop override at line ~400. xfce4-panel ships its own
# /etc/xdg/autostart/xfce4-panel.desktop, so we install ours at a
# Mackes-prefixed filename and rely on mackes-enforce-session to kill
# any xfce4-panel that xfce4-session started directly. Belt-and-braces
# for the XDG-autostart spawn path.
cat > %{buildroot}%{_sysconfdir}/xdg/autostart/mackes-suppress-xfce4-panel.desktop <<'XFP_EOF'
[Desktop Entry]
Type=Application
Name=xfce4-panel suppressor (Mackes)
Comment=mackes-panel replaces xfce4-panel on Mackes installs.
Exec=true
Hidden=true
NoDisplay=true
X-XFCE-Autostart-enabled=false
X-GNOME-Autostart-enabled=false
XFP_EOF
chmod 0644 %{buildroot}%{_sysconfdir}/xdg/autostart/mackes-suppress-xfce4-panel.desktop

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
# v2.0.0 Phase D.5 — sway config.
%{_datadir}/mde/sway/config
%{_datadir}/mde/sway/config.d/mackes-defaults.conf
%{py3_sitelib}/mackes/
%{py3_sitelib}/mackes_shell-%{version}.dist-info/
%{_datadir}/%{name}/
%{_datadir}/applications/mackes-shell.desktop
%{_datadir}/applications/mackes-clipboard.desktop
%{_datadir}/applications/mackes-mesh-uri-handler.desktop
# v2.0.0 Phase 0.9 — MDE-namespaced .desktop + metainfo aliases.
%{_datadir}/applications/mde.desktop
%{_metainfodir}/dev.mackes.MDE.metainfo.xml
# Phase 8.3 — autostart entries that bring up mackes-panel and override
# xfdesktop on Mackes installs (Q39/Q40). The mackes-enforce-session
# entry (1.0.8 hotfix) idempotently swaps xfwm4 → i3 and quits any
# xfce4-panel/xfdesktop that xfce4-session spawned from its Failsafe
# client list.
%config %{_sysconfdir}/xdg/autostart/mackes-panel.desktop
%config %{_sysconfdir}/xdg/autostart/xfdesktop.desktop
%config %{_sysconfdir}/xdg/autostart/mackes-enforce-session.desktop
%config %{_sysconfdir}/xdg/autostart/mackes-suppress-xfce4-panel.desktop
%config %{_sysconfdir}/xdg/autostart/kdeconnect-indicator.desktop
%{_metainfodir}/io.github.matthewmackes.MackesShell.metainfo.xml
%{_metainfodir}/shell.mackes.Panel.metainfo.xml
%{_datadir}/gvfs/mounts/mesh.mount
%{_datadir}/icons/hicolor/scalable/apps/mackes-shell.svg
%{_datadir}/thumbnailers/mackes-mesh.thumbnailer
%{_unitdir}/mackes-node.service
%{_unitdir}/mackes-tailscale-bootstrap.service
%{_unitdir}/mackesd.service
# v2.0.0 Phase B.13 retired 10 standalone systemd units (the 8
# named services + 3 paired .timer files); their roles now run
# inside `mackesd serve` as workers (Phase A.2 supervisor).
# v2.0.0 Phase D.6 mde-session.service ships as the user-session
# orchestrator (replaces mackes-enforce-session on the v2.0.0 line).
%{_userunitdir}/mde-session.service
%{_prefix}/lib/systemd/user-preset/90-mackes.preset
%config(noreplace) /etc/sudoers.d/mackes-shell
# C panel plugins + their descriptors
%{_libdir}/xfce4/panel/plugins/mackes-clipboard
%{_datadir}/xfce4/panel/plugins/mackes-clipboard.desktop
# v1.6.2 — Mackes launcher (Super+M → mackes --drawer)
%{_libdir}/xfce4/panel/plugins/mackes-launcher
%{_datadir}/xfce4/panel/plugins/mackes-launcher.desktop
# v2.2.0 — Notification Drawer pill (replaces Conky HUD + tray + popover)
%{_libdir}/xfce4/panel/plugins/mackes-drawer
%{_datadir}/xfce4/panel/plugins/mackes-drawer.desktop
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

%changelog
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
