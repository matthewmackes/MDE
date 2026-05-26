#!/usr/bin/env bash
# .claude/hooks/commit-msg.sh — installed as .git/hooks/commit-msg
# via `make install-hooks`.
#
# Runs commit-msg-class lints AFTER the commit message is
# written + BEFORE the commit lands. Currently invokes:
#
#   install-helpers/lint-visual-citation.sh   (gate #11)
#
# Each lint receives the commit-message file path as $1 (the
# standard commit-msg-hook contract).

set -u

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MSG_FILE="$1"

# Gate #11 — visual-citation lint (TUNE-9 / 25-Q Q6).
if [ -x "${REPO_ROOT}/install-helpers/lint-visual-citation.sh" ]; then
    if ! "${REPO_ROOT}/install-helpers/lint-visual-citation.sh" "$MSG_FILE"; then
        exit 1
    fi
fi

exit 0
