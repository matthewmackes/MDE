# Mackes Shell — User Guide

Welcome to Mackes Shell — your single control panel for XFCE on Fedora and
the mesh fabric that connects all your machines.

## What Mackes Shell does

Mackes Shell replaces `xfce4-settings` as your daily control surface and
adds first-class mesh networking on top: every machine running Mackes can
share files, clipboards, notifications, and media services with every
other machine in your mesh, regardless of physical location.

Eight task tabs:

- **[Dashboard](dashboard.md)** — live status, drift detection, quick actions
- **[Look & Feel](look-and-feel.md)** — themes, fonts, icons, wallpaper
- **[Devices](devices.md)** — display, keyboard, mouse, sound, power
- **[Network](network.md)** — Wi-Fi, VPN, mesh, firewall, SSH
- **[System](system.md)** — window manager, workspaces, session, notifications
- **[Apps](apps.md)** — install curated apps, remove bloat
- **[Maintain](maintain.md)** — snapshots, repair, logs, uninstall
- **[Help](help.md)** — this guide

## The mesh

Mackes ships a complete peer-to-peer mesh built on five layers:

- **[Mesh VPN](mesh-vpn.md)** — Headscale + Tailscale-bootstrap, WireGuard data plane
- **[Mesh in Thunar](mesh-thunar.md)** — `mesh:///` shows peers, clipboard, notifications, shared files
- **[Mesh SSH](mesh-ssh.md)** — auto-distributed keys + identity-based access
- **[Mesh Media Services](mesh-services.md)** — discover Jellyfin, Airsonic, Plex, etc. across the mesh
- **[Headless Node Mode](headless.md)** — full mesh on fileservers without a display

## Quick links

- **[Getting Started](getting-started.md)** — first-run wizard walkthrough
- **[Presets](presets.md)** — the 4+1 shipped presets explained
- **[CLI Reference](cli-reference.md)** — every `mackes` subcommand
- **[Keyboard shortcuts](keybindings.md)**
- **[Troubleshooting](troubleshooting.md)**

## About

Mackes Shell 1.0.0 — "XFCE Provisioner". Released 2026-05-16.
GPL-3.0. Source: https://github.com/matthewmackes/MAP2-RELEASES
