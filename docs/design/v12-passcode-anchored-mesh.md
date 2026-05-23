# Phase 12 — Passcode-Anchored Mesh (locked 2026-05-23)

Companion document to `PROJECT_WORKLIST.md § Phase 12.24–12.33` and
`docs/design/v12-connectivity-scope.md`. Closes the 10 gaps surfaced
by the 2026-05-23 mesh-functionality evaluation by promoting the
existing 16-char enrollment passcode (`crates/mackesd/src/passcode.rs`,
`mackes/wizard/pages/mesh_passcode.py`) from "authentication-only
credential" to **cryptographic anchor for the entire mesh**.

This is Round 2 of the connectivity work; Round 1 lives in
`v12-connectivity-scope.md` (Phase 12.14–12.23). When the two
documents conflict, this one wins per the §1 "newer wins silently"
rule in `.claude/CLAUDE.md`.

## /goal directive (2026-05-23)

> Create solutions to solve for the mesh-eval gaps. Propose 10. All
> should involve a 16-digit shared key for all operations in the mesh.

## Gap snapshot — what the 2026-05-23 eval found

| # | Gap                                                      | Audit citation |
|---|----------------------------------------------------------|----------------|
| 1 | No public rendezvous for the control plane               | `mackes/mesh_vpn.py:323` hardcodes `server_url=http://0.0.0.0:8080`; mDNS only publishes local hostname (`mesh_vpn.py:448`) |
| 2 | NAT traversal stops at Tailscale's public DERP           | `crates/mackesd/src/stun.rs:1-30` exists but never called from `update()` (Phase 12.17 scaffolding) |
| 3 | No CGNAT / NAT-type detection                            | Zero matches for `cgnat\|upnp\|natpmp\|igd` outside a `mesh_derp.py` docstring |
| 4 | No dynamic-IP handling                                   | `mackes/mesh_derp.py:86` accepts `public_ip` param that is never populated; no DDNS code |
| 5 | No out-of-band discovery without a join link             | `mackes/mesh_discovery.py:196-211` chain is clipboard → LAN mDNS → manual paste; nothing crosses the internet |
| 6 | Mesh-id → control-URL resolution rots on IP rotation     | Join link bakes the control URL at issue time |
| 7 | No firewall/port configuration                            | `mackes/workbench/network/firewall.py:72` exposes generic services only; SEED wizard never calls it |
| 8 | No TLS provisioning for public Headscale                 | `mackes/caddy_gateway.py:68-72` uses Caddy *internal* CA only; `crates/mackesd/src/https_fallback.rs:11-23` is "future work" |
| 9 | No multi-control / failover                              | `crates/mackesd/src/leader.rs:1-9` is single-leader filesystem-lock + 60 s lease |
| 10| Wizard reports false-green                               | `mackes/wizard/headscale_setup.py:580-596` `_s_verify` + `_s_generate_link` check only local `tailscale status --json` |

The architectural inversion that compounds these: `mackesd_core`
(`docs/design/v12.0-enterprise-mesh.md:41-43`) explicitly states
*"No networked API exists — peer communication happens through the
shared filesystem only"*. The control plane assumes the data plane
is already up. **Round 2 makes the passcode itself carry enough
information that the data plane can bootstrap with no other input.**

## Cryptographic foundation

The passcode `P` is a 16-char URL-safe string drawn from
`[A-Za-z0-9_-]` — 64^16 ≈ 2^96 bits of entropy. The shape is
already enforced by `crates/mackesd/src/passcode.rs:35-44` and
mirrored in `mackes/wizard/pages/mesh_passcode.py:_VALID_PASSCODE_*`.

On peer start, `P` is fed through a memory-hard KDF exactly once,
then HKDF derives purpose-bound subkeys:

