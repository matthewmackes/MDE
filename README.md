<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brand/readme-banner-dark.svg">
    <img alt="MDE — Mackes Desktop Environment for Workgroups" src="assets/brand/readme-banner-light.svg" width="480">
  </picture>
</p>

# MDE — Mackes Desktop Environment

**In one line:** MDE is a custom desktop for Fedora Linux that makes
all your computers feel like one machine.

---

## What is a "desktop environment"?

When you turn on a Linux computer, the screen you see — the bar along
the bottom, the start menu, the wallpaper, the way windows open and
close — is called a **desktop environment**. It is the part of the
operating system you actually touch.

MDE is one of those. It replaces Fedora's normal desktop with one
built around a single idea: **your computers should work together as
a team**, not as separate islands.

## What does "work together" really mean?

We call that team a **mesh**. Each computer in it is called a **peer**.
You can have up to 16 peers in a single mesh. They do not have to be
on the same Wi-Fi or even in the same country.

Once two computers are on the same mesh, they can:

- **Share files live.** Every peer shows up as a folder on every
  other peer. Drop a video in your laptop's mesh folder, and your
  TV's media server sees it instantly.
- **Share clipboard.** Copy a link on one machine, paste it on
  another.
- **Send notifications** back and forth.
- **Stream media** (Jellyfin, Plex, Sonarr, Home Assistant, and 30+
  more) from any peer to any other peer.
- **Open a terminal** on any peer by typing `ssh peer-name`. No keys
  to copy, no IP addresses to remember.

The mesh handles the hard network stuff — routers, firewalls, VPNs —
so you don't have to.

## Who is this for?

MDE is built for people who own a handful of Linux machines and want
them to feel connected without thinking about networking. For example:

- A photographer with a desktop, a laptop, and a NAS at home.
- A small business with a few workstations and a fileserver.
- A developer with a powerful home tower and a thin travel laptop.
- Anyone tired of copying files around with USB sticks.

## What you see on screen

- **One bar at the bottom of the screen.** It shows your open apps,
  the clock, the battery, the network, and your mesh status.
- **A Start menu** that opens with the Super key (the Windows key on
  most keyboards).
- **A focused-app hero** in the bottom-left corner that tells you,
  at a glance, which window is active.
- **A notification drawer** that slides in from the right with
  messages from your apps and your other peers.
- **A clean, dark theme** by default. You can pick a different look
  from four built-in **presets** if you want a different vibe.

### The five presets

| Preset | Vibe |
|---|---|
| **#!** | Black, sparse, monospace — a nod to the old CrunchBang Linux. The default. |
| **Mackes** | Warm-dark house style with VS Code, Terminator, and a curated dev toolkit. |
| **Daylight** | Light gray with a cool yellow accent. LibreOffice, GIMP, Thunderbird. |
| **Vanilla** | Fedora's normal XFCE look. MDE only helps with mesh and snapshots — never touches the theme. |
| **Node** | Headless. For fileservers and machines without a screen. Mesh on, GUI off. |

You can switch presets later from the **Maintain** tab.

## How to install

You need a Fedora 44 (or newer) computer. Open a terminal and run:

```sh
curl -fsSL https://raw.githubusercontent.com/matthewmackes/MAP2-RELEASES/main/install.sh | bash
```

That command:

1. Figures out which Fedora version you have.
2. Downloads the latest MDE release.
3. Asks for your password and installs it with `dnf`.
4. Opens a short setup wizard.

The wizard walks you through joining a mesh, picking a preset, and a
few other choices. The whole thing takes about two minutes.

### No screen? No problem.

If you run the same install command on a fileserver or NAS with no
monitor, MDE notices there's no display and asks the setup questions
in plain text right in the terminal instead. The machine joins the
mesh as a "headless peer" — it can serve files and run services for
your other computers, but it never draws a desktop.

### Other install paths

If you'd rather use `dnf` directly:

```sh
sudo dnf config-manager --add-repo \
    https://matthewmackes.github.io/MAP2-RELEASES/data/dnf/mackes-shell.repo
sudo dnf install mde
```

