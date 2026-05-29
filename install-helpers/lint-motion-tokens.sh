#!/bin/sh
# install-helpers/lint-motion-tokens.sh — SWAY-7 / Q93 pre-commit gate.
#
# Catches ad-hoc motion-grid Duration literals in `crates/mde-*/src`.
# The canonical animation timing contract lives in
# `mde_theme::motion::Motion::*` presets (panel_mount / dialog_mount /
# tooltip_fade / notification_pulse) + `data/css/motion-vocabulary.css`.
#
# Motion grid values (sway-native-shell.md §3):
#   100 ms — minimum / exit one-tier-faster
#   120 ms — tooltip / compact dismiss
#   150 ms — short entrance
#   180 ms — panel / dialog mount
#   200 ms — standard entrance
#
# Bare `Duration::from_millis(N)` for these values in animation-path
# code should reference `mde_theme::motion::Motion::*().duration` or
# a named constant from `mde_theme::animation` instead.
#
# Per CLAUDE.md §0.7 gate #16.
#
# Exits 0 = clean, exits 1 = violations found.

set -eu

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

# mde-* crates only; exclude mde-theme (canonical source) + mackes-* (legacy).
SCAN_PATHS=""
for d in crates/mde-*/src; do
    crate_name="$(basename "$(dirname "$d")")"
    case "$crate_name" in
        mde-theme) continue ;;  # canonical source of the tokens
    esac
    SCAN_PATHS="$SCAN_PATHS $d"
done

# Motion-grid Duration literals. egrep-compatible.
PATTERN='Duration::from_millis\((100|120|150|180|200)\b'

TMPFILE="$(mktemp)"
trap 'rm -f "$TMPFILE"' EXIT

# Scan. grep exits 1 if no match — suppress that.
# shellcheck disable=SC2086
grep -rnE "$PATTERN" $SCAN_PATHS 2>/dev/null > "$TMPFILE" || true

# Filter out test files.
sed -i.bak '/_tests\.rs:/d' "$TMPFILE" && rm -f "$TMPFILE.bak"
sed -i.bak '/\/tests\//d' "$TMPFILE" && rm -f "$TMPFILE.bak"
sed -i.bak '/\#\[cfg(test/d' "$TMPFILE" && rm -f "$TMPFILE.bak"

# Filter out infrastructure patterns (poll ticks, sleeps, timeouts) —
# these are not animation durations.
sed -i.bak '/sleep(/d' "$TMPFILE" && rm -f "$TMPFILE.bak"
sed -i.bak '/timeout(/d' "$TMPFILE" && rm -f "$TMPFILE.bak"
sed -i.bak '/every(/d' "$TMPFILE" && rm -f "$TMPFILE.bak"
sed -i.bak '/tick_interval/d' "$TMPFILE" && rm -f "$TMPFILE.bak"
sed -i.bak '/poll_interval/d' "$TMPFILE" && rm -f "$TMPFILE.bak"
sed -i.bak '/reconnect/d' "$TMPFILE" && rm -f "$TMPFILE.bak"
# Filter pure comment lines.
sed -i.bak '/^[^:]*:[0-9]*:[[:space:]]*\/\//d' "$TMPFILE" && rm -f "$TMPFILE.bak"

# Snapshot allow-list (2026-05-29). Pre-existing non-animation uses:
#   mde-bus/src/subs.rs    — DEFAULT_TICK_INTERVAL: 100 ms file mtime-poll cadence
#   mde-bus/src/rpc.rs     — DEFAULT_POLL_INTERVAL:  200 ms ntfy RPC poll cadence
#   mde-popover/src/toasts.rs — inline #[cfg(test)] block (visible_for fixture)
# Format: one path:line-number prefix per line.
ALLOWLIST='
crates/mde-bus/src/subs.rs:58
crates/mde-bus/src/rpc.rs:45
crates/mde-popover/src/toasts.rs:529
crates/mde-popover/src/toasts.rs:530
'

echo "$ALLOWLIST" | while IFS= read -r prefix; do
    prefix="$(printf '%s' "$prefix" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
    case "$prefix" in
        ''|'#'*) continue ;;
    esac
    esc="$(printf '%s' "$prefix" | sed 's/[\/&]/\\&/g')"
    sed -i.bak "/^${esc}/d" "$TMPFILE" && rm -f "$TMPFILE.bak"
done

if [ -s "$TMPFILE" ]; then
    echo "lint-motion-tokens.sh: §0.7 gate #16 violations:"
    echo
    cat "$TMPFILE"
    echo
    echo "Motion-grid Duration literals must come from mde_theme::motion::Motion"
    echo "presets, not bare from_millis() calls. Use:"
    echo "  Motion::panel_mount().duration    // 180 ms"
    echo "  Motion::dialog_mount().duration   // 180 ms"
    echo "  Motion::tooltip_fade().duration   // 120 ms"
    echo "  Motion::notification_pulse().duration // 2000 ms"
    echo "Or add a named const in mde_theme::motion if the preset is new."
    echo "If this is a non-animation Duration (poll tick, sleep), move it"
    echo "out of the pattern match or add a comment to the allow-list above."
    exit 1
fi

echo "lint-motion-tokens.sh: clean (no ad-hoc motion-grid Duration literals)"
exit 0
