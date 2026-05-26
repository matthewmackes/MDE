#!/bin/sh
# install-helpers/fetch-material-symbols.sh
# EPIC-UI-MATERIAL.svg-swap — Material Symbols SVG asset fetcher.
#
# Downloads the 46 Material Symbols SVGs (outlined variant, weight
# 400) at 3 optical sizes (20 / 24 / 40 px) plus fill variants for
# the fill-eligible icons. Pulls from Google's official
# material-design-icons GitHub repo:
#
#   https://raw.githubusercontent.com/google/material-design-icons/
#       master/symbols/web/<name>/materialsymbolsoutlined/
#       <name>[_fill1]_<size>px.svg
#
# Locked by `docs/design/icon-mapping.md`. Apache-2.0 licensed at
# source.
#
# Idempotent: skips files that already exist on disk (size > 0).
# Re-run after editing the mapping to fetch new entries; delete a
# single file to force a re-fetch.
#
# Exit code 1 if any download produces a non-SVG response (catches
# silent 404 HTML pages).

set -eu

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST_DIR="$REPO_ROOT/assets/icons/material-symbols"
SIZES="20 24 40"
BASE_URL="https://raw.githubusercontent.com/google/material-design-icons/master/symbols/web"

mkdir -p "$DEST_DIR"

# ──────────────────────────────────────────────────────────────
# Mapping table — Carbon name : Material name : fill-mode
#
# fill-mode legend:
#   none   — outlined only (most icons)
#   always — always rendered filled (Q38 carry-over: status dots,
#            notification bell, playbook play-glyph)
#   active — outlined by default, filled on IconState::Active
#            (nav / sidebar entries that respond to selection)
#
# Heuristic mapping per Round 2 lock — closest semantic match
# against Google's Material Symbols catalog. Disagreements get
# reverted in follow-on commits.
# ──────────────────────────────────────────────────────────────

MAPPING='
dashboard:dashboard:active
application:apps:active
network--3:network_check:active
devices:devices:active
color-palette:palette:active
settings:settings:active
tools:build:active
network--public:public:active
help:help:active
save:save:none
machine-learning-model:memory:none
list:list:none
rocket:rocket_launch:none
volume-up:volume_up:none
screen:desktop_windows:none
printer:print:none
battery-charging:battery_charging_full:none
usb:usb:none
time:schedule:none
image:image:none
text-font:text_fields:none
user:person:none
notification--filled:notifications:always
wifi:wifi:none
vpn-connection:vpn_lock:none
firewall-classic:security:none
play-filled:play_arrow:always
recently-viewed:history:none
list-boxes:checklist:none
workbench:handyman:none
files:folder:active
subtract:remove:none
maximize:fullscreen:none
checkmark--filled:check_circle:always
warning--alt--filled:warning:always
error--filled:error:always
help--filled:help:always
renew:refresh:none
add:add:none
trash-can:delete:none
edit:edit:none
checkmark:check:none
close:close:none
search:search:none
chevron--right:chevron_right:none
chevron--down:expand_more:none
'

download_one() {
    name="$1"
    fill_suffix="$2"   # empty or "_fill1"
    size="$3"
    target="$DEST_DIR/${name}${fill_suffix}_${size}px.svg"

    if [ -s "$target" ]; then
        return 0
    fi

    url="${BASE_URL}/${name}/materialsymbolsoutlined/${name}${fill_suffix}_${size}px.svg"
    tmp="${target}.tmp"

    if ! curl -fsSL "$url" -o "$tmp"; then
        echo "$0: FAILED to download $url" >&2
        rm -f "$tmp"
        return 1
    fi

    # Sanity-check: must be an SVG (`<svg` opener). Google sometimes
    # returns an HTML 404 page with a 200 status from raw.github,
    # so trust the body, not just the status.
    if ! head -c 200 "$tmp" | grep -q '<svg'; then
        echo "$0: $url returned non-SVG content (likely a 404 page)" >&2
        rm -f "$tmp"
        return 1
    fi

    mv "$tmp" "$target"
}

count_total=0
count_downloaded=0
count_skipped=0

# Each MAPPING row: carbon_name:material_name:fill_mode
# Use IFS-newline iteration so spaces inside aren't a problem.
old_ifs="$IFS"
IFS='
'
for row in $MAPPING; do
    [ -z "$row" ] && continue
    IFS=':'
    # shellcheck disable=SC2086
    set -- $row
    IFS="$old_ifs"

    material_name="$2"
    fill_mode="$3"

    # Always-outlined download (every icon gets these 3 sizes).
    for size in $SIZES; do
        count_total=$((count_total + 1))
        target="$DEST_DIR/${material_name}_${size}px.svg"
        if [ -s "$target" ]; then
            count_skipped=$((count_skipped + 1))
        else
            download_one "$material_name" "" "$size"
            count_downloaded=$((count_downloaded + 1))
        fi
    done

    # Fill variant download (only for fill-eligible icons).
    if [ "$fill_mode" = "always" ] || [ "$fill_mode" = "active" ]; then
        for size in $SIZES; do
            count_total=$((count_total + 1))
            target="$DEST_DIR/${material_name}_fill1_${size}px.svg"
            if [ -s "$target" ]; then
                count_skipped=$((count_skipped + 1))
            else
                download_one "$material_name" "_fill1" "$size"
                count_downloaded=$((count_downloaded + 1))
            fi
        done
    fi

    IFS='
'
done
IFS="$old_ifs"

echo "$0: $count_total target SVGs (downloaded $count_downloaded, skipped $count_skipped already present)"
ls "$DEST_DIR" | wc -l | awk '{ printf "%s: %s files on disk in '"$DEST_DIR"'\n", "'"$0"'", $1 }'