```text
K          = Argon2id(P, salt="mackes-mesh-v1", mem=64MB, t=3, p=1)
                       → 256-bit root key, ~1 s to derive on a laptop
K_rdv      = HKDF-Expand(K, "rendezvous",   32)  → DHT topic + record auth
K_sign     = HKDF-Expand(K, "ed25519-seed", 32)  → mesh root signing key
K_trans    = HKDF-Expand(K, "transport",    32)  → wire encryption (ChaCha20-Poly1305)
K_ctrl     = HKDF-Expand(K, "control-elect",32)  → leader-election vote signing
K_probe    = HKDF-Expand(K, "probe-relay",  32)  → reachability-probe HMAC

mesh_id    = BLAKE3(K || "mesh-id")[:16]         → 16-byte deterministic identity
ca_keypair = ed25519::from_seed(K_sign)          → mesh root CA (TLS)
```

Every peer with the same `P` recomputes the same subkeys. The
passcode itself is **never transmitted** and is stored only in
libsecret (`mackes/wizard/pages/mesh_passcode.py` already does
this for v1.x). All transmitted material is HMAC- or
signature-authenticated by a passcode-derived subkey.

**Threat model:** Anyone with `P` is fully trusted (same model as
a Tailscale auth-key, scaled down to human-typeable). Anyone
without `P` cannot read or inject mesh traffic, cannot find the
mesh on the rendezvous, and cannot present a trusted TLS cert.

## The 10 solutions

Each solution names: the gap it closes, the mechanism, the
subkey it consumes, and the dependency edges. Implementation
detail lives in the corresponding `PROJECT_WORKLIST.md § 12.24–
12.33` entries.

### 12.24 — Passcode-Derived Rendezvous (PDR)

*Closes gap 1, 5, 6.*

Use a public Kademlia DHT (libp2p `kad` 0.46 / IPFS DHT). The
rendezvous topic is `T_hour = BLAKE3(K_rdv || epoch_hour)`. Every
peer publishes a `RendezvousRecord { peer_id, current_endpoint,
expires_at }` under `T_hour`, signed with `K_sign`-derived per-peer
subkey. A joining peer with the passcode recomputes `T_hour`,
fetches records, verifies signatures, dials the freshest endpoint
for the leader role.

The mesh is invisible to anyone without the passcode — the DHT
topic itself is unguessable without `K_rdv`. Rolling topics
(per-hour) limit damage from a topic-leak to a one-hour window.

### 12.25 — Passcode-Authenticated ICE/STUN bootstrap

*Closes gap 2. Activates Phase 12.17 (`stun.rs`).*

`crates/mackesd/src/stun.rs:1-30` already implements RFC 5389/8489.
Wire it into the peer bootstrap path. For each peer-pair, ICE
candidates (host / server-reflexive / relayed) are encrypted with
`K_trans` and published to a peer-pair sub-topic
`BLAKE3(K_rdv || peer_a || peer_b)`. The other side polls,
decrypts, picks the best candidate by RTT.

PDR (12.24) replaces the third-party signaling server typical of
ICE deployments — the rendezvous IS the signaling channel. Both
ends authenticate exclusively via passcode-derived subkeys.

### 12.26 — Passcode-Keyed Reachability Probes

*Closes gap 3. Feeds 12.33.*

A small fleet of public Mackes probe relays (~3 cheap x86_64 VPSes,
single-region per Q4 lock) accepts only requests authenticated by
`HMAC(K_probe, body)`. Cost-of-service is paid in passcode entropy.

SEED wizard sends `ProbeMe { my_endpoint, nonce }`. Each relay
attempts a reverse connection to `my_endpoint` and returns
`ProbeResult { reachable, observed_public_ip, observed_port }`.
From two answers the SEED learns: (a) its real public IP, (b)
whether `observed_ip` matches its STUN view (CGNAT detection),
(c) whether inbound works at all.

Relays refuse unsigned requests — Mackes infrastructure won't probe
arbitrary IPs on behalf of strangers.

### 12.27 — Rolling Passcode-Signed Endpoint Records (DDNS killer)

*Closes gap 4.*

