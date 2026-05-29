<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brand/readme-banner-dark.svg">
    <img alt="MDE — Mackes Desktop Environment for Workgroups" src="assets/brand/readme-banner-light.svg" width="480">
  </picture>
</p>

<p align="center">
  <code>SECURE&nbsp;·&nbsp;SIMPLE&nbsp;·&nbsp;NO-FIXED-CENTER&nbsp;WORKGROUP</code>
</p>

<p align="center">
  <img alt="License GPL-3.0" src="https://img.shields.io/badge/License-GPL--3.0-0b3d91?style=flat-square">
  <img alt="Built with Rust" src="https://img.shields.io/badge/Built_with-Rust-0b3d91?style=flat-square&logo=rust&logoColor=white">
  <img alt="Wayland / sway" src="https://img.shields.io/badge/Wayland-sway-0b3d91?style=flat-square">
  <img alt="Fedora 44+" src="https://img.shields.io/badge/Fedora-44%2B-0b3d91?style=flat-square&logo=fedora&logoColor=white">
  <img alt="Mesh up to 8 peers" src="https://img.shields.io/badge/Mesh-%E2%89%A4_8_peers-0b3d91?style=flat-square">
</p>

---

# MDE — Mackes Desktop Environment

**In one line:** MDE is a custom desktop for Fedora Linux that makes a handful
of your computers behave like one machine.

---

## 01 · What it is

When you turn on a Linux computer, the part you actually touch — the bar along
the bottom, the start menu, the way windows open and close — is the **desktop
environment**. MDE replaces Fedora's default desktop with one built around a
single idea:

> **Your computers should work together as a team, not as separate islands.**

That team is a **mesh**; each computer in it is a **peer**. Up to **8 peers**
join a single mesh — they don't have to be on the same Wi-Fi, or even in the
same country. The mesh handles the hard network parts (routers, firewalls,
NAT, encryption) so you don't have to.

There is **no fixed center**. Every peer is equal in identity and trust — any
peer can take on any role and failover is automatic, so there is no permanent
hub to misconfigure, overload, or attack. (A few subsystems, such as the
mesh-storage metadata master, hold a *floating* role that moves between peers —
never a fixed one.)

```
   peer ──── peer ──── peer
     │  ╲   ╱   ╲   ╱   │
     │   ╲ ╱     ╲ ╱    │
     │    ╳       ╳     │     no hub
     │   ╱ ╲     ╱ ╲    │     every peer equal
     │  ╱   ╲   ╱   ╲   │     encrypted overlay
   peer ──── peer ──── peer
        up to 8 peers · LAN + WAN
```

## 02 · What a mesh gives you

Once two machines are on the same mesh, they can:

- **Share files live.** Every peer shows up as a folder on every other peer.
  Drop a video into your laptop's mesh folder and your media box sees it
  instantly.
- **Share the clipboard.** Copy a link on one machine, paste it on another.
- **Send notifications** back and forth between peers.
- **Stream media** (Jellyfin, Plex, Home Assistant, and 30+ more) from any
  peer to any other peer.
- **Open a shell anywhere** by typing `ssh peer-name` — no keys to copy, no IP
  addresses to remember.

## 03 · Who it's for

People who own a handful of Linux machines and want them to feel connected
without thinking about networking:

- A photographer with a desktop, a laptop, and a NAS.
- A small team with a few workstations and a fileserver.
- A developer with a powerful home tower and a thin travel laptop.
- Anyone tired of shuttling files around on USB sticks.

## 04 · What you see on screen

- **One 40&nbsp;px bar** at the bottom — open apps, clock, battery, network, and
  live mesh status.
- **A Start menu** on the Super key (the Windows key on most keyboards).
- **A focused-app hero** in the corner telling you which window is active.
- **A notification drawer** sliding in from the right with messages from your
  apps and your other peers.
- **A clean, dark theme** by default, with switchable presets:

| Preset | Vibe |
|---|---|
| **#!**       | Black, sparse, monospace — a nod to the old CrunchBang. The default. |
| **Mackes**   | Warm-dark house style with a curated dev toolkit. |
| **Daylight** | Light gray with a cool accent — office apps. |
| **Vanilla**  | Fedora's stock XFCE look; MDE only adds mesh + snapshots. |
| **Node**     | Headless. For fileservers and screenless machines — mesh on, GUI off. |

