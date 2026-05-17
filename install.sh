#!/usr/bin/env bash
# Mackes Shell — curl-pipe-bash bootstrap (Q8 + Q20 locks)
#
#   curl -L https://github.com/matthewmackes/MAP2-RELEASES/releases/latest/download/install.sh | bash
#
# Fetches the latest RPM from GitHub Releases and installs it via dnf.
# Then exec's `mackes`, which routes into the first-run wizard.

set -euo pipefail

REPO="${MACKES_REPO:-matthewmackes/MAP2-RELEASES}"
GH_API="https://api.github.com/repos/$REPO/releases/latest"

err() { printf '\033[31m%s\033[0m\n' "$*" >&2; exit 1; }
inf() { printf '\033[34m▸ %s\033[0m\n' "$*"; }
ok()  { printf '\033[32m✓ %s\033[0m\n' "$*"; }

[ "$(id -u)" -ne 0 ] || err "Do not pipe this to sudo. The script asks for sudo only when it needs it."

command -v dnf       >/dev/null 2>&1 || err "dnf not found — Mackes Shell targets Fedora."
command -v rpm       >/dev/null 2>&1 || err "rpm not found."
command -v curl      >/dev/null 2>&1 || err "curl not found."

fedora_ver="$(rpm -E %fedora)"
arch="$(uname -m)"
inf "Fedora detected:    $fedora_ver  ($arch)"
inf "Resolving latest:   $REPO"

tag="$(curl -fsSL "$GH_API" | grep -oP '"tag_name":\s*"\K[^"]+' | head -1 || true)"
[ -n "$tag" ] || err "Could not resolve latest release tag for $REPO."

version="${tag#v}"
rpm_url="https://github.com/$REPO/releases/download/$tag/mackes-shell-${version}-1.fc${fedora_ver}.${arch}.rpm"

inf "Release tag:        $tag"
inf "RPM URL:            $rpm_url"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

inf "Downloading RPM…"
curl -fL --progress-bar -o "$tmp/mackes-shell.rpm" "$rpm_url"
ok "RPM downloaded ($(du -h "$tmp/mackes-shell.rpm" | cut -f1))"

inf "Installing…"
sudo dnf install -y "$tmp/mackes-shell.rpm"
ok "Installed."

if [ -n "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ]; then
  inf "Launching first-run wizard…"
  exec mackes
else
  ok "Mackes Shell is installed. Run \`mackes\` from a graphical session to provision this machine."
fi
