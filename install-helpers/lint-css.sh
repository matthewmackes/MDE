#!/bin/sh
# install-helpers/lint-css.sh — pre-commit gate for data/css/*.css.
#
# Called from .claude/CLAUDE.md §0.7 when any data/css/ file is touched.
# Loads each CSS file through GtkCssProvider and reports any parsing
# error that GTK emits. Three classes of warnings are whitelisted as
# pre-existing (since 1.1.0 Carbon refresh) and not failure conditions:
#   - 'text-transform' (GTK CSS doesn't implement it)
#   - 'font-feature-settings' value parsing quirk
#   - 'cursor' property warnings
#
# Exit 0 on success (no new errors), 1 on real syntax errors.
# Run as: install-helpers/lint-css.sh [path...]
# With no args, lints data/css/*.css.

set -eu

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

if [ "$#" -eq 0 ]; then
    set -- $(ls data/css/*.css 2>/dev/null)
fi

if [ "$#" -eq 0 ]; then
    echo "lint-css: no CSS files to lint"
    exit 0
fi

python3 - "$@" <<'PY'
import sys

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk

# Substrings that flag a pre-existing, accepted warning. The actual
# GTK error messages include these tokens.
WHITELIST = (
    "text-transform",         # GTK CSS does not implement this
    "font-feature-settings",  # value-parsing quirk in tokens.css
    "cursor",                 # accepted, just noisy
    "line-height",            # GTK CSS computes line-height itself
)

failed = False
for path in sys.argv[1:]:
    errors = []
    def on_error(provider, section, error, _errors=errors):
        msg = error.message
        if any(w in msg for w in WHITELIST):
            return
        _errors.append(msg)
    p = Gtk.CssProvider()
    p.connect("parsing-error", on_error)
    try:
        p.load_from_path(path)
    except Exception as e:
        # Some GTK errors raise rather than emit. Treat exception body.
        msg = str(e)
        if not any(w in msg for w in WHITELIST):
            errors.append(msg)
    if errors:
        failed = True
        print(f"FAIL  {path}")
        for m in errors:
            print(f"   - {m}")
    else:
        print(f"OK    {path}")

sys.exit(1 if failed else 0)
PY
