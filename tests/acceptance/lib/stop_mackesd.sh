#!/usr/bin/env bash
# NF-9.6 helper — stop the mackesd service on the current host.
#
# Used by tests/acceptance/test_nebula_fabric.py to drive the
# leader-kill scenario. Asserting the new-leader election + CA epoch
# bump happens harness-side.
set -euo pipefail

sudo systemctl stop mackesd

# Confirm it really stopped — `systemctl stop` exits 0 even if the
# unit was already inactive; we want to fail loudly if something
# restarted it under us (e.g. a Restart=always loop).
if systemctl is-active --quiet mackesd; then
    echo "stop_mackesd.sh: mackesd is still active after stop" >&2
    exit 65
fi
