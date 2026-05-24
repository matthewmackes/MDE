#!/usr/bin/env bash
# NF-9.4 helper — remove the UDP-egress block applied by
# block_udp_egress.sh.
#
# The harness invokes this in a try/finally around the NF-9.4
# scenario so a failed assertion doesn't leave the bench host
# permanently UDP-blackholed.
set -euo pipefail

# Delete every OUTPUT rule whose comment matches our tag. Looping
# because `iptables -D` only removes one rule per call and we want
# to be robust to stacked-rule mistakes during operator iteration.
while sudo iptables -C OUTPUT -p udp -j DROP \
        -m comment --comment "nf9.4-bench" >/dev/null 2>&1; do
    sudo iptables -D OUTPUT -p udp -j DROP \
        -m comment --comment "nf9.4-bench"
done

# Surface the resulting chain for the bench transcript.
sudo iptables -S OUTPUT
