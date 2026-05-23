#!/bin/bash
# sync-user-sway-exec-lines.sh — v4.0.1 BUG-11 systemic fix.
#
# Some users' ~/.config/sway/config was copied at v1.x by the birthright
# wizard and never refreshed when later MDE versions added new
# `exec mde-*` lines to data/sway/config. The most common symptom:
# the Win10 watermark + toast popovers never spawn (their `exec
# mde-popover {watermark,toast}` lines exist in the shipped config but
# not in the user's home copy).
#
# This script idempotently appends every required `exec mde-popover *`
# line that's MISSING from ~/.config/sway/config. It NEVER:
#   - reorders existing lines,
#   - removes user customizations,
#   - copies the whole system config over the user's,
#   - touches anything beyond the named exec lines.
#
# Each line we own is fingerprinted by an exact-match `grep -Fq` against
# the user's file. If the line is already present (with any leading
# whitespace), it's left alone.
#
# Invoked from mde-session.service ExecStartPost on every login.

set -u

USERCFG="${HOME}/.config/sway/config"

# If the user's config doesn't exist yet (fresh login on a brand-new
# account) the birthright wizard hasn't fired — bail out, the wizard
# will write the right file.
[ -f "$USERCFG" ] || exit 0

# Required exec lines, in the order they should appear if appended.
# Adding a new entry here makes it stick on the next login of every
# existing operator without forcing them through the wizard again.
REQUIRED_LINES=(
    "exec mde-popover watermark"
    "exec mde-popover toast"
)

append_if_missing() {
    local line="$1"
    if grep -Fq "$line" "$USERCFG"; then
        return 0
    fi
    {
        printf '\n# v4.0.1 BUG-11 sync — appended by sync-user-sway-exec-lines.sh\n'
        printf '%s\n' "$line"
    } >>"$USERCFG"
    # Reload sway so the line takes effect this session, not just the
    # next one. `swaymsg reload` re-reads the config + re-runs exec
    # lines; `|| true` so a missing swaymsg (running outside sway)
    # doesn't fail the session bring-up.
    swaymsg reload >/dev/null 2>&1 || true
}

for line in "${REQUIRED_LINES[@]}"; do
    append_if_missing "$line"
done
