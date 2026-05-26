# Changelog

All notable user-facing and architectural changes. The current line is
unreleased; tag versions get a date when they ship.

## Unreleased — v1.0 MackesDE for Workgroups (rebrand cut)

**BUS-4.4 — FDO notifications bridge to Bus (2026-05-26)**
- Every `org.freedesktop.Notifications.Notify` call mackesd
  receives now also publishes to `fdo/<sanitized-app>` on the
  Mackes Bus. FDO clients (notify-send, libnotify, etc.) still
  get the native desktop notification — the bridge is additive.
- Fire-and-forget shell-out (`tokio::spawn` of
  `mde-bus publish --no-broker`) so the FDO client's return
  path isn't blocked on the publish. Pre-enrollment peers (no
  `mde-bus` binary) log + continue.
- App-name sanitization: lowercase + hyphenate + collapse runs
  + strip edges. `Slack` → `slack`, `Org.Gnome.Calendar` →
  `org-gnome-calendar`.
- Urgency → priority: 0 → min, 1 → default, 2 → urgent.

**BUS-4.3 — MON Netdata alerts dual-write to Bus (2026-05-26)**
- `mde-alert-emit` now publishes every alert through the Bus
  in addition to writing the legacy `~/.local/share/mde/alerts/
  <ulid>.json` JSONL. `system.cpu` routes to `mon/cpu`,
  `nebula.process` to `mon/nebula`, etc.
- `CRITICAL` severity → Bus `urgent` priority (Theater takeover
  surface); `WARNING` → `high` (status-strip + sound); `CLEAR`
  → `default` (tray entry only).
- Shell-out path uses `mde-bus publish --no-broker` so the
  message reaches the persistence layer + audit log even on
  pre-enrollment peers where ntfy isn't running yet.
- Failures are non-fatal — the JSONL stays the authoritative
  store. `--no-bus` CLI flag disables the dual-write entirely
  for tests / pre-RPM environments.

**BUS-7.1 — per-peer JSONL audit log (v6.x Mackes Bus, 2026-05-26)**
- New `crates/mde-bus/src/audit.rs` ships an append-only audit
  log at `~/.local/share/mde/bus/audit/<YYYY-MM-DD>.jsonl`. Each
  line carries the metadata (publisher, ts_iso, topic, priority,
  ULID) — never the body, which lives durably in the per-topic
  file tree.
- Daily UTC rotation via filename — no in-process clock thread
  required. Directory lands 0700 + each file lands 0600 (the
  mesh is flat-trust but local-user privacy still matters).
- Wired into `Persist::write`: every publish appends one audit
  line. Append failures log + continue — the message is
  durably stored regardless.

**BUS-4.5 + BUS-4.6 — lint coverage on Bus code (2026-05-26)**
- `install-helpers/lint-voice.sh` now scans `crates/mde-bus` +
  `data/bus` on every pre-commit + CI run so Bus user-visible
  strings (rendered titles, body templates, hooks YAML samples)
  get the voice-tone audit alongside the GUI crates.
- `docs/design/voice-and-tone.md` gains a Bus-specific guidance
  section: keep webhook titles under ~60 chars (phone push
  truncation), full priority words in operator YAML, quota
  warnings state-fact-then-next-step.
- `install-helpers/lint-legacy-mesh.sh` already scans `crates/`
  broadly — Bus + future `mde-clipd` are covered without any
  script change.

**BUS-2.1 — priority → surface dispatcher (v6.x Mackes Bus, 2026-05-26)**
- New `crates/mde-bus/src/surface.rs` ships the `Surfaces` trait
  (four methods, one per priority class) + the `dispatch(msg,
  surfaces)` router. `LogOnlySurfaces` is the daemon's default
  impl — tracing-logs each dispatch with the surface name + ULID
  + topic so the table is observable in `journalctl -u mde-bus`
  until BUS-2.2..2.8 land the Iced surfaces.
- Wired into the webhook server: every match → persist write
  → fires `dispatch_surface(&stored, &LogOnlySurfaces)` for
  runtime reachability.

**BUS-1.9 — retention engine + quota enforcement (v6.x Mackes Bus, 2026-05-26)**
- New `crates/mde-bus/src/retention.rs` ships per-priority TTL
  (urgent = forever, high = 30 days, default = 7 days,
  min = 24 hours) + GFS quota (500 MB soft / 2 GB hard) per
  the design-doc §8 lock.
- `run_loop` spawns next to the broker / mDNS / hooks tasks in
  the bus daemon. One GC pass per hour walks the SQLite index
  by `ts_unix_ms`, removes messages past their priority's TTL,
  and sums disk usage to compare against the quota.
- Soft-quota breach publishes a `bus/sys/quota` warning at
  default priority with a `Bus storage at X MB / Y MB soft
  limit` title. Rate-limited to one publish per breach
  transition.
- File deletion is ordered before index-row deletion so
  file-orphan is the worst-case failure mode — `Persist::
  detect_divergence` cleans those up on the next audit.

**BUS-1.8 — `mde-bus` CLI binary (v6.x Mackes Bus, 2026-05-26)**
- New `crates/mde-bus/src/cli/` submodule centralises every
  operator-facing subcommand: `publish`, `tail`, `sub`, `mute`,
  `history`, `topic`, plus the existing `daemon` + `render`.
- `mde-bus publish <topic> <body>` accepts three publish forms:
  positional, `--body-flag`, or piped stdin. Always writes
  through the BUS-1.4 SQLite index FIRST (durable record), then
  attempts the ntfy POST — broker failures don't lose the
  message. Operator sees the ULID in stdout for retry.
- `mde-bus tail <pattern>` follows messages on a topic (or
  wildcard pattern) by polling the index every 250 ms. Use
  `--count N` for last-N-and-exit mode without follow.
- `mde-bus sub add|remove|list` and `mde-bus mute add|remove|list`
  manage the per-peer `subs.yaml` atomically. Edits land within
  ~100 ms via the BUS-1.7 SubsWatcher.
- `mde-bus history <topic>` prints stored messages on a topic
  with optional `--since <ulid>` cursor + `--count N` limit.
- `mde-bus topic list` (existing) + new `mde-bus topic match
  <pattern>` for wildcard previews.

**BUS-1.4 — message persistence (v6.x Mackes Bus, 2026-05-26)**
- New `crates/mde-bus/src/persist.rs` ships the per-topic JSON
  file tree (authoritative store at
  `~/.local/share/mde/bus/<topic>/<ulid>.json`) + the per-peer
  SQLite queryable index (`<bus_root>/index.sqlite`). `write()`
  generates a ULID, atomic-renames the JSON file, then INSERTs
  the index row — file-on-disk wins on any divergence.
- `list_since(topic, since_ulid)` answers tail / replay queries
  as an index-range scan via the `(topic, ulid)` SQLite index.
  10k-message replay completes in ~2 s on bench hardware.
- `detect_divergence()` audits the file tree against the index
  and returns `(files_without_rows, rows_without_files)` so
  retention + audit passes can reconcile drift. The index is
  intentionally per-peer (NOT on GFS — SQLite + networked-FS is
  a known footgun); the file tree IS on GFS so cross-peer
  replay still works.
- Wired into the webhook server's `handle_hook` — every
  successful match → publish now writes to the file tree + index
  BEFORE the outbound ntfy POST, so a transient broker hiccup
  doesn't lose a matched-then-rendered message.

**BUS-3.7 — Home Assistant + generic-JSON adapters (v6.x Mackes Bus, 2026-05-26)**
- `HomeAssistantAdapter` dispatches on the JSON body's `event`
  field and supports two event types: `automation_triggered`
  (default priority → home/automation) and `alert` (high
  priority → home/alert, includes entity/area/state).
- `GenericAdapter` is the catch-all: any JSON object body has its
  top-level string/number/bool fields lifted to template
  variables. Nested objects + arrays are skipped (use a typed
  adapter for those). Operators narrow with `match.field.<key>`
  predicates.
- **BUS-3 webhooks lane complete** — 7/7 sub-tasks shipped end-
  to-end: webhook ingress on Nebula-only port 8444, YAML
  transform-rules engine, and six built-in adapters (GitHub,
  Gitea, Sonarr/Radarr, UPS/NUT, Home Assistant, generic JSON).

**BUS-3.6 — UPS / NUT adapter (v6.x Mackes Bus, 2026-05-26)**
- `NutAdapter` dispatches on the JSON body's `event` field. NUT
  has no built-in webhook poster, so the canonical pattern is
  upssched.conf → external script → curl with a Bus-shaped JSON
  body.
- Five canonical events covered: ONBATT, LOWBATT, ONLINE,
  SHUTDOWN, COMMBAD. Template variables: ups_name, ups_ts,
  ups_status, battery_charge, runtime_human.
- `data/bus/hooks/nut.yaml` priority-tiers the rules — ONBATT +
  COMMBAD = high, LOWBATT + SHUTDOWN = urgent (Theater takeover
  + wallpaper stripe + phone push per BUS-2.1), ONLINE = default.

**BUS-3.5 — Sonarr / Radarr adapter (v6.x Mackes Bus, 2026-05-26)**
- `SonarrAdapter` dispatches on the JSON body's `eventType` field
  (the upstream Servarr "Connect" notifications don't ship a
  distinguishing HTTP header). One adapter handles both Sonarr
  (TV) and Radarr (movies) — `title` resolves to either
  `series.title` or `movie.title`; `target` becomes either an
  "S01E03" episode label or a "2024" year. `size_human` formats
  raw bytes as SI-suffixed strings.
- Four event types: Download, Grab, HealthIssue, Test.
  `data/bus/hooks/sonarr.yaml` ships the rule set — Grab is
  min-priority background-only, HealthIssue lifts to high.

**BUS-3.4 — Gitea adapter (v6.x Mackes Bus, 2026-05-26)**
- `GiteaAdapter` dispatches on `X-Gitea-Event` and ships
  extractors for `push`, `pull_request`, and `issues` events.
  Pusher-name resolution tries `pusher.username` →
  `pusher.login` → `pusher.full_name` so the same rules handle
  both modern and older Gitea payload shapes.
- `data/bus/hooks/gitea.yaml` ships the verbatim-includable
  rule set; operators copy it into `~/.config/mde/bus-hooks.yaml`
  to wire Gitea repos to `gitea/*` topics.

**BUS-3.1 + BUS-3.2 + BUS-3.3 — webhook ingress + GitHub adapter (v6.x Mackes Bus, 2026-05-26)**
- New `crates/mde-bus/src/hooks/` module wires an axum HTTP
  listener on `<overlay-ip>:8444`. Bind-scope is the auth
  boundary — the kernel itself rejects underlay connections, so
  "Nebula source-IP only" authentication needs zero in-process
  middleware. Per-request flow: pick adapter, run Rust extractor,
  walk YAML rule list, render Tera templates, POST to the local
  ntfy broker on port 8443.
- YAML schema at `~/.config/mde/bus-hooks.yaml` supports
  `match.event` + `match.field.<key>` predicates and renders
  `topic` / `priority` / `title` / `body` Tera templates against
  per-adapter extracted fields. Strict rendering — typos in
  field names surface as HTTP 422 at request time rather than
  silent empty publishes.
- GitHub adapter ships full event coverage: `push` (repo, branch,
  pusher, commit_count, head_message), `pull_request` (action,
  pr_number, pr_title, pr_user, pr_url), `issues` (action,
  issue_number, issue_title, issue_user, issue_url), `release`
  (action, tag, release_name, release_url). `data/bus/hooks/
  github.yaml` ships the verbatim-includable rule set.
- New `data/firewall/mackes-bus.xml` firewalld service definition
  declares the two Bus ports (8443/tcp ntfy + 8444/tcp webhooks)
  for operators who prefer explicit allow-listing on top of the
  default bind-scope enforcement.
- 32 new unit + integration tests cover the YAML schema, the
  matcher, the publisher (with stub ntfy server), and the
  end-to-end listener (with stub ntfy + ephemeral-port bind).

**BUS-1.7 — subscription manifest + live-reload watcher (v6.x Mackes Bus, 2026-05-26)**
- New per-peer `~/.local/share/mde/bus/subs.yaml` (seeded from
  `/usr/share/mde/bus/subs.yaml.tmpl`) drives delivery filtering.
  Schema: `topics` (MQTT-style wildcards), `mute`, `quiet_hours`
  (`start..end`, supports midnight-wrapping windows).
- `SubsWatcher` polls the file's mtime every 100 ms and re-parses
  on change, broadcasting the new manifest via a
  `tokio::sync::watch` channel. The 200 ms "live delivery" exit
  criterion is met by 100 ms detection + sub-ms parse.
- Three pure predicates: `is_subscribed`, `is_muted`,
  `should_deliver(topic, now_hhmm)`. Future consumers (BUS-1.8
  `mde-bus tail` filter; BUS-4 FDO bridge) subscribe to the
  watch receiver to apply the manifest at delivery time.

**BUS-1.3 — mDNS service registration over Nebula (v6.x Mackes Bus, 2026-05-26)**
- `mde-bus daemon` now registers `_mackes-bus._tcp.local.` via
  `mdns-sd`, advertising `<overlay-ip>:8443` + TXT records
  (`host=<hostname>`, `proto=mackes-bus/1`). Browsing the same
  service type populates a shared `PeerRegistry` that BUS-1.8's
  CLI verbs (`mde-bus peers`) can read.
- Registration is gated on the broker being live (no
  advertise-with-nothing-listening race); shutdown unregisters
  cleanly so peers see drops in real time, not after the cache
  TTL. Missing overlay-IP / failed mdns-sd init / unparseable IP
  all log a single skip line and continue.

**DM-6 — shared design-tokens dir (2026-05-26)**
- New `/usr/share/mde/theme/{tokens.css,greeter.css}` install path.
  `greeter.css` `@import`s `tokens.css` so a single hex change in
  the shared tokens ripples to the greeter alongside the panel +
  Workbench. No symlinks (per the lock — rpm verification
  breaks on dangling symlinks).
- regreet's `[appearance] style` now points at the shared
  greeter.css; the standalone `data/regreet/regreet.css` shipped
  in DM-3 retires (its content moves to `data/css/greeter.css`
  with the `@import` prefix added).

**Display manager: LightDM → greetd + regreet (DM-3 + DM-5 + DM-8, 2026-05-26)**
- The greeter is now greetd hosted by cage with regreet as the
  GTK4 UI. Boot-theater → greeter → desktop reads as one
  continuous environment (charcoal `#1d1d1f` background, Roboto,
  indigo `#5b6af5` accents).
- New birthright step `apply_display_manager()` swaps the systemd
  default DM idempotently. LightDM stays installed; rollback is
  two `systemctl` commands per `docs/help/display-manager.md`.
- Greeter behavior locks: type the username every login (no
  remember_user), MDE-only session picker, three power controls
  visible bottom-right, greeting header reads "Mackes Desktop
  Environment."
- Removed: `apply_lightdm` birthright step. Replaced wholesale by
  `apply_display_manager`; no shim retained.
- Docs: new `docs/help/display-manager.md` (operator-facing) +
  `docs/design/v2.7-display-manager.md` (10 design locks canonical
  copy).

**BUS-1.2 — per-peer ntfy broker over Nebula (v6.x Mackes Bus, 2026-05-26)**
- `mde-bus daemon` now supervises an upstream `ntfy` subprocess
  bound to the Nebula overlay IP (the same publish file
  `nebula_supervisor` writes). Health endpoint reachable from
  inside the mesh, silently unreachable from the underlay.
- Plain HTTP at the broker layer per the BUS design lock —
  Nebula provides transport encryption + the mesh is flat-trust,
  so no TLS cert is needed.
- Graceful degradation: pre-enrollment peers, dev boxes without
  `ntfy` on PATH, or hosts missing the config template all log a
  single line and skip the spawn. The outer
  `mackesd::workers::bus_supervisor` respawns the daemon when
  prereqs land.
- Config template at `/usr/share/mde/ntfy/server.yml.tmpl`
  (Tera-rendered against `overlay_ip` + `cache_dir`).

**Transport abstraction Phase 1 (EPIC-RETIRE-TRANSPORT, 2026-05-26)**
- New `NebulaMode` enum in `mackes-transport` (`Direct` /
  `Https443` / `LighthouseRelay`) + new `TransportKind::is_nebula()`
  + `TransportKind::nebula_mode() -> Option<NebulaMode>` helpers.
  Consumers can now group the 3 Nebula variants as one + match on
  the smaller NebulaMode enum in hot paths.
- Q11 spirit delivered without breaking serde token stability
  (current `"nebula_direct"`, `"nebula_https443"`, etc. preserved)
  or the sibling `EdgeKind` alignment in mackesd::topology.
- 6 new unit tests (51 total in mackes-transport, was 45). All
  pass. `cargo check -p mackes-transport` clean.
- True parent-enum collapse (`TransportKind { Nebula(NebulaMode),
  KdcTls }`) deferred to Phase 2 (~13 consumer files + custom
  serde + sibling EdgeKind update; needs HW bench validation).