## 05 · What's inside

A **full Wayland desktop environment built in Rust**. Every interactive piece
of the desktop is replaced with a mesh-aware equivalent:

```
┌─ MDE STACK ──────────────────────────────────────────────────┐
│ WM    sway            Wayland tiling + floating, low latency  │
│ UI    mde-panel       Iced layer-shell bottom taskbar         │
│ UI    mde-workbench   settings + control center (nine groups) │
│ FS    mde-files       mesh-first file manager                 │
│ SVC   mded            unified Rust meta-daemon (one process)   │
│ NET   Nebula + Bus    encrypted overlay (UDP/4242) · event bus │
└────────────────────────────────────────────────────────────────┘
```

- **sway compositor** — tile + float window management, no flicker, low latency.
- **`mde-panel`** — a single bottom taskbar via Wayland layer-shell: start
  menu, focused-app hero, status cluster, clock.
- **`mde-workbench`** — settings and control center in native Iced widgets.
- **`mde-files`** — a mesh-first file manager: peers, inbox, and outbox lead;
  drop a file onto a peer card and it lands on that peer.
- **`mded`** — every long-running service folds into **one** supervised Rust
  process with an in-process worker pool: clipboard, file sync, media sync,
  notification relay, heartbeat, and `org.freedesktop.Notifications`.
- **Nebula overlay + Bus** — an encrypted mesh data plane plus a topic-based
  message bus carrying events, clipboard, and notifications between peers.

Everything else — your apps, files, printers, and games — works exactly the
way Fedora normally works.

### Specification

| | |
|---|---|
| **Identity**        | Secure · Simple · No-Fixed-Center Workgroup |
| **Workgroup unit**  | One person, 3–8 of their own devices |
| **Fleet cap**       | 8 peers |
| **Reach**           | Mixed LAN + WAN, always-reachable |
| **Language**        | Rust |
| **Display server**  | Wayland (sway) |
| **Transport**       | Nebula encrypted overlay |
| **Shared storage**  | LizardFS mesh-storage (goal = N) |
| **IPC**             | Message Bus |
| **Platform**        | Fedora 44+ |
| **License**         | GPL-3.0 |

## 06 · Under the hood

Five ideas hold the platform together: a **layered network**, a **services
mesh** running on top of it, one **unified design**, **zero-trust controls** at
the edge, and continuous **drift detection** that keeps every peer converged.

### Layered network

MDE never touches your physical network — routers, NAT, and firewalls stay
exactly as they are (the *underlay*). On top of it, Nebula builds one encrypted
*overlay*: a single flat address space every peer shares, no matter whose Wi-Fi
or which country each machine is on. The overlay picks the best path
automatically and falls back when a path dies:

```
overlay path        when it's used
─────────────       ──────────────────────────────────────────────
Direct UDP          peers can reach each other (hole-punched)   ← fastest
Lighthouse relay    direct fails; a lighthouse relays the frames
HTTPS/443 tunnel    hostile network; Nebula wrapped in TLS 1.3,
                    byte-indistinguishable from ordinary HTTPS
```

Only **two ports** ever face the public internet: **UDP/4242** for the overlay
and **TCP/443** for the tunnel fallback (lighthouses only). Every other MDE
listener binds the overlay interface and is invisible from outside the mesh.

Above the wire, the mesh daemon (`mackesd`) is itself layered — each tier reads
only the one beneath it:

```
Layer 8  GUI panels — Workbench mesh view + topology renderer
Layer 7  library facade (mackesd_core)
Layer 6  service traits
Layer 5  reconciliation engine                ← drift detection
Layer 4  domain logic — topology · policy · validation · CA
Layer 3  telemetry ingest
Layer 2  persistent store
Layer 1  process supervisor — leader election · systemd · Nebula lifecycle
Layer 0  fabric — nebula.service on every peer
```

### Services mesh

Once the overlay is up, peers stop behaving like separate computers. A small
set of always-on services makes them act like one:

