#!/bin/sh
# install-helpers/lint-dbus-shape.sh — pre-commit gate #8
# (added 2026-05-25 per Q12 + Q20 + Q96 + EPIC-PROC-LINT of the
# 100-Q tightening survey).
#
# Catches NET-NEW D-Bus method declarations on MDE-internal
# services. Per Q20 + Q96 the canonical IPC for MDE-internal
# control + events is **Bus** (action/<domain>/<verb> for
# commands, reply/<ulid> for responses, domain topics for
# events); D-Bus retires entirely by 1.0 except for FDO
# interop (org.freedesktop.* surfaces).
#
# This gate flags any net-new `#[interface]` block (zbus
# interface declaration) in `crates/mackesd/src/ipc/` or
# any new `dbus_macros::dbus_interface` macro, EXCEPT inside
# the FDO-interop allow-list.
#
# Allow-list:
# - `crates/mded/src/fdo_*.rs` — `org.freedesktop.Notifications`
#   bridge per BUS-4.4 (and any future FDO interop surfaces)
# - `crates/mde-kdc*/src/dbus*.rs` — KDC2 KDE Connect uses D-Bus
#   for the per-app dispatch layer; that's a vendor protocol,
#   not MDE-internal
# - Existing services in `crates/mackesd/src/ipc/{nebula,gluster,
#   portal,notifications,shell}*.rs` — pre-existing surface
#   awaiting EPIC-RETIRE-DBUS migration; new methods on those
#   are TOLERATED until migration ships
#
# Per `.claude/CLAUDE.md` §0.7 gate #8.
#
# Exits 0 = clean, exits 1 = net-new MDE-internal D-Bus surface.

set -eu

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

SCAN_INCLUDES='--include=*.rs'
SCAN_PATHS='crates/'

# D-Bus interface markers
DBUS_PATTERNS='#\[interface\b|dbus_macros::dbus_interface|#\[zbus::interface'

# Allow-listed path prefixes — existing services + FDO interop.
# Snapshot taken 2026-05-25 of every file with a current
# `#[interface]` declaration; everything outside is net-new.
# As EPIC-RETIRE-DBUS migrates a service to Bus, REMOVE its
# entry from this allow-list — that way the lint catches
# any regression.
ALLOWED_PREFIXES='
crates/mded/src/fdo_
crates/mde-kdc/src/dbus
crates/mde-kdc-proto/
crates/mackesd/src/ipc/nebula
crates/mackesd/src/ipc/gluster
crates/mackesd/src/ipc/portal
crates/mackesd/src/ipc/notifications
crates/mackesd/src/ipc/shell
crates/mackesd/src/ipc/healthz
crates/mackesd/src/ipc/session
crates/mackesd/src/ipc/fleet
crates/mackesd/src/ipc/settings
crates/mackesd/src/ipc/files
crates/mde-session/src/session.rs
crates/mde-workbench/src/dbus.rs
crates/mde-portal/src/portal_full_main.rs
crates/mde-portal/src/dbus.rs
'

# Comment-line allow-list (talking ABOUT D-Bus, not declaring it).
# Pattern matches AFTER the `file:line:` prefix grep -n inserts —
# anchoring on `^` here would catch the file path, not the source.
COMMENT_PREFIXES=':[0-9]+:[[:space:]]*(///|//!|//|#|/\*|\*)'

# Build the grep allow-list filter
ALLOW_FILTER=""
for prefix in $ALLOWED_PREFIXES; do
  [ -z "$prefix" ] && continue
  ALLOW_FILTER="${ALLOW_FILTER}|^${prefix}"
done
ALLOW_FILTER="${ALLOW_FILTER#|}"

violations=$(
  grep -rn -E "$DBUS_PATTERNS" $SCAN_INCLUDES $SCAN_PATHS 2>/dev/null \
    | grep -vE "$ALLOW_FILTER" \
    | grep -vE "$COMMENT_PREFIXES" \
    || true
)

if [ -n "$violations" ]; then
  echo "$0: net-new MDE-internal D-Bus interface declarations (Q20 + Q96):"
  echo "$violations"
  echo ""
  echo "Per the 100-Q survey Q20 + Q96, Bus replaces D-Bus for MDE-internal"
  echo "IPC by 1.0. New commands should publish to action/<domain>/<verb>;"
  echo "new responses should subscribe to reply/<original-ulid>; new events"
  echo "should publish to domain topics. FDO interop (org.freedesktop.*)"
  echo "stays on D-Bus and is allow-listed in this script."
  exit 1
fi

echo "$0: no net-new MDE-internal D-Bus interfaces — clean."
exit 0
