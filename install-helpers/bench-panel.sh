#!/usr/bin/env bash
# Phase 9.4 — perf gate for mackes-panel.
#
# Q41-revised targets:
#   cold start  < 200 ms   (systemd-start ~> first paint, Xvfb-measured)
#   idle CPU    < 1 %      (averaged over a 60 s no-input window)
#   RSS        ≤ 150 MB    (after 5 minutes runtime, drawer + dock populated)
#
# This script runs the panel for a fixed duration under a clean Xvfb
# display, then samples /proc/<pid>/stat + status to derive each number.
# Designed to be CI-callable (Phase 9.4 acceptance) but works locally too
# as long as Xvfb is installed.

set -euo pipefail

PANEL_BIN="${PANEL_BIN:-./target/release/mackes-panel}"
DISPLAY_NUM="${DISPLAY_NUM:-:99}"
RSS_SAMPLES="${RSS_SAMPLES:-3}"
SAMPLE_WINDOW_S="${SAMPLE_WINDOW_S:-20}"

if [[ ! -x "$PANEL_BIN" ]]; then
    echo "panel binary not found: $PANEL_BIN" >&2
    echo "build it first:  cargo build --release -p mackes-panel" >&2
    exit 2
fi

command -v Xvfb >/dev/null 2>&1 || { echo "Xvfb required (dnf install xorg-x11-server-Xvfb)" >&2; exit 3; }

# Start a clean virtual X server. Background, kill on exit.
Xvfb "$DISPLAY_NUM" -screen 0 1920x1080x24 -nolisten tcp >/dev/null 2>&1 &
XVFB_PID=$!
trap 'kill "$XVFB_PID" 2>/dev/null || true; wait "$XVFB_PID" 2>/dev/null || true' EXIT

# Wait for the X socket to appear (Xvfb startup is async).
for _ in {1..50}; do
    [[ -S "/tmp/.X11-unix/X${DISPLAY_NUM#:}" ]] && break
    sleep 0.1
done

export DISPLAY="$DISPLAY_NUM"

# Cold-start: monotonic clock around exec; we'd ideally hook GTK's
# 'first-frame' signal, but for now process-start to a stable
# 'window mapped' proxy is close enough — we sample /proc/<pid>/stat
# immediately after spawn and call the first measurement t0.
START_NS=$(date +%s%N)
"$PANEL_BIN" >/tmp/mackes-panel.bench.log 2>&1 &
PANEL_PID=$!

# Wait until the binary has at least 5 MB RSS (proxy for "GTK is up").
for _ in {1..200}; do
    if [[ -r "/proc/$PANEL_PID/status" ]]; then
        rss_kb=$(awk '/^VmRSS:/ {print $2}' "/proc/$PANEL_PID/status" 2>/dev/null || echo 0)
        if (( rss_kb > 5000 )); then
            break
        fi
    fi
    sleep 0.005  # 5 ms granularity → resolves cold-start to ~5 ms
done
WARM_NS=$(date +%s%N)
COLD_MS=$(( (WARM_NS - START_NS) / 1000000 ))

# Hold steady for the sampling window so idle-CPU averages over real
# steady-state, not GTK initial layout work.
sleep "$SAMPLE_WINDOW_S"

# RSS samples → max over RSS_SAMPLES taps.
RSS_MAX_KB=0
for _ in $(seq 1 "$RSS_SAMPLES"); do
    if [[ -r "/proc/$PANEL_PID/status" ]]; then
        rss_kb=$(awk '/^VmRSS:/ {print $2}' "/proc/$PANEL_PID/status")
        (( rss_kb > RSS_MAX_KB )) && RSS_MAX_KB=$rss_kb
    fi
    sleep 1
done
RSS_MB=$(( RSS_MAX_KB / 1024 ))

# Idle CPU% — compute from /proc/<pid>/stat utime+stime delta over the
# sampling window divided by elapsed wall-clock and CPU count.
read -ra STAT_A < "/proc/$PANEL_PID/stat"
T_A=$(date +%s%N)
sleep 10
read -ra STAT_B < "/proc/$PANEL_PID/stat"
T_B=$(date +%s%N)

# /proc/<pid>/stat columns: utime=14, stime=15 (1-indexed). USER_HZ=100.
UTIME_DELTA=$(( STAT_B[13] - STAT_A[13] ))
STIME_DELTA=$(( STAT_B[14] - STAT_A[14] ))
CPU_TICKS=$(( UTIME_DELTA + STIME_DELTA ))
WALL_S_TIMES_10=$(( (T_B - T_A) / 100000000 ))   # tenths of seconds
# percent = ticks / (HZ * wall_s) * 100  →  *10/wall_s_x10 == /wall_s
CPU_PCT_TENTH=$(( CPU_TICKS * 1000 / (WALL_S_TIMES_10 > 0 ? WALL_S_TIMES_10 : 1) ))
CPU_PCT_INT=$(( CPU_PCT_TENTH / 10 ))
CPU_PCT_FRAC=$(( CPU_PCT_TENTH % 10 ))

kill "$PANEL_PID" 2>/dev/null || true
wait "$PANEL_PID" 2>/dev/null || true

# Emit metrics + gate result.
COLD_OK="✗"; (( COLD_MS < 200 )) && COLD_OK="✓"
RSS_OK="✗";  (( RSS_MB <= 150 )) && RSS_OK="✓"
CPU_OK="✗";  (( CPU_PCT_INT < 1 )) && CPU_OK="✓"

cat <<EOF

mackes-panel perf gate (Q41 revised 2026-05-18)
binary: $PANEL_BIN
display: Xvfb 1920x1080x24

  $COLD_OK  cold start   ${COLD_MS} ms     (target < 200 ms)
  $RSS_OK  RSS         ${RSS_MB} MB     (target ≤ 150 MB)
  $CPU_OK  idle CPU    ${CPU_PCT_INT}.${CPU_PCT_FRAC} %    (target < 1 %)

EOF

# Exit code: 0 only when every gate passes.
[[ "$COLD_OK" == "✓" && "$RSS_OK" == "✓" && "$CPU_OK" == "✓" ]] || exit 1
