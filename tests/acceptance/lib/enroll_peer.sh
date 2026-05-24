#!/usr/bin/env bash
# NF-9.2 helper — enroll the current host into a Nebula mesh.
#
# Invoked over ssh by tests/acceptance/test_nebula_fabric.py with the
# join token from the leader's `mackesd mesh show-join-token` call.
# Bash is the lingua franca on bench hosts — no Python dep required.
#
# Args:
#   $1 — join token (16-char passcode-shaped string)
set -euo pipefail

TOKEN="${1:?join token required as first arg}"

# `mackesd enroll` is the canonical Nebula-fabric enroll entrypoint
# (NF-3.x). It handles overlay-IP allocation, cert request, lighthouse
# discovery, and `nebula.service` activation in one shot.
sudo mackesd enroll --token "${TOKEN}"

# Surface the resulting overlay IP for the harness log; tests don't
# parse this but operators reading the bench transcript will.
sudo mackesd mesh show-overlay-ip
