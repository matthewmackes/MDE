# Network

Eight network-related panels under one tab.

## Wi-Fi & Ethernet

Live list of networks via `nmcli`. Connect / disconnect / forget; password
prompt for new joins. Doesn't try to replace `nm-applet` — that stays in
the system tray for daily use.

## VPN

NetworkManager VPN connections (OpenVPN, WireGuard, IPsec). Import `.ovpn`
or `.conf` files. The mesh-VPN (see below) is a separate, always-on
system — these are user-imported corporate / commercial VPNs.

## Quick Network Mesh (QNM)

The custom peer-discovery + transport layer Mackes inherited from
xfce11-unified v2.2. Status, start/stop/restart of the `qnmd` daemon.
Embeds the QNM GUI for peer-list management.

## Mesh VPN

See **[mesh-vpn.md](mesh-vpn.md)** for the full guide. The summary panel
here shows:

- Mesh status (connected / disconnected / control node)
- Peer count + 16-peer cap indicator
- Add Peer button (generates QR + paste-link)
- Diagnostics (DERP RTT, current control node, snapshot age)
- Advanced (ACLs, exit nodes, DERP servers)

## Mesh SSH

See **[mesh-ssh.md](mesh-ssh.md)**. Four sub-sections:

- Discovered Peers — Tile-per-peer with "Open Terminal" buttons
- Key Distribution — Layer A auto-key state
- Access Policy — Layer B identity-based ACL editor
- Audit Log — recent SSH sessions across the mesh

## Mesh Services

See **[mesh-services.md](mesh-services.md)**. Five-section panel exposing
every discovered HTTP service on every mesh peer.

## Firewall

`firewalld` zones, services, ports. Default Mackes zone is
`FedoraWorkstation`; the headless `node` preset uses `FedoraServer`.

## Mesh in Thunar

See **[mesh-thunar.md](mesh-thunar.md)** for the `mesh:///` Thunar
extension that exposes peers, clipboard, notifications, and Object Store
as browsable folders.
