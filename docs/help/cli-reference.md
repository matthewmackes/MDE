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
            --enable-on-boot             # enable mde-session.service
            --skip-snapshot              # don't create initial snapshot
            --yes                        # accept all defaults

mackesd enroll --token '<join-token>'    # join an existing Nebula mesh
mackes leave                             # leave the mesh (keeps MDE installed)
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

## Nebula mesh

```
mackesd enroll --token '<join-token>'    # enroll this peer into an existing mesh
mackesd nebula status                    # overlay IP, cert expiry, active transport
mackesd nebula peer-list                 # all peers + overlay IPs + cert expiry
mackesd nebula regen-certs               # request cert renewal for this peer
```

## Mesh passcode

Every peer in the fleet shares one 16-character passcode. Generate it
on the first peer, then paste the same code on every other peer at
enrol time. `--store` encrypts the code at rest via `systemd-creds`
(TPM when present, host key otherwise) at
`/var/lib/mackesd/mesh-passcode.cred` — the plaintext never lands on
disk. The cred is host-local: each peer encrypts the shared passcode
under its own key, so the file does not replicate across the mesh.

```
mackesd generate-passcode                # print a fresh 16-char passcode
mackesd generate-passcode --store        # also encrypt it to the cred file
            [--cred-path <PATH>]         # override the cred-file location
mackesd rotate-passcode                  # print a new passcode; peers refresh
            [--store] [--cred-path <PATH>]  #   on next heartbeat (reconcile loop)
mackesd show-passcode                    # decrypt + print the stored passcode
            [--cred-path <PATH>]         #   (inverse of generate/rotate --store)
```

## Nebula CA (lighthouse operators only)

```
mackesd ca mint --mesh-id <name> [--cert-lifetime-days 365]
                                         # create a new CA (first-boot only)
mackesd ca rotate [--cert-lifetime-days 365]
                                         # rotate the CA; re-issues all peer certs
mackesd ca list                          # all signed certs + expiry + revocation status
mackesd ca dump-ca                       # print CA cert in PEM format
mackesd ca sign <node-id> [--groups lighthouse,peer]
                                         # sign (or re-sign) a peer cert
mackesd ca revoke <node-id>              # revoke a peer cert + push CRL to all peers
mackesd ca export                        # export encrypted CA backup bundle (stdout)
mackesd ca import                        # import a CA backup bundle (stdin)
```

### Ban list (compromised nodes)

A ban refuses a node-id enrolment mesh-wide — even with a valid
passcode and across a CA rotation. Bans propagate to every peer via
GFS-replicated mesh-home; the enrolment gate enforces the *union* of
every peer's list, so there is no override. `ca unban` only lifts the
entry the local peer set — a ban another peer set must be lifted there.

```
mackesd ca ban <node-id>                 # add a node-id to this peer's ban list
            [--qnm-root <PATH>]          #   (env: QNM_SHARED_ROOT)
mackesd ca unban <node-id>               # lift this peer's ban entry
            [--qnm-root <PATH>]
mackesd ca ban-list                      # print the mesh-wide union the gate enforces
            [--qnm-root <PATH>]
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
`devices`, `network`, `system`, `apps`, `maintain`, `mesh`, `mesh-nebula`,
`mesh-thunar`, `mesh-ssh`, `mesh-services`, `mesh-admin`, `mesh-ops`,
`headless`, `presets`, `troubleshooting`, `keybindings`, `cli-reference`.

## Exit codes

- `0` — success
- `1` — generic error (operation failed)
- `2` — usage error (invalid args / unknown subcommand)
- `3` — not provisioned (mesh op called before `mackes init`)
- `4` — mesh capacity reached (8-peer cap)
- `5` — auth failure (join token rejected, cert expired, etc.)
- `124` — operation timed out
- `127` — required binary not in PATH

## Environment variables

- `MACKES_CONFIG_DIR` — override `~/.config/mackes-shell/`
- `MACKES_DATA_DIR` — override `~/.local/share/mackes-shell/`
- `MACKES_LOG_LEVEL` — `debug` / `info` / `warn` / `error`
- `MACKES_DRY_RUN=1` — print actions without executing (best-effort)
- `MACKES_HEADLESS=1` — equivalent to `--headless`
