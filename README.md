<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brand/readme-banner-dark.svg">
    <img alt="MDE — Mackes Desktop Environment for Workgroups" src="assets/brand/readme-banner-light.svg" width="480">
  </picture>
</p>

<p align="center">
  <code>SECURE&nbsp;·&nbsp;SIMPLE&nbsp;·&nbsp;CENTERLESS&nbsp;WORKGROUP</code>
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

There is **no central server**. Every peer is equal — the workgroup is
*centerless* by design, which means there is no hub to misconfigure, overload,
or attack.

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
| **Identity**        | Secure · Simple · Centerless Workgroup |
| **Workgroup unit**  | One person, 3–8 of their own devices |
| **Fleet cap**       | 8 peers |
| **Reach**           | Mixed LAN + WAN, always-reachable |
| **Language**        | Rust |
| **Display server**  | Wayland (sway) |
| **Transport**       | Nebula encrypted overlay |
| **Shared storage**  | Gluster mesh-home |
| **IPC**             | Message Bus |
| **Platform**        | Fedora 44+ |
| **License**         | GPL-3.0 |

## 06 · Install

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

## 07 · The Workbench

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

## 08 · Build from source

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
| Mackes Desktop Environment | Secure · Simple · Centerless | Rust | Wayland / sway | GPL-3.0 |

</sub>
