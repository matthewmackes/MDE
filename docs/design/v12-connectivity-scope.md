# Phase 12 — Connectivity scope & intent (locked 2026-05-19)

> **SUPERSEDED 2026-05-23 by `v2.5-nebula-fabric.md`** for
> everything below the `mackesd_core` library facade. The
> Tailscale + Headscale + self-hosted DERP architecture this
> document codified is retired; Nebula replaces all three. The
> 25-question audience / SLO locks (Q1–Q5 audience, Q7–Q8
> latency, Q16 throughput, Q22 roaming, etc.) remain in force —
> Nebula has to clear the same bar. The transport-mechanism
> locks (Q6 self-hosted DERP, Q10 TCP/443 implementation) are
> retired in favor of the v2.5 design lock.
>
> Read this doc for the user-felt SLOs that still apply. Read
> `v2.5-nebula-fabric.md` for the fabric implementation that
> meets them.

Companion document to `PROJECT_WORKLIST.md § Phase 12.14–12.23`.
Captures the 25-question survey the user ran on 2026-05-19 to lock
the scope, goals, and intent of the Mesh networking platform's
connectivity layer.

The locks here trump anything in earlier design docs when they
conflict — this is the most recent authoritative source on what
"fit-for-purpose" means for Mesh connectivity.

## /goal directive (2026-05-19)

> Review the full sourcecode, documents, worklist and plans. Update
> the worklist with 10 items that will make this platform more
> fit-for-purpose. Do not change the scope, and do not add any new
> security, or monitoring requirements. Primary goal is to allow node
> members to connect efficiently when located on the same LAN but,
> allow instant connectivity from behind any firewall on other networks
> around the world.

## 10 worklist items added (see `PROJECT_WORKLIST.md § 12.14–12.23`)

1. **12.14** — LAN peer auto-detection + direct UDP data path (bypass Tailscale tunnel when peers share a broadcast domain).
2. **12.15** — IPv6-first direct-path preference (skip NAT entirely when both peers have GUAs).
3. **12.16** — Self-hosted DERP relay pool, default-on (replaces Tailscale's public DERP for mesh traffic).
4. **12.17** — ICE/STUN augmentation for symmetric-NAT edges.
5. **12.18** — HTTPS-tunneled fallback over TCP/443 for restrictive firewalls.
6. **12.19** — Multi-path concurrent send for latency-sensitive flows.
7. **12.20** — Roaming-aware connection migration (Wi-Fi → LTE → ethernet).
8. **12.21** — Eager connection bootstrap (sub-50 ms first packet).
9. **12.22** — Same-LAN traffic isolation policy.
10. **12.23** — LAN multicast for high-fanout services.

## 25-question survey — locks

| #  | Area         | Question                                              | **Lock** |
|----|--------------|-------------------------------------------------------|----------|
| 1  | Audience     | Target user profile                                   | **Small business / club, ≤16 peers** |
| 2  | Audience     | Peer-count cap                                        | **16 hard cap** |
| 3  | Audience     | Same-LAN ratio                                        | **~50% LAN, ~50% WAN** |
| 4  | Audience     | Geographic spread                                     | **Single region (one country / continent)** |
| 5  | Audience     | Headless / server peers                               | **First-class — same UX, same features** |
| 6  | Connectivity | Public DERP vs self-hosted                            | **Self-hosted first, public Tailscale DERP as fallback** |
| 7  | Connectivity | LAN-direct detection SLO                              | **Under 30 s (eventual)** |
| 8  | Connectivity | First-packet latency (any-network) SLO                | **Under 3 s** |
| 9  | Connectivity | IPv6 policy                                           | **IPv4-only for now; defer IPv6 to a future phase** |
| 10 | Connectivity | Covert / anti-censorship transports                   | **TCP/443 only — but make it indistinguishable from real HTTPS** |
| 11 | Failure      | When every path fails                                 | **Show offline + persistent diagnostics banner** |
| 12 | Failure      | LAN-path drops, relay still up                        | **Subtle indicator in the panel (no banner)** |
| 13 | Failure      | Re-probe cadence after failure                        | **Gentle exponential backoff: 5 s → 10 s → 20 s → … → 5 min cap** |
| 14 | UX           | Mesh health surface in chrome                         | **Existing status cluster's Mesh indicator is enough** |
| 15 | UX           | Connection-quality warnings                           | **Toast on quality-state transition** |
| 16 | Performance  | LAN-direct throughput ceiling                         | **Gigabit-ethernet-class: ~900 Mbps sustained** |
| 17 | Performance  | `mackesd` idle CPU budget                             | **Best-effort — measure later (no upfront budget)** |
| 18 | Performance  | Battery-aware cadence                                 | **No — same cadence regardless of power source** |
| 19 | Platform     | ARM Linux support                                     | **x86_64 only — ARM is opt-in / unsupported** |
| 20 | Platform     | Multi-homed peers (Wi-Fi + Ethernet)                  | **Use the kernel's chosen single interface; no multi-path** |
| 21 | Platform     | Phone (Android / iOS) peers                           | **Out of scope — Linux desktop only** |
| 22 | Roaming      | Wi-Fi → LTE → Wi-Fi handoff SLO                       | **Under 10 s; brief 'reconnecting' state visible** |
| 23 | Trade-offs   | Latency vs throughput when LAN-direct is slower       | **Throughput wins — pick the higher-bandwidth path** |
| 24 | Migration    | 1.0.6 → 1.0.7 upgrade UX                              | **One-time wizard on first launch after upgrade** |
| 25 | Acceptance   | Phase 12.14–12.23 Done definition                     | **User-felt SLO: 6-peer test fleet, 5 scenarios, 7-day window** |

## Evaluation — how the locks improve each worklist item

Each lock either tightens, loosens, or constrains a specific
Phase 12.14–12.23 item. The rows below trace the impact.

### 12.14 — LAN peer auto-detection + direct data path

- **Q3** (50% LAN ratio): LAN-direct matters for half of all
  peer-pairs in the target fleet — fully worth building.
- **Q7** (under-30 s detection SLO): downgrades from the original
  "sub-500 ms" target. **Simplifies the implementation** —
  polling-based mDNS browse every 5–10 s is fine; no need for
  event-driven netlink-triggered fast-path.
- **Q12** (subtle panel indicator on LAN drop): adds a small
  status-cluster bit to surface "now on relay" vs "now on LAN
  direct."
- **Q16** (gigabit LAN throughput target): tune socket buffers +
  reuse Tailscale's WireGuard ChaCha20 (no jumbo frames, no kernel
  offload).
