# Bench-acceptance fleet topology

The six v2.5 bench scenarios (NF-9.1..NF-9.6, locked in
`docs/design/v2.5-nebula-fabric.md`) execute against a fleet of
hosts the operator stands up. The harness in
`tests/acceptance/test_nebula_fabric.py` discovers the fleet by
reading the JSON file path supplied via the `MDE_NEBULA_BENCH_FLEET`
environment variable.

## Schema

```json
{
  "fleet_id": "bench-001",
  "hosts": [
    {"node_id": "host-a", "ssh": "user@10.0.0.10", "role": "host"},
    {"node_id": "host-b", "ssh": "user@10.0.0.11", "role": "peer"},
    {"node_id": "host-c", "ssh": "user@10.0.0.12", "role": "candidate"}
  ]
}
```

### Top-level keys

| Key        | Type   | Required | Description                                                      |
|------------|--------|----------|------------------------------------------------------------------|
| `fleet_id` | string | yes      | Mesh id passed to `mackesd mesh init` for NF-9.1.                |
| `hosts`    | array  | yes      | Ordered list of bench hosts. NF-9.1 always runs on `hosts[0]`.   |

### Per-host keys

| Key        | Type   | Required | Description                                                                 |
|------------|--------|----------|-----------------------------------------------------------------------------|
| `node_id`  | string | yes      | Stable identifier for the host. Used in promote/demote + lighthouse roster. |
| `ssh`      | string | yes      | `user@host` target. Key-auth only — `BatchMode=yes` is enforced.            |
| `role`     | string | no       | One of `host`, `peer`, `candidate`. Default: `peer`.                        |

### Role conventions

- `host` — bench host that runs as a Nebula lighthouse for the
  duration of the scenario suite. The harness assumes at least one
  is present; NF-9.1 runs on the first entry of `hosts`.
- `peer` — non-lighthouse member of the mesh. NF-9.2 picks the
  first non-host peer.
- `candidate` — peer used only by NF-9.5's promote/demote scenario.
  Tag at most one as `candidate`; if absent, NF-9.5 reuses
  `hosts[2]`.

## Scenario-by-scenario fleet requirements

| Scenario | Minimum hosts | Notes                                                                 |
|----------|---------------|-----------------------------------------------------------------------|
| NF-9.1   | 1             | Runs on `hosts[0]`.                                                    |
| NF-9.2   | 2             | Skipped if fewer than 2 hosts.                                         |
| NF-9.3   | 1             | Runs on `hosts[0]`. Default route's interface is flapped.              |
| NF-9.4   | 1             | UDP egress is blocked + restored on `hosts[0]`.                        |
| NF-9.5   | 3             | Promotes `hosts[2]`. Skipped if fewer than 3 hosts.                    |
| NF-9.6   | 2             | Kills mackesd on `hosts[0]`. Skipped if fewer than 2 hosts.            |

The 6-peer reference fleet locked in the v2.5 design doc satisfies
every scenario.

## SSH expectations

The harness invokes ssh with these options on every call:

- `BatchMode=yes` — never prompts for a password.
- `StrictHostKeyChecking=accept-new` — auto-trusts unseen hosts the
  first time, then enforces (the same flag is applied to `scp`).
- `ConnectTimeout` — set to the per-call timeout, capped at 10 s.

Bench hosts must have key-based authentication wired for the SSH
user named in each `ssh` field. The user must have passwordless
`sudo` for `mackesd`, `systemctl`, `iptables`, `ip`, `stat`, and
`busctl`.

### Out-of-band management network

NF-9.3 flaps the host's default-route interface. If the SSH
connection to that host shares the same physical interface, the
control channel drops mid-test. Bench fleets must therefore
provide either:

- A **second interface** for management (the `ssh` field points
  at the management IP, not the data-plane IP), or
- An **out-of-band BMC / IPMI / serial console** the operator can
  point at — set the `ssh` field at the BMC's gateway.

`link_flap.sh` mitigates by detaching the flap from the SSH
session (`nohup … & disown`), so the down-up cycle completes even
if the ssh connection drops, but the harness still needs to be
able to reconnect to poll the recovery SLO.

## Where to put the file

There is no enforced path. Common conventions:

- `/etc/mde/bench-fleet.json` (operator-shipped, root-owned 0600).
- `~/.config/mde/bench-fleet.json` (developer-local).

The path is whatever you set `MDE_NEBULA_BENCH_FLEET` to before
invoking pytest.
