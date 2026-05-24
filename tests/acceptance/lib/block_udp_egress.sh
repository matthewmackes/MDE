#!/usr/bin/env bash
# NF-9.4 helper — block all UDP egress on the current host.
#
# Used by tests/acceptance/test_nebula_fabric.py to assert that the
# HttpsFallbackState transitions Nebula traffic to its TCP/443 path
# within 30 s of UDP becoming unreachable.
#
# The companion restore_udp_egress.sh undoes this rule. The harness
# always pairs the two — leaving the rule in place would poison every
# subsequent scenario.
set -euo pipefail

# Tag the rule with a comment so restore_udp_egress.sh can find it
# even if other iptables rules accumulate during the bench window.
sudo iptables -A OUTPUT -p udp -j DROP -m comment --comment "nf9.4-bench"

# Surface a status snapshot for the bench transcript.
sudo iptables -S OUTPUT | grep nf9.4-bench || true
