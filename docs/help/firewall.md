# Firewall

## Overview

MDE uses **firewalld** as the system firewall. The Workbench Firewall
panel lets you change the default zone and toggle common services without
opening a terminal.

## Zones and services

The **default zone** controls the trust level for incoming connections.
`FedoraWorkstation` (or your distribution's default) is a reasonable
starting point: it allows SSH and blocks most other inbound traffic.

The service list shows common services. Toggle a service on to allow
inbound connections; toggle it off to block them. Changes require polkit
authorization (a password prompt appears).

## Nebula mesh traffic

Nebula mesh peers communicate on **UDP/4242**. Lighthouses also accept
**TCP/443** as a covert fallback when UDP is blocked. The
`mackesd::firewall_preset` worker keeps these ports open automatically —
you do not need to add them manually.

## Activity — denied-packet monitoring

MDE automatically records external packets that firewalld denied. This
data powers the **Activity** section in the Firewall panel.

### What is monitored

Every inbound packet that firewalld blocks is logged when `LogDenied` is
set to `all` (MDE sets this during setup). The `mackesd` daemon reads
these journal entries every five seconds and stores them in
`/mnt/mesh-storage/firewall/<hostname>.jsonl`.

### What is filtered

The following traffic is **not** recorded — it is expected and harmless:

- **UDP/4242** — Nebula overlay tunnel packets from your mesh peers.
- **TCP/443** — Lighthouse covert-listener traffic.
- **RELATED,ESTABLISHED** — Replies to outbound connections you
  initiated (conntrack-tracked).

### What you see in the panel

- **Recent denials** — the last 20 external packets blocked, showing
  source IP, protocol, destination port, and which peer recorded it.
- **Top sources** — the five source IPs with the most denials across
  your fleet, so you can spot scanners or brute-force attempts at a
  glance.
- **Per-peer counts** — how many denials each peer recorded, useful
  for spotting an exposed peer.

Data is read from the union of every peer's JSONL file in
`/mnt/mesh-storage/firewall/`, so the panel shows fleet-wide activity,
not just the local peer's.

### Threshold alerts

When a single source IP triggers **10 or more denials in 60 minutes**,
MDE fires a desktop notification. The alert fires once per source per
window — you won't be flooded if a scanner keeps going. The raw Bus
event topic is `event/firewall/<hostname>`.

Records older than **7 days** are automatically removed.

### Bench-verify acceptance

- Packet denied from an off-mesh IP → appears in the JSONL within 10 s.
- UDP/4242 or TCP/443 from a mesh peer → not recorded.
- 10 denials from one source in < 60 min → one desktop notification.
- Records from 8 days ago → absent after the next trim tick.
