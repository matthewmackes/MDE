#!/bin/sh
# install-helpers/lint-public-ports.sh — pre-commit gate #10
# (added 2026-05-26 per Q60 + EPIC-SEC-PUBLIC-PORT-LINT of the
# 100-Q tightening survey).
#
# Catches NET-NEW listeners that bind a public-facing port outside
# the locked allow-list. Per Q60 of the 100-Q survey, the only
# public ports MDE peers expose are:
#
#   - UDP/4242  — Nebula overlay (lighthouse listens; peers may
#                 also bind for NAT-traversal hole-punch)
#   - TCP/443   — Nebula HTTPS-tunnel fallback (lighthouses only,
#                 for peers behind UDP-blocking firewalls)
#
# Every other listener MUST bind on the Nebula overlay interface
# (e.g., `nebula0` / the overlay IP) — never on `0.0.0.0` or a
# real public interface. BUS-1.2 ntfy brokers, gluster mountd,
# mackesd D-Bus, KDC2 listeners — all overlay-only.
#
# Per `.claude/CLAUDE.md` §0.7 gate #10.
#
# Exits 0 = clean, exits 1 = net-new public-port binds.

set -eu

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

# Scan source + config that could declare listeners
SCAN_INCLUDES='--include=*.rs --include=*.py --include=*.toml --include=*.yaml --include=*.yml --include=*.conf --include=*.service'
SCAN_PATHS='crates/ mackes/ data/'

# Patterns that suggest a public-facing listener:
# - `bind` to `0.0.0.0` (catch-all)
# - `bind` to `[::]` (IPv6 catch-all)
# - sshd_config `ListenAddress 0.0.0.0`
# - systemd `BindAddress=0.0.0.0`
# - Dockerfile/podman `EXPOSE` (cross-checked against the allow-list)
# - explicit string literal "0.0.0.0" in source — strong signal
PATTERNS='bind.*"?0\.0\.0\.0|bind.*"?\[::\]|ListenAddress[[:space:]]+0\.0\.0\.0|BindAddress=0\.0\.0\.0|^EXPOSE[[:space:]]|"0\.0\.0\.0"'

# Allow-listed path prefixes — files where 0.0.0.0 binds are
# legitimate. Snapshot taken 2026-05-26 against the pre-existing
# tree per the lint-introduction discipline used by
# lint-dbus-shape.sh + lint-material-symbols.sh. Each entry below
# carries an inline rationale comment so future security audits
# can re-evaluate; net-new files NOT in this list trigger the gate.
ALLOWED_PREFIXES='
crates/mackesd/src/workers/nebula_supervisor.rs
crates/mackesd/src/workers/nebula_https_listener.rs
crates/mackesd/src/workers/sshd_overlay_bind.rs
crates/mackesd/src/workers/firewall_preset.rs
crates/mackes-nebula-https-tunnel/
crates/mackesd/src/https_fallback.rs
crates/mackesd/src/transport/https443.rs
crates/mackesd/src/topology/
data/nebula/
data/systemd/nebula
mackes/headscale_postgres.py
mackes/mesh_nebula.py
crates/mackes-panel/
tests/
crates/mackesd/tests/
crates/mackesd/src/workers/lan_discovery.rs
crates/mackesd/src/workers/voice_config.rs
crates/mackesd/src/workers/wol.rs
crates/mackesd/src/voice/materialize.rs
crates/mde-voice-config/src/lib.rs
mackes/wizard/pages/network.py
'

# Snapshot-allow-list rationales (security-audit reference):
#
# - workers/lan_discovery.rs : LAN-discovery probe binds 0.0.0.0:0
#   on the local network for mDNS scanning. Test-only binds.
# - workers/voice_config.rs : voice config test fixtures with the
#   "0.0.0.0" placeholder mesh_bind_address (in-source #[cfg(test)]).
# - workers/wol.rs : WoL magic packet REQUIRES a broadcast UDP
#   socket bound to 0.0.0.0:0 (RFC 2153 / WoL spec). Not a listener.
# - voice/materialize.rs : default mesh_bind_address placeholder
#   gets replaced with the per-peer overlay IP by voice_config.rs
#   on materialize; "0.0.0.0" is a fallback for tests + initial
#   bootstrap before Nebula enrolment publishes the overlay IP.
# - mde-voice-config/src/lib.rs : same — default config struct
#   placeholder that the materializer overrides.
# - wizard/pages/network.py : port-availability probe binds the
#   port briefly + immediately closes; not a long-lived listener.

# Comment-line allow-list (talking ABOUT public ports, not binding them).
# Pattern matches AFTER grep's `file:line:` prefix injection.
COMMENT_PREFIXES=':[0-9]+:[[:space:]]*(///|//!|//|#|<!--|/\*|\*)|"0\.0\.0\.0"[[:space:]]*//|allow-list|allowlist|carve-out|retired|legacy|superseded|allow this'

# Build the grep allow-list filter
ALLOW_FILTER=""
for prefix in $ALLOWED_PREFIXES; do
  [ -z "$prefix" ] && continue
  ALLOW_FILTER="${ALLOW_FILTER}|^${prefix}"
done
ALLOW_FILTER="${ALLOW_FILTER#|}"

violations=$(
  grep -rn -E "$PATTERNS" $SCAN_INCLUDES $SCAN_PATHS 2>/dev/null \
    | grep -vE "$ALLOW_FILTER" \
    | grep -vE "$COMMENT_PREFIXES" \
    || true
)

if [ -n "$violations" ]; then
  echo "$0: net-new public-port binds detected (Q60 + EPIC-SEC-PUBLIC-PORT-LINT):"
  echo "$violations"
  echo ""
  echo "Per the 100-Q survey Q60, MDE peers must bind every listener"
  echo "on the Nebula overlay interface (e.g. nebula0 / the overlay"
  echo "IP), NEVER on 0.0.0.0 or a real public interface. The only"
  echo "public ports allowed are UDP/4242 (Nebula) + TCP/443 (HTTPS"
  echo "fallback on lighthouses). If your listener legitimately"
  echo "needs a public bind, add its source file to the allow-list"
  echo "in this script with a comment citing the security rationale."
  exit 1
fi

echo "$0: no net-new public-port binds — clean."
exit 0
