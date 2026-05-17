# Mesh VPN

The VPN layer keeps peers connected regardless of physical network.

## Architecture

- **Data plane**: WireGuard tunnels between every peer pair
- **Control plane**: Self-hosted **Headscale** (open-source Tailscale
  control server). One peer at a time hosts it; the role auto-fails over.
- **NAT traversal**: Tailscale-operated DERP relays (free public
  infrastructure) when direct UDP fails
- **DNS**: MagicDNS — every peer reachable as `<hostname>.mesh`

## Mesh IPs

Every peer gets a stable `100.64.x.x` mesh IP assigned by Headscale.
The IP survives network changes — your laptop has the same mesh IP at
home, at the coffee shop, and on cellular tethering.

## Adding peers

### Same LAN

1. Install Mackes on the new peer.
2. Wizard's Network screen detects the existing mesh via mDNS.
3. One click on **Join existing mesh on `laptop-mm.local`**.
4. 5–10 seconds later: connected.

No URLs typed, no codes entered, no Tailscale signin on the new peer.

### Cross-network

1. On any existing peer, open Mackes → Network → Mesh VPN → **Add Peer**.
2. Modal shows a QR code + paste-link valid for 10 minutes:
   `mesh-join://?code=412753&ts-key=<scoped-key>&seed-tag=mackes-<mesh-id>`
3. Install Mackes on the new peer (different network).
4. Wizard's Network screen prompts for the join link — scan QR or paste.
5. Mackes queries Tailscale's API with the scoped key, retrieves the
   seed peer's current endpoint, contacts it via DERP, exchanges the
   code for a Headscale pre-auth key, joins Headscale.

The new peer never sees a Tailscale login. Tailscale only knows about
the seed peer's endpoint.

## Tailscale-bootstrap (Option C)

Only the **seed peer** (the first peer in the mesh) signs into Tailscale's
free Personal tier — one OAuth click via Google / Microsoft / GitHub /
email during the wizard. Tailscale serves one role only: knowing where
the seed peer's public-facing endpoint is at any given moment, so remote
peers can find their way in.

When the control role fails over to another peer (see below), the new
control node takes over the Tailscale presence — re-registers under the
same `tag:mackes-<mesh-id>` tag using the scoped API key from NATS.

**Tailscale's free tier**: up to 100 nodes per account. Mackes registers
exactly one node per mesh — the current control peer. So we're at 1/100
forever and the dependency stays free.

## Control-node election

The first peer to install becomes the control node implicitly. If it
goes offline, an election runs:

- After 120s of missed NATS heartbeats, the next peer in deterministic
  order (lowest peer_id) takes over.
- New leader restores headscale state from the latest snapshot in the
  `mesh.vpn-state` NATS Object Store bucket (30s checkpoint cadence).
- Tailscale presence is reassumed.
- Existing WireGuard tunnels — established peer-to-peer, not through the
  control node — keep working throughout.
- Toast notification on every peer: "Mesh control role moved to <peer>"

Only **new** peer joins and ACL changes wait for the failover (~60s
typically after the 120s grace). Existing connectivity is unaffected.

## ACLs

Headscale supports rich ACLs (which peer can talk to which peer on which
ports). Mackes → Network → Mesh VPN → Advanced → ACLs opens an editor.
Default: full mesh — every peer can reach every peer on every port.
Suitable for trusted personal/family meshes.

For tighter deployments, restrict by tag: `tag:family` can reach
`tag:family`, `tag:guest` can reach only the media server, etc.

## Status & diagnostics

Mackes → Network → Mesh VPN panel:

- **Top bar**: connected / disconnected, peer count, control-node flag
- **Add Peer / Leave Mesh / Diagnostics** buttons
- **Peers DataTable**: hostname, mesh-IP, route type (direct vs.
  DERP-relayed), RTT, last-seen
- **Control node** info + snapshot age
- **Advanced**: ACLs editor, DERP servers (self-host option), exit nodes,
  subnet routes

## Troubleshooting

- **Can't reach a peer**: check the peer's status in the DataTable. If
  "DERP-relayed" with high RTT, both peers are behind hostile NAT — try
  a self-hosted DERP server on a peer with a public IP.
- **Control node lost**: should auto-recover within 60s. Check
  `mackes status` for current control peer.
- **Tailscale signin needed again**: only on the seed peer if you wiped
  Tailscale state. Re-run wizard's Network screen.
- **17th peer fails**: hard cap. Remove a peer first via Mackes →
  Network → Mesh VPN → peer row → Remove.
