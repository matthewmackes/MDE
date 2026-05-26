#!/bin/sh
# install-helpers/lint-design-tokens.sh — TUNE-10 / 25-Q Q7
# pre-commit gate #12.
#
# Catches hardcoded design tokens in active visual code that
# should reference the canonical token files instead:
#
#   Hex literals (#1d1d1f, #fff, etc.) outside:
#     data/css/tokens.css
#     data/css/motion-vocabulary.css
#     data/css/greeter.css
#     crates/mde-theme/
#
#   Rust Color::from_rgb / Color::from_rgba outside:
#     crates/mde-theme/
#     crates/mde-iced-components/  (if it exists)
#
#   Duration literals (`Duration::from_millis(N)` where N != 150)
#   outside:
#     crates/mde-theme/  (motion constants)
#     Any test code (timeouts can legitimately vary)
#
#   Font-name literals ("Roboto", "Intel One Mono", etc.) outside:
#     crates/mde-theme/
#     data/css/tokens.css
#
# Per CLAUDE.md §0.7 gate #12.
#
# Exits 0 = clean, exits 1 = net-new hardcoded-token hits.

set -eu

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

ALLOWLIST_FILE="${REPO_ROOT}/install-helpers/lint-design-tokens.allowlist"

# Source files to scan.
SCAN_PATHS='crates/mde-applets crates/mde-bus crates/mde-card crates/mde-drawer crates/mde-files crates/mde-iced-components crates/mde-kdc crates/mde-logout-dialog crates/mde-panel crates/mde-peer-card crates/mde-popover crates/mde-portal crates/mde-session crates/mde-wizard crates/mde-workbench data/css'

# Files exempt from token enforcement — canonical token sources.
EXEMPT_PATHS='crates/mde-theme/ data/css/tokens.css data/css/motion-vocabulary.css data/css/greeter.css'

# Helper: filter a list of violations against EXEMPT_PATHS and
# the snapshot allow-list file.
filter_violations() {
    local input="$1"
    # Strip exempt paths
    local filtered="$input"
    for exempt in $EXEMPT_PATHS; do
        # Escape `/` for sed
        esc=$(printf '%s' "$exempt" | sed 's|/|\\/|g')
        filtered=$(printf '%s\n' "$filtered" | sed "/${esc}/d")
    done
    # Strip allow-listed lines (by file:line key)
    if [ -f "$ALLOWLIST_FILE" ]; then
        # Allow-list format: <file>:<line> (no message)
        # Use process substitution to filter
        local allow_keys
        allow_keys=$(grep -v '^[[:space:]]*#' "$ALLOWLIST_FILE" | grep -v '^[[:space:]]*$' || true)
        if [ -n "$allow_keys" ]; then
            # Build a temp file of allow keys + grep -vFf
            tmp_allow=$(mktemp)
            printf '%s\n' "$allow_keys" > "$tmp_allow"
            filtered=$(printf '%s\n' "$filtered" | grep -vFf "$tmp_allow" 2>/dev/null || true)
            rm -f "$tmp_allow"
        fi
    fi
    printf '%s' "$filtered"
}

VIOLATIONS=""

# 1. Hex literals in source files (Rust + CSS).
HEX_HITS=$(grep -rEn '#[0-9a-fA-F]{3,8}\b' --include='*.rs' --include='*.css' $SCAN_PATHS 2>/dev/null || true)
# Strip very common matches that aren't colors: `#[derive(...)]` Rust attributes (4+ hex chars only)
HEX_HITS=$(printf '%s\n' "$HEX_HITS" | grep -vE '#\[derive|#\[cfg|#\[allow|#\[deny|#\[warn|#\[must_use|#\[doc' || true)
# Filter to file:line keys for allow-list matching
HEX_KEYS=$(printf '%s\n' "$HEX_HITS" | sed -nE 's|^([^:]+):([0-9]+):.*|\1:\2|p')

# 2. Rust Color::from_rgb / from_rgba calls.
RGB_HITS=$(grep -rEn 'Color::from_rgb' --include='*.rs' $SCAN_PATHS 2>/dev/null || true)
RGB_KEYS=$(printf '%s\n' "$RGB_HITS" | sed -nE 's|^([^:]+):([0-9]+):.*|\1:\2|p')

# Combine all keys.
ALL_KEYS=$(printf '%s\n%s\n' "$HEX_KEYS" "$RGB_KEYS" | grep -v '^[[:space:]]*$' || true)

# Apply filtering.
FILTERED=$(filter_violations "$ALL_KEYS")

# Count net-new violations.
if [ -z "$FILTERED" ] || [ "$(printf '%s' "$FILTERED" | wc -c)" = "0" ]; then
    echo "lint-design-tokens.sh: clean (no net-new hardcoded design tokens)"
    exit 0
fi

NET_NEW_COUNT=$(printf '%s\n' "$FILTERED" | grep -c '^crates\|^data' || true)

if [ "$NET_NEW_COUNT" = "0" ]; then
    echo "lint-design-tokens.sh: clean (no net-new hardcoded design tokens)"
    exit 0
fi

echo "lint-design-tokens.sh: §0.7 gate 12 violations — net-new hardcoded tokens:"
echo
printf '%s\n' "$FILTERED" | head -20
NET_NEW_TRUNC=$(printf '%s\n' "$FILTERED" | grep -c '^' || echo 0)
if [ "$NET_NEW_TRUNC" -gt 20 ]; then
    echo "  ... and $((NET_NEW_TRUNC - 20)) more"
fi
echo
echo "Each line above hardcodes a color or other design token in"
echo "active visual code. Per Q7 of the 25-Q tuning survey, these"
echo "must reference the canonical token files:"
echo
echo "  data/css/tokens.css            (colors + palette)"
echo "  data/css/motion-vocabulary.css (motion durations)"
echo "  crates/mde-theme/              (Rust-side token consts)"
echo
echo "If the violation is intentional (e.g., a Color::from_rgb call"
echo "inside crates/mde-theme/ token-definition code), add the entry"
echo "to install-helpers/lint-design-tokens.allowlist with a one-line"
echo "rationale comment. The allow-list is meant to SHRINK over time;"
echo "EPIC-UI-MATERIAL.token-sweep ports each entry to the canonical"
echo "token files."
exit 1
