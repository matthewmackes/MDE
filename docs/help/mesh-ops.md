# Mesh operator runbook

Operator-facing reference for the day-2 tasks of running a Mackes
mesh. Covers enrollment, decommission, passcode rotation,
split-brain recovery, and reading the audit log. Pairs with the
architecture overview in `docs/design/v12.0-enterprise-mesh.md`.

## Enrolling a peer

Every new peer needs to register with the mesh's leader.

### From the new peer

```bash
# Get the shared 16-character passcode from the Host operator
# (it lives in libsecret on the Host; `mackesd show-passcode`
# prints it after the AdminSession prompt).
mackesd enroll --passcode <16-character-code>
```

The command is **idempotent by hardware fingerprint** — re-running
on the same machine refreshes credentials without creating a
duplicate node row.

### What enrollment does

1. Generates an Ed25519 keypair at `~/.local/share/mackes/node.key`
   (per 12.3.2).
2. Hashes the public key + machine-id and registers the fingerprint
   in the leader's `nodes` table.
3. Issues a per-node bearer token + a Tailscale auth key.
4. Starts `mackesd.service` + connects the peer to the Headscale
   coordinator.

A successful enrollment ends with the peer reporting `healthy` to
the leader's next heartbeat aggregation tick (≤ 10 seconds).

## Decommissioning a peer

When a peer is permanently leaving the mesh:

```bash
mackesd decommission <node-id>          # graceful
mackesd decommission <node-id> --force  # if the peer is unreachable
```

The leader revokes the bearer token, asks Tailscale to expire the
node, and marks the row decommissioned. **The historical row is
preserved** — audit + topology snapshots that referenced the
node still resolve.

If the node ever comes back (maybe a laptop that was offline for
months), the operator can re-enroll it without losing the
historical link — `mackesd reenroll <node-id>` issues fresh
credentials against the existing row.

## Rotating the passcode

The shared 16-character passcode gates both peer enrollment AND
service-to-service authentication (per 12.10.1). Rotate it any
time the operator suspects compromise, or on a fixed schedule.

```bash
# On the Host peer (the only one that holds the canonical libsecret entry):
mackesd rotate-passcode
```

After the rotation:

1. The new passcode is written to libsecret as
   `org.mackes.mesh.passcode`.
2. Every enrolled peer gets a fresh bearer token on its next
   heartbeat (≤ 10 seconds).
3. Offline peers require **manual re-entry** — they can't pull the
   new code through the mesh because their old token is dead.

**Show the new passcode to the operator once.** It's not recoverable
from libsecret without the AdminSession prompt.

## Recovering from split-brain

Two peers both think they're leader. Symptoms:

- `mackesd healthz` on both peers reports `is_leader: true`.
- The shared `~/QNM-Shared/.mackesd-leader.lock` has a contested
  state.
- Recent `applied_changes` rows diverge between the two leaders'
  stores.

### The automatic path (per 12.A.5)

On lease conflict, **the side with the older `applied_revision`
yields automatically**:

1. Detects the conflict via the lockfile.
2. Marks its in-memory state stale.
3. Re-reads the store from the side with the newer revision.
4. Resumes as a follower.

This usually completes within one lease cycle (≤ 60 seconds).

### Manual intervention

If automatic resolution fails (e.g. both peers crashed mid-write
and the lockfile is broken):

```bash
# On the peer you want to keep as leader:
mackesd take-leadership --force

# On the other peer:
mackesd yield-leadership

# Re-verify:
mackesd healthz | jq '.is_leader, .applied_revision'
```

`take-leadership --force` rewrites the lockfile with the current
peer's node-id and bumps the lease epoch by 1, so any other peer
with a stale lease will yield on its next renewal attempt.

## Reading the audit log

Every config change, auth event, and lifecycle action lands in the
`events` table with a hash-chained `prev_hash` field (per 12.6.3).

```bash
# Tail the log live:
mackesd logs --kind=audit

# Verify the hash chain:
mackesd audit verify

# Filter by node + time range:
mackesd events --node <node-id> --since '2026-05-19 09:00'
```

`mackesd audit verify` walks the chain forward and reports the
first row whose `prev_hash` doesn't match the previous row's hash.
A failed verify is a serious finding — it means either the store
was tampered with directly or there's a `mackesd` bug; do not
trust audit data past the break point.

## Common diagnostics

### A peer shows `unreachable` but ping works

The peer is reachable on the network but `mackesd` hasn't
heartbeated in ≥ 30 seconds (three missed cycles). Check:

```bash
ssh <peer> systemctl --user status mackesd
ssh <peer> journalctl --user -u mackesd -n 50
```

Common causes: `mackesd.service` crashed, the QNM-Shared mount is
broken, or the peer's SQLite file is locked by a hung process.

### Drift surfaces but never auto-repairs

The reconciler only auto-repairs drift whose `severity` is
`auto-repairable` AND policy allows. If a drift row has
`severity = manual-review`, surface it in the panel's Pending
Changes inbox; the operator must approve the repair before it
fires.

### Telemetry latency numbers look stale

Link telemetry lands in `topology_link_health` every 30 seconds
per 12.6.2. If a peer's metrics haven't moved in > 2 minutes, its
local prober is stuck — restart `mackesd` on that peer.

### What if my mesh can't reach the internet over UDP

Some corporate / hotel / captive-portal networks block outbound
UDP entirely while leaving TCP/443 open. On the v2.5 Nebula
fabric, the explicit `Https443` transport (Phase 12.18) has been
**rerouted to the Nebula lighthouse-relay tunnel introduced in
NF-1**: the lighthouse runs a TLS-wrapped TCP/443 listener and
relays UDP-style Nebula frames over it whenever a peer's
direct-UDP and lighthouse-UDP paths both fail.

Operator-side that means:

- No separate `mde-https-fallback` daemon to enable. The
  lighthouse handles it; peers fail over automatically once the
  router worker's HTTPS-fallback state machine fires (same
  policy thresholds as the legacy 12.18 path — 3 consecutive
  direct-UDP + lighthouse-UDP failures).
- The `MDE_HTTPS_FALLBACK_HOST` env var is replaced by the
  lighthouse's `nebula.https_relay.host` config key; the v2.5
  migration tool copies the old value over on first boot.
- Diagnostics: `mackesd healthz | jq .transport.active` now
  reports `nebula_https443` instead of the legacy `https_443`.
