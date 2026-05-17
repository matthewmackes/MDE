#!/usr/bin/env bash
# apply-xfce-baseline.sh — restore a captured XFCE baseline on this machine.
#
# Inverse of capture-xfce-baseline.sh. Loads each xfconf channel XML via
# `xfconf-query --channel X --load` and rsyncs the captured panel/*.rc
# files into ~/.config/xfce4/panel/.
#
# Usage:
#   install-helpers/apply-xfce-baseline.sh <preset-name>
#   install-helpers/apply-xfce-baseline.sh hashbang

set -euo pipefail

PRESET="${1:-hashbang}"

if [[ -z "$PRESET" || "$PRESET" =~ [^a-zA-Z0-9_-] ]]; then
    echo "usage: $0 <preset-name>  (alphanumeric + dash/underscore only)" >&2
    exit 2
fi

# Locate baseline: prefer system install (RPM), fall back to dev repo.
for candidate in \
    "/usr/share/mackes-shell/data/xfce-baseline/$PRESET" \
    "$(cd "$(dirname "$0")/.." && pwd)/data/xfce-baseline/$PRESET"; do
    if [[ -d "$candidate" ]]; then
        BASELINE_DIR="$candidate"
        break
    fi
done

if [[ -z "${BASELINE_DIR:-}" ]]; then
    echo "ERROR: no baseline found for preset '$PRESET'" >&2
    echo "Tried:" >&2
    echo "  /usr/share/mackes-shell/data/xfce-baseline/$PRESET" >&2
    echo "  data/xfce-baseline/$PRESET (dev repo)" >&2
    exit 1
fi

echo "Applying baseline from: $BASELINE_DIR"

# ---- 1. Replay each channel dump (parsed line-by-line) ----
#
# The dump is the output of `xfconf-query --channel X --list --verbose`,
# i.e. `<key>  <value>` per line. We can't use --load because that wants
# the XML format from --export (dropped in 4.18). Instead, parse and
# replay via --set, inferring type (bool / int / float / string).

APPLIED=()
for dump in "$BASELINE_DIR"/*.txt; do
    [[ -e "$dump" ]] || continue
    chan="$(basename "$dump" .txt)"
    keys_loaded=0
    while IFS= read -r line; do
        # Skip blanks and header
        [[ -z "$line" || "$line" == Property* ]] && continue
        # Split on first whitespace run: key + value
        key="${line%%[[:space:]]*}"
        value="${line#"$key"}"
        value="${value#"${value%%[![:space:]]*}"}"   # ltrim
        [[ "$key" == /* ]] || continue

        # Type inference
        if [[ "$value" == "true" || "$value" == "false" ]]; then
            xfconf-query --channel "$chan" --property "$key" --create --type bool --set "$value" 2>/dev/null \
                || xfconf-query --channel "$chan" --property "$key" --set "$value" 2>/dev/null \
                || true
        elif [[ "$value" =~ ^-?[0-9]+$ ]]; then
            xfconf-query --channel "$chan" --property "$key" --create --type int --set "$value" 2>/dev/null \
                || xfconf-query --channel "$chan" --property "$key" --set "$value" 2>/dev/null \
                || true
        elif [[ "$value" =~ ^-?[0-9]+\.[0-9]+$ ]]; then
            xfconf-query --channel "$chan" --property "$key" --create --type double --set "$value" 2>/dev/null \
                || xfconf-query --channel "$chan" --property "$key" --set "$value" 2>/dev/null \
                || true
        else
            xfconf-query --channel "$chan" --property "$key" --create --type string --set "$value" 2>/dev/null \
                || xfconf-query --channel "$chan" --property "$key" --set "$value" 2>/dev/null \
                || true
        fi
        keys_loaded=$((keys_loaded + 1))
    done < "$dump"
    APPLIED+=("$chan ($keys_loaded)")
    echo "  ✓ loaded channel: $chan ($keys_loaded keys)"
done

# ---- 2. Restore panel/*.rc files ----

PANEL_DST="$HOME/.config/xfce4/panel"
PANEL_SRC="$BASELINE_DIR/xfce4-panel.rcs"
if [[ -d "$PANEL_SRC" ]] && compgen -G "$PANEL_SRC/*.rc" >/dev/null; then
    mkdir -p "$PANEL_DST"
    cp -p "$PANEL_SRC"/*.rc "$PANEL_DST/"
    n=$(find "$PANEL_SRC" -name '*.rc' | wc -l)
    echo "  ✓ restored $n panel .rc files to $PANEL_DST"
fi

# ---- 3. Signal xfsettingsd to re-read and restart xfce4-panel ----

if command -v pkill >/dev/null; then
    pkill -HUP -x xfsettingsd 2>/dev/null || true
fi
if command -v xfce4-panel >/dev/null; then
    xfce4-panel -r 2>/dev/null || true
fi
if command -v xfdesktop >/dev/null; then
    xfdesktop --reload 2>/dev/null || true
fi

echo ""
echo "Applied baseline '$PRESET'."
echo "  - ${#APPLIED[@]} channels loaded"
echo "  - xfsettingsd signaled, xfce4-panel + xfdesktop reloaded"
