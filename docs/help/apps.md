# Apps

Three sub-panels: Install, Remove, Installed.

## Install

The active preset's `apps.install` list, rendered with per-app rows
showing install backend (Fedora repo / third-party repo / AppImage / npm
global) and current install state.

### Curated catalog

Mackes-known apps live in `mackes/app_mgmt.py:CATALOG`. Adding a new app
means adding a `CATALOG` entry; preset YAMLs can name anything in the
catalog or fall back to plain `dnf install <name>` for unknown names.

Current catalog includes:
- **Browsers / clients**: Microsoft Edge, Firefox (Fedora repo)
- **Code editors**: VS Code (Microsoft repo), Cursor (AppImage), Claude
  Code CLI (npm global)
- **Terminals**: Alacritty, Terminator
- **Media**: VLC, Jellyfin Media Player, Strawberry
- **Files / remote**: FileZilla, Remmina, Midnight Commander
- **Utilities**: neofetch, conky, menulibre

## Remove

**Single combined Bloat list** (Q15 lock) covering GNOME-on-XFCE apps +
LibreOffice + XFCE extras (asunder, parole, pragha, xfburn,
transmission-gtk, claws-mail, pidgin). Tick the rows you want gone, click
**Remove selected bloat**. Each removal is logged and tracked in
`~/.config/mackes-shell/removed-by-mackes.json` so it survives across
preset switches.

The old "XFCE components replaced by Mackes" sub-section was retired in
1.0 — the standard XFCE shell is now what Mackes ships, so removing its
core components no longer makes sense.

## Installed

Full `rpm -qa` browser with a filter box and per-row remove. Useful for
finding rogue packages or auditing what's on the system.

## Notes

- All installs use `pkexec dnf install -y` (interactive auth prompt the
  first time, cached after that).
- Third-party repo adds (Microsoft Edge, VS Code) are logged so you know
  what repo got added to your system.
- AppImage installs go to `~/.local/bin/` with a `.desktop` file in
  `~/.local/share/applications/`. Removing them needs manual cleanup.
- npm global installs use `sudo npm install -g <pkg>`.
