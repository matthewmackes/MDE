#!/usr/bin/env bash
# NF-9.3 helper — simulate a LAN-cable replug by toggling the link.
#
# Drops the named interface for 2 s then brings it back up. Used by
# tests/acceptance/test_nebula_fabric.py to assert the
# LinkWatchWorker's reconnect SLO (under 5 s).
#
# IMPORTANT: if the bench host's ssh session enters over this same
# interface, this script will drop its own control channel. Bench
# fleets MUST provide an out-of-band management network (a second
# interface or BMC) the harness can ssh over while the data-plane
# interface flaps. docs/help/bench-acceptance.md documents this
# expectation. The script self-detaches via `nohup … &` + `disown`
# so the down-up cycle survives ssh disconnect; the harness then
# reconnects to poll the recovery SLO.
#
# Args:
#   $1 — interface name (e.g. enp3s0, eno1)
set -euo pipefail

IFACE="${1:?interface name required as first arg}"

if ! ip link show "${IFACE}" >/dev/null 2>&1; then
    echo "link_flap.sh: interface '${IFACE}' not found" >&2
    exit 64
fi

# Detach the flap from the ssh session so dropping the data-plane
# interface doesn't kill the script mid-cycle. The parent ssh
# returns immediately; the harness polls the recovery state and
# enforces its own 5 s SLO.
nohup bash -c "
    sudo ip link set '${IFACE}' down
    sleep 2
    sudo ip link set '${IFACE}' up
" >/dev/null 2>&1 &
disown

# Give the detached job a beat to start before the parent ssh
# returns; otherwise the harness might race the link-down edge.
sleep 0.2
