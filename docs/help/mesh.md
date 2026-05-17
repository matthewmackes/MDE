# The Mesh — overview

The "mesh" is the peer-to-peer fabric Mackes provisions between your
machines. Up to 16 peers; works across LANs and over the public internet.

## Architecture

Five layered subsystems, all running through one daemon (`qnmd`):

| Layer | What it does | Backed by |
|---|---|---|
| **VPN** | Routes packets between peers regardless of network | Headscale + Tailscale (bootstrap-only) + WireGuard data plane |
| **Filesystem** | Live cross-mounted filesystems (`~/QNM-Mesh/<peer>/`) | SSHFS-over-QNM via `qnmd mesh-fs` module |
| **Sync** | Clipboard, notifications, snapshots, themes, generic blob-drop | NATS JetStream + NATS Object Store via `qnmd mesh-sync` module |
| **Services** | Discover Jellyfin / Airsonic / Plex / etc. on any peer | `qnmd mesh-services` + 5 access layers |
| **SSH** | `ssh peer.mesh` works between any two peers | OpenSSH + auto-distributed keys + Tailscale-SSH identity |

## Where mesh content shows up

- **Thunar** — `mesh:///` URI scheme exposes peers, clipboard, notifications,
  Object Store as browsable folders. See **[mesh-thunar.md](mesh-thunar.md)**.
- **Mackes Dashboard** — "Mesh activity" widget shows recent clipboard
  items, unread notifications, recently-dropped objects.
- **Whisker Menu** — "Quick Network Mesh" category surfaces QNM-specific
  actions (Browse Mesh, Show Peers, Start/Stop QNM, Open qnm-gui).
- **xfdesktop right-click** — "Drop on mesh…" entry opens the destination
  picker for a quick file send.
- **Headless `mackes` CLI** — `mackes peers`, `mackes shares`,
  `mackes status` etc. for terminal-only nodes.

## Capacity & limits

- **16-peer hard cap** (Q-MX18). 17th peer-add fails with a Carbon Toast
  reading "Mesh capacity (16/16)".
- Each peer auto-mirrors every other peer's clipboard ring (100 items
  each) + Saved/ (uncapped) + notifications + Object Store buckets.
- Worst case at full capacity: ~1600 cached clipboard items + replicated
  buckets across all peers. Practical disk use: ~1–5 GB per peer.

## Identity & trust

- **Single shared mesh keypair** (Q25 lock). One ed25519 keypair owns
  the mesh; all peers carry the same key. Compromise of one peer =
  compromise of the mesh. Security model favors simplicity over
  least-privilege — acceptable for personal / family / small-team
  meshes.
- **Mesh VPN** uses the shared key for auth + WireGuard handshake.
- **Mesh SSH** Layer A uses per-peer ed25519 keys distributed via NATS
  Object Store; Layer B uses Headscale's identity model (no SSH keys).

## Privacy

- Tailscale (used only for cross-network bootstrap, only by the seed
  peer) sees: the seed peer's current public IP and a tag (`tag:mackes-
  <mesh-id>`). It never sees other peers, never sees any traffic.
- DERP relays (Tailscale-operated) see: encrypted packet sizes + source/
  destination peer IDs. Cannot decrypt payload.
- NATS, SSHFS, Headscale all run *between* mesh peers — no external
  party sees their traffic.

## Sub-guides

- **[Mesh VPN](mesh-vpn.md)** — adding peers, control-node election, NAT
  traversal
- **[Mesh in Thunar](mesh-thunar.md)** — the `mesh:///` filesystem
- **[Mesh SSH](mesh-ssh.md)** — three-layer SSH model
- **[Mesh Media Services](mesh-services.md)** — discover Jellyfin/Airsonic/etc.
- **[Headless Node Mode](headless.md)** — running on a fileserver
