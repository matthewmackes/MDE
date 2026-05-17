# Getting Started

The first time you launch Mackes Shell on a fresh machine, it runs the
**first-run wizard** — a 9-screen flow that brings a stock Fedora XFCE
install to a known-good state.

## Wizard screens

1. **Welcome** — spare splash with the Mackes logo.
2. **Environment Scan** — detected hardware, OS, XFCE version, missing
   required packages.
3. **Choose Preset** — pick from `#!` (default), `Mackes`, `Daylight`, or
   `Vanilla`. The headless `Node` preset is hidden here — it's auto-selected
   by `mackes init` on machines without a display.
4. **Appearance** — preview and tweak GTK theme, icons, fonts, wallpaper.
5. **Hardware** — display arrangement, audio sink, power profile.
6. **Network** — enable Mesh VPN, firewall zone, optional VPN import.
   First time only: opens the Tailscale OAuth flow in your browser
   (one-click sign-in via Google / Microsoft / GitHub / email). Subsequent
   peers join via mDNS or a one-time join link — they never see Tailscale.
7. **Restore Point** — opt in to an automatic "initial" snapshot before
   any system changes. Strongly recommended.
8. **Review** — full diff of what will be applied.
9. **Apply** — narrated progress bar streaming every action.

Each step has a live preview where possible — Back un-applies cleanly.

## After the wizard

The wizard hands you off to the **Dashboard**:

- Status strip at top: dots for xfce4-panel, xfdesktop, xfsettingsd,
  xfconf, NetworkManager, sshd
- Drift card (only if your live state has diverged from the active preset)
- Hardware summary
- Quick actions: Appearance · Display · Network · Snapshot · Health · Logs
- Recent activity log

## What got applied

A successful first-run wizard does the following, all logged to
`~/.local/share/mackes-shell/logs/mackes.log`:

- xfconf channels (xsettings, xfwm4, xfce4-panel, xfce4-desktop,
  xfce4-notifyd, xfce4-power-manager, …) set to preset values
- Wallpaper applied per-monitor
- LightDM greeter configured to match (PadOS theme, Carbon icons, IBM Plex
  Sans, standard wallpaper)
- Mackes menu entry installed; legacy xfce4-settings entries hidden
- Initial snapshot created (if you opted in)
- Mesh VPN enabled (on the seed peer's machine)
- ssh enabled by default (`systemctl enable --now sshd`)
- state.json marked provisioned with the chosen preset name

## Joining an existing mesh

If another peer on your local LAN is already running Mackes, the wizard's
Network screen detects it via mDNS and offers a one-click "Join existing
mesh". You don't need to sign into Tailscale or know any URLs — see
**[Mesh VPN](mesh-vpn.md)** for the full discovery story.

For peers on a different network: ask any existing peer to open
**Mackes → Network → Mesh VPN → Add Peer**. They get a QR + paste-link.
Scan or paste it into your fresh peer's wizard.

## Re-running the wizard

Open the workbench header menu → **Run First-Run Wizard…** It re-runs the
wizard against your current state — useful for changing presets or
re-applying after a manual config change.
