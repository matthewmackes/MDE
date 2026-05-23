#!/bin/sh
# install-helpers/lint-voice.sh — voice-and-tone verb-discipline +
# forbidden-strings gate.
#
# Locked by docs/design/voice-and-tone.md (v4.0.1). Enforces:
#
#   1. Forbidden marketing/celebratory strings: Oops, Whoops, Yikes,
#      Lorem ipsum, etc.
#   2. Forbidden placeholder strings reachable from user-visible
#      code: lorem, foo/bar/baz/qux, test123, placeholder.
#   3. Verb-discipline misuse in user-visible button-label-shaped
#      strings: e.g. "Create" / "New" where "Add" is the lock,
#      "Save" where "Apply" is the lock for config changes.
#
# Scans (per voice-and-tone.md §"Where this doc applies"):
#
#   - crates/mde-*/src/  (Iced views, panel labels)
#   - mackes/workbench/  (residual GTK surfaces)
#   - mackes/wizard/     (onboarding copy)
#   - data/applications/*.desktop (launcher Name= / Comment=)
#
# Excludes: comments (//, #), test code, `crates/mackes-panel/`
# (legacy GTK panel, frozen). The verb-discipline check is
# intentionally conservative — it targets clear button-label
# shapes (UPPER first letter, ≤ 3 words, ends with no punctuation)
# to avoid false positives in narrative prose / comments / log
# strings.
#
# Exit 0 = clean, exit 1 = violations found.
# Run as: install-helpers/lint-voice.sh [path...]

set -eu

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

if [ $# -gt 0 ]; then
    SCAN_PATHS="$*"
    ACTIVE_PATHS="$*"
else
    # All paths get the forbidden-strings checks (marketing words,
    # lorem ipsum, etc. shouldn't appear ANYWHERE).
    SCAN_PATHS="crates/mde-applets crates/mde-drawer crates/mde-files crates/mde-kdc crates/mde-logout-dialog crates/mde-panel crates/mde-peer-card crates/mde-popover crates/mde-session crates/mde-wizard crates/mde-workbench mackes/workbench mackes/wizard data/applications"
    # v4.0.2 cleanup: verb-discipline scans run only against active
    # surfaces. `mackes/workbench/*` and `mackes/wizard/*` are the
    # legacy v1.x GTK Python tree being retired by CB-1.x — their
    # button-label vocabulary predates the voice-and-tone lock and
    # won't be relabeled before retirement. Forbidden-strings stays
    # active there.
    ACTIVE_PATHS="crates/mde-applets crates/mde-drawer crates/mde-files crates/mde-kdc crates/mde-logout-dialog crates/mde-panel crates/mde-peer-card crates/mde-popover crates/mde-session crates/mde-wizard crates/mde-workbench data/applications"
fi

# Filter pattern for source lines: exclude comments + tests.
# Iced strings are typically inside text("..."), button(text("...")),
# .placeholder("...") — we grep across all and let the verb
# pattern + word-boundary do the filtering.

EXIT_CODE=0
TOTAL_VIOLATIONS=0
TMPFILE=$(mktemp)
trap 'rm -f "$TMPFILE"' EXIT

scan() {
    local label="$1"
    local pattern="$2"
    local description="$3"
    local args="$4"
    # 5th positional arg: which path set to scan. "active" for
    # verb-discipline checks (skips legacy Python tree),
    # "all" (default) for forbidden-strings.
    local path_set="${5:-all}"
    local paths
    case "$path_set" in
        active) paths="$ACTIVE_PATHS" ;;
        *)      paths="$SCAN_PATHS" ;;
    esac
    > "$TMPFILE"
    # shellcheck disable=SC2086
    grep -rn -E "$pattern" $args $paths 2>/dev/null \
        | grep -v 'voice-allow' \
        > "$TMPFILE" || true
    # v4.0.2 cleanup: per-line `voice-allow:<class>` annotation
    # silences a match. Use sparingly — only for cases that
    # comply with the lock's "destroy" exception or that ship
    # in legacy retired-surface paths the verb table doesn't
    # cover.
    if [ -s "$TMPFILE" ]; then
        local count
        count=$(wc -l < "$TMPFILE")
        TOTAL_VIOLATIONS=$((TOTAL_VIOLATIONS + count))
        printf '\n[%s] %d hit(s) — %s\n' "$label" "$count" "$description"
        head -10 "$TMPFILE"
        if [ "$count" -gt 10 ]; then
            printf '  ... %d more hits suppressed\n' $((count - 10))
        fi
        EXIT_CODE=1
    fi
}