- **`mded`** folds every long-running job into one supervised process with an
  in-process worker pool: clipboard sync, file sync, media sync, notification
  relay, heartbeat, and `org.freedesktop.Notifications`.
- **Bus** — a per-peer message broker carried over the overlay. Commands go to
  `action/<domain>/<verb>` topics and replies return on `reply/<id>`; events
  publish to domain topics (`mesh/conflict`, `mon/cpu`). It moves notifications,
  clipboard, and audit between peers.
- **`mesh-storage`** — a LizardFS volume where *every peer holds every chunk*
  (`goal = N`). Your `~/Documents`, `~/Pictures`, `~/Music`, `~/Videos`, and
  `~/Downloads` *are* the mesh. Metadata has a single master elected among the
  lighthouses; every peer also runs an auto-promotable shadow, so failover is
  automatic and no machine is permanently in charge.
- **Service catalog** — each peer advertises what it runs (Jellyfin, Plex, Home
  Assistant, and 30+ more); any peer reaches any other peer's services straight
  over the overlay.

### Unified design

Every surface — panel, Workbench, file manager, notifications — speaks one
visual language, so the desktop feels like a single product rather than a bag of
apps:

| | |
|---|---|
| **Language** | ChromeOS Classic — flat, calm, `#202124`-class palette |
| **Accent**   | Material You indigo |
| **Icons**    | Material Symbols |
| **Type**     | Roboto (UI) · Intel One Mono (code) |
| **Shape**    | 4 px corners; *flat-but-elevated* — windows stay flat, MDE overlays get soft M3 shadows |
| **Density**  | Three modes — compact 24 px · regular 28 px · comfortable 32 px |
| **Motion**   | Functional, 150 ms ease-out |

Color, spacing, and motion all come from one set of **design tokens**
(`data/css/tokens.css`); a pre-commit lint rejects any hardcoded hex so the
language can't quietly drift. Four presets ship on top (ChromeOS Classic
Light/Dark + Ableton 12 Light/Dark).

### Zero-trust controls

Inside the mesh, trust is deliberately flat — you own every peer, so they fully
trust each other. The *boundary*, though, trusts nothing by default: being on
the same wire grants exactly zero access.

- **No implicit network trust.** Every packet between peers is mutually
  authenticated and AEAD-encrypted by Nebula. LAN position alone gets you
  nowhere — a sniffer sees only encrypted overlay traffic.
- **Per-peer identity.** Each peer carries a certificate minted by the mesh CA;
  the CA private key lives only on the leader and never leaves it.
- **One enrollment credential.** A single passcode gates the join, sealed with
  `systemd-creds` (TPM where available) — the operator's master credential.
- **Revoke + ban.** A lost or stolen peer is CA-revoked and ban-listed: refused
  re-join *even with the correct passcode*.
- **Least privilege.** Every component runs as your user; the only system
  service (Nebula) is capability-bounded (`CAP_NET_ADMIN` only,
  `NoNewPrivileges`) under SELinux enforcing.
- **Bind-scope, enforced.** Every MDE listener binds the overlay interface, and
  a pre-commit lint blocks any new `0.0.0.0` bind from landing.

Full rationale: [`docs/design/security-posture.md`](docs/design/security-posture.md).

### Drift detection

A mesh is only as good as its ability to notice when reality stops matching
intent. `mackesd`'s reconciliation engine wakes on a ~30 s tick, compares
**desired** state against **observed** state, and sorts every difference:

- **Auto-repairable** drift (a transient overlay route dropping, say) is fixed
  silently — the reconciler re-pushes the desired state, backing off
  exponentially (1 s → 60 s) if a repair keeps failing.
- **Manual-review** drift (an *unexpected* peer adjacency that could mean
  tampering) is never touched automatically; it lands in a **Pending Changes**
  inbox for you to approve or reject.

Config changes ride a small state machine — *Draft → Validated → Approved →
Deploying → Applied → Verified* — with explicit *FailedValidation* and
*RolledBack* exits, so a bad revision rolls back instead of half-applying.

Drift is watched at three levels:

