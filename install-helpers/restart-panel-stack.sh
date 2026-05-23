#!/bin/bash
# v4.0.1 PARITY-6 (2026-05-23) — restart the panel + popover stack
# so a freshly-installed binary takes effect without operator
# intervention.
#
# Context: `data/sway/config` declares `exec mde-panel` /
# `exec mde-popover watermark` / `exec mde-popover toast` as
# one-shots (NOT `exec_always`). When the parity overlay
# replaces `/usr/bin/mde-panel` with a new build, the running
# process keeps its old code in memory until killed — and
# `swaymsg reload` only re-runs `exec_always` lines, so a
# `reload` doesn't help either.
#
# This helper pkills the affected processes and respawns them
# with the new binary. The parity overlay calls this after
# every successful install phase when at least one panel-stack
# binary changed; the operator can also run it manually.
#
# Idempotent: if a process isn't running, the pkill is a no-op
# and the spawn brings it up fresh.
#
# Usage:
#   restart-panel-stack.sh            # restart all three
#   restart-panel-stack.sh panel      # just mde-panel
#   restart-panel-stack.sh popovers   # just the two popovers
#   restart-panel-stack.sh watermark  # just the watermark daemon
#
# Exit codes:
#   0 — every requested process was respawned (or was a no-op
#       because the binary isn't installed).
#   1 — sway isn't reachable (no $SWAYSOCK / $DISPLAY).

set -uo pipefail

SCOPE="${1:-all}"
LOG_PREFIX="restart-panel-stack:"
SLEEP_AFTER_KILL=0.3

log() {
    echo "$LOG_PREFIX $*"
}

# Check the session is actually a graphical one we can spawn
# into. Without $WAYLAND_DISPLAY / $DISPLAY a `setsid mde-panel
# &` would still spawn but immediately die — better to bail with
# a useful exit code than leave zombies.
session_alive() {
    [ -n "${WAYLAND_DISPLAY:-}" ] || [ -n "${DISPLAY:-}" ]
}

# Kill a process by its exact comm-name + wait briefly for it
# to actually exit. `pkill -x` matches the 15-char comm name
# (not the full argv), which is what we want for binaries like
# `mde-panel` that shouldn't catch `mackes-panel` etc.
kill_by_comm() {
    local comm="$1"
    if pgrep -x "$comm" >/dev/null 2>&1; then
        log "killing existing $comm"
        pkill -x "$comm" 2>/dev/null || true
        sleep "$SLEEP_AFTER_KILL"
        # Force-kill stragglers.
        if pgrep -x "$comm" >/dev/null 2>&1; then
            log "force-killing stubborn $comm"
            pkill -9 -x "$comm" 2>/dev/null || true
            sleep "$SLEEP_AFTER_KILL"
        fi
    fi
}

# Spawn a binary in the background detached from this shell so
# it survives this script exiting. `setsid` puts the child in
# its own session so signal propagation stays clean; the
# `--fork` flag isn't portable across coreutils versions so we
# use `&` + `disown`. stdin redirected from /dev/null + stdout/
# stderr merged into journald via systemd-cat when available.
spawn_detached() {
    local bin="$1"
    shift
    local args=("$@")
    if ! command -v "$bin" >/dev/null 2>&1; then
        log "skip $bin — binary not installed"
        return 0
    fi
    log "spawning $bin ${args[*]}"
    if command -v systemd-cat >/dev/null 2>&1; then
        ( setsid systemd-cat -t "$bin" "$bin" "${args[@]}" </dev/null >/dev/null 2>&1 & )
    else
        ( setsid "$bin" "${args[@]}" </dev/null >/dev/null 2>&1 & )
    fi
}

if ! session_alive; then
    log "ERROR: no \$WAYLAND_DISPLAY or \$DISPLAY — not in a graphical session"
    exit 1
fi

case "$SCOPE" in
    all)
        kill_by_comm mde-panel
        kill_by_comm mde-popover
        spawn_detached mde-panel
        spawn_detached mde-popover watermark
        # v4.0.1 BUG-17 (2026-05-23): toast popover autostart
        # restored after the transparent-empty fix landed.
        spawn_detached mde-popover toast
        ;;
    panel)
        kill_by_comm mde-panel
        spawn_detached mde-panel
        ;;
    popovers)
        kill_by_comm mde-popover
        spawn_detached mde-popover watermark
        spawn_detached mde-popover toast
        ;;
    watermark)
        # Watermark is a singleton headless daemon — kill the
        # one running and respawn just that variant.
        # mde-popover's comm is 'mde-popover' regardless of the
        # subcommand argv, so we pgrep -f for "watermark" to
        # avoid killing the toast variant.
        if pgrep -f "mde-popover watermark" >/dev/null 2>&1; then
            log "killing existing mde-popover watermark"
            pkill -f "mde-popover watermark" 2>/dev/null || true
            sleep "$SLEEP_AFTER_KILL"
        fi
        spawn_detached mde-popover watermark
        ;;
    toast)
        if pgrep -f "mde-popover toast" >/dev/null 2>&1; then
            log "killing existing mde-popover toast"
            pkill -f "mde-popover toast" 2>/dev/null || true
            sleep "$SLEEP_AFTER_KILL"
        fi
        spawn_detached mde-popover toast
        ;;
    *)
        log "usage: $0 [all|panel|popovers|watermark|toast]"
        exit 1
        ;;
esac

log "done"
exit 0