Every peer (not just leader) re-publishes its 12.24 rendezvous
record every 60 s with a 180 s TTL. On IP rotation, the next
heartbeat carries the new endpoint; stale records age out. There
is no "DNS" to update — the rendezvous IS the live endpoint table.

Records are signed by both `K_sign` (mesh-wide root) and a
per-peer ed25519 subkey, so forgery requires the passcode AND the
peer's local private key.

### 12.28 — Passcode-as-Locator join link

*Closes gap 5 / 6 user-facing.*

Today's `mackes://join/<mesh-id>?control=<url>` embeds a control
URL that rots when the seed's IP rotates (`mackes/mesh_vpn.py:323`).
Replace with `mackes://join#<16-char-passcode>` — a URL fragment
whose entire payload is the passcode. The client derives `mesh_id`,
`T_hour`, and all crypto material from the passcode alone; the
current leader's endpoint is looked up live via PDR (12.24).

Old link format still parsed during a one-release migration window;
the SEED wizard stops generating it. The on-screen "passcode" chip
in `mackes/wizard/pages/mesh_passcode.py` becomes the primary
shareable artifact (QR + copy + libsecret).

### 12.29 — Passcode-Anchored Self-Signed PKI

*Closes gap 8.*

Headscale requires HTTPS for non-localhost (`mackes/mesh_vpn.py:323`
binds `0.0.0.0:8080`). Today the product has neither ACME nor a
domain story (`mackes/caddy_gateway.py:68-72` uses Caddy *internal*
CA, mesh-internal only).

Instead: `K_sign` seeds an ed25519 root keypair = the mesh root CA.
SEED wizard uses `rcgen` to mint a TLS cert for the control
endpoint signed by this root. Every peer with the passcode
recomputes the root pubkey and pins it in its
Tailscale/Headscale client TLS config. The Headscale `server_url`
can now be a bare IP — DNS ownership is no longer required.

Trust boundary: "knows the passcode" — exactly the trust boundary
the mesh already requires for enrollment.

### 12.30 — Passcode-Authorized Port Plumbing + HTTPS-on-443 fallback

*Closes gap 7. Activates Phase 12.18 (`https_fallback.rs`).*

SEED wizard tries inbound-port plumbing in order:

1. **UPnP / NAT-PMP / PCP** via the `igd` crate. Opens 8080
   (Headscale) + 3478/UDP (DERP).
2. **Passcode probe (12.26)** confirms external reach.
3. On failure: fall back to **HTTPS-on-443 covert transport** —
   `crates/mackesd/src/https_fallback.rs` (currently "future work")
   wired up. Real TLS handshake using the passcode-derived cert
   from 12.29, realistic SNI, wire bytes encrypted with `K_trans`.

To DPI: indistinguishable from a corporate intranet HTTPS host.
Satisfies the Q10 lock from `v12-connectivity-scope.md:124-129`.

### 12.31 — Passcode-Elected Multi-Control

*Closes gap 9.*

Replace `crates/mackesd/src/leader.rs:1-9`'s filesystem-lock with
rendezvous-published leases. Each candidate peer publishes
`VoteFor { candidate_id, term, expires_at }` signed by `K_ctrl`.
Election order is deterministic from `HMAC(K_ctrl, peer_id)` — every
peer agrees on the priority without negotiation.

Lease = 60 s, heartbeat every 20 s. On missed heartbeat the
next-ranked peer assumes the role and publishes its takeover.
Headscale state replicates via `mackesd_core::reconcile`'s existing
log. The 16-peer cap (Q2 lock) keeps consensus trivial — leases,
not Raft.

Only passcode-holders can vote or be elected.

### 12.32 — Passcode-Sealed Enrollment Envelope

*Closes the stubbed remote-redemption at `mackes/mesh_vpn.py:819-829`
("simulate by trying to redeem locally") and replaces the manual
inbox-drop at `crates/mackesd/src/bin/mackesd.rs:368`.*