| Scope | What it watches | Where |
|---|---|---|
| **Topology** | desired vs. observed peer adjacency | reconciliation engine |
| **Preset**   | active preset vs. live system — three-way per key (revert · adopt · ignore) | Workbench → Maintain |
| **Version**  | each peer's `mde-core` version, surfaced as a skew table | `mesh-storage` peer files |

## 07 · Install

You need a Fedora 44 (or newer) machine. The quickest path:

```sh
curl -fsSL https://raw.githubusercontent.com/matthewmackes/MDE/main/install.sh | bash
```

That command detects your Fedora version, installs MDE, and opens a short setup
wizard that walks you through joining a mesh and picking a preset.

**Prefer `dnf` directly?** MDE ships as two packages — a headless **`mde-core`**
substrate and the **`mde-desktop`** Wayland shell on top:

```sh
sudo dnf config-manager --add-repo \
    https://matthewmackes.github.io/MDE/data/dnf/mde.repo

# Headless / lighthouse substrate only (great on a Fedora Server CLI):
sudo dnf install mde-core

# Full Wayland desktop (sway + the MDE shell):
sudo dnf install mde-core mde-desktop
```

The recommended path is a **clean install from a minimal Fedora Server (CLI)**:
`dnf install mde-core` lands the headless substrate, then `dnf install
mde-desktop` (or `sudo mde-install --profile=full`) builds up to the full
desktop. Or grab an RPM from the
[Releases page](https://github.com/matthewmackes/MDE/releases) and install it
offline — `mde-core` `Provides: mde`, so `dnf install mde` and upgrades from
older 1.x boxes keep resolving.

### No screen? No problem.

Run the install on a fileserver or NAS with no monitor and MDE notices there's
no display: it asks the setup questions in plain text right in the terminal.
The machine joins the mesh as a **headless peer** (`mde-core` only) — it serves
files and runs services for your other computers, but never draws a desktop.

## 08 · The Workbench

Workbench is the settings and control center. Nine groups:

| Group | Covers |
|---|---|
| **Dashboard**   | At-a-glance status of every system service. |
| **Look & Feel** | Theme, fonts, icons, wallpaper. |
| **Devices**     | Display, keyboard, mouse, sound, power. |
| **Fleet**       | Push settings + revisions across peers. |
| **Network**     | Wi-Fi, Ethernet, VPN, mesh peers, firewall, SSH. |
| **System**      | Window manager, workspaces, session, notifications. |
| **Apps**        | Install or remove software, curated per preset. |
| **Maintain**    | Snapshots, drift checks, updates, repair tools. |
| **Help**        | Short topics covering every feature. |

Every Workbench action also has a `mde` subcommand — run `mde help` for the
topic list.

## 09 · Build from source

```sh
git clone https://github.com/matthewmackes/MDE.git
cd MDE
make rpm        # builds the mde-core + mde-desktop RPMs
make test       # runs the test suite
```

The repo holds two halves side by side — a Rust workspace (`crates/`) and a
Python tree (`mackes/`) — and one `make rpm` builds them into installable
packages. Build and contribution details live in
[`CONTRIBUTING.md`](CONTRIBUTING.md).

## Upgrading from 1.x

**v1.x → v2.0.0 is a hard switch:** XFCE is removed, sway becomes the session,
and the binary is renamed `mackes` → `mde` (with one-release shims). `dnf
upgrade` lands the new package automatically; the next login picks up the new
**Mackes Desktop Environment** session from the greeter. See
[`docs/MIGRATION_FROM_V1.md`](docs/MIGRATION_FROM_V1.md) for the walkthrough.

## More

- **What changed in each version:** [`CHANGELOG.md`](CHANGELOG.md)
- **Help pages:** [`docs/help/`](docs/help/)
- **Design + governance:** [`docs/AI_GOVERNANCE.md`](docs/AI_GOVERNANCE.md)
- **Report a bug:** <https://github.com/matthewmackes/MDE/issues>

## License

GPL-3.0. © 2026 Matthew Mackes.

---

<sub>

| PROJECT | IDENTITY | LANGUAGE | DISPLAY | LICENSE |
|---|---|---|---|---|
| Mackes Desktop Environment | Secure · Simple · No-Fixed-Center | Rust | Wayland / sway | GPL-3.0 |

</sub>