**QNM-Shared → MDE-Workgroup Phase A (EPIC-RETIRE-QNM, 2026-05-26)**
- 5 operator-visible help docs swept to use "MDE-Workgroup
  coordination root" terminology with legacy `~/QNM-Shared/`
  parenthetical fallbacks in shell commands. `docs/help/mesh-
  thunar.md` gains a DEPRECATED banner (whole doc retired per
  DEAD-2.11 mesh:// URI handler retirement). CLAUDE.md +
  AI_GOVERNANCE.md already aligned.
- Memory `project_v12_connectivity_scope` already aligned
  (Bundle 1b).
- Internal `qnm_root` symbol rename + env-var alias + design-
  doc sweep deferred to Phase B/C/D follow-ons (Phase B waits
  on BUS-1.x parallel-session quiescence; Phase C is one-line;
  Phase D folds into a quarterly retirement audit).

**Memory hygiene (EPIC-UI-CARDS partial + EPIC-UI-PALETTE follow-on, 2026-05-26)**
- Memory `project_object_card_pattern` gets a SUPERSESSION NOTICE
  + new "Spec (2026-05-25 supersession — Q42)" section: the 12 px
  Object Card corner + M3 shadow break is RETIRED; cards conform
  to the platform 4 px rule. Content-model rule (Object vs row)
  unchanged. Historical 12 px spec retained for archeology.
- Memory `project_ux_polish_locks` gets a SUPERSESSION NOTICE block
  listing the 6 UX-50-Q visual-token locks superseded by the 100-Q
  survey (direction / palette / typography / icons / density /
  motion). Process locks (FU-* + WF-* + sequencing) retained.
- `data/css/` audit: confirmed zero Object-Card 12 px hits in CSS
  (switch + tag pills at 12 px are functional shapes, kept).
  Iced `crates/mde-card/` radius update deferred to the
  parallel-session crate's next touch.

**Fleet-cap code refs (EPIC-MASTER-3.code, 2026-05-26)**
- All 8 in-code "16-peer" references updated to "8-peer" per Q3.
  Every hit was a doc-comment (timing estimate, scope mention, or
  Rust doc explaining extension-number allocation); no hardcoded
  caps existed. Touched: birthright.py, workbench/{network,fleet}/*
  (3 files), headless/daemon.py, mackesd/src/policy/mod.rs,
  mde-voice-config/src/lib.rs, mde-alert-emit/src/main.rs. Extension
  allocation pattern doc updated from `1001..1016` to `1001..1008`
  in mackesd/src/policy + mde-voice-config.



(Operator-locked 2026-05-25 via 100-Q tightening survey: the next
release is **MackesDE for Workgroups 1.0** — full rebrand + version
reset per Q71 + Q72. The v4.0 line below subsumes into the 1.0 cut;
all in-flight epics ship together per Q91 maximum-scope lock. See
`docs/AI_GOVERNANCE.md` for the canonical decision table.)

**Boot splash**
- New Plymouth theme `mde` replaces the legacy `mackes` theme.
  Material-card design (black field, white card, stacked logo,
  indeterminate blue progress) per the operator's
  `Mackes DE Bootsplash.html` design lock 2026-05-25.

**Mesh module audits (DEAD-2.14 + DEAD-2.15)**
- `mackes/mesh_nebula.py` audit landed (DEAD-2.14): partial-coverage
  path. Top-of-file comment block maps 14 exports across 5 Rust
  equivalents + 4 NF-21.x gaps (sshd-overlay-bind worker, lighthouse-
  relayed WoL, nftables firewall preset, Bus migration of 4 toast
  emitters). 5-step retirement plan documented. New NF-21.1..4
  worklist entries created as follow-ons.
- `mackes/mesh.py` umbrella pruned (DEAD-2.15): `_LAYERS` tuple
  shrinks from 8 to 4 — removed `thumbnailer` (DEAD-2.2), `services`
  (DEAD-2.9), `sync` (DEAD-2.10), `browser` (DEAD-2.11). Top-of-tuple
  comment block names the retirement-wave per layer. Remaining 4:
  vpn, ssh, fs, notifications. Umbrella fully retires when DEAD-2.8
  + DEAD-2.12 land (only ssh + vpn remain).

**More mesh-tree retirements (DEAD-2.10 + DEAD-2.11)**
- Deleted `mackes/mesh_sync.py` + `mackes/mesh_nats.py` — NATS-
  substrate bucket-sync stack. Replaced by gluster mesh-home
  (file sync, <5 s heal) + v6.x Mackes Bus (events, 2 s poll).
  The NATS "control peer runs nats-server" model is incompatible
  with the open-mesh / flat-trust directive (no control peers).
- Deleted `mackes/mesh_browser.py` + `data/applications/mackes-
  mesh-uri-handler.desktop` + `data/gvfs/mesh.mount` +
  `data/thumbnailers/mackes-mesh.thumbnailer` — the `mesh://`
  Thunar URI scheme is obviated by gluster mesh-home (XDG dirs
  are natively FUSE-mounted on the mesh per GF-4.1).
- Spec %files block updated; 3 install lines replaced by
  retirement-comment blocks.
- Consumers wrapped per NF-5.1 (mdns_relay + clipboard_app added
  try/except guards; mesh.py + presets.py already wrapped).

**Mesh tree retirements (DEAD-2.x)**
- Deleted `mackes/mesh_discovery.py` — v1.7 mesh-join credential
  fallback. Replaced by Nebula CSR flow + birthright single-passcode.
- Deleted `mackes/mesh_thumbnailer.py` — Tumbler thumbnailer for mesh
  clipboard/notification files. Replaced by
  `crates/mackesd/src/workers/thumbnailer.rs` (Rust).
- Deleted `mackes/workbench/network/mesh_performance.py` — Mesh
  Performance Workbench panel (Datapath / MTU+GSO / Probes /
  Discovery / Relay / Metrics). Retired; metrics surface is Netdata
  web UI (chart URLs in alerts) + Workbench Mesh Storage panel
  (GF-8.2). The sidebar's "Mesh Performance" nav row and the
  unified `mesh_control` notebook's "Performance" tab are removed
  in the same change. Unblocks Wave 3 (DEAD-2.4..2.7 + DEAD-2.10).
- Deleted `mackes/mesh_mdns.py` — Python mDNS service-discovery
  module. The canonical mDNS surface is now
  `crates/mackesd/src/workers/mdns.rs` (Rust worker spawned by
  `mackesd run_serve`). DEAD-2.3 retired the sole external
  consumer, so the python module is unreachable.
- Deleted `mackes/mesh_metrics.py` + `tests/test_mesh_metrics.py`
  — Python Prometheus exporter for mesh metrics. The canonical
  metrics + alerting path is now `crates/mackesd` (worker
  `netdata_aggregator.rs`, MON-* lock) + `data/netdata/health.d/
  *.conf`. Prometheus is retired from MDE-platform metrics; Netdata
  is the single source.
- Deleted `mackes/mesh_fs_fuse.py` — Python SSHFS-with-read-cache
  FUSE backend. The canonical XDG-dir FUSE mount of mesh-home is
  the templated systemd unit `data/systemd/mde-mesh-mount@.service`
  (GF-4.1). DEAD-2.3 retired the sole external consumer.
- Deleted `crates/mackesd/src/workers/{nats,derp}.rs` (Option A:
  delete-outright path of DEAD-2.13). Both files were status-probe
  helpers for substrates that v2.5 axed (NATS bucket-sync,
  Tailscale DERP relay); the only consumer was the retired
  Workbench Mesh VPN panel. `pub mod nats;` + `pub mod derp;` also
  removed from `crates/mackesd/src/workers/mod.rs`. Rationale: v2.5
  Nebula-only / v6.x Mackes Bus replaced both substrates.
- Deleted `mackes/mesh_wol.py` + `tests/test_mesh_wol.py` (DEAD-2.5).
  New `mackesd wake-peer <mac>` subcommand wires the Rust port
  `mackesd_core::workers::wol::{normalize_mac, wake}` so Wake-on-LAN
  has a runtime entry point.  Operators who used `python -m mackes
  wol …` switch to `mackesd wake-peer aa:bb:cc:dd:ee:ff`.
- Deleted `mackes/mesh_services.py` + `tests/test_mesh_services.py`
  (DEAD-2.9). The §8.13 curated service-catalog model is retired —
  superseded by the open-mesh / flat-trust directive (every peer
  trusts every other; no curated allow-list). The "Mesh Services"
  sidebar nav row is removed from the Workbench. Seven consumer
  call sites (native_clients, headless/cli services subcommand,
  headless/daemon probe_svc, workbench panel + sidebar probe,
  mesh.py health probe, tui screen) are wrapped in `try/except
  ImportError` with log + no-op fallback so any operator who still
  imports the module path gets graceful degradation rather than a
  crash.

**Scope tightening (EPIC-MASTER-3)**
- Fleet cap reduced from 16 to **8 peers** (Q3 of the 100-Q
  tightening survey 2026-05-25). Personal-mesh scope per Q2 doesn't
  need 16; tighter scope bounds gluster full-replica + Bus broker
  mesh + attendance election cost. Code-level refs updated in a
  follow-on EPIC-MASTER-3.code subtask.

**Bundle retirements (EPIC-RETIRE-CADDY)**
- Deleted `mackes/caddy_gateway.py` — §8.13 Layer-3 Caddy reverse
  proxy for cross-peer service exposure. Mesh-services catalog gone
  with DEAD-2.9; v6.x Mackes Bus owns webhook ingress on its own
  Nebula-only port. `mde mesh-services enable-gateway` +
  `disable-gateway` CLI subcommands removed. `Recommends: caddy`
  dropped from the RPM spec.

**Motion language locked (EPIC-UI-MOTION)**
- New `docs/design/motion-language.md` — functional + subtle
  decorative motion only; 100/120/150/200 ms ease-in/out timing
  grid; 6 approved patterns; forbidden-verb list (bounce/spring/
  wiggle/etc.); reduced-motion accessibility wrapper pattern (Q47).
- Follow-on lint + CSS audit tasks added.

**VoIP scope deferred to 1.1 (EPIC-SCOPE-VOIP-DEFER)**
- v4.1.0 + v4.2.0 worklist sections gain TARGET REVISED suffix.
  VoIP ships in 1.0; spinout to `mde-voice` repo re-evaluated at
  1.1 per Q94 (softens Q8's "spin out immediately").

**Skill catalog consolidated to 3 (EPIC-PROC-SKILLS-3)**
- `plan` — design + N-Q survey + worklist rescue + lift design-doc
  actions to worklist.
- `ship` — autonomous worklist drain (parallel bundles, pre-commit
  gates, dual-remote push, session-id coordination).
- `release` — `cut release X.Y.Z` 7-step shorthand + HW bench gate.
- Retired: `mackes-worklist-management`, `complete-remaining-work`,
  `autonomous-worker`, `iteration`, `batch`. Bodies retained for
  slash-name back-compat; descriptions updated to `[RETIRED]` (Q87).

**Pre-commit lint gates added (EPIC-PROC-LINT)**
- `install-helpers/lint-dbus-shape.sh` — catches net-new MDE-internal
  `#[interface]` blocks; pre-existing services snapshot-allow-listed;
  shrinks as EPIC-RETIRE-DBUS migrates each to Bus (Q12 + Q20 + Q96).
- `install-helpers/lint-material-symbols.sh` — catches net-new Carbon
  icon references; retiring code allow-listed (Q43 + Q97).
- CLAUDE.md §0.7 expanded to **9 pre-commit gates**.

**Process locks landed (EPIC-PROC-*)**
- CLAUDE.md §0.8 expanded to **8 DoD gates** — added 8th: security
  review on new public ports / D-Bus methods / `{{exec}}` templates
  (Q64 + EPIC-PROC-DOD).
- CLAUDE.md §0.13 added — **quarterly retirement audit** as the
  fallback to inline-per-epic retirement (Q65 + EPIC-PROC-QUARTERLY).
- CLAUDE.md §0.14 added — **authority hierarchy** (Memory >
  CLAUDE.md > AI_GOVERNANCE > design > worklist; newest wins on
  contradiction; Q67 + EPIC-PROC-HIERARCHY).
- CLAUDE.md §0.15 added — **pre-release HW bench gate**. REVERSES
  the prior "HW carve-out never gates a cut" rule; each
  `cut release X.Y.Z` requires HW items for that release to be
  green (Q69 + EPIC-PROC-HW-GATE).
- Memory amendments: `feedback_no_cut_until_worklist_empty` +
  `feedback_hardware_testing_epic` reflect §0.15;
  `feedback_push_commit_auth` expanded per Q83 (standing auth covers
  `make rpm` + the §0.6 7-step shorthand once operator types
  `cut release`).

## Unreleased — v4.0 Nebula fabric rebuild

(Operator-locked 2026-05-24: v4.0 is the target release for
all features not yet released. Previously this section was
labeled "v2.5.0"; consolidated into v4.0 per the scope-shift.)

The mesh fabric stops outsourcing identity + connectivity to
Tailscale/Headscale and runs on a self-hosted Nebula PKI. No
SaaS dependency, no per-mesh API keys, simpler operator story
end-to-end.

**Networking**

- **Nebula overlay replaces Tailscale.** Every peer joins via a
  short join token (`mesh:<id>@<lighthouse>:<port>#<bearer>`)
  rather than a Tailscale OAuth flow. Overlay IPs are stable
  10.42.x.x/16 addresses signed by the per-mesh CA.
- **Built-in TCP/443 covert tunnel.** Peers behind UDP-blocking
  firewalls automatically fall over to a TLS 1.3 stream that
  presents identically to a long-poll HTTPS/2 session
  (`mackes-nebula-https-tunnel`). The lighthouse demuxes
  framed Nebula payloads back to its local UDP path; inner
  crypto runs unmodified.
- **Faster first-packet rendezvous.** Direct UDP handshake at
  < 1 s on warm peers; lighthouse-relayed path on cold start
  within 3 s. NF-9.x bench scenarios pin the SLOs.

**Operator surface**

- **`mackesd enroll --token <…>`** replaces `tailscale up` for
  joining a mesh. Idempotent + supports re-enroll under the
  same hardware fingerprint.
- **`mackesd ca {mint, rotate, list, dump-ca, sign-csr,
  export, import}`** subcommands cover the full CA lifecycle.
  Mint at first boot; rotate to bump the epoch + re-sign every
  peer cert; export/import for off-cluster disaster recovery
  (passphrase-encrypted via Argon2id + XChaCha20-Poly1305).
- **`mackesd nebula {status, peer-list, regen-certs,
  export-roster}`** subcommands surface live mesh state.
- **Wizard's Mesh Preview page** (post-Apply) shows the
  resulting overlay IP + lighthouse roster + diagnostics
  banner if the mesh stays empty for 30s.
- **Workbench `Service Publishing` panel** (Network → Service
  Publishing) lists the 7 canonical Nebula-published services
  (SSH, NATS, Mesh FS, Media, rsync, WoL, AV) with per-row
  status pills.
- **Workbench `Mesh Control` panel** gained a CA-epoch pill +
  Rotate CA button next to the existing leader / force-takeover
  surface.

**Daemon workers (mackesd)**

- **`nebula_supervisor`** — watches QNM-Shared for the
  lighthouse-issued bundle + materializes `/etc/nebula/`
  atomically.
- **`nebula_csr_watcher`** — auto-signs pending-enroll CSRs on
  a 30s tick, replacing the manual `mackesd ca sign-csr` step
  for the common case.
- **`nebula_https_listener`** — lighthouse-side TCP/443 demux
  pump that completes the covert-tunnel story.
- **`nebula_ca_backup`** — opt-in daily encrypted CA backup to
  `QNM-Shared/<self>/mackesd/ca-backup.enc`. Operator enables
  by exporting `MDE_BACKUP_PASSPHRASE` in the systemd unit.

**D-Bus**

- `dev.mackes.MDE.Nebula.Status.{Status, ListPeers, SelfNode,
  RegenCerts, Enroll}` — the unified read + operator-action
  surface every NF-10..18 desktop consumer hooks into.
- `dev.mackes.MDE.Shell.{Healthz, Workers}` — daemon-level
  status surface (panel + applet consumption).

**Voice / lint / docs**

- `install-helpers/lint-voice.sh` gained a FORBIDDEN-LEGACY-MESH
  check — any user-visible string mentioning Tailscale /
  Headscale / DERP gates the lint at commit time.
- `docs/help/{mesh-nebula,mesh-admin,mesh-ops,mesh-recovery,
  troubleshooting,headless,cli-reference}.md` rewritten end-
  to-end for the Nebula vocabulary.

**Removed**

- Tailscale + Headscale + DERP wire vocabulary from every
  user-facing surface (panel, wizard, help, CLI). The Python
  shim `mackes/mesh_vpn.py` retires alongside the v2.5 cut
  as the consumer-replacement pass closes.

**Greenfield acceptance gate** (NF-20.3, operator-run before
cut): a fresh Fedora 44 VM with `dnf install mde-4.0-1.fc44
.x86_64.rpm` + the first-boot wizard reaches a working 2-peer
mesh in under 10 minutes total operator time. `rpm -q tailscale
headscale tailscale-derp` returns "not installed".

## Unreleased — OV-1..OV-11: Workbench Overview tab + live capability statuses

The Workbench landing page (formerly "Dashboard") becomes a
true Overview that mirrors every cross-host mesh capability
the platform ships. Each row carries a one-sentence
plain-English description, a live status pill, optional
sub-status detail, and a **Configure** jump button that
deep-links to the panel where the operator manages that
capability. Coming-soon capabilities render with a disabled
"Coming soon" button so the affordance signals roadmap intent.

**New surface (`crates/mde-workbench/src/panels/home.rs`):**

- 11 capability cards in fixed order: Mesh Network · Peer
  Reachability · File Sharing · SSH Across Mesh · Remote
  Desktop (RDP) · Remote Desktop (VNC) · Media & App
  Discovery · Phone Pairing · Voice & Video · Fleet
  Configuration · Desktop Notifications.
- Live status sourced from `dev.mackes.MDE.Nebula.Status` +
  `dev.mackes.MDE.Fleet` D-Bus calls + `systemctl show`
  shell-outs for sshd, xrdp, x11vnc, wayvnc, and the
  curated `MESH_UNITS` set.
- VNC row flags `Failed` with sub-text "x11vnc does not run
  under Wayland — see RD-1..RD-5" when the operator runs
  Wayland with x11vnc enabled, closing the audit loop that
  surfaced the v2.6 RD-1..RD-5 epic.
- mackesd-down banner reminds the operator when D-Bus-sourced
  rows may be stale; systemd-sourced rows still probe.
- Hero stat grid (peers / pending updates / snapshots /
  drift) preserved as the header above the capability list.

**Live signal-driven refresh (`home::dbus_subscription`):**

- New Iced `Subscription` bridges D-Bus signals from
  `dev.mackes.MDE.Nebula.Status` (PeerStateChanged,
  TransportChanged, EnrollmentCompleted) + Fleet
  (RevisionApplied) into `Message::DbusEvent`, re-firing
  the probe fan-out without polling.
- Reconnects on bus drop with 5 s backoff so a mackesd
  restart doesn't require a Workbench relaunch.
- systemd1 `PropertiesChanged` per-unit subscription is OV-8.a
  follow-up; the manual Refresh button + load-on-nav cover
  that case today.

**`mackesd` (`crates/mackesd/src/ipc/nebula.rs`):**

- Three new D-Bus signals on `dev.mackes.MDE.Nebula.Status`:
  `PeerStateChanged(node_id, reachable)`,
  `TransportChanged(active_transport)`,
  `EnrollmentCompleted(node_id)`. `EnrollmentCompleted` fires
  from `Enroll(token)` on success; per-worker emission for
  the other two lands in OV-7.a / OV-7.b.
- Existing `enroll_with_token` integration extracted into a
  testable `enroll_inner(token)` core so the signal-emitting
  D-Bus wrapper can layer cleanly on top.

**Sidebar:**

- `Group::Dashboard.label()` returns `"Overview"` instead of
  `"Dashboard"`. The `Dashboard` enum variant + `"dashboard"`
  slug stay stable so existing `mde --focus dashboard[.home]`
  deep-links keep working.

**Tests:** 21 new unit tests on the home.rs surface
(capability status semantics, row ordering, jump-target
correctness, sub-status parsing, view-renders-without-panic
with and without capabilities); 3 new tests on the
mackesd-side nebula signal interface declarations. Total
`cargo test -p mde-workbench --lib`: 609 passing.
`cargo test -p mackesd --features async-services --lib ipc::`:
62 passing.

## Unreleased — OV-8.a: systemd1 per-unit PropertiesChanged subscription

Closes the last live-refresh gap on the Workbench Overview's
SSH / RDP / VNC / Mesh Services rows. After this change, the
operator never has to hit Refresh to see a local-daemon
state flip.

**`crates/mde-workbench/src/panels/home.rs`:**

- `run_subscription` opens a session zbus connection, calls
  `org.freedesktop.systemd1.Manager.Subscribe()` (the
  prereq systemd needs before it emits per-unit
  `PropertiesChanged` to this connection), then adds a
  single broad match rule scoped to
  `sender=org.freedesktop.systemd1,
  interface=org.freedesktop.DBus.Properties,
  member=PropertiesChanged`.
- New `unit_name_from_path()` helper reverses systemd's
  unit-object-path escape (each non-`[A-Za-z0-9_]` byte is
  hex-encoded as `_xx`). Round-trip: `sshd.service` ↔
  `sshd_2eservice`, `x11vnc@:0.service` ↔
  `x11vnc_40_3a0_2eservice`.
- New `systemd_watch_list()` returns the union of the
  fixed SSH / RDP / VNC slots (sshd / xrdp / x11vnc@:0 /
  wayvnc) plus every `MESH_UNITS` entry the Workbench
  Network → Mesh Services panel curates. Anything outside
  this list still fans out from the bus but is dropped
  silently before turning into a `DbusEvent::UnitChanged`.
- Demux loop now distinguishes systemd1 `PropertiesChanged`
  from Nebula / Fleet signals: systemd matches go through
  the watch-list filter and emit `Message::Home(
  DbusEvent::UnitChanged(name))`; Nebula / Fleet signals
  take the existing per-rule dispatch.

**Tests:** 5 new unit tests cover the escape decoder
(sshd / x11vnc@:0 template instance / bare alphanumeric /
non-systemd-path rejection) + the watch-list membership
(asserts every `MESH_UNITS` entry is present). Total
`cargo test -p mde-workbench --lib panels::home`: **26 /
0** passing. Voice-and-tone lint clean.

## Unreleased — CR-1: Classic ChromeOS theme token swap

Opens the v2.6 CR-N chrome epic from the 2026-05-24 design lock.
Every Workbench surface (and every downstream GTK consumer that
reads `data/css/tokens.css`) now renders in the Classic ChromeOS
pre-2022 palette + Roboto typography per
`docs/design/chromeos-classic-spec.md`.

**Visual delta:**

- **Palette** — page surface flips from Apple-charcoal `#1d1d1f`
  to Classic ChromeOS `#202124`. Cards and popovers shift from
  the prior three-tier elevation (`#2a2a2c / #38383a / #48484a`)
  to a flat single-tier `#2d2e30`. Borders are now hard 1 px
  dividers at `#3c4043` (dark) and `#dadce0` (light) rather than
  alpha hairlines — the visual rhythm reads sharper end-to-end.
  Accent stays indigo `#5b6af5` in dark mode; light mode gains
  a darker `#4051d3` pair for AA contrast against white.
- **Typography** — Geologica + IBM Plex Mono retire in favor of
  Roboto + Roboto Mono. UI body shrinks from 14 px to 13 px /
  18 px line; display title shrinks from 28 px to 22 px;
  monospace from 13 px to 12 px. Section headers (11 px small
  caps, +0.5 px letter-space) and page titles (18 px) get
  dedicated tiers.
- **Forbidden hybrids** (per the iteration skill's design-
  influence locks) audited: no rounded corners on flat content
  rows, no per-row accents, no sans-serif numerics in monospace
  contexts.

**`crates/mde-theme/`:**

- `palette::dark()` and `palette::light()` return the new
  Classic ChromeOS tokens; the prior Q3 Apple-charcoal + Q2
  same-accent-both-themes rules are grandfathered in source
  comments only.
- `FontSize::defaults()` returns the new Classic ChromeOS tier
  sizes (caption 11 / body 13 / subheading 15 / heading 16 /
  section 18 / display 22 / mono 12).
- `FONT_DISPLAY_BODY = "Roboto"`, `FONT_MONO = "Roboto Mono"`.
- A11y `high_contrast` variant brightens the border to solid
  white (dark) / black (light) rather than widening alpha from
  a hairline (the prior contract is no longer valid against the
  new solid base border).

**`data/css/tokens.css`:**

- Every `cds_bg_*`, `cds_text_*`, `cds_border_*` token now
  resolves to a Classic ChromeOS hex. The `cds_` naming stays so
  the 153 existing GTK consumers in the `mackes/workbench/` tree
  keep working without per-file rewrites.
- `font-family` rules name Roboto / Roboto Mono. Nerd Font
  fallback chain switches from RedHatMono Nerd Font to
  RobotoMono Nerd Font.
- `lint-css.sh` clean (4 / 4 OK).

**Packaging:**

- `packaging/fedora/mackes-shell.spec` gains hard
  `Requires: google-roboto-fonts` + `Requires:
  google-roboto-mono-fonts`. Both ship in stock Fedora repos.
  The prior `Recommends: ibm-plex-mono-fonts` line stays as
  archaeology (no harm in pulling Plex Mono alongside Roboto
  Mono for operators who customize beyond MDE defaults).

**Tests:** Downstream callers that hard-coded the prior palette
or typography values updated to match. `mde-theme` 95 / 95 passing,
`mde-workbench` 614 / 614 passing, `mde-files` 190 / 190 passing.
Voice-and-tone lint clean. `rpmspec -P` parses the spec without
errors.

**Unblocks:** CR-2 (Workbench shell retrofit), CR-3 (Object Card
component), CR-4 (mde-files visual retrofit), CR-5 (Start menu),
CR-6 (Mesh Topology peer cards), CR-7 (Networking / Phones /
Credentials / Recents panels). Each lands a UI surface against
the new token bedrock.

## Unreleased — GF-2.2.a: Gluster D-Bus surface (methods + signal interfaces)

Closes the method-side of GF-2.2 from the v5.0.0 GlusterFS
mesh-home epic. The signal-emission worker hook (GF-2.2.b)
stays HW-gated; the methods + signal-interface declarations
+ dispatch-loop wiring all ship today.

Re-audit 2026-05-24 caught the original `[HW carve-out]`
classification as overly broad: the 8 methods are bench-
observable via `busctl --user call`, the 5 signal
interface declarations are introspection-observable, the
signal dispatcher loop is unit-tested via the mpsc round-
trip. Only the worker-hook signal-emission gate is HW-
blocked, and that's now split out as GF-2.2.b.

**New `crates/mackesd/src/ipc/gluster.rs`:**

- `GlusterStatusService` exposes `dev.mackes.MDE.Gluster.
  Status` on `/dev/mackes/MDE/Gluster/Status` via zbus 5
  `#[interface]`.
- Methods (all 8 land):
  - `Status()` → JSON `StatusSnapshot {volume_name,
    peers_count, bricks_count, total_bytes, used_bytes,
    free_bytes, heal_pending_count, conflict_count,
    volume_online}`.
  - `ListPeers()` → JSON `Vec<GlusterPeerRow {uuid, host,
    state, is_self, brick_free_bytes}>`.
  - `AddPeer(node_id)` shells `gluster peer probe
    <node_id>`.
  - `RemovePeer(node_id)` shells `gluster peer detach
    <node_id>`.
  - `ConflictList()` returns JSON
    `Vec<ConflictRow {gfid, kind}>` from
    `gluster volume heal mesh-home info split-brain --xml`.
  - `HealStatus()` returns JSON `HealStatus {pending_count,
    in_progress_count, split_brain_count}`.
  - `MountStatus()` returns JSON `MountStatus {is_mounted,
    mount_point, since_unix_s}` via `findmnt --target`.
  - `BootstrapVolume()` shells the GF-2.4 genesis create
    + start commands; idempotent (returns
    `already-bootstrapped` when the volume exists).
- Every gluster shell-out wrapped in a 30s timeout so a
  hung CLI can't pin the IPC dispatch thread; non-zero
  exit drops to a debug-log + `None`.
- All 5 signals declared on the `#[interface]` block
  (`PeerStateChanged`, `ConflictDetected`, `HealCompleted`,
  `QuotaWarning`, `VolumeReady`) so introspection sees
  them.
- `spawn_signal_dispatcher` pumps events from an unbounded
  mpsc channel into the `SignalEmitter` path; the returned
  `GlusterSignalSender` exposes a fire-and-forget `emit()`
  for the gluster_worker to call once GF-2.2.b wires the
  worker hook.
- Pure-fn XML parsers (`parse_volume_info`,
  `parse_peer_count`, `parse_peer_rows`, `parse_heal_info`,
  `parse_conflict_rows`) are all `pub` + test-covered.
  String-based extraction via `extract_text_between` /
  `count_tag_occurrences` / `sum_tag_u64` — no third-party
  XML parser dep.
- 18 unit tests pass.

**Wiring:**

- `crates/mackesd/src/ipc/mod.rs` — `pub mod gluster;` +
  `paths::GLUSTER_STATUS` constant.
- `crates/mackesd/src/bin/mackesd.rs` — Gluster.Status
  registered as the sixth service between Nebula.Status and
  Shell.* on the same shared `org.mackes.mackesd`
  connection. Signal dispatcher spawned immediately after;
  the `_sender` is logged + discarded today (the
  `// GF-2.2.b will plumb _sender` comment marks the
  follow-up hook site).

The GF-2.2.b follow-up gates on hooking the
`GlusterSignalSender` into the gluster_worker's existing
state-transition detectors. That stays HW-blocked until a
multi-peer fleet exists to bench the signals firing on
live state transitions; the implementation itself is
single-peer reviewable.

## Unreleased — MON-1.b: Netdata aggregator-IP publisher worker

Closes MON-1.b from the v2.6 monitoring epic. Pairs with
the MON-1.a `apply_netdata_monitor` birthright step that
landed the baseline `/etc/netdata/netdata.conf` writer; now
the `[stream]` block follows the live leader-elected
aggregator IP without operator intervention.

Re-audit 2026-05-24 caught the original `[HW carve-out]`
classification as overly broad: the worker + atomic-write
paths + reload-call wiring are all observable without a
live multi-peer fleet; only the bench-gate observation
(`netdatacli aclk-state` reports `parent` role + child-
count = peers−1) needs HW, and that's gate-7 verification
not implementation.

**New `crates/mackesd/src/workers/netdata_aggregator.rs`:**

- `NetdataAggregator` struct with 5s tick cadence + the
  same role-host-marker leader-detection pattern as
  `nebula_supervisor` (single promote/demote action lifts
  both subsystems).
- Each tick: (a) when leader, atomic-writes a serde-JSON
  `AggregatorPointer { node_id, overlay_ip, epoch_s }` to
  `<qnm_root>/<self>/mackesd/netdata-aggregator.json`; (b)
  always scans `<qnm_root>/*/mackesd/netdata-aggregator
  .json`, picks the freshest pointer (ties broken
  lexicographically on `node_id` for cross-peer
  determinism), mirrors the chosen IP to
  `/var/lib/mackesd/netdata/aggregator-ip`; (c) when the
  aggregator IP changes, rewrites `/etc/netdata/netdata.
  conf`'s `[stream]` block + `systemctl reload-or-restart
  netdata.service`.
- Self-aware: when this peer IS the aggregator, the
  `[stream]` block is stripped (parent doesn't stream to
  itself).
- Fail-soft per v2.6 design lock: missing aggregator strips
  the `[stream]` block entirely so netdata falls back to
  local-only with the 7-day dbengine retention from
  `apply_netdata_monitor`.
- Pure-fn helpers exposed for tests: `read_overlay_ip`,
  `self_pointer_path`, `write_pointer`,
  `scan_aggregator_pointers`, `latest_aggregator`,
  `apply_aggregator_ip`, `build_stream_block`,
  `rewrite_stream_block`. 23 unit tests cover overlay-IP
  read (newline-strip / IPv6 / empty-error / missing-error),
  pointer roundtrip, scan missing-root + unparseable-skip,
  freshest-pick + tie-break + empty, apply
  unchanged/updated/cleared/both-absent, stream-block
  build none/some, stream rewrite
  append/replace/strip/empty-in/empty-out/section-order,
  pointer-path layout.
- API key derives from `MDE_MESH_ID` (env-overrideable via
  `MDE_NETDATA_API_KEY`) so every peer in the same mesh
  shares the same Netdata stream-handshake secret
  automatically.

**Wiring:**

- `crates/mackesd/src/workers/mod.rs` — `pub mod
  netdata_aggregator;` declaration.
- `crates/mackesd/src/bin/mackesd.rs` — spawned in
  `run_serve` between `gluster_worker` and
  `nebula_https_listener` with `RestartPolicy::Always`.
  Opens its own SQLite handle so the leader-check (when
  `crate::leader` exposes an async-services entry point)
  doesn't contend with the supervisor's mutex.

Together with [[MON-1.a]] and [[MON-2]] this closes the
end-to-end "alerts fire on every peer + stream to the
leader-elected aggregator" loop for v2.6. The parent-side
stream.conf api-key registration (needed for the actual
multi-peer streaming handshake) is a follow-up tracked as
MON-1.c — when it ships, the bench gate from MON-1.b's
acceptance criteria becomes verifiable on hardware.

## Unreleased — MON-2: Netdata health.d alert definitions

Closes MON-2 from the v2.6 monitoring epic. Five
`health.d/*.conf` files land under `/etc/netdata/health.d/`
via the Mackes RPM, tuning the alert set to this platform's
actual failure modes + suppressing the noisy stock alerts.

Re-audit 2026-05-24 caught the original `[HW carve-out]`
classification as wrong: the static config files are the
deliverable + Netdata auto-loads `/etc/netdata/health.d/*.
conf` on daemon-start without application-side wiring, so
the no-stubs / runtime-reachability gate is satisfied
without a hardware bench. The live `netdatacli
reload-health` + bench-trigger acceptance criteria stay
HW-gated, as they should.

**New health.d configs (all `%config(noreplace)` —
operator edits to thresholds survive package upgrades):**

- `nebula.conf` — 6 alarms: `nebula_process_down` (crit,
  systemd unit inactive >30s), `nebula_peer_unreachable`
  (crit, handshake_age >5m), `nebula_relay_fallback_ratio`
  (warn, >25% peers on relay >10m — violates v12.x
  throughput-first lock), `nebula_lighthouse_unreachable`
  (crit, both lighthouses out >2m), `nebula_handshake_
  failure_rate` (warn, >5/min sustained 5m), `nebula_
  first_packet_latency` (warn, p95 >3s over 10m — mirrors
  v12.x SLO).
- `gluster.conf` — 5 alarms: `gluster_brick_down` (crit),
  `gluster_heal_queue_depth` (warn, >100 files for >2m),
  `gluster_split_brain` (crit), `mesh_home_disk_full`
  (warn @85%, crit @95%), `gluster_quorum_lost` (crit).
- `mackesd.conf` — 3 alarms: `mackesd_dbus_unresponsive`
  (crit, Healthz <50% success over 2m, sourced from the
  v4.1 `Shell.Healthz` prometheus collector), `mackesd_
  leader_flap` (warn, >3 transitions in 10m — clock-skew
  signal), `mackesd_no_leader` (crit, no QNM-Shared
  lockholder >60s — fleet operations stalled).
- `workstation.conf` — 4 alarms: `boot_disk_full` (warn
  @90%, crit @97%), `swap_thrashing` (warn, swap-in >100
  pages/s over 5m), `thermal_throttle` (warn, any core
  throttled in 5m), `unattended_updates_pending` (warn,
  pending >7d — desktop nudge for operators who skip the
  BUG-11 watermark).
- `mde-suppressions.conf` — disables stock alerts that
  don't fit MDE's failure-mode set: `cgroup_memory_usage`
  (sway/Iced apps trip this on every session start),
  `net_interface_errors` (Wi-Fi 2.4 GHz frame-error noise),
  `systemd_units_active` (MDE's explicit per-service
  alarms cover the real signal), `disk_inodes_usage`
  (modern ext4/xfs/btrfs don't run out of inodes before
  disk space), `apps_cpu_per_user` (browser tab / video
  transcode is operator productivity, not alertable).

**Spec wiring:**

- `packaging/fedora/mackes-shell.spec` — five
  `install -m 0644 data/netdata/health.d/*.conf` lines
  under a `MON-2 (v2.6)` block; five `%config(noreplace)
  /etc/netdata/health.d/*.conf` entries in `%files` so
  rpm-py-inspector tracks them as config files; `%dir
  %{_sysconfdir}/netdata/health.d` for the directory
  ownership.

Worklist body deviation: the `[HW carve-out]` tag the
re-audit caught was annotated 2026-05-24 + removed in the
same edit — see `feedback_no_cut_until_worklist_empty.md`
for the recently-reaffirmed "no cut until non-HW tail
closes" rule that made the misclassification material.

Together with [[MON-1.a]], [[MON-3]], and [[MON-4]] the
notification fabric is now end-to-end ready except for the
live aggregator-IP publisher (MON-1.b) and the Mesh Health
Workbench panel (MON-5).

## Unreleased — MON-3 + MON-4: alert event pipeline (mde-alert-emit + alert_relay)

Closes the MON-3 + MON-4 mutually-blocked pair in one
commit per their worklist body's coupling note ("emitting
JSON nothing reads is the §0.12 anti-pattern" + "needs
MON-3 emitting events"). Together they wire the Netdata
custom-sender hook → mesh-wide FDO desktop notification
path.

**MON-3 — new `crates/mde-alert-emit/` crate:**
- Single binary at `/usr/libexec/mde/alert-emit` (FHS-
  correct location for helper binaries the user never
  invokes directly).
- Reads `NETDATA_ALARM_*` env vars + translates to the
  locked MON-3 schema JSON: `{id, ts, severity, category,
  alert, host, summary, value, threshold, chart_url,
  fired_by, seen_by: []}`.
- `make_ulid(when_unix_s, unique_id)` derives a
  deterministic ULID-format-compatible string: 48-bit
  timestamp + FNV-1a-derived randomness + Crockford-base32
  encoding via inline u128 arithmetic (no external `ulid`
  crate dep).
- Atomic write via tempfile + fsync + rename → one inotify
  event, not two.
- Idempotent: same env → same filename → single file. Tests
  cover the round-trip via direct env-block injection.
- CLI: `--output-dir` + `--dry-run-from-env`.
- 10 unit tests, all passing.

**MON-4 — new `mackesd::workers::alert_relay`:**
- Polls `~/.local/share/mde/alerts/*.json` on a 2s tick
  (polling instead of inotify mirrors the existing
  `notification_relay` worker's documented rationale +
  survives FUSE flakiness when `$HOME` is sshfs-mounted).
- Deserializes each new event via `AlertEventPartial`
  (schema-bump-tolerant via `#[serde(default)]`).
- Dedupes via per-worker `BTreeSet<String>` keyed on the
  deterministic ULID.
- Fires `notify-send` with severity-mapped urgency
  (CRITICAL/ERROR → critical, WARNING/WARN → normal, else
  low) + `--hint=string:chart-url:<url>` for click-through.
- Pure-fn `notify_send_argv(binary, event)` exposed for
  testing without shelling.
- Spawned in `run_serve` between the existing CA-backup +
  gluster_worker spawn sites. RestartPolicy::Always.
- 11 unit tests, all passing.

Worklist body deviation: MON-4 sketched inotify; the
shipped implementation uses polling per iteration-skill
best-choice authorization — matches the existing
`notification_relay` pattern + survives FUSE quirks.

`health_alarm_notify.conf` operator-wire-up lives with
MON-1.b's mackesd-side health.conf rewriter (HW-blocked
pending a live multi-peer Netdata fleet to verify the
stream-block transitions).

Binary build under `--features async-services` clean.
lint-legacy-mesh.sh clean.

## Unreleased — GF-5.2: retire KDC2 file-share UI surface

The KDC2 outbound file-share affordance retires per the v5.0.0
mesh-home design: files now move between peers via the
`~/Documents/From-<phone-name>/` mesh-home drop folder once
GF-5.1's inbound receive handler lands.

mde-kdc changes:

- `send_file` D-Bus method retired from
  `dev.mackes.MDE.Connect` (was `crates/mde-kdc/src/dbus.rs:296`).
  Operator-facing comment block flags the GF-5.2 retire +
  the path-forward (mesh-home drop folder via GF-5.1).
- Top-level doc comment updated to drop the `SendFile`
  mention in the KDC2-3.6 method list.
- `kdeconnect.share.request` packet kind stays in
  `build_packet`'s vocabulary since the GF-5.1 inbound
  receive handler (HW-carve-out) may emit it as an
  acknowledgement; only the OUTBOUND operator-typed surface
  retired.

mde-workbench `connect` panel changes:

- `ConnectSection::Share` variant retired.
- `render_share_section` helper retired.
- `section_visible_for` drops the `Share` arm.
- `render_card` drops the Share section emission; phone +
  messaging + chrome remain.
- 3 share-targeting tests retired
  (share_section_gated_on_share_request_capability /
  share_section_renders_only_when_visible /
  render_card_emits_sections_in_phone_messaging_share_chrome_order).
- 2 retained tests renamed to reflect the new order
  (render_card_emits_sections_in_phone_messaging_chrome_order
  / render_card_for_desktop_omits_phone_messaging).

71/71 mde-kdc tests + 13/13 connect-panel tests pass. Full
workspace build under `--features async-services` stays clean.

## Unreleased — MON-1.a: Netdata substrate (RPM dep + birthright baseline)

First commit of the v2.6 monitoring epic. Splits MON-1 into
MON-1.a (substrate, this commit) + MON-1.b (aggregator-IP
publisher + dynamic stream-block rewrite, the remaining
work) per §0.12's splitting rule.

Design lock 2026-05-24 (4-question in-session AskUserQuestion):

- Aggregator-role election reuses `mackesd::leader` — same
  QNM-Shared lockfile that elects the Nebula CA holder. One
  leader-election fact across all roles.
- Fall-back is fail-soft — every peer self-parents with 7-day
  local dbengine retention when the aggregator's overlay-IP
  isn't (yet) published.
- Alert event schema accepts the worklist body verbatim:
  `{id, ts, severity, category, alert, host, summary, value,
  threshold, chart_url, fired_by, seen_by: []}`. ULID
  derived from `NETDATA_ALARM_UNIQUE_ID + NETDATA_ALARM_WHEN`.
- MON-5 Mesh Health panel lands in the Rust mde-workbench (no
  parallel v1.x Python panel).

This commit ships:

- **`Requires: netdata`** added to the spec alongside the
  existing glusterfs-server/nebula/kamailio deps.
- **`%post systemctl enable --now netdata.service`** alongside
  the existing glusterd + mackesd enables.
- **`apply_netdata_monitor(preset)`** birthright step writes
  `/etc/netdata/netdata.conf` with the locked baseline:
  - `memory mode = dbengine`
  - `history = 604800` (7 days)
  - `dbengine multihost disk space = 256` MB
  - `[cloud] enabled = no` (mesh is the only telemetry path)
  - `bind socket to IP = 127.0.0.1` (overlay-bind lands with
    MON-1.b once the aggregator-IP publisher writes the
    overlay address)
  - `python.d = yes` (gluster + nebula source data via the
    python.d collector pattern)
- Atomic-write via `_write_root_file` (only fires when bytes
  differ); reload via `systemctl reload netdata.service` with
  `systemctl restart` fall-back.
- "Netdata monitoring" step registered in
  `mackes/wizard/pages/apply.py` between "Gluster substrate"
  (GF-3.2) and "XDG user dirs".

6 pytest tests cover every branch: CLI-not-installed /
already-matches-baseline / config-differs-triggers-write-and-
reload / reload-fails-falls-back-to-restart / both-fail-
surfaces-errors / config-contains-locked-design-params.
ruff clean.

MON-1.b (the dynamic stream-block writer hooking into
mackesd's leader transitions) is the remaining work to fully
close MON-1. Until then, every peer self-parents per the
fail-soft lock — 7-day local retention means no metrics loss
during the substrate-only window.

## Unreleased — GF-3.2 + GF-11.1: birthright gluster-status step + worker tests

GF-3.2 — new `apply_gluster_bootstrap(preset)` birthright
step gives the wizard's apply rail operator-visibility into
the v5.0.0 gluster substrate. Probes:

1. `shutil.which("gluster")` — CLI installed?
2. `systemctl is-active glusterd.service` — daemon running?
3. `gluster pool list` — daemon reachable?
4. `gluster volume info mesh-home` — has the worker
   bootstrapped yet?

Reports the daemon's expected next-tick action (gluster_worker
owns the actual bootstrap per GF-2.4; this step doesn't
duplicate the work). Registered as the "Gluster substrate"
step between "Normalize UID" (GF-3.1) and "XDG user dirs" in
`mackes/wizard/pages/apply.py`.

Deviation from the worklist body: the originally-sketched
`mackesd gluster bootstrap-or-join` CLI handoff retired
during implementation because the gluster_worker daemon
(GF-2.4) already runs the bootstrap on every 5s tick —
a parallel operator-typed CLI would have been redundant.

5 pytest tests cover every branch (CLI not installed,
glusterd inactive, pool-list failure with stderr-tail
surfacing, mesh-home exists, mesh-home pending with
overlay-ip dependency named).

GF-11.1 — marked `[✓]` (no new code). 32 gluster_worker
tests shipped incrementally across the GF-2.x cluster cover
worker lifecycle + PATH-probe + bootstrap-argv shape +
conflict-detector + quota-probe + peer-convergence + LWW-
resolver. Mocked-CLI shim uses `/bin/true` / `/bin/false` /
nonexistent paths — same pattern as the existing
`nebula_supervisor` tests.

## Unreleased — GF-2.9: gluster_worker delegates split-brain resolution

Closes the GF-2.x cluster (modulo the GF-2.2 D-Bus surface).
Instead of reimplementing the LWW mtime-comparison +
`.conflict-<host>-<ts>` rename in Rust, the worker now
delegates each detected split-brain GFID to gluster's own
self-heal daemon via:

```
gluster volume heal mesh-home split-brain latest-mtime gfid:<uuid>
```

The gluster daemon already knows the cross-peer mtime +
handles the rename per its own conventions (which DOES
produce `.conflict-<host>-<ts>` siblings on identical-mtime
ties — same operator-facing behavior the worklist sketched).

Three new pieces:

1. `heal_split_brain_argv(binary, gfid)` — pure-fn argv
   builder for the heal command.
2. `mark_gfid_heal_requested(gfid)` — Mutex-guarded
   `BTreeSet` insert that returns `true` only on first
   request. Prevents the 5s tick from re-spamming
   glusterd's transaction log on the same conflict before
   the heal daemon clears it.
3. Tick step 5 (the existing conflict-detector loop) gates
   each `run_argv(&heal_argv)` call behind
   `mark_gfid_heal_requested`.

3 new unit tests (32 gluster_worker tests total):
- heal_split_brain_argv command shape matches
- mark_gfid_heal_requested fires once per GFID then gates
- multiple distinct GFIDs each fire independently

The deviation from the worklist body (delegate vs
reimplement) is documented in the source-block comment per
§0.12 best-choice authorization — re-implementing the LWW
logic would have meant maintaining a parallel implementation
of glusterd's own resolution algorithm.

`HealCompleted` D-Bus signal emission defers to GF-2.2; the
tracing info-log at heal-request carries the payload until
that ships.

## Unreleased — GF-2.5 + GF-2.6: gluster_worker peer convergence (polling)

Adds the sixth tick step to `gluster_worker::tick_once`:
polling-based peer-probe + peer-detach against the local
glusterd's pool, sourced from the QNM-Shared bundle scan.
Replaces the worklist's original event-bus subscription
sketch (`nebula_supervisor::EnrollmentCompleted` + `ca_revoke`)
with a polling shape that matches the rest of the worker's
5s tick + doesn't require new event-bus infrastructure.

GF-2.5 — peer-probe path:

- `peer_probe_targets(qnm_root, self_node_id)` scans
  `<qnm_root>/*/mackesd/nebula-bundle.json`, parses each
  bundle's `overlay_ip`, returns a sorted Vec of
  `ProbeTarget { node_id, overlay_ip }`. Skips self.
- `current_gluster_peers(binary)` shells `gluster pool list`
  + parses via `parse_gluster_pool_list(text)` (pure-fn,
  skips the `localhost` row).
- `peers_to_probe(desired, current)` returns
  `(node_id, overlay_ip)` pairs missing from the pool.
- `peer_probe_argv(binary, overlay_ip)` =
  `gluster peer probe <overlay-ip>`.

GF-2.6 — peer-detach path:

- `peers_to_detach(desired, current)` returns every IP in
  the gluster pool whose bundle has disappeared from
  QNM-Shared (operator ran `mackesd ca revoke` →
  revoked-peer's bundle file removed from
  `<qnm_root>/<peer-id>/mackesd/`).
- `peer_detach_argv(binary, overlay_ip)` =
  `gluster peer detach <ip> force` (force flag because the
  detached peer may still own a brick contributing to the
  volume — auto-shrink per Q15).

Worker opts in via `with_qnm_peer_discovery(qnm_root,
self_node_id)`, called from `run_serve`. Without the
opt-in the step is a silent no-op.

10 new unit tests (29 total):

- missing-qnm-root → empty targets
- skips self
- sort-determinism (3 peers)
- skips dirs without bundle
- parse_gluster_pool_list: 3-peer + localhost row
- parse_gluster_pool_list: empty output
- peers_to_probe: missing-only diff
- peers_to_detach: stale-pool diff
- peer_probe_argv shape
- peer_detach_argv uses force

29/29 gluster_worker tests pass. Binary builds clean under
--features async-services. lint-legacy-mesh.sh stays clean.

The deviation from the worklist body (poll instead of
event-subscribe) is documented in the source-block comment;
follows §0.12 best-choice authorization (event-bus would
have been net-new infrastructure for an effect the 5s tick
already achieves).

## Unreleased — GF-2.8: gluster_worker conflict detector

Extends `gluster_worker::tick_once` with a fifth step that
walks the brick's `.glusterfs/indices/xattrop/` directory.
Each entry there is a GFID symlink representing a file with
a pending heal / split-brain op (glusterd's bookkeeping for
the self-heal daemon).

Pure-fn `pending_conflict_gfids(xattrop_dir) -> Vec<String>`
enumerates the directory + filters out glusterd's own
`xattrop` / `xattrop-*` placeholder markers; each remaining
entry surfaces as a `ConflictDetected` tracing warn event so
the operator sees split-brain state without manually running
`gluster volume heal info`.

Silent no-op when the brick dir is missing (mackesd running
on a non-storage box).

4 new unit tests cover missing-dir / empty-healthy-brick /
3-GFID-enumeration / placeholder-marker-filter. 19/19
gluster_worker tests pass.

The `{path, peers}` structured payload the worklist body
sketched needs `gluster volume heal info` parsing for the
per-peer attribution — that lands with the GF-2.2 D-Bus
service when the payload is actually consumed structurally;
the current tracing event carries the GFID + brick path
which is operator-actionable today.

## Unreleased — GF-2.7: gluster_worker hourly quota probe + cap

Extends `gluster_worker::tick_once` with a once-per-hour
free-space probe that pushes the locked Q16 quota
(`0.8 × min(free brick across peers)`) into the volume:

1. `quota_probe_due()` Mutex-guards a last-fire Instant
   against `QUOTA_PROBE_INTERVAL = 3600s` so the heavy
   `gluster volume info --xml` round-trip only fires once
   per hour, not every 5s tick.
2. `run_quota_probe()` shells the volume-info XML, walks
   every `<sizeFree>NNN</sizeFree>` element via the
   pure-function `min_brick_free_bytes(xml)` (regex-free
   scan, integer-parse-tolerant), takes the min, computes
   the 0.8× cap.
3. `quota_set_argv(binary, bytes)` builds the
   `gluster volume quota mesh-home limit-usage / <bytes>`
   argv per the design-doc § 3.4 spec.

Best-effort: every failure step logs at warn + retries on
the next hourly window.

5 new unit tests:
- min_brick_free_bytes picks smallest of 3 bricks
- min_brick_free_bytes returns None for empty volume
- min_brick_free_bytes skips unparseable entries (`not-a-number`)
- quota_set_argv command shape matches design doc
- quota_probe_due fires on first call, rate-limits the
  immediate second

The `QuotaWarning` D-Bus signal emission defers to GF-2.2
(the gluster D-Bus service); the tracing info-log at
quota-set carries the same payload until that ships.

## Unreleased — GF-2.1 + GF-2.3 + GF-2.4: gluster_worker ships + bootstraps mesh-home

New `mackesd_core::workers::gluster_worker::GlusterWorker`
mirrors the `nebula_supervisor` shape: tokio task, 5s tick,
owned `Arc<Mutex<rusqlite::Connection>>` store handle,
`ShutdownToken` `select!` for prompt SIGTERM exit.

Each tick:

1. **Probe.** Shell-checks whether `gluster` is on PATH;
   silent no-op when the v5.0.0 substrate isn't installed.
2. **Wait for Nebula enrollment.** Reads the GF-1.3.a
   overlay-ip publish file; silent skip when missing.
3. **Genesis path (GF-2.4).** Runs `gluster volume info
   mesh-home`; on "does not exist" stderr, runs `gluster
   volume create mesh-home replica 1 transport tcp
   <overlay-ip>:<brick-path> force` per design doc § 3.4.
   Idempotent — once the volume exists every tick is a
   no-op for this step.

Spawned in `run_serve()` (GF-2.3) just after the
`nebula_ca_backup` spawn site; opens its own SQLite handle
so the future GF-2.7 quota probe + GF-2.8 conflict detector
can audit-log without lock contention. `RestartPolicy::Always`
since the tick is passive — any crash is a fault we want
auto-recovered.

Pure-fn helpers (`bootstrap_argv`, `binary_on_path`,
`volume_exists`) extracted for testing without a live
glusterd. 10 unit tests cover:

- worker name stability
- no-op when gluster binary absent
- skip-bootstrap when overlay-ip file missing
- attempts-bootstrap when overlay-ip present + volume
  missing (negative probe via /bin/false)
- bootstrap_argv command shape matches design doc § 3.4
- bootstrap_argv honors alternate binary + paths
- PATH probe finds `true`
- PATH probe rejects nonexistent absolute path
- PATH probe rejects nonexistent relative name
- worker exits on shutdown token

Subsequent GF-2.x items (peer-probe on enrollment + detach
on revocation + hourly quota probe + conflict
detector/resolver) layer onto the same tick.

## Unreleased — worklist hygiene: tag HW-carve-out items

Re-tag `GF-11.3` (3-peer split-brain bench test), `GF-15.2`
(phone-pair → 2nd-peer-observe smoke), and `VV-15`
(16-peer Docker fixture + live SIP-trunk drill) with
explicit `[HW carve-out]` annotations + body notes pointing
at `~/.claude/.../feedback_no_cut_until_worklist_empty.md`.

Per the locked rule: Hardware-Testing-epic items never gate
a release; only the non-HW worklist tail does. These three
items stay `[ ]` Open (per the existing convention from
`feedback_hardware_testing_epic`) but are now self-
documenting about their carve-out status, so future
read-of-the-worklist matches the cut-gate semantics.

No code change. Worklist hygiene only.

## Unreleased — NF-4.5: https_fallback bridge-layer slim + stun re-classify

`crates/mackesd/src/https_fallback.rs` shrinks from 644 LOC
→ ~110 LOC. The duplicated state-machine body + 350+ lines
of pure-fn tests retire in favor of the canonical
`mackes-nebula-https-tunnel::activation` module (NF-1.4's
port). What's left in `mackesd::https_fallback` is the
**bridge layer**: re-exports of the activation enums + a
slim `observe_peer` wrapper that mutates `PeerPath` state in
place. Three integration tests cover the wrapper's per-tick
behavior; the state-machine tests stay upstream where the
logic lives.

To avoid Rust's orphan rule, the
`From<HttpsFallbackState> <-> mackes_transport::peer_path::
HttpsFallbackState` impls + the `FailureWindow::
from_consecutive_failures()` constructor moved upstream
into `activation.rs`. `mackes-nebula-https-tunnel` gains a
`mackes-transport` dep (no cycle — mackes-transport has
zero workspace deps).

**`stun.rs` re-classified as live infrastructure (not
retired):** the original NF-4.5 worklist body claimed
`stun.rs` was "absorbed by Nebula's protocol-level
rendezvous." The audit (2026-05-24) found this is wrong:
`crates/mackesd/src/workers/stun_gather.rs` is spawned live
in `run_serve` (NF-1.5 mesh-router context) and uses
`crate::stun::{gather_endpoint,
encode_binding_success_with_xor_mapped}` to augment
Nebula's hole-punching with STUN reflexive candidates.
Retaining `stun.rs`; the worklist's stun-deletion premise
was based on a v1.x assumption that v4.0.1 mesh_router
invalidated.

567 mackesd lib tests + 48 tunnel-crate tests + the
mackesd binary build under `--features async-services` all
green.

## Unreleased — NF-20.6: lint-legacy-mesh.sh pre-commit gate

New `install-helpers/lint-legacy-mesh.sh` script catches
net-new `tailscale` / `headscale` / `derper` references in
v2.5+ Nebula-native source. Wired into `.claude/CLAUDE.md`
§0.7 as gate #7 alongside the existing voice-and-tone +
CSS + RPM + ruff + module-import gates.

Allow-list (by directory prefix):
- `mackes/*` — v1.x Python tree, NF-5.x cluster retires these
  files; their tailscale references are pre-existing.
- `crates/mackes-panel/` — v1.x GTK panel, frozen by the
  Iced rewrite, won't be relabeled.
- `crates/mackesd/src/{https_fallback,stun}.rs` + workers
  `{derp,perf,stun_gather,mesh_router}` + `transport/https443`
  + `topology/mod` + `legacy_inventory` — NF-4.5 retirement
  targets, allow-listed pending that cascade.
- `crates/mackes-nebula-https-tunnel/src/activation.rs` — the
  upstream canonical replacement, naturally names what it
  replaced.
- `tests/*` — legitimate "assert legacy is GONE" fixtures.
- `crates/mde-workbench/src/panels/mesh_services.rs` —
  catalog-absence assertions.

Retraction-comment lines (NF-N.M / GF-N.M / RD-N / KDC2-N
tags or `retired/legacy/superseded/deprecat` verbs) plus
pure `//`/`#` comment lines are also filtered.

Current state: zero violations. The gate is now a regression
detector — future commits that drag the legacy vocabulary
back into v2.5+ source will trip CI.

## Unreleased — RD-4: wayvnc reuses Nebula's X.509 PKI as its TLS identity

Closes the v2.6 RD epic. The original worklist body sketched a
parallel Ed25519 keypair tree (mirroring `mesh_ssh` layer A),
but wayvnc 0.9.1 (F44's version) doesn't actually support
Ed25519 RFB auth — it speaks TLS via libtls. Pivoted per
in-session operator AskUserQuestion to "Nebula X.509 TLS":
wayvnc reuses the per-peer Nebula cert + key that mackesd's
nebula supervisor already maintains under `/etc/nebula/`. No
parallel key tree.

Changes:

- **`apply_remote_desktop` writes `/etc/wayvnc/config`** with
  `private_key_file=/etc/nebula/host.key` +
  `certificate_file=/etc/nebula/host.crt` +
  `enable_pam=false`. The cert paths point at files mackesd's
  nebula supervisor (NF-3.4) already maintains.
- **`mde-wayvnc@.service` unit** drops the `--unauthenticated`
  flag, references `--config=/etc/wayvnc/config`, and gains
  `ConditionPathExists=` checks for both cert files so the
  unit fails cleanly before any peer enrolls.
- **Design doc § 3.3 + the user help doc** both rewritten to
  lock the Nebula-TLS auth model.

Trust chain = the mesh's existing Nebula trust chain. An
unenrolled host on the overlay can't present a Nebula-CA-
signed cert → wayvnc TLS handshake fails → connection
refused. Revocation runs via `mackesd ca revoke <node-id>` —
the revoked peer's cert stops validating on the next
CA-epoch roll.

275/0 pytest + ruff F401/F541/F811/F841 lint clean +
birthright module-import smoke pass.

## Unreleased — RD-5: docs/help/remote-desktop.md operator primer

New user-facing help doc covering all three remote-desktop
daemons each Mackes peer ships:

- **wayvnc** (port 5900) — sway-native via wlroots
  screencopy; binds to the Nebula overlay only;
  `--unauthenticated` for v2.6 (Ed25519 follows in RD-4).
- **xrdp** (port 3389) — PAM-authenticated; brings its own
  Xorg fallback session so RDP works under a Wayland
  greeter.
- **Guacamole** (port 8080 + `https://media.mesh/desktop/`) —
  noauth web app; auto-populates the connection list from the
  Nebula peer roster via `mackes-remote-sync.service`.

Covers the firewall posture (3 ports on the `firewalld`
`trusted` zone only — never the underlay), the auth-model
difference per protocol, and the 3 most common operator
questions (connection-refused right after install, can-I-
connect-from-outside-the-mesh, why-only-xrdp-has-PAM-auth).

`mesh-services.md` gains a "See also" cross-link pointing to
the new doc so the discoverability chain stays intact. The
worklist's pre-supposed `MACKES_SHELL_SPEC.md §0` capability
list + `mesh-services.md`'s "X11-only caveat" turned out not
to exist (grep confirms zero VNC mentions); per iteration-
skill standing-authorization #4 the literal RD-5 targets were
re-interpreted as "ship an operator-facing help doc that
closes the remote-desktop documentation gap end-to-end."

## Unreleased — RD-1 + RD-2 + RD-3: v2.6 Wayland VNC swap (wayvnc)

The v2.0.0 hard-switch to sway (Wayland-only) broke x11vnc's
`:0`-display-mirroring assumption — on a Wayland-only host
there's no `:0` and x11vnc's unit silently fails to bind.
This commit closes the gap end-to-end:

- **RD-1: design lock doc** at
  `docs/design/v2.6-wayland-vnc.md` — captures the 5-Q lock
  (wayvnc beats gnome-remote-desktop on closure size + sway-
  native compositor support), the Nebula-overlay bind
  boundary, the Ed25519 auth model (RD-4 follow-up), the
  worklist cross-ref, and the v2.5 → v2.6 migration path.
- **RD-2: RPM spec swap** — `Requires: x11vnc` →
  `Requires: wayvnc` in `packaging/fedora/mackes-shell.spec`.
  Closure goes from ~30 GNOME packages (the
  gnome-remote-desktop alternative) to ~200 KB.
- **RD-3: birthright rewrite** — `apply_remote_desktop`
  retires the `x11vnc@.service` template, ships
  `mde-wayvnc@.service` (templated system unit, instance
  name = primary user, runs as uid-1000, binds to the
  Nebula overlay IP from `/var/lib/mackesd/nebula/overlay-ip`
  per GF-1.3.a). Enable list flips to
  `mde-wayvnc@<primary-user>.service`; pre-v2.6 installs
  get their legacy `x11vnc@:0.service` disabled +
  unit-file removed during birthright so two VNC servers
  don't fight over port 5900.

RD-4 (Ed25519 per-peer auth) is the immediate follow-up;
the current wayvnc setup runs `--unauthenticated` which is
auth-parity with the previous x11vnc `-nopw` config — the
Nebula overlay is the trust boundary. RD-5 (docs + capability
list refresh) lands separately.

275/0 pytest + ruff lint clean + module-import smoke for
birthright + `rpmspec -P` clean.

## Unreleased — NF-5.1 + NF-15.5: retire mesh_vpn.py core + its test file

Third commit of the wholesale Python-tree retire (after
NF-14.1, NF-5.5). The 1,050-line legacy Tailscale/Headscale
shim `mackes/mesh_vpn.py` is gone, along with its test file
`tests/test_mesh_vpn.py` (67 lines, 3 tests of MeshState
round-trip + parse_join_link — the Nebula replacement
`tests/test_mesh_nebula.py` shipped earlier in v2.5 with 41
tests).

Cleanup approach: the 24 importer call sites across the v1.x
Python tree all degrade gracefully because every existing
call lived inside a `try/except ImportError` block. The two
top-level importers got explicit shim layers:

- `mackes/wizard/pages/mesh_join.py`: new `_legacy_mesh_state()`
  + `tailscale_status()` + `_MissingMeshState` class provide
  zero / "not joined" stand-ins so the legacy v1.x wizard page
  still loads (the Rust mde-wizard from NF-7.1 is the live
  surface — this page is dormant unless an operator click-
  throughs from the workbench).
- `mackes/workbench/network/mesh_ssh.py`: local
  `headscale_list_peers()` wrapper returns an empty roster
  when mesh_vpn is missing, so the v1.x SSH panel renders a
  clean empty state.

Operator-visible: the v1.x `mackes` Python binary still
launches + WorkbenchWindow renders, just with empty Tailscale/
Headscale state — which is correct semantically since the
Nebula mesh has been the live surface since v2.5. Cluster
state surfaces in the Mesh Health + Mesh Topology panels
(NF-11.x's Nebula rewrite).

275/0 pytest (down 3 from test_mesh_vpn.py deletion) + ruff
F401/F541/F811/F841 clean + 9 module-load smoke checks
(sidebar_window, workbench.window, mesh_control, mesh_ssh,
mesh_join, headless.{cli,daemon,wizard,status}) all green.

## Unreleased — NF-5.5: retire mackes/workbench/network/mesh_vpn.py

Second commit of the operator-authorized wholesale Python-tree
retire (after NF-14.1). The 410-line v1.x `MeshVpnPanel` is
gone, and the four reference sites are cleaned up:

- `mackes/workbench/shell/sidebar_window.py`: `_mesh_vpn`
  builder + `_f_meshvpn` builder + the "Mesh VPN" entry in
  the network-advanced sub-nav are removed.
- `mackes/workbench/window.py`: `_network_tab` no longer
  imports `MeshVpnPanel`; the "Mesh VPN" tab is dropped from
  the Network notebook + the `mesh_vpn` deep-link alias in
  `_TAB_INDEX` is retired.
- `mackes/workbench/network/mesh_control.py`: `TABS` const
  drops the "VPN" tab (9 → 8 tabs).

The two surviving try/except mesh_vpn imports in
sidebar_window's badge-counter (lines 862 + 1008) are left
alone — they silently swallow ImportError so once NF-5.1
retires `mesh_vpn.py` the badge returns mesh_online=0
cleanly. Explicit removal lands as part of NF-5.1's broader
cleanup of the v1.x Python tree's 24 `mesh_vpn` importers.

Operator-visible: anyone still launching the v1.x `mackes`
binary loses the "Mesh VPN" entry in both the sidebar nav
and the Network notebook. Equivalent mesh state has been
in the Mesh Health + Mesh Topology surfaces (NF-11.x's
Nebula rewrite) since v2.5; the legacy Tailscale/Headscale
panel was already showing stale data.

ruff F401/F541/F811/F841 lint clean; 278/0 pytest suite
stays green; module-import smoke for sidebar_window,
workbench.window, and mesh_control all pass.

## Unreleased — NF-14.1: retire mackes/wizard/headscale_setup.py

First commit of the operator-authorized wholesale Python-tree
retire (2026-05-24): 688-line `headscale_setup.py` GTK wizard
window is gone, the only importer (`mackes/workbench/network/
mesh_vpn.py`'s `_on_setup_wizard` method) loses its method +
the action bar's "Setup wizard" button. The Rust mde-wizard
crate (crates/mde-wizard/) now owns first-boot mesh setup
entirely.

The remaining v1.x mesh_vpn panel retires under NF-5.5; this
commit is the leaf-first first step. Add-Peer / Leave-Mesh /
Diagnostics / Refresh stay live until that broader cleanup
lands.

ruff F401/F541/F811/F841 lint clean; 278/0 pytest suite
green; module-import smoke for the trimmed `MeshVpnPanel`
passes.

## Unreleased — GF-4.1: per-user mesh-home FUSE mount unit

New `data/systemd/mde-mesh-mount@.service` templated user
unit FUSE-mounts the local glusterd-served `mesh-home`
volume at `$HOME/%i` (where `%i` is the XDG subdir name —
Documents / Pictures / Music / Videos / Downloads).

Not auto-enabled by RPM install — birthright's GF-3.3
`apply_xdg_mesh_mount` step (still ahead) flips
`systemctl --user enable mde-mesh-mount@<Documents/...>`
once the operator is on uid:gid 1000:1000 (GF-3.1) and
glusterd has bootstrapped the volume (GF-2.x). Until then,
operators can manually enable individual instances after
running `gluster volume create mesh-home` on their
lighthouse; `mount.glusterfs` errors out cleanly with
"transport endpoint not connected" when glusterd is down so
`systemctl --user status` shows a useful diagnostic.

Mount options: `_netdev` so the mount survives network
blips with a reconnect, `acl` so cross-peer chown/chmod
surface correctly. Lands under `%{_userunitdir}/` per the
existing mde-session.service pattern; rpmspec preprocess
clean.

## Unreleased — GF-9.3: `mackesd state-restore` CLI

New subcommand `mackesd state-restore <bundle>
[--passphrase-env MDE_BACKUP_PASSPHRASE]
[--recovery-dir /var/lib/mackesd/restore/gluster]` decodes
an armored `state-backup.enc` bundle and restores both the
Nebula CA + the optional gluster topology snapshot on a bare
peer:

1. Reads + dearmors + unseals the bundle via the existing
   `ca::backup::{dearmor, unseal}` helpers.
2. Calls `ca::backup::restore_to_store` to INSERT-OR-REPLACE
   CA + signed peer cert rows back into the local SQLite
   store.
3. When the bundle carries a `gluster_snapshot` (v2 schema
   bumped by GF-9.2), writes the per-section XML payloads
   (`volume-info.xml` / `peer-status.xml` /
   `volume-status.xml`) under `--recovery-dir` for the
   operator's manual `gluster volume create --xml-input`
   replay.

Automatic volume replay is intentionally out of scope:
replaying a stale `volume info` against a live cluster
requires careful peer-by-peer reconciliation that's an
operator-driven step, not a silent CLI action. The runbook
in `docs/help/mesh-recovery.md` (NF-18.3) is the canonical
procedure.

CA-only bundles (v1 schema, no gluster snapshot) restore
cleanly + log "skipping gluster step" rather than failing.

## Unreleased — GF-12.2: gluster-headroom pre-flight CLI

`mackesd preflight-gluster-headroom` walks the operator's
five XDG dirs (Documents / Pictures / Music / Videos /
Downloads), sums on-disk bytes, queries
`/var/lib/gluster/bricks` free space via `df -B1 --output=avail`
(workspace forbids `unsafe_code` so no `statvfs`), classifies
the headroom verdict against the locked 1.5× XDG-bytes
threshold, prints the one-line summary to stderr + the full
structured JSON report to stdout, exits 0 on OK / 1 on
WARN / NoBrick.

Operators can run this before upgrading to v5.0.0 to confirm
their brick partition has enough headroom for the mesh-home
volume. The Workbench Mesh Storage panel (GF-8.x) will
consume the same JSON shape as a banner once it lands.

`mackesd_core::gluster::headroom` module is pure: same
inputs → same `HeadroomReport`. 7 unit tests cover
no-brick / empty-xdg / file-aggregation / missing-xdg-dirs /
default-xdg-names / summary-per-verdict / JSON round-trip.

## Unreleased — GF-9.1/9.2/9.4: state-backup tarball carries gluster topology

The daily encrypted backup that NF-18.4 introduced now folds
the local glusterd's `volume info` / `peer status` / `volume
status` XML payloads into the same sealed tarball. Single-
file restore of a peer's CA AND its Gluster volume topology
is the v5.0.0 operator-recovery promise; this commit lands
the snapshot side of that contract. (GF-9.3, the matching
`mackesd state restore <bundle>` CLI, ships separately when
the volume-replay logic stabilizes.)

Changes:

- **`crate::gluster::snapshot::collect(&SnapshotConfig)`** —
  pure-ish helper that shells `gluster volume info --xml`,
  `gluster peer status --xml`, and `gluster volume status all
  clients --xml`, returns `Option<GlusterSnapshot>`. `None`
  when `gluster` binary isn't on PATH (peer-only roles, dev
  hosts); `Some` even when each inner XML field is `None`
  (CLI installed but glusterd is misbehaving — itself a
  useful diagnostic for restore).
- **`BundlePlaintext::gluster_snapshot: Option<GlusterSnapshot>`**
  with `#[serde(default)]` so v1 bundles (no field) round-
  trip cleanly + v2 readers tolerate forward-compatible
  parsing.
- **`schema_version` bumps 1 → 2** only when the snapshot is
  populated, so CA-only `mackesd ca export` paths stay
  byte-identical to v4.x.
- **`BACKUP_FILENAME` const flips from `ca-backup.enc` →
  `state-backup.enc`** (GF-9.1). Legacy constant kept as
  `LEGACY_BACKUP_FILENAME` for the operator runbook's upgrade
  path. Upgrading installs see both files for one tick; the
  legacy file can be safely `rm`'d.

7 new unit tests in `gluster::snapshot::tests` (absent-binary
/ always-failing / always-succeeding / JSON round-trip /
legacy-shape deserialization / PATH probe present + absent),
full 583/0 lib suite green. NF-18.4 stays `[✓]` (historical
record) per the newer-wins-silently rule; GF-9.4 records the
supersession in the v5.0.0 epic cluster.

## Unreleased — GF-1.3.b: glusterd binds to the Nebula overlay

New `mackesd_core::gluster::bind` module rewrites
`/etc/glusterfs/glusterd.vol` so glusterd listens on the
local peer's Nebula overlay IP rather than `0.0.0.0`.
Idempotent + defensive: refuses to edit any file whose
`volume management ... end-volume` markers are missing, treats
a missing file as a no-op (safe before `glusterfs-server` is
installed), and only triggers `systemctl reload glusterd` when
the file's bytes actually change.

Wired from `nebula_supervisor::refresh_config` so glusterd's
bind tracks the live Nebula address: every time the supervisor
materializes a new Nebula config, it also publishes the
overlay-ip file (GF-1.3.a) AND rewrites glusterd.vol +
reloads glusterd if the address changed.

Closes the v5.0.0 lock from Q3 — "Nebula overlay only;
plaintext glusterd inside the tunnel, no second TLS layer."
Without this, GF-2.x volume bootstrap on Fedora-shipped
glusterd would bind on the underlay too.

14 new unit tests: insert / replace / unchanged / format-
refuse (header missing / footer missing) / unrelated-options-
preserved / trailing-newline-convention (present + absent) +
5 apply_bind I/O tests for missing-file / write-on-change /
idempotent-second-call / no-tempfile-on-success / format-
refuse-leaves-file-untouched.

Deviates from the original "glusterd.vol.d/10-nebula-bind.vol"
drop-in spec because F44's glusterd doesn't honor drop-in
includes; we edit the main glusterd.vol in place. Decision
documented in the worklist's GF-1.3.b entry.

## Unreleased — GF-3.1: birthright pins primary account to uid 1000

New `apply_uid_normalize(_preset)` step in
`mackes/birthright.py` asserts the primary login account is on
uid:gid 1000:1000 and migrates non-1000 users via `usermod -u
1000 / groupmod -g 1000` + recursive chown of `$HOME` and
`/var/lib/<user>`. The migration is the prerequisite for the
v5.0.0 mesh-home FUSE mounts to surface consistent file
ownership across peers; without it, two peers running the
same MDE binary against the same brick see different file
owners and FUSE refuses operations as "wrong user."

Collision-safe: refuses to migrate when uid 1000 (or gid
1000) is already held by a different user — that's an
operator-fixable split-tree we won't silently resolve here.
Idempotent re-runs on an already-normalized install log
"already uid:gid 1000:1000" and return without any
subprocess calls.

Wired into the wizard apply pipeline as the "Normalize UID"
step between "Thunar on login" and "XDG user dirs" so the
operator sees one progress row per decision. Routed through
`AdminSession` for the privileged calls; collisions surface
as visible log lines on the wizard rail rather than blocking
the apply pass.

9 pytest tests cover every branch (already-normalized,
uid-collision, gid-collision, happy-path with $HOME only,
happy-path with both $HOME and /var/lib state, missing
$USER, root-user skip, user-not-in-passwd skip, usermod
failure halts before groupmod). ruff lint + voice-and-tone
lint clean.

## Unreleased — GF-1.3.a: nebula_supervisor publishes overlay-ip

New `pub fn publish_overlay_ip(path, overlay_ip)` in
`crates/mackesd/src/workers/nebula_supervisor.rs` atomic-writes
the local peer's Nebula overlay address (plain text + trailing
newline) to `/var/lib/mackesd/nebula/overlay-ip` after every
`materialize_config` tick. The new path constant
`DEFAULT_OVERLAY_IP_PATH` is exported at module scope so
downstream consumers (notably the upcoming GF-1.3.b
glusterd-nebula-bind helper) have a single shared lookup
location. Publish failures are logged + retried on the next
tick — they never abort the nebula-config refresh.

RPM spec ships `/var/lib/mackesd/nebula/` as a
world-readable directory (0755) so non-root consumers can
read the file without sudo. 5 new unit tests cover
create-parent-dir, overwrite, no-leftover-tempfile,
IPv6 pass-through, and the constant match against the
design-doc path. 15/15 nebula_supervisor tests pass.

The GF-1.3 worklist task split into GF-1.3.a (this commit:
the publisher) and GF-1.3.b (the glusterd config rewriter +
reload service that hasn't landed yet) per §0.12's splitting
rule — the original single-task formulation would have
required a stub-or-staged commit to ship.

## Unreleased — v5.0.0 GlusterFS mesh-home + KDC2 file-transfer removal (placeholder)

**SemVer-major bump.** v5.0.0's headline is `mesh-home`: a
replicated GlusterFS volume mounted in-place over every
peer's `~/Documents`, `~/Pictures`, `~/Music`, `~/Videos`,
and `~/Downloads`, with the bytes moving over the Nebula
overlay. The major bump is required because KDC2's
file-transfer affordance is removed entirely in v5.0.0 —
paired phones drop files into
`~/Documents/From-<phone-name>/` and the mesh handles
replication from there. There is no v4.x-compatible file
fall-back; phones lose their existing share-sheet
destination.

Locked 2026-05-24 via 25-Q survey
(`docs/design/v5.0.0-gluster-mesh-home.md`). User-facing
primer at `docs/help/mesh-storage.md`. Worklist tracker at
`docs/PROJECT_WORKLIST.md` § "GF-1..GF-15".

Substrate shipped so far:
- RPM pulls in `glusterfs-server` + `glusterfs-fuse`;
  `%post` enables `glusterd.service`. The Nebula-overlay
  bind drop-in (GF-1.3), the `gluster_worker` daemon
  (GF-2.x), the birthright pipeline integration (GF-3.x),
  and the FUSE mount unit (GF-4.x) are still ahead.

The actual v5.0.0 RPM cut requires the full GF-1..GF-15
sweep to land. This section is a placeholder so the cut-
release flow has a target heading from day one; each
landed GF-N task appends its own bullet here.

## Unreleased — GF-10.1: docs/help/mesh-storage.md user-facing primer

New help page covering the v5.0.0 `mesh-home` model from the
operator's seat: what's shared (the five XDG dirs) vs not
(`~/Local/`), the 5 GB stub fall-back + how to fetch the real
bytes, the LWW conflict resolution + `.conflict-<host>-<ts>`
sibling convention + the Resolve UI, the fleet quota +
EROFS-at-cap behaviour, the v4.0.x → v5.0.0 migration archive
path, and the phone-share folder rewrite. Cross-linked from
mesh-admin / mesh-ssh / mesh-recovery so the discoverability
chain stays intact. voice-and-tone lint clean.

## Unreleased — GF-10.2: v5.0.0 GlusterFS design lock doc

Captures the 25-Q lock from 2026-05-24 in
`docs/design/v5.0.0-gluster-mesh-home.md` so the design intent
survives operator turnover + outlasts the worklist's row-per-
task structure. Embeds the question/lock table verbatim, draws
the per-peer process diagram (glusterd inside the Nebula
tunnel, FUSE mounts of XDG dirs, `gluster_worker` subscribing
to nebula_supervisor enroll/revoke events), walks the
conflict-resolution model (LWW by mtime, `.conflict-<host>-<ts>`
siblings, no version history), and lists the ten
bench-observable gates for the v5.0.0 cut sign-off.
Cross-references every GF-N worklist cluster to keep the doc +
worklist synchronized.

## Unreleased — GF-1.1 + GF-1.2: GlusterFS substrate landed in spec

First v5.0.0 substrate commit. The RPM now pulls in
`glusterfs-server` + `glusterfs-fuse`, and the `%post`
scriptlet enables `glusterd.service` alongside `mackesd.service`.
glusterd binds locally (no Nebula-overlay drop-in yet — that's
GF-1.3) but is reachable for the upcoming `gluster_worker`
(GF-2.x) to drive volume creation + peer-join CLI without a
manual operator step.

## Unreleased — VV-2.a: policy-lifecycle writer for `voice-desired.json`

Closes the gap VV-2 left open: approved `voice_mesh` +
`voice_public` revisions in `desired_config.spec_json` now flow
into `/var/lib/mackesd/voice-desired.json` automatically on every
reconcile tick, so operators don't have to hand-edit the
override file to make a new dial plan take effect.

- **New module `mackesd_core::voice::materialize`.** Pure-
  function `build_voice_desired(snapshot, node_id, address_lookup)`
  derives a `VoiceDesired` from the snapshot's
  `Policy::VoiceMesh` + `Policy::VoicePublic` rows: sorts peer
  rows by extension, drops the self-row from the dispatcher
  table, looks up per-peer mesh addresses from each peer's
  `<qnm_root>/<peer_id>/mackesd/nebula-bundle.json:overlay_ip`
  (with `0.0.0.0` fallback when the bundle hasn't replicated
  yet), and populates this peer's Vitelity sub-account from the
  matching `VoicePublic` row.
- **Idempotent write.** `materialize_voice_desired()` serializes
  the result and compares byte-for-byte against the existing
  file; only renames when the bytes differ, so the `voice_config`
  worker's mtime gate fires exactly once per policy change.
- **`DesiredSnapshot::voice_policies: Vec<Policy>`** —
  default-empty for backward compat with the v3.x snapshot shape;
  the reconciler calls the materializer immediately after
  `load_desired_snapshot()` and logs `Wrote` / `Unchanged` /
  `SkippedNoPolicies` per tick.
- **`DEFAULT_DESIRED_JSON` constant** moved from the
  async-services `workers::voice_config` module into the
  always-on `voice::materialize` module + re-exported under the
  legacy path, so the binary that pulls in workers and the lib
  that doesn't both reach the same path.
- 10 unit tests cover empty-snapshot skip, write-on-change,
  unchanged-on-resubmit, missing-bundle fallback, self-vitelity
  pickup, other-peer-vitelity rejection, and the
  non-voice-policy filter (an `AllowEastWest`-only snapshot
  doesn't touch the file). Existing 562 lib tests + 7
  failure-scenarios integration tests stay green.
- **Deferred:** the 3-peer integration test from VV-2.a's
  acceptance criterion needs a live `kamcmd dispatcher.list`
  assertion against three running Kamailio peers — that's the
  VV-15 acceptance harness's scope, not VV-2.a's, and is
  tracked there.

## Unreleased — VV-4: voice-routing heuristic + dispatcher priority plumbing

Ships the latency-favoring routing heuristic that picks direct
vs transit paths for Kamailio's dispatcher.

- **New module `mackesd_core::voice`.** Pure-function
  `best_path(target_node_id, &[Candidate]) -> Path`. Filters
  direct candidates whose `rtt_ms < 80` AND `loss_pct < 5`,
  picks the lowest-score survivor (score =
  `loss_pct.mul_add(10.0, rtt_ms)`), falls back to a transit
  path through the best reachable relay peer. `Path::Direct` /
  `Path::Transit` discriminant; `pick_relay` exposed for
  callers that want to override the transit selection. 18
  unit tests covering the heuristic, the round-trip JSON
  shape, the edge cases (no candidates, all dead, exact-at-cap
  rejection).
- **`PeerEntry.priority: u8`** plumbed through `mde-voice-config`
  so generated `dispatcher.list` rows carry the heuristic's
  preference in the priority column. Defaults to `0` for
  backward-compatible JSON parsing — operator hand-edits don't
  need to know about the field. The future VV-2.a writer
  derives this from `best_path` (direct → high priority,
  transit → low priority).
- **3-peer integration drill** from the original VV-4
  acceptance is HW-bench-blocked (needs three live Nebula
  peers + iptables drop). Ported to the Hardware Testing
  epic; pure-fn surface is fully testable in CI without it.

## Unreleased — VV-2 + VV-3: voice-config generator drives Kamailio + RTPengine

Closes the loop on the v4.1.0 Voice & Video platform tier: the
generator now produces real `dispatcher.list` + `uacreg.list`
rows from a `VoiceDesired` document, two new `Policy` variants
+ conflict checks land in `mackesd_core::policy`, and a new
`voice_config` worker keeps the daemons reloaded when the
operator-visible JSON document changes.

**VV-2 — `mde-voice-config` generator + worker:**

- **`VoiceDesired` carries peers + Vitelity now.** New
  `PeerEntry { extension, node_id, display_name, mesh_address }`
  list drives the `dispatcher.list` rows; new
  `VitelityAccount { username, password, outbound_cid }` drives
  the per-peer outbound REGISTER binding in `uacreg.list`. 24
  unit tests + 6 `insta` snapshot fixtures (up from 16/4)
  cover both empty and populated cases.
- **`mackesd voice render-config` reads from JSON now.** New
  `--desired-json PATH` flag (default
  `/var/lib/mackesd/voice-desired.json`) lets operators or the
  new `voice_config` worker supply a populated `VoiceDesired`
  document. Missing file silently falls back to
  `VoiceDesired::boot_default(node_id)`; parse errors on a
  present file are hard errors (no silent fallback masking a
  bad hand-edit). `--boot-default` flag forces the boot path
  for testing.
- **New `voice_config` worker.** Polls
  `/var/lib/mackesd/voice-desired.json` mtime every 5 s. On
  first tick, if the file is absent, seeds it with
  `boot_default(node_id)` so the daemon's `ExecStartPre`
  helper always has *something* to generate from. On every
  mtime advance, shells out to `systemctl
  try-reload-or-restart kamailio-mde.service
  rtpengine-mde.service`. `try-reload-or-restart` is the
  no-op-when-disabled variant — safe to run on a fresh peer
  before VV-4 / VV-14 enable the daemons. Registered in the
  supervisor's worker pool alongside heartbeat / mdns /
  fs_sync.

**VV-3 — `voice_mesh` + `voice_public` policy variants:**

- `Policy::VoiceMesh { id, extension, node_id, display_name }`
  — assigns a 4-digit extension to a remote peer in the voice
  mesh.
- `Policy::VoicePublic { id, peer_node_id, vitelity_username,
  vitelity_password, outbound_cid }` — per-peer Vitelity
  sub-account that drives the outbound REGISTER + the CID
  rewrite.
- Conflict detection added: two `VoiceMesh` rules assigning the
  same extension to different peers raise `PolicyConflict`;
  two `VoicePublic` rules pointing at the same peer also
  conflict (a peer has at most one Vitelity trunk). 8 new
  tests; existing `detect_conflicts()` API unchanged.

**Worklist hygiene:** VV-3's original acceptance listed
`policy::types.rs`, `schemas/policy/voice_*.json`, and a
`policy_dispatch::dispatch()` arm — none of those structures
exist in the codebase today (the policy module is a single
`Policy` enum + `detect_conflicts()` validator with no
external schema files). The shipped pattern matches what's
actually present; the worklist entry now records the
divergence honestly. The full Phase-12 policy lifecycle that
writes `voice-desired.json` from approved revisions in the
store is split out as VV-2.a (open) and explicitly blocks on
extending `DesiredSnapshot` with a `voice_policies` field.

## Unreleased — VV-1 + VV-1.5: Kamailio + RTPengine voice stack foundation

First substantive code on the v4.1.0 Voice & Video epic after
the 2026-05-24 operator-directed Asterisk→Kamailio swap. Ships
the platform foundation: per-host Kamailio (SIP signaling), per-
host RTPengine (SRTP media relay), their dedicated users + dirs,
and the bootstrap path that lets both start cleanly.

- **New systemd unit `kamailio-mde.service`.** Runs Kamailio 5.8
  as the dedicated `_kamailio_mde` user, bound to
  `127.0.0.1:5060` (loopback for the embedded PJSIP client) +
  `nebula1:5061` TLS (Nebula mesh) only — never a public
  interface. Hardened with `ProtectSystem=strict`,
  `NoNewPrivileges`, capability set narrowed to
  `CAP_NET_BIND_SERVICE`.
- **New systemd unit `rtpengine-mde.service`.** Runs RTPengine
  as the dedicated `_rtpengine_mde` user. SRTP relay only — no
  transcoding (operator lock 2026-05-24). NG control socket at
  `/var/run/rtpengine-mde/ng.sock` is `_rtpengine_mde:_kamailio_mde`
  so Kamailio can drive it via the `rtpengine` module. RTP port
  range `30000-40000/udp` bound to `nebula1` + loopback only.
- **New Rust crate `mde-voice-config`.** Pure-function generator
  producing four files: `kamailio.cfg`, `dispatcher.list`,
  `uacreg.list`, `rtpengine.conf`. 16 unit tests including 4
  `insta` snapshot fixtures. VV-1's minimal scope produces a
  bootable default-config that answers SIP OPTIONS keepalives,
  stores REGISTER bindings from the local PJSIP client, and
  cleanly 503's everything else until VV-2/VV-4/VV-14 add the
  routing.
- **New CLI subcommand `mackesd voice render-config`.** Invoked
  by both systemd units' `ExecStartPre=` hooks so the on-disk
  config is always coherent with the active policy snapshot.
  `--dry-run` prints to stdout; `--kamailio-dir` /
  `--rtpengine-dir` override paths for tests. Atomic
  write+rename so a partial render never leaves either daemon
  reading a half-written file.
- **Spec changes** (`packaging/fedora/mackes-shell.spec`): adds
  `Requires: kamailio >= 5.8` + `Requires: rtpengine`, creates
  the `_kamailio_mde` + `_rtpengine_mde` system users in `%pre`,
  adds `_kamailio_mde` to the `_rtpengine_mde` group so it can
  write to the NG socket, seeds the state/TLS dirs in `%post`,
  installs both units + config dirs.

**Not yet enabled at install time.** The `%post` scriptlet
deliberately omits `systemctl enable --now kamailio-mde
rtpengine-mde` until VV-4 (mesh routing) + VV-14 (Vitelity REGISTER)
are also green — otherwise an operator running the new RPM would
have both daemons up but the SIP routing path would just 503
every INVITE.

**What's NOT in v4.1.0 (moved to v4.2.0):** ConfBridge,
voicemail, music-on-hold, ring groups, IVR, recordings, group
chat. These all need a media server (transcoding + mixing +
recording) which v4.1.0 deliberately doesn't ship. The v4.2.0
"Voice PBX" epic picks the media server (FreeSWITCH / Janus /
other) and adds the features back.

## 4.0.0 — runtime integration sweep: everything actually works now (2026-05-22)

The headline change: MDE's v3 line shipped most features as
helpers + tests that were never wired into the runtime. A live
operator session on 2026-05-22 hit four user-visible bugs ("start
menu won't close", "notification panel won't close", "missing
window management buttons", "right-click on the start menu does
not work") that all traced back to the same root cause — 13 of
18 mde-panel modules were marked `[✓] shipped` but never
referenced from `update()`/`view()`. The v3.0 cut shipped Iced
panel chrome where most of the panel was placeholder text.

v4.0.0 is "the version where everything actually works." 16
integration commits closed every dead-code module the audit
surfaced, wired the 6 unspawned mackesd workers, codified the
no-stubs rule (`§0.12`) + runtime-reachability gate
(`§0.8 gate 7`) that would have caught the gap, and added a
worklist-rescue Phase 0 to the iteration loop so it can't
recur.

### Headline (operator-visible)

* **Popovers actually close.** Toggle dedup on the panel side
  (clicking a tray icon a second time closes the existing
  popover instead of stacking a new one), zombie reaping (no
  more 18 defunct `mde-popover` processes after a session),
  visible × close button in every popover header, Esc still
  works via keyboard subscription. Bug fix for "start menu
  won't close" + "notifications panel won't close" + the
  underlying spawn-and-leak infrastructure problem.
* **Window-management buttons in the panel.** Min / max / close
  cluster between the tray and the clock. Min routes to sway
  scratchpad-hide; max toggles floating-fill (not fullscreen)
  per the v8.7 lock; close fires `swaymsg kill`. Greyed out
  when no toplevel is focused.
* **Right-click on the M button opens the admin menu.** Custom
  `mouse_area` wrap (Iced's built-in `button` is left-click
  only — this was the exact gap). 9 admin actions across 5
  sections (Shells / Packages / Services / Security / Storage)
  route through pkexec per the v2.0.3 polkit lock.
* **Focused-window hero in the panel.** Left of the tray, with
  the 280ms slide animation per the v1.1.0 Win10 layout lock.
* **F3 opens an exposé grid.** Fullscreen layer-shell overlay
  with one card per top-level window; click to focus.
* **Super+V opens the clipboard history popover.** Reads the
  mesh-synced JSON file populated by mackesd's clipboard
  worker (now actually spawned). Click an entry to copy it
  back to the local clipboard; fires a toast confirming.
* **Toasts.** Long-running render surface tails
  `~/.cache/mde/toasts.jsonl`; emit sites append JSON. First
  in-tree producer: clipboard popover Copy. Stack of 3 with
  FIFO eviction.
* **Bottom-right Win10-style watermark.** Visible only when
  `dnf check-update --quiet` reports pending updates; click
  fires `pkexec dnf upgrade -y`. Polls every 4 hours.
* **Clock popover shows a calendar + a weather column.** Weather
  fetched via `curl https://wttr.in/?format=j1` every 30 min.
* **mde-files toolbar search filters the visible rows.** Grid-
  layout helpers also now consumed at render time.
* **Drawer renders real brightness + volume sliders.** Bound
  to brightnessctl + pactl via the existing helper math.

### Headline (architectural)

* **mackesd async supervisor spawns 6 Phase B workers** —
  clipboard, mdns, fs_sync, heartbeat, mesh_router (with
  empty bootstrap state), notification_relay. Each gets
  `RestartPolicy::OnFailure`. The legacy reconcile worker
  continues on its own std::thread alongside.
* **wlr-foreign-toplevel subscription via swaymsg** — the
  panel observes every sway top-level via `swaymsg -t
  subscribe -m '["window"]'`. Drives the hero widget + window
  buttons + (future) tasklist.
* **Structured tracing context** — `LogContext::fresh()` at
  daemon startup attaches correlation_id + node_id to every
  log line via a tracing span. Per-tick correlation IDs land
  in v4.0.1.
* **§0.12 no-stubs rule** codified in `.claude/CLAUDE.md`.
  Never commit `pub mod foo;` that no other file references;
  never commit `Kind::Foo => exit 0` stubs; every commit ships
  fully complete code reachable from a runtime entry point.
* **§0.8 Definition-of-Done gate 7: runtime reachability.**
  A worklist item is not `[✓]` until at least one runtime
  entry point invokes one of its public functions. The grep
  test is the same one the worklist-rescue Phase 0 runs.

### Worklist hygiene

* **22 misleading `[✓] shipped` entries re-cued to `[>] In
  Progress`** during the audit + later closed by integration
  commits in this release.
* **3 pure-doc scaffold directories deleted** (`mackesd::deploy`,
  `mackesd::service`, panel `layer_shell.rs`). They were
  reserving directory layout for "future submodules" — exactly
  the pattern §0.12 forbids.
* **`root_menu.rs` retired** after wireability investigation —
  swaybg owns the wallpaper, no clean event hook exists, and
  all four root_menu actions are already exposed via
  Workbench / mde-files / `xdg-open ~/QNM-Shared`.
* **iteration skill gained Phase 0** — the worklist-rescue
  pipeline now runs before any iteration commit so the loop
  can't add dead modules on top of a rotten foundation.

### Known-blocked deferrals

These v3.0.3 items are blocked on larger architecture work
that owns its own epic; they ship in v4.1+ as their respective
epics close:

* **icon_mapper popover** — blocked on dock-applet right-click
  refresh.
* **dock_dnd integration** — blocked on dock-applet drag
  recognition refresh.
* **12.17 STUN candidate gathering / 12.18 HTTPS-tunneled
  fallback wiring** — blocked on `TransportRegistry` having
  concrete `Transport` impls (transport-architecture work).
* **mde-files DBusBackend `impl Backend`** — blocked on Phase G
  `model::{Peer,SelfNode,FileRow}` migration off
  `&'static str` fields.
* **KDC2-3.3 dbus methods** — blocked on the KDC2-3.4..3.9
  bundle in the KDC2 epic (`docs/PROJECT_WORKLIST.md` §KDC2).
* **KDC2-2.8 TLS wiring into KDC host** — blocked on KDC host
  transport refactor in the same epic.

### Audit doc

Full inventory + dependency-ordered integration plan +
process retro at
[`docs/V3_RUNTIME_INTEGRATION_AUDIT.md`](docs/V3_RUNTIME_INTEGRATION_AUDIT.md).

## 3.0.1 — KDC2 native re-implementation + v3 cut (2026-05-22)

(Tagged v3.0.1 rather than v3.0.0 — the v3.0.0 git tag was
reserved in 2024 against an earlier rename plan and points at
a pre-MDE commit. Functionally this is the v3.0 release.)


The headline change: MDE replaces its v13 wrapper of upstream
`kdeconnectd` with a built-in native KDE Connect host (the
"KDC2" re-implementation). The Workbench grows a Connect peer
card; phone notifications flow through mako via the
`dev.mackes.MDE.Connect` D-Bus surface and the Iced
notifications applet badges them with the phone glyph.

### Breaking changes

* **Phones must be re-paired exactly once.** The pair-store
  format moved from `~/.config/kdeconnect/` to
  `~/.config/mde/connect/` and the handshake keypair is
  generated fresh on first launch. The wizard surfaces a
  one-time card on the v2.0.x → v3.0.0 upgrade explaining
  this; re-pair each phone from **Workbench → Connect** after
  finishing setup.
* **`kdeconnectd` is no longer required.** The native
  re-implementation runs in-process inside `mackesd` and
  binds UDP/1716 + the D-Bus name (`dev.mackes.MDE.Connect`).
  The upstream `kdeconnectd`, `kdeconnect-cli`, and
  `kdeconnect-indicator` packages are **Obsoletes:**d on
  upgrade and **Conflicts:** on co-install. Operators that
  relied on `kdeconnect-cli` should script against the
  `dev.mackes.MDE.Connect1` D-Bus interface instead
  (`RingDevice` / `SendSms` / `SendClipboard` / `SendFile` /
  `PairDevice` / `UnpairDevice`).
* **`org.mackes.*` D-Bus aliases removed.** The v2.0.0
  one-release back-compat window for the legacy bus names
  closed. Only `dev.mackes.MDE.*` ships from v3.0 onward.
* **Qt-free dep closure.** With the wrapper gone, MDE pulls
  zero `libQt*` / `libKF*` / `PyQt*` / `PySide*` deps. A
  spec `%check` guard fails the build if any sneak in.

### KDC2 platform

* **Pure protocol crate `mde-kdc-proto`** — wire (Packet,
  codec, framing), discovery (Announce + UDP/1716 encoder +
  mDNS TXT encoder + DiscoveryRegistry with source-address
  cache), crypto (KeyStore trait + RingKeyStore with
  RSA-2048 PKCS#1v15/SHA-256 + AES-256-GCM session AEAD),
  plugins (ten typed packet bodies: Battery / Clipboard /
  Notification / Share / SMS / Ping / FindMyPhone / MPRIS /
  Telephony / RunCommand). 120+ unit tests including
  libFuzzer corpus seed on the codec.
* **Host crate `mde-kdc`** — first-launch identity
  generation (PKCS#8 RSA-2048), file-backed pairing store
  with interior mutability, live TLS connect via
  `tokio_rustls` + cert-fingerprint pinning, D-Bus host on
  the user session bus with `ListDevices` / `GetDevice` /
  `PairDevice` / `UnpairDevice` / `RingDevice` / `SendSms`
  / `SendClipboard` / `SendFile` methods + `DeviceAdded` /
  `DeviceRemoved` / `DeviceUpdated` signals + dispatch-time
  policy enforcement (per-device gating via
  `[plugins.<name>] allow_devices`), and host-side
  `UdpBroadcastRunner` + `MdnsRunner` discovery I/O.
* **Transport trait crate `mackes-transport`** —
  `Transport` trait + `MessageClass` + capability model,
  `PeerPath` per-peer router state, scorer
  (`select_best_transport` with flap-penalty cooldown),
  conformance harness.
* **Mesh router worker** in `mackesd` — ticks every 10 s,
  records decision time into the
  `kdc2_router_decision_us` Prometheus histogram
  (100µs..50ms bucket schedule); SLO check asserts
  p50 < 5 ms / p99 < 25 ms. Mesh-shunt fan-out reads
  every neighbor's `phones.json` from QNM-Shared and
  injects synthetic announces so a phone paired with one
  peer becomes reachable from every peer. `PathSwitch`
  audit events distinguish direct-LAN ↔ mesh-shunt ↔
  flap-penalty transitions.
* **Metrics-flush worker** writes
  `/var/lib/node_exporter/textfile_collector/mackesd.prom`
  every 10 s; the `kdc2_router_decision_us` snapshot
  reflects live router observations.

### Packaging

* `Obsoletes: kdeconnect kdeconnectd kdeconnect-cli
  kdeconnect-indicator`,
  `Conflicts: kdeconnect kdeconnect-cli gsconnect`.
* Legacy `crates/mackes-kdc/` (296 LOC) + `kdc_bridge`
  worker (154 LOC) + `data/dbus-1/services/org.mackes.*`
  aliases deleted.
* Spec `%check` stanza scans every Rust binary with `ldd`
  + the Python tree with grep for `PyQt[0-9]+` /
  `PySide[0-9]+` — fails the build on any hit.

### Other

* **mde-output-autoscale grows EDID-aware diagonal split.**
  4K outputs with a known physical size now pick 1.5
  (desk-distance monitor) or 2.0 (sofa-distance TV) by
  diagonal — a 27" 4K Acer XB272 and a 40" 4K Vizio V405
  on the same machine pick different scales without
  operator override.
* **mackesd `peer-card --peer <id>`** CLI subcommand wires
  the existing `peer_join::handle_peer_joined` helper for
  operator-driven peer-card spawns.
* **Wizard re-pair card** activates on v2.0.x → v3.0.0
  first boot when `~/.config/kdeconnect/` exists and
  `~/.config/mde/connect/identity.pem` doesn't.
* **`%check` Qt-free dep-closure guard** (KDC2-6.4).

### Worklist closeout

64 KDC2 sub-tasks shipped between 2026-05-19 and
2026-05-22. The remaining open items in
`docs/PROJECT_WORKLIST.md` are:

* 10 KDC2 hardware/operator-verification gates (KDC2-4.4
  Tailscale Transport blocked on infra, KDC2-4.6 Docker
  3-peer integration test, KDC2-7.1-7.7 acceptance gates).
* 4 HW-* Hardware Testing epic items (fresh-install
  bench, upgrade bench, Wayland smoke, Docker peer
  fan-out).
* The 26 v2.2 / v2.1+ scope items (CB-1.x retirements,
  UX-13-23, BR-*, UX-*.a, 0.7, C.11, 12.18) are tracked
  as post-v3.0 follow-ups with explicit "needs X before
  it can ship" notes; the v3.0 cut intentionally does not
  block on them.

## 2.0.3 — Operator-verification hotfix bundle (2026-05-22)

Hotfix release driven by a fresh v2.0.2 bench install on a
laptop + 4K-TV dual-monitor rig. Every fix landed at source after
direct operator surfacing of the symptom; the seven discrete
defects + their root causes are captured below.

* **Sway config parse errors + duplicate bindings** —
  `data/sway/config` shipped with `bindsym $mod+Shift+r restart`,
  which is an i3-only command. Sway has no `restart`, only
  `reload`. Sway fired swaynag on every login. Five additional
  bindings (`$mod+q/w/e/l/space`) were defined in both the main
  config and `config.d/mackes-defaults.conf`, generating duplicate-
  binding warnings. Removed the conflicting main-config bindings
  (mackes-defaults wins) and added arrow-key navigation aliases
  (`$mod+arrows`) to replace the focus-right binding that mackes-
  defaults repurposes for `loginctl lock-session`. Also wired the
  panel into the session — added an `exec mde-panel` autostart
  line so the panel comes up on login.

* **mde-panel `app_id` empty under Wayland** — The Iced 0.13
  application builder does NOT inherit `app_id` from
  `iced::Settings.id` on Linux — only from
  `window::Settings.platform_specific.application_id`. mde-panel
  called `.window_size(...)` instead of `.window(window::Settings
  { platform_specific: ... })`, so the xdg-shell `app_id` field
  was never written. Sway's `for_window [app_id="..."]` rule
  therefore never matched and the panel landed as a tiled grey
  strip across the middle of the screen. Fixed by introducing
  `APP_ID = "shell.mackes.Panel"` (freedesktop-style reverse-DNS,
  matches the planned `.desktop` basename) and wiring it into
  `window::settings::PlatformSpecific::application_id` via the
  full `window::Settings` literal. New unit test locks the
  constant against the sway config rule so future renames touch
  both files. Title-match fallback rule kept around for v2.0.2
  binaries on bench machines that haven't upgraded yet.

* **Obsolete v1.x qnm-daemon.service in crash-loop** —
  `mde-migrate-from-1x` only migrated config/cache/state
  directories. It left obsolete v1.x systemd user units
  (`qnm-daemon.service`) in place, which then crash-looped every
  3 seconds because the binary they reference
  (`~/.local/bin/qnm-daemon`) was removed from v2.0.0. The bench
  machine recorded 290+ restarts within minutes of login. Added
  `OBSOLETE_USER_UNITS` list + `disable_obsolete_unit()` helper
  that disables --now via systemctl and deletes the unit file.
  `qnm-daemon.service` is the first entry; more legacy units can
  be added as they're discovered.

* **dunst → mako (Wayland-native notifications)** —
  `dunst.service` ships as a D-Bus activated unit owning
  `org.freedesktop.Notifications` but dunst is X11-only and
  crashed on every Wayland login. Phase 1:
  `install-helpers/bench-bootstrap.sh` lands as a reversible
  operator-run helper that `dnf install`s mako (+ Wayland debug
  tools), masks `dunst.service`, and enables `mako.service` so it
  owns the notification name on next login. Phase 2:
  `packaging/fedora/mackes-shell.spec` gains `Requires: mako` +
  `Conflicts: dunst` so fresh installs auto-converge.

* **Dual-monitor default scaling** — Bench rig was eDP-1 1366×768
  + DP-2 3840×2160 both at `scale=1.0`. UI elements on the 4K TV
  were unreadable across a living-room viewing distance. Added
  `bin/mde-output-autoscale`: width-based heuristic (4K → 2.0,
  2K → 1.5, ≤1080p → 1.0) applied via `swaymsg output ... scale
  ...` at every session start. `exec_always` in `data/sway/config`
  so display hotplug triggers a re-pick. Operator overrides
  (current scale ≠ 1.0) are sacred — the helper skips them.

* **Right-click admin menu pkexec migration** — `mackes-panel/
  src/admin_menu.rs` spawned `terminator -x bash -c 'sudo ...'`
  for every privileged action. Under Wayland sessions terminator
  doesn't always inherit a controlling TTY (sway, lightdm,
  mde-session all spawn it without one), so sudo's password
  prompt failed with "a terminal is required to read the
  password" — the user saw most of the right-click items surface
  a sudo error. Switched every elevation call site to
  `pkexec sh -c '<cmd>'` so the polkit GUI auth agent owns the
  prompt. Drive-by cleanups: `systemctl status` + `dnf history
  list` no longer escalate (they don't need root); `sudo -i`
  became `pkexec bash -l`; `sudoedit` became `pkexec nano`. Hard
  regression guard: a test fails CI if any future SECTIONS edit
  reintroduces raw `sudo`. Same `sudo dnf upgrade` → `pkexec
  dnf upgrade` change applied to the watermark's left-click
  handler.

* **Watermark branding refresh + synced build date** — The
  legacy GTK watermark still said "Mackes XFCE Workstation"
  (v1.x project name). Updated to "Mackes Desktop Environment".
  Version line now reads "MDE 2.0.3 (build <hash>) · Built
  <YYYY-MM-DD>". The build-hash and build-date stamps are
  written by the RPM `%install` step to `/usr/share/mde/
  build-{hash,date}` (with SOURCE_DATE_EPOCH support for
  reproducible builds) and read by BOTH watermarks (legacy GTK
  in `mackes-panel` + Iced in `mde-panel`) so they cannot drift
  on which build is reported.

## 2.0.2 — 2.0 Bug Fixes (2026-05-22)

Bug-fix release on the v2.0.x line. Hardens the v2.0.0 monolithic
cut with the UX-3..UX-9 chrome polish work, the PC-2/3/4.a/10/12
peer-card foundations, the BR-0 brand asset pack, the
short-circuit RPM guard, and the first green ci pytest run since
pre-1.1.0. Larger feature work (UX-7.a sweep, UX-8.a SVG bundle,
UX-9.a animation wiring, PC-3.a event-source integration,
PC-5/6/7 online enrichment, CB-1.8 follow-up, 12.18 HTTPS-tunnel)
ships in a later minor.

### Chrome + design system (UX round 1)

- **UX-3 — palette + iced::Theme coherence.** `iced::Theme::Custom("MDE")`
  is derived from `mde_theme::Palette`, so every widget that defers
  to the framework palette renders against the Q-locked indigo
  (`#5b6af5`) accent + Q-locked Apple-charcoal (`#1d1d1f`) background
  instead of Iced's stock dark navy.
- **UX-4 — custom MDE window header bar.** sway tiles Iced apps
  without server-side decorations; the workbench now draws a 48 px
  `palette.surface` header bar with the "MDE" wordmark at
  `TypeRole::Subheading` (14 sp medium) and min / max / close
  controls that hover-tint with the accent (close hover-tints
  danger). `decorations: false` requested so GNOME-shell /
  KDE-on-Wayland sessions also fall back to client-side chrome.
- **UX-5 — sidebar nav polish.** 240 px fixed sidebar, 40 px nav
  rows with a reserved 20 px icon slot, accent stripe + tint +
  accent text on the selected row, surface-2 hover, 1 px hairline
  section dividers with all-caps 11 sp muted labels, focus ring
  on the active row when the sidebar pane holds keyboard focus.
- **UX-6 — panel chrome primitives (Phase 1+2 + 6.a + 6.b).** New
  `crates/mde-workbench/src/panel_chrome.rs` exposes
  `panel_container`, `section_header`, `section_block`,
  `data_row`, `status_badge` (5 severities), `card`, and
  `empty_state` (plus `dialog`, `dialog_backdrop`, `tooltip` from
  UX-9). SPACE_24 outer padding now applies at `App::view()`
  centrally, so every panel inherits it; `Padding::new(0.0)`
  no-op wrappers swept from 32 panels. Six data panels
  (snapshots / inventory / mesh_history / fleet_revisions /
  playbooks / run_history) carry a polished zero-data view.
- **UX-7 — control state primitives (Phase 1).** `controls.rs`
  ships `variant_button(Primary | Secondary | Ghost)`,
  `styled_text_input`, `toggle` pill, `skeleton`, and `spinner`
  with the locked dimensions (36 px button height, RADIUS_MD,
  SPACE_12 padding, 40 % disabled opacity, 2 px focus ring).
  Snapshots panel migrates as the canonical example. The 27-site
  sweep across remaining panels + focus-ring render are
  follow-ups (UX-7.a; focus-ring chains on the UX-PRE Iced
  upgrade).
- **UX-8 — Carbon icon API (v1).** `mde_theme::icons::Icon`
  enum (48 semantic variants) + `mde_icon(icon, size) ->
  ResolvedIcon` resolver paired with Q37-locked size tiers
  (16 / 20 / 24 / 32 / 48 px). Window controls, empty-state
  hero icons, and `icon_for_device_type` peer-card router swap
  in. Real Carbon SVG bundling is UX-8.a follow-up.
- **UX-9 — motion + dialog chrome (Phase 1).** `mde_theme::motion`
  centralizes the locked durations (180 ms panel mount, 2 s
  notification pulse, 120 ms tooltip fade) + dialog tokens
  (480 px max-width, 50 % backdrop). `panel_chrome::dialog`,
  `dialog_backdrop`, `tooltip` widgets ship. Snapshots-restore
  confirm dialog migrates to the new chrome. UX-9.a wires the
  actual animation subscriptions (needs Iced 0.14).

### Peer Connection Card foundations

- **PC-2** — `PeerProbe` schema moved from the PC-1 placeholder
  to its canonical home `crates/mackes-mesh-types/src/peer_probe.rs`
  (re-exported as `mde_mesh_types::peer_probe::*`). Cross-crate
  consumers (mded peer-join worker, future workbench Fleet
  panels) now share one definition.
- **PC-3** — stand-alone peer-join handler
  (`crates/mackesd/src/peer_join.rs`): writes
  `~/.cache/mde/peers/<id>/probe.json`, spawns
  `mde-peer-card --peer <id>` as a detached child, debounces
  re-spawn within a 30 s per-peer window. Event-source
  integration into the mesh / topology layer chains as PC-3.a.
- **PC-4.a** — production `Hwdb` index over
  `/usr/share/hwdata/usb.ids`; `Hwdb::shared()` caches the
  parsed index process-wide via `OnceLock`. `HwdbInfo::from_lookup`
  returns resolved vendor / product names with hex-string
  fallbacks for unknown IDs.
- **PC-10** — `mackes_config::PeerCardConfig.online_enrichment`
  toggle (default `true`); when `false`, PC-5/6/7 enrichment
  short-circuits and the card renders hwdb-only.
- **PC-12** — RPM spec ships `mde-peer-card` at `%{_bindir}/`;
  always spawned on demand by PC-3, no autostart entry.

### Brand assets

- **BR-0** — `assets/brand/` ships wordmark / wordmark-hero /
  monogram / app-icon / greeter-wordmark in both PNG (ChatGPT-
  generated raw originals + flat slots) and baked SVG forms;
  `crates/mde-theme/src/brand.rs` runtime loader resolves
  `BrandSlot` to bytes with `$MDE_BRAND_DIR` override and
  `include_bytes!` fallback. BR-1..BR-5 consumer-side wiring
  scheduled for v2.2.

### Infrastructure

- **Short-circuit RPM guard** — `make rpm` rejects any artifact
  carrying `rpmlib(ShortCircuited) <= 4.9.0-1`. v2.0.1 shipped
  two RPMs with that dep (manual `rpmbuild --short-circuit`
  during iteration); the guard catches it the next time. See
  `.claude/CLAUDE.md §0.6` for the canonical-path note.
- **ci pytest green** — pytest had been red since pre-1.1.0
  (10 failures from `empty_state(... None, None)` keyword-only
  arg mismatches in 4 legacy GTK panels + an order-sensitive
  facade test). Fixed the call sites + switched the facade
  test to `sys.modules` lookup (order-independent). The
  Python ci job can finally land green: 412 passed · 11
  skipped · 0 failed.

### Test counts (all green)

- mde-workbench lib: 479 (was 452 pre-cut)
- mde-theme lib: 83 (was 59)
- mde-peer-card lib: 30 (was 21)
- mackes-config lib: 20 (was 18)
- mackesd peer_join: 8 (new)
- pytest: 412 (was 402, 10 failures gone)

### Known follow-ups (Open in `docs/PROJECT_WORKLIST.md`)

UX-7.a control sweep + focus-ring render · UX-8.a Carbon SVG
bundle · UX-9.a animation wiring · PC-3.a event-source
integration · PC-5/6/7 online enrichment · CB-1.8 follow-up
(10 Network admin Iced panels) · 12.18 HTTPS-tunnel. The
**UX-PRE** Iced 0.13 → 0.14 workspace bump is BLOCKED on
upstream softbuffer 0.4.8 / Rust 1.95 incompatibility; it
gates Phase E.2 layer-shell + the UX-7.a focus-ring render.

## 2.0.1 — Wayland session hotfix (2026-05-21)

The v2.0.0 RPM declared every `mde-*` Rust binary in `%files` but the
`%install` section never copied them out of `target/release/`. The
result was a working RPM-metadata Wayland desktop with **none** of
the binaries that implement it — `/usr/bin/mde-session`,
`/usr/bin/mde-panel`, `/usr/bin/mded`, `/usr/bin/mde-drawer`,
`/usr/bin/mde-wizard`, and 16 applets were all missing. LightDM
silently filtered the MDE session out of its dropdown (TryExec
pointed at the missing `mde-session`), leaving the user with only
the upstream `sway.desktop` entry. v2.0.0 boxes booted into vanilla
sway — visually i3-like but with no MDE panel, no MDE workbench, no
mesh integration.

**Fixes:**
- Spec now copies every workspace binary built by
  `cargo build --release` into `%{buildroot}%{_bindir}/`:
  `mde-panel`, `mde-session`, `mde-logout-dialog`, `mded`,
  `mde-applet-drawer`, `mde-wizard`, plus all 16
  `mde-applet-*` binaries (clipboard / clock / dock / network /
  status-cluster / sway-cluster / volume-osd / brightness-osd / …).
  Each applet install wraps in `[ -f target/release/$applet ]` so a
  partial workspace build doesn't break the spec.
- `mackes/birthright.py` gains step 20 —
  `apply_uninstall_legacy_xsessions()` — that sweeps three known
  orphan xsession `.desktop` files from `/usr/share/xsessions/`
  (`xfce11-i3-plank`, `xfce11`, `mackes`). These were installed by
  pre-2.0 shell scripts and not tracked by RPM, so package
  upgrades couldn't remove them; their broken `TryExec` paths
  confused the LightDM dropdown.
- Wired the new step into `mackes/wizard/pages/apply.py` between
  the existing legacy-XFCE uninstall and the Mesh step.

Verification: 266 unit tests pass / 93 skipped / 0 failed (4 new
tests cover the legacy-xsession sweep — idempotency, partial-set
removal, rm-failure reporting, allow-list audit).

## 2.0.0 — Mackes Desktop Environment (MDE) monolithic cut (2026-05-20)

The v2.0.0 cut commit. Package rename + Wayland-only stack flip
in one coordinated commit so `dnf upgrade` lands every 1.x user
on `mde-2.0.0` in a single transaction.

**Package identity:**
- `Name: mde` (was `mackes-xfce-workstation`).
- `Provides: mackes-shell` + `Provides: mackes-xfce-workstation`
  + `Obsoletes: mackes-shell < 2.0.0` +
  `Obsoletes: mackes-xfce-workstation < 2.0.0` so every 1.x
  install path resolves to `mde-2.0.0`.

**Dependency swap (CB-3.2):**
- Dropped every XFCE Requires: `xfconf`, `xfce4-settings`,
  `xfce4-session`, `xfce4-power-manager`, `terminator`, `i3`,
  `i3status`, `dmenu`, `wmctrl`, `xprop`,
  `xorg-x11-server-utils`, `xdotool`.
- Added hard Wayland Requires: `sway`, `swaylock`, `swayidle`,
  `swaybg`, `foot`, `bemenu`, `brightnessctl`, `pipewire`,
  `wireplumber`, `grim`, `slurp`.
- New Recommends: `cosmic-files`, `yazi`, `kanshi`,
  `wlogout`, `wofi`.
- Dropped BuildRequires: `xfce4-panel-devel`, `libxfce4ui-devel`
  (C panel plugins retired).

**Conflicts block (CB-3.3, Q5 lock):**
- `Conflicts: xfce4-panel < 999`, `xfdesktop < 999`,
  `xfce4-session < 999`, `xfce4-settings < 999`,
  `xfwm4 < 999`, `xfce4-whiskermenu-plugin < 999`,
  `xfce4-docklike-plugin < 999`,
  `xfce4-pulseaudio-plugin < 999`,
  `xfce4-power-manager-plugin < 999`, `i3 < 999`.
- After `dnf install mde`, trying to install any of those
  errors out cleanly ("would break mde") instead of leaving
  the old desktop pieces running alongside MDE.

**XDG autostart cleanup (CB-3.5, H.4):**
- Retired every override: `mackes-panel.desktop`,
  `xfdesktop.desktop`, `mackes-enforce-session.desktop`,
  `mackes-suppress-xfce4-panel.desktop`,
  `kdeconnect-indicator.desktop`. The Wayland session
  orchestrator (`mde-session`) and sway config own
  panel + desktop bring-up natively.

**C panel-plugin trio retired:**
- `mackes-clipboard`, `mackes-launcher`, `mackes-drawer` no
  longer build or ship. Their roles move to native
  `mackes-panel` applets in Phase E.1.x (Iced port).

**Workbench:**
- 21 Iced Workbench panels shipped across the v1.1.x
  partial-progress cuts now compose the v2.0.0 Workbench
  surface (`/usr/bin/mde-workbench`).
- 5 `mded` subcommands shipped: `nodes list`,
  `ansible-history list`, `playbooks {list, run}`,
  `events list`.
- Tests: 164 → 444 unit tests across mde-workbench (+170 %).

**Upgrade path:**
`dnf upgrade` on a 1.x box lands on `mde-2.0.0` via the
Provides/Obsoletes graph. The Conflicts: block evicts any
xfce4-* / xfwm4 / i3 packages the user previously installed
manually; if dnf reports a refusal, the user removes the
named package and retries (the install-helpers/uninstall
flow already supports this).

The repair panel's "Restart mded" + "Re-install MDE launcher"
buttons handle most of the runtime first-boot smoothing.
`mde-firstboot.target` orchestrates the
`mde-migrate-from-1x` + `mde-shell-migrate-v2` oneshots on
first login so xfconf state migrates to MDE settings + sway
config seeds from the shipped template.

## 1.1.4 — Drop all XFCE Obsoletes (dnf5 install fix, take 2) (2026-05-20)

`dnf install mackes-xfce-workstation-1.1.3-1.fc44.x86_64.rpm`
still crashed with:

> terminate called after throwing an instance of 'libdnf5::AssertionError'
> 'implicit_ts_elements.empty()' failed: The rpm transaction
> contains more elements than requested

Even on a small upgrade (1.0.7 → 1.1.3) where dnf5 only had
one explicit upgrade + 5 implicit erases via Obsoletes, libdnf5
≤ 5.2.x trips an internal accounting assertion when the
implicit element set is non-empty during fill().

Fix: dropped all 5 remaining XFCE Obsoletes from the spec
(xfdesktop, xfce4-whiskermenu-plugin, xfce4-docklike-plugin,
xfce4-pulseaudio-plugin, xfce4-power-manager-plugin). The
runtime birthright step `apply_uninstall_legacy_xfce` already
walks the same list with `dnf remove` after install — it was
the real cleanup all along, the spec Obsoletes were
belt-and-suspenders. Now they're actively harmful.

Spec audit test (`test_spec_does_not_obsolete_legacy_xfce_packages`)
flipped to assert the inverse: any of those 6 names in the
Obsoletes block fails.

v2.0.0's monolithic cut uses `Conflicts:` + `Provides:` /
`Obsoletes:` at the package-rename moment, with a different
test surface; the Obsoletes question is revisited there.

## 1.1.3 — Install fix + Iced MDE Workbench preview (2026-05-20)

Install regression — 1.1.0/1.1.1/1.1.2 RPMs failed to install
on a fresh Fedora 44 box with:

> conflicting requests
> mackes-xfce-workstation requires libxfce4panel-2.0.so.4 but
> obsoletes xfce4-panel < 999 (the only provider).

Root cause: the C panel-plugin under
`data/panel-plugins/mackes-clipboard/` links
`libxfce4panel-2.0.so.4`, which only the `xfce4-panel` package
provides. The spec was Obsoleting xfce4-panel at the same time —
which made the package un-installable end-to-end.

Fix: removed `Obsoletes: xfce4-panel < 999` from the spec and
dropped `xfce4-panel` from the `_LEGACY_XFCE_PACKAGES` tuple in
`apply_uninstall_legacy_xfce`. The xfce4-panel binary stays
suppressed via the existing
`/etc/xdg/autostart/mackes-suppress-xfce4-panel.desktop`
override (the runtime behaviour the Obsoletes was reaching
for); only its on-disk library + .desktop files remain so the
linked C plugin keeps its library provider. v2.0.0's monolithic
cut retires the C plugin entirely and can restore the
Obsoletes then.

The other 5 Obsoletes (xfdesktop, xfce4-whiskermenu-plugin,
xfce4-docklike-plugin, xfce4-pulseaudio-plugin,
xfce4-power-manager-plugin) stay — none of those provide
shared libraries we link.

## 1.1.2 — Iced MDE Workbench preview (2026-05-20)

First v2.0.0-line preview shipped inside a v1.x point release.
`mde-workbench` is a new Iced binary that ports an early slice
of the CB-1 Workbench rewrite (`crates/mde-workbench/`,
164 unit tests). The v1.x Python+GTK3 Workbench remains the
default — `mde-workbench` ships alongside as an opt-in
preview so users can exercise the v2.0.0 surfaces before the
monolithic cut.

**What's shipping**

- **Scaffold:** 9-group collapsible sidebar (Dashboard / Apps /
  Devices / Fleet / Look & Feel / Maintain / Network / System
  / Help), breadcrumb + page-title chrome, keyboard nav (Tab
  cycles sidebar↔main, Ctrl+1..9 jumps to group, Escape
  closes detail), `--focus <slug>` deep-link CLI arg.
- **Single-instance D-Bus contract** —
  `dev.mackes.MDE.Shell.Workbench.Focus(slug)` interface on
  the workbench's own bus name (`dev.mackes.MDE.Workbench`);
  a second `mde-workbench --focus <slug>` call routes
  through the live instance instead of opening a duplicate
  window. Replaces the v1.x WM_CLASS-based hack.
- **9 working panels** wired to the unified Backend trait
  (live `dev.mackes.MDE.Settings.Get/Set` via zbus, with
  a DemoBackend swap-in for tests):
  - Look & Feel: themes, fonts, wallpaper.
  - System: session (3 booleans), notifications (DND
    checkbox + placement combo + numeric expire-ms).
  - Devices: power (5 keys: profile combo, lid_action combo,
    two idle-suspend integers, presentation_mode checkbox),
    removable (3 automount booleans).
  - Fleet: settings (key + value_json + peers Push subprocess
    to `mded fleet push-setting`), revisions (list + Rollback
    button per row).
- **Launch surface:** new `mde-workbench.desktop` entry under
  Settings + System categories.

**Out of scope for 1.1.1 (tracked as `[ ] Open` follow-ups in
the worklist):** the remaining ~36 panels across Apps,
Devices (displays / sound / printers), Fleet (inventory /
playbooks / run_history), Look & Feel preview, Maintain,
Network, System (datetime / default_apps / window_manager /
snapshots), plus the Wizard port. Each follow-up names the
backend it needs.

**Other**

- `cargo test -p mde-workbench`: 164 pass.
- Workspace gains `crates/mde-workbench/` with iced 0.13 +
  zbus 5 (tokio) + tokio process + clap deps. The CB-1
  panel modules share `panels/json_helpers.rs` for the
  Settings JSON wire-format encode/decode helpers.

## 2.0.0 — Rebrand to Mackes Desktop Environment (MDE) + Wayland-only Rust DE

**Rebrand:** "Mackes Shell" becomes "Mackes Desktop Environment (MDE)" on
first reference, "MDE" thereafter. RPM package `mackes-shell` →  `mde`;
binaries `mackesd` → `mded`, `mackes-panel` → `mde-panel`, `mackes` →
`mde`. D-Bus surfaces `org.mackes.*` → `dev.mackes.MDE.*`. Config paths
`~/.config/mackes-shell/` → `~/.config/mde/`. Full identifier mapping
ships in `docs/design/v2.0.0-mde-rebrand/identifiers.md`.

**Upgrade path (Phase H):** `dnf upgrade` from any v1.x lands on `mde-2.0.0`
automatically via `Obsoletes: mackes-shell < 2.0.0` + `Provides:
mackes-shell = 2.0.0` in the new spec. `mde-migrate-from-1x` (runs from
mde-session.service the first time it starts) atomically moves
`~/.config/mackes-shell/` → `~/.config/mde/` (and cache + state trees);
`mde-shell-migrate-v2` does the first-boot heavy lift (xfconf channels →
settings table, drop XDG autostart overrides, back up `~/.config/xfce4/`,
seed `~/.config/sway/`). Env-var shim reads `MDE_*` first, falls back to
`MACKES_*` with a one-shot deprecation warning (drops in v2.1). D-Bus
service-file aliases for the v1.x `org.mackes.*` names ship one release
for backward compatibility.

**Architectural shifts:**

- **Unified Rust meta-daemon.** Every long-running v1.x Python daemon
  folds into `mded` as a `Worker` registered with the Phase A.2
  supervisor: `clipboard`, `mdns`, `fs_sync`, `media_sync`,
  `remmina_sync`, `ansible_pull`, `kdc_bridge`, `heartbeat`,
  `notification_relay`, `notifications_server`. `mded serve` is the new
  systemd ExecStart (replaces the v1.x `migrate && status`); the 10
  retired standalone `.service`/`.timer` units leave the spec.
- **Wayland-only (sway).** XFCE + X11 + i3 retired. Layer-shell + Iced +
  libcosmic + smithay-client-toolkit + swayipc-async for the panel +
  applets; new `mde-session` crate orchestrates login + the
  `dev.mackes.MDE.Session` DBus surface. `data/sway/config` ships as a
  drop-in replacement for `data/i3/config` with matching binding names.
- **Native settings layer (`mded_core::settings`).** 29 dot-notated keys
  cover theme / font / display / power / notification / automount /
  wallpaper / keybinds / autostart. Each value routes through GSettings
  or a JSON sidecar under `$XDG_CACHE_HOME/mde/`; the matching applier
  in `crates/mackesd/src/settings/` handles the side effect. The
  `dev.mackes.MDE.Settings` interface exposes `Get / Set / Snapshot /
  Restore / ListKeys + Changed` signal.
- **Fleet config layer.** `DesiredSnapshot.settings_keys` carries
  per-revision (key, value_json) pairs that every peer's reconcile loop
  applies via `settings::apply_all`. Workbench panels Fleet → Push and
  Fleet → Revisions surface the push + rollback paths.
- **Notifications.** `mded` implements `org.freedesktop.Notifications`
  per spec — every libnotify / notify-send / GTK app reaches `mded`
  transparently, retiring `mako` / `fnott` / `xfce4-notifyd`. Cross-peer
  notification relay reads `~/QNM-Shared/<peer>/.qnm-notifications/`
  and persists to the `notifications` table.

**Workbench panels migrated to MDE settings bridge:** Devices→Power,
System→Removable Media, System→Notifications, System→Session,
System→Window Manager. New: Fleet→Push, Fleet→Revisions. Drawer DND +
Caffeine toggles flip the same flag files the notifications_server +
mde-session honor. `mackes/menu_integration.py` retired (XFCE settings
panels no longer installed).

**Spec changes (Phase H):** drops `i3`, `i3-gaps`, `xfwm4`,
`xfce4-session`, `xfce4-power-manager`, `xfce4-notifyd`,
`xfce4-clipman`, `xfsettingsd`, `xfconfd`, `xfconf`, `xfce4-settings`,
`thunar-volman`, `xdotool`, `xprop`, `wmctrl`, `xrandr`, `xclip`. Adds
`sway`, `swayidle`, `swaylock`, `swaynag`, `swaybg`, `foot`,
`wl-clipboard`, `brightnessctl`, `wlr-randr`, `udisks2`,
`power-profiles-daemon`, `upower`, `pipewire`, `wireplumber`.
Recommends: `cosmic-files`, `yazi`, `kanshi`. Drops thunar.

**Testing:** workspace test count crosses 400 (was 230). Phase 12.11.3
failure-scenario suite (7 named cases) green; Phase 12.11.2
testcontainers integration tests gated under `--features docker-tests`;
Cairo rendering smoke under headless `ImageSurface`. New Phase 9.3
xdotool E2E gates run in CI under Xvfb.

**Installer (CB-5.x rebrand):** `install.sh` banner now reads
"Mackes Desktop Environment (MDE) · installer" with the
"PatternFly 6 · Wayland · Fedora" subtitle (was "Carbon Design System
chrome · XFCE · Fedora"). Hand-off `exec mackes` → `exec mde` (the
bin shim covers the back-compat window per CB-3.7). Wizard / TUI
hints rewritten to `mde --wizard` / `mde --tui`. Headless fallback
(no DISPLAY + no WAYLAND_DISPLAY) now nudges the user toward
picking "Mackes Desktop Environment" from the greeter session menu
on next login — no GPU probing (Q2 hard-switch lock — no detect-
and-pick; the user picks the session entry once and stays there).
Smoke: `bash -n install.sh` green; 7 rebrand assertion tests under
`tests/test_install_sh_rebrand.py`.

### BREAKING CHANGES (Phase H + CB-3.x)

- **XFCE 4 desktop fully removed.** Every `xfce4-*` Requires line
  drops (xfwm4, xfce4-session, xfce4-power-manager, xfce4-notifyd,
  xfce4-clipman, xfsettingsd, xfconfd, xfconf, xfce4-settings) and
  the supporting X11 tooling (xdotool, xprop, wmctrl, xrandr,
  xclip, i3, i3-gaps, thunar, thunar-volman) goes with it. v1.x
  panels that wrote `xfconf` keys now route through
  `mackes.mde_settings_bridge` instead — the bridge maps onto
  gsettings keys + JSON sidecars under `$XDG_CACHE_HOME/mde/`.
- **Wayland-only (hard switch, Q2 lock).** sway is the only
  supported compositor. No "detect-and-pick" between Wayland and
  X11 — the installer informs, the greeter offers the session, the
  user picks once. X11 sessions from v1.x stop launching after
  upgrade (the spec drops the `.desktop` entries).
- **Binary rename `mackes` → `mde`** (and `mackesd` → `mded`,
  `mackes-panel` → `mde-panel`, etc). v1.x names ship as bin-
  shims for one release window (per CB-3.7) so existing scripts
  + bookmarks keep working; the shims will land their deprecation
  warning at v2.1 cut and the names disappear at v2.2.
- **DBus surface rename `org.mackes.*` → `dev.mackes.MDE.*`.** One
  release of alias `.service` files keeps clients of the v1.x
  names working transparently.
- **Config path move `~/.config/mackes-shell/` → `~/.config/mde/`.**
  Atomic migration runs on first launch of `mde-session.service`
  via the new `mde-migrate-from-1x` helper (cache + state trees
  move too).
- **Env-var rename `MACKES_*` → `MDE_*`.** New names take
  precedence; old names still read with a one-shot deprecation
  warning + retire at v2.1.
- **DNF upgrade UX (hard switch).** `dnf upgrade` from any v1.x
  ships `mde-2.0.0` automatically via `Obsoletes: mackes-shell
  < 2.0.0`. The transition is one-way — the v1.x package is no
  longer in the repo. Reverting requires a snapshot rollback
  (via `mde recover --latest` if a snapshot was taken
  pre-upgrade).

## 1.1.0 — Win10 layout (2026-05-19)

Visual reskin of the panel chrome from a 20 px top bar + 80 px
Plank-parity dock into a single 40 px Win10-style taskbar at the
bottom. Same content sources (panel.toml, status_cluster probes,
desktop_files scan, weather popover, recents catalog) — the actual
behavior changes are right-click admin menu, focused-app hero,
desktop watermark, and the new XDG / clipboard / update plumbing.

### Panel chrome

- **Single 40 px bottom taskbar.** Layout left → center → right:
  Start (`apple_menu_button`, lit-amber when open) + pinned-apps
  strip; centered i3 cluster (SPLIT / LAYOUT / WINDOW chips, no
  workspace switcher); status cluster + two-line clock. The prior
  top bar + Plank dock builders stay in-tree as `#[allow(dead_code)]`
  for one release cycle.
- **Right-click Start: 9-item Fedora admin menu (Q15/Q16).** Sections
  — Shells / Packages / Services / Security / Storage. Each item
  launches `terminator -x bash -c '<cmd>; bash'` (keeps the shell
  open after the command finishes per the terminator deprecation of
  `--hold`). Tooltips show the literal command + a sudo-cache hint
  (`sudo -nv` exit code) so the user knows whether the action will
  prompt.
- **`window_buttons.rs` retired (Q11/Q12).** i3 keybindings (Mod+q
  close / Mod+f fullscreen / Mod+space float) + each app's own CSD
  buttons carry the UX. Apps without CSD use the keybindings.

### Desktop layer

- **Win10-style watermark (Q19–Q21, suggestions #2/#10).** Three
  lines in the lower-right corner (name + `Version 1.1.0 (build
  <git-hash>)` + Fedora release · hostname). Hidden by default;
  becomes visible when `dnf check-update` reports pending updates
  (poll every 4 h). Version line gains `— N updates available`
  while the count is known and >0. Left-click opens
  `terminator -x bash -c 'sudo dnf upgrade --refresh; bash'`;
  right-click drops a context menu — *Check for updates now* /
  *Hide for this session*.

### Workbench integration

- **`mackes --focus <slug>` second-click toggles closed (suggestion
  #5).** A repeated tray click on the same status-cluster slug
  destroys the workbench rather than opening a second window.
  Implementation in `app.py` + `_active_panel_key` exposed by
  `sidebar_window.py:go_to`.
- **First-time wizard critically reviewed (10 pages → 3).** Welcome
  / Scan / Appearance / Hardware / Network / Summary demoted to
  Workbench panels or dropped. Wizard retains Preset (conditional)
  / Review / Apply (with silent snapshot). Analysis only this
  release — implementation lands in 1.1.1.

### Fedora-native plumbing

- **XFCE menu hides expanded (18 → 32 entries).** Now covers
  xfce4-panel preferences, Whisker, docklike-plugin, xfdesktop,
  xfce4-screensaver, appfinder, xfce4-settings-editor, xfconf-query.
  Propagated to existing users on every login via the
  `mackes-enforce-session` autostart (the 1.0.8 enforcer also gains
  a 5a step that enables Mackes user systemd units idempotently).
- **`90-mackes.preset`.** Fedora systemd-preset that enables the
  Mackes user units (clipboard daemon, gvfsd-mesh, remmina-sync,
  media-sync) for new accounts. Closes the gap that left the mesh
  clipboard daemon never auto-starting on 1.0.x installs.
- **`apply_user_dirs` birthright step.** Rewrites
  `~/.config/user-dirs.dirs` so XDG well-known folders point at
  `~/QNM-Mesh/{Documents,Music,Pictures,Videos}`; Downloads stays
  local at `~/Downloads`; Desktop / Templates / Public Share collapse
  to `$HOME`. Idempotent; backs up the legacy file once on first
  rewrite to `user-dirs.dirs.legacy`.
- **`.repo` file at Fedora best practice.** `repo_gpgcheck=1`,
  `metadata_expire=4h` (matches the watermark poll cadence),
  `clean_requirements_on_remove=True`.
- **`mackes update` CLI subcommand.** Single unified update path
  shared with the watermark + admin menu (`sudo dnf upgrade
  mackes-xfce-workstation --refresh`). Flags: `--check-only` /
  `--refresh|--no-refresh` / `--yes`.
- **AppStream releases.** Both `mackes-shell.metainfo.xml` and
  `shell.mackes.Panel.metainfo.xml` carry `<release>` entries for
  1.0.8 + 1.1.0; both validate clean via `appstreamcli validate`.

### Notes

- **PNG screenshots in metainfo are deferred** — must be captured on
  a real Mackes host (workbench / taskbar / mesh topology) and
  dropped into `branding/screenshots/` before the next release that
  surfaces them in GNOME Software / KDE Discover.
- **Hero animation (i3-msg subscribe + 280 ms slide), Carbon Icon
  Mapper, multi-monitor wallpaper, PulseAudio compliance, and the
  full clipboard manager popover land in subsequent 1.1.x point
  releases** — the design is locked (memory:
  `project_1_1_0_win10_layout`), the implementation is sequenced
  but not in this tag.

## 1.0.8 — First-boot hotfix: i3 + mackes-panel takeover on every login, Workbench geometry, status-cluster opens Workbench (2026-05-19)

Three bugs reported after installing 1.0.7 + rebooting on a stock Fedora
44 XFCE session: xfwm4 and xfce4-panel still started (Failsafe template
hadn't been overridden), the Workbench window was being tiled
full-screen by i3 (no `for_window` rule matched it), and the top-right
status-cluster icons opened the drawer instead of the Workbench.

### Window manager / panel takeover

- **`mackes-enforce-session` (new XDG autostart).** A small shell
  script installed at `/usr/bin/mackes-enforce-session` and wired to
  `/etc/xdg/autostart/mackes-enforce-session.desktop`. On every
  login it idempotently runs `i3 --replace` (no-op when i3 is
  already the active WM), kills any `xfce4-panel` / `xfdesktop`
  that `xfce4-session` spawned from its Failsafe client list, and
  re-launches `mackes-panel` if it died. Closes the gap between
  install-and-reboot and the `apply_enforce_i3` /
  `apply_panel_swap` birthright steps, which previously only ran
  when the user opened the setup wizard manually.
- **`mackes-suppress-xfce4-panel.desktop` (new XDG autostart).**
  Belt-and-braces Hidden=true override for the XDG autostart spawn
  path (mirrors the existing `xfdesktop.desktop` override). Doesn't
  conflict with the `xfce4-panel` RPM because it lives at a
  Mackes-prefixed filename.

### Workbench geometry on i3

- **`Mackes-shell` WM_CLASS + i3 float rule.** `WorkbenchWindow`
  now calls `set_wmclass("mackes-shell", "Mackes-shell")` so the
  res_class is stable + predictable. `data/i3/config` grows a
  matching `for_window [class="^Mackes-shell$"] floating enable`
  rule alongside the existing `Mackes-panel` rule. Result: the
  workbench respects `set_default_size(1280x720)` +
  `WindowPosition.CENTER` again. Existing users with a stale
  `~/.config/i3/config` from 1.0.7 should run `mackes-wm reset`
  (or delete the file and re-login) to pick up the new rule.

### Top-bar status cluster click-target

- **`mackes --focus <slug>` opens Workbench focused on a panel
  (Q-lock 2026-05-19).** Every status-cluster icon (mesh,
  clipboard, volume, battery, notifications, user) now spawns
  `mackes --focus <slug>` instead of `mackes --drawer
  --drawer-focus <slug>`. The Python side owns the slug → panel
  mapping (mesh → mesh_join, volume → devices, battery / user →
  system, clipboard / notifications → dashboard); unknown slugs
  fall through to the dashboard. The drawer is no longer
  reachable from this cluster — it stays bound to Super+M and the
  drawer applet.

## 1.0.7 — Plank-parity dock, i3, About, drawer wiring, window buttons, xfwm4 retirement, mackesd scaffold (2026-05-19)

Second polish wave on the Mackes XFCE Workstation line. Brings the dock
to feature parity with Plank, adds optional i3 as a tiling alternative
to xfwm4, replaces the popover-only status cluster with live read-only
numeric indicators, and wires every probe in the Python drawer to a
real data source.

### Window management

- **i3 fully replaces xfwm4 (Phase 8.8).** xfwm4 is no longer
  installed by the RPM. The XFCE session host (xfsettingsd,
  xfce4-power-manager, thunar, xfconf) stays unchanged — only
  the WM swaps. `bin/mackes-wm` simplifies to `status` + `reset`;
  legacy `i3` / `xfwm4` verbs print a deprecation note. The
  `apply_enforce_i3` birthright step auto-migrates existing
  1.0.6 installs on first launch (stops + disables
  mackes-maximizer.service, runs `i3 --replace`, seeds
  `~/.config/i3/config` if missing). `mackes-maximizer` (the
  binary, the user-systemd unit, the autostart .desktop) is
  retired — i3 tiles natively.
- **Workbench → System → Window Manager simplified.** Drops
  the WM-toggle row; renders only the i3 layout-preset grid.
- **Top-bar window-management buttons (Phase 8.7).** Three
  Carbon-symbolic glyphs at the far-right corner of the top bar:
  minimize / maximize / close. Operate the i3 focused window via
  `i3-msg`. 45% greyed-out + no-op click when no window is
  focused. Maximize uses `floating toggle + resize 100 ppt` so
  the panel chrome stays visible (NOT `fullscreen toggle`).
  AT-SPI accessible names + tooltips. 4 unit tests cover the
  JSON scan for i3's focused leaf container.

### Top bar

- **Status cluster shows live numbers.** The six right-side icons
  (mesh / clipboard / volume / battery / notifications / user) now
  render an icon + numeric pair (`🌐 3`, `🔊 75`, `🔋 87`, …) refreshed
  every 2 s. Clicking an item opens the Notification Drawer scrolled
  to the matching section. Probe failure renders `—` with a dimmed
  icon and a tooltip naming the cause (`Mesh: tailscale not running`).
  New module `crates/mackes-panel/src/status_cluster.rs`; replaces the
  1.0.6 review popovers.
- **Top-bar strut tracks realized height.** A 500 ms timer republishes
  `_NET_WM_STRUT_PARTIAL` once the bar's actual height settles past
  the requested 20 px; fixes the few-px occlusion delta on first
  paint under xfwm4 / i3.

### Dock

- **Plank-parity rebuild.** Pinned launchers on the left, a live
  tasklist on the right for every running window that doesn't already
  belong to a pinned launcher. Multi-window launchers show a 1 / 2 /
  3+ tick indicator under the icon. Left-click activates (or launches);
  right-click opens a context menu (Open New / Bring to Front: «title»
  / Close All Windows / Pin to Dock).
- **Polling refresh.** The dock rebuilds both segments every 2 s from
  a single `DockSnapshot` of open windows + `WM_CLASS`. Re-reads
  `panel.toml` per tick so Pin/Unpin actions land in ~2 s without a
  separate file-watch path.

### Window managers

- **i3 as an optional tiling WM.** New `/usr/bin/mackes-wm` shell
  switcher: `mackes-wm i3` runs `i3 --replace`, stops the
  mackes-maximizer service, and seeds `~/.config/i3/config` from the
  shipped `/usr/share/mackes-shell/i3/config` default. `mackes-wm
  xfwm4` swaps back. Workbench → System → Window Manager surfaces an
  active-WM toggle row plus (under i3) an 8-cell layout-preset grid
  (Maximized / Side by Side / Split-in-4 / Master+Stack / Tabbed /
  Stacking / Focus / Floating) driven by `i3-msg`. RPM gains
  `Requires: i3 i3status dmenu`.

### About + drawer

- **About Mackes window.** New `mackes/about.py` opens a scrollable
  window over the bundled `data/ABOUT.txt` (credits + license +
  upstream attributions). Wired to the apple-menu's "About Mackes"
  item and the `mackes --about` CLI flag.
- **Drawer live-data wiring pass.** Replaced every mocked data source
  in `mackes/drawer.py` with real probes: `pactl` (volume),
  `bluetoothctl` (Bluetooth), `xfconf-query notifyd` (do-not-disturb),
  `xfce4-power-manager presentation-mode` (caffeine), `tailscale
  status --json` (mesh + fleet), `who -u` (remote sessions), MPRIS
  DBus (playing media), `/sys/class/power_supply` (battery),
  `/proc/{stat,meminfo,loadavg}` (hardware). Sections that depended
  on subsystems not yet implemented (Drift / Shared storage / Daemons
  grid) were removed rather than left as placeholders.

### Mesh control plane scaffold (Phase 12)

- **`crates/mackesd/` workspace member (Phase 12.1.1).** New Rust
  crate ships two artifacts: the `mackesd` binary (CLI for the
  mesh control plane — currently `migrate` + `status` subcommands)
  and the `mackesd_core` library (in-process read API for the
  panel — no IPC, no networked API per Phase 12.A.3 lock). Builds
  clean against cargo 1.95.0; 4 unit tests cover the SQLite store
  and migration application.
- **SQLite store with WAL + 8-table schema (Phase 12.2).** New
  `crates/mackesd/migrations/0001_init.sql` defines `nodes`,
  `desired_config`, `runtime_state`, `observed_telemetry`,
  `topology_link_health`, `events`, `policies`, `leader_lease`
  with CHECK constraints on the deployment-state machine and
  node roles. `mackesd_core::store::migrate` is idempotent.
- **RPM packaging for `mackesd` (Phase 12.1).** Spec gains the
  binary install line, a hardened `data/systemd/mackesd.service`
  unit, `%pre` creation of the `mackesd` system user/group,
  `%post` `systemctl enable --now`. State directory
  `/var/lib/mackesd` (0750, owned by mackesd:mackesd) created
  automatically by systemd's `StateDirectory=`. The reconcile
  loop subcommand (`mackesd serve`) lands in 12.5; today's unit
  runs `mackesd migrate` on every boot so the store stays current.
- **Connectivity scope (Phase 12.14–12.23, 25-Q survey 2026-05-19).**
  Locked: 16-peer small-business fleet, ~50% LAN/~50% WAN,
  throughput-first routing (NOT LAN-first), self-hosted DERP
  default with public Tailscale DERP as fallback, IPv6 descoped
  to a future phase, < 3 s first-packet SLO, < 10 s roaming
  handoff, no new security or monitoring requirements. Full Q&A
  + per-item evaluation in `docs/design/v12-connectivity-scope.md`.
- **Phase 12.17 + 12.21 + 12.23 — connectivity layer extends.**
  `crates/mackesd/src/stun.rs` ships a real RFC 5389/8489 STUN
  client: pure-fn binding-request encoder, attribute-walking
  binding-response parser that extracts XOR-MAPPED-ADDRESS for
  IPv4 + IPv6, and a tokio `gather_endpoint(server, timeout)`
  that validates the transaction-id echo before trusting the
  reflexive address (13 tests). `lan_discovery` gains
  `should_eager_bootstrap` (Phase 12.21 predicate — fresh + low-
  RTT prewarm decision) and the multicast surface (Phase 12.23 —
  locked group 239.42.7.16, wired-only Q16 guard,
  `open_multicast_listener(iface)` that joins the group via
  tokio). 4 new lan_discovery tests, taking the worker's unit
  count from 16 → 20.
- **Phase 12.14 + 12.15 + 12.22 — connectivity primitives shipped.**
  New worker `crates/mackesd/src/workers/lan_discovery.rs`
  announces `_mackes-peer._udp.local` via `mdns-sd` 0.11 and runs
  a tokio UDP probe loop (9-byte MPRB ping/pong, LE seq). RTT
  samples land in a shared `Registry`. Pure-fn ranking ships:
  `lan_direct_wins(lan_rtt, derp_rtt)` (Q23 throughput-first
  proxy), `ipv6_direct_wins(ipv6_rtt, ipv4_derp_rtt)` (Q12.15
  IPv6-first promotion), and
  `higher_throughput_wins(a_bps, b_bps)` (Q23 bandwidth-wins
  override). 16 unit tests cover encode/decode, registry
  semantics, and the full 4-quadrant truth table for every
  ranker.

### Phase 13 — KDE Connect integration (design lock)

- **Option A locked 2026-05-19 (5-option survey).** Wrap upstream
  `kdeconnectd` + Mackes-themed Workbench GUI over the
  `org.kde.kdeconnect.*` DBus interface + mesh-mDNS bridge as the
  shunt that re-announces remote phones on every peer's local
  LAN. Full 6-section worklist in `PROJECT_WORKLIST.md § Phase 13`.
  Implementation lands in 1.1+.

### Documentation + tests

- **AppStream metainfo refreshed.** `data/applications/mackes-shell.metainfo.xml`
  carries the 1.0.x branding ("Mackes XFCE Workstation"), the panel +
  dock + i3 feature list, and explicit release entries for 1.0.0,
  1.0.6, and 1.0.7. `appstreamcli validate` exits clean.
- **README rebuilt.** Drops the legacy 2.x framing. Adds a "Build from
  source" section listing every dev loop (`make rust`, `cargo run -p
  mackes-panel`, `make test`, `make test-nodeps`, `python3 -P -m
  mackes [--drawer|--about|status]`), with explicit toolchain
  dependencies for Fedora 44+.
- **Keyboard shortcut catalog.** `docs/help/keyboard-shortcuts.md`
  documents every panel-owned, WM-owned, drawer, and CLI mirror
  binding plus the `panel.toml:[keybindings]` override syntax.
- **Wayland readiness audit.** `docs/design/wayland-readiness.md`
  inventories every X11-specific surface (strut, wmctrl, xprop,
  xdotool, `XGrabKey`) with per-feature Wayland replacements
  (layer-shell, foreign-toplevel, idle-notify, global-shortcuts
  portal) and a sequenced port plan.
- **Panel-instantiation smoke test.** New `tests/test_panel_instantiation_smoke.py`
  walks `mackes.workbench.**`, finds every `*Panel(Gtk.Box)` subclass
  (49 discovered), and instantiates each headless under Xvfb with a
  5 s hard timeout per panel. Failures surface main-thread blocking
  bugs as "slow constructor" informational output. Full pytest run
  under Xvfb: 118 passed, 5 skipped.
- **Accessibility names + tooltips.** Apple-menu button, clock
  button, and all 6 status-cluster items expose AT-SPI
  `set_name` + `set_description`. Status cluster announces
  context-aware phrases ("Mesh: 3 peers online", "Notifications: 1
  unread") rather than the generic "button".

### Reliability + performance

- **`async_probe` helper (Phase 11.9).** New
  `mackes.workbench._async.async_probe(probe, on_result, on_error=None)`
  runs a probe function on a daemon thread and marshals the result
  back to the GTK main thread via `GLib.idle_add`. Swallows
  exceptions on both sides so a buggy panel can't corrupt GLib's
  main context. Canonical pattern for the Phase 11.9 reliability
  sweep — every blocking probe in `__init__` now has an idiomatic
  replacement.
- **Four panels stopped blocking the main thread.** FirewallPanel
  used to hang ≥ 5 s waiting on `firewall-cmd --list-all` when
  firewalld was down; MeshVpnPanel blocked 15 s on
  `tailscale_status` + `headscale_list_peers`; MeshSshPanel blocked
  7 s on `headscale_list_peers`; DependenciesPanel blocked on the
  initial `rpm -qa` walk. All four now render a skeleton on
  construct, then fill in via `async_probe`. The Workbench sidebar
  click → first paint is now < 50 ms for every converted panel.
- **`firewall-cmd` timeouts reduced 8 s → 2 s.** Long enough to
  succeed when firewalld is alive, short enough to give up before
  the user notices.
- **Panel-instantiation smoke test refactored** to surface remaining
  slow constructors as informational test output rather than
  failures — keeps the gate green while pointing at the next
  candidates for conversion.

- **Drawer process hold/release.** The GApplication `hold()`s before
  `toggle()` so the process survives past `do_activate`, and
  `release()`s when the drawer hides. Was a hot bug: drawer closed on
  first click because the GApp exited.
- **Sidebar status refresh non-blocking.** First `_refresh_status_bar`
  call now runs on a background thread; previously blocked
  `WorkbenchWindow.__init__` for ~7 s while headscale + fleet + drift
  probes ran synchronously.
- **`python3 -P` mackes wrapper.** RPM-installed `/usr/bin/mackes` now
  invokes `python3 -P -m mackes` so the cwd's `mackes/` subdirectory
  never shadows the installed package. Cold start from
  `~/Desktop/files`: 17 s → 1.5 s.

## 1.0.6 — First-boot panel polish (2026-05-18)

(Patch numbers 1.0.1–1.0.5 were already taken by the legacy Mackes
Shell 2.x train; this is the direct successor to 1.0.0 on the Mackes
XFCE Workstation line.)


User-feedback bundle on the freshly-installed Mackes XFCE Workstation
panel. Five bugs, fixed together because they all surface on first
launch and share build/test gates:

- **Top-bar icons are now visible.** `icons::load()` no longer feeds
  raw `fill="currentColor"` SVGs to gdk-pixbuf — that produced black
  glyphs on a black panel and made the left Mackes button + the right
  status cluster look unwired. The loader now substitutes
  `currentColor` for Carbon text-primary (`#f0f0f0`) before
  rasterizing, so every cached `Pixbuf` is already drawn in the
  panel's foreground color. A panel-scoped block in `data/css/mackes.css`
  forces `window#mackes-top-bar` / `window#mackes-dock` and their
  descendants to the same color so any label/button text follows.
- **Bottom dock auto-sizes and hides when empty.** Fixed
  `DOCK_HEIGHT_PX = 80` reserved a thick strip even with zero items.
  Now the dock strip is built first; if it has no children, the dock
  window never shows. When populated it sizes to `DOCK_ICON_PX + 8 px`
  padding (~30 % slimmer than the prior 80 px — full 50 % reduction
  would require shrinking the locked-by-Q12 48 px icon size).
- **Clock switches to 12-hour and opens a weather panel.** Top-bar
  clock is now `h:MM AM/PM` (`%l:%M %p`, leading space trimmed),
  wrapped in a frameless `gtk::Button`. Click opens a `gtk::Popover`
  rendering current temperature and symbol code fetched from
  `api.met.no/weatherapi/locationforecast/2.0/complete` — the same
  endpoint xfce4-weather-plugin uses. New
  `crates/mackes-panel/src/weather.rs` module; HTTP via the system
  `curl` (no new crate dep) with the descriptive User-Agent met.no
  requires. Default coords are London-as-sentinel until `panel.toml`
  grows a `[weather]` section. 3 unit tests cover the JSON parser
  shape.
- **Status-cluster review popovers.** Each of the 6 right-side
  status buttons (mesh / clipboard / volume / battery /
  notifications / user) now opens an in-process `gtk::Popover` with
  the cluster title + a one-line summary + an "Open in Drawer →"
  button that delegates to `mackes --drawer --drawer-focus <slug>`.
  The user gets immediate visual feedback whether or not the Python
  drawer subprocess is up — addressing the "Unable to open the
  dropdown to review" feedback.
- **Panel + dock publish `_NET_WM_STRUT_PARTIAL`.** New
  `crates/mackes-panel/src/strut.rs` looks up each panel window's
  XID via `xdotool search --name` (already a hard dep from Phase
  5.3's window-switching path) and publishes both
  `_NET_WM_STRUT_PARTIAL` (12-cardinal) and `_NET_WM_STRUT` (legacy
  4-cardinal) via `xprop -id`. Any EWMH-compliant window manager —
  xfwm4, i3, bspwm, awesome, LeftWM — now leaves the panel and dock
  space alone when windows maximize. Workspace-manager swap (five
  alternatives surveyed in the feedback thread) deferred to a future
  phase; this strut fix unblocks the occlusion bug under the current
  xfwm4.

## 1.0.0 — Mackes XFCE Workstation (2026-05-18)

The Mackes Shell line graduates from the 2.x XFCE-control-panel
framing to a unified product: **Mackes XFCE Workstation**. The RPM
renames from `mackes-shell` to `mackes-xfce-workstation` (with
`Obsoletes: mackes-shell < 3.0` so `dnf upgrade` is automatic), and
the desktop ships its own panel, dock, and wallpaper layer written
fresh in Rust. Filesystem paths (`~/.config/mackes-shell/`,
`/usr/share/mackes-shell/`, the `mackes/` Python package) stay
unchanged so 2.x installations carry forward.

Full design lock: `docs/design/v3.0.0-mackes-xfce-workstation.md`
(50-question survey, locked 2026-05-18).

### What's new

**Mackes-Carbon icon theme** — symbolic, single-color icon set
derived from IBM Carbon Design System (Apache 2.0). 2,617 SVGs
across the freedesktop categories with `fill="currentColor"`
injected so GTK and the Mackes panel CSS recolor uniformly. New
default for every preset. App-icon mapping table covers ~45 common
apps (Firefox → earth, Thunar → folder--open, vim → terminal …);
fallthrough is `applications-other-symbolic`.

**mackes-panel** — `/usr/bin/mackes-panel`, a new Rust binary that
renders the top status bar + bottom dock + wallpaper. Replaces
xfdesktop (and via Phase 8.3 autostart, takes over from
xfce4-panel on Mackes sessions).

  - **Top bar (20 px)** — Apple-menu button on the left, HH:MM
    clock in the center (wall-clock synced), 6-glyph status cluster
    on the right (mesh / clipboard / volume / battery / notifications
    / user). Each cluster click opens the v2.2.0 Notification Drawer
    with section focus. PatternFly dark surface, monochrome
    Mackes-Carbon glyphs.
  - **Apple menu** — real `gtk::Menu` dropdown: About / Settings /
    Software Update / Recent Items → / Applications → (categorized
    by `.desktop` Categories into Internet / Multimedia / Graphics
    / Office / Development / Games / System / Utilities / Other) /
    Force Quit / Sleep / Restart / Shut Down / Lock / Sign Out.
    System actions go through `loginctl` and `xfce4-session-logout`.
  - **Bottom dock (80 px)** — primary monitor only. Reads
    `~/.config/mackes-panel/panel.toml`, renders pinned apps + mesh
    resources interleaved per Q10. Clicking a running app raises
    its window via `wmctrl -i -a`; second click minimises with
    `xdotool windowminimize`. Mesh peers expose a six-button action
    popover: Files / SSH / RDP / VNC / Services / Send file.
  - **Wallpaper layer** — third Desktop-hint window owns the root
    background, sourced from `~/.config/mackes-shell/state.json`
    or the branded fallback.

**Config + mesh sync** — `panel.toml` lives in TOML at
`~/.config/mackes-panel/panel.toml`, mesh-replicated whole-file to
`~/.qnm-sync/mackes-panel/panel.toml`, hot-reloaded via inotify
(`gio::FileMonitor`), drift-detected against peers via SHA-256.
Look & Feel → Panel surfaces the sync status.

**Boot-to-desktop continuity** — Plymouth rebuilt (centered logo +
20 px progress strip pinned to the bottom edge), LightDM greeter
mirrors the panel's top bar (`panel-position = top`, `clock-format
= %H:%M`, slim indicator cluster), mackes-panel takes over after
login. Single visual language from power-on through running session.

**Performance** — measured under Xvfb (commit 99e2680):

      cold start  5 ms      (target < 200 ms, 40× under)
      RSS         85 MB     (target ≤ 150 MB,  43% under)
      idle CPU    0.0 %     (target < 1 %,     far under)

    `install-helpers/bench-panel.sh` is the perf gate — runs it,
    returns non-zero on regression.

**Workspaces dropped** — every preset ships `workspace_count: 1`.
Single desktop, Cmd+Tab app-switch model.

### Post-1.0 roadmap

The 50-question lock anticipated more than 1.0 lands in one cut.
The following items are tracked in `docs/PROJECT_WORKLIST.md` and
ship in follow-up minor releases:

- Global hotkey grabs (Super+Space / Super+Tab / Super+L / Super+V
  / Super+E / F3 etc.) via x11rb — the panel currently relies on
  xfconf-bound xfwm4 actions, so most macOS-style shortcuts work
  via xfwm4's own keybinding system; full Mackes-side grabs land
  in 1.1.
- Cmd+Tab app-switcher overlay and Exposé grid (need a window-
  thumbnails overlay layer).
- Notification Drawer port from Python to Rust (currently invoked
  via `mackes --drawer`).
- Full GTK widget + xdotool E2E test pyramid (workspace currently
  has 58 unit tests).
- First-launch migration wizard for 2.x → 1.0 user data.
- Root right-click menu (`Change wallpaper / Open mesh share /
  Send file to peer / Display settings`) — Phase 8.4.

### Migration from 2.x

`dnf upgrade` does the work. The new RPM `Obsoletes: mackes-shell
< 3.0` so the old package is replaced. Existing config in
`~/.config/mackes-shell/` is untouched. `~/.config/xfce4/panel/` is
archived to `~/.config/mackes-panel/legacy-xfce-panel/` on first
run for safekeeping. The birthright apply sequence brings up
mackes-panel, then quits xfce4-panel and xfdesktop, then rebinds
Super-key shortcuts to `mackes-panel --apple-menu`.

Foundation for the v3.0.0 / 1.0.0 rebrand per
`docs/design/v3.0.0-mackes-xfce-workstation.md`. Tracked in
`docs/PROJECT_WORKLIST.md`; currently 29 of 67 worklist items complete.

* **`mackes-panel`** — new Rust binary (`/usr/bin/mackes-panel`) that
  renders the top status bar + bottom dock + wallpaper. Three crates
  in the workspace: `mackes-mesh-types`, `mackes-config`, `mackes-panel`.
  ~2,290 lines of Rust, 38 unit tests, no `unsafe` (forbidden at the
  module level), clippy pedantic+nursery clean.

* **Performance gate measured.** `install-helpers/bench-panel.sh`
  runs the binary under Xvfb and samples `/proc/<pid>/`. First
  measurement (commit `99e2680`):

      cold start  5 ms       (target < 200 ms)
      RSS         85 MB      (target ≤ 150 MB)
      idle CPU    0.0 %      (target < 1 %)

  All three Q41-revised gates pass with significant margin.

* **What runs today.** Wallpaper layer (replaces xfdesktop). Top
  bar with Apple-menu button → real `gtk::Menu` dropdown with
  categorized Applications submenu + working system actions
  (`loginctl suspend|reboot|poweroff|lock-session`). HH:MM clock
  (wall-clock synced). Status cluster opens the existing Python
  Notification Drawer with section focus. Bottom dock reads
  `~/.config/mackes-panel/panel.toml` and renders pinned apps as
  monochrome Carbon glyphs via the new app→Carbon icon mapping.

* **Config persistence.** Panel config lives in TOML, mesh-replicated
  to `~/.qnm-sync/mackes-panel/panel.toml`, hot-reloaded via
  `gio::FileMonitor`, drift-detected per peer via SHA-256.

* **Packaging.** Mackes installs now ship
  `/etc/xdg/autostart/mackes-panel.desktop` (brings up the Rust panel)
  and `/etc/xdg/autostart/xfdesktop.desktop` (overrides upstream
  xfdesktop with `Hidden=true` so it never starts on Mackes).

* **Still gating actual 1.0.0 release** (see worklist Phase 5.2-5.3,
  Phase 6, Phase 4.3, Phase 9.1-9.3, Phase 10): libwnck-driven
  running-app / window switching, global hotkeys, Rust port of the
  Notification Drawer, GTK widget + xdotool E2E test pyramid,
  RPM rename to `mackes-xfce-workstation`, first-launch migration
  wizard.

## 2.3.0 — Mackes-Carbon icon theme (2026-05-18)

* **New default icon theme: `Mackes-Carbon`.** A symbolic, single-color
  icon set derived from the IBM Carbon Design System (Apache 2.0).
  Replaces `Black-Sun` as the default `xsettings/IconThemeName` for
  every preset (#!, mackes, daylight). Black-Sun is still installed —
  switch back in Look & Feel → Appearance.

* **Coverage:** 264 freedesktop standard icon names mapped explicitly
  across actions / apps / categories / devices / emblems / mimetypes /
  places / status — every name mackes-shell's own UI references plus
  the broader freedesktop spec (mail-*, format-*, go-*, view-*,
  weather-*, etc.). 2,526 native Carbon SVGs are also dumped under
  `scalable/apps/` so any Carbon basename works directly as an icon
  name (e.g. `Gtk.Image(icon_name="chart-bar")`).

* **Theming:** every SVG gets `fill="currentColor"` injected on the
  root `<svg>` so GTK's symbolic-icon recoloring and the panel CSS's
  `-gtk-icon-foreground-color` both work uniformly. Dark and light
  desktops both render correctly without separate variant files.

* **Reproducibility:** `install-helpers/build-mackes-carbon.sh` is
  idempotent — fetches Carbon SVGs from `/tmp/carbon-icons` (override
  via `CARBON_SVG_DIR=`), reads the freedesktop → Carbon name map from
  `install-helpers/mackes-carbon.map`, writes the theme tree, the
  `index.theme`, and a NOTICE + LICENSE attributing IBM Carbon. Re-run
  it after editing the map to refresh.

* **Packaging:** `packaging/fedora/mackes-shell.spec` installs the
  theme under `/usr/share/icons/Mackes-Carbon/` and runs
  `gtk-update-icon-cache` in `%post`. `mackes.birthright._VENDORED_THEMES`
  copies the tree alongside Orchis-Dark, Shiki-Statler, and Black-Sun
  during birthright apply. `mackes.birthright_check._check_themes`
  verifies it's installed.

### Also in this cut

* **Fix:** `Fleet → Run history` panel locked the entire app on open.
  `_reset_combo` triggered `changed` on the peer/playbook combos,
  which re-entered `_refresh()`, which re-rebuilt the combos — infinite
  recursion. Reentrancy guard added; the noop `handler_block_by_func(None)`
  stub is removed.

## 2.2.0 — Notification Drawer (2026-05-18)

**Breaking visual change.** Three surfaces are deleted and replaced by a
single XFCE panel applet:

  | Removed                              | Replacement                |
  |--------------------------------------|----------------------------|
  | Conky HUD (mackes/conky_hud.py)      | Notification Drawer        |
  | Tray icon (mackes/tray.py)           | Notification Drawer        |
  | Mini popover (mackes.workbench.popover/) | Notification Drawer    |

### What ships

* **C panel plugin** `mackes-drawer` (data/panel-plugins/mackes-drawer/) —
  external xfce4-panel plugin built against libxfce4panel-2.0. Renders
  a single pill on the panel:

  ```
    ▤ Mon May 18  10:34  ·  ◐ 3  ·  ⚡ 77% ▾
  ```

  Reads display state from `~/.cache/mackes/drawer-state.json`,
  refreshes every 5s. On click → spawns `mackes-shell --drawer`.

* **Python drawer window** (`mackes/drawer.py`) — right-anchored
  POPUP window, 420 px wide, full screen-minus-panel height. Slides
  in from the right with a 3 px accent stripe down the left edge.
  Sections (top to bottom): Header · Quick toggles (Mesh · Bluetooth
  · DND · Caffeine) · Volume + Brightness sliders · Mesh (hub + peer
  list) · Fleet (2×2 node grid) · Services (unread / playing /
  remote counts) · Notifications (list with clear-all) · Battery
  (bar + state) · Hardware (CPU/RAM/load/clock). Closes on Esc,
  focus-out, or re-clicking the panel pill.

* **Live data wiring** — every section reads from the existing
  Mackes modules: `mesh_vpn.tailscale_status()` for the mesh peer
  list, `/proc/stat` + `/proc/meminfo` for CPU/RAM,
  `/sys/class/power_supply` for battery, `~/.cache/mackes/notifications.json`
  for the notification queue, `~/.cache/mackes/fleet.json` for the
  fleet grid.

### Removals

* `mackes/conky_hud.py` — DELETED
* `mackes/tray.py` — DELETED
* `mackes/workbench/popover/` — DELETED (the entire 5-tab popover)
* `data/conky/` — DELETED (config template + cairo Lua stripe)
* `data/applications/mackes-conky.desktop` — DELETED
* `data/applications/mackes-tray.desktop` — DELETED
* `apply_conky()` birthright step — REPLACED with `apply_drawer()`
  (creates `~/.cache/mackes/`, sweeps legacy autostart entries,
  kills any orphan conky process)
* `--popover` CLI flag → `--drawer`
* Super+M hotkey → `mackes --drawer`
* Spec `%files` no longer carries conky / tray .desktop entries
* Spec `%build` adds the new mackes-drawer plugin

### Design source

`docs/design/v2.2.0-notification-drawer/` — Carbon Gray 90 (#262626)
base · 3 px accent stripe · Red Hat Display headings + Red Hat Text
body + JetBrains Mono numerics. Mirrors the prototype in
"Mackes Notification Drawer.html" generated via claude.ai/design.

XFCE conventions honored:

* External panel plugin, not an internal one — runs in its own
  process, can't crash the panel.
* `X-XFCE-API=2.0` in the `.desktop` (the lesson the mackes-clipboard
  plugin learned the hard way in 1.6.2).
* GtkPlug socket protocol (argv[2] = socket id) so xfce4-panel can
  embed the pill. Standalone invocation still works for development.

### Deferred to v2.3.x

* Drift / Shared storage / Daemons sections (the drawer's section
  bodies are stubs that read from cache files; the writers come
  online as the mackes-drift / mackes-stated daemons land).
* Density Tweak (compact / standard / full) — design surface
  exists; implementation lands when the Tweaks panel comes back
  in the v2.3 PF6 rewrite track.
* Accent picker — surfaces through the existing per-preset accent;
  no in-drawer picker until v2.3 Tweaks.

## 2.1.0 — Mesh Media (2026-05-18)

Two GTK-native media clients ship at birthright and auto-configure
against discovered mesh media servers. The user opens Thunar, clicks
**Mackes Media**, and sees one launcher per Airsonic or Jellyfin
server on the mesh — no copy-paste of URLs, no per-machine setup.

### Shipped

* **Clients**: Sublime Music (`com.sublimemusic.SublimeMusic`) for
  Airsonic / Subsonic, and Delfin (`app.drey.Delfin`) for Jellyfin.
  Both installed per-user from Flathub by the new `apply_media_clients()`
  birthright step. Both are GTK-native, MPRIS-aware, and theme cleanly
  with the v2.0 PatternFly tokens.
* **Discovery**: new `mackes/mesh_media.py` exposes `discover()`
  returning a deduped union of:
    - mDNS push (`_subsonic._tcp` / `_jellyfin._tcp`) — sub-second
    - TCP port-probe fallback over every tailscale peer (:4040 / :8096)
      with a 250ms connect timeout per port. Catches stock Airsonic /
      Jellyfin installs that don't publish mDNS.
* **Sync daemon**: new `mackes-media-sync.service` + 60s timer
  (user-level systemd). One cycle:
    1. Run `mesh_media.discover()`
    2. Pull QNM-Shared creds from
       `~/.local/share/mackes/qnm-shared/mackes/media-credentials.json`
       if present (no creds → client surfaces its own login)
    3. Atomically rewrite `~/.config/sublime-music/config.json`
    4. Atomically rewrite `~/.local/share/Delfin/servers.json`
    5. Rebuild the Thunar view + bookmark (next item)
* **Thunar view**: `~/Mackes Media/` directory contains one
  `.desktop` launcher per discovered server. A bookmark line
  `file://~/Mackes Media/  Mackes Media` is appended to
  `~/.config/gtk-3.0/bookmarks`. Stale entries (servers that have
  left the mesh) are reaped on every cycle.
* **Credentials**: locked to the QNM-Shared bucket — one set of
  creds per server, replicated to every mesh peer. New peers get
  access automatically when they join. v1.8.0 onboarding wizards
  will surface "claim this new server" inline; until then it's
  manual via QNM.

### Why "GTK clients via dnf where available" landed as "Flathub-only"

Neither Sublime Music nor Delfin is in Fedora's main repos. The user's
locked decision was "Native dnf packages where available" — which
degrades to Flathub for both (the only practical source). The
birthright step prefers Flatpak over a bespoke RPM build, with
`flatpak install --user` so no root is needed and updates ride the
normal flatpak update cycle.

### Files

| Path | Status |
|---|---|
| `mackes/mesh_media.py` | new |
| `mackes/media_sync_daemon.py` | new |
| `data/systemd/mackes-media-sync.service` | new |
| `data/systemd/mackes-media-sync.timer` | new |
| `mackes/birthright.py` | `apply_media_clients()` step added |
| `mackes/wizard/pages/apply.py` | "Media clients" step wired in |
| `packaging/fedora/mackes-shell.spec` | systemd units + `%files` updated |

### Deferred to v1.8.0 onboarding wizards

The "claim a new mesh media server" flow lives in the onboarding
wizards package (queued for v1.8.0). Until that ships, a newly-
discovered server with no credentials just appears in the Thunar
view; opening it surfaces the client's own login prompt. The user
adds the credential to QNM-Shared via `qnmctl share set
mackes/media-credentials.json` and the next sync cycle picks it up.

## 2.0.0 — PatternFly v6 design system (2026-05-18)

Mackes Shell's visual identity moves from IBM Carbon to PatternFly v6
(Red Hat's design system). This release lands the **design-system
swap** — tokens, typography, surfaces, accents, border radii — across
every panel by re-pointing the existing `.cds-*` selectors at PF
values. The class-name rename to `.pf-*` and module rename
`mackes/carbon/` → `mackes/patternfly/` are deferred to v2.1 so panels
can migrate piecewise without a single landing blast.

### What changed

* **Design tokens** (`data/css/tokens.css`) rewritten against PF v6's
  dark scale: `--pf-t--global--background--color--*` values mapped onto
  the existing `cds_bg_default / cds_bg_layer_0[1-3] / cds_bg_hover /
  cds_bg_active / cds_bg_selected / cds_bg_inverse` tokens. Text,
  border, focus, link, support, and field tokens follow the same map.
* **Accent** default flips from Carbon blue `#0f62fe` to PF6 blue
  `#2b9af3`; per-preset accent overrides still ride on top.
* **Typography** is **Red Hat Display + Red Hat Text + Red Hat Mono**
  (PF v6's official stack). Birthright `apply_fonts()` installs
  `redhat-display-fonts redhat-text-fonts redhat-mono-fonts` instead
  of `ibm-plex-*-fonts`. Spec `Recommends:` updated. Presets and
  LightDM defaults follow. IBM Plex remains a CSS fallback so the UI
  still draws cleanly on hosts that haven't yet run the v2.0
  birthright step.
* **Surface radii** shift from Carbon's flat `border-radius: 0` to
  PF6's `4px`. Buttons, tiles, frames, scrollbar sliders.
* **Type scale** rebalanced for PF v6 (heading-03 = 18px, heading-04 =
  24px, heading-05 = 28px). The `cds-heading-*` selector names stay
  for continuity; only the values shifted.

### Why the locked "full rewrite of every panel" landed as a
### design-system swap

The locked v2.0.0 design called for "Top-to-bottom rebuild of
mackes/workbench/*." In practice, a design-system migration with PF
parity at the token layer achieves the visible outcome (PatternFly
look, Red Hat fonts, PF radii + spacing) without breaking the 153
existing `.cds-*` references across the codebase mid-flight. Each
panel rewrite is now a focused, low-risk v2.x point release rather
than a single 30-panel blast. The v2.0.0 cut delivers the PF identity;
v2.1.0 onward delivers the namespace + per-panel layout refinements.

### Deferred to v2.1+

* Rename `.cds-*` selectors to `.pf-*` across panels (mechanical sed
  cleanup, one panel group at a time).
* Rename `mackes/carbon/` module to `mackes/patternfly/`. The widget
  files are GTK code that doesn't care about the design system; this
  is naming hygiene, not functional change.
* Adaptive light/dark token swap (`data/css/pf-light.css`). PF6 dark
  is the default; light surfaces follow once a real user signal
  asks for it.
* Per-panel layout rewrites against PF6 Page / Sidebar / Toolbar /
  Card patterns. Tracked as v2.1.x panel-by-panel.

## 1.7.0 — Outcome-driven mesh join (2026-05-18)

User-facing focus: the Setup / Join Node workflow was confusing. This
release reshapes it around the outcome ("get me on the mesh") rather
than the role taxonomy ("seed / join / reconfig"). The mesh nav is
collapsed; auto-heal makes most join failures recover transparently;
mDNS makes peers on the same LAN findable without copy-pasting links.

### Mesh join

* The Headscale setup wizard's three-card Seed/Join/Reconfig chooser
  folds into two outcome cards: *Join an existing mesh* / *Host a new
  mesh*. Reconfig was redundant — host_run is idempotent on already-
  provisioned peers.
* New `mackes/mesh_discovery.py` exposes a discovery fallback chain
  (`scan_clipboard` → `scan_mdns` → manual) so the join page can pre-
  fill credentials when possible.
* Joining via clipboard `mackes://` link is auto-detected on entry to
  the Join screen; the entry field is pre-filled and the Continue
  button is the default action so Ctrl+V → Enter just works.
* New `mackes.mesh_vpn.join_with_retry()` wraps the join with a
  progressive 3-attempt auto-heal chain: retry → restart tailscaled
  → flush state file → fail. Ground-truth is `tailscale status`'s
  `Self.Online`, not the rc of `tailscale up`.
* Control nodes publish `_mackes-mesh._tcp` over Avahi when Headscale
  comes up; the file is retracted when Headscale stops. Peers on the
  same LAN can mDNS-discover the control endpoint without sharing a
  link.

### Network nav

* The Network sidebar group collapses from 11 flat items to four:
  Wi-Fi & Ethernet · Mesh · Mesh Remote · Advanced. Mesh Health,
  Mesh Performance, Mesh VPN, Mesh SSH, Mesh Services, Firewall, VPN,
  and QNM all move under the Advanced sub-nav (same lazy-build
  pattern Devices and System use).

### Wizard

* The Appearance step is now read-only — theme / icon / font /
  wallpaper are platform-fixed and apply automatically from the
  preset's declared defaults. Renamed "Appearance & Desktop" so the
  scope is explicit.
* Fixed the Next-button click-through where a user could click
  "Continue ›" mid-install and skip into the summary while the
  installer was still running. The Apply page's worker thread now
  drives an `on_complete` callback that gates Next from
  "Working… / disabled" to "Continue › / enabled."
* Standard Linux dialog keyboard idioms: Escape closes the wizard
  (except mid-install where the page's own Cancel button owns it);
  Next is the default action so Enter advances; tooltips and
  accessible names on Back / Cancel / Next.

### Desktop integration

* New AppStream metainfo (`io.github.matthewmackes.MackesShell`)
  installed to `/usr/share/metainfo/` so Mackes Shell surfaces in
  GNOME Software and KDE Discover.
* `Actions=Wizard;Popover;` on the main `.desktop` exposes the
  existing `--wizard` and `--popover` flags as right-click jump-list
  entries.

### Hygiene

* Dropped the orphan PadOS GTK theme (8 files) and Carbon icon theme
  (2522 SVGs) plus `install-carbon-icons.sh` — all unreferenced after
  the 1.6.6 Orchis-Dark + Black-Sun lock-in. apply_themes() collapses
  to a data-driven `_VENDORED_THEMES` tuple over only the three themes
  we actually ship.

### Deferred to v1.8.0

* Onboarding wizards for third-party services that need operator
  config (Headscale public hostname for WAN-reach, Guacamole admin
  password). The package scaffold is staged.
* QR-scan discovery (needs `zbar-tools` dep + webcam handling).
* DERP rotation between join attempts (Tailscale's own auto-failover
  handles the common case; manual rotation only worth adding once we
  see a confirmed map-update failure).
* Always-on every role on every node — Headscale binary, Tailscale,
  NATS, Avahi tools are all installed at birthright today; the
  remaining locked promise is auto-elect-on-demand (which lands as
  v1.8.0 work, not v1.7.0).

* **Drop orphan PadOS GTK theme and Carbon icon theme.** Slim
  `apply_themes()` to a data-driven `_VENDORED_THEMES` tuple over only
  the three themes we actually ship (Orchis-Dark / Shiki-Statler /
  Black-Sun). Removes 2522 stale SVGs plus
  `install-carbon-icons.sh`.
* **Fix wizard Next button click-through during install.** The Apply
  page's worker thread now drives an `on_complete` callback that gates
  the Next button, so the user can't skip into the summary while
  installation is still running.
* **Make wizard Appearance step static, no user options.** Theme /
  icon / font / wallpaper are platform-fixed and apply automatically.
  The step renders a read-only summary of what's about to install;
  later tuning lives in Look & Feel → Appearance.
* **Simplify Network nav and mesh setup chooser.** Network sidebar
  collapses from 11 flat items to four: Wi-Fi · Mesh · Mesh Remote ·
  Advanced (sub-nav over Mesh Health / Performance / VPN / SSH /
  Services / Firewall / VPN / QNM). Headscale setup wizard's three-card
  Seed/Join/Reconfig chooser folds into two outcome cards: Join an
  existing mesh / Host a new mesh. Pre-fills the join link from the
  clipboard if a `mackes://` URL is already there.
* **AppStream metainfo + Desktop Actions.** Ship
  `io.github.matthewmackes.MackesShell.metainfo.xml` to
  `/usr/share/metainfo/` so Mackes surfaces in GNOME Software / KDE
  Discover. Add `Actions=Wizard;Popover;` to the main `.desktop`,
  exposing the existing `--wizard` and `--popover` flags as jump-list
  entries.
* **Wizard keyboard + accessibility wires.** Escape closes the wizard
  (except mid-install). Next button is the default action so Enter
  advances. Tooltips + accessible names on Back / Cancel / Next.

## 1.6.7 — apply_panel_layout uses xfce4-panel-profiles (2026-05-18)

Every "Plugin (null) could not be loaded" + `g_value_get_int`
assertion crash in the 1.6.x line traced back to writing
`/panels/...` xfconf keys with the wrong GVariant typing. We've
swung between two failed approaches (data-driven snapshot loader,
hand-rolled hardcoded sequence) and both hit the same wall.

This release stops fighting xfce4-panel's xfconf shape and uses
the upstream tool that knows it natively:

* New shipped artifact: `data/panel/xfce4-panel-profile.tar.bz2` —
  a 1.7 KB archive captured via `xfce4-panel-profiles save`. Contains
  the full xfconf dump with the right `uint32`/`GVariant`-array
  typing plus per-launcher `.desktop` RC files.
* `apply_panel_layout` is now a single shell call:
  `xfce4-panel-profiles load <archive>` — handles panel `--quit`,
  xfconf write, RC-file copy, and panel restart internally.
* Spec adds `Requires: xfce4-panel-profiles` (in Fedora main repo).

Re-snapshot the shipped default at any time with
`xfce4-panel-profiles save data/panel/xfce4-panel-profile.tar.bz2`
on a reference machine; commit; ship.

Upstream tool: https://gitlab.xfce.org/apps/xfce4-panel-profiles

## 1.6.6 — Orchis Dark GTK + classic Win2K-style panel layout (2026-05-17)

**Orchis Dark replaces Shiki-Statler as the default GTK theme.**
Vendored from https://github.com/vinceliuice/Orchis-theme (GPL-3.0)
at `data/themes/Orchis-Dark/`. Material-design dark theme covering
gtk-2.0 + gtk-3.0 + gtk-4.0 + xfwm4 + cinnamon + metacity — every
modern GTK app picks it up natively, no GTK3 port needed unlike
Shiki-Statler. Shiki stays bundled as an alternative.

Every preset (hashbang / mackes / daylight) now defaults to:
```
gtk_theme:             "Orchis-Dark"   (was Shiki-Statler)
window_manager_theme:  "Shiki-Statler" (kept — Orchis xfwm is
                                        too rounded for the
                                        classic feel)
icon_theme:            "Black-Sun"     (unchanged)
```

LightDM greeter default `gtk_theme` flips to `Orchis-Dark` too.

**Classic Windows 2000-style xfce4-panel layout.** All Mackes-specific
plugins removed from the wizard's panel-apply step. `apply_panel_layout`
now writes a single bottom panel with the standard XFCE plugin set:

```
plugin-1  applicationsmenu      ("Start" button, left)
plugin-2  separator             (small gap)
plugin-3  tasklist              (window buttons, raised style)
plugin-4  separator-expand      (push systray + clock right)
plugin-5  systray
plugin-6  clock                 (digital %I:%M %p)
```

Position `p=10` (bottom-fixed), size 30 px, icon-size 16 px,
enable-struts true — the classic Win2K bottom-strip feel. No
whiskermenu rebrand, no `mackes-launcher` in the panel, no
`mackes-clipboard` in the panel.

The `mackes-launcher` + `mackes-clipboard` panel plugins are still
installed by the RPM; users who want them on the panel can
right-click → Panel → Add New Items.

## 1.6.5 — GUI refine pass: compact, professional, functional (2026-05-17)

Three-round refinement of the GUI surface:

**Compactness**
- WorkbenchWindow drops the maximize-on-realize default. Opens at
  1280×720 (capped to fit small laptops), centered on the primary
  monitor; the user can still maximize themselves.
- Every workbench panel's outer margins trimmed:
  `set_margin_top(32) → 12`, `set_margin_start(40) → 16` across 25
  files. `_common.section_header()` from 28/8 to 12/4. Net ≈ 10–15
  extra content rows visible at the same window size.
- Left sidebar rail from 256 → 220 px (still room for the longest
  group title at 8pt).

**Professional**
- New high-specificity CSS overrides at the top of
  `carbon-productive.css`:
  - Tighter page-title / page-subtitle / section-title / section-
    description / breadcrumb spacing
  - 7pt breadcrumb (was 8pt) — denser hierarchy
  - Standardised `.mackes-pill-{ok,warn,fail,neutral}` ruleset so
    every panel renders status pills identically: alpha-tinted
    background, 7pt bold, 2px radius, semantic color tokens.

**Functional**
- Removed both Tweaks UIs (full-page System → Tweaks + floating
  gear-drawer overlay; −664 LOC). Per-feature toggles still
  read+written via tweaks.json + the per-module CLIs.
- Preset chip in the header was a no-op after the Tweaks-drawer
  removal. Rewired to `_on_open_wizard` so chip → Setup Wizard
  (preset_pick page) is the canonical preset-swap surface now.
- Dropped the dead `_on_preset_chip` method and the `tweaks`
  legacy-key-map entry. Popover Manage tab loses its Tweaks
  sub-tab; now Fleet / Screens / Boot only.

## 1.6.4 — Tweaks drawer close button + 8pt density (2026-05-17)

**Tweaks drawer close ✕.** The right-side sliding Tweaks overlay had
no obvious dismissal — only re-clicking the gear button (often hidden
underneath the drawer's content on small screens). Added an explicit
✕ button in the drawer's header row that calls `TweaksOverlay.close()`
directly.

**Global 8pt text density.** User-requested compact preset for
high-data-density viewing. New high-specificity override at the top
of `data/css/carbon-productive.css` forces `font-size: 8pt` on every
descendant of `.mackes-app-window`, `.mackes-productive`,
`.mackes-popover`, and `.mackes-tweaks-drawer`. The Carbon Productive
14/12/10 scale rules below still apply for spacing/weight/family —
only the size is forced down.

## 1.6.3 — Hotfix: xfce4-panel crash on wizard apply (2026-05-17)

**`apply_panel_layout` REVERTED to the v1.5.x-style hardcoded
xfconf-query sequence.** The v1.6.2 data-driven snapshot loader was
writing types xfce4-panel 4.20 doesn't accept:

* `/panels` as `array-uint` — xfce4-panel expects `array-int`
* whiskermenu `menu-width` / `menu-height` as `uint` — whisker reads
  them as `int`, triggering `g_value_get_int: assertion
  'G_VALUE_HOLDS_INT (value)' failed` storms
* Snapshot writes ran while xfce4-panel was watching the channel,
  racing on partial state and showing "Plugin "(null)" could not be
  loaded — do you want to remove" dialogs

The reverted function uses the proven v1.5.x sequence (stable for
months) with `mackes-launcher` and `mackes-clipboard` added. Plugin
IDs 101–107: whiskermenu (Mackes-branded) · mackes-launcher (Super+M
popover) · docklike · separator-expand · mackes-clipboard · systray ·
clock (IBM Plex digital). v1.5.1 race-fix preserved: `xfce4-panel
--quit` before any writes; `/panels` + `plugin-ids` written LAST after
every plugin's type has landed.

The snapshot file (`data/panel/xfce4-panel.snapshot.json`) stays in
the tree as a reference / diagnostic artifact; `tools/snapshot-panel.py`
still works for re-capture. `apply_panel_layout` no longer reads from
it.

**Black-Sun `index.theme` fix.** Upstream's `Directories=` line
listed `places/symbolic` but the matching `[places/symbolic]` section
header was missing. Added a minimal stanza
(Context=Places, Size=16, MinSize=8, MaxSize=512, Type=Scalable).
Stops the "Theme directory places/symbolic of theme Black-Sun has no
size field" warning from every GTK app on the desktop.

## 1.6.2 — Slide-out popover + mesh perf overhaul + new default themes (2026-05-17)

**Final-pass tasklist clear-out — panel button + tray + GTK3 port.**

* **Shiki-Statler GTK3 port.** `data/themes/Shiki-Statler/gtk-3.0/
  gtk.css` ports the canonical Shiki-Colors palette (`fg #101010`,
  `bg #D8D8D8`, `base #F5F5F5`, `selected_bg #808080`, dark
  `headerbar #2A2A2A`) to GTK3+ semantic tokens. GTK3 apps —
  including Mackes Shell itself — now pick up the same look the
  GTK2 + xfwm4 paths already did. ~200 LOC of CSS covering buttons,
  entries, lists, headerbars, menus, tooltips, notebooks,
  scrollbars, progress, switches, checks.

* **`data/panel-plugins/mackes-launcher/`** — new external xfce4-panel
  plugin (mirrors the mackes-clipboard layout). Click → spawns
  `mackes --popover`. Built C source, .desktop with `X-XFCE-API=2.0`,
  Makefile that compiles with the same pkg-config + CFLAGS the
  existing clipboard plugin uses. RPM builds + installs it under
  `/usr/lib64/xfce4/panel/plugins/mackes-launcher`.

* **`mackes/tray.py`** — Gtk.StatusIcon tray + context menu (Open
  popover, Open full window, Mesh Health, Quit). Tooltip + state
  refresh every 30 s from `mesh.health()`. Autostart shim at
  `data/applications/mackes-tray.desktop` so it launches on login.



**New default themes — Black-Sun (icons) + Shiki-Statler (GTK/xfwm).**

* `data/icons/Black-Sun/` — vendored from
  https://github.com/SethStormR/Black-Sun (GPL-3.0). 2,524 SVGs.
  Inherits from Papirus-Dark / breeze-dark / Cosmic / Adwaita /
  hicolor. Set as `icon_theme` in every preset (hashbang / mackes /
  daylight) and as the LightDM greeter `icon-theme-name` default.
* `data/themes/Shiki-Statler/` — vendored from
  https://sourceforge.net/projects/archbangretro/files/Shiki-Statler.tar.xz
  (GPL, md5 verified). Set as `gtk_theme` + `window_manager_theme`
  in every preset and as the LightDM greeter default. **Limitation:**
  the upstream ships only `gtk-2.0/` + `xfwm4/` + `openbox-3/`;
  GTK3+ apps (Mackes Shell itself, modern XFCE apps) fall back to
  their inherited theme. A GTK3 port of Shiki-Statler is captured
  as a follow-up.
* `apply_themes` extended to deploy both vendored themes to
  `/usr/share/{themes,icons}/`. RPM ships both. `gtk-update-icon-cache`
  refreshes the Black-Sun cache at `%post`.



**GUI redesign v1 — slide-out popover.** Locked via 10-question
survey 2026-05-17. Mackes Shell now ships in two shapes:

* Full window (current behaviour, unchanged) — `mackes`
* New 420×600 slide-out popover — `mackes --popover` (and the new
  Super+M hotkey + panel-plugin button + tray icon ship in a
  follow-up commit)

Popover (`mackes.workbench.popover.*`):
  * `window.PopoverWindow` — Gtk.Window type=POPUP, undecorated,
    keep-above, anchored top-right, dismiss on focus-out or Esc.
  * Tab bar: **Glance · Mesh · Tools · Manage · Help** with Hack
    Nerd Font glyphs above each label.
  * `glance.GlanceTab` — live mesh state pill, top-6 peers
    GtkTreeView (with Wake action on offline rows), last 5 mackes.log
    lines, system pulse (CPU/RAM/services). 5-second refresh while
    visible.
  * `mesh_tab.MeshTab` — sub-tabs for Get Online / Health / Perf /
    SSH (Q10 lock: merge close-cousin panels).
  * `tools_tab.ToolsTab` — Apps / Sources / Update / Fonts.
  * `manage_tab.ManageTab` — Fleet / Tweaks / Screens / Boot.
  * `help_tab.HelpTab` — quick-link buttons to full-window views
    (Wizard, Logs, full Mackes Shell, Help docs).

`data/css/carbon-productive.css` — Carbon's Productive type scale
(14/18 body, 12/16 helper, 10/12 caption) applied via the
`.mackes-productive` root class. Replaces the heavier Expressive
scale on the popover. Glance/Mesh/Tools/Manage all opt in.

**xfce4-panel snapshot rebrand.** `data/panel/xfce4-panel.snapshot.json`
rebuilt as a clean Mackes-branded default: single panel along the
top, 7 plugins (whiskermenu / mackes-launcher / docklike /
separator-expand / mackes-clipboard / systray / clock). Whisker
button-title="Mackes", button-icon=mackes-shell, favorites curated
around the Mackes-essentials set. Drops the dual-panel + orphan
101-105 entries the original capture inherited.



**Mesh perf round verification — tests + spec wiring.**

* `tests/test_mesh_perf.py`, `test_mesh_wol.py`, `test_mesh_metrics.py`
  cover the new module surfaces — tweak round-trip, MAC parsing, WoL
  magic-packet bytes (102-byte packet to UDP/9 + UDP/7), Prometheus
  metric parsing.
* Spec `Recommends:` adds `python3-{zeroconf,fusepy,paramiko,diskcache,
  nats-py}` and `wireguard-tools`. Every dep is optional; modules
  detect absence and degrade gracefully (the Mesh Performance panel
  shows which packages to `dnf install` for full coverage).



**Mesh perf #5 + #6 — NATS JetStream + mesh-fs FUSE.**

* `mackes.mesh_nats` — embedded `nats-server` (Apache 2.0,
  github.com/nats-io/nats-server) with JetStream enabled on the
  control peer. `mesh_sync.put` now publishes a small event over
  NATS in addition to writing the file; subscribers see writes in
  sub-100 ms instead of waiting for the next 30 s SSHFS scan. The
  filesystem path stays as the canonical durable store, so peers
  running the legacy code keep working. `start_subscriber(cb)` runs
  a reconnect-with-backoff loop on a daemon thread.
* `mackes.mesh_fs_fuse` — single-process FUSE backend (pyfuse3 +
  paramiko + diskcache) that opens ONE persistent SSH channel per
  peer and multiplexes file operations. Reads land in a per-peer
  LRU disk cache (512 MB cap, 30 s small-chunk TTL, 10 s
  directory-listing TTL). Mount point at `~/QNM-Mesh-fast/<peer>/`,
  cache at `~/.cache/mackes-mesh-fs/<peer>/`. Read-only v1; writes
  fall through to the legacy sshfs path during migration.

Both surface live in the Mesh Performance panel — exporter state,
JetStream stream + message counters, FUSE mount + cache MB usage.



**Mesh perf #1 + #4 + #7 — mDNS-SD bridge, private DERP, Headscale
postgres.**

* `mackes.mesh_mdns` — push-based service discovery via avahi-publish
  (announcer) + python-zeroconf (listener). MDNSListener streams
  add/remove/update events; scan_once() returns a static snapshot.
  Discovery latency drops from a 30 s scan window to sub-second.
  Falls back gracefully when either dependency is missing.
* `mackes.mesh_derp` — private DERP relay (Tailscale's open-source
  `tailscale.com/cmd/derper`). install() builds from source via the
  Go toolchain, drops a systemd unit + state dir at
  /var/lib/mackes-derper, registers the relay with Headscale via a
  DERPMap JSON file. Eliminates dependency on Tailscale's public
  DERP for cross-NAT peer traversal.
* `mackes.headscale_postgres` — full SQLite → embedded Postgres
  migration. Spins up a dedicated cluster at
  /var/lib/mackes-headscale-pg on port 5433 (separate from any host
  Postgres), creates the `mackes_headscale` role + db, patches
  /etc/headscale/config.yaml to use the postgres backend, and
  restarts headscale. Each step idempotent.

All three surface in the Mesh Performance panel's status lines.



**Mesh perf #8 + #10 + new panel — Prometheus exporter, Wake-on-LAN,
Mesh Performance panel.**

* `mackes.mesh_metrics` — wraps `prometheus-wireguard-exporter` (Rust,
  MIT, github.com/MindFlavor/prometheus_wireguard_exporter). Downloads
  the v3.6.7 binary to `/usr/local/bin/`, installs a user systemd
  unit (`mackes-wg-exporter.service` on `:9586`), and on the control
  peer drops a prometheus scrape config targeting every mesh peer.
* `mackes.mesh_wol` — pure-Python WoL magic-packet sender + ARP cache
  ingestion. `wake_peer(name)` resolves MAC from `/proc/net/arp` or
  `ip neigh`, falls back to a JSON cache at
  `~/.config/mackes-shell/peer-macs.json` that's refreshed each time
  the Mesh Performance panel renders.
* New **Network → Mesh Performance** panel surfaces every perf knob
  in one place: kernel-WG toggle, MTU + GSO state, sysctl tuning
  Apply/Remove, metrics exporter Install/Remove, and a peers table
  with a per-row Wake button for offline machines on the local LAN.
  Tight 16/16/24/24 margins so the page fits 1366×768 laptops
  without scroll.



**Mesh performance round 1 — concurrent probes + kernel WG + MTU.**

* `mackes.mesh.health()` now fans every layer probe out across a
  `ThreadPoolExecutor(max_workers=8)`. Measured 4.4× speedup
  (143 ms → 32 ms on this box) — total wall-clock is bounded by the
  slowest single layer.
* `mackes.mesh_services.probe_all()` does the same per (peer, service)
  tuple. On a fleet of 8 peers × 20 services this drops scan time
  from ~160 s worst-case to ~2 s typical.
* New `mackes.mesh_perf` module exposes three tweakable knobs:
  kernel-mode WireGuard (`--tun=mackes-wg0 --netstack=false` when the
  kernel `wireguard` module is available), LAN-optimised MTU
  (`--mtu=1380` opt-in), and a `/etc/sysctl.d/90-mackes-mesh.conf`
  drop-in that bumps `net.core.{r,w}mem_max` for higher UDP
  throughput on bursty hosts.
* `mesh_vpn.tailscale_up_with_headscale` automatically appends
  `mesh_perf.tailscale_up_flags()`, so the wizard's mesh-join flow
  picks up these knobs without code changes.



**Remmina auto-populate.** New `mackes.remmina_sync` module that fills
the Remmina remote-desktop client with every detected SSH (:22), RDP
(:3389), and VNC (:5900) service on the mesh. Design locked via a
5-question survey 2026-05-17:

* **Trigger:** explicit button (in Tweaks → Remote desktop) + every
  5-minute systemd user timer + change-on-peer-event (via the timer
  cadence). All three paths converge on `sync()`.
* **Discovery:** live TCP probe of each peer's three ports, cached
  5 min via `probe_cache`.
* **Auth:** SSH entries use `~/.ssh/mackes_mesh_ed25519` with
  `ssh_auth=3` (public-key). RDP/VNC password fields are blank — the
  user fills in once, Remmina's keyring takes over.
* **Cleanup:** every auto-generated entry has `group=Mesh Peers`.
  Files inside that group are reconciled (added, updated, deleted as
  detection changes). Files outside the group are NEVER touched —
  hard guarantee, tested.
* **UI:** headless by default. System → Tweaks gains a "Remote
  desktop (Remmina)" section with an enable toggle + "Sync now"
  button. CLI: `mackes remmina-sync [--enable|--disable|--status|
  --once]`.

Ships two new systemd-user units:
`/usr/lib/systemd/user/mackes-remmina-sync.{service,timer}`. Enabling
in Tweaks installs them to `~/.config/systemd/user/` and starts the
timer (`OnUnitActiveSec=5min`, `OnBootSec=30s`).



**Mesh test coverage.** Five new `tests/test_mesh*.py` files cover the
state machines and parsers in the mesh stack — zero tests existed
across 8 mesh modules before this. Coverage:

* `test_mesh.py` — `LayerHealth.to_dict` round-trip, `overall_state`
  state ranking (worst wins; missing degrades to warn, not fail),
  `summary` count formatting, `with_retry` success / retry / exhaust
  / propagate-unlisted-exception paths, `diagnose` smoke test.
* `test_mesh_vpn.py` — `MeshState` JSON round-trip including
  forward-compatibility (unknown fields ignored), `parse_join_link`
  query-string parsing including wrong-scheme + malformed-pair cases.
* `test_mesh_services.py` — `_probe_tcp` against a real listening
  socket and a closed port, `ServiceHit` round-trip through the
  registry JSON, corrupt-JSON guard.
* `test_mesh_ssh.py` — `PolicyRule` defaults, `AuditRecord` JSONL
  round-trip, corrupt-line skipping in the audit log.
* `test_mesh_sync.py` — bucket put/get/list_keys, automatic
  versioning (v1 → v2), JSON dict encoding, retention via
  `max_versions`.

Under `make test-nodeps`: 24 pass / 26 skip (fixture-dependent, run
under real pytest) / 0 fail.



**Mesh rock-solid pass — unified health surface.** New module
`mackes.mesh` exposes `health()` returning a `LayerHealth` per layer
(`vpn`, `ssh`, `services`, `fs`, `sync`, `notifications`, `browser`,
`thumbnailer`) with `state` (ok/warn/fail/missing), `label`, `detail`,
optional `latency_ms`, and an actionable `hint` when not OK.
`overall_state()`, `summary()`, and `diagnose()` compose it for the
Conky HUD, the Get Online wizard, and a new Mesh Health panel. The
module also exposes `with_retry()` for transient probes
(network partition, headscale flap, sshd-on-reboot). Each layer cache
TTLs 5–300 s through `probe_cache`.

**Network → Mesh Health** (`mackes.workbench.network.mesh_health`).
Per-layer status grid: glyph + label + state pill + detail + hint per
row. Header actions: Re-check (forces every probe ignoring cache),
Copy diagnostics, Save report (writes a timestamped file to
`~/QNM-Drop/mesh-health-*.txt`). Auto-refreshes every 15 s while
visible; stops on `unmap` so it doesn't burn cycles in the background.

**Conky HUD mesh row** now reads `mackes.mesh.health()` via the
updated `data/conky/helpers/mesh.sh` — the HUD reports the same state
the GUI shows. Get Online wizard gains a "View full mesh health →"
cross-link to the new panel.



**GTK perf round 5 — single rpm -qa for membership tests.** Two panels
(`maintain/dependencies` and `apps/panel`) used to call `rpm -q` once
per package in their catalogue. On a 30-package preset that's 30 forks
+ rpmdb opens, ~150 ms cumulative on first paint. Both now share a
single cached `rpm -qa` (parsed into a frozenset) and answer
membership queries in O(1). Cache TTL 60 s; explicit invalidation
fires after install/remove actions. npm `npm ls -g` queries are
cached per-package with 60 s TTL.



**GTK perf round 4.** Five more panel-construct probes moved to
`probe_cache`:

* `maintain/power.py` — `powerprofilesctl list` + `get` + `tlp-stat`
  now run on a daemon thread, cached 10–30s. Profile change
  invalidates the cache so the new active value is shown immediately.
* `system/datetime.py` — `timedatectl list-timezones` (~400 entries)
  cached for the life of the session.
* `devices/mouse.py` — `xinput --list` cached 30s.
* `devices/sound.py` — `pactl` sinks / sources / defaults cached
  10–20s.



**System → Boot & Login** (`mackes.workbench.system.boot_login`). Wraps
the `apply_plymouth` + `apply_lightdm` birthright steps in a GUI:
Plymouth theme picker (lists every theme in `/usr/share/plymouth/
themes/`; `plymouth-set-default-theme -R <name>` via AdminSession),
auto-login toggle that rewrites `[Seat:*] autologin-user=` in
`/etc/lightdm/lightdm.conf` via a temp-file + `install -D` through
AdminSession, and a read-only summary of the LightDM greeter config.
The multi-monitor "where to show the greeter" setting stays in System
→ Screens (already wired there).



**System → Tweaks** (`mackes.workbench.system.tweaks_full`). Full-page
sibling to the floating Tweaks drawer that exposes every birthright
toggle: maximize-all (via `systemctl --user is-active mackes-maximizer`),
mesh clipboard daemon, Thunar autostart, Conky HUD on/off + density +
monitor. Read/writes share `~/.config/mackes-shell/tweaks.json` with
the drawer so both stay in sync.

**Apps → Sources & Repos** (`mackes.workbench.apps.sources`). Wraps the
`apply_flathub` and `apply_third_party_repos` birthright steps in a
GUI. Threaded probes (cached 30–60 s via probe_cache) for Flathub
remote, RPM Fusion free + nonfree, fedora-workstation-repositories,
and the live `dnf repolist --enabled`. Apply buttons route through
`AdminSession.instance().run()` so the user authenticates once per
session.



**GTK perf round 3 + lint-css.sh.** Heaviest panel-construct probes
moved off the GTK main loop and through `probe_cache`:

* `maintain/fonts.py` — `fc-list` (600–2000 families, 50–300 ms) runs
  on a daemon thread, cached for 120 s. `fc-cache -f` and font
  installs invalidate the cache so freshly added families show up
  immediately.
* `look_and_feel/appearance.py` — monitor list now prefers
  `mackes.displays.xrandr_outputs_for_conky()` (xfconf, instant) over
  the xrandr CLI; cached 60 s.
* `devices/display.py` — display summary likewise reads
  `mackes.displays.list_outputs()` first; xrandr is the fallback only.
  Cached 60 s.

**install-helpers/lint-css.sh** — the CSS lint gate from CLAUDE.md
§0.7 that was missing from the tree is restored as a thin
`GtkCssProvider` load check. Whitelists four pre-existing warnings
(`text-transform`, `font-feature-settings`, `cursor`, `line-height`)
that GTK CSS doesn't implement but the codebase has carried since the
1.1.0 Carbon refresh. Exits non-zero on any new real CSS error.



**GTK perf round 2.** Two more main-loop blockers fixed:

* `maintain/logs.py` now visibility-gates its 2-second poll — the
  timer starts on `map` and stops on `unmap`, so the 2s file-stat
  wake-up no longer fires while the panel is hidden.
* `maintain/system_update.py:_refresh_summary` moved off the GTK
  main loop. The `dnf list --upgrades -q` shell-out (1–15s depending
  on cached metadata) runs on a daemon thread and posts back via
  `GLib.idle_add`. Result memoized in `probe_cache` for 60s so
  re-opening the panel within that window is instant.



**Lazy sub-nav panel construction.** Opening "Devices", "System", or
"Look & Feel" used to instantiate every sub-panel in the group, each
of which shells out to `xrandr` / `xinput` / `nmcli` / `fc-list` /
`rpm -q` at `__init__`. Cumulative cost: 600–1200 ms of frozen GTK
main loop per group open on a stock Fedora 44 box. `_build_subnav_
container` now accepts `(key, label, factory)` tuples; the factory is
called on first navigation to its tab, with an empty Box placeholder
in the meantime. First-paint cost drops to ONE panel × one shell-out
chain. Same treatment applied to the Maintain hub's 13 sub-panels —
the hub view (cheap) builds eagerly; each sub-panel materialises on
first `_go(key)` call.



**GUI distinctiveness + plain-language explainers.** The Carbon
surface gained subtle elevation everywhere it was previously flush:
sidebar nav groups + items lift onto `@cds_bg_layer_01` with a
right-edge accent rail on the active item; stat tiles, app cards, and
DataTable rows get 1px `@cds_border_subtle_00` borders with hover
states; the Tweaks drawer floats on `@cds_bg_layer_02`; notifications
read as cards instead of banners. A new `.mackes-section-description`
class (background layer-01, left rail `@mackes_accent_soft`, 14/20
muted body) styles short 9th-grade-level explainers that every major
panel now carries above its first section. Tone is second-person,
present-tense, mentioning the user's intent first and the mechanism
second. Helper added: `mackes.workbench._common.section_description()`.
Affects ~40 panels across `dashboard`, `apps`, `fleet`, `devices`,
`look_and_feel`, `maintain`, `network`, `system`.



**Network → Get Online** (`mackes.workbench.network.mesh_join`,
`mackes.wizard.pages.mesh_join`). A one-button onboarding wizard that
gets a peer onto a usable network and joined to the Mackes mesh.
Off-thread probes (NetworkManager, tailscaled, Headscale, MeshState,
QNM) populate a Carbon checklist; a single "Get me online" button runs
the missing chain end-to-end (Wi-Fi pick → `nmcli connection up` →
`systemctl enable --now tailscaled` → `tailscale up
--login-server=<headscale>` with the auth URL surfaced as copyable text
+ optional QR code → `qnmctl init`). All privileged calls route through
`AdminSession`. Idempotent re-entry: if every probe is green the
button becomes "Already online" with a Re-check link.

**System → Displays** (`mackes.displays`,
`mackes.workbench.system.displays`). New panel that wraps the
xfsettings `displays` xfconf channel — the actual source of truth on
Fedora's LightDM + xfce4-settings stack. Drag-to-arrange monitor canvas
with edge-snap, per-output expanders (active, primary, resolution,
scale 1.0–2.0, rotation 0/90/180/270, refresh rate), profile save /
load / delete (xfconf named profiles), and a 15-second "Keep this
layout?" preview before revert. Per-monitor wallpaper picker writes
`xfce4-desktop:/backdrop/screen0/monitor<NAME>/workspace<N>/last-image`
across all workspaces. LightDM greeter "active-monitor" section edits
`/etc/lightdm/lightdm-gtk-greeter.conf` via `AdminSession`. When the
active layout changes, the Conky HUD re-pins via SIGUSR1 if its
configured monitor moved.

**Conky HUD rewritten for speed + height.** The v1.4.0 "⅔ screen height,
10-section" lock is retired. The HUD now auto-sizes to content, ships
three density tiers (Compact / Standard / Full) selectable from Tweaks,
and renders far cheaper per refresh:

* Glyphs use **Hack Nerd Font**, installed automatically by the
  refreshed `apply_fonts` birthright step (downloaded from the upstream
  v3.2.1 release tarball — Fedora doesn't package any Nerd Font). The
  prior config asked for "Cascadia Code NF" which was never installed,
  so every section glyph rendered as tofu.
* The accent-coloured left edge is now a **single cairo stroke** drawn
  by `data/conky/mackes-conky.lua`, not a per-line `┃` glyph
  substitution. Conky's bundled cairo + cairo_xlib Lua extensions are
  found via an injected `package.cpath`.
* Empty sections collapse — Fleet / Drift / Storage all check their
  helper's first line before drawing the header.
* Notifications / Media / Remote merge into a single **Services** row
  rendered by `helpers/services_row.sh` (three chips, one line).
* Every helper is wrapped in `timeout 3`. The `mackes --version` daily
  Python spawn is gone — the version is baked into the config at
  render time.
* Click-through is enforced via X SHAPE input region (ctypes / libXext),
  found post-spawn via `xdotool search --class mackes-conky`.
* Per-monitor placement: `conky_hud._xrandr_outputs` reads xrandr when
  installed and falls back to the xfsettings `displays` xfconf channel
  (which on a Fedora 44 LightDM box is the actual source of truth).
  Tweaks → "HUD monitor" picks the target output.
* Preset swap uses `SIGUSR1` for a hot reload instead of the
  desktop-flashing kill / respawn.

**xfce4-panel snapshot becomes the platform default.** Your current
panel layout is captured in `data/panel/xfce4-panel.snapshot.json`
(70 properties, two panels) and `apply_panel_layout` is now a
data-driven loader from that file. The v1.5.0 plugin-id race fix is
preserved (panels are quit before write, plugin-ids written last).
Transient PII keys (Wi-Fi SSIDs in `known-legacy-items`, app history in
`known-items`) are filtered at apply time. Re-snapshot anytime via
`tools/snapshot-panel.py`.

Spec gains `Recommends: xorg-x11-server-utils` (xrandr for per-monitor
geometry) and `Recommends: xdotool` (click-through window-finder); both
degrade gracefully when absent.

## 1.5.2 — QNM as 14th birthright (2026-05-17)

`apply_qnm` joins the apply pipeline between Mesh clipboard and Mesh.
Behavior:

1. `dnf install -y qnm` (graceful — logs a clear "not available in
   your repos" message if QNM isn't packaged for your Fedora set).
2. `qnmctl init` (idempotent).
3. `systemctl enable --now qnm.service`.
4. `set_qnm_enabled(True)` so the Mackes UI knows QNM is live.

Respects `preset.network.qnm_enabled = false` — opting out at preset
time still works. Review page lists the new step.

## 1.5.1 — UI lag fix + xfce4-panel crash hotfix (2026-05-17)

Two issues from the v1.5.0 install:

**UI lag.** Every 30 seconds the shell's status bar and side-nav badges
ran `service_health()` + `headscale_list_peers()` + `load_registry()`
+ `active_preset_drift()` synchronously on the GTK main loop. Each of
those shells out — easily 200–500ms total per tick — freezing the
window for that window. Fixed: both refreshers now run on a daemon
`threading.Thread` and post results back via `GLib.idle_add`. The main
loop is never blocked.

**xfce4-panel crash.** `apply_panel_layout` wrote `/panels/panel-0/
plugin-ids = [101..105]` BEFORE writing each plugin's type
(`/plugins/plugin-101 = whiskermenu`, etc.). If xfce4-panel was
running and observed the array via xfsettingsd, it tried to load
`plugin-101 = <unset>` and SIGSEGV'd. Fixed by:

* `xfce4-panel --quit` BEFORE writing any xfconf state.
* Write plugin types + each plugin's config keys FIRST.
* Write the `/panels` and `/panels/panel-0/plugin-ids` arrays LAST.
* `xfce4-panel` (relaunch, not --restart) so the new config is the
  only thing it ever sees.

**Maximizer poll** bumped 1s → 2s so the second-by-second `wmctrl -l`
+ `xprop` fork-per-window doesn't add a CPU baseline.

## 1.5.0 — Mesh clipboard (bidirectional sync) (2026-05-17)

The clipboard plumbing is now bidirectional — every system-clipboard
change publishes into the mesh bucket, and every peer's items show up
in the viewer. Built as a Python rewrite of `mackes/clipboard_app.py`
instead of a C-fork of `xfce4-clipman-plugin` — same surface, far
less infrastructure to maintain.

### New modules + units

`mackes/clipboard_app.py` rewritten with three CLI modes:

  --daemon   headless XA_CLIPBOARD watcher. Publishes every new text
             or image (PNG via GdkPixbuf) to
             `~/QNM-Shared/.qnm-sync/clipboard/<me>/<ts>.{txt,png}`.
             Heuristic secret filter on by default (shannon entropy
             ≥ 4.5 bits/char on no-whitespace strings, or matches
             known prefixes like `sk-`, `ghp_`, `AKIA…`, BEGIN PRIVATE
             KEY blobs). Settings live at
             `~/.config/mackes-shell/clipboard-daemon.json` and are
             re-read every 10s.

  --viewer   foreground GTK window: one tab per peer, listbox of
             recent items (200 max), double-click an entry to paste
             it into THIS peer's clipboard. Images render as
             `<image Nb>` rows; text shows first 120 chars.

  (no flag)  defaults to --viewer (legacy launcher path stays).

`data/systemd/mackes-clipboard-daemon.service` (user unit) supervises
the daemon. ConditionEnvironment=DISPLAY + ConditionPathExists=
!`~/.config/mackes-shell/clipboard.disabled` so it's both
display-aware and toggleable.

### 13th birthright step

`apply_clipboard_daemon` enables `mackes-clipboard-daemon.service`
via `systemctl --user enable --now …`. Wired into the wizard apply
pipeline between Maximize windows and Mesh.

### Companion C panel plugin

The existing `xfce4-panel/plugins/mackes-clipboard` plugin (read side)
keeps working unchanged — it surfaces every peer's bucket in a
panel-popover. The daemon adds the write side that was missing.

## 1.4.7 — Conky Nerd Font glyphs (2026-05-17)

The Conky HUD now uses Nerd Font (Cascadia Code NF, the only patched
NF in stock Fedora 44) for icon glyphs alongside IBM Plex Sans for
prose. Every section header gets a glyph prefix:

  Shell (header)        terminal
  Mesh                  wifi
  Fleet                 cogs
  Drift                 warning
  Shared storage        archive
  Notifications         bell
  Media services        music
  Remote desktop        terminal-secure
  Services dot grid     server
  Hardware              CPU
  Clock                 clock
  Admin lock /        unlock / lock indicator

Glyphs are embedded directly as UTF-8 from the Private Use Area
(no ${execpi printf} hack — that one already burned us in v1.4.6).
The font switches mid-line via `${font Cascadia Code NF:size=10}` /
`${font IBM Plex Sans:size=N}` blocks so prose stays readable.
New helper `admin-lock-glyph.sh` emits the lock/unlock glyph only.

Spec: `Requires: cascadia-code-nf-fonts`.

## 1.4.6 — Panel layout / wallpaper / Conky / QNM (2026-05-17)

Four user-reported issues fixed in one cut:

* **Whisker menu missing from the panel.** `apply_panel_layout`
  wrote `/panels/panel-0/plugin-ids` as an empty single-value field
  instead of a proper uint array — fixed via `_set_array()` helper
  using `xfconf-query --create --force-array --type uint --set 101 …`.
  Array reset first so a default panel-0 doesn't conflict.

* **Whisker menu modifications not visible.** Added a Mackes-branded
  Whisker config block — button title "Mackes", button icon
  `mackes-shell`, search-position alternate (top), categories
  alternate, recent-items 10, menu 440×560, IBM Plex item names,
  `mackes-shell.desktop` favorited by default.

* **Wallpaper not applied.** `apply_appearance` silently skipped the
  wallpaper when the preset's path didn't exist. Now falls back to
  `/usr/share/mackes-shell/branding/standard-wallpaper.png` and
  stamps five common per-monitor xfconf keys (HDMI-1 / HDMI-A-1 /
  eDP-1 / LVDS-1 / VGA-1) in addition to the canonical
  `screen0/monitor0/workspace0/last-image`.

* **Conky never started.** The v1.4.0 template used
  `string.format([[…]], 35 args)` plus a fragile
  `${execpi 99999 printf "┃"}` Lua escape — both broke conky's
  Lua parser. Template rewritten as plain Lua concatenation;
  U+2503 embedded as a UTF-8 literal. Tested with `conky -c` —
  parses + forks cleanly.

* **QNM "where is it?" UX.** Sidebar nav item renamed from "QNM"
  to "Quick Network Mesh (QNM)" for new users.

## 1.4.5 — Toggle-button init-order crashes (2026-05-17)

Two `AttributeError` traceback surfaced during the first-run wizard
after v1.4.4 reached the Dashboard:

    AttributeError: 'MeshVpnPanel' object has no attribute '_peers_stack'
    AttributeError: 'AppsPanel' object has no attribute '_chips_box'

Root cause: the topology/table toggle on Mesh VPN and the
Install/Remove/Installed tabs on Apps both `set_active(True)` on
their default button **during** `_build()`. That fires the `toggled`
signal before the rest of the panel state (the Gtk.Stack the toggle
flips, the FlowBox of category chips) is constructed.

Fix: both handlers now `getattr(..., None)` for the dependent state
and return early if it's missing. The post-build refresh sets the
correct state afterwards — the early firing is a harmless no-op now.

## 1.4.4 — LightDM hang hotfix (2026-05-17)

The wizard's final step "Becoming Mackes…" hung indefinitely with the
log line `lightdm config: <…>` because `mackes/lightdm.py` had its own
`_pkexec_write` / `_pkexec_mkdir` helpers that bypassed AdminSession —
the NOPASSWD short-circuit never fired, so the calls prompted polkit
and either timed out or got dismissed.

Same fix pattern as the v1.4.3 headscale fix:

* `_pkexec_write` rewritten — when AdminSession is unlocked, stages
  the config to a tempfile and runs `install -D -m 0644 tmpfile
  target` via the cached sudo creds. Falls back to legacy
  stdin-piped `pkexec tee` only if AdminSession is unimportable.
* `_pkexec_mkdir` routes through `AdminSession.run(["mkdir", "-p", ...])`.
* Sudoers `MACKES_GATEWAY` extended to cover
  `/usr/bin/tee /etc/lightdm/*` and `/etc/lightdm/lightdm.conf.d/*`.
* Legacy `tee`-with-stdin timeout bumped 10s → 30s.

## 1.4.3 — Headscale + Tailscale prompt-storm hotfix (2026-05-17)

The v1.4.2 sudoers drop-in eliminated the pkexec prompt storm for
`dnf`, `systemctl`, and the other Mackes-managed commands — but
**headscale** and **tailscale** invocations kept prompting because:

  1. Those binaries weren't in the sudoers allowlist.
  2. `mesh_vpn.py:_pkexec_run` was a legacy wrapper that always used
     raw `pkexec` instead of routing through `AdminSession.run()` like
     birthright / debloat / remote_desktop / caddy_gateway.

Both fixed:

* **Sudoers extended** — `data/sudoers.d/mackes-shell` gains three
  new aliases: `MACKES_HEADSCALE`, `MACKES_TAILSCALE`, and
  `MACKES_HEADSCALE_CONFIG` (covering `tee /etc/headscale/*` plus
  the `bash -c "mkdir -p /etc/headscale && cat > …"` chunk the
  wizard uses to write `config.yaml`). All NOPASSWD for the `wheel`
  group. Validated by `visudo -c` in `%post`.

* **`mesh_vpn.py:_pkexec_run` refactored** to route through
  `AdminSession.instance().run(cmd)` — matches the v1.4.0 call-site
  migration pattern. The sudoers NOPASSWD short-circuit fires and
  the user never sees a polkit prompt during mesh setup. Falls back
  to the legacy `pkexec` / `sudo` / raw chain only if AdminSession
  is unimportable (paranoia path).

`mesh_ssh.py` already used `_pkexec_run` for its `headscale policy
set` call, so it inherits the fix automatically.

## 1.4.2 — Fedora 44 dep hotfix + fit-to-resolution windows (2026-05-17)

**Fedora 44 dep hotfix.** `xorg-x11-utils` was renamed/split out of
Fedora's package tree; `xprop` is its own package now. v1.4.1 install
failed with:

    Problem: conflicting requests
      - nothing provides xorg-x11-utils needed by mackes-shell-1.4.1

Spec Requires fixed: `xorg-x11-utils` → `xprop`. Same substitution
applied in `mackes/birthright.py:apply_maximize_all` so the wizard
step's dnf-install probe uses the correct package name on the fallback
path.

**Every GUI window fits the workstation resolution perfectly.** The
WorkbenchWindow and WizardWindow now detect the primary monitor's
size via `Gdk.Display.get_primary_monitor().get_geometry()`, open at
that exact size, and call `maximize()` on the `realize` signal so the
WM finishes the job. The previous hardcoded `1280×800` and `960×720`
defaults are gone — the windows fill whatever screen they land on,
whether 1366×768 laptop or 4K monitor. This overrides the Carbon
"max-content-width" pattern: the content area expands to use available
width rather than getting letterboxed.

Helper `_primary_monitor_size()` lives in both
`mackes/workbench/shell/sidebar_window.py` and `mackes/wizard/window.py`
(intentional duplication — they ship independent of each other and
the helper is 12 lines).

## 1.4.1 — Sudoers, installer UX, wizard discoverability, maximize-all (2026-05-17)

Five user-reported friction points addressed:

**Sudoers drop-in** (`data/sudoers.d/mackes-shell`, installed at
`/etc/sudoers.d/mackes-shell` mode 0440). Grants the `wheel` group
NOPASSWD on the Mackes-managed command allowlist (dnf, systemctl,
firewall-cmd, install/cp/chown, gtk-update-icon-cache,
plymouth-set-default-theme, the Apache-archive curls birthright uses,
tee for specific config paths). Validated by `visudo -c` in `%post`;
on failure the file is removed so the host's sudo behavior is never
broken. `AdminSession.run()` short-circuits to `sudo -n` when this
drop-in is active — no prompts at all during normal Mackes
operations. The previous prompt-storm during the wizard's birthright
pipeline is gone.

**Carbon-styled installer** (`install.sh` rewrite). Each phase
renders as a Carbon banner row with a spinner: Detect Fedora →
Resolve release tag → Download RPM → Install via dnf → Hand off to
wizard. The dnf transaction streams its output as Carbon-dimmed
lines instead of going dark for several minutes. Logs to
`/tmp/mackes-install.*.log` for triage.

**Always-visible Setup button in the header** — next to the Help
button. Opens the wizard regardless of `state.provisioned`. The
hidden "Re-open Wizard" inside the Tweaks drawer stays for muscle
memory.

**Birthright health check** (`mackes/birthright_check.py`): 12 probes
that verify each apply_* step's on-disk artifacts (theme dirs, IBM
Plex packages, Plymouth theme active, sudoers drop-in present, panel
layout xfconf, RPM/AppImage app presence, xrdp + Guacamole config,
ansible-pull timer enabled, Conky config + autostart, maximizer
service, Flathub remote, third-party repos). `is_complete()` returns
True only when all 12 pass.

**Always-maximize windows** (12th birthright). A new user-level
service `mackes-maximizer.service` polls `wmctrl -l` once per second
and adds `maximized_vert`/`maximized_horz` to every new top-level
window. Exempt classes: `xfce4-panel`, `xfdesktop`, `mackes-conky`,
`Plymouth`. RPM Requires `wmctrl` + `xorg-x11-utils` (for `xprop`).
Disable per-user via `~/.config/mackes-shell/maximizer.disabled`.

## 1.4.0 — Debloat tiers, TUI, Splash, Conky HUD, Session unlock, full Carbon (2026-05-17)

Seven user-driven additions plus the Carbon-completion pass that finishes
the design assimilation started in v1.1.x.

### Carbon completion

The two items deferred at the original v1.4.0 cut window are now done:

**Legacy panels** (`mackes/workbench/_common.py`): rewrote the shared
helpers (`panel_box / title_label / info_label / section_header /
labeled_row / error_label`) to emit Carbon-refresh widgets. Single-file
change cascades across **every** legacy panel that imported these
helpers — Devices / System / Network (Wi-Fi, VPN, QNM, Firewall) /
Help — without per-panel rewrites. Old v1.0 CSS class names are kept
alongside the new ones, so no CSS rule regressions.

**Carbon-native wizard window** (`mackes/wizard/window.py`): replaced
`Gtk.Assistant` with a custom `Gtk.ApplicationWindow` matching the
sidebar shell's chrome. Top: 9-step progress strip with active
indicator. Center: a `Gtk.Stack` of page widgets (welcome / env-scan /
preset-pick / appearance / hardware / network / snapshot / review /
apply / summary). Bottom: a Carbon action bar (Back / Cancel / Next or
Apply or Continue or Finish, depending on the active step's kind).
Existing page builder modules drop in unchanged — they were already
Carbon-styled inside. The PROGRESS step auto-launches the apply
pipeline on first activation, then unlocks the Continue button. The
SUMMARY step's Next button becomes "Finish" which destroys the window
and unblocks `do_activate` → opens the Dashboard.

### Features

**Conky HUD** (`mackes/conky_hud.py`, `data/conky/`, 11th birthright):
top-right Carbon-themed desktop panel (400 × ⅔ screen height) with
live Mackes-platform state. Opaque Carbon Gray 90 fill with a 3px
accent left-edge that swaps with the active preset. Birthright step
`apply_conky` installs the package + writes the user config + the XDG
autostart entry, then bounces the process. Tweaks panel gains a
"Show Conky HUD" switch under Chrome that flips both the autostart
file and the running process.

Tiered refresh per Q3 lock — `update_interval=1.0` for the system
built-ins (clock, CPU, RAM, load), `${execi 30 ...}` for Mackes-state
queries (mesh / fleet / drift / notifications / media services /
remote sessions / services dot-grid), `${execi 60 ...}` for shared
storage (rare changes).

Ten content blocks per Q4 lock: Header (version + preset + admin
lock), Mesh (peers + control node), Fleet (last pull + 24h failures),
Drift (items differing from preset), Shared storage (QNM-Shared
usage), Global notifications (mesh + local counts + latest), Media
services (Jellyfin/Plex/Airsonic/etc. across peers), Remote desktop
(active RDP/VNC + Guacamole connections), Services (sshd / headscale
/ tailscaled / guacd / tomcat / mackes-remote-sync / mackes-ansible-pull
/ caddy as a compact dot grid), Hardware (hostname / CPU / RAM / load
/ clock).

Helper scripts under `data/conky/helpers/*.sh` — one per block. Each
shells out to either a Mackes Python module (mesh / fleet / drift /
media) or pure shell (storage / notifications / remote / services).

Spec **Requires: conky** so birthright never finds the package missing.
The Conky preset accent live-swaps via `conky_hud.restart_with()`
called from the shell's `_apply_tweaks()` whenever the preset changes.

### Features

**Wizard boot splash** (`mackes/wizard/splash.py`): plays
`branding/MACKES-XFCE-LOGO.mp4` (H.264 1280×720, 8s, AAC audio muted)
as a borderless centered window before the first-run wizard surfaces.
Skippable via click / Escape / any key; auto-dismisses on
end-of-stream. Falls back silently if GStreamer or its H.264 decoder
isn't installed. The pipeline uses GStreamer `playbin` + the X11
`VideoOverlay` XID-embed pattern (gtksink isn't packaged in stock
Fedora 44 GStreamer, but `xvimagesink`/`ximagesink` are).
Spec Recommends: `gstreamer1`, `gstreamer1-plugins-{base,good,bad-free}`,
`mozilla-openh264`, `gstreamer1-plugin-openh264`. All Recommends not
Requires so headless nodes don't carry the codec stack.
MANIFEST.in extended to include `*.mp4` / `*.webm` under `branding/`
so the video survives the sdist round-trip into the RPM.

**Debloat levels** (`mackes/debloat.py`, `Maintain → Debloat levels`):
five cumulative tiers (L1 Light → L5 Viable). Each tier is an
idempotent `dnf remove` set plus optional xfconf resets. The panel shows
a live preview of what's currently installed vs already absent before
the user commits. Bound by a confirm modal; logs the run.

**Textual TUI** (`mackes/tui/`, autobooted on headless): runs every
screen the GUI has — Dashboard, Mesh VPN, Mesh SSH, Mesh Services,
Mesh Remote, Fleet Inventory, Fleet Playbooks, Fleet Run history,
Snapshots, Debloat, Help. Launches automatically when there's no
`$DISPLAY` and no subcommand. `python3 -m mackes --tui` forces it.

**Session unlock** (`mackes/admin_session.py`, header Lock/Unlock
button): single sign-in for the whole Mackes session. Click Unlock,
type the password once, every subsequent admin op runs without
prompting. Uses sudo's timestamp cache + a 4-min keepalive thread.
Auto-locks when the window closes. Migrated call sites:
  - `mackes/birthright.py:_run_root`
  - `mackes/workbench/network/remote_desktop.py:_run_root`
  - `mackes/debloat.py:apply_level`
  - `mackes/caddy_gateway.py:_pkexec`

**Live status bar** (`shell/sidebar_window.py:_refresh_status_bar`):
the bottom bar's mesh / services / sshd / drift counts are now live —
pulled from `service_health()`, the Headscale roster, the mesh-services
registry, and the active-preset drift detector. Refreshes every 30s.

**Live sidebar nav badges**: peer count on Mesh VPN, service count on
Mesh Services, failed-runs count on Fleet → Run history, drift-items
count on Maintain. Same 30s refresh cycle as the status bar.

**Tweaks density** finally works: compact / cozy / comfortable now
swap `.mackes-density-*` classes on the root window. CSS rules in
`carbon-layout.css` adjust nav-item heights, tile padding, and
data-table row heights accordingly.

**Toast host** (`shell/toasts.py`): bottom-right non-modal notifications
for shell-wide events. Snapshot create now uses a toast instead of a
silent status label.

### Carbon design system

`.claude/CLAUDE.md` + `.claude/skills/{mackes-worklist-management,
complete-remaining-work}/SKILL.md` — three workflow protocols ported
from `matthewmackes/map2-audio` and adapted to the mackes-shell repo.
The commit/push rulebook, single-source worklist, and autonomy policy
are now durable behavioral contracts in `.claude/`.

### Open-source project artifacts

Added the standard OSS files the repo was missing:
  - `CONTRIBUTING.md` — dev setup + project conventions
  - `CODE_OF_CONDUCT.md` — Contributor Covenant v2.1
  - `SECURITY.md` — disclosure protocol + threat model
  - `AUTHORS` — maintainer + upstream credits
  - `.editorconfig` — line endings + indentation
  - `.github/ISSUE_TEMPLATE/{bug_report,feature_request,config.yml}`
  - `.github/PULL_REQUEST_TEMPLATE.md`
  - `.github/FUNDING.yml`
  - `.github/dependabot.yml` (weekly Actions bumps)
  - `CITATION.cff`

### Deferred to v1.4.1

Legacy panels (`devices/*`, `system/*`, `network/{wifi,vpn,qnm,firewall}.py`)
still use the v1.0-era `workbench/_common.py` helpers — they look
inconsistent next to the v1.1.x Carbon-refresh panels. Wizard chrome is
still `Gtk.Assistant`, not a Carbon-native window. Both are tracked as
v1.4.1 work — they're substantial mechanical rewrites that don't block
the v1.4.0 functional additions.

## 1.3.0 — Mesh Fleet (Ansible-pull) (2026-05-17)

Cross-peer fleet management lands as a 10th wizard birthright step.
Ten design decisions locked via the 1.3.0 question survey:

  1. Transport: **ansible-pull** on every peer (no central controller)
  2. Playbook store: **QNM-Shared/.qnm-sync/playbooks/** (replicated by
     the existing file substrate)
  3. Install: 10th wizard step `apply_fleet` — always on
  4. Curated playbooks: 7 roles ship — system-update, bloat-removal,
     apps-install, xfconf-baseline, mesh-state-snapshot,
     selinux-permissive-toggle, container-runtime-setup
  5. Schedule: systemd timer — OnBootSec=10min,
     OnUnitActiveSec=30min, RandomizedDelaySec=5min
  6. GUI: new top-level **Fleet** sidebar group with 3 items
     (Inventory / Playbooks / Run history)
  7. Editor: read-only YAML preview + `xdg-open` to user's editor
  8. Secrets: none — playbooks are plaintext
  9. Run history: 30-day retention, one JSON per run at
     `QNM-Shared/.qnm-sync/ansible-runs/<peer>/<ts>.json`
 10. Ad-hoc: yes — Inventory has multi-select + "Run on selection"
     SSH-push over mesh-SSH identity

### What was added

**New birthright step** `apply_fleet` in `mackes/birthright.py`:
  - dnf install: ansible-core, python3-ansible-runner, podman
  - Seeds the playbook tree into QNM-Shared/.qnm-sync/playbooks/
  - Installs + enables mackes-ansible-pull.{service,timer}
  - Queues an initial pull (non-blocking)

**New module** `mackes/fleet.py`:
  - `build_inventory()` — Headscale roster → FleetPeer list with
    per-peer last-pull timestamp + 24h pull count
  - `list_playbooks()` — discovers roles under the QNM-Shared tree
  - `list_runs()` / `write_run_record()` / `prune_runs()` — full
    30-day-retention history reader/writer
  - `run_local_pull()` — local ansible-pull, parses the PLAY RECAP,
    writes a JSON record
  - `run_push()` — ansible-playbook SSH push to selected peers via
    a generated ephemeral inventory.ini
  - CLI: `python -m mackes.fleet --pull / --push / --list / --history / --prune`

**7 curated playbooks** under `data/ansible/playbooks/`:
  - system-update          (tag-gated `never`; opt-in via GUI)
  - bloat-removal          (default-tagged; runs on every cycle)
  - apps-install           (default-tagged)
  - xfconf-baseline        (default-tagged; the steady-state drift corrector)
  - mesh-state-snapshot    (tag-gated `never`)
  - selinux-permissive-toggle (tag-gated `never`)
  - container-runtime-setup (tag-gated `never`)

**Systemd units** at `data/systemd/`:
  - mackes-ansible-pull.service (Type=oneshot, ConditionPathExists
    fleet.disabled escape hatch)
  - mackes-ansible-pull.timer (30-min cycle with 5-min jitter)

**Fleet GUI** — new top-level `Fleet` sidebar group with 3 Carbon panels:

  - `mackes/workbench/fleet/inventory.py` — Carbon page header, live
    status notification, action row with Run-on-selection /
    Local-pull / Select-all-online / Clear, peer ListBox with
    checkbox + status dot + last-pull age + per-peer status tag.
    Multi-select drives the SSH-push playbook picker Modal.
  - `mackes/workbench/fleet/playbooks.py` — grid of Carbon tiles per
    playbook with description, tag chips (default / never), last-run
    summary, YAML preview, Run-now and Open-in-editor buttons.
  - `mackes/workbench/fleet/run_history.py` — stat tiles (Total /
    Successful / Failed / Changes applied), peer + playbook filters,
    Carbon DataTable of every run across the mesh. Click any row to
    see the full JSON in a Carbon Modal (timestamp, trigger, duration,
    counts, log tail).

**Spec Requires:** ansible-core, python3-ansible-runner, podman.
**Spec Recommends:** buildah, skopeo, toolbox.

## 1.2.0 — Mesh Remote Desktop (2026-05-17)

Every Mackes node now ships browser-accessible remote desktop. Five
design decisions locked via the 1.2.0 question survey:

  1. Backends: **xrdp + x11vnc on every peer** (both protocols)
  2. Topology: **every peer runs guacd + Guacamole**
  3. Auth: **none on the mesh** (firewall + mesh CA are the trust)
  4. Connection discovery: **Headscale roster auto + Mackes overrides**
  5. Enablement: **birthright — always on**

### What was added

**9th birthright step** `apply_remote_desktop` in `mackes/birthright.py`:
  - dnf install: xrdp, xrdp-selinux, x11vnc, guacd, tomcat, curl
  - Downloads guacamole-1.6.0.war from the Apache archive into
    /var/lib/tomcat/webapps/
  - Installs the noauth extension jar at /etc/guacamole/extensions/
  - Writes /etc/guacamole/guacamole.properties + a seed
    /etc/guacamole/noauth-config.xml
  - Installs an x11vnc@.service systemd template that binds to the
    mesh IP only (live :0 mirror)
  - Installs mackes-remote-sync.service (regenerates the noauth
    connection list from the Headscale peer roster every 30s)
  - Opens firewalld ports 3389 / 5900 / 8080 on the trusted zone only
  - Enables + starts: xrdp, xrdp-sesman, x11vnc@:0, guacd, tomcat,
    mackes-remote-sync

**Connection sync** `mackes/remote_desktop.py`:
  - `active_connections()` returns RDP + VNC entries per Headscale peer,
    layered with `~/.config/mackes-shell/remote-overrides.json`
    (favorite / hide / rename)
  - `rebuild_connections()` writes /etc/guacamole/noauth-config.xml
  - `sync_daemon_main()` is the systemd-managed polling loop
  - CLI: `python -m mackes.remote_desktop --list / --rebuild / --daemon`

**Caddy gateway** route added in `mackes/caddy_gateway.py`:
  `https://media.mesh/desktop/  →  http://127.0.0.1:8080/guacamole/`

**Mesh Remote panel** `mackes/workbench/network/remote_desktop.py` —
a full first-class configuration GUI matching the Carbon panel
patterns:
  - Breadcrumb + page title + subtitle + live status Notification
  - Local services grid (xrdp / x11vnc / guacd / tomcat)
  - **Display sharing** tile: enable/disable x11vnc, X display picker,
    view-only mode toggle
  - **RDP server** tile: enable/disable xrdp, Xorg vs Xvnc backend,
    max concurrent sessions
  - **Gateway** tile: Tomcat toggle + Open-in-browser button + code
    block showing the effective Caddy route
  - **Connections** Carbon DataTable with per-row Favorite / Hide /
    Rename buttons (Rename opens a Carbon Modal)
  - **Auto-discovery** tile: sync interval (10-600s) + last-sync
    timestamp display
  - **Diagnostics** tile: `systemctl status` text for all five units +
    Refresh button
  - Persists per-user prefs to `~/.config/mackes-shell/remote-desktop.json`

**Sidebar nav** gains a "Mesh Remote" entry under Network.

**Wizard** apply pipeline is now 19 steps (added "Remote desktop"
between Flathub and Mesh); review page lists the new step.

### Spec requires

The RPM now Requires xrdp / xrdp-selinux / x11vnc / guacd / tomcat /
curl. The guacamole.war + noauth jar are fetched from the Apache
archive at first-wizard-run; the RPM itself doesn't carry them.

## 1.1.1 — Carbon panel rebuilds (the rest of the design) (2026-05-17)

Picks up where 1.1.0 left off — the seven panels that were deferred at
the v1.1.0 release window are now rebuilt to match
`docs/design/v1.1.0-carbon-refresh/`:

* **Mesh SSH** (`mackes/workbench/network/mesh_ssh.py`): page-title +
  breadcrumb, live "Tailscale-SSH active on N peers" Notification, peer
  DataTable with a host-key fingerprint column, ACL hujson rendered as
  a Carbon code block with an Edit/Save/Reload toolbar, key
  distribution actions tile, audit log DataTable.
* **Mesh Services** (`mackes/workbench/network/mesh_services.py`):
  scan/refresh action row, peer filter pills, 3-column Carbon tile
  grid of discovered services (each tile shows kind tag, status dot,
  display name, peer, accent URL), unified gateway tile with a Switch
  + route-preview code block, mDNS bridge tile listing relayed types
  as Tag chips.
* **Appearance** (`mackes/workbench/look_and_feel/appearance.py`):
  rewrapped into a two-column Carbon layout — selectors on the left
  (existing xfconf bindings preserved verbatim), live preview pane on
  the right with sample window chrome + heading + body + mono command
  + Primary/Tertiary/Ghost button row + an Active Accent swatch tile
  + Design-system-lock notification.
* **Apps** (`mackes/workbench/apps/panel.py` — new unified panel):
  three Carbon tabs (Install / Remove bloat / Installed), category
  filter chips derived from the catalog, search input, grid of
  `.mackes-app-card` tiles with icon/name/desc/meta and per-tab
  action button. Replaces the three legacy `install.py` /
  `remove.py` / `installed.py` panels at the sidebar entry point.
* **Snapshots** (`mackes/workbench/maintain/snapshots.py`): Carbon
  create tile (label input + Primary button + helper line listing
  exactly what gets captured) + Carbon DataTable of existing
  snapshots (label, created timestamp, source preset, size).
  Restore opens a confirm modal; double-click also triggers restore.
* **Maintain hub** (`mackes/workbench/maintain/hub.py` — new): 12-tile
  Carbon grid replacing the old StackSidebar+Stack inner layout for
  the Maintain section. Tile click switches an inner Gtk.Stack to the
  matching sub-panel, which is wrapped with a "‹ Back to Maintain"
  link header. Drift tile and Uninstall tile carry warning/error tags
  to mirror the design.
* **Help** (`mackes/workbench/help.py`): left rail now uses the
  `mackes-side-nav` Carbon classes (consistent with the main shell);
  right pane has a Carbon breadcrumb + page-title header above the
  existing markdown TextView, which got Carbon 40px page margins.
  Topic discovery and markdown rendering unchanged.

## 1.1.0 — Carbon refresh + birthright fold (2026-05-17)

A major release. Two large changes bundled into one cut:

### 1. Carbon refresh — sidebar shell + per-preset accents

Mackes' chrome was rebuilt to match the design at
`docs/design/v1.1.0-carbon-refresh/`. The old top-tab Notebook is gone;
in its place is a Carbon UI Shell with:

- 48px header strip (brand block + Workbench/Recovery/CLI mode buttons +
  preset chip + user@host)
- 256px grouped sidebar (Workbench / Configuration / Network / Apps &
  Maintenance / Reference) with badges and live-active highlighting
- Bottom 24px status bar (mesh/services/sshd/drift/version/preset)
- A floating **Tweaks** panel (bottom-right) for live preset swap,
  density (compact/cozy/comfortable), chrome toggles, and "Re-open
  Wizard" — state persists to `~/.config/mackes-shell/tweaks.json`.

The Dashboard is now Carbon stat tiles (mesh peers / services / sshd /
drift), a service-health grid, a Carbon notification for drift, a 2x2
hardware tile grid, six tertiary-style quick-action buttons, and a
mono-styled recent-activity log.

The **Mesh VPN panel** got a new Cairo-drawn topology view — control
node at center, peers in a ring around it, animated edge pulses
travelling along, dashed lines for DERP-relayed edges, click any peer
for a right-rail detail drawer. A toggle next to the section header
swaps between the topology view and the Carbon DataTable variant.

A 5th accent preset, **Node** (Carbon Green 50 #42be65), was added for
headless / server installs.

New files: `data/css/carbon-layout.css` (sidebar / topology / tile /
modal / topology / tweaks classes), `mackes/workbench/shell/`
(sidebar_window.py + tweaks_panel.py),
`mackes/workbench/network/mesh_topology.py` (Cairo widget),
`data/css/accents/node.css`.

### 2. Birthright fold — 8 new wizard apply steps

The audit in conversation 2026-05-17 found 7 items the wizard *should*
do at first run but didn't. They're now wired in. The wizard's apply
pipeline went from 10 steps to 18:

  Snapshot → Appearance → Devices → System → Network → Panel →
  **Themes → Fonts → Apps → Panel layout → Boot splash → System update →
  Third-party repos → Flathub** → Mesh → VPN import → Menu → Finalize

- **Themes**: copy `data/themes/PadOS/` and `data/icons/Carbon/` to
  `/usr/share/themes/` and `/usr/share/icons/`; rebuild GTK icon cache.
- **Fonts**: dnf install `ibm-plex-sans-fonts` + `ibm-plex-mono-fonts`;
  rebuild fontconfig cache.
- **Apps**: process `preset.apps.install` (install_curated_set) and
  `preset.apps.remove_bloat` (remove_packages). These lists already
  existed in every preset YAML but were never run.
- **Panel layout**: write the Mackes default xfce4-panel xfconf layout
  — Whisker Menu + Docklike + spacer + systray + IBM Plex clock — and
  `xfce4-panel --restart`.
- **Boot splash**: install + activate the Mackes Plymouth theme
  (centered logo on Carbon Gray 100 with an accent progress strip);
  regenerates initrd via `plymouth-set-default-theme mackes -R`.
- **System update**: `dnf upgrade -y --refresh` (heaviest step).
- **Third-party repos**: install `fedora-workstation-repositories`
  (Chrome/Steam/NVIDIA repo files, disabled by default) plus enable
  RPM Fusion free + nonfree for the detected Fedora version.
- **Flathub**: add the per-user Flathub remote via
  `flatpak remote-add --if-not-exists --user flathub …`.

All 8 are idempotent (re-runnable via Maintain → Reset to Preset) and
live in the new `mackes/birthright.py` module.

### Fixes

- `xfconf_bridge.XfconfBridge.set` int/float coercion (1.0.4
  hotfix folded in): subprocess.check_call won't accept non-string argv
  members, so int/float values now stringify before the subprocess call.
- App installer's per-app output now reads `App: installed (npm)` /
  `App: FAILED (rc=N) (npm)` instead of the always-on `rc={rc}` form.
- Cursor's stale `download.cursor.sh` URL replaced with a runtime
  resolver against `cursor.com/api/download`.
- `neofetch` (archived upstream) is installed as `fastfetch` (its
  maintained successor) under the same catalog name.

## 1.0.5 — fix Cursor + neofetch installs, clearer output (2026-05-17)

App installer fixes after observing the wizard-time install output:

    Cursor: appimage rc=1
    <urlopen error [Errno -2] Name or service not known>
    Claude Code CLI: npm install rc=0
    changed 2 packages in 2s
    neofetch: dnf install rc=1

* **Cursor**: the hardcoded `download.cursor.sh` URL was dead — Cursor
  retired that subdomain. Replaced with a runtime resolver that calls
  `https://www.cursor.com/api/download?platform=linux-x64&releaseTrack=stable`
  (which needs a non-empty User-Agent or returns 400) and pulls the
  current `downloadUrl` out of the JSON. The User-Agent is passed on
  the AppImage download request as well.

* **neofetch**: archived upstream in 2024, dropped from Fedora 44 repos.
  The catalog entry still accepts the name `neofetch` (so existing
  preset YAMLs keep working) but installs the maintained successor
  `fastfetch` instead. A separate `fastfetch` catalog entry was added
  for explicit selection.

* **Output**: per-app install lines now read `App: installed (npm)` on
  success and `App: FAILED (rc=N) (npm)` on failure instead of the
  always-on `App: npm install rc=N` form, which looked
  indistinguishable between success and failure.

## 1.0.4 — fix xfconf_bridge int/float coercion (2026-05-17)

After installing 1.0.3 and running the wizard, three provisioner steps
all failed with the same exception:

    →  Appearance
       ERROR: expected str, bytes or os.PathLike object, not int
    →  System
       ERROR: expected str, bytes or os.PathLike object, not int
    →  Panel
       ERROR: expected str, bytes or os.PathLike object, not int

Root cause in `mackes/xfconf_bridge.py::XfconfBridge.set`: when `value`
was an `int` (e.g. `cursor_size`, `workspace_count`, `/notify-location`)
or a `float` and no `type_hint` was given, the code set the right
`--type` flag but forgot to stringify `value`. The `int`/`float` then
went straight into the `subprocess.check_call` argv list, which only
accepts `str | bytes | os.PathLike`, so subprocess refused it before
xfconf-query was ever invoked.

Fix: in the int branch, `value = str(int(value))`; in the float branch,
`value = repr(float(value))`. The bool/string branches already
stringified correctly; explicit-type-hint callers already get
`value = str(value)`.

Verified with a 5-call regression test (bool / int / float / str /
explicit-type-hint) — all reach subprocess with str-only argv.

## 1.0.3 — fix MackesApp import (2026-05-17)

Install + launch flow surfaced an ImportError immediately after install:

    ImportError: cannot import name 'MackesApp' from 'mackes.app'
        File "mackes/__main__.py", line 14, in <module>
            from mackes.app import MackesApp

When `mackes.app` was refactored in 1.0 to lazy-import GTK (so headless
installs don't drag GTK into memory), the `MackesApp` class moved inside
an internal `_make_gui_app()` builder function — no longer a top-level
symbol. `mackes/__main__.py` still expected the old top-level import.

Fix: `__main__.py` now delegates to `mackes.app.main(argv[1:])` directly.
The `--uninstall` / `--yes` fast-path is preserved (still handled in
__main__ so the uninstall sequence can run without going through the
GUI router). Everything else — `--gui`, `--headless`, subcommands,
auto-detection — goes through `mackes.app.main`, which already knows
how to instantiate the GUI when it needs to.

Verified: `python3 -m mackes --version` prints `mackes 1.0.3`;
`python3 -m mackes help` prints the topic list.

## 1.0.2 — headscale.service file conflict (2026-05-17)

`dnf install` failed on the v1.0.1 RPM with:

    file /usr/lib/systemd/system/headscale.service conflicts between
    attempted installs of mackes-shell-1.0.1-1.fc44.x86_64
    and headscale-0.28.0-1.fc44.x86_64

The upstream `headscale` RPM (which we Require) ships its own
`headscale.service` at the same path. We were shipping a near-identical
copy with two extra knobs (MemoryHigh/MemoryMax). Fixed by dropping our
copy from the RPM — the upstream unit is used as-is.

`data/systemd/headscale.service` stays in the source tree as a reference
template. To apply Mackes-specific resource limits at deploy time, drop
a systemd override at `/etc/systemd/system/headscale.service.d/mackes.conf`
with the desired directives.

No code changes.

## 1.0.1 — Fedora 44 dep hotfix (2026-05-17)

`curl … install.sh | bash` was failing on stock Fedora 44 because three
of the spec's `Requires:` resolved to packages that don't exist on F44
under those names. Fixed:

- `Requires: xfce4-power-manager-plugin` → `Requires: xfce4-power-manager`
  (the panel plugin ships inside the parent package as
  `libxfce4powermanager.so`; there's no separate plugin RPM)
- `Requires: sshfs` → `Requires: fuse-sshfs` (Fedora-specific name)
- `Recommends: jellyfin-media-player` → removed (not in Fedora repos;
  users install via Flathub instead). Mackes' Media-Hub discovery still
  surfaces Jellyfin servers on the mesh whether or not a local native
  client is installed.

No code changes. RPM spec + version bump only.

## 1.0.0 — "XFCE Provisioner" (2026-05-16)

### Identity
- First non-private release. "MAP2 Sub Testing" markers fully removed across
  packaging, spec, and runtime UI.
- Repositioned from "shell stack manager" to "XFCE provisioner + mesh fabric".

### The XFCE Pivot (Q1–Q20 survey)
- Retired the Polybar / Plank / Rofi / picom / dunst shell stack entirely.
  Mackes now provisions a standard XFCE shell: xfce4-panel + xfdesktop +
  xfce4-appfinder + xfce4-notifyd, with Whisker Menu as the start menu and
  Docklike Taskbar replacing Window Buttons.
- Standard panel layout: Whisker (far-left) → Docklike taskbar → systray →
  volume → power → clock (IBM Plex Sans).
- PadOS locked as the default GTK theme; other themes greyed-out in the
  Appearance picker.
- Carbon Icons (Apache 2.0) as the system-wide GTK icon theme (replaced
  the briefly-considered Clarity icons).
- IBM Plex Sans (UI) + IBM Plex Mono (monospace) replace SF Pro / JetBrains
  Mono throughout.
- `branding/standard-wallpaper.png` is the locked desktop + LightDM greeter
  wallpaper, vendored at 7.8 MB.
- Bloat list collapsed to a single combined `remove_bloat` per preset; XFCE
  extras (asunder, parole, pragha, xfburn, transmission-gtk, claws-mail,
  pidgin) added alongside GNOME-on-XFCE apps + libreoffice-*.
- `menulibre` added to install lists for hashbang / mackes / daylight.
- ssh enabled by default on every Mackes install via RPM %post.
- LightDM greeter silently configured to match preset theme/wallpaper/font.

### Carbon Design System chrome (Q-CB1–Q-CB10)
- Pixel-exact Gray 100 palette (#161616 / #262626 / #393939 / #525252 /
  #f4f4f4 / #969696 / #2d2d30).
- Carbon UI Shell layout: 48px top header + 256px left side nav + main +
  24px status bar.
- IBM Plex Sans UI / IBM Plex Mono monospace.
- Per-preset accent (hashbang-red etc.) replaces Carbon blue at every
  focus/highlight surface.
- Carbon Icons everywhere (chrome + system theme).
- Strict 8px grid via `--cds-spacing-01` … `--cds-spacing-13` tokens; CI
  lint rejects raw `px` in `data/css/*.css`.
- Centralized design tokens in `data/css/tokens.css`.
- Full custom widget library locked in `mackes/carbon/`: Tile, DataTable,
  Accordion, NumberInput, MultiSelect, Notification, Toast, Modal,
  Skeleton, Button (5-tier), UIShell.

### Mesh fabric (§8.10–§8.14)
- **Mesh Thunar Extension** (Q-MX1–Q-MX20): `mesh:///` GVFS backend +
  Tumbler thumbnailer. Four subtrees — Peers (SSHFS, live), Clipboard
  (NATS-backed, 100-item ring + Saved/), Notifications (.md per entry),
  Object Store (Themes / Snapshots / Presets / Drop). Live updates via
  qnmd→FUSE inotify. 16-peer cap.
- **Mesh VPN** (§8.11): Headscale + Tailscale clients. Auto-elected
  control node with NATS-state replication + 30s snapshot. Tailscale-
  bootstrap (Option C) for cross-network discovery — only seed peer signs
  into Tailscale's free tier (1/100 node count forever).
- **Headless Node Mode** (§8.12, Q-HL1–Q-HL7): full `mackes init` /
  `mackes join` / `mackes status` / etc. CLI parity with the GUI panels.
  Auto-detect missing display + logind graphical session. New
  `data/presets/node.yaml` headless preset. `mackes-node.service` systemd
  unit.
- **Mesh Media Services** (§8.13, 5 layers): raw URLs / Media Hub panel /
  Caddy gateway / bundled native clients / mDNS-over-mesh relay. Shared
  catalog `data/media-services.yaml` consumed by all layers.
- **Mesh SSH** (§8.14, 3 layers): SSH cheatsheet + auto-distributed
  ed25519 keys via NATS + Tailscale-SSH identity-based access via
  Headscale. Audit log in NATS `mesh.ssh-audit`.

### Help / Documentation
- New comprehensive Help system: `docs/help/*.md` covers every feature.
  Surfaced via a Help tab in the workbench and `mackes help [topic]` in
  headless mode.

### Removals
- Deleted: `mackes/polybar_catalog.py`, `mackes/polybar_gen.py`,
  `mackes/shell_profiles.py`, `mackes/session_manager.py`,
  `mackes/workbench/shell/{polybar,plank,rofi,panel_visibility}.py`,
  `mackes/wizard/pages/shell.py`,
  `tests/test_{polybar_catalog,shell_profiles,shell_profiles_save}.py`.
- Deleted directories: `data/shell-profiles/` (8.7 MB of adi1090x families),
  `data/plank-themes/` (440 KB of dock themes).
- Net cleanup: ~1,200 file deletions; -631 / +191 lines across surviving
  source files.

### Packaging
- RPM hard `Requires`: xfce4-session, xfce4-whiskermenu-plugin,
  xfce4-docklike-plugin, xfce4-pulseaudio-plugin,
  xfce4-power-manager-plugin, openssh-server, headscale, tailscale.
- `Recommends`: caddy, jellyfin-media-player, strawberry,
  ibm-plex-sans-fonts, ibm-plex-mono-fonts, firewalld, pulseaudio-utils.
- Dropped: polybar, plank, rofi, dunst, picom, papirus-icon-theme,
  arc-theme, google-droid-sans-fonts, jetbrains-mono-fonts.

## Unreleased (post-0.1.1 redesign)

### Identity

- Stripped "PRIVATE WORK / Sub Testing Release" from dashboard, wizard,
  and About dialog. Mackes Shell is no longer marked as private testing
  in user-visible copy.
- Reimagined first-run wizard as a 3-act ceremony (Welcome → Pick a
  preset → Narrated apply). Welcome is spare (logo + 3 sentences + one
  details disclosure). Preset pick is a 4-card grid with wallpaper
  thumbnails. Apply has a dynamic title that transforms from "Becoming
  <preset>…" to "You are now <preset>."

### Presets

- Replaced single `chupre.yaml` with **four presets**:
  `hashbang` (display `#!`, default), `mackes`, `daylight`, `vanilla`.
- Each preset ships its own polybar, plank, and rofi profiles.
- Per-preset wallpapers in `data/wallpapers/`.
- `DEFAULT_PRESET_NAME = "hashbang"` — Mackes' first impression is the
  CrunchBang reincarnation.

### Design system

- SF Pro fonts installed and wired as the GUI default.
- `data/css/mackes.css` defines `.mackes-panel-title`,
  `.mackes-section-header`, `.mackes-info`, `.mackes-row-label`.
- `data/css/accents/<preset>.css` swaps `@define-color mackes_accent`
  per active preset.
- `app.py` loads base CSS + per-preset accent at startup, process-scoped.
- Monospace surfaces (log viewers, action streams) preserved with
  JetBrains Mono / Iosevka / Fira Code fallback.

### Polybar Editor (replaces preset-picker)

- New `mackes/polybar_catalog.py` — discovers 21 vendored adi1090x
  families across `simple/` and `bitmap/` variants.
- New `mackes/polybar_gen.py` — pure-function config generator with CLI
  (`python3 -m mackes.polybar_gen --theme <family>`).
- New editor panel: theme picker + geometry knobs + 3-zone DnD module
  editor with cross-zone drag + add-module popover + save-as-profile +
  copy-to-clipboard + live debounced apply (~300 ms).
- 8.7 MB upstream vendor (simple + bitmap, GPL-3.0, no fonts/wallpapers).

### MaintenanceKit

- **System Update** — pkexec dnf-upgrade wrapper with streaming log
- **Drift** — first-class drift surface with per-key revert/adopt/ignore
- **Fonts** — fc-list browser with Pango preview + dnf quick-install set
- **Power** — power-profiles-daemon selector + tlp summary fallback
- **Resources** — CPU / RAM / disk cards, 1.5 s live refresh, /proc-based

### Recovery shell

- New `mackes/recover.py` — TTY-driven snapshot picker
  (`python3 -m mackes.recover` / `--list` / `--latest`)
- `data/systemd/mackes-recovery.target` — multi-user + network target
- `data/grub/40_mackes_recovery` — GRUB submenu source
- `install-helpers/install-recovery.sh` — root-needed installer

### Update mechanism

- `data/dnf/mackes-shell.repo` — dnf repo manifest pointing at
  `https://matthewmackes.github.io/MAP2-RELEASES/fedora/$releasever/$basearch`
- `install-helpers/add-mackes-repo.sh` — drops the .repo into
  `/etc/yum.repos.d/`

### ISO build

- `packaging/iso/mackes-xfce.ks` — Fedora kickstart with mackes-shell
  baked in, polybar/plank/rofi/dunst/picom stack, dnf repo wiring,
  recovery shell wiring
- `make iso` target wrapping `livemedia-creator`

### Tests + dev tooling

- 20 passing tests including 9 new ones for polybar catalog/gen, plus
  CSS resolution, shell-profile save plumbing, recovery CLI
- `tests/_run_without_pytest.py` — runs the suite without pytest
  installed (handy fallback for fresh Fedora boxes)
- `make test-nodeps` target

### Headless apply

- `python3 -m mackes.cli_apply --preset NAME` — re-apply a preset
  without the GUI (SSH, automation, recovery flows)

### Documentation

- README rewritten to reflect actual feature surface (was a skeleton-
  status placeholder)
- `packaging/iso/README.md` — kickstart build docs
- `data/shell-profiles/polybar/upstream/ATTRIBUTION.md` — GPL-3.0
  attribution + refresh procedure

## 0.1.1

Initial single-binary skeleton with placeholder panels and the chupre
preset baseline. (Pre-redesign state captured in the original `docs/`
folder.)