- **Q23** (throughput-wins trade-off): when the LAN path measures
  slower than the WAN path (saturated home Wi-Fi vs idle fiber
  uplink), `mackesd` honors the bandwidth measurement and chooses
  the higher-throughput path even if both ends share a LAN.
  Materially changes the routing policy: **not "always prefer LAN
  when LAN-direct is reachable"**, but **"prefer the higher-
  bandwidth path, periodically measured."**

### 12.15 — IPv6-first direct-path preference

- **Q9** (IPv4-only mesh, defer IPv6): **12.15 is descoped from
  Phase 12.14–12.23.** Moves to a future phase. Phase 12 assumes
  every peer has working IPv4 (NAT'd or public).

### 12.16 — Self-hosted DERP relay pool

- **Q4** (single region): **only one self-hosted relay needed.**
  Drop the "multi-region relay pool" complexity. The Host-role peer
  runs the relay; that's it.
- **Q5** (headless first-class): the relay can run on a Pi 4 NAS
  without a GUI. Implementation must not assume X11 / GTK on the
  relay-running peer.
- **Q6** (own relay first, public as backup): keeps a fallback path
  for resilience. Headscale DERP map advertises `[self-hosted,
  tailscale-public]` in that order.
- **Q19** (x86_64 only): the relay binary doesn't need an ARM build.
  Simplifies the RPM matrix.

### 12.17 — ICE/STUN augmentation

- **Q1** (small-business audience): users WILL hit symmetric NAT
  (corporate Wi-Fi, hotel networks) — keep this item.
- **Q8** (sub-3 s first packet): the ICE candidate-gathering step
  must complete in under ~1.5 s so the overall handshake fits the
  3 s budget. Adds a deadline constraint.

### 12.18 — HTTPS-tunneled fallback

- **Q1** (small-business audience): corporate firewalls are in
  scope. **High priority.**
- **Q10** (TCP/443 only + make it indistinguishable from real
  HTTPS): **scope-tightens 12.18.** Real TLS handshake, realistic
  SNI, valid cert chain (Let's Encrypt against a domain Headscale
  already serves). Not just "raw bytes over port 443" — the
  fallback must survive deep-packet-inspection that flags
  non-HTTPS traffic.

### 12.19 — Multi-path concurrent send

- **Q3** (50% LAN ratio) + **Q8** (sub-3 s first packet): the case
  where multi-path matters most is the cross-boundary peer pair
  (one LAN + one WAN end) — those exist in 47% of pairs in the
  expected fleet.
- **Q23** (throughput wins): when the relay path is higher-
  bandwidth than the direct path, multi-path send actually HURTS —
  the slower direct path consumes WAN bandwidth without speeding
  up the flow. **Add a guard:** multi-path is enabled only when
  both paths have RTT under 50 ms AND comparable bandwidth (±50%).

### 12.20 — Roaming-aware connection migration

- **Q22** (under-10 s handoff with brief "reconnecting" visible):
  **loosens the original 2 s target.** Means we don't need
  WireGuard session resumption at the SCTP-style multi-path level.
  Simpler: tear down + re-handshake + restore on the new path.
  In-flight TCP streams reset (the user sees their SSH disconnect
  once during a network change — acceptable).
- **Q20** (single best interface): simplifies migration logic. No
  need to keep both Wi-Fi and Ethernet sockets open simultaneously.

### 12.21 — Eager connection bootstrap (sub-50 ms first packet)

- **Q8** (sub-3 s first packet): **12.21 may not be strictly needed
  to meet the SLO.** The 3 s budget is generous enough that the
  full 200–500 ms WireGuard handshake fits comfortably. **Demote
  12.21 from "must-have" to "optimization once 12.14–12.20 ship."**
- **Q17** (best-effort idle CPU): eager bootstrap costs some CPU
  to pre-derive sessions. If we're optimizing CPU later anyway,
  this can wait.

### 12.22 — Same-LAN traffic isolation policy

- **Q23** (throughput wins): **12.22 needs revision.** Forcing
  LAN-direct even when WAN is faster contradicts the locked
  policy. Reframe 12.22 as: "When LAN-direct is the higher-
  throughput path AND has acceptable RTT, prefer it. When WAN
  measures higher-throughput (saturated Wi-Fi case), honor the
  measurement."

### 12.23 — LAN multicast

- **Q3** (50% LAN ratio): LAN multicast pays off when 3+ peers
  receive the same stream. In a 16-peer fleet with ~50% LAN
  sharing, average LAN cluster size is ~6 peers — multicast helps.
- **Q16** (gigabit LAN throughput): multicast at gigabit is fine
  on commodity wired switches. Wi-Fi multicast is famously
  hobbled (capped at the slowest associated client's rate);
  **add a capability probe**: multicast enabled only when every
  receiver is on Ethernet (not Wi-Fi).

## Cross-cutting refinements

### What got descoped

- **12.15 IPv6** (Q9) — moved out of Phase 12.14–12.23 entirely.
- **Battery-aware probe cadence** (Q18) — not implementing.
- **ARM packaging** (Q19) — opt-in / unsupported.
- **Phone peers** (Q21) — out of scope.
- **Multi-region DERP pool** (Q4) — simplified to one relay.
- **Sub-500 ms LAN detect** (Q7) — relaxed to 30 s.
- **Sub-2 s roaming handoff** (Q22) — relaxed to 10 s.
- **Sub-50 ms eager bootstrap** (Q8) — deprioritized.

### What got tightened

- **TCP/443 fallback must look like real HTTPS** (Q10) — adds a
  hard requirement on TLS realism.
- **Self-hosted DERP becomes the default path** (Q6) — was
  optional in the original 12.16.
- **Routing prefers throughput over LAN-presence** (Q23) — flips
  the original 12.22 policy.
- **6-peer test fleet acceptance** (Q25) — concrete done-criterion
  the team can run against.

### Items the survey didn't change

- **12.14 LAN auto-detect** — still in, just at relaxed SLO.
- **12.17 ICE/STUN** — still in, with a new deadline.
- **12.18 HTTPS fallback** — still in, with realism tightening.
- **12.20 roaming migration** — still in, at relaxed SLO.

## Outcome

The /goal directive is met:

- Review of source/docs/worklist: **done** (audit summary in
  `PROJECT_WORKLIST.md § 12.14 header`).
- 10 worklist items added: **done** (`12.14`–`12.23`).
- 25 questions asked + answered: **done** (lock table above).
- Canonical documentation: **this file**.
- Evaluation of improvements: **above section "Evaluation — how the
  locks improve each worklist item"**.

The user did **not** add new security or monitoring requirements —
every locked answer either refines connectivity behavior or
constrains existing items. The 16-peer hard cap, single-region
assumption, and Linux-desktop-only scope all hold.

A two-line summary the operator should remember:

> Phase 12.14–12.23 builds Mesh connectivity for a **small business
> fleet of up to 16 Linux peers in one country**, roughly half
> sharing a LAN. The mesh's own relay is the preferred path with
> Tailscale's public DERP as fallback; corporate firewalls are
> handled via TLS-looking HTTPS-on-443; **the routing layer always
> chooses the higher-throughput path**, even when LAN-direct is
> available. Done = a 6-peer fleet passes 5 named scenarios over a
> 7-day window.
