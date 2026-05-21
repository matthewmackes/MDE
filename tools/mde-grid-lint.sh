#!/usr/bin/env bash
# UX-12 (2026-05-21) — grid lint for the modular spacing scale.
#
# Scans crates/mde-*/src/ for hardcoded pixel literals at
# spacing-relevant call sites:
#
#   - .padding(<n>)             — widget padding
#   - .spacing(<n>)             — column / row spacing
#
# Allowed values come from the NFU-1 lock:
#   4 / 6 / 8 / 10 / 14 / 17 / 20 / 24 / 28 / 34 / 40 / 48
# Plus 0 (no padding/spacing is a valid choice, not a violation).
#
# COMPONENT DIMENSIONS — `width`, `height`, `Length::Fixed`,
# `Length::FillPortion` — are intentionally NOT linted. UX-24
# sub-lock: component sizes (32 px nav row, 36 px button, 240 px
# sidebar, etc.) are locked separately and are not required to
# fit the spacing scale.
#
# Exit codes:
#   0  no violations (or --warn-only mode + non-zero violations)
#   1  ≥ 1 violation (default strict mode)
#
# Usage:
#   tools/mde-grid-lint.sh                       scan all mde-* crates strictly
#   tools/mde-grid-lint.sh path …                scan specified paths only
#   tools/mde-grid-lint.sh --debug               include ok'd lines in output
#   tools/mde-grid-lint.sh --warn-only           print violations but exit 0
#
# DEFAULT (today): --warn-only mode. The lint reports but doesn't
# block. After UX-2..UX-9 migrate consumer sources to mde-theme
# tokens, flip the default to strict. See UX-12 in the worklist.

set -u

TOKENS_RE='^(0|4|6|8|10|14|17|20|24|28|34|40|48)$'
SKIP_RE='/(target|build|dist|node_modules|.git)/'

DEBUG=0
WARN_ONLY=1  # default: warn-only until UX-2..UX-9 land
PATHS=()
for arg in "$@"; do
    case "$arg" in
        --debug)     DEBUG=1 ;;
        --warn-only) WARN_ONLY=1 ;;
        --strict)    WARN_ONLY=0 ;;
        *)           PATHS+=("$arg") ;;
    esac
done

if [ ${#PATHS[@]} -eq 0 ]; then
    if [ -d crates ]; then
        while IFS= read -r d; do
            PATHS+=("$d")
        done < <(find crates -maxdepth 1 -type d -name 'mde-*' 2>/dev/null)
    fi
fi

if [ ${#PATHS[@]} -eq 0 ]; then
    echo "mde-grid-lint: no paths to scan (run from repo root or pass paths)" >&2
    exit 0
fi

# Only spacing-relevant call sites. POSIX ERE.
PATTERN='\.(padding|spacing)\([[:space:]]*([0-9]+)'

# Use a temp file to count violations (subshell propagation issue
# with `while read` piped from grep otherwise).
tmp_violations="$(mktemp -t mde-grid-lint.XXXXXX)"
trap 'rm -f "$tmp_violations"' EXIT

while IFS= read -r file; do
    [[ "$file" =~ $SKIP_RE ]] && continue
    grep -nHE "$PATTERN" "$file" 2>/dev/null | while IFS= read -r line; do
        path="${line%%:*}"
        rest="${line#*:}"
        lineno="${rest%%:*}"
        content="${rest#*:}"
        n="$(echo "$content" | grep -oE "$PATTERN" | grep -oE '[0-9]+' | head -1)"
        if [ -z "$n" ]; then
            continue
        fi
        if echo "$n" | grep -qE "$TOKENS_RE"; then
            [ "$DEBUG" = "1" ] && echo "  ok  $path:$lineno  $n"
            continue
        fi
        # Snap to nearest token for the hint
        nearest="$(awk -v t="$n" 'BEGIN {
            split("4 6 8 10 14 17 20 24 28 34 40 48", a, " ");
            best=a[1]; bd=(t-a[1]); if (bd<0) bd=-bd;
            for (i=2; i<=12; i++) {
                d=(t-a[i]); if (d<0) d=-d;
                if (d<bd) { best=a[i]; bd=d }
            }
            print best
        }')"
        printf "  ✗ %s:%s  %d  (did you mean %s?)\n" "$path" "$lineno" "$n" "$nearest" >&2
        echo "x" >> "$tmp_violations"
    done
done < <(find "${PATHS[@]}" -name '*.rs' -type f 2>/dev/null)

violations="$(wc -l < "$tmp_violations" | tr -d ' ')"

if [ "$violations" -gt 0 ]; then
    echo "" >&2
    echo "mde-grid-lint (UX-12): $violations off-grid spacing literal(s)" >&2
    echo "  Allowed tokens (NFU-1): 4 6 8 10 14 17 20 24 28 34 40 48 (and 0)" >&2
    echo "  See docs/design/visual-identity.md § 10." >&2
    if [ "$WARN_ONLY" = "1" ]; then
        echo "  (warn-only mode — exiting 0; pass --strict to gate)" >&2
        exit 0
    fi
    exit 1
fi

echo "mde-grid-lint: ok (0 violations)"
exit 0