Or download the RPM file from the [Releases page](https://github.com/matthewmackes/MAP2-RELEASES/releases)
and install it offline. (The package name flipped from
`mackes-shell` to `mde` at the 2.0.0 cut — older 1.x boxes pick up
the new name automatically via `dnf upgrade` thanks to the
`Obsoletes` rule on the new spec.)

## What's inside

MDE is a **full Wayland desktop environment** built in Rust. It
replaces every interactive piece of a normal Fedora desktop with
mesh-aware equivalents:

- **Wayland compositor (sway)** — replaces xfwm4 + i3 from the 1.x
  line. Tile + float window management, no compositor flicker, low
  latency.
- **Iced panel (`mde-panel`)** — single 40 px bottom taskbar via
  Wayland layer-shell. Start menu, focused-app hero, status
  cluster, clock.
- **Iced Workbench (`mde-workbench`)** — settings + control center.
  Nine groups, every panel ports to native Iced + libcosmic
  widgets. Theme via the `mackes-theme` Carbon-token adapter.
- **`mde-files` (Artifact Manager)** — mesh-first file manager.
  Sidebar leads with peers + inbox + outbox; LOCAL is collapsed
  behind a disclosure. Drop a file onto a peer card → it lands on
  that peer.
- **`mded` (unified meta-daemon)** — every long-running v1.x Python
  daemon folds into one Rust process with an in-process worker
  pool: clipboard, mdns, fs_sync, media_sync, remmina_sync,
  ansible-pull, kdc_bridge, heartbeat, notification relay, and
  `org.freedesktop.Notifications`. One systemd unit, one process,
  one supervisor.
- **Mesh fleet control plane** — Headscale + Tailscale-WireGuard
  data plane, plus self-hosted DERP relay on the Host-role peer
  (`mde-derper.service`). 16-peer small-business fleet, ~3 s
  first-packet, < 10 s roaming reconnect.

Everything else — your apps, your files, your printer drivers, your
games — works exactly the way Fedora normally works.

## The Workbench app

Workbench is the settings and control center for MDE. Nine groups:

- **Dashboard** — at-a-glance status of every system service.
- **Look & Feel** — theme, fonts, icons, wallpaper.
- **Devices** — display, keyboard, mouse, sound, power.
- **Fleet** — push settings + revisions + Ansible-pull playbooks.
- **Network** — Wi-Fi, Ethernet, VPN, mesh peers, firewall, SSH.
- **System** — window manager, workspaces, session, notifications.
- **Apps** — install or remove software, with curated lists per preset.
- **Maintain** — snapshots, drift checks, updates, repair tools.
- **Help** — short topics covering every feature.

CLI: every Workbench action also has a `mde` subcommand. Run
`mde help` for the topic list.

## Upgrading from MDE 1.x (a.k.a. "Mackes Shell")

**v1.x → v2.0.0 is a hard switch.** XFCE is removed; sway becomes
the session; the binary rename `mackes` → `mde` takes effect with
one-release bin-shims for back-compat. `dnf upgrade` lands the new
package automatically; the next login picks up the new
**Mackes Desktop Environment** session entry from the greeter.

See [`docs/MIGRATION_FROM_V1.md`](docs/MIGRATION_FROM_V1.md) for
the full walkthrough.

## More info

- **What changed in each version:** [`CHANGELOG.md`](CHANGELOG.md)
- **Help pages:** [`docs/help/`](docs/help/) — the same pages you see
  inside Workbench's Help tab.
- **The full design spec** (for the curious): [`docs/MACKES_SHELL_SPEC.md`](docs/MACKES_SHELL_SPEC.md)
- **Report a bug:** <https://github.com/matthewmackes/MAP2-RELEASES/issues>

## License

GPL-3.0. © 2026 Matthew Mackes.

---

### A note for developers

If you want to build MDE from source, run the test suite, or
contribute, the technical build steps live in
[`CONTRIBUTING.md`](CONTRIBUTING.md). In short:

```sh
git clone https://github.com/matthewmackes/MAP2-RELEASES.git mackes-shell
cd mackes-shell
make rpm        # builds the RPM
make test       # runs the test suite
```

The repo holds two projects side by side: a **Python** workbench
(`mackes/`) and a **Rust** panel (`crates/`). One `make rpm` command
builds both into a single installable package.
