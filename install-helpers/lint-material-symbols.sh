#!/bin/sh
# install-helpers/lint-material-symbols.sh — pre-commit gate #9
# (added 2026-05-25 per Q43 + Q97 + EPIC-PROC-LINT of the 100-Q
# tightening survey).
#
# Catches NET-NEW Carbon icon references in v1.0+ MackesDE code.
# Per Q43 the icon set pivots from Carbon → Material Symbols;
# per Q97 the migration must finish before the 1.0 rebrand cut.
# This gate catches Carbon regressions in code that has migrated.
#
# Allow-list strategy mirrors `lint-legacy-mesh.sh`: the source
# of the Carbon icon set itself (`data/icons/Mackes-Carbon/`)
# stays as historical asset until icon-set deletion (a separate
# DEAD-N task). The v1.x Python workbench (`mackes/workbench/`)
# is retiring entirely per Q49 in 1.0; its Carbon references are
# pre-existing, not net-new. The legacy GTK panel
# (`crates/mackes-panel/`) is frozen; same story.
#
# Per `.claude/CLAUDE.md` §0.7 gate #9.
#
# Exits 0 = clean, exits 1 = net-new Carbon references found.

set -eu

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

SCAN_INCLUDES='--include=*.rs --include=*.py --include=*.css --include=*.desktop'
SCAN_PATHS='crates/mde-applets/ crates/mde-portal/ crates/mde-files/ crates/mde-workbench/ crates/mde-popover/ crates/mde-panel/ crates/mde-peer-card/ crates/mde-drawer/ crates/mde-wizard/ crates/mde-session/ data/css/ data/applications/'

# Allow-listed path prefixes. Net-new Carbon references inside
# these are tolerated as historical / retiring code.
ALLOWED_PREFIXES='
data/icons/Mackes-Carbon/
mackes/workbench/
mackes/carbon/
crates/mackes-panel/
crates/mackes-theme/
crates/mde-panel/src/icon_mapper.rs
data/css/carbon-icons.css
data/css/carbon-
'

# Carbon icon patterns:
# - String literals naming Carbon icons (`"carbon-<name>"` or `"@carbon/icons-<name>"`)
# - Carbon CSS classes (`carbon--`, `bx--`, `cds--` prefix)
# - Imports from `@carbon/icons*` JS / TS / TSX packages (legacy panels-a.jsx etc.)
CARBON_PATTERNS='carbon-[a-zA-Z0-9-]+|@carbon/icons|bx--|cds--'

# Lines containing these comment-marker prefixes are allow-listed
# (talking ABOUT Carbon retirement, not REFERENCING Carbon).
# Pattern matches AFTER the `file:line:` prefix grep -n inserts.
COMMENT_PREFIXES=':[0-9]+:[[:space:]]*(///|//!|//|#|<!--)|//[[:space:]]*EPIC-UI-MATERIAL|//[[:space:]]*Q43|//[[:space:]]*retired|//[[:space:]]*legacy|//[[:space:]]*superseded|Material Symbols replaces|carbon-?retire|carbon-?deletion'

# Build the grep allow-list filter
ALLOW_FILTER=""
for prefix in $ALLOWED_PREFIXES; do
  [ -z "$prefix" ] && continue
  ALLOW_FILTER="${ALLOW_FILTER}|^${prefix}"
done
# Strip leading |
ALLOW_FILTER="${ALLOW_FILTER#|}"

# Scan and filter
violations=$(
  grep -rn -E "$CARBON_PATTERNS" $SCAN_INCLUDES $SCAN_PATHS 2>/dev/null \
    | grep -vE "$ALLOW_FILTER" \
    | grep -vE "$COMMENT_PREFIXES" \
    || true
)

if [ -n "$violations" ]; then
  echo "$0: net-new Carbon icon references detected (Q43 + Q97):"
  echo "$violations"
  echo ""
  echo "Per the 100-Q survey Q43, Carbon icons are retired in favor"
  echo "of Material Symbols. Migrate the reference, or if it lives in"
  echo "code that's also being retired, add it to the allow-list in"
  echo "this script with a comment citing the retiring epic."
  exit 1
fi

echo "$0: no net-new Carbon icon references — clean."
exit 0
