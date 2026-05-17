#!/usr/bin/env bash
# mesh-ca-trust.sh — install Mackes' mesh-gateway CA root into the
# system trust store. Required for browsers to trust
# https://media.mesh URLs without warnings.
#
# Idempotent: re-running with the same cert is a no-op.

set -euo pipefail

for candidate in \
    /var/lib/caddy/.local/share/caddy/pki/authorities/local/root.crt \
    /etc/caddy/pki/authorities/local/root.crt \
    /var/lib/mackes/ca/mesh-ca.crt
do
    if [[ -f "$candidate" ]]; then
        SRC="$candidate"
        break
    fi
done

if [[ -z "${SRC:-}" ]]; then
    echo "ERROR: no mesh CA root found. Start the gateway first:"
    echo "  mackes services enable-gateway"
    exit 1
fi

DEST="/etc/pki/ca-trust/source/anchors/mackes-mesh-ca.crt"
install -m 0644 "$SRC" "$DEST"
echo "installed $SRC -> $DEST"

update-ca-trust extract
echo "system trust store updated"
