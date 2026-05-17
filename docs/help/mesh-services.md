# Mesh Media Services

Audio / video / monitoring / IoT / dev services running on any mesh peer
are reachable from every other peer, no matter where they are. Mackes
layers five surfaces on top of that raw reachability — each addresses a
different client audience. All five share the same service catalog and
live registry.

## Shared catalog

`/usr/share/mackes-shell/data/media-services.yaml` lists 34+ known
service types out of the box: Jellyfin, Airsonic, Plex, Navidrome,
Kodi-web, Komga, Sonarr, Radarr, Lidarr, Prowlarr, qBittorrent,
Transmission, Home Assistant, Node-RED, ESPHome, Nextcloud, Syncthing,
File Browser, Pi-hole, AdGuard Home, Vaultwarden, Authelia, Grafana,
Prometheus, Uptime Kuma, Netdata, Gitea, Forgejo, code-server, Jupyter,
plus mDNS-discovered Chromecast / AirPlay / Spotify Connect / network
printers.

Add your own: `~/.config/mackes-shell/media-services.yaml` overrides
shipped entries by `name`. Format:

```yaml
services:
  - name:        my-app
    display:     "My App"
    category:    media       # media | media-mgmt | iot | storage | networking | security | monitoring | dev | cast | hardware
    port:        9000
    https-port:  9443        # optional
    path:        "/"
    icon:        my-app
    mdns-type:   "_my-app._tcp"   # optional
    native-client: my-app-cli      # optional
    description: "What it does."
```

## Live registry

qnmd's `mesh-services` module port-probes every mesh peer every 60s
against the catalog. The matrix is published to a `mesh.services` NATS
bucket so all five layers (and any peer) read the same data.

## Layer 1 — Raw mesh URLs

`http://<peer>.mesh:<port>` works day one — Mesh VPN + MagicDNS give
every peer reachability + name resolution.

Mackes → Network → Mesh Services → **Help cheatsheet** renders a
generated table of every detected service as a clickable URL. No new
code; mesh VPN does everything.

## Layer 2 — Mesh Media Hub panel

Mackes → Network → Mesh Services. Each detected service rendered as a
Carbon Tile: peer name + service icon + green/grey status dot +
**Open** button → `xdg-open http://<peer>.mesh:<port>`. Filter chips at
the top by category. Updates live from the NATS `mesh.services` registry.

One-click launch in your default browser. No URL memorization.

## Layer 3 — Unified `https://media.mesh` gateway (Caddy)

Opt-in via Mackes → Network → Mesh Services → **Enable Unified Gateway**.
Mackes installs Caddy as a service on every peer; Caddy is auto-configured
from the live registry to expose every service under one URL space:

```
https://media.mesh/jellyfin/headless-server/
https://media.mesh/airsonic/headless-server/
https://media.mesh/jellyfin/laptop-mm/
https://media.mesh/grafana/headless-server/
```

TLS via a Mackes-managed private CA. CA root is distributed via NATS
Object Store (`mesh.ca-root` bucket) and installed into each peer's
trust store via pkexec helper.

One browser bookmark = the entire mesh's HTTP service catalog. Failed
peers' routes return a Carbon-styled 502 page; recovered peers reappear
automatically.

## Layer 4 — Bundled native clients

Apps → Install includes:

- **Jellyfin Media Player** (`jellyfin-media-player` Fedora package).
  Mackes writes `~/.local/share/jellyfinmediaplayer/servers.json` listing
  every mesh peer running Jellyfin.
- **Strawberry** (Subsonic-compatible). Server list pre-populated for
  every mesh peer running Airsonic.

Server-list refresh runs on mesh-peer events from NATS. Native clients
connect direct to `<peer>.mesh:<port>` — they don't traverse Caddy.
Better playback UX than browsers (proper buffering, offline downloads,
native gestures).

## Layer 5 — mDNS-over-mesh relay

qnmd's `mdns-relay` module bridges each peer's `avahi-daemon` across the
WireGuard tunnels:

1. Local mDNS announcements (`_jellyfin._tcp.local`, `_googlecast._tcp.local`,
   `_airplay._tcp.local`, `_ipp._tcp.local`, …) captured on each peer.
2. Captured announcements republished to NATS subject
   `mesh.mdns.<peer-id>.<service-type>`.
3. On every other peer, qnmd subscribes and re-broadcasts received
   announcements on the local LAN — substituting the originating peer's
   mesh IP for the source LAN IP.

Result: any mDNS-aware client (Jellyfin Roku app, Plex iPhone,
Chromecast, AirPlay speaker, network printer, Home Assistant device
discovery) sees every mesh peer's services as if they were local-LAN.

### Per-service-type opt-out

Mackes → Network → Mesh Services → **mDNS bridge** shows checkboxes per
service type. Canonical media types default ON; printer / file-share
types default OFF.

### Privacy

Announcements carry an `origin-peer-id` field; receivers never
re-publish their own announcements (anti-loop). Name-collision handling
renames services to `jellyfin-headless-server.local` (hostname suffix)
before local rebroadcast to avoid `jellyfin.local` collisions.

## Layer interactions

- L2 (Media Hub) and L3 (Caddy proxy) read the same registry. Tile click
  can route via direct URL or proxy URL; user toggle in Settings.
- L4 (native clients) always connects direct, bypassing L3. Playback
  quality benefits from no proxy hop.
- L5 (mDNS relay) feeds discoveries into the same registry L2/L3
  consume. A Chromecast announced via mDNS but not on a probed port
  still appears in the Media Hub.
- L3's Caddy can optionally serve L4 native-client config endpoints,
  letting non-Mackes-instance clients bootstrap their server list.

## CLI

- `mackes services list` — print discovered services
- `mackes services launch <name> [--peer <peer>]` — open in default
  browser
- `mackes services enable-gateway` — enable Layer 3 (interactive auth)
- `mackes services disable-gateway` — disable Layer 3
