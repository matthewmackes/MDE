#!/usr/bin/env bash
# register-gvfs-mesh.sh — register the mesh:// URI scheme with GVFS +
# install the user systemd unit + start the FUSE daemon for this user.
#
# Idempotent. Run once per user (the wizard / `mackes init` calls it).

set -euo pipefail

# 1. Make sure the FUSE mount point exists
MOUNT_POINT="$HOME/.local/share/mackes-mesh-fuse"
mkdir -p "$MOUNT_POINT"

# 2. Register the URI scheme so `xdg-open mesh:///` works
if command -v xdg-mime >/dev/null 2>&1; then
    xdg-mime default mackes-mesh-uri-handler.desktop x-scheme-handler/mesh || true
fi

# 3. Install the user systemd unit (system unit already in /usr/lib/...)
USER_UNIT_DIR="$HOME/.config/systemd/user"
mkdir -p "$USER_UNIT_DIR"
SRC="/usr/share/mackes-shell/data/systemd/mackes-gvfsd-mesh.service"
[[ -f "$SRC" ]] || SRC="$(dirname "$0")/../data/systemd/mackes-gvfsd-mesh.service"

if [[ -f "$SRC" ]]; then
    ln -sf "$SRC" "$USER_UNIT_DIR/mackes-gvfsd-mesh.service"
fi

# 4. Reload user systemd + enable
systemctl --user daemon-reload || true
systemctl --user enable --now mackes-gvfsd-mesh.service || true

echo "mesh:// GVFS surface registered for $USER"
echo "  mount point:  $MOUNT_POINT"
echo "  systemctl --user status mackes-gvfsd-mesh.service"
