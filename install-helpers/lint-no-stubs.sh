#!/bin/sh
# install-helpers/lint-no-stubs.sh — TUNE-2 / 25-Q Q8 pre-commit gate #13.
#
# Catches §0.12 "no stubs / no skeletons / no staged work"
# violations in committed Rust code:
#
#   - todo!()
#   - unimplemented!()
#   - panic!("not yet …")
#   - panic!("todo …")
#   - panic!("not implemented …")
#
# Pairs with the voice-tone lint (gate #6, extended in TUNE-5)
# which catches user-visible "coming soon" / "TBD" / "WIP" / etc.
# strings. Together they enforce the §0.12 + 25-Q Q8 + Q9 lock.
#
# Per CLAUDE.md §0.7 gate #13.
#
# Exits 0 = clean, exits 1 = violations found.

set -eu

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

# Scan Rust source only. Python is being retired per Q49 + Q95;
# no need to catch new stub violations in dying code.
SCAN_INCLUDES='--include=*.rs'
SCAN_PATHS='crates/'

# Patterns that signal a stub. egrep-compatible.
PATTERNS='(\<todo!\s*\()|(\<unimplemented!\s*\()|(panic!\s*\(\s*"\s*not yet)|(panic!\s*\(\s*"\s*todo)|(panic!\s*\(\s*"\s*not implemented)'

# Snapshot allow-list. Empty at lint introduction (2026-05-26):
# grep across the live repo on 2026-05-26 found ZERO matches for
# the patterns above in crates/. The mde-popover Kind::Network
# arm CLAUDE.md §0.12 originally grandfathered has already
# retired. Going forward, any match is a regression.
#
# Format: one path per line, leading + trailing whitespace ignored.
# Lines starting with `#` are comments.
ALLOWLIST_PREFIXES='
'

TMPFILE="$(mktemp)"
trap 'rm -f "$TMPFILE"' EXIT

# `grep -rnE` for portable POSIX regex (no PCRE dep).
grep -rnE $SCAN_INCLUDES "$PATTERNS" $SCAN_PATHS > "$TMPFILE" 2>/dev/null || true

# Filter out allow-listed paths.
echo "$ALLOWLIST_PREFIXES" | while IFS= read -r prefix; do
    prefix="$(printf '%s' "$prefix" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
    case "$prefix" in
        ''|'#'*) continue ;;
    esac
    # Escape for sed pattern.
    esc="$(printf '%s' "$prefix" | sed 's/[\/&]/\\&/g')"
    sed -i.bak "/^${esc}/d" "$TMPFILE"
    rm -f "$TMPFILE.bak"
done

# Filter out lines inside #[cfg(test)] modules (tests can use
# unimplemented!() / todo!() as placeholders for test fixtures).
# This is approximate (sed line-based) but catches the common
# case: lines inside files whose path contains `/tests/` or where
# the immediate preceding context is `#[cfg(test)]`. For v1 of
# the lint we use the path-based heuristic.
sed -i.bak '/\/tests\//d' "$TMPFILE"
rm -f "$TMPFILE.bak"

# Filter out lines inside test modules (files ending in
# `*_tests.rs` or paths under any `tests/` subdirectory).
sed -i.bak '/_tests\.rs:/d' "$TMPFILE"
rm -f "$TMPFILE.bak"

if [ -s "$TMPFILE" ]; then
    echo "lint-no-stubs.sh: §0.12 violations found:"
    echo
    cat "$TMPFILE"
    echo
    echo "Each line above ships a stub (todo!() / unimplemented!() /"
    echo "panic-not-yet). Per CLAUDE.md §0.12 every commit must ship"
    echo "END-TO-END. Either complete the implementation, split the"
    echo "task at write-time so each sub-commit is complete, or add"
    echo "the path to the snapshot allow-list above with a one-line"
    echo "rationale comment."
    exit 1
fi

echo "lint-no-stubs.sh: clean (no net-new todo!() / unimplemented!() / panic-not-yet hits)"
exit 0
