# Headless Node Mode

Mackes runs as a full mesh node on headless servers (fileservers, NAS
boxes, VPSes) without a display manager. Same backend code, no GUI, full
CLI subcommand surface, systemd-managed lifecycle.

## Activation

On launch, `mackes` checks `$DISPLAY`, `$WAYLAND_DISPLAY`, and `loginctl
show-session $XDG_SESSION_ID` for a graphical session. If none of those
are present, headless mode is selected automatically. Most cloud-init /
SSH provisioning sessions hit auto-detect with no flag needed.

Force the choice:
- `mackes --headless` — always use the CLI path
- `mackes --gui` — always use the GTK path (errors if no display)

## First-time setup

```bash
$ ssh user@my-fileserver
$ curl -sL https://matthewmackes.github.io/MAP2-RELEASES/install.sh | sudo bash
$ mackes init
```

`mackes init` runs through the wizard equivalent using pure stdin prompts:

```
→ Welcome to Mackes Shell 1.0.0 (headless mode)

→ Environment scan...
   Hostname:  fileserver
   OS:        Fedora Linux 44 (Server Edition)
   CPU:       AMD Ryzen 5 5600G
   RAM:       16 GB

→ Preset: node (auto-selected for headless)
   Mesh VPN: enabled
   Mesh FS:  enabled
   Mesh Sync: enabled

→ Mesh VPN setup (Tailscale account for cross-network discovery)
   This peer will be the seed/control node for a new mesh.
   Open https://login.tailscale.com/a/abc123 on any device, sign in,
   then press Enter here.
   [Enter]

   ✓ Tailscale account linked. Generating mesh state...
   ✓ Headscale started.
   ✓ Mesh ID: a3f9c712 · Mesh IP: 100.64.1.1

→ Apply preset 'node'...
   ✓ snapshot created: 2026-05-16T22-08-12_node-baseline
   ✓ qnmd started
   ✓ mesh-fs enabled (sharing ~/QNM-Shared)
   ✓ mesh-sync enabled (NATS replica)
   ✓ mesh-ssh keys distributed

→ Auto-start mesh node on boot? [Y/n] _
```

On Y: `systemctl enable --now mackes-node`. Done. The fileserver is now
on the mesh and rejoins automatically on every boot.

## Joining an existing mesh

```bash
$ mackes join 'mesh-join://?code=412753&ts-key=tskey-...&seed-tag=mackes-a3f9'
✓ Contacted seed peer (100.64.1.1, RTT 12ms via DERP)
✓ Code accepted; received Headscale pre-auth key
✓ Joined Headscale; assigned mesh IP 100.64.1.4
✓ qnmd started
✓ mesh-ssh keys received from 3 peers
$ mackes status
Connected · 4 peers · This peer: fileserver (100.64.1.4)
```

The seed peer's admin generates the join link via Mackes → Network →
Mesh VPN → Add Peer (GUI) or `mackes mesh add-peer` (CLI).

## Cloud-init / fully automated

For provisioning at scale:

```bash
mackes init --preset node \
            --tailscale-authkey=tskey-auth-... \
            --enable-on-boot
```

Or for joiners:

```bash
mackes join 'mesh-join://?code=...&ts-key=...&seed-tag=...' --enable-on-boot
```

Zero interaction. Ideal for cloud-init `runcmd:` blocks.

## Subcommands

Comprehensive parity with the GUI panels:

| Command | Equivalent GUI panel |
|---|---|
| `mackes init` | First-run wizard |
| `mackes join <link>` | Wizard's join screen |
| `mackes status` | Dashboard |
| `mackes peers` | Network → Mesh VPN peer DataTable |
| `mackes shares` | Mesh in Thunar `Peers/` subtree |
| `mackes snapshot create [label]` | Maintain → Snapshots → Create |
| `mackes snapshot list` | Maintain → Snapshots list |
| `mackes snapshot restore <name>` | Maintain → Snapshots → Restore |
| `mackes maintain repair` | Maintain → Repair |
| `mackes maintain health` | Maintain → Health Check |
| `mackes maintain logs [N]` | Maintain → Logs (tail) |
| `mackes apps install <name>` | Apps → Install |
| `mackes apps remove <name>` | Apps → Remove |
| `mackes apps list` | Apps → Installed |
| `mackes preset list` | Look & Feel → preset picker |
| `mackes preset apply <name>` | Re-apply preset |
| `mackes services list` | Network → Mesh Services |
| `mackes services launch <name>` | Media Hub Tile click |
| `mackes ssh <peer>` | Network → Mesh SSH → Open Terminal |
| `mackes notify <peer> "<msg>"` | Send mesh notification from cron/scripts |
| `mackes help [topic]` | Help tab |
| `mackes uninstall` | Maintain → Uninstall |

## Mesh role for headless nodes

A headless node participates as **backend-services-only**:

- ✅ SSHFS share/mount (your shared dir + every other peer's)
- ✅ NATS replica (clipboard / notifications / Object Store backed up here)
- ✅ Headscale-eligible (often *preferred* for control role — fileservers
  stay online)
- ✅ Mesh-VPN data plane (full WireGuard to every peer)
- ❌ Does NOT originate clipboard items (no X11 selection to read)
- ❌ Does NOT render notifications (no display)

Use `mackes notify <peer> "message"` from cron/scripts to push
notifications from a headless node to a desktop peer.

## systemd unit

`/etc/systemd/system/mackes-node.service`:

```ini
[Unit]
Description=Mackes Shell — mesh node services
After=network-online.target
Wants=network-online.target

[Service]
Type=notify
ExecStart=/usr/bin/mackes daemon
Restart=on-failure
RestartSec=10s
User=mackes
Group=mackes

[Install]
WantedBy=multi-user.target
```

Runs as the `mackes` system user (created by the RPM's `%post`). Auto-
restarts on failure. Depends on network-online.target.

`systemctl status mackes-node` shows the daemon state; `journalctl -u
mackes-node` shows logs.
