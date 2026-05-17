#!/usr/bin/env bash
# Create the `mackes` system user/group used by mackes-node.service.
# Called from the RPM %post scriptlet. Idempotent.
set -euo pipefail

if ! getent group mackes >/dev/null 2>&1; then
    groupadd --system mackes
fi

if ! getent passwd mackes >/dev/null 2>&1; then
    useradd --system \
        --gid mackes \
        --home-dir /var/lib/mackes \
        --create-home \
        --shell /usr/sbin/nologin \
        --comment "Mackes Shell mesh-node daemon" \
        mackes
fi

# Ensure fuse group membership (for sshfs)
if getent group fuse >/dev/null 2>&1; then
    usermod -aG fuse mackes
fi

install -d -m 0755 -o mackes -g mackes /var/lib/mackes
install -d -m 0700 -o mackes -g mackes /var/lib/mackes/.ssh
install -d -m 0755 -o mackes -g mackes /var/lib/mackes/QNM-Shared
install -d -m 0755 -o mackes -g mackes /var/lib/mackes/QNM-Mesh

# Make /var/lib/headscale exist for headscale.service
install -d -m 0750 /var/lib/headscale

echo "mackes user/group ready"
