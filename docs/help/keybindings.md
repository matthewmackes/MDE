# Keyboard shortcuts

Mackes Shell respects standard XFCE keybindings + ships a small set of
Mackes-specific accelerators when the workbench window is focused.

## Mackes window

| Shortcut | Action |
|---|---|
| `Ctrl+,` | Open Look & Feel → Appearance |
| `Ctrl+Shift+S` | Create a new snapshot (Maintain → Snapshots) |
| `Ctrl+Shift+R` | Reapply active preset (Maintain → Repair) |
| `Ctrl+Shift+H` | Open Help (this guide) |
| `Ctrl+Shift+L` | Open Logs (Maintain → Logs) |
| `Ctrl+W` | Close window |
| `F5` | Refresh current panel |
| `Esc` | Close popover / dialog |

## Mesh in Thunar

When viewing `mesh:///`:

| Shortcut | Action |
|---|---|
| `Ctrl+F` | Per-folder search (native Thunar) |
| `Ctrl+Shift+F` | Mesh-wide search (Mackes-augmented) |
| `Ctrl+D` | Drop selected files on mesh (opens destination picker) |
| `Ctrl+Shift+P` | Pin selected clipboard items |
| `Delete` | Delete from mesh (with confirm) |

## XFCE-managed shortcuts

Configured in **Devices → Keyboard → Keyboard Shortcuts** (writes to
`xfce4-keyboard-shortcuts` xfconf channel). Common defaults:

| Shortcut | Action |
|---|---|
| `Super` | Open Whisker Menu |
| `Super+E` | Open Thunar (Files) |
| `Super+T` | Open Terminator |
| `Super+L` | Lock screen |
| `Super+D` | Show desktop |
| `Super+1..9` | Switch to workspace N |
| `Ctrl+Alt+T` | Open terminal |
| `Print` | Screenshot (full screen) |
| `Alt+Print` | Screenshot (active window) |
| `Shift+Print` | Screenshot (region) |

## Mesh SSH

`mackes ssh <peer>` from any terminal opens a session to the named peer.
Tab-completion of peer names if you've enabled the Mackes shell
completions (`source /usr/share/bash-completion/completions/mackes`).