A joiner with only the passcode constructs `EnrollmentRequest {
peer_id, peer_pubkey, requested_name }`, encrypts with `K_trans`,
publishes to `BLAKE3(K_rdv || "enroll")`. The current leader (12.31)
polls that topic, decrypts, validates against policy (16-peer cap,
name uniqueness from `mackesd_core::policy`), and publishes a signed
`EnrollmentResponse { headscale_preauth_key, peer_assigned_ip }`
under `BLAKE3(K_rdv || peer_id)`.

No HTTP API, no inbox file. The "shared filesystem" pattern that
`v12.0-enterprise-mesh.md:41-43` locks in becomes a
passcode-encrypted public bulletin board, which is fit-for-purpose
for cross-NAT peers in a way the filesystem alone never could be.

### 12.33 — Reachability Oath in the SEED wizard

*Closes gap 10. The user-visible glue for everything above.*

Insert a new step in `mackes/wizard/headscale_setup.py` between
`_s_start_headscale` (line 426) and `_s_generate_link` (line 431):

```python
_Step("Verify external reachability", _s_verify_reachable),
```

The step runs (a) STUN to learn the public IP (12.25), (b) UPnP/PCP
port-open attempt (12.30), (c) passcode-keyed probe (12.26), (d)
joiner simulation — a passcode-authorized probe relay performs the
full enrollment flow (12.32) against the seed's published endpoint
using a throwaway peer-id.

The step **must return green** before `_s_generate_link` is
reachable. On failure, the wizard branches:

- **HTTPS-fallback mode** (12.30) — accept slower throughput, no
  inbound port needed.
- **Host-mode-on-another-peer** — declare this node non-viable as
  seed; surface a list of other peers on the same passcode whose
  probes passed.
