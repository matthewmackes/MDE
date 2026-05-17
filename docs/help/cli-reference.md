# CLI reference

Every `mackes` subcommand documented. All subcommands work in both GUI
and headless mode (the GUI binary delegates CLI subcommands to the
headless code path).

## Entry point

```
mackes [--gui|--headless] [--version] [subcommand [args…]]
```

Without a subcommand, `mackes` launches the GUI workbench (or wizard if
not yet provisioned). Auto-detects headless mode if no display is found.

Global flags:
- `--version` — print `mackes <version>` and exit
- `--gui` — force GTK path
- `--headless` — force CLI path

## Setup

```
mackes init                              # interactive first-run setup
mackes init --preset <name>              # preset name (default: node headless / picker GUI)
            --tailscale-authkey=<key>    # skip Tailscale OAuth (cloud-init)
            --enable-on-boot             # enable mackes-node.service
            --skip-snapshot              # don't create initial snapshot
            --join '<mesh-join://link>'  # join existing mesh instead of creating
            --yes                        # accept all defaults

mackes join '<mesh-join://?code=…&ts-key=…&seed-tag=…>'
mackes leave                             # leave the mesh (keeps Mackes installed)
```

## Status

```
mackes status                            # current state: preset, peers, services
mackes peers                             # list mesh peers (DataTable equivalent)
mackes peers --json                      # JSON output for scripting
mackes shares                            # list SSHFS shares (in/out)
mackes services list                     # discovered media services across mesh
mackes services list --peer <peer>       # filter to one peer
```

## Snapshots

```
mackes snapshot create [label]
mackes snapshot list
mackes snapshot restore <name>
mackes snapshot delete <name>
mackes snapshot show <name>              # print manifest.json
```

## Maintain

```
mackes maintain repair                   # re-apply preset + restart panel/desktop
mackes maintain health                   # run all checks; rc=0 if all pass
mackes maintain logs [N]                 # tail last N log lines (default: 50)
mackes maintain logs --follow            # tail -f equivalent
mackes maintain reset                    # reset to preset (overwrites local changes)
```

## Apps

```
mackes apps install <name> [name …]      # install by catalog name (or any dnf pkg)
mackes apps remove <name> [name …]       # uninstall
mackes apps list                         # rpm -qa equivalent (catalog-aware)
mackes apps list --installed-by-mackes   # only what Mackes installed
mackes apps catalog                      # print the curated app catalog
```

## Presets

```
mackes preset list                       # all available presets
mackes preset apply <name>               # apply named preset
mackes preset show <name>                # print preset YAML
mackes preset diff                       # show drift (current vs. active preset)
```

## Mesh services

```
mackes services list
mackes services launch <name>            # xdg-open the service URL
mackes services launch <name> --peer <peer>
mackes services enable-gateway           # enable Layer 3 (Caddy proxy + CA install)
mackes services disable-gateway
mackes services catalog                  # print the service catalog
```

## Mesh SSH

```
mackes ssh <peer-name>                   # open SSH session (prefers Layer B / TS-SSH)
mackes ssh <peer-name> --layer A         # force Layer A keys
mackes ssh <peer-name> --layer B         # force Layer B identity
mackes ssh <peer-name> -- <command>      # run command non-interactively
mackes ssh keys list                     # see distributed keys
mackes ssh keys redistribute             # force re-publish
mackes ssh policy show                   # current ACL
mackes ssh policy edit                   # opens $EDITOR on policy YAML
mackes ssh audit [N]                     # last N SSH audit records
```

## Mesh notifications

```
mackes notify <peer> "message"           # send notification to peer
mackes notify <peer> "title" --body "long body" --urgency=high
mackes notify --all "message"            # broadcast to every peer
```

Useful from cron / scripts on headless nodes.

## Mesh VPN

```
mackes mesh status                       # VPN-only status
mackes mesh add-peer                     # generate join link (prints + writes to ~/.config/...)
mackes mesh remove-peer <name>           # revoke a peer
mackes mesh elect-control                # force re-election (dev/debug)
mackes mesh snapshot list                # VPN state snapshots (separate from app snapshots)
```

## Daemon

```
mackes daemon                            # long-running process; what mackes-node.service runs
                                         # supervises qnmd + mesh modules; not for interactive use
```

## Uninstall

```
mackes uninstall                         # interactive (TUI confirm)
mackes uninstall --yes                   # bypass confirm (for scripts)
mackes uninstall --keep-snapshots        # don't delete ~/.local/share/mackes-shell/snapshots
```

## Help

```
mackes help                              # list all help topics
mackes help <topic>                      # print topic (rendered as plain text)
mackes help --open <topic>               # open in $PAGER
```

Available topics: `index`, `getting-started`, `dashboard`, `look-and-feel`,
`devices`, `network`, `system`, `apps`, `maintain`, `mesh`, `mesh-vpn`,
`mesh-thunar`, `mesh-ssh`, `mesh-services`, `headless`, `presets`,
`troubleshooting`, `keybindings`, `cli-reference`.

## Exit codes

- `0` — success
- `1` — generic error (operation failed)
- `2` — usage error (invalid args / unknown subcommand)
- `3` — not provisioned (mesh op called before `mackes init`)
- `4` — mesh capacity reached (16-peer cap)
- `5` — auth failure (Tailscale OAuth declined, code expired, etc.)
- `124` — operation timed out
- `127` — required binary not in PATH

## Environment variables

- `MACKES_CONFIG_DIR` — override `~/.config/mackes-shell/`
- `MACKES_DATA_DIR` — override `~/.local/share/mackes-shell/`
- `MACKES_LOG_LEVEL` — `debug` / `info` / `warn` / `error`
- `MACKES_DRY_RUN=1` — print actions without executing (best-effort)
- `MACKES_HEADLESS=1` — equivalent to `--headless`
