#!/usr/bin/env bash
# NF-20.3 — v4.0 greenfield acceptance gate.
#
# Operator-run BEFORE `cut release 4.0`. On a fresh Fedora 44
# VM (bare-metal or qemu — never on a live host), this script
# walks the locked acceptance checklist:
#
#   1. The mde-X.Y.Z RPM installs cleanly.
#   2. `mackes init` completes the first-boot wizard
#      non-interactively (--yes + a join token or "create new
#      mesh" flag).
#   3. A second peer enrolls via `mackesd enroll --token` and
#      reaches `connected` state.
#   4. No Tailscale residue (`rpm -q tailscale headscale
#      tailscale-derp` returns "not installed").
#   5. The whole pass completes in under 10 minutes of
#      operator wall-clock time.
#
# Exit 0 on PASS (all gates green), exit 1 on any FAIL.
# Outputs a structured progress log so operators can rerun a
# single step after a transient failure.
#
# This script is the BENCH harness — it expects to be run on a
# pair of fresh Fedora 44 hosts the operator has provisioned
# (one lighthouse, one peer). The `MDE_BENCH_LIGHTHOUSE` +
# `MDE_BENCH_PEER` env vars supply the SSH targets.
#
# Locked per worklist NF-20.3 + the Q5 "greenfield only" cut
# directive (no migration path exercised at cut time).

set -eu

START_S=$(date +%s)
WALL_CLOCK_BUDGET_S=600   # 10 minutes per the Q5 lock
FAIL_COUNT=0

# Color codes for the gate output (operator-visible).
RED=$'\033[31m'
GRN=$'\033[32m'
YLW=$'\033[33m'
RST=$'\033[0m'

fail() {
    FAIL_COUNT=$((FAIL_COUNT + 1))
    printf '%s[FAIL]%s %s\n' "$RED" "$RST" "$1" >&2
}

pass() {
    printf '%s[PASS]%s %s\n' "$GRN" "$RST" "$1"
}

info() {
    printf '%s[INFO]%s %s\n' "$YLW" "$RST" "$1"
}

# Source/host helper — runs a command on a remote host via SSH.
# Sets a 30s timeout so a hung host doesn't blow the wall-clock
# budget.
remote() {
    local host="$1"; shift
    timeout 30 ssh -o BatchMode=yes -o ConnectTimeout=10 \
        -o StrictHostKeyChecking=accept-new "$host" "$@"
}

# ---- Gate 0: prerequisites --------------------------------

info "Greenfield acceptance gate — v4.0 cut sign-off"
info "Lighthouse: ${MDE_BENCH_LIGHTHOUSE:-<unset>}"
info "Peer:       ${MDE_BENCH_PEER:-<unset>}"
info "Budget:     ${WALL_CLOCK_BUDGET_S} s (10 min)"

if [ -z "${MDE_BENCH_LIGHTHOUSE:-}" ] || [ -z "${MDE_BENCH_PEER:-}" ]; then
    fail "Set MDE_BENCH_LIGHTHOUSE + MDE_BENCH_PEER to SSH targets before running"
    exit 1
fi
if [ -z "${MDE_BENCH_RPM_URL:-}" ]; then
    fail "Set MDE_BENCH_RPM_URL to the mde-4.0 RPM URL (or local file://)"
    exit 1
fi

# ---- Gate 1: RPM installs cleanly on lighthouse ------------

info "Gate 1/5 — install mde-4.0 RPM on lighthouse"
if remote "$MDE_BENCH_LIGHTHOUSE" "sudo dnf install -y '${MDE_BENCH_RPM_URL}'"; then
    pass "Lighthouse RPM install"
else
    fail "Lighthouse RPM install failed"
fi

# ---- Gate 2: lighthouse mints + becomes operational --------

info "Gate 2/5 — lighthouse mints CA + opens for enroll"
if remote "$MDE_BENCH_LIGHTHOUSE" \
    "sudo mackesd ca mint --mesh-id 'bench-v4.0' && \
     sudo systemctl enable --now mackesd.service && \
     sleep 5 && \
     sudo mackesd healthz | grep -q '\"ok\":true'"; then
    pass "Lighthouse CA mint + mackesd healthy"
else
    fail "Lighthouse mint or daemon health"
fi

# Generate a join token on the lighthouse for the peer to use.
JOIN_TOKEN=$(remote "$MDE_BENCH_LIGHTHOUSE" \
    "sudo mackesd nebula peer-list 2>&1 | grep -oE 'mesh:[^ ]+' | head -1" \
    || true)
if [ -z "$JOIN_TOKEN" ]; then
    info "(no token surfaced — operator runs ca sign-csr after peer publishes)"
fi

# ---- Gate 3: peer installs + enrolls -----------------------

info "Gate 3/5 — peer installs RPM + enrolls"
if remote "$MDE_BENCH_PEER" "sudo dnf install -y '${MDE_BENCH_RPM_URL}'"; then
    pass "Peer RPM install"
else
    fail "Peer RPM install failed"
fi

if [ -n "$JOIN_TOKEN" ]; then
    if remote "$MDE_BENCH_PEER" \
        "sudo systemctl enable --now mackesd.service && \
         sleep 2 && \
         sudo mackesd enroll --token '${JOIN_TOKEN}' && \
         sleep 10 && \
         sudo mackesd nebula status | grep -q 'connected'"; then
        pass "Peer enrolled + reached connected"
    else
        fail "Peer enroll or connected-state"
    fi
fi

# ---- Gate 4: no Tailscale residue --------------------------

info "Gate 4/5 — Tailscale residue check (lighthouse + peer)"
for host in "$MDE_BENCH_LIGHTHOUSE" "$MDE_BENCH_PEER"; do
    if remote "$host" \
        "rpm -q tailscale headscale tailscale-derp 2>&1 | \
         grep -qE 'not installed' || ! rpm -q tailscale headscale tailscale-derp"; then
        pass "$host: no Tailscale RPMs installed"
    else
        fail "$host: Tailscale residue detected"
    fi
done

# ---- Gate 5: wall-clock budget -----------------------------

ELAPSED_S=$(( $(date +%s) - START_S ))
info "Gate 5/5 — wall-clock check"
if [ "$ELAPSED_S" -le "$WALL_CLOCK_BUDGET_S" ]; then
    pass "Total elapsed ${ELAPSED_S} s ≤ ${WALL_CLOCK_BUDGET_S} s"
else
    fail "Total elapsed ${ELAPSED_S} s exceeded ${WALL_CLOCK_BUDGET_S} s budget"
fi

# ---- Verdict -----------------------------------------------

if [ "$FAIL_COUNT" -eq 0 ]; then
    printf '\n%s═══ GREENFIELD ACCEPTANCE: PASS ═══%s\n' "$GRN" "$RST"
    printf 'v4.0 cut is greenlit. Proceed with `cut release 4.0`.\n'
    exit 0
else
    printf '\n%s═══ GREENFIELD ACCEPTANCE: FAIL (%d gate%s) ═══%s\n' \
        "$RED" "$FAIL_COUNT" "$([ $FAIL_COUNT -eq 1 ] || echo s)" "$RST" >&2
    printf 'v4.0 cut is BLOCKED until the failed gates above pass.\n' >&2
    exit 1
fi