- **Manual override** with explicit warning copy ("This seed will
  only be reachable from your LAN — peers outside this network
  will not be able to join").

The passcode chip (12.28) is the prominent shareable artifact.
The join link becomes a convenience pre-paste, not the locator.

## Implementation phases

Ship in three layers, each independently useful and bench-testable
against the Q25 6-peer fleet acceptance gate from
`v12-connectivity-scope.md:63`.

### Layer A — Rendezvous & locator (Phase A)

**Items:** 12.24, 12.27, 12.28, 12.32.
**Delivers:** any peer with the passcode can find the mesh; dynamic
IPs handled; old link format retired; enrollment crosses NATs.
**Replaces:** broken `mackes/mesh_discovery.py` chain; stubbed
`mackes/mesh_vpn.py:819-829` redemption; manual inbox-drop in
`mackesd.rs:368`.

### Layer B — Reachability & trust (Phase B)

**Items:** 12.26, 12.29, 12.30, 12.33.
**Delivers:** SEED wizard tells the truth about reachability; TLS
works with no domain; HTTPS-on-443 fallback usable from the
operator surface for corporate firewalls; no false-green join
links.
**Builds on:** the shipped Phase 12.18 D.1 (`ab8e1ee`, worklist
line 859) + D.2 Https443 Transport (`4442522`, line 910). 12.30
adds UPnP/PCP/NAT-PMP, the passcode-derived TLS cert (12.29), and
makes the SEED wizard the first operator-visible caller of the
just-shipped transport. Coordinates with the still-open 12.18 D.3
(MeshRouterWorker tick wiring at line 975) — ship together so the
HTTPS-fallback story is end-to-end operator-visible in one cut.

### Layer C — Distributed control (Phase C)

**Items:** 12.25, 12.31.
**Delivers:** real NAT traversal beyond Tailscale public DERP via
passcode-signed ICE; real leader failover within the 16-peer cap.
**Builds on:** the shipped Phase 12.17 STUN candidate-gathering
wire (`2d04d67`, worklist line 816). 12.25 does not re-wire
STUN — it adds a passcode-derived signaling layer on top of the
already-active candidate machinery, so the rendezvous (12.24) is
the signaling channel instead of Tailscale's. 12.31 retires the
filesystem-lock leader-election in `crates/mackesd/src/leader.rs`
in favor of passcode-signed rendezvous-published leases.

## Security note

96 bits of passcode entropy is enough for symmetric crypto but is
**online-guessable** without rate-limiting. Mitigations:

- **Argon2id** (memory-hard, ~1 s/derivation on a laptop) on every
  passcode use — brute-force costs seconds, not microseconds.
- **Per-hour rendezvous topics** (12.24) — past topics expire; a
  one-hour passive capture doesn't enable indefinite observation.
- **Probe-relay token buckets** (12.26) — each source IP rate-
  limited; no offline pre-computation farm hammering for valid
  passcodes against the relay.
- **Silent enrollment-failure drop** (12.32) — leader does not
  respond to invalid enrollment requests; no oracle for an attacker.
- **No passcode in transit** — only HKDF-derived subkeys travel.
  The passcode lives in libsecret on the peer's local host.

If a passcode IS compromised, recovery requires regenerating the
passcode + re-enrolling every peer. The wizard surfaces this as an
explicit "Rotate mesh passcode" action under
`workbench/network/mesh_control.py` (new — added under 12.33).

## Cross-cutting refinements vs. Round 1 (Phase 12.14–12.23)

| Round 1 item | Round 2 impact |
|--------------|----------------|
| 12.16 self-hosted DERP, default-on | Stays — DERP now optional once 12.25 ICE works, but kept as the always-on fallback for symmetric-NAT pairs |
| 12.17 STUN candidate gathering | Shipped 2026-05-23 in `2d04d67`. 12.25 **builds on it** by adding passcode-derived signaling on the rendezvous side — no more dependency on Tailscale's signaling for ICE candidate exchange |
| 12.18 D.1 + D.2 (state machine + Https443 Transport) | Shipped 2026-05-23 in `ab8e1ee` + `4442522`. 12.30 **builds on them** by adding UPnP/NAT-PMP/PCP probing and making the SEED wizard the first operator-visible caller of the transport. Coordinates with the still-open 12.18 D.3 (MeshRouterWorker tick wiring) |
| 12.20 roaming migration | Stays — rolling endpoint records (12.27) make migration cheaper since the rendezvous already advertises the new endpoint within a heartbeat |

## Open questions

1. **DHT choice.** libp2p `kad` is the obvious default. Risks: heavy
   dep (adds ~80 crates), boot-up time. Alternative: a smaller
   custom Kademlia (~2k LOC) or the existing IPFS public DHT (zero
   infra). Decision deferred to the 12.24 implementation task.
2. **Probe-relay hosting.** Three single-region VPSes (Q4 lock) is
   the minimum for the 2-of-3 quorum probe. Open: does Mackes
   operate them, or does the operator BYO? Defer to 12.26 task.
3. **Passcode storage on headless peers** (Q5 lock — "headless
   first-class"). libsecret needs a logged-in session. For headless
   peers, fall back to a 0600 file under
   `/var/lib/mde/mesh-passcode` owned by the `mackesd` user;
   document explicitly. Defer to 12.24 task.
4. **DPI realism budget** for 12.30. The Q10 lock requires the 443
   fallback to survive deep-packet-inspection. Concrete test rig
   (bench firewall, what kind of DPI) needs spec before
   implementation can claim acceptance. Lift into 12.30 acceptance.

## Outcome

The /goal directive is met within scope: 10 solutions proposed,
all anchored on the existing 16-char passcode, each closing one
of the 10 gaps surfaced by the 2026-05-23 evaluation. None
introduce new security or monitoring requirements per the §0.10
Round-1 constraint; every solution **reuses** the passcode the
product already mints.

A two-line summary the operator should remember:

> Round 2 promotes the 16-char passcode from auth-credential to
> cryptographic anchor: the same passcode that today gates
> enrollment now derives the rendezvous topic, the TLS root CA,
> the leader-election votes, and the wire encryption. Any peer
> with the passcode finds the mesh anywhere in the world; any
> peer without it cannot even see the mesh exists.
