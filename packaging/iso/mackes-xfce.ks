# Mackes XFCE — kickstart for a custom Fedora live ISO.
#
# Build (on a Fedora host):
#
#     sudo dnf install -y lorax pykickstart
#     sudo livemedia-creator \
#         --make-iso \
#         --ks ./packaging/iso/mackes-xfce.ks \
#         --no-virt \
#         --resultdir ./dist/iso \
#         --project "Mackes XFCE" --releasever "$(rpm -E %fedora)" \
#         --volid "MACKES_XFCE"
#
# Result lands in ./dist/iso/<...>.iso. ~20-30 min on modern hardware.
#
# The OEM-mode wizard is wired via the post-install %post section, which
# drops a /etc/skel/.config/mackes-shell/state.json marking the install as
# un-provisioned so the wizard runs on first login.

lang en_US.UTF-8
keyboard us
timezone --utc Etc/UTC
selinux --enforcing
firewall --enabled --service=mdns
services --enabled=NetworkManager,sshd
rootpw --plaintext mackes
auth --useshadow --passalgo=sha512
firstboot --disable

bootloader --location=mbr --append="rhgb quiet"
zerombr
clearpart --all --initlabel
part / --size=8192 --grow --asprimary --fstype=ext4

# ---- Base + XFCE -------------------------------------------------------
url --mirrorlist=https://mirrors.fedoraproject.org/mirrorlist?repo=fedora-$releasever&arch=$basearch

repo --name=updates --mirrorlist=https://mirrors.fedoraproject.org/mirrorlist?repo=updates-released-f$releasever&arch=$basearch

# Mackes Shell repo (gh-pages — see data/dnf/mackes-shell.repo)
repo --name=mackes-shell \
    --baseurl=https://matthewmackes.github.io/MAP2-RELEASES/fedora/$releasever/$basearch \
    --includepkgs=mackes-shell

%packages
@core
@base-x
@xfce-desktop-environment
NetworkManager-wifi
NetworkManager-vpnc
NetworkManager-openvpn
firewalld
xfce4-power-manager
xfce4-power-manager-plugin
xfce4-pulseaudio-plugin
xfce4-whiskermenu-plugin
xfce4-docklike-plugin
xfce4-clipman-plugin
xfce4-notifyd
openssh-server
menulibre
dnf-plugins-core
flatpak
mackes-shell
# Typography — Carbon Design System (IBM Plex)
ibm-plex-sans-fonts
ibm-plex-mono-fonts
%end

# ---- Post-install ------------------------------------------------------
%post

# Make sure the Mackes Shell wizard runs on first login of any user.
mkdir -p /etc/skel/.config/mackes-shell
cat > /etc/skel/.config/mackes-shell/state.json <<'EOF'
{
  "provisioned": false,
  "active_preset": null,
  "schema_version": 1
}
EOF

# Add the Mackes dnf repo for future upgrades.
if [ -x /usr/share/mackes-shell/install-helpers/add-mackes-repo.sh ]; then
    /usr/share/mackes-shell/install-helpers/add-mackes-repo.sh || true
fi

# Wire the recovery boot entry (idempotent on re-runs).
if [ -x /usr/share/mackes-shell/install-helpers/install-recovery.sh ]; then
    /usr/share/mackes-shell/install-helpers/install-recovery.sh || true
fi

# Branding hook — install the standard Mackes wallpaper as the system-wide
# default backdrop. Used by the LightDM greeter and the first-run wizard.
if [ -f /usr/share/mackes-shell/branding/standard-wallpaper.png ]; then
    install -D -m 0644 /usr/share/mackes-shell/branding/standard-wallpaper.png \
        /usr/share/backgrounds/mackes-default.png
fi

%end
