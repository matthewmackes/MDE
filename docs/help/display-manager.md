# Display manager — greetd + regreet

**Status:** shipping in v2.7
**Source:** DM-1 through DM-8 (locked 2026-05-24 via the 10-question
display-manager survey)

In v2.7 the Mackes Desktop Environment replaces LightDM with the
greetd + regreet stack. greetd is a minimal display manager;
regreet is its GTK4-based greeter UI. The swap brings the login
screen onto Wayland and gives the greeter the same visual identity
as the rest of the desktop.

## What changed at login

- **Charcoal background** (`#1d1d1f`) — matches the Classic
  ChromeOS palette the rest of MDE uses, so the boot-theater →
  greeter → desktop sequence reads as one continuous environment.
- **Header reads "Mackes Desktop Environment"** instead of the
  generic LightDM "Welcome."
- **You type your username every time.** Neither the username
  nor the last-used session is remembered. This is deliberate
  per the DM survey (Q3 + Q5): a shared workgroup machine
  shouldn't surface "the last person to log in here."
- **Session picker shows MDE only.** Stray `*.desktop` entries
  from other Wayland environments (Plasma, GNOME, etc.) don't
  appear — regreet reads exclusively from `/var/lib/mde/wayland-sessions/`,
  which contains only `mde.desktop`.
- **Three power glyphs bottom-right** — shutdown, reboot, suspend.
  Available before authentication so you can recover from a peer
  that won't accept a password.
- **Clock at the top** in `weekday · YYYY-MM-DD · HH:MM` form,
  refreshing every 5 seconds.

## What didn't change

- **Your password.** PAM stack is unchanged from Fedora's default
  (DM-4 closed as verify-and-document — no custom rules needed).
- **Auto-login.** Still off. If you had LightDM auto-login configured
  before the swap, it doesn't carry over; reconfigure in greetd's
  `[initial_session]` block manually if you really want it.
- **Multi-user systems.** Standard Linux multi-user — log out, type
  another username, log back in.

## Rolling back to LightDM

The swap is reversible. Run as root:

```
sudo systemctl disable greetd.service
sudo systemctl enable lightdm.service
sudo systemctl restart display-manager.service
```

LightDM is still installed (the DM-* commits don't remove it). The
rollback is one command per direction.

## Debugging a failed login

When the greeter rejects a password that should work:

```
sudo journalctl -u greetd.service --boot
```

That stream covers regreet's GTK lifecycle + PAM's auth verdict.
For PAM-specific issues:

```
sudo journalctl SYSLOG_FACILITY=10 --boot
```

When regreet itself fails to start (black screen with no UI),
check the cage compositor:

```
sudo journalctl -u greetd.service --boot | grep cage
```

## Mesh-status chip (DM-7, Tier 2)

When DM-7 ships, the greeter gains a small mesh-status chip
upper-right. Three states:

| State    | Color       | Meaning                                |
|----------|-------------|----------------------------------------|
| Online   | indigo dot  | Nebula overlay is up, peers reachable. |
| Joining  | amber dot   | Overlay starting; not yet routable.    |
| Offline  | grey dot    | No overlay; mesh-storage unavailable.  |

The chip is informational only — login works regardless of mesh
state, and the chip doesn't gate authentication. Click it to view
the full mesh diagnostic page (post-login).

## Files

- `/etc/regreet/regreet.toml` — operator-tunable config; survives
  package upgrades (`%config(noreplace)`).
- `/usr/share/mde/theme/greeter.css` — greeter stylesheet; `@import`s
  `tokens.css` from the same dir so a single token change ripples
  through the greeter alongside the panel + Workbench (DM-6).
- `/usr/share/mde/theme/tokens.css` — shared design tokens
  (colors, fonts, spacing). Single source of truth for cross-
  process theming.
- `/usr/share/mde/greetd/config.toml` — greetd substrate config;
  `apply_display_manager()` (DM-5) copies it into place at
  install-time, replacing Fedora's stock `/etc/greetd/config.toml`.
- `/var/lib/mde/wayland-sessions/` — the curated session dir
  regreet reads from. Contains exactly one `.desktop` file:
  `mde.desktop`.
