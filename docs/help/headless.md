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
→ Welcome to Mackes Desktop Environment (MDE) 2.5.0 (headless mode)

→ Environment scan...
   Hostname:  fileserver
   OS:        Fedora Linux 44 (Server Edition)
   CPU:       AMD Ryzen 5 5600G
   RAM:       16 GB

→ Preset: chromeos-classic-dark (auto-selected; node-style headless variant)
   Mesh:       enabled (Nebula overlay; legacy Tailscale retired)
   File sync:  enabled (LizardFS mesh-storage — XDG dirs FUSE-mounted; was GlusterFS mesh-home)
   Event bus:  enabled (Mackes Bus over ntfy + Nebula; was NATS)

→ Mesh setup
   Is this the first peer in a new mesh? [Y/n] Y
   Minting CA... done.
   Lighthouse overlay IP: 10.42.0.1
   Join token: mesh:a3f9c712@203.0.113.5:4242#eyJhbGci...

   (share this token with peers that want to join)

→ Apply preset 'node'...
   ✓ snapshot created: 2026-05-16T22-08-12_node-baseline
   ✓ mded started (supervisor + workers)
   ✓ nebula.service started
   ✓ LizardFS mesh-storage enabled (XDG dirs FUSE-mounted)
   ✓ Mackes Bus enabled (ntfy broker on nebula0)
   ✓ mesh-ssh keys distributed

→ Auto-start mesh node on boot? [Y/n] _
```

On Y: `systemctl enable --now mde-session.service`. Done. The
fileserver is now on the mesh and rejoins automatically on every boot.

## Joining an existing mesh

```bash
$ mackesd enroll --token 'mesh:a3f9c712@203.0.113.5:4242#eyJhbGci...'
✓ Lighthouse contacted (10.42.0.1, RTT 12 ms via direct UDP)
✓ Cert signed; overlay IP: 10.42.0.4
✓ /etc/nebula/ written; nebula.service started
✓ mesh-ssh keys received from 3 peers
$ mackesd nebula status
state: connected  overlay_ip: 10.42.0.4  peers: 4  transport: nebula_direct
```

The lighthouse operator generates the join token via Workbench →
Network → Mesh → + Add Peer (GUI) or `mackesd ca sign` (CLI).

## Cloud-init / fully automated

For provisioning at scale (seed peer):

```bash
mackes init --preset node --enable-on-boot
# copy the printed join token into your secrets store
```

For subsequent peers:

```bash
mackesd enroll --token '<join-token-from-lighthouse>'
systemctl enable --now mde-session.service
```

Zero interaction. Ideal for cloud-init `runcmd:` blocks.

## Subcommands

Comprehensive parity with the GUI panels:

| Command | Equivalent GUI panel |
|---|---|
| `mackes init` | First-run wizard |
| `mackesd enroll --token <t>` | Wizard's join screen |
| `mackes status` | Dashboard |
| `mackesd nebula peer-list` | Network → Mesh peer DataTable |
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
Description=Mackes Desktop Environment (MDE) — mesh node services
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
