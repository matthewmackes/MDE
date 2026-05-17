#!/usr/bin/env bash
# install-carbon-icons.sh — fetch + package the IBM Carbon icon set as a
# GTK icon theme installed at /usr/share/icons/Carbon/.
#
# Carbon Icons upstream ships SVGs at https://github.com/carbon-design-
# system/carbon/tree/main/packages/icons/src/svg/32 (Apache 2.0).
# This script clones the icons package, packs the SVGs into a GTK icon-
# theme directory layout with a synthesized index.theme.
#
# Idempotent.

set -euo pipefail

CARBON_REPO="${CARBON_REPO:-https://github.com/carbon-design-system/carbon.git}"
CARBON_REF="${CARBON_REF:-main}"
SCOPE="${1:-system}"

case "$SCOPE" in
    system) DEST="/usr/share/icons"; SUDO="${SUDO:-pkexec}";;
    user)   DEST="$HOME/.local/share/icons"; SUDO="";;
    *) echo "usage: $0 [system|user]" >&2; exit 2;;
esac

if [[ -n "$SUDO" ]] && ! command -v "$SUDO" >/dev/null 2>&1; then
    SUDO="sudo"
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "Cloning Carbon Design System ($CARBON_REF) — this is large; trimming to packages/icons"
git clone --depth 1 --branch "$CARBON_REF" --filter=blob:none --sparse \
    "$CARBON_REPO" "$WORK/carbon"

cd "$WORK/carbon"
git sparse-checkout set packages/icons/src/svg

SVG_ROOT="$WORK/carbon/packages/icons/src/svg/32"
if [[ ! -d "$SVG_ROOT" ]]; then
    echo "ERROR: could not find Carbon icons at $SVG_ROOT" >&2
    exit 1
fi

# Build the icon-theme layout
THEME_DIR="$WORK/Carbon"
mkdir -p "$THEME_DIR/scalable/actions"
mkdir -p "$THEME_DIR/scalable/apps"
mkdir -p "$THEME_DIR/scalable/categories"
mkdir -p "$THEME_DIR/scalable/devices"
mkdir -p "$THEME_DIR/scalable/emblems"
mkdir -p "$THEME_DIR/scalable/places"
mkdir -p "$THEME_DIR/scalable/status"
mkdir -p "$THEME_DIR/scalable/mimetypes"

# Naive copy — every Carbon svg goes under scalable/apps/<name>.svg.
# Mapping to freedesktop standard icon names happens via symlinks below.
cp -r "$SVG_ROOT"/*.svg "$THEME_DIR/scalable/apps/" 2>/dev/null || true

# Minimal freedesktop icon-name mapping (extend as more are needed)
declare -A MAP=(
    [folder]=folder
    [folder-open]=folder--open
    [user-home]=home
    [system-shutdown]=power
    [system-reboot]=restart
    [system-search]=search
    [system-software-install]=download
    [edit-cut]=cut
    [edit-copy]=copy
    [edit-paste]=paste
    [edit-undo]=undo
    [edit-redo]=redo
    [document-new]=document
    [document-save]=save
    [document-open]=folder--open
    [view-refresh]=renew
    [list-add]=add
    [list-remove]=subtract
    [help-browser]=help
    [preferences-system]=settings
    [network-wired]=network--3
    [network-wireless]=wifi
    [audio-volume-high]=volume-up
    [audio-volume-low]=volume-down
    [audio-volume-muted]=volume-mute
    [view-refresh-symbolic]=renew
    [window-close]=close
    [emblem-default]=checkmark
    [dialog-warning]=warning
    [dialog-error]=error
    [dialog-information]=information
    [dialog-question]=help
)

for fd_name in "${!MAP[@]}"; do
    cb_name="${MAP[$fd_name]}.svg"
    if [[ -f "$THEME_DIR/scalable/apps/$cb_name" ]]; then
        ln -sf "$cb_name" "$THEME_DIR/scalable/apps/${fd_name}.svg"
    fi
done

cat > "$THEME_DIR/index.theme" <<'EOF'
[Icon Theme]
Name=Carbon
Comment=IBM Carbon Design System icon set (packaged by Mackes Shell)
Inherits=Adwaita,hicolor
Directories=scalable/actions,scalable/apps,scalable/categories,scalable/devices,scalable/emblems,scalable/places,scalable/status,scalable/mimetypes
Example=settings

[scalable/actions]
Context=Actions
MinSize=8
MaxSize=512
Size=32
Type=Scalable

[scalable/apps]
Context=Applications
MinSize=8
MaxSize=512
Size=32
Type=Scalable

[scalable/categories]
Context=Categories
MinSize=8
MaxSize=512
Size=32
Type=Scalable

[scalable/devices]
Context=Devices
MinSize=8
MaxSize=512
Size=32
Type=Scalable

[scalable/emblems]
Context=Emblems
MinSize=8
MaxSize=512
Size=32
Type=Scalable

[scalable/places]
Context=Places
MinSize=8
MaxSize=512
Size=32
Type=Scalable

[scalable/status]
Context=Status
MinSize=8
MaxSize=512
Size=32
Type=Scalable

[scalable/mimetypes]
Context=MimeTypes
MinSize=8
MaxSize=512
Size=32
Type=Scalable
EOF

# Move to destination
if [[ -n "$SUDO" ]]; then
    $SUDO rm -rf "$DEST/Carbon"
    $SUDO cp -r "$THEME_DIR" "$DEST/Carbon"
    $SUDO gtk-update-icon-cache -f -t "$DEST/Carbon" 2>/dev/null || true
else
    mkdir -p "$DEST"
    rm -rf "$DEST/Carbon"
    cp -r "$THEME_DIR" "$DEST/Carbon"
    gtk-update-icon-cache -f -t "$DEST/Carbon" 2>/dev/null || true
fi

echo "installed Carbon icon theme -> $DEST/Carbon"
