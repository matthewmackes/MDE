# Troubleshooting

Common issues and where to look.

## Logs

Two log sources:

- **Mackes actions**: `~/.local/share/mackes-shell/logs/mackes.log` —
  every xfconf write, snapshot, preset apply, mesh event Mackes
  performed. Plain text, tail-friendly.
- **xfsettingsd journal**: `journalctl --user -u xfsettingsd` (or
  `xfce4-session` for session-level issues).

Headless: `journalctl -u mackes-node` for the systemd-managed daemon.

## "Nothing happens when I click Apply"

Mackes does immediate-apply on every widget change — there's no batch
"Apply" button. If a switch toggle doesn't seem to take effect, check
`mackes.log` for an `xfconf set failed:` entry.

## "Drift card appears even though I didn't change anything"

Probably xfsettingsd or another XFCE tool re-wrote the value. Examples:
- Theme picker in xfce4-appearance-settings (Mackes hides the menu entry
  but the binary still runs if invoked manually)
- xfce4-panel plugin preferences
- Direct xfconf-query writes by other tools

Reset via Maintain → Reset to Preset.

## "Wizard doesn't find the mesh on my LAN"

mDNS issues:
- Avahi running? `systemctl status avahi-daemon`
- Multicast blocked by router? Some enterprise/hotel networks filter
  `224.0.0.251`.
- Fallback: ask the seed peer's admin for a join link (Mackes → Network
  → Mesh VPN → Add Peer).

## "Cross-network peer can't join"

- Confirm the seed peer's Tailscale presence: `tailscale status` on
  the seed should show it registered.
- The join link expires after 10 minutes — regenerate if stale.
- If both peers are behind hostile NAT, the connection uses Tailscale's
  DERP relays — check DERP RTT in Mackes → Network → Mesh VPN →
  Diagnostics.

## "Mesh peer says offline but I can ping it"

Mesh VPN goes through Headscale's control plane. Possible causes:
- Control node went offline; failover takes ~120s.
- DERP relay temporarily unreachable; retry in ~30s.
- Peer's tailscale daemon crashed; on the peer: `systemctl restart
  tailscaled`.

## "Clipboard items aren't syncing"

- Check qnmd's `mesh-sync` module: `mackes status` should show it as
  running.
- NATS server health: `mackes maintain health` includes NATS reachability
  checks.
- 100-item cap means very-old items roll off — that's expected.

## "Thunar shows mesh:/// but it's empty"

- `gvfsd-mesh` is the GVFS backend. Check it's installed:
  `which gvfsd-mesh` should return `/usr/libexec/gvfsd-mesh`.
- qnmd not running → no mesh state to render.
- mDNS issues hiding peers — see Mesh VPN troubleshooting above.

## "ssh peer.mesh: Permission denied"

- Layer A keys may not have synced yet. Wait 30s for NATS propagation.
- Check `~/.ssh/authorized_keys` for the `# managed-by-mackes-mesh-<peer>`
  marker. If absent, Mackes hasn't received the key yet — check qnmd
  logs.
- Wrong username? Default is the wizard-running user; Mackes → Network
  → Mesh SSH → Key Distribution lets you override per peer.

## "Mackes Shell won't start"

Recovery mode:
```bash
$ python3 -m mackes.recover --list      # list snapshots
$ python3 -m mackes.recover --latest    # restore most recent snapshot
```

Or boot the recovery target via GRUB:
```
mackes-recovery.target
```
Drops you to a console with snapshot restore tools.

## "I want to start over"

```bash
$ mackes uninstall            # GUI or CLI
```

Removes everything Mackes installed, restores xfconf defaults, restores
hidden xfce4-settings menu entries, deletes user files, optionally
removes the package. A final tarball snapshot lands on `~/Desktop/` as
the only artifact that survives.

## Where Mackes writes on your machine

```
~/.config/mackes-shell/              user state
  state.json                         provisioned + active_preset
  presets/                           user-custom presets (overrides shipped)
  overrides/                         backup of xfce4-settings menu entries Mackes hid
  removed-by-mackes.json             list of packages Mackes uninstalled
  media-services.yaml                user-added service entries (optional)

~/.local/share/mackes-shell/         user data
  logs/mackes.log                    unified log
  snapshots/                         restore points

~/.ssh/                              Mesh SSH adds files here
  mackes_mesh_ed25519                per-peer mesh SSH key
  mackes_mesh_ed25519.pub
  authorized_keys                    (appended with mesh peer pubkeys)

~/QNM-Shared/                        files exposed to other mesh peers via SSHFS
~/QNM-Mesh/                          mount points for other peers' shares
```

System paths Mackes installs to:
```
/usr/bin/mackes                      entry point
/usr/lib/python3.X/site-packages/mackes/
/usr/share/mackes-shell/             data tree (presets, css, branding, help)
/usr/share/applications/mackes-shell.desktop
/etc/systemd/system/mackes-node.service   (headless only, enabled via init)
```
