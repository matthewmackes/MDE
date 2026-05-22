#!/bin/sh
# mde-installer-complete — wizard success-path cleanup.
#
# Invoked by mde-installer-complete.service after the wizard
# exits 0. Job:
#   1. Restore graphical.target as the default boot target.
#   2. Remove the firstboot flag so the installer.target won't
#      re-fire next boot.
#   3. Clear any /etc/motd installer banner (set by the abort
#      path on a previous decline).
#   4. systemctl reboot so the user lands in MDE.

set -eu

FLAG=/var/lib/mde/installer/firstboot.needed
MOTD=/etc/motd.d/00-mde-installer
LOG=/var/log/mde/installer-complete.log

mkdir -p /var/log/mde
exec >> "$LOG" 2>&1
printf '\n=== mde-installer-complete %s ===\n' "$(date -Iseconds)"

# 1. Restore graphical.target as default.
echo "restoring graphical.target as default"
systemctl set-default graphical.target

# 2. Clear the flag.
if [ -e "$FLAG" ]; then
    echo "removing $FLAG"
    rm -f "$FLAG"
fi

# 3. Clear any motd banner from a previous abort.
if [ -e "$MOTD" ]; then
    echo "clearing motd banner $MOTD"
    rm -f "$MOTD"
fi

# 4. Hand off to systemd for the reboot. The wizard already
#    showed the "rebooting into MDE" message before invoking us.
echo "rebooting into graphical.target"
systemctl reboot
