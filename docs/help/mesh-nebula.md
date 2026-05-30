# Mesh networking (Nebula)

MDE's mesh fabric runs on [Nebula](https://github.com/slackhq/nebula) —
a self-hosted, certificate-based overlay network. No SaaS, no OAuth
sign-in, no cloud dependency.

## Architecture

- **Overlay network**: every peer gets a stable overlay IP (e.g.
  `10.42.0.x`) from the Nebula CA. The IP survives network changes.
- **Certificate authority**: the first peer (the *lighthouse*) mints a CA
  private key and signs a cert for every peer that joins.
- **Lighthouses**: always-on peers that help other peers find each other.
  At least one peer should be a lighthouse (usually a desktop or
  server that's always on).
- **NAT traversal**: Nebula uses UDP hole-punching (port 4242) with a
  TCP/443 fallback tunnel when UDP is blocked. No third-party relay.
- **DNS**: peers are reachable by overlay IP. Hostname-based lookup
  (`<peer>.mesh`) is handled by mDNS within the LAN.

## Setting up a new mesh

The first peer creates the mesh during the first-boot wizard:

1. Wizard → Network → Mesh → **Create a new mesh**.
2. A CA keypair is minted on this peer and sealed in the mackesd store.
3. A join token is printed: `mesh:<mesh_id>@<lighthouse_ip>:4242#<token>`.
   Copy it or share it as a QR code.
4. This peer becomes the first lighthouse.

## Adding peers

### From the GUI (same or different network)

1. Open MDE Workbench → Network → Mesh → **Join existing mesh**.
2. Paste the join token or scan the QR code.
3. The wizard calls `mded.Nebula.Enroll(token)` via D-Bus. The lighthouse
   signs a cert for this peer and writes the Nebula config to
   `/etc/nebula/`.
4. Nebula starts and the overlay comes up in < 10 seconds.

### Headless

```bash
mackesd enroll --token 'mesh:<mesh_id>@<lighthouse_ip>:4242#<bearer>'
```

The command is idempotent — re-running refreshes the cert without
creating a duplicate node row.

## Overlay IPs

Overlay IPs are assigned from the range locked in the CA config at
mesh-init time (default `10.42.0.0/16`). Each cert carries the IP;
it doesn't change unless the CA is rotated or the peer re-enrolls.

## Lighthouses

A lighthouse is any peer whose Nebula process listens on a stable
public or LAN IP. The lighthouse list is embedded in every cert bundle.

Add more lighthouses: Workbench → Network → Mesh → peer row →
**Promote to lighthouse**. The panel calls `mackesd ca sign` with
the `lighthouse` group, then restarts `nebula.service` on the
promoted peer.

## Checking mesh status

```bash
# Current peer's overlay IP + cert expiry
mackesd nebula status

# All enrolled peers + their overlay IPs + cert expiry
mackesd nebula peer-list

# Trigger cert renewal (operator-initiated)
mackesd nebula regen-certs
```

In the Workbench: Network → Mesh shows the live roster with overlay
IPs, cert-expiry badges, and lighthouse flags.

## Rotating the shared passcode

Every peer shares one 16-character passcode. To roll it (e.g. after a
peer leaves the company), rotate on any peer and let the reconcile loop
propagate the change:

```bash
mackesd rotate-passcode --store     # new code printed; encrypted at rest
mackesd show-passcode               # re-print the current stored code
```

`--store` encrypts the code via `systemd-creds` (TPM when present, host
key otherwise) at `/var/lib/mackesd/mesh-passcode.cred` — the plaintext
never lands on disk. The cred is host-local: each peer holds its own
encrypted copy, so the file does not replicate across the mesh. See the
[CLI reference](cli-reference.md) for the full passcode command set.

## Revoking and banning compromised peers

Two distinct tools, used together when a peer is lost or compromised:

- **Revoke** invalidates a peer's *current cert* and pushes a CRL so the
  rest of the mesh drops it. The node can still re-enrol with a fresh
  cert if it still holds the passcode.
- **Ban** refuses a node-id enrolment *mesh-wide* — even with a valid
  passcode and across a CA rotation. Use this when the identity itself
  is compromised, not just one cert.

```bash
mackesd ca revoke <node-id>         # drop the current cert (push CRL)
mackesd ca ban <node-id>            # refuse re-enrolment mesh-wide
mackesd ca ban-list                 # print the enforced mesh-wide union
mackesd ca unban <node-id>          # lift THIS peer's ban entry
```

Bans propagate via LizardFS mesh-storage and the enrolment gate
enforces the **union** of every peer's list — there is no override, so a
single peer banning a stolen node-id protects the whole fleet. `unban`
only lifts the entry the local peer set; a ban another peer set must be
lifted there. After revoking + banning a lost peer, rotate the passcode
so the stolen credential can't enrol a *different* node-id.

## Troubleshooting

### Peer doesn't appear in the roster after enrolling

Check `journalctl -u nebula.service` on the new peer. Common causes:

- Port 4242/UDP blocked by firewall. Run:
  `firewall-cmd --permanent --add-port=4242/udp && firewall-cmd --reload`
- Lighthouse is unreachable. Confirm `ping <lighthouse_overlay_ip>` works
  from another peer. If the lighthouse is behind NAT, ensure its public
  IP and UDP/4242 are forwarded.

### TCP/443 fallback mode active

When direct UDP fails, Nebula tunnels through the lighthouse's TCP/443
listener. You'll see `active_transport: nebula_https443` in
`mackesd nebula status`. This is normal on strict corporate or hotel
networks. Performance is slightly lower than direct UDP.

### Cert expiry warning

Certs expire after 365 days by default (configurable at CA mint time).
`mded` emits a warning toast when any peer cert is within 7 days of
expiry. Rotate proactively with `mackesd ca rotate`.

See [mesh-admin.md](mesh-admin.md) for CA operations and
[mesh-recovery.md](mesh-recovery.md) for disaster recovery.
