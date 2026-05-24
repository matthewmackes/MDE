# Troubleshooting

Common issues and where to look.

## Logs

Three log sources:

- **MDE actions**: `~/.local/share/mde/logs/mde.log` — every
  settings write, snapshot, preset apply, and mesh event MDE
  performed. Plain text, tail-friendly.
- **mde-session journal**: `journalctl --user -u mde-session
  .service` for session-level issues (sway didn't start, the
  first-boot migrator hit an error, the Iced panel autostart
  failed).
- **mded journal**: `journalctl -u mded.service` for the
  unified meta-daemon (worker restarts, supervisor decisions,
  D-Bus surface errors).

Headless: `journalctl -u mded.service` covers everything; there's
no separate "node" daemon on the v2.0.0 line.

## "Nothing happens when I click Apply"

MDE does immediate-apply on every widget change — there's no batch
"Apply" button. If a switch toggle doesn't seem to take effect,
check `mde.log` for a `bridge.set_setting failed:` entry — the
likely cause is the matching Rust applier in `mded` rejected the
value (look in `journalctl -u mded.service`).

## "Drift card appears even though I didn't change anything"

Probably an external tool wrote to the same sidecar / gsettings
key MDE owns. On the v2.0.0 line:

- Direct `gsettings set` calls outside MDE.
- A second app holding a write-handle on the same JSON sidecar
  under `$XDG_CACHE_HOME/mde/`.
- `mded` rejected the write after staging (check
  `journalctl -u mded.service`).

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

### NAT traversal on the v2.5 Nebula fabric

The v2.5 rebuild retires the explicit ICE/STUN candidate-gathering
step (Phase 12.17). Nebula uses protocol-level UDP hole-punching:
both peers send simultaneous packets through their respective NATs
under coordination from the lighthouse, and the resulting NAT
mappings stay open for the lifetime of the tunnel. No separate
STUN servers, no `stun.l.google.com` probes — the lighthouse
handshake carries the reflexive addresses inline. Symptoms that
used to map to "STUN gather timed out" now map to "lighthouse
unreachable" — see Mackes → Network → Mesh VPN → Diagnostics for
the lighthouse RTT card.

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

## "MDE won't start"

Recovery mode (works on the 2.0.0+ binary `mde`; the 1.x binary
`mackes` is still installed for one release as a transitional alias):
```bash
$ mde recover --list      # list snapshots
$ mde recover --latest    # restore most recent snapshot
```

Or boot the recovery target via GRUB:
```
mde-recovery.target
```
Drops you to a console with snapshot restore tools.

## "I want to start over"

```bash
$ mde uninstall            # GUI or CLI
```

Removes everything MDE installed, restores default gsettings,
deletes user state under `~/.config/mde/` + `~/.cache/mde/` +
`~/.local/share/mde/`, optionally removes the `mde` package. A
final tarball snapshot lands on `~/Desktop/` as the only artifact
that survives.

## Where MDE writes on your machine

```
~/.config/mde/                       user state (post-v2.0.0)
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