# ──────────────────────────────────────────────────────────────
# Forbidden marketing / celebratory strings (locked §Forbidden)
# ──────────────────────────────────────────────────────────────

scan FORBIDDEN-MARKETING \
    '\b(Oops|Whoops|Yikes)\b' \
    'celebratory/apologetic words banned in user strings' \
    '--include=*.rs --include=*.py --include=*.desktop'

scan FORBIDDEN-LOREM \
    '\b(Lorem ipsum|dolor sit amet)\b' \
    'lorem ipsum placeholder reached production' \
    '--include=*.rs --include=*.py --include=*.desktop'

scan FORBIDDEN-FOO \
    '"(foo|bar|baz|qux)"' \
    'metasyntactic variables as visible strings (use real names)' \
    '--include=*.rs --include=*.py'

scan FORBIDDEN-TEST \
    '"(test123|testing123|placeholder)"' \
    'placeholder/test default values shipping in production' \
    '--include=*.rs --include=*.py --include=*.desktop'

# ──────────────────────────────────────────────────────────────
# Verb discipline (locked §Verb discipline)
# Targets clear button-label-shape strings only: capitalized
# first letter inside double-quotes, ends with quote (no
# punctuation), ≤ 3 words. Logs / errors / multi-sentence prose
# fall outside the shape and are not matched.
# ──────────────────────────────────────────────────────────────

# "Create" / "New" → use "Add"
# Match: "Create peer", "New file", "Create" etc.
scan VERB-CREATE-VS-ADD \
    '"\b(Create|New)\b( \w+){0,2}"' \
    'use "Add ..." not "Create/New ..." (voice-and-tone §Verb discipline)' \
    '--include=*.rs --include=*.py --include=*.desktop' \
    active

# "Delete X" where the action is removal-from-set, not destruction
# This is harder to disambiguate; flag bare "Delete" in button-shape
# strings (≤ 3 words) so the author can choose Remove or keep Delete.
scan VERB-DELETE-VS-REMOVE \
    '"\bDelete\b( \w+){0,2}"' \
    'consider "Remove ..." for set-removal; "Delete ..." reserved for destroy (voice-and-tone)' \
    '--include=*.rs --include=*.py --include=*.desktop' \
    active

# "Save" / "Confirm" → use "Apply" for config changes
scan VERB-SAVE-VS-APPLY \
    '"\b(Save|Confirm)\b( \w+){0,2}"' \
    'use "Apply ..." not "Save/Confirm ..." for config changes (voice-and-tone)' \
    '--include=*.rs --include=*.py --include=*.desktop' \
    active

# "Stop" / "Abort" → use "Cancel"
scan VERB-STOP-VS-CANCEL \
    '"\b(Abort)\b( \w+){0,2}"' \
    'use "Cancel ..." not "Abort ..." (voice-and-tone)' \
    '--include=*.rs --include=*.py --include=*.desktop' \
    active

# "Execute" / "Trigger" / "Launch" → use "Run"
scan VERB-EXECUTE-VS-RUN \
    '"\b(Execute|Trigger)\b( \w+){0,2}"' \
    'use "Run ..." not "Execute/Trigger ..." (voice-and-tone)' \
    '--include=*.rs --include=*.py --include=*.desktop' \
    active

# ──────────────────────────────────────────────────────────────
# Summary
# ──────────────────────────────────────────────────────────────

if [ "$EXIT_CODE" -eq 0 ]; then
    echo "lint-voice.sh: clean ($SCAN_PATHS)"
else
    printf '\nlint-voice.sh: %d total violation(s) across the scanned paths.\n' "$TOTAL_VIOLATIONS" >&2
    printf 'See docs/design/voice-and-tone.md for the locked verb table + forbidden strings.\n' >&2
fi

exit "$EXIT_CODE"
