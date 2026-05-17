# Mesh SSH

Three layers that ship together — together they give you "ssh anywhere
in the mesh, no friction".

## Layer 0 — Raw SSH cheatsheet

`ssh user@peer.mesh` works day one because the Mesh VPN + MagicDNS give
every peer routability and name resolution.

Mackes → Network → Mesh SSH → **Cheatsheet** renders a copy-paste table
generated from the live peer registry:

```
ssh mm@laptop-mm.mesh        # your laptop
ssh mm@desktop-mm.mesh       # workstation
ssh mm@phone-mm.mesh         # SSH from phone (Termux etc.)
ssh root@headless-server.mesh # fileserver
```

You manage `~/.ssh/id_ed25519` and `~/.ssh/authorized_keys` per usual.
Standard SSH all the way down. Works with scp, rsync, sshfs, mosh, VS
Code Remote — anything that speaks SSH.

## Layer A — Auto-distributed keys via NATS

Mackes generates `~/.ssh/mackes_mesh_ed25519` on each peer at install
time. The pubkey is published to the `mesh.ssh-keys` NATS Object Store
bucket keyed by peer-id. qnmd subscribes on every peer; on receive, it
appends the remote pubkey to the configured target user's
`~/.ssh/authorized_keys` bracketed with surgical markers:

```
# managed-by-mackes-mesh-<peer-id> begin
ssh-ed25519 AAAA... mackes-mesh-key-laptop-mm
# managed-by-mackes-mesh-<peer-id> end
```

Result: `ssh peer.mesh` from any peer to any peer just works, no manual
key exchange. New peer joins → its pubkey propagates mesh-wide in
seconds. Peer leaves → its pubkey is removed from every other peer's
authorized_keys.

### Username mapping

By default, mesh-distributed keys are appended to *the wizard-running
user's* `authorized_keys` on each peer. Mackes → Network → Mesh SSH →
**Key Distribution** lets you pick a different target user per peer (e.g.
`root` on the fileserver, `mm` everywhere else).

### Opt-out

Per-peer opt-out toggle ("Don't accept auto-keys from this peer") in the
Key Distribution sub-panel.

## Layer B — Identity-based SSH (Tailscale SSH via Headscale)

For users who prefer no key management at all: Headscale's experimental
Tailscale-SSH support is enabled by default in Mackes 1.0. `ssh
mesh-username@peer.mesh` authenticates via the mesh identity itself —
the same key Headscale uses for the WireGuard tunnel. No SSH keys
involved.

ACLs configured visually in Mackes → Network → Mesh SSH → **Access
Policy**:

```yaml
ssh:
  - action: accept
    src:    [tag:mackes-admin]
    dst:    ['*']
    users:  [root, mm]
  - action: accept
    src:    [tag:mackes-user]
    dst:    [tag:mackes-fileserver]
    users:  [mm]
```

Every accepted SSH session writes to the `mesh.ssh-audit` NATS bucket
(timestamp, source peer, source user, target peer, target user,
session-id, exit-status).

## Mackes SSH UI

Mackes → Network → Mesh SSH has four sections:

1. **Discovered Peers** — Carbon Tile per peer with "Open Terminal"
   button. Defaults to identity-based session (Layer B); falls back to
   Layer A keys if Headscale SSH is unavailable.
2. **Key Distribution** — Layer A status; per-peer toggle.
3. **Access Policy** — Layer B visual ACL editor.
4. **Audit Log** — DataTable of the last 1000 SSH session records from
   NATS.

## CLI

`mackes ssh <peer-name>` opens a session against the named peer (auto-
selects Layer B if available, Layer A otherwise). Lives in `mackes/cli/
mesh_ssh.py` for both GUI and headless installs.

## Security notes

- The **shared mesh keypair** (Q25 lock) is the trust root for *mesh
  membership*, not for SSH sessions. SSH keys (Layer A) and Tailscale
  identities (Layer B) are separate credentials carried over the mesh
  fabric.
- Layer A's per-peer keys are scoped to the mesh — they don't appear in
  your default `~/.ssh/id_ed25519` and don't get propagated outside
  Mackes peers.
- Layer B's audit log is durable in NATS; persists across control-node
  failover.
- SSH still binds to port 22 on every peer; if you want it firewalled
  outside the mesh, set firewalld to block 22/tcp on non-mesh zones.
