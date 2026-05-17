#!/usr/bin/env bash
# capture-xfce-baseline.sh — dump every xfconf channel + ~/.config/xfce4
# into data/xfce-baseline/<preset>/ so the preset can replay this machine's
# current XFCE layout on any future install.
#
# Per Q8 lock: presets ship the full XFCE baseline (all xfconf channels
# captured into per-preset directories) so a fresh install reproduces the
# captured machine's panel/desktop/wm/notification configuration.
#
# Usage:
#   install-helpers/capture-xfce-baseline.sh <preset-name>
#   install-helpers/capture-xfce-baseline.sh hashbang
#
# Output:
#   data/xfce-baseline/<preset-name>/
#     xsettings.xml
#     xfwm4.xml
#     xfce4-panel.xml
#     xfce4-desktop.xml
#     xfce4-notifyd.xml
#     xfce4-power-manager.xml
#     xfce4-session.xml
#     xfce4-appfinder.xml
#     thunar-volman.xml
#     keyboards.xml
#     keyboard-layout.xml
#     pointers.xml
#     displays.xml
#     xfce4-panel.rcs/        copy of ~/.config/xfce4/panel/*.rc files
#     manifest.json           hostname, timestamp, captured channels list
#
# Restore is the inverse: load each XML via `xfconf-query --channel X --load`
# and rsync the xfce4-panel.rcs/ tree back into ~/.config/xfce4/panel/.

set -euo pipefail

PRESET="${1:-hashbang}"

if [[ -z "$PRESET" || "$PRESET" =~ [^a-zA-Z0-9_-] ]]; then
    echo "usage: $0 <preset-name>  (alphanumeric + dash/underscore only)" >&2
    exit 2
fi

# Locate repo root: helper lives at install-helpers/<this>.sh so repo root
# is two parents up. Allow override via $REPO_ROOT.
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/.." && pwd)}"
OUT_DIR="$REPO_ROOT/data/xfce-baseline/$PRESET"

mkdir -p "$OUT_DIR"
mkdir -p "$OUT_DIR/xfce4-panel.rcs"

# Channels per Q8 lock: all XFCE channels go into per-preset baseline.
CHANNELS=(
    xsettings
    xfwm4
    xfce4-panel
    xfce4-desktop
    xfce4-notifyd
    xfce4-power-manager
    xfce4-session
    xfce4-appfinder
    thunar-volman
    keyboards
    keyboard-layout
    pointers
    displays
    xfce4-terminal
    xfce4-keyboard-shortcuts
)

# ---- 1. Dump xfconf channels via --export (XML format) ----

CAPTURED=()
SKIPPED=()
# xfconf-query 4.18+ dropped --export. We use --list --verbose which prints
# `<key> <value>` lines for every property in the channel; apply-xfce-
# baseline.sh parses these back and replays via `xfconf-query --set`.
for chan in "${CHANNELS[@]}"; do
    out="$OUT_DIR/$chan.txt"
    dump=$(xfconf-query --channel "$chan" --list --verbose 2>/dev/null) || dump=""
    if [[ -n "$dump" ]]; then
        printf '%s\n' "$dump" > "$out"
        CAPTURED+=("$chan")
        keys=$(grep -c "^/" "$out" || true)
        echo "  ✓ captured channel: $chan ($keys keys, $(wc -c < "$out") bytes)"
    else
        SKIPPED+=("$chan")
        echo "  · skipped channel: $chan (no properties / not present)"
    fi
done

# ---- 2. Copy ~/.config/xfce4/panel/*.rc (launcher items, plugin state) ----

PANEL_SRC="$HOME/.config/xfce4/panel"
if [[ -d "$PANEL_SRC" ]]; then
    panel_files=("$PANEL_SRC"/*.rc)
    if [[ -e "${panel_files[0]}" ]]; then
        cp -p "$PANEL_SRC"/*.rc "$OUT_DIR/xfce4-panel.rcs/" 2>/dev/null || true
        n=$(find "$OUT_DIR/xfce4-panel.rcs" -name '*.rc' | wc -l)
        echo "  ✓ captured $n panel .rc files"
    else
        echo "  · no panel .rc files to capture"
    fi
else
    echo "  · ~/.config/xfce4/panel does not exist (running on XFCE defaults)"
fi

# ---- 3. manifest.json ----

cat > "$OUT_DIR/manifest.json" <<EOF
{
  "preset":     "$PRESET",
  "hostname":   "$(hostname -s)",
  "captured":   "$(date -Iseconds)",
  "xfce4_version": "$(xfce4-about --version 2>/dev/null | head -1 || echo unknown)",
  "channels":   $(printf '%s\n' "${CAPTURED[@]}" | python3 -c 'import sys, json; print(json.dumps([l.strip() for l in sys.stdin]))'),
  "skipped":    $(printf '%s\n' "${SKIPPED[@]}" | python3 -c 'import sys, json; print(json.dumps([l.strip() for l in sys.stdin]))')
}
EOF

echo ""
echo "Baseline captured at: $OUT_DIR"
echo "  - ${#CAPTURED[@]} channels dumped"
echo "  - ${#SKIPPED[@]} channels skipped (not present)"
echo "  - manifest.json written"
echo ""
echo "To apply this baseline on another machine:"
echo "  install-helpers/apply-xfce-baseline.sh $PRESET"
