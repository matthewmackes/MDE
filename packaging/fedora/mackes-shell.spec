# Disable the auto-generated debuginfo / debugsource subpackages. Our C
# panel plugin is tiny (~60 KB stripped) and we don't ship separate
# debug builds. Avoids "Empty %files file debugsourcefiles.list" errors
# from rpmbuild on Fedora 40+.
%global debug_package %{nil}

Name:           mackes-shell
Version:        1.5.0
Release:        1%{?dist}
Summary:        Mackes Shell — XFCE control panel and shell manager for Fedora

License:        GPL-3.0
URL:            https://github.com/matthewmackes/MAP2-RELEASES
Source0:        %{name}-%{version}.tar.gz

# Arch-specific (was BuildArch:noarch in 0.x): the package now carries a
# compiled C xfce4-panel external plugin under %{_libdir}/xfce4/panel/
# plugins/mackes-clipboard. The rest is pure-Python + data files.
ExclusiveArch:  %{ix86} x86_64 aarch64

# Build dependencies — python3 for site-packages discovery + C toolchain
# for the panel plugin.
BuildRequires:  python3
# C panel-plugin compile-time deps
BuildRequires:  gcc
BuildRequires:  make
BuildRequires:  pkgconfig
BuildRequires:  gtk3-devel
BuildRequires:  xfce4-panel-devel
BuildRequires:  libxfce4ui-devel

# Runtime deps the entry point and all panels need
Requires:       python3
Requires:       python3-gobject
Requires:       gtk3
Requires:       python3-pyyaml
Requires:       xfconf
Requires:       xfce4-settings
Requires:       xfce4-session

# XFCE shell pieces required by the standard layout (Q19 lock)
Requires:       xfce4-whiskermenu-plugin
Requires:       xfce4-docklike-plugin
Requires:       xfce4-pulseaudio-plugin
# Note: on Fedora 44+ the power-manager panel plugin ships inside the
# parent xfce4-power-manager package (libxfce4powermanager.so); there
# is no separate xfce4-power-manager-plugin RPM.
Requires:       xfce4-power-manager

# SSH enabled by default on every Mackes install
Requires:       openssh-server

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

# Always-maximize windows (v1.4.1 birthright) — mackes-maximizer is a
# user-level service that listens for new top-level windows and adds
# maximized_vert + maximized_horz via wmctrl. Toggleable via Tweaks.
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

