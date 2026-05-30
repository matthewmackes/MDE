#!/bin/sh
# install-helpers/lint-visual-citation.sh — TUNE-9 / 25-Q Q6
# pre-commit gate #11 (visual-citation requirement).
#
# Every commit that touches a visual surface MUST include in
# its commit message:
#
#   Cite: <doc>.md §X.Y[; ref: <target>]
#
# Where:
#   <doc>.md is one of:
#     visual-identity.md
#     motion-language.md
#     chromeos-classic-spec.md
#     icon-mapping.md
#     audio-video-compliance.md
#   <target> (when present) is one of:
#     Apple System Settings | Linear | Raycast | Arc |
#     Vercel dashboard | Cursor
#
# Invocation:
#   - As a commit-msg hook: $1 is the commit message file path.
#   - As a stand-alone smoke test: no args, scans HEAD's message
#     against the staged-or-HEAD-touched files.
#
# Per CLAUDE.md §0.7 gate #11.
#
# Exits 0 = clean, exits 1 = violation.

set -eu

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

# Where to read the commit message from.
if [ -n "${1:-}" ] && [ -f "$1" ]; then
    MSG_FILE="$1"
elif [ -n "${GIT_COMMIT_MSG_FILE:-}" ] && [ -f "$GIT_COMMIT_MSG_FILE" ]; then
    MSG_FILE="$GIT_COMMIT_MSG_FILE"
else
    # Stand-alone mode: check HEAD's message against working-
    # tree-changed files. Useful for `make lint` style invocation.
    MSG_FILE="$(mktemp)"
    trap 'rm -f "$MSG_FILE"' EXIT
    git log -1 --format='%B' > "$MSG_FILE" 2>/dev/null || true
fi

# Which files trigger the citation requirement.
# Visual surfaces: Iced sources under crates/mde-*/src/, plus
# data/css/* tokens + design-doc CSS, plus the new presets
# directory (when EPIC-UI-PRESETS lands).
VISUAL_SURFACES_REGEX='^(crates/mde-[^/]+/src/.*\.rs$|data/css/.*\.css$|data/presets/.*$)'

# …but not every crates/mde-* is a UI. These are headless CLI /
# protocol / type / lib crates with NO Iced/visual surface, so a
# commit touching only them does not warrant a design-doc citation.
# Keep this list tight — only crates with genuinely no rendered UI.
#   mde-installer    — mde-install / mde-update CLI (INST-3)
#   mde-alert-emit   — MON-3 alert→JSON CLI
#   mde-kdc          — KDC2 host integration (TLS/D-Bus/protocol — no Iced UI)
#   mde-kdc-proto    — KDC2 wire-protocol library
#   mde-mesh-types   — shared mesh type definitions
#   mde-clipd        — BUS-5 Wayland clipboard daemon (no Iced UI)
#   mde-theme        — pure math/token library (palette, tween, ease, lerp);
#                      provides building blocks consumed by Iced surfaces but
#                      has no rendered Iced view() itself.
NON_VISUAL_CRATES_REGEX='^crates/(mde-installer|mde-alert-emit|mde-kdc|mde-kdc-proto|mde-mesh-types|mde-clipd|mde-theme)/'

# Get the staged files (commit-msg hook) OR HEAD's modified
# files (stand-alone mode).
if [ -n "${1:-}" ]; then
    CHANGED="$(git diff --cached --name-only 2>/dev/null || true)"
else
    CHANGED="$(git diff --name-only HEAD 2>/dev/null || git diff-tree --no-commit-id --name-only -r HEAD 2>/dev/null || true)"
fi

# Filter to visual surfaces, then drop the non-visual CLI/lib crates.
VISUAL_HITS="$(printf '%s\n' "$CHANGED" | grep -E "$VISUAL_SURFACES_REGEX" | grep -vE "$NON_VISUAL_CRATES_REGEX" || true)"

if [ -z "$VISUAL_HITS" ]; then
    echo "lint-visual-citation.sh: clean (no visual surface in this commit)"
    exit 0
fi

# Visual surfaces touched — citation required.
# Pattern: `Cite: <doc>.md §<section>`
#   optional `; ref: <target>` afterwards
if grep -Eq '^Cite: [a-z][-a-z0-9_]*\.md[[:space:]]*§' "$MSG_FILE"; then
    echo "lint-visual-citation.sh: clean (Cite: line present in commit message)"
    exit 0
fi

# Some commits are mass-retirements / deletions that don't
# warrant a citation (e.g., "delete dead module foo.rs" — no
# new visual design landed). Accept commit messages whose
# first line starts with a retirement-vocabulary verb as
# escaping the citation requirement.
SUBJECT="$(head -n 1 "$MSG_FILE" 2>/dev/null || echo '')"
case "$SUBJECT" in
    'Retire '*|'retire '*|'Delete '*|'delete '*|'Remove '*|'remove '*|'Drop '*|'drop '*|'Revert '*|'revert '*)
        echo "lint-visual-citation.sh: clean (retirement/deletion subject — citation waived)"
        exit 0
        ;;
esac

echo "lint-visual-citation.sh: §0.7 gate 11 violation."
echo
echo "Commit touches these visual surfaces:"
printf '%s\n' "$VISUAL_HITS" | sed 's/^/  /' | head -10
test "$(printf '%s\n' "$VISUAL_HITS" | wc -l)" -gt 10 && echo "  ... and more"
echo
echo "but the commit message does NOT contain a 'Cite:' line."
echo
echo "Per Q6 of the 25-Q tuning survey, every visual commit must"
echo "cite a design-doc section + a Material 3 reference target."
echo "Add a line like the following to the commit body:"
echo
echo "  Cite: visual-identity.md §1.2; ref: Apple System Settings"
echo "  Cite: motion-language.md §3; ref: Linear"
echo "  Cite: chromeos-classic-spec.md §4.1; ref: Raycast"
echo
echo "Accepted design docs:"
echo "  visual-identity.md / motion-language.md /"
echo "  chromeos-classic-spec.md / icon-mapping.md /"
echo "  audio-video-compliance.md"
echo
echo "Accepted reference targets:"
echo "  Apple System Settings / Linear / Raycast / Arc /"
echo "  Vercel dashboard / Cursor"
echo
echo "If the commit is genuinely a retirement / deletion / revert"
echo "(no new visual design), start the commit subject with one"
echo "of: Retire, Delete, Remove, Drop, Revert."
exit 1