# Typography defaults — Carbon Design System (IBM Plex)
Recommends:     ibm-plex-sans-fonts
Recommends:     ibm-plex-mono-fonts

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
# Conky HUD (v1.4.0) — config template + helper scripts
cp -r data/conky          %{buildroot}%{_datadir}/%{name}/data/
chmod 0755 %{buildroot}%{_datadir}/%{name}/data/conky/helpers/*.sh
# Mackes Conky autostart .desktop
install -D -m 0644 data/applications/mackes-conky.desktop \
    %{buildroot}%{_datadir}/applications/mackes-conky.desktop
cp -r data/systemd        %{buildroot}%{_datadir}/%{name}/data/
# Vendored PadOS GTK theme + Carbon GTK icon theme — system-wide install
install -d %{buildroot}%{_datadir}/themes
cp -r data/themes/PadOS   %{buildroot}%{_datadir}/themes/
install -d %{buildroot}%{_datadir}/icons
cp -r data/icons/Carbon   %{buildroot}%{_datadir}/icons/
# Plymouth Mackes boot theme — installed but NOT activated at %post; the
# wizard's birthright step (mackes.birthright.apply_plymouth) activates it
# only when the user opts in (initrd rebuild is heavy and disruptive).
install -d %{buildroot}%{_datadir}/plymouth/themes
cp -r data/plymouth/mackes %{buildroot}%{_datadir}/plymouth/themes/
# C plugin install (compiled in %%build above). Pass libdir explicitly
# so on 64-bit the binary lands at %{_libdir}=/usr/lib64, not /usr/lib.
make -C data/panel-plugins/mackes-clipboard install \
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

# 4. Install helper scripts (called from %%post / %%preun, plus the
#    capture/apply pair admins use to manage XFCE baselines, plus the
#    theme/icon bootstrap, lightdm config, and mackes-user creation)
install -d %{buildroot}%{_datadir}/%{name}/install-helpers
for helper in \
    hide-xfce-settings.sh restore-xfce-settings.sh \
    add-mackes-repo.sh install-recovery.sh \
    capture-xfce-baseline.sh apply-xfce-baseline.sh \
    configure-lightdm.sh create-mackes-user.sh \
    install-carbon-icons.sh \
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
install -m 0644 data/systemd/mackes-mdns-relay.service       %{buildroot}%{_unitdir}/
# Fleet management (v1.3.0) — ansible-pull timer + service
install -m 0644 data/systemd/mackes-ansible-pull.service     %{buildroot}%{_unitdir}/
install -m 0644 data/systemd/mackes-ansible-pull.timer       %{buildroot}%{_unitdir}/
# Always-maximize windows (v1.4.1) — user-level systemd unit
install -d %{buildroot}%{_userunitdir}
install -m 0644 data/systemd/mackes-maximizer.service        %{buildroot}%{_userunitdir}/
# Mesh clipboard daemon (v1.5.0) — XA_CLIPBOARD watcher
install -m 0644 data/systemd/mackes-clipboard-daemon.service %{buildroot}%{_userunitdir}/
# Sudoers drop-in (v1.4.1) — grants NOPASSWD on Mackes-managed commands
install -D -m 0440 data/sudoers.d/mackes-shell               %{buildroot}/etc/sudoers.d/mackes-shell
# Maximizer binary
install -D -m 0755 bin/mackes-maximizer                       %{buildroot}%{_bindir}/mackes-maximizer
# Maximizer autostart .desktop
install -D -m 0644 data/applications/mackes-maximizer.desktop \
    %{buildroot}%{_datadir}/applications/mackes-maximizer.desktop
# headscale.service is owned by the upstream `headscale` RPM at the same
# path; shipping our copy would cause a file-conflict on dnf install.
# Our data/systemd/headscale.service is kept in the source tree as a
# reference but not installed. To customize (MemoryHigh, etc.), drop a
# systemd drop-in at /etc/systemd/system/headscale.service.d/mackes.conf.
install -d %{buildroot}%{_userunitdir}
install -m 0644 data/systemd/mackes-gvfsd-mesh.service       %{buildroot}%{_userunitdir}/

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
install -D -m 0644 data/applications/mackes-mesh-uri-handler.desktop \
    %{buildroot}%{_datadir}/applications/mackes-mesh-uri-handler.desktop
install -D -m 0644 data/icons/mackes-shell.svg \
    %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/mackes-shell.svg

# 6. /usr/bin/* wrappers
install -d %{buildroot}%{_bindir}
cat > %{buildroot}%{_bindir}/mackes <<'EOF'
#!/usr/bin/env bash
exec python3 -m mackes "$@"
EOF
chmod 0755 %{buildroot}%{_bindir}/mackes
cat > %{buildroot}%{_bindir}/mackes-clipboard <<'EOF'
#!/usr/bin/env bash
exec python3 -m mackes.clipboard_app "$@"
EOF
chmod 0755 %{buildroot}%{_bindir}/mackes-clipboard
# GVFS-mesh wrappers
install -m 0755 bin/mackes-gvfsd-mesh %{buildroot}%{_bindir}/mackes-gvfsd-mesh
install -m 0755 bin/mackes-mesh-open  %{buildroot}%{_bindir}/mackes-mesh-open

%post
/usr/share/%{name}/install-helpers/create-mackes-user.sh || :
/usr/share/%{name}/install-helpers/hide-xfce-settings.sh || :
# Enable SSH by default on every Mackes install (per design lock).
systemctl enable --now sshd.service || :
# Refresh systemd unit cache so the new mackes-* units are visible.
systemctl daemon-reload || :
# Validate the sudoers drop-in we shipped; on failure remove it so we
# never break the host's sudo behavior.
visudo -c -f /etc/sudoers.d/mackes-shell >/dev/null 2>&1 \
    || rm -f /etc/sudoers.d/mackes-shell
# Rebuild the GTK icon cache for the vendored Carbon icon theme
gtk-update-icon-cache -f -t %{_datadir}/icons/Carbon 2>/dev/null || :

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
%{_bindir}/mackes-gvfsd-mesh
%{_bindir}/mackes-mesh-open
%{_bindir}/mackes-maximizer
%{py3_sitelib}/mackes/
%{py3_sitelib}/mackes_shell-%{version}.dist-info/
%{_datadir}/%{name}/
%{_datadir}/applications/mackes-shell.desktop
%{_datadir}/applications/mackes-clipboard.desktop
%{_datadir}/applications/mackes-conky.desktop
%{_datadir}/applications/mackes-maximizer.desktop
%{_datadir}/applications/mackes-mesh-uri-handler.desktop
%{_datadir}/gvfs/mounts/mesh.mount
%{_datadir}/icons/hicolor/scalable/apps/mackes-shell.svg
%{_datadir}/thumbnailers/mackes-mesh.thumbnailer
%{_unitdir}/mackes-node.service
%{_unitdir}/mackes-tailscale-bootstrap.service
%{_unitdir}/mackes-mdns-relay.service
%{_unitdir}/mackes-ansible-pull.service
%{_unitdir}/mackes-ansible-pull.timer
%{_userunitdir}/mackes-gvfsd-mesh.service
%{_userunitdir}/mackes-maximizer.service
%{_userunitdir}/mackes-clipboard-daemon.service
%config(noreplace) /etc/sudoers.d/mackes-shell
# C panel plugin + its descriptor
%{_libdir}/xfce4/panel/plugins/mackes-clipboard
%{_datadir}/xfce4/panel/plugins/mackes-clipboard.desktop
# Vendored PadOS GTK theme
%{_datadir}/themes/PadOS/
# Vendored Carbon GTK icon theme
%{_datadir}/icons/Carbon/
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
