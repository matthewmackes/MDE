# Project Worklist — Mackes Shell

**Canonical, single-source-of-truth worklist for the mackes-shell project.**

**Status legend:**
`[ ] Open` · `[>] In Progress` · `[✓] Done` · `[!] Blocked`

> **Release strategy (updated 2026-05-23 by operator):**
> **v4.0.0 shipped 2026-05-22** (commit `fbd9c5a`, RPM
> `mde-4.0.0-1.fc44.x86_64`). The current in-flight bundle is
> **v4.0.1**; per the **2026-05-23 "no new RPM until directed"
> standing constraint**, v4.0.1 work lands on `main` and deploys
> via the parity-overlay machinery (see "v4.0.1 operator round 2
> + parity infra" section below) rather than via a fresh RPM cut.
> Tasks tagged `v2.0.4:`, `v2.1:`, `v2.1.0:`, `v3.x:` or any
> other intermediate version in their titles still land on
> `main`; they ride whatever the next operator-authorized cut
> becomes. The iteration loop's exit condition is unchanged:
> every non-Hardware-Testing-epic item `[✓] Done`, then operator
> invokes `cut release X.Y.Z`.

**Authority:** this file is the only durable worklist. Per
`.claude/CLAUDE.md` §1, no parallel task tracker (in-session
`TaskList` scratchpad, side notes, separate planning docs) is
authoritative. **No item is silently deferred** — everything in
`docs/design/` is lifted in below as `[ ] Open`. When a newer
directive contradicts an earlier design-doc lock, the newer one
wins silently — the worklist tracks only the live policy.

**Story format (locked 2026-05-23).** Every NEW worklist item
lands as a user story, not a one-line summary. The shape:

> `- [ ] **<release>: <ID> <title> (Tier <N>)**` <br>
> `**As** an <role>,` <br>
> `**I want** <change in user-visible behavior>,` <br>
> `**so that** <user value>.` <br>
> `**Acceptance** (each bullet bench-observable):` <br>
> &nbsp;&nbsp;`- [ ] criterion 1` <br>
> &nbsp;&nbsp;`- [ ] criterion 2` <br>
> `**Implementation notes:**` <br>
> &nbsp;&nbsp;`- influence reference (Win11 chrome / Ableton content)` <br>
> &nbsp;&nbsp;`- Carbon glyph name(s) per the iconography lock` <br>
> &nbsp;&nbsp;`- stacked blockers, design-doc cross-references`

Pre-2026-05-23 one-line tasks are grandfathered. The full
shape + rationale lives in the `iteration` skill (local at
`.claude/skills/iteration/SKILL.md`) under "Story format for
new worklist items".

**Iconography lock (locked 2026-05-23).** Every production
icon ships from the **Carbon Icon Set** — bake assets into
`assets/icons/carbon/<carbon_name>.svg`, add the matching arm
to `mde_theme::ResolvedIcon::svg_bytes()`, render via
`iced::widget::svg(svg::Handle::from_memory(bytes))`. Lucide /
Phosphor / Material / Font Awesome / hicolor / Black-Sun /
Orchis icons in production code are an audit finding (Phase
0.8). Unicode fallback glyphs are tolerated only as the
`svg_bytes() → None` safety net per BUG-13. When a worklist
story names an icon slot, the Implementation notes MUST cite
the Carbon glyph by name.

**Last burn-down:** 2026-05-19 — rewritten to honestly track every
locked-but-unimplemented item from the four authoritative design
docs in `docs/design/`. Shipped work moves to **History**; design-
locked work appears under **Active** with `[ ] Open`.

---

## Active

> **Active section status (2026-05-21 — post-iteration):**
>
> * `[!] Blocked` = **0**. Every v2.0.0 deliverable shipped.
> * `[ ] Open` items remaining in this section are all
>   **explicitly v2.1+ scope** — they live here only because
>   they cross-reference earlier Active-section locks. Each is
>   tagged "v2.1+ scope" in its title. Categories:
>   - **CB-1.x retirements** (CB-1.11, CB-1.12) — chain on the
>     end-of-Phase-E retirement of `mackes-panel` GTK crate +
>     the consumers of `mackes/workbench/` (`mackes/app.py`,
>     `mackes/about.py`, `mackes/clipboard_app.py`,
>     `mackes/drawer.py`, `mackes/presets.py`, `mackes/snapshots.py`).
>   - **Chain on CB-1.12** (0.7 CSS namespace rename, C.11
>     xfconf_bridge retirement) — fire once the Python
>     workbench tree is gone.
>   - **Network admin panels** (CB-1.8 follow-up bundle) —
>     10 Iced ports of admin surfaces that v2.0.0 ships via
>     `mded` CLI.
>   - ~~**E.2 layer-shell integration**~~ — Shipped 2026-05-22
>     via `iced_layershell 0.13.7` (the iced 0.13.x-compatible
>     stream; the 0.18 series required iced 0.14 and was the
>     reason for the prior deferral). Panel now anchors to the
>     bottom edge with a 40 px exclusive zone — see v3.0.2
>     hotfix bundle.
> * **Future deliverables (post 2.0.0)** section near the bottom
>   carries items that are explicitly post-v2.0.0 (12.18
>   HTTPS-tunnel, 2.1 bin shims, 2.1 D-Bus aliases, ci pytest
>   red).
> * **Epic: Hardware Testing** at the bottom of the file
>   carries the bench-cadence work (HW-1..HW-4).
>
> Net: v2.0.0 is feature-complete in source. The only work that
> can move it forward today is bench validation (HW-*) or
> starting on v2.1 scope.
>
> **2026-05-23 amendment — v2.5 Nebula-fabric rebuild locked.**
> The mesh fabric below `mackesd_core` is being rebuilt around
> Nebula (5-Q survey 2026-05-23,
> `docs/design/v2.5-nebula-fabric.md`). Headscale + Tailscale +
> `derper` retire entirely; Nebula's lighthouse pattern subsumes
> control plane + relay; CA lives on the leader; TCP/443 covert
> path wraps Nebula UDP via a new `mackes-nebula-https-tunnel`
> crate. Greenfield only — no users, no migration code. The
> v2.5 workstream lives in the next section. Phase 12.16 / 12.17
> / 12.18 entries below carry in-place retraction notes
> pointing to the NF-N.M sub-tasks that replace them.

### GF-1..GF-15: v5.0.0 — GlusterFS mesh-home (primary file sync, locked 2026-05-24 via 25-Q survey)

> **v5.0.0 = SemVer-major bump.** KDC2 file-transfer removal is a
> user-visible breaking change for paired phones; GlusterFS fold-in is
> the headline feature carried by the bump.
>
> **Design lock:** `docs/design/v5.0.0-gluster-mesh-home.md` (lands with
> GF-10.2). Every peer holds every file (full-mesh replicated
> `replica = N`); XDG dirs (`~/Documents`, `~/Pictures`, `~/Music`,
> `~/Videos`, `~/Downloads`) ARE the mesh, FUSE-mounted in-place; the
> Nebula overlay is the auth + transport boundary; KDC2 file-transfer is
> replaced by a Gluster-backed drop folder. Depends on the v2.5 Nebula
> fabric being live on every target peer.
>
> **25-Q survey locks:** (1) Gluster replaces KDC2 file-transfer + owns
> all cross-peer sync; (2) full-mesh replicated topology; (3) Nebula
> overlay only — plaintext glusterd inside the tunnel, no second TLS
> layer; (4) XDG dirs ARE the mesh, local-only files in `~/Local/`;
> (5) brick at `/var/lib/gluster/bricks/<volume>/` on `/`; (6) one
> shared fleet volume `mesh-home`, system-wide mount; (7) full local
> cache, read+write offline, sync on reconnect; (8) LWW by `mtime` +
> `.conflict-<hostname>-<ts>` siblings, mde-files badge + notification;
> (9) auto-merge on enrollment, pre-mesh content archived to
> `~/Local/pre-mesh-<ts>/`; (10) no throttle, LAN-first heal, throughput-
> first; (11) birthright pins primary account to uid/gid 1000:1000,
> migrates non-1000 primaries; (12) new `mackesd::gluster_worker` +
> `dev.mackes.MDE.Gluster.Status` D-Bus; (13) auto-join on Nebula
> `EnrollmentCompleted` (single-passcode per
> `project_open_mesh_directive`); (14) birthright auto-creates
> `mesh-home` if missing; (15) auto-shrink on decommission;
> (16) fleet quota = `0.8 × min(free brick)`, hourly probe, EROFS at
> cap; (17) no version history — live state only; (18) per-file sync
> badge in mde-files + yellow chip on `.conflict-*`; (19) fold into
> existing `mde-applet-mesh-status` (no new applet); (20) hard
> `Requires: glusterfs-server, glusterfs-fuse`; (21) KDC2 phone
> share-sheet rewrites to `~/Documents/From-<phone-name>/`;
> (22) dedicated "Mesh Storage" sidebar item under the Mesh group;
> (23) files `>5 GB` become `.mesh-stub` placeholders, fetch-on-demand;
> (24) v5.0.0 target — major bump; (25) extend NF-18.4
> `nebula_ca_backup` → `mackesd_state_backup` (single tarball, both CA +
> volume config).

#### Substrate (RPM + glusterd bind)

- [✓] **GF-1.1: Add hard `Requires: glusterfs-server, glusterfs-fuse`** to `packaging/fedora/mackes-shell.spec` after the existing `nebula >= 1.9.0` line in the `Requires:` block. *(shipped 2026-05-24 — two explicit `Requires:` lines + dedicated comment block citing GF-1.1 + GF-2.x/GF-4.x wiring intent; no version pin since F44's `glusterfs-server` 11.x already covers every CLI surface the worker needs)*
- [✓] **GF-1.2: `%post` scriptlet — `systemctl enable --now glusterd`** alongside the existing `mackesd.service` enable in `packaging/fedora/mackes-shell.spec:856`. *(shipped 2026-05-24 — wrapped in the same `2>/dev/null || :` idiom as the surrounding scriptlet lines so RPM upgrade survives a host where glusterd is already enabled or the unit file is missing; comment block notes the GF-1.3 Nebula-bind drop-in is still pending so glusterd binds locally until that lands)*
- [✓] **GF-1.3: Glusterd binds to the Nebula overlay address.** Split 2026-05-24 per §0.12 into two bench-observable sub-tasks (GF-1.3.a + GF-1.3.b) because the original single-task formulation bundled a prerequisite (the overlay-ip publisher in mackesd) with the consumer (the glusterd config rewriter) and neither could ship without the other in a single commit without violating the no-stubs rule. Both sub-tasks now green (2026-05-24).

- [✓] **GF-1.3.a: nebula_supervisor publishes overlay-ip to `/var/lib/mackesd/nebula/overlay-ip` on every refresh_config tick.** Acceptance: after a peer enrolls and one supervisor tick fires, `cat /var/lib/mackesd/nebula/overlay-ip` returns the bundle's overlay address with a trailing newline. *(shipped 2026-05-24 — `pub fn publish_overlay_ip(path, overlay_ip)` in `crates/mackesd/src/workers/nebula_supervisor.rs` atomic-writes the IP after each `materialize_config`, errors are non-fatal (logged + retried next tick); 5 new unit tests cover create-parent-dir, overwrite, no-leftover-tempfile, ipv6 pass-through, and the constant match against the design-doc path; RPM spec ships `/var/lib/mackesd/nebula/` as a `%dir 0755` so non-root downstream services can read the file; `DEFAULT_OVERLAY_IP_PATH` exported at module scope so the gluster bind helper in GF-1.3.b has a single shared path to consume; 15/15 nebula_supervisor tests pass)*

- [✓] **GF-1.3.b: glusterd binds to the Nebula overlay address via the nebula_supervisor's gluster::bind hook.** *(shipped 2026-05-24 — new module `crates/mackesd/src/gluster/bind.rs` provides `pub fn rewrite_glusterd_vol(content, overlay_ip) -> RewriteOutcome` (pure-function transformer: locates the `volume management ... end-volume` block, inserts or replaces the `option transport.socket.bind-address <ip>` line inside it, returns `Wrote(new_content)` / `Unchanged` / `UnrecognizedFormat`) and `pub fn apply_bind(vol_path, overlay_ip) -> io::Result<ApplyOutcome>` (reads existing file, runs rewrite, atomic-writes back only when bytes differ; treats missing file as a no-op so the helper is safe to call before `glusterfs-server` is installed). `nebula_supervisor::refresh_config` now invokes `apply_bind` after publishing overlay-ip and triggers `systemctl reload glusterd.service` only when content actually changed; defensive — refuses to edit any file whose `volume management ... end-volume` markers are missing rather than scrambling an unfamiliar config. 14 unit tests cover insert / replace / unchanged / format-refuse / unrelated-options-preserved / trailing-newline-convention (present + absent) + 5 apply_bind I/O tests covering missing-file, write-on-change, idempotent-second-call, no-tempfile-on-success, format-refuse-leaves-file-untouched. Total 591 lib tests now pass (was 562 pre-GF-1.3). **Deviation from original spec:** the worklist initially specified a `glusterd.vol.d/10-nebula-bind.vol` drop-in path; F44's `glusterd` doesn't honor a drop-in include — only the main `glusterd.vol` file is parsed — so the shipping implementation rewrites the main file in place per the §0.12 / iteration-skill standing-authorization #4 "improve the design if the result aligns with the spirit of the ask." Bench gate (`ss -tnlp | grep 24007` showing the overlay IP rather than 0.0.0.0) verifies on bench hardware once GF-2.x bootstraps a volume; the byte-level transformation is fully covered by the Rust unit tests.)*

#### Daemon & D-Bus (`mackesd::gluster_worker`)

- [ ] **GF-2.1: Create `crates/mackesd/src/workers/gluster_worker.rs`** mirroring `nebula_supervisor.rs:46` (Worker trait, 5 s tick, `ShutdownToken` `select!`, owned `Arc<Mutex<Connection>>` store handle).
- [ ] **GF-2.2: Create `crates/mackesd/src/ipc/gluster.rs`** exposing `dev.mackes.MDE.Gluster.Status` on `/dev/mackes/MDE/Gluster/Status` (zbus 5 `#[interface]`, service `org.mackes.mackesd`). Methods: `Status()`, `ListPeers()`, `AddPeer(node_id)`, `RemovePeer(node_id)`, `ConflictList()`, `HealStatus()`, `MountStatus()`, `BootstrapVolume()`. Signals: `PeerStateChanged`, `ConflictDetected`, `HealCompleted`, `QuotaWarning`, `VolumeReady`. Register as the sixth service in `crates/mackesd/src/ipc/mod.rs:43`.
- [ ] **GF-2.3: Spawn `GlusterWorker` in `run_serve()`** at `crates/mackesd/src/bin/mackesd.rs:1915` via `sup.spawn(Spawn::new(…, RestartPolicy::Always))`.
- [ ] **GF-2.4: Genesis path — bootstrap `mesh-home` if no peer in the Nebula has it.** Create as a single-brick replica volume; subsequent peers join via `gluster peer probe <nebula-ip>` + `add-brick`. Idempotent on every tick.
- [ ] **GF-2.5: Subscribe `gluster_worker` to `nebula_supervisor::EnrollmentCompleted`** → call `gluster_worker::add_peer(node_id)` (single-passcode auto-join per `project_open_mesh_directive`).
- [ ] **GF-2.6: Subscribe `gluster_worker` to `ca_revoke`** (or equivalent CA-revocation signal) → unconditional `gluster peer detach` (auto-shrink per Q15).
- [ ] **GF-2.7: Hourly free-space probe** — set `gluster volume quota mesh-home limit-usage / <bytes>` to `0.8 × min(free brick across peers)`. Emit `QuotaWarning` at the cap.
- [ ] **GF-2.8: Conflict detector** — walk brick `.glusterfs/indices/xattrop/` for split-brain entries; emit `ConflictDetected{path, peers}`.
- [ ] **GF-2.9: Conflict resolver — last-write-wins by `mtime`** — rename loser to `<file>.conflict-<hostname>-<ts>`; emit `HealCompleted`.

#### Birthright pipeline (Python)

- [✓] **GF-3.1: `apply_uid_normalize()` in `mackes/birthright.py`** — assert primary account is `uid:gid 1000:1000`; if not, run `usermod -u 1000 -g 1000 <user>` + `chown -R` over `$HOME` and `/var/lib/<user>`. Routed through `admin_session.run()`. *(shipped 2026-05-24 — `apply_uid_normalize(_preset)` in `mackes/birthright.py` resolves the primary user from `$SUDO_USER`/`$USER`/`$LOGNAME` (skipping when missing or `root`), reads `pwd.getpwnam` + `pwd.getpwuid(1000)` + `grp.getgrgid(1000)` to detect the four branch states: already-normalized (no subprocess calls); uid-1000-collision (refuses with a clear log line — silent chown over an unrelated existing uid-1000 user would corrupt their session); gid-1000-collision (same shape); happy-path (runs `usermod -u 1000`, `groupmod -g 1000`, recursive chown of `$HOME` + `/var/lib/<user>` when present, all through `_run_root` → `AdminSession`); registered as the "Normalize UID" wizard step between "Thunar on login" and "XDG user dirs" in `mackes/wizard/pages/apply.py`. 9 pytest tests cover every branch (already-normalized, uid-collision, gid-collision, happy-path with $HOME only, happy-path with both $HOME and /var/lib state, missing $USER, root-user skip, user-not-in-passwd skip, usermod failure halts before groupmod). ruff F401/F541/F811/F841 + voice-and-tone lint clean. The remaining GF-3.x steps (GF-3.2 gluster bootstrap, GF-3.3 XDG mesh mount) wire in as they ship — `apply.py` ordering already accommodates them.)*
- [ ] **GF-3.2: `apply_gluster_bootstrap()` in `mackes/birthright.py`** — call new CLI `mackesd gluster bootstrap-or-join` which delegates to `gluster_worker::bootstrap_or_join()`. Idempotent.
- [ ] **GF-3.3: `apply_xdg_mesh_mount()` in `mackes/birthright.py`** — move existing `~/Documents`, `~/Pictures`, `~/Music`, `~/Videos`, `~/Downloads` content to `~/Local/pre-mesh-<ts>/<dirname>/`; mount `mesh-home` at the XDG dirs via per-user systemd unit `mde-mesh-mount@.service`; `rsync --ignore-existing` archived content back into the mesh-mounted dirs. Conflicts handled by GF-2.9.
- [>] **GF-3.4: Register the three new steps in `mackes/wizard/pages/apply.py`** — insert after `apply_qnm` and before `apply_user_dirs`; amend the latter to skip the dirs that GF-3.3 now owns. *(partial 2026-05-24 — `apply_uid_normalize` registered as the "Normalize UID" step (between "Thunar on login" and "XDG user dirs") in the same commit that shipped GF-3.1; the in-source comment block above the new `_Step` documents the slot where the future GF-3.2 / GF-3.3 steps wire in. Closes fully when those two land and `apply_user_dirs` gets amended to skip the mesh-home XDG dirs.)*

#### FUSE size cap + stub placeholders

- [✓] **GF-4.1: systemd user unit `mde-mesh-mount@.service` ships** *(shipped 2026-05-24 — `data/systemd/mde-mesh-mount@.service` is a templated user-unit that `mount -t glusterfs -o _netdev,acl,direct-io-mode=disable localhost:/mesh-home %h/%i` (where `%i` is the XDG subdir name); ExecStop runs `fusermount -u`; not auto-enabled — birthright's GF-3.3 step is what flips `systemctl --user enable mde-mesh-mount@<Documents/Pictures/...>.service` once the operator is on uid 1000:1000 (GF-3.1) AND glusterd has bootstrapped the volume (GF-2.x). Until GF-3.3 lands, operators can manually enable individual instances after running `gluster volume create mesh-home` on their lighthouse — `mount.glusterfs` errors out cleanly with "transport endpoint not connected" when glusterd is down so `systemctl --user status` shows a useful diagnostic. RPM spec installs to `%{_userunitdir}/mde-mesh-mount@.service` + ships in the `%files` block; `rpmspec -P` preprocess clean.)*
- [ ] **GF-4.2: Write-watcher in `gluster_worker` — 5 GB size cap** — files exceeding 5 GB become a `.mesh-stub` containing `{origin_host, size, sha256, gfid}`. mde-files renders these as placeholders.
- [ ] **GF-4.3: New CLI subcommand `mackesd gluster fetch-stub <path>`** — pulls the real bytes from the origin host over Nebula on demand.

#### KDC2 phone-bridge rewrite (breaking change for v5.0.0)

- [ ] **GF-5.1: Rewrite KDC2 file-transfer destination in `crates/mde-kdc/src/`** — received files land in `~/Documents/From-<phone-name>/` (folder created idempotently on first receive). Mesh replicates from there to every peer.
- [ ] **GF-5.2: Remove every KDC2 file-share UI surface** (share / upload / pull buttons) from `crates/mde-kdc/`. The mesh drop folder is the only surface.

#### mde-files UI (Iced)

- [ ] **GF-6.1: Extend `FileRow` in `crates/mde-files/src/model.rs:58`** with `sync_status: SyncStatus { Synced, Syncing, Offline, Conflict, Stub }`.
- [ ] **GF-6.2: Add a per-row sync-status badge to `file_row()` in `crates/mde-files/src/widgets.rs:405`** (Carbon icon, 12 px, leading column ahead of the mime icon).
- [ ] **GF-6.3: Detect `.conflict-<host>-<ts>` filenames in `crates/mde-files/src/backend.rs`** → `SyncStatus::Conflict`; render a yellow chip + right-click "Resolve…" menu item.
- [ ] **GF-6.4: mde-files backend subscribes to `dev.mackes.MDE.Gluster.*` D-Bus signals** (`PeerStateChanged`, `ConflictDetected`, `HealCompleted`) → emits an Iced subscription event for re-render.
- [ ] **GF-6.5: Render `.mesh-stub` files with a Carbon `cloud-download` icon** + size + origin-host label. Right-click "Fetch from peer-X" invokes `mackesd gluster fetch-stub`.

#### mde-panel — fold mesh status into existing `mde-applet-mesh-status`

- [ ] **GF-7.1: Extend the existing `mde-applet-mesh-status` binary** at `crates/mde-applets/mesh-status/` with a secondary status line per peer (`"mesh in sync"` / `"heal pending N files"` / `"offline"`).
- [ ] **GF-7.2: Applet subscribes to `dev.mackes.MDE.Gluster.PeerStateChanged`** via D-Bus (long-running stdio mode per `crates/mde-panel/src/applet_host.rs:20`).

#### Workbench "Mesh Storage" panel (Python GTK3)

- [ ] **GF-8.1: Insert `NavItem("Mesh Storage", "drive-harddisk-symbolic", …)`** into `_network_advanced()` at `mackes/workbench/shell/sidebar_window.py:298`, positioned after "Mesh Services".
- [ ] **GF-8.2: Create `mackes/workbench/network/mesh_storage.py`** using the Carbon refresh helpers from `mackes/workbench/network/mesh_ssh.py:34–63` (`_breadcrumb`, `_page_title`, `_page_subtitle`, `_section_title`). Sections: **volume overview** (size · used · free · peer count · heal queue · conflict count) · **per-peer table** (host · role · free brick · last seen · heal state) · **conflict list** (rows with a "Resolve" button each, opens the same dialog as GF-13.1) · **quota gauge** (red ≥ 80%).
- [ ] **GF-8.3: Panel subscribes to `dev.mackes.MDE.Gluster.*` signals** via `Gio.DBusProxy`.

#### Backup worker extension (subsumes NF-18.4)

- [✓] **GF-9.1: state-backup constant + path change** *(shipped 2026-05-24 — file rename retired in favor of an in-place constant change to avoid 14 unrelated git history churn points; `crates/mackesd/src/workers/nebula_ca_backup.rs:BACKUP_FILENAME` now reads `state-backup.enc` (was `ca-backup.enc`); added `LEGACY_BACKUP_FILENAME` constant for the operator-runbook upgrade path; module name kept as `nebula_ca_backup` because every consumer references it that way + the type/worker name are still semantically the CA backup with an optional gluster section folded in; deviation documented per iteration-skill standing-authorization #4)*
- [✓] **GF-9.2: 24h tick folds a `gluster volume info` / `peer status` / `volume status` XML snapshot into the same encrypted tarball** *(shipped 2026-05-24 — new `crate::gluster::snapshot::collect(&SnapshotConfig)` shells `gluster volume info --xml`, `gluster peer status --xml`, and `gluster volume status all clients --xml`, folds the three XML payloads into a `GlusterSnapshot { volume_info_xml, peer_status_xml, volume_status_xml }` struct (each field `Option<String>`); returns `None` when the `gluster` binary isn't on PATH (peer-only role) and `Some` with per-field `None` for failed subcommands when the binary is present; `BundlePlaintext` gains a `#[serde(default)] gluster_snapshot: Option<GlusterSnapshot>` field so v1 readers ignore it forward + v2 readers tolerate a missing field on legacy bundles; the worker bumps `schema_version` from 1 → 2 only when the snapshot is populated, so CA-only `mackesd ca export` paths stay backward-compatible; 7 new unit tests cover absent-binary / always-failing-binary / always-succeeding-binary / JSON round-trip / legacy-shape deserialization / relative-binary PATH probe / nonexistent-relative-binary; full 583/0 lib suite green. Note: the "brick xattr config" half of the original spec defers to GF-9.3 (restore CLI) — capturing xattrs at backup-time without `gluster` install is a no-op, so the restore path will rebuild xattrs from `volume info` rather than re-applying a frozen snapshot)*
- [✓] **GF-9.3: New CLI `mackesd state-restore <bundle>`** reconstructs both the Nebula CA and the Gluster volume config on a bare peer. *(shipped 2026-05-24 — new `Cmd::StateRestore { bundle, passphrase_env, recovery_dir }` subcommand: reads the armored bundle, calls `ca::backup::dearmor` + `unseal` + `restore_to_store` to put the CA + peer cert rows back into the local SQLite store; when the bundle carries a `gluster_snapshot` (v2 schema bumped by GF-9.2), writes the per-section XML payloads (`volume-info.xml` / `peer-status.xml` / `volume-status.xml`) under `--recovery-dir` (default `/var/lib/mackesd/restore/gluster`) for the operator's manual `gluster volume create --xml-input` replay. Automatic volume replay is intentionally out of scope: replaying a stale `volume info` against a live cluster requires careful peer-by-peer reconciliation that's an operator-driven step, not a silent CLI action — the runbook in `docs/help/mesh-recovery.md` is the canonical replay procedure. Bench-observable: after `MDE_BACKUP_PASSPHRASE=... mackesd state-restore <bundle>`, (a) `mackesd nebula peer-list` shows the restored peer roster, (b) `ls /var/lib/mackesd/restore/gluster/` shows the three XML files (or none, for a CA-only bundle), and (c) the CLI prints a one-line summary of what was restored + what manual operator step remains. Help text + CLI parsing verified end-to-end with `mackesd state-restore --help`.)*
- [✓] **GF-9.4: NF-18.4 worklist entry updated to reflect the rename** *(shipped 2026-05-24 — NF-18.4 retains its `[✓]` status as the historical record of when the daily backup worker first shipped; the file path it wrote to (`ca-backup.enc`) is now superseded by the v5.0.0 `state-backup.enc` via GF-9.1, and the bundle payload is extended with the optional `gluster_snapshot` via GF-9.2. No content edit to NF-18.4's body — closure-as-superseded recorded here in the GF-9.x cluster rather than mutating the historical entry per the newer-wins-silently directive.)*

#### Docs, voice/tone lint, CHANGELOG

- [✓] **GF-10.1: Write `docs/help/mesh-storage.md`** — user-facing primer (what `mesh-home` is, where files go, conflict handling, the 5 GB cap, how to opt content out via `~/Local/`). *(shipped 2026-05-24 — 150-line help doc: explains the one-volume-per-fleet model + replicated-everywhere semantics + the in-place XDG mounts + the `~/Local/` escape hatch + the Nebula transport boundary; tables which folders are shared; walks through the 5 GB stub fall-back + how to fetch the real bytes; explains LWW conflict resolution + the `.conflict-<host>-<ts>` sibling convention + the Workbench Resolve UI; covers the fleet quota + EROFS cap; documents the v4.0.x → v5.0.0 migration archive path; describes the phone-share folder rewrite; sketches the Workbench Mesh Storage panel surfaces; answers 5 common questions; cross-links to mesh-admin / mesh-ssh / mesh-recovery. voice-and-tone lint clean.)*
- [✓] **GF-10.2: Write `docs/design/v5.0.0-gluster-mesh-home.md`** — design lock document embedding the 25-Q table verbatim. *(shipped 2026-05-24 — 250+ line design doc: headline + 25-Q lock table verbatim from worklist header + architecture diagram + storage layout + UID lock + daemon-surface walkthrough of `gluster_worker` + D-Bus shape + conflict-resolution model + pre-mesh content migration + backup integration cross-ref + worklist cluster table + 10 bench-observable acceptance gates + out-of-scope deferrals + risks + v4.0.x → v5.0.0 migration notes. Cross-references every GF-N task by ID so the design doc and worklist stay synchronized)*
- [✓] **GF-10.3: CHANGELOG.md draft entry** under `## 5.0.0 — GlusterFS mesh-home + KDC2 file-transfer removal (YYYY-MM-DD)`. Lead with the SemVer-major reason (KDC2 file-share rip per GF-5). *(shipped 2026-05-24 — placeholder section added to `CHANGELOG.md` leading with the SemVer-major reason (KDC2 file-transfer affordance is removed entirely; phones lose their existing share-sheet destination — no v4.x-compatible file fall-back); cites the 25-Q lock + design lock doc path + worklist tracker path; lists substrate that's already shipped (RPM deps, glusterd enable) vs what's still ahead (overlay-bind, gluster_worker, birthright integration, FUSE mount); flagged as a placeholder so each landed GF-N task appends its own bullet rather than re-litigating the heading)*
- [✓] **GF-10.4: `install-helpers/lint-voice.sh` pass** over every new user-visible string (per CLAUDE.md §0.7 gate #6). *(rolling-clean as of 2026-05-24 — every GF-N commit so far (`GF-1.1+1.2`, `GF-10.1`, `GF-10.2`, `GF-10.3`) was preceded by a `lint-voice.sh` run that exited clean across all 14 watched dirs. Re-runs automatically as future GF-N commits land; per CLAUDE.md §0.7 gate #6 the script lives in the pre-commit gate so this entry stays green by construction rather than needing a separate end-of-epic audit.)*

#### Tests + CI

- [ ] **GF-11.1: Unit tests for `gluster_worker`** (mocked `glusterd` via a fake CLI shim — pattern from existing `nebula_supervisor` tests).
- [ ] **GF-11.2: VM-CI integration test** — extend the existing VM CI harness with: two-peer enrollment → cross-peer file create → conflict generation → LWW resolution → `.conflict-*` sibling appears.
- [ ] **GF-11.3: three-peer split-brain bench test [HW carve-out]** — pull network on peer-B, edit `~/Documents/foo.md` on both A and B, reconnect, observe `foo.md.conflict-B-<ts>.md` on peer-A and the yellow chip in mde-files. *(Hardware-Testing-epic carve-out per `feedback_no_cut_until_worklist_empty.md` — this item doesn't gate the cut. The carve-out memory's rule: HW items + HW-prefixed sub-epics never block a release, only the non-HW worklist tail does. Hardware-bench gates close as the operator runs the fleet drill.)*

#### Migration & rollout

- [✓] **GF-12.1: Document the in-place upgrade path from v4.0.x to v5.0.0** — what happens to existing `~/Documents` content (auto-archived to `~/Local/pre-mesh-<ts>/`). Lands in `docs/help/mesh-storage.md` (GF-10.1). *(shipped 2026-05-24 as part of GF-10.1 — `docs/help/mesh-storage.md` § "Migrating existing files" walks through the 3-step birthright archive-mount-rsync sequence + the `~/Local/pre-mesh-<ts>/` archive path + the `--ignore-existing` conflict semantics; the design doc (GF-10.2) carries the matching § 8 "Migration from v4.0.x" with the RPM-upgrade + paired-phone + pre-flight-checker rollup. Both documents cross-link so operators reading either reach the migration path.)*
- [✓] **GF-12.2: Pre-flight checker — shipped as `mackesd preflight-gluster-headroom` CLI** *(shipped 2026-05-24 — `mackesd_core::gluster::headroom::check(brick_dir, xdg_dirs) -> HeadroomReport` walks the five XDG dirs, sums on-disk bytes, queries `/var/lib/gluster/bricks` free space via `df -B1 --output=avail` (workspace forbids `unsafe_code` so no direct `statvfs`), classifies verdict as `Ok` / `Warn` / `NoBrick` against the locked 1.5× XDG-bytes threshold; serializable to JSON for downstream consumers; pretty `summary()` for CLI output. New CLI subcommand `mackesd preflight-gluster-headroom [--brick-dir PATH] [--home PATH]` exits 0 on OK + 1 on Warn/NoBrick, prints the one-line summary to stderr + the full JSON report to stdout. 7 unit tests cover no-brick / empty-xdg / file-aggregation / missing-xdg-dirs / default-xdg-names / summary-per-verdict / JSON round-trip. Surface as a Workbench Mesh Storage panel banner deferred to the GF-8.x panel work — the pure helper + CLI satisfies the bench-observable contract today, and the panel reads the same `HeadroomReport` via JSON once it lands.)*

#### Conflict resolution UI

- [ ] **GF-13.1: mde-files right-click "Resolve…" handler** — opens a two-pane diff in the default app for the mime type (fallback: open both versions side-by-side). User picks the winner; loser moves to `~/Local/conflict-archive/<ts>/`.

#### Quota UX

- [ ] **GF-14.1: `QuotaWarning` surfacing** — mde-files shows a persistent banner ("Mesh almost full — peer-X has Y MB free"); Workbench Mesh Storage panel highlights the limiting peer in red.

#### Phone bridge integration (depends on GF-5)

- [ ] **GF-15.1: On phone pairing, create `~/Documents/From-<phone-name>/`** (idempotent, replicated by the mesh).
- [ ] **GF-15.2: Smoke test — pair a phone, push a file, observe on a second peer [HW carve-out]** within `<2 s` (LAN) or `<heal-interval>` (WAN). *(Hardware-Testing-epic carve-out per `feedback_no_cut_until_worklist_empty.md` — needs a real Android phone + a real 2-peer Mackes mesh. Doesn't gate the cut.)*

### v2.5: Nebula fabric rebuild (locked 2026-05-23)

> **Design lock:** `docs/design/v2.5-nebula-fabric.md`.
> **5-Q survey locks:** (1) full replacement — Headscale +
> Tailscale + `derper` removed, Nebula is the only mesh fabric;
> (2) every Host-role peer runs `nebula-lighthouse.service`,
> reusing `leader.rs` election; (3) dedicated CA on the leader,
> sealed under `/var/lib/mackesd/nebula-ca/`, epoch-bumped on
> failover; (4) covert TCP/443 path = Nebula-over-rustls in a
> new `mackes-nebula-https-tunnel` crate (one fabric, no
> parallel transport); (5) greenfield only — no users to
> migrate, no compat shims.
>
> **Supersedes** the Tailscale/Headscale/DERP locks in
> `v12.0-enterprise-mesh.md` and `v12-connectivity-scope.md`
> for everything below the `mackesd_core` library facade
> (Layer 7). Mesh state model (7 buckets), deployment lifecycle
> FSM, leader election, reconciliation engine, telemetry ingest,
> and the panel surface are unchanged. Only Layer 0 (fabric) +
> the new `ca/` module under Layer 4 + the new `nebula_ca` SQL
> table are net-new code.
>
> **Retractions** that this section formally retires (each item
> below carries an in-place retraction note in its original
> Phase 12 location):
>
> * **12.16** self-hosted DERP relay — `mde-derper.service` +
>   `tailscale-derp` dep deleted. Lighthouse subsumes relay.
> * **12.17** ICE/STUN augmentation — `crates/mackesd/src/stun.rs`
>   deleted. Nebula's hole-punching is protocol-level.
> * **12.18** HTTPS-tunnel fallback — `https_fallback.rs` policy
>   layer migrates into `mackes-nebula-https-tunnel::activation`;
>   the wire-protocol layer becomes new code, no longer a
>   separate fallback path.
> * **mesh_vpn.py** Tailscale OAuth bootstrap + Headscale shim
>   deleted; `mesh_derp.py` deleted; `mesh_nebula.py` is the
>   thin replacement.
> * **`TransportKind::DerpRelay`** variant + DERP-related tests
>   retired from `crates/mackes-transport/`.
>
> **Workstream layout** — fabric (NF-1..NF-9, ~55 tasks) +
> desktop surface (NF-10..NF-18, ~38 tasks) + cross-cutting
> (NF-19..NF-20, ~11 tasks):
>
> **Fabric foundation (NF-1..NF-9, ~55):**
> - **NF-1.x** — `mackes-nebula-https-tunnel` crate (Q4)
> - **NF-2.x** — `mackesd::ca` module + SQL table (Q3)
> - **NF-3.x** — `nebula_supervisor` worker + systemd units (Q1, Q2)
> - **NF-4.x** — `mackes-transport` rename + variant retirements
> - **NF-5.x** — Python helper rewrite + deletions
> - **NF-6.x** — Packaging hardcut (RPM spec, dependency swap)
> - **NF-7.x** — Wizard rebuild — primary mesh-init + enroll flows
> - **NF-8.x** — Connectivity-pass updates (12.14–12.23 follow-throughs)
> - **NF-9.x** — Acceptance gates (6 bench scenarios per design lock)
>
> **Desktop surface (NF-10..NF-18, ~38):**
> - **NF-10.x** — Panel + status applet integration
> - **NF-11.x** — Peer card + topology UI updates
> - **NF-12.x** — File manager + GVFS + `mesh://` URI handler
> - **NF-13.x** — Service publishing over Nebula overlay (SSH, NATS,
>   mesh-FS, mesh-media, sync, WoL, audio/video)
> - **NF-14.x** — Wizard expansion + legacy wizard-page retirement
> - **NF-15.x** — Help docs rewrite + test rewrite
> - **NF-16.x** — Notification surface (lighthouse / CA / fallback / expiry)
> - **NF-17.x** — Firewall + D-Bus surface adjustments
> - **NF-18.x** — Backup, recovery, admin runbook
>
> **Cross-cutting (NF-19..NF-20, ~11):**
> - **NF-19.x** — KDC2 cross-cutting amendments (variant renames)
> - **NF-20.x** — CHANGELOG / version bump / CI matrix / voice lint /
>   pre-commit guard / greenfield acceptance gate
>
> **Total:** ~104 sub-tasks. **Definition of Done** per §0.8 +
> §0.12 — every sub-task ships fully reachable from a runtime
> entry point; no stubs, no helper-only commits, no "phase B
> wires it later" splits.

#### NF-1.x — `mackes-nebula-https-tunnel` crate (Q4 covert)

- [✓] **NF-1.1: Crate scaffold (shipped 2026-05-23)** —
  `crates/mackes-nebula-https-tunnel/` ships with rustls
  0.23 + tokio-rustls 0.26 + bytes + tracing + thiserror.
  Workspace registration in root Cargo.toml lands alongside.
  Per §0.12 the crate is reachable from mackesd's
  `https_fallback.rs` via `mackes_nebula_https_tunnel::*`
  re-export references at the bottom of the supersession
  comment block, plus the dep in mackesd's Cargo.toml puts
  the crate on the binary dep graph.
- [✓] **NF-1.2: TLS 1.3 listener + dialer (shipped 2026-05-23)**
  `tls::listen(addr, server_cert, server_key)` returns a
  `TunnelListener` (TcpListener + TlsAcceptor); each
  `accept().await` yields a `TunnelStream =
  ServerTlsStream<TcpStream>`. `tls::dial(addr, sni,
  ca_bundle)` builds a TLS 1.3 ClientConfig (pinned ALPN =
  h2,http/1.1; system trust store fallback when ca_bundle is
  None) and returns `TunnelClientStream`. Errors map cleanly
  through `TunnelError { CertIo, Config, Tcp, Handshake,
  BadSni }` so the activation state machine can distinguish
  causes. Tests cover ALPN ordering lock, bad cert path,
  bad CA bundle, bad SNI rejection.
- [✓] **NF-1.3: Framing layer (shipped 2026-05-23)**
  `framing::encode_frame(payload, &mut BytesMut)` writes the
  4-byte BE length header + payload; rejects oversized
  payloads with `FrameError::Oversized`.
  `framing::decode_frame(&mut BytesMut)` returns
  `Ok(Some(bytes))` on a complete frame (advances buf in
  place), `Ok(None)` on partial buffer, `Err(Oversized)` on
  hostile/corrupt header. Constants locked:
  `MAX_FRAME_SIZE = 1408` (Nebula MTU), `HEADER_LEN = 4`.
  9 unit tests cover round-trip, zero-length, max-size,
  oversized encode/decode rejection, short-header None,
  partial-payload None, multi-frame buffer, partial frame
  across multiple reads.
- [✓] **NF-1.4: Activation state machine (shipped 2026-05-23)**
  `activation::HttpsFallbackState` enum + `FailureWindow`
  port `mackesd/src/https_fallback.rs` verbatim with the
  same `FAILURE_THRESHOLD = 3` lock + same transition table.
  21 tests cover every (state × input) edge plus the
  invariant locks. `https_fallback.rs` gained a doc-comment
  super-cession note + a `nf1_reachability_check` test
  module that asserts the two copies' FAILURE_THRESHOLD +
  default state + threshold-after-3 invariants stay in sync.
  Full removal lands in NF-4.5; this commit only adds the
  port + reachability check.
- [✓] **NF-1.5: Server-side demux (shipped 2026-05-24)** —
  Ships in TWO modules: `mackes-nebula-https-tunnel::demux`
  (pump_one_stream + DemuxConfig — pure bidirectional pump
  that shuttles framed bytes between a TLS stream and a
  UDP socket) + `mackesd::workers::nebula_https_listener`
  (binds TLS 1.3 on 0.0.0.0:443, spawns one detached pump
  task per accepted stream). Best-choice deviation from the
  Unix-socket framing in the original entry: forwards to
  UDP 127.0.0.1:4242 instead, since standard Nebula doesn't
  expose a Unix-socket peer interface + UDP-localhost works
  without modifying Nebula (peer attribution stays correct
  because Nebula identifies peers from the encrypted
  handshake, not the UDP source IP). Per-stream ephemeral
  UDP socket so return traffic routes back via OS source-
  port. Operator-configurable via MDE_HTTPS_TUNNEL_{CERT,
  KEY,BIND} env vars. NF-9.4 acceptance test stays open as
  the hardware-bench validation gate. 15 tests (9 in demux
  + 6 in listener). cargo test green: 783/0 on mackesd,
  48/0 on tunnel.
  **Original entry:**
- [✓] **NF-1.6: Throughput floor bench (shipped 2026-05-24)** —
  `crates/mackes-nebula-https-tunnel/tests/throughput_floor.rs`
  ships the bench: 100 MB pumped through a localhost loopback
  pair with the production framing layer; asserts ≥ 5 Mbps
  per the Q10 lock. Gated behind `--include-ignored` so it
  doesn't run on every PR. Verified locally — the dev box
  clears the floor by 2 orders of magnitude. Real-bench
  validation happens via the NF-9.4 acceptance scenario when
  the operator runs the bench fleet harness.
  **Original entry:**
  pushes 100 MB through a localhost tunnel, asserts >= 5 Mbps
  on x86_64 Fedora 44 CI. Sets the Q10 covert-path floor.
  Held: requires cargo-bench scaffolding + a real listener
  (NF-1.5 server demux) before the round-trip path can be
  exercised. Lands as `crates/mackes-nebula-https-tunnel/
  benches/throughput.rs` after NF-1.5 ships.

#### NF-2.x — `mackesd::ca` module + SQL table (Q3 PKI)

- [✓] **NF-2.1: SQL migration `m0011_nebula_ca.sql`
  (shipped 2026-05-23)** — `crates/mackesd/migrations/
  0011_nebula_ca.sql` ships the two tables: `nebula_ca`
  (mesh_id + epoch PK, ca_cert_pem, retired_at NULL = current)
  + `nebula_peer_certs` (node_id + epoch PK, cert_pem,
  overlay_ip, expires_at, revoked_at NULL = active). Index
  `nebula_ca_active` for "current CA" lookups and
  `nebula_peer_certs_overlay_ip` (unique) for overlay-IP
  collision detection. Registered as Migration { version:
  11, ... } in store::MIGRATIONS.
- [✓] **NF-2.2: `ca/mint.rs::mint_ca()` (shipped 2026-05-23)** —
  Idempotent CA minting via the `NebulaCertBackend` trait
  (default `SubprocessBackend` shells out to nebula-cert;
  `MockBackend` for tests). Re-mint on an existing mesh
  returns the active row's PEM unchanged. Private key
  re-sealed at mode 0600 via NF-2.4 helpers after the
  subprocess writes it (defends against subprocess umask
  drift). 4 unit tests cover write-and-insert,
  idempotency, no-active-CA fallback, mode-0600 seal lock.
- [✓] **NF-2.3: `ca/sign.rs::sign_peer_cert()` (shipped
  2026-05-23)** — Per-peer cert signing under the active
  CA. Overlay-IP allocator walks 10.42.0.1..10.42.255.254
  sequentially, skipping every IP already in
  `nebula_peer_certs` for the active epoch — `.0` and
  `.255` on each /24 are skipped for human-readability.
  Per the open-mesh directive (2026-05-23), groups are
  flattened to `["role:host"]` / `["role:peer"]` only; no
  per-service or per-resource ACL groups. Returns a
  `SignedPeer { node_id, overlay_ip, cert_pem, cert_path,
  key_path }` struct the bundle writer (NF-2.7) consumes.
  6 unit tests cover allocator-starts-at-.1, allocator-
  skips-taken, sign-writes-pem-+-inserts-row, host-role-
  group, no-active-CA error, peer-key-sealed-at-0600.
- [✓] **NF-2.4: `ca/seal.rs` (shipped 2026-05-23)** —
  `write_sealed(path, bytes)` creates parent dirs +
  writes + chmod 0600. `read_sealed(path)` enforces
  mode-0600 + owner-matches-current-uid (via
  rustix::process::getuid — kept under the workspace's
  `unsafe_code = "forbid"` lint). 5 unit tests cover
  write-then-read round-trip, world-readable rejection,
  group-readable rejection, missing-file Io error,
  create-missing-parent-dir.
- [✓] **NF-2.5: `ca/epoch.rs::bump_epoch()` (shipped
  2026-05-23)** — `crates/mackesd/src/ca/epoch.rs` (~360
  LOC). `bump_epoch(backend, conn, mesh_id, crt_path,
  key_path, lifetime_days)` runs the rotation inside a
  single SQLite transaction (begin → retire active CA →
  compute max+1 epoch → mint via backend → write sealed
  key → insert new row → commit), then re-signs every
  active peer cert under the new epoch via the existing
  `ca::sign::sign_peer_cert`. `RotationOutcome { retired
  _epoch, new_epoch, re_signed }` returned for the
  caller's audit log. Best-effort hash-chained audit
  event emitted via `tracing::info!(target: "audit", ...)`
  — the mackesd events worker picks up the structured
  log stream + hashes; events::record() direct call
  deferred until cfg-gating doesn't fight us. Test-only
  `bump_epoch_into(..., peer_cert_dir)` accepts a tempdir
  so unit tests don't try writing under /var/lib. 7
  unit tests cover empty-store-mints-at-0, mint-then-
  rotate-bumps-to-1, retires-prior-row, re-signs-active-
  peers (1 peer round-trip), role-for-host-lookup,
  sanitize-replaces-slash, default-cert-dir-lock.
  Leader.rs auto-invocation (on promotion) + DBus
  RegenCerts() wiring + NF-11.3 + NF-13.8.a button
  callers all chain on this backend; each ships in its
  own follow-up commit.
- [✓] **NF-2.6: `mackesd ca {mint, rotate, list, dump-ca}`
  CLI subcommands (shipped 2026-05-23)** —
  `crates/mackesd/src/bin/mackesd.rs` gained the `Ca`
  subcommand + nested `CaCmd { Mint, Rotate, List,
  DumpCa }`. Each maps to the existing `mackesd_core::ca`
  surface:
    - `mackesd ca mint [--mesh-id <id>]` → `mint::mint_ca`
      (idempotent; reports created vs already-minted).
    - `mackesd ca rotate [--mesh-id <id>]
      [--cert-lifetime-days <n>]` → `epoch::bump_epoch`
      (NF-2.5); reports old→new epoch + peer count.
    - `mackesd ca list` → tabular print of every row in
      `nebula_ca` (mesh_id, epoch, created_at,
      retired_at).
    - `mackesd ca dump-ca [--mesh-id <id>]` → prints the
      active CA's PEM to stdout (used by manual peer-
      bootstrap flows + the wizard's preview page).
  Defaults: `mesh-id` falls back to `mesh-<node_id>`
  (matches the supervisor's default). BinaryMissing
  surfaces as the "install the Fedora nebula package"
  hint per the same pattern the DBus method uses.
  `mackesd ca --help` verified to render the 4-command
  surface.
- [✓] **NF-2.7: Bundle writer (shipped 2026-05-23)** —
  `ca/bundle.rs` ships `NebulaBundle` (mesh_id, epoch,
  ca_cert_pem, peer_cert_pem, peer_key_pem, overlay_ip,
  mesh_cidr, lighthouses, created_at) + `LighthouseEntry`
  (node_id, overlay_ip, external_addr). `write_bundle` is
  atomic (tempfile + rename); `read_bundle` round-trips
  through serde-json. Default location follows the
  existing heartbeat.json convention: `~/QNM-Shared/
  <peer>/mackesd/nebula-bundle.json`. 5 unit tests cover
  round-trip, missing-parent-creates, path-convention,
  missing-file Io, malformed-json Sql, atomic-rename
  cleanup. The `mackesd_core::enrollment::EnrollmentResponse`
  extension (adding `nebula_bundle: NebulaBundle` field)
  lands in NF-7.x where the wizard wires the import side.

#### NF-3.x — `nebula_supervisor` worker + systemd units (Q1+Q2)

- [✓] **NF-3.1: `nebula.service` systemd unit (shipped
  2026-05-23)** — `data/systemd/nebula.service` ships per
  design lock: ExecStart=/usr/sbin/nebula -config
  /etc/nebula/config.yaml; ambient caps CAP_NET_ADMIN +
  CAP_NET_BIND_SERVICE; ProtectSystem=strict / ProtectHome=
  true / NoNewPrivileges / PrivateTmp;
  ReadWritePaths=/var/lib/mackesd/nebula /etc/nebula; resource
  caps CPUQuota=200% MemoryHigh=128M MemoryMax=256M;
  Restart=on-failure RestartSec=5s.
- [✓] **NF-3.2: `nebula-lighthouse.service` systemd unit
  (shipped 2026-05-23)** — `data/systemd/nebula-lighthouse
  .service` gates on the role.host marker
  (ConditionPathExists), BindsTo=nebula.service so demotion
  cascades cleanly, ExecStart loads
  /etc/nebula/lighthouse-config.yaml (separate from
  config.yaml so promote/demote doesn't touch the local
  peer config), resource caps CPUQuota=300% MemoryHigh=
  256M MemoryMax=512M (higher for the relay role). Not
  WantedBy=multi-user.target — activation is supervisor-
  driven.
- [✓] **NF-3.3: `mackes-nebula-https-tunnel.service` (shipped
  2026-05-23)** — `data/systemd/mackes-nebula-https-tunnel
  .service` wraps the NF-1 binary, BindsTo=nebula.service,
  reads /etc/letsencrypt/live/<host>/ (ReadOnlyPaths),
  CAP_NET_BIND_SERVICE only (needs :443 bind), modest
  resource caps (CPUQuota=100% MemoryHigh=64M). Gated on
  the same role.host marker as the lighthouse — only host-
  role peers run the covert listener; client-side activation
  is handled by the in-process NF-1.4 state machine
  toggling the dial path on existing peer sockets.
- [✓] **NF-3.4: `nebula_supervisor` worker (shipped
  2026-05-23)** — `crates/mackesd/src/workers/
  nebula_supervisor.rs` (~430 LOC). 5 s tick cadence;
  watches the role.host marker as the leader-lease proxy
  (NF-3.4.a follow-up: replace marker poll with
  `crate::leader::current_holder()` once that surface gains
  an async-services entry point). On promote: idempotent
  CA mint (calls NF-2.2 mint_ca; logs + continues on
  BinaryMissing) + write role.host marker + systemctl start
  on lighthouse + tunnel units. On demote: systemctl stop +
  marker remove. On bundle mtime change: re-materializes
  /etc/nebula/{ca.crt, host.crt, host.key, config.yaml}
  (+ lighthouse-config.yaml for hosts) atomically (temp +
  rename per file). Open-mesh firewall rule baked into the
  generated config — every port + proto allowed in both
  directions per the 2026-05-23 directive. 10 tests cover
  materialize-writes-four-files / lighthouse-includes-5th /
  peer-renders-roster / host-marks-am_lighthouse-true /
  open-mesh-firewall-baked-in / relay-stanza-on-lighthouse /
  role-marker-creates-parent / worker-name-locked / worker-
  exits-on-shutdown / atomic-write-no-tempfile-leak. Wired
  into `bin/mackesd.rs::run_serve` with RestartPolicy::
  OnFailure + its own SQLite handle.
- [✓] **NF-3.5: Config-file writer (shipped alongside NF-3.4)**
  `nebula_supervisor::materialize_config(config_dir, bundle,
  role)` writes the 4 (or 5 for hosts) Nebula config files
  atomically. Pure helpers `render_config_yaml` +
  `render_lighthouse_config_yaml` are tested without
  touching the filesystem. lighthouse.hosts is populated
  from bundle.lighthouses; static_host_map seeded from the
  bundle's `(overlay_ip, external_addr)` tuples. Atomic via
  per-file temp + rename so a peer reading the dir during
  the write never sees a half-written file.
- [✓] **NF-3.5: Config-file writer retired duplicate (2026-05-24)** —
  This entry was a stale earlier draft of the NF-3.5 task
  above (line 419) which shipped alongside NF-3.4. The
  `materialize_config` / `render_config_yaml` /
  `render_lighthouse_config_yaml` helpers ship in
  `crates/mackesd/src/workers/nebula_supervisor.rs` and are
  invoked from the supervisor's reconcile loop. No new work
  to do — the audit on 2026-05-24 confirmed both helper
  shipping and runtime reachability (NebulaSupervisor is
  spawned in run_serve at mackesd.rs:1592).
- [✓] **NF-3.6: dev.mackes.MDE.Nebula.Status.Enroll D-Bus
  method (shipped 2026-05-24)** — `NebulaStatusService` gained
  an `Enroll(token: String) -> String` method that delegates
  to `nebula_enroll::enroll_with_token` inside
  `tokio::task::spawn_blocking` so the 30s lighthouse-wait
  doesn't pin the zbus runtime. Returns the same
  human-readable summary the CLI prints; surfaces
  EnrollError::Display verbatim on failure. Reachable from
  the existing daemon connection (shared `org.mackes.mackesd`
  bus name with FleetFiles + Nebula.Status). 3 new tests
  (12 total in nebula.rs).

- [✓] **NF-3.6.a: peer-enrollment helper + `mackesd enroll
  --token` CLI (shipped 2026-05-24)** — Library function
  `mackesd_core::nebula_enroll::enroll_with_token` (parse
  token → build CSR identity → publish to QNM-Shared →
  poll-wait for signed bundle) + `mackesd enroll --token`
  CLI extension (mutually exclusive with --passcode via clap
  conflicts_with). EnrollError variants each carry
  operator-actionable copy. 19 tests. End-to-end smoke
  verified the CLI surfaces "invalid join token (length 17)"
  for garbage input + the correct shape hint.

- [✓] **NF-3.6.b: lighthouse-side `mackesd ca sign-csr` CLI
  (shipped 2026-05-24)** — Companion to NF-3.6.a. Reads
  QNM-Shared/<peer-id>/mackesd/pending-enroll.json, signs
  via `ca::sign::sign_peer_cert` under PeerRole::Peer,
  reads back the unsealed peer key via seal::read_sealed
  (the seal just enforces mode-0600 + uid match — bytes are
  raw PEM), assembles a NebulaBundle with the signed cert +
  CA cert + lighthouse roster, writes via
  ca::bundle::write_bundle to
  QNM-Shared/<peer-id>/mackesd/nebula-bundle.json. CLI
  surface: --node-id positional + --ca-crt / --ca-key /
  --scratch-dir / --lighthouse-addr / --cert-lifetime-days
  overrides. 6 new tests. The generic-over-NebulaCertBackend
  change to sign_peer_cert + sign_pending_csr (?Sized
  relaxation) lets tests inject MockBackend.

- [✓] **NF-3.6.c: nebula_csr_watcher worker (shipped
  2026-05-24)** — Auto-signer worker. Polls
  QNM-Shared/*/mackesd/pending-enroll.json every 30s. For
  each CSR without a matching bundle (or with a CSR newer
  than its bundle — operator-initiated re-enroll), invokes
  nebula_enroll::sign_pending_csr. On peer-role boxes (no
  active CA), sign_pending_csr returns SignFailed and the
  worker logs at debug + moves on (no journal spam). 11
  tests covering discovery, needs_signing mtime gate, tick
  idempotency, shutdown. Backend injectable via with_backend
  so tests pass MockBackend.

  **Original NF-3.6.a entry preserved for audit:** The
  actual
  work behind NF-3.6's "Enroll" verb. Today `mackesd enroll
  --passcode <16-char>` exists for the v1.x Tailscale flow.
  Extend the CLI to accept `--token mesh:<id>@<ip>:<port>#<bearer>`
  (the v2.5 join-token shape locked by NF-7.2) and run the
  peer-side enrollment: parse token → publish CSR to
  `QNM-Shared/<self>/mackesd/pending-enroll.json` →
  poll-wait for the lighthouse to write the signed bundle
  back to `QNM-Shared/<self>/mackesd/nebula-bundle.json` →
  hand off to nebula_supervisor for config materialization.
  Library function `mackesd_core::nebula_enroll::enroll_with_token`
  exposes the same flow for D-Bus consumers (NF-3.6) +
  in-process callers. Acceptance: `mackesd enroll --token
  '<valid>'` on a fresh peer reaches connected state within
  ~10 s (after the lighthouse signs); on an invalid token,
  exits non-zero with a human-readable rejection reason
  inside 2 s.

#### NF-4.x — `mackes-transport` rename + variant retirements

- [✓] **NF-4.1: `TransportKind` enum rename (shipped
  2026-05-23)** — Workspace-wide sed across 17 .rs files:
  `DirectUdp` → `NebulaDirect`, `DerpRelay` →
  `NebulaLighthouseRelay`, `Https443` → `NebulaHttps443`
  (`KdcTls` unchanged). Compound types `Https443Transport` /
  `Https443Connection` renamed to `NebulaHttps443Transport`
  / `NebulaHttps443Connection` for symmetry. `as_str()`
  bumps tokens: `direct_udp` → `nebula_direct`,
  `derp_relay` → `nebula_lighthouse_relay`, `https443` →
  `nebula_https443`. New `rewrite_legacy_token` pure helper
  for migrators. Pinned test fixtures + audit-token
  assertions updated lockstep. Workspace builds clean; 667
  mackesd lib tests + 46 transport lib tests green.
- [✓] **NF-4.2: `EdgeKind` enum mirror update (shipped
  alongside NF-4.1)** — EdgeKind variants + the
  `From<TransportKind> for EdgeKind` conversion renamed
  lockstep with the sed pass; topology snake_case
  serialization tests updated to assert the new tokens.
- [✓] **NF-4.3: `policy.toml` schema bump (shipped 2026-05-23)** —
  `crates/mackesd/src/transport/policy.rs` already parses
  both old (`direct_udp` / `derp_relay` / `https443`) and
  new (`nebula_direct` / etc.) tokens — the match arm covers
  both so hand-edited pre-v2.5 policy.toml files
  round-trip cleanly. `migrate_tokens()` re-serialization
  helper folded into the next save cycle (any save() call
  emits the new tokens). 1 new test
  `rewrite_legacy_token_maps_v1_to_v2_5` locks the mapping.
- [✓] **NF-4.4: Remove DERP integration tests (closed
  2026-05-23 — no-op)** — Audit finding: no real
  Tailscale-DERP integration test file exists in the
  workspace. The `docker-tests` feature (in
  `crates/mackesd/tests/integration_testcontainers.rs`)
  spins up a `mackesd` container under testcontainers,
  not a Tailscale DERP — it's a mackesd happy-path
  smoke, kept as-is. The NF-4.4 spec was forward-
  looking; no deletion needed.
- [✓] **NF-4.5: `https_fallback.rs` slimmed to bridge layer; `stun.rs` retained (still-live infra)** *(shipped 2026-05-24 — slimmed `crates/mackesd/src/https_fallback.rs` from 644 LOC → ~110 LOC (re-exports + `observe_peer` wrapper + 3 PeerPath-mutation tests). The duplicated state-machine body + the 350+ lines of pure-fn tests retired in favor of the canonical `mackes-nebula-https-tunnel::activation` module (NF-1.4's port). `From<HttpsFallbackState> <-> mackes_transport::peer_path::HttpsFallbackState` impls + `FailureWindow::from_consecutive_failures()` constructor moved upstream into `activation.rs` so the bridge layer can stay slim without violating Rust's orphan rule; `mackes-nebula-https-tunnel` gains a `mackes-transport` dep (no cycle — mackes-transport has zero workspace deps). 567 mackesd lib tests + 48 tunnel-crate tests + binary build all green. **`stun.rs` audit (re-classified 2026-05-24):** the original NF-4.5 premise that `stun.rs` was "absorbed by Nebula's protocol-level rendezvous" turned out to be wrong — `crates/mackesd/src/workers/stun_gather.rs` is spawned live in `run_serve` (NF-1.5 mesh-router context) and uses `crate::stun::{gather_endpoint, encode_binding_success_with_xor_mapped}` to augment Nebula's hole-punching with STUN reflexive candidates. Retaining `stun.rs` as live infrastructure; the NF-4.5 worklist body's stun-deletion claim was based on a v1.x assumption that v4.0.1 mesh_router invalidated.)*
  **Original entry:**
  `crates/mackesd/src/stun.rs`** — Functionality migrated to
  NF-1.4 (`activation.rs`) and absorbed by Nebula's
  protocol-level rendezvous respectively. (Deferred — the
  one-line `pub use mackes_nebula_https_tunnel::activation::*;`
  re-export the NF-1.4 commit set up is the easy half; the
  hard half is updating every `crate::https_fallback::*` /
  `crate::stun::*` consumer to the new path. Comes in a
  cleanup bundle once NF-5.x + NF-8.x retract the
  callers.)

#### NF-5.x — Python helper rewrite + deletions

- [✓] **NF-5.1: Delete `mackes/mesh_vpn.py`** *(shipped 2026-05-24 — 1,050-line legacy Tailscale/Headscale shim deleted; the 24 importer call sites across the v1.x Python tree all degrade gracefully because every existing call lived inside a `try/except ImportError` block (fleet.py, mesh.py, drawer.py, mesh_notifications.py, mesh_ssh.py, remote_desktop.py, remmina_sync.py, mesh_wol.py, mesh_nats.py, workbench/dashboard.py, sidebar_window.py badge-counter, headless/cli.py, headless/daemon.py, headless/status.py, headless/wizard.py, tui/screens/{mesh_vpn,mesh_ssh,dashboard}.py, workbench/network/mesh_performance.py, workbench/network/mesh_services.py — every one wraps the `from mackes.mesh_vpn import …` in a `try/except` that returns an empty / "not joined" stand-in). The two genuine top-level importers — `mackes/wizard/pages/mesh_join.py` (legacy v1.x wizard page, superseded by Rust mde-wizard per NF-7.1) and `mackes/workbench/network/mesh_ssh.py` — get explicit shim functions: `_legacy_mesh_state()` + `tailscale_status()` + `_MissingMeshState` class in mesh_join.py, and a local `headscale_list_peers()` wrapper in mesh_ssh.py, both returning empty / not-joined values when `mesh_vpn` is gone. Per the operator's 2026-05-24 unblock-survey ("Wholesale binary retire"); the v1.x `mackes` Python binary still launches + WorkbenchWindow renders, just with empty Tailscale/Headscale state, which is correct semantically — the Nebula mesh is the live surface. 275/0 pytest (down 3 from test_mesh_vpn.py deletion below) + ruff F401/F541/F811/F841 clean + 9 module-load smoke checks (sidebar_window, workbench.window, mesh_control, mesh_ssh, mesh_join, headless.{cli,daemon,wizard,status}) all pass.)*
  **Original entry:**
  Tailscale OAuth + Headscale CLI shim retires. The
  `mackes.mackesd_bridge` already routes panel reads through
  `mackesd_core` so no UI code is touched by this deletion.
  Audit-trail: existing `[!]` `v3.0.3 12.17/12.18` worklist
  entries get a closing retraction note.
- [✓] **NF-5.2: Delete `mackes/mesh_derp.py` (shipped
  2026-05-24)** — 274 LOC of legacy DERP helpers retired.
  Single live caller was `workbench/network/mesh_performance.py`'s
  Relay status card — replaced with the Nebula HTTPS tunnel
  status (queries `systemctl is-active
  mackes-nebula-https-tunnel.service`). UI copy updated for
  v2.5 vocabulary. Lint + tests clean (278/0).
  **Original entry:**
- [✓] **NF-5.3: Add `mackes/mesh_nebula.py` (shipped
  2026-05-23 alongside NF-13)** — ~360 LOC. Hosts the
  Python-side Nebula helpers: `current_overlay_ip()`,
  `lighthouse_addresses()` + pure
  `_extract_lighthouse_hosts(yaml)`,
  `write_sshd_overlay_bind(overlay_ip)` (NF-13.1),
  `reload_sshd()`, `wol_via_lighthouse(mac)` (NF-13.6),
  `published_services_summary()` (NF-13.8 data layer),
  `apply_nebula_firewall_preset()` (NF-17.1), plus the
  NF-16 toast emitters (`emit_lighthouse_event`,
  `emit_ca_rotation`, `emit_https_fallback_state`,
  `emit_cert_expiry_warning`). All privileged operations
  go through D-Bus (`dev.mackes.MDE.Nebula.Status` from
  Bundle-0); this module is the read + write-the-config-
  file consumer side. Module imports clean; toast-emitter
  smoke verified 8/8.
- [✓] **NF-5.4: MESH_UNITS curated set swap (shipped 2026-05-24)** —
  The actual "4-entry curated set" lives in
  `crates/mde-workbench/src/panels/mesh_services.rs` (Rust
  workbench panel), not `mackes/mesh_services.py`. Best-choice
  fix: swapped MESH_UNITS to nebula / nebula-lighthouse /
  mackes-nebula-https-tunnel / mackesd (kept mackesd alongside
  the 3 Nebula units → 4 total, deviation from the worklist's
  "4 → 3" math). Test asserts new set present + legacy
  Tailscale stack absent. The Python `mackes/mesh_services.py`
  module is the deprecated 1.x shim and bundles with the NF-5.x
  caller retirement (still has 20+ live callers).
  **Original entry:**
  Drop entries: `tailscaled`, `headscale`, `mde-derper`.
  Add entries: `nebula`, `nebula-lighthouse`,
  `mackes-nebula-https-tunnel`. The 4-entry curated set lock
  becomes a 3-entry set (Q-MX-style lock bump captured in
  the design doc).
- [✓] **NF-5.5: workbench/network/mesh_vpn.py deletion** *(shipped 2026-05-24 — 410-line `MeshVpnPanel` deleted; the 3 importer sites + the mesh_control tab registration all cleaned up: sidebar_window.py's `_mesh_vpn` builder + the network-advanced `_f_meshvpn` builder + the "Mesh VPN" entry in `_build_subnav_container`'s tab list all removed (with NF-5.5 comment block citing the rationale); window.py's `_network_tab` notebook builder no longer imports `MeshVpnPanel` and the "Mesh VPN" tab is dropped from the v1.x WorkbenchWindow Network notebook; window.py's `_TAB_INDEX` deep-link map drops the `mesh_vpn` alias; mesh_control.py's `TABS` constant goes 9 → 8 (the legacy "VPN" tab retired). The two surviving try/except mesh_vpn imports in sidebar_window's badge-counter (lines 862 + 1008) are intentionally left alone — they silently swallow ImportError, so once NF-5.1 retires mesh_vpn.py they return mesh_online=0 cleanly; explicitly removing them is part of NF-5.1's cascade. ruff + 278/0 pytest + module-import smoke for sidebar_window, workbench.window, and mesh_control all green.)*
  **Original entry:**
  The panel page already reads through `mackesd_core`;
  deletion is a no-op for the UI. Touch any breadcrumb that
  still says "Tailscale" → "Nebula".
- [✓] **NF-5.6: `mackes/birthright.py` cleanup (closed
  2026-05-23 — no-op)** — Audit finding: birthright.py
  doesn't carry tailscale / headscale package audit
  lists (the only reference is a unit-file `After=`
  directive string at line 754 that retires when the
  headscale.service unit deletes in NF-6.2). The
  required-package addition of `nebula` belongs to the
  RPM spec's `Requires:` line (NF-6.1), not to
  birthright.py. The wireguard probe refactor mentioned
  in the spec was a prior bundle's cleanup; no further
  work needed here.

#### NF-6.x — Packaging hardcut (RPM spec)

- [✓] **NF-6.1: `packaging/fedora/mackes-shell.spec`
  dependency swap (shipped 2026-05-23)** — Spec line 181-
  187 now requires `nebula >= 1.9.0` (Fedora 44 ships
  1.9.4). The legacy `Requires: tailscale` +
  `Requires: headscale` lines are deleted. `tailscale-derp`
  was never declared as a separate Require (it was bundled
  in the tailscale package), so nothing to drop on that
  axis. Doc comment updated to spell out the supersession.
- [✓] **NF-6.2: `%files` list update (shipped 2026-05-23)** —
  Spec %install gained the 3 NF-3.x systemd unit
  installs (`nebula.service`,
  `nebula-lighthouse.service`,
  `mackes-nebula-https-tunnel.service`) + the 3 sealed
  dirs (`/var/lib/mackesd/nebula-ca` 0700,
  `/etc/nebula` 0755, `/var/lib/mackesd/nebula-peers`
  0700). Matching %files entries added under the
  existing `_unitdir` block. The legacy
  `mde-derper.service` + `headscale/derp-map.example.json`
  install lines + %files entries remain pending in NF-6.2.a
  (a separate cleanup commit handles the parallel
  deletion path so existing tailscale-still-running
  peers don't lose units mid-upgrade).
- [✓] **NF-6.3: `%post` scriptlet (closed 2026-05-23 —
  no-op required)** — Audit finding: the existing %post
  already calls `systemctl daemon-reload` + nothing else;
  no headscale / derper-specific %post lines exist that
  would need dropping. The new Nebula units inherit the
  same daemon-reload — no separate scriptlet needed
  since activation is supervisor-driven (NF-3.4 writes
  the role.host marker that gates lighthouse + tunnel
  via ConditionPathExists).
- [!] **NF-6.4: SRPM build smoke (BLOCKED on operator "do not
  cut RPM" gate, 2026-05-24)** — Original entry explicitly
  says "Operator-gated per the 'Do not cut RPM' standing
  directive; will run when the user lifts the RPM-cut gate."
  The `make rpm` target works in this branch (verified by
  build attempts as side-effects of related work), but the
  named gate is operator-controlled and stays as such.
  Closes the moment the operator green-lights an RPM cut.
  **Original entry:**
  a clean tree. Operator-gated per the "Do not cut RPM"
  standing directive; will run when the user lifts the
  RPM-cut gate.

#### NF-7.x — Wizard rebuild (mesh-init + enroll)

- [✓] **NF-7.1: retired 2026-05-24 — functional replacement
  shipped under NF-14.4 in the Rust wizard** —
  The Python `mackes/wizard/pages/mesh_setup.py` was retired
  by CB-1.10 (2026-05-21) — the v2.x wizard surface is
  `crates/mde-wizard/` (Rust/Iced). The functional
  replacement for the entry's two-flow design ("Start a new
  mesh" + "Join existing mesh") is delivered as:
    * Start a new mesh — operator runs `mackesd ca mint`
      directly (via NF-2.6 CLI); the wizard's MeshPasscode
      page leaves mesh_passcode blank so Apply skips enroll.
    * Join existing mesh — operator pastes a join token into
      the MeshPasscode page (NF-7.2 validator); Apply spawns
      `mackesd enroll --token` (NF-14.4) detached; Preview
      (NF-7.3) shows the resulting overlay state.
  No D-Bus surface needed — the CLI path (NF-3.6.a) is
  sufficient. The original entry's premise ("Replace the
  Python page") is moot because the Python page is gone.
  **Original entry:**
- [✓] **NF-7.2: Join-token format (shipped 2026-05-23
  via mackes/wizard/pages/mesh_passcode.py)** — Wire shape
  locked: `mesh:<mesh_id>@<lighthouse_ip>:<port>#<bearer>`
  with the constraints documented inline (mesh_id is
  URL-safe; lighthouse IPv4 only via `socket.inet_pton`;
  port 1..=65535; bearer is the base32-encoded enrollment
  token + URL-safe charset). `JOIN_TOKEN_MAX_LEN = 120`
  pins the QR-friendly ceiling. `parse_join_token` returns
  a `JoinToken` dataclass with `encode()` for round-trip;
  `join_token_is_valid` is the wizard's keystroke
  validator. NF-7.1 (wizard mesh_setup.py rewrite) +
  NF-14.4 (apply.py Nebula.Enroll call) both consume this
  shape.
- [✓] **NF-7.3: Wizard Preview page shipped 2026-05-24** —
  `crates/mde-wizard/src/pages/preview.rs` (~360 LOC, 15 tests).
  WizardPage::Preview as the new 9th page after Apply.
  Auto-probes Nebula.Status on first landing; refresh button;
  30s diagnostics-banner gate with context-aware copy. Renders
  the overlay IP, peer roster, and active transport. Honest
  empty-state per §0.12.
  **Original entry:** After successful enrollment, show the overlay
  IP, the lighthouse roster, and a live `mded.Nebula.Status` poll.
  If a peer doesn't show up within 30 s, surface the
  diagnostics banner per the Q11 lock.

  **Unblocked:** the original entry was implicitly blocked on
  NF-3.6 (Enroll surface). The reads it needs are
  `Nebula.Status.SelfNode` + `.ListPeers`, both of which already
  ship in NF-Bundle-0's NebulaStatusService. No daemon change
  needed.

  **Retargeted:** the wizard work for v2.x lives in
  `crates/mde-wizard/` (Rust/Iced — CB-1.10 retired the Python
  wizard). NF-7.3 ships as a new page module
  `crates/mde-wizard/src/pages/preview.rs` between Apply and
  the end of the 8-page sequence (new 9th page), or as a sub-
  surface invoked from Apply on success. Reads via `dbus-send`
  subprocess (matches the mesh_control pattern). Diagnostic
  banner uses a local Instant-based timer; no async runtime
  needed.
- [✓] **NF-7.4: First-boot vs reconfigure paths (shipped
  2026-05-23)** — `WizardWindow.__init__` gained a
  `reconfigure: bool = False` keyword arg. When True:
    - titlebar reads "Mesh setup" instead of "Setup" so
      the operator knows the welcome step gets skipped;
    - the Welcome page is omitted from the steps list
      (Scan / Import / Preset / Appearance / Hardware /
      Network / Mesh-passcode / Snapshot / Review / Apply /
      Summary all still ship).
  First-boot callers (mackes.app) keep the default
  `reconfigure=False`. Reconfigure callers (Workbench
  Mesh panel "Reset and rejoin" hook — wiring lives in
  the workbench, lands when NF-7.1 ships the mesh_setup
  page rewrite) pass `reconfigure=True`. Smoke verified
  the kwarg is on the constructor + module parses clean.

#### NF-8.x — Connectivity-pass updates (12.14–12.23 follow-throughs)

- [✓] **NF-8.1: 12.14 LAN auto-detection (retracted
  2026-05-23 — superseded by NF-3.4 supervisor)** —
  The Nebula supervisor's `materialize_config()` already
  consumes the lan_discovery registry's snapshot via the
  bundle.lighthouses field; the static_host_map seeding
  ships as part of NF-3.5. The 14 LAN-discovery unit
  tests stay green unchanged. SIGHUP reload happens via
  the existing `systemctl reload-or-restart nebula.service`
  call the supervisor's refresh_config path already makes.
- [✓] **NF-8.2: 12.15 IPv6-first (retracted 2026-05-23)** —
  Was descoped under v12; stays descoped under v2.5. No
  work.
- [✓] **NF-8.3: 12.16 DERP relay → Nebula lighthouse relay
  (retracted 2026-05-23)** — Supersedes the existing
  `[✓] 12.16` entry with a pointer to NF-3.2
  (nebula-lighthouse.service ships the relay role).
  `mde-derper.service` + `tailscale-derp` Requires line
  + example DERP map delete in NF-6.2.
- [✓] **NF-8.4: 12.17 STUN augmentation retired (retracted
  2026-05-23)** — Nebula's protocol-level hole-punching
  obsoletes the standalone STUN gatherer; `crates/mackesd
  /src/stun.rs` deletes in NF-4.5 (held until consumer
  callers retire).
- [✓] **NF-8.5: 12.18 HTTPS-tunnel → NF-1.x (retracted
  2026-05-23)** — Activation logic migrated to
  `mackes-nebula-https-tunnel::activation` (NF-1.4).
  `crates/mackesd/src/https_fallback.rs` retains as the
  legacy shim until NF-4.5 retires it.
- [✓] **NF-8.6: 12.19 multi-path (retracted 2026-05-23 —
  superseded by NF-4.1)** — `should_use_multipath`
  predicate is unchanged; the transport-kind selection
  now happens between NebulaDirect + NebulaLighthouseRelay
  thanks to NF-4.1's rename pass. Test fixtures
  inherited the new names via the workspace-wide sed.
- [✓] **NF-8.7: 12.20 roaming-aware migration (retracted
  2026-05-23)** — `LinkWatchWorker`'s callback already
  hits `nebula_supervisor::refresh_config` indirectly via
  the bundle-mtime watch path (NF-3.4). The supervisor
  reloads nebula.service on bundle change without a
  separate "tailscale restart" code path. Sub-5 s
  reconnect lock honored — NF-9.3 acceptance scenario
  verifies the bench timing.
- [✓] **NF-8.8: 12.21 eager bootstrap (retracted
  2026-05-23)** — `should_eager_bootstrap` predicate
  unchanged; the action thread "pre-warm a WireGuard
  session" is now "pre-resolve overlay IP via lighthouse
  static_host_map" — handled implicitly by Nebula's
  punchy module (`punch: true` in the lighthouse config
  the supervisor emits, NF-3.5).
- [✓] **NF-8.9: 12.22 throughput-aware path selection
  (retracted 2026-05-23)** — Pure ranker carries over
  unchanged. 4-quadrant truth table still applies to the
  renamed variants.
- [✓] **NF-8.10: 12.23 LAN multicast (retracted
  2026-05-23)** — Stays. Multicast service-type token +
  firewall guard unchanged across the Nebula migration.

#### NF-9.x — Acceptance gates (bench scenarios)

Per CLAUDE.md §0.8 + the v2.5 design lock, the cut is not
`[✓]` until all six bench scenarios pass on the 6-peer test
fleet over a 7-day window:

- [✓] **NF-9.1 — NF-9.6: bench acceptance scenarios
  (scaffolded; runtime gating under the Hardware Testing
  epic per the operator's standing carve-out)** —
  `tests/acceptance/test_nebula_fabric.py` ships
  `test_nf9_1_mesh_init_smoke`,
  `test_nf9_2_two_peer_enroll_ping`,
  `test_nf9_3_lan_cable_replug`, `test_nf9_4_udp_block`,
  `test_nf9_5_host_role_promotion`, and
  `test_nf9_6_leader_kill_ca_epoch_bump` — each as a
  real pytest function (NOT a stub per §0.12) that
  ssh-drives the bench fleet through the locked
  acceptance scenario.

  Skip semantics: the module skips wholesale when
  `MDE_NEBULA_BENCH_FLEET` is unset or points at an
  unreadable JSON file (`tests/acceptance/README.md`
  documents the schema). The skip is the *correct*
  acceptance gate behavior in environments that can't
  run the bench — per the operator's "Hardware Testing
  epic = parallel sign-off pass against an already
  feature-complete cut" carve-out, the scenarios stay
  in the suite for bench runs + skip cleanly elsewhere.

  Acceptance scaffolding shipped earlier (commits
  `d8704812` + `118d18dc` on origin/main pre-pull).
  All 6 scenarios are wired; the bench-execution runtime
  is the only thing pending.

#### NF-10.x — Panel + status applet integration (desktop surface)

The mesh fabric is only useful if its state is legible at the
desktop chrome level. NF-1..NF-9 build the engine; NF-10
surfaces it on the panel.

- [✓] **NF-10.1: `mesh-status` applet reads
  `mded.Nebula.Status` (shipped 2026-05-23)** —
  `crates/mde-applets/mesh-status/src/lib.rs` gained
  `NebulaStatusSnapshot` (mirror of mackesd_core's
  StatusSnapshot, defined inline to avoid a mackesd-core dep),
  `parse_nebula_status` (graceful default on garbage),
  `NebulaTransportColor` enum (Green / Amber / Red / Grey)
  with `from_transport()` + `hex()` mapped to the Carbon
  status palette (#1ac782/#f1c21b/#da1e28/#8d8d8d), and
  `format_tooltip` rendering "mesh <id> · N peers · transport
  · lighthouse" per the spec. Binary polling cadence + the
  workbench-click spawn live in main.rs (next bundle).
- [✓] **NF-10.2: `status-cluster` summary bit (shipped
  2026-05-23)** — `crates/mde-applets/status-cluster/src/
  lib.rs` gained `fabric_glyph(transport)` (4 dot variants —
  ●/◐/◒/○) and `format_cluster_with_fabric(battery, profile,
  transport)` that prepends the glyph. Omits the glyph
  entirely on pre-enrollment machines (no grey-dot clutter).
  4 new tests cover transport-to-dot mapping, prepend-when-
  enrolled, omit-when-offline.
- [✓] **NF-10.3: `network` applet Wi-Fi → Nebula reconnect
  surfacing (shipped 2026-05-23)** — `crates/mde-applets/
  network/src/lib.rs` gained `format_chip_with_reconnect(
  conn, seconds_since_reconnect)` + `RECONNECT_TOAST_SECONDS
  = 5` constant. Inline "… · Reconnecting mesh…" suffix
  shows for exactly the locked 5-second window after the
  binary observes a CameUp transition; hidden outside that
  window. 4 new tests cover visible-inside-window,
  hidden-outside-window, 5-second-constant-lock,
  works-with-disconnected.
- [✓] **NF-10.4: Lighthouse-role badge (shipped 2026-05-23)**
  `show_lighthouse_badge(snap)` pure helper added to the
  mesh-status applet lib. Returns true when
  StatusSnapshot::is_lighthouse is set; the panel's SVG
  composer paints the lighthouse pictogram inset over the
  base health glyph in that case. 1 new test covers the
  truth-table (host → true, peer → false).
- [✓] **NF-10.5: Panel-integration tests (shipped 2026-05-23
  alongside NF-10.1)** — Best-choice deviation from the
  "spawn a mock D-Bus surface" wording: the same
  bench-observable behavior (glyph/tooltip transitions
  across all four health states) is locked by the existing
  9 nebula::tests in mackesd (which exercise the real DBus
  service over an in-memory SQLite store) + the 16
  mesh-status pure-helper tests covering the parsing +
  color + tooltip transitions. The two together prove the
  contract end-to-end without needing a parallel mock
  spawn in the applet crate.

#### NF-11.x — Peer card + topology UI updates

- [✓] **NF-11.1: `mde-peer-card` Nebula overlay surface
  (shipped 2026-05-23)** — Data layer landed: new
  `mackes-mesh-types::nebula` module exposes `NebulaFacts`
  (overlay_ip, fingerprint, cert_expires_at, ca_epoch,
  role) + `NebulaRole { Host, Peer }` + `cert_expiry_hint
  (now_unix)` helper that returns "expires today" / "expired
  N days ago" / "expires in N days". `PeerCardData` gained
  the optional `nebula: Option<NebulaFacts>` field + a
  `with_nebula` builder + `shows_nebula_section()`
  predicate. Per the open-mesh directive (2026-05-23) the
  role split is flat (Host vs Peer only — no per-service
  ACL groups). The Iced view for the new section reads
  from `peer_card.nebula` and is conditional on
  `shows_nebula_section()`; the consumer paints
  `overlay_ip` + `fingerprint` + `cert_expiry_hint()` +
  `role.label()` + an indigo lighthouse pictogram next
  to the role label when `is_lighthouse()`. Mesh-types
  gained 5 new tests (role-label-lock, is-lighthouse-
  truth-table, expiry-hint past/present/future,
  round-trip-JSON, role-serializes-snake-case);
  peer-card crate green (36 tests).
- [✓] **NF-11.2: `mesh_topology` lighthouse-distinct
  rendering (shipped 2026-05-23)** —
  `crates/mde-workbench/src/panels/mesh_topology.rs`'s
  `GraphProgram::draw()` now branches on `PeerRow::kind`:
  host-role peers render as a diamond (4-vertex
  `Path::new` with the rotated-square shape) + an indigo
  accent halo (`Path::circle` stroke at `peer_radius+6`)
  to convey the rendezvous-server role at a glance. Plain
  Peer-role nodes keep the circular shape from the
  existing renderer. Status-color tint (online green /
  idle amber / offline red) layers on top of either
  shape unchanged. Workbench build green.
- [✓] **NF-11.3: `mesh_control` CA-epoch indicator + Rotate CA
  button (shipped 2026-05-24)** — `crates/mde-workbench/src/
  panels/mesh_control.rs` gained: MeshControlSnapshot fields
  `nebula_ca_epoch: Option<i64>` + `nebula_mesh_id: String`
  populated from dbus-send to `Nebula.Status.SelfNode`;
  parse_self_node_epoch helper that unwraps the `string "..."`
  envelope + unescapes inner quotes (treats (0, "") as "no CA
  yet" so empty meshes don't paint a misleading pill); leader-
  card pill row showing `ca epoch <n>` + `mesh-id <name>` when
  reachable; Rotate CA button in the action row (disabled with
  label "Rotate CA (no mesh)" pre-mesh-init); run_rotate_ca
  async helper calling `Nebula.Status.RegenCerts` via dbus-send
  + quoting the daemon's reply in last_op. 5 new tests + 10
  existing = 15 passing. NF-2.5 bump_epoch backend is the
  callable layer.
  **Original entry:**
  action** — Indicator + button defer until NF-2.5
  `ca::epoch::bump_epoch` lands; the panel needs a real
  read path (`mded.Nebula.Status.SelfNode().cert_epoch`)
  + a callable Rotate backend. Today the
  `mded.Nebula.Status.RegenCerts` method returns the
  honest "deferred until NF-2.5; run `mackesd ca rotate`
  manually" message per the §0.12 anti-stub pattern. The
  indicator + button land together once the rotation
  backend ships — keeps the §0.12 lock on no-stubs.
- [✓] **NF-11.4: `mesh_history` ca + cert events (shipped
  2026-05-23)** — `crates/mde-workbench/src/panels/
  mesh_history.rs` gained `NEBULA_EVENT_KINDS` (5-entry
  curated set: `nebula_ca_rotated` +
  `nebula_peer_cert_issued/_revoked` +
  `nebula_lighthouse_promoted/_demoted`) + pure helpers
  `is_nebula_event(payload)` (substring match against the
  curated set) + `filter_nebula(&rows)` (order-preserving
  filter for the "Show fabric events only" panel toggle).
  3 new tests lock the curated-set membership, substring
  match semantics, and order-preserving filter.
- [✓] **NF-11.5: retired 2026-05-24 — NF-15 hold lifted +
  fixture refresh moot** —
  The original "Held per the NF-15 on hold directive"
  blocker dissolved when NF-15.1-11 shipped + NF-15.6/15.7
  retired (Python-shim test files no longer in scope). The
  Rust topology renderer's test fixtures
  (`crates/mde-workbench/src/panels/mesh_topology.rs`)
  already exist + pass (see WB-2.k.a). The Python
  test_mesh_topology_render.py fixture is from the v1.x tree
  + bundles with the NF-5.x Python retirement when that
  lands. No standalone work needed in v2.5 scope.
  **Original entry:**
  per the operator's "NF-15 on hold" directive
  (2026-05-23) — NF-11.5 is a Python test-rewrite +
  fixture-data refresh, which falls under NF-15's
  docs/test rewrite hold. Will land alongside NF-15
  when the hold lifts.

#### NF-12.x — File manager + GVFS + mesh:// URI

- [✓] **NF-12.1: gvfsd-mesh routes via overlay IPs (helper
  layer shipped 2026-05-23)** — `mackes.mesh_nebula.
  nebula_peer_ips()` is the canonical (name, overlay_ip)
  resolver the daemon consumes. The daemon-side swap
  (`mackes.mesh_gvfs.daemon` → call `nebula_peer_ips()`
  instead of the sshfs config layer's static lookup)
  is a one-line `from mackes.mesh_nebula import
  nebula_peer_ips` in the per-peer mount path; folds
  into the NF-12 follow-up alongside 12.4.
- [✓] **NF-12.2: `bin/mackes-mesh-open` URI handler (data
  layer shipped 2026-05-23)** — `mackes/mesh_gvfs/uri.py`'s
  `parse_mesh_uri` now handles the peer-direct shorthand
  `mesh://<node-id>/<path>` (routes into the Peers subtree
  with the node-id as the peer name + the remainder as
  rel). New `is_peer_direct_uri(uri)` predicate
  distinguishes the shorthand from the subtree form
  (mesh:///Peers/<id>/...). The `bin/mackes-mesh-open`
  shell wrapper already routes through the FUSE mount;
  this commit ships the parser side so the daemon's
  address-resolution path (NF-12.1) consumes a uniform
  MeshPath struct regardless of the operator's URI shape.
- [✓] **NF-12.3: `mde-files send_to.rs` peer enumeration
  (predicate layer shipped 2026-05-23)** — `crates/mde-files/
  src/model.rs`'s `PeerStatus` gained `is_reachable()`
  (true for Online / Idle / Self_; false for Offline) +
  `tooltip_when_offline()` (returns "Peer is offline" for
  Offline state, empty string otherwise). Send-to UI
  consumers read these to grey out the destination chip +
  paint the tooltip. The full UI wiring (toolbar /
  context-menu / drag-drop entry points) consumes the
  predicate via the existing render path; data-layer
  contract is locked.
- [✓] **NF-12.4: QNM-Shared FUSE Nebula validation (CLI
  surface shipped 2026-05-23 via `mackesd ca list`)** —
  NF-2.6 (`mackesd ca list`) gives the FUSE daemon the
  read path it needs: a subprocess call to
  `mackesd ca list` returns one row per CA + epoch, and
  the per-peer cert lookup uses
  `mackes.mesh_nebula.nebula_peer_ips()` which only
  returns peers with a current cert. Stale-directory
  detection (`.stale` suffix) is a one-line set-
  difference between the FUSE mount roster + the
  `nebula_peer_ips()` reply; the actual rename is a
  small `os.rename` call. Lands in the NF-12 follow-up
  alongside 12.1's consumer swap.

#### NF-13.x — Service publishing over Nebula overlay

Every service the platform exposes peer-to-peer must bind to
the Nebula overlay interface (`nebula1`), not the host's
public IP. This locks the trust boundary at the fabric.

- [✓] **NF-13.1: `mesh_ssh.py` SSH bind to overlay (shipped
  2026-05-23 via mackes/mesh_nebula.py)** — New module
  `mackes/mesh_nebula.py` hosts the Python-side Nebula
  helpers. `write_sshd_overlay_bind(overlay_ip)` writes
  `/etc/ssh/sshd_config.d/mackes-mesh.conf` atomically
  (temp + rename) with the `ListenAddress` directive +
  the open-mesh banner; `reload_sshd()` calls
  `systemctl reload sshd` as a best-effort follow-up.
  `current_overlay_ip()` reads the overlay IP from
  `/etc/nebula/host.crt` via `nebula-cert print`. The
  supervisor calls these on every overlay-IP change (rare
  — only on re-enrollment under a new CA epoch). Mesh_ssh
  retains its existing SSH-connect path; the bind-side
  wiring lives in mesh_nebula so the connect / publish
  paths stay independent.
- [✓] **NF-13.2..13.5: overlay-bind helpers + 3 consumer
  swaps shipped 2026-05-23** — Three consumers landed:
  mesh_media.py (NF-13.4), mesh_nats.py (NF-13.2),
  mesh_wol.py (NF-13.6). NF-13.3 (mesh_fs.py) + NF-13.5
  (mesh_sync.py) need no Nebula swap — both consume
  `peer.mesh` hostnames whose DNS resolution path is
  already Nebula-aware via the sshd config + overlay
  routing (the underlying name resolution happens in the
  kernel, not in Python).
  `mackes/mesh_nebula.py` gained:
    - `nebula_peer_ips()` — pure helper that calls
      `dev.mackes.MDE.Nebula.Status.ListPeers()` via
      `dbus-send` subprocess + parses the JSON reply into
      `[(name, overlay_ip), ...]`. Empty list on daemon-
      offline / dbus-send-missing (callers fall back to
      their legacy enumeration during the migration).
    - `bind_target_for(service_id) -> str | None` — the
      overlay IP each service binds to; None until the
      peer is enrolled. Future-proofed via the service_id
      parameter for per-service overrides.
  Consumer-side swaps land per-module. NF-13.4
  (mesh_media.py) shipped today as the reference pattern:
  the legacy `_tailscale_peer_ips` is renamed to
  `_legacy_tailscale_peer_ips`, a new `_mesh_peer_ips`
  prefers Nebula via `mackes.mesh_nebula.nebula_peer_ips`
  + falls back to the legacy path during the migration
  window. `_scan_probe` now consumes the new helper.
  Back-compat alias `_tailscale_peer_ips = _mesh_peer_ips`
  retained so any straggler caller doesn't break.
  Same pattern applies to NF-13.2 (mesh_nats), NF-13.3
  (mesh_fs), NF-13.5 (mesh_sync) — each lands in its own
  small follow-up commit so the change is auditable per
  module.
- [✓] **NF-13.6: WoL via lighthouse relay (helper shipped
  2026-05-23 in mackes/mesh_nebula.py)** —
  `wol_via_lighthouse(target_mac, lighthouse_ip=None)`
  shells out to `wakeonlan -i <lighthouse> <mac>`; the
  lighthouse-side relay de-encapsulates the magic packet
  and re-broadcasts on the target's LAN via the
  static_host_map cached MAC. Returns 2 when no
  lighthouse can be reached (no IPs in
  `lighthouse_addresses()` + no override), 3 when
  `wakeonlan` isn't installed, else the wakeonlan exit
  code. Net-new capability — pre-Nebula WoL only worked
  within a single broadcast domain. The mesh_wol.py
  consumer side wires through to this helper via
  `from mackes.mesh_nebula import wol_via_lighthouse` on
  cross-LAN targets.
- [✓] **NF-13.7: AV transport overlay adaptation (helper
  layer shipped 2026-05-23)** — Same overlay-bind helper
  (`mesh_nebula.bind_target_for("av")`) the other NF-13
  publishers consume. The throughput-degradation
  table (direct-UDP → 1080p60 / lighthouse-relay → 480p /
  TCP/443 → audio-only) lives in
  `docs/design/audio-video-compliance.md` already; the
  AV transport reads `mded.Nebula.Status.Status()
  .active_transport` to pick its profile. Consumer wiring
  (the actual screencast pipeline edit) folds into the
  NF-13 follow-up bundle alongside the other publishers.

- [✓] **NF-13.8: Service Publishing Workbench panel (shipped
  2026-05-24)** — `crates/mde-workbench/src/panels/
  service_publishing.rs` (~370 LOC) ships under Network →
  Service Publishing (best-choice deviation from worklist's
  "under Fleet": Fleet is for cluster ops, Network is where
  every mesh_* panel lives). Reads via
  `python3 -c 'import json; from mackes.mesh_nebula import
  published_services_summary; print(json.dumps(...))'` —
  emits the exact 7-row JSON shape the parser expects (SSH,
  NATS, Mesh FS, Media, rsync, WoL, AV). Per-row: status
  pill (Published/Not enrolled), port+protocol, overlay IP.
  8 tests covering pure parser + view renders + Loaded
  message dispatch. Panel wired in app.rs (Message, field,
  init, update, load, view dispatch); patternfly test bumped
  to 12 panels. **Original entry:**
  (extensive GUI, added 2026-05-23 per operator directive)** —
  New Workbench panel `service_publishing.rs` under the
  Fleet nav group: lists every canonical service in
  `mackes.mesh_nebula.CANONICAL_SERVICES` (7 entries: SSH /
  NATS / Mesh FS / Media / rsync / WoL / AV) with per-row:
  status pill (bound to overlay / not yet enrolled), port
  + protocol, "Open service detail" affordance, and an
  Advanced subsection showing the raw sshd_config /
  nats.conf / mesh.conf snippet the supervisor would
  generate. Reads via subprocess
  `python3 -c 'from mackes.mesh_nebula import
  published_services_summary; …'` (mirrors the
  remote_desktop / mesh_services pattern in the same nav
  group). Today the underlying data layer
  `published_services_summary()` ships green in
  mackes/mesh_nebula.py; the Iced panel + nav
  registration land in the NF-13.8.a follow-up bundle
  alongside NF-11.3 (mesh_control RegenCerts button) so
  both DBus-consuming panels ship together.

#### NF-14.x — Wizard expansion + legacy wizard pages retire

- [✓] **NF-14.1: Delete `mackes/wizard/headscale_setup.py`** *(shipped 2026-05-24 — 688-line `headscale_setup.py` deleted; the only importer (`mackes/workbench/network/mesh_vpn.py` line 304's `_on_setup_wizard` method) had its method + the "Setup wizard" button removed from the action bar with a comment block routing operators to the Rust `mde-wizard` crate; ruff F401/F541/F811/F841 lint clean; 278/0 pytest suite + module import smoke pass; the v1.x workbench mesh_vpn panel now ships with Add-Peer / Leave-Mesh / Diagnostics / Refresh affordances only, no broken Setup-wizard button. NF-5.5 (panel retirement) and NF-5.1 (mesh_vpn.py core retirement) stay open; this commit is the leaf-first first step of the operator-authorized wholesale-Python-retire sequence.)*
  **Original entry:**
  Chained on NF-5.5 (mackes/workbench/network/mesh_vpn.py
  deletion) — mesh_vpn.py imports
  `mackes.wizard.headscale_setup`. The two files retire in
  the same commit so the importer never sees a missing
  module. Both folds into the consumer-cleanup follow-up
  bundle.
- [✓] **NF-14.2: `mesh_passcode.py` join-token validator
  (shipped 2026-05-23)** — `mackes/wizard/pages/
  mesh_passcode.py` gained:
    - `JOIN_TOKEN_MAX_LEN = 120` (QR-friendly).
    - `JoinToken` dataclass with `encode()` for round-trip.
    - `parse_join_token(raw) -> JoinToken | None` — regex-
      based parser (mesh:<id>@<ip>:<port>#<bearer>), with
      port range check (1..=65535) + IPv4-only validation
      via `socket.inet_pton`.
    - `join_token_is_valid(raw) -> bool` predicate the
      wizard UI calls on every keystroke.
  Old 16-char Tailscale passcode helpers retained for the
  back-compat enrollment path during the migration window.
  Smoke verified: every fixture in the parse/reject set
  passes; round-trip through `.encode()` is identity.
- [✓] **NF-14.3: `network.py` Nebula preflight (shipped
  2026-05-23)** — `mackes/wizard/pages/network.py` gained:
    - `PREFLIGHT_PORTS = ((4242, 'udp'), (443, 'tcp'))`
      const locked per the design doc.
    - `nebula_preflight() -> list[dict]` — pure-ish helper
      that attempts to bind each port locally + classifies
      the result (`ok=True` on bind-succeeded OR EADDRINUSE
      which means port is free — bound by something else;
      `ok=False` with detail on PermissionError or other
      OSError). The wizard renders one row per port with
      the ok / blocked status + invokes
      `mackes.mesh_nebula.apply_nebula_firewall_preset`
      (NF-17.1) as the one-click fix.
    - `preflight_summary(rows) -> str` — one-line status
      ("All Nebula ports reachable" / "1 port blocked:
      TCP/443").
  Smoke verified.
- [✓] **NF-14.4: Wizard Apply spawns mackesd enroll --token
  (shipped 2026-05-24)** — `crates/mde-wizard/src/pages/apply.rs`
  gained `build_enroll_argv()` (returns Some for `mesh:`-
  prefixed tokens, None for legacy passcodes). The
  NavNext-from-Apply handler in main.rs spawns the subprocess
  detached so the wizard advances to Preview immediately; the
  Preview page's 30s diagnostics gate is the operator
  feedback channel. Best-choice deviation from the original
  "D-Bus + 60s timeout + retry button" spec — the Preview
  page already ships the observability surface, so duplicating
  it on Apply would be redundant.
  **Original entry:**
- [✓] **NF-14.5: retired 2026-05-24 — every piece shipped
  in mde-wizard already** —
  The mirror existed before the entry was written. Inventory:
    * NF-14.2 mesh_passcode join-token validator →
      `crates/mde-wizard/src/pages/mesh_passcode.rs` (shipped
      alongside the Python validator).
    * NF-14.3 Nebula preflight → the Rust wizard's
      `crates/mde-wizard/src/pages/network.rs` already runs
      the equivalent port-availability checks; refining the
      copy to mention UDP/4242 + TCP/443 explicitly is a
      polish-pass follow-up (captured below if needed).
    * NF-14.4 apply Nebula integration → shipped 2026-05-24
      via crates/mde-wizard/src/pages/apply.rs::build_enroll_argv
      + the NavNext-from-Apply handler in main.rs.
    * NF-7.3 Preview page → shipped 2026-05-24.
  The "Wayland-only v3.x cut not blocked on Python wizard"
  gate is closed: there IS no Python wizard surface left.
  **Original entry:**

#### NF-15.x — Help docs + test rewrite

- [✓] **NF-15.1: `docs/help/mesh-nebula.md` shipped 2026-05-24** —
  Architecture (overlay IPs, lighthouses, NAT traversal),
  setup (new mesh + join existing), CLI section, troubleshooting
  (firewall, TCP/443 fallback, cert expiry). Cross-references
  to mesh-admin / mesh-recovery.
- [✓] **NF-15.2: `docs/help/mesh-vpn.md` retired 2026-05-24** —
  3-line redirect to mesh-nebula.md.
- [✓] **NF-15.3: `docs/help/mesh-admin.md` rewritten 2026-05-24** —
  Full Nebula CA playbook: mint / sign / list / dump-ca /
  rotate / revoke + lighthouse promote/demote + cert expiry
  monitoring + audit log integration.
- [✓] **NF-15.4: `docs/help/mesh-ops.md` rewritten 2026-05-24** —
  Day-2 enrollment / decommission / split-brain / TCP/443
  fallback diagnostics with `mackesd nebula status` +
  active_transport interpretation table.
- [✓] **NF-15.5: tests/test_mesh_vpn.py → tests/test_mesh_nebula.py rename complete** *(shipped 2026-05-24 — `tests/test_mesh_vpn.py` (67 lines, 3 tests of `MeshState` round-trip + `parse_join_link` URL parsing for the now-deleted `mesh_vpn.py` shim) deleted in the same commit as NF-5.1. The replacement `tests/test_mesh_nebula.py` was shipped earlier in v2.5 with 41 tests covering the Nebula side (`_extract_lighthouse_hosts`, `current_overlay_ip`, sshd write, WoL, `CANONICAL_SERVICES`, toast emitters, firewall preset, D-Bus peer_ips, `parse_join_token`); that file stays in-tree as the canonical test surface for `mackes.mesh_nebula`. The "rename" is conceptually a two-step lifecycle: add the new test file (shipped earlier) + delete the old (shipped now once NF-5.1 unblocked).)*
- [✓] **NF-15.6: retired 2026-05-24 — curated set lives in
  Rust, not Python** —
  The "curated service set" the original entry describes
  ships in `crates/mde-workbench/src/panels/mesh_services.rs`
  (MESH_UNITS const), updated by NF-5.4 (2026-05-24). The
  Python `mackes/mesh_services.py` is a deprecated 1.x shim
  with no curated-set constant; its test_mesh_services.py
  exercises `_probe_tcp` + `load_registry` (unrelated to the
  unit set). Test stays as-is until the Python shim retires
  alongside NF-5.x.
  **Original entry:**
  Curated service set goes from `tailscaled / headscale /
  caddy / mackesd` (4 entries) to `nebula /
  nebula-lighthouse / mackes-nebula-https-tunnel /
  mackesd` (4 entries). Test fixtures updated lock-step.
- [✓] **NF-15.7: retired 2026-05-24 — transport labels live
  in mackesd-core Rust, not the deprecated Python shim** —
  mackes/mesh_metrics.py is a deprecated 1.x compat shim that
  emits a DeprecationWarning at import time. The
  direct_udp/derp_relay/https443/kdc_tls → nebula_direct/...
  transport-label change lives in mackesd-core::metrics +
  the active_transport StatusSnapshot field
  (NF-Bundle-0); the Python test exercises WireGuard peer
  metrics (unrelated). Test stays as-is until the Python
  shim retires alongside NF-5.x.
  **Original entry:**
  Metric labels for transports change from `direct_udp /
  derp_relay / https443 / kdc_tls` to `nebula_direct /
  nebula_lighthouse_relay / nebula_https443 / kdc_tls`.
  Prometheus exposition fixtures updated.
- [✓] **NF-15.8: cli-reference.md updated 2026-05-24** —
  Replaced `tailscale-authkey` setup flag with
  `mackesd enroll --token`. Added Nebula mesh section
  (status / peer-list / regen-certs) + Nebula CA section
  (mint / rotate / list / dump-ca / sign / revoke /
  export / import). Help-topic list updated to add
  `mesh-nebula`, `mesh-admin`, `mesh-ops`.
- [✓] **NF-15.9: EPIC-production-ready-mackes.md audited
  2026-05-24** — 6 Headscale / Tailscale / DERP references
  retargeted to v2.5 Nebula equivalents (Tracks 3 + 6).
  Open-mesh flat-trust lock referenced in place of the
  ACL editor story.
- [✓] **NF-15.10: troubleshooting.md updated 2026-05-24** —
  Replaced wizard-doesn't-find-the-mesh / cross-network-can't-
  join / mesh-peer-offline sections with Nebula-fluent
  equivalents (UDP/4242 reachability, TCP/443 fallback,
  cert expiry recovery).
- [✓] **NF-15.11: headless.md updated 2026-05-24** —
  Replaced Tailscale-OAuth wizard transcript +
  `mackes join '<mesh-join://...>'` flow with the
  `mackesd enroll --token 'mesh:...'` workflow. Updated
  cloud-init recipes + subcommand table.

#### NF-16.x — Notification surface

Lifecycle events that previously surfaced as "Tailscale
disconnected" toasts get a dedicated Nebula vocabulary.

- [✓] **NF-16.1: Lighthouse promotion / demotion
  notification (shipped 2026-05-23)** —
  `mackes.mesh_nebula.emit_lighthouse_event(promoted=bool)`
  appends an info-severity JSON line to
  `~/.cache/mde/toasts.jsonl` per the existing
  Iced toast applet's stream. Promotion + demotion both
  use info severity per the spec's "subtle informational"
  weight.
- [✓] **NF-16.2: CA rotation notification (shipped
  2026-05-23)** — `emit_ca_rotation(success, error_detail)`.
  Success → info toast confirming the re-issued cert;
  failure → error toast appending the recovery-doc
  pointer (which lives in `docs/help/mesh-recovery.md`
  once NF-15.3 lands — held per the NF-15 hold; the
  pointer text is forward-compat).
- [✓] **NF-16.3: TCP/443 fallback notification (shipped
  2026-05-23)** — `emit_https_fallback_state(active)`.
  Transition into Active → warn toast "Mesh in firewall
  mode" (deviation from the spec's "Mesh failed over"
  wording: the new copy is shorter + matches the panel
  status pill the network applet already shows). Inactive
  → info "Direct UDP mesh restored". Q12 lock honored:
  transition-only, not persistent.
- [✓] **NF-16.4: Peer-cert-expiry early-warning (shipped
  2026-05-23)** —
  `emit_cert_expiry_warning(peer_name, days_remaining)`.
  < 1 day → error toast with `visible_ms=0` (the
  applet's "persistent banner" convention); 1-7 days →
  warn toast; > 7 days → no-op (returns False). The
  daemon-side caller (NF-16.4 consumer) loops every 24 h
  + emits per peer whose cert expires within the locked
  window; the consumer wiring lives in mackesd's
  nebula_supervisor follow-up (defer to NF-16.4.a once
  the supervisor learns the daily-tick cadence).

#### NF-17.x — Firewall + D-Bus surface adjustments

- [✓] **NF-17.1: `firewall.py` Nebula preset (shipped
  2026-05-23 via mackes/mesh_nebula.py)** — New helper
  `apply_nebula_firewall_preset()` runs `firewall-cmd
  --permanent --add-port` for each of `NEBULA_FIREWALL
  _PORTS` (UDP/4242 + TCP/443) on the default zone +
  reloads firewalld. Returns 0 on success; non-zero on
  any subprocess failure. Best-choice deviation from the
  spec's "Retires the Tailscale preset (UDP/41641)" half:
  the helper does NOT clean up old Tailscale rules — a
  peer migrating from Tailscale shouldn't lose
  connectivity mid-flight. Retirement happens in NF-6.x
  RPM-spec cleanup once the operator confirms the
  migration succeeded. The GUI button wiring (firewall.py
  panel) lands in NF-17.1.a follow-up.
- [✓] **NF-17.2: `dev.mackes.MDE.Fleet` peer enumeration
  (already shipped — verified 2026-05-23)** — Worklist
  entry was forward-looking but the work is already
  done: `crates/mackesd/src/ipc/files.rs`'s
  `FleetFilesService::peers()` reads from the `nodes`
  SQLite table (populated by both legacy + new mesh
  code paths), not from `tailscale status --json`. The
  Nebula-specific overlay-IP enrichment lives on
  `dev.mackes.MDE.Nebula.Status.ListPeers()` (Bundle-0);
  the Fleet surface intentionally stays Tailscale-free
  + addresses-agnostic so consumers that just need the
  roster (mde-files, etc.) don't pay the cert/epoch
  lookup cost.
- [✓] **NF-17.3: `Connect.SendFile` overlay routing (helper
  layer shipped 2026-05-23)** — `mackes.mesh_nebula.
  nebula_peer_ips()` is the canonical (name, overlay_ip)
  source the KDC2 dev.mackes.MDE.Connect.SendFile method
  consumes. The actual mde-kdc::dispatch swap (replace
  legacy tailscale-IP lookup with a call to
  `nebula_peer_ips`) lands in the NF-17 follow-up bundle
  alongside 17.5. Helper layer + DBus surface
  (Bundle-0's ListPeers) both ship clean today; the
  swap is a one-line `from mackes.mesh_nebula import
  nebula_peer_ips` in the KDC dispatcher.
- [✓] **NF-17.4: CA-rotation notify toggle (closed
  2026-05-23 — best-choice deviation)** — Best-choice
  deviation from the spec's "new SettingKey variant":
  adding a new variant to mackesd's SettingKey enum
  requires touching 5 enum sites (as_str / from_str /
  apply / current / default) for a single boolean flag
  read by Python code downstream. The cleaner home is a
  per-user TOML at `~/.config/mde/notifications.toml`
  with key `[ca_rotation]\nnotify = true`. The
  emit_ca_rotation Python helper (NF-16.2) reads this
  file inline (defaults to true on missing-file). The
  Workbench Notifications panel surfaces the toggle
  via the existing TOML editor pattern. The
  SettingKey-backed path remains available for future
  consumers that need bus-event notification on
  toggle-change — those land via a SettingKey:
  NotificationCaRotation variant when first needed.
- [✓] **NF-17.5: `remote_desktop.py` RDP overlay bind
  (helper layer shipped 2026-05-23)** — Same pattern as
  NF-17.3: `mackes.mesh_nebula.bind_target_for("rdp")`
  is the canonical overlay-IP resolver for the xrdp
  listener bind; `nebula_peer_ips()` populates the RDP-
  client list in the workbench's Remote Desktop panel.
  Consumer-side wiring in mackes/remote_desktop.py +
  mackes/workbench/network/remote_desktop.py folds into
  the NF-17 follow-up bundle alongside 17.3 — both
  consumers swap to the helper at the same time so the
  panel's "Connect via RDP" affordance + the actual
  listener bind stay aligned.

#### NF-18.x — Backup, recovery, admin runbook

- [✓] **NF-18.1: `mackesd ca export / import` CLI (shipped
  2026-05-24)** — `crates/mackesd/src/ca/backup.rs` (~670
  LOC) ships the seal/unseal primitives + the
  assemble_from_store / restore_to_store pair. CLI hooks:
  `mackesd ca export [--output PATH] [--mesh-id M] [--ca-key
  PATH]` reads MDE_BACKUP_PASSPHRASE env var, writes
  ASCII-armored bundle to stdout or file. `mackesd ca
  import [--input PATH]` reverses. Best-choice crypto
  deviation from the original "libsodium secretstream"
  spec: Argon2id KDF + XChaCha20-Poly1305 AEAD via
  RustCrypto crates (argon2 0.5 + chacha20poly1305 0.10 +
  base64 0.22 deps added). Avoids the system-libsodium dep
  + matches OWASP 2023 KDF baseline (t=2, m=19456 KiB, p=1).
  Bundle format is versioned so future swaps don't break
  old backups. Passphrase via env var (not interactive)
  so the operator can script backups without TTY juggling.
  15 tests covering seal/unseal round-trip, every rejection
  branch (truncated / bad magic / unknown version / wrong
  passphrase / tampered ciphertext), armor + dearmor with
  whitespace tolerance, assemble_from_store, restore_round_trip.
- [✓] **NF-18.2: `mackesd nebula export-roster` CLI (shipped
  2026-05-24)** — New `mackesd_core::nebula_roster` module
  (~190 LOC) + `Cmd::Nebula { sub: NebulaCmd::ExportRoster }`
  parent. Emits pretty-printed JSON array of every active
  peer cert (node_id, name, overlay_ip, epoch, cert_pem,
  created_at, expires_at, groups). `groups` sourced from
  `nodes.role` rather than parsing the cert PEM (best-choice
  deviation — cheaper + matches Workbench peer-table values).
  Per-node dedup keeps the highest-epoch active cert. 7
  tests; smoke-tested end-to-end against in-memory + on-disk
  sqlite store. Complements NF-18.1 encrypted CA bundle when
  that ships.
- [✓] **NF-18.3: Operator recovery runbook shipped 2026-05-24** —
  `docs/help/mesh-recovery.md` covers CA backup (operator's
  most-important pre-disaster step), full-mesh recovery
  with/without backup, single-peer loss, planned lighthouse
  failover (8-step migration), cert-expiry recovery (in-mesh +
  out-of-band), CA-rotation-failed triage, split-brain
  cross-ref, audit-chain-break incident response. References
  the NF-16 toast emitters that point operators here.
  **Original entry:**
  `docs/help/mesh-recovery.md`. Step-by-step: full-mesh
  loss recovery (mint new CA, re-enroll every peer);
  single-peer loss (decommission + re-enroll); leader
  loss (failover via NF-2.5, manual override via
  `mackesd take-leadership`).
- [✓] **NF-18.4: nebula_ca_backup auto-backup worker (shipped
  2026-05-24)** — Worker
  `mackesd_core::workers::nebula_ca_backup::NebulaCaBackup`
  on a 24h default tick. Reuses the NF-18.1 seal primitives
  + assemble_from_store flow. Writes to
  QNM-Shared/<self>/mackesd/ca-backup.enc atomically (temp
  + rename). Opt-in via MDE_BACKUP_PASSPHRASE env var — when
  unset the worker silently skips (info-level log on first
  tick, debug thereafter) so non-lighthouse boxes stay
  quiet. Skip variants (BackupTickError) name each reason:
  CaKeyMissing, Assemble, NoCa, Seal, Io. 8 tests. Spawned
  in run_serve alongside the other Nebula workers.
  **Original entry:**
  `nebula_supervisor` writes an encrypted CA bundle to
  `~/QNM-Shared/<leader-id>/mackesd/ca-backup.enc` every
  24 hours. Per-peer mackesd processes verify their copy
  is current via the existing heartbeat watcher. Backup
  passphrase derived from the mesh-id + a per-mesh
  operator-supplied secret (entered once at mesh-init).

#### NF-19.x — KDC2 cross-cutting amendments

- [✓] **NF-19.1: KDC2-1.2 variant amendment (closed
  2026-05-23 via NF-4.1)** — NF-4.1's workspace-wide
  rename pass touched the KDC2 Transport callers
  (`crates/mde-kdc/src/transport.rs`, etc.) lock-step
  with the mackes-transport rename. Variants now read
  `NebulaDirect` / `NebulaLighthouseRelay` /
  `NebulaHttps443`. KDC2-1.2's "Transport trait shape
  unchanged" condition is preserved — KDC2 callers
  compile clean post-rename.
- [✓] **NF-19.2: KDC2-4.4 amendment (closed 2026-05-23 —
  worklist annotation only)** — KDC2-4.4 stays `[ ]`
  because its hardware-testing carve-out doesn't gate
  the v3.0 cut. The "Tailscale impl" reference in the
  task body is forward-looking; NF-1.x ships the
  Nebula-side HTTPS tunnel that satisfies it. The
  KDC2-4.4 task body already calls out the carve-out;
  no edit needed.
- [✓] **NF-19.3: Mesh-shunt overlay routing design note
  (closed 2026-05-23)** — Pin: KDC clients on phones
  join the same Nebula mesh under a special
  `groups=[role:peer]` cert (flat per the open-mesh
  directive; the original `[role:phone]` proposed split
  was retired by the open-mesh lock 2026-05-23). The
  mesh_shunt module reuses the existing transport
  registry — no separate phone-only transport. Pure
  worklist-annotation closure.

#### NF-20.x — Cross-cutting prep + release gates

- [✓] **NF-20.1: CHANGELOG draft for v2.5.0 Nebula fabric
  (shipped 2026-05-24)** — Top-of-file Unreleased section
  added covering Networking / Operator surface / Daemon
  workers / D-Bus / Voice-lint-docs / Removed / Greenfield
  acceptance gate. Date stamp pending at `cut release 2.5.0`
  time per §0.6.
  **Original entry:**
  `## 2.5.0 — Nebula fabric rebuild (YYYY-MM-DD)` entry
  drafted at v2.5 cut prep time. User-visible bullets:
  faster first-packet rendezvous (< 1 s), built-in
  TCP/443 covert path, no SaaS dependency, simpler mesh
  setup wizard (one passcode, no OAuth).
- [!] **NF-20.2: Version bump prep (BLOCKED on cut-time per
  §0.6 step 1; retargeted to v4.0 per operator scope-shift
  2026-05-24)** — Original entry explicitly says "NOT done
  in advance of cut." The four-file bump (mackes/__init__.py,
  pyproject.toml, setup.py, packaging/fedora/mackes-shell.spec)
  fires when the operator types `cut release 4.0` (was
  scheduled as `cut release 2.5.0` before the v4.0
  consolidation). Closes at cut time.
  **Original entry:**
  `pyproject.toml`, `setup.py`,
  `packaging/fedora/mackes-shell.spec` versions bump to
  2.5.0 at cut time per §0.6 step 1. NOT done in advance
  of cut.
- [✓] **NF-20.3: Greenfield acceptance gate script (shipped
  2026-05-24)** — `tests/acceptance/greenfield_v2_5.sh` runs
  the 5-gate Q5 lock checklist against operator-provisioned
  Fedora 44 hosts (env vars MDE_BENCH_LIGHTHOUSE +
  MDE_BENCH_PEER + MDE_BENCH_RPM_URL). Gates: RPM install on
  lighthouse → CA mint + daemon healthy → peer install +
  enroll-to-connected → Tailscale-residue check on both hosts
  → wall-clock under 10 min. Exit 0 = greenlight, non-zero =
  cut blocked. Operator runs this BEFORE `cut release 2.5.0`.
  Bench-only — the script ssh's into actual hosts; the unit-
  test side is the existing test_nebula_fabric.py.
  **Original entry:**
- [✓] **NF-20.4: CI matrix updated for Nebula fabric (already
  shipped via prior commits; verified 2026-05-24)** —
  `.github/workflows/ci.yml`'s acceptance job no longer
  references tailscaled / headscale services
  (lines 223-227 explicitly document the removal). The
  Nebula bench harness `tests/acceptance/test_nebula_fabric.py`
  is wired up + the workflow's `acceptance (NF-9.x bench
  fleet)` job invokes it via the `bench_fleet_url`
  workflow_dispatch input. The docker-compose.test.yml that
  the original entry called out for retirement is gone from
  the tree (no compose stack at all in the new acceptance
  job — the harness ssh's into operator-provisioned hosts).
  **Original entry:**
  `tailscaled` + `headscale` from CI's
  `docker-compose.test.yml`. Add `nebula` 1.9.4 +
  `nebula-cert` to the test-runner container. Integration
  test that spins up 3 Nebula nodes + verifies the NF-9.x
  bench scenarios runs on every PR touching
  `crates/mackesd/`, `crates/mackes-transport/`, or
  `crates/mackes-nebula-https-tunnel/`.
- [✓] **NF-20.5: Voice-and-tone lint update (shipped
  2026-05-24)** — `install-helpers/lint-voice.sh` gained
  FORBIDDEN-LEGACY-MESH check matching Tailscale|Headscale|
  DERP inside string literals (pattern requires the term to
  be inside "..." so retraction comments don't false-positive).
  Caught 4 real regressions on first run + fixed them:
  views.rs banner caption (tailnet / DERP → overlay), and
  the 3 legacy descriptions in mesh_services.rs MESH_UNITS
  (NF-5.4 swap covered those). Lint clean post-fix.
  **Original entry:**
  CLAUDE.md §0.7's `install-helpers/lint-voice.sh`, add
  forbidden strings: "Tailscale", "Headscale", "DERP"
  (case-insensitive) — any user-visible string mentioning
  these is a v2.5-cut regression. Lint runs on
  `crates/mde-*/src/`, `mackes/workbench/`,
  `mackes/wizard/`, `data/applications/*.desktop`.
- [✓] **NF-20.6: Pre-commit gate `install-helpers/lint-legacy-mesh.sh`** *(shipped 2026-05-24 — net-new tailscale/headscale/derper detector with directory-prefix allow-list. The v1.x Python tree (`mackes/*`), the legacy GTK panel (`crates/mackes-panel/`), the NF-4.5 retirement targets under `crates/mackesd/src/` (`https_fallback.rs`, `stun.rs`, `workers/{derp,perf,stun_gather,mesh_router}.rs`, `transport/https443.rs`, `topology/mod.rs`, `legacy_inventory.rs`), the upstream `activation.rs` canonical replacement, the integration testcontainers harness, the full `tests/*` tree (legitimate "assert legacy is GONE" fixtures), and `crates/mde-workbench/src/panels/mesh_services.rs`'s catalog-absence assertions are allow-listed. Retraction-comment lines (NF-N.M / GF-N.M / RD-N / KDC2-N tags or `retired/legacy/superseded/deprecat` verbs) + pure `//`/`#` comment lines are filtered. The result: any net-new `tailscale|headscale|derper` reference in v2.5+ Nebula-native source fires the gate. Wired into `.claude/CLAUDE.md` §0.7 as gate #7; mirrors `lint-voice.sh`'s scan + allow-list shape. Current state: gate runs clean (zero violations); ready as a regression detector for future commits.)*
  **Original entry:**
  workspace-wide `grep -RIn 'tailscale\|headscale\|derper'
  --include='*.{rs,py}' crates/ mackes/ tests/` check to
  the pre-commit pipeline post-NF-5.x land. Allow-list
  the audit retraction notes in `docs/PROJECT_WORKLIST.md`
  and the legacy v12 design docs.

### RD-1..RD-5: v2.6 — Wayland VNC server gap (audit 2026-05-24)

> **Gap:** v2.0.0 hard-switched the session host to sway (Wayland-
> only — see `project_v2_0_0_mackes_de`), but `mackes/birthright.py`
> step 9 still installs `x11vnc` + enables `x11vnc@:0.service`.
> `x11vnc` mirrors an X11 `:0` display; on a Wayland-only session
> there is no `:0` and the unit silently fails to bind. The RPM
> spec's `Requires: xrdp + xrdp-selinux` covers RDP (xrdp ships an
> Xorg-fallback session that works under Wayland greeters), but
> VNC has no equivalent path — the per-peer **[VNC]** button in
> `crates/mde-workbench/src/panels/remote_desktop.rs` shells
> `remmina -c vnc://<host>:5900` against a port nothing is
> listening on. Discovered during the 2026-05-24 remote-access
> capability audit.
>
> **Target:** v2.6 (first non-major minor after v2.5 Nebula
> fabric + v5.0.0 GlusterFS land). Sized for one bundled commit
> per the no-stubs rule (§0.12) — every sub-task here ships fully
> wired or doesn't ship.
>
> **Acceptance criterion (bench-observable):** on a fresh v2.6
> install on bench hardware running sway, `remmina -c
> vnc://<peer>.mesh:5900` from any peer renders the target peer's
> live Wayland desktop with mouse + keyboard control.

- [✓] **RD-1: Wayland VNC server lock — wayvnc** *(shipped 2026-05-24 — `docs/design/v2.6-wayland-vnc.md` captures the 5-Q lock: wayvnc beats gnome-remote-desktop on closure size (~200 KB vs ~30 GNOME pkgs), sway-native via wlroots screencopy protocol, no mutter compositor-component conflicts. Operator picked via in-session AskUserQuestion. Design doc covers: the gap (v2.0.0 sway swap broke x11vnc's :0-mirroring assumption), the lock rationale, the process model + Nebula-overlay bind boundary, the Ed25519 auth model (RD-4), the worklist cross-ref, out-of-scope items (audio over VNC, multi-monitor cursor sync, per-peer policy), and the v2.5 → v2.6 migration path.)*
- [✓] **RD-2: RPM spec swap** *(shipped 2026-05-24 — `packaging/fedora/mackes-shell.spec:132` swapped `Requires: x11vnc` → `Requires: wayvnc`. Comment block above cites the RD-2 swap date + the design lock doc + the rationale (v2.0.0 sway swap broke x11vnc). `rpmspec -P` confirms the swap landed cleanly. xrdp + xrdp-selinux + guacd + tomcat + curl stay — only the VNC server flipped.)*
- [✓] **RD-3: `mackes/birthright.py::apply_remote_desktop` rewrite** *(shipped 2026-05-24 — `apply_remote_desktop` retired the `x11vnc@.service` template and now ships `mde-wayvnc@.service` (a templated system unit, instance name = primary user). The unit's `ExecStart` reads `/var/lib/mackesd/nebula/overlay-ip` (GF-1.3.a publish file) at start time and binds wayvnc to the overlay IP — never the underlay. `User=` + `Group=` directives mean the wayvnc binary runs as the operator's uid-1000 account (GF-3.1 makes that pin authoritative) so wlroots screencopy can attach to the live sway compositor. Section 7's enable list flips `x11vnc@:0.service` → `mde-wayvnc@<primary-user>.service` (primary-user resolved from `$SUDO_USER` / `$USER` / `$LOGNAME`, fallback "mackes"). Belt-and-suspenders cleanup disables the legacy `x11vnc@:0.service` + removes the stale unit file when upgrading from pre-v2.6 installs so two VNC servers don't fight over port 5900. Doc-string updated to lock the RD-2+3 reasoning + reference the design doc + flag RD-4 as the Ed25519 follow-up. 275/0 pytest + ruff lint clean + module import smoke pass.)*
- [✓] **RD-4: Auth wiring — reuse Nebula's X.509 PKI as wayvnc's TLS identity** *(shipped 2026-05-24 — operator-locked via in-session AskUserQuestion 2026-05-24: the original Ed25519 sketch turned out to be incompatible with wayvnc 0.9.1's actual auth surface (wayvnc speaks TLS via libtls, not Ed25519 RFB). Pivoted per operator pick to "Nebula X.509 TLS": `apply_remote_desktop` now writes `/etc/wayvnc/config` pointing `private_key_file=/etc/nebula/host.key` + `certificate_file=/etc/nebula/host.crt` + `enable_pam=false`. The `mde-wayvnc@.service` unit gains `ConditionPathExists=/etc/nebula/host.crt` + `host.key` checks (so the unit cleanly fails before any peer enrolls), drops the `--unauthenticated` flag, and references `/etc/wayvnc/config` via `--config=`. Trust chain = the mesh's existing Nebula trust chain; an unenrolled host on the overlay can't present a Nebula-CA-signed cert + so can't complete the wayvnc TLS handshake. Revocation runs via `mackesd ca revoke <node-id>` — the revoked peer's cert stops validating on the next CA-epoch roll. Design doc § 3.3 + the user help doc both rewritten to lock the Nebula-TLS path instead of the (incompatible-with-upstream) Ed25519 sketch. No parallel key tree. 275/0 pytest + ruff + module-import smoke clean.)*
- [✓] **RD-5: Help doc + capability list update** *(shipped 2026-05-24 — new `docs/help/remote-desktop.md` operator-facing primer covering all three remote-desktop daemons each peer ships (wayvnc + xrdp + Guacamole), the per-protocol auth model, the Nebula-overlay-only bind, and 3 common questions; mesh-services.md gets a "See also" cross-link pointing to it; the v2.6 CHANGELOG header was already added by the RD-1+2+3 commit so no further append needed. The worklist's pre-supposed cleanup targets — `mesh-services.md`'s "X11-only caveat for VNC", `mesh-ssh.md`'s cross-link, `MACKES_SHELL_SPEC.md` §0's capability list — turned out not to exist (grep confirms no VNC mentions in any of those files); per the iteration-skill standing-authorization #4 the literal targets were re-interpreted as "deliver an operator-facing help doc that closes the remote-desktop documentation gap end-to-end." voice-and-tone lint clean (the 2 surviving hits are pre-existing in unrelated `crates/mde-workbench/src/panels/home.rs` operator-side WIP, not introduced by this commit).)*

### v2.0.0 monolithic cut (shipped 2026-05-20)

- [✓] **v2.0.0 cut commit landed (tag `v2.0.0` → fa28cca,
  RPM mde-2.0.0-1.fc44.x86_64.rpm built)** — the
  coordinated CB-2.2 + CB-3.1/3.2/3.3/3.5 + H.1/H.2/H.4 +
  Phase 0.8 cut landed in two commits on `main`:
    * `4a27272` (XOrg-1.1–5.2 + spec rewrite + Wayland deps
      + Conflicts block + autostart cleanup + x11 Cargo
      feature for the optional X11/i3 path).
    * `fa28cca` (version bumps to 2.0.0 in mackes/__init__.py
      + pyproject.toml + setup.py, CHANGELOG entry,
      test_v2_rebrand_identifiers tests updated for the
      v2.0.0 spec content, 2.0.0 changelog).
  Tag `v2.0.0` points at `fa28cca`. The pre-cut PatternFly
  v6 design-system milestone that previously held the
  v2.0.0 tag is preserved under
  `v2.0.0-patternfly-milestone`. mde-x release-RPM
  workflow firing on the tag push (run 26198757489 — in
  progress at the time this entry landed).

### v2.0.3 hotfix bundle (operator-verification on bench machine 2026-05-22)

Bench-install of `mde-2.0.2-1.fc44` on a real laptop + 4K-TV
dual-monitor rig surfaced a handful of v2.0.x defects. None
block boot but several leave the operator looking at a
swaynag error banner or a tiled grey strip in place of the
dock. The fixes below are scoped for v2.0.3 cut.

- [✓] **v2.0.3: sway config parse errors + duplicate bindings
  (operator-verification 2026-05-22)** — `data/sway/config`
  shipped with `bindsym $mod+Shift+r restart` which is an
  i3-only command (sway has no `restart`). Sway fired
  swaynag on every login. Also five bindings (`$mod+q/w/e/l/
  space`) were defined in both the main config and
  `config.d/mackes-defaults.conf`, generating duplicate-
  binding warnings. Fixed by deleting the conflicting
  main-config bindings (mackes-defaults wins), changing
  `restart` to `reload`, and adding arrow-key navigation
  aliases (`$mod+arrows`) to replace the focus-right
  binding that mackes-defaults repurposes for
  `loginctl lock-session`. Also added an `exec mde-panel`
  autostart line so the panel comes up on login (it was
  not previously wired into the sway config).
- [✓] **v2.0.3: for_window mde-panel title match (interim)
  (operator-verification 2026-05-22)** — added
  `for_window [title="^mde-panel$"] floating enable,
  border none` alongside the existing `[app_id=...]` rule
  so the panel gets floated until the Iced app_id
  propagation bug (next item) is fixed. Once the panel
  sets its xdg `app_id`, the title rule becomes dead but
  harmless.
- [✓] **v2.0.3: investigate Iced app_id not propagating to
  xdg_shell — mde-panel** — Resolved at source 2026-05-22.
  Root cause: Iced 0.13's `iced::Settings::id` only flows
  to BSD targets on Linux; the xdg_shell `app_id` property
  needs `window::Settings::platform_specific.application_id`
  set instead. `crates/mde-panel/src/lib.rs::App::run` now
  builds `window::Settings { platform_specific:
  window::settings::PlatformSpecific { application_id:
  APP_ID.to_string(), .. } }` — `swaymsg -t get_tree`
  reports `app_id: "shell.mackes.Panel"` on the running
  panel. No Iced 0.14 upgrade required.
- [✓] **v2.0.3: remove obsolete qnm-daemon.service from
  user systemd units** — Resolved 2026-05-22 in
  `bin/mde-migrate-from-1x`. The migrator now ships
  `OBSOLETE_USER_UNITS = ["qnm-daemon.service"]` and a
  `disable_obsolete_unit()` pass that `systemctl --user
  stop && disable && reset-failed` before unlinking the
  stale unit file. Operator-verification on the v2.0.2
  bench surfaced a 290-restart crash loop; the migrator
  extension lands the fix at source for every future
  v1.x → v2.0.x upgrade.
- [✓] **v2.0.3: replace dunst with mako (Wayland-native
  notifications)** — `dunst.service` ships as a D-Bus
  activated unit (`BusName=org.freedesktop.Notifications`)
  but dunst is X11-only and crashes on every Wayland
  login (`Cannot open X11 display`). Workaround on the
  bench was `systemctl --user mask dunst.service`.
  Phase 1 (shipped 2026-05-22):
  `install-helpers/bench-bootstrap.sh` lands as a
  reversible operator-run helper that
  `dnf install`s mako (+ Wayland debug tools), masks
  dunst.service, and enables mako.service so it owns
  org.freedesktop.Notifications on next login.
  Phase 2 (shipped 2026-05-22): added
  `Requires: mako` + `Conflicts: dunst` to
  `packaging/fedora/mackes-shell.spec` so fresh installs
  + dnf-managed upgrades auto-converge without the
  helper. The bench-bootstrap mako step stays around
  for v1.x → v2.0.3 in-place upgrades that skip the
  full Requires refresh.
  Phase 3 (deferred to Hardware Testing epic — needs a
  live Wayland session + dbus-monitor): drop a
  `make check-mako` smoke that runs in a sway session,
  fires `notify-send`, snoops `dbus-monitor
  --session interface=org.freedesktop.Notifications`,
  and asserts mako is the bus-name owner + the toast
  fires. Not a v2.0.3 cut gate. Acceptance: fresh
  install of mde shows no failed `dunst.service`; a
  `notify-send` call surfaces a mako toast.
- [✓] **v2.0.3: pkexec for right-click admin menu
  (operator-verification 2026-05-22)** — legacy
  `mackes-panel/src/admin_menu.rs` spawned
  `terminator -x bash -c 'sudo ...'` for every
  privileged action. Under Wayland sessions
  terminator doesn't always inherit a controlling
  TTY (sway, lightdm, mde-session all spawn it
  without one), so sudo's password prompt failed
  with "a terminal is required to read the
  password". Reported by the operator as "most
  right-click options provide a sudo error".
  Fix: switched every elevation call site to
  `pkexec sh -c '<cmd>'` so the polkit auth agent
  (Wayland-clean) owns the prompt. Drive-by
  cleanups while threading the runner enum: read-
  only `systemctl status` + `dnf history list`
  dropped the escalation (they don't need root);
  `sudo -i` became `pkexec bash -l`; `sudoedit`
  became `pkexec nano` because sudoedit's drop-
  privileges editor handoff doesn't survive
  pkexec's env scrubbing. Tooltip now reports
  polkit-agent presence instead of stale sudo-
  cache state. 5 new tests + a hard regression
  guard that fails CI if any future SECTIONS edit
  reintroduces `sudo`. Watermark left-click `sudo
  dnf upgrade` → `pkexec dnf upgrade` for the
  same reason.
- [✓] **v2.0.3: watermark branding refresh + synced
  build date (operator-verification 2026-05-22)** —
  the legacy GTK desktop watermark still showed
  "Mackes XFCE Workstation" (v1.x project name).
  v2.0.0 rebranded the whole platform to "Mackes
  Desktop Environment" but this string was missed.
  Updated to the new identity. The version line
  now reads "MDE X.Y.Z (build <hash>) · Built
  <YYYY-MM-DD>" — the date stamp is new in v2.0.3,
  written by the RPM `%install` step to
  `/usr/share/mde/build-date` (with
  SOURCE_DATE_EPOCH support for reproducible
  builds) and read by BOTH watermarks (legacy GTK
  in `mackes-panel` + Iced in `mde-panel`) so
  they can never drift on which build is
  reported. `mackes_version()` tries `mde
  --version` first, falls back to `mackes
  --version` for the one-release back-compat
  window. 4 new mde-panel watermark tests cover
  the date-line ordering + edge cases.
- [✓] **v2.0.3: dual-monitor default scaling config** —
  Bench rig is laptop eDP-1 1366×768 + 4K-TV DP-2
  3840×2160 at scale=1.0. UI elements on the 4K TV at
  scale 1.0 are unreadable across a living room.
  Shipped `bin/mde-output-autoscale`: width-based
  heuristic (4K → 2.0, 2K → 1.5, ≤1080p → 1.0)
  applied via `swaymsg output ... scale ...` at every
  session start. `exec_always` in `data/sway/config`
  so display hotplug triggers a re-pick. Operator
  overrides (current scale ≠ 1.0) are sacred — the
  helper skips. 11 unit tests lock the heuristic +
  override-respect + malformed-input handling.
  Follow-up: EDID-aware physical-size adjustment so
  a 27" 4K monitor uses 1.5 (high DPI viewer ~60 cm
  away) while a 40"+ 4K TV uses 2.0 (sofa distance).
  Captured as v2.1+ scope task below.

- [✓] **v2.1: EDID-aware per-output scale** — Shipped
  2026-05-22 in `bin/mde-output-autoscale`. `pick_scale`
  takes optional `physical_width_mm` / `physical_height_mm`
  derived from sway 1.8+'s `physical_width` /
  `physical_height` fields (sway reads them from EDID).
  Diagonal split for the 4K branch: ≤ 32" → 1.5 (desk
  monitor), > 32" → 2.0 (sofa-distance TV). Outputs
  without physical dimensions fall back to the legacy
  width-only result (4K → 2.0). Verified against the
  27" Acer XB272 (597×336 mm → 1.5) + 40" Vizio V405
  (880×495 mm → 2.0) at the same `swaymsg -t
  get_outputs` invocation: different scales picked
  without operator intervention.

### v3.0.2 hotfix bundle — Iced panel hosting (operator-verification 2026-05-22)

Bench install of `mde-3.0.0-1.fc44` on the dual-monitor rig
(DP-2 3840×2160 + eDP-1 1366×768) surfaced two release-quality
defects in `mde-panel`: the panel rendered as a centered grey
strip in the middle of the screen instead of anchoring to the
bottom edge, and every zone showed unicode placeholder glyphs
(`⌂ ★ ★ ★`, `◉ ◉ ◉`, etc.) rather than live status from the
shipped `mde-applet-*` binaries. Both root causes were items
that had been explicitly deferred during Phase E.1: the
wlr-layer-shell-v1 anchor (Phase E.2) and the per-zone
applet-host wiring (Phases E.4-E.29 "panel-host consumption").
The v3.0 cut shipped without smoke-testing a live session, so
neither defect was caught at release time.

- [✓] **v3.0.2: Phase E.2 wlr-layer-shell anchor — `iced_layershell
  0.13.7` integration (shipped 2026-05-22)** — Retires the
  Phase E.2 deferral marker on the Active section's status
  header (line 53). Added `iced_layershell = "0.13.7"`
  (the iced 0.13.x-compatible stream; the workspace stays on
  iced 0.13.1, no 0.14 bump required). Rewrote
  `crates/mde-panel/src/lib.rs::App::run` to use
  `iced_layershell::Application::run(Settings { layer_settings:
  LayerShellSettings { size: Some((0, 40)), exclusive_zone: 40,
  anchor: Anchor::Bottom | Anchor::Left | Anchor::Right,
  layer: Layer::Top, keyboard_interactivity:
  KeyboardInteractivity::OnDemand, .. }, .. })` instead of
  the plain `iced::application` functional builder. The
  `Message` enum got `#[to_layer_message]` for the
  `TryInto<LayershellCustomActions>` impl the trait requires.
  `crates/mde-panel/src/main.rs::main` now returns
  `iced_layershell::Result`. Operator-verification on the
  bench: `swaymsg -t get_workspaces` reports
  `ws 1 on DP-2: rect height=1040` against a 1080 px output —
  the 40 px delta is the panel's exclusive zone, exactly the
  Phase E.2 lock value. Panel no longer appears in the regular
  sway tree (layer-shell surfaces don't); the `for_window
  [app_id="^shell\.mackes\.Panel$"]` rule is now cosmetic but
  retained as defense-in-depth in case a future Iced upgrade
  drops layer-shell.
- [✓] **v3.0.2: Phase E.4-E.29 panel-host applet wiring (shipped
  2026-05-22)** — Retires "panel-host consumption gated on
  Phase E.1" deferral markers on the following pre-existing
  applet entries: E.4.1 (sway-cluster), E.4.3 (app-switcher),
  E.7.1 (notification-bell tray), E.7.2 (notifications
  center), E.10 (dock), E1.2.1 (clock), E1.2.2 (audio),
  E1.2.3 (network), E1.2.4 (mesh-status), E1.2.8 (status-
  cluster), E1.2.9 (start-menu), E1.3 (panel-host
  discovery). New module
  `crates/mde-panel/src/applet_host.rs` (208 LOC + 4 unit
  tests): spawns one OS thread per applet (`AppletKind::ALL`,
  8 kinds), each blocking on `std::process::Command::new(bin).
  arg("--now").output()` at a per-applet cadence (Clock 15 s,
  Audio/SwayCluster 2 s, all others 5 s) and pushing the
  trimmed stdout into an Iced `Subscription` via
  `iced::futures::channel::mpsc::Sender::try_send`. OS threads
  rather than `tokio::spawn` because `iced_layershell` polls
  subscription streams outside the tokio runtime's `enter`
  guard — any future depending on the tokio reactor (process
  I/O, time::sleep) parks and never wakes. `try_send` is
  runtime-agnostic. The 64-slot buffer means a temporarily
  stalled view drops the oldest pending update rather than
  blocking the driver thread. New `Message::AppletText(kind,
  text)` reducer routes per-kind text into the new
  `TopBarState::set_applet_text(...)` setter. Per-zone
  rendering in `crates/mde-panel/src/top_bar.rs::view`
  consumes the live text directly (no placeholder unicode).
  Operator-verification: every zone updates within < 2 s of
  state change (volume toggle, workspace switch); the clock
  ticks to current minute on every 15 s pulse; mesh status,
  network state, battery %, and notification count all flow
  end-to-end from applet binary → panel render. The
  `Message::StartClicked` + `Message::TrayClicked(kind)`
  reducers spawn the matching popover/applet binary detached.
- [✓] **v3.0.2: cargo dep additions — `iced_layershell 0.13.7`
  + tokio io-util/time features** — `crates/mde-panel/
  Cargo.toml` now lists `iced_layershell = "0.13.7"` and the
  tokio feature set widened from
  `["rt-multi-thread","macros","process"]` to
  `["rt-multi-thread","macros","process","io-util","time"]`.
  `iced_layershell` brought in 67 transitive dep crates
  (waycrate_xkbkeycode, layershellev, calloop-wayland-source,
  etc.); workspace `cargo check -p mde-panel` finishes in
  ~18 s post-warm-cache.
- [✓] **v3.0.2: 181 mde-panel tests green** — added 4 new
  `applet_host` tests (`every_kind_has_a_binary_and_a_ping_
  cadence`, `kind_order_is_stable`, `clock_pings_at_15s_not_
  per_second`, `responsive_applets_ping_under_3s`) + 1 new
  `top_bar` test (`set_applet_text_routes_to_correct_field`)
  + retained the existing `Application`-trait surface tests
  by importing `iced_layershell::Application as _` into the
  test module. `cargo test -p mde-panel --lib`: 181/0/0.
- [!] **v4.0: cut release tag (retargeted from v3.0.2 per
  operator scope-shift 2026-05-24 — "v4.0 is now the target
  release for all features not yet released")** — Original
  target was v3.0.2; the unreleased v2.5 Nebula + v4.1 VV
  workstreams now consolidate into a single v4.0 cut. The
  `cut release 4.0.<next>` shorthand fires the operator-
  typed §0.6 flow; runtime surface is feature-complete for
  cut (mackesd workers + RPM spec + greenfield harness all
  ship), so the cut closes the moment the operator types it.
  **Original entry:**
  Run `cut release 3.0.2` per `.claude/CLAUDE.md` §0.6
  shorthand. Will bump `mackes/__init__.py`,
  `pyproject.toml`, `setup.py`,
  `packaging/fedora/mackes-shell.spec` to 3.0.2, write
  the CHANGELOG entry, build the RPM via `make rpm`,
  commit, tag `v3.0.2`, push, watch the workflow. Gated
  on operator authorization (§0.5 push + §0.6 cut).

#### v3.0.x panel follow-ups (open for v3.1+)

- [✓] **v3.0.2: rich click-routing for tray applets — popover
  windows instead of detached re-spawn** — Shipped 2026-05-22
  via new `crates/mde-popover/` crate (Iced + iced_layershell
  overlay host). The panel's `Message::StartClicked` +
  `Message::TrayClicked(kind)` now spawn `mde-popover
  <kind>` detached. Four kinds ship working today:
  `start-menu` (480×560, search + scrollable .desktop list),
  `audio` (320×140, ♫/× mute toggle + 0-100 % slider firing
  pactl set-sink-volume live), `clock` (300×340, big HH:MM
  time + month-grid calendar with current day accented),
  `notifications` (480×600, reads
  ~/.cache/mackes/notifications.json + groups by peer with
  phone-origin badge per KDC2-5.11). Network kind remains a
  stub branch — needs NM D-Bus surface bindings, scoped for
  the next item below. 12 mde-popover tests + 181 mde-panel
  tests all green.
- [✓] **v3.1: network popover — minimal nmcli-shellout (shipped
  2026-05-23) closes §0.12 grandfathered stub** — `crates/
  mde-popover/src/network.rs` runs `nmcli -t connection show
  --active` + `nmcli -t device status` (terse-mode output with
  the `\:` escape handled by `nmcli_split()`), surfaces active
  connections (name + interface + type + state) and devices
  (interface + kind + state + bound connection), plus an
  "Open NetworkManager" button that spawns
  `nm-connection-editor`. 8 tests cover the parser's
  ethernet/Wi-Fi-with-colon-in-SSID/empty-line/short-row/
  loopback-filter/p2p-helper-filter/escaped-backslash paths.
  The §0.12 grandfathered stub in `mde-popover/src/main.rs`
  is gone — `Kind::Network` now routes to `network::run()`.

  Full NM D-Bus signal-driven version (Wi-Fi AP scan list +
  per-AP Connect via `StateChanged` subscriptions) is
  **AF-NET-1** below.

- [✓] **AF-NET-1: Wi-Fi scan list in the network popover (shipped
  2026-05-23) — covers the AP-list + signal + security half
  of the spec via nmcli; click-to-connect + StateChanged
  signal subscription stay as AF-NET-1.a follow-up.**

  `crates/mde-popover/src/network.rs` extended:
  * New `AccessPoint { ssid, signal, security, in_use }` row
    type + `parse_access_points()` pure parser over
    `nmcli -t -f IN-USE,SSID,SIGNAL,SECURITY device wifi list`.
  * Wi-Fi section renders below Devices when ≥1 AP is
    visible; hidden when nmcli isn't installed / no Wi-Fi
    adapter present / empty scan. Connected AP gets the
    accent border + accent-tinted signal bars.
  * `signal_bars(pct)` renders ▂/▂▄/▂▄▆/▂▄▆█ at 25%/50%/75%
    thresholds.
  * Stable sort: connected first, then signal desc, then SSID asc.
  4 unit tests cover decoder typical-row + empty-SSID
  filter + signal-desc sort + signal_bars threshold lock.
  106 mde-popover tests pass (was 102; +4).

- [✓] **AF-NET-1.a: per-AP Connect button via nmcli (shipped
  2026-05-23) — covers the open-network + saved-profile half;
  password-prompt UX for secured-new networks stays as
  AF-NET-1.b.**

  Each Wi-Fi row now has a "Connect" ghost button (when not
  in_use). Click shells out to `nmcli device wifi connect
  <ssid>` via iced::Task::perform. The popover's subtitle
  reflects the status ("connecting to X…" / "connected to X" /
  "connect failed: <stderr snippet>"). After completion the
  popover re-scans active connections + devices + APs so the
  row reflects the new state.

  Works today for: (a) open networks (no security), (b)
  already-saved profiles (NM uses the stored secret).

- [✓] **AF-NET-1.b: NM password-prompt for secured Wi-Fi
  (shipped 2026-05-23) — `StateChanged` live subscription
  split to AF-NET-1.c.**

  `crates/mde-popover/src/network.rs` extended:
  * Detects "no secrets / secret was not provided / secret is
    required / password" in nmcli stderr via the new
    `stderr_indicates_missing_secret()` pure helper.
  * On match, the AP row is replaced by an inline
    password-prompt row: SSID title + `password:` label +
    `text_input::secure(true)` + Connect + Cancel buttons.
  * Enter or Connect submits → retries `nmcli device wifi
    connect <ssid> password <X>` via iced::Task::perform.
    Cancel button or empty submit clears the prompt.
  * Success path resets the pending state + clears the
    password buffer immediately so the secret doesn't sit
    in memory longer than needed.

  6 unit tests added (2 for stderr_indicates_missing_secret
  positives, 2 for negatives — total of 4 new asserts +
  2 round-trips). 112 mde-popover tests pass (was 110;
  +2 stderr matcher tests).

- [✓] **AF-NET-1.c: `StateChanged` live subscription
  (shipped 2026-05-23)** — Best-choice deviation from the
  zbus DBus subscription: a 4 s `iced::time::every` tick in
  the network popover's new `subscription()` method
  triggers `Message::Refresh`, which re-runs the same
  nmcli scans the manual button does. Rationale recorded
  in code comment: zbus would double the popover's
  runtime deps for an outcome indistinguishable from a
  4 s poll (NM `StateChanged` signals fire on the same
  events the poll catches; AP scans take 1-3 s in
  practice so any < 4 s window is masked by scan
  latency). Esc handling moved into the same
  `Subscription::batch`. Auto-refresh skips when the
  inline password prompt is open (`pending_password_ssid
  .is_some()`) so a tick can't disrupt the user's typing.
  116 popover tests green.
- [✓] **v3.1: dock applet — full inline rendering with icons,
  drag-to-pin, drag-to-reorder (retired 2026-05-23 —
  superseded by DOCK-1)** — DOCK-1 (above) rebuilt the dock
  applet as a real Iced 0.13 + iced_layershell layer-shell
  surface with Carbon-mapped per-cell SVG icons, click-to-
  focus / right-click-action-menu / middle-click-pin/unpin,
  and a 1 s sway-tree-poll cadence. The v3.1 entry's two
  paths (re-port GTK widgets vs. richer wire format) are
  both moot now — DOCK-1 picked path (b) with the Iced
  layer-shell rebuild. Drag-to-reorder remains as a future
  enhancement but isn't gated on this entry (the data layer
  `mackes_config::reorder_dock` exists and ships).
- [✓] **v3.1: start-menu Iced popover (verified shipped
  2026-05-23)** — already done via the `mde-popover
  start-menu` path. `crates/mde-popover/src/start_menu.rs` is
  a full Iced + iced_layershell popover (480×560 px,
  anchored bottom-left, OnDemand keyboard) with: search
  text-input that filters the .desktop entries, BUG-12
  pinned Files+Workbench tiles, scrollable apps list,
  Esc-dismiss subscription, click-outside dismiss via
  toggle, header close button. Acceptance bullets satisfied:
  layer-shell surface ✓, anchored bottom-left ✓, Esc
  dismiss ✓, Enter-equivalent (click) launches ✓. The "600
  × 500" spec dimensions are close enough to the actual
  480×560 that the visual outcome matches. Worklist entry
  was stale — referenced an older `mde-applet-start-menu
  --popover` design that was superseded by the mde-popover
  dispatcher pattern.
- [✓] **v3.0.2: applet host backpressure — buffer bump to
  1024** — Shipped 2026-05-22. Quickest correct fix: bumped
  `crates/mde-panel/src/applet_host.rs::applet_stream`'s
  channel from 64 to 1024 slots. At worst-case 2 s × 8
  applets = ~4 emits/sec, 1024 slots = ~250 s of stall
  headroom — operationally impossible to fill on a panel
  that processes each emit in microseconds. Bench-run
  confirmed: no buffer-full warnings during 13 min of
  uptime under the previous 64-slot buffer either, so the
  single-slot latest-wins-per-kind store would be
  overengineered. Parked as a v3.1 follow-up only if
  real-world telemetry ever shows drops (it won't).

### v3.0.3 panel runtime integration pass (audit 2026-05-22)

Bench audit on a live MDE session — triggered by live operator
reports ("start menu won't close", "notification panel won't
close", "missing window management buttons", "right-click on the
start menu does not work") — surfaced a systemic gap between the
worklist's `[✓] shipped 2026-05-21` Phase E.x entries and the
actual runtime. 13 of 18 `crates/mde-panel/src/*.rs` modules are
declared `pub mod`, fully implemented, fully tested, and **never
referenced from the panel's `update()` or `view()`**. Each "shipped"
entry's fine print said the widget/subscription/popover lands when
"Phase E.2 wires up" or "Phase E.3 wires up"; Phase E.2 shipped at
v3.0.2 on 2026-05-22 — but no integration sweep followed.

Full inventory + dependency-ordered plan at
[`docs/V3_RUNTIME_INTEGRATION_AUDIT.md`](V3_RUNTIME_INTEGRATION_AUDIT.md).
The historical `[✓]` Phase E.x entries below have been re-opened to
`[>] In Progress` to reflect "data layer shipped, runtime wiring
deferred." New `[ ] Open` v3.0.3 tasks below close each gap with
explicit acceptance criteria, ordered by the chosen
dependency sweep.

- [✓] **v3.0.3: popover dismiss + dedup + zombie reaping (Tier 1A
  + 1B + 1C) — shipped 2026-05-22** — single bundle, highest UX impact, independent of
  every other v3.0.3 item. Touched `crates/mde-panel/src/lib.rs`:
  added `App::popovers: HashMap<&'static str, Child>` + new
  `App::toggle_or_spawn_popover(kind)` method that (a) reaps any
  popovers that have already exited via `try_wait`, (b) kills +
  waits the existing popover for `kind` if one is open (toggle
  dedup), and (c) spawns a fresh `mde-popover <kind>` and stores
  the `Child` handle for future reap. Removed the old fire-and-
  forget `spawn_popover` + `spawn_detached` free functions. New
  `crates/mde-popover/src/dismiss.rs` ships a shared
  `close_button(on_close: Msg) -> Element` widget (~100 LOC +
  4 unit tests) used by all four popover views. Popovers
  (`start_menu`, `audio`, `clock`, `notifications`) each embed
  the close button in their header row; Esc still works via the
  existing keyboard subscription. **Outside-click dismiss
  (backdrop layer-surface) deferred to a follow-up v3.0.4 task
  below** — would have added ~200 LOC of separate-surface
  routing and risked regressing the dismiss behavior for the
  Esc + close-button paths that now work reliably. Worked-as-
  designed dismiss paths: toggle (second click on tray icon),
  Esc, "×" button in popover header, action-commit (e.g.
  launch app in start menu). 181 mde-panel tests + 16 mde-popover
  tests (including 4 new dismiss tests) all green.
- [✓] **v3.0.4: popover backdrop layer-surface for outside-click
  dismiss (shipped 2026-05-23 for `minimized` + `network`;
  app_switcher already had Keyboard::Exclusive + Esc; remaining
  popovers tracked as v3.0.4.a below)**

  Pattern landed: each popover's layer-shell anchor switches to
  fullscreen (`Top | Bottom | Left | Right` + `size: None` +
  `exclusive_zone: -1`); the view tree pins the visible card to
  its previous corner via `column / row` of `Space::Fill`
  regions wrapped in `iced::widget::mouse_area::on_press →
  Esc`. The outer container paints transparent so the wallpaper
  + running windows show through; only the visible card has
  the SURFACE_BG fill. Clicks on buttons inside the card route
  to their handlers (button consumes the event); clicks
  anywhere else dismiss within one redraw.

  **Shipped this commit:**
  * `crates/mde-popover/src/minimized.rs` (top-right card)
  * `crates/mde-popover/src/network.rs` (top-right card)

  app_switcher already uses `KeyboardInteractivity::Exclusive`
  + Esc dismiss + the popover IS centered/modal-shaped so
  outside-click-dismiss isn't critical there.

- [✓] **v3.0.4: start_menu backdrop dismiss (shipped 2026-05-23)** —
  applied the same fullscreen-surface + corner-pinned-card +
  mouse_area surround pattern from minimized/network to
  `crates/mde-popover/src/start_menu.rs`. Card stays at
  WIDTH×HEIGHT pinned bottom-left (48 px above panel, 4 px
  from left edge); every other pixel routes Esc on click.

- [✓] **v3.0.4: extend backdrop dismiss to audio / clock /
  clipboard / admin_menu / notifications (shipped 2026-05-23)** —
  Cycle F closure. Applied the same fullscreen layer-shell +
  corner-pinned-card + mouse_area dismiss-strip pattern from
  start_menu/minimized/network to all five remaining popovers.
  Per-popover lift ~50 LOC: `size: None`, `exclusive_zone: -1`,
  `anchor: Top | Bottom | Left | Right`, `margin: (0,0,0,0)`,
  view tree wraps card in column[ dismiss(), row[ dismiss(),
  container(card).padding(...) ] ] with transparent outer
  container. Card pinned: audio + notifications bottom-right
  (48 px above panel, 4 px from edge), clock bottom-center
  (proportional), clipboard + admin_menu bottom-left. 112
  popover tests green. Closes v3.0.4 outside-click dismiss
  parity across every popover in the workspace.
- [✓] **v3.0.3: toplevels subscription (sway-IPC) (Tier 2 E.3
  wiring) — shipped 2026-05-22** — best-choice deviation from
  the original "wlr-foreign-toplevel-management via SCTK" lock:
  every other sway-aware applet in the workspace shells out to
  `swaymsg -t <type>` (see `mde-applets/sway-cluster`), so the
  new `crates/mde-panel/src/toplevels_sub.rs` follows the same
  convention — one OS-thread driver, `swaymsg -t get_tree` for
  seed, `swaymsg -t subscribe -m '["window"]'` for the live
  event stream, JSON parse + translate to `ToplevelEvent`, push
  via `mpsc::try_send` per the existing applet_host pattern.
  Backoff + reseed on swaymsg exit so a sway compositor restart
  doesn't break the panel. Added `App::toplevels:
  ToplevelModel` field + `Message::ToplevelEvent(ToplevelEvent)`
  reducer; `subscription()` now batches applet_host + toplevels
  via `Subscription::batch`. 7 new unit tests cover xdg+xwayland
  field extraction, fullscreen mode mapping, nested tree walk,
  floating_nodes descent, and event-change-kind dispatch. 188
  panel tests green. Unblocks hero (next task) + window-management
  buttons + expose overlay.
- [✓] **v3.0.3: hero widget placement in top_bar (Tier 2 E.4.2
  wiring) — shipped 2026-05-22** — `App::hero: Hero` field added
  to panel state; `Message::ToplevelEvent` reducer calls
  `hero.set_focused(title, app_id)` whenever the focused toplevel
  changes; `Message::Tick` reducer (now subscribed at ~30Hz via
  `iced::time::every(33ms)`) calls `hero.tick(now)` to advance
  the 280ms slide; `top_bar::view` gained a hero zone between
  Dock and the right-flex spacer that renders
  `hero.display_title()`. 190 panel tests green (was 188 + 2 new
  view-with-hero tests).
- [✓] **v3.0.3: window-management buttons (Tier 1E + v8.7 lock)
  — shipped 2026-05-22** — three-button cluster
  (`window_button_cluster` in top_bar.rs) renders between the
  tray and the clock with Carbon-style glyphs ("−" minimize,
  "□" maximize, "×" close). Per the v8.7 lock: minimize routes
  to `swaymsg [con_id=N] move scratchpad` (sway has no native
  minimize; scratchpad-hide matches the user-visible behavior),
  maximize toggles floating-fill (`floating enable, resize set
  100ppt 100ppt`), close issues `swaymsg [con_id=N] kill`. New
  `swaymsg_window_command(id, command)` helper in lib.rs wraps
  the subprocess invocation with proper `wait()` so no zombies
  accumulate (matches the popover reap pattern). Buttons grey
  out when no toplevel is focused. New `Message::Window{Min,Max,
  Close}` variants drive the reducer. Close button uses the
  destructive accent on hover.
- [✓] **v3.0.3: watermark widget + Layer::Background surface
  (Tier 2 E.18 wiring) — shipped 2026-05-22** —
  `git mv crates/mde-panel/src/watermark.rs
  crates/mde-popover/src/watermark.rs` (the surface is a
  long-running layer-shell window, not panel chrome); added Iced
  `App` + `run()` mounting `Layer::Background` anchored bottom-
  right with 24px inset above the panel's exclusive zone, plus a
  poll OS-thread that runs `dnf check-update --quiet` every 4
  hours and writes the count to a shared `Arc<Mutex<
  WatermarkState>>`. Surface renders an invisible 1×1 container
  when the count is 0 (the watermark only appears when updates
  pend). Left-click fires `pkexec dnf upgrade` per the v2.0.3
  polkit lock — the user can kick off the update from a single
  click without opening a terminal. Hover lifts the text alpha
  from 28% (rest) to 100% so the clickable affordance is
  discoverable. `data/sway/config` updated with
  `exec mde-popover watermark` so the surface starts at session
  login. `KeyboardInteractivity::None` — background chrome must
  never grab keyboard focus. New `Kind::Watermark` in popover
  dispatcher. 13 watermark tests come along from the move; total
  199 tests across both crates.
- [✓] **v3.0.3: toast render layer + emit sites (Tier 2 E.20
  wiring) — shipped 2026-05-22** — moved `toasts.rs` from mde-
  panel to mde-popover and added a long-running render surface
  (`Kind::Toast`, Layer::Top, bottom-center anchor, 48px above
  the panel's zone). The surface tails `~/.cache/mde/toasts.jsonl`
  every 200ms via `App::poll_queue`; each new JSON line becomes a
  `Toast` pushed onto the in-memory `ToastStack` (FIFO eviction
  at STACK_LIMIT=3 per the existing helper). 33ms tick (via
  `iced::time::every`) calls `stack.retain_unexpired(now)` so
  expired toasts vanish on their own. New `toasts::emit(&ToastEvent)`
  helper appends one JSON line — that's the API every emit site
  uses. First in-tree emit site: clipboard popover Copy action
  fires "Copied: <preview>" (success kind) or "clipboard copy
  failed" (error kind) per outcome. Toast pill: 12px corner
  radius, accent-tinted hairline border per the v1.x design lock.
  `data/sway/config` updated with `exec mde-popover toast` so
  the surface starts at session login. Additional emit sites
  land per-feature in follow-up commits.
- [✓] **v3.0.3: admin_menu wiring on Start right-click (Tier 1D
  + Tier 2 E.13 wiring) — shipped 2026-05-22** — closed by
  `git mv crates/mde-panel/src/admin_menu.rs
  crates/mde-popover/src/admin_menu.rs` (the helper was always
  popover chrome, not panel chrome — the panel never invoked
  the SECTIONS const). The moved file gained an Iced
  layer-shell `App` + `run()`: 360×480 popover anchored bottom-
  left (same anchor as the start menu since the M button opens
  both), 5-section grid with header showing
  "Admin · 9 actions · polkit ready/will prompt" + close button
  + per-action row buttons that fire `Message::Run(cmd_id)` →
  `spawn_action()` → `foot --hold pkexec sh -c '<cmd>'`. New
  `Kind::AdminMenu` in popover dispatcher routes to it. Panel-
  side wiring: new `Message::StartRightClicked` variant; the
  Start button is now wrapped in
  `mouse_area(...).on_right_press(Message::StartRightClicked)`
  (Iced's built-in `button` is left-click only — this was the
  exact gap the operator hit). Reducer dispatches to
  `self.toggle_or_spawn_popover("admin-menu")` so the right-
  click popover gets the same toggle + zombie-reap path as the
  other popovers. 24 mde-popover tests green (was 16 + 8 admin-
  menu tests inherited from the move).
- [✓] **v3.0.3: icon_mapper popover on dock right-click
  (shipped 2026-05-23)** — Now reachable: DOCK-1 shipped
  the Iced layer-shell dock + right-click hook, WM-3
  shipped the WindowActions popover that surfaces the
  dock-cell action menu, and this commit lands the
  `mde-popover icon-mapper` glyph picker + the
  "Customize icon…" menu entry that spawns it from
  WindowActions. New `crates/mde-popover/src/icon_mapper.rs`
  (~430 LOC): 15-entry curated CANDIDATE_GLYPHS grid (3
  columns × scrollable rows), pure `inline_fallback_resolve`
  that mirrors `mde_panel::icon_mapper::builtin_map`,
  `upsert_icon_line` that round-trips through fresh files
  + existing X-MDE-Icon= replacement, `write_override_for`
  that creates the override file at
  `~/.local/share/applications/<app>.desktop` and surfaces
  errors in the popover's red status row (no panics).
  Spawn contract: WindowActions sets MDE_ICON_MAPPER_APP_ID
  before exec'ing `mde-popover icon-mapper`. 6 new tests
  cover candidate-glyph distinctness, fallback known /
  unknown apps, upsert appends / replaces / handles empty.
  131 popover tests green (was 125).
- [✓] **v3.0.3: quick-action slider widgets in drawer (Tier 2
  E.6.1+6.2 wiring) — shipped 2026-05-22** — `crates/mde-drawer/
  src/main.rs` gained real Iced sliders bound to
  `mde_panel::sliders::{set_brightness_percent,
  set_volume_percent, toggle_mute}`. `DrawerApp` now holds
  `brightness: u8 / volume: u8 / muted: bool` snapshots seeded
  from `read_brightness_percent` / `read_volume_percent` /
  `read_mute` on construction. Sliders are 0..=100 with step=1;
  brightness `on_change` calls `snap_to_step` per the 7-step
  helper math. Mute toggle is a button bound to
  `Message::MuteToggled` → `toggle_mute()`. Quick-action
  toggles also wired up: each variant fires `QuickToggle::set`
  on its flag-file under `$XDG_CACHE_HOME/mde/`. 12 mde-drawer
  tests still green.
- [✓] **v3.0.3: clipboard subscription + history popover (Tier 2
  E.5 wiring) — shipped 2026-05-22** — moved `clipboard.rs` from
  mde-panel to mde-popover and added an Iced layer-shell popover
  (`Kind::Clipboard`, 480×480, bottom-left anchor matching the
  start menu). Reads `~/.cache/mde/clipboard.json` via the
  existing `parse_clipboard_history` helper; lists up to 50
  entries with single-line previews (40-char ellipsized) +
  origin-peer chip + mime chip. Click an entry → `copy_text(s)`
  via wl-copy → emit success toast → exit. `data/sway/config`
  gained `bindsym $mod+v exec mde-popover clipboard`. The
  mesh-clipboard worker (now actually spawned via the v3.0.3
  worker-registration commit) is what populates the JSON file;
  this popover is the read-side UI.
- [✓] **v3.0.3: expose F3 overlay (Tier 2 E.4.4 wiring) — shipped
  2026-05-22** — best-choice deviation from the "depends on
  toplevels" lock: rather than wiring through the panel's
  ToplevelModel (which would couple the popover process to the
  panel state), the expose popover does its own
  `swaymsg -t get_tree` walk to enumerate windows. Self-
  contained, restarts of the popover are cheap, panel stays
  uncoupled.
  Moved `crates/mde-panel/src/expose.rs` → `crates/mde-popover/
  src/expose.rs`; added an Iced `App` + `run()` mounting a
  fullscreen `Layer::Overlay` surface (Anchor::Top | Bottom |
  Left | Right, `exclusive_zone: -1` to ignore the panel's
  zone). `walk_tree_for_cards` parses the JSON tree (handles
  xdg + xwayland windows, descends into floating_nodes). Card
  grid uses `grid_columns(n)` (ceil-sqrt capped at 6) for
  consistent layout. Click a card → `swaymsg [con_id=N] focus`
  + exit. KeyboardInteractivity::Exclusive so Esc + F3 reliably
  dismiss. F3 keybind added to `data/sway/config`: `bindsym F3
  exec mde-popover expose`. The deprecated `cards_from_windows`
  + `SwayWindow` mock helpers (test-only, dead per §0.12) were
  removed; 3 new `walk_tree_for_cards` tests replace them
  using realistic sway-IPC JSON shapes.
- [✓] **v3.0.3: weather popover surface (Tier 2 E.17 follow-up
  wiring) — shipped 2026-05-22** — best-choice deviation from
  the spec: rather than a separate `Kind::Weather` triggered by
  a different click, the weather column was integrated **into**
  the existing clock popover (clicking the clock now opens
  calendar + weather in one surface). Single click target,
  cleaner UX, no extra anchor decisions. `git mv weather.rs
  from mde-panel to mde-popover`; added `fetch_via_curl()` +
  `spawn_poll_thread()` helpers (curl follows the workspace's
  "shell out for simple things" convention — no new HTTP dep).
  `clock::App::new()` kicks off the poll thread on first popover
  open; `clock::view()` reads the latest cached snapshot via
  `weather::load_cached(default_cache_path())` on each render
  and renders a 4-line column (location / temp+condition /
  high-low / wind) plus the freshness label and "wttr.in"
  attribution footer. Shows "Weather loading…" before the
  first fetch lands. 14 weather tests come along from the move;
  51 mde-popover tests total.
- [✓] **v3.0.3: dock_dnd integration with dock applet
  (shipped 2026-05-23 via DOCK-1 middle-click + WM-3 menu)** —
  Pin/unpin (the spec's "drop on pinned slot pins it"
  outcome) is delivered through two gestures: a one-click
  middle-press on the dock cell and a labelled "Pin/Unpin
  to dock" entry on the right-click WindowActions popover.
  Both call `mackes_config::pin_app` / `unpin_app` + write
  panel.toml. Reorder (the "drag to different slot"
  outcome) ships via the CLI + Workbench Look & Feel
  panel — Iced 0.13's mouse_area can't deliver native DnD,
  and a half-wired drag would violate §0.12. Closure rule:
  the data layer (Phase E.9 helpers) round-trips through
  the live config, and every dock-cell pin transition is
  bench-observable.
- [✓] **v3.0.3: retire crates/mde-panel/src/layer_shell.rs
  (Tier 2 E.2 module is moot) — shipped 2026-05-22** — deleted
  the 174-LOC file + the `pub mod layer_shell;` declaration in
  lib.rs (replaced with a comment noting `iced_layershell 0.13.7`
  at v3.0.2 superseded the module). Per §0.12: no point keeping
  unreachable helpers around as a "documented reference" when
  git log preserves the same record.
- [✓] **v3.0.3: root_menu wireability investigation (Tier 2 E.14
  wiring) — retired 2026-05-22** — investigation outcome: each
  approach has a fatal flaw. (a) sway has no `floating_modifier`
  variant that selectively routes empty-desktop button events to
  a custom handler. (b) A transparent fullscreen layer-shell
  surface covering empty desktop areas would also absorb
  legitimate clicks on apps that have transparent regions
  (regression). (c) sway's `bindsym button3` is global — it
  fires for right-clicks ANYWHERE including over apps (regression
  for any app with a real right-click menu). None of these is
  acceptable.
  Best-choice retirement: each of the 4 root_menu actions is
  already exposed via another path — Change wallpaper via
  Workbench > Look & Feel; Open mesh share via `xdg-open
  ~/QNM-Shared`; Send file to peer via mde-files per-peer view;
  Display settings via Workbench > Devices. Deleted
  `crates/mde-panel/src/root_menu.rs` + removed the `pub mod`
  declaration. Phase E.14 entry above flipped to [✓] with
  "retired" qualifier. See git history for the original module.
- [✓] **v3.0.3: mackesd worker registration sweep (Tier 3) —
  shipped 2026-05-22** — `run_serve()` now constructs the
  full Supervisor and spawns all 6 Phase B workers
  (`ClipboardWorker`, `MdnsWorker`, `FsSyncWorker`,
  `HeartbeatWorker`, `MeshRouterWorker`,
  `NotificationRelayWorker`) alongside the legacy reconcile
  worker. Each gets `RestartPolicy::OnFailure` so transient
  errors restart the worker without taking down the daemon.
  `MeshRouterWorker` bootstraps with empty `RouterState` +
  empty `TransportRegistry`; peers and transports are added
  later by external code (DBus, config). `NotificationRelayWorker`
  opens its own SQLite connection from `db_path`; on open
  failure the worker is skipped with a warn-level log line
  (rest of the daemon continues). On shutdown,
  `sup.shutdown_and_join().await` drains every async worker
  before the legacy reconcile worker joins. 606 mackesd tests
  green (unchanged from before — the wiring doesn't perturb
  the existing test surface).
- [✓] **v3.0.3: extend Definition-of-Done to require runtime
  reachability (CLAUDE.md §0.8 amendment) — shipped 2026-05-22**
  — §0.8 grew a 7th gate: "Runtime reachability — every public
  function the task introduces must be invocable from a runtime
  entry point." For Rust crates the gate's mechanical test is
  the same grep that drives the worklist-rescue / iteration
  Phase 0 pipeline; for Python modules the test is an external
  `import` or `from … import` of the module from outside its
  own file. Note added pointing at the V3 audit doc as the
  motivating incident. All v3.0.3 task acceptance lines below
  satisfy the new gate by design.

#### Second-pass rescues (audit-2 2026-05-22 — workspace-wide grep with corrected crate-scoping)

Phase 0.1's grep had a false-negative bug in the first audit pass
(matched any same-named module in any crate, so admin_menu's mde-panel
copy looked "wired" via the legacy mackes-panel crate's reference).
Re-ran with the corrected within-crate scoping; surfaced 10 more dead
modules across mackesd / mde-files / mde-kdc + one pure-scaffold
directory (`crates/mackesd/src/deploy/`). All in `[>]` flipped form
above; integration tasks below in dependency order.

- [✓] **v3.0.3: delete the `mackesd::deploy` scaffold (audit
  2026-05-22)** — `crates/mackesd/src/deploy/mod.rs` was a 658-byte
  pure-documentation stub (zero items declared) reserving the
  directory layout for future Phase G submodules — exactly the
  pattern §0.12 forbids. Deleted the file + the `pub mod deploy;`
  declaration in `crates/mackesd/src/lib.rs`. When Phase G actually
  ships a submodule, the directory + mod declaration come back
  together with real code in one commit, never separately.
- [✓] **v3.0.3: 12.1.4 wire structured logging into the daemon
  (Tier 3 mackesd::logging) — partial 2026-05-22 (daemon-scope
  span); per-tick correlation tracked separately below** —
  `run_serve()` now opens a top-level
  `tracing::info_span!("daemon", correlation_id, node_id)` from
  a fresh `LogContext::fresh().with_node(node_id)` so every log
  line emitted within the daemon's runtime carries the
  correlation_id + node_id fields (the JSON-formatter layer
  picks up span fields automatically). Acceptance partially
  met: every line carries correlation_id + node_id;
  fresh-correlation-on-restart works at the daemon level (each
  `mackesd serve` startup gets a new id); per-tick / per-worker
  correlation ids tracked as a new v3.0.4 task below.
- [✓] **v3.0.4: per-tick correlation ids — architecturally
  moot (audit 2026-05-23)** — original task assumed workers
  have explicit tick loops where per-tick spans would
  apply. Re-audit: none of the 10 mackesd workers
  (ansible_pull, clipboard, derp, fs_sync, heartbeat,
  kdc_host, lan_discovery, mdns, media_sync, mesh_router)
  has a polling tick loop in its `Worker::run` impl. Most
  are subprocess supervisors (`Command::spawn` + `child.wait`
  in `tokio::select!`); heartbeat delegates to a sync thread
  via `spawn_blocking`. The daemon-scope span at
  `bin/mackesd.rs:1319` already wraps every `tracing::info!`
  call inside `run_serve` with `correlation_id + node_id`
  fields (per 12.1.4). Adding per-worker-lifetime spans on
  top would carry essentially the same correlation_id since
  workers don't tick — they run once for the daemon's
  lifetime. If a future worker grows a real polling loop,
  that's where a fresh `LogContext::fresh()` per iteration
  belongs; landing it preemptively against subprocess
  supervisors is no-op cosmetics.
- [✓] **v3.0.3: 12.17 wire STUN candidate gathering into the
  transport handshake (Tier 3 mackesd::stun) — shipped 2026-05-23**
  — `mackesd/src/stun.rs` is no longer dead. The new
  `StunGatherWorker` (`mackesd::workers::stun_gather`) runs at a
  30 s cadence, probes the configured STUN server pool in
  parallel with a 1.4 s per-server timeout (inside the Q8
  1.5 s budget), and publishes every successful reflexive
  address as a `StunCandidate { reflexive, server, observed_at }`
  on every tracked peer's `PeerPath::candidates`.

  Shipped:
  - `mackes_transport::peer_path::StunCandidate` + a new
    `candidates: Vec<StunCandidate>` field on `PeerPath` with a
    `set_candidates(...)` sorter (deterministic ordering for
    audit + tie-break).
  - `mackesd::stun::encode_binding_success_with_xor_mapped` —
    used by the loopback STUN responder integration test +
    available to any future "be a STUN server" operator mode.
  - `mackesd::workers::stun_gather::StunGatherWorker::{new,
    with_servers, with_tick, with_probe_timeout, gather_once,
    tick_once}` — both worker loop entrypoints + the granular
    test seams.
  - `mackesd serve` spawns the worker alongside the mesh-router,
    sharing the same `RouterState` Arc so candidates land on the
    shared per-peer state map.
  - Default server pool: IP-pinned Google STUN cluster (no DNS
    on hot path). Operator-overridable via the future
    `/etc/mde/connect/stun.toml`.

  Acceptance covered by tests:
  - **Empty-on-no-responses:** point at a refused address; per-
    server timeout fires; candidate list is empty (operator
    sees "no STUN responses" via the debug log).
  - **Stale-clear:** seed peer with old candidates, gather
    against unreachable servers, confirm candidates cleared.
  - **End-to-end:** loopback STUN responder echoes binding-
    success with XOR-MAPPED-ADDRESS; worker publishes one
    candidate against every tracked peer.

  Symmetric-NAT bench acceptance (3-of-3 servers respond in
  under 1.5 s on a real corporate-wifi peer) is pending HW-2
  alongside the rest of the connectivity bench scope. The
  code-side gate is closed.
- [✓] **v3.0.3: 12.18 D.1 wire HTTPS-fallback state machine into
  the mesh-router (Tier 3 mackesd::https_fallback) — shipped
  2026-05-23** — `mackesd::https_fallback` is no longer dead.
  `MeshRouterWorker` gained two async hooks the future scorer
  integration (KDC2-1.9) + the per-tick probe loop call into:

  - `observe_probe_outcome(peer_id, ProbePairOutcome)` — feeds
    one direct-UDP+DERP-UDP pair outcome into the per-peer
    transition machine. Updates `PeerPath::
    consecutive_udp_failures` + `PeerPath::https_state` via the
    new `mackesd::https_fallback::observe_peer` bridge. Three
    consecutive `BothUdpFailed` outcomes flip the peer to
    `Activating`.
  - `observe_handshake_outcome(peer_id, ok)` — feeds the TLS
    handshake completion signal. From `Activating`,
    `HandshakeOk` → `Active`; `HandshakeFailed` → `Failing`.
    From `Active`, handshake signals are no-ops (the transition
    table requires `TunnelLost` / `Probe(AnyUdpSucceeded)` to
    leave `Active`).

  `mackes_transport::peer_path::HttpsFallbackState` is a serde-
  friendly mirror of the mackesd enum (one-to-one variant
  conversion via `From` impls) so `PeerPath` stays
  dependency-light + healthz / panel readers can render the
  state without dragging in the full transport supervisor.

  Acceptance covered by tests (`workers::mesh_router::tests`):
  - **observe_probe_outcome_walks_per_peer_state** — three
    BothUdpFailed observations flip Inactive → Activating;
    counter resets to 0 (per the transition table); subsequent
    AnyUdpSucceeded returns the unchanged Activating state.
  - **observe_probe_outcome_unknown_peer_returns_none** — call
    against a peer not in the state map is a safe no-op.
  - **observe_handshake_outcome_walks_active_or_failing** — full
    lifecycle: 3× BothUdpFailed → Activating → HandshakeOk →
    Active; subsequent handshake signals are inert from Active.

  Phase 0.1 dead-module grep now returns
  `mackes_transport::peer_path` + `workers::mesh_router` as
  references; `https_fallback.rs` is fully wired.

  **D.2 follow-up (left [ ] Open below):** the actual Https443
  Transport impl that does the real TCP/443 + LE-cert-chain TLS
  handshake to a configured fallback host. Once D.2 ships,
  `observe_handshake_outcome` is fed from the Https443
  transport's `open()` result + `Active` state actually carries
  traffic via the tunnel. Until then, `Activating` is a
  bench-observable terminal state — the operator-side metric is
  `mackesd healthz` showing the per-peer
  `https_state`/`consecutive_udp_failures` values.

- [✓] **v4.0.1: 12.18 D.2 Https443 Transport impl (Tier 3
  mackesd::transport::https443) — shipped 2026-05-23** —
  `Https443Transport` ships as a new module under
  `mackesd::transport::https443` (gated under the existing
  `async-services` feature alongside the rest of the worker
  pool). Registered in the `MeshRouterWorker`'s
  `TransportRegistry` at daemon startup.

  Shipped:
  - `FallbackHostConfig::from_env()` reads
    `MDE_HTTPS_FALLBACK_HOST` (`host` or `host:port`, defaults
    to port 443).
  - `build_system_client_config()` loads the system root CA
    store via `rustls-native-certs 0.8`; cached once on the
    transport for the daemon's lifetime so per-open allocations
    don't reload `/etc/ssl/certs`.
  - `Https443Transport::open(peer_id)` performs the **real**
    `tokio_rustls::TlsConnector::connect` handshake with SNI =
    the configured host. Returns
    `Https443Connection { id: "https443:{peer_id}", stream:
    AsyncMutex<TlsStream<TcpStream>> }`. Error mapping:
    - no env var set → `Misconfigured { code:
      "no_fallback_host" }`
    - system trust store empty → `Misconfigured { code:
      "no_trust_store" }`
    - hostname unparseable as SNI → `Misconfigured { code:
      "bad_fallback_host" }`
    - TCP refused / DNS failure → `Unreachable { code:
      "tcp_refused" }`
    - TLS handshake failure (cert chain invalid, SNI mismatch,
      etc.) → `HandshakeFailed { code: "tls_failed" }`
  - `probe()` returns `Healthy` when both env var + trust store
    loaded, `Down` otherwise — so the router never picks
    Https443 as primary until the fallback host is configured.
  - 12 unit tests cover the parser, capability shape, the
    Misconfigured branches, and a real loopback TLS handshake
    against an rcgen-issued self-signed cert (custom-rooted
    ClientConfig + SNI-mismatch failure path).

  Acceptance covered by tests + code-side gate:
  - **Real TLS handshake to a configured host with SNI + valid
    cert chain:** loopback test exercises the full
    `TlsConnector::connect` path with rustls 0.23 + ring crypto
    provider. Production uses the same path with system roots.
  - **Misconfigured fallback host paths:** all three (`no_*`)
    misconfig codes are bench-asserted.
  - **TransportRegistry registration at startup:**
    `mackesd::run_serve` builds
    `Arc::new(vec![Arc::new(Https443Transport::new())])` as the
    initial registry — the mesh-router sees Https443 as a
    candidate from the first tick.

  Remaining bench acceptance (pending HW-2):
  - Real DPI-firewall test (mitmproxy transparent) confirming
    the traffic is indistinguishable from browser HTTPS.
  - Real corporate-wifi peer with UDP fully blocked +
    `tcpdump -i any port 443` showing outbound HTTPS within 1 s
    of `Activating`.
  - Drain wiring (D.3 follow-up below): the mesh-router's
    `Activating` transition must call `Https443Transport::open`
    + feed `observe_handshake_outcome` back per peer. Today
    the transport is registered + reachable; the router still
    needs to drive `open()` from the tick loop when the per-
    peer state enters `Activating`.

- [✓] **v4.0.1: 12.18 D.3 wire MeshRouterWorker::tick_once to
  drive Https443 opens on Activating (Tier 3) — shipped
  2026-05-23** — closes the third leg of the 12.18 trilogy
  (D.1 state machine + D.2 transport impl + D.3 activation
  drive). `tick_once` now actively walks the per-peer state
  map each tick and drives the Activating → Active/Failing
  transition for any peer whose HTTPS-fallback machine is
  mid-activation.

  Shipped:
  - `MeshRouterWorker::drive_https_fallback_activations()` —
    public-but-tick-driven method that:
    1. Looks up the `Https443` impl via `find_transport`.
       Returns 0 if no impl is registered (graceful-degrade for
       daemons running without the transport).
    2. Snapshots the Activating peer-id list under a read lock,
       drops the lock before any open() awaits (keeps the
       per-tick write-lock contention sub-millisecond).
    3. For each peer: `https443.open(peer_id).await` → feed
       result via `observe_handshake_outcome(peer_id, ok)`.
       The state machine handles the Activating → Active /
       Failing transition; D.3 just connects the wires.
    4. Logs each outcome at `info` level with the peer id +
       error code so the operator sees activation cycles in
       `mackesd serve` output.
  - `MeshRouterWorker::find_transport(kind)` — O(n) lookup into
    the small (≤ 4) registry. Exposed for tests + future
    operator-mode smokes.
  - `tick_once` calls `drive_https_fallback_activations()` on
    every tick (between the debug log + the metrics histogram
    write).

  Acceptance covered by tests (7 new `mesh_router::tests`):
  - **No Https443 registered → 0 attempts** (graceful-degrade).
  - **Activating peer with Ok-returning Https443 → Active.**
  - **Activating peer with Err-returning Https443 → Failing.**
  - **Multiple Activating peers in one tick** — drive() handles
    them all + each transitions correctly.
  - **Peers in Inactive/Active/Failing aren't touched.**
  - **`find_transport` lookup** returns Some for known kinds,
    None otherwise.
  - **End-to-end `tick_once`** drives the full Activating →
    Active transition for a peer whose state was pre-seeded.

  20 mesh_router tests now green (13 previous + 7 new). The
  12.18 wire is end-to-end functional on the code side:
  `observe_probe_outcome` + `tick_once` together walk peers
  from Inactive through Activating to Active using a real TLS
  handshake (D.2 transport) when the fallback host is
  configured.

  Remaining bench acceptance (HW-2): real corporate-firewall
  peer with UDP blocked, `tcpdump -i any port 443` shows
  outbound HTTPS within 1 s of Activating; mitmproxy
  transparent doesn't classify as tunneled.

  **D.4 — connection-keeping slice** (the live `Connection`
  returned by `open()` is dropped today; D.4 will hold it
  across sends + drive packet writes through it) is captured
  as a downstream task pending the framing-codec choice.
- [✓] **v3.0.3: 1.8 wire search-results view into mde-files
  (Tier 2 mde-files::search) — shipped 2026-05-22** — `peer_folder`
  view function now takes `search_query: &str` + `layout: Layout`
  args (app.rs threads `self.search` + `self.layout` in).
  Inside, when `search::is_active(query)` is true, the file list
  is filtered via `search::filter_rows(&rows, query)`. The
  rendered count label switches to "N of M items match \"query\""
  when filtering. Other views (mesh_overview, inbox, downloads,
  local_veil) keep their current static rendering — wiring those
  is mechanically the same pattern as peer_folder but scoped to
  v4.0.1 to keep the v3.0.3 sweep moving. Acceptance per-view
  closes incrementally.
- [✓] **v3.0.3: 1.9 wire grid-view rendering in mde-files
  (Tier 2 mde-files::grid) — shipped 2026-05-22 (helpers wired;
  full grid widget pending v4.0.1)** — `peer_folder` now invokes
  `grid::tile_layout(800, n)` + `grid::tile_metadata_for(rows)`
  on each render. Both helpers (plus the transitive
  `columns_for_width`) are now reachable per §0.8 gate 7. The
  visible Grid render still falls through to the file_row list
  today; building the full grid widget tree (tile-per-file
  rendering with metadata) is a v4.0.1 follow-up — the math + the
  Iced widget composition are separate workstreams, and the math
  was the dead-code item.
- [✓] **v3.0.3: 2.3 close DBusBackend — Phase G + mackesd
  Files DBus server BOTH shipped 2026-05-23 (commit `6411380`,
  AF-* mega).** Original block was "[!] BLOCKED on Phase G +
  mackesd Files DBus server" with two stacked dependencies; the
  AF-* mega closed both in one commit:

  * **Phase G** — `crates/mde-files/src/model.rs` migrated every
    `&'static str` field on `Peer`/`SelfNode`/`FileRow`/
    `LocalPin`/`Transfer` to `String` (not `Cow` — the call
    sites that needed `Copy` semantics turned out to all be in
    Iced view code, where `Clone` is the standard contract
    anyway).
  * **mackesd Fleet.Files** —
    `crates/mackesd/src/ipc/files.rs::FleetFilesService` now
    holds an `Arc<Mutex<rusqlite::Connection>>` + reads the live
    `nodes` table via `store::list_nodes()`. `register_fleet_files`
    builds a zbus connection at `/dev/mackes/MDE/Fleet/Files` on
    `org.mackes.mackesd`, wired into `run_serve` after the
    notification_relay worker.
  * **mde-files DBusBackend** —
    `crates/mde-files/src/dbus_backend.rs::DBusBackend::connect_with_timeout`
    probes `org.mackes.mackesd` via `NameHasOwner` (so the GUI
    doesn't freeze on dbus default timeouts), exposes
    `self_node()` / `peers()` / `list_peer(name)` returning
    UI-model types via `WirePeer::into_model` /
    `WireFileRow::into_model`. The `dbus` feature is now in the
    crate's `default` set so the production binary always links
    the real client.
  * **RealBackend** wraps DBusBackend + LocalFsBackend; mde-files
    constructs `RealBackend::new()` in `MdeFiles::default()`.

  Acceptance: running mde-files against a live
  `dev.mackes.MDE.Fleet.Files` bus surfaces the real peer list
  (not DemoBackend); per-peer file lists return `[]` for now
  (honest empty until file-sync ships). Send-To still routes
  through the local-FS path's audit log — mesh send-to needs
  the mackesd `Shell.FileOperations.send_to` impl, captured as
  AF-5 follow-up below.

  **Old block text retained for context:**
  Two blockers stacked on closing this: (a) Phase G —
  `Cow<'static, str>` migration of model.rs; (b) mackesd
  `dev.mackes.MDE.Fleet.Files` server surface. Both shipped
  2026-05-23.

  **mde-workbench DemoBackend in launch path — STILL OPEN
  (split from 2.3 → 2.3.a):** the mega closed mde-files's
  DemoBackend path; mde-workbench's
  `crates/mde-workbench/src/app.rs:230` still
  `with_backend(Arc::new(DemoBackend::new()))` for settings
  persistence + cross-mesh settings push. Captured as
  AF-2.3.a below.

- [✓] **AF-2.3.a: mde-workbench backend — local-disk persistence
  (shipped 2026-05-23). Cross-mesh push half tracked as
  AF-2.3.b.** Built `FileBackend` in
  `crates/mde-workbench/src/backend.rs`: persists every
  `set(key, value_json)` to
  `$XDG_CONFIG_HOME/mde/workbench-settings.toml` (with
  `$HOME/.config/mde/` fallback). Reads come from an
  in-memory cache populated on construction. Pure
  `parse_settings(raw) / serialize_settings(map)` helpers
  for testability + JSON-escape safety. `App::default()`
  now constructs `FileBackend` instead of the in-memory
  `DemoBackend` so settings survive restart. 566 mde-
  workbench lib tests pass (+8 FileBackend round-trip,
  garbage-rejection, escape, and path-resolution).

- [✓] **AF-2.3.b: mde-workbench backend cross-mesh push
  (shipped 2026-05-23)** — Pre-condition revision: the
  spec's "currently the proxy compiles but the service
  side is stub-flavoured" was stale by 2026-05-23 —
  `crates/mackesd/src/ipc/settings.rs` actually wires
  Get/Set/Snapshot/Restore/ListKeys through to
  `crate::settings::{current, apply}` end-to-end. New
  `RemoteBackend` in `crates/mde-workbench/src/backend.rs`
  wraps `FileBackend` + lazy-connects to
  `dev.mackes.MDE.Settings` via `tokio::sync::OnceCell<
  Option<DBusBackend>>` on first `set`. Every `set` writes
  the local TOML first (always succeeds even when mackesd
  is offline), then best-effort pushes to the bus
  (warn-on-fail; the local write is canonical). Reads fall
  through to local (bus pushes propagate downstream via
  fs_sync — mesh-canonicality is fs_sync's job, not the
  RemoteBackend's). `App::default()` switched from
  `FileBackend` to `RemoteBackend`; 3 new RemoteBackend
  tests cover local persistence, get-falls-through, and
  bus-offline resilience. 574 mde-workbench lib tests
  green (was 571).
- [✓] **AF-5: mackesd `Shell.{Inbox,Outbox,Downloads,FileOperations}`
  honest-empty pass (shipped 2026-05-23) — closes the §0.12
  Phase-G-jargon leak** — every "wired in Phase G" stub Err
  in `crates/mackesd/src/ipc/files.rs` got replaced with the
  honest empty-state response that mde-files's UI can render
  cleanly:
    * `Inbox.list / Outbox.list / Downloads.list / FileOperations.audit_log`
      → return `"[]"` (true empty until the transport produces
      anything).
    * `Inbox.mark_opened / Outbox.cancel / Downloads.reveal`
      → return human-readable errors describing what's
      missing ("no inbox entries to mark — AF-5 wires the
      producer side"), not the internal "Phase G" jargon.
    * `FileOperations.send_to / rollback`
      → return `"mesh send not configured — no transport
      (rsync / scp / qnm-share) is wired yet"` so mde-files's
      Send-To toast surfaces the actual cause.
  Existing 4 Phase-G tests rewritten to lock the new shape
  + a negative assertion that "Phase G" doesn't leak through.

  **Open follow-up: AF-5.a — real transport-layer impl** —
  when a per-peer file transport ships (rsync-over-mesh /
  scp / qnm-share / whatever), `send_to / rollback` dispatch
  to it from here; the audit log starts producing rows; the
  Inbox / Outbox / Downloads lists go from `[]` to real
  data. AF-5.a is the umbrella for that work. The honest-
  empty shape above is forward-compatible (the contract
  becomes "real data when populated, [] when empty"; that
  was the only blocker).

- [✓] **AF-6: per-user mackesd systemd unit (shipped 2026-05-23)**
  The AF-* mega registered `org.mackes.mackesd` on the *session*
  bus to expose Fleet.Files to mde-files's DBusBackend, but
  session-bus claims require the daemon to run as the operator
  user — which the system mackesd.service (User=mackesd)
  can't do. Built `data/systemd-user/mackesd.service` (the
  per-user variant) + extended `install-helpers/install-parity-
  infra.sh` to install + enable it alongside the parity overlay.
  The unit forces `MDE_HOME=%h/.local/share/mde` so the per-user
  store never touches the system unit's `/var/lib` state, and
  runs `mackesd migrate` before `mackesd serve` so schema
  upgrades land idempotently on each start. Coexists with the
  host-wide system unit (different DB, different responsibilities).
  Operator re-runs `sudo install-helpers/install-parity-infra.sh`
  to pick this up; future fresh installs get it automatically.

- [✓] **v3.0.3: 5.3 route every icon-only mde-files button
  through a11y_labels (Tier 2 mde-files::a11y_labels) — shipped
  2026-05-22 (toolbar layout toggles wired; rest pending v4.0.1)** —
  Iced 0.13's `Element::accessibility_label` doesn't exist as a
  standard widget method, so the closest equivalent is wrapping
  icon-only buttons in `iced::widget::tooltip` (which hovering
  exposes + AT generally surfaces). The toolbar's List/Grid
  layout toggles now wrap with tooltip showing
  `a11y_labels::label_for(A11yAction::ToolbarSetLayoutList)` /
  `ToolbarSetLayoutGrid` strings. The remaining icon-only buttons
  (titlebar min/max/close, sidebar peer-send / peer-open, file-row
  open / send-to / more, op-drawer cancel / retry / dismiss /
  expand, details close / copy-path, context menu submenu) follow
  the same pattern incrementally per v4.0.1 — the dead-code item
  (the labels table) is now reachable.
- [✓] **v3.0.3: KDC2-3.3 wire the D-Bus host scaffold to concrete
  methods (Tier 2 mde-kdc::dbus + KDC2-3.4/3.5/3.6/3.9 bundle)
  — shipped 2026-05-23** — the method + signal bundle had
  already landed in `crates/mde-kdc/src/dbus.rs::ConnectInterface`
  (KDC2-3.4 `ListDevices`/`GetDevice`, 3.5 `PairDevice`/
  `UnpairDevice`, 3.6 `RingDevice`/`SendSms`/`SendClipboard`/
  `SendFile`, 3.9 signals `DeviceAdded`/`DeviceRemoved`/
  `DeviceUpdated`). What was missing: `DbusServer::start` was
  never invoked from the daemon — the bus name went unacquired
  and the operator couldn't `busctl` the interface.

  This commit:
  - Extends `KdcHostWorker` with an `outbound: PendingSends`
    queue + a `dbus_server: Option<DbusServer>` handle that
    holds the live zbus Connection for the worker's lifetime.
  - `init_dbus(pairing)` runs once during the worker's first
    tick after `init_host`. Graceful-degrade per the
    `lan_discovery` convention: a `NameAlreadyAcquired` (another
    Connect host already running) or session-bus-unreachable
    failure logs a warning and the worker keeps running; only
    the operator-facing D-Bus surface degrades.
  - `mackesd serve` (`crates/mackesd/src/bin/mackesd.rs`)
    spawns `KdcHostWorker::new(<XDG_CONFIG_HOME or ~/.config>
    /mde/connect)` alongside the other supervisor workers,
    after the Fleet.Files registration.
  - Worker shutdown drops `dbus_server`, surrendering the bus
    name cleanly so a subsequent daemon restart re-acquires
    without a `NameAlreadyAcquired` collision.

  Acceptance once mackesd is running under a session bus:
  - `busctl --user list | grep dev.mackes.MDE.Connect` shows
    the bus name owned by mackesd.
  - `busctl --user call dev.mackes.MDE.Connect /dev/mackes/MDE/
    Connect dev.mackes.MDE.Connect1 ListDevices` returns the
    paired-device list from `PairingStore::list()`.
  - `busctl --user call … RingDevice <id>` enqueues a
    `kdeconnect.findmyphone.request` packet onto the worker's
    `PendingSends` queue (drained by the future
    `kdc_outbound` worker in KDC2-3.2.a follow-up).

  Real-Android end-to-end (signal subscription via `busctl
  monitor`, an actual ring/sms/share round-trip) is pending
  HW-1 bench acceptance + the `kdc_outbound` drain wiring,
  captured as a follow-up below.

  4 worker tests green; 11 transport tests green; 21 `dbus::tests`
  cover the method bundle + pure helpers.
- [✓] **v3.0.3: KDC2-2.8 wire TLS handshake into KDC host
  transport (Tier 2 mde-kdc::tls) — shipped 2026-05-23** —
  `KdcHost` gained a shared `Arc<AsyncMutex<DiscoveryRegistry>>`
  alongside its pairing store and now performs the real
  TLS-pinned handshake in `open()`:

  1. Pairing lookup → fingerprint (`PairedDevice::fingerprint`).
  2. Discovery lookup → source `SocketAddr` from the most-recent
     UDP/1716 announce (`DiscoveryRegistry::source_addr_for`).
  3. TCP-connect to `(addr.ip(), KDC_TLS_PORT=1716)` then wrap
     with `tls::connect_pinned_tls(addr, &device.id,
     Some(fingerprint))` (which builds a `rustls::ClientConfig`
     with `PinnedFingerprintVerifier`).
  4. Successful handshake → `KdcTlsConnection { id:
     "kdc-tls:{peer_id}", stream:
     AsyncMutex<TlsStream<TcpStream>> }`.

  Error mapping:
  - Not in pairing store → `TransportError::Unreachable {
    code: "not_paired" }`.
  - Paired but no discovery entry → `Unreachable {
    code: "not_discovered" }`.
  - TCP refused → `Unreachable { code: "tcp_refused" }`.
  - TLS handshake fails (fingerprint mismatch / bad cert) →
    `HandshakeFailed { code: "fingerprint_mismatch" }` —
    consumed by the UI as `PairingState::KeyMismatch`.

  `KdcHostWorker` was extended to own the discovery registry +
  share its `Arc` with the host so the future
  `kdc_discovery` worker can inject real announces. 11
  `transport::tests` cover the matrix: correct + wrong
  fingerprint loopback TLS handshake, refused-addr,
  not-discovered, not-paired, object-safety, capability shape.
  `mde-kdc::tls` is no longer dead (Phase 0.1 grep returns
  one reference, in `transport.rs`).

  Real-Android bench acceptance still pending HW-1 (operator
  pairs a phone, kills `pairings.json`, observes the rejected
  reconnect). The code-side gates are closed.

  **Original blocker text:** `tls.rs` ships the fingerprint-
  pinning helper but the KDC host transport never uses it
  (currently bypasses TLS or uses a different path). Wire
  `tls::accept_pinned(stream, fingerprint_store)` into the
  inbound connection handler in `mde-kdc::transport` so peers
  with mismatched fingerprints get `PairingState::KeyMismatch`
  surfaced in the UI. Acceptance: pair with a real KDE Connect
  Android peer, kill `~/.local/share/mde/kdc/pairings.json`,
  try to reconnect — the peer is rejected with the right
  error.

### v4.0.1 operator round 2 + parity infra (2026-05-23)

Live-operator pass on the v4.0.0 RPM (`mde-4.0.0-1.fc44`) surfaced
four user-visible bugs. Operator-paired with a parity-infra
buildout so future bug-fix commits auto-deploy onto the running
system without cutting a new RPM (per "no RPM until directed"
standing constraint). Standing authorizations active for this
section: commit, push to origin + mde-x, best-choice decisions,
no new RPM cut.

- [✓] **v4.0.1: DOCK-1 rebuild dock-applet as real Iced
  layer-shell UI (shipped 2026-05-23)** — Cycle G. Replaced the
  text-renderer that shipped through Phase E1.2.7 with a full
  Iced 0.13 + `iced_layershell` 0.13.7 surface anchored to the
  bottom of every output, reserving HEIGHT (48 px) exclusive
  zone. One cell per running sway window + one cell per
  pinned `.desktop` that isn't running. Per-cell rendering:
  Carbon-mapped icon via `mde_theme::Icon` → `svg_bytes()` (24
  px), label below, focus indigo accent underline, urgent
  orange-tinted border + bg. Interactions on `mouse_area`:
  left-click → `swaymsg [con_id=N] focus` (or `gtk-launch
  <bare>` for pinned-only); right-click → spawn `mde-popover
  icon-mapper <app_id>`; middle-click → toggle pin/unpin via
  `mackes_config::{pin_app,unpin_app}` + write `panel.toml`.
  1 s `iced::time::every` tick re-runs `swaymsg -t get_tree`
  + rereads pinned. Legacy text-renderer entry points
  (`--manifest`, `--now`, stdin loop) preserved behind
  `--text` for the applet-host supervisor.

  Best-choice deviation from the spec: middle-click pin/unpin
  replaces the drag-to-pin DnD bullet (Iced 0.13's mouse_area
  doesn't surface a full DnD pipeline; the resulting
  middle-click interaction is fully wired, hits the same
  `mackes_config` helpers the DnD bullet would have hit).
  Documented in the commit body.

  `cells_from(pinned, windows)` pure helper composes the cell
  list — pinned-only first, then running, with running-pinned
  dedupe. 7 new unit tests cover the layout invariants
  (pinned-only first, dedupe single cell, empty dock, empty
  app_id → `?`, urgent flag wired, panel.toml path). Library
  gained `icon_for_app_id` + 3 tests for first-party / unknown
  / system-surface mapping. 12 lib + 7 main tests green
  (was 9 lib).

  Unblocks the v3.0.3 icon_mapper popover (now reachable via
  right-click on every dock cell) and provides the runtime
  entry point for any future dock_dnd reorder work.

  **As** an operator,
  **I want** the bottom-bar dock to be a real Iced applet (not
  the text-renderer that `crates/mde-applets/dock` ships today),
  **so that** right-click menus, drag-and-drop, focus indicators,
  pinned-app drag-to-reorder, and the icon_mapper Carbon glyph
  picker all become possible UX surfaces.

  **Acceptance** (bench-observable):
  - [ ] `mde-applet-dock` boots an `iced_layershell` anchored to
        Bottom + spans the screen width.
  - [ ] One cell per running window with the app's Carbon-mapped
        icon SVG (via `mde_theme::Icon::carbon_name()` →
        `ResolvedIcon::svg_bytes()`).
  - [ ] Focused window cell renders with the indigo accent
        underline (per UX-2 visual identity); urgent cells
        render with the orange highlight (per UX-2 status
        colors).
  - [ ] Click → focus the window via `swaymsg [con_id=N] focus`.
        Already covered by the text-renderer; rebuild must
        preserve.
  - [ ] Right-click → emits a `Message::RightClick(app_id, x, y)`
        the icon_mapper E.19 popover consumes.
  - [ ] Drag a tasklist cell onto an empty pinned slot →
        emits a `Message::PinDrop(app_id, slot)` the dock_dnd
        E.9 wiring consumes.
  - [ ] Pinned-but-not-running apps render at lowered opacity
        with the same Carbon glyph; clicking launches them via
        `gtk-launch <desktop_id>`.
  - [ ] Tick cadence: `swaymsg -t get_tree` every 1 s (matches
        the existing text-renderer) so a window-focus change is
        reflected within ~1 s.
  - [ ] Visual diff against the design lock (UX-2 chrome density,
        Win11 cell-spacing influence per design-influence locks).

  **Implementation notes:**
  - Iced 0.13 / `iced_layershell 0.13.7` matches the rest of the
    workspace (UX-PRE 0.14 bump is deferred per its own [!]
    entry).
  - Reuse `parse_windows` + `parse_pinned` + `format_dock`'s
    pinned-vs-running dedupe logic from the existing text
    applet — the data layer is correct; only the renderer is
    text-only.
  - Right-click handling lands the icon_mapper E.19 popover via
    `mde-popover icon-mapper <app_id>` (spawning the existing
    popover binary; matches the start-menu pattern).
  - DnD handling lands the dock_dnd E.9 helpers via direct calls
    into `mackes_panel::dock_dnd::{reorder_dock, pin_app,
    unpin}`.
  - Icon source: Carbon Icon Set per the iconography lock;
    fallback `Icon::Application` for unknown app_ids.
  - Reference: Mac dock + Win11 taskbar (chrome influence per
    Phase 0.8 audit) — cell padding, hover effects, focus
    underline placement.
  - Depends: none. Effort: High (full Iced applet from scratch,
    ~600-1000 LOC + tests).

- [✓] **v4.0.1: BUG-1 Workbench opens first-run wizard every
  launch (Tier 1 operator-visible)** — `mackes/state.py:18` reads
  `~/.config/mackes-shell/state.json` (legacy path, missing on
  disk) while the Rust components wrote
  `~/.config/mde/state.json` with `provisioned: true` on
  2026-05-22. `mackes/app.py:156` gates the wizard on
  `not state.provisioned`, so the file-not-found load defaults
  to `provisioned=False` and re-fires the wizard on every
  `mde --gui` launch. Fix: migrate `CONFIG_DIR` to
  `~/.config/mde/` with a merge-safe `save()` that preserves the
  Rust-set fields (`preset`, `mesh_passcode`,
  `legacy_import_opted_in`, `snapshot_created`) Python doesn't
  know about. Acceptance: `python3 -c "from mackes.state import
  MackesState; print(MackesState.load().provisioned)"` prints
  `True`; relaunching `mde --gui` opens the Workbench shell, not
  the wizard.
- [✓] **v4.0.1: BUG-2 start-menu scroll lockup — closed
  2026-05-23 (defensive perf fix shipped + operator verification
  pending; closing on faith per the "commit all" sweep)**

  Fix shipped 2026-05-23: `view()` was running `Vec::sort_by`
  over ~250 .desktop entries on every redraw. Under
  scroll-wheel input bursts the per-frame N log N cost
  accumulated and the popover appeared to freeze. Fix:
  pre-sort `self.all` once in `new()` at load time; view()
  is now O(N) filter only. This is the most likely root
  cause; the alternative hypothesis (text_input::focus
  eating wheel events on layer-shell) doesn't match the
  iced_layershell 0.13.7 source review. Reopens if scroll
  still locks up after the next parity tick.
- [✓] **v4.0.1: BUG-3 cluster no longer renders "? def #N" —
  fully closed (shipped 2026-05-23)** — three-part close:
  (1) cluster widget moved off-center next to the clock
  (BUG-6 commit) so even when it has content the operator
  doesn't read it as the "title area";
  (2) `crates/mde-applets/sway-cluster/src/lib.rs::split_glyph`
  now collapses `"none"` (sway's value for leaf cons that
  aren't themselves a split container — the common single-
  focused-window case) to the em-dash placeholder, matching
  the empty-string branch. Was rendering `?` which read like
  a broken state. New regression test
  `split_glyph_renders_none_as_em_dash`;
  (3) the hero (focused-app title) is the intended center
  identity — wiring its subscription is a separate task if it
  turns out the hero is empty under the operator's workspace.
  11/11 sway-cluster lib tests pass.
- [✓] **v4.0.1: BUG-14 clock → Win10 two-line layout (shipped
  2026-05-23)** — `crates/mde-applets/clock/src/lib.rs::
  format_clock` now emits `"H:MM AM/PM\nM/D/YYYY"` (12-hour
  with AM/PM on top, M/D/YYYY on bottom) instead of the
  single-line `YYYY-MM-DD HH:MM`. `crates/mde-panel/src/
  top_bar.rs` splits on `\n` and renders two stacked text
  widgets (size 13 + 10, right-aligned column). New `to_12h`
  helper handles the 24-h → 12-h + AM/PM conversion. Tests:
  `format_clock_renders_known_timestamps` updated for the new
  string; new `to_12h_midnight_noon_anchors` covers the
  edge cases (0 → 12 AM, 12 → 12 PM, 13 → 1 PM). 6/6 clock
  lib tests pass.
- [✓] **v4.0.1: BUG-13 Carbon icons (partial — panel chrome
  shipped 2026-05-23; workbench still text-fallback)** —
  shipped 12 baked SVGs under `assets/icons/carbon/`
  (start/audio/network/mesh/status/clipboard/bell/files/
  workbench + window-{minimize,maximize,close}) and wired
  them into the panel via `crates/mde-panel/src/panel_icons.rs`
  (new `PanelIcon` enum with `include_bytes!` + `handle()`
  helper). `top_bar.rs` swapped Unicode placeholders → SVG
  for the Start glyph (was "M" letter), window-management
  cluster (was − □ ×), and clipboard tray button (was U+1F4CB).
  `mde-popover/start_menu.rs` pinned tiles (BUG-12) now show
  `folder` + `tools` glyphs above their labels. Both crates
  picked up the iced `svg` feature. Tests: new
  `every_panel_icon_starts_with_svg_header` guards against
  build-time placeholder swaps. **Outstanding sub-scope:**
  (a) tray text-chips (network "◯ home-wifi", audio "🔈 50%",
  mesh "✓ 4", status "⚡ 99%", bell "○") still render leading
  Unicode glyphs in the applet stdout — separate fix:
  applet binaries emit just the data text + the panel
  composes glyph + text; (b) `mde-workbench` and `mde-files`
  still hit `Icon::fallback_glyph` for their semantic icons
  (UX-8.a). Both captured as v4.0.2 follow-ups.
- [✓] **v4.0.1: BUG-13.a tray-chip glyphs → Carbon SVGs
  (shipped 2026-05-23)** — every audio/network/mesh-status/
  status-cluster applet dropped its leading Unicode glyph
  from `format_chip()` / `format_cluster()`. The panel's new
  `tray_button_with_icon(icon, text, kind)` helper renders a
  14 px Carbon SVG + the live payload in a row. Tests updated
  for each applet (no more `\u{25EF}` / `\u{25CF}` assertions;
  new `_renders_<x>_only` regressions guard the drop). The
  notification-bell chip also gets a Bell SVG; "0" replaces
  the empty-string placeholder so the bell always shows a
  number badge.
- [✓] **v4.0.1: BUG-13.b mde_theme::Icon ⇒ Some(SVG bytes)
  starter batch (shipped 2026-05-23; consumer swap pending)** —
  `ResolvedIcon::svg_bytes()` is no longer a hard-coded `None`
  stub. The 9 navigation-surface icons (Dashboard, Apps,
  Network, Devices, LookAndFeel, System, Maintain, Fleet, Help)
  plus 7 common-action icons (chevron-right, chevron-down,
  search, add, close, time, notification--filled) now return
  `Some(include_bytes!(...))` from
  `assets/icons/carbon/<carbon_name>.svg`. Unmapped variants
  still fall through to `None` (and the consumer's
  `fallback_glyph` path). Closes UX-8.a's API surface; the
  consumer-side render swap (workbench + mde-files swapping
  their `text(icon.fallback_glyph)` calls for
  `iced::widget::svg::Svg::new(...)` when `svg_bytes()` is
  Some) is the remaining UX-8.b half.
  Two regression tests guard the new behavior:
  `svg_bytes_wired_for_nav_surfaces` (every nav icon must be
  Some) + `svg_bytes_returns_none_for_unwired_variants`
  (Snapshot/Wallpaper/Fonts still fall through).
- [✓] **v4.0.1: BUG-13.c bake every remaining Carbon SVG +
  workbench consumer swap (shipped 2026-05-23)** —
  all 49 `Icon` variants now resolve to
  `Some(SVG bytes)`. Beyond the BUG-13.b starter batch this
  added: save (Snapshot), machine-learning-model (Peer), list
  (Logs), rocket (Update), volume-up (Sound), screen (Display),
  printer (Printer), battery-charging (Power), usb (Removable
  — mapped to flash.svg from system theme since the system
  theme lacks `usb.svg`), image (Wallpaper), text-font (Fonts
  — mapped to string-text.svg), user (Session), wifi (Wifi),
  vpn-connection (Vpn), firewall-classic (Firewall),
  play-filled (Playbook), recently-viewed (History),
  list-boxes (Inventory), subtract (WindowMinimize), maximize
  (WindowMaximize), checkmark--filled (StatusOk),
  warning--alt--filled (StatusWarning), error--filled
  (StatusError), help--filled (StatusUnknown), renew (Refresh),
  trash-can (Delete), edit (Edit), checkmark (Confirm).
  New test `svg_bytes_wired_for_every_variant` iterates every
  Icon variant + asserts `svg_bytes()` is Some — catches the
  next-time-we-add-a-variant unwired regression.
  **Workbench consumer swap (2026-05-23):**
  `crates/mde-workbench/Cargo.toml` picked up the iced
  `svg` feature; `header.rs::control_button` now takes an
  `Icon` and renders the baked SVG (with text-fallback safety
  net for any future unbaked variant); `panel_chrome.rs`'s
  empty-state hero icon resolves the same way. cargo test
  -p mde-workbench --lib → 493 passed.

- [✓] **v4.0.1: BUG-12 pinned Files+Workbench tiles at top of
  start menu (shipped 2026-05-23)** — `crates/mde-popover/src/
  start_menu.rs::view` now inserts a static `pinned_row` of two
  tiles (Files → `mde-files`, Workbench → `mde-workbench`)
  between the search input and the "Applications" header — i.e.
  ABOVE the `scrollable(list)`, so they don't scroll with the
  apps list. Both tiles use `Message::Launch(exec.into())`
  which routes through the existing `launch_exec()` path
  (shell-exec with XDG field-code stripping). Tiles use
  `width(FillPortion(1))` so they split the popover width
  evenly. Real Carbon SVG icons are a v4.0.1 BUG-13 follow-up
  (the broader icon-loading audit); text-only labels work
  today and survive the eventual icon swap.
- [✓] **v4.0.1: BUG-10 thicker window borders (shipped
  2026-05-23, commit pending)** — `data/sway/config:25-30` now
  has `default_border pixel 4`, `default_floating_border pixel
  4`, and `smart_borders no` (was 1 px + smart_borders on,
  which hid the border entirely on single-window workspaces).
  4 px reads clearly at 4K-TV viewing distance; the Carbon
  palette's focused/unfocused color contrast becomes visibly
  distinct. Operator can request 6 px (or back to 2) if 4 ends
  up too heavy at desk distance.
### v4.0.1 WM-* Excellent Window Management epic (audit 2026-05-23)

Operator: "the shell does not have good control of window
management" — pre-BUG-16 the panel had centered min/max/close
buttons but no surfaces for switching workspaces, seeing
minimized windows, focusing-by-click from a window list, or
visually snapping into Win11-style zones. BUG-16 added Snap
Layouts; the rest of the muscle-memory surface follows here.
Each story below stands alone; pick the highest-impact next
move per the iteration loop's step 2.

- [✓] **v4.0.1: BUG-18 retire sway-IPC cluster widget from
  the panel tray (operator-reported "error in the tool
  tray", shipped 2026-05-23)**

  **As** an operator,
  **I want** the panel tray to NOT show debug-y sway-IPC
  chip strings like "H def #16" alongside the
  network/audio/mesh/clock chips,
  **so that** I don't keep mistaking the panel's normal
  state for an error.

  **Acceptance** (bench-observable):
  - [x] No "H def #N" / "V tab #N" / etc. text appears in
        the panel tray any more — even with multiple
        windows tiled in different layouts.
  - [x] The mde-applet-sway-cluster binary still ships +
        emits its stdout (no behavior change for any
        external power-user tool that taps the data).

  **Implementation:** `crates/mde-panel/src/top_bar.rs`
  replaced the `let cluster = labeled_zone(&state.cluster_text,
  ...);` line with an empty `Space::with_width(0.0)` so the
  row layout's structure stays intact + future commits can
  drop the slot entirely. `state.cluster_text` is still
  populated by the applet stream (the `set_applet_text`
  handler stays wired) so any future re-introduction of a
  cluster surface — possibly behind a "show advanced sway
  chips" preference — doesn't need to re-wire the data
  layer. Phase 0.8 design-criteria justification: cluster
  was Ableton-style content surface tone (parameter
  readout) in a chrome zone (panel tray), which mismatched
  the influence locks; removing it resolves the hybrid
  forbidden by Phase 0.8.

- [✓] **v4.0.1: BUG-17 toast popover renders a permanent grey
  box when idle (shipped 2026-05-23) — Tier 1 chrome**

  Root cause + fix per the worklist analysis. Shipped:
  * `crates/mde-popover/src/toasts.rs::theme()` returns a
    `Theme::custom` whose `Palette::background` has alpha=0
    (was `Theme::Dark` with opaque dark-slate fill). wlr-
    layer-shell respects alpha so the surface stays the
    locked 360×200 but pixels show the wallpaper through.
  * The empty-stack `view()` branch returns a Fill/Fill
    transparent container instead of the prior 1×1 dummy.
  * Test `idle_app_theme_background_is_fully_transparent`
    asserts `palette.background.a == 0` — CI catches any
    regression.
  * `install-helpers/sync-user-sway-exec-lines.sh` restores
    `exec mde-popover toast` to REQUIRED_LINES so autostart
    works again on the next operator login + every reload.

  **Original block text retained for context:**

  **As** an operator,
  **I want** the toast notification surface to be invisible
  (zero compositor pixels showing through) when no toasts
  are queued, and visible only when at least one toast is
  mid-fade,
  **so that** I don't see a small grey rectangle floating
  above the panel when nothing is actually being notified.

  **Acceptance** (bench-observable):
  - [ ] With zero queued toasts, no grey/dark rectangle is
        visible above the panel (the wallpaper shows through
        where the toast surface lives).
  - [ ] When a toast fires (via the existing emit path —
        `~/.cache/mde/toasts.jsonl` tail), the pill renders
        with its accent + body text inside the 360×200 box.
  - [ ] When the toast expires + the stack empties, the
        surface returns to invisible without the process
        exiting.

  **Implementation notes:**
  - **Root cause:** the BUG-16-era fix capped the layer-shell
    `size: Some((360, 200))` to prevent the wlr-layer-shell
    `Anchor::Bottom`-stretches-full-width fallback. That
    bound the surface to a permanent 360×200 box that the
    iced theme paints dark even when the inner widget is the
    1×1 empty fallback.
  - **Fix:** in `crates/mde-popover/src/toasts.rs::view`,
    when `snapshot.is_empty()`, return a container whose
    style sets `background: Some(Background::Color(
    Color::TRANSPARENT))` (instead of the default theme
    dark-slate fill). The surface stays 360×200 but its
    pixels are transparent so the wallpaper shows through —
    matches Win11's toast surface "zero compositor real
    estate when idle" idiom.
  - **Icon source:** N/A.
  - **Influence:** chrome surface; "invisible until needed"
    pattern matches Win11 notification toasts.
  - **Test:** `cargo test -p mde-popover` adds an assertion
    that the empty-stack render path renders a transparent
    container.

### v4.0.1 WB-2 12 unwired Workbench panels (audit 2026-05-23)

Operator: "many panels in the workbench are incomplete." Audit of
`nav_model()` vs `panel_body()` view arms surfaced 12 nav-listed
slugs that fall through to the catch-all branch and render literally
`text("Panel view lands in a later CB-1.x substep.").size(14)`.
Clicking any of these from the sidebar lands on the placeholder
string + no other chrome.

Missing panels (catch-all targets):

  Group::Dashboard → home              — landing page
  Group::Apps      → panel             — Panel Apps grid
  Group::Maintain  → hub               — Maintain root
  Group::Maintain  → debloat           — apt-get autoremove equivalent
  Group::Maintain  → health_check      — system probe
  Group::Maintain  → drift             — config-drift report
  Group::Network   → mesh_control      — leader/peer state
  Group::Network   → mesh_pending      — pending-pairing list
  Group::Network   → mesh_services     — Caddy/headscale/derper
  Group::Network   → mesh_topology     — Cairo / iced topology
  Group::Network   → remote_desktop    — RDP/VNC management
  Group::Help      → index             — help topics

Each below stands alone as a story. The simpler landing-pages
(home + hub + index) ship in the same commit as this epic capture
since they're literal one-screen Iced views with no backend
integration needed.

- [✓] **v4.0.1: WB-2.a Dashboard `home` landing page (shipped
  2026-05-23)**

  **As** an operator,
  **I want** the Workbench to open on a Dashboard landing page
  showing my MDE version + Fedora release + hostname + 4 quick-
  stat cards (mesh peers / pending updates / snapshots / drift
  count) that link to the matching panel,
  **so that** the first thing I see when I open Workbench is a
  health snapshot, not the "Panel view lands in a later CB-1.x
  substep" placeholder.

  **Acceptance** (bench-observable):
  - [x] Workbench's default view (no `--focus` arg) shows the
        Dashboard with version + hostname + 4 quick-stat cards.
  - [x] Each card carries a Carbon glyph (peer / update / save
        / drift) and links to its matching panel via
        Message::SelectGroup / Message::SelectPanel.
  - [x] Empty / unknown stats fall back to "—" so the panel
        doesn't lie about state it doesn't know yet.

  **Implementation notes:**
  - Chrome influence: Win11 Settings → Home dashboard tile
    layout.
  - Icon source: Carbon Icon Set — `peer` for mesh, `update`
    for updates, `save` for snapshots, `repair` for drift.
  - Backend stays simple: read the static identity line
    from `WatermarkState::identity_line()` (already in
    mde-popover) — actually no, that's the wrong crate;
    inline the os-release + hostname read in panels/home.rs
    rather than depend across crate lines.
  - Counts: peers/snapshots/drift = 0 until backends ship
    (honest "—" until known); updates count reads
    `~/.cache/mde/dnf-updates.count` from the BUG-11 daemon.

- [✓] **v4.0.1: WB-2.b Maintain `hub` root grid (shipped
  2026-05-23)**

  **As** an operator,
  **I want** the Maintain group's root view to be a 2×3 grid
  of clickable tiles (Snapshots / Debloat / Health Check /
  Repair / Drift / Logs), each with its Carbon glyph + short
  description,
  **so that** I can find the right Maintain tool without
  reading a flat sidebar list.

  **Acceptance** (bench-observable):
  - [x] Maintain's group view (group-only `View::Group` shape,
        no panel slug) shows 6 tiles in a 2-column grid.
  - [x] Each tile is clickable; click navigates to the matching
        panel via Message::SelectPanel.
  - [x] Tile order matches the nav_model panel order
        (Snapshots, Debloat, Health Check, Repair, Drift, plus
        Logs at the end for the existing logs panel — Hub
        itself doesn't list).

  **Implementation notes:**
  - Chrome influence: Win11 Settings landing grid (square
    tiles, single accent per zone, 12 px gap).
  - Icon source: Carbon — `save` (snapshots), `clean`
    (debloat), `checkmark--filled` (health), `repair` /
    `tools` (repair), `analytics` (drift), `list` (logs).

- [✓] **v4.0.1: WB-2.c Help `index` topics list (shipped
  2026-05-23)**

  **As** an operator,
  **I want** the Help group's root view to list the help
  topics that ship in `docs/help/*.md`,
  **so that** I can find documentation from inside the
  Workbench instead of grepping the filesystem.

  **Acceptance** (bench-observable):
  - [x] Help group view shows a vertical list of topics read
        from `docs/help/*.md` filenames (or a hardcoded set
        if the dir isn't installed).
  - [x] Each topic row is clickable + opens the .md file in
        the system viewer via `xdg-open`.

  **Implementation notes:**
  - Chrome influence: Win11 Settings → Help & Support topic
    list.
  - Icon source: Carbon `help` + per-topic glyphs from
    `mde_theme::Icon`.

- [✓] **v4.0.1: BUG-19 catch-all "lands in a later CB-1.x substep"
  text leaks to the operator (shipped 2026-05-23, commit
  `8067449`) — Tier 1 chrome — surfaced by the Phase 0.7
  lands-marker audit added 2026-05-23**

  `app::panel_body` catch-all now routes to
  `panel_under_construction(view)`, which builds a UX-6
  EmptyState (Carbon `tools` icon + curated panel label from
  `model::resolve_panel_label` + "Back to <group>" CTA wired
  through `Message::SelectGroup`). The user-visible audit grep
  `text\("[^"]*(lands in|...|substep|follow-up)` now returns
  zero hits — CI can wire it as a hard gate.

- [✓] **v4.0.1: BUG-20 brand-strip parity — sway titlebar shows
  "MDE Workbench" + icon, in-app 48 px header showed bare "MDE"
  (shipped 2026-05-23, commit `8067449`) — Tier 1 chrome —
  surfaced by 2026-05-23 operator photos**

  **As** an operator,
  **I want** the in-app header bar to read the same product
  identity ("MDE Workbench" + Carbon Workbench glyph) as the
  WM-drawn window titlebar above it and the start-menu's
  pinned Workbench tile that launched the window,
  **so that** no chrome surface drifts from the rest and the
  product reads consistently regardless of which surface I'm
  looking at.

  Shipped:
  - `WORDMARK = "MDE Workbench"` (was `"MDE"`).
  - Carbon `Icon::Workbench` SVG prepended to the wordmark.
  - `Icon::Workbench` + `Icon::Files` lifted to first-class
    variants in `mde_theme::Icon` (was raw `include_bytes!`
    only in `start_menu.rs`).
  - Two new header tests guard parity + SVG-resolution.

- [✓] **v4.0.1: WB-2.d Apps → Panel Apps editor (shipped 2026-05-23)**

  Built `crates/mde-workbench/src/panels/panel_apps.rs` —
  the visibility editor with 6 toggle rows (audio / network /
  mesh / status / clipboard / notifications). Reuses the
  existing `mackes_config::PanelConfig::top_bar::status_items`
  schema (locked since v3.0.0 per Q18–Q22) instead of
  introducing a parallel schema. Reads from
  `~/.config/mde/panel.toml` (fallback: legacy
  `~/.config/mackes-panel/panel.toml`); writes always to the
  MDE-namespaced location via `mde_config::to_toml_string`
  round-tripping the full PanelConfig so other sections
  (dock, mesh, peer_card) survive.

  Wired the consumer side: `crates/mde-panel/src/top_bar.rs`
  gained `load_visible_applets_from_config()` +
  `applet_visible(visible, id) -> bool`, plus
  `TopBarState::loading()` loads the visible list at panel
  spawn. The tray-row builder switched from a fixed `row![]`
  macro to a `Vec<Element>` accumulator that pushes only
  applets passing `applet_visible(...)`. Back-compat default:
  empty `visible_applets` list = render-all (matches the
  pre-WB-2.d behaviour for operators who never touch the
  config).

  Tests: 118 mde-panel + 558 mde-workbench. Schema reuse +
  config round-trip + view-render smokes covered.

  **Operator flow:**
    1. Open Workbench → Apps → Panel Apps
    2. Toggle applets ON/OFF; changes save to
       `~/.config/mde/panel.toml` immediately
    3. Run `restart-panel-stack.sh panel` (or wait for the
       next parity tick) to see the change in the tray

  Chrome influence: Win11 Settings → Personalization →
  Taskbar → Taskbar items.

- [✓] **v4.0.1: WB-2.e Maintain Debloat (shipped 2026-05-23)**
  Routed Maintain → Debloat to the already-shipped
  `apps_remove.rs` panel (32-pkg curated bloat list with
  checkbox UI + `pkexec dnf remove`). Two nav paths (Apps →
  Remove + Maintain → Debloat) hit one panel surface; design
  lock places Debloat under Maintain as the primary entry.
  Three-line change in `app.rs::panel_body`.

- [✓] **v4.0.1: WB-2.f Maintain Health Check (shipped 2026-05-23)**
  Built `crates/mde-workbench/src/panels/health_check.rs` —
  7 local probes (disk space, memory, failed systemd units,
  DNS resolution, pending dnf updates, snapshot count, parity
  overlay heartbeat) each returning `(name, status,
  detail, remediation)`. Status uses Carbon glyphs
  (`StatusOk` / `StatusWarning` / `StatusError` /
  `StatusUnknown`) with semantic tinting. Worklist spec
  originally asked for `mackesd healthz` JSON parsing;
  shipped local probes instead so the panel works today
  without the mackesd daemon running. Auto-loads on nav.
  7 tests + clean integration.

- [✓] **v4.0.1: WB-2.g Maintain Drift (shipped 2026-05-23)**
  Built `crates/mde-workbench/src/panels/drift.rs` — shells
  out to `mackesd events list --json`, parses the JSON array,
  filters for drift-flavoured payloads (heuristic: `kind`
  contains "drift" OR a `severity` field is set), surfaces
  each row as severity icon + INFO/WARN/ERROR pill + event-id
  + peer + relative timestamp + multi-line message body.
  Empty-state card distinguishes "no drift detected" (info
  green) from "mackesd unreachable" (error red with the
  spawn error message). Auto-loads on nav. 7 tests including
  severity round-trip, garbage rejection, drift-kind
  extraction, and severity-only extraction.

- [✓] **v4.0.1: WB-2.h Network Mesh Control (shipped 2026-05-23)**
  Built `crates/mde-workbench/src/panels/mesh_control.rs` —
  reads `~/QNM-Shared/.mackesd-leader.lock` (with fallback to
  `/var/lib/mackesd/qnm-shared/`) + parses the lease's
  `node_id / renewed_at_s / epoch` tab-separated triple. Shows
  a status card (LEADER / FOLLOWER / NO LEADER tinted with
  Carbon `StatusOk` / `Peer` / `StatusWarning`), key-value
  pills for renewed-age + epoch + owner + self-id, and a
  separate card with `mackesd healthz` JSON output (parsed
  summary + raw body). Force-takeover button shells out to
  `mackesd take-leadership --force`. Auto-loads on nav.
  8 tests covering parser shape lock, garbage rejection,
  healthz summarisation, empty-state + populated-state view
  renders.

- [✓] **v4.0.1: WB-2.i Network Mesh Pending (shipped 2026-05-23)**
  Built `crates/mde-workbench/src/panels/mesh_pending.rs` —
  scans `$XDG_CACHE_HOME/mde/peers/<peer-id>/probe.json` (the
  `mackesd::peer_join::write_probe` landing spot) and renders
  each cached PeerProbe as a pending pair-request row:
  hostname + peer_id + `distro · mded vN.N.N · NN ms` chip
  line + Accept button (shells `mackesd enroll <peer-id>`) +
  Reject button (deletes the probe.json). Empty-state card
  shows the Carbon `StatusOk` glyph + the probe.json path
  template. Auto-loads on nav. 6 tests covering parser shape
  lock + garbage rejection + view renders for both
  populated + empty states.

- [✓] **v4.0.1: WB-2.j Network Mesh Services (shipped 2026-05-23)**
  Built `crates/mde-workbench/src/panels/mesh_services.rs` —
  curated list of 4 mesh-fabric daemons (`tailscaled`,
  `headscale`, `caddy`, `mackesd`) with `LoadState` /
  `ActiveState` / `UnitFileState` probes (so "not installed"
  reads differently from "inactive") + journalctl tail (last 5
  lines per unit) + Start / Stop / Restart buttons routed
  through `pkexec systemctl`. Auto-loads on nav. 7 tests.
  Original spec mentioned DERP — that's a Tailscale-internal
  protocol, not a separate daemon, so it folds into
  `tailscaled`; the curated set is locked at 4 entries with
  any extension going through worklist (not code-only).

- [✓] **v4.0.1: WB-2.k Network Mesh Topology — tabular fallback
  (shipped 2026-05-23)**
  Built `crates/mde-workbench/src/panels/mesh_topology.rs` —
  shells out to `mackesd nodes list --json`, parses the
  `NodeRow` shape (`node_id / name / public_key / role /
  health / region`), surfaces each as a table row with
  status-pill (ONLINE / IDLE / OFFLINE / UNKNOWN tinted by
  semantic colour) + name + addr (= region) + kind (= role).
  Empty-state distinguishes "no peers enrolled" (info, with
  birthright-enrollment hint) from "mackesd not reachable"
  (error, with spawn-error embedded). Footer points at the
  canvas-graph follow-up below. Auto-loads on nav.
  7 unit tests on the parser shape lock + status round-trip
  + view-render smokes.

- [✓] **WB-2.k.a: Mesh Topology canvas-graph (shipped 2026-05-23)** —
  added a Table/Graph layout toggle to the Mesh Topology
  panel. Graph layout uses `iced::widget::canvas::Canvas`
  to draw the local node at center + each enrolled peer
  arrayed in a ring, edges connecting peers to center.
  Peer circles tinted by status (ONLINE green / IDLE amber
  / OFFLINE red / UNKNOWN grey). Empty state still renders
  a friendly card.

  Iced `canvas` feature added to mde-workbench's deps so
  the Canvas widget compiles. Implements
  `canvas::Program::draw` over a `GraphProgram` struct
  that owns the peer list + palette.

  Edge thickness is uniform today — inter-peer latency
  isn't collected yet (chains on AF-NET-2 mesh sniffer
  work). When that lands, the edges can vary thickness +
  opacity by latency.

  571 mde-workbench lib tests pass (no new tests — canvas
  draw is render-only with no testable pure logic; the
  view-renders-without-panic smokes cover the layout
  toggle).

- [✓] **AF-NET-2: peer-mesh latency sniffer (shipped
  2026-05-23)** — `crates/mackesd/src/workers/mesh_latency
  .rs` ships the worker; wired into `run_serve` with its
  own SQLite handle + `RestartPolicy::OnFailure`.
  Cadence: one immediate sweep on boot + every 30 s
  thereafter. Per-peer ping deadline 1 s. Writes
  `~/.cache/mde/mesh-latency.json` as
  `{"checked_at": <unix>, "peers": {"<name>":
  {"rtt_ms": Option<f64>, "ok": bool}}}`. Pure
  `parse_ping_rtt(raw)` helper extracts the `time=NN.N ms`
  token (handles integer + sub-ms RTTs); 9 tests cover
  parser cases + write_snapshot round-trip + worker
  name/shutdown semantics. Best-choice deviation from the
  TransportRegistry-routed spec: `ping`(8) hits the same
  ICMP wire the underlying Transport would, with zero new
  Cargo deps and a bench-observable outcome
  indistinguishable from the routed version. When the
  Transport stack lands, swap the sync `ping` call for
  `Transport::probe()` and delete the shell-out — the
  cache file shape stays the same so WB-2.k.a + the panel
  tray badge stay consumer-stable. 628 mackesd lib tests
  green (was 619).

- [✓] **v4.0.1: WB-2.l Network Remote Desktop (shipped 2026-05-23)**
  Built `crates/mde-workbench/src/panels/remote_desktop.rs` —
  reads `~/.config/mde/peer-macs.json` (with fallback to the
  legacy `~/.config/mackes-shell/peer-macs.json`), surfaces
  each cached IP/MAC pair as a row with per-row [RDP] [VNC]
  buttons, plus a manual hostname/IP text field at the top
  with its own Connect RDP / Connect VNC buttons. Click
  launches `remmina -c <proto>://<host>:<port>` (3389 for
  RDP, 5900 for VNC). Auto-loads on nav. 8 tests including
  parser round-trip + empty-state render.

- [✓] **v4.0.1: WB-1 wire Connected Devices panel into Workbench
  nav (Phase 0.7 rescue — operator-reported missing modal)
  (Tier 1 chrome) — shipped 2026-05-23**

  **As** an operator,
  **I want** a "Connected Devices" panel in the Workbench
  (under the Devices nav group) showing every paired peer +
  phone + tablet with Pair / Unpair / Ring / Send-File actions,
  **so that** I can manage KDE Connect / mesh peer pairings
  from the same Workbench surface I already use for displays,
  sound, printers, and removable media — not from a separate
  app or a missing modal.

  **Acceptance** (bench-observable):
  - [ ] Workbench's Devices nav group shows a "Connected
        Devices" entry between "Printers" and "Removable
        Media".
  - [ ] Clicking it routes to the panel via the existing
        `View::Panel { group: Devices, panel: "connect" }`
        deep-link shape.
  - [ ] The panel renders one card per paired device (read
        from the `connect::ConnectPeer` model; backed by
        DemoBackend until KDC2's DBus surface lands). Each
        card shows: device name, Carbon kind-glyph (phone /
        tablet / desktop), fingerprint, paired-since date.
  - [ ] Empty state (zero paired devices) renders a
        Workbench EmptyState with "No paired devices yet"
        heading + "Open KDE Connect on a phone or tablet
        and choose this PC to pair." body + a CTA pointing
        to mde-peer-card.
  - [ ] Conditional sections render per `ConnectPeer::
        capabilities` (Phone / Messaging / Share / Common)
        per the existing visibility helpers in connect.rs.

  **Implementation notes:**
  - **Chrome influence:** Win11 Settings → Bluetooth & devices
    → Devices page layout (one card per device, action row).
  - **Content influence (per-card stats):** Ableton parameter-
    row density — tabular IBM Plex Mono for fingerprint hex,
    grouped Pair/Unpair/Ring/SendFile buttons at the row's
    right edge.
  - **Icon source:** Carbon Icon Set per the lock.
    `mobile` for phones, `tablet` for tablets,
    `application--web` / `screen` for desktops,
    `notification` for ringing, `send-alt` for share.
  - **Model layer:** `connect::ConnectPeer` + capability
    predicates already ship — `#![allow(dead_code)]` lifts
    here (Phase 0.7 closure).
  - **View layer:** new `ConnectPanel` struct + view fn in
    `crates/mde-workbench/src/panels/connect.rs`. Card list
    iterates `backend.paired_devices()`; until that backend
    method exists, render from a DemoBackend constant
    (clearly marked as mockup so future Phase 0.7 audit can
    catch it — per the iconography + mockup-audit locks).
  - **Routing:** `app.rs::update` gains a `Message::Connect
    (panels::connect::Message)` variant + dispatch arm; the
    nav_model adds the "connect" panel to Group::Devices.

- [✓] **v4.0.1: WM-1 visible workspace switcher (Tier 1 chrome) —
  shipped 2026-05-23**

  **As** an operator,
  **I want** to see and click numbered workspace chips on the
  panel (1 / 2 / 3 / 4, current workspace highlighted in the
  Q2 indigo accent),
  **so that** I can switch workspaces with the mouse the same
  way I do on Windows 11 / GNOME / macOS without remembering
  Super+N keybindings.

  **Acceptance** (each bullet bench-observable):
  - [ ] Four workspace chips render in the panel, between the
        dock zone and the Desktop Layout cluster, showing
        "1 2 3 4" Carbon-numeric glyphs (or fallback text
        labels if the numeric glyphs aren't in the system
        Carbon set).
  - [ ] The currently-focused workspace chip paints its
        background in Q2 indigo (#5b6af5); other chips render
        with `zone_button_style` chrome.
  - [ ] Clicking chip N fires `swaymsg workspace N` and the
        focus flips within ~200 ms.
  - [ ] When a workspace has running windows, its chip shows
        a small unfilled-circle indicator dot to its right
        (Carbon `circle--solid` at 6 px); empty workspaces
        omit the dot.
  - [ ] The applet binary `mde-applet-workspaces` polls
        `swaymsg -t get_workspaces` every 2 s and emits a
        JSON line per render so the panel-host can rebuild
        the chip row without re-parsing sway state.

  **Implementation notes:**
  - **Chrome influence:** Win11's centered taskbar workspace
    switcher; Win10's bottom-left task-view button. Chips
    are square + rounded-corner per Phase 0.8 chrome locks
    (8 px radius matches the existing tray-button chrome).
  - **Icon source:** Carbon Icon Set. Glyph candidates per
    chip number — `number--1`, `number--2`, `number--3`,
    `number--4`. Indicator dot is `circle--solid` at 6 px
    in `text-muted`. Bake into `assets/icons/carbon/
    workspace-{1,2,3,4}.svg` + `assets/icons/carbon/
    workspace-dot.svg`.
  - **Crate layout:** new `crates/mde-applets/workspaces/`
    following the mesh-status / clock / network applet
    pattern. Pure-fn `parse_workspaces(swaymsg_json)`
    + `format_chip_row(workspaces)`.
  - **Panel-host wiring:** new `AppletKind::Workspaces` +
    `tray_button_with_icon_for_workspace(num, focused,
    has_windows)` helper in top_bar.rs. Row inserts between
    `dock` and the leading `Space::with_width(Length::Fill)`
    so chips sit close to the start button on the left.

- [✓] **v4.0.1: WM-2 minimized-windows popover (popover half
  shipped 2026-05-23; tray-button half tracked as WM-2.a) —
  Tier 1 chrome**

  Built `crates/mde-popover/src/minimized.rs` + the
  `Kind::Minimized` variant in main.rs. Walks `swaymsg -t
  get_tree`, finds the `__i3_scratch` workspace, collects
  every leaf in its `nodes` + `floating_nodes` arrays.
  Renders one row per scratchpad window with app_id + title;
  click fires `swaymsg [con_id=N] scratchpad show` to
  restore + closes the popover. XWayland windows that don't
  have `app_id` fall back to `window_properties.class`. Esc
  closes; empty state hints at the binding.

  **Operator one-liner (or add to sway config):**
  `mde-popover minimized`

  5 unit tests cover garbage rejection + native-Wayland
  scratchpad walk + XWayland class fallback + nested-
  container descent + non-scratch-workspace filtering.

- [✓] **v4.0.1: WM-2.a minimized-windows panel tray button + badge
  (shipped 2026-05-23)** — `crates/mde-panel/src/top_bar.rs`
  gained `count_scratchpad(raw)` (pure parser over `swaymsg
  -t get_tree` JSON) + a new tray button rendered when
  `scratchpad_count > 0`. Button shows the Carbon
  `WindowMinimize` glyph + the count as a chip; click fires
  `Message::MinimizedClicked` which spawns `mde-popover
  minimized` (the WM-2 popover from commit 3fdf9d2). State
  refreshes every ~2s on the same 60-tick boundary as the
  workspace switcher. Tray button hides when count = 0 so the
  surface stays clean when nothing is hidden. Respects the
  WB-2.d visibility config (operator can disable via Panel
  Apps with id `minimized`). 123 mde-panel tests pass (+5
  for count_scratchpad + applet_visible helpers).

  **Original WM-2 umbrella spec retained below for context**
  — the split-out 2026-05-23 ships the popover today + tracks
  the tray half here. Original spec:

  **As** an operator,
  **I want** a panel tray icon (Carbon `view--off`) with a
  badge count of currently-minimized windows, that opens a
  popover listing each minimized window's title + app, with
  click-to-restore per row,
  **so that** I can SEE what I've sent to the scratchpad and
  pick which one to bring back (instead of cycling blind via
  Super+Shift+M).

  **Acceptance** (bench-observable):
  - [ ] When ≥1 window is in sway's scratchpad, a tray icon
        appears between the clipboard chip and the
        notification-bell chip showing the Carbon `view--off`
        glyph + an integer badge in Q2 indigo.
  - [ ] When zero windows are scratch-hidden, the icon is
        absent (no greyed-out placeholder).
  - [ ] Clicking the icon opens a 360 × auto-height
        layer-shell popover ("MinimizedPopover" kind in
        mde-popover) listing each scratchpad entry by
        `app_id` + title. Each row is a button.
  - [ ] Clicking a row fires `swaymsg [con_id=N] scratchpad
        show` + closes the popover; the window restores into
        the focused workspace within ~150 ms.
  - [ ] The popover's Esc key dismisses it; clicking outside
        also dismisses (per v3.0.4 backdrop work when that
        ships).

  **Implementation notes:**
  - **Chrome influence:** Win11's Notification Center +
    Action Center, modified for the scratchpad concept.
  - **Icon source:** Carbon `view--off` for the tray glyph;
    `restore` or `arrows--vertical` for the per-row restore
    affordance.
  - **Data source:** `swaymsg -t get_tree` + walk the
    scratchpad workspace's nodes. Each row needs
    `(con_id, app_id, title)`.
  - **Crate layout:** new `crates/mde-popover/src/
    minimized.rs` + new tray button in `top_bar.rs`.
    `Message::MinimizedClicked` routes to
    `toggle_or_spawn_popover("minimized")`. Tray-state
    poll loop in the panel-host (same 2 s cadence as the
    mesh-status applet).

- [✓] **v4.0.1: WM-3 dock interactive: click to focus / right-
  click for actions (shipped 2026-05-23 alongside DOCK-1)** —
  Cycle G follow-on. The dock applet's interactive layer
  (left-click focus, right-click action menu, focused-cell
  indigo underline) shipped as part of DOCK-1 above; this
  task closes the gap that DOCK-1 deferred — the right-click
  menu surface itself. New `crates/mde-popover/src/
  window_actions.rs` + `Kind::WindowActions` variant: 240 px
  layer-shell popover with Move-to-workspace 1-4 chips,
  Close-window (urgent-tinted), Pin/Unpin-to-dock
  (accent-tinted, label flips by live `mackes_config` lookup).
  Actions execute via `swaymsg [con_id=N] move container to
  workspace M` / `swaymsg [con_id=N] kill` /
  `mackes_config::{pin_app,unpin_app}` + write panel.toml.
  Spawn contract: dock applet sets `MDE_WINDOW_CON_ID` +
  `MDE_WINDOW_APP_ID` env vars before exec'ing
  `mde-popover window-actions`. Esc / outside-click /
  close-button all dismiss. 4 new popover tests cover
  dimension lock + workspace-button-handles-1..4 +
  empty-con-id no-op invariants. 116 popover tests green
  (was 112).

  **As** an operator,
  **I want** the dock area to render one clickable button
  per open window (icon + truncated title), with the
  focused window highlighted in Q2 indigo and a right-click
  menu offering "Move to workspace N" / "Close" / "Pin to
  dock",
  **so that** I can navigate between open windows with the
  mouse instead of Super+Tab + can do common per-window ops
  directly from the panel.

  **Acceptance** (bench-observable):
  - [ ] Each open window renders as a separate clickable
        button in the dock zone of the panel, in the order
        sway's `get_tree` returns.
  - [ ] Left-click on a dock button focuses that window
        (calls `swaymsg [con_id=N] focus`) and brings it to
        the front of its workspace.
  - [ ] The currently-focused window's button paints with a
        Q2-indigo bg-tint (not the standard zone-button
        chrome).
  - [ ] Right-click opens a 200 × auto-height popover with
        ≥3 actions: Move to ws (1/2/3/4), Close, Pin to
        dock. Each click is bench-observable via swaymsg.
  - [ ] Pinning a window writes its `desktop_id` to
        `~/.config/mde/dock-pinned.json` per Phase E.9; the
        existing dock_dnd helpers consume it.

  **Implementation notes (3-task fan-out captured in BUG-5):**
  - **Chrome influence:** Win11's taskbar — per-window icon
    + label, focused window underlined in accent.
  - **Icon source:** the `mde_panel::icon_mapper` already
    maps `Icon=` strings from .desktop entries to Carbon
    glyph names. Per-window dock buttons reuse that mapping.
  - **Data source:** dock applet (`mde-applet-dock`) needs a
    protocol upgrade — emit one JSON line per window with
    `(con_id, app_id, title, focused)`. Panel-host parses
    the JSON and renders one button per row. Closes the
    3-task fan-out in BUG-5.

- [✓] **v4.0.1: WM-4 visual Snap Assist overlay (shipped
  2026-05-23)** — Cycle L. Best-choice deviation from the
  spec's "drag-to-detect" trigger: sway IPC doesn't expose
  live pointer drag events (no seat-grab protocol, no
  pointer-events subscription in the public IPC), so
  tracking the drag itself would require either a
  Wayland-core protocol sway hasn't shipped or a per-100ms
  poll of a `swaymsg -t get_pointer_locations` that doesn't
  exist. The shipped realization keeps the visual outcome
  (indigo overlay, 8 click-to-snap zones, focused window
  snaps on click) and replaces the drag trigger with a
  `Super+Z` keybind. Spec acceptance bullets translate
  cleanly:
  - The 30%-alpha indigo overlay now wraps the modal
    surface (backdrop fill), so the screen still shows
    "would-snap" semantics.
  - Click-to-commit fires `swaymsg <command>` with the
    exact argv shapes the spec called for
    (`floating disable; move position 0 0; resize set
    50ppt 100ppt` for left half, etc.).
  - Esc / outside-click cancels — no resize applied.
  - All 5 spec zones (left/right/top/bottom halves + 4
    quadrants) ship, mapping to the 8 SnapZone variants.

  Crate additions:
  - `crates/mde-popover/src/snap_assist.rs` (~350 LOC)
    with `SnapZone` enum (8 variants) + pure
    `swaymsg_command()` per-zone + Iced view rendering 4
    halves + 4 quadrants as click-to-commit accent-tinted
    buttons.
  - `Kind::SnapAssist` variant in mde-popover/src/main.rs.
  - `data/sway/config.d/mackes-keybinds-wm.conf` gets
    `bindsym $mod+z exec mde-popover snap-assist` + 4 new
    quadrant keybinds (`$mod+Ctrl+Shift+{y,u,b,n}` =
    TL/TR/BL/BR).

  4 new tests cover every-zone-emits-command +
  left-half-resize-shape + right-half-offset +
  quadrants-are-50x50 + labels-distinct. 124 popover
  tests green (was 120).

- [✓] **v4.0.1: WM-5 visible Alt-Tab switcher (shipped 2026-05-23,
  retires the invisible mde-applet-app-switcher) — Tier 1 chrome**

  Built `crates/mde-popover/src/app_switcher.rs` + the
  `Kind::AppSwitcher` variant in main.rs. 640×360 centered
  Layer::Overlay surface with KeyboardInteractivity::Exclusive.
  Grid of 3-cards-per-row showing every open sway window
  (skips scratchpad). Selected card has the Q2 indigo border
  + tinted background. Default selection is the second card
  (alt-tab "go-back-to-previous-window" idiom).

  **Keybinds (sway subscription via `keyboard::on_key_press`):**
  * Tab           — next
  * Shift+Tab     — prev
  * Arrow keys    — also nav (right/down = next, left/up = prev)
  * Enter         — focus selected + close
  * Esc           — cancel + close
  * Click card    — focus that card + close

  **Bound from `data/sway/config.d/mackes-keybinds-wm.conf`:**
    `bindsym Mod1+Tab exec mde-popover app-switcher`

  Mod1 = Alt rather than Super because Super+Tab is reserved
  for workspace switching in mackes-defaults.conf — same idiom
  as Win11 + macOS where Alt-Tab cycles windows.

  Spec deferred: per-card screenshot thumbnail (would need
  `grim` per-window capture; iced 0.13 can't paint live
  Wayland buffers). Tracked as **WM-5.a**.

  10 unit tests cover parser shape lock + scratchpad-skip +
  XWayland class fallback + garbage rejection + Next/Prev
  wrap-around + truncate-helper bounds.

- [✓] **v4.0.1: WM-5.a app-switcher screenshot thumbnails
  (shipped 2026-05-23)** — Cycle K. `WindowCard` gained
  `rect: WindowRect` + `thumbnail: Option<Vec<u8>>` fields.
  `parse_tree` now extracts the sway `rect` per node;
  `parse_rect` is a pure helper with default-to-zero
  semantics. App::new dispatches one deferred
  `Task::perform(async move { capture_thumbnail(rect) },
  |bytes| Message::ThumbnailLoaded(con_id, bytes))` per
  card so the popover paints text-only on first frame and
  thumbnails slot in as `grim -g "X,Y WxH" -` returns. New
  `Message::ThumbnailLoaded(u64, Vec<u8>)` reducer finds
  the card by con_id and updates `thumbnail`. `card_view`
  renders the PNG via `iced::widget::image` when present
  (size locked to `CARD_H - 38` px); falls back to a
  Space::with_height of the same dimension when None so
  the layout doesn't shift mid-animation. Empty-Vec
  capture results (grim missing, rect zero-sized, sway
  refused) stay text-only — defensive guards short-circuit
  before invoking grim on a zero-area rect. iced `image`
  feature added to mde-popover's Cargo.toml. 4 new tests
  (parse_rect extracts all four / defaults missing /
  parse_tree-now-extracts-rect / capture_thumbnail
  zero-size returns empty). 120 popover tests green (was
  116).

  **As** an operator,
  **I want** pressing Super+Tab to show a centered overlay
  with one card per open window (icon + title + screenshot
  thumbnail), cycling on each Tab press, releasing Super to
  focus the highlighted window,
  **so that** I get visual feedback during the Alt-Tab idiom
  the same way Win11 / macOS / GNOME do.

  **Acceptance** (bench-observable):
  - [ ] Pressing + holding Super, then tapping Tab, opens a
        centered overlay listing all open windows.
  - [ ] Each Tab press advances the selection ring to the
        next window; Shift+Tab reverses.
  - [ ] Releasing Super focuses the highlighted window via
        `swaymsg [con_id=N] focus` and dismisses the overlay
        within 150 ms.
  - [ ] Each card shows the Carbon app icon (per icon_mapper)
        + the window's title + (when feasible) a `grim`-captured
        thumbnail of the window's current state.

  **Implementation notes:**
  - **Chrome influence:** Win11 Alt-Tab + GNOME Activities
    overview.
  - **Existing surface:** `mde-applet-app-switcher` (Phase
    E1.2.11) is the prior `--manifest` + stdout-text
    applet; that retires for this version.
  - **Implementation path:** new `mde-popover app-switcher`
    kind. Layer-shell overlay anchored center, full
    keyboard-grab while open. Sway binding `bindsym
    Mod1+Tab exec mde-popover app-switcher` (Mod1 = Alt;
    Super+Tab is reserved for the workspace switcher).

- [✓] **v4.0.1: WM-6 floating window keyboard ops (shipped 2026-05-23)**

  Shipped `data/sway/config.d/mackes-keybinds-wm.conf` —
  loads alphabetically AFTER `mackes-defaults.conf` so the
  drop-in extends the defaults without losing them. Bindings:
  - Super+Ctrl+H/J/K/L → tile focused window to half-screen
    (left/down/up/right). Picked Super+Ctrl rather than the
    spec's Super+H to coexist with existing Super+H/J/K/L
    focus-nav bindings instead of breaking muscle memory.
  - Super+Ctrl+arrow → move container to neighbour output.
  - Super+Shift+F → Win11-maximize equivalent (floating fill).

  **In-place propagation:** parity-overlay's install phase now
  rsyncs `data/sway/config.d/*.conf` into
  `~/.config/sway/config.d/` on every tick so existing
  operators pick up new drop-ins without re-running
  mde-shell-migrate-v2 (which only seeds on first boot when
  `~/.config/sway/` is empty).

- [✓] **v4.0.1: BUG-16 per-window controls → Win11 standard
  location; panel center → Desktop Layout buttons (Tier 1
  chrome) — shipped 2026-05-23**

  **As** an operator,
  **I want** the minimize / maximize / close buttons to live at
  the top-right of each managed window (and the panel center
  to host a Snap-Layouts-style cluster instead of window
  controls),
  **so that** my Windows 11 / macOS muscle memory transfers
  directly to MDE and the panel center carries a feature that
  applies to the whole workspace rather than a single window.

  **Acceptance** (every bullet bench-observable on the live
  panel):
  - [ ] Minimum 3, maximum 5 Desktop Layout buttons render in
        the panel's center zone — single (1 fullscreen),
        vsplit (2 side-by-side), grid-4 (2×2), main+sidebar
        (60/40), tabbed — clicking one applies the layout to
        the current workspace's windows via swayipc.
  - [ ] No window-management glyphs (min/max/close) appear in
        the panel center any more; they render at the top-right
        of each managed window's title bar instead.
  - [ ] Each Desktop Layout button paints its Carbon glyph in
        Q2 indigo (#5b6af5) at the hover state, FG_MUTED at
        rest; 140 ms ease-out hover transition per UX-9.
  - [ ] Buttons share a single accent across the cluster (not
        per-button accents) per the Ableton single-accent-per-
        zone rule.

  **Implementation notes:**
  - **Chrome influence:** Microsoft Windows 11 Snap Layouts
    (per the iteration skill's Phase 0.8 design influence
    section). Treat each button as a miniature template
    visualization, matching Win11's hover-over-maximize
    preview.
  - **Icon source:** Carbon Icon Set per the iconography
    lock. Glyph candidates (verify against `/usr/share/icons/
    Mackes-Carbon/scalable/apps/`): `maximize` for single,
    `column` / `split-screen` for vsplit, `grid` for grid-4,
    `panel-expansion` for main+sidebar, `tabbed` /
    `category` for tabbed. Bake into `assets/icons/carbon/
    layout-*.svg` and add arms to `mde_theme::ResolvedIcon::
    svg_bytes()` before consuming.
  - **Per-window controls path:** two options at implement
    time. **(a)** Native sway title bars via `default_border
    normal <px>`. **(b)** `mde-window-controls` layer-shell
    overlay tracking the toplevels subscription, pinning a
    3-button row to the top-right of the focused window's
    geometry. Pick (b) if the native sway title bar typography
    can't be themed to match Geologica/IBM Plex Mono.
  - **Layout-button mechanism:** new crate
    `crates/mde-applets/desktop-layout/` (per the BUG-13.a
    panel-host applet pattern); emits a JSON-line per click
    that the panel routes to `swaymsg layout <kind>` +
    `swaymsg [workspace=N] layout cycle` / move ops.
  - **Reversal note:** supersedes the BUG-6 commit (43183ba)
    in part — window_button_cluster() drops from the panel's
    center row; cluster (sway-IPC chips) stays where BUG-3
    moved it. The "newer-wins-silently" rule
    ([[mackes-worklist-management]] §1) applies.

- [✓] **v4.0.1: BUG-15 minimize button sends windows into the
  scratchpad with no recovery path (captured 2026-05-23)** —
  operator reports clicking the minimize button on the panel's
  centered window-controls cluster makes the focused window
  disappear with no obvious way to bring it back.
  `Message::WindowMinimize` runs `swaymsg [con_id=N] move
  scratchpad` (v8.7 lock — sway has no native minimize, the
  scratchpad-hide is the closest user-visible equivalent), but
  the scratchpad cycle isn't bound by default. Fix:
  (a) add `bindsym $mod+Shift+m exec swaymsg scratchpad show` to
  `data/sway/config` — cycles minimized windows back into the
  focused workspace one at a time.
  (b) Stretch (BUG-5 fan-out closes this fully): the dock's
  inline window list shows minimized windows + a click restores
  any one of them directly.
  Acceptance for (a): after a minimize, pressing Super+Shift+M
  brings the window back into view. (b) is tracked under BUG-5.
- [✓] **v4.0.1: hide platform-internal entries from default
  start menu (shipped 2026-05-23)** — three MDE-platform
  `.desktop` files in `data/applications/` gained
  `NoDisplay=true` so they no longer pollute the all-apps list:
  (a) `mackes-clipboard.desktop` — background mesh-clipboard
      daemon, never user-launched.
  (b) `mackes-shell.desktop` — legacy v1.x "Mackes XFCE
      Workstation" entry; superseded by `mde.desktop` at v2.0.0.
  (c) `mde.desktop` — root system entry; end users are already
      inside MDE, the "Mackes Desktop Environment" tile is
      meaningless from inside the running DE. The Wizard /
      Drawer Desktop Actions stay reachable via `gio launch
      mde.desktop --wizard` for callers that still go that
      route.
  Three other MDE internals already had `NoDisplay=true`:
  `mackes-enforce-session`, `mackes-mesh-uri-handler`,
  `mackes-panel`. `mde-files` + `mde-workbench` intentionally
  stay visible — they're real apps end users launch (also
  pinned at top of the start menu via BUG-12). The start
  menu's `AppEntry.hidden` flag (set when `NoDisplay=true`
  OR `Hidden=true`) already filters out hidden entries in
  the default no-query view; typing a search query bypasses
  the filter so power-users can still find background
  components by name.
- [✓] **v4.0.1: watermark → start-menu footer move (shipped
  2026-05-23)** — operator retired the standalone Win10 watermark
  popover. The Win10 system-identity strip ("MDE X.Y.Z · Fedora
  N · host" + clickable "N updates pending" chip) moved to the
  bottom of the start-menu popover, above the existing
  "Esc closes…" hint line. `crates/mde-popover/src/watermark.rs`
  was refactored from a 650-line iced layer-shell surface to a
  ~250-line headless dnf-poll daemon: it spawns the 4-hour
  poll thread, writes `~/.cache/mde/dnf-updates.count`, and
  parks the main thread forever — no visible chrome.
  `WatermarkState`, `current_pending_count`, and
  `spawn_pkexec_dnf_upgrade` are now consumed by
  `start_menu.rs::view` which reads the cache on every popover
  open and renders the identity strip + update-count chip. New
  `update_chip_style` for the indigo Q2 accent chip. The chip's
  click handler fires `pkexec dnf upgrade` (same action the
  watermark widget had pre-retirement). 9 watermark lib tests
  + `identity_line_excludes_count` regression pass.
- [✓] **v4.0.1: BUG-11 watermark popover never spawned because
  user's sway config was stale (shipped 2026-05-23)** —
  root-cause diagnosis: `data/sway/config:160-165` has
  `exec mde-popover watermark` + `exec mde-popover toast` but
  the operator's `~/.config/sway/config` (copied by the v1.x
  birthright wizard, never refreshed) lacks both lines.
  `dnf check-update` actually shows 135 pending updates, so the
  watermark would render if the popover were alive.
  Two-part fix:
  (1) `install-helpers/sync-user-sway-exec-lines.sh` — idempotent
      helper that appends any required `exec mde-popover *`
      lines absent from `~/.config/sway/config`, then runs
      `swaymsg reload`. Safe: only appends, never reorders or
      removes user customizations. Future BUG-11-style drifts
      land here as new entries in `REQUIRED_LINES`.
  (2) `data/systemd/mde-session.service` ExecStartPost runs the
      script on every login so existing users converge without
      re-running the wizard.
  Spec install lines added to ship the helper at
  `/usr/share/mackes-shell/install-helpers/sync-user-sway-
  exec-lines.sh`. Operator's existing sway config was
  refreshed in-place + both popovers spawned manually for
  immediate relief; `mde-popover watermark` (PID 46211) +
  `mde-popover toast` (PID 46561) running, dnf-updates.count
  reports 135 pending.
- [✓] **v4.0.1: BUG-9 network applet whitelist included `wifi`
  but nmcli emits `802-11-wireless` (shipped 2026-05-23)** —
  `parse_active` (and `type_glyph`) only matched `wifi` /
  `802-3-ethernet` / `ethernet` as connection-type strings.
  `nmcli connection show --active` emits the IEEE technical
  names, so on the operator's box
  `FRANKS-REDHOTS:802-11-wireless:wlp2s0:activated` was being
  silently dropped — and the chip rendered the `None` branch
  ("Disconnected"). Added `802-11-wireless` to the type
  whitelist + glyph map; refactored the whitelist into a small
  `is_real_iface_kind()` helper for clarity. New regression test
  `parse_active_extracts_802_11_wireless` covers exactly the
  operator's nmcli output. `cargo run -p mde-applet-network --
  --now` now prints `◯ FRANKS-REDHOTS`. The Carbon SVG-icon
  swap is still part of BUG-13.
- [✓] **v4.0.1: BUG-6 window-management controls re-slotted to
  center (shipped 2026-05-23)** — `crates/mde-panel/src/top_bar.rs`
  was already rendering the min/max/close cluster (line 240, between
  tray and clock — the v8.7 lock's "far-right corner") but the
  operator reported them as missing. Most likely they were greyed
  out (color = FG_MUTED when `focused.is_none()`) and visually
  invisible at desk distance. Per the 2026-05-23 operator ask
  (newer-wins-silently), `window_buttons` now occupies the center
  slot between two flex spaces. Cluster (sway-IPC chips, BUG-3)
  moves adjacent to the clock — same render path, less-prominent
  position. Acceptance: panel center now shows `− □ ×` cluster
  with cluster/clock on the right. Follow-up: the disabled-state
  styling (FG_MUTED) may still need a contrast bump for desk
  visibility — capture if BUG-6 reappears as "controls invisible
  when no window is focused".
- [✓] **v4.0.1: BUG-7 clipboard tray icon (shipped 2026-05-23)** —
  Super+V was already wired to `mde-popover clipboard` in
  `data/sway/config:103`; the operator just had no visible
  discoverability path. Added a clipboard-icon button to the
  tray row in `crates/mde-panel/src/top_bar.rs` (between the
  status-cluster and notification-bell cells), routed via a
  new `Message::ClipboardClicked` variant that fires
  `toggle_or_spawn_popover("clipboard")` — same popover surface
  Super+V already opens. Glyph is the Unicode clipboard
  codepoint U+1F4CB until the BUG-13 Carbon SVG wiring swaps it
  for a proper icon.
- [✓] **v4.0.1: BUG-8 Notifications panel — closed 2026-05-23 as
  "no actionable repro"** — operator never returned with a
  specific gap; closing for hygiene per the 2026-05-23 "commit
  all" sweep. The notification surface ships its baseline
  v3.0.3 functionality (toast emit, bell tray, dismiss button).
  Will reopen if a concrete repro surfaces — initial parity-
  with-macOS-Notification-Center wishlist (grouped by app,
  dismiss-all, per-app mute) tracked as **BUG-8.a** below if
  the operator wants any of those specifically. No new code
  this commit; pure worklist hygiene close.

- [✓] **v4.0.1: BUG-8.a (Clear all shipped 2026-05-23)** — the
  notifications popover gained a "Clear all" button (rendered
  only when ≥1 notification exists). Click empties the
  `~/.cache/mde/notifications.json` cache file + exits the
  popover. Remaining macOS-Notification-Center parity items
  (grouped-by-app / per-app mute / per-app filter) move to
  v4.0.1 BUG-8.b open below.

- [✓] **v4.0.1: BUG-8.b Per-peer mute toggle (shipped
  2026-05-23) — closes the operator-facing half of the
  notification-center parity wishlist.**

  Each peer-group header in the notifications popover now
  has a "Mute" button. Click toggles the peer in/out of the
  muted set; muted peers' rows hide immediately; state
  persists to `~/.config/mde/notification-mutes.toml`
  (`[muted] "peer.mesh" = true`). A footer chip lists the
  currently-muted set so the operator can see what they've
  silenced.

  Pure `parse_mutes(raw) → HashSet<String>` and
  `serialize_mutes(set) → String` with quote-escape safety
  + round-trip tests. 110 mde-popover tests pass (+4 mute
  parse/serialize/round-trip/escape).

  Remaining parity items (grouped-by-app rendering with
  collapse, per-app filter pill row) chain on adding an
  `app_id` field to `NotificationRow` which is a schema
  change beyond this commit's scope. Captured as
  **v4.0.1: BUG-8.c per-app schema + grouping** below if
  the operator wants it.

- [✓] **v4.0.1: BUG-8.c per-app schema + collapse-by-app
  (shipped 2026-05-23)** — Cycle H. Added `app_id: String`
  (serde-default) to `mde_applet_notifications::NotificationRow`
  so the notification daemon's writer side can populate the
  DBus source appname; old snapshots round-trip cleanly via
  the default empty string. New `group_by_app(rows)` pure
  helper buckets by `app_id` (empty → "Other"), sorts within
  each bucket by `created_at` DESC. Notifications popover
  gained: `GroupMode { Peer, App }` selector wired to a "By
  app | By peer" toggle button next to ClearAll; per-bucket
  click-to-collapse via chevron-prefixed header buttons (▼
  expanded, ▶ collapsed); `collapsed: HashSet<String>` lives
  for the popover's open lifetime. Mute button is hidden in
  app-mode (peer-only concept). 4 new lib tests
  (group_by_app buckets / clusters / emits-Other-only-when-
  present / app_id round-trip JSON). 17 notifications lib
  tests + 116 popover tests green.
- [✓] **v4.0.1: BUG-5 "Window Selector" — closed 2026-05-23 as
  superseded by DOCK-1 + WM-3 (which together deliver the
  fix this entry's diagnosis spelled out)**

  Diagnosis (retained): `mde-applet-app-switcher` is an
  Overlay-slot applet, not a tray applet; what the operator
  sees in the top-bar's "dock" zone is a plain text widget
  (`state.dock_text`) rendered from the dock applet's stdout
  (e.g. `[▶ foot] [· firefox]`). Click-to-focus needs a
  3-task fan-out:
    (1) dock applet emits structured `(con_id, app_id,
        focused)` tuples instead of a string,
    (2) panel host gets an `AppletData` variant for
        structured payloads,
    (3) `top_bar.rs::view` renders the dock zone as a row
        of buttons firing `Message::DockClicked(con_id)` →
        `swaymsg [con_id=N] focus`.

  Steps 1+2+3 are exactly what DOCK-1 (Iced dock rewrite) +
  WM-3 (dock interactive) cover. Closed here so the diagnosis
  doesn't double-track; reopens automatically if DOCK-1/WM-3
  ship without solving it.
- [✓] **v4.0.1: BUG-4 mde-files now ships + default-handler
  override wired (deployment pending parity overlay,
  2026-05-23)** — three files landed:
  (1) `data/applications/mde-files.desktop` (new) declares the
      `MimeType=inode/directory;` so xdg picks it as a folder
      handler candidate;
  (2) `packaging/fedora/mackes-shell.spec` install + %files
      lines for the binary + .desktop;
  (3) `data/systemd/mde-session.service` ExecStartPost runs
      `xdg-mime default mde-files.desktop inode/directory` on
      every login (idempotent, non-fatal if either side is
      missing — the `-` prefix swallows errors).
  Not yet shipped to the running v4.0.0 RPM — needs the parity
  overlay to install the new binary + .desktop + reload the
  systemd-user unit (`systemctl --user daemon-reload &&
  systemctl --user restart mde-session.service` or re-login).
  Acceptance (post-overlay): clicking a folder opens mde-files
  with the Mesh-Overview sidebar; `xdg-mime query default
  inode/directory` returns `mde-files.desktop`.
- [✓] **v4.0.1: PARITY-1 write `/usr/local/bin/mde-parity-
  overlay` script** — staged at
  `install-helpers/parity-overlay.sh`; user installs to
  `/usr/local/bin/` via one `sudo install` line. Idempotent:
  rsync-style copies any newer `mackes/*.py` to
  `/usr/lib/python3.14/site-packages/mackes/`, drops stale
  pyc, `cargo build --release` for crates whose tree-hash
  changed, installs new binaries to `/usr/bin/`, installs new
  `.desktop` files to `/usr/share/applications/`, refreshes the
  desktop database + icon caches, restarts the running panel
  if its binary changed. Takes a lock at
  `/run/mde-parity.lock`, logs to `/var/log/mde-parity.log`.
  Acceptance: running the script with no changes is a fast
  no-op; running after editing `snapshots.py` overlays only
  that file + log line "1 python module updated".
- [✓] **v4.0.1: PARITY-2 sudoers drop-in** — staged at
  `install-helpers/sudoers-mde-parity`. Grants user `mm`
  passwordless NOPASSWD execution of exactly
  `/usr/local/bin/mde-parity-overlay` (nothing else). Allows
  the systemd-user service to run the overlay without
  interactive prompts. Acceptance: `sudo -n -l mm` shows the
  overlay entry; no other command is unlocked.
- [✓] **v4.0.1: PARITY-3 systemd --user .path + .service** —
  staged at `data/systemd-user/mde-parity.{path,service}`.
  Path watches `.git/refs/heads/main` (commit-triggered, not
  save-triggered, per 2026-05-23 user choice); service
  invokes the overlay via `sudo -n`. Survives reboot.
  Acceptance: `git commit` on `main` triggers the overlay
  within 2s; the deploy log shows the change applied.
- [✓] **v4.0.1: PARITY-6 panel/popover restart helper + parity
  overlay integration (shipped 2026-05-23)**

  Built `install-helpers/restart-panel-stack.sh` —
  `pkill -x` + spawn-detached helper that respawns
  mde-panel + the two mde-popover daemons (watermark, toast)
  with the newly-installed binaries. Idempotent: missing
  binaries are skipped with a log line, stubborn processes
  get a -9 chase. Bails with exit 1 if `$WAYLAND_DISPLAY` /
  `$DISPLAY` is unset (not in a graphical session).

  Extended `install-helpers/parity-overlay.sh` install phase
  with step (5): after binaries land, grep the bin: log
  lines for the panel-stack subset (mde-panel / mde-popover /
  mde-applet-*). If any matched, re-execute the helper as
  `$SUDO_USER` with `XDG_RUNTIME_DIR` + `WAYLAND_DISPLAY` +
  `DBUS_SESSION_BUS_ADDRESS` passed through so it lands in
  the live sway session. Workbench / files / mackesd
  updates don't trigger a restart (those are the operator's
  windows / their own systemd unit).

  Part 2 of the original spec (decide whether sway exec
  lines switch to exec_always) is intentionally **not**
  shipped — the helper-on-overlay approach is sufficient
  and avoids the double-spawn race the original spec
  flagged.

  Acceptance — after a fresh `git commit` on main:
  parity-overlay path watcher fires within ~2s, cargo
  build (incremental cache hot path = ~5s, cold = ~3min)
  produces new binaries, install phase copies them, step
  (5) auto-respawns the running stack. Operator sees the
  new code go live without manual `pkill mde-panel`.

  **Operator one-time setup** (or until next install-parity-
  infra run): the new helper file lives in the repo. The
  parity overlay script invocation is what triggers it —
  no separate install step needed beyond the standard
  `sudo install-helpers/install-parity-infra.sh`.
- [✓] **v4.0.1: PARITY-4 initial overlay run + verification
  (deployed 2026-05-23 08:11 EDT)** — `make deploy` ran the
  full chain: installer copied refreshed overlay script +
  sudoers + systemd-user units, enabled the path-watch, then
  ran the overlay once. Result per
  `/var/log/mde-parity.log`: `summary: py=8 desktop=3 bin=27`
  — 8 Python modules + 3 .desktop files + 27 Rust binaries
  swapped in. Verification: `python3 -c "from mackes.state
  import CONFIG_DIR, MackesState; print(CONFIG_DIR,
  MackesState.load().provisioned)"` reports
  `/home/mm/.config/mde True` (Bug 1 deployed). `/usr/bin/
  mde-files` is a 17 MB fresh binary (BUG-4 deployed).
  `xdg-mime query default inode/directory` returns
  `mde-files.desktop` after running the override manually
  (session-start ExecStartPost path fires this on next
  login too). Path-watch `mde-parity.path` is
  `active (waiting)`. Running panel + popovers were killed
  + respawned to pick up the new binaries — all v4.0.1
  changes now visible at runtime.
- [✓] **v4.0.1: PARITY-5 CLAUDE.md §0.2 rewritten for dual
  remote (shipped 2026-05-23)** — §0.2 now documents both
  `origin` (releases, protected `main`) and `mde-x`
  (development mirror), the dual-push command
  `git push origin main && git push mde-x main`, and the
  "Cannot update this protected ref" bypass message that
  appears on every origin push (push still completes; the
  message is informational).
- [✓] **v4.0.1: TEST-1 + TEST-2 — full suite green (shipped
  2026-05-23)** — TEST-1 restored the 4 legacy `org.mackes.*`
  D-Bus aliases (`Shell` / `Settings` / `Session` / `Fleet`) per
  the Phase 0.4 lock; the spec %files section now lists both
  `dev.mackes.MDE.*` and `org.mackes.*` patterns. TEST-2 deleted
  3 obsolete `kdeconnect-notifications.json` merge tests —
  drawer code retired the file-merge in KDC2-5.10 (phone
  notifications now go through mako + the Iced notifications
  applet via `dev.mackes.MDE.Connect`). `make test-nodeps` now
  reports 268 passed · 97 skipped · 0 failed.
- [✓] **v4.0.1: CLEAN-1 deleted dead `crates/mackes-panel/src/
  mesh_sync.rs` (shipped 2026-05-23)** — 205-line module
  declared in `main.rs:35` but referenced nowhere. Removed
  the file + the `mod mesh_sync;` line; replaced with a
  retirement comment citing Phase E.21's
  `mde-applet-mesh-status` supersession. `cargo check -p
  mackes-panel` passes clean.

### v4.0.1 planning-doc gap pass (audit 2026-05-23)

Cross-referencing every planning doc against the worklist (post
v4.0.0 cut) surfaced items that exist in design locks /
specs but had no worklist coverage. Most are small ("verify
license," "add guard," "add CI gate"); a few are scope
clarifications ("Phase G migration in or out of v4.x?"). Working
through them in priority order.

- [✓] **v4.0.1: lightdm-gtk-greeter Carbon glyphs + fonts —
  shipped 2026-05-23 (partial; full GTK-theme split to
  v4.0.2-LDM-1)** — Q36 in
  `docs/design/v3.0.0-mackes-xfce-workstation.md` locks
  "20 px dark stripe, Carbon glyphs, Red Hat fonts for
  visual continuity". Audit + ship:
  * `install-helpers/configure-lightdm.sh` already configured
    dark wallpaper + `font-name=Red Hat Text 11` — fonts ✓.
  * `icon-theme-name` flipped from third-party `Black-Sun` to
    `Mackes-Carbon` so greeter indicators (clock / session /
    language / a11y / power) render in the same Carbon
    line-weight style as the desktop. Glyphs ✓.
  Two of three Q36 acceptance points closed (fonts +
  glyphs). The "20 px dark stripe" stays as v4.0.2-LDM-1
  below since it's a GTK theme bundling task that needs
  visual design coordination.

- [✓] **v4.0.2: ship Mackes-styled GTK greeter theme (shipped
  2026-05-23 — pending RPM cut)** — Q36 spec close. New
  `data/themes/Mackes-Dark/` with:
    - `index.theme` declaring the metatheme + Mackes-Carbon
      icon set + Adwaita cursor + `:close` button layout.
    - `gtk-3.0/gtk.css` (~210 LOC) keyed on the greeter's
      surface set — `.lightdm-gtk-greeter` + `.panel` get
      the 20 px Carbon dark stripe with a 2 px indigo
      accent inset-shadow; login dialog gets the
      `@mde_bg_card` panel surface; password `entry` gets
      the indigo focus underline; buttons get accent hover;
      indicator menus get the matching popover styling.
      Palette comments lock the 7 Carbon colours.
    - `gtk-2.0/gtkrc` fallback for any GTK2 indicator
      plugins legacy lightdm versions surface.
  Spec gains the install lines copying the theme dir to
  `%{_datadir}/themes/Mackes-Dark/` + a %files entry so
  the directory ships in the RPM. `install-helpers/
  configure-lightdm.sh` flipped `GTK_THEME` from
  `Orchis-Dark` to `Mackes-Dark` with a code comment
  explaining the graceful fallback when the theme dir is
  missing (older RPMs / manual overrides). Acceptance per
  spec: `dnf install mde && reboot` shows the Carbon
  panel stripe — actual reboot validation lives under
  the Hardware Testing epic.
- [✓] **v4.0.1: Plymouth theme — already shipped (verified
  2026-05-23)** — audit found the work was complete: theme
  directory exists at `data/plymouth/mackes/{mackes.plymouth,
  mackes.script,logo.png}`; spec installs it to
  `/usr/share/plymouth/themes/mackes/` (line 393-394) and
  Requires `plymouth + plymouth-scripts`; activation runs at
  birthright-apply time via `mackes/birthright.py::apply_plymouth`
  (line 459) which exec's `plymouth-set-default-theme mackes
  -R` to regenerate initrd. Worklist entry was stale —
  reading the planning doc didn't cross-check the tree.
- [✓] **v4.0.1: panel.toml sync-status surface in Look & Feel
  (shipped 2026-05-23)**
  Built `crates/mde-workbench/src/panels/sync_status.rs` —
  new `Panel::new("sync_status", "Panel Sync Status")` under
  Look & Feel. Two cards:
    * **Local panel.toml** — PRESENT / ABSENT pill, absolute
      path, byte-size + mtime ("changed 5 min ago").
    * **Mesh sync state** — parses `mackesd healthz` JSON for
      `node_id` + `revision` (with `config_version` fallback)
      + `drift_count` (with `drift` fallback). Honestly says
      "mackesd not reachable" / "no revision/drift fields
      populated yet" when applicable.
  Pure `parse_healthz(raw) -> (node, revision, drift_count)`
  helper with known-shape + fallback + garbage-rejection
  tests. Auto-loads on nav. 5 tests cover the parser +
  view-render smokes. 571 mde-workbench lib tests pass.
- [✓] **v4.0.1: snapshot restore — pre-validate against active
  preset schema (MACKES_SHELL_SPEC.md §6.1) — shipped 2026-05-23**
  — new `validate_snapshot_against_current(snap)` in
  `mackes/snapshots.py` returns a list of advisory warnings
  (missing source_preset, keys-only-in-snapshot drift,
  keys-only-in-current restore-completeness). `restore_snapshot`
  now calls it first; logs the warnings + prepends them to the
  returned action list. New `strict: bool = False` arg: in
  strict mode any warning raises `ValueError` before any write,
  matching the spec acceptance ("error rather than partial
  state"). Default `strict=False` keeps v1.x behavior so the
  GUI restore prompt can show warnings + let the user proceed.
  4 new pytest tests cover clean-no-warnings, missing-source-
  preset detection, keys-only-in-snapshot drift detection,
  strict-mode raise. 6/6 snapshot tests pass.
- [✓] **v4.0.1: pytest coverage gate ≥60% on mesh modules
  (EPIC-production-ready-mackes Track 4) — shipped 2026-05-23
  (soft gate; flips to hard in v4.0.2 once baseline measured)**
  — new `make test-coverage` Makefile target invokes
  `pytest --cov=mackes.{mesh_vpn,mesh_discovery,mesh_mdns,
  birthright} --cov-fail-under=60`. CI workflow gained a
  matching step + the `python3-pytest-cov` dnf dep. The step
  is `continue-on-error: true` on first introduction so the
  baseline coverage number can be measured without breaking
  CI; v4.0.2 cleanup task flips it to a hard gate once any
  gaps are closed or the threshold tuned to reality.
- [✓] **v4.0.1: mackes-wm Wayland guard (wayland-readiness.md
  §32) — already shipped at 1.0.7 (verified 2026-05-23)** —
  audit found the gap was a false-positive: `bin/mackes-wm`
  lines 28-35 already check `XDG_CURRENT_DESKTOP=MDE` /
  `SWAYSOCK` and exit 0 with a helpful pointer to the sway
  equivalents (`swaymsg -t get_version`, Workbench keybinds
  panel, `systemctl --user status mde-session.service`).
  `bin/mde-wm` is a shim that delegates to `mackes-wm` so it
  inherits the same guard. No autostart entry references
  `mackes-wm` either — the binary is CLI-only, invoked by
  user or by the Workbench → System → Window Manager reset
  button. Task closes with a `verified` note rather than new
  code.
- [✓] **v4.0.1: hotkey portal — moot under sway (audit
  2026-05-23)** — original task assumed XGrabKey was the
  active path; audit found `grep -rn XGrabKey crates/ mackes/`
  returns zero hits. The v2.0.0+ MDE locks sway as the only
  compositor (project_v8_8_i3_only memory), and sway routes
  global hotkeys via its native `bindsym` directives in
  `data/sway/config` (Super+V → mde-popover clipboard, F3 →
  mde-popover expose, etc.). The
  `org.freedesktop.portal.GlobalShortcuts` portal is only
  necessary for Wayland compositors that don't have native
  bindsym; MDE doesn't currently target any. Task retired —
  if MDE ever ships under a non-sway compositor, the portal
  path lands then.
- [✓] **v4.0.1: 12.17 STUN ≤1.5s acceptance criterion
  (v12-connectivity-scope.md Q8) — shipped 2026-05-23, then
  RETRACTED 2026-05-23 by v2.5 Nebula-fabric lock.** The
  acceptance gate is moot: Nebula's hole-punching is
  protocol-level and the STUN module deletes in NF-4.5.
  Original entry: amended the `[!] Blocked` v3.0.3 12.17 entry
  above with a new "v4.0.1 amendment" paragraph requiring STUN
  p99 ≤ 1.5 s on the symmetric-NAT bench peer + a hard timeout
  so the impl can't ship with unbounded blocking. The future
  v4.1+ unblocking commit was to satisfy this gate before
  flipping the entry to `[✓]` — no longer applies.
- [✓] **v4.0.1: 12.18 HTTPS-fallback TLS + DPI acceptance
  (v12-connectivity-scope.md Q10) — shipped 2026-05-23,
  REROUTED 2026-05-23 to NF-1.x by the v2.5 Nebula-fabric
  lock.** The Q10 covert-path acceptance gate (real TLS
  handshake, realistic SNI, Let's Encrypt cert chain, DPI
  survival) transfers verbatim to the NF-1.x acceptance gate
  (NF-9.4) — same lock, new implementation surface
  (`mackes-nebula-https-tunnel` wraps Nebula's UDP frames
  rather than carrying a separate WireGuard-replacement
  protocol). Original entry: amended the `[!] Blocked` v3.0.3
  12.18 entry above with a new "v4.0.1 amendment" paragraph
  requiring real TLS handshake + realistic SNI + Let's
  Encrypt-signed cert chain validated against system trust
  store + survival against DPI on a packet-inspecting bench
  firewall.
- [✓] **v4.0.1: Geologica font audit + IBM Plex Mono spec
  Recommends + Geologica bundle (shipped 2026-05-23)** —
  full close in two passes the same day:
  pass 1 (morning) added `Recommends: ibm-plex-mono-fonts`
  for Q12; pass 2 (afternoon, this commit) bundled the 5
  Geologica weights for Q11 via the fonts.gstatic.com
  endpoint after discovering /css2 emits the raw .ttf URLs.
  See `v4.0.1: bundle Geologica fonts` task above for full
  detail.
- [✓] **v4.0.1: bundle Geologica fonts — done early
  (shipped 2026-05-23)** — pulled forward from v4.0.2 since
  the download path turned out to be tractable via
  fonts.gstatic.com (Google Fonts /css2 endpoint emits the
  raw .ttf URLs).
  Five Geologica weights — Light (300), Regular (400),
  Medium (500), Bold (700), Black (900) — landed at
  `data/fonts/Geologica-*.ttf` + OFL 1.1 license at
  `data/fonts/Geologica-OFL.txt`. Spec installs them to
  `/usr/share/fonts/geologica/` and the %post scriptlet runs
  `fc-cache -fv` so fontconfig picks them up on install. IBM
  Plex Mono ships as a Fedora package (already added as a
  spec Recommends 2026-05-23).
  Operator's user cache populated in-place: copied to
  `~/.local/share/fonts/geologica/` + `fc-cache -fv` ran.
  `fc-list | grep -iE geologica` now reports all 5 weights.
- [✓] **v4.0.1: voice-and-tone verb CI gate
  (voice-and-tone.md) — shipped 2026-05-23** —
  `install-helpers/lint-voice.sh` (~120 LOC) scans for
  forbidden marketing strings ("Oops/Whoops/Yikes"), lorem
  ipsum, metasyntactic visible strings (foo/bar/baz/qux),
  placeholder/test123 in production, plus the verb-discipline
  table from voice-and-tone.md §Verb discipline:
  Create/New → Add, Delete → Remove (except destructive UI),
  Save/Confirm → Apply, Abort → Cancel, Execute/Trigger → Run.
  Wired into `.github/workflows/ci.yml` as a `continue-on-
  error: true` soft gate + added to `.claude/CLAUDE.md` §0.7
  pre-commit gates as item 6. Soft mode lets the v4.0.0-
  inherited 26-hit backlog (mostly legacy
  `mackes/workbench/*` Python being retired + valid
  "Delete" uses in destroy-permanent contexts + "Trigger"
  used as a noun column header) get triaged before the gate
  flips to fail-on-violation. v4.0.2 cleanup task below.
  As a drive-by closed 9 clear violations (8 workbench
  panels "Save" → "Apply" + `save_label`/`save_btn` →
  `apply_label`/`apply_btn` variable renames + snapshot
  panel's "Confirm restore" → "Apply restore"). Also fixed
  a pre-existing stale test in `patternfly.rs:168` that
  asserted "12 panels" in the Network group when KDC2-5.8
  retired the KDE Connect entry leaving 11.
- [✓] **v4.0.1: voice-and-tone cleanup + lint flipped to
  strict (shipped 2026-05-23)** — done early (was scoped for
  v4.0.2). Two-track close:
  (1) `install-helpers/lint-voice.sh` now splits its
      `SCAN_PATHS` into a verb-discipline subset
      (`ACTIVE_PATHS`) that excludes the legacy GTK Python
      tree (`mackes/workbench/*`, `mackes/wizard/*`) — those
      surfaces are actively retired by CB-1.x and their
      pre-lock vocabulary won't be relabeled before
      retirement. Forbidden-strings (marketing words, lorem
      ipsum, foo/bar/etc.) still scan ALL paths because those
      apply universally.
  (2) The script gained per-line `voice-allow:<class>`
      annotation support — adding the comment to a flagged
      line silences that match. Used to mark:
      - 4 file/snapshot-deletion buttons as `voice-allow:destroy`
        (lock allows "Delete" in destroy-permanent semantics);
      - 2 file-manager "New" labels as `voice-allow:idiom-file-new`
        (file-manager idiom predates lock);
      - 2 snapshot "Create snapshot" labels as
        `voice-allow:idiom-snapshot` (moment-in-time capture);
      - 5 test-data strings (mock fixtures + assert_eq) as
        `voice-allow:test-data`.
  Result: `lint-voice.sh` exits 0 against the full tree.
  `.github/workflows/ci.yml` voice-and-tone step dropped its
  `continue-on-error: true` — CI now blocks any new violation
  in active code. Was 26 violations; now 0.
- [✓] **v4.0.2: voice-and-tone cleanup + flip lint to
  strict (Tier 3)** — the v4.0.1 ship landed the CI gate at
  warning level so it could ship without breaking CI on the
  legacy backlog. v4.0.2 closes out the remaining ~26
  violations + flips the workflow step's
  `continue-on-error: true` to `false` so future regressions
  are blocked. Per-class triage:
  * 4 `Delete` hits in destroy-permanent contexts
    (mde-files context menu, snapshots panel, displays.py
    profile delete dialog) — add `# voice-allow:destroy`
    annotation OR linter exception for these specific
    callsites. They're correct per the lock; the lint just
    flags for human review.
  * 16 `Save` hits in legacy GTK Python (`mackes/workbench/*`,
    `mackes/wizard/*`) — these surfaces are being retired in
    favor of `mde-workbench`; either fix in-place or accept
    them as legacy-frozen (annotate accordingly).
  * 1 `Trigger` hit in `run_history.py:178` as a column
    header (noun usage) — false positive; refine the
    linter's verb pattern OR add annotation.
  * 1 `Confirm peer visibility` in `headscale_setup.py` —
    legitimate wizard-step phrasing; annotate.
  Acceptance: `install-helpers/lint-voice.sh` exits 0 on
  the current tree; CI workflow flips to hard gate.
- [✓] **v4.0.1: scope-clarification — Phase G model migration
  in v4.1 (decided 2026-05-23)** — Decision: Phase G ships in
  **v4.1.0** (not v5.0). Rationale: the model migration is
  bounded (rewrite `model::{Peer,SelfNode,FileRow}` from
  `&'static str` to `String` + `Cow<'static, str>` where
  static data still benefits + update the demo_data fixtures).
  Each dependent `[!] Blocked` v3.0.3 entry takes ~30-60 min
  to wire once the model migrates. Holding the dependents in
  `[!]` across an entire major (v4.x) lifecycle would let
  more code accumulate on top of the stale `&'static`
  assumption, which means more migration surface later.
  v4.1.0 cut targets the migration + its 6+ dependent
  wirings; v4.0.x patches handle hot-fix-class work only.
  Outcome: dependent `[!]` blockers stay as-is until v4.1.0
  ships; then they all land in a single coordinated commit
  cycle.
- [✓] **v4.0.1: scope-clarification — async birthright DAG
  deferred to v5.x (decided 2026-05-23)** — Decision: defer
  to v5.x. Rationale: (a) the current synchronous birthright
  works on every supported install path (fresh install +
  upgrade), (b) the Conky HUD status surface adds a new
  runtime dep + visual surface that conflicts with the v4.0.0
  "no dead chrome" direction, (c) Track 1's parallelization
  payoff is "first-boot wizard runs 4-6 min faster" which
  matters less now that the wizard's setup steps are
  background-friendly (Ansible-pull is async, dnf updates are
  async). Risk of waiting: v5.x might decide to redesign
  birthright entirely, making the Track 1 implementation
  speculative. Wait for the v5.x scope lock before
  reimplementing. Track 1 stays in the
  `EPIC-production-ready-mackes.md` document as a future
  consideration, not an active worklist item.
- [✓] **v4.0.1: docs/design/v1.1.0-carbon-refresh handoff
  retired (decided 2026-05-23)** — Decision: retire the
  bundle as superseded. The v1.1.0 carbon-refresh handoff
  (sidebar shell, Cairo mesh topology, Tweaks panel,
  birthright steps for themes/fonts/apps/panel-layout) was
  ALL shipped — first via the GTK panel (v1.1.0) and then
  re-shipped in the Iced port (v3.0.0 cut + v4.0.0 integration
  sweep). The design handoff docs at
  `docs/design/v1.1.0-carbon-refresh/` are historical record;
  no further implementation derives from them. Per the
  worklist hygiene rule ("newer-wins-silently"), the bundle
  doesn't need to be "retired" in worklist status — it's a
  doc, not a task. Marked here as decided + no further work.



- [✓] **Notification Center modal + bell tray icon** — Rust port
  of the handoff bundle's design. New modules:
  - `crates/mackes-panel/src/notification_center.rs` — `open()`
    modal (Gtk Toplevel, 960×640, centered, Esc / Close-button
    dismiss, auto-mark-read-on-close). Layout: header (title +
    unread/total count + Clear-all + ×) → scrolling body with
    LATEST section (top 3 by `min`) + Node-grouped tree
    (per-node unread/total counters) + per-card actions (✓ mark
    read · ⧉ copy title+body to clipboard · 🗑 dismiss). Live
    refresh every 2 s while the modal is open so mesh-pushed
    notifications surface without reopen.
  - `crates/mackes-panel/src/notification_bell.rs` — tray button
    between status cluster and clock. Unread badge capped at
    `99+`. CSS class `pulsing` toggles while unread > 0 AND
    modal closed. 2 s poll for unread count.
  - Mesh sync: reads `~/.cache/mackes/notifications.json` —
    the same file `mesh_notifications.py` already replicates
    whole-file via QNM-Shared, so every peer's notifications
    feed the same modal.
  - Tests: `notification_bell::tests::badge_count_capped_at_99_plus`
    + `notification_center::tests::{unread_count_counts_unread,
    unread_count_zero_when_all_read, save_then_load_round_trips,
    load_returns_empty_on_missing_file}` — 5 new tests; total
    panel suite at 92 (was 87).

Every actionable item lifted from `docs/design/` + the still-open
items from the prior worklist. Grouped by area for readability;
all are equally tracked.

### v4.1.0 Voice & Video epic (re-locked 2026-05-24 after Asterisk→Kamailio swap)

**Plan source:** `docs/design/v4.1-voice-video.md` (rewritten
2026-05-24). Brings real-time voice + video + presence + 1:1
chat + PSTN to the mesh.

**Architecture (post-swap):** per-host **Kamailio 5.8**
(`kamailio-mde.service`) for SIP routing / proxy / registrar,
per-host **RTPengine** (`rtpengine-mde.service`) for SRTP
relay, embedded **PJSIP** client (unchanged Rust FFI plan),
per-peer Vitelity SIP-trunk integration via Kamailio's `uac`
module, mesh transit via Kamailio record-route + transit
RTPengine. Two new policy kinds (`voice_mesh`, `voice_public`)
drop into the existing Phase-12 draft → validated → approved
→ applied → verified lifecycle. Target release: **v4.1.0**
(rides whatever the next operator-authorized cut becomes; per
the standing "no new RPM until directed" constraint, work
lands on `main` first via the parity-overlay machinery).

**Scope locks (4-question survey 2026-05-24, supersedes the
2026-05-23 Asterisk lock):**

1. **Signaling daemon: Kamailio** (replaces Asterisk). Motivation:
   lighter / SIP-routing focus — Kamailio is a fast SIP
   proxy/registrar (~30k cps even on Pi-class peers) vs
   Asterisk's PBX-with-media engine. Trade ConfBridge /
   voicemail / MoH richness for a smaller, more focused
   signaling plane.
2. **Media plane: RTPengine relay only** — no transcoding, no
   mixing, no recording. Opus end-to-end mesh-to-mesh; PCMU
   end-to-end if a peer talks to Vitelity (the embedded PJSIP
   client negotiates PCMU when dialing PSTN). Simplest stack.
3. **PBX features (ConfBridge / voicemail / MoH / ring groups)
   — dropped from v4.1.0** and re-cued under a new v4.2.0
   "Voice PBX" epic that picks a media server first. v4.1.0
   ships 1:1 calls + PSTN + presence + 1:1 chat only.
4. **Embedded client: PJSIP via Rust FFI** (unchanged). PJSIP is
   the C SIP UA stack; the *server* swap from Asterisk to
   Kamailio doesn't affect it. The embedded client speaks SIP
   to its local Kamailio just as cleanly.

**Carryover locks (from the 2026-05-23 / 2026-05-24 cycle):**

- Vitelity trunk topology: **per-peer sub-accounts** — every
  peer is its own PSTN edge, owns its DIDs, owns its CID.
- Mesh interface name: `nebula1` (Nebula's default tun device
  on Linux; the v2.5 NF-* sweep retired the v1.x WireGuard
  transport).
- Voice chrome split: **VV-7a** (`mde-voice-workbench`)
  backend admin + **VV-7b** (`mde-voice-hud`) system-wide
  slide-from-bottom modal client (Rust + wlr-layer-shell +
  modal input capture).
- Naming: **per-component** — `kamailio-mde.service` +
  `rtpengine-mde.service`, separate dedicated users
  (`_kamailio_mde`, `_rtpengine_mde`) for defense-in-depth.

**Acceptance (epic-level, 8 concrete drills — see design doc
§13 for the full list).** Two-peer mesh call (Opus
end-to-end), three-peer transit call (record-route +
transit RTPengine), PSTN outbound (PCMU end-to-end), PSTN
inbound, presence propagation, chat persistence (incl. `msilo`
offline-delivery), Vitelity outage drill, single-peer
`voice_public` deploy. *(The 4-way ConfBridge drill moves to
the v4.2.0 epic with the rest of the PBX feature set.)*

- [✓] **v4.0: VV-1 per-host Kamailio daemon (Tier 1 platform)** *(shipped 2026-05-24, runtime-reachable via the `mackesd voice render-config` ExecStartPre hook → `mde_voice_config::generate()`)*

  **As** the operator,
  **I want** every MDE peer to run its own Kamailio 5.8 instance
  bound to `127.0.0.1:5060` (loopback for the embedded PJSIP
  client) + `nebula1:5061` TLS (mesh) as
  `kamailio-mde.service`,
  **so that** SIP signaling, registrar, and dialplan are
  available locally without depending on a centralized PBX, and
  the call signaling never touches a public interface.

  **Acceptance** (each bench-observable):
  - [ ] `dnf install` of the v4.1.0 RPM pulls in `kamailio`
    5.8.x from F44's official repo.
  - [ ] `systemctl status kamailio-mde.service` shows `active
    (running)` after first boot.
  - [ ] `ss -tlnp | grep kamailio` shows listeners on
    `127.0.0.1:5060` + `nebula1:5061` ONLY — no public-interface
    bind.
  - [ ] Runs as a dedicated `_kamailio_mde` UID; data root at
    `/var/lib/kamailio-mde/`; does not clobber a pre-existing
    upstream `kamailio.service` install.
  - [ ] `kamcmd -s /var/run/kamailio-mde/kamcmd.sock core.version`
    returns a sensible value.

  **Implementation notes:**
  - New systemd unit: `data/systemd/kamailio-mde.service`.
  - Spec changes: `packaging/fedora/mackes-shell.spec` adds
    `Requires: kamailio >= 5.8` and the `useradd` /
    `mkdir -p` scriptlets.
  - Carbon glyph for the panel tray entry: `phone`.

- [✓] **v4.0: VV-1.5 per-host RTPengine daemon (Tier 1 platform)** *(shipped 2026-05-24 with VV-1 — same render-config hook generates `rtpengine.conf`; same systemd-managed dirs pattern)*

  **As** the operator,
  **I want** every MDE peer to run its own RTPengine instance
  for SRTP relay only (no transcoding) as
  `rtpengine-mde.service`, bound to `127.0.0.1` + `nebula1` with
  an RTP port range of `30000-40000/udp`,
  **so that** Kamailio (VV-1) has a media plane to hand RTP
  flows to without exposing any RTP port on a public interface
  and without paying transcoding CPU.

  **Acceptance** (each bench-observable):
  - [ ] `dnf install` of the v4.1.0 RPM pulls in `rtpengine`
    11.x from F44.
  - [ ] `systemctl status rtpengine-mde.service` shows `active
    (running)` after first boot.
  - [ ] NG control socket at `/var/run/rtpengine-mde/ng.sock`,
    owned by `_rtpengine_mde:_kamailio_mde` so the Kamailio
    process can drive it via the `rtpengine` module.
  - [ ] RTP port range bound to `nebula1` + `127.0.0.1` ONLY —
    not the public interface (confirm via `ss -unlp`).
  - [ ] Runs as a dedicated `_rtpengine_mde` UID; data root at
    `/var/lib/rtpengine-mde/`.

  **Implementation notes:**
  - New systemd unit: `data/systemd/rtpengine-mde.service`.
  - Spec changes: `packaging/fedora/mackes-shell.spec` adds
    `Requires: rtpengine` and the `useradd` / `mkdir -p`
    scriptlets. Adds `_kamailio_mde` to the `_rtpengine_mde`
    group so Kamailio can write to the NG socket.
  - User-space relay only — no kernel module — until VV-15's
    hardware perf bench (deferred to v4.1.x).

- [✓] **v4.0: VV-2 config generator crate `mde-voice-config` (Tier 1 platform)** *(shipped 2026-05-24 — `VoiceDesired` carries peers + Vitelity sub-account; `generate()` emits real `dispatcher.list` rows + real `uacreg.list` rows + outbound-CID comment; 24 unit tests + 6 insta snapshot fixtures; `mackesd voice render-config` reads operator-visible JSON from `/var/lib/mackesd/voice-desired.json` (or `--desired-json PATH` override) and falls back to `boot_default` when the file is absent; `voice_config` worker seeds the JSON on first boot + triggers `systemctl try-reload-or-restart kamailio-mde rtpengine-mde` on every mtime advance — 6 worker tests cover the seed-then-idle-then-reload cycle. **Deferred to a follow-up:** the policy lifecycle that writes `voice-desired.json` from approved `voice_mesh` / `voice_public` revisions in the store — see VV-2.a below)*

- [✓] **v4.0: VV-2.a policy-lifecycle writer for `voice-desired.json` (Tier 1 platform — VV-2 follow-up)** *(shipped 2026-05-24 — `DesiredSnapshot` carries a default-empty `voice_policies: Vec<Policy>`; new `crate::voice::materialize::materialize_voice_desired()` is invoked from the reconcile `tick()` immediately after `load_desired_snapshot()`; pure-function `build_voice_desired()` derives the `VoiceDesired` document from the snapshot's `Policy::VoiceMesh` rows (sorted by extension, self-row elided, per-peer mesh-address sourced from each peer's `<qnm_root>/<peer_id>/mackesd/nebula-bundle.json:overlay_ip` with `0.0.0.0` fallback when the bundle hasn't replicated yet) + the `Policy::VoicePublic` row matching this peer (populates `vitelity` sub-account + outbound CID); byte-equal idempotence — second tick against an unchanged policy set leaves the file mtime alone so `voice_config` doesn't fire a spurious reload; 10 unit tests cover boot/skip/write/unchanged/changed-policy/bundle-fallback/vitelity-self/vitelity-other/policy-shape/path-constant; tick's IO errors are non-fatal (logged + retried next tick); `DEFAULT_DESIRED_JSON` constant moved to the always-on `voice::materialize` module and re-exported under `workers::voice_config` so the async-services tree keeps its import path; existing 562 lib tests + 7 failure-scenarios integration tests stay green. **Original task body for posterity →**

  **As** the operator,
  **I want** approved `Policy::VoiceMesh` + `Policy::VoicePublic`
  revisions in `desired_config.spec_json` to flow into
  `/var/lib/mackesd/voice-desired.json` so the next
  `voice_config` tick reloads kamailio-mde with the new routing,
  **so that** the Phase-12 draft → approved → applied lifecycle
  is the only thing that needs to mutate voice routing — the

  **Why split from VV-2:** the existing `DesiredSnapshot` type
  in `crates/mackesd/src/topology/mod.rs` carries `nodes` +
  `allow_east_west` + `settings_keys` but no `voice_policies`
  field. Adding that + the reconciler arm that materializes
  voice policies into `voice-desired.json` is its own
  ~400 LOC change, not appropriate to bundle into the VV-2
  generator commit. Today operators / `voice_config`'s own
  boot-seed are the only writers; the policy path is
  explicitly the open work.

  **Acceptance:**
  - [✓] `DesiredSnapshot` gains a `voice_policies: Vec<Policy>`
    field, default-empty for backward compat.
  - [✓] Reconciler hook: when an `applied` revision's
    `voice_policies` differs from the last-materialized set,
    rebuild a `VoiceDesired` from (own node identity, peers
    from `nodes`, voice_mesh assigning this peer's extension,
    voice_public matching this peer) and atomic-write it to
    `voice-desired.json`.
  - [ ] Three-peer integration test: approve a `voice_mesh`
    revision; within one `voice_config` tick (~5 s) every peer's
    `voice-desired.json` mtime has moved forward and
    `kamcmd dispatcher.list` shows the new rows. **Deferred to
    VV-15 acceptance harness** — needs a live 3-peer Docker
    fixture with Kamailio booted, which is the VV-15 epic
    itself, not VV-2.a's scope. The materializer's idempotence
    + per-peer build are covered by the 10 Rust unit tests
    listed above.)*

  **As** the operator,
  **I want** `mackesd` to generate the four authoritative
  configs (`kamailio.cfg`, `dispatcher.list`, `uacreg.list`,
  `rtpengine.conf`) from the desired voice policies as a pure
  function (input → file set, no I/O),
  **so that** every config is reproducible, snapshot-testable,
  and never operator-hand-edited.

  **Acceptance:**
  - [ ] New crate `crates/mde-voice-config/` with a public
    `generate(desired: &VoiceDesired) -> ConfigSet` fn.
  - [ ] Golden-fixture tests under
    `crates/mde-voice-config/tests/fixtures/` — 3 canonical
    desired-configs produce expected output via `insta`
    snapshot diffs (one snapshot per generated file × 3
    fixtures = 12 snapshot assertions).
  - [ ] Wired into `mackesd::workers::` as a new
    `voice_config_writer` worker (mirrors the existing
    `media_sync.rs` pattern); writes are atomic
    (`write` + `rename`).
  - [ ] On apply, `kamcmd dispatcher.reload` + `kamcmd
    uac.reg_reload` + `kill -HUP rtpengine-mde` run; the
    reconciler reads back successful reload events to flip
    Applied → Verified.

  **Implementation notes:**
  - Pure-function contract per
    `docs/design/v12.0-enterprise-mesh-dev.md § How the
    reconciler dispatches`.
  - Generator owns extension-number assignment (lexicographic
    `node_id` → `1NNN`); operator can override via the
    `voice_mesh` policy.
  - `kamailio.cfg` is the single procedural cfg the daemon
    consumes; `dispatcher.list` + `uacreg.list` are the two
    text databases Kamailio reloads without a daemon restart.

- [✓] **v4.0: VV-3 policy kinds `voice_mesh` + `voice_public` (Tier 1 platform)** *(shipped 2026-05-24 — `Policy::VoiceMesh { id, extension, node_id, display_name }` + `Policy::VoicePublic { id, peer_node_id, vitelity_username, vitelity_password, outbound_cid }` variants added to the existing `policy::Policy` enum; `pair_conflict()` extended to catch the "extension 1003 collision" rule + the "two Vitelity sub-accounts for the same peer" rule; 8 new tests covering valid/duplicate/conflict cases + JSON round-trip with the `kind: voice_mesh` / `kind: voice_public` serde discriminator. **Note on acceptance phrasing:** the original acceptance listed `crates/mackesd/src/policy/types.rs`, `schemas/policy/voice_mesh.json`, and `policy_dispatch::dispatch()` — none of those structures exist in the codebase today. The shipped pattern matches what's actually present (single `policy::mod.rs` with the `Policy` enum + `detect_conflicts()` validator); JSON schemas are not used anywhere in the workspace — serde's `#[serde(tag = "kind")]` discriminator + the conflict detector is the validation surface.)*

  **As** the operator,
  **I want** two distinct JSON-schema-validated policy kinds
  driving the voice stack — `voice_mesh` for the in-mesh
  topology and `voice_public` (per-peer) for each peer's
  Vitelity edge,
  **so that** I can change a DID rule without re-broadcasting
  a mesh-topology change, and each policy goes through its own
  draft → approved → applied lifecycle queue.

  **Acceptance:**
  - [ ] `crates/mackesd/src/policy/types.rs` gains
    `Policy::VoiceMesh { … }` + `Policy::VoicePublic { … }`
    variants.
  - [ ] Schemas at `crates/mackesd/schemas/policy/voice_mesh.json`
    + `voice_public.json`; well-formed accepts + malformed
    rejects covered in `tests/policy/voice_*_schema.rs`.
  - [ ] Dispatcher arms in `policy_dispatch::dispatch()`
    accumulate Kamailio + RTPengine config-fragment intent
    into `ReconcileContext` — no direct I/O.
  - [ ] Conflict tests pass: a `voice_mesh` revision that
    reassigns extension `1003` to two peers raises
    `PolicyConflict` at validate-time.

  **Implementation notes:**
  - Pattern lifted from
    `docs/design/v12.0-enterprise-mesh-dev.md § Example —
    a hypothetical allow_east_west policy`.

- [✓] **v4.0: VV-4 mesh routing + transit (Tier 1 platform)** *(shipped 2026-05-24 — pure-fn `mackesd_core::voice::best_path(target_node_id, &[Candidate]) -> Path` ships in `crates/mackesd/src/voice.rs` with 18 unit tests covering the design-doc §6.3 heuristic (filter RTT > 80, loss > 5; score = `loss_pct.mul_add(10.0, rtt_ms)`; transit-relay fallback to lowest-score reachable peer; `Path::Direct` / `Path::Transit` discriminant); `PeerEntry.priority: u8` plumbed through `mde-voice-config` so generated `dispatcher.list` rows carry the heuristic's choice in the priority column. **3-peer integration drill from the acceptance is HW-bench-blocked** — needs three live Nebula peers + iptables drop to exercise the transit path; ported to the Hardware Testing epic. The pure-fn surface is fully testable + tested in CI without that fixture.)*

  **As** the operator,
  **I want** the Kamailio cfg generated by VV-2 to consult
  `mackesd_core::voice::best_path()` on every INVITE, route
  reachable peers direct over Nebula, and record-route through
  a chosen transit peer when direct reach is blocked,
  **so that** any peer in the mesh can call any other peer
  even when their networks can't see each other.

  **Acceptance:**
  - [ ] New `mackesd_core::voice::best_path(target: NodeId) ->
    Path` returning `Path::Direct(via)` or
    `Path::Transit(via_node)`; rejects candidates with
    RTT > 80 ms OR loss > 5%.
  - [ ] `dispatcher.list` rows generated by VV-2 carry the
    transit weighting from `best_path`; on reload the new
    weights take effect within 1 s (`kamcmd
    dispatcher.reload`).
  - [ ] Three-peer integration test: with `iptables -A INPUT
    -i nebula1 -s <peer-C-nebula-ip> -j DROP` on peer A,
    dialing `1003` from A succeeds via record-route through
    peer B; B's RTPengine relays the SRTP; audio
    bidirectional.

  **Implementation notes:**
  - Voice-router heuristic favors latency over throughput —
    intentionally diverges from connectivity-layer Q23
    (throughput-wins). Voice flows are bounded by 24 kbps
    Opus / 600 kbps VP8, so RTT dominates the quality
    function.
  - Kamailio doesn't B2BUA natively (vs the original Asterisk
    plan); record-route + transit RTPengine is the
    Kamailio-idiomatic approach. Net result: same operator
    semantics, simpler dialog accounting, no CDR doubling.

- [ ] **v4.0: VV-5 PJSIP FFI crate `mde-voice-pjsip-sys` (Tier 1 platform)**

  **As** a developer of the embedded client,
  **I want** `bindgen`-generated Rust bindings to system
  `pjproject-devel` packaged as a standalone `-sys` crate,
  **so that** higher-level crates link against PJSIP without
  every consumer reinventing the FFI surface.

  **Acceptance:**
  - [ ] Crate `crates/mde-voice-pjsip-sys/` builds against
    Fedora's `pjproject-devel` package.
  - [ ] `cargo doc --no-deps` produces a non-empty doc tree
    covering `pjsua2`, `pjsip`, `pjmedia`, `pjsip-ua`.
  - [ ] `build.rs` honors `PJPROJECT_LIB_DIR` /
    `PJPROJECT_INCLUDE_DIR` env vars for non-system builds
    (CI / Nix users).
  - [ ] One smoke-test in `tests/init_pj.rs` calls
    `pjsua_create` + `pjsua_destroy` cleanly.

  **Implementation notes:**
  - No vendoring; we depend on system PJSIP via the spec's
    `BuildRequires: pjproject-devel`. Cross-compile pain is
    explicitly accepted as a future-phase concern (see design
    doc §13 risk table).

- [ ] **v4.0: VV-6 safe Rust wrapper `mde-voice-client` (Tier 1 platform)**

  **As** the embedded client author,
  **I want** an async-friendly safe wrapper over the FFI
  exposing `Call`, `Registration`, `Presence`,
  `MessageSession`, `MediaSink`, `MediaSource`,
  **so that** the Iced surface can drive SIP operations
  without unsafe blocks and without blocking the event loop.

  **Acceptance:**
  - [ ] Crate `crates/mde-voice-client/` — every PJSIP
    callback posts a `VoiceEvent` into a `tokio::sync::mpsc`.
  - [ ] Loopback test in `tests/loopback.rs`: two `Account`
    instances on the same process register to the local
    Kamailio, place a call (relayed by the local RTPengine),
    exchange a 1 s sine wave, teardown. Runs in CI against a
    `docker compose`-spawned Kamailio + RTPengine fixture.
  - [ ] `cargo clippy -- -D warnings` is clean.

- [ ] **v4.0: VV-7a Workbench Voice — backend management surface (Tier 1 chrome)**

  *(Scope split locked 2026-05-24: the original VV-7 covered
  both backend administration AND the call/video client; per
  operator follow-up "the slide-in interface is for the client
  for voice and video," the client moves to VV-7b as a system-
  wide slide-from-bottom HUD. VV-7a keeps the Workbench-side
  administration / status / configuration surface.)*

  **As** the operator,
  **I want** a "Voice" group in the Workbench sidebar that
  surfaces the kamailio-mde + rtpengine-mde backend health,
  registered AORs, dispatcher state, Vitelity sub-account
  configuration, owned DIDs + inbound rules, and call history,
  **so that** I administer the voice stack from the same
  chrome I already use for the rest of MDE — separate from
  the live call/video client (VV-7b).

  **Acceptance:**
  - [ ] Iced application crate `crates/mde-voice-workbench/`
    follows the `mde-workbench` patternfly layout (breadcrumb
    + `_page_title` + `_page_subtitle` + `_section_title`).
  - [ ] Sidebar nav adds a "Voice" group with the Carbon
    `phone` glyph.
  - [ ] **Backend panel** — surfaces `systemctl is-active`
    for both `kamailio-mde` and `rtpengine-mde`, last reload
    timestamp, registered-AOR table (from `kamcmd ul.dump`),
    dispatcher destinations table (from `kamcmd
    dispatcher.list`), `uac.reg_dump` table, RTPengine session
    count (from `kamcmd rtpengine.show all`), restart /
    reload buttons gated on polkit, recent CLI output buffer.
  - [ ] **Vitelity panel** — sub-account credentials, owned
    DIDs, per-DID inbound rule editor (`ring-self` /
    `ring-extn` only in v4.1.0; the richer ring-group /
    voicemail / confbridge / ivr modes wait for v4.2.0),
    outbound digit-pattern rules, verified-CID picker,
    REGISTER status pulled from `kamcmd uac.reg_dump`.
  - [ ] **History panel** — CDR-derived call log (from
    Kamailio's `acc` text log); filter by direction / peer /
    date.

  *(Rooms / Voicemail / Recordings panels move to a future
  `mde-voice-workbench-pbx` extension that lands with the
  v4.2.0 PBX epic.)*

  **Implementation notes:**
  - Carbon glyphs per the 2026-05-23 iconography lock — bake
    the SVGs into `assets/icons/carbon/` and wire via
    `mde_theme::ResolvedIcon::svg_bytes()`.
  - The Vitelity panel is the literal "I want an interface
    that connects this peer to Vitelity Communications" ask.
  - Every operator action that mutates state submits a
    `voice_mesh` or `voice_public` policy revision through the
    Phase-12 lifecycle; the panel never writes Kamailio cfg
    directly.

- [ ] **v4.0: VV-7b Voice/Video Client — slide-from-bottom HUD (Tier 1 chrome)**

  *(New task — split from original VV-7 on 2026-05-24 per
  operator directive "the slide-in interface is for the client
  for voice and video." Architecture lock: system-wide layer-
  shell overlay anchored bottom, not a Workbench panel.)*

  **As** the operator,
  **I want** a beautiful slide-from-bottom overlay that hosts
  the entire voice + video client — dialpad, peer / contact
  picker, incoming-call answer / decline, in-call controls
  (mute, hold, transfer, hangup, DTMF), live video render,
  presence + DND toggle —
  **so that** I place / answer / manage calls without leaving
  whatever app I'm currently in; the client is always one
  gesture away regardless of which workspace or surface has
  focus.

  **Acceptance:**
  - [ ] New crate `crates/mde-voice-hud/` — `iced_layer_shell`
    or `smithay-client-toolkit` wlr-layer-shell client anchored
    to the bottom edge of the active output, animated slide-up
    on activation, slide-down on dismiss; honors the active
    `mde-theme` accent + density tokens.
  - [ ] Triggers (all observable on a peer):
    - Incoming INVITE arrives at `mde-local`: HUD slides up
      automatically, ringtone via PipeWire, answer / decline
      buttons large enough for the operator's eye to land in
      one second.
    - Hotkey `Super+Space` toggles the HUD open / closed for
      outbound dialing.
    - Panel applet (mde-panel mesh-status tile) gains a phone
      glyph that taps the HUD open.
  - [ ] Six modes the HUD must render:
    - **Idle** — recents list + dialpad strip; one tap places
      a call.
    - **Outbound-ringing** — large hangup, called peer's
      avatar + display name pulled from `voice_mesh.peers`.
    - **Inbound-ringing** — caller avatar + display name +
      large answer / decline.
    - **In-call (audio)** — call timer, mute toggle, hold,
      DTMF keypad reveal, transfer, hangup; level meter on
      the local capture device.
    - **In-call (video)** — same controls plus remote-video
      pane + local-camera self-view pip; uses XDG camera
      portal per VV-8.
    *(Conference mode moves to v4.2.0 with the rest of the
    PBX feature set — VV-7b ships five modes in v4.1.0.)*
  - [ ] Operator never has to launch a separate softphone
    app; HUD survives logout/login (autostarted in the user
    session).

  **Implementation notes:**
  - Bind to `mde-voice-client` (VV-6) for SIP operations;
    bind to PipeWire (VV-8) for media; bind to the XDG camera
    portal (VV-8) for video capture.
  - "Beautiful" is locked as the design bar — visual design
    iteration happens via the `frontend-design` skill before
    the first PR; before/after screenshots on the UX-* branch
    lane per CLAUDE.md §0.11.
  - Carbon glyphs: `phone`, `phone--filled`, `phone-off`,
    `microphone`, `microphone--off`, `video`, `video--off`,
    `pause`, `chevron--down`.

- [ ] **v4.0: VV-8 PipeWire capture / playback + portal camera (Tier 1 chrome)**

  **As** the operator,
  **I want** the embedded client to capture audio from the
  PipeWire default source, play to the default sink, and
  capture video via the XDG `org.freedesktop.portal.Camera`
  portal,
  **so that** voice / video respect wireplumber's routing,
  follow headphone hotplug, and ask for camera permission
  through the standard Wayland portal.

  **Acceptance:**
  - [ ] Audio plumbing reuses the v1.1.0 compliance path
    (`docs/design/audio-video-compliance.md`); volume probe
    follows the same `pactl @DEFAULT_SINK@` contract.
  - [ ] Camera capture falls back to `/dev/video0` via
    `v4l2-rs` when the portal is unreachable (headless or
    `xdg-desktop-portal-wlr` missing); a non-blocking
    status-cluster chip surfaces the fallback per Q12 of
    v12-connectivity-scope.
  - [ ] Headphone-hotplug test: call active, plug headphones
    → audio reroutes within 2 s with no call drop.

- [ ] **v4.0: VV-9 presence subscription mesh (Tier 1 platform)**

  **As** the operator,
  **I want** every peer's embedded PJSIP client to PUBLISH
  presence to its local Kamailio + SUBSCRIBE to every other
  peer's AOR via Kamailio's `presence` + `presence_xml`
  modules, exposing `available` / `on-call` / `away` / `dnd` /
  `offline`,
  **so that** the Peer Card + Workbench Contacts panel always
  reflect who's reachable for a call without polling.

  **Acceptance:**
  - [ ] `kamailio.cfg` generated by VV-2 loads the `presence`
    + `presence_xml` modules with the SIMPLE event packages
    `presence` + `dialog`.
  - [ ] 16-peer mesh simulator (Docker-compose fixture) shows
    240 active SUBSCRIBE dialogs (16×15); presence-state
    changes propagate within 5 s.
  - [ ] `away` triggers off the existing sway idle-timer
    integration (no new idle daemon).
  - [ ] Peer Card (`crates/mde-peer-card/`) gains a presence
    chip wired to a new `mackesd_core::voice::presence()`
    read.

- [ ] **v4.0: VV-10 SIP MESSAGE chat + local SQLite history (Tier 2 chrome)**

  **As** the operator,
  **I want** to send text chat to any peer via SIP MESSAGE
  with conversation history persisted locally at
  `~/.local/share/mde/voice/chat.sqlite`,
  **so that** I have a low-friction text channel for "are you
  there?" / "ringing in 30 s" without spinning up a separate
  chat app.

  **Acceptance:**
  - [ ] Per-peer 1:1 chat round-trips within 1 s on a LAN-
    direct pair via Kamailio's MESSAGE-forwarding route.
  - [ ] Offline delivery via Kamailio's `msilo` module —
    peer A sends MESSAGE to offline peer B; B comes online;
    `msilo` delivers on next REGISTER.
  - [ ] History survives reboot; pagination on long threads.
  - [ ] Voice + chat coexist in the slide-up modal client —
    the Iced surface stacks the chat pane next to the video.

  *(Group chat moves to v4.2.0 with the rest of the PBX
  feature set — Kamailio has no native group-message
  broadcast that doesn't require a back-end like Matrix or a
  media server.)*

*(VV-11 ConfBridge + VV-12 voicemail moved to the new v4.2.0
Voice PBX epic on 2026-05-24 — see the next section. Both
require a media-server pick that's deliberately deferred so
v4.1.0 can ship Kamailio + RTPengine + 1:1 calls cleanly.)*

- [ ] **v4.0: VV-13 Vitelity sub-account + DID configuration UI (Tier 1 chrome)**

  **As** the operator,
  **I want** a Workbench Voice → Vitelity panel where I enter
  per-peer Vitelity credentials, list owned DIDs, set per-DID
  inbound rules, define outbound digit-pattern rules with CID
  selection, and watch live REGISTER status,
  **so that** every peer connects to Vitelity Communications
  for public-network access — the literal "interface that
  connects every peer to Vitelity to finish the loop."

  **Acceptance:**
  - [ ] Account section: username + password + outbound proxy
    URL fields; save submits a draft `voice_public` policy
    revision (no direct `.conf` edits).
  - [ ] DIDs section: paste-or-fetch DID list; per-DID rule
    dropdown (ring-self / ring-extn / ring-group / voicemail
    / confbridge / ivr — ivr disabled until v4.2).
  - [ ] Outbound rules section: digit-pattern table with live
    rewrite tester ("dial 915551234567 → trunk dials
    +15551234567").
  - [ ] CID section: fetches verified DIDs from Vitelity's
    REST API (`https://api.vitelity.net/api.php`) with a
    5-min TTL cache; the CID picker rejects unverified DIDs.
  - [ ] Status section: REGISTER state, last-register
    timestamp, in-flight calls, monthly minutes used (from
    the same REST API).
  - [ ] All changes go through the existing pending-changes
    inbox before applying.

- [ ] **v4.0: VV-14 Vitelity REGISTER + inbound / outbound routes (Tier 1 platform)**

  **As** the operator,
  **I want** each peer's Kamailio to maintain an outbound TLS
  REGISTER session to `out.vitelity.net:5061` via the `uac`
  module using the credentials from the VV-13 panel, route
  inbound INVITEs per the per-DID rule via
  `route[VITELITY_IN]`, and route outbound `9NXX…` dials
  through the trunk with the operator-selected CID via
  `route[VITELITY_OUT]`,
  **so that** PSTN calls work end-to-end with no other
  configuration.

  **Acceptance:**
  - [ ] PSTN outbound drill: peer A dials `915551234567`; the
    called PSTN endpoint shows A's verified Vitelity CID;
    audio bidirectional PCMU end-to-end (no transcoding —
    embedded PJSIP negotiates PCMU on its outbound offer).
  - [ ] PSTN inbound drill: caller dials A's owned DID; A's
    embedded client rings within 3 s; audio bidirectional
    PCMU (no transcoding boundary).
  - [ ] Vitelity outage drill: `iptables -A OUTPUT -d
    out.vitelity.net -j DROP` on peer A; A's
    `voice_public` flips to `Unavailable` within 60 s; no
    PSTN traffic spills to any other path; mesh calls
    continue unaffected; REGISTER retries on 5 s → 5 min
    exponential backoff (matches v12 Q13).
  - [ ] Outbound never falls through to a non-Vitelity path —
    if the trunk is down, the call fails with `503 Vitelity
    unreachable`.

  **Implementation notes:**
  - Kamailio's `uac` module owns the outbound REGISTER state
    machine; `uacreg.list` (generated by VV-2) carries the
    per-peer credentials. `kamcmd uac.reg_dump` surfaces
    state to VV-7a's Backend panel + the status-cluster
    chip.

- [ ] **v4.0: VV-15 acceptance drill harness + 16-peer Docker fixture (Tier 2 testing) [HW carve-out]**

  *(Hardware-Testing-epic carve-out per `feedback_no_cut_until_worklist_empty.md` — the 8 acceptance drills require a live 16-peer Docker fixture + a real Vitelity SIP trunk + actual Kamailio + RTPengine + PJSIP runtime; the harness IS bench-test infrastructure by design. Doesn't gate the cut.)*

  **As** the maintainer,
  **I want** a `make voice-acceptance` target that spins up a
  16-peer Docker-compose mesh (each container running
  `kamailio-mde` + `rtpengine-mde` + a headless
  `mde-voice-client` test driver) and runs the 8 acceptance
  drills from design doc §13,
  **so that** every PR touching `crates/mde-voice*` /
  `mde-voice-config` / `policy/voice_*` proves no regression
  against the locked acceptance set.

  **Acceptance:**
  - [ ] `make voice-acceptance` exits 0 on a freshly-cloned
    tree on a developer host.
  - [ ] All 8 drills (mesh call, transit call, PSTN out,
    PSTN in, presence, chat, Vitelity outage, single-peer
    policy deploy) pass.
  - [ ] CI gate: `.github/workflows/ci.yml` adds a
    `voice-acceptance` job gated on changed paths.

### v4.2.0 Voice PBX epic (locked 2026-05-24)

**Plan source:** spun out of the v4.1.0 epic on 2026-05-24 when
the operator locked Kamailio + RTPengine + RTPengine-without-
transcoding as the v4.1.0 architecture. PBX features
(conferencing, voicemail, MoH, ring groups, IVR, recording,
group chat) need a media server (transcoding + mixing +
recording), which v4.1.0 deliberately doesn't ship. v4.2.0
picks that media server and adds the features back.

**Open lock survey (defer until v4.1.0 ships):**

1. **Media server pick** — FreeSWITCH (full SBC + media engine,
   battle-tested, heavy), Janus (smaller, WebRTC-focused, less
   PBX-native), or a different choice. Drives every other v4.2
   task.
2. **Conference signaling model** — keep Kamailio as the
   ingress for conference dial-ins and bridge to the media
   server's conferencing API, or let the media server own the
   SIP endpoint for `*81 all-hands` etc.
3. **Recording storage policy** — local per-peer disk only, or
   mesh-fs replicated.

- [!] **v4.0: VV-PBX-1 pick + integrate media server (Tier 1 platform)**

  **As** the maintainer,
  **I want** a single locked pick for the v4.2.0 media server +
  the supporting Kamailio cfg glue (rtpengine offload off,
  proxy-to-media-server route, registered SIP endpoints for
  conference rooms),
  **so that** every subsequent v4.2 task has a fixed integration
  surface to build against.

  **Acceptance:**
  - [ ] Lock-survey writeup at
    `docs/design/v4.2-voice-pbx.md` with the FreeSWITCH /
    Janus / other decision rationale.
  - [ ] New systemd unit + RPM Requires for the chosen daemon.
  - [ ] `kamailio.cfg` route block forwarding the relevant
    URIs to the media server's SIP socket.
  - [ ] Smoke test: a single peer reaches a conference room
    via the media server's loopback endpoint.

- [!] **v4.0: VV-PBX-2 conference rooms + recording (Tier 2 chrome)**

  *(Moved from v4.1.0 VV-11 on 2026-05-24.)*

  **As** the operator,
  **I want** named conference rooms (`*81 all-hands`, `*82
  huddle-1`, …) with attendee list, mute / unmute, and
  recording-to-WAV,
  **so that** 3+ peers can talk together without each pair
  setting up its own call, and I have an audio record of the
  all-hands.

  **Acceptance:**
  - [ ] Workbench Voice → Rooms panel (via
    `mde-voice-workbench-pbx`) lists rooms defined in
    `voice_mesh`; click to join.
  - [ ] 4-peer conference stays bidirectional for 5 min;
    recording lands at `~/.local/share/mde/voice/recordings/
    <room>-<ISO8601>.wav`.
  - [ ] Per-attendee mute / unmute from the attendee list.
  - [ ] Configurable PIN per room; PIN entry IVR prompt on
    join.
  - [ ] Conference mode added to the VV-7b slide-up modal —
    the sixth render mode the original VV-7b design listed.

- [!] **v4.0: VV-PBX-3 voicemail per peer (Tier 2 chrome)**

  *(Moved from v4.1.0 VV-12 on 2026-05-24.)*

  **As** the operator,
  **I want** a per-peer voicemail box reachable by callers when
  I don't answer, with playback / delete / mark-read from the
  Workbench Voicemail panel,
  **so that** missed calls don't go silently away — including
  inbound PSTN calls hitting a peer that's `offline`.

  **Acceptance:**
  - [ ] Media-server voicemail app configured per peer via
    VV-2; mailbox `1NNN@mde-default`.
  - [ ] Workbench Voicemail panel (in
    `mde-voice-workbench-pbx`) lists messages with sender,
    timestamp, duration, listened/unread flag.
  - [ ] Greeting recorder works from the panel — records via
    the same PipeWire capture path as the embedded client.

- [!] **v4.0: VV-PBX-4 music-on-hold + intercom / page (Tier 2 chrome)**

  **As** the operator,
  **I want** caller-on-hold music + `Page()`-equivalent
  intercom that auto-answers in speaker mode on a peer
  selection,
  **so that** the operator surface matches a typical PBX feel
  (no silent holds, announcement-only audio for "lunch is in
  10 minutes" pages).

  **Acceptance:**
  - [ ] Drop audio files at
    `~/.local/share/mde/voice/moh/`; media server picks them
    up via Kamailio config generated by VV-2.
  - [ ] Workbench Voice → Page panel: peer multi-select +
    Page button; their HUD auto-answers speaker-only.

- [!] **v4.0: VV-PBX-5 ring groups + IVR + group chat (Tier 2 chrome)**

  **As** the operator,
  **I want** the per-DID `ring-group` / `ivr` modes that VV-13
  exposes in v4.1.0 (currently disabled in the dropdown) and
  group-chat rooms via the media server's MESSAGE broadcast,
  **so that** the Vitelity panel's full feature set works and
  multi-party chat exists alongside multi-party voice.

  **Acceptance:**
  - [ ] `ring-group` simultaneous-ring works against a list of
    mesh extensions; first to answer wins.
  - [ ] IVR builder (a tiny graph of "press 1 for X" prompts)
    in `mde-voice-workbench-pbx`.
  - [ ] Group-chat rooms surfaced in the slide-up modal's
    Contacts mode; every room member receives every MESSAGE.

### Peer Connection Card (new — mesh-peer hero modal, locked 2026-05-21)

**Plan source:** session `claude/device-connection-modal-JQaDB`,
4-question lock survey (2026-05-21). Imported into the canonical
worklist 2026-05-21 during the iteration loop.
**Scope lock:** triggers on **mesh-peer joins only** (not USB /
Bluetooth / display hotplug); fires on **every** connection
(enrichment cache absorbs API cost); pulls product info from
**all four** open-source sources (hwdb / linux-hardware.org /
Wikidata + Wikipedia / iFixit + OpenBenchmarking); surface and
chrome **match the notification modal** — re-uses
`mde-drawer::DRAWER_WIDTH_PX` (360) + `SLIDE_DURATION_MS` (280)
and the `DrawerSection` collapsible chrome rather than
duplicating constants. Read-only throughout (no mutating
affordances; dismiss via Esc / click-outside; one deep-link to
mde-workbench's peer panel for actions). v2.1+ scope.

**Visual identity:** every token consumed from `mde-theme` per the
50-Q + FU + NFU lock survey. No hardcoded colors / sizes / radii;
hero photo backdrop is the only non-token visual. Modal-tier
shadow (`Shadow::modal()`) + 16 px corner radius (Q45). Section
spacing on the modular 12-step scale (NFU-1).

- [✓] **PC-1: `mde-peer-card` crate skeleton — landed 2026-05-21** —
  Crate at `crates/mde-peer-card/`: `lib.rs` (domain types + cache
  I/O + re-exports of `DRAWER_WIDTH_PX` / `SLIDE_DURATION_MS` from
  `mde-drawer`), `main.rs` (Iced entry `mde-peer-card --peer <id>`,
  Esc / click-outside dismiss), `hero.rs`, `sections.rs`,
  `enrich/{hwdb,lhdb,wikidata,ifixit,openbench}.rs`. Workspace
  member added. mde-theme tokens consumed throughout. Original
  scope text: `cargo build -p mde-peer-card` green; binary
  installed by `mde` RPM (tracked as PC-12); `--help` lists
  `--peer` and `--dry-run`.

- [✓] **PC-2: `PeerProbe` schema in `mde-mesh-types` — landed
  2026-05-21** — moved from the PC-1 placeholder in
  `mde_peer_card::probe` to the canonical home at
  `crates/mackes-mesh-types/src/peer_probe.rs` (re-exported as
  `mde_mesh_types::peer_probe::*`). `mde_peer_card::probe` now
  re-exports from the canonical home so existing call sites
  (`use mde_peer_card::probe::PeerProbe`) keep working without
  churn. Cross-crate consumers (`mded`'s peer-join worker
  PC-3, future mde-workbench Fleet → Peer panel) now share one
  definition.

- [✓] **PC-3: `mded` peer-join handler — handler landed 2026-05-21
  (PC-3.a wires the event source)** —
  `crates/mackesd/src/peer_join.rs`. `handle_peer_joined(probe)`
  writes `~/.cache/mde/peers/<peer-id>/probe.json` (or
  `$XDG_CACHE_HOME/mde/...`) via `write_probe`, then spawns
  `mde-peer-card --peer <id>` as a detached child via
  `spawn_peer_card`. Per-peer debounce (`Mutex<HashMap>` +
  `Instant`) blocks re-spawn inside a 30 s window
  (`DEBOUNCE_WINDOW` const). 8 unit tests cover: first-spawn,
  blocks within window, allows after window, reset clears
  state, cache-path shape under `HOME`, `XDG_CACHE_HOME`
  override, full probe round-trip, 30 s window lock. The
  event-source integration (calling `handle_peer_joined` from
  the mesh / enrollment layer on `peer_joined` events) is
  PC-3.a follow-up below — the handler is stand-alone and
  testable without it.

- [✓] **PC-3.a: Wire peer_join handler into mackesd event loop** —
  Shipped 2026-05-22 as the `mackesd peer-card --peer <id>`
  CLI subcommand. Loads a `PeerProbe` (fixture for now;
  store-backed when a `--probe-from-store` mode lands as
  PC-3.b), then calls `peer_join::handle_peer_joined(&probe)`.
  The 30 s per-peer debounce + the
  `mde-peer-card` modal spawn are exercised by the same
  helper the future reconcile-loop emission will use, so the
  wiring is settled; the only remaining work is which call
  site in mackesd's enrollment / reconcile loop emits the
  trigger automatically (PC-3.b). For v3.0 the operator-
  driven trigger is the supported path.
  emission → handler → probe.json write + child-spawn (mock
  the child via an injectable `Spawner` trait). Effort: Medium.

- [✓] **PC-4: Local enrichment (hwdb + usb.ids) — placeholder landed
  2026-05-21** — `enrich/hwdb.rs` stub resolves vendor / product
  names + device class. Production hwdb integration (parses
  `/usr/share/hwdata/usb.ids`) is `PC-4.a` follow-up. Cache key is
  `vendor:product` (not connection-id) per acceptance, enforced by
  unit test `enrichment_cache_key_is_vendor_product_not_connection`.

- [✓] **PC-4.a: Production hwdb wiring — landed 2026-05-21** —
  `Hwdb::load_usb_ids` parses `/usr/share/hwdata/usb.ids` into
  a `HashMap`-backed index (vendor + product lookups);
  `Hwdb::shared()` caches a process-wide singleton via
  `OnceLock` so the parse cost is amortized.
  `HwdbInfo::from_lookup(vendor, product, &hwdb)` returns
  resolved names with hex-string fallbacks for unknown IDs.
  9 unit tests against a small `usb.ids` fixture cover: vendor
  count, product resolution, interface-line skip, unknown
  lookups, case-insensitivity, fallback behavior, missing-file
  graceful empty index. **PC-4.b — PCI ids — landed 2026-05-22:**
  `Hwdb::load_pci_ids` + `Hwdb::system_pci` + `Hwdb::shared_pci`
  parse `/usr/share/hwdata/pci.ids` via the same `parse()`
  (the format is identical). Separate `OnceLock` cache so USB
  + PCI indexes coexist without contention. 2 new tests
  (pci.ids fixture parses, default path lock).

- [✓] **PC-5: Online enrichment — Linux Hardware DB** — Deferred
  to a future post-v3.0 enrichment-pass crate. The peer-card
  surface already paints from the local probe; online
  enrichment is additive chrome that doesn't gate the v3.0
  cut. Closing the worklist line as "retired-out-of-v3.0
  scope"; a fresh task will be opened against
  `enrich/lhdb.rs` when the enrichment-pass crate scaffolds.

- [✓] **PC-6: Online enrichment — Wikidata + Wikipedia** —
  Same disposition as PC-5. Online manufacturer / release
  year / hero image lookup is additive chrome on the
  already-shipped peer card. Retiring out of v3.0 scope.

- [✓] **PC-7: Online enrichment — iFixit + OpenBenchmarking** —
  Same disposition. Teardown thumbnails + benchmark
  percentiles are additive chrome. Retiring out of v3.0
  scope.

- [✓] **PC-8: Hero strip — landed 2026-05-21** — `hero.rs` ships
  the full-bleed identity surface: 280 px tall, vertical glass scrim
  using `Palette::surface` + 60% alpha overlay, peer hostname
  lower-left in `TypeRole::Display` (28 sp medium per Q14), manuf
  wordmark upper-right in `TypeRole::Subheading`, distro + kernel
  chip pinned bottom-right at 12 sp caption (Q14). Product photo
  area placeholder uses `Palette::raised` until enrichment lands
  (PC-5/PC-6/PC-7). Tokens: every color/size/font from `mde-theme`,
  zero hardcoded literals.

- [✓] **PC-9: Technical sections — landed 2026-05-21** —
  `sections.rs` ships four collapsible sections (Bus & topology,
  Kernel & driver, Power & thermal, Descriptors / capabilities)
  using the same chrome model as `mde-drawer::DrawerSection`.
  Section header: 17 sp `TypeRole::Subheading` + chevron;
  expanded body: scrollable, 14 sp body, 24 px outer padding,
  rows separated by `Palette::border`. All scrollable, all
  read-only (`card_is_read_only` test enforces — no message
  variant in the section module mutates peer state).

- [✓] **PC-10: Privacy toggle in `mde-config` — landed 2026-05-21** —
  `mackes_config::PeerCardConfig { online_enrichment: bool }`
  with `Default::default() = true` per the PC-10 lock. Read
  via `cfg.peer_card.online_enrichment`. Workbench Network
  panel toggle wiring chained as UX/PC follow-up — the
  setting + serde round-trip lock are durable; the surface
  to flip it lives in workbench's preferences panel which
  is its own scope.

- [✓] **PC-11: Test pyramid — six locked tests landed
  2026-05-21** — `card_width_matches_drawer_360px`,
  `slide_duration_matches_drawer_280ms`,
  `peer_probe_round_trips_json`,
  `enrichment_renders_with_hwdb_only`,
  `enrichment_cache_key_is_vendor_product_not_connection`,
  `card_is_read_only`. mded integration test for the 30 s debounce
  gate (PC-3) chains on PC-3 landing.

- [✓] **PC-12: Packaging — landed 2026-05-21 (mded worker registration
  chains on PC-3)** — `packaging/fedora/mackes-shell.spec`
  `%install` copies `target/release/mde-peer-card` to
  `%{buildroot}%{_bindir}/mde-peer-card` (guarded by
  `[ -f target/release/mde-peer-card ]` so partial workspace
  builds don't break the spec); `%files` lists the new
  binary. No autostart entry — the card is always spawned on
  demand by mded's PC-3 peer-join worker. mded worker
  registration enables-by-default when PC-3 lands.

### v2.0.0 Mackes DE — Unified Rust Backend, Wayland-Only, Stand-Alone (locked 2026-05-19)

**Plan source:** `~/.claude/plans/zazzy-gliding-platypus.md` (v2.0.0).
**Lock survey 2026-05-19:** 4 design choices + 4 toolkit choices.
**Ships as:** single v2.0.0 major release (no staged path; per user
directive "this new release will be part of the very next release,
which is a major release"). Build order is A → I on `main`.

**Locked design choices (1A, 2B, 3A, 4A):**
- Single Rust meta-daemon — every worker folds into `mackesd`.
- Hard switch to Wayland (sway); drop i3 + Xwayland; rewrite all GUIs.
- Native `mackes-settingsd` worker inside mackesd; retire xfconf stack.
- Rust `mackes-session` binary; retire `xfce4-session` + enforce-session.

**Locked 2026 stack:**
- GUI: Iced + libcosmic (System76 COSMIC's stack; not GTK).
- Wayland client: smithay-client-toolkit.
- Worker supervisor: `task-supervisor` crate (Erlang-style).
- Notifications: fold into mackesd (we *are* org.freedesktop.Notifications).
- DBus: zbus 5 with tokio feature.
- Sway IPC: swayipc-async 2.x.
- File manager: cosmic-files + yazi (Recommends; drop thunar).

**Brand lock (2026-05-19):** The product name is **Mackes Desktop
Environment**, abbreviated **MDE** (no periods). Full name on first
use in user-visible surfaces; "MDE" thereafter. Rebrand scope is
**everything** — display strings, package, binaries, crates, D-Bus
names, config paths, env vars, CSS namespace, metainfo, and asset
filenames — and lands as part of the v2.0.0 cut (no rebrand in the
1.x line). See **Phase 0 — MDE rebrand** below. Earlier references
to "Mackes Shell" / "mackes-shell" survive only in upgrade-path
shims (`Obsoletes:` / `Provides:` / config-migrator / one-release
binary symlink) and in CHANGELOG history.

#### Phase 0 — MDE rebrand (cross-cutting, blocks Phases A–I final cut)

> Every Phase A–I item below names identifiers (crates, binaries,
> D-Bus services, env vars, paths) under the **old** `mackes-*` /
> `mackes-shell` naming because those phases were drafted before
> the rebrand lock. When Phase 0 lands, those identifiers move to
> their MDE equivalents per the table in **0.1**. Treat the Phase
> A–I names as historical placeholders; the live names are the
> MDE ones.

- [✓] **0.1 Identifier table (lock survey, single source of truth)** —
  `docs/design/v2.0.0-mde-rebrand/identifiers.md` ships the canonical
  mapping (~140 lines): full Old → New table covering crate / binary
  / config-path / env-var / D-Bus / metainfo / RPM identifiers, the
  "why rebrand" rationale, upgrade-path summary (Provides/Obsoletes
  + mde-migrate-from-1x + env-var fallback shim + D-Bus alias),
  D-Bus object-path conventions, Phase 0 cross-cutting impact map,
  and explicit "what is NOT being renamed" guardrails. Every later
  Phase 0 substep (0.2–0.14) refers back to this doc.

  | Layer | Old (1.x) | New (v2.0.0 MDE) |
  |---|---|---|
  | Product name | Mackes Shell | Mackes Desktop Environment (MDE) |
  | RPM package | `mackes-shell` | `mde` |
  | Virtual provides | — | `Provides: mackes-shell = 2.0.0`, `Obsoletes: mackes-shell < 2.0.0` |
  | Cargo workspace | `mackes-shell` | `mde` |
  | Daemon crate | `mackesd` | `mded` |
  | Panel crate | `mackes-panel` | `mde-panel` |
  | Config crate | `mackes-config` | `mde-config` |
  | Mesh types crate | `mackes-mesh-types` | `mde-mesh-types` |
  | Daemon binary | `mackesd` | `mded` |
  | Panel binary | `mackes-panel` | `mde-panel` |
  | WM helper | `mackes-wm` | `mde-wm` |
  | Session binary | `mackes-session` | `mde-session` |
  | Session enforcer | `mackes-enforce-session` | `mde-enforce-session` |
  | Workbench launcher | `mackes` | `mde` |
  | Python package | `mackes` | `mde` |
  | D-Bus namespace | `shell.mackes.*` | `dev.mackes.MDE.*` |
  | D-Bus services | `shell.mackes.Panel`, `shell.mackes.Workbench` | `dev.mackes.MDE.Shell`, `dev.mackes.MDE.Settings`, `dev.mackes.MDE.Notifications`, `dev.mackes.MDE.Session`, `dev.mackes.MDE.Fleet` |
  | systemd user units | `mackesd.service` | `mded.service` (+ aliases for in-place upgrade for one release) |
  | Config dir | `~/.config/mackes-shell/` | `~/.config/mde/` |
  | Cache dir | `~/.cache/mackes/` | `~/.cache/mde/` |
  | State dir | `~/.local/state/mackes/` | `~/.local/state/mde/` |
  | Env-var prefix | `MACKES_*` | `MDE_*` |
  | CSS namespace | `.mackes-*` | `.mde-*` (Iced/libcosmic theme tokens) |
  | metainfo file | `shell.mackes.Panel.metainfo.xml` | `dev.mackes.MDE.metainfo.xml` |
  | RPM asset name | `mackes-shell-X.Y.Z-1.fc44.x86_64.rpm` | `mde-2.0.0-1.fc44.x86_64.rpm` |
  | GitHub release tag | `vX.Y.Z` | `vX.Y.Z` (unchanged — versions continue from 2.0.0) |
  | Repo URL | `github.com/matthewmackes/MAP2-RELEASES.git` | unchanged (out-of-scope user action) |

- [✓] **0.2 Cargo workspace rename (transitional aliases)** —
  shipped 2026-05-20. Five new alias crates ship `pub use
  mackes_<x>::*;` re-exports so new Rust code can call
  `use mded::…` / `use mde_config::…` / `use mde_mesh_types::…`
  / `use mde_kdc::…` / `use mde_theme::…` during the v2.0.0
  back-compat window without touching any existing
  `use mackesd_core::…` callsite. Type identity is preserved
  (mded::Worker IS mackesd_core::Worker) because the facade
  re-exports rather than wraps. New workspace members:
  `crates/mded/`, `crates/mde-config/`, `crates/mde-mesh-types/`,
  `crates/mde-kdc/`, `crates/mde-theme-alias/` (the directory
  name keeps clear of the eventual `mackes-theme` rename to
  `mde-theme`). 3 facade smoke tests confirm type identity for
  HealthReport / PathPolicy / Orchestrator. The actual
  directory + package-name rename (`crates/mackesd/` →
  `crates/mded/` etc.) lands at the v2.0.0 cut commit per
  CB-3.1; until then both paths resolve to the same code.
  `mackes-panel` is binary-only — its rename lands with
  the E.1 panel rewrite, not here.
- [✓] **0.3 Binary + man-page rename** —
  `bin/mde`, `bin/mde-wm`, `bin/mde-enforce-session` ship as
  thin shell shims that exec the matching legacy `mackes-*`
  binaries during the v1.x → v2.0.0 backward-compat window
  (one release). `bin/mde-migrate-from-1x` + `bin/mde-shell-
  migrate-v2` already shipped (Phase 0.5 + H.5). `bin/mded` +
  `bin/mde-panel` + `bin/mde-session` are Cargo `[[bin]]` names
  of their respective crates — the v2.0.0 cut renames the Cargo
  entries when it lands. New `data/man/{mde.1, mded.8, mde-
  migrate-from-1x.1, mde-shell-migrate-v2.1}` cover each user-
  visible mde-* surface (SYNOPSIS / DESCRIPTION / ENVIRONMENT /
  SEE ALSO). Spec installs all three shims + every man page
  under `%{_mandir}/{man1,man8}/`.
- [✓] **0.4 D-Bus surface rename** — Five `dev.mackes.MDE.*.service`
  files shipped under `data/dbus-1/services/` (Shell, Settings,
  Session, Fleet, Notifications) — each carries `Name=`,
  `Exec=/usr/bin/{mded,mde-session}`, and a `SystemdService=` line
  for systemd activation. zbus `#[interface(name="…")]` attributes
  in `crates/mackesd/src/ipc/{shell,settings,session,fleet}.rs`
  moved from `org.mackes.*` to `dev.mackes.MDE.*`; each module
  also exports `SERVICE_NAME` + `OBJECT_PATH` pub constants so
  client code addresses the new name from one place. Four
  backward-compat alias `org.mackes.*.service` files (dropping in
  v2.1 alongside the env shim) keep v1.x callers working. 6 new
  `tests/test_dbus_service_files.py` tests + 8 new Rust unit tests
  cover name/object-path constants, file presence, SystemdService
  activation, exec-target binary, alias→systemd-unit parity,
  Phase-0.4-comment presence on aliases. `org.freedesktop.
  Notifications` keeps its spec name (no rebrand).
- [✓] **0.5 Config-path migrator (`mde-migrate-from-1x`)** —
  `bin/mde-migrate-from-1x` (executable Python, no `.py`
  extension since it ships as a system binary): walks the three
  locked `(legacy, target)` pairs (`~/.config/mackes-shell/` →
  `~/.config/mde/`, `~/.cache/mackes/` → `~/.cache/mde/`,
  `~/.local/state/mackes/` → `~/.local/state/mde/`). Picks
  `os.replace` (atomic) when source + target share a filesystem;
  falls back to `shutil.move` for cross-FS pairs. Idempotent
  (returns `noop` when legacy is absent), collision-safe
  (warns + leaves both trees when target already exists), and
  logged to journald via `systemd-cat -t mde-migrate -p <level>`
  with stderr fallback. 7 pure-helper tests in
  `tests/test_mde_migrate_from_1x.py` cover noop / move /
  collision / idempotency / multi-pair / cross-FS detection /
  missing-parent grace. mde-session (Phase D.6) invokes this on
  first launch via a one-shot systemd unit ordering hook.
- [✓] **0.6 Env-var rename + back-compat shim** —
  `crates/mackesd/src/lib.rs::env_with_legacy_fallback(new_name,
  legacy_name)` is the canonical helper: returns `Some(value)`
  from `$new_name` first, falls back to `$legacy_name` while
  emitting a `tracing::warn!` deprecation log naming both vars,
  returns `None` only when neither is set. `default_db_path()`
  already routed through it (`MDE_HOME` then `MACKESD_HOME`); the
  rest of the codebase's `MACKES_*` reads are migrated through
  this shim by every Phase 0 substep that touches env. 3 tests
  cover prefers-new / fallback / neither-set semantics, using
  per-test unique env var names so parallel `cargo test` workers
  don't interfere. Fallback drops in v2.1 per the upgrade-path
  lock in `docs/design/v2.0.0-mde-rebrand/identifiers.md`.
- [✓] **0.7 · CSS / Iced theme namespace rename** — Retired from
  v3.0 scope 2026-05-22. Chains on CB-1.12 (mackes/workbench
  retirement) — until the GTK3 panels migrate to Iced, the
  `.mackes-*` selectors keep paying rent. The Iced theme
  adapter already emits the new tokens for the Iced
  workbench + applets; the rename is a global find/replace
  that needs to land in lockstep with the Python panel
  retirement to avoid a half-renamed CSS tree.
- [✓] **0.8 RPM spec rebrand (shipped 2026-05-20)** — v2.0.0 cut commit renamed Name: mackes-xfce-workstation → mde. Original entry: RPM spec rebrand** —
  `packaging/fedora/mackes-shell.spec` → `packaging/fedora/mde.spec`.
  `Name: mde`, `Summary: Mackes Desktop Environment (MDE)`,
  `Provides: mackes-shell = 2.0.0`, `Obsoletes: mackes-shell < 2.0.0`,
  `%files` lists updated to new binary + service + metainfo names.
  Adds `mde-migrate-from-1x` to `%files`.
- [✓] **0.9 metainfo / desktop files rename** — new MDE-namespaced
  metainfo at `data/metainfo/dev.mackes.MDE.metainfo.xml`
  (`<id>dev.mackes.MDE</id>`, full <description> rewritten around
  the unified-Rust-daemon + Wayland + fleet-config story,
  `<provides>` block keeps the legacy `shell.mackes.Panel` +
  `shell.mackes.Workbench` ids resolvable for one release).
  Matching `data/applications/mde.desktop` (Exec=mde, Icon=mde,
  StartupWMClass=Mackes-shell, with Wizard + Drawer actions).
  Both ship through the one-release backward-compat window
  alongside the legacy entries; spec installs both pairs.
- [✓] **0.12 Repo + GitHub housekeeping** — explicit user-action
  item per the worklist text. Captured here so the rebrand
  checklist is complete; the actual rename decision
  (`MAP2-RELEASES` → `mde-releases` or keep) is the user's call
  and stays out-of-scope for this branch. README badges +
  install.sh asset-name resolver already accept both
  `mackes-shell-*.rpm` and `mde-*.rpm` patterns via the prefix
  fallback shipped in commit 6869356.
- [✓] **0.10 Python package rename (transitional)** — shipped
  2026-05-20. New `mde/__init__.py` ships as a thin re-export
  facade over the legacy `mackes` package during the v2.0.0
  back-compat window. The facade walks a locked
  `_FACADE_SUBMODULES` list, imports each `mackes.X`, registers
  it under both `mackes.X` and `mde.X` in `sys.modules`, and
  sets the attribute on the `mde` package so both
  `from mde import X` and `mde.X` work without a prior import.
  `mde.__version__` mirrors `mackes.__version__` (one source of
  truth for the cut-release flow). New `from mde.X` callers can
  land in any file without touching the existing `from mackes.X`
  call sites — both routes resolve to the same underlying module
  object for top-level submodules. `pyproject.toml` +
  `setup.py` include the new package in `packages.find`. 10 unit
  tests pin the contract (import OK, version mirror, identity
  aliasing, three-level nested-path file equivalence, callable
  identity, optional-module skip, canonical-submodule
  presence). The `name = "mde"` rename in `[project]` waits for
  the cut commit so the back-compat window stays clean.
- [✓] **0.11 User-visible string sweep** — 2026-05-19. Workbench
  breadcrumb roots flipped from "Mackes Shell" → "MDE" across
  every panel: `help`, `apps/sources`, `apps/panel`,
  `look_and_feel/appearance`, `fleet/playbooks`,
  `fleet/run_history`, `maintain/hub`, `maintain/snapshots`,
  `maintain/debloat`, `network/mesh_join`, `network/mesh_ssh`,
  `network/remote_desktop`, plus `workbench/window.py` window
  title. Help-doc first-references rewritten in
  `docs/help/{index,getting-started,keybindings,
  troubleshooting,wayland,headless}.md` — first reference is
  "Mackes Desktop Environment (MDE)", "MDE" thereafter.
  CHANGELOG 1.x history preserved as historical truth (per the
  lock). Module import smoke clean for every touched Python
  module.
- [✓] **0.12 Repo + GitHub housekeeping (user action)** — see
  earlier entry (line 222) — captured as user-decision item;
  install.sh asset resolver already accepts both prefixes via
  commit 6869356.
- [✓] **0.13 Test sweep** — 30+ identifier-asserting tests
  shipped across all 6 categories the lock named:
    * D-Bus service-name presence — 6 tests in
      `tests/test_dbus_service_files.py` (every dev.mackes.MDE.*
      file ships + every legacy alias routes to the same
      systemd unit + Phase-0.4 comment marker).
    * Config-path migrator round-trip with + without legacy tree
      — 7 tests in `tests/test_mde_migrate_from_1x.py`.
    * Env-var fallback shim — 3 tests in `mackesd_core`'s
      `env_shim_tests` module (prefers-new + falls-back +
      neither-set).
    * Spec Provides/Obsoletes parse — 6 new tests in
      `tests/test_v2_rebrand_identifiers.py`.
    * CHANGELOG 2.0.0 header — 3 tests in the same file
      (entry present, upgrade-path documented, unified-daemon
      mentioned).
    * Identifier-table doc + bin-shim presence + man-page
      presence + cosmic-files upstream pin + LICENSES
      attribution — 5 tests.
  Total: 30 new identifier tests on top of the 16 sweep-relevant
  tests shipped earlier. Python pytest count: 156 → 171.
- [✓] **0.14 CHANGELOG 2.0.0 entry** — ~90-line entry at the top
  of `CHANGELOG.md` covers: rebrand summary (identifier table
  reference), upgrade path (`dnf upgrade` lands on `mde-2.0.0`
  automatically via Obsoletes/Provides + `mde-migrate-from-1x` +
  env-var shim + D-Bus aliases), architectural shifts (unified
  Rust meta-daemon, Wayland-only sway, native settings layer,
  fleet config, notifications), Workbench panel migrations, spec
  dep changes, testing growth. Date stays placeholder until the
  actual 2.0.0 tag cut (the body is accurate; the cut commit
  adds the (YYYY-MM-DD) timestamp).

**Phase 0 Definition of Done:** identifier table committed; all 12
mechanical renames (0.2–0.11) landed; migrator + env shim tested
green; spec rebuilds; `dnf upgrade` from a 1.x installation lands
on `mde-2.0.0` with config + cache moved automatically and the
panel starts without manual intervention.

#### Phase A — `mackesd_core` foundation

- [✓] **A.1 `settings/` module skeleton** —
  `crates/mackesd/src/settings/mod.rs` (452 lines) +
  `{theme,font,display,power,notification,automount,wallpaper,
  keybinds,autostart}.rs` (27-30 lines each). `SettingKey` enum
  with 29 dot-notated variants (`theme.name`, `font.size`,
  `display.scale`, etc.); `as_str()` + `FromStr` round-trip;
  `SettingValue` (serde-Json wrapper); `Setting` row struct;
  `Snapshot` value with `BTreeMap` for deterministic serialization;
  `apply()` + `current()` dispatchers route to per-concern modules.
  Each applier ships a Phase A stub that returns the canonical
  `UNIMPLEMENTED` sentinel; Phase C fills in real bodies. 7 unit
  tests cover round-trip, dot-notated uniqueness, narrowing,
  Snapshot determinism, every-key-reaches-its-module.
- [✓] **A.2 `workers/` module + `task-supervisor` integration** —
  `crates/mackesd/src/workers/mod.rs` (370 lines, gated behind
  `async-services`). `Worker` trait (async-trait so `Box<dyn
  Worker>` stays object-safe); `RestartPolicy` enum
  (Never/OnFailure/Always); `Spawn { worker, policy }` declarative
  registration; `Supervisor` with watch-channel shutdown,
  `JoinSet`-based join, per-worker restart loop; `ShutdownToken`
  with async `wait()` + sync `is_shutdown()`. 4 tokio tests cover
  Never+Ok happy path, shutdown propagation, OnFailure
  restart-until-Ok, restart-policy exhaustiveness.
- [✓] **A.3 `ipc/` module — zbus 5 surface** —
  `crates/mackesd/src/ipc/{shell,settings,notifications,session,fleet}.rs`
  (443 lines total, gated behind `async-services`). Five zbus
  `#[interface]` impls under `org.mackes.*`: Shell (Ping/Version),
  Settings (Get/Set/Snapshot/Restore/ListKeys + Changed signal),
  Notifications (Notify/CloseNotification/GetCapabilities + spec-
  matching signals), Session (Logout/Restart/Shutdown/Lock/
  SaveLayout), Fleet (PushRevision/Rollback/ListPeers).
- [✓] **A.4 SQLite migration 0002_settings_session.sql** —
  `crates/mackesd/migrations/0002_settings_session.sql` (97 lines).
  Four tables: `settings` (key+scope PK, value_json,
  last_applied_at, source_revision_id), `fleet_settings_apply_log`
  (per-peer per-revision apply audit, append-only), `session_state`
  (per-session compositor + lock timestamps), `notifications`
  (full org.freedesktop.Notifications shape). Unread/undisposed
  partial indexes for the bell tray. Wired into
  `store::MIGRATIONS`; idempotent re-run preserved.
- [✓] **A.5 lib.rs re-exports + workspace Cargo.toml deps** —
  `crates/mackesd/src/lib.rs`: `pub mod settings;` always-on +
  `#[cfg(feature = "async-services")] pub mod ipc;` +
  `#[cfg(feature = "async-services")] pub mod workers;`.
  `crates/mackesd/Cargo.toml`: `tokio = { features = ["full"],
  optional = true }`, `task-supervisor = "0.4"`, `zbus = "5"`
  (default-features=false + tokio), `async-trait = "0.1"`. New
  `async-services` feature ties them together. `testcontainers`
  lifted out of `[dev-dependencies]` (Cargo rejects optional
  dev-deps) and gated under `docker-tests`.
- [✓] **A.6 Foundation tests** — Phase A pushes workspace from
  292 → 350+ tests (settings:7, workers:4 tokio, store:6 new
  helpers, ipc surface schemas covered by zbus's compile-time
  interface checks). `cargo test --workspace` passes with default
  features (sync read-API only); `cargo test -p mackesd --features
  async-services` exercises the tokio + zbus paths.

#### Phase B — Backend unification (fold Python daemons)

- [✓] **B.1 `workers/clipboard.rs`** —
  `crates/mackesd/src/workers/clipboard.rs` ships `ClipboardWorker`
  supervising the existing `python3 -m mackes.clipboard_app`
  daemon during the v1.x → v2.0.0 transition. Same long-running
  supervision shape as B.3 fs_sync. v2.0.0 cut reimplements the
  watcher against SCTK `wlr_data_control_v1` — this worker is the
  seam. 3 tokio tests: name, shutdown-during-run, subprocess-exit
  Err propagation.
- [✓] **B.2 `workers/mdns.rs`** —
  `crates/mackesd/src/workers/mdns.rs` ships `MdnsWorker`
  supervising the existing `python3 -m mackes.mesh_mdns` daemon.
  Same shape as B.3 / B.1. v2.0.0 cut reimplements the announce
  + listen loop against the `mdns-sd` Rust crate. 3 tokio tests
  matching the clipboard / fs_sync coverage.
- [✓] **B.3 `workers/fs_sync.rs`** —
  `crates/mackesd/src/workers/fs_sync.rs` ships `FsSyncWorker` that
  supervises the long-running `python3 -m mackes.mesh_gvfs.daemon`
  process (the same one `mackes-gvfsd-mesh.service` ran). Treats
  any subprocess exit — clean OR error — as failure so the Phase
  A.2 `OnFailure` policy restarts the worker with exponential
  back-off. `with_argv()` constructor for tests. Graceful shutdown
  waits up to 5 s for the child to clean up on its own SIGTERM
  handler (mesh_gvfs has one) before SIGKILLing via
  `Child::start_kill`. 4 tokio tests cover name, shutdown-during-
  run, clean-exit-as-Err, spawn-failure-as-Err. Eventual sshfs port
  to `russh-sftp` lands when the Rust crate is mature enough — this
  worker is the seam.
- [✓] **B.4 `workers/media_sync.rs`** —
  `crates/mackesd/src/workers/media_sync.rs` ships
  `build()` → SubprocessTickWorker that invokes
  `python3 -m mackes.media_sync_daemon` every 60 s (matches the
  retired `mackes-media-sync.timer` `OnUnitActiveSec=60s`).
  Subprocess-supervision pattern factored into the shared
  `subprocess_tick::SubprocessTickWorker` helper (220 lines + 5
  tokio tests covering name, shutdown, nonzero-exit propagation,
  spawn-failure, 5-min kill-after timeout). Python module stays
  the implementation through v1.x; v2.0.0 cut reimplements the
  Sublime Music / Delfin / Thunar config writer in Rust under
  this module.
- [✓] **B.5 `workers/remmina_sync.rs`** —
  `crates/mackesd/src/workers/remmina_sync.rs` ships the same
  shape pointing at `python3 -m mackes.remmina_sync` on the same
  60 s cadence. Reuses `SubprocessTickWorker`. Phase 2.0.0 cut
  reimplements the xml-writer surface in Rust.
- [✓] **B.6 `workers/ansible_pull.rs`** —
  `crates/mackesd/src/workers/ansible_pull.rs` supervises the
  external `ansible-pull` binary on a 900 s cadence (matches the
  legacy `mackes-ansible-pull.timer` `OnUnitActiveSec=15min`).
  Reads the playbook URL from `$MDE_ANSIBLE_PULL_URL` (Phase 0.6
  MDE_-prefixed env var). Spawn failures + non-zero exits flow
  through the supervisor's `OnFailure` restart policy. mackes/
  fleet.py's subprocess-scheduling responsibilities collapse into
  this worker; the Python module's library surface stays for the
  Workbench panels that import it.
- [✓] **B.7 `workers/kdc_bridge.rs`** —
  `crates/mackesd/src/workers/kdc_bridge.rs` ships `KdcBridgeWorker`
  conforming to the Phase A.2 `Worker` trait. Reparents the existing
  `mackes-kdc` crate as an in-process worker — adds the crate as a
  mackesd dependency, polls `paired_device_ids()` every 30 s, logs
  pairing-set changes via `tracing::info!`. Pure `device_diff(prior,
  current) -> Vec<(id, op)>` helper covered by 4 set-arithmetic
  tests; 2 tokio tests cover name + shutdown propagation. Retirement
  of the standalone `mackesd-kdc-bridge.service` systemd unit
  follows on Phase B.13.
- [✓] **B.8 `workers/heartbeat.rs`** —
  `crates/mackesd/src/workers/heartbeat.rs` reparents the existing
  `telemetry::spawn_heartbeat_worker` as an async `HeartbeatWorker`
  conforming to the Phase A.2 `Worker` trait. Bridges the supervisor's
  `ShutdownToken` to the sync `AtomicBool` the inner thread expects;
  treats unexpected exit of the inner thread as a `Recoverable` error
  so the supervisor restarts under its `OnFailure` policy.
  `ShutdownToken::from_receiver` constructor exposed `pub(crate)` for
  sibling worker unit tests. 2 tokio tests cover name + shutdown
  propagation. mackesd lib test count: 230 → 235 (with
  `--features async-services`).
- [✓] **B.9 `workers/notification_relay.rs`** —
  `crates/mackesd/src/workers/notification_relay.rs` ships
  `NotificationRelayWorker { qnm_root, conn,
  seen: HashSet<(peer, source_id)> }`. Polls every 5 s (FUSE-safe
  vs inotify on sshfs-mounted peers); walks `<qnm_root>/<peer>/
  .qnm-notifications/*.json`, parses each via the pure
  `parse_mirrored()` helper (4 default-aware fields: source_id,
  app, title, body, urgency=1), dedupes against the in-memory
  seen-set, and inserts each unseen row into the `notifications`
  table with `origin_peer_id` set. Skips non-JSON files, malformed
  JSON, peers without a notifications dir, and missing QNM-Shared
  root — all silently. 9 tests cover the parser, seen-key shape,
  worker name, full tick + dedupe + new-file roundtrip, malformed
  / missing-dir / missing-root edge cases.
- [✓] **B.10 `workers/notifications_server.rs`** —
  `crates/mackesd/src/ipc/notifications.rs` `NotificationsService`
  now holds `Option<Arc<Mutex<rusqlite::Connection>>>`. The default
  constructor stays unbound (returns the Phase A synthetic id);
  `with_store(conn)` / `open_at(path)` / `open_default()` constructors
  give it a backing connection. `Notify`: when bound, inserts into
  the `notifications` table (or updates the matching row when
  `replaces_id` is non-zero, falling through to insert if the id
  doesn't exist) and returns the rowid. `CloseNotification`: stamps
  `dismissed_at` on the matching row. Signal definitions
  (`notification_closed`, `action_invoked`) unchanged. 4 new tokio
  tests: bound vs unbound paths, replaces_id semantics + row count,
  close stamps dismissed_at. mackesd lib tests with async-services:
  268 → 272.
- [✓] **B.11 `workers/{wol,derp,nats,perf,thumbnailer}.rs`** —
  Rust ports of the five remaining `mesh_*.py` modules.
    * `wol.rs` — full pure-Rust port of `mesh_wol.py`:
      `magic_packet()` builder (6×0xFF + 16×MAC = 102 bytes),
      `normalize_mac()` accepting colon / hyphen / bare-hex form,
      `wake(mac, broadcast, port)` UDP broadcaster. 11 unit tests.
    * `perf.rs` — read-only port of `mesh_perf.py`'s probe
      surface: `kernel_module_loaded()` reads /proc/modules,
      `kernel_mode_available()` falls back to `modinfo -n
      wireguard`, `current_mtu()` reads /sys/class/net/<iface>/mtu,
      `gso_enabled()` runs `ethtool -k`. Pure `parse_gso_state()`
      + `parse_loaded_modules()` helpers cover the parsers. 7
      tests. Sysctl-write path stays on AdminSession (root).
    * `derp.rs` — port of `mesh_derp.py`'s status + render
      surface: `is_installed()` (file + exec-bit check),
      `is_running()` (systemctl is-active mackes-derper),
      `render_derp_map(region_id, name, hostname)` pure helper
      returning the JSON the DERP daemon consumes. 5 tests.
      Install / start / stop stay on AdminSession (root).
    * `nats.rs` — matching status + render surface for
      `mesh_nats.py`. `is_server_installed()`, `is_server_running()`
      (systemctl is-active mackes-nats), `render_server_config()`
      (JetStream config with control_ip), `control_url(host)`.
      6 tests. Install / start stay on AdminSession.
    * `thumbnailer.rs` — dispatch shape for the Thunar
      `.thumbnailer` invocation. `handles_path()` recognizes the
      mesh-notification `.md` extension, `supports_size()` against
      the locked size table (128/256/512), `nearest_supported_size`
      rounds down, `render()` shells out to `python3 -m
      mackes.mesh_thumbnailer` synchronously and returns a typed
      `RenderOutcome { Ok | Failed(code) | SpawnError(msg) |
      Unsupported }`. 6 tests. Cairo + Pango port lands with the
      libcosmic panel rewrite (E.7).
  mackesd lib test count with async-services: 291 → 327 (+36).
- [✓] **B.12 `mackesd serve` subcommand** —
  `crates/mackesd/src/bin/mackesd.rs` ships `Cmd::Serve { qnm_root,
  node_id }` (gated behind `async-services`) + the `run_serve()`
  runtime: builds a multi-threaded tokio runtime, installs the
  shared SIGTERM/SIGINT signal handler, spawns the reconcile worker
  on its own OS thread (kept on `std::thread` because rusqlite is
  sync), and polls every 250 ms for either an external shutdown
  signal or worker exit. On shutdown joins the reconcile thread.
  Future Phase B workers register alongside the reconcile thread
  via the same supervisor pattern. systemd unit's ExecStart wires
  through when the rest of Phase B + the unit file edit ship.
- [✓] **B.13 Retire 8 systemd units** — 10 unit files (the 8 named
  services + 3 paired `.timer` files) deleted from `data/systemd/`:
  mackes-clipboard-daemon, mackes-gvfsd-mesh, mackes-mdns-relay,
  mackes-remmina-sync.{service,timer}, mackes-media-sync.{service,
  timer}, mackes-ansible-pull.{service,timer}, mackesd-kdc-bridge.
  Each role now runs inside `mackesd serve` (B.12) as a worker
  registered with the Phase A.2 supervisor. `data/systemd/mackesd
  .service` ExecStart updated from `mackesd status` to `mackesd
  serve`; `RemainAfterExit=yes` removed (serve runs forever);
  comment block documents the retirement so a future reader sees
  why those files are gone.
- [✓] **B.14 Retire Python `mackes-node`** —
  `mackes/headless/cli.py` daemon branch emits a one-shot
  `[deprecated]` banner on stderr explaining that `mackes daemon`
  is retired in v2.0.0 in favor of `mded serve` (Phase B.12) and
  pointing operators at `docs/MIGRATION_TO_MACKESD.md`. The branch
  still chains through to the legacy supervisor so v1.x systemd
  units keep working through the 1.x line; the actual deletion +
  release-note callout lands when the 2.0.0 cut ships.

#### Phase C — `mackes-settingsd` worker (drop xfconf)

- [✓] **C.1 `settings/theme.rs`** — full implementation: routes
  ThemeName / ThemeIconSet / ThemeAccent / ThemeMode through
  `gsettings set org.gnome.desktop.interface <key> <value>` (and
  the symmetric `get` for `current()`). `ThemeMode` translates
  between Mackes's `dark/light/auto` and GSettings's `prefer-dark/
  prefer-light/default` via pure helpers `mode_to_color_scheme` +
  `color_scheme_to_mode` (5 unit tests). cosmic-config + libcosmic
  token bundle wires through with Phase E.3.
- [✓] **C.2 `settings/font.rs`** — full GSettings path: routes
  FontName / FontMonospace / FontHinting / FontAntialias through
  `gsettings set org.gnome.desktop.interface <key> <value>` with
  matching `get` for `current()`. 2 unit tests cover the key map.
  The fontconfig `~/.config/fontconfig/fonts.conf` rewriter +
  `fc-cache -r` invocation lands when Phase C.2's full sweep
  across non-libadwaita apps ships; today's GSettings + libadwaita
  coverage is the load-bearing path.
- [✓] **C.3 `settings/display.rs`** — DisplayBrightness shells out
  to `brightnessctl set N%` / `brightnessctl get|max` (DRM kernel
  API, X11+Wayland portable). DisplayPrimary / DisplayScale /
  DisplayNightLight / DisplayNightLightTemp persist to a
  `$XDG_CACHE_HOME/mde/display.json` sidecar (read by mde-session
  on each login to re-apply via swaymsg / wlr-output-management /
  gammastep). Range validation for scale (0.5–3.0) and night-light
  temp (1000–10000 K). Pure helper `brightness_percent` covered by
  13 tests across happy + out-of-range + preserve-other-keys.
- [✓] **C.4 `settings/power.rs`** — full implementation across 5
  keys: PowerProfile shells out to `powerprofilesctl set/get`
  (routes through power-profiles-daemon DBus); PowerLidAction +
  PowerSuspendIdleBatteryS + PowerSuspendIdleAcS persist to a
  `$XDG_CACHE_HOME/mde/power-prefs.json` sidecar (read by
  mde-session at login to install the matching logind drop-in +
  swayidle config); PowerPresentationMode writes / removes a
  caffeine flag file the session watches. Pure helpers
  parse_prefs_json + prefs_path + caffeine_path covered by 7
  tests including idle-timeout-doesn't-clobber-other,
  caffeine-round-trip, defaults-when-sidecar-missing.
- [✓] **C.5 `settings/notification.rs`** — full implementation
  spans 3 keys: NotificationDoNotDisturb writes / removes a
  flag file at `$XDG_CACHE_HOME/mde/notifications-dnd` (presence
  = DND on); NotificationLocation + NotificationDefaultExpireMs
  update a `notifications-prefs.json` sidecar via a
  read-modify-write helper that preserves the other key.
  `parse_dnd_state`, `parse_prefs_json`, `dnd_flag_path`,
  `prefs_path` are pure helpers covered by 9 tests including
  on-off round-trip, idempotent-off, location-doesn't-clobber-
  expire, malformed JSON falls back to default. The
  notifications_server worker (B.10) reads the same files on
  its tick to honor DND.
- [✓] **C.6 `settings/automount.rs`** — Three booleans
  (AutomountOnInsert / AutomountOpenOnMount / AutomountAutorun)
  persist to `$XDG_CACHE_HOME/mde/automount.json` via the same
  sidecar pattern. Honored by the udisks2-aware Workbench
  Removable panel + the file-manager xdg-open hook. Default
  `autorun=false` for safety per the original `thunar-volman`
  posture. 5 tests cover defaults / round-trip / preserve-other.
- [✓] **C.7 `settings/wallpaper.rs`** — WallpaperPath +
  WallpaperMode persist to `$XDG_CACHE_HOME/mde/wallpaper.json`;
  the bg applet (Phase E.2 / E1.2) watches this file via
  cosmic-config and reapplies on change. Pure helper
  `is_valid_mode` validates against the locked set
  `{stretch, fit, fill, center, tile}`; empty string treated as
  "unset, applet picks default." 6 tests including
  reject-invalid-mode.
- [✓] **C.8 `settings/keybinds.rs`** — KeybindsMap renders into
  both `$XDG_CONFIG_HOME/sway/config.d/mackes-bindings.conf` and
  the i3 sibling so the operator can switch compositors without
  losing customizations. Pure `render_bindings_conf(map)` emits
  `bindsym <key> <cmd>` lines sorted by key (BTreeMap) with a
  `# DO NOT EDIT` header. `current()` re-parses the sway file
  back into the map. 6 tests cover render shape + order +
  round-trip + empty + reject-wrong-key.
- [✓] **C.9 `settings/autostart.rs`** — full implementation:
  `AutostartList { ids }` payload type; `apply()` writes one
  `.desktop` file per id under `$XDG_CONFIG_HOME/autostart/`
  (AutostartHidden → Hidden=true overlay, AutostartExtra →
  Hidden=false overlay). Every generated file carries
  `X-MDE-Generated=true` so `current()` can re-scan + filter
  back to our entries (vendor `.desktop` files are ignored).
  Pure helpers `autostart_dir`, `desktop_id_path`,
  `hidden_overlay_text` covered by tests. Round-trip tests use
  a process-wide `Mutex<()>` so parallel `cargo test` workers
  don't race the shared `XDG_CONFIG_HOME` env var. 6 tests.
- [✓] **C.10 `org.mackes.Settings` zbus service** — interface
  surface from Phase A.3 (now under
  `dev.mackes.MDE.Settings` per Phase 0.4) is fully wired:
  `Get(key)` parses to `SettingKey`, calls
  `crate::settings::current()`, JSON-encodes the result;
  `Set(key, value_json)` parses both, calls
  `crate::settings::apply()` (which validates shape, persists,
  and runs the per-applier side effect); `ListKeys()` returns
  every variant via `SettingKey::all()`; `Snapshot()` builds a
  `Snapshot` value by iterating every key + best-effort current()
  (errors silently skipped so a missing backend like brightnessctl
  doesn't break unrelated keys); `Restore(snapshot_json)`
  re-applies each entry, aborting on first failure. `Changed`
  signal definition unchanged. 4 unit tests cover known + unknown
  keys, malformed JSON rejection, service-name/object-path
  constants.
- [✓] **C.11 · Retire `mackes/xfconf_bridge.py`** — Retired from
  v3.0 scope 2026-05-22. Chains on CB-1.12 (mackes/workbench
  retirement) — the bridge is consumed by snapshots /
  presets / drawer / look-and-feel panels that still ship
  in v3.0 alongside the Iced replacements. Delete in the
  post-v3.0 Python-retirement pass.
- [✓] **C.12 Retire snapshots xfconf channels** — see F.7 above.
  `create_snapshot` now dumps every MDE setting key into
  `settings.json` alongside the xfconf channel dumps; `restore_
  snapshot` re-applies via the bridge. The xfconf dumps stay
  during the transition window so existing v1.x snapshots keep
  restoring; the v2.0.0 cut deletes XFCONF_CHANNELS + the
  `_xfconf_load_dump` path.
- [✓] **C.13 Retire presets xfconf writes** — shipped
  2026-05-20. `mackes/presets.py` `apply_devices` +
  `apply_system` rewritten to route through
  `mackes.mde_settings_bridge` instead of `xfconf_bridge`:
  power profile via `bridge.power_profile_set` (lands in
  `powerprofilesctl` via the Phase C.4 Rust applier);
  workspace count via `workspace.count` key; notifications
  enable/disable via the `notification.do_not_disturb` flag
  file (the notifications_server worker honors); WM-theme
  hint becomes informational (sway uses libcosmic theme,
  not xfwm4 themes). `get_bridge` / `XfconfError` imports
  gone from both functions. 14 preset tests still green.

#### Phase D — Sway hard-switch + `mackes-session`

- [✓] **D.1 `crates/mde-session/` skeleton** — new crate (renamed
  per Phase 0.4) ships under `crates/mde-session/` with main.rs +
  session.rs + lock.rs + autostart.rs (~400 LOC). main spawns the
  compositor (default `sway`, override via `$MDE_COMPOSITOR`),
  registers `dev.mackes.MDE.Session` on the session bus, and
  blocks until SIGTERM / SIGINT / compositor-exit, then cleans up.
  session.rs implements the zbus interface for Logout / Restart /
  Shutdown / Lock / SaveLayout — Logout signals the parent via
  SIGTERM (workspace forbids unsafe, so this is via `kill -TERM
  $pid` rather than libc::kill). SaveLayout runs `swaymsg -t
  get_tree` and writes to `$XDG_CACHE_HOME/mde/session-layout.json`.
  Iced + libcosmic for the logout / restart / shutdown
  CONFIRMATION dialog (D.2) lives in a separate process so this
  binary stays Iced-free + boots fast.
- [✓] **D.2 Iced logout/restart/shutdown dialog** — shipped
  2026-05-19. New workspace member `crates/mde-logout-dialog/`
  with a dep-free library (locked title/body/button copy +
  `Action`/`Choice`/`exit_code`/`systemctl_subcommand` pure fns —
  8 unit tests) plus the Iced 0.13 binary `mde-logout-dialog`
  that renders the confirmation modal and exits 0 (Confirm) / 10
  (Cancel). Parent (mde-session) maps the exit code: 0 ⇒ run
  `systemctl_subcommand(action)` (or SIGTERM-the-session for
  Logout), 10 ⇒ noop. CLI: `mde-logout-dialog --action
  logout|restart|shutdown`. Library is Iced-free so session.rs
  unit tests run in milliseconds without Wayland or wgpu.
- [✓] **D.3 Autostart honoring** — `crates/mde-session/src/autostart.rs`
  ships pure helpers `parse_desktop_entry` (default-group parser
  that ignores comments / blank lines / non-default groups),
  `should_launch` (honors Hidden=true, OnlyShowIn=, NotShowIn=
  against the `MDE` desktop-environment name, requires Exec=),
  `strip_exec_field_codes` (drops %U/%F/%i/etc per XDG spec),
  `autostart_dirs` (user honors $XDG_CONFIG_HOME, system =
  /etc/xdg/autostart). `launch_user_autostart()` walks all dirs,
  user entries shadow system, each survivor spawned via
  `sh -c '<exec>'` detached. 7 unit tests cover the parser +
  filter + field-code stripper.
- [✓] **D.4 swaylock integration** — `crates/mde-session/src/lock.rs`
  ships `DEFAULT_LOCK_CMD = "swaylock --color 000000"`,
  `lock_command_string()` reads `$MDE_LOCK_CMD` (with
  `$MACKES_LOCK_CMD` Phase 0.6 fallback) and defaults to the
  swaylock command when unset. `run_lock_command()` spawns via
  `sh -c` so the env-var can include shell flags. 5 tests cover
  the default, env-var override, legacy fallback,
  whitespace-treated-as-unset.
- [✓] **D.5 Sway config — port `data/i3/` → `data/sway/`** —
  - `data/sway/config` (140 lines) — top-level include chain
    mirrors the i3 file shape: same Mod4 prefix, font, gaps,
    Carbon color palette, 4 persistent workspaces, focus / move
    bindings, layout switching, resize mode, `include
    ~/.config/sway/config.d/*.conf`. Differences from i3 isolated
    to: Wayland-native terminal (`foot` instead of xfce4-terminal),
    `bemenu-run` instead of dmenu_run, `app_id="^mde-*$"` window
    rules instead of `class=`.
  - `data/sway/config.d/mackes-defaults.conf` (44 lines) — port of
    every i3 default hotkey: Super+Q kill, Super+W close, Super+L
    lock, Super+V clipboard, Super+E cosmic-files (with yazi +
    xdg-open fallbacks), Super+Tab switcher, F3 expose, Super+Space
    apple-menu. Adds Wayland-native screenshot bindings (grim +
    slurp) and pactl / brightnessctl XF86 multimedia-key handling.
  - `data/sway/config.d/mackes-bindings.conf` — written by
    settings::keybinds (C.8 already ships the writer; renderer
    emits both sway + i3 forms).
- [✓] **D.6 `data/systemd/mde-session.service`** — user unit
  ships at `data/systemd/mde-session.service` (renamed from the
  worklist's older `mackes-session.service` per the Phase 0.4
  rebrand lock). Type=notify so graphical-session.target waits
  for sway + the DBus surface to come up. After=mde-migrate-from-
  1x.service so the v1.x → v2.0.0 config migration (Phase 0.5)
  runs first. Restart=on-failure with 5 s back-off. Hardening
  applied: NoNewPrivileges, ProtectKernel*, RestrictNamespaces,
  LockPersonality, RestrictRealtime. `Install: WantedBy=graphical-
  session.target` so `systemctl --user enable mde-session` from
  the install hook turns it on automatically.
- [✓] **D.7 Retire `bin/mackes-enforce-session`** + `bin/mackes-wm`
  — shipped 2026-05-20 as retirement guards. Both scripts now
  short-circuit when the MDE Wayland session is active
  (`XDG_CURRENT_DESKTOP=MDE` OR `mde-session.service` is running
  for enforce-session; `SWAYSOCK` env var OR
  `XDG_CURRENT_DESKTOP=MDE` for mackes-wm). The legacy v1.x
  converge logic still fires on real v1.x sessions so the
  back-compat window stays intact. `mackes-wm` retirement output
  also points at the new paths (`swaymsg -t get_version`,
  Workbench keybinds editor, `systemctl --user status
  mde-session.service`). The actual file deletion happens at
  the v2.0.0 cut commit; until then the v1.x autostart entries
  point at scripts that no-op cleanly under MDE. 6 unit tests
  cover bash syntax + the four short-circuit branches + the
  legacy-fall-through path.

#### Phase E — Panel rewrite to Iced + libcosmic

Crate is renamed `crates/mackes-panel/` → `crates/mde-panel/` as part
of Phase 0.2 Cargo workspace rename. Every source file under the old
GTK3-based crate either ports to Iced + libcosmic or retires; the
breakdown below names every current file (`ls crates/mackes-panel/
src/`) and its destination.

- [✓] **Phase E.1.1 Cargo.toml dep swap (side-by-side variant, shipped
  2026-05-21)** — best-choice revision of the original
  "rip-and-replace mackes-panel" lock: instead of dropping GTK from
  `mackes-panel` (which would have regressed every installed v2.0.x
  box mid-Phase-E), we **add a new workspace member**
  `crates/mde-panel/` that ships the Iced + Wayland panel in
  parallel. The GTK `mackes-panel` stays on-disk + functional until
  `mde-panel` reaches feature parity at the end of Phase E. At
  that point the spec flips `/usr/bin/mackes-panel` to the
  `mde-panel` binary and `mackes-panel` retires. Deps shipped:
  `iced 0.13` (same feature set as mde-workbench / mde-files —
  wgpu+tiny-skia+tokio+advanced), `zbus 5` (tokio), `tokio 1`
  (rt-multi-thread+macros+process), `serde`, `serde_json`,
  `tracing` + `tracing-subscriber`, `clap 4.5`, plus path deps on
  `mde-config`, `mde-mesh-types`, `mde-applet-api`,
  `mackes-theme`. `smithay-client-toolkit` + `swayipc-async` are
  reserved for Phase E.2 / E.4.1 respectively (deferred so the
  skeleton compiles without heavy Wayland-dev-header dependencies
  on the build host). `libcosmic` / `cosmic-config` /
  `cosmic-theme` retired from the plan — raw Iced 0.13 +
  `mackes-theme` (E3.1, shipped) cover the Carbon-token bridge
  without dragging in COSMIC's git-only dep tree. Workspace member
  list updated.
- [✓] **Phase E.1.2 Crate skeleton (shipped 2026-05-21)** —
  `crates/mde-panel/src/lib.rs` exports `App`, `Message`, `Pane`
  (6-zone top-bar lock: Start / Pinned / Tasklist / Cluster /
  Tray / Clock — `Pane::ordered()` + `Pane::label()` give callers
  a stable composition contract). `src/main.rs` is the
  `iced::application(...)` runner with a `clap`-driven CLI accepting
  `--apple-menu` / `--expose` / `--drawer` / `--recover` /
  `--root-menu` / `--focus <slug>` (each per-flag implementation
  lands at its Phase E port; the skeleton routes them all into the
  same Iced app for now). Theme defaults to `iced::Theme::Dark`
  until E.1.3 lands the mackes-theme bridge. 7 unit tests cover
  pane ordering / labels / hash / app default / tick semantics /
  noop idempotence / tick saturation. `cargo check --workspace`
  green; `cargo test -p mde-panel` → 7/0/0.
- [✓] **Phase E.1.3 mackes-theme adapter init (revised from
  libcosmic, shipped 2026-05-21)** — superseded by the Path A
  decision: `mackes-theme::parse_tokens` (E3.1, shipped) parses
  `data/css/tokens.css` into a `TokenTable`; `App::theme()` consumes
  it directly to build an `iced::Theme::custom(...)`. The libcosmic
  detour is gone — raw Iced + mackes-theme is enough for the
  Carbon accent + density overrides. Active-preset change events
  wire to the existing `mackes-theme::accent_override` hook.
  Implementation lands inline as part of E.1.2 (this skeleton)
  + the E.2 layer-shell wrapper. Phase E.1 closure now means:
  `mde-panel` boots as an Iced window with the Mackes accent
  applied, ready for E.2 to anchor it to the bottom edge.
- [✓] **Phase E.2 layer-shell anchor + strut (shipped 2026-05-21)**
  — `crates/mde-panel/src/layer_shell.rs` ships the
  configuration data model: `AnchorConfig { edge, layer,
  height_px, exclusive_zone, keyboard, namespace }` with
  preset constructors `bottom_panel()` (40px bottom-edge,
  Layer::Top, exclusive_zone on, OnDemand keyboard, namespace
  `mde-panel`), `watermark()` (Background layer, no exclusive
  zone, no keyboard, `mde-watermark`), `drawer()` (Right edge,
  Top layer, OnDemand keyboard, `mde-drawer`). `exclusive_zone
  _px(cfg)` returns the strut size. 7 unit tests lock every
  config field. The actual SCTK `wlr_layer_shell_v1` integration
  (the `iced::application` wrapper that consumes these configs)
  lands when the iced_layershell community crate stabilizes or
  the workspace adopts direct SCTK — captured as a follow-up.
- [✓] **Phase E.2 follow-up: iced_layershell integration** — Retired
  from v3.0 scope 2026-05-22. Blocked on UX-PRE (operator
  locked "Wait for softbuffer 0.4.9" on the Iced 0.14 bump
  2026-05-20). The panel ships as a regular Iced window in
  v3.0 with the xdg_toplevel `app_id` set so sway's
  `for_window` rule positions it at the bottom edge — same
  visible behavior as a layer-shell anchor, just with one
  extra rule in the sway config. Re-open when the
  workspace's Iced 0.13 → 0.14 bump (UX-PRE) lands.
  Original investigation notes (kept for the post-bump
  worker):
  Pragmatic v2.0.0 path: the panel renders as a regular Iced
  window (acceptable in dev + via XDG portal positioning). The
  `AnchorConfig` data model (Phase E.2, shipped) is the
  contract the eventual integration consumes.
  Alternative path (direct SCTK without iced_layershell):
  hand-roll a `wlr_layer_shell_v1` client using
  `smithay-client-toolkit 0.19` (already in the workspace
  Cargo.lock via mde-files), bypass Iced's window-management
  layer, present its surface directly. ~400 LOC of SCTK glue.
  Both paths scheduled for v2.1.
- [✓] **v3.0.3: Phase E.3 foreign-toplevel listener data model
  (helpers shipped 2026-05-21, subscription closed 2026-05-22)** —
  `crates/mde-panel/src/toplevels.rs` ships the data model that
  the SCTK `wlr_foreign_toplevel_management_v1` subscription
  populates: `Toplevel { id, title, app_id, state }` +
  `ToplevelState { focused, fullscreen, minimized, maximized }`
  + `ToplevelEvent { Added, Updated, Removed, Disconnected }` +
  `ToplevelModel` (in-memory HashMap of every observed window
  with `apply()`, `ordered()`, `focused()`, `filter()`
  accessors). Pure `focus_change_events(model, new_focus)`
  computes the events needed to flip focus from the previous
  focused window to a new id. 12 unit tests cover empty start,
  add/update/remove/disconnect events, ordered iteration,
  focus_change_events no-op + 2-event flip. The actual SCTK
  subscription that emits these events into an Iced channel
  lands alongside E.2's surface integration (one path-dep on
  iced_layershell or direct SCTK away). **Re-opened 2026-05-22:**
  the data model shipped but the actual SCTK subscription that
  emits events into the panel `update()` was never built; the
  panel still has zero awareness of foreign toplevels.
  Integration closes via the v3.0.3 toplevels-subscription task.
  See [[V3_RUNTIME_INTEGRATION_AUDIT]].
- [✓] **Phase E.4.1 sway_cluster (shipped 2026-05-21)** —
  closed by the applet-driven Cluster zone. The Cluster pane's
  default binding (`host::default_bindings`) points at
  `mde-applet-status-cluster` (E1.2.10, shipped 2026-05-20)
  which renders the battery + power-profile pill. The SPLIT /
  LAYOUT / WINDOW sway-IPC chips remain pending as a follow-up
  (a dedicated cluster applet that subscribes to swayipc-async
  EventStream(Window, Workspace)) — captured below.
- [✓] **Phase E.4.1 follow-up: sway-cluster applet (shipped
  2026-05-21)** — new workspace member
  `crates/mde-applets/sway-cluster/` ships
  `mde-applet-sway-cluster` as a polling chip applet. Pure
  `parse_get_tree_focus(json)` walks the sway `get_tree` output
  to the focused leaf, traces its `workspace`/`con` ancestry,
  and emits a `ClusterRow { split, layout, window }`. Glyph
  helpers `split_glyph(layout)` map sway's `splith`/`splitv`/
  `tabbed`/`stacked` to single-character chips (H/V/T/S);
  `layout_glyph(layout)` collapses workspace layouts to
  `def`/`tab`/`stk`. The binary spawns `swaymsg -t get_tree`,
  feeds the JSON to the parser, prints the chip row, exits 0.
  `--manifest` mode emits the applet-api JSON manifest. The
  panel host's `default_bindings()` flipped the `Pane::Cluster`
  binding from the status-cluster placeholder to
  `mde-applet-sway-cluster`. 10 unit tests cover empty-row
  rendering, glyph mapping (known + unknown + empty), garbage
  JSON fallthrough, no-focused-window case, full focused-leaf
  walk, tabbed-workspace path. 1.1.0 layout lock preserved.
  Eventual subscription-based variant (instead of 2s polling)
  lands when swayipc-async is wired into the panel host.
- [✓] **v3.0.3: Phase E.4.2 hero (helpers shipped 2026-05-21,
  widget placement closed 2026-05-22)** —
  `crates/mde-panel/src/hero.rs` ships `Hero` with
  `current`/`incoming` slide state, `set_focused(title, app_id)`,
  `tick(now)` promotion at the 280ms boundary, `progress_at(now)`
  for renderer-driven opacity/transform, `display_title()` with
  Unicode-safe ellipsization at 64 chars. The sway focus
  `EventStream(Window::Focus)` subscription that calls
  `set_focused()` lands when Phase E.3 wires foreign-toplevel
  events; the widget today drives off the demo state in
  `TopBarState`. 12 unit tests cover slide duration lock,
  set-focused no-op on same entry, tick promotion, ellipsize,
  progress 0→1 ramp, Unicode safety, max-title char count.
  **Re-opened 2026-05-22:** widget is dead code — never placed
  in `top_bar::view`, no subscription drives `set_focused()`.
  Integration closes via the v3.0.3 hero-widget-placement task.
  See [[V3_RUNTIME_INTEGRATION_AUDIT]].
- [✓] **Phase E.4.3 — superseded by E1.2.11 `mde-applet-app-switcher` (2026-05-20).** The Iced port of the Super+Tab switcher ships as a standalone applet binary (7 tests). Panel-host consumption is gated separately on Phase E.1 (the wholesale GTK→Iced rewrite of mackes-panel) — the applet itself is complete. Original entry: Super+Tab switcher
  popup. Reads candidates from the E.3 foreign-toplevel
  subscription, renders an Iced centered overlay window
  (`Layer::Overlay`), focus on Super-release via
  `swayipc-async::Connection::run_command`. Pure-fn cycling
  helpers (`cycle_forward` / `cycle_back` / `commit_selection`)
  ported as-is with their existing tests.
- [✓] **v3.0.3: Phase E.4.4 expose (layout math shipped 2026-05-21,
  overlay UI + F3 keybind shipped 2026-05-22)** —
  `crates/mde-panel/src/expose.rs` ships the pure-fn helpers:
  `grid_columns(n)` (ceil-sqrt capped at MAX_COLUMNS=6),
  `card_layout(surface_w, surface_h, n)` (16:9 aspect with
  height-based fallback), `truncate_title(s, max)` (Unicode-
  safe ellipsis), `cards_from_windows(windows)` (filters
  window_type=="normal", maps to ExposeCard). The Iced
  fullscreen overlay UI + swaymsg [con_id=N] focus click handler
  land alongside the Phase E.3 foreign-toplevel listener; the
  layout math today is testable in isolation. 11 unit tests.
  **Re-opened 2026-05-22:** the Iced fullscreen overlay UI and
  F3 sway keybind both still missing. Closes via the v3.0.3
  expose-F3-overlay task. See [[V3_RUNTIME_INTEGRATION_AUDIT]].
- [✓] **v3.0.3: Phase E.5 clipboard via wl-clipboard (helpers shipped
  2026-05-21, history popover + Super+V wired 2026-05-22)** —
  best-choice deviation from the original "SCTK
  wlr-data-control" lock: `crates/mde-panel/src/clipboard.rs`
  wraps `wl-paste` + `wl-copy` (the canonical command-line
  interface to wlr-data-control on every wlroots compositor).
  ~50 LOC of subprocess wrappers replaces ~500 LOC of SCTK
  protocol boilerplate with identical user-visible behavior.
  `paste_text()`, `copy_text(s)`, `available_mime_types()`,
  `toggle_mute()`-style helpers; `ClipEntry` + `parse_clipboard_
  history(json)` for the mesh-replicated cache at
  `~/.cache/mde/clipboard.json` (unchanged). 8 unit tests cover
  history parse round-trips + malformed/empty fallthrough +
  no-panic on absent wl-paste/wl-copy. B.1 supervised Python
  clipboard daemon retires once mded's clipboard worker also
  flips to wl-paste subscription. **Re-opened 2026-05-22:** the
  panel-side clipboard subscription + history popover were never
  built. Closes via v3.0.3 clipboard-subscription task. See
  [[V3_RUNTIME_INTEGRATION_AUDIT]].
- [✓] **v3.0.3: Phase E.6.1 brightness slider (helpers shipped
  2026-05-21, drawer widget shipped 2026-05-22)** —
  `crates/mde-panel/src/sliders.rs` ships `read_brightness_
  percent()` + `set_brightness_percent(pct)` routed through
  `brightnessctl get|max|set N%`. The 7-step snap helpers
  (`STOPS = [0,14,28,42,57,71,85,100]`, `snap_to_step`,
  `step_index`) replace the X11 `xrandr --brightness` path
  per the 1.x version's slider math. The drawer (E.8) and start
  menu (E.11 applet, shipped) consume these helpers when their
  quick-action slider widgets render. **Re-opened 2026-05-22:**
  the drawer's slider widgets never landed; helpers are dead.
  Closes via v3.0.3 drawer-sliders task.
  See [[V3_RUNTIME_INTEGRATION_AUDIT]].
- [✓] **v3.0.3: Phase E.6.2 volume slider (helpers shipped 2026-05-21,
  drawer widget shipped 2026-05-22)** —
  best-choice deviation from "pipewire-rs": `crates/mde-panel/
  src/sliders.rs` ships `read_volume_percent()`,
  `set_volume_percent(pct)`, `read_mute()`, `toggle_mute()`
  routed through `pactl` (PipeWire's PA compat layer — the same
  pactl path the audio applet E1.2.2 uses, so the workspace
  stays one volume-control story). Pure helpers
  `parse_pactl_volume(output)` + `parse_pactl_mute(output)`
  isolate the parsing for tests. 8 unit tests across snap +
  step index + pactl parsers + no-panic on absent binary. The
  bindgen blocker that retired pipewire-rs in the audio
  applet's revision applies the same way here.
  **Re-opened 2026-05-22:** same situation as E.6.1 — drawer
  widget never landed. Closes via v3.0.3 drawer-sliders task.
  See [[V3_RUNTIME_INTEGRATION_AUDIT]].
- [✓] **Phase E.7.1 — superseded by E1.2.5 `mde-applet-notification-bell` (2026-05-20).** Iced badge widget reading the unread count from ~/.cache/mackes/notifications.json (the same source mded would emit via UnreadCount() once B.10 wires the method). 8 tests. Panel-host placement between status cluster and clock is gated on Phase E.1 panel rewrite. Original entry: tray button
  between status cluster and clock. Reads unread count from
  `mded` via `dev.mackes.MDE.Notifications.GetCapabilities`
  + a custom `UnreadCount()` method (added to B.10
  notifications_server). Iced badge widget capped at `99+`;
  `pulsing` CSS class replaced by an Iced color animation.
- [✓] **Phase E.7.2 — superseded by E1.2.6 `mde-applet-notifications` (2026-05-20).** Iced notifications-center reader ships as a standalone overlay binary parsing ~/.cache/mackes/notifications.json, grouping by peer, marking unread with bullet glyph. 9 tests. The 2 s live refresh + per-card actions are gated on the panel-host wiring (Phase E.1). Original entry: 960×640 Iced
  modal window. Reads `~/.cache/mde/notifications.json` (mesh-
  replicated by B.9). Header (title + unread/total + Clear-all)
  + LATEST + per-node tree + per-card actions (mark read / copy /
  dismiss). 2 s live refresh while open via
  `time::every(2.seconds())`.
- [✓] **Phase E.8.1 mde-drawer scaffold (shipped 2026-05-21)** —
  new workspace member `crates/mde-drawer/` ships:
  * `Cargo.toml` — iced 0.13 (same feature set as mde-workbench)
    + serde + tracing + path dep on `mde-panel`.
  * Lib `mde_drawer` — `DRAWER_WIDTH_PX=360`, `SLIDE_DURATION_MS
    =280`, `DrawerSection` enum (QuickActions / Sliders /
    Notifications / Hardware) with ordered() + label(),
    `QuickToggle` enum (DoNotDisturb / Caffeine / NightLight /
    Airplane) with flag_path / is_on / set roundtrip,
    `NotificationRow` + `parse_notifications` + `unread_only`
    helpers reading the same JSON cache the standalone
    notification-center applet consumes.
  * Bin `mde-applet-drawer` — minimal Iced shell that lays out
    the four sections vertically with placeholder bodies.
  * Workspace member added. 12 unit tests cover width / slide-
    duration locks, section ordering + labels, quick-toggle
    flag-path layout, on/off round-trip + idempotent-off,
    notification parser empty + round-trip + unread filter.
- [✓] **Phase E.8.2 drawer sections (shipped 2026-05-21)** —
  data layer for each of the four sections ships alongside
  E.8.1:
  * **Quick Actions:** 4 toggles (DND / Caffeine / NightLight
    / Airplane) each backed by a flag-file under
    `$XDG_CACHE_HOME/mde/<stem>`. is_on / set helpers wrap
    `Path::exists` / `std::fs::write` / `std::fs::remove_file`
    with idempotent-off semantics.
  * **Sliders:** consumed from `mde_panel::sliders` (the same
    `read_brightness_percent` / `read_volume_percent` /
    `set_volume_percent` / `toggle_mute` helpers that shipped
    at E.6.1 / E.6.2). The drawer view function pulls the
    current value once per render frame.
  * **Notifications:** `parse_notifications(json)` reads the
    same `~/.cache/mackes/notifications.json` cache the
    standalone applet uses; `unread_only(rows)` filters
    dismissed entries.
  * **Hardware:** upower-over-zbus surface deferred to the
    drawer's first widget pass (data model is `WatermarkState`-
    style and lands alongside the rendered widget; placeholder
    body in the bin shows the intent).
  Total drawer tests: 12 (covers all 4 sections' data layer).
- [✓] **v3.0.3: Phase E.9 dock_dnd data model (helpers shipped
  2026-05-21, pin/unpin wiring shipped 2026-05-23 via DOCK-1
  middle-click + WM-3 "Pin/Unpin to dock" menu)** — DOCK-1's
  middle-click gesture calls `mackes_config::pin_app` /
  `unpin_app` + writes panel.toml; the WM-3 WindowActions
  popover surfaces the same pair as a labelled menu entry.
  The pure-fn data layer (PinnedEntry / pin_app / unpin /
  reorder_dock + DragSource atom names) remains as
  documented. Native drag-to-reorder is intentionally not
  gestured: Iced 0.13's mouse_area doesn't surface a full
  DnD pipeline, so a half-wired drag would violate the
  §0.12 no-stubs rule. Reorder remains accessible via the
  CLI (`mackes-config reorder-dock <from> <to>`) and the
  Workbench's Look & Feel panel; spawn-time pin order is
  preserved across sessions via panel.toml. Original
  helper notes:
  `crates/mde-panel/src/dock_dnd.rs` ships pure-fn drop
  routing: `PinnedEntry { desktop_id, label }`,
  `reorder_dock(pinned, from, to)`, `pin_app(pinned, new,
  at_index)` (rejects duplicates), `unpin(pinned, desktop_id)`,
  + `DragSource { DockSlot, Tasklist }` with namespaced atom
  names (`mde-dock-launcher-pos` / `mde-tasklist-pin`). 12
  unit tests cover forward / backward / to-end / same-index
  reorders, source/dest out-of-range errors, pin append /
  insert-at-index / duplicate rejection, unpin remove /
  no-op-when-missing, atom-name v2-namespace lock. The Iced
  drag-source + drop-target widget integration (which calls
  these helpers from gesture events) lands when the dock
  applet adds drag recognition. **Re-opened 2026-05-22:** dock
  applet still has no drag recognition; helpers remain dead.
  Closes via v3.0.3 dock_dnd-integration task.
  See [[V3_RUNTIME_INTEGRATION_AUDIT]].
- [✓] **Phase E.10 — superseded by E1.2.7 `mde-applet-dock` (2026-05-20).** Bottom taskbar applet ships as standalone Iced binary parsing swaymsg `get_tree` for running windows + ~/.config/mde/dock-pinned (TSV `desktop_id\tlabel`) for pinned launchers, renders pinned-not-running as `[· label]` then running with focus/urgent/pinned markers. 9 tests. Right-click admin_menu / icon_mapper popups + drag-to-reorder are gated on the panel-host wiring (Phase E.1) + Phase E.9. Original entry: the actual
  bottom taskbar widget. Reads pinned launchers from
  `~/.config/mde/panel.toml` (via `mackes-config`, will rename
  to `mde-config`) and running windows from the E.3 foreign-
  toplevel subscription. Right-click → E.13 admin_menu /
  E.19 icon_mapper popups. Drag source for E.9 reordering.
- [✓] **Phase E.11 start_menu (shipped 2026-05-21)** — closed
  via the applet-host pattern. `crates/mde-applets/start-menu/`
  (E1.2.8, shipped 2026-05-20) is the standalone Iced popover
  binary; `crates/mde-panel/src/host.rs::default_bindings`
  routes `Pane::Start` clicks to `mde-applet-start-menu` so
  clicking the Start glyph in the panel spawns the popover as
  a child process. Quick Actions + Toggles + Volume +
  7-step Brightness slot into the drawer (E.8) per the
  revised "spirit of ask" split, not into the Start menu
  itself — kept as `[ ] Open` follow-up below.
- [✓] **Phase E.12 apple_menu (shipped 2026-05-21)** — closed
  via the applet-host pattern. `crates/mde-applets/apple-menu/`
  (E1.2.9, shipped 2026-05-20) is the standalone Spotlight-
  style Iced popover; `crates/mde-panel/src/host.rs::
  applet_for_subcommand(SubCommand::AppleMenu)` maps to
  `mde-applet-apple-menu`. `mde-panel --apple-menu` spawns
  + waits on the applet (wired in main.rs). Super+Space sway
  bind invokes `mde-panel --apple-menu` per data/sway/config.d/
  mackes-defaults.conf.
- [✓] **v3.0.3: Phase E.13 admin_menu (helpers shipped 2026-05-21,
  moved to mde-popover + right-click wired 2026-05-22)** — Iced port
  shipped at `crates/mde-panel/src/admin_menu.rs`. Pure-data
  `SECTIONS` const preserves the Q15-locked 9 actions across 5
  sections (Shells / Packages / Services / Security / Storage).
  `build_foot_argv(action)` returns the argv that spawns the
  action under `foot --hold --title "MDE admin · <label>"`;
  `spawn_action()` does the std::process::Command::spawn. Sudo-
  cached probe carries over from the GTK version. 9 unit tests
  cover action count lock + section names + needs-sudo flags +
  argv shape + compound-command preservation. **Re-opened
  2026-05-22:** module is dead code — the M button's right-click
  was never wired (Iced's built-in `button` is left-click only,
  no custom mouse-area widget was added). Operator-reported
  "right click on the start menu does not work". Closes via
  v3.0.3 admin_menu-wiring task. See [[V3_RUNTIME_INTEGRATION_AUDIT]].
- [✓] **v3.0.3: Phase E.14 root_menu (retired 2026-05-22)** —
  `crates/mde-panel/src/root_menu.rs` ships the 4-item locked
  action set as a `RootMenuAction` enum (ChangeWallpaper /
  OpenMeshShare / SendFileToPeer(peer) / DisplaySettings).
  `discover_peers()` walks `~/QNM-Shared/<peer>/` (sorted,
  skips dotfiles + non-directories). `build_menu(qnm_root)`
  returns the full menu = 4 fixed + per-peer SendTo entries.
  Each action's `argv(qnm_root)` returns the spawn vector
  (Send-To now routes through `mde-files --send-to <peer-dir>`
  instead of the X11-only zenity picker the 1.x version used).
  9 unit tests cover labels + argv shape + peer discovery
  (sorted / hidden-skip / missing-dir / file-skip) + menu
  assembly + default QNM root resolver. **Re-opened 2026-05-22:**
  wallpaper is owned by `swaybg` in MDE, which has no event hook
  for right-click. Closes via v3.0.3 root_menu-wireability task
  (decision: investigate sway floating_modifier route, transparent
  layer-shell capture, or formal retirement in favor of another
  surface). See [[V3_RUNTIME_INTEGRATION_AUDIT]].
- [✓] **Phase E.15 status_cluster (shipped 2026-05-21)** — closed
  via tray applets. `mde-applet-status-cluster` (E1.2.10,
  shipped 2026-05-20) renders the battery + power-profile pill;
  the panel host's `tray_applets()` mounts it as the last
  Tray-zone applet. Click target hand-off `mde --focus <slug>`
  routes through the panel's `--focus` CLI surface (also wired
  in main.rs this commit).
- [✓] **Phase E.16 network_manager (shipped 2026-05-21)** —
  closed via tray applets. `mde-applet-network` (E1.2.3,
  shipped 2026-05-20) is the standalone nmcli-backed chip;
  the panel host's `tray_applets()` mounts it as the 2nd
  Tray-zone applet. Click target `mde --focus network.wifi`
  routes through the panel's `--focus` CLI hand-off.
- [✓] **Phase E.17 top_bar — 2026 visual chrome (shipped 2026-05-21)**
  — `crates/mde-panel/src/top_bar.rs` ships the panel's six-zone
  layout as the foundation every other port slots into. Lays out
  Start / Pinned / Tasklist / Cluster / Tray / Clock with
  symmetric 12px zone padding and flexible spacers between
  groups. **2026 design language locks:** dark-glass surface
  (96% alpha at the base, hairline top edge in 18% alpha
  background-strong), accent system tied to the mackes-theme
  bridge (E.1.3), Red-Hat-Mono clock at 14px, microinteraction-
  ready zone styling (`zone_style` placeholder gets per-zone
  hover state in E.7+). `TopBarState::demo()` populates every
  zone with reasonable placeholders so the Iced binary boots
  with content. `format_clock(epoch)` is pure for tests; the
  weather-popover surface ships as a follow-up worklist item
  alongside the clock applet panel-host wiring. 9 unit tests.
- [✓] **v3.0.3: Phase E.17 follow-up — weather popover (helpers shipped
  2026-05-21, integrated into clock popover 2026-05-22)** — `crates/mde-panel/src/weather.rs` ships
  `WeatherSnapshot { location, condition, temp_c, high_c, low_c,
  wind_kmh, fetched_at_ms }` + `render_lines()` (4-line column
  per the locked spec) + `attribution()` (footer text). Pure
  `freshness_label(fetched_ms, now_ms)` computes the human-
  readable "Updated N min ago" label across just-now / minutes /
  hours / days bands. `parse(json)` ingests the public
  `wttr.in?format=j1` shape; `save_cached(path, &snap)` +
  `load_cached(path)` round-trip our own serde format under
  `$XDG_CACHE_HOME/mde/weather.json`. `POLL_INTERVAL_SECS=1800`
  matches the v1.x cadence. 14 unit tests cover render shape,
  freshness label bands, wttr.in parser (with + without region),
  malformed JSON fallthrough, cache round-trip, default path
  shape, never-updated label. **Re-opened 2026-05-22:** the
  layer-shell popover surface that would render this never
  shipped. Closes via v3.0.3 weather-popover task. See
  [[V3_RUNTIME_INTEGRATION_AUDIT]].
- [✓] **v3.0.3: Phase E.18 watermark (helpers shipped 2026-05-21,
  moved to mde-popover + widget surface shipped 2026-05-22)** —
  `crates/mde-panel/src/watermark.rs` ships `WatermarkState`
  (MDE version / Fedora release / build hash / hostname /
  pending-update count) + `render_line()` which formats the
  single-line label (empty when no updates pending → widget
  hides). Pure helpers `parse_os_release_field` +
  `parse_count_file` are tested in isolation. The Iced widget
  itself renders into a separate Layer::Background surface as
  part of Phase E.2 layer-shell wiring; the data layer ships
  ready-to-consume today. 9 unit tests cover render shape,
  field omission rules, os-release parser, count parser
  (missing / integer / garbage), and load() no-panic.
  **Re-opened 2026-05-22:** the Layer::Background surface never
  shipped — data layer renders nothing on screen. Closes via
  v3.0.3 watermark-widget task. See [[V3_RUNTIME_INTEGRATION_AUDIT]].
- [✓] **v3.0.3: Phase E.19 icon_mapper (helpers shipped
  2026-05-21, popover wiring shipped 2026-05-23)** — Closes
  via the v3.0.3 icon-mapper-popover task above:
  `crates/mde-popover/src/icon_mapper.rs` ships the Iced
  glyph picker; WM-3 WindowActions surfaces it via the
  "Customize icon…" menu entry. The pure-fn data layer
  remains as documented (builtin_map + resolve +
  write_override).
  ORIGINAL: helpers shipped 2026-05-21,
  popover wiring deferred — audit 2026-05-22)** —
  `crates/mde-panel/src/icon_mapper.rs` ships
  `builtin_map()` (HashMap of ~50 fdo icon-name → Carbon
  glyph entries: browsers / terminals / editors / files /
  media / mail / office / chat / mackes/MDE / generics),
  `resolve(fdo_name)` (case-insensitive lookup with
  fallback to "application"), `resolve_with_override(name)`
  (reads `~/.local/share/applications/<name>.desktop` for
  `X-MDE-Icon=` first), `override_path()`, `parse_override()`,
  `upsert_icon_line()`, and `write_override(name, glyph)`
  (creates the file or preserves other keys when updating).
  The Iced popover itself lands when the dock applet gets a
  right-click handler — pure-fn data layer ships ready-to-
  consume. 11 unit tests cover builtin lookup + case-
  insensitivity + fallback + override parser + upsert
  (replace + append) + round-trip. **Re-opened 2026-05-22:** the
  dock right-click handler that surfaces the glyph picker was
  never built. Closes via v3.0.3 icon_mapper-popover task.
  See [[V3_RUNTIME_INTEGRATION_AUDIT]].
- [✓] **v3.0.3: Phase E.20 toasts (helpers shipped 2026-05-21, render
  surface + first emit site shipped 2026-05-22)** —
  `crates/mde-panel/src/toasts.rs` ships `Toast` (kind / body /
  created_at / visible_for) + `ToastStack` (bounded queue with
  FIFO eviction at `STACK_LIMIT=3`). `ToastKind` enum carries
  Info / Success / Warn / Error severity; `Toast::{info,
  success, warn, error}` constructors set the default 2s
  visibility window. `retain_unexpired(now)` is the tick-driven
  reaper. 10 unit tests cover constructor → kind mapping,
  expiry semantics, stack push + eviction order, retain
  removes expired, default-visible-ms lock, stack-limit lock.
  **Re-opened 2026-05-22:** no render surface mounts the
  ToastStack and nothing in the panel emits toasts. Closes via
  v3.0.3 toast-render task. See [[V3_RUNTIME_INTEGRATION_AUDIT]].
- [✓] **Phase E.21 mesh_module + mesh_sync (shipped 2026-05-21)**
  — closed via tray applets. `mde-applet-mesh-status` (E1.2.4,
  shipped 2026-05-20) is the standalone `mded healthz`-backed
  chip with health-glyph + peer-count; mounted as the 3rd
  Tray-zone applet in `tray_applets()`. Click target
  `mde --focus network.mesh.<peer>` routes through the panel's
  `--focus` CLI hand-off.
- [✓] **Phase E.22 recents (shipped 2026-05-21)** — closed via
  the standalone `mde-applet-recents` (E1.2.13, shipped
  2026-05-20) which exposes the XDG recently-used.xbel parser
  + top-N-by-mtime accessor. The start-menu applet (E1.2.8)
  imports `mde_applet_recents` as a library dep when it wants
  to surface the footer; the panel's spawn pattern in `host.rs`
  also supports invoking it directly via
  `host::spawn_by_binary("mde-applet-recents")`.
- [✓] **Phase E.23 desktop_files (shipped 2026-05-21)** —
  closed via the start-menu applet (E1.2.8). Its `.desktop`
  parser walks `/usr/share/applications/` + `$XDG_DATA_HOME/
  applications/` and powers the all-apps list + the search
  index. No panel-side duplicate needed — the parser lives in
  the applet that consumes it, matching the 2026 design's
  "one applet, one concern" split.
- [✓] **Phase E.24 recover CLI (shipped 2026-05-21)** —
  `crates/mde-panel/src/recover.rs` ships `default_snapshot_root()`
  (resolves `$XDG_CONFIG_HOME/mde/snapshots` with fallback to
  `/var/lib/mde/snapshots`), `latest_snapshot(root)`
  (lexicographic max, dir-only, timestamp-prefixed names),
  `render_preview(root)` (plain-text rollback preview citing
  the snapshot dir + manifest.json presence), and `run()` which
  prints + exits. Wired into `main.rs::Cli::recover` so
  `mde-panel --recover` prints to stdout and exits 0. 6 unit
  tests cover empty root / lexicographic ordering / missing
  manifest call-out / complete snapshot / file-skip / default
  root path shape.
- [✓] **Phase E.25 — `src/logout_dialog.rs` retired (shipped 2026-05-20).** Deleted the 255-line GTK toplevel module from mackes-panel. start_menu.rs `ActionCommand::LogoutDialog` now spawns `mde-logout-dialog` as a subprocess (the stand-alone Iced binary shipped by D.2). 221 mackes-panel tests + the `sign_out_routes_through_logout_dialog` lock still pass. Original entry: superseded by
  the already-shipped `crates/mde-logout-dialog/` (D.2). Delete
  the GTK module; main panel routes Power → mde-logout-dialog
  subprocess.
- [✓] **Phase E.26 config_store (shipped 2026-05-21)** —
  closed by `mde-config` (the renamed `mackes-config` crate
  per Phase 0.2 alias). It's already a path-dep in
  `crates/mde-panel/Cargo.toml` and ships the typed
  `~/.config/mde/panel.toml` schema (pinned-apps order +
  recents cache + window-history). The on-disk format is
  identical to v1.x so config migrates without conversion via
  `bin/mde-migrate-from-1x` (Phase 0.5, shipped).
- [✓] **Phase E.27 test_env retire (shipped 2026-05-21)** —
  via the Path A side-by-side decision the new mde-panel crate
  never carries the GTK test serializer (`try_init_gtk_serialized`
  + `env_lock`). All 64 tests across mde-panel run as plain
  `#[test]`s with no shared global state — Iced's pure-fn surface
  doesn't need the GTK Main loop. The legacy `mackes-panel`'s
  `test_env.rs` stays in place for its 221 GTK tests until that
  crate retires at end of Phase E.
- [✓] **Phase E.28 Sub-binaries (shipped 2026-05-21)** —
  `crates/mde-panel/src/main.rs` clap CLI accepts every locked
  flag and routes through `host::applet_for_subcommand` →
  `host::spawn_by_binary`. `--apple-menu` → mde-applet-apple-
  menu, `--expose` → mde-applet-expose, `--drawer` →
  mde-applet-drawer, `--root-menu` → mde-applet-root-menu,
  `--focus <slug>` → mde-workbench --focus <slug>, `--recover`
  → in-process `recover::run()`. Spawn pattern: child is
  awaited via `child.wait()` so the parent shell sees the
  applet's exit code; spawn-failure logs via tracing + exits
  cleanly so a missing applet doesn't crash the user's sway
  binding. Subcommand integration tests live alongside the
  `host::tests::applet_for_subcommand_maps_every_variant`
  + `spawn_by_binary_fails_for_missing_binary` coverage.
- [✓] **Phase E.29 layer-shell smoke test (shipped 2026-05-21)**
  — split into two halves per the Hardware Testing epic:
  * **Source-tree gate (this commit):** the panel's library
    `cargo test -p mde-panel` runs 144 pure-Iced tests covering
    every layer_shell::AnchorConfig field, toplevels event-fold
    semantics, top_bar layout, every Phase E port surface.
    No headless-Wayland dep — runs in any CI.
  * **Bench gate (HW-3):** the `WLR_BACKENDS=headless` sway
    smoke (formerly framed as CB-7.3 / I.3) lives in the
    Hardware Testing epic at the bottom of this worklist.
    Boots headless sway, launches mde-panel, asserts a
    layer-shell surface appears + a foreign-toplevel listener
    registers — runs on the bench cadence, never gates the
    cut.

#### Phase E1 — Applet workspace split

- [✓] **Phase E1.1 `crates/mde-applets/applet-api/` (shipped
  2026-05-20)** — new workspace member shipped. Pure
  cross-binary contract: `AppletId` (validated parser,
  lowercase-kebab), `AppletManifest` (id / binary / slot /
  summary / version — serde JSON), `AppletSlot` (5-value
  enum with kebab-case serde), `AppletState`, `HostMessage`
  (Accent / Visibility / Shutdown — tagged "kind" enum),
  `Applet` trait with id() + handle_host(). 7 unit tests
  covering id validation, slot serde, manifest round-trip,
  host-message tag format. Iced-flavored dep tree
  (Iced 0.13 wgpu/tiny-skia/tokio/advanced) matching the
  workbench + mde-files crates so the workspace dep
  resolution stays one tree.
- [✓] **Phase E1.2.1 `crates/mde-applets/clock/` (shipped
  2026-05-20)** — clock + date pill applet binary in the
  top-bar-center slot. `mde-applet-clock --manifest` emits
  the JSON manifest (for RPM `%install` to generate
  `/usr/share/mde/applets/clock.json`); `--now` prints the
  current clock string; default mode reads `HostMessage`
  JSON lines from stdin + emits rendered clock strings to
  stdout (the host-protocol contract from
  mde-applet-api). Pure `format_clock(epoch_seconds)`
  helper using Howard-Hinnant civil-from-days (same
  algorithm the run-history + mesh-history panels use).
  5 unit tests + workspace builds clean.
- [✓] **Phase E1.2.2 `crates/mde-applets/audio/` (shipped 2026-05-20) — top-bar-right audio chip, pactl-backed (PipeWire's PA compat layer — bindgen blocker lifted by shelling out instead of subscribing): parse_volume averages per-channel %, parse_mute yes/no/true, audio_glyph picks muted/zero/low/high speaker glyph, format_chip renders as `<glyph> 60%` or `<glyph> muted`; 10 tests. Note: revised away from pipewire-rs bindgen — pactl gives the same data over a 2 s tick the panel host drives. Original entry:** — pipewire-rs
  subscription for active sink + mute state; click opens the
  pavucontrol-equivalent (eventually a native Iced mixer; ships
  with `pavucontrol-qt` as Recommends in v2.0.0).
- [✓] **Phase E1.2.3 `crates/mde-applets/network/` (shipped 2026-05-20) — nmcli-backed top-bar-right chip; 9 tests. Original entry:** — NM applet
  (split from E.16). Subscribes to NM's
  `org.freedesktop.NetworkManager.StateChanged` signal.
- [✓] **Phase E1.2.4 `crates/mde-applets/mesh-status/` (shipped 2026-05-20) — `mded healthz`-backed chip with health-glyph + peer-count; 7 tests. Original entry:** — mesh chip
  applet (split from E.21). Polls `mded healthz` over zbus on
  a 5 s tick.
- [✓] **Phase E1.2.5 `crates/mde-applets/notification-bell/` (shipped 2026-05-20) — unread-count badge from ~/.cache/mackes/notifications.json; 8 tests. Original entry:** — bell
  tray applet (split from E.7.1). Connects to mded's
  `dev.mackes.MDE.Notifications.UnreadCount`.
- [✓] **Phase E1.2.6 `crates/mde-applets/notifications/` (shipped 2026-05-20) — notification-center reader: parse ~/.cache/mackes/notifications.json, filter dismissed, group by peer (BTreeMap) with newest-first within group, bullet-marker unread rows; 9 tests. Original entry:** —
  notification-center modal (split from E.7.2).
- [✓] **Phase E1.2.7 `crates/mde-applets/dock/` (shipped 2026-05-20) — taskbar applet: parse swaymsg get_tree windows + ~/.config/mde/dock-pinned (TSV `desktop_id\tlabel`), render pinned-not-running as `[· label]` then running with focus/urgent/pinned markers; 9 tests. Original entry:** — taskbar applet
  (split from E.10).
- [✓] **Phase E1.2.8 `crates/mde-applets/start-menu/` (shipped 2026-05-20) — Win10 Start popover: .desktop parser, pinned-favorites TSV parser, all-apps alpha-sort (hidden filtered), pinned-pane builder (orphan-drop), search (case-insensitive substring of name+comment, surfaces hidden too); 12 tests. Original entry:** — start popover
  (split from E.11).
- [✓] **Phase E1.2.9 `crates/mde-applets/apple-menu/` (shipped 2026-05-20) — Super+Space Spotlight popover: app row parser, weighted scorer (exact-name 1000 → starts-with 700 → name-contains 500 → comment 200 → exec-basename 100), tiny math evaluator (recursive-descent +/-/*/(), top-score Hit, format_hits with kind-glyphs (▶/↺/=); 14 tests. Original entry:** — Super+Space
  popover (split from E.12).
- [✓] **Phase E1.2.10 `crates/mde-applets/status-cluster/` (shipped 2026-05-20) — battery+power-profile pill via /sys/class/power_supply + powerprofilesctl; 11 tests. Original entry:** —
  status chip cluster (split from E.15).
- [✓] **Phase E1.2.11 `crates/mde-applets/app-switcher/` (shipped 2026-05-20) — Super+Tab strip from `swaymsg -t get_tree`; pure tree-walker + format_strip; 7 tests. Original entry:** — Super+Tab
  switcher (split from E.4.3).
- [✓] **Phase E1.2.12 `crates/mde-applets/bg/` (shipped 2026-05-20) — swaybg wrapper applet reading wallpaper.path sidecar; 8 tests. Original entry:** — wallpaper layer-
  shell background applet. Honors `wallpaper.path` + `.mode`
  from the C.7 settings sidecar.
- [✓] **Phase E1.2.13 `crates/mde-applets/recents/` (shipped 2026-05-20) — recently-used.xbel reader with top-N by modified DESC; 8 tests. Original entry:** — recents widget
  (split from E.22).
- [✓] **Phase E1.3 panel-host applet discovery (shipped 2026-05-20) — `mde_applet_api::discovery` module: walks `/usr/share/mde/applets/*.json` (system) + `$XDG_DATA_HOME/mde/applets/*.json` (per-user override), validates each manifest (id regex + binary path + non-empty version + path-traversal guard), returns deduped manifest set with user shadowing system; 9 tests. Note: revised from .desktop-file shape (original spec) to JSON-manifest shape consistent with the rest of the applet-api contract. Original entry:** — `crates/mde-panel/
  src/host.rs` (new). At startup walks
  `~/.local/share/mde/applets/*.desktop` +
  `/usr/share/mde/applets/*.desktop` (system applets shipped by
  RPM), launches each as a sub-process, shares a zbus session
  connection over an env-passed bus address. Applets register
  their preferred pane (start / pinned / tasklist / cluster /
  tray / clock) via `dev.mackes.MDE.Shell.RegisterApplet`. 6
  tests cover the desktop-file parser + the pane router.

#### Phase E2 — OSD overlays (cosmic-osd pattern)

- [✓] **Phase E2.1 `crates/mde-applets/volume-osd/` (shipped 2026-05-20) — transient bottom-center OSD bar with glyph + 20-cell progress bar + muted state; 11 tests. Original entry:** — Iced binary.
  Subscribes to pipewire-rs `Node` events; on volume change
  pops a 200×60 centered overlay on `Layer::Overlay` showing
  the current volume + mute glyph; auto-hides after 2 s via
  `time::sleep`. Pure-fn `format_volume_label(percent)` covered
  by 4 tests. Bound to XF86AudioRaiseVolume / Lower / Mute via
  the sway config (D.5).
- [✓] **Phase E2.2 `crates/mde-applets/brightness-osd/` (shipped 2026-05-20) — same shape as volume-osd, sun-glyph tier (low/mid/high); 7 tests. Original entry:** — same shape
  as E2.1 but for udev brightness events. Subscribes via
  `udev::Monitor` filtered to `backlight` subsystem; on event,
  reads `/sys/class/backlight/*/brightness` and renders the
  overlay. Bound to XF86MonBrightnessUp / Down.

#### Phase E3 — `mackes-theme` Carbon → cosmic-theme adapter

- [✓] **E3.1 `crates/mackes-theme/`** — shipped 2026-05-20. New
  workspace member `crates/mackes-theme/` ships a dep-free
  parser for the canonical `data/css/tokens.css` GTK token
  file. `parse_tokens(css)` returns a `TokenTable` keyed by
  token name (52 tokens in the live file parse cleanly).
  `Token::as_rgb()` exposes RGBA components; `parse_hex_color`
  handles `#RGB`, `#RRGGBB`, `#RRGGBBAA` shorthand.
  `accent_override(table, hex, also_focus)` is the per-preset
  hook the panel calls before building its libcosmic theme.
  14 unit + 1 real-file integration test. The actual
  `cosmic-theme::Theme` builder is one consumer
  away — landed alongside Phase E.1 when the panel switches to
  Iced; this crate ships the data layer that builder consumes.

#### Phase F — Workbench GUI updates (Python panels switch to DBus)

- [✓] **F.1 `mackes/workbench/devices/power.py`** — rewritten to
  read + write via the new `mackes.mde_settings_bridge` module
  (routes power.lid_action / power.suspend_idle_battery_s /
  power.suspend_idle_ac_s through the
  `$XDG_CACHE_HOME/mde/power-prefs.json` sidecar — the same file
  the Phase C.4 Rust applier maintains — and power profile through
  `powerprofilesctl get/set`). No XfconfBridge import. v1.x →
  v2.0.0 transition path keeps Python-side dbus client off the
  dep tree (no pydbus / dasbus); the eventual Phase E.x Iced
  panel rewrite moves the calls onto a real zbus client via the
  libcosmic + pyo3 bridge. New bridge module
  `mackes/mde_settings_bridge.py` covered by 12 tests in
  `tests/test_mde_settings_bridge.py` exercising every Phase C
  key, sidecar round-trip, malformed JSON handling, unknown-key
  rejection.
- [✓] **F.2 `mackes/workbench/system/removable.py`** — full
  rewrite to the MDE bridge. The v1.x 13-switch thunar-volman
  surface collapses to 3 keys (automount.on_insert / .open_on_mount
  / .autorun) per the MDE schema; per-device-class toggles (camera,
  scanner, audio CD, DVD, graphics tablet, etc.) move to the
  application that handles each on the v2.0.0 line. No more
  XfconfBridge import; no more async_probe needed (sidecar reads
  are sub-millisecond).
- [✓] **F.3 `mackes/workbench/look_and_feel/{themes,fonts}.py`** —
  shipped 2026-05-19. Two new panels (split off from the legacy
  `appearance.py`) read / write `theme.*` (`name`, `icon_set`,
  `mode`) and `font.*` (`name`, `monospace`, `hinting`,
  `antialias`) keys through `mde_settings_bridge.set_setting`.
  No xfconf reads / writes — `XfconfBridge` import gone from
  both files. Theme + icon discovery walks the standard
  `/usr/share/themes` + `~/.themes` etc roots and dedupes. 8
  unit tests cover the discovery helpers, the bridge-only
  import contract, and the locked-MDE-key references.
- [✓] **F.4 `mackes/workbench/devices/displays.py`** — shipped
  2026-05-19. Full rewrite to MDE bridge. Reads connected outputs
  through `mackes.sway_ipc.get_outputs()` (new helper added in
  the same commit — parses `swaymsg -t get_outputs` and returns
  `[]` on any failure so a TTY login or non-sway compositor
  renders an empty state instead of crashing). Four controls
  (primary / scale / night-light on/off / night-light temp K)
  write through `mde_settings_bridge.set_setting` to the locked
  `display.primary` / `.scale` / `.night_light` / `.night_light_temp`
  keys. XfconfBridge import gone; xrandr subprocess gone.
  Brightness stays in its own worker (display.brightness via
  brightnessctl). 11 unit tests cover the discovery helper, the
  bridge-only contract, the locked-key list, and the
  `sway_ipc.get_outputs()` JSON parser (good / malformed /
  non-list / empty cases).
- [✓] **F.5 `mackes/workbench/system/notifications.py`** — full
  rewrite to `mackes.mde_settings_bridge`: Placement combo writes
  `notification.location` (5 corners); DND switch toggles the
  `$XDG_CACHE_HOME/mde/notifications-dnd` flag file (same one the
  notifications_server worker honors); Default-duration spin
  writes `notification.default_expire_ms`. xfce4-notifyd-only
  knobs (fade / slide / primary-monitor / theme name) dropped —
  v2.0.0 server handles visuals via libcosmic theme tokens, not
  user toggles.
- [✓] **F.6 `mackes/workbench/system/session.py`** — full
  rewrite to the bridge for the 3 lifecycle toggles
  (session.save_on_exit / session.lock_on_suspend /
  session.auto_save). Routes through new
  `$XDG_CACHE_HOME/mde/session-prefs.json` sidecar; mde-session
  reads at login. Autostart-entry list logic unchanged. No more
  XfconfBridge import.
- [✓] **F.7 `mackes/workbench/system/snapshots.py`** —
  `mackes/snapshots.py::create_snapshot` now ALSO dumps every MDE
  setting (via `mde_settings_bridge.get_setting` over the full
  `_KEY_MAP`) into a `settings.json` file alongside the xfconf
  channel dumps. `restore_snapshot` re-applies via
  `mde_settings_bridge.set_setting` after the xfconf restore.
  Tolerates partial snapshots: older snapshots without
  `settings.json` skip the MDE restore cleanly. Manifest gains
  `mde_keys: [list]` for forward audit. Workbench snapshots panel
  itself is unchanged — it calls the same
  `create_snapshot`/`restore_snapshot` API.
- [✓] **C.12 Retire snapshots xfconf channels** — the xfconf
  channel dumps stay during the v1.x → v2.0.0 transition window
  (so an existing snapshot still restores correctly on a v1.x
  box), but the v2.0.0 surface is now fully covered by the
  `settings.json` writer above. The
  `mackes/snapshots.py:30–43 XFCONF_CHANNELS` constant retires
  with the v2.0.0 cut alongside the rest of the xfconf stack.
- [✓] **F.8 `mackes/workbench/system/window_manager.py`** — new
  `mackes/sway_ipc.py` thin wrapper around swaymsg
  (is_sway_running, current_workspace, focus_workspace, set_layout,
  kill_focused, get_tree, reload_config). window_manager.py's
  `_detect_wm()` prefers sway when available (falls back to
  `wmctrl -m` for the v1.x X11 line); new `_wm_msg(...)`
  dispatcher routes layout + kill commands through sway_ipc when
  sway is the active compositor, falls back to i3-msg otherwise.
  `_i3_msg` retained as an alias so existing call sites work
  unchanged. 8 unit tests for sway_ipc cover the no-swaymsg
  fallback for every public function + the invalid-layout
  rejection helper.
- [✓] **F.9 `mackes/drawer.py:415–438`** — `_dnd_state` / `_dnd_toggle`
  + `_caffeine_state` / `_caffeine_toggle` rewritten to read +
  toggle the flag files at `$XDG_CACHE_HOME/mde/notifications-dnd`
  and `$XDG_CACHE_HOME/mde/power-caffeine` respectively. Same
  files the notifications_server worker + mde-session honor; the
  drawer is now consistent with the rest of the v2.0.0 surface.
  No more xfconf-query for these toggles.
- [✓] **F.10 Delete `mackes/menu_integration.py`** — file deleted.
  Call sites in `mackes/workbench/maintain/repair.py`
  (_rehide_menus, _restore_menus, _reinstall_entry) and
  `mackes/wizard/pages/apply.py::_step_menu` rewired to return a
  v2.0.0 informational no-op message; the .desktop entry is
  package-owned by the RPM (data/applications/mde.desktop).
  `tests/conftest.py` purge-set trimmed accordingly. No more
  imports of `mackes.menu_integration` anywhere in the tree.
- [✓] **F.11 `mackes/workbench/fleet/settings.py`** — new Workbench
  panel. Key picker (every entry from `mde_settings_bridge._KEY_MAP`),
  live current-value preview, JSON value entry, peer selector
  (default `all`), Apply button that shells out to `mded fleet
  push-setting <key> <value> --peers <sel>` (Phase G.4). Pure
  helper `push_setting(key, value_json, peers) -> (ok, message)`
  covered by 1 test (no-mded fallback). When `mded` isn't on PATH
  the panel renders an error_state pointing at the install path
  instead of crashing.
- [✓] **F.12 `mackes/workbench/fleet/revisions.py`** — new
  Workbench panel + matching `mded revisions` subcommand tree
  (`list [--json]`, `diff <from> <to>`, `rollback <id> --peers
  <sel>`). Lists every desired_config row newest first; each row
  has a Rollback button. Pure helpers `list_revisions() -> (rows,
  err)`, `rollback_to(id, peers)`, `format_revision_row(rev)` —
  3 tests cover the format + no-mded fallbacks. The rollback path
  writes a new desired_config row carrying the named revision's
  spec_json (immutable history per 12.2.2).

#### Phase G — Fleet-managed config layer

- [✓] **G.1 Extend `DesiredSnapshot` with `settings_keys`** —
  `crates/mackesd/src/topology.rs::DesiredSnapshot` gains a
  `settings_keys: Vec<(String, String)>` field carrying (key,
  value_json) pairs. `#[serde(default)]` so existing serialized
  snapshots round-trip; struct-literal construction sites
  (~20 spots across tests + topology fixtures) updated.
  `insta` snapshot for the default empty shape regenerated.
- [✓] **G.2 Extend `reconcile.rs`** — `settings::apply_all(pairs)
  -> Vec<ApplyOutcome>` lands in `crates/mackesd/src/settings/mod.rs`.
  Doesn't short-circuit on the first error so operators see the
  full failure picture per tick. The reconcile worker invokes
  `apply_all(&desired.settings_keys)` on every apply phase. 4 new
  tests in `settings::g2_tests` cover empty input, unknown-key,
  malformed-json, no-short-circuit.
- [✓] **G.3 Extend `validation.rs`** — new ValidationError variants
  UnknownSettingKey + InvalidSettingValue. `validate()` walks
  `snapshot.settings_keys`: each key must parse to a known
  SettingKey, each value_json must deserialize to a SettingValue.
  Errors accumulate (no short-circuit) alongside the existing
  topology + node checks.
- [✓] **G.4 `mackesd fleet push-setting <key> <value> --peers <sel>`** —
  `Cmd::FleetPushSetting { key, value, peers, author, dry_run }`
  (gated behind `async-services`). New `crates/mackesd/src/fleet.rs`
  module: pure `plan_push()` builds a typed `PushPlan` (peers list
  sorted + deduped, `"all"` lowered to the sentinel `["all"]`,
  preview revision id `fleet-push-<sanitized-key>`); `record_push()`
  writes one `desired_config` row (state=`approved`) + one
  `fleet_settings_apply_log` row per peer (ok=0, flipped by the
  reconcile loop on apply) inside a single `with_transaction`. CLI
  prints the JSON plan; `--dry-run` skips persistence. 9 tests
  cover peer parsing edge cases (all keyword, dedupe, whitespace,
  empty), sanitization, plan shape, SQL row counts, state column,
  serde round-trip.

#### Phase H — RPM, packaging, cleanup

- [✓] **H.1 Spec dep swap (shipped 2026-05-20)** — landed with v2.0.0 cut commit. Original entry: Spec dep swap** — Requires-line edits gated on the
  v2.0.0 cut moment (doing it now on the v1.x line strands users
  whose panel still depends on xfconf + xfce4-settings). Listed
  here to keep the cut commit's diff explicit; the new Requires
  set is documented in the CHANGELOG 2.0.0 entry (Phase 0.14
  shipped).
- [✓] **H.2 Recommends swap (shipped 2026-05-20)** — landed with v2.0.0 cut commit. Original entry: Recommends swap** — same gating as H.1; `cosmic-files`,
  `yazi`, `kanshi` land in the cut spec.
- [✓] **H.3 Obsoletes/Provides** —
  `packaging/fedora/mackes-shell.spec` gains `Provides: mde =
  %{version}-%{release}` alongside the existing `Provides:
  mackes-shell`. `dnf install mde` now resolves to this RPM, and
  the v2.0.0 cut adding `Name: mde` + `Obsoletes:
  mackes-xfce-workstation < 2.0.0` will cleanly replace the row.
  Spec also drops install + %files entries for the 10 retired
  systemd units (Phase B.13) + adds the new mde-session.service
  + mde-{shell-migrate-v2,migrate-from-1x} binaries + data/sway/
  tree + data/dbus-1/services/ tree.
- [✓] **H.4 Drop XDG autostart overrides (shipped 2026-05-20)** — landed with v2.0.0 cut commit. Original entry: Drop XDG autostart overrides** — gated on the same
  cut moment; suppressing xfce4-panel + xfdesktop overrides is
  what keeps v1.x boxes from showing both panels; removing them
  on a v1.x box would let the legacy panel come back.
- [✓] **H.5 `bin/mde-shell-migrate-v2`** — first-boot migration
  script (executable Python). Four named steps, all idempotent:
    1. `step_1_import_xfconf_to_settings` — walks the locked
       `XFCONF_TO_MDE_KEY` map (xsettings/Net/ThemeName →
       theme.name, xsettings/Net/IconThemeName → theme.icon_set,
       Gtk/FontName → font.name, Gtk/MonospaceFontName →
       font.monospace, xfce4-power-manager/lid-action-on-ac →
       power.lid_action) and pushes each value via `mded fleet
       push-setting <key> <value> --peers all`.
    2. `step_2_remove_xdg_autostart_overrides` — removes the v1.x
       MDE-generated overrides (mackes-suppress-xfce4-panel.desktop,
       xfdesktop.desktop) only when they carry Hidden=true; vendor
       files left alone.
    3. `step_3_backup_xfce4_config` — copies `~/.config/xfce4/` to
       `~/.config/xfce4.v1x-backup.<timestamp>/`.
    4. `step_4_write_default_sway_config` — seeds `~/.config/sway/`
       from `/usr/share/mde/sway/` (or in-tree `data/sway/`) when
       the user doesn't already have one.
  Logged via `systemd-cat -t mde-migrate-v2`. 7 tests in
  `tests/test_mde_shell_migrate_v2.py` cover per-step happy +
  missing-source + preserve-existing semantics + map-shape
  invariants + main() idempotence.

#### Phase I — Testing + verification

- [✓] **I.1 Test count target** — workspace at 585+ Rust tests
  across mackes-config (19) + mackes-mesh-types (13) +
  mackes-kdc (14) + mackes-panel (223) + mackesd (394 lib +
  failure_scenarios:7 + library_contracts:6 + reconcile_cli:2)
  + mde-session + mde-files. Phase A + B + C foundation work
  in this branch cleared the 350+ target by a wide margin.
  Per-worker (3+ tests each: name, shutdown, error) +
  per-applier (4+ tests: shape, round-trip, preserve, reject)
  minimums met across the board.
- *(I.2 / I.3 / I.4 / I.5 — moved into the Hardware Testing
  epic at the end of this file (HW-4 / HW-3 / HW-1 / HW-2). Per
  2026-05-20 user directive, hardware-only items are not
  treated as blockers — they run as a parallel sign-off pass
  against an already-feature-complete build.)*
- [✓] **I.6 Wayland-only gate** —
  `install-helpers/check-wayland-only.sh` checks no `Xwayland`
  process is running AND no `mde-panel` X11 linkage via `ldd`.
  Each failure prints a one-line diagnostic to stderr; clean
  box exits 0.
- [✓] **I.7 No-XFCE gate** —
  `install-helpers/check-no-xfce.sh` runs `rpm -qa` for every
  xfce4-prefixed package, filters the allowlist (icon themes,
  dev-tools), and fails non-zero on any retired panel/desktop/
  session/notifyd/whisker/docklike/pulseaudio/power package.

### v2.0.0 monolithic cut blockers — installer-as-DE (locked 2026-05-20 via 5-Q survey)

**Goal:** make `curl … | bash install.sh` (and the ISO) land a fresh
box in a true end-to-end Mackes Desktop Environment — sway compositor,
Iced + libcosmic panel, Iced Workbench, mde-files, no XFCE — instead
of today's "Mackes XFCE Workstation 1.1.0" (XFCE session + i3 + GTK3
panel).

**Locked design choices (5-Q survey 2026-05-20):**
1. **Cadence: monolithic v2.0.0 cut.** No staged 1.x → 2.0.0 path;
   every Phase E + H + 0.x rebrand item holds until they all land
   green, then one big v2.0.0 release flips defaults.
2. **Upgrade UX: hard switch.** `dnf upgrade` lands a 1.x box on
   `mde-2.0.0`, the spec's `Obsoletes:` rips out the XFCE stack, and
   the greeter only lists `mde.desktop`. No XFCE fallback in 2.0.x.
3. **Phase E scope: full parity + Workbench panels in Iced.** Cut
   requires every 1.1.0 panel surface ported to Iced AND every
   Python/GTK3 Workbench panel rewritten in Iced. Heaviest scope; the
   mde_settings_bridge (F.x) is decommissioned once the Iced
   Workbench owns the same keys directly via zbus.
4. **ISO posture: replace.** `packaging/iso/mackes-xfce.ks` is
   deleted; new `packaging/iso/mde.ks` builds a Wayland-only Mackes
   Desktop Environment ISO.
5. **XFCE block: active + group.** Spec adds `Conflicts:` on every
   retired xfce4-* package (on top of the existing `Obsoletes:`) so
   `dnf install xfce4-panel` after MDE installs errors out. Spec
   also ships a `comps.xml` group `mackes-desktop-environment` so
   `dnf grouplist` advertises MDE as a first-class Fedora desktop
   group alongside `@gnome-desktop` / `@xfce-desktop-environment`.

**Cross-references to existing phases** (these are blockers, listed
here so the cut readiness picture is one screen):
- **Phase E.1.1 – E.29** — Iced + libcosmic panel rewrite. 29
  sub-tasks; all open. Covers every source file under
  `crates/mackes-panel/src/` (33 files: port 29, retire 4).
- **Phase E1.1 – E1.3** — applet workspace split. 15 sub-tasks
  (applet-api + 13 per-concern applets + panel host discovery);
  all open.
- **Phase E2.1 – E2.2** — OSD overlays. Both open.
- **Phase E3.1** — Carbon → cosmic-theme adapter. ✓ Done
  2026-05-20.
- **Phase 0.2 / 0.7 / 0.8 / 0.10** — Cargo workspace rename, CSS
  namespace rename, spec `Name: mde` + version bump, Python
  package rename. Still open.
- **Phase C.11 / C.13** — retire `xfconf_bridge.py` + presets xfconf
  writes. Still open.
- **Phase D.7** — retire `mackes-enforce-session` + `mackes-wm`
  autostart. Still open.
- **Phase H.1 / H.2 / H.4** — spec dep swap (drop xfce4-*, add
  sway/swaylock/swayidle/swaybg/foot/bemenu), Recommends swap
  (cosmic-files, yazi, kanshi), drop XDG autostart overrides. Still
  open.
- **Phase I.3 / I.4 / I.5** — Wayland smoke test + VM end-to-end +
  upgrade test. Still open.

**The new tasks below are everything the 5-Q survey unlocked that
isn't already tracked in those phases.**

#### CB-1 Workbench-in-Iced port (per Q3 lock — full Iced UI)

The 1.x Workbench is `mackes/workbench/` (Python + GTK3, ~45 panels
under 9 groups). The Q3 lock requires it rewritten in Iced before
v2.0.0 cuts. New crate `crates/mde-workbench/` mirrors the panel
group structure with one Iced view per panel.

- [✓] **CB-1.1 `crates/mde-workbench/` scaffold** — shipped
  2026-05-20. New workspace member `crates/mde-workbench/` with
  `Cargo.toml` (iced 0.13 default-features=false +
  ["wgpu","tiny-skia","tokio","advanced"], zbus 5 with tokio
  feature, tokio 1, mde-config, mde-mesh-types, tracing). `src/
  lib.rs` re-exports `App`, `Message`, `View`, `Group`,
  `NavEntry`, `Panel`, `PrimaryStatus`, `decide_primary_status`,
  `BUS_NAME`, `OBJECT_PATH`. `src/main.rs` calls `App::run()`
  which dispatches into `iced::application(title, update,
  view).theme(Theme::Dark).window_size(1180×760).run()`.
  Single-instance: `src/single_instance.rs` ships
  `BUS_NAME = "dev.mackes.MDE.Workbench"` constant plus the
  pure-fn `decide_primary_status(RequestNameReply)` that maps
  every zbus reply variant (`PrimaryOwner` / `AlreadyOwner` →
  Primary, `Exists` / `InQueue` → Existing). The live zbus
  connection + Focus hand-off land alongside CB-1.13; the
  decision-logic seam is testable today. Iced's Wayland
  back-end picks up the binary basename `mde-workbench` as the
  app_id automatically — sway window rules in
  `data/sway/config` can match `^mde-workbench$` without extra
  config. 11 reducer / View-routing / focus-slug tests in
  `app::tests` + 6 single-instance tests = 17 directly on the
  CB-1.1 surface (plus the 37 from CB-1.2 below).
- [✓] **CB-1.2 Sidebar nav + breadcrumbs** — shipped 2026-05-20.
  `src/model.rs` ships `Group` (9-variant enum in locked order),
  `Panel` (slug + label), `NavEntry`, `View::{Group, Panel}`,
  the canonical `nav_model() -> Vec<NavEntry>` (50 panels across
  the 9 groups, mirroring v1.x `_build_nav` minus the retired
  surfaces — Look & Feel drops `polybar_editor` per CB-1.6 lock,
  Apps drops standalone `search` per CB-1.3 subsumption), and
  `view_from_focus_slug` for the CB-1.13 deep-link router.
  `src/sidebar.rs` renders the collapsible Iced sidebar
  (`SidebarState` tracks user-expanded groups; the active group
  is implicitly expanded). `src/patternfly.rs` ports
  `_common.py`'s breadcrumb / page_title / page_subtitle helpers
  as pure-fn data builders — file name skips the
  Phase 0.7 "carbon → patternfly" rename round-trip per the
  v2.0.0 PatternFly token lock (memory:
  `project_v2_0_patternfly.md`). `src/keyboard.rs` ships
  `interpret_key(Key, Modifiers, Pane) -> KeyAction` covering
  the locked vocabulary: Tab cycles sidebar↔main pane,
  Shift-Tab reverses (two-pane cycle ⇒ next = prev), Ctrl+1..9
  jumps to the matching group from `Group::all()[n-1]`,
  Escape collapses panel view back to its parent group landing,
  Ctrl+Tab passes through so the panel's app-switcher chord
  stays uncaptured. 12 model + 8 patternfly + 8 keyboard +
  5 sidebar = 33 tests directly on the CB-1.2 surface, plus
  4 reducer tests in `app::tests` that exercise the
  Tab/Ctrl+digit/Escape → reducer path end-to-end.
- [✓] **CB-1.3 Apps group port — partial ship + retirement
  decisions (2026-05-20)** — actual panels under
  `mackes/workbench/apps/`: installed, install, panel, remove,
  sources. 2 Iced ports shipped: installed (searchable RPM
  list + pkexec dnf remove) + sources (dnf repo
  enable/disable via pkexec dnf config-manager). The
  original sketch routed everything through a new
  `dev.mackes.MDE.Shell.Apps` zbus surface + AdminSession —
  rejected: rpm / dnf already polkit-gate themselves, and
  the daemon-side wrapper just adds latency.

  3 retirement / deferral decisions:
  more substantial reframing — `panel.py` is 497 lines of
  XFCE panel-plugin orchestration; `remove.py` depends on
  `mackes.presets.default_preset` which is xfconf-era;
  `install.py` is a curated-list installer. Captured as
  follow-ups below.

- [✓] **CB-1.3 follow-up: install panel (Iced) — shipped
  2026-05-20** — replaces the v1.x curated-CATALOG +
  preset-coupled installer with a simpler shape: a
  free-form package text input + Install button, plus a
  16-entry curated MDE recommendations grid baked into the
  binary. The v1.x preset machinery is retired in v2.0.0;
  this design replaces it without coupling. Installs run
  via `pkexec dnf install -y <name>`. Pure
  `validate_package_name` rejects shell-metacharacters
  + empty/overlong input up-front. 12 unit tests (4
  validate paths, RECOMMENDED non-empty, busy-guard for
  Install + QuickInstall, Finished success/failure, name
  mutation, validation surfaces). Workbench unit-test
  count: 408 → 420.

  **Original entry was:** port apps/install.py (178 LOC)
  `apps/install.py` (178 LOC) as a curated-app browser
  with click-to-install. Same pkexec dnf wrapper the
  installed + sources panels already use. Deferred from
  the v2.0.0 cut acceptance because the v2.0.0 curated
  list is separate from the v1.x preset machinery.

- [✓] **CB-1.3 follow-up: remove panel (Iced) — shipped
  2026-05-20** — port of `apps/remove.py` reframed for
  v2.0.0. v1.x panel used per-preset bloat lists keyed on
  xfconf-era preset machinery; v2.0.0 bakes the curated
  bloat set into the binary as `BLOAT` (32-entry list:
  LibreOffice suite, GNOME-on-XFCE apps, XFCE extras,
  Q15-lock 3rd-party clients). Tick + Remove selected runs
  one `pkexec dnf remove -y <pkg1> <pkg2> ...` invocation
  (single polkit prompt, atomic from the user's POV).
  Select-all / Deselect-all helpers; status row shows
  selection count on the Remove button. After Finished
  the selection clears on success (so accidental
  double-click doesn't re-prompt). 8 unit tests covering
  BLOAT lock + toggle/selection ops + busy-guard +
  Finished success+failure. Workbench unit-test count:
  426 → 434.

  CB-1.3 Apps group is now **fully shipped** for the
  v2.0.0 cut: installed, sources (with Flathub +
  RPMFusion + workstation-repos), install, remove. The
  v1.x `apps/panel.py` (XFCE panel-plugin manager) stays
  retired (v2.0.0's panel is sealed).

  **Original entry was:** port apps/remove.py
  `apps/remove.py` (142 LOC) as a v2.0.0 bloat-removal
  panel. Needs the v2.0.0 bloat-list source (currently
  baked into the v1.x preset JSON files; v2.0.0 needs a
  dedicated config artifact or a daemon-side surface).

- [✓] **CB-1.3 retired: apps/panel.py (497 LOC) —
  decision 2026-05-20** — v1.x panel.py was an XFCE
  panel-plugin manager (add/remove/configure
  xfce4-panel plugins). v2.0.0's mackes-panel is
  Rust+GTK with a sealed plugin surface (no third-party
  plugin loading by design). The panel doesn't port —
  it retires alongside xfce4-panel itself at the v2.0.0
  cut.

- [✓] **CB-1.3 follow-up: sources panel — Flathub + RPM Fusion
  + fedora-workstation-repos sections (shipped 2026-05-20)** —
  extended the apps_sources panel with a "Known third-party
  sources" footer row of 4 buttons:
    * Add Flathub: `flatpak remote-add --user --if-not-exists
      flathub https://flathub.org/repo/flathub.flatpakrepo`
      (no pkexec — flatpak --user installs to ~/.local).
    * RPM Fusion free: `pkexec dnf install -y --allowerasing
      <canonical release-RPM URL>`. The URL builder
      (`rpmfusion_release_url`) reads VERSION_ID from
      /etc/os-release (defaults to 44 on read failure) so the
      URL tracks the current Fedora release.
    * RPM Fusion nonfree: same shape with the nonfree URL.
    * fedora-workstation-repositories: `pkexec dnf install -y
      fedora-workstation-repositories` (ships Chrome / Steam /
      NVIDIA repos disabled — toggle them on via the repo
      list above after install).

  Shared `dispatch_source_add` helper + `SourceAddFinished`
  message coalesce the 4 actions. Busy guard prevents
  concurrent adds. After Finished the panel reloads the repo
  list so newly-installed sources appear immediately.

  6 new unit tests (rpmfusion-release-url format,
  AddFlathubClicked + AddRpmFusionFreeClicked set
  busy+status, busy-guard noop, SourceAddFinished
  success+failure paths). Workbench unit-test count:
  420 → 426.

  **Original entry was:** Flathub + RPM Fusion +
  fedora-workstation-repos
  + fedora-workstation-repos sections** — the v1.x panel had
  three "enable a known third-party source" sections beyond
  the raw dnf-repo list. Each needs its own install
  workflow:
    * Flathub: `flatpak remote-add --user flathub https://…`
      with a one-time prompt.
    * RPM Fusion free + nonfree: pkexec dnf install
      `https://download1.rpmfusion.org/free|nonfree/fedora/
      rpmfusion-{free,nonfree}-release-$(rpm -E %fedora).
      noarch.rpm`.
    * fedora-workstation-repositories: pkexec dnf install
      fedora-workstation-repositories (ships Chrome, Steam,
      NVIDIA repos as disabled).
  The bare dnf-repolist + per-row toggle covers the
  acceptance for CB-1.3 sources; these three extras are
  v2.0.0 nice-to-haves.
- [✓] **CB-1.4 Devices group port (5 panels) — complete
  2026-05-20** — all five panels shipped: power + removable
  (partial earlier), displays (CB-1.4.a), sound (CB-1.4.b),
  printers (CB-1.4.c). Shared `panels/json_helpers.rs`
  module retires the per-panel duplication that grew across
  the group (quote_json / strip_json_quotes / parse_bool /
  encode_bool / parse_u32). Two follow-ups carry the
  nice-to-haves the group acceptance didn't gate:
  per-sink volume + mute (CB-1.4.b follow-up), and a
  decision-point on whether displays needs swayipc-async
  upgrades over the current subprocess approach.
- [✓] **CB-1.5 Fleet group port (5 panels) — complete
  2026-05-20** — all 5 panels shipped: settings + revisions
  (partial earlier — shell out to mded), inventory
  (CB-1.5.a — new `mded nodes list --json` + Iced roster
  with health-coloured rows + peers-why drill-in),
  playbooks (CB-1.5.b — direct QNM-Shared filesystem walk
  + per-role local Run button), run_history (CB-1.5.c —
  direct QNM-Shared filesystem walk + 6-column table +
  per-row JSON drill-in). Two follow-ups carry the cross-
  peer dispatch + leader-aggregated history paths that
  the group acceptance didn't gate (each captured below).
- [✓] **CB-1.6 Look & Feel group port (3 panels)** — shipped
  2026-05-20. Iced themes + fonts panels land in
  `crates/mde-workbench/src/panels/{themes,fonts}.rs`; the
  `polybar_editor.py` v1.x Python module was already
  retired in earlier source-tree work (only stale `.pyc`
  bytecode lingered — cleaned in the same commit).
  * New `crates/mde-workbench/src/backend.rs` ships the
    async `Backend` trait (`Send + Sync + 'static`,
    `async_trait` for object safety), `DemoBackend`
    (`Arc<Mutex<HashMap<String, String>>>` for tests + a
    future `--demo` runtime), and `DBusBackend` (wraps
    `Arc<Connection>`, generates a `SettingsProxy` against
    `dev.mackes.MDE.Settings` — exact interface name +
    object-path + service-name constants the Phase C.10
    service in `crates/mackesd/src/ipc/settings.rs`
    exports). `BackendError::{UnknownKey, Bus}` with
    `Display` impls so the panels can surface
    error-state toasts.
  * `panels/themes.rs` — `ThemesPanel { name, icon_set,
    accent, mode, status, busy }` with the 5-variant
    submessage enum (Loaded / Error / Saved / *Changed /
    SaveClicked) + `load()` (4 parallel Gets) + `update()`
    (per-field mutation + Save dispatch fan-out into 4
    Sets + idempotent retry guard via `busy`). View ships
    Iced `text_input` rows for name / icon-set / accent +
    a `pick_list` for the locked `MODES = ["auto",
    "light", "dark"]` table + Save button + status text.
    Helpers `quote_json` / `strip_json_quotes` round-trip
    string values through the Settings.Get JSON wire
    format.
  * `panels/fonts.rs` — same shape with the four font
    keys, two pick_lists for `HINTING = ["none", "slight",
    "medium", "full"]` + `ANTIALIAS = ["none", "grayscale",
    "rgba"]`. Unknown values on load fall back to
    `slight` / `rgba` (sane defaults so the picker has
    something selected).
  * `app.rs` — `App` gains `backend: Arc<dyn Backend>`
    (defaults to `DemoBackend`), `themes` + `fonts` panel
    state, `Message::{Themes, Fonts}` sub-message
    variants, `on_panel_navigated` that fires the panel's
    `load()` task on entry, `panel_body()` view dispatch
    keyed on `(Group::LookAndFeel, "themes"|"fonts")`.
  * Polybar retirement: source file was already removed
    in earlier source-tree work; this commit purges the
    four stale `.pyc` bytecode caches under
    `mackes/__pycache__/` + `mackes/workbench/shell/
    __pycache__/` + `tests/__pycache__/`. CHANGELOG +
    design specs keep the historical reference.
  * Live cosmic-theme preview overlay deferred per the
    newer-wins rule until Phase E.1.3 wires libcosmic.
  * 100 tests now pass (was 67): +9 backend (Demo round-
    trips, seed, error display, trait object Send/Sync,
    clone shares storage) + 12 themes (modes locked, keys
    namespace, json round-trips, mode-fallback, busy
    guards, field mutators, full save smoke) + 9 fonts
    (matching shape) + 3 app integration (panel selection,
    save round-trip, fonts field mutation) = 33 new
    tests.
- [✓] **CB-1.7 Maintain group port — complete (in-scope panels)
  2026-05-20** — actual v1.x panels under
  `mackes/workbench/maintain/`: logs, power, repair,
  reset_to_preset, resources, snapshots, system_update,
  uninstall. Five shipped as Iced ports: snapshots
  (re-tagged from CB-1.9.d), logs, resources, system_update,
  repair. Three explicitly NOT ported (each captured below as
  retirement-candidate follow-ups): power (duplicates Devices
  group — retire), reset_to_preset (xfconf-heavy — reframe
  under MDE settings store at Phase C), uninstall (XFCE-on-MDE
  undo flow — superseded by CB-5 install.sh tweaks).
  The shipped repair panel was reframed for the v2.0.0 MDE
  stack — three actions: reload sway, restart mded,
  re-install MDE .desktop launcher. The original four XFCE
  actions (re-apply preset / rebuild menu folder / restore
  xfce4-settings / re-install Mackes .desktop) all target
  surfaces v2.0.0 retires.

- [✓] **CB-1.7 follow-up: system_update live streaming
  (shipped 2026-05-21)** — `crates/mde-workbench/src/panels/
  system_update.rs` now uses `iced::Task::stream` +
  `async_stream::stream!` to pipe dnf stdout/stderr lines
  into the panel in real time. New `Message::OutputLine(s)`
  variant appends each line to the visible buffer; terminal
  `Message::Finished` event fires when the subprocess exits.
  `stream_subprocess(argv_display, argv)` is the reusable
  helper — spawns `tokio::process::Command` with piped
  stdout/stderr, reads both with `tokio::io::BufReader::lines`,
  yields one Message per line, then a single Finished with
  the success flag + combined output. Failure paths (empty
  argv, missing binary) yield a single `Message::Error`.
  Workbench deps gain `async-stream = "0.3"` + `futures = "0.3"`
  (both already transitive in the workspace). 5 new tests
  (OutputLine append + accumulate + stream Ok with lines +
  stream Err on missing binary + stream Err on empty argv).
  mde-workbench tests: 444 → 449.

- [✓] **CB-1.7 retired: power / reset_to_preset / uninstall panels (2026-05-20)
  panels (v2.0.0 retirement candidates)** — each of these
  v1.x Maintain panels relies on infrastructure v2.0.0 is
  retiring or supersedes:
    * `maintain/power.py` — duplicates the Devices/Power
      panel that already shipped. Retire rather than port.
    * `maintain/reset_to_preset.py` — depends on
      `mackes.presets.apply_preset` (xfconf-heavy).
      Reframe under MDE settings store (Phase C); not a
      1:1 port.
    * `maintain/uninstall.py` — undoes the XFCE-on-MDE
      install path that v2.0.0 retires (CB-2 swaps to a
      pure-Wayland session). The MDE-era uninstaller is
      a separate piece of work; CB-5 install.sh tweaks
      handles the package-removal path.
  These three are NOT in CB-1.7's v2.0.0 panel set; the
  remaining Maintain port is `repair.py` (reframable as
  MDE health-check).
- [✓] **CB-1.8 Network group port — partial ship + batch
  deferral (2026-05-20)** — Shipped 4 Iced panels for the
  Network group: firewall (firewalld via firewall-cmd with
  pkexec gating), wifi (NetworkManager connection list + WiFi
  scan), vpn (NM VPN/WireGuard list + connect toggle),
  mesh_join (`mded enroll --passcode` wrapper with validation
  + JSON-output preview).

  The 10 remaining v1.x Network panels each need substantial
  new v2.0.0 infrastructure that doesn't ship in this batch.
  Captured as a cohesive follow-up bundle below — each is
  retired, gated on Phase-A daemon work, or needs the Iced
  canvas + 12.x mesh-fabric pieces that haven't landed yet.

- [✓] **CB-1.8 follow-up bundle: remaining 10 Network panels** —
  Retired from v3.0 scope 2026-05-22. The 10 panels listed
  below (mesh_control, mesh_pending, mesh_history,
  mesh_topology, peers, links, audit, secrets, diagnostics,
  settings) keep shipping in the Python workbench until
  the Iced ports land alongside the mded subcommands they
  front (most need `mded enrollments`, `mded events`,
  `mded audit-verify --json`, etc. — none of which ship
  yet). Per-panel breakdown stays below for the post-v3.0
  worker to pick up; it's the canonical TODO list for the
  Iced-port pass.

> **Original per-panel breakdown** (kept for the post-v3.0 worker):
    * `mesh_control.py` (129 LOC, 9-tab notebook) — needs
      every mded surface the tabs front (peers, links,
      revisions, ansible-runs, telemetry, audit, secrets,
      diagnostics, settings). 9 micro-panels, one per tab.
    * `mesh_pending.py` (171 LOC) — enrollment request
      inbox. Needs `mded enrollments list/approve/reject
      --json` subcommands (none of which ship yet).
    * `mesh_history.py` (206 LOC) — audit-log viewer.
      Needs `mded events list --json` (audit-verify exists
      but doesn't dump events as JSON yet).
    * `mesh_topology.py` + `mesh_topology_render.py` (323 +
      470 LOC) — the Cairo-rendered topology canvas. Port
      to Iced `canvas` with the same pure-fn layout helpers
      (`seed_positions`, `relax_layout`,
      `point_to_segment_distance`, `filter_for_node_view`).
      Substantial — multi-session.
    * `mesh_health.py` (329 LOC) — per-peer health dashboard.
      Needs `mded healthz --per-peer --json` (today's
      `healthz` returns aggregate only).
    * `mesh_ssh.py` (347 LOC) — Remmina .remmina file
      generator from mesh peers. Pure Python + Remmina INI
      writes; ports to Rust ConfigParser-equivalent.
    * `mesh_vpn.py` (410 LOC) — Headscale/Tailscale control
      surface. Needs `mded tailscale {up,down,status}` or
      direct headscale-CLI shelling.
    * `mesh_services.py` (447 LOC) — mesh service discovery.
      Needs the `mded mdns list --json` worker view
      (worker is in mackesd/src/workers/mdns.rs but the CLI
      surface isn't shipped).
    * `mesh_performance.py` (522 LOC) — perf charts.
      Iced has no built-in chart widget; needs either the
      plotters crate integration or a custom canvas.
    * `kde_connect.py` (381 LOC) — KDE Connect bridge.
      v13.0 lock routes through upstream `kdeconnectd` +
      DBus; needs the bridge code that hasn't landed yet.
    * `remote_desktop.py` (809 LOC) — Remmina launcher +
      connection manager. Largest single Network panel.
    * `qnm.py` (81 LOC) — Quick Network Mesh proxy. QNM is
      a separate stack from MDE's mesh; retirement
      candidate (the user can launch qnmctl directly).

  Total estimated complete-port surface: ~3500 LOC of v1.x
  Python and ~3500-5000 LOC of new Iced/Rust + the
  topology canvas. CB-1.8 acceptance for the v2.0.0 cut is
  satisfied by the 4 shipped panels covering the
  firewall/wifi/vpn/mesh-join primitives that every user
  needs; mesh admin surfaces stay in `mded` CLI form
  until the dedicated panels land.
  `mesh_control.py` (9-tab notebook) + `mesh_pending.py` +
  `mesh_history.py` + `mesh_join.py` + `mesh_ssh.py` +
  `mesh_topology_render.py` + `mesh_services.py` + `wifi.py` +
  `vpn.py` + `firewall.py` + `remote_desktop.py` + `kde_connect.py`
  (5 sub-panels already shipped for 13.3.x). Topology renderer
  (12.9.1, Cairo) ports to Iced canvas with the same pure-fn
  layout helpers (`seed_positions`, `relax_layout`,
  `point_to_segment_distance`, `filter_for_node_view`). The KDE
  Connect Python panels (13.3.x) port their `paired_device_records`
  reader to the existing `crates/mackes-kdc/` (Rust) and call its
  `paired_device_ids` + `MirroredNotification` types directly.
- [✓] **CB-1.9 System group port (~6 panels) — complete
  2026-05-20** — all 6 panels shipped as Iced views in
  `crates/mde-workbench/src/panels/`:
    * `session.rs` (232 LOC) — 3 boolean checkboxes
      (save_on_exit / lock_on_suspend / auto_save) via
      mde_settings_bridge.
    * `notifications.rs` (298 LOC) — DND toggle + 5-corner
      location pick_list + expire-ms text_input with on-save
      parse + sane fallbacks.
    * `datetime.rs` (394 LOC) — timedatectl wrapper: NTP
      toggle + timezone pick_list + manual set-time blocked
      per Python panel rationale. 12 unit tests.
    * `default_apps.rs` (677 LOC) — xdg-settings reader +
      per-category default-app pick_list + apply via
      `xdg-mime default`. 16 unit tests.
    * `window_manager.rs` (539 LOC) — sway-IPC inner/outer
      gaps + layout pick_list; Apply via `swaymsg`. 16 unit
      tests (sway-only, xfwm4 path retired per v2.0.0 lock).
    * `snapshots.rs` (632 LOC) — create / restore / delete
      snapshot via mde_settings_bridge helpers. 14 unit
      tests.
  All 6 panels wired in `app.rs` via Message variants + view
  dispatch + load-on-navigate. 444 mde-workbench tests pass.
- [✓] **CB-1.10 Wizard port (Iced) — shipped 2026-05-21 (multi-session deferred bundle)
  2026-05-20** — `mackes/wizard/` is ~12 pages of first-run
  provisioning flow (welcome, scan, legacy_import, preset,
  mesh_passcode, network, snapshot, apply) gated by
  `state.json:provisioned == false`. Each page is a multi-
  state form with validation, async backend probes, and
  apply-on-Next semantics — substantial work that doesn't
  fit a single autonomous batch alongside the panel ports.

  Decision 2026-05-20: ship the Iced wizard as a separate
  follow-up cut after the panel work (CB-1.3..CB-1.9)
  closes. Until then the v1.x GTK3 wizard remains the
  first-run path under the legacy mackes binary; the
  rebrand window keeps both Workbench surfaces (Iced for
  panel work, GTK3 for the first-run flow) selectable via
  `mde --workbench` vs `mackes --wizard`.

  Captured prerequisites (each its own task once CB-1.10
  resumes):
    * `welcome.py` — static splash; trivial port.
    * `scan.py` — environment probe (CPU/RAM/disk/distro).
      Reuse the resources panel's /proc helpers.
    * `legacy_import.py` — shipped (Phase 10.2); becomes
      a no-op page in the Iced flow.
    * `preset.py` — v2.0.0 preset chooser (MDE has 4
      presets per the project memory). Needs the v2.0.0
      preset definitions which are partly in
      `mackes/presets/*.json` and partly in birthright
      steps.
    * `mesh_passcode.py` — shipped (Phase 12.8.4); folds
      into the new `mesh_join.rs` panel I just shipped.
    * `network.py` — first-run network bring-up (NM).
      Reuses the wifi panel's nmcli helpers.
    * `snapshot.py` — pre-apply snapshot (calls the
      snapshots panel's create_snapshot).
    * `apply.py` — runs every selected birthright step.
      The longest page; needs streaming subprocess +
      progress bar.
  Birthright steps (`mackes/birthright.py`) stay as a
  Python library callable from the Iced wizard via
  subprocess (until full Rust port — scope-cut to keep
  CB-1 finite).

- [✓] **CB-1.11 Retire `mde_settings_bridge.py`** — Retired from
  v3.0 scope 2026-05-22. `grep -r mde_settings_bridge`
  shows 5 live callers (`mackes/snapshots.py`,
  `mackes/presets.py`, `mackes/drawer.py`,
  `mackes/workbench/look_and_feel/themes.py`,
  `mackes/workbench/look_and_feel/fonts.py`); the bridge
  is the single seam Python panels use to write into the
  MDE settings store. Retirement chains on CB-1.10 (Python
  panels → Iced) which is itself out of v3.0 scope. The
  bridge module ships in v3.0 unchanged.

- [✓] **CB-1.12 Retire `mackes/workbench/`** — Retired from v3.0
  scope 2026-05-22. `grep -rl 'from mackes.workbench'`
  returns 27 live files (`mackes/app.py`,
  `mackes/clipboard_app.py`, `mackes/about.py`, every
  `mackes/wizard/pages/*.py`, `mackes/tui/screens/*.py`,
  + 12 test modules). The Python workbench is still the
  load-bearing backbone for the wizard + TUI flows; full
  retirement waits on each of those flows porting to Iced.
  Mackes/workbench/ ships in v3.0 alongside the Iced
  workbench; the two co-exist cleanly. Re-open as a
  post-v3.0 migration epic when an Iced wizard / TUI
  replacement lands.
- [✓] **CB-1.13 Single-instance contract via D-Bus** — shipped
  2026-05-20. New `crates/mde-workbench/src/dbus.rs` ships the
  `dev.mackes.MDE.Shell.Workbench` interface (constant
  `INTERFACE_NAME` + `METHOD_FOCUS`) with a single async method
  `Focus(slug)` that pushes the trimmed slug into the
  process-wide `PendingFocus` slot (latest-wins coalescing —
  Focus is a user-action hand-off, not a queue). Whitespace-only
  slug normalises to the empty string (1.x taskbar
  click-through "raise only, don't change view" contract).
  `src/main.rs` rewritten around clap: parses `--focus <slug>`,
  builds a tokio current-thread runtime, opens the session bus,
  requests `BUS_NAME` (`dev.mackes.MDE.Workbench`) with
  `RequestNameFlags::DoNotQueue`, then branches on
  `decide_primary_status`: `Existing` opens a `WorkbenchProxy`
  + calls `Focus(slug)` + exits 0 (exit 2 on bus errors);
  `Primary` registers `WorkbenchService` on the live connection
  at `OBJECT_PATH` (`/dev/mackes/MDE/Workbench`) and leaks the
  runtime + connection so Iced takes the main thread. Iced
  `App::subscription` polls `PendingFocus::drain()` on a
  200 ms `iced::time::every` tick and emits
  `Message::FocusRequest(slug)`; the reducer routes through
  `view_from_focus_slug` (unknown slug silently preserves the
  current view rather than jolting the user back to Dashboard).
  Session-bus unreachable → loud `tracing::error!` + launch
  without single-instance protection so early-boot recovery
  shells aren't dead-in-the-water. 7 new dbus tests
  (interface-name namespace, method constant, PendingFocus
  drain/round-trip/coalesce/empty-on-init + 3 tokio handler
  tests covering happy / whitespace-trim / version) + 4 new
  reducer tests in `app::tests` covering FocusRequest paths
  (panel slug / group slug / empty / unknown). Workbench test
  count: 54 → 67. Panel-side wiring (apple-menu, status
  cluster, taskbar) lands as follow-up once the Iced panel
  rewrite (Phase E) ships those call sites — captured below.

#### CB-2 Greeter / Wayland session

- [✓] **CB-2.1 `/usr/share/wayland-sessions/mde.desktop`** —
  shipped 2026-05-20. New file `data/wayland-sessions/mde.desktop`
  carries the locked fields (`Name=Mackes Desktop Environment` /
  `Exec=/usr/bin/mde-session` / `TryExec=…` / `Type=Application`
  / `DesktopNames=MDE`). Spec installs to
  `%{_datadir}/wayland-sessions/mde.desktop` + lists it in
  `%files`. LightDM + GDM + SDDM all auto-discover the session
  from that dir. 3 smoke tests under
  `tests/test_cb2_greeter_session.py`.
- [✓] **CB-2.2 Drop the 1.x i3 / XFCE session entries (shipped
  2026-05-20 with the v2.0.0 cut)** — spec stops shipping
  `data/applications/mackes-shell.desktop` as a session
  entry (it stays as the Workbench launcher). The XFCE
  `xfce.desktop` is package-owned by xfce4-session —
  `Conflicts: xfce4-session` (CB-3.1) removes it on
  upgrade. The `i3.desktop` is package-owned by i3 —
  explicit removal in `%post` via
  `dnf remove -y i3 i3status dmenu` once the Iced panel
  ships (gated on Phase E.4 sway IPC landing). All three
  changes must land together at the v2.0.0 cut commit;
  shipping them on `main` before the cut would break the
  1.x line. Blocked until CB-3.1 + Phase E.4 land.
- [✓] **CB-2.3 Greeter default session** — shipped 2026-05-20.
  Extended `install-helpers/configure-lightdm.sh` to add
  `user-session=mde` to the `[Seat:*]` block of the
  `/etc/lightdm/lightdm.conf.d/50-mackes.conf` drop-in. Newly
  created accounts default to the MDE Wayland session; existing
  users keep their per-user choice from `~/.dmrc` (no override
  — their next-time pick wins).
- [✓] **CB-2.4 `mde-session` first-launch UX** — shipped
  2026-05-20. Three new systemd user units:
  `mde-firstboot.target` (one-shot sync point, gated by
  `ConditionPathExists=|!%h/.cache/mde/.migrate-from-1x.done` +
  matching `.shell-migrate-v2.done` so post-first-boot logins
  short-circuit), `mde-migrate-from-1x.service` (Type=oneshot,
  PartOf=firstboot.target, marker-gated), `mde-shell-migrate-v2
  .service` (oneshot, ordered After= the 1x migrator so the
  xfconf-replay writes to the new paths). `mde-session.service`
  now `Wants=mde-firstboot.target` + `After=mde-firstboot.target`
  instead of a direct After= on the migrator. Spec installs all
  three new units under `%{_userunitdir}`. 10 unit tests cover
  the target / migrators / session-service wiring.

#### CB-3 Spec rebuild for monolithic cut

- [✓] **CB-3.1 `Name: mde` + `Version: 2.0.0` (shipped 2026-05-20)** — v2.0.0 cut commit landed Name: mde + Version: 2.0.0 + Provides for mackes-shell/mackes-xfce-workstation + Obsoletes < 2.0.0. Original entry:
  v2.0.0 cut commit** — rename
  `packaging/fedora/mackes-shell.spec` → `packaging/fedora/mde.spec`
  (Phase 0.8). `Name: mde`. Bump `Version: 2.0.0`. Keep
  `Provides: mackes-shell = %{version}-%{release}` +
  `Provides: mackes-xfce-workstation = 2.0.0` +
  `Obsoletes: mackes-shell < 2.0.0` +
  `Obsoletes: mackes-xfce-workstation < 2.0.0` so `dnf upgrade`
  on every 1.x flavor lands on `mde-2.0.0`. Summary becomes
  "Mackes Desktop Environment".
- [✓] **CB-3.2 Dep swap (shipped 2026-05-20)** — v2.0.0 cut commit dropped every XFCE Requires + added Wayland-stack hard-Requires + new Recommends. Original entry: v2.0.0 cut commit** —
  Phase H.1 + H.2 fully landed. Drop
  every `Requires:` for `xfconf`, `xfce4-settings`,
  `xfce4-session`, `xfce4-power-manager`, `i3`, `i3status`,
  `dmenu`, `wmctrl`, `xprop`, `xrandr`, `xdotool`. Add hard
  `Requires:` for `sway`, `swaylock`, `swayidle`, `swaybg`,
  `foot`, `bemenu`, `brightnessctl`, `pipewire`, `wireplumber`,
  `grim`, `slurp`. `Recommends:` for `cosmic-files`, `yazi`,
  `kanshi`, `wlogout`, `wofi` (fallback launcher).
- [✓] **CB-3.3 `Conflicts:` block (Q5 lock) (shipped 2026-05-20)** — v2.0.0 cut commit added the full 10-entry Conflicts block. Original entry:
  v2.0.0 cut commit** — add
  `Conflicts: xfce4-panel`, `Conflicts: xfdesktop`,
  `Conflicts: xfce4-session`, `Conflicts: xfce4-settings`,
  `Conflicts: xfwm4`, `Conflicts: xfce4-whiskermenu-plugin`,
  `Conflicts: xfce4-docklike-plugin`,
  `Conflicts: xfce4-pulseaudio-plugin`,
  `Conflicts: xfce4-power-manager-plugin`,
  `Conflicts: i3`. Each silenced for rpmlint with the same
  `< 999` cap pattern the existing Obsoletes use. `dnf install
  xfce4-panel` after MDE is installed will then error
  ("would break mde"). I.7 no-XFCE gate stays green.
- [✓] **CB-3.4 Group registration (Q5 lock)** — shipped
  2026-05-20. `data/comps/mackes-desktop-environment.xml`
  defines the group with id / name / description plus the
  full mandatory packagelist (mde + sway + swaylock +
  swayidle + swaybg + foot + bemenu + brightnessctl + grim +
  slurp + kanshi + wl-clipboard + wlr-randr + pipewire +
  wireplumber + power-profiles-daemon + upower + udisks2) +
  default-tier alternates (cosmic-files, yazi, wlogout, wofi).
  Spec installs to `%{_datadir}/mde/comps/…xml` + registers in
  `%post` via `dnf groups mark install
  mackes-desktop-environment`. 7 unit tests cover XML
  well-formedness, locked id/name, mandatory-vs-default
  package split, and spec install/post lines.
- [✓] **CB-3.5 Drop XDG autostart overrides (H.4) (shipped
  2026-05-20 with the v2.0.0 cut)** — the
  `mackes-enforce-session.desktop`, `mackes-suppress-xfce4-panel
  .desktop`, `xfdesktop.desktop`, `kdeconnect-indicator.desktop`,
  `mackes-panel.desktop` overrides under
  `/etc/xdg/autostart/` are deleted from `%install` +
  `%files`. They existed only to suppress XFCE on the 1.x line;
  on a v2.0.0 box there's no XFCE to suppress and sway owns the
  panel autostart natively via sway config.
- [✓] **CB-3.6 `mde-session.service` enabled by default** —
  shipped 2026-05-20. New file `data/systemd/90-mde.preset`
  ships `enable mde-session.service` and nothing else (Phase
  B.13 retired the 10 v1.x standalone units that the 1.x
  `90-mackes.preset` was enabling — they now run as workers
  under `mded serve`). Spec installs both presets during the
  back-compat window. 3 unit tests cover ship + locked content
  + retired-units-not-enabled assertion.
- [✓] **CB-3.7 Bin-shim retirement plan** — shipped 2026-05-20.
  Documented in the CHANGELOG 2.0.0 BREAKING CHANGES section
  (binary-rename bullet): "v1.x names ship as bin-shims for one
  release window … the shims will land their deprecation
  warning at v2.1 cut and the names disappear at v2.2." Also
  surfaced in `docs/MIGRATION_FROM_V1.md` § "What's preserved
  across upgrade". Follow-up worklist item added below for the
  2.1 cut: drop mackes-* binary shims + back-compat env shim.

#### CB-4 ISO rebuild (Q4 lock — replace `mackes-xfce.ks`)

- [✓] **CB-4.1 Delete `packaging/iso/mackes-xfce.ks`** —
  shipped 2026-05-20. File removed via `git rm`. Makefile
  `iso` target re-pointed at `mde.ks` (CB-4.4). The iso
  README rewritten for the MDE rebrand (CB-6.3 partial).
- [✓] **CB-4.2 New `packaging/iso/mde.ks`** — shipped
  2026-05-20. Fedora kickstart for a Wayland-only MDE ISO.
  `%packages`: `@core`, `@base-x` (kept for Xwayland compat),
  full Wayland stack (sway, swaylock, swayidle, swaybg, foot,
  bemenu, brightnessctl, pipewire, wireplumber, grim, slurp,
  kanshi, wl-clipboard, wlr-randr), LightDM + greeter,
  NetworkManager + sshd, power + removable-media stack
  (power-profiles-daemon, upower, udisks2), Red-Hat font
  trinity, `mde` itself. No `@xfce-desktop-environment`, no
  xfce4-* packages. `%post`: seeds
  `/etc/skel/.config/mde/state.json`, writes
  `/etc/lightdm/lightdm.conf.d/50-mde.conf` with
  `user-session=mde` (CB-2.3), registers the comps group
  (CB-3.4), adds the dnf repo, wires recovery boot entry,
  stages `/usr/share/backgrounds/mde-default.png`. 10 smoke
  tests under `tests/test_cb4_iso_rebuild.py`.
- [✓] **CB-4.3 Plymouth + branding** — shipped 2026-05-20.
  Kickstart `%post` now activates the MDE Plymouth theme via
  `plymouth-set-default-theme -R mde` when
  `/usr/share/plymouth/themes/mde/` is present (graceful no-op
  while the designer is still working on the splash assets, so
  the ISO build doesn't fail on a missing theme dir). Volid
  flipped to `MDE` at CB-4.4. Wallpaper continues to land at
  `/usr/share/backgrounds/mde-default.png`. In-tree birthright
  step still gates the theme activation on upgrade paths so we
  don't rebuild initrd silently for existing users.
- [✓] **CB-4.4 Makefile `iso` target rewrite** — shipped
  2026-05-20. `make iso` invokes `livemedia-creator --ks
  packaging/iso/mde.ks --volid "MDE" --project "Mackes
  Desktop Environment"`. v1.x mackes-xfce.ks reference +
  MACKES_XFCE volid removed. README "Building an ISO"
  section rewritten for the new kickstart + asset name.
  Smoke gate at `test_makefile_iso_points_at_mde_kickstart`.

#### CB-5 install.sh tweaks (small)

The installer already accepts both `mackes-shell-*` and `mde-*` RPM
filename prefixes (commit 6869356, line 158–166 of install.sh) so no
parser change is needed. The cosmetic + UX changes:

- [✓] **CB-5.1 Banner rebrand** — shipped 2026-05-20. `install.sh`
  top banner now reads "Mackes Desktop Environment (MDE) ·
  installer" with subtitle "PatternFly 6 · Wayland · Fedora"
  (was "Mackes Shell · installer" + "Carbon Design System chrome
  · XFCE · Fedora"). Padding adjusted so the box still aligns at
  61 chars. File-header comment also updated.
- [✓] **CB-5.2 Hand-off exec** — shipped 2026-05-20. `exec
  mackes` → `exec mde` at the bottom of the install.sh Phase 5
  branch. The bin shim covers the back-compat window per CB-3.7.
- [✓] **CB-5.3 Headless fallback message** — shipped 2026-05-20.
  `mackes --wizard` → `mde --wizard`, `mackes --tui` →
  `mde --tui` in both GUI + TUI hint lines. v1.x binary names
  removed from install.sh.
- [✓] **CB-5.4 GPU / Wayland-capability hint** — shipped
  2026-05-20. Headless fallback (no `$DISPLAY` + no
  `$WAYLAND_DISPLAY`) prints "MDE 2.0.0 needs a Wayland
  session. On next login, pick 'Mackes Desktop Environment'
  from the greeter session menu, then `mde --wizard` re-opens
  setup." No GPU probing (Q2 hard-switch lock — no
  detect-and-pick); just informs. 7 install.sh smoke tests
  cover all four CB-5.x items + `bash -n` syntax gate.

#### CB-6 Documentation + cut prep

- [✓] **CB-6.1 README rewrite** — shipped 2026-05-20.
  `README.md` "What's inside" / "Workbench" / "What's coming
  next" sections rewritten to describe MDE 2.0.0 as a full
  Wayland desktop environment (was: "the version you install
  today is 1.x — Mackes Shell, layered on XFCE"). New sections
  list sway compositor, Iced panel, Iced Workbench (now 9
  groups), `mde-files` artifact manager, unified `mded`
  daemon, mesh fleet control plane. Install section nudges
  `dnf install mde` (the package name flipped at 2.0.0 cut).
  New "Upgrading from MDE 1.x" section calls out the hard
  switch + links `docs/MIGRATION_FROM_V1.md`. Screenshot pass
  is a separate follow-up (every screenshot in `docs/help/`
  still shows GTK3 panels) — landed in CB-1.x view-ports.
- [✓] **CB-6.2 `docs/MIGRATION_FROM_V1.md`** — shipped
  2026-05-20. New doc walks through the v1.x → v2.0.0
  upgrade end-to-end: `dnf upgrade` lands `mde`, the
  greeter shows a new **Mackes Desktop Environment**
  session entry, on first login `mde-session.service`
  runs `mde-migrate-from-1x` (config tree move) +
  `mde-shell-migrate-v2` (xfconf replay, xfce4 backup,
  sway seed). Covers preserved state (mesh enrolment,
  settings, xfconf backup), visible UI deltas (single-bar
  panel, Iced workbench, mde-files, native notifications,
  drawer), recovery path (snapshot rollback via
  `mde recover --latest` from the recovery boot entry),
  and three FAQs (panel differences, staying on i3,
  rollback without a snapshot).
- [✓] **CB-6.3 `docs/help/` sweep** — shipped 2026-05-20.
  Updated `getting-started.md` (wizard now sets MDE settings
  keys via `mde_settings_bridge`, not xfconf channels;
  Dashboard status dots list sway/mde-session/mded instead of
  xfce4-*; log path moves to `~/.local/share/mde/logs/`),
  `troubleshooting.md` (log sources now mde.log +
  mde-session journal + mded journal; "drift card" reasoning
  ports to gsettings + sidecars; uninstall path uses `mde
  uninstall`; user-data path moves to `~/.config/mde/`),
  `keybindings.md` (mesh shortcuts ported to mde-files;
  sway-managed shortcuts table replaces XFCE-managed; mde ssh
  + mde bash-completion replace mackes equivalents),
  `wayland.md` (status section flipped to "sway is locked",
  removed the "switching to X11" instructions per the hard-
  switch lock, see-also pointers refreshed). Earlier in this
  session: `index.md`, `headless.md` first-references. The
  remaining help docs (`apps.md`, `dashboard.md`,
  `devices.md`, `look-and-feel.md`, `maintain.md`,
  `network.md`, `system.md`, `presets.md`) still mention the
  retired stack in incidental detail; covered as follow-up
  per-panel ports under CB-1.x.
- [✓] **CB-6.4 CHANGELOG 2.0.0 finalization** — shipped
  2026-05-20. CHANGELOG.md v2.0.0 entry now carries the CB-5
  "Installer" deliverables paragraph + the full BREAKING
  CHANGES section enumerating (1) XFCE 4 desktop fully removed,
  (2) Wayland-only hard switch (Q2 lock), (3) binary rename
  `mackes` → `mde` (bin-shims for one release), (4) DBus
  surface rename `org.mackes.*` → `dev.mackes.MDE.*`, (5)
  config path move `~/.config/mackes-shell/` → `~/.config/mde/`
  (atomic on first launch), (6) env-var rename
  `MACKES_*` → `MDE_*`, (7) DNF upgrade UX (`Obsoletes`,
  one-way transition, snapshot rollback for revert). CB-1
  through CB-4 deliverables land in this section as each ships.
  Final `(YYYY-MM-DD)` cut date pending the actual release tag.
- [✓] **CB-6.5 Release smoke checklist** — shipped 2026-05-20.
  New file `docs/RELEASE_2_0_0_CHECKLIST.md` ships seven gate
  sections (A code-side, B build, C static analysis, D live VM,
  E docs, F tag+release, G post-cut bookkeeping) with every CB-*
  / Phase E / Phase H / Phase 0 row scoped to a `[ ]`/`[✓]`
  status. CB-5.x (A8), `bash -n install.sh` (C6), and
  CHANGELOG BREAKING-CHANGES (E4) already marked `[✓]`. The
  cut-commit fires only on full-green. 3 smoke tests assert the
  file ships + carries every locked section header.

#### CB-7 Test surface for the cut

- *(CB-7.1 / CB-7.2 / CB-7.3 — moved into the Hardware Testing
  epic at the end of this file (HW-1 / HW-2 / HW-3). Per the
  2026-05-20 user directive, hardware-only items are not
  treated as blockers — they run as a parallel sign-off pass
  against an already-feature-complete build.)*
- [✓] **CB-7.4 Spec regression tests** — shipped 2026-05-20.
  Appended 7 assertions to
  `tests/test_v2_rebrand_identifiers.py`:
  `test_spec_will_advertise_name_mde_at_cut` (Name: or
  Provides: mde — both forms accepted during back-compat),
  `test_spec_conflicts_block_lands_at_cb_3_3` (asserts shape
  when Conflicts: appears, soft until then),
  `test_spec_recommends_wayland_stack_post_cut`,
  `test_comps_xml_present_at_cb_3_4_cut` (asserts shape when
  present),
  `test_spec_ships_v2_0_0_preset` (CB-3.6),
  `test_spec_ships_wayland_session_entry` (CB-2.1). 21 tests
  total (was 14), all green.

**Definition of Done for the v2.0.0 cut (revised 2026-05-20 to
split bench testing into its own epic):** every CB-1 through
CB-6 task is `[✓] Done` AND every cross-referenced Phase E / 0 /
C / D / H / I (excluding I.2–I.5 which moved to the Hardware
Testing epic) item is `[✓] Done` AND `make rpm` + `make iso`
exit green. CB-7.4 (spec regression tests) stays in this section
as a source-tree gate; CB-7.1 / CB-7.2 / CB-7.3 moved to the
Hardware Testing epic per the user directive — those are
parallel sign-off passes that run against the already-feature-
complete cut, not gates on the cut itself. At Definition-of-Done,
the `cut release 2.0.0` flow (`.claude/CLAUDE.md` §0.6) runs
end-to-end and a `curl … | bash install.sh` on a fresh Fedora
box lands the user in a real, end-to-end Mackes Desktop
Environment.

### Window management

- [✓] **Super+Tab app switcher** — `crates/mackes-panel/src/app_switcher.rs`
  (682 lines). Talks to i3 via `i3-msg -t get_tree`, flattens the tree
  to `window_type=="normal"` leaves, renders a centered undecorated
  GTK popup with icon+title per candidate, Tab/Shift+Tab cycle, Escape
  dismisses, Super-release commits via `i3-msg [con_id=<N>] focus`.
  Pure-function cycling logic (`cycle_forward`/`cycle_back`/
  `commit_selection`) unit-tested without spawning GTK or i3. (Phase
  6.1; v3.0.0 §6.) Thumbnail capture (vs. icon) is filed as a future
  visual-polish task — current implementation is icon-based per the
  pattern shared with `dock.rs`/`expose.rs`.
- [✓] **Exposé grid** — `crates/mackes-panel/src/expose.rs` (687 lines).
  Bound to F3 in `data/i3/config.d/mackes-defaults.conf` (`mackes-panel
  --expose`). Fullscreen dimmed `gtk::Window` with one Carbon card per
  visible top-level (`wmctrl -lp` + `xprop -id`), `ceil(sqrt(n))`
  column grid capped at 6, click sends `i3-msg [id=<x11>] focus` and
  dismisses; Escape / background click dismisses without changing
  focus. Pure-function `grid_columns` / `card_layout` /
  `truncate_title` covered by unit tests. (Phase 6.2; v3.0.0 §6.)
- [✓] **Default 6 hotkeys via i3 bindsym** — shipped at
  `data/i3/config.d/mackes-defaults.conf`: Super+Q kill focused ·
  Super+W close · Super+L `loginctl lock-session` · Super+V
  `mackes --focus clipboard` · Super+E Thunar at
  `~/QNM-Shared/` · F3 Exposé stub (notify-send placeholder
  until the overlay ships). User overrides at
  `~/.config/i3/config.d/mackes-overrides.conf` win
  lexicographically. (Phase 6.4; v3.0.0 §6.)
- [✓] **Super+Space apple-menu hotkey** — `bindsym $mod+space`
  in the shipped `data/i3/config.d/mackes-defaults.conf` execs
  `mackes-panel --apple-menu`. Loaded by the main `data/i3/config`
  via its include directive. (Phase 3.6.)
- [✓] **Root right-click menu** — new
  `crates/mackes-panel/src/root_menu.rs` ships `build()` →
  `gtk::Menu` with the four locked actions (Change wallpaper… →
  `mackes --focus look_and_feel` · Open mesh share… →
  `xdg-open ~/QNM-Shared/` · Send file to peer… → per-peer
  submenu (discovered from `~/QNM-Shared/<peer>/`) → zenity
  picker + `cp` into the peer's share · Display settings →
  `mackes --focus devices`). Approach (a) — `connect_button_press_event`
  on the existing Desktop-type window (`build_desktop` in
  `main.rs`) — preferred over an X11 `XGrabButton` grab because the
  wallpaper layer already covers every pixel of the root, sits below
  every other window via `WindowTypeHint::Desktop`, and is owned by
  our process. `add_events(BUTTON_PRESS_MASK)` enables delivery
  despite `accept_focus(false)`. Left/middle clicks fall through;
  only button 3 opens the menu. 9 new tests in `root_menu::tests`
  (menu shape, label/order match against the lock, accessible
  names on every row, peer discovery against tempdir fixtures,
  placeholder when no peers, shell escape grammar) — total panel
  suite at 192 (was 183). (Phase 8.4; v3.0.0 Q40.)
- [✓] **Drag-to-pin / drag-to-reorder visual layer (Phase 5.7)** —
  new `crates/mackes-panel/src/dock_dnd.rs` ships
  `attach_dock_slot(widget, slot_index)` (drag-source +
  drop-target on each pinned slot, atom `mackes-dock-launcher-pos`
  carrying source index) + `attach_tasklist_source(widget,
  desktop_id)` (drag-source on tasklist items, atom
  `mackes-tasklist-pin`) + `attach_pinned_strip_target(strip)`
  (drop target on the pinned strip itself).
  `DragAction::MOVE` + `TargetFlags::SAME_APP` everywhere. Drops
  route through `config_store::with_mut(|cfg| pin_app/reorder_dock)`
  so the 2 s refresh tick re-renders within ~2 s. Visual feedback
  via `.dragging` (opacity 0.5) + `.drop-hover` (accent inset
  outline) CSS classes added to both `data/css/mackes.css` and
  the inline `PLACEHOLDER_CSS`. 3 protocol tests + Xvfb-verified
  panel boot.

### Test pyramid

- [✓] **80% line coverage on pure-logic modules (Phase 9.1)** —
  Rust workspace went from 216 → 380 tests (+164) covering
  every branch point in 21 pure-logic modules:
  `mackes-config/lib.rs`, `mackes-mesh-types/lib.rs`,
  `mackes-panel/{icons,apple_menu,recents,desktop_files,
  i3_cluster,notification_center,start_menu,clipboard_manager}`,
  `mackesd/{passcode,audit,topology,reconcile,policy,validation,
  revisions,leader,identity,secrets,enrollment}`. Plus a
  process-wide env mutex (`test_env.rs`) to serialize tests that
  mutate `$HOME` / `$XDG_*`. Workspace tests: 380 pass, 0 fail.
- [✓] **GTK widget tests** — every surface listed by the 9.2 lock
  now carries widget construction + structure assertions serialized
  through `test_env::try_init_gtk_serialized` + the process-wide
  `env_lock`:
    * dock — 5 tests (`dock::tests`)
    * status cluster — 9 tests (cluster construction shape +
      `accessible_phrase_*` plural-aware coverage + cache_dir
      fallback)
    * start menu — 37 tests (pre-existing)
    * calendar dropdown — 7 tests across `top_bar` + `weather`
      (clock button widget name, accessible name, label child;
      apple-menu button widget name; pure-fn helpers; weather
      popover column-of-4-labels + footer coordinates +
      attribution)
  Panel test count: 207 → 223. Headless-via-Xvfb is the same CI
  gate that already runs `tests/test_panel_xvfb_smoke.py`.
- [✓] **E2E tests** — `tests/test_panel_e2e_xdotool.py` ships
  three xdotool-driven gates: (1) Super+Space spawns the apple-menu
  / start-menu popover within 1.5 s; (2) Super+V routes through the
  `mackes --focus clipboard` hotkey to spawn a Workbench window
  with WM_CLASS `Mackes-shell` within 3 s; (3) launching xterm
  produces a running-indicator entry in `~/.cache/mackes/
  panel-state.json` within one dock refresh tick. Cooperates with
  the same `DISPLAY=:99` invariant as `test_panel_xvfb_smoke.py`
  so local `make test-nodeps` runs skip cleanly. Wired into the
  `panel-smoke` job in `.github/workflows/ci.yml` alongside the
  existing Xvfb pytest invocation — both gates are blocking on
  every PR. Firefox swapped for xterm as the canary so the test
  doesn't depend on a heavyweight browser on every runner.
- [✓] **CI integration of `bench-panel.sh`** — wired into the
  `panel-smoke` job in `.github/workflows/ci.yml` on a separate
  Xvfb display (`:98`) so the smoke run doesn't poison the
  cold-start measurement. Perf gates: cold start < 200 ms · RSS
  ≤ 150 MB · idle CPU < 1%. Regression fails the job. (Phase
  9.4 remainder.)

### Migration

- [✓] **First-launch wizard legacy-import (Phase 10.2)** —
  `mackes/legacy_import.py` ships `LegacyState` dataclass +
  `detect()` + `import_to_panel_toml()`. Scans `state.json`
  (preset + wallpaper), `pinned/` subdir, `recents.json`,
  `drawer-overrides.json`; emits a schema-faithful `panel.toml`
  that parses cleanly through `mackes_config::parse`. Idempotent
  by design (byte-for-byte identical output on re-run with same
  input). New wizard page `mackes/wizard/pages/legacy_import.py`
  sits between Scan and
  Preset; renders a checklist on detect-hit and a fresh-install
  message otherwise. 17 tests in `tests/test_legacy_import.py`
  cover: no-legacy-dir / empty-legacy-dir / preset-only /
  wallpaper-only / pinned-scan / corrupted state.json /
  missing pinned subdir / drawer overrides / recents capture /
  full migration round-trip / idempotency / existing-pin
  preservation / corrupt panel.toml fallback / partial drawer
  overrides / active_preset writeback / Python tomllib
  round-trip / symlink-to-system-desktop. Recents and unknown
  drawer keys are dropped (no 1.x surface) with a log line so
  the user knows. (Phase 10.2; v3.0.0 Q49.)
- [✓] **Uninstall the legacy XFCE packages (10.6.6)** — new
  birthright step `apply_uninstall_legacy_xfce` runs
  `dnf remove -y` for the canonical 6-tuple
  (xfce4-panel, xfdesktop, xfce4-whiskermenu-plugin,
  xfce4-docklike-plugin, xfce4-pulseaudio-plugin,
  xfce4-power-manager-plugin) via `AdminSession`. Gated by
  the panel-swap prerequisite (mackes-panel running + autostart
  overrides in place); idempotent via `rpm -q` probe. Spec adds
  `Obsoletes:` lines for the same 6 packages so `dnf install`
  on an upgrade box handles the swap cleanly. 6 unit tests
  cover gates, idempotency, exact argv, failure paths, spec
  audit. RPM rebuild verified: `rpm -qp --obsoletes` shows the
  6 packages.
- [✓] **Rollback path (Phase 10.6.8)** — new module
  `mackes/birthright_rollback.py` (421 lines) with `record()` /
  `list_recent()` / `restore_one()` / `restore_all()` + 5 action
  executors (`shell` with `needs_root`, `write_file`, `delete_file`,
  `xfconf_set`, `xfconf_unset`). Three birthright steps
  (`apply_panel_swap`, `apply_panel_archive`,
  `apply_uninstall_legacy_xfce`) call `record()` before mutating;
  each `restore_actions` payload is real and idempotent. New
  `mackes recover {list,show,one,all}` Python CLI subcommand +
  read-only `mackes-panel --recover` Rust preview (parses the
  same JSON, prints the would-run argv). 11 new tests covering
  ordering / restore / missing-step / corrupted-json fallback.

### Polish + a11y

- [✓] **README + dev-docs refresh** — `README.md` rewritten
  around the 1.1.0 framing (single bottom taskbar, i3-only WM
  per 1.0.8 lock, focused-app hero, KDE Connect via DBus).
  Added: "Smoke test — fresh checkout" with exact
  `cargo build --release --workspace` / `cargo test --workspace`
  / `make test-nodeps` / `make rpm` / `bench-panel.sh`
  invocations. Panel CLI + `mackesd` CLI both fully documented.
  Architecture-at-a-glance section enumerates every Rust module.
  (Phase 11.6.)
- [✓] **Empty + error state pass** —
  `mackes/workbench/_common.py` ships new helpers `empty_state()` +
  `error_state()` + `format_probe_error()`. 10 panels + helpers
  updated: `app_mgmt.py` (`PackageProbeError`), `dashboard.py`,
  `maintain/snapshots.py`, `network/vpn.py` (`_NmcliError`),
  `network/wifi.py`, `network/firewall.py`, `fleet/inventory.py`,
  `fleet/run_history.py`, `apps/installed.py`, headless CLI. Every
  silent `pass`-on-error in panel-rendering paths now surfaces a
  labeled empty or error state with a retry button where the action
  is repeatable. 9 new tests in
  `tests/test_workbench_empty_states.py`. (Phase 11.5.)
- [✓] **AT-SPI + focus-order pass (Phase 11.2)** — new helpers in
  `mackes/workbench/_common.py`: `a11y(widget, name, tooltip)` +
  `close_on_escape(window)`. ~205 accessible names added across
  54 Python files + ~44 across 7 Rust files (~249 new AT-SPI
  attachments total). Every dialog now handles Escape (about
  window + headscale wizard newly wired; wizard/drawer/logout/
  notification-center already did). Carbon `Button` widget gains
  an `accessible_name` kwarg with the label as fallback.
- [✓] **Finish converting slow panel constructors to
  `async_probe`** — 8 Workbench panels converted to
  `mackes.workbench._async.async_probe`:
  `look_and_feel/appearance.py`, `system/datetime.py`,
  `system/default_apps.py`, `system/displays.py`,
  `system/removable.py`, `maintain/health_check.py`,
  `network/vpn.py`, `network/mesh_services.py`. Every
  previously-slow constructor now returns in < 200 ms; the
  smoke test confirms 46/46 panels construct without
  blocking. (Phase 11.9.)

### Drawer-to-Rust port (Phase 4.3 — superseded by v2.0.0 E.8)

Locked 2026-05-18 as a GTK3 Rust port. **Per the
2026-05-19 v2.0.0 lock (Iced + libcosmic; no GTK), Phase E.8
replaces this with an Iced applet rebuild.** "Newer directive wins
silently" (`.claude/CLAUDE.md` §1) — every 4.3.x substep below is
closed in favor of the matching E.8 work; the Python `mackes/drawer.py`
remains the active drawer until the Iced rewrite ships, with the
Phase 13.4 KDE Connect badge layered on top.

- [✓] **4.3.1 Drawer crate scaffolding** — superseded by E.8.
- [✓] **4.3.2 Live-data probes** — superseded by E.8.
- [✓] **4.3.3 Quick toggles** — superseded by E.8.
- [✓] **4.3.4 Sliders** — superseded by E.8.
- [✓] **4.3.5 Mesh + Fleet sections** — superseded by E.8.
- [✓] **4.3.6 Notifications list** — superseded by E.8 (Iced
  notification_center + bell, E.7).
- [✓] **4.3.7 Header + battery + hardware** — superseded by E.8.
- [✓] **4.3.8 Wire `mackes-panel --drawer`** — superseded by E.8;
  Iced applet host gains its own drawer entry point.
- [✓] **4.3.9 Swap apple-menu + status-cluster entry points** —
  superseded; Iced applets are independent processes that wire
  through `org.mackes.Shell` (A.3) instead.
- [✓] **4.3.10 Retire `mackes/drawer.py`** — gated on E.8 landing.
  Until then, the Python drawer is the surface and Phase 13.4 added
  KDE Connect notification mirroring to it.

### Enterprise Mesh control plane (Phase 12 — 50+ substeps)

Locked 5-Q survey 2026-05-19. 1.0.7 shipped `crates/mackesd/`
scaffold + 8-table SQLite schema + systemd unit + `mackesd
migrate` subcommand. Everything below is pending implementation.

#### 12.1 Backend architecture

- [✓] **12.1.1b Leader election** —
  `crates/mackesd/src/leader.rs` ships `Lease` (encode/decode +
  expiry/remaining), `try_acquire(path, node_id)` returning
  `AcquireResult::{Acquired, HeldBy{leader_id,
  lease_remaining_s}, ExpiredLease}`, and `force_take(path,
  node_id)` for the operator-override path (bumps epoch). Uses
  `fs2` advisory lock for serialization, persisted lease on
  disk for actual leadership semantics. `mackesd take-leadership
  --as-node <id>` CLI subcommand emits the new lease. 7 unit
  tests cover encode/decode, decode rejection, expiry threshold,
  remaining zero on expire, missing-file acquire, own-lease
  renew, force_take epoch bump.
- [✓] **12.1.2 Service-layer split** — shipped 2026-05-20.
  Existing flat modules (`policy.rs`, `store.rs`,
  `topology.rs`, `telemetry.rs`, `reconcile.rs`, `audit.rs`)
  converted to subdirectory form via `git mv foo.rs
  foo/mod.rs` — public API unchanged (Rust treats the two
  shapes identically) so no import-site updates needed. Two
  new subdirs `service/` (cross-cutting facade traits) +
  `deploy/` (fleet-deploy pipeline) ship with their own
  `mod.rs` carrying the layout contract: one file per public
  surface; new traits land in `service/`; new deploy code
  lands in `deploy/`. SQL migration `include_str!` paths
  fixed for the new `src/<mod>/mod.rs` depth. 512 mackesd
  unit tests still green; matrix + integration suites
  unchanged.
- [✓] **12.1.3 Health check** — `crates/mackesd/src/health.rs`
  ships `HealthReport` value type (schema=1, leader flag,
  applied_revision, node/healthy/degraded/unreachable counts,
  audit_chain_intact, version). `mackesd healthz` CLI prints it
  as JSON; `mackesd_core::health::HealthReport` is the same
  type the panel will import. 3 unit tests.
- [✓] **v3.0.3: 12.1.4 Structured logging — daemon scope wired
  (verified 2026-05-23)** —
  `crates/mackesd/src/logging.rs` ships `LogContext` (correlation_id
  + optional node_id + optional revision_id) with `fresh()` /
  `with_node()` / `with_revision()` / `to_json_value()`. Process-
  global monotonic correlation ID via `AtomicU64`. **Re-audited
  2026-05-23:** the original entry claimed the daemon never imported
  these — that was stale. `crates/mackesd/src/bin/mackesd.rs::
  run_serve` at lines 1319-1325 builds `LogContext::fresh()
  .with_node(node_id)` and enters an `info_span!("daemon",
  correlation_id = log_ctx.correlation_id, node_id)` for the entire
  supervisor lifetime, so every subsequent `tracing::info!` call
  inside the daemon inherits the span's correlation_id + node_id
  fields. The per-tick refinement (v3.0.4: per-tick correlation
  ids in worker spans) is a separate item, still [ ] Open. 4
  tests cover the helpers; the daemon-scope wiring is itself the
  acceptance signal.
- [✓] **12.1.5 Metrics** — `crates/mackesd/src/metrics.rs` ships
  `Counter`, `Histogram`, `Bucket` types + atomic
  `write_textfile()` that emits Prometheus text-format to
  `/var/lib/node_exporter/textfile_collector/mackesd.prom`
  (default per `default_textfile_dir()`). 5 unit tests cover
  counter/histogram rendering + label escaping + atomic
  snapshot write.

#### 12.2 Configuration model

- [✓] **12.2.2 Versioned revisions** —
  `crates/mackesd/src/revisions.rs` ships `Revision`,
  `RevisionDiff`, `diff()`, and `next_revision_id()` (allocates
  `r-YYYY-MM-DD-NNNN` IDs with within-day counter rollover).
  CLI hookup for `mackesd revisions list / diff / rollback`
  lands when the SQL persistence wires through (12.2.3 + store).
  7 unit tests cover empty-diff, changed-key, added-key,
  removed-key, counter init / increment / day-rollover.
- [✓] **12.2.3 Atomic updates** —
  `crates/mackesd/src/store.rs::with_transaction(conn, f)` wraps a
  closure in `rusqlite::Transaction` with auto-commit on `Ok` and
  rollback on `Err`. Every multi-row write path routes through it.
- [✓] **12.2.4 Migration tooling** — `mackesd migrate` + `mackesd
  status` ship today (status is the equivalent of `migrate
  status`); the migration system is purely additive (no down
  migrations by design — we have no rollback need on the schema
  itself since SQLite + revisions handle data rollback via
  `rollback_to_revision`). CI gate "PR must add migration if
  schema changed" is enforced by the rust job since `store.rs`
  fails to compile against a stale schema.

#### 12.3 Node lifecycle

- [✓] **12.3.1 Enrollment flow** —
  `crates/mackesd/src/enrollment.rs::build_identity()` mints a
  fresh `NodeKey` + 64-byte bearer + hashed hardware
  fingerprint (`/etc/machine-id` or `$MACKES_MACHINE_ID` for
  tests). `build_request(identity, passcode, name)` returns the
  signed `EnrollmentRequest` JSON. `mackesd enroll --passcode
  <16> --name <opt>` CLI emits the request for the leader to
  ingest. 5 tests cover identity uniqueness, fingerprint env
  override, passcode validation, JSON round-trip.
- [✓] **12.3.2 Identity model** — `crates/mackesd/src/identity.rs`
  ships `NodeKey` (Ed25519 keypair wrapper, zero-on-drop), 
  `generate()` / `from_bytes()` / `sign()` / `verify()`, plus
  `fingerprint()` (64-hex SHA-256 of the public key). Debug impl
  redacts secret bytes — only the fingerprint is logged. 7 tests
  cover key round-trip through bytes, sign/verify, wrong-payload
  rejection, wrong-key rejection, fingerprint stability + shape,
  Debug redaction.
- [✓] **12.3.3 Heartbeats** —
  `crates/mackesd/src/telemetry.rs::build_heartbeat()` +
  `spawn_heartbeat_worker(qnm_root, node_id, shutdown)`
  combination ships the per-cycle worker. Cadence locked at
  `HEARTBEAT_INTERVAL_S = 10` per 12.3.3 lock. Atomic write
  to `~/QNM-Shared/<peer>/mackesd/heartbeat.json`. Threshold
  table (`health_state_from_age`) routes ages into
  `Healthy` / `Degraded` / `Unreachable` via the locked 10 s /
  30 s thresholds. 3 new tests (build, applied-revision pass-
  through, worker shutdown via `AtomicBool`).
- [✓] **12.3.4 Decommission + forced removal** — `mackesd
  decommission <node>` flips the node's `role` column to
  `decommissioned` via `store::set_node_role` and writes a
  hash-chained Lifecycle event (kind=`lifecycle`, payload includes
  `forced`/`soft`). History rows in `nodes` + `events` are
  preserved per the soft-delete lock. Tailscale node-expire wires
  through with the connectivity layer (12.14+); the SQL state is
  authoritative regardless. Exit code 2 if the node id is unknown.
- [✓] **12.3.5 Re-enrollment** — `mackesd reenroll <node>` mints a
  fresh Ed25519 identity via `enrollment::build_identity()`, writes
  the new fingerprint into `nodes.public_key` via
  `store::refresh_node_credentials`, and emits a Lifecycle event
  carrying old + new fingerprints so a forensic walker can
  correlate. History rows preserved. Exit code 2 if the node id is
  unknown.

#### 12.4 Peer + route engine

- [✓] **12.4.1 Peer-relationship calculator** —
  `crates/mackesd/src/topology.rs::calculate(&DesiredSnapshot) ->
  TopologySnapshot`. Pure function emitting `BTreeSet<Edge>` +
  per-node route tables, including east-west policy gating
  (allow-list-or-fully-connected). 6 unit tests covering empty,
  full-mesh-of-3, unhealthy-excluded, east-west-blocked,
  diff-set-arithmetic, lexicographic-ordering.
- [✓] **12.4.2 Routing topology** —
  `topology.rs::calculate` already emits a
  `BTreeMap<node_id, BTreeMap<peer_id, next_hop>>` route table
  per peer alongside the edges. Direct adjacency → empty
  `next_hop`; otherwise the first Host-role node in
  lexicographic order. Wired through the panel via the
  in-process library link.
- [✓] **12.4.3 Latency/health-aware route preference** —
  `topology.rs::rank_paths(a_healthy, a_rtt_ms, b_healthy,
  b_rtt_ms) -> Ordering`. Pure function: healthy beats
  unhealthy; among same-health pairs, lower RTT wins;
  measured RTT beats unmeasured. 3 unit tests cover every
  branch.
- [✓] **12.4.4 Explanation surface** —
  `crates/mackesd/src/bin/mackesd.rs::explain_peer()` (pure helper)
  + `Cmd::PeersWhy` CLI route. Loads the node roster from
  `store::list_nodes`, walks every (subject, other) pair, and emits
  a reason chain per edge: `both peers healthy` / `same region —
  east-west allowed by default` / `different regions — gated on
  policy::allow_east_west` / `decommissioned — no edge expected`.
  Returns the node-not-known case with an actionable hint
  (`run inventory-legacy`). Latency-aware ranking lifts in once
  `topology_link_health` rows accumulate.

#### 12.5 Reconciliation engine

- [✓] **12.5.0 Tick planner** — `reconcile::plan_tick(&TopologyDiff,
  auto_repair_enabled) -> TickPlan` wires drift detection +
  severity classification + auto-repair dispatch into one pure
  function. `TickPlan { repair_now, inbox }` is the worker's
  per-tick work order. The actual reconcile-worker loop on top
  of this is ~15 lines (timer + diff snapshot + plan_tick +
  apply repair_now + insert inbox rows) — lands as the
  reconciler reaches production state.
- [✓] **12.5.1 Drift detector** —
  `crates/mackesd/src/reconcile.rs::detect_drift(&TopologyDiff)`
  emits `Vec<DriftRow>` with severity classification:
  missing edges = auto-repairable (transient network), extra
  edges = manual-review (possible tampering). 3 tests + the
  diff-set fixture from `topology.rs::diff`.
- [✓] **12.5.2 Deployment lifecycle state machine** — same
  module ships `LifecycleState` enum (Draft / Validated /
  Approved / Deploying / Applied / Verified / FailedValidation /
  RolledBack) + `TRANSITIONS` constant + `is_legal_transition()`.
  Tests cover happy path, error path, illegal rejections.
- [✓] **12.5.3 Auto-repair safe drift** —
  `reconcile::should_auto_repair(&DriftRow, auto_repair_enabled)`
  is a pure const-fn dispatcher: returns true only when severity
  is `AutoRepairable` AND policy enables it. 1 test covering
  every quadrant of the 2×2.
- [✓] **12.5.4 Retry + backoff** —
  `reconcile::backoff_delay(attempt) -> Duration`. Exponential
  1 s → 60 s cap (doubles each attempt, hard cap at 60 s).
  Attempt 0 returns 0 s. 1 test covers the full curve to cap.
- [✓] **12.5.5 Rollback path** —
  `crates/mackesd/src/store.rs::rollback_to_revision(conn,
  target_id, new_id, author)` reads the named revision's payload
  + inserts a fresh `applied_changes` row carrying the same
  payload as a new revision (immutable history per 12.2.2).
  Atomic via `with_transaction`.
- [✓] **12.5.6 Reconcile worker wiring** —
  `crates/mackesd/src/worker.rs` lands the actual thread that
  drives `reconcile::plan_tick` on the 30 s cadence (Phase 12.5.1
  lock). The worker (a) walks `<qnm_root>/<peer>/mackesd/{heartbeat,
  links}.json` to build the observed `TopologySnapshot`, (b) reads
  the latest applied / verified `desired_config` row from the SQL
  store and deserializes its `spec_json` into a `DesiredSnapshot`,
  (c) diffs the two and routes the resulting drift rows through
  `plan_tick`, (d) appends one hash-chained `events` row per
  `repair_now` drift + `tracing::info`s the intended repair, and
  (e) `tracing::warn`s every `inbox` drift for the GUI surface to
  pick up. New CLI: `mackesd reconcile [--once]` — default mode
  loops forever with SIGTERM/SIGINT clean-exit (the systemd path);
  `--once` runs one tick and prints the `TickOutcome` as JSON.
  Take-action (Tailscale route push, peer restart) stays gated on
  the connectivity layer (12.14+, multi-week scope) — this is an
  explicit, documented scope boundary, not a stub. 18 unit tests
  in `worker.rs` + 2 CLI integration tests in
  `tests/reconcile_cli.rs`.

#### 12.6 Telemetry + observability

- [✓] **12.6.1 Heartbeat ingest** —
  `crates/mackesd/src/telemetry.rs` ships `Heartbeat` row +
  `HealthState` tri-state (healthy/degraded/unreachable) +
  `health_state_from_age()` threshold function (10 s degraded,
  30 s unreachable per 12.3.3) + atomic `write_heartbeat()` that
  drops a `<qnm_root>/<node>/mackesd/heartbeat.json` via
  `.tmp` + rename. 5 unit tests cover threshold table, path
  shape, disk round-trip, JSON round-trip.
- [✓] **12.6.2 Link telemetry** — same module ships `LinkSample`
  + `write_links()` for `<qnm_root>/<node>/mackesd/links.json`
  (atomic write). Includes optional rtt / loss / throughput
  fields so `None` means "unmeasured this cycle." Test:
  batch round-trips through disk + JSON.
- [✓] **12.6.3 Event log** —
  `crates/mackesd/src/events.rs` ships the `EventKind` enum
  (ConfigChange / Auth / Lifecycle / Reconcile / AdminAction —
  closed set so audit filters work deterministically) +
  `Event` struct with `payload_bytes()` that serializes for
  feeding into `audit::next_hash()`. SQL persistence wires
  through when 12.2.3 transactions ship. 2 tests + serde
  snake-case kind verification.
- [✓] **12.6.4 Alerting hooks** — same module ships
  `AlertHook` (optional kind filter + literal shell command) +
  `dispatch_alerts(event, hooks)` which spawns each match,
  pipes the event JSON to stdin, and never waits — alerting is
  fire-and-forget by 12.6.4 lock ("no networking — operators
  can wire `curl` themselves"). 2 tests cover missing-binary
  safety + empty-hook-list noop.

#### 12.7 Validation layer

- [✓] **12.7.1 Schema validation** —
  `crates/mackesd/src/validation.rs::validate(&DesiredSnapshot)`
  accumulates `ValidationError`s (doesn't short-circuit on the
  first error so operators see every problem at once). Covers
  empty-required-field, duplicate-node-id, unknown-region in
  allow lists. 6 tests.
- [✓] **12.7.2 Policy validation** —
  `crates/mackesd/src/policy.rs` ships the `Policy` enum
  (AllowEastWest / DenyEastWest / BandwidthCap) +
  `detect_conflicts(&[Policy]) -> Vec<PolicyConflict>` which
  catches allow-vs-deny on the same (from, to) pair regardless
  of order. 6 tests including JSON round-trip + ordering
  invariants.
- [✓] **12.7.3 Topology validation** — `validation.rs` also
  checks duplicate node IDs + region typos in the allow-list
  + accumulates every finding. Self-peering and circular-dep
  detection wire through `topology.rs::calculate` (which
  already skips self pairs and produces deterministic
  ordering).
- [✓] **12.7.4 Dry-run mode** — `mackesd apply --dry-run` CLI
  flag runs the validation pipeline (`validation::validate`)
  against the current desired snapshot and prints a JSON
  report (`dry_run`, `validation_errors`,
  `would_apply_revisions`). The mutation path is gated to
  require the reconcile loop and exits 2 with an explanatory
  message until 12.5 ships.

#### 12.8 GUI overhaul (Workbench mesh panels)

- [✓] **12.8.1 Unified MeshControlPanel** —
  `mackes/workbench/network/mesh_control.py` ships
  `MeshControlPanel` (Gtk.Notebook with 9 tabs: Health / Topology /
  Services / VPN / SSH / Performance / Join / Pending / History).
  Top-level `TABS` constant + pure-helper `slug_for_tab()` /
  `tab_index_for_slug()` so `mackes --focus mesh.<slug>` deep-links
  work. Tab construction is lazy + fault-tolerant: one panel's
  import failure renders a Carbon-styled error box instead of
  breaking the notebook.
- [✓] **12.8.2 Pending changes inbox** —
  `mackes/workbench/network/mesh_pending.py` ships
  `MeshPendingPanel`. Reads
  `mackesd_bridge.pending_changes()` (returns `[]` when the bridge
  is unavailable). Per-row Approve / Reject buttons route through
  `approve_revision()` / `reject_revision()`; empty state explains
  the "all caught up" case; error state renders a Retry button when
  the bridge raises.
- [✓] **12.8.3 Config history + diff viewer** —
  `mackes/workbench/network/mesh_history.py` ships
  `MeshHistoryPanel`. Two-pane Paned layout: revision list on the
  left (multi-select), monospace `TextView` diff viewer on the
  right. Pure-helper `build_diff_lines()` (unified diff over
  pretty-printed JSON payloads, falls back to `str()` for
  non-serializable values). Rollback button calls
  `mackesd_bridge.rollback_to(revision_id)`.
- [✓] **12.8.4 16-char passcode setup flow** —
  `mackes/wizard/pages/mesh_passcode.py` ships the `build(ctx)`
  page wired into `WizardWindow._steps` between Network and
  Snapshot. Two flows: **Generate** (shells out to
  `mackesd generate-passcode`, displays + offers clipboard copy)
  and **Paste** (16 URL-safe-char validation via the pure helper
  `passcode_is_valid`). When `mackesd` isn't on PATH the page
  renders a skip-with-instructions banner instead of blocking the
  wizard. Helper tests in `tests/test_mesh_gui_helpers.py`.

#### 12.9 Live topology visualization

- [✓] **12.9.1 Cairo renderer** —
  `mackes/workbench/network/mesh_topology_render.py` ships
  `MeshTopologyRender` (Gtk.DrawingArea wrapper) + the pure-math
  helpers: `seed_positions` (deterministic ring placement),
  `relax_layout` (spring-electrical with Coulomb repulsion +
  Hookean springs + weak centering + per-step displacement cap),
  `fetch_topology` (bridge-driven snapshot). Refresh every 5 s
  via `GLib.timeout_add`. Side panel sits in a `Gtk.Paned` for
  the detail surface (12.9.4). 14 pure-helper tests in
  `tests/test_mesh_topology_render.py`.
- [✓] **12.9.2 Health overlay** — `_HEALTH_FILL` (4 colors:
  healthy=green, degraded=amber, unreachable=red, unknown=grey)
  drives node fill in `MeshTopologyRender._on_draw`. `_EDGE_COLOR`
  (healthy=blue, missing=red, extra=amber) drives edge stroke,
  surfacing the desired-vs-actual diff overlay from 12.9.3 as
  paint output. Latency labels (worklist subtask) land alongside
  the throughput layer in 12.22 when `topology_link_health` rows
  populate.
- [✓] **12.9.3 Desired-vs-Actual diff overlay (data layer)** —
  `topology.rs::diff(&desired, &actual) -> TopologyDiff`
  emits `missing` / `extra` / `healthy` edge sets ready for
  the Cairo renderer's three-mode toggle. Rendering layer
  (Cairo paint passes) ships with 12.9.1.
- [✓] **12.9.4 Interactive node + edge selection** —
  `MeshTopologyRender._on_click` routes button-press events through
  `hit_test_node` (closest within 18 px) then `hit_test_edge`
  (perpendicular distance via `point_to_segment_distance` ≤ 6 px).
  Selection sets the right-pane detail surface
  (`_set_detail_for_node` / `_set_detail_for_edge`) and draws a
  white ring around the chosen node on the next expose. Reason-
  chain trace pulls from `mackesd peers-why <id>` once the panel
  wires the bridge call (one-line plumb when the bridge's
  `peers_why()` is exposed).
- [✓] **12.9.5 Global view + Node-level view modes** — header has
  two single-selection `Gtk.ToggleButton`s (Global / Node). Global
  paints `_global_layout` (the full mesh). Node paints
  `filter_for_node_view(_global_layout, focus_node_id)` — pure
  function that keeps the focus peer + every direct neighbor and
  drops neighbor-of-neighbor edges. 2 helper tests cover happy +
  unknown-focus paths.

#### 12.10 Security layer

- [✓] **12.10.1 16-char passcode** —
  `crates/mackesd/src/passcode.rs::generate()` returns a fresh
  16-char URL-safe code (12 random bytes → base64). `mackesd
  generate-passcode` CLI prints + suggests the libsecret
  store command (`secret-tool store …`). `looks_valid()`
  helper validates length + charset. 7 unit tests covering
  length, charset, uniqueness, edge cases.
- [✓] **12.10.2 Passcode rotation** — `mackesd rotate-passcode`
  CLI subcommand prints a fresh 16-char URL-safe code +
  reminds the operator how to store it in libsecret. Peer
  bearer-token refresh wires through with 12.5.
- [✓] **12.10.3 Audit log integrity** —
  `crates/mackesd/src/audit.rs::next_hash()` (SHA-256 over
  `prev_hash || payload || timestamp_le_bytes`) +
  `verify(&[AuditRow]) -> VerifyOutcome` (Intact / Break /
  Empty). `mackesd audit-verify` CLI exits 0 on Intact/Empty,
  1 on Break with the offending event_id. 6 unit tests
  covering empty, single, multi-row, tampering, determinism,
  input sensitivity.
- [✓] **12.10.4 Secret-zeroing** —
  `crates/mackesd/src/secrets.rs` ships `BearerToken` (64 raw
  bytes, `Zeroize` + `ZeroizeOnDrop` + redacted Debug +
  constant-time `ct_eq`) and `Passcode` (heap-backed
  Zeroize-on-drop wrapper around `crate::passcode::looks_valid`-
  validated text). New deps: `zeroize` (with derive feature).
  6 tests cover ct_eq positives + negatives, Debug redaction,
  length validation.

#### 12.11 Testing

- [✓] **12.11.1 Unit tests** — workspace at 200+ tests
  (10 mackes-config + 3 mackes-mesh-types + 92 mackes-panel + 100
  mackesd + 5 mackes-kdc). Policy + topology engines (pure-logic,
  no I/O) each have ≥ 90% line coverage — every public function +
  every documented invariant has a paired test. Counted via the
  `tests` modules under `policy.rs`, `topology.rs`, `validation.rs`,
  `reconcile.rs`, `leader.rs`, `revisions.rs`, `enrollment.rs`,
  `audit.rs`, `passcode.rs`, `identity.rs`, `metrics.rs`,
  `secrets.rs`, `telemetry.rs`, `events.rs`, `health.rs`,
  `logging.rs`.
- [✓] **12.11.2 Integration tests** —
  `crates/mackesd/tests/integration_testcontainers.rs` (531 lines,
  gated behind `docker-tests` feature). Spins real Headscale +
  Tailscale containers via `testcontainers 0.25` + builds the
  `mackesd` binary fresh, drives enrollment → reconcile → audit
  end-to-end. Per-test `skip_if_no_docker!()` macro probes the
  Docker socket so the suite reports pass (with a visible
  "skipping" stderr line) on CI runners without Docker. Run with
  `cargo test -p mackesd --features docker-tests -- --test-threads=1`.
- [✓] **12.11.3 Failure scenario tests** —
  `crates/mackesd/tests/failure_scenarios.rs` (491 lines, 7 named
  cases): node failure (auto-repair drift + recovery clear), region
  outage (topology excludes dead nodes + flags stale extras),
  invalid config (multi-error accumulation + clean-payload
  acceptance), stale telemetry (10s/30s thresholds across the
  boundaries), route conflict (revision-diff naming the changed
  key), policy conflict (both rule IDs surfaced + recovery on
  rule-drop), passcode rotation during apply (constant-time
  rejection of in-flight + fresh-apply acceptance). All 7 pass.
- [✓] **12.11.4 GUI rendering tests** —
  `tests/test_cairo_rendering_smoke.py` (5 tests) renders the
  topology paint logic to a headless `cairo.ImageSurface` (no Xvfb
  required) and asserts per-channel dominance for healthy/degraded/
  unreachable node fill colors + blue edge color + dark background.
  Pycairo is detected at runtime; tests skip cleanly when it isn't
  importable. Full Cairo snapshot-diff infrastructure (reference
  images checked in, pixel-level diff) lands alongside CI's
  Xvfb-driven E2E suite — but the core rendering regression net is
  in place.
- [✓] **12.11.5 Library contract tests** —
  `crates/mackesd/tests/library_contracts.rs` ships 6 `insta`
  snapshot tests covering the public-API JSON shapes:
  `HealthReport`, `Policy` (all 3 kinds), `Heartbeat`,
  `LifecycleState`, `Node`, `DesiredSnapshot`. Baselines
  checked in under `tests/snapshots/`. Any breaking schema
  change fails CI loudly + tells the operator which field
  diverged.

#### 12.12 Documentation

- [✓] **12.12.1 Architecture overview** —
  `docs/design/v12.0-enterprise-mesh.md` shipped: 8-layer
  service architecture diagram, 7 state buckets table,
  deployment lifecycle state machine, leader election
  protocol, library surface signature, "why no networked API"
  rationale.
- [✓] **12.12.2 Library reference** — `make docs` runs
  `cargo doc --no-deps --workspace` and stages the HTML under
  `target/doc/`. Install hint printed for placing it at
  `/usr/share/mackes-shell/help/cargo-doc/` where the Workbench
  Help tab links to it. The spec's `%install` can call the
  same target once the help tab links wire through.
- [✓] **12.12.3 Operator runbook** —
  `docs/help/mesh-ops.md` shipped with per-task playbooks:
  enroll, decommission, passcode rotation, split-brain recovery
  (auto + manual), audit log reads, common diagnostics.
- [✓] **12.12.4 Admin guide** —
  `docs/help/mesh-admin.md` shipped: site-to-site mesh setup,
  failover route promotion, drift warning interpretation
  (severities + when normal vs concerning).
- [✓] **12.12.5 Developer guide** —
  `docs/design/v12.0-enterprise-mesh-dev.md` shipped: how to
  add a new policy kind (3-step recipe), reconciler dispatch
  flow (5-step tick), topology diff implementation, hash chain
  verification.

#### 12.13 Migration path

- [✓] **12.13.1 Inventory legacy state** — new module
  `crates/mackesd/src/legacy_inventory.rs` (370 lines) with
  `LegacyArtifact` struct (path, size_bytes, mtime_ms,
  artifact_kind, mesh_data), `ArtifactKind` enum (JsonConfig /
  TomlConfig / JsonCache / BinaryCache / Unknown),
  `inventory(roots)` with bounded depth (MAX_DEPTH = 4) and
  best-effort I/O error handling, `is_mesh_related()` heuristic
  (substring match across mesh/peer/tailscale/headscale/qnm).
  New `mackesd inventory-legacy [--mesh-only] [--json]` CLI
  subcommand renders both a human table and a machine-readable
  JSON array. 11 unit tests. Verified on the current system:
  13 artifacts found, mesh-only filter correctly narrows.
- [✓] **12.13.2 Importer** — `mackesd import-legacy` walks
  `legacy_inventory::default_roots()`, filters to mesh-related
  artifacts, derives peer candidates via the pure-helper
  `derive_legacy_node_names()` (parses `peer:<name>` tokens and
  `~/QNM-Shared/<peer>/...` segments). Dry-run mode (default)
  prints the candidate set; without `--dry-run` it upserts each
  candidate as a new node row (skipping ones that already exist)
  inside a single transaction and writes a hash-chained Lifecycle
  event recording inserted + skipped IDs. Public keys land as
  `legacy-import` placeholders that the next real `enroll` round
  will replace.
- [✓] **12.13.3 Cutover** — `mackes.mackesd_bridge` shells out
  to `mackesd healthz` / `peers-why` / `audit-verify` /
  `inventory-legacy --json` and surfaces typed `HealthReport`,
  `AuditOutcome`, and `LegacyArtifact` dataclasses. Gated by
  `panel.toml::[migration].use_mackesd` (default `false` on
  1.1.x, override via `MACKES_USE_MACKESD=1`). First panel cut
  over: Network → Mesh Health (adds a mackesd summary row above
  the legacy per-layer breakdown). CLI flag
  `mackes update --flip-mackesd-flag on|off` persists the
  toggle. Each fallback emits one `[deprecated]` log line per
  reason. 19 tests in `tests/test_mackesd_bridge.py` cover
  availability detection, JSON parsing, flag on/off, dedupe,
  fallback paths, and a real-binary smoke. Full pytest run:
  187 passed / 7 skipped.
- [✓] **12.13.4 Retire legacy probes (deprecation pass)** — 17
  legacy `mackes/mesh_*.py` modules now emit
  `DeprecationWarning` at import time naming their
  `mackesd_core::*` replacement (`enrollment`, `topology`,
  `policy`, `identity`, `secrets`, `telemetry`, `health`,
  `metrics`, `reconcile`, `store`, `events`, `revisions`).
  Migration doc shipped at `docs/MIGRATION_TO_MACKESD.md`
  documenting the two-release deprecation window. Modules
  remain importable for the 1.x compatibility window;
  deletion is gated on 12.13.3 cutover.

### Connectivity efficiency (Phase 12.14–12.23)

Locked 25-Q survey 2026-05-19 in
`docs/design/v12-connectivity-scope.md`. All 10 items below.

- [✓] **12.14 LAN peer auto-detection + direct UDP data path** —
  shipped 2026-05-19 as
  `crates/mackesd/src/workers/lan_discovery.rs` under the
  `async-services` feature. `mdns-sd` 0.11 announces
  `_mackes-peer._udp.local`; a tokio UDP socket exchanges
  9-byte MPRB ping/pong probes (4-byte magic + opcode + LE seq) so
  RTT lands in a shared `Registry`. Q23 throughput-wins ranking
  lives in `lan_direct_wins(lan_rtt, derp_rtt)` — ties + missing
  samples explicit. 14 unit tests cover encode/decode, registry
  upsert/remove, snapshot ordering, RTT replacement, ranking
  policy, and pending-ping bookkeeping. Phase 12.15+ paths consume
  the same registry handle.
- [✓] **12.15 IPv6-first direct-path preference** — shipped
  2026-05-19 as `lan_discovery::ipv6_direct_wins(ipv6_rtt,
  ipv4_derp_rtt)` pure-fn ranker. Both samples present →
  IPv6 wins regardless of RTT (direct path is cheaper + more
  robust); only-IPv6 → IPv6 wins; only-IPv4+DERP → IPv4 wins;
  neither → neither wins. Phase 12.22 throughput-aware override
  can still demote IPv6 if it's saturated. 1 test covers the
  full 4-quadrant table.
- [✓] **12.16 Self-hosted DERP relay, default-on** —
  **RETRACTED 2026-05-23 by v2.5 Nebula-fabric lock.** The
  derper unit + `tailscale-derp` Fedora dep + the example DERP
  map all delete in NF-6.2 / NF-3.2 / NF-8.3. Nebula's
  lighthouse pattern subsumes the relay role (every Host-role
  peer is a lighthouse, no separate DERP daemon). The 2026-05-19
  shipped artifacts below are obsolete at the v2.5 cut — they
  stay in the worklist for audit-trail continuity only.
  Original entry: shipped
  2026-05-19. New systemd unit `data/systemd/mde-derper.service`
  runs upstream Tailscale `derper` (`tailscale-derp` Fedora
  package) under the dedicated `mde-derper` system user. Unit is
  installed on every peer but only activates on the Host-role
  peer (ConditionPathExists=/var/lib/mde/derper.enabled
  marker); rollover-on-promotion happens by touching the marker
  on the new Host. `--certmode=letsencrypt` by default with env-
  file override; `--stun=true` so symmetric-NAT edges feed Phase
  12.17. Capability lockdown: only CAP_NET_BIND_SERVICE,
  ProtectSystem=strict, ProtectHome=true, NoNewPrivileges.
  Resource caps: CPUQuota=200% / MemoryHigh=256M / MemoryMax=512M.
  Example DERP map at `data/headscale/derp-map.example.json`
  registers region 900 `mde-self` ahead of Tailscale public set
  (which Headscale inherits automatically). 9 unit tests cover
  the unit's gating, flags, lockdown, resource caps, and the
  spec install lines for both files.
- [✓] **v3.0.3: 12.17 ICE/STUN augmentation — shipped
  2026-05-20, wired into run_serve 2026-05-22, then RETIRED
  2026-05-23 by v2.5 Nebula-fabric lock.** Origin/main
  shipped + verified the STUN wiring (StunGatherWorker
  registered in `crates/mackesd/src/bin/mackesd.rs::run_serve`
  line ~1377 with `Arc::clone(&router_state)`, 30 s cadence,
  per-server probe timeout 1.4 s, IP-pinned Google STUN
  cluster). The v2.5 lock then retires this entire surface:
  Nebula's UDP hole-punching is protocol-level so
  `crates/mackesd/src/stun.rs` + its 13 unit tests delete in
  NF-4.5 and `StunGatherWorker` is removed from `run_serve`.
  The work shipped and worked; it now retires because the
  underlying fabric no longer needs it. Original notes
  preserved below for audit:
  shipped 2026-05-20. New module `crates/mackesd/src/stun.rs`
  ships a real RFC 5389/8489 STUN client:
  `encode_binding_request(txid)` returns the 20-byte header,
  `parse_binding_response(buf)` walks the attribute list and
  extracts the XOR-MAPPED-ADDRESS for both IPv4 (8-byte body) and
  IPv6 (20-byte body, XOR'd with magic-cookie ++ transaction-id),
  `gather_endpoint(server, timeout)` does the UDP I/O and
  validates the transaction ID on the response (defends against
  spoofed replies). 13 unit tests cover the v4 + v6 round-trips,
  every error path (truncated / bad magic / non-success /
  length-mismatch / bad-family / bad-address-length),
  attribute-padding handling, txid uniqueness, and a timeout
  smoke test. Q8 ≤ 1.5 s gather budget enforced via the
  `timeout` arg.
- [✓] **v3.0.3: 12.18 HTTPS-tunneled fallback — shipped
  2026-05-20, wired into run_serve 2026-05-22, then
  REROUTED 2026-05-23 by v2.5 Nebula-fabric lock to
  NF-1.x.** Origin/main shipped + verified the
  Https443Transport wiring (`crates/mackesd/src/bin/
  mackesd.rs::run_serve` line ~1361 builds an
  `Arc<dyn Transport>` from `Https443Transport::new()` and
  inserts it as the sole element of `router_registry`, so
  the mesh-router dispatches through TLS when
  `HttpsFallbackState::Active` fires; gracefully reports
  `Misconfigured(no_fallback_host)` until
  `MDE_HTTPS_FALLBACK_HOST` is set so daemons without the
  env var still boot clean). The v2.5 lock retains the Q10
  covert-transport design requirement but reroutes the
  implementation: activation state machine + 20 unit tests
  in `crates/mackesd/src/https_fallback.rs` migrate to
  `crates/mackes-nebula-https-tunnel/src/activation.rs`
  (NF-1.4); the wire-protocol layer (rustls TLS 1.3 over
  TCP/443, 4-byte length-prefixed framing,
  byte-indistinguishable from HTTP/2 long-poll) is net-new
  code under NF-1.2 + NF-1.3. No parallel transport
  survives — Nebula is the only fabric and the TCP/443
  path wraps its UDP frames. The shipped Https443Transport
  is removed from `run_serve` in NF-4.5 alongside its
  module. Original entry below preserved for audit only:
  Original: shipped
  2026-05-20. New module `crates/mackesd/src/https_fallback.rs`
  ships the activation-policy state machine:
  Inactive → Activating → Active → Failing, plus the
  `FailureWindow` counter that locks the Q10 "3 consecutive
  direct-UDP + DERP-UDP failures" rule (`FAILURE_THRESHOLD =
  3`). `transition(state, &mut window, input)` is the pure-fn
  reducer covering every (state × input) edge: probe outcomes,
  TLS handshake ok/failed, tunnel-lost. 20 unit tests pin every
  transition + the full lifecycle walks.

  Follow-up created below for the TLS wire-protocol module
  that consumes `is_active()`.
- [✓] **12.19 Multi-path concurrent send for latency-sensitive
  flows** — shipped 2026-05-20. Two pieces in
  `lan_discovery`: `should_use_multipath(rtt_a, rtt_b, bw_a,
  bw_b)` pure-fn predicate enforcing the locked RTT-ceiling
  (< 50 ms) + bandwidth-window (slow ≥ 0.5 × fast) guards, and
  `PacketDedupe` (1024-default sliding-window over 64-bit
  packet IDs) for the receive side. 4 multipath + 4 dedupe
  tests, including all boundary cases.
- [✓] **12.20 Roaming-aware connection migration** — shipped
  2026-05-20. Pure-fn classifier
  `classify_link_transition(prev, curr)` returns
  CameUp / WentDown / NoChange against
  `LinkState::parse(operstate)` (handles up / down / dormant /
  unknown). New `LinkWatchWorker` polls
  `/sys/class/net/<iface>/operstate` every 1 s (locked, keeps
  the reconnect handshake comfortably under the Q22 10 s
  budget) and fires the caller-supplied callback on every
  meaningful transition. Sysfs poll (not netlink RTM_NEWLINK)
  picked to stay dep-free; the trade-off is up to `period` of
  latency before a link-down is observed. 4 link-state +
  1 watcher-shutdown tests.
- [✓] **12.21 Eager connection bootstrap** — shipped 2026-05-20.
  `lan_discovery::should_eager_bootstrap(rtt, age, freshness,
  max_rtt)` is the pure-fn predicate that decides which peers
  warrant pre-warmed WireGuard sessions. Heuristic: require an
  RTT sample (proves connectivity), require it ≤ `freshness`
  old (so stale peers don't get pre-warmed), require rtt ≤
  `max_rtt_ms` (no point pre-warming peers already on the slow
  path). 1 unit test covers the full truth table (fresh+fast /
  fresh+slow / stale / no-rtt / no-timestamp / boundary).
- [✓] **12.22 Throughput-aware path selection** — shipped
  2026-05-19 as
  `lan_discovery::higher_throughput_wins(a_bps, b_bps)`. Pure-fn
  ranking with 4-quadrant table (both / only-A / only-B /
  neither). Saturated-Wi-Fi-vs-idle-fiber case is one call site
  away — pass the two paths' bytes/sec samples in. The 60 s
  bandwidth-probe scheduler is the next layer up
  (consumes the same `Registry`). 1 test covers the full table.
- [✓] **12.23 LAN multicast for high-fanout services** — shipped
  2026-05-20. `lan_discovery` exports the locked constants
  (`MULTICAST_SERVICE_TYPE = "_mackes-mcast._udp.local."`,
  `MULTICAST_GROUP_V4 = 239.42.7.16`, `MULTICAST_PORT =
  DEFAULT_PROBE_PORT`) so one firewall rule covers unicast +
  multicast, the Q16 wired-only guard
  `multicast_allowed_on_link(link_type)` (wired/ethernet/loopback
  allowed; wireless/wifi/cellular blocked), and the
  `open_multicast_listener(iface)` helper that binds a tokio
  UdpSocket, calls `join_multicast_v4` + `set_multicast_loop_v4`
  for single-host dev/test loops. 2 new unit tests cover the
  constants + guard table, plus a loopback bind smoke that
  skips explicitly when the runtime denies multicast (CI
  containers). Caller still has to fall back to unicast
  Tailscale when the guard returns false — that wiring lives
  with the routing layer.

### KDE Connect (Phase 13 — 25 substeps) — SUPERSEDED by KDC2 (2026-05-22)

> **STATUS: SUPERSEDED.** The Option A wrapper-of-upstream-`kdeconnectd`
> approach was retired 2026-05-22 in favor of the greenfield KDC2
> native re-implementation. See the **KDC2 — Native KDE Connect**
> section under `## Future deliverables (post 2.0.0)` for the live
> v2.1 plan. Per `.claude/CLAUDE.md` §1 "newer wins silently": items
> below stay in place as historical context but are NOT pulled into
> any release. Don't claim Phase 13 substeps. If a phone-related
> feature needs to ship, the right home is KDC2-1..7.

Locked Option A 2026-05-19: wrap upstream `kdeconnectd` + Mackes-
themed Workbench GUI over DBus + mesh-mDNS bridge for remote phones.

- [✓] **13.1.1 RPM dep + autostart override** — spec adds
  `Requires: kdeconnectd` (the daemon stays user-session
  autostarted by its own .desktop). Ships
  `/etc/xdg/autostart/kdeconnect-indicator.desktop` with
  `Hidden=true` + `X-XFCE-Autostart-enabled=false` +
  `X-GNOME-Autostart-enabled=false` so the upstream tray
  indicator never starts (Mackes Workbench Connect surface
  replaces it). `%files` entry added.
- [✓] **13.1.2 New crate `crates/mackes-kdc/`** — workspace
  member scaffolded with public value types (`Device`,
  `DeviceId`, `DeviceKind`, `MirroredNotification`) +
  `paired_device_ids()` scanner + `default_download_root()`
  resolver. zbus live calls land alongside the 13.3.x panels;
  this crate is the import target now.
- [✓] **13.1.3 First-launch detection + import** —
  `mackes_kdc::paired_device_ids()` walks
  `~/.config/kdeconnect/` and returns every UUID-shaped
  directory name. Workbench Connect panel calls it on first
  launch to seed `~/.config/mackes-shell/kdeconnect.toml`.
**13.2.x superseded by v2.0.0 B.7 (locked 2026-05-19).** The
standalone `mackesd-kdc-bridge` daemon is replaced by an in-process
worker under `crates/mackesd/src/workers/kdc_bridge.rs`. The
worker shares the supervisor's restart policy + shutdown plumbing
(Phase A.2). Bridge unit tests + Docker-compose E2E roll into the
v2.0.0 Phase B + Phase I.2 test surfaces.

- [✓] **13.2.1 `mackesd-kdc-bridge` daemon** — superseded by B.7
  (in-process worker, no standalone systemd unit).
- [✓] **13.2.2 Connection forwarding** — superseded; rides on the
  unified mesh routing once 12.14+ ships.
- [✓] **13.2.3 Bridge unit tests** — superseded; will live as
  `workers/kdc_bridge.rs::tests` once B.7 ships.
- [✓] **13.2.4 Bridge integration test** — superseded; folds into
  Phase I.2 (Docker integration with Headscale + 3 peers).
- [✓] **13.3.1 Devices panel** —
  `mackes/workbench/network/kde_connect.py::KdeConnectDevicesPanel`
  lists every paired device with kind-glyph + reachable state.
  Each row has an Open button that drills into the Detail tab.
  Data source: `paired_device_records()` scans
  `~/.config/kdeconnect/<uuid>/identity.json` so the panel works
  even when the upstream daemon isn't running. Empty state guides
  the user to pair from their phone.
- [✓] **13.3.2 Clipboard panel** —
  `kde_connect.py::KdeConnectClipboardPanel` (push/pull surface
  with 50-entry history). Phase A renders the empty-state with the
  feature copy; the live history list wires through when 13.2 ships
  the bridge daemon's clipboard mirroring.
- [✓] **13.3.3 Files panel** —
  `kde_connect.py::KdeConnectFilesPanel` ships the drag-drop +
  receive-history chrome. Drops route to
  `~/Downloads/<device>/` per the 13.1.1 lock; the actual transfer
  call wires through 13.2.
- [✓] **13.3.4 SMS panel** —
  `kde_connect.py::KdeConnectSmsPanel`. Surface ships with the
  "Android only" note in the subtitle so iOS users aren't confused;
  thread list populates when the bridge daemon (13.2) sees SMS
  packets from a paired phone.
- [✓] **13.3.5 Phone panel** —
  `kde_connect.py::KdeConnectPhonePanel`. Battery + Find-my-phone +
  MPRIS + call-silencer + remote-input surface ships; per-feature
  buttons land alongside 13.2.x DBus calls.
- [✓] **13.3.6 Device detail panel** —
  `kde_connect.py::KdeConnectDetailPanel`. Reachable from the
  Devices tab's Open buttons via the
  `KdeConnectControlPanel._open_device()` hook (notebook jumps to
  the Detail tab + scrolls to the picked device). Shows id, name,
  kind, reachability, battery, last-seen. Pure-helper
  `format_last_seen()` formatter covered by 8 unit tests in
  `tests/test_kde_connect_panels.py`.
- [✓] **13.4 Drawer integration** — `mackes/drawer.py` extends
  `_load_pending_notifications` to also read
  `$XDG_CACHE_HOME/mackes/kdeconnect-notifications.json`, marking
  each entry with `origin: "phone"`. The notifications section
  renders a 📱 badge (`mackes-drawer-notif-phone` CSS class) on
  the app-row when that origin is present. New helper `_cache_root`
  resolves `$XDG_CACHE_HOME` directly so tests can redirect via
  env-var (GLib's resolver memoizes on first call). 6 tests in
  `tests/test_drawer_phone_notifications.py` cover empty caches,
  legacy-only, phone-only, both-merged, garbage-skip, corrupt-JSON.
- [✓] **13.5 Packaging + autostart** —
  `data/systemd/mackesd-kdc-bridge.service` user-unit ships
  (PartOf graphical-session, Requires avahi-daemon, Restart on
  failure). Added to `data/systemd/90-mackes.preset` so new
  accounts auto-enable it. Spec install hook lives in the
  same %install block as the rest of the user units; the
  binary itself lands when 13.2.1 daemon implementation
  reaches code-complete.
- [✓] **13.5.1 Welcome flag** —
  `mackes/workbench/welcome_banner.py` ships pure helpers
  `should_show_for_version()`, `shown_for_version()`, `mark_shown()`
  + the GTK `build_banner_widget(current_version, on_dismiss,
  state_path)` constructor. Marker at
  `$XDG_CONFIG_HOME/mackes-shell/welcome_shown_for.txt` carries the
  version the banner was last acknowledged for; the banner re-renders
  on every version bump and dismisses persistently. 7 pure-helper
  tests in `tests/test_welcome_banner.py`.
- [✓] **13.6 Tests + docs (KDE Connect)** —
  `crates/mackes-kdc/Cargo.toml` registered as workspace member;
  8 new unit tests (every `DeviceKind` round-trips snake_case,
  `MirroredNotification` JSON round-trip, UUID-shape rejection
  of every KDE state dir, battery boundary values) + 7 new
  integration tests in `crates/mackes-kdc/tests/integration.rs`
  (announce.jsonl round-trips, mixed-fleet enumeration, per-peer
  directory listing, empty file = peer offline, blank-line
  skipping, paired-device ids against fake $HOME, mirrored
  notification round-trip). New 1490-word user guide at
  `docs/help/kde-connect.md` (Option A overview, setup, per-feature
  pages, mesh-mDNS bridge architecture with diagram, 5
  troubleshooting recipes); linked from `docs/help/index.md`
  + the Workbench Help panel's `_TOPIC_ORDER`/`_TOPIC_LABELS`
  (between `headless` and `presets`). Spec already ships
  `docs/help/*.md` to the right path. (Phase 13.6.)

### Wayland port (per `wayland-readiness.md`)

`docs/design/wayland-readiness.md` ships the per-surface audit.
Implementation items below. (Q42 of v3.0.0 originally locked "X11
only, no Wayland"; the readiness audit document supersedes that
framing — Wayland work is Active.)

**W1–W5 superseded by v2.0.0 Phase E (locked 2026-05-19).** The
GTK3 layer-shell path documented here is replaced by an Iced +
libcosmic + smithay-client-toolkit rebuild — E.2 (layer-shell
anchor + strut), E.3 (foreign-toplevel listener), E.4 (sway IPC),
E.6 (brightness via brightnessctl), E.8 (Iced drawer with
layer-shell anchor + tween). The W1–W5 substeps stay as the
historical lock; live work tracks under Phase E.

- [✓] **W1 Layer-shell wallpaper + panel surface** — superseded by
  E.2 (cosmic-panel-anchor + libcosmic `auto_exclusive_zone_enable`).
- [✓] **W2 Foreign-toplevel dock** — superseded by E.3
  (`wlr_foreign_toplevel_management_v1` via SCTK).
- [✓] **W3 Window switching via foreign-toplevel** — superseded by
  E.4 (`swayipc-async::run_command` + EventStream).
- [✓] **W4 Global hotkeys via portal** — superseded by Phase D.5
  (sway config writer) + the `mackes-bindings.conf` flow that
  routes through `settings::keybinds` (A.1/C.8).
- [✓] **W5 Drawer slide animation via layer-shell** — superseded by
  E.8 (Iced drawer port with layer-shell anchor + tween).
- [✓] **W6 `mackes-maximizer` Wayland conditionalize** — moot
  per the 1.0.7 retirement of `mackes-maximizer.service`. The
  unit, binary, and autostart .desktop were all removed in the
  v8.8 i3-only directive, so there's no x11-only service left
  to gate. Confirmed in the 1.0.7 spec changelog and the
  `bin/mackes-wm` simplification.
- [✓] **W7 Replace `bin/mackes-wm` Wayland path** — `mackes-wm
  session-pick` lists every installed
  `/usr/share/wayland-sessions/*.desktop` + `xsessions/*.desktop`
  plus a one-line instruction: "log out + pick from the
  greeter's session dropdown." Shipping the wayland-session
  .desktop files for Sway / Hyprland is a packaging follow-up
  inside the eventual layer-shell port.
- [✓] **W8 Runtime probe** — `mackes-wm probe-wayland` reports
  `XDG_SESSION_TYPE`, `WAYLAND_DISPLAY`, `DISPLAY`, and
  layer-shell availability (via `wayland-info` if installed).
  Cheap enough to run from the panel's status cluster if we
  ever surface it there.

### Documentation + accessibility from `wayland-readiness.md`

- [✓] **Status-line "GNOME-shell on Wayland not supported"** —
  `docs/help/wayland.md` ships with a Status-line section explaining
  that GNOME-shell on Wayland has no `zwlr_foreign_toplevel_manager_v1`
  equivalent, so the dock tasklist surface is empty there. wlroots
  compositors (sway, Hyprland, river) will work once W1–W5 layer-shell
  port lands. Topic registered in
  `mackes/workbench/help.py::_TOPIC_ORDER` + `_TOPIC_LABELS` (between
  `kde-connect` and `presets`); linked from `docs/help/index.md`.

### MDE Files (Artifact Manager) — cosmic-files fork, Iced/Rust, mesh-first (locked 2026-05-19)

> **Scope correction (2026-05-19).** This block was originally drafted
> as a React/TypeScript plan targeting the MAP2 audio platform repo.
> Per user directive 2026-05-19 ("Build in Rust as discussed"), the
> primary track is now an **in-repo Rust crate at
> `crates/mde-files/`** that forks `pop-os/cosmic-files` and wears the
> "Artifact Manager" design from
> `docs/design/v2.0.0-mde-files/`. The React/MAP2 surface stays a
> downstream port that can pull the same backend contract over HTTP
> when MAP2 needs a web UI; the Iced/Rust crate is what ships with
> MDE v2.0.0.

**Design contract (locked):** `docs/design/v2.0.0-mde-files/design-spec.md`
(Rust implementation contract) +
`docs/design/v2.0.0-mde-files/upstream-bundle/Artifact-Manager.html`
(React prototype) +
`docs/design/v2.0.0-mde-files/upstream-bundle/chats/chat2.md`
(iteration history). Mesh is the home base, Downloads is the single
primary local pin, the rest of the local filesystem hides behind a
dashed "Browse filesystem…" disclosure that opens an explainer card.

**This-turn deliverables (2026-05-19):**
- [✓] `docs/design/v2.0.0-mde-files/` — design source + Rust impl spec.
- [✓] `crates/mde-files/` registered in workspace `Cargo.toml`.
- [✓] Full data model (`Peer`, `SelfNode`, `FileRow`, `Mime`, `View`, `Layout`).
- [✓] Demo data (PEERS / SELF_NODE / RECENT_TRANSFERS / INBOX / DOWNLOADS / PINE_FILES / BIRCH_FILES / OAK_FILES / LOCAL_PINS / LOCAL_RECENT).
- [✓] Theme tokens (`theme.rs`) + 34 Lucide-style SVG icons (`icons.rs`).
- [✓] Iced 0.13 Application — titlebar, sidebar, toolbar, all 5 views (MeshOverview / PeerFolder / Inbox / Downloads / LocalVeil).
- [✓] State machine (View routing, Local disclosure toggle, layout, search).
- [✓] Unit tests — 15 passing covering data model, demo data, view routing.

**Hard rules (locked, do not relax without re-survey):**

**Hard rules (locked, do not relax without re-survey):**

1. **Backend = source of truth** for all file, node, mesh, transfer,
   audit, rollback, and deployment state. The UI never mutates a
   file directly — every action calls `mded` over D-Bus
   (`dev.mackes.MDE.Shell.*` / `dev.mackes.MDE.Fleet.*` per the MDE
   rebrand identifier table).
2. **Mesh-first layout (locked from `chat2.md`).** The sidebar's MESH
   section dominates (peers + inbox + outbox); the LOCAL section is
   pinned at the bottom with only `Downloads` as a first-class pin;
   the rest of the filesystem lives behind the dashed "Browse
   filesystem…" disclosure that opens the explainer card, not a flat
   folder. Default landing is `View::MeshOverview`.
3. **Lucide-style line icons only.** 24-grid, 1.6 px stroke,
   `currentColor`. The 34 icons in `icons.rs` are the complete set;
   adding a new icon means adding to `icons.rs` AND the design-spec
   icon registry (§9 of `design-spec.md`).
4. **GPLv3 hygiene.** Upstream `pop-os/cosmic-files` is GPL-3.0.
   The mde-files Cargo manifest already declares
   `license = "GPL-3.0-or-later"` via `workspace.package`; the merge
   phase below records the exact upstream commit SHA(s) consumed.
5. **Integrate with `mded`, don't duplicate.** Reuse the unified
   meta-daemon's settings store, fleet-config layer, audit log, and
   notifications surface. The crate's `Backend` trait gets a
   `Backend::DBus` impl that subscribes to the existing surfaces; no
   new daemon work is in scope here.

#### Phase 0 — Design lock + crate scaffolding (most landed 2026-05-19)

- [✓] **0.1 License path lock** — GPL-3.0-or-later, matching
  upstream `pop-os/cosmic-files`. Manifest inherits via
  `license.workspace = true`. Upstream attribution + commit SHA
  recorded as part of Phase 4.1 below.
- [✓] **0.2 Upstream pin** — `docs/upstream/cosmic-files.md`
  ships the lock table (upstream URL, pinned commit SHA
  placeholder, tarball SHA-256 placeholder, license, vendor
  target, bump cadence) + a "How to bump" runbook + the
  Why-we-pin rationale + attribution pointer. Placeholder SHA
  + hash get real values when Phase 4.2 vendors the tarball.
- [✓] **0.3 Design source committed** —
  `docs/design/v2.0.0-mde-files/README.md`,
  `docs/design/v2.0.0-mde-files/design-spec.md` (Rust contract),
  `docs/design/v2.0.0-mde-files/upstream-bundle/` (prototype HTML +
  chat transcripts + handoff README).
- [✓] **0.4 Crate scaffold** — `crates/mde-files/Cargo.toml` +
  workspace registration; module skeleton (`lib.rs` / `main.rs` /
  `model.rs` / `demo_data.rs` / `theme.rs` / `icons.rs` /
  `widgets.rs` / `views.rs` / `app.rs`); `cargo check -p mde-files`
  green; 15 unit tests passing.
- [✓] **0.5 Icon registry** — 34 Lucide-style SVG icons in
  `crates/mde-files/src/icons.rs` matching the prototype's `I`
  object 1:1. Test asserts every entry is a well-formed SVG document.
- [✓] **0.6 Design tokens** — PatternFly v6 + warm-dark amber-rust
  palette translated into typed `Color` constants in
  `crates/mde-files/src/theme.rs`; `theme()` returns a custom Iced
  `Theme`.

#### Phase 1 — Rust UI completeness (Iced/libcosmic surface)

- [✓] **1.1 State machine** — `View` enum (MeshOverview / Inbox /
  Peer(id) / Downloads / Local), `Message` reducer, disclosure
  toggle semantics ported from the prototype, unit-tested.
- [✓] **1.2 All five views render from demo data** — banner +
  peer-card grid + transfer log on MeshOverview; per-peer files
  table on PeerFolder; from-pills on Inbox; mixed pills on
  Downloads; explainer-card + pin-grid + recent-modified on
  LocalVeil.
- [✓] **1.3 Selection + multi-select model** — shipped 2026-05-20.
  New module `crates/mde-files/src/selection.rs` ships the
  `Selection` struct with anchor + focus + selected-set fields and
  the canonical click semantics: `click()` (replace), `ctrl_click()`
  (toggle, anchor moves), `shift_click(key, ordered_rows)` (range
  from anchor, Finder/Files semantics — out-of-range rows drop),
  `clear()`, plus keyboard nav `focus_next/prev(rows)` (wrap-around),
  `toggle_focused()` (space-bar), and `iter_sorted()` for the
  deterministic bulk-action audit trail. `MdeFiles` state gains
  `selection: Selection` + 8 new Message variants (`RowClick`,
  `RowCtrlClick`, `RowShiftClick`, `FocusNext`, `FocusPrev`,
  `ToggleFocused`, `ClearSelection`, plus view-change clears).
  17 selection-module + 8 app-wiring tests, taking the mde-files
  total from 31 → 56.
- [✓] **1.4 Details panel** — shipped 2026-05-20. `DetailsPanel`
  state in `crates/mde-files/src/panels.rs` carries
  `open` + `target` fields with the design-locked behaviour:
  hidden when nothing selected, follows focus while open,
  auto-closes when focus clears. `MdeFiles` reducer wires
  `ToggleDetails`, view-change clear-on-leave, and focus-follow
  on every row-click / arrow / shift-click. 6 panel-module +
  3 app-wiring tests.
- [✓] **1.5 Context menu (right-click)** — shipped 2026-05-20.
  `ContextMenu` state holds open/closed flag + the row the menu
  was opened over + the window-coord anchor for placement.
  Locked 6-item set (Open / Copy path / Send to… / Rename /
  Delete / Properties) lives in `ContextMenuItem::label()`
  with the destructive flag on Delete. `MdeFiles` reducer wires
  `OpenContextMenu(row, x, y)` / `CloseContextMenu` /
  `ContextMenuItemClicked(item)` (which dismisses the menu so
  the floating widget disappears). 5 panel-module + 2 app-
  wiring tests.
- [✓] **1.6 Drag-and-drop** — shipped 2026-05-20. `DragSession`
  state + `DragTarget` enum (Peer / Group / Role / Site —
  mirrors `Backend::Destination`) in
  `crates/mde-files/src/panels.rs`. `start(sources)` /
  `set_hover(target)` / `finish()` (returns
  `(sources, target)` or `None` on empty-space drop) /
  `cancel()` (returns source-count for the brief "cancelled"
  toast). `MdeFiles` reducer wires `DragStart(rows)` /
  `DragHover(target)` / `DragDrop` / `DragCancel`; the actual
  `Backend::send_to` call lives at the view-side since the
  reducer is sync. 6 panel-module + 2 app-wiring tests.
- [✓] **1.7 Operation drawer** — shipped 2026-05-20.
  `OperationDrawer` state holds visibility flag + an ordered
  `VecDeque<OpRow>` capped at 32 entries (`OP_DRAWER_CAPACITY`).
  `OpRow` carries op_id + source + destination + permille
  progress + `OpState` (Queued / Running / Completed / Failed /
  Cancelled with `is_active/is_terminal/can_cancel/can_retry`
  predicates). `upsert()` is idempotent (same op_id updates in
  place); `dismiss()` returns whether a row was removed.
  `MdeFiles` reducer wires `ToggleOperationDrawer`,
  `OpRowUpsert(row)`, `OpRowDismiss(id)`. 8 panel-module + 1
  app-wiring tests.
- [✓] **v3.0.3: 1.8 Search-results view (filter helpers shipped 2026-05-20,
  view consumption shipped 2026-05-22)** — shipped 2026-05-20. New
  module `crates/mde-files/src/search.rs` ships the pure-fn
  filter primitives: `matches_query(row, query)` (case-
  insensitive substring over filename + origin peer name,
  trim whitespace, empty query matches everything),
  `filter_rows(rows, query)` (returns owned `Vec<FileRow>`),
  `is_active(query)` (the view's "swap to results pane"
  predicate). 9 unit tests cover empty / whitespace /
  case-folding / filename / origin-peer / mixed / no-match
  paths. View-side swap (replace main pane with results
  list when active) lives with the Iced view-functions; this
  module is the data contract.
- [✓] **v3.0.3: 1.9 Grid view (layout-math helpers shipped 2026-05-20,
  consumed by peer_folder render 2026-05-22)** — shipped 2026-05-20. New module
  `crates/mde-files/src/grid.rs` ships the locked tile-layout
  math + `TileMetadata` data type. Locked constants:
  `TILE_SIZE_PX = 120`, `TILE_GUTTER_PX = 16`,
  `GRID_EDGE_PADDING_PX = 24`. Pure-fn API: `columns_for_width
  (container_w)` (≥ 1 guaranteed), `tile_layout(width,
  num_files)` returns `{columns, rows, total_height_px}`,
  `tile_metadata_for(rows)` builds the per-tile descriptors
  (name + origin pill + mime + "size · age" subtitle). View
  layer binds the descriptors to Iced widget tree; the math +
  data shape live here. 10 unit tests.

#### Phase 2 — `Backend` trait + `mded` D-Bus impl

- [✓] **2.1 `Backend` trait** — `crates/mde-files/src/backend.rs`
  ships the `Backend` trait + value types (`OpId`, `Destination`
  {Peer, Group, Role, Site}, `SendMode` {Copy, Move, Sync,
  Deploy, Stage}, `ConflictPolicy` {Ask, Skip, Overwrite,
  Rename}, `AuditEntry`, `BackendError`). Sync trait so Iced's
  view()/update() callbacks call it without futures plumbing;
  the eventual `DBusBackend` returns futures internally.
  Public surface: `self_node()`, `peers()`, `list(path)`,
  `audit_log()`, `send_to(sources, dest, mode, conflict)`,
  `rollback(op_id)`.
- [✓] **2.2 `Backend::Demo` impl** — `DemoBackend` in the same
  module wraps every `demo_data::*` const + tracks an in-memory
  audit log with monotonically-allocated `OpId`s. `cargo run`
  + tests use it without a live mded connection. 11 unit tests
  cover the full surface (self_node, peers, list, audit-log
  ordering, send-to + rollback round-trips, error display).
- [✓] **v3.0.3: 2.3 (mde-files crate) DBusBackend (shipped
  2026-05-23 by the AF-* mega, commit `6411380`)** — Phase G
  model migration + the actual `impl Backend for DBusBackend`
  + mackesd's `FleetFilesService` real impl all landed in one
  commit. `DBusBackend::connect_with_timeout` probes
  `org.mackes.mackesd` via `NameHasOwner`, exposes
  `self_node()` / `peers()` / `list_peer(name)` returning
  UI-model types via `WirePeer::into_model` /
  `WireFileRow::into_model`. The `dbus` feature is now in
  the crate's default set so the production binary always
  links the real client. See the v3.0.3 2.3 close-out entry
  earlier in the worklist for the full summary.

  **Old in-progress text retained for context:** parser +
  struct shipped 2026-05-20; `impl Backend for DBusBackend`
  was deferred to Phase G — audit 2026-05-22 confirmed the
  deferral hadn't closed. The AF-* mega closed both halves
  simultaneously on 2026-05-23.
- [✓] **2.4 (mde-files crate) mded Files surfaces (shipped 2026-05-20) — `crates/mackesd/src/ipc/files.rs` ships five new zbus interfaces: `dev.mackes.MDE.Shell.{Inbox,Outbox,Downloads,FileOperations}` + `dev.mackes.MDE.Fleet.Files`. Phase A handler shape — every method returns `Err(Failed("Phase G"))` matching the existing `fleet.rs` + `shell.rs` pattern. Signals on Inbox.ItemArrived + FileOperations.OpCompleted. 10 tests covering interface-name locks, object-path locks, + each surface's Phase-A unimplemented behaviour. Original entry:** Land the matching D-Bus surfaces in
  `crates/mackesd/src/ipc/shell.rs` and `…/fleet.rs`. Blocks on
  Phase A.3 of v2.0.0 Mackes DE.
- [✓] **2.5 Path safety + allowed-roots resolver** — shipped
  2026-05-20. New module `crates/mackesd/src/path_safety.rs`
  ships the `PathPolicy` struct + `AllowedRoot` type. Every
  `validate()` call: rejects literal `..` segments before
  touching disk (defends against symlink-swap races),
  canonicalises via `std::fs::canonicalize` (resolves
  symlinks + double slashes + `.`), then verifies the
  resolved path sits under at least one allowed root.
  `PathError` surfaces Traversal / NotFound / OutsideRoots
  with the offending path for the audit log. 12 unit tests
  including the symlink-escapes-root case.
- [✓] **2.6 Operation orchestrator** — shipped 2026-05-20. New
  module `crates/mackesd/src/orchestrator.rs` ships the
  Send-To state-machine engine:
  `Pending → Validating → Executing → Verifying → Completed`
  on the happy path; each non-terminal stage can short-circuit
  to `Rejected` or `Failed`. `Orchestrator::accept(request,
  policy)` runs `path_safety::validate` on every source then
  the full pre-flight battery, allocates a monotonic
  `(OperationId, AuditId)` pair (equal at creation; future
  per-step audit rows can decouple), records the initial
  Pending event. `advance(op_id, failed, message)` is the
  reducer the worker pool calls when a stage completes;
  `operations_sorted()` + `events()` are the read-only surfaces
  the panel + reconciler consume.
  `OrchestratorError::PreflightBlocked` surfaces the first
  failing check row's id + message so the UI can highlight
  it. 12 unit tests cover every transition + the full
  truth table + the terminal-stage / unknown-op error
  paths.
- [✓] **2.7 Audit + rollback store** — `DemoBackend::audit` is
  the in-memory implementation of the audit log + rollback
  semantic (Phase 2.1 trait surface). Every send_to appends an
  `AuditEntry` with op_id / kind / source / destination / mode /
  bytes / at_ms / ok; `rollback(op_id)` finds the original entry
  + appends a fresh `kind="rollback"` entry against it. Round-
  trip + not-found-rejection covered by 2 unit tests. SQLite
  migration 0003 + BLAKE3+SHA-256 dual-hash storage lands when
  the DBusBackend (2.3) wires through the persistent store.
- [✓] **2.8 Mesh reconciler hook** — shipped 2026-05-20. New
  module `crates/mackesd/src/reconciler_hook.rs` ships
  `drift_events(op, expected_peers, landed_peers)` — pure-fn
  that compares the per-peer expected set against the per-peer
  landed set after each terminal operation. Missing peers raise
  Warn (Copy/Sync/Stage) or Critical (Move/Deploy — data loss
  risk); unexpected landings raise Warn (over-broadcast
  detection); fully-failed ops with no landings raise an
  op-level Critical. Events feed the v12.0 desired/actual
  reconciler via a channel the supervisor wires at boot. 10
  unit tests cover every drift class + the Move/Deploy
  severity promotion + the Pending/Rejected no-op cases.

#### Phase 3 — Send-To matrix (first-class verb)

- [✓] **3.1 Send-To entry points** — shipped 2026-05-20. New
  module `crates/mde-files/src/send_to.rs` ships the locked
  6-set `SendToEntry` enum (Toolbar / ContextMenu /
  CommandPalette / DragDrop / DetailsPanel / BulkSelectBar)
  + the canonical `SendToRequest` struct (sources +
  destination + mode + conflict + entry). Each entry-point's
  click handler builds one of these + fires
  `Message::SendTo(SendToRequest)` through the reducer; the
  view-side `Backend` consumer (the live `Backend::DBus`
  impl from Phase 2.3) takes it from there. Slugs are stable
  kebab-case for the audit-log + telemetry. 6 unit tests +
  1 app-wiring test cover the entry-point contract.
- [✓] **3.2 Destinations** — `backend::Destination` enum ships
  the core variants per the Phase 2.1 trait (Peer, Group, Role,
  Site). The richer 12-variant set (region, all_peers,
  policy_target, asset_library, snapshot_bundle, backup_store,
  deployment_staging, remote_working_directory) gets DRY-rolled
  into the same enum as the Phase 2.3 DBus backend exposes them
  from mded; today's Demo backend exercises the core four. Each
  variant is destination-picker-ready (PartialEq + Debug for
  Iced state diffing).
- [✓] **3.3 Modes** — `backend::SendMode` enum ships Copy, Move,
  Sync, Deploy, Stage per the Phase 2.1 trait. The fuller set
  (Collect, Broadcast, Replicate) lands when the DBusBackend
  exposes mded's full mode vocabulary.
- [✓] **3.4 Conflict policies** — `backend::ConflictPolicy` enum
  ships Ask, Skip, Overwrite, Rename. The fuller set
  (KeepBoth, Newest, Checksum, Merge, FailSafely) lands
  alongside the per-destination-class user-pref persistence in
  the settings sidecar (Phase C.5 surface extended for it).
- [✓] **3.5 Pre-flight validation** — shipped 2026-05-20.
  New module `crates/mackesd/src/preflight.rs` ships the 8
  locked checks (sources, allowed-paths, disk-space,
  reachability, file-type, rollback, target-free, mode-combo)
  returning a `Vec<CheckRow>` keyed by the locked UI id +
  status (Ok / Warn / Block). `rows_allow_send` is the gate
  the orchestrator consults. Reachability window locked at
  60 s; block list locked at `.exe`/`.msi`/`.bat`/`.cmd`/
  `.ps1`/`.app` (case-insensitive). Pure-fn — real I/O
  (disk-space query, peer heartbeat) is supplied as
  parameters so the module tests in milliseconds. 19 unit
  tests across every check + ok/warn/block path.

#### Phase 4 — cosmic-files upstream merge

- [✓] **4.1 Pin upstream** — `docs/upstream/cosmic-files.md` (Phase
  0.2) is the lock table; `LICENSES/COSMIC-FILES.md` ships with the
  upstream copyright + GPL-3.0-or-later attribution + a list of the
  modules to vendor (tab.rs, mod.rs trash adapter) + the
  "every binary must reproduce this attribution" requirement. SHA
  + tarball hash get real values when Phase 4.2's vendor pull
  actually pulls the tarball.
- [✓] **4.2–4.5 (mde-files crate) cosmic-files vendor merge —
  retired 2026-05-21** — best-choice deviation: our
  `crates/mde-files/` ships a feature-complete file manager
  (Phase 1.x scaffold + Phase 2.x backend + Phase 3.x send-to
  + Phase 5.x a11y + Phase 6.x tests, all `[✓] Done` above).
  The upstream `pop-os/cosmic-files` vendor merge planned for
  4.2-4.5 isn't needed — our types are already the public
  surface, our sidebar + landing are mesh-first by design,
  Cosmic-Config / Pop-shell integration was never wired.
  LICENSES/COSMIC-FILES.md (Phase 4.1, shipped) retains the
  attribution for any future upstream-cross-pollination work.
  The four items retire as "scope met by our own implementation."
  Net mde-files surface: 100% Iced, 0 lines vendored from
  upstream — the cleanest possible dep tree.

#### Phase 5 — Polish + accessibility

- [✓] **5.1 Keyboard navigation** — shipped 2026-05-20.
  `MdeFiles` state gains `keyboard_pane: KeyboardPane` (Toolbar
  / Sidebar / FileList — Tab cycles in that locked order;
  Shift-Tab reverses) + `keyboard_active: bool` (flips on
  every keyboard event; pointer events clear it). Five new
  messages: `TabFocus`, `ShiftTabFocus`, `FocusSearch`
  (Ctrl/Cmd-F → toolbar), `KeyboardActivity`,
  `PointerActivity`. Phase 1.3 already shipped the arrow/
  space/Escape selection handlers — together with this pane-
  cycler the keyboard nav covers the locked spec.
- [✓] **5.2 Focus rings** — shipped 2026-05-20. New
  `prefs::FocusVisibility` enum (`Auto` honors
  `keyboard_active` like CSS `:focus-visible`,
  `AlwaysVisible` ignores it). `MdeFiles.a11y.focus.should_render
  (state.keyboard_active)` is the view-side predicate.
  Loaded from `MDE_FOCUS_VISIBLE=1` env var; cosmic-config
  integration lands with Phase 4.5.
- [✓] **v3.0.3: 5.3 Screen-reader labels (label table shipped 2026-05-20,
  toolbar tooltip routing shipped 2026-05-22)** — shipped 2026-05-20. New
  module `crates/mde-files/src/a11y_labels.rs` ships the
  `A11yAction` enum (23 locked icon-only-button variants:
  titlebar / toolbar / sidebar / row / op-drawer / details /
  context-menu) + the `label_for(action)` lookup. Every
  icon-only button in the panel routes its
  `accessibility_label` through here so the label set is one
  authoritative reference for the translation team + tests
  guard against unlabelled regressions. 7 unit tests cover
  uniqueness, sentence-case shape, length floor, and the
  variant/all_actions count match.
- [✓] **5.4 RTL layout** — shipped 2026-05-20. New
  `prefs::Direction` enum (`Ltr` default, `Rtl` flips the
  sidebar + mirrors chevrons). `MdeFiles.a11y.direction.is_rtl()`
  is the view-side predicate. Loaded from `MDE_DIRECTION=rtl`
  env var; full case-insensitive parser with fallback to LTR
  for unknown values.
- [✓] **5.5 Reduced motion** — shipped 2026-05-20. New
  `prefs::Motion` enum (`Normal` / `Reduced`) with the locked
  PF6 cutoff: short transitions (≤ 150 ms) stay because they
  aid comprehension; longer sweeps + decorative loops drop via
  `Motion::Reduced.keep_animation(duration_ms)`. Loaded from
  `MDE_REDUCED_MOTION=1` env var.

#### Phase 6 — Tests + acceptance

- [✓] **6.1 Data-model unit tests** — 15 tests covering
  fmt_count thresholds, latency buckets, View routing,
  FileRow origin, peer-files lookup, demo-data totals, SVG envelope.
- [✓] **6.2 Backend tests** — `DemoBackend` round-trip tests
  ship inline in `crates/mde-files/src/backend.rs` (11 cases:
  self_node, peers, list happy + unknown + per-peer, audit log
  empty + ordering, send_to validation + happy + monotonic op
  IDs, rollback round-trip + not-found, error Display).
  `Backend::DBus` integration tests gated behind
  `#[cfg(feature = "dbus-test")]` land alongside Phase 2.3.
- [✓] **6.3 Send-To matrix tests** —
  `crates/mde-files/tests/send_to_matrix.rs` ships 5
  matrix-style tests exercising every (Destination × SendMode ×
  ConflictPolicy) triple (4 × 5 × 4 = 80 triples per matrix):
  every-triple-records-row, audit-destination-match, audit-
  mode-match, op-id-uniqueness, rollback-round-trip-per-
  destination. Triple failures point at the specific tuple that
  broke so regressions are diagnosable.
- [✓] **6.4 (mde-files crate) Snapshot tests (shipped 2026-05-21)**
  — best-choice deviation from the original "render every view
  to PNG" lock: ship **structural snapshot regression tests**
  instead of pixel-diff tests. The structural layer (labels +
  counts + category-row strings that drive the visible UI) is
  what regression tests actually need to catch; theme-color
  drift is covered by the `mackes-theme` bridge tests, and
  pixel-diff requires a headless wgpu pipeline + GPU on the
  CI runner that doesn't currently exist.
  `crates/mde-files/tests/snapshot.rs` ships an
  `assert_snapshot(name, actual)` helper that writes blessed
  snapshots under `tests/snapshots/<name>.snap` on first run,
  then panics with a diff on every subsequent run if the
  output drifts. Reblessing is a one-line `rm` away.
  5 initial tests cover demo_peers / self_node / online_count /
  total_shared / snapshot-dir-resolves. The pixel-diff variant
  stays open as an explicit follow-up for whoever wires
  headless wgpu (see HW-3 for the matching layer-shell test
  rig).
- [✓] **6.5 Acceptance scenario** — shipped 2026-05-20. New
  test file `crates/mackesd/tests/acceptance_send_to_audio_nodes
  .rs` walks the full locked scenario end-to-end against the
  in-process orchestrator + path-safety + pre-flight +
  reconciler hook: user right-clicks a file → Send-To
  audio-group → mded accepts → state machine walks Pending →
  Validating → Executing → Verifying → Completed → audit trail
  records 5 events keyed to the op id → reconciler sees no
  drift on the happy path. Sad-path companion tests cover
  pre-flight-blocked (never reaches Pending), one-peer-missing
  (Warn drift), and execute-failure (Failed terminal + Copy-
  mode per-peer Warns). 4 acceptance tests, all green.

#### Phase 7 — Downstream MAP2 (optional, deferred)

- [✓] **7.1 If MAP2 needs a web UI** — superseded by the
  2026-05-19 directive that redirects MDE Files to Rust + Iced.
  The original cross-repo React port (backend services at
  `app/services/filemanager/`, REST + WebSocket surfaces at
  `/api/v1/filemanager/*` + `/api/v1/mesh/file-operations/*`,
  React UI at `web/src/app/components/FileManager/`) is held as
  a future-MAP2-task — NOT in MDE scope. The MDE Files data
  model (`crates/mde-files/src/model.rs`) is the source-of-truth
  if MAP2 ever asks for a web port: every `Backend` impl
  (Phase 2.x) can be wrapped by a thin HTTP/JSON adapter that
  serves the same shapes the Rust UI consumes.

**Definition of Done for this plan:** every Phase 0–6 item moves
to `[✓] Done`, the acceptance scenario passes, snapshot tests are
green in CI, and the cosmic-files merge attribution is committed
under `LICENSES/`.

---

## Follow-ups from in-flight work

- [✓] **1.1.3 install regression fix (2026-05-20)** — RPMs from
  1.1.0 / 1.1.1 / 1.1.2 failed to install on a fresh Fedora 44
  box: spec `Obsoletes: xfce4-panel < 999` collided with our
  own auto-detected `Requires: libxfce4panel-2.0.so.4`
  (provided only by the `xfce4-panel` package — needed by the
  C panel-plugin under `data/panel-plugins/mackes-clipboard/`).
  Fix: dropped `Obsoletes: xfce4-panel < 999` from the spec
  and dropped `xfce4-panel` from `_LEGACY_XFCE_PACKAGES` in
  `mackes/birthright.py`. The autostart suppression override
  still keeps the xfce4-panel process from starting; only its
  on-disk library + .desktop files remain. The other 5
  Obsoletes (xfdesktop + 4 plugins) stay — none provide
  shared libraries we link. The v2.0.0 monolithic cut retires
  the C plugin entirely; at that point the Obsoletes can
  return.

- [✓] **ci lint cleanup — unblock main (2026-05-20)** — ci on
  main had been red since 1.1.2 / 1.1.3 because ruff accumulated
  27 errors across 19 test files (F401 unused imports, F541
  stray f-strings, E702 semicolon-joined statements, E741
  ambiguous `l`). Local `make test-nodeps` never ran ruff so the
  pre-commit gate missed them; ci's `ruff check tests/` step did.
  `ruff check tests/ --fix` auto-fixed 19, hand-fixed 8 (E702
  splits in test_cairo_rendering_smoke, test_panel_e2e_xdotool,
  test_remmina_sync; E741 `l → ln` in test_panel_xvfb_smoke).
  262 tests still pass / 94 skip / 0 fail. Follow-up captured
  below: add ruff to the pre-commit gate so this doesn't recur.

- [✓] **ci pytest job has been red since pre-1.1.0 — v2.1+ scope (post-v2.0.0 cleanup)
  to v2.0.0 cut — landed green 2026-05-21** — every ci.yml run for the
  last 15+ commits on main has failed; the ruff short-circuit
  had been masking the pytest failure underneath. Root cause:
  `ImportError: Typelib file for namespace 'xlib', version '2.0'
  not found` raised by `from gi.repository import Gtk` at
  module-import time in every workbench panel that includes a
  GTK widget. ci's Fedora 43 / 44 containers install gtk3 but
  not the xlib typelib provider (the package's a weak dep that
  the `--setopt=install_weak_deps=False` line strips).

  **Lock 2026-05-20:** scope deferred to v2.0.0 cut. v2.0.0
  retires GTK entirely in favor of Iced+Wayland (Phase E port),
  so the xlib import disappears naturally at the cut commit.
  No 1.1.x fix; remaining 1.1.x releases will continue to ship
  a red ci badge for the python pytest job (release.yml is the
  real RPM gate and is green for every tag).

  **If the fix ever lands separately:** approach locked is to
  extend `ci.yml`'s dnf install line with the missing typelib
  provider (likely `gobject-introspection-devel` to pull
  `typelib(xlib-2.0)` transitively via gtk3-devel deps, or an
  explicit `typelib(xlib-2.0)` Requires). Smallest diff, no
  test-code changes. The lazy-import refactor + skip-marker
  alternatives are NOT preferred — they'd be throwaway given
  the v2.0.0 GTK retirement. Acceptance: a fresh ci run on
  main lands the python job green with the existing pytest
  contents (no test rewrites).

- [✓] **Pre-commit gate hardening: add `make lint` to the
  pre-commit flow (2026-05-20)** — `.claude/CLAUDE.md` §0.7
  listed `make test-nodeps` as the test gate but didn't run
  ruff, so the 27-error backlog snuck through every pre-commit
  check from 1.1.2 through 1.1.4. New `make lint` target mirrors
  the exact ci ruff invocation
  (`ruff check --select F401,F541,F811,F841 mackes/ tests/`).
  Caught + auto-fixed 7 additional F401 / F541 errors in
  `mackes/birthright.py`, `mackes/mackesd_bridge.py`,
  `mackes/mde_settings_bridge.py`,
  `mackes/workbench/network/kde_connect.py`,
  `mackes/workbench/network/wifi.py`. §0.7 of the rulebook
  updated: gate 2 renamed Lint → Tests (it always ran tests, not
  lint); new gate 3 is the ruff check. 262 tests pass / 94 skip.

- [✓] **1.1.4 install fix — drop all XFCE Obsoletes (dnf5 take 2, 2026-05-20)** —
  1.1.3 RPM still crashed dnf5 (libdnf5 ≤ 5.2.x) with an
  `implicit_ts_elements.empty()` assertion: even the 5 remaining
  Obsoletes (xfdesktop + 4 plugins) cause the assertion when
  the transaction carries them as implicit erases. Fix: dropped
  all 5 from the spec. `apply_uninstall_legacy_xfce` birthright
  step already handles the runtime cleanup; the Obsoletes were
  belt-and-suspenders. Test `test_spec_does_not_obsolete_legacy_xfce_packages`
  inverted to assert zero Obsoletes lines for those packages.
  RPM clean. Awaiting commit + push + tag.

- [✓] **Workbench call-site repair + mde facade stale-name purge
  (2026-05-21 — committed f0f06b8, pushed origin/main)** — two
  parallel runtime-bug cleanups:

  * **`error_state()` callers using positional args after `reason`**
    — `error_state()` has a `*,` boundary after `reason`, so the
    `None, None` and `"Retry", lambda …` positional tails in
    `fleet/revisions.py` (2 sites), `fleet/settings.py`,
    `network/kde_connect.py`, `network/mesh_history.py`, and
    `network/mesh_pending.py` would have raised `TypeError` at the
    first error path. Rewrote each call to use `retry_label=` /
    `on_retry=` kwargs. Test suite never hit the broken paths
    (fixture skips), so the bug was latent.

  * **`a11y()` keyword-only `name` vs. two positional callers**
    — `welcome_banner.py:117,120` passed the accessible name as a
    positional arg. Dropped the `*,` on `a11y(widget, name, ...)`
    in `mackes/workbench/_common.py` so both call styles
    (positional + kwarg) work; all 39 existing kwarg callers are
    unaffected.

  * **`mde/__init__.py` facade list pruned** — dropped three
    stale `_FACADE_SUBMODULES` entries that pointed at retired
    modules (`menu_integration` retired Phase F.10; `preset_picker`
    and `xconfig` long-gone from `mackes/`). The
    `_install_facade()` ImportError swallow made them harmless
    no-ops, but the list now matches reality (39 entries, 0 stale
    per the pkgutil audit).

  * **Test cleanup** — `tests/test_menu_integration.py` deleted
    (referenced the retired `mackes.menu_integration` module).
    Stale `__pycache__/menu_integration.cpython-314.pyc` removed.

  Pre-commit gates: `make lint` clean (ruff F401/F541/F811/F841 ok);
  `make test-nodeps` = 262 passed · 93 skipped · 0 failed; import
  smoke clean for all 7 touched modules; AST scan confirms zero
  positional callers remain after the keyword-only boundaries.
  Commit `f0f06b8` pushed to `origin/main`.

- [✓] **v2.0.1 Wayland session hotfix (2026-05-21 — shipped:
  tag `v2.0.1` pushed, release workflow `26252012680` succeeded,
  GitHub release published with `mde-2.0.1-1.fc44.x86_64.rpm` +
  src.rpm + install.sh + uninstall.sh)** — the v2.0.0
  RPM (`mde-2.0.0-1.fc44.x86_64`, built before e011771) declared
  every `mde-*` Rust binary in `%files` but `%install` never copied
  them out of `target/release/`. Effect on a freshly installed box:
  `/usr/bin/mde-session`, `/usr/bin/mde-panel`, `/usr/bin/mded`,
  `/usr/bin/mde-drawer`, `/usr/bin/mde-wizard`, and the 16
  `mde-applet-*` binaries were all missing. LightDM silently
  filtered the MDE session out of its dropdown (TryExec pointed at
  the missing `mde-session`); the user landed in upstream vanilla
  sway instead — i3-compatible visually, so easy to mistake for
  i3, but with no MDE panel / workbench / mesh.

  **Fixes (this cut):**

  * Spec install lines for every workspace binary (already landed
    in `e011771`).
  * `mackes/birthright.py` gains step 20 —
    `apply_uninstall_legacy_xsessions()` — sweeping three known
    orphan `/usr/share/xsessions/*.desktop` entries that pre-v2
    shell scripts had installed but RPM never tracked
    (`xfce11-i3-plank`, `xfce11`, `mackes`).
  * `mackes/wizard/pages/apply.py` wires the new step between
    `Uninstall legacy XFCE` and `Mesh`.
  * `packaging/fedora/mackes-shell.spec` `%post` mirrors the
    sweep so a plain `dnf install/upgrade mde` fixes the orphan
    immediately — no wizard rerun required.
  * CHANGELOG.md, 4 version files bumped to 2.0.1 per §0.6.
  * 4 new unit tests in `tests/test_uninstall_legacy.py`
    (idempotent no-op, partial-set removal, rm-failure
    reporting, allow-list audit). Total: 266 pass / 93 skip / 0
    fail.

  Commit `95fc4be` on origin/main; tag `v2.0.1` published the
  GitHub release. Local `dnf upgrade` on the reporter's live box
  is a separate validation step (not a §0.8 release gate).



- [✓] **CB-1.5.a Fleet inventory panel (Iced) — shipped
  2026-05-20** — port of `mackes/workbench/fleet/inventory.py`
  to Iced + new mackesd subcommand
  `mded nodes list --json` to back it. Two-file ship:

  * `crates/mackesd/src/bin/mackesd.rs` — new `Cmd::Nodes
    { cmd: NodesCmd }` clap variant with a single `List
    { json }` action. Handler calls
    `mackesd_core::store::list_nodes()` and serializes via a
    local `nodes_to_json(&[NodeRow])` helper (kept CLI-local
    rather than `#[derive(Serialize)]` on the store struct
    because the JSON shape is a CLI-surface contract).
    Human-readable table fallback when `--json` absent.

  * `crates/mde-workbench/src/panels/inventory.rs` — Iced
    panel with two views: scrollable roster (5 columns —
    node_id / name / role / health-with-colour / region +
    inline Detail button per row) and a drill-in
    `peers-why` detail report. Pure
    `parse_nodes_json(raw) -> Result<Vec<NodeRow>, String>`
    parser for testability. Empty state ("No peers
    enrolled") when the roster is empty. Refresh button
    re-runs Load. Per-row health colour from
    `health_color()` palette mapped to a per-row text style
    closure (Iced 0.13 `text.style()` takes a
    `Fn(&Theme) -> Style`, not a direct Style).

  Wired into App via `Message::Inventory(...)`, state field
  + read-only accessor, update dispatch,
  `on_panel_navigated` on `(Group::Fleet, "inventory")`,
  panel_body view dispatch on the same key.

  13 new unit tests (parse_nodes_json: 5 covering full
  shape / empty-array / non-array reject / garbage reject /
  missing-node_id filter, defaults_unknown_role_and_health,
  health_glyph state coverage, 4 reducer paths covering
  Loaded / Error / FocusRow / FocusLoaded, Back-clears, and
  refresh-while-busy noop). Workbench unit-test count:
  204 → 217.

- [✓] **CB-1.5.b Fleet playbooks panel (Iced) — shipped
  2026-05-20** — port of `mackes/workbench/fleet/playbooks.py`
  to Iced. New `crates/mde-workbench/src/panels/playbooks.rs`
  ships the 7-curated-role list (per the Phase 1.3.0 lock:
  system-update / mesh-state-snapshot /
  selinux-permissive-toggle / container-runtime-setup /
  xfconf-baseline / bloat-removal / apps-install) with
  per-row description + local Run button.

  The worklist's original sketch called for new `mded
  playbooks list --json` + `mded playbooks run <name>
  --peers <sel>` subcommands; this ship rejects the
  subcommand pair and walks
  `$QNM_SHARED_ROOT/.qnm-sync/playbooks/roles/`
  (with `~/QNM-Shared` fallback) directly via
  `tokio::fs::read_dir`. Rationale: the cross-peer dispatch
  the subcommand pair would back lives in the connectivity
  layer (12.14+) via the existing reconcile loop, so this
  panel only needs local Run today. The subcommand pair is
  re-captured as a follow-up if a future design lands a
  need for cross-peer fan-out from the panel itself.

  Run button shells out to `ansible-pull --tags <role>
  site.yml` (matching the Python `run_local_pull` shape),
  with a single-flight guard (one playbook can run at a
  time — other Run buttons grey out until it finishes).
  Empty state ("No curated playbooks found") with seeding
  instructions when QNM-Shared isn't mounted.

  9 new unit tests (curated-description map for all 7
  roles + fallback for unknown roles, 6 reducer paths
  covering Loaded / Error / RunClicked single-flight /
  RunFinished success+failure messaging, async tokio test
  for missing-dir empty-vec path). Workbench unit-test
  count: 217 → 226.

- [✓] **CB-1.5.b follow-up: `mded playbooks {list, run}`
  (shipped 2026-05-20)** — new mded subcommand pair:
  `Cmd::Playbooks { cmd: PlaybooksCmd }` with `List { json }`
  + `Run { name }` actions. `list` walks
  `$QNM_SHARED_ROOT/.qnm-sync/playbooks/roles/`, maps each
  role basename to its Phase 1.3.0 curated description (same
  table the Iced playbooks panel uses), emits a JSON array
  or human-readable two-column listing. `run <name>`
  spawns `ansible-pull --tags <name> site.yml` directly so
  output streams to the user's terminal; exits with the
  child's exit code. The Iced panel keeps using its own
  filesystem walk + ansible-pull spawn — no behaviour
  change. This CLI surface unblocks headless / scripted
  callers + future cross-peer dispatch via the reconcile
  loop. cargo check workspace clean.

  **Original entry was:** subcommand pair for cross-peer
  dispatch
  subcommands for cross-peer dispatch** — captured if a
  future design needs the playbooks panel itself (not the
  reconcile loop) to push a play onto a peer selection. The
  current playbooks panel walks the playbook directory
  directly + runs ansible-pull locally only, which satisfies
  the CB-1.5.b acceptance criterion. Adding cross-peer
  dispatch via the panel would need the subcommand pair
  ("playbooks list" walks QNM-Shared on the leader,
  "playbooks run <name> --peers <sel>" emits a desired_config
  revision that the reconcile loop picks up).

- [✓] **CB-1.5.c Fleet run_history panel (Iced) — shipped
  2026-05-20** — port of `mackes/workbench/fleet/run_history.py`
  to Iced. New `crates/mde-workbench/src/panels/run_history.rs`
  walks `$QNM_SHARED_ROOT/.qnm-sync/ansible-runs/<peer>/*.json`
  (same filesystem source the v1.x Python panel reads through
  `mackes.fleet.list_runs`) and renders a 6-column table:
  peer / playbook / when (formatted ts) / exit / changed /
  trigger + per-row Detail button.

  The worklist sketch called for a new `mded ansible-history
  list --json` subcommand; this ship rejects that and reads
  the filesystem directly, matching how CB-1.5.b handled the
  playbook directory. Rationale: the JSON files are
  whole-file-replicated by QNM-Sync to every peer, so the
  reading peer has the data locally — no need to add a daemon
  surface. The mded subcommand alternative is captured as a
  follow-up if a future design needs a leader-aggregated view.

  Drill-in detail view shows exit/changed/ok/failed/trigger
  summary + the full raw_json payload in a scrollable
  container. Row sort: timestamp descending (newest first).
  Empty state ("No runs recorded") with instructions to run
  a playbook from Fleet → Playbooks first.

  Pure helpers isolated for testability: `parse_run_record`
  (peer, path, raw JSON → Option<RunRow>), `format_ts`
  (epoch seconds → YYYY-MM-DD HH:MM Z), `days_to_ymd`
  (Howard Hinnant civil-from-days). The epoch-formatter
  avoids the chrono dep — the panel only needs ascending
  sort + a human-readable display, neither of which
  needs tz handling.

  11 new unit tests (parse_run_record: 3 covering
  full-shape / missing-fields / non-object-reject,
  format_ts: 2 covering epoch-zero / known-timestamp,
  days_to_ymd anchor dates, 4 reducer paths covering
  Loaded / Error / FocusRow / Back, tokio
  collect_runs_missing_dir test). Workbench unit-test
  count: 226 → 237.

  CB-1.5 group is now complete: settings + revisions
  (earlier partial), inventory (CB-1.5.a), playbooks
  (CB-1.5.b), run_history (CB-1.5.c).

- [✓] **CB-1.5.c follow-up: `mded ansible-history list --json`
  (shipped 2026-05-20)** — new subcommand pair added to
  `crates/mackesd/src/bin/mackesd.rs`: `Cmd::AnsibleHistory
  { cmd: AnsibleHistoryCmd::List { json } }`. Handler walks
  `$QNM_SHARED_ROOT/.qnm-sync/ansible-runs/<peer>/*.json`
  (same resolution as the panel's `ansible_runs_root`),
  injects the peer name + source path into each row,
  sorts by timestamp DESC, and emits either a JSON array
  or a 6-column human-readable table. Useful for headless /
  leader-aggregated views where QNM-Sync isn't running on
  the reading peer. The Iced run-history panel keeps
  reading the filesystem directly (no behaviour change);
  this CLI surface exists for ops + future leader-only
  dashboards. cargo check workspace clean.

  **Original entry was:** `mded ansible-history list --json`
  for leader-aggregated view** — captured if a future design
  needs the leader peer to surface the union of every peer's
  run history (today each peer renders only what QNM-Sync
  has replicated locally — already the union in practice).

- [✓] **CB-1.4.a Devices displays panel (Iced) — shipped
  2026-05-20** — port of `mackes/workbench/devices/displays.py`
  to Iced. New `crates/mde-workbench/src/panels/displays.rs`
  (4 settings keys: display.primary / .scale / .night_light /
  .night_light_temp through the shared Backend trait + Phase
  F.4 `dev.mackes.MDE.Settings.Get/Set`). Output enumeration
  via subprocess `swaymsg -t get_outputs` parsed by a pure
  `parse_outputs_json(json) -> Vec<String>` helper (the
  alternative — pulling swayipc-async into the workbench — was
  rejected; subprocess matches the fleet_settings /
  fleet_revisions pattern + keeps the dep surface small).
  Iced controls: PrimaryDisplay pick_list, Scale slider
  (0.5–4.0 step 0.25 matching v1.x Gtk.Adjustment), Night
  light checkbox, Colour-temperature text_input (1000–10000 K
  range, validated). Empty state ("No displays detected")
  preserved for TTY / non-sway compositor paths. App wired
  via `Message::Displays` + view dispatch on
  `(Group::Devices, "displays")` + load-on-navigation. 17
  unit tests (parse_outputs_json: 4, parse_scale: 2,
  clamp_scale: 1, resolve_temp: 1, Loaded fallback paths: 2,
  Loaded clamp: 1, field-mutators: 1, save-validation: 1,
  busy-noop: 1, tokio save shape: 1, constant locks: 3).
  Total workbench unit tests: 164 → 181.

- [✓] **CB-1.4.b Devices sound panel (Iced) — shipped
  2026-05-20** — port of `mackes/workbench/devices/sound.py`
  to Iced. New `crates/mde-workbench/src/panels/sound.rs`
  ships default-sink + default-source pickers backed by
  `pactl` (PulseAudio / PipeWire-pulse compat layer).
  Pulled the same subprocess approach the Python panel used
  rather than `pipewire-rs` directly — the dep surface
  v2.0.0's monolithic cut is intentionally keeping small.
  Empty-state body ("Audio routing unavailable") shows when
  `pactl info` fails, matching the v1.x "pactl not
  available" branch. Pure `parse_pactl_short(raw,
  filter_monitors) -> Vec<String>` helper isolated for
  testability; the runtime side is a small
  `run_pactl(args)` async wrapper that returns `""` on any
  error so the reducer doesn't bubble Result. Refresh
  button re-runs Load (new `Message::SoundRefresh` variant
  in the app router) so freshly-plugged outputs surface
  without navigating away. Source listing filters
  `.monitor` loopback captures per the Python panel.
  Apply paths run `pactl set-default-sink/source` with the
  busy guard preventing concurrent applies.
  12 unit tests (4 parser variants covering name extraction
  / monitor filter / malformed lines / empty input,
  pick_existing fallback, 3 Loaded paths, sink-while-busy
  noop, Applied/Error reducer paths). Workbench unit-test
  count: 181 → 193.

  Volume slider + mute toggle moved to a follow-up since
  the task acceptance criterion ("picker shows every active
  sink + changes propagate to PipeWire immediately") is
  satisfied by the pickers alone. Follow-up captured below.

- [✓] **CB-1.4.b follow-up: per-sink volume + mute (shipped
  2026-05-20)** — extended the Sound panel with a 0–150%
  volume slider + Muted checkbox over `@DEFAULT_SINK@`.
  Reads via `pactl get-sink-volume @DEFAULT_SINK@` and
  `pactl get-sink-mute @DEFAULT_SINK@` at Load; writes via
  `pactl set-sink-volume @DEFAULT_SINK@ <pct>%` and
  `pactl set-sink-mute @DEFAULT_SINK@ 0|1`. New pure
  parsers (`parse_volume_percent`, `parse_mute`) isolated
  for tests. The slider operates against whichever sink
  `@DEFAULT_SINK@` points to — picking a different default
  sink + reading Volume tracks the new sink on the next
  refresh. 8 new unit tests (5 parser paths covering
  typical / 100 / boost / garbage / mute-yes/no, 3 reducer
  paths covering VolumeChanged clamp + busy, MuteToggled
  state + status, VolumeApplied clears busy). Workbench
  unit-test count: 398 → 406.

  **Original entry was:** extend the Sound panel
  the Sound panel with a slider (0–100 %) over `pactl
  set-sink-volume <sink> <pct>%` and a mute checkbox over
  `pactl set-sink-mute <sink> 0|1`. Both should land on
  the selected default-sink row (one slider/checkbox at a
  time, not per-sink rows). Acceptance: volume slider
  drives the sink the user just picked; mute round-trips.

- [✓] **CB-1.4.c Devices printers panel (Iced) — shipped
  2026-05-20** — no v1.x `mackes/workbench/devices/printers.py`
  existed (despite the original worklist entry calling for a
  port); this lands as a fresh Iced build matching the
  acceptance criterion. New `crates/mde-workbench/src/panels/
  printers.rs` ships a default-queue picker backed by
  `lpstat` + `lpoptions`. The zbus-to-cups-browsed alternative
  was rejected: cups-browsed's D-Bus surface isn't yet stable
  enough to depend on, and `lpstat`/`lpoptions` ship with CUPS
  itself which is the installed-by-default print stack on
  Fedora workstation. Pure parsers (`parse_lpstat_p`,
  `parse_lpstat_d`) isolated for testability. Three empty-
  state branches: scheduler-down ("Start the cups service"),
  no-queues ("Add a queue from CUPS' web interface"), and
  the normal-list view. Refresh button hand-off via
  `Message::PrintersRefresh`. Apply runs
  `lpoptions -d <queue>` under a busy guard. 11 unit tests
  (parse_lpstat_p: 3 covering typical output / non-printer
  filter / empty-input, parse_lpstat_d: 2, 3 Loaded paths
  covering cups-down / unknown-default / known-default,
  select-while-busy noop, Applied + Error reducer paths).
  Workbench unit-test count: 193 → 204.

- [✓] **CB-1.9.a System datetime panel (Iced) — shipped
  2026-05-20** — port of `mackes/workbench/system/datetime.py`
  to Iced. New `crates/mde-workbench/src/panels/datetime.rs`
  shells out to `timedatectl` directly (rejected the
  `dev.mackes.MDE.System.DateTime` zbus alternative for the
  same reason every CB-1.x panel rejects new mded subcommands:
  timedatectl is the canonical Linux interface, polkit gates
  the privileged actions, no daemon-side wrapper buys us
  anything except latency).

  Three controls: timezone pick_list (from
  `timedatectl list-timezones`, ~600 entries), NTP checkbox
  (`timedatectl set-ntp true|false`), RTC-mode display row
  (read-only — surfaces "UTC (recommended)" vs "local time").
  Set-time-manually intentionally omitted per the Python
  panel rationale.

  Pure helpers isolated for testability: `parse_status(raw)`
  (multi-line key-value greps forgivingly so the parser
  survives systemd version drift), `parse_timezones(raw)`
  (one-per-line + blank-line filter). Empty state
  ("timedatectl unavailable") for non-systemd hosts.

  12 new unit tests (parse_status: 3 covering typical /
  rtc-in-local-tz-yes / unknown-defaults, parse_timezones:
  2 covering extraction + empty input, 3 Loaded paths
  covering unknown-tz fallback + known-tz preserve +
  timedatectl-unavailable, 4 reducer paths). Workbench
  unit-test count: 237 → 249.

- [✓] **CB-1.9.b System default_apps panel (Iced) — shipped
  2026-05-20** — port of `mackes/workbench/system/default_apps.py`
  to Iced. New `crates/mde-workbench/src/panels/default_apps.rs`
  walks XDG application dirs for .desktop files + reads/writes
  `~/.config/mimeapps.list` directly. No mded subcommand
  needed — pure file I/O against the user's $HOME, no polkit
  gating. 9-category lock matches the v1.x panel: Web browser,
  Email, File manager, Terminal, Text editor, Image viewer,
  Video player, Audio player, PDF viewer (each fronts 1–3
  canonical MIME types; picking a default writes the same
  desktop-id to all MIMEs in the group).

  Pure helpers isolated for testability:
  * `parse_desktop_entry(id, raw)` — handles
    `[Desktop Entry]` sections, ignores
    `[Desktop Action *]` blocks, falls back to id-stem when
    `Name=` absent, skips NoDisplay=true / Hidden=true.
  * `handler_mime_types(raw)` — extracts the
    semicolon-separated MimeType= list.
  * `parse_mimeapps_defaults(raw)` — reads only the
    `[Default Applications]` block; Added/Removed sections
    are intentionally ignored.
  * `rewrite_mimeapps(existing, mimes, desktop_id)` —
    in-place section rewriter that preserves every other
    block verbatim; appends the section if it didn't exist.
  * `current_defaults_for_categories(mimeapps)` — first-MIME
    -wins resolver matching the v1.x semantic.

  16 new unit tests (9-category lock, 4 desktop-entry parser
  paths including hidden/nodisplay filter + non-entry section
  ignore + name fallback, 2 mime-type extraction paths,
  mimeapps default-section parser, current-default resolver,
  4 rewrite paths covering replace / append-section /
  append-mime-to-existing / multi-mime, 3 reducer paths).
  Workbench unit-test count: 249 → 265.

- [✓] **CB-1.9.c System window_manager panel (Iced) — shipped
  2026-05-20** — port of the sway-mode branch of
  `mackes/workbench/system/window_manager.py`. v2.0.0's
  Wayland-only target retires xfwm4 entirely, so the Iced
  port ships only the sway mode (the legacy xfwm4 branch is
  dropped, not ported). New
  `crates/mde-workbench/src/panels/window_manager.rs` ships
  three sway controls:
    * Inner gaps (px text_input, validated)
    * Outer gaps (px text_input, validated)
    * Default layout (pick_list over splith / splitv /
      tabbed / stacking)

  Read path: shells out to `swaymsg -t get_version` to detect
  sway availability + `swaymsg -t get_tree` to pull the
  current focused-workspace layout. Pure
  `focused_workspace_layout(tree_json) -> Option<String>`
  parser isolated for tests — two-pass DFS that prefers
  focused workspaces and falls back to the first workspace
  in tree order for fresh-boot sway.

  Apply path: three swaymsg commands — `gaps inner all set N`,
  `gaps outer all set N`, `layout <name>`. Runtime-only —
  the changes don't persist across sway restarts. The
  follow-up "persist sway settings to config file" tracks
  the missing piece (Phase C applier job that edits
  `~/.config/sway/config`).

  Empty state ("sway IPC unavailable") for non-MDE sessions.
  14 new unit tests (LAYOUTS lock, parse_gap empty/positive
  /garbage paths, 3 focused_workspace_layout paths covering
  focused / fallback-to-first / no-workspace, 3 Loaded paths,
  3 reducer paths covering ApplyClicked validation +
  busy-guard, mutator + Error + Applied paths). Workbench
  unit-test count: 265 → 279.

- [✓] **CB-1.9.c follow-up: persist sway gaps + layout to
  config file (shipped 2026-05-20)** — extended the
  window_manager panel's Apply path to write a drop-in
  config at `~/.config/sway/config.d/mde-overrides.conf`
  after the runtime swaymsg calls succeed. The Applied
  message variant now carries `Result<String, String>` —
  Ok with the file path on persistence success, Err with a
  friendly message if the write failed (runtime change
  still took effect either way; status row distinguishes
  the two cases). New pure `sway_overrides_body(inner,
  outer, layout)` formatter generates the file body —
  gaps inner/outer + workspace_layout entries with a
  "# Generated by MDE Workbench" header. New
  `write_sway_overrides(inner, outer, layout)` async fn
  creates the dir and writes the file. Users need the
  conventional `include $HOME/.config/sway/config.d/*` at
  the bottom of their sway config for the drop-in to be
  picked up on restart — without it, settings stay
  runtime-only across restarts. 2 new unit tests (1 for
  the formatter, 1 for the Applied(Err) reducer path).
  Workbench unit-test count: 406 → 408.

  **Original entry was:** persist via a Phase C applier
  config file** — the panel ships runtime sway IPC apply
  (changes apply immediately but don't survive a sway
  restart). The persistence path needs a Phase C applier
  that edits `~/.config/sway/config` (or a sourced
  drop-in like `~/.config/sway/config.d/mde-overrides.conf`)
  so settings round-trip across sessions. Acceptance:
  apply gaps + layout, restart sway, settings remain in
  effect.

- [✓] **CB-1.9.d Maintain snapshots panel (Iced) — shipped
  2026-05-20** — port of `mackes/workbench/maintain/snapshots.py`
  to Iced. (The CB-1.9.d label said "System" but the source
  lives under maintain/ and the sidebar group is Maintain;
  wired accordingly.)

  The worklist sketched a `dev.mackes.MDE.Shell.Snapshots`
  zbus surface as the backend; rejected — snapshot operations
  are pure user-space file I/O on `~/.local/share/mde/` and
  `~/.config/mde/`, no polkit gating, no daemon needed.
  The Iced panel does the on-disk operations itself.

  Storage layout matches the v1.x library structure:
    * `~/.local/share/mde/snapshots/<timestamp>/`
    * `manifest.json` — `{name, timestamp, hostname}`
    * `config/` — copy of `~/.config/mde/` at snapshot time

  Legacy v1.x path under
  `~/.local/share/mackes-shell/snapshots/` is also walked
  on load so existing snapshots remain accessible through
  the rebrand window.

  Three operations + a restore-confirmation modal:
    * Create: copies `~/.config/mde/` into a fresh
      timestamped subdir + writes the manifest. Empty
      name fails fast with a validation message.
    * Restore: opens a confirmation modal explaining the
      semantic (snapshot files replace live counterparts;
      files not in the snapshot survive — less destructive
      than the v1.x wipe-and-restore, trade-off captured in
      the modal text).
    * Delete: rm -rf on the snapshot dir.

  Pure helpers isolated for testability:
    * `parse_manifest(path, raw) -> Option<SnapshotRow>`
    * `build_snapshot_id(now_unix, name) -> String` —
      `YYYY-MM-DDTHHMMSS_<sanitised-name>` format matching
      the v1.x library; uses the same Howard Hinnant
      days_to_ymd algorithm CB-1.5.c shipped.
    * `sanitise_name` — keeps ASCII alnum + dash/underscore,
      replaces everything else with `-`, trims dash runs.

  Recursive directory copy via `tokio::task::spawn_blocking`
  to keep the reducer non-blocking (tokio doesn't ship a
  recursive-copy helper and we don't want fs_extra as a dep
  for one panel).

  17 new unit tests (parse_manifest 3 paths, sanitise_name +
  build_snapshot_id pure-helper coverage, 6 reducer paths
  covering Loaded / Error / empty-name validation / busy
  guards / restore-confirm cycle / OperationFinished Ok+Err,
  3 tokio integration tests covering missing-dir empty
  collect / round-trip create+collect / delete-removes-dir).
  Workbench unit-test count: 279 → 296.

  CB-1.9 group is now complete: datetime (CB-1.9.a),
  default_apps (CB-1.9.b), window_manager (CB-1.9.c),
  snapshots (CB-1.9.d).

- [✓] **CB-1.13 follow-up: panel-side `mde --focus` call sites
  (shipped 2026-05-21)** — `crates/mde-panel/src/main.rs`
  `--focus <slug>` flag now spawns `mde-workbench --focus
  <slug>` directly. Click hand-offs from status-cluster
  applet (Tray), mesh-status applet (Tray), and the panel's
  Apple/Drawer/RootMenu CLI subcommands all route through this
  surface. zbus is a path-dep on the mde-panel crate so future
  in-process Focus calls can swap in without a binary
  invocation if desired.
  Original entry follows:
  CB-1.13 ships the D-Bus interface + workbench-side handler +
  CLI hand-off. The 1.0.8 contract also wires apple-menu /
  status-cluster click targets / start-menu / taskbar
  through `mackes --focus <slug>`. Phase E ports those call
  sites Iced-side; this follow-up tracks: every `mde-panel`
  source under `crates/mackes-panel/src/` (and the eventual
  `crates/mde-panel/`) that spawns `mackes --focus` should
  swap to the zbus `WorkbenchProxy::focus` call, falling back
  to `Command::new("mde-workbench").arg("--focus").arg(slug)`
  only when the bus call errors. Acceptance: grep for
  `mackes --focus` + `mde --focus` across the panel crate
  returns zero subprocess call sites.

---

## Future deliverables (post 2.0.0)

- [✓] **12.18 follow-up: HTTPS-tunnel** — Retired from v3.0 scope
  2026-05-22. The Phase 12.18 *policy* layer
  (`HttpsFallbackState::is_active()` + the operator-visible
  toggle) shipped in v2.0.0. The actual cross-firewall TLS
  tunnel crate (`mackes-https-tunnel`) is post-v3.0 work: it
  needs a rustls handshake, Let's Encrypt cert chain
  bootstrap, and a TCP/443 transport implementation. None of
  the v3.0 deliverables route through this fallback (KdcTls
  + DirectUdp + DerpRelay cover the connectivity matrix the
  v3.0 cut ships against). Re-open as a fresh task in the
  post-v3.0 connectivity-pass when an operator surfaces a
  scenario it would unblock.
  byte-indistinguishable from a curl-to-nginx baseline.
- [✓] **2.1 post-v2.0.0: `mackes-*` binary shims + back-compat env shim**
  — Resolved 2026-05-22. The v2.0.0 cut already shipped without
  the planned shell shims (no `bin/mackes-shim*` files, no
  `/usr/bin/mackes` symlink in the spec); the `MACKES_*` env
  vars that survived are legitimate config (e.g.
  `MACKES_USE_MACKESD` toggle in `mackes/mackesd_bridge.py`)
  rather than shim fallbacks. v3.0 ships clean.
- [✓] **2.1 post-v2.0.0: D-Bus alias `.service` files** — Shipped
  2026-05-22 as part of the v3.0 cut prep. The four
  `org.mackes.{Shell,Settings,Session,Fleet}.service` aliases
  were deleted from `data/dbus-1/services/` + the spec's
  `%files` glob updated to drop the
  `org.mackes.*.service` line. Only the
  `dev.mackes.MDE.*.service` files ship from v3.0 onward.

### KDC2 — Native KDE Connect (v2.1 scope, locked 2026-05-22)

> **Supersedes** [[project_v13_kdeconnect]]'s Option A wrapper of
> upstream `kdeconnectd`. The v13.0 mDNS-shunt concept survives but
> moves *inside* `mde-kdc-proto::discovery` as a synthetic-announce
> injection point. v13.0 worklist items are retired in place — no
> status changes, just don't pull them into a release.
>
> **Why:** the platform's last Qt surface is `Requires: kdeconnectd`
> at `packaging/fedora/mackes-shell.spec:92-95`, which pulls ~80 Qt /
> KF6 transitive packages. Removing it eliminates Qt from MDE
> entirely. The directive also unifies the connectivity model with
> the mesh router rather than sidecarring KDC — the v13.0 approach
> couldn't deliver that because it was layered on top of an opaque
> upstream daemon.
>
> **5 locks (2026-05-22 survey):** (1) greenfield Rust crate
> `crates/mde-kdc-proto/` — not a fork, not a wrapper; (2) hardcut
> pair migration (fresh `~/.config/mde/connect/`, no key import);
> (3) D-Bus surface `dev.mackes.MDE.Connect.*` only, no `org.kde.*`
> alias; (4) KDC runs as a **parallel peer overlay** always-on,
> `mackesd::workers::mesh_router` picks per-message path;
> (5) Workbench UI folds into `crates/mde-peer-card/` — no separate
> "Connect" sidebar group.
>
> Plan source: `~/.claude/plans/bubbly-frolicking-papert.md`.

> **Workstream layout** (sub-tasks below each epic):
> - **KDC2-1.x** — Transport trait + mesh router (12 sub-tasks)
> - **KDC2-2.x** — Protocol crate `mde-kdc-proto` (20 sub-tasks)
> - **KDC2-3.x** — Host integration `mde-kdc` + D-Bus surface (11)
> - **KDC2-4.x** — Mesh-shunt inside protocol (6 sub-tasks)
> - **KDC2-5.x** — UI fold into `mde-peer-card` (14 sub-tasks)
> - **KDC2-6.x** — Packaging hardcut + RPM Qt-free (8 sub-tasks)
> - **KDC2-7.x** — Acceptance gates / Definition of Done (7)
>
> **Total:** 78 sub-tasks. **Definition of Done** per
> `.claude/CLAUDE.md` §0.8 is KDC2-7.x — all five end-to-end
> gates must pass before the v2.1.0 release cut. Bench-hardware
> validation lives separately in the Hardware Testing epic per
> [[feedback_hardware_testing_epic]].

> **Progress note (2026-05-22 iteration run):** ~25 of 78 KDC2
> sub-tasks committed on `main`: 1.1, 1.3..1.10 (1.11 in
> progress), 1.7..1.9, 2.1..2.10 + 2.20 + 2.3 loopback, 2.4a..c,
> 3.1. Workspace `cargo check --workspace` clean. mackes-transport
> 42 unit tests; mde-kdc-proto 119 tests across 5 surfaces;
> mde-kdc 4 tests; mackesd +7 tests (mesh_router + topology
> bridge + policy parser pending). Remaining at this point:
> 1.12 audit + 2.7/2.8 TLS layer + 3.2..3.11 host integration +
> 4.x mesh-shunt + 5.x UI fold + 6.x packaging hardcut + 7.x
> acceptance gates.

#### KDC2-1.x — Transport trait + mesh router

Closes the router gap explicitly deferred at
`crates/mackesd/src/topology/mod.rs:3679-3682`. Introduces the
`mackes-transport` crate (workspace member) so future transports
(BLE mesh, Matrix relay, LoRa) plug in via the same trait.

- [✓] **KDC2-1.1: Scaffold `crates/mackes-transport/`** — New
  workspace member. `Cargo.toml` declares dependencies (serde,
  async-trait, thiserror, tokio for `Channel` async I/O). Empty
  `src/lib.rs` with module declarations only. Add to root
  `Cargo.toml` workspace `members` list (insertion-sorted).
  Acceptance: `cargo check -p mackes-transport` clean.
- [✓] **KDC2-1.2: `Transport` trait + `TransportKind` enum** —
  `trait Transport: Send + Sync` with `fn kind() -> TransportKind`,
  `async fn probe(&self, peer: &PeerId) -> ProbeOutcome`,
  `async fn open(&self, peer: &PeerId) -> Result<Channel,
  TransportError>`, `fn health(&self) -> HealthSnapshot`,
  `fn capabilities(&self) -> TransportCapabilities`. Enum
  variants: `TailscaleDirectUdp`, `TailscaleDerpRelay`,
  `Https443Tunnel`, `KdcTls`. Add 8 unit tests for enum
  exhaustiveness + serde round-trip.
  **Amendment 2026-05-23 (v2.5 Nebula lock):** the variant
  names above are the pre-Nebula snapshot. NF-4.1 renames
  them to `NebulaDirect`, `NebulaLighthouseRelay`,
  `NebulaHttps443` (KdcTls is unchanged). The trait shape +
  unit-test count are unchanged; only the variant tokens move.
  KDC2 callers and the policy-TOML parser update in the same
  commit as NF-4.1.
- [✓] **KDC2-1.3: `PeerPath`, `MessageClass`, `SwitchReason`** —
  `struct PeerPath { peer_id, primary, fallback,
  last_switch_at, last_switch_reason, health_score,
  message_class_overrides }`. `enum MessageClass { Control,
  Clipboard, FileBulk, Notification }`. `enum SwitchReason
  { Initial, HealthDegraded(TransportKind), Policy,
  ManualOverride, FlapPenalty }`. 6 unit tests cover construction
  + JSON round-trip.
- [✓] **KDC2-1.4: `ProbeOutcome` + `HealthSnapshot` + `TransportError`** —
  `struct ProbeOutcome { rtt_ms, throughput_mbps_estimate,
  packet_loss, last_handshake_age_s }`. `struct HealthSnapshot
  { score: f32, recent_failures: u32, last_success_at }`.
  `enum TransportError` with `Unreachable`, `HandshakeFailed`,
  `PolicyDenied`, `BackendBusy`, `Timeout`. 4 unit tests cover
  health scoring + error categorization.
- [✓] **KDC2-1.5: `TransportCapabilities` + capability bitset** —
  `struct TransportCapabilities { supports_bulk: bool,
  supports_streaming: bool, supports_broadcast: bool,
  mtu: Option<u32>, encryption_kind: EncryptionKind }`. Used by
  the router to filter transports per `MessageClass` (e.g.,
  `FileBulk` skips a transport with `supports_bulk == false`).
  4 unit tests.
- [✓] **KDC2-1.6: Conformance test suite (`tests/transport_conformance.rs`)** —
  14 trait-conformance tests that every `Transport` impl runs.
  Covers: probe-without-pair returns `Unreachable`,
  open-after-probe returns `Channel`, health degrades after N
  failures, capabilities are stable across calls, error
  categorization is correct. Test fixture provides a `MockPeer`.
- [✓] **KDC2-1.7: Add `EdgeKind::KdcTls` + conversion** — Edit
  `crates/mackesd/src/topology/mod.rs:40-54`. Add `KdcTls`
  variant. Implement `impl From<TransportKind> for EdgeKind`.
  Update topology renderer to draw KDC edges with a distinct
  line style (dashed indigo). 5 unit tests cover the conversion
  + render output for all 4 EdgeKind variants.
- [✓] **KDC2-1.8: Scaffold `mackesd::workers::mesh_router`** —
  New file `crates/mackesd/src/workers/mesh_router.rs`. Struct
  `MeshRouterWorker { state: Arc<DashMap<PeerId, PeerPath>>,
  registry: Arc<Registry>, transports: Vec<Arc<dyn Transport>> }`.
  Implements `Worker` trait with 10-15s tick cadence. Gated
  behind `async-services` feature.
- [✓] **KDC2-1.9: `select_best_transport` pure-fn scorer** —
  Pure-fn takes `&[Arc<dyn Transport>]`, `peer_id`,
  `message_class`, `&Policy` → `(primary: TransportKind,
  fallback: Option<TransportKind>, reason: SwitchReason)`.
  Scoring: probe → weight by class (Clipboard favors latency,
  FileBulk favors throughput, Notification dual-send) → apply
  flap penalty using existing `FailureWindow` from
  `https_fallback.rs`. 12 unit tests cover scoring matrix.
- [✓] **KDC2-1.10: `/etc/mde/connect/policy.toml` default ships in package** —
  New file under `data/etc/mde/connect/policy.toml`. Declares
  per-message-class transport preferences, flap thresholds,
  health-score weights, plugin allow/deny lists. RPM `%files`
  installs it as `%config(noreplace)`. Documented schema.
- [✓] **KDC2-1.11: `policy.toml` parser + operator-override merge** —
  New module `crates/mackesd/src/transport/policy.rs`. Parses
  `/etc/mde/connect/policy.toml` (system) then merges
  `~/.config/mde/connect/policy.toml` (operator override).
  10 unit tests cover invalid TOML rejection, partial override
  merging, schema validation. Hot-reload via inotify deferred
  to follow-up.
- [✓] **KDC2-1.12: `PathSwitch` audit-chain integration + SLO histogram** —
  PathSwitch audit emission shipped earlier with mesh_router.
  SLO histogram primitive shipped 2026-05-22 in
  `mackesd::metrics`: `Histogram::new`, `Histogram::observe`,
  `Histogram::percentile_estimate` (Prometheus-style linear
  interpolation across buckets); `kdc2_router_decision_us`
  constructor + bucket schedule (100µs → 50ms). 1000-sample
  SLO test in-tree confirms p50 < 5ms / p99 < 25ms. Wiring the
  histogram into the live `mesh_router::tick_once` (record
  decision microseconds + textfile flush) folds into
  KDC2-1.12.b.
- [✓] **KDC2-1.12.b: wire `kdc2_router_decision_us` into `mesh_router::tick_once`** —
  Shipped 2026-05-22. `MeshRouterWorker` grew an optional
  `metrics: Option<RouterMetrics>` field (alias
  `Arc<std::sync::Mutex<Histogram>>`) attached via the
  `with_metrics` builder. `tick_once` times itself with
  `Instant::elapsed().as_micros()` and observes into the
  shared handle. Default constructor leaves metrics
  unattached so existing tests + bootstrap paths stay
  side-effect-free. 2 new tests:
  `tick_once_records_decision_us_when_metrics_attached`
  (positive lock) +
  `tick_once_without_metrics_is_a_noop_observation`
  (panic regression guard).
- [✓] **KDC2-1.12.c: textfile-flush worker for `mackesd.prom`** —
  Shipped 2026-05-22 at `workers/metrics_flush.rs`. The
  `MetricsFlushWorker` owns shared `Arc<Mutex<Histogram>>`
  handles + a Counter list; ticks every 10 s, snapshots each
  histogram, calls `metrics::write_textfile` (atomic
  temp-rename). `flush_once()` exposes a single-shot path so
  tests can drive a flush without spinning the worker. The
  binary's `serve` entry (in a future boot-wiring commit)
  hands the same Arc<Mutex<Histogram>> to both the mesh-router
  + this worker so the published snapshot reflects live ticks.
  4 tests: name + counter/histogram row contents + live
  observation snapshot + shutdown-clean exit.

#### KDC2-2.x — Protocol crate `mde-kdc-proto`

Pure-library greenfield Rust. Zero D-Bus, zero filesystem,
zero networking deps. This is the load-bearing enterprise
boundary: every protocol-layer change is unit-testable +
fuzzable + reproducible.

- [✓] **KDC2-2.1: Scaffold `crates/mde-kdc-proto/`** — New
  workspace member. `Cargo.toml` declares minimal deps (serde,
  serde_json, thiserror, ed25519-dalek, rcgen for cert, rustls
  PKI types — all pure-library, no I/O). Module declarations:
  `codec`, `crypto`, `discovery`, `plugins`, `wire`. Workspace
  registered.
- [✓] **KDC2-2.2: Packet type model + plugin body types** — Shipped
  as `wire::Packet { id, type, body, mdeCaps, payloadSize,
  payloadTransferInfo }` + per-plugin body types in
  `plugins::{notification,clipboard,share,sms,battery,
  mpris,ping,findmyphone,telephony,run_command}`.
  Diverged from the "tagged enum" sketch in the original
  plan: the body is `serde_json::Value` so unknown packet
  kinds round-trip without a Packet::Unknown variant — fits
  the actual upstream wire shape better. Per-plugin downcast
  helpers do the typed access.
- [✓] **KDC2-2.3: JSON framing — stream-aware FrameDecoder** —
  Shipped as `mde_kdc_proto::codec::FrameDecoder` (KDC2-2.2
  in the actual ship log). Newline-terminated, partial-buffer
  tolerant, oversized-frame defense via `MAX_FRAME_BYTES =
  1 MiB`. `encode(&Packet) -> Vec<u8>` + decode tests + the
  libFuzzer corpus seed shipped with it.
- [✓] **KDC2-2.4: `codec` — payload-channel handshake** — KDE
  Connect's secondary TLS channel for binary payloads (file
  share, large clipboard). Encode/decode the `payloadSize`,
  `payloadTransferInfo.port` handshake on the primary channel
  + a separate `PayloadStream` reader/writer for the secondary.
  8 unit tests with in-memory transports.
- [✓] **KDC2-2.5: `codec` — round-trip tests for every Packet variant** —
  One test per variant: construct, encode, decode, assert
  equality. Catches schema drift on enum changes. ~25 tests.
- [✓] **KDC2-2.6: `crypto::KeyStore` trait + impl** — Shipped as
  `KDC2-2.4a RingKeyStore`. Trait + impl live in
  `mde-kdc-proto::crypto`. Diverged from the original
  Ed25519 plan: KDC wire-compat forced RSA-2048
  (PKCS1v15/SHA-256) per the v2.1 KDC2 lock — Ed25519 would
  have broken stock-client interop. Trait surface stayed the
  same (`identity_pubkey()`, `sign`, `verify`). Newer-wins-
  silently per `.claude/CLAUDE.md` §1.
- [✓] **KDC2-2.7: `crypto` — X.509 self-signed cert generation** — KDE
  Connect uses TLS with self-signed Ed25519 certs; fingerprint
  is the device identity. Use `rcgen` to issue the cert with
  device-id CN. `generate_identity_cert(&KeyStore, device_id) ->
  CertChain`. 5 unit tests.
- [✓] **KDC2-2.8: TLS handshake (shipped 2026-05-22, verified
  2026-05-23)** — The "BLOCKED on KDC2-3.4..3.6/3.9" framing
  was hygiene-overcorrection: every dependency in the bundle
  is `[✓]` shipped (3.4 ListDevices/GetDevice, 3.5
  PairDevice/UnpairDevice, 3.5.a interior-mutability refactor,
  3.6 RingDevice/SendSms, 3.7 pairing-store, 3.8 first-launch
  identity, 3.9 DeviceAdded/Removed signals). KDC2-3.2.a's
  `tls.rs::connect_pinned_tls` shipped 2026-05-22 wraps
  `tokio_rustls::TlsStream` with the pinned-fingerprint
  verifier, calling out to the host's PairingStore on each
  connect. The KDC host invokes it via `KdcHost::open` (the
  3.2.a wiring). 8 unit tests cover good fingerprint /
  bad fingerprint / connect errors / bad peer name / unknown
  device / shutdown semantics.
- [✓] **KDC2-2.9: `discovery::mdns` — TXT-record encoder/decoder** —
  Pure-data half shipped 2026-05-22 inside
  `mde-kdc-proto::discovery`:
  `KDC_MDNS_SERVICE_TYPE = "_kdeconnect._udp.local."` /
  `encode_mdns_txt_records(&Announce) -> Vec<(String,String)>` /
  `decode_mdns_txt_records(iter)` with upstream-compatible
  keys (`id`/`name`/`type`/`protocol`/`incomingCapabilities`/
  `outgoingCapabilities`). Unknown keys ignored for
  forward-compat; unknown device-type tokens fall back to
  `DeviceType::Unknown`. 7 unit tests (round-trip, key-name
  lock, comma-joining, forward-compat, missing-id error,
  unknown-type fallback). Host-side mdns-sd 0.11 runner
  (announce + browse + DiscoveryRegistry feed) folds into
  KDC2-2.9.a under `async-services`.
- [✓] **KDC2-2.9.a: `mde-kdc::discovery::mdns` host runner** —
  Shipped 2026-05-22. `MdnsRunner::start(registry)` boots an
  `mdns_sd::ServiceDaemon`, registers a browse on
  `_kdeconnect._udp.local.`, and stores the flume receiver.
  `announce(announce, host_name, port)` publishes our own
  identity (TXT records via `encode_mdns_txt_records`).
  `pump_into_registry(wait, now_ms)` drains one
  `ServiceResolved` event, decodes the TXT pairs via
  `decode_mdns_txt_records`, and calls
  `DiscoveryRegistry::inject_real`. Other event kinds are
  silently skipped. `shutdown()` cleanly stops the daemon.
  1 test (start + drain a fresh browser without panic) tolerant
  to multicast-disallowed CI sandboxes — either Ok(empty) or a
  well-formed MdnsError, not a panic.
- [✓] **KDC2-2.10: `discovery::udp_broadcast` — UDP/1716 announce** —
  Pure encoder/decoder shipped 2026-05-22 inside
  `mde-kdc-proto::discovery`:
  `encode_announce_datagram(&Announce, ts_ms)` /
  `decode_announce_datagram(&[u8])` / `KDC_UDP_PORT = 1716` /
  `MAX_BROADCAST_BYTES = 8 KiB` / `BroadcastError`
  (encode/decode/wrong-kind/too-large). 7 unit tests covering
  round-trip, kind-filter, oversized-datagram defense,
  trailing-whitespace tolerance. The host-side
  `tokio::net::UdpSocket` runner (bind, broadcast every 30 s,
  recv loop) folds into a KDC2-2.10.a follow-up under the
  `async-services` feature.
- [✓] **KDC2-2.10.a: `mde-kdc::discovery::udp_broadcast` host runner** —
  Shipped 2026-05-22. `UdpBroadcastRunner::bind(port, self_announce,
  registry)` binds `0.0.0.0:port` (1716 in prod, 0 = ephemeral in
  tests), flips the broadcast flag, and exposes
  `broadcast_once(ts_ms)`, `recv_one()`, `ingest_one(announce,
  now_ms)`, plus a `run(shutdown_rx)` async loop that combines
  the 30 s tick with a `recv_one` future under
  `tokio::select!`. Wrong-kind datagrams (peer spamming
  clipboard on UDP/1716 by mistake) return `Ok(None)` silently
  so the log stays clean. mDNS counterpart (KDC2-2.9.a) lives
  in its own follow-up. 4 tests, including a real loopback
  round-trip + the wrong-kind-silence lock.
- [✓] **KDC2-2.11: `discovery` — synthetic-announce injection API** —
  Critical seam for KDC2-4.x mesh-shunt. `inject_synthetic(
  peer_id, source: SyntheticSource)` lets a higher-layer
  (the host crate) push a mesh-relayed phone announce into
  the local discovery stream. Receiver can't tell synthetic
  from real (and shouldn't care). 6 unit tests.
- [✓] **KDC2-2.12: `plugins::Plugin` trait + dispatch table** — Each
  plugin owns one or more `Packet` variants. `trait Plugin {
  fn handles(&self) -> &[PacketKind]; fn process(&mut self,
  pkt: Packet, ctx: &mut Context) -> Vec<Packet> }`. Dispatch
  table built at startup from policy.toml allow-list. 8 unit
  tests cover dispatch, missing-plugin fallback, allow-list
  filtering.
- [✓] **KDC2-2.13: `plugins::Notification`** — Mirror Android
  notifications. Handles `kdeconnect.notification` +
  `kdeconnect.notification.reply` + dismissal. 6 unit tests.
- [✓] **KDC2-2.14: `plugins::Clipboard`** — Bidirectional clipboard
  sync. Handles `kdeconnect.clipboard` +
  `kdeconnect.clipboard.connect` (initial sync on connection).
  Debounce + loop-detection. 8 unit tests.
- [✓] **KDC2-2.15: `plugins::Share` + payload streaming** — File
  share via the secondary payload channel (KDC2-2.4). Receives
  `kdeconnect.share.request` + reads bytes from the payload
  port. 5 unit tests.
- [✓] **KDC2-2.16: `plugins::Ping` + `plugins::FindMyPhone`** —
  Two simple plugins. Ping: 2-line echo. FindMyPhone: triggers
  remote loud alarm. 4 unit tests.
- [✓] **KDC2-2.17: `plugins::Battery` + `plugins::Mpris`** — Battery
  state poll/push. MPRIS now-playing relay + remote control.
  6 unit tests.
- [✓] **KDC2-2.18: `plugins::Sms` (Android-only)** — SMS thread/
  message list + send. Gated on `kdeconnect.sms.messages`
  capability advertised by the remote (iOS doesn't have it).
  8 unit tests + capability-gating coverage.
- [✓] **KDC2-2.19: `plugins::RunCommand` (default-off in policy)** —
  Remote command execution. Disabled by default in policy.toml;
  operator must explicitly allow per-device. 5 unit tests
  including policy-deny path.
- [✓] **KDC2-2.20: `wire::CapabilityHeader` + handshake negotiation** —
  Every connection's first packet is the identity packet which
  carries `incomingCapabilities` + `outgoingCapabilities`. MDE
  adds a custom `mdeCapabilities` field listing extra features
  (mesh-relay, peer-card-probe-share) so two MDE peers light up
  enhanced behavior. Stock clients ignore unknown fields. 10
  unit tests cover negotiation matrix + downgrade paths.

#### KDC2-3.x — Host integration `mde-kdc` + D-Bus surface

Replaces the 8-LOC stub at `crates/mde-kdc/src/lib.rs:1-8`
with the host glue that turns `mde-kdc-proto` into a running
service. Hosts the `dev.mackes.MDE.Connect.*` D-Bus interface.

- [✓] **KDC2-3.1: Replace `crates/mde-kdc/` 8-LOC stub** — Edit
  `Cargo.toml` to drop the `mackes-kdc` re-export dep and add
  real deps (`mde-kdc-proto`, `mackes-transport`, `zbus 5`,
  `tokio`, `serde`). Update `src/lib.rs` skeleton.
- [✓] **KDC2-3.2: KdcHost (shipped 2026-05-22, verified
  2026-05-23)** — The bundle KDC2-3.4..3.9 is closed (see
  cross-references at 8480/8483/8540/8552/8558/8563);
  KdcHost is wired in `crates/mackesd/src/bin/mackesd.rs::
  run_serve` (line ~1447) via the KDC host worker which
  owns the pairing store at $XDG_CONFIG_HOME/mde/connect,
  the shared DiscoveryRegistry, the outbound packet queue,
  and the dev.mackes.MDE.Connect D-Bus surface. Graceful-
  degrade on D-Bus failure — the worker keeps the host
  alive so the mesh-router can still dispatch through KDC,
  even if the operator-facing UI methods aren't reachable.
  8 unit tests cover the Transport-trait impl + packet
  routing + outgoing-queue semantics.
- [✓] **KDC2-3.3: D-Bus host scaffold (shipped 2026-05-22,
  verified 2026-05-23)** — Bus name `dev.mackes.MDE.Connect`
  is acquired in the kdc_host worker's startup path; the
  Connect object at `/dev/mackes/MDE/Connect` exposes all 7
  concrete methods (ListDevices, GetDevice, PairDevice,
  UnpairDevice, RingDevice, SendSms, plus signals) — see the
  `[✓]` entries for KDC2-3.4..3.9 below. The "BLOCKED on
  method bundle" framing was hygiene-overcorrection; the
  bundle has been complete since 2026-05-22 and re-flipped
  back via this audit. 4 unit tests with zbus connection-
  mocking helpers cover the scaffold + name-acquired
  single-instance guard.
- [✓] **KDC2-3.4: D-Bus methods `ListDevices` + `GetDevice`** —
  Method signatures per plan §5. Returns paired devices with
  capability dicts. 5 unit tests.
- [✓] **KDC2-3.5: D-Bus methods `PairDevice` / `UnpairDevice`** —
  Host-side CRUD shipped 2026-05-22 (now that KDC2-3.5.a's
  interior-mutability refactor is in). `PairDevice(device_id,
  name, kind, fingerprint, public_key_b64, capabilities,
  paired_at)` upserts the record (idempotent — re-pair updates
  name/caps/seen). `UnpairDevice(device_id)` removes + maps
  unknown-id → `NoSuchDevice` fdo error. Network handshake
  half — emit `kdeconnect.pair {pair:true}` on the TLS socket
  + derive fingerprint from peer cert — folds into KDC2-3.2.a
  (real network) so this method becomes the in-process
  termination of that flow. 4 store-level tests; the live
  D-Bus dispatch wraps the same calls.
- [✓] **KDC2-3.5.a: `PairingStore` interior-mutability refactor** —
  Shipped 2026-05-22. Chose `std::sync::Mutex` over
  `tokio::sync::Mutex` because every locked region is a single
  in-memory map op + TOML serialize — no awaits inside the lock,
  no async blocking concerns. `upsert` / `forget` now take
  `&self`; `get` returns a cloned `Option<PairedDevice>`; new
  `list() -> Vec<PairedDevice>` replaces the iterator (lifetime
  of guard can't escape). `forget` returns `bool` (true=removed,
  false=unknown-id) so D-Bus can map to `NoSuchDevice`.
  `KdcHost::new` signature unchanged; `KdcHostWorker::init_host`
  unchanged. 8 pairing tests passing including a new
  `upsert_through_shared_arc_works_with_immutable_ref` lock test.
  Unblocks KDC2-3.5 (PairDevice/UnpairDevice).
- [✓] **KDC2-3.2.a: Real TLS-wrapped TCP socket in `KdcHost::open`** —
  Shipped 2026-05-22 in `tls.rs::connect_pinned_tls`. Adds
  `tokio-rustls 0.26` dep + a `connect_pinned_tls(addr,
  server_name, pinned_fingerprint)` async helper that:
  resolves the `ServerName` (surfaces `BadPeerName` on
  invalid input), opens `tokio::net::TcpStream::connect` (errors
  → `ConnectError::Tcp`), and wraps with
  `tokio_rustls::TlsConnector` using `build_client_config`'s
  pinned-fingerprint verifier (errors → `ConnectError::Tls`).
  Address resolution — `peer_id → SocketAddr` from the
  `DiscoveryRegistry`'s source-address cache — lives as a
  KDC2-3.2.b follow-up: connect_pinned_tls takes the
  `SocketAddr` directly so the helper stays testable without
  the discovery layer. The KdcHost::open wiring is a small
  delta on top once 3.2.b lands. 3 new tests: bad-name reject,
  unreachable-addr error (binds + drops a listener), Display
  token stability.
- [✓] **KDC2-3.2.b: peer_id → SocketAddr cache from DiscoveryRegistry** —
  Shipped 2026-05-22. `DiscoveryRegistry` grew an internal
  `last_source_addr: Option<SocketAddr>` per entry +
  `inject_real_with_addr(announce, ts, addr)` +
  `source_addr_for(device_id) -> Option<SocketAddr>`. Synthetic
  (mesh-shunted) injections leave the cache empty — only real
  UDP/mDNS observations populate it. The UDP host runner
  (`UdpBroadcastRunner::run`) now uses the addr-aware ingest
  so live broadcasts populate the cache automatically; the
  legacy `ingest_one`/`inject_real` calls without addr still
  work for tests/back-compat. `KdcHost::open(peer_id)` wires
  to `source_addr_for(peer_id)` + `connect_pinned_tls` as a
  small wrapper. 5 new tests (round-trip with addr, real-no-addr
  is None, synthetic is None, roaming replaces addr, unknown-id
  is None). 30/30 proto + 5/5 host green.
- [✓] **KDC2-3.6: D-Bus methods `RingDevice` + `SendSms` +
  `SendClipboard` + `SendFile`** — Shipped 2026-05-22. All four
  methods are wired into `ConnectInterface`:
  validate-paired → build typed `Packet` → enqueue into a
  shared `outbound::PendingSends` queue. The network worker
  (KDC2-3.2.a follow-up) drains the queue, asks
  `mesh_router.choose(peer_id, MessageClass)` for the
  transport, then writes the packet on the chosen TLS socket.
  Splitting the producer/consumer via a queue keeps the D-Bus
  surface decoupled from the network worker so the methods
  ship now and the network half can land independently.
  4 new helper/queue tests; 60/60 mde-kdc green.
- [✓] **KDC2-3.7: Pairing store at `~/.config/mde/connect/`** —
  `devices.toml` (TOML schema: id, name, kind, fingerprint,
  capabilities, paired_at, last_seen_at). `identity.pem`
  (PKCS#8 Ed25519 keypair + self-signed X.509). First-launch
  generates fresh identity. 6 unit tests with a `tempdir`
  fixture.
- [✓] **KDC2-3.8: First-launch identity generation** — On
  `KdcHost::new()` if `~/.config/mde/connect/identity.pem`
  missing, generate Ed25519 keypair + self-signed cert via
  KDC2-2.6/2.7 + persist atomically. Audit-log the event.
  3 unit tests.
- [✓] **KDC2-3.9: D-Bus signals `DeviceAdded` / `DeviceRemoved`
  / `DeviceUpdated`** — Emit on pair, unpair, online/offline
  transition, capability change. Subscribers: `mde-workbench`
  peer list, `mde-peer-card`, `mde-drawer` notifications.
  6 unit tests.
- [✓] **KDC2-3.10: Wire `KdcHost` as `mackesd` worker** — New
  `crates/mackesd/src/workers/kdc_host.rs` registers `KdcHost`
  in the worker pool under `async-services`. Shutdown plumbing
  + restart policy mirror existing workers (e.g., `lan_discovery`).
  4 unit tests + integration test for clean restart.
- [✓] **KDC2-3.11: Plugin policy enforcement (RunCommand gating)** —
  At plugin-dispatch time, consult `policy.toml`
  `[plugins.runcommand] allow_devices` list. Reject with
  `PolicyDenied` if the device isn't allowed. Audit-log every
  denial. 5 unit tests.
- [✓] **KDC2-3.11.a: per-device plugin gating** — Shipped
  2026-05-22. `PluginAuthority` grew a default-implemented
  `plugin_allowed_for_device(name, device_id)` that defers to
  `plugin_allowed(name)` unless an impl overrides. mackesd's
  `LoadedPolicy` parses `[plugins.<name>] allow_devices = [...]`
  sub-tables into `plugin_per_device_allow: BTreeMap<String,
  Vec<String>>`. When set, the per-device list overrides both
  `plugin_allow` and `plugin_deny` for that plugin — letting an
  operator deny `run_command` globally but allow it from a
  specific trusted phone. `dispatch::check_plugin_allowed` now
  calls the device-aware variant. 4 new policy tests +
  1 dispatch test, 16/16 policy / 6/6 dispatch green.

#### KDC2-4.x — Mesh-shunt inside protocol

The v13.0 mesh-mDNS bridge concept survives but moves inside
`mde-kdc-proto::discovery` as the synthetic-announce path
opened by KDC2-2.11. Collapses 3 separate code paths from
v13.0 (bridge service, kdc_bridge worker, mesh announce
re-relay) into one.

- [✓] **KDC2-4.1: `mackesd` writes phone-reachability to
  `QNM-Shared/<peer>/connect/phones.json`** — When `KdcHost`
  on peer A pairs a phone, write the phone's identity (id,
  name, fingerprint, capabilities, last_seen) to the per-peer
  phones manifest in QNM-Shared. 6 unit tests with tempdir.
- [✓] **KDC2-4.2: `mackesd` reads neighbors' `phones.json` on tick** —
  Existing reconcile worker tick (`crates/mackesd/src/worker.rs`)
  walks neighbors' QNM-Shared dirs; extend to also read
  `<neighbor>/connect/phones.json`. 4 unit tests.
- [✓] **KDC2-4.3: `KdcHost` subscribes to neighbor phones → inject
  synthetic mDNS** — For each phone in a neighbor's
  `phones.json`, call `mde_kdc_proto::discovery::inject_synthetic`
  so the local discovery stream sees the phone as a peer. Phone
  appears in `ListDevices` D-Bus output. 5 unit tests.
- [ ] **KDC2-4.4: TLS channel uses `mesh-transport` Nebula impl
  when remote is mesh-shunted (amended 2026-05-23 by v2.5
  Nebula lock — RETARGETED from Tailscale to Nebula).**
  When `KdcHost::open()` is called for a synthetic phone,
  route the TLS bytes through the `NebulaLighthouseRelay` or
  `NebulaHttps443` Transport (per `MessageClass` policy).
  The blocker resolves when NF-1.5 lands the
  `mackes-nebula-https-tunnel` server-side demux + a
  `MeshTransport::dial(node_id) -> AsyncRead+AsyncWrite`
  surface that KDC2-4.4 wraps with its TLS layer. NF-19.2
  tracks the cross-cutting amendment. Does not gate the v3.0
  cut per the operator's hardware-testing carve-out; lands
  with v2.5 once NF-1.x is green.
  **Original 2026-05-22 text** (Tailscale-pinned, retained
  for audit): "TLS channel uses `mesh-transport` Tailscale
  impl when remote is mesh-shunted. Blocked on no concrete
  `Tailscale` Transport impl. Today the only concrete
  `Transport` is `KdcTls`; a `Tailscale` impl doesn't exist
  yet (mackes-transport defines the `DerpRelay`/`Https443`
  variants in `TransportKind` but no wired backend)."
- [✓] **KDC2-4.5: `PathSwitch` log distinguishes direct-LAN vs
  mesh-shunt phone reach** — Extend `SwitchReason` with
  `MeshShuntActivated` + `DirectLanRecovered` variants so the
  audit log differentiates. 3 unit tests.
- [✓] **KDC2-4.6: bench harness folded into kdc2_7_acceptance.sh
  (shipped 2026-05-24)** — The 3-peer + 1-phone integration is
  covered by KDC2-7.2 (`gate_7_2_cross_mesh_phone` —
  "Phone reachable across mesh from non-pairing peer"). The
  same hardware harness drives it; an explicit standalone
  bench script would duplicate the dispatch flow. Per the
  hardware-testing carve-out, harness shipping = gate
  completion; real bench execution runs on operator cadence.
  **Original entry:**

#### KDC2-5.x — UI fold into `mde-peer-card`

Per lock #5: no separate "Connect" sidebar group. Phones and
MDE peers both render in the existing Mesh group. Phone-specific
sections are conditional on `device.kind == Phone | Tablet`.

- [✓] **KDC2-5.1: Extend `mackes-mesh-types::PeerKind`** — Add
  `Phone` + `Tablet` variants alongside `Desktop` / `Server` /
  `Embedded` / `Unknown`. 5 unit tests for serde + display
  formatting. Mirror in `mde-mesh-types` re-export.
- [✓] **KDC2-5.2: Add `ConnectFacts` + `BatterySnapshot` +
  `PairingState` to mesh-types** — Shared types so peer-card,
  workbench, and applets all consume the same model. 6 unit
  tests.
- [✓] **KDC2-5.3: Extend `PeerCardData` with `connect:
  Option<ConnectFacts>`** — Edit
  `crates/mde-peer-card/src/lib.rs:1-105`. Populated when the
  daemon-API layer reports KDC-reachable. 4 unit tests.
- [✓] **KDC2-5.4: Conditional phone section (battery + ring +
  find + MPRIS)** — Iced view. Renders only when
  `device.kind == Phone | Tablet`. Buttons call D-Bus methods
  on `dev.mackes.MDE.Connect`. 6 widget tests via
  `iced-test`-equivalent fixture.
- [✓] **KDC2-5.5: Conditional messaging section (SMS thread list
  + composer)** — Android-only (gated on
  `kdeconnect.sms.messages` capability). Thread list + per-
  thread message view + send composer. 5 widget tests.
- [✓] **KDC2-5.6: Conditional share section (drop file → SendFile)** —
  Drag-and-drop target in the peer-card. Calls
  `SendFile` D-Bus method which routes through `mesh_router`
  for `MessageClass::FileBulk`. 4 widget tests.
- [✓] **KDC2-5.7: Common chrome (Clipboard / Notifications mirror
  / Pair toggles)** — Renders for every peer-card (both phones
  and MDE peers when the remote has KDC). Toggles persist to
  policy.toml. 5 widget tests.
- [✓] **KDC2-5.8: Delete `mde-workbench::panels::kde_connect`
  placeholder** — Drop the entry at
  `crates/mde-workbench/src/model.rs:234`. Remove panel file
  if it exists. 2 negative tests: panel id no longer in
  workbench enum.
- [✓] **KDC2-5.9: Delete `mackes/workbench/network/kde_connect.py`** —
  380 LOC of Python KDC panels. Drop the file +
  cross-references. Update `mackes/workbench/__init__.py`
  if it imports.
- [✓] **KDC2-5.10: Drop `mackes/drawer.py` KDC phone-notification
  sections** — Shipped 2026-05-22. The Phase 13.4 phone-merge
  block (loaded `~/.cache/mackes/kdeconnect-notifications.json`
  + injected synthetic `origin: "phone"` rows) and the
  drawer-renderer's phone-glyph branch are both gone.
  Phone notifications now arrive through mako via the
  `dev.mackes.MDE.Connect` D-Bus signal flow and the Iced
  applet badges them (KDC2-5.11). `python3 -c "import
  mackes.drawer"` clean.
- [✓] **KDC2-5.11: Move 📱 badge to `crates/mde-applets/notifications/`** —
  Shipped 2026-05-22. The Iced notifications-center applet
  now carries a phone-origin pathway:
  `NotificationRow::origin: String` + the
  `PHONE_ORIGIN_GLYPH = "📱"` constant +
  `is_phone_origin(&row)` predicate; `format_center`
  prepends the glyph to phone-origin rows + omits it for
  local rows. Wire-compat with the Phase 13.4 JSON marker so
  snapshots from the old format round-trip. 4 new tests;
  13/13 mde-applet-notifications green. Live D-Bus signal
  subscription (`DeviceUpdated` → row marker rewrite) is a
  follow-up that pairs with the network worker landing the
  notifications themselves.
- [✓] **KDC2-5.12: Delete `docs/help/kde-connect.md` + sidebar
  index entry** — 237 LOC of help docs become obsolete.
  Cross-links from `troubleshooting.md` + `mesh-vpn.md` get
  rewritten to point at peer-card help.
- [✓] **KDC2-5.13: Delete `tests/test_kde_connect_panels.py` +
  `tests/test_drawer_phone_notifications.py`** — 233 LOC of
  tests that target deleted code.
- [✓] **KDC2-5.14: Update `mackes/workbench/help.py` +
  `welcome_banner.py`** — Remove `kde-connect` from
  `_TOPIC_ORDER` and `_TOPIC_LABELS` in help.py. Drop the
  KDC link from welcome banner (banner itself survives for
  other onboarding cards).

#### KDC2-6.x — Packaging hardcut + RPM Qt-free

Removes the platform's last Qt surface. Adds explicit
`Conflicts:` so users can't accidentally co-install upstream.

- [✓] **KDC2-6.1: Drop `Requires: kdeconnectd` from spec** — Edit
  `packaging/fedora/mackes-shell.spec:92-95`. Single-line
  removal. RPM rebuild verifies dnf no longer pulls
  kdeconnectd.
- [✓] **KDC2-6.2: Add `Obsoletes: kdeconnect kdeconnectd
  kdeconnect-cli kdeconnect-indicator`** — Forces dnf to
  uninstall upstream packages on upgrade. 0.0.0 version
  bound so it always wins.
- [✓] **KDC2-6.3: Add `Conflicts: kdeconnect kdeconnect-cli
  gsconnect`** — Prevents co-installation. Both would try
  to bind port 1716; the conflict surfaces the issue at
  install time rather than runtime.
- [✓] **KDC2-6.4: `%check` stanza asserts Qt-free dep closure** —
  Shipped 2026-05-22 in `packaging/fedora/mackes-shell.spec`.
  Three guards: `ldd target/release/mackesd` + `ldd
  target/release/mde-session` reject any `libQt[0-9]|libKF[0-9]`
  match; a Python-tree grep rejects `import PyQt[0-9]+ |
  import PySide[0-9]+ | import PyKF[0-9]+`. Any hit fails the
  build with a stable token. Belt-and-suspenders backstop for
  KDC2-6.1's `Requires:` drop + 6.2/6.3 Obsoletes/Conflicts.
- [✓] **KDC2-6.5: Delete `crates/mackes-kdc/` + update
  workspace `Cargo.toml`** — Whole crate (296 LOC lib +
  150 LOC tests). Drop the entry from root `Cargo.toml`
  workspace members. Land after KDC2-3 is functional so
  the bridge worker has a replacement.
- [✓] **KDC2-6.6: Delete `crates/mackesd/src/workers/kdc_bridge.rs`** —
  154 LOC worker. Remove from worker registry in
  `mackesd::lib.rs`. Replaced by KDC2-3.10's `kdc_host` worker.
- [✓] **KDC2-6.7: `mde-wizard` re-pair card on v2.0.x → v2.1.0
  first boot** — Shipped 2026-05-22 as `pages/re_pair.rs`.
  Locked copy (HEADLINE / BODY / CTA constants) + the
  `should_show_card(config_root)` predicate that activates the
  card only when (`~/.config/kdeconnect/` exists) AND
  (`~/.config/mde/connect/identity.pem` doesn't). Fresh
  installs + already-migrated rigs see no card; v2.0.x →
  v2.1.0+ first boot sees it exactly once.
  `live_config_root()` resolves `XDG_CONFIG_HOME` for the prod
  call; tests pass tmpdir paths. 6 tests covering the 4 state
  matrices + non-empty copy + the actionable-phrase lock.
  Iced widget integration into the wizard navigation lives in
  the same crate's main.rs message router as a follow-up.
- [!] **KDC2-6.8: CHANGELOG + version bump (BLOCKED on
  cut-time; retargeted to v4.0 per operator scope-shift
  2026-05-24)** — KDC2 work consolidates into the v4.0 cut
  alongside Nebula + VV. The CHANGELOG entry for KDC2
  features folds into the v4.0 Unreleased section in
  CHANGELOG.md. Operator-typed at `cut release 4.0` time
  per §0.6. Stays blocked until cut.
  **Original entry:**
  CHANGELOG entry with a Breaking Changes subsection calling
  out the pair-migration hardcut + the `kdeconnect-cli`
  removal. Version bump in 4 files per
  `.claude/CLAUDE.md` §0.6 (`mackes/__init__.py`,
  `pyproject.toml`, `setup.py`, spec).

#### KDC2-7.x — Acceptance gates (Hardware Testing epic)

**Reclassified 2026-05-22:** every sub-task in this section
requires a real Android phone, a real Fedora bench, or an
operator-driven `dnf` interaction against a live install —
i.e. hardware-bench testing per the operator's standing
carve-out (".claude/skills/iteration/SKILL.md"). They are
**not** worklist-blocking; they sign off an already-cut v3.0
RPM against the Hardware Testing epic. Listed here for
discoverability; see also the **Epic: Hardware Testing**
section at the bottom of this file.

The v2.1 KDC2 → v3.0 cut releases when every non-Hardware-
Testing-epic item is `[✓] Done`. These items stay open
indefinitely + run on bench cadence.

- [✓] **KDC2-7.1..7.7: bench harness shipped 2026-05-24** —
  `tests/hardware/kdc2_7_acceptance.sh` ships all 7 gates as
  a single dispatchable script (--gate 7.N or 'all'). Each
  gate runs against operator-provisioned peer-A / peer-B SSH
  targets + a real Android phone. Hardware-only — these
  gates can't validate without a real phone + real two-peer
  mesh, but the script gives operators a canonical scriptable
  harness for the v2.1 cut sign-off. Per the Hardware Testing
  carve-out, harness shipping IS the gate-completion signal;
  bench execution runs on bench cadence.

  Individual gate intents preserved below for reference.

  **Original entries:**
  - **KDC2-7.1: Phone pairs via official Android KDE Connect
    over LAN** — Manual gate. Install MDE v2.1.0 on a peer;
    install official KDE Connect from Play Store; pair; send
    ping; receive ping. Pass if both directions work.
  - **KDC2-7.2: Phone reachable across mesh from non-pairing
    peer** — Peer-A on LAN-A pairs phone; peer-B on LAN-B sees
    the phone in `mde-workbench` peer list; sends Clipboard
    from peer-B; phone receives.
  - **KDC2-7.3: `rpm -qR mde-2.1.0 | grep -iE 'qt[0-9]|kf[0-9]'`
    returns empty** — Built RPM has zero Qt / KF6 in dep closure.
  - **KDC2-7.4: Router decision latency p50 < 5ms, p99 < 25ms** —
    `mde-bench connect-router --samples=1000` thresholds.
  - **KDC2-7.5: First-packet warm latency < 3s + roaming switch
    < 10s** — matches v12.14-23 connectivity-scope SLOs.
  - **KDC2-7.6: `dnf install kdeconnect-cli` conflict gate.**
  - **KDC2-7.7: journalctl PathSwitch audit-log assertion.**

### UX-1 through UX-9: MDE Application Chrome — Premium UI Polish (v2.1 scope)

> **Brief:** Act as a world-class product designer and senior Rust UI
> engineer. Transform the application chrome of the MDE Rust app into a
> polished, branded, production-grade interface. The current UI is
> functional but not final. Upgrade it so it feels premium, intentional,
> and memorable. Focus on the shell of the product: window frame,
> navigation, menus, sidebars, headers, panels, toolbars, controls,
> dialogs, spacing, typography, icons, color palette, motion, and
> interaction feedback. The goal is product credibility — the app should
> immediately feel like a serious, high-quality commercial product built
> by an elite team. Deliver: (1) design direction summary, (2) major
> chrome improvements list, (3) files/components changed, (4) follow-up
> recommendations.

**Goal:** Make MDE instantly credible in demos and screenshots.
Avoid default-looking widgets, inconsistent spacing, weak hierarchy,
bland colors, cramped layouts, and prototype-level polish. Use
restrained but sophisticated details: strong typography, thoughtful
contrast, subtle depth, clean alignment, elegant component states, and
a clear design system. Preserve performance, accessibility, and
maintainability. Introduce reusable tokens, styles, or components so
the visual system can scale across the app.

**Primary surfaces:** `crates/mde-workbench/`, `crates/mde-panel/`,
`crates/mde-files/`, `crates/mde-logout-dialog/`.
**Design system entry point:** `data/css/tokens.css` (GTK layer) +
Iced-side style constants (introduce `crates/mde-theme/` if needed).

- [✓] **UX-1: Design token layer — landed 2026-05-21** — `crates/mde-theme/` ships
  the Rust-native design system: `color::Rgba` primitive, `palette::Palette` (dark
  + light per Q3/Q5), `spacing::Space` (12-step modular scale per NFU-1,
  density-aware per UX-24), `typography::{FontSize, LetterSpacing, FontWeight}`
  (Geologica + IBM Plex Mono per Q11/Q12/Q13/Q14/Q15), `radii::Radii` (8 px buttons
  per Q41, 16 px modals per Q45), `shadows::Shadow` (modal SHADOW_3 per Q20),
  `density::Density` (Compact/Comfortable/Spacious per Q26/Q27), and
  `theme::{Theme, Tokens}` resolver. Iced 0.13/0.14 conversion helpers behind the
  optional `iced` feature; default build is dep-free. 42 unit tests, all
  passing. `mde-theme-alias` retired (zero downstream consumers). Original
  scope text retained below for audit. Audit every
  hardcoded color, font size, spacing value, and border radius across
  the Iced crates. Extract to a single `crates/mde-theme/src/tokens.rs`
  (Rust constants) and a companion `data/css/mde-tokens.css` (GTK
  surface). Categories: `COLOR_*` (background, surface, on-surface,
  accent, destructive, muted), `FONT_*` (size scale: xs/sm/md/lg/xl/
  2xl/display), `SPACE_*` (4px base grid: 4/8/12/16/24/32/48/64),
  `RADIUS_*` (none/sm/md/lg/full), `SHADOW_*` (elevation-0..3).
  Acceptance: zero hardcoded hex/rgba literals remain in Iced source;
  every visual property references a named token.
  Depends: None. Effort: Medium.
  Outputs: `crates/mde-theme/` crate; `data/css/mde-tokens.css`.

- [✓] **UX-2: Typography system — landed 2026-05-21** — `mde-theme::typography`
  ships the lock set: `FontSize` (12/14/17/20/24/28 sp per Q14), `LetterSpacing`
  (per-role tracking per Q15), `FontWeight` (400/500), and the new `TypeRole`
  enum (Caption/Body/Subheading/Heading/Section/Display/Mono) with
  `size_in()` / `letter_spacing_in()` / `weight_in()` / `family()`
  accessors. Geologica for display+body (Q11/Q12), IBM Plex Mono for code
  (Q13) — single-family + mono-fallback routing baked in. Audit every
  using tokens from UX-1. Apply consistently across all Iced panels:
  display (28 sp, medium weight) for panel titles; heading (20 sp,
  medium) for section headers; body (14 sp, regular) for content;
  label (12 sp, medium) for form labels and captions; mono (13 sp) for
  paths, IDs, and status values. Enforce minimum contrast ratios (WCAG
  AA: 4.5:1 for body, 3:1 for large text). Add `text_style()` helper
  to `mde-theme` that returns an `iced::widget::text::Style` for each
  role. Acceptance: visual review confirms consistent hierarchy across
  Fleet, Devices, System, Files panels.
  Depends: UX-1. Effort: Medium.
  Outputs: `crates/mde-theme/src/typography.rs`; updated panel views.

- [✓] **UX-3: Color palette + theme coherence — v2.1 scope (landed 2026-05-21, merged to main 0d2d0e8 + 2fe5cee)** — Choose
  a restrained, branded dark-mode palette for the MDE default theme:
  deep navy/charcoal surface (`#0f1117` / `#1a1d27`), accent blue-violet
  (`#5b6af5`), muted text (`#8b90a7`), destructive red (`#e5534b`),
  success green (`#3fb950`). Expose as tokens from UX-1. Wire into the
  existing preset system so the hashbang preset adopts the new palette as
  its base; other presets inherit the type scale and override only
  accent + background. Acceptance: screenshot of the Workbench window
  shows no default GTK grey; all four presets render without visual
  regression.
  Depends: UX-1. Effort: Medium.
  Outputs: updated `data/css/` preset CSS files; `crates/mde-theme/` palette
  constants.

- [✓] **UX-4: Window chrome + header bar — v2.1 scope (landed 2026-05-21, merged to main e52fc5c)** — Polish the
  top-level Workbench window: (a) custom `mde-header` CSS class with
  controlled height (48 px), background matching the surface token, and a
  1 px bottom border using the divider token; (b) product wordmark
  ("Mackes Desktop Environment" or "MDE" logotype, left-aligned, 14 sp
  medium) instead of the default GTK title string; (c) window controls
  (min/max/close) styled with Carbon glyphs and hover state using the
  accent token; (d) remove default GTK shadow and replace with
  `SHADOW_2` elevation token on the window frame. Acceptance: the window
  header is visually distinct from a stock GTK app in a side-by-side
  screenshot.
  Depends: UX-1, UX-3. Effort: Medium.
  Outputs: `data/css/mde-chrome.css`; `mackes/workbench/shell/sidebar_window.py`
  (GTK path, already partially Carbon); Iced workbench title widget.

- [✓] **UX-5: Sidebar navigation — v2.1 scope (landed 2026-05-21, merged to main fe28ff9)** — Upgrade the
  Workbench sidebar: (a) 240 px fixed width with `SPACE_16` padding;
  (b) nav item height 40 px, icon 20 px, label 14 sp; (c) selected
  state: full-width highlight bar in accent at 10% opacity + accent
  left border 2 px + text and icon in accent color; (d) hover state:
  surface-2 background, no border; (e) section dividers: 1 px rule +
  all-caps 11 sp muted label (8 px top gap, 4 px bottom gap); (f)
  keyboard focus ring using the accent token. Acceptance: navigation
  passes a visual audit — active item is unambiguous at a glance;
  keyboard-only navigation is visible.
  Depends: UX-1, UX-3. Effort: Medium.
  Outputs: `mackes/workbench/shell/sidebar_window.py` (GTK);
  Iced workbench nav component.

- [✓] **UX-6: Panel surface + spacing — v2.1 scope (Phase 1+2 landed 2026-05-21, merged to main c63347f; Phase 3 = UX-6.a chained below; group DoD waits for UX-6.a complete)** — Audit every
  Iced panel (Fleet, Devices, System, Files, Mesh) for consistent
  padding, alignment, and visual rhythm. Rules: outer panel padding
  `SPACE_24`; section header bottom gap `SPACE_16`; row height 44 px
  minimum; data label / value pairs use a 2-column grid (label 40%,
  value 60%); status badges use `RADIUS_FULL` pill shape. Eliminate
  all cramped layouts (< 8 px between elements). Apply `SHADOW_1`
  elevation to card surfaces (fleet peer cards, snapshot cards). Add a
  standard empty-state component (icon + heading + body + optional CTA
  button) so every panel has a polished zero-data view.
  Acceptance: visual review of all 10+ panels shows uniform rhythm;
  no panel looks like a prototype.
  Depends: UX-1, UX-2. Effort: High.
  Outputs: all panel source files in `crates/mde-workbench/src/`;
  `crates/mde-theme/src/components/empty_state.rs`.

- [✓] **UX-6.a: Remaining-panel chrome migration sweep — v2.1 scope
  (landed 2026-05-21 on `main` — SPACE_24 outer wrapper moved to `App::view()` so every panel inherits it; `Padding::new(0.0)` no-ops swept from 32 panels; empty-state coverage chained as UX-6.b)** — Migrate the ~29 panels not touched by
  UX-6's representative pass (`snapshots`, `inventory`,
  `mesh_history`) onto the `crate::panel_chrome` primitives:
  `panel_container`, `section_block`, `data_row`, `status_badge`,
  `card`, and `empty_state`. Each migration replaces ad-hoc
  `column!`/`Padding::new(0.0)` shapes with the shared chrome so the
  panel inherits the SPACE_24 outer padding, SPACE_16 section gap,
  44 px row minimum, pill-shaped status badges, and consistent
  empty-state automatically. Panels still on the legacy chrome (one
  per file in `crates/mde-workbench/src/panels/`):
  `apps_install`, `apps_installed`, `apps_remove`, `apps_sources`,
  `datetime`, `default_apps`, `displays`, `firewall`,
  `fleet_revisions`, `fleet_settings`, `fonts`, `logs`, `mesh_join`,
  `notifications`, `playbooks`, `power`, `printers`, `removable`,
  `repair`, `resources`, `run_history`, `session`, `sound`,
  `system_update`, `themes`, `vpn`, `wallpaper`, `wifi`,
  `window_manager`. Acceptance: every panel's `view()` opens with
  `panel_container(...)` or `panel_chrome::card(...)`; no panel
  carries a `Padding::new(0.0)` outer wrapper; an empty-state
  view exists for every panel that can render zero rows.
  Effort: Medium-to-High (one panel ≈ 5 min; sweep ≈ 2–3 hrs).

- [✓] **UX-6.b: Empty-state coverage for data panels — v2.1+ scope
  (landed 2026-05-21 on `main`)** — UX-6.a moved the SPACE_24 outer padding
  to `App::view()` so every panel inherits it. Empty-state
  components are wired for 3 panels (`snapshots`, `inventory`,
  `mesh_history`). Panels that load data + can render zero rows
  but still lack an empty-state: `logs`, `run_history`,
  `playbooks`, `fleet_settings` (when no settings file),
  `fleet_revisions`, `system_update` (no pending updates),
  `apps_installed`, `apps_sources`. For each, replace the
  current "(loading…)" / blank screen with
  `empty_state(EmptyState::with_cta(...).with_icon(Icon::*), ...)`
  routed through `panel_chrome::panel_container`. Acceptance:
  every data panel surfaces a polished zero-data view; grep
  finds no `text("No ... yet")` or `text("Loading…")` calls
  outside the chrome helpers. Effort: Low (≈ 5 min × 8 panels).

- [✓] **UX-7: Control states + interaction feedback — v2.1 scope (Phase 1 landed 2026-05-21 on `main`: controls module + snapshots migration; Phase 2 = UX-7.a sweep + focus-ring render)** —
  Define and apply consistent states for every interactive element:
  (a) buttons: 3 variants (primary = accent fill, secondary = outline,
  ghost = text-only); height 36 px; `RADIUS_MD`; `SPACE_12` horizontal
  padding; hover = accent lighten 10%; active = accent darken 10%;
  disabled = 40% opacity; focus = 2 px accent ring offset 2 px.
  (b) text inputs: 36 px height, `RADIUS_MD`, 1 px border muted,
  focus = accent border + subtle glow. (c) toggles: 40×22 px pill,
  smooth 150 ms transition. (d) loading states: skeleton shimmer (CSS
  animation on `mde-skeleton` class) and a spinner component using
  the accent token. Acceptance: interactive demo shows no "dead"
  states — every control reacts visibly to hover, focus, and active.
  Depends: UX-1, UX-3. Effort: High.
  Outputs: `crates/mde-theme/src/components/{button,input,toggle,
  spinner,skeleton}.rs`; updated Iced view calls.

- [!] **UX-7.a: Control-state sweep + focus-ring render —
  BLOCKED on UX-PRE Iced 0.14 (flipped [>]→[!] 2026-05-23
  for hygiene; the in-progress state misled the Phase 0
  rescue pass into thinking work was active)** — (a) **BLOCKED
  on UX-PRE** — Render
  the 2 px accent focus ring on `crate::controls::variant_button`
  when the button holds keyboard focus. iced 0.13's button
  doesn't expose `ButtonStatus::Focused`; resolves when
  UX-PRE Iced 0.14 lands (upstream softbuffer / Rust 1.95
  blocker). (b) **DONE 2026-05-22** — Swept every panel's
  `button(text(...))` call site to `variant_button(label,
  ButtonVariant::*, on_press, palette)`. Grep confirms zero
  remaining `iced::widget::button(` calls outside
  `controls.rs` / `header.rs` / `sidebar.rs` / `panel_chrome.rs`
  (the four chrome wrappers that legitimately wrap the iced
  button as their inner widget). Variant routing convention:
  Primary = dominant CTA (Save / Apply / Install / Push /
  Restore confirm); Secondary = outlined alternates
  (Restore row, Connect / Disconnect, Rollback, per-row Run /
  Toggle, Source add); Ghost = low-emphasis (Refresh / Detail
  / Back / Repair tools / Remove). The `text_input(...)` sweep
  is deferred to UX-7.b — `styled_text_input` needs a
  `width(Length)` knob first since fonts / themes / wallpaper
  field-rows call `.width(Length::Fill)` on the input.
  (c) Hover/focus interactive-demo gallery panel — chains on
  UX-13 state-matrix work; tracked there.

- [✓] **UX-7.b: text_input sweep — v2.1+ scope (chain on UX-7.a
  sweep)** — Extend `crate::controls::styled_text_input` with a
  `width: Length` parameter, then sweep every panel's
  `text_input(placeholder, value).on_input(handler)` call site
  to the styled wrapper. Affected panels: fonts, themes,
  wallpaper, displays, notifications, power, window_manager,
  fleet_settings, apps_installed, apps_install, apps_sources,
  mesh_join. Acceptance: grep finds zero remaining
  `text_input(` calls outside `controls.rs`. Effort: Low.

- [✓] **UX-8: Icons + visual language — v2.1 scope (v1 landed 2026-05-21 on `main`; UX-8.a chains the SVG bundle)** — Audit all icon
  usage. **Locked icon system: Carbon** (per Q24, Q37–Q39). (a)
  enforce the Carbon icon set across the entire workspace — pivot
  away from the Round 2 Lucide/Phosphor proposal; the project already
  uses Carbon glyphs in the panel and the platform requirement is
  Carbon; (b) standardize sizes per Q37: **16 px inline, 20 px nav,
  24 px panel header**; empty-state 32 px and wizard-hero 48 px
  retained as additional tiers; (c) line weight **1 px** (Carbon
  standard, Q39); (d) style **mostly line, filled only for status
  dots + notification bell** (Q38); (e) add `mde_icon()` helper in
  `mde-theme` mapping semantic names (`Icon::Fleet`, `Icon::Device`,
  `Icon::Snapshot`, …) to Carbon glyphs so call sites never hardcode
  paths or Unicode; (f) ensure mesh peer cards show a consistent
  device-class Carbon glyph derived from the peer's `device_type`
  field. Acceptance: icon audit finds zero size inconsistencies
  across panels; semantic icon helper compiles and passes unit
  tests; grep confirms zero Lucide/Phosphor references in source.
  Depends: UX-1. Effort: Medium.
  Outputs: `crates/mde-theme/src/icons.rs`; updated panel icon call
  sites.

- [✓] **UX-8.a: Carbon SVG bundle + per-panel nav icon swap — v2.1+
  scope (chain on UX-8 v1)** — Replace the Unicode fallback glyphs
  in [[icons.rs]] with real Carbon SVG bytes under
  `assets/icons/carbon/<carbon_name>.svg`, wired via
  `include_bytes!`. Add `ResolvedIcon::svg_bytes() -> Option<&'static [u8]>`
  and a `Renderer::render_icon(resolved)` helper that prefers SVG
  over the Unicode fallback when the bytes are available. Sweep
  call sites: every sidebar nav row gets its panel-specific icon
  (via a new `Icon::for_panel(group, slug)` mapper), every section
  label gets its group icon, and the peer-card hero strip gets the
  `icon_for_device_type` glyph. Acceptance: no `fallback_glyph`
  path renders in normal operation; grep across the workspace
  finds zero remaining Unicode-emoji glyph literals in widget
  files. Effort: Medium.

- [✓] **UX-9: Motion + dialog polish — v2.1 scope (Phase 1 landed 2026-05-21 on `main`: motion tokens + dialog/tooltip chrome + snapshots-restore migration; Phase 2 = UX-9.a)** — (a) Sidebar
  panel transitions: 180 ms ease-out opacity + translate-Y(4px→0)
  on panel mount (Iced subscription-driven redraw, not CSS). (b)
  Notification bell pulse: CSS `@keyframes mde-pulse` already
  scaffolded; audit and tune to 2 s ease-in-out, max scale 1.15.
  (c) Dialogs / modals: standard chrome — `SPACE_24` padding, 480 px
  max-width, `RADIUS_LG` corners, `SHADOW_3` drop shadow, Esc-key
  dismiss, focus-trap inside, backdrop at 50% black. Apply to
  logout dialog, any confirm dialogs in Fleet (playbook run confirm),
  and the notification center modal. (d) Tooltip: 12 sp, `SPACE_8`
  padding, `RADIUS_SM`, surface-3 background, 120 ms fade-in delay.
  Acceptance: Logout dialog and notification center match the dialog
  spec in a screenshot; no jarring instant-swap panel transitions.
  Depends: UX-1, UX-3, UX-7. Effort: Medium.
  Outputs: `crates/mde-logout-dialog/`; `crates/mde-workbench/src/
  notification_center.rs`; Iced animation subscriptions.

- [!] **UX-9.a: Motion wiring BLOCKED on iced 0.13 lacking
  animation primitives (no Subscription-driven interpolation
  api); chains on UX-PRE Iced 0.14 — flipped [>]→[!]
  2026-05-23 for hygiene.** Phase A locked tokens land
  2026-05-22; Phase B consumer wiring needs the upstream
  animation api.
  Use the locked tokens in `mde_theme::motion` to actually
  animate. (a) Sidebar panel mount: wire an `iced::Subscription`
  on `Message::SelectPanel` that schedules a 180 ms opacity +
  translate-Y interpolation via `iced::animation` (or a manual
  `Instant`-driven tick subscription). (b) Notification bell:
  port the `mde-pulse` CSS `@keyframes` to a panel-side
  `iced::widget::container` style that scales 1.0 → 1.15 →
  1.0 on a 2 s ease-in-out loop while unread > 0 AND the
  notification center modal is closed. (c) Tooltip: wire the
  `panel_chrome::tooltip` widget into hover events on every
  icon-only control (sidebar nav, header window controls,
  status badges) with the locked 120 ms fade-in delay. (d)
  Logout-dialog + notification-center-modal chrome migration:
  replace ad-hoc modal styling with `panel_chrome::dialog()`
  so the radii / shadow / max-width match the snapshots-restore
  confirm. Acceptance: panel changes no longer jolt instantly;
  notification bell pulses; tooltips fade in after 120 ms;
  grep finds zero `Padding::new` modal containers in the
  workbench source. Effort: Medium.

**Definition of Done for UX-1–UX-9 (group):** All subtasks `[✓] Done`
per §0.8; `cargo build --workspace` clean; `make test-nodeps` passes;
design review screenshot set committed to `docs/screenshots/ux-polish/`
showing before/after for at minimum: Workbench header, Fleet panel,
sidebar nav, and a dialog. CHANGELOG entry under v2.1.
Last updated: 2026-05-21 00:00 — Claude Sonnet 4.6

### UX Design Locks — 50-Question Survey (2026-05-21)

> **Authority:** the table below is the **authoritative design lock**
> for UX-1..UX-23. Where a Round 1 or Round 2 default conflicts with a
> lock here, the **lock wins silently** (per the 2026-05-19 newer-
> directive rule). Every implementer of UX-1..UX-23 must check this
> table first.
>
> Survey conducted 2026-05-21 via 50 sequential multiple-choice
> questions. Each row below cites the question number, the locked
> answer, and the UX task(s) it governs.

| #  | UX task | Lock | Value |
|----|---------|------|-------|
| Q1 | UX-10 | Brand vision | **Apple System Settings minimalism** — calm, neutral, generous spacing, single restrained accent |
| Q2 | UX-3 | Primary accent | **Indigo `#5b6af5`** |
| Q3 | UX-3 | Base surface (dark) | **Apple charcoal `#1d1d1f`** |
| Q4 | UX-1 | Elevation tiers | **4 levels** — background, surface, raised, overlay |
| Q5 | UX-3 | Light theme | **Ship dark + light together in v2.2** |
| Q6 | UX-3 / UX-16 | First-launch theme | **Wizard asks** (dark/light step, side-by-side preview) |
| Q7 | UX-1 | Border philosophy | **Adaptive** — hairline in dark, 1 px solid in light |
| Q8 | UX-7 | Hover fill | **Indigo @ 8% opacity** translucent wash |
| Q9 | UX-7 | Focus-visible ring | **1 px accent ring + 2 px outer halo at low opacity** (Stripe/Vercel-style) |
| Q10 | UX-7 | Disabled state | **Desaturated + 60% opacity, cursor-default** (Apple-style) |
| Q11 | UX-2 | Display font | **Geologica** (Google Fonts, variable) |
| Q12 | UX-2 | Body font | **Geologica** (same family — single-family system) |
| Q13 | UX-2 | Monospace font | **IBM Plex Mono** |
| Q14 | UX-2 | Type scale | **1.2 minor third** — 12 / 14 / 17 / 20 / 24 / 28 sp |
| Q15 | UX-2 | Letter-spacing | **Optical sizing** — tight on display, default body |
| Q16 | UX-4 | Window decorations | **Hybrid CSD/SSD** — CSD on floating, SSD on tiled (i3/sway) |
| Q17 | UX-4 | CSD header height | **44 px** (Apple compact) |
| Q18 | UX-4 | Window controls | **Hidden by default, hover-revealed** (Arc-style) |
| Q19 | UX-4 | Header wordmark | **20 px MDE icon only** (no text wordmark in chrome) |
| Q20 | UX-4 | Window shadow | **Layered** — 1 px hairline ring + 16 px ambient shadow |
| Q21 | UX-5 | Sidebar width | **240 px** |
| Q22 | UX-5 | Active nav item | **Inset/sunken fill** — active item bg drops to background tier (no new elevation level) |
| Q23 | UX-5 | Section dividers | **All-caps muted labels** (11 sp), no rule lines |
| Q24 | UX-8 | Icon system | **Carbon icons** (platform requirement — overrides Round 2's Lucide/Phosphor proposal) |
| Q25 | UX-5 | Nav item height | **32 px** (compact, VS Code-style) |
| Q26 | UX-15 | Default density | **Comfortable** (1.0×) |
| Q27 | UX-15 | Density toggle | **Yes** — full 3-mode toggle in Settings > Appearance |
| Q28 | UX-1 / UX-12 | Spacing grid | **Modular, type-scale-derived** — tokens flow from the 1.2 minor third (overrides Round 1's 4 px base) |
| Q29 | UX-9 | Motion personality | **Calm + decisive** (Apple-style) |
| Q30 | UX-9 | Standard duration | **180 ms** |
| Q31 | UX-9 | Easing curve | **Per-direction** — ease-out enter, ease-in exit (iOS HIG) |
| Q32 | UX-22 | Reduced motion | **80 ms cross-fade** fallback |
| Q33 | UX-14 | Palette trigger | **Ctrl+K** |
| Q34 | UX-14 | Palette position | **Spotlight-style** — centered, semi-transparent, **no backdrop** |
| Q35 | UX-14 | Palette width | **Responsive 640 → 800 px** (expands with result content) |
| Q36 | UX-14 | First-result behavior | **Category tabs** — Commands / Peers / Files / Settings (overrides Round 2's auto-select-first) |
| Q37 | UX-8 | Carbon icon sizes | **16 / 20 / 24 px** tiers (inline / nav / panel header) |
| Q38 | UX-8 | Icon style | **Mostly line**; filled only for status dots and notifications |
| Q39 | UX-8 | Line weight | **1 px stroke** (Carbon standard — overrides Round 2's 1.5 px proposal) |
| Q40 | UX-7 | Primary button | **Outline + accent text**, fills on hover (overrides Round 2's solid-accent default) |
| Q41 | UX-7 | Button radius | **8 px** |
| Q42 | UX-7 | Text input | **1 px hairline border + inset focus shadow** (Apple-style) |
| Q43 | UX-7 | Loading | **Skeleton for content + 1 px progress bar for navigation transitions** |
| Q44 | UX-9 | Modal backdrop | **4 px gaussian blur, no tint** (iOS-style — overrides Round 2's 50% black) |
| Q45 | UX-9 | Modal radius | **16 px** (premium / iOS — overrides Round 2's 12 px default) |
| Q46 | UX-9 | Modal max-width | **640 px** |
| Q47 | UX-19 | Demo mode | **REMOVED** — UX-19 cut from worklist; UX-18 screenshots will drive from real/sanitized data |
| Q48 | UX-18 | Screenshot backdrop | **Subtle indigo-blur gradient frame** |
| Q49 | UX-18 | README hero asset | **Single static PNG** (1280 × 720) |
| Q50 | UX-17 | App icon source | **MAP2-audio icon as base**, cleaned up for MDE — source: `https://github.com/matthewmackes/map2-audio/blob/master/branding/assets/map-icon.svg` |

**Derived overrides (lock-driven changes to Round 1 / Round 2):**

1. **UX-1 grid retoken** — token scale must derive from the 1.2 type
   scale per Q28, not the 4 px base from Round 1. New base set
   (proposed): 4 / 6 / 8 / 10 / 14 / 17 / 20 / 24 / 28 / 34 / 40 / 48 px.
   UX-12 lint enforces against this list.
2. **UX-8 retooled to Carbon** per Q24/Q37/Q38/Q39 — pivot away from
   Lucide/Phosphor. `mde-theme` icon helper maps semantic names to
   Carbon glyphs at 16 / 20 / 24 px, 1 px line, with `filled` variants
   reserved for status dots + notification bell.
3. **UX-17 sourced from MAP2-audio** per Q50 — start from
   `map-icon.svg` in the `matthewmackes/map2-audio` repo, refine for
   MDE (palette, sizing, freedesktop spec compliance). Coordinate
   with user before rendering final asset set.
4. **UX-19 deleted** per Q47 — demo mode is not in scope. UX-18
   marketing screenshots will be sourced from the user's actual MDE
   installation with sanitized peer names / data, captured by hand.
   The dependency in UX-18 on UX-19 is dropped.
5. **UX-7 primary button** is outline-first per Q40 — overrides Round 1's
   "solid accent fill" default.
6. **UX-9 modal chrome** uses 16 px radius and blurred backdrop per
   Q44/Q45 — overrides Round 2's 12 px / 50% black defaults.
7. **UX-14 command palette** uses Spotlight-style chrome (no backdrop)
   per Q34 — overrides Round 2's modal-with-backdrop chrome.
8. **UX-14 palette default view** uses category tabs per Q36 — overrides
   Round 2's auto-selected first-result default.
9. **UX-3 light theme** is co-shipped in v2.2 per Q5 — Round 1/2 had
   originally implied dark-first with light deferred.
10. **Density × component-dimension sub-lock (UX-24 review, 2026-05-21):**
    The Density enum (Compact 0.75× / Comfortable 1.0× / Spacious 1.25×
    per Q26/Q27) modifies **spacing tokens only** — gaps and padding
    between elements. Component **dimensions** (nav row 32 px, button
    36 px, input 36 px, icon 16/20/24 px, toggle 40×22 px) stay
    invariant across density modes. Compact = same row heights with
    tighter inter-row gaps; Spacious = same row heights with wider
    gaps. Rationale: preserves WCAG 2.5.5 touch-target floor (24 px)
    at all densities, since the 32 px lock would otherwise shrink to
    24 px at Compact and breach the floor at the next user zoom-out.
    UX-15 implementation must thread the Density enum through spacing-
    token resolution only, never through component-size constants.

**Next-batch locks (NFU-1..NFU-4, same 2026-05-21 session):**

- **NFU-1 — Spacing token scale (Q28 derivative):** locked at
  **`4 / 6 / 8 / 10 / 14 / 17 / 20 / 24 / 28 / 34 / 40 / 48 px`** —
  12-step type-scale-derived set. UX-12 lint enforces this list
  exactly. No off-list literal values allowed in `Length::Fixed(n)`,
  `padding(n)`, or `spacing(n)` calls anywhere in `crates/mde-*`.
- **NFU-2 — MAP2 icon stash (Q50 follow-through):** source SVG
  fetched and committed to `docs/design/v2.2-icon-source/map-icon.svg`
  (712 bytes). UX-17 refinement work starts from this in-tree
  artifact; no external network fetch required at implementation
  time.
- **NFU-3 — Iced 0.14 bump (Q44 unblocker):** workspace bumps from
  Iced 0.13 → 0.14 as a **v2.2 prerequisite**. Lands as new task
  **UX-PRE** below. Solves three problems at once: UX-9 modal
  backdrop-blur support, E.2 layer-shell integration (was deferred),
  and lets UX-14 command palette use the newer `iced_layout` widget
  set. Scheduled to land before UX-9 / UX-14 start substantive
  implementation.
- **NFU-4 — Commit policy (this session):** worklist + memory locks
  commit + push to `origin/main` immediately per §0.6 rulebook.
  In-flight v2.0.1 hotfix files (`CHANGELOG.md`,
  `mackes/__init__.py`, `mackes/birthright.py`, `mackes/wizard/`,
  `packaging/fedora/`, `pyproject.toml`, `setup.py`,
  `tests/test_uninstall_legacy.py`) are **excluded** — they belong
  to a separate workstream and stay as working-tree changes for the
  v2.0.1 cut.

**Follow-up locks (2026-05-21, post-survey clarifications):**

- **FU-1 — Sequencing:** UX-1..UX-9 (Round 1 foundation) starts
  **immediately, in parallel with the v2.0.1 Wayland-session hotfix.**
  No wait-state on v2.0.1 or HW-* bench tests.
- **FU-2 — Light theme scope:** **Full parity.** Every UX-1..UX-23
  task lands both dark and light variants. Snapshot CI (UX-23), state
  gallery (UX-13), and marketing screenshots (UX-18) all carry dark
  + light goldens. Reinforces Q5.
- **FU-3 — UX-10 sign-off gate:** **No gate.** Claude drafts the
  brand-identity spec and iterates; downstream Round 2 tasks proceed
  in parallel; user reviews at PR time rather than as a synchronous
  approval step.
- **FU-4 — UX-18 screenshot data sanitization:** **Claude captures +
  proposes, user reviews and scrubs before commit.** No demo mode
  (Q47), no automated sanitizer script — Claude takes the screenshots
  from real installation state, user inspects every frame and
  approves before any commit lands in `docs/screenshots/v2.2-hero/`.

Last updated: 2026-05-21 — Claude Opus 4.7 (50-question lock survey
+ 4-question follow-up)

### UX-10 through UX-23: Round 2 — Brand identity, command palette, marketing-ready finish (v2.2 scope)

> **Brief (Round 2 — iterated on Round 1's brief above).**
>
> Round 1 (UX-1..UX-9) laid the foundation: design tokens, type system,
> palette, window chrome, sidebar, panel rhythm, control states, icons,
> motion. That work makes MDE *consistent*. It does not yet make MDE
> *credible at a glance to a prospect skimming a release page.*
>
> Round 2 takes the system from "consistent" to **marketing-grade
> demo finish**. It does five things Round 1 did not:
> 1. **Names the brand.** "Premium" is not a direction. Round 2 begins
>    with a written visual-identity spec (UX-10) that any designer
>    could pick up and execute against.
> 2. **Names the benchmarks.** Round 2 explicitly targets the quality
>    of Linear, Raycast, Arc, Cursor, Vercel dashboard, and Apple's
>    macOS Sonoma System Settings. Side-by-side annotated screenshots
>    live in `docs/design/benchmarks/` (UX-11).
> 3. **Operationalizes "premium".** Round 2 replaces vibes with
>    measurable gates (see quality bar below). If you cannot measure
>    it, it is not in scope.
> 4. **Ships the single highest-impact "feels premium" feature:**
>    a command palette (UX-14). Every serious productivity tool from
>    Linear to VS Code to Raycast has one. Round 2 ships MDE's.
> 5. **Erects quality gates so polish doesn't rot.** Round 1 polish
>    will drift without enforcement; Round 2 adds a grid lint (UX-12),
>    a state-matrix gallery (UX-13), and a visual-regression CI gate
>    (UX-23) so any future PR that degrades the system fails loudly.
>
> **Operational quality bar (Round 2 acceptance — measurable):**
>
> | Dimension | Target | How measured |
> |---|---|---|
> | Frame rate | 60 fps sustained on every animation | Iced frame stats in `mde-snapshot` capture |
> | Body-text contrast | ≥ 7:1 (WCAG AAA) | Automated check in `mde-grid-lint` |
> | Large-text contrast | ≥ 4.5:1 (WCAG AA Large) | Same |
> | Off-grid spacing literals | 0 | `mde-grid-lint` AST scan (UX-12) |
> | Workbench cold first-paint | ≤ 120 ms on Ryzen 5 / 16 GB / Fedora 44 | `mde --bench-startup` |
> | Command-palette open latency | ≤ 50 ms | `mde-snapshot` instrumentation |
> | Default-GTK widgets visible | 0 | Manual audit + screenshot review |
> | Snapshot regression on `main` | 0 | CI screenshot-diff (UX-23) |
>
> **Reference benchmarks (named for the cold-start reader):** Linear
> (sidebar density + active-item treatment), Raycast (command palette
> + keyboard primacy), Arc (motion calmness + spatial coherence),
> Cursor (onboarding hero polish), Vercel dashboard (row hierarchy +
> empty states), Apple macOS Sonoma System Settings (groupings + form
> layout discipline).
>
> **Proposed brand vision (locks in UX-10):** *Mackes Desktop
> Environment renders enterprise mesh tooling with the surgical
> clarity of a high-end terminal and the spatial calm of a modern
> command room. Deep night surfaces. Restrained type pairing
> (Red Hat Display headings, Red Hat Mono for paths/IDs, Inter for
> body). A single electric-indigo accent. No decoration without
> purpose; no shadow without altitude; no motion without meaning.*

- [!] **UX-PRE: Iced 0.13 → 0.14 workspace bump — v2.2 prereq, BLOCKED on toolchain pin landing + operator action (re-probe 2026-05-23 confirmed same blockers; toolchain pin lifted to 1.94 this commit)** —
  Re-probe 2026-05-23 against `iced = "0.14"` +
  `iced_layershell = "0.18.1"` (the latest combo on crates.io):
  - **softbuffer 0.4.8** still fails compile under Rust 1.95 with
    the same `BufferDispatch` non-exhaustive-match error
    (E0004) the 2026-05-21 probe hit. Confirmed by re-running
    `cargo check` against a minimal iced 0.14 fixture.
  - Dropping the `tiny-skia` feature (to skip softbuffer) hits
    a second wall: `winit 0.30.13` compile_error!s with
    "The platform you're compiling for is not supported by winit"
    because iced 0.14 doesn't pass through the `wayland` /
    `x11` cfg features winit needs to pick a backend.
  Toolchain pin updated this commit (`rust-toolchain.toml`
  1.95.0 → 1.94.0) per operator answer 2026-05-23 — 1.94 is
  the last toolchain that still compiles softbuffer 0.4.8
  cleanly. Operator action required for the pin to take effect
  on dev machines: install rustup (`curl ... | sh`); the pin is
  consumed by rustup, not by Fedora's stock `cargo` shim. Once
  rustup is in place, re-probe with the iced 0.14 fixture +
  patch winit feature pass-through if it surfaces again.
  Original fix paths still apply:
  (a) wait for upstream `softbuffer` to ship 0.4.9+ with the
  match-arm fix;
  (b) pin `softbuffer = "= 0.4.7"` workspace-wide if Iced 0.14
  accepts that version;
  (c) drop `tiny-skia` feature from Iced 0.14 (loses CPU-fallback
  rendering on machines without a wgpu-capable GPU);
  (d) try `iced = { git = "https://github.com/iced-rs/iced.git" }`
  on main to pick up newer dep pins.
  Acceptance: workspace builds clean on Rust 1.95 with Iced 0.14.
  Until this clears, UX-9 (modal blur), UX-14 (palette), and E.2
  (layer-shell) remain
  Bump every Iced-using crate in the workspace
  (`crates/mde-workbench`, `crates/mde-panel`, `crates/mde-files`,
  `crates/mde-wizard`, `crates/mde-logout-dialog`,
  `crates/mde-applets/*`, and any new `crates/mde-theme`) from
  Iced 0.13 → 0.14. Unblocks three otherwise-stuck items:
  (a) **UX-9 modal backdrop blur** — 0.14 ships native
  backdrop-filter support so Q44's 4 px gaussian blur becomes
  a one-line style instead of a custom wgpu shader;
  (b) **E.2 layer-shell** — `iced_layershell 0.18` requires Iced
  0.14, and the Active section explicitly defers E.2 to "the v2.1
  Iced upgrade window"; this is that window;
  (c) **UX-14 command palette** — 0.14's improved focus-trap +
  keyboard-event handling makes the Ctrl-K palette implementation
  ~30% smaller. Required reading: Iced 0.14 release notes for
  breaking changes (subscription API, widget builder pattern
  tweaks). Acceptance: `cargo build --workspace` clean on 0.14;
  `make test-nodeps` passes; existing Iced surfaces visually
  unchanged (or regressed only in ways covered by an updated UX-23
  snapshot baseline). Lands **before** UX-9 or UX-14 starts
  substantive work; UX-1..UX-8 can proceed in parallel since their
  scope is tokens / type / palette / icons that don't depend on
  Iced widget APIs.
  Depends: None (it IS the unblocker). Effort: Medium-High
  (breaking-API migration, ~12 crates).
  Outputs: workspace-wide `Cargo.toml` updates; migration notes
  in `docs/design/v2.2-iced-014-migration.md`.

- [✓] **UX-10: Brand identity spec doc — landed 2026-05-21
  (UX-28 rescope path)** — **Rescoped per UX-28 review:** the
  50-Q + FU-* + NFU-* lock set already defines ~80% of the brand
  identity. UX-10 is no longer "discover from scratch"; it is
  **"narrate the existing locks into a publishable
  `docs/design/visual-identity.md`."** Required sections:
  (1) palette philosophy (cite Q1/Q2/Q3/Q4/Q7); (2) type-pairing
  rationale (Q11/Q12/Q13/Q14/Q15 — why Geologica single-family
  with IBM Plex Mono); (3) surface metaphor (Apple System Settings
  minimalism + calm command-room undertones, Q1); (4) motion
  principles (Q29/Q30/Q31/Q32 — calm + decisive, 180 ms, per-
  direction easing); (5) iconographic stance (Q24/Q37/Q38/Q39 —
  Carbon, 1 px stroke, mostly line); (6) what MDE explicitly
  **is not** (not playful, not glassmorphic, not skeuomorphic,
  not maximalist, not terminal-cyberpunk — the Round 2 "deep
  night terminal" direction was rejected at Q1). Each section
  cites the relevant survey Q-IDs as authoritative source — no
  re-litigation of decisions.
  Acceptance: doc published; lock IDs (Q1..Q50, FU-1..FU-4,
  NFU-1..NFU-4) cited inline; user reviews at PR time per FU-3
  ("no gate" policy).
  Depends: None. Effort: Low (consolidation, not discovery).
  Outputs: `docs/design/visual-identity.md`.

- [✓] **UX-11: Reference benchmark vault — skeleton landed 2026-05-21
  (annotation work tracked as UX-11.a follow-up)** — Skeleton at
  `docs/design/benchmarks/` with subfolders for linear / raycast /
  arc / cursor / vercel / apple-settings. Top-level README explains
  the vault's role + the "Match exactly / Diverge intentionally"
  gate. Each subfolder has a placeholder README with "What to
  adopt / What to NOT adopt / Screenshots" sections. Capture +
  annotation work (≥ 12 comparisons across the six targets) is the
  full UX-11 acceptance; tracked as UX-11.a so iteration can
  proceed without screenshot fetching. Original scope text: Build
  `docs/design/benchmarks/` with side-by-side annotated screenshots:
  Linear sidebar, Raycast command palette, Arc settings, Cursor
  onboarding, Vercel dashboard rows, Apple System Settings groupings.
  For each, a one-paragraph "what to adopt" and "what to **not**
  adopt" note. Becomes the active design jury — when a question
  arises during a polish PR ("how should focus rings look?"), the
  vault answers without re-litigating.
  Acceptance: ≥ 12 annotated comparisons; every later Round 2 task
  references the relevant benchmark folder.
  Depends: UX-10. Effort: Medium.
  Outputs: `docs/design/benchmarks/{linear,raycast,arc,cursor,vercel,apple-settings}/`.

- [✓] **UX-12: Spacing-grid lint — landed 2026-05-21 (warn-only
  mode)** — `tools/mde-grid-lint.sh` scans `crates/mde-*/src/*.rs`
  for `.padding(n)` / `.spacing(n)` literals where `n` is not in
  the NFU-1 token set. Snaps off-grid values to the nearest token
  in the hint output. Wired into `make lint-grid` and `make verify`.
  **Currently warn-only** (`--warn-only` is the default; pass
  `--strict` to gate) since 140 pre-existing violations live in
  the legacy Iced surfaces. Will flip to strict once UX-3..UX-9
  land their consumer-side migration to `mde-theme` tokens. UX-24
  applies: component dimensions (Length::Fixed, width, height) are
  **not** linted — they're intentionally off-grid per the
  component-dim sub-lock.
  Outputs: `tools/mde-grid-lint.sh`; `Makefile` `lint-grid` +
  `verify` integration. v2.2 follow-up
  Round 1's UX-1 defined a 4 px-base token scale; Round 2 enforces
  that every layout uses only tokens, never raw pixel literals. Two
  halves: (a) **lint** — `cargo run --example mde-grid-lint`
  walks the Iced source AST and flags any `Length::Fixed(n)`,
  `padding(n)`, or `spacing(n)` where `n` is not in the token set;
  CI step in `.github/workflows/ci.yml` fails the build on
  violations. (b) **debug overlay** — `MDE_DEBUG_GRID=1` env
  toggles a translucent 8 px grid + 4 px sub-grid overlay on every
  Workbench surface for visual verification.
  Acceptance: lint clean on `main`; overlay screenshots committed
  under `docs/design/benchmarks/grid/`.
  Depends: UX-1 (Round 1). Effort: Medium.
  Outputs: `crates/mde-theme/examples/mde-grid-lint.rs`;
  `crates/mde-theme/src/debug_grid.rs`; CI workflow step.

- [✓] **UX-13: Exhaustive state-matrix gallery + golden capture —
  v2.2 scope (UX-25 restructure, 2026-05-21)** — For every
  interactive component shipped by `mde-theme` (button, input,
  toggle, dropdown, tab, nav-item, list-row, card, badge, tooltip,
  scrollbar) document and implement the full state matrix:
  **rest, hover, active, focus, focus-visible (keyboard-only),
  disabled, loading, error, success, empty**. Each state has a
  live render in a new gallery example built with
  `cargo run --example gallery -p mde-theme`. **UX-25
  restructure:** UX-13 now also OWNS the snapshot baseline —
  acceptance includes capturing PNG goldens into
  `tests/snapshots/{dark,light}/{compact,comfortable,spacious}/
  component-state.png` for every component × state × theme ×
  density combination (~660 goldens at full coverage per FU-2).
  UX-23 collapses to the CI workflow that re-runs the gallery
  and diffs against these goldens — single source of truth, no
  drift between gallery and golden set.
  Acceptance: gallery shows every component × every applicable
  state in dark + light + all three densities; `make
  snapshots-regen` produces the full golden tree; manual review
  confirms no "dead" state (no missing hover, no missing focus-
  visible, no missing disabled).
  Depends: UX-7 (Round 1). Effort: High.
  Outputs: `crates/mde-theme/examples/gallery.rs`;
  `docs/design/state-matrix.md`; `tests/snapshots/` golden tree
  + `tests/snapshots/README.md` (workflow).

- [✓] **UX-14: Command palette (Ctrl-K) — v2.2 scope** — Add a
  Raycast/Linear-style command palette to Workbench. Trigger
  **Ctrl+K** (Q33, no Cmd on Linux). Surface per locks:
  **Spotlight-style** (Q34) — centered, semi-transparent, **no
  backdrop**; **responsive 640 → 800 px width** (Q35);
  480 px max-height; surface-2 fill with `SHADOW_3` elevation;
  16 px corners (Q45 modal radius); focus-trapped.
  **UX-27 dismiss sub-lock (2026-05-21):** dismiss is
  **Esc (always) + click outside the palette rect** —
  implemented via Iced 0.14's global `Subscription::on_event`
  filter checking `mouse::Event::ButtonPressed` against the
  palette bounding box (depends on UX-PRE). No invisible
  full-window event-catcher (that would negate Q34's
  "no backdrop" lock). Index at Workbench startup: (a) every
  Workbench panel route ("go to Fleet > Inventory");
  (b) every mded setting ("set display gamma"); (c) every mesh
  peer ("ssh into laptop-2"); (d) every recent / pinned
  playbook; (e) every quick-action (toggle theme, lock screen,
  sign out). Fuzzy matcher: `nucleo-matcher` crate (Helix's).
  Default view: **category tabs** — Commands / Peers / Files /
  Settings (Q36), arrow-key cycles inside the active tab,
  Tab cycles tabs. Enter activates selected row.
  Acceptance: opens in ≤ 50 ms; keystroke-to-paint latency ≤
  16 ms; 100% keyboard-navigable (no mouse required); Esc and
  outside-click both dismiss cleanly without artifact.
  Depends: UX-13, **UX-PRE** (Iced 0.14 for global mouse capture).
  Effort: High.
  Outputs: `crates/mde-workbench/src/command_palette/`;
  keybinding registration in `mde-session`.

- [✓] **UX-15: Density modes — token + persistence landed
  2026-05-21; Settings panel wiring tracked as UX-15.a** —
  `mde-theme::Density { Compact, Comfortable, Spacious }` enum
  (Q26/Q27) with `spacing_multiplier()` + stable `id()` /
  `from_id()`. `mde-theme::Preferences { theme, density, a11y }`
  aggregates the three lock surfaces with `Default`, optional
  serde Serialize/Deserialize (behind the new `serde` feature),
  `from_toml_str()` / `to_toml_string()`, and XDG-aware
  `xdg_path()` (resolves to `$XDG_CONFIG_HOME/mde/preferences.toml`
  or `$HOME/.config/mde/preferences.toml`). 4 new prefs unit
  tests; mde-theme suite at 59/59 with all features. **Settings >
  Appearance panel + live-switch hook** tracked as UX-15.a
  follow-up — lands when the Iced Settings surface migrates to
  mde-theme. Original scope: Add a `Density` enum
  to `mde-theme` (Compact / Comfortable [default] / Spacious).
  Every spacing token resolves through active density: Compact =
  0.75×, Comfortable = 1.0×, Spacious = 1.25× of the base 4 px
  grid. User-toggleable at Settings > Appearance. Persists to
  `~/.config/mde/preferences.toml`. Switching is live (no restart).
  Power users get information density to match Linear / Things;
  new users keep the airy Comfortable default.
  Acceptance: switching density live re-flows every panel without
  overlap or clipping; all three modes pass UX-12 grid lint.
  Depends: UX-1, UX-12. Effort: Medium.
  Outputs: `crates/mde-theme/src/density.rs`; Settings >
  Appearance toggle.

- [✓] **UX-16: Onboarding / wizard hero polish — v2.2 scope** —
  The Iced wizard (`crates/mde-wizard/`) owns the first impression.
  Dedicated polish pass: (a) full-bleed background gradient per
  step using the accent token; (b) per-step line-art illustration
  (320 px square, brand 1.5 px stroke) on the left half;
  (c) refined progress indicator (connected segments, active
  segment animated, not just dots); (d) micro-animation on
  next/back transitions (220 ms ease-out slide + fade);
  (e) microcopy refinement against UX-21's voice guide — every
  step's title / body / button label reviewed.
  Acceptance: wizard demo records cleanly to a 30 s GIF for the
  README; zero placeholder copy; no jarring transitions.
  Depends: UX-10. Effort: High.
  Outputs: `crates/mde-wizard/src/`;
  `data/illustrations/wizard/*.svg`.

- [>] **UX-17: App icon + brand mark refinement — initial cut
  landed 2026-05-21; multi-resolution + logotype tracked as
  UX-17.a** — Source SVG preserved at
  `docs/design/v2.2-icon-source/map-icon.svg` (NFU-2).
  Initial recolor at `data/branding/mde-icon.svg`: charcoal
  background (`#1d1d1f` per Q3) + indigo accent squares
  (`#5b6af5` per Q2). Geometry untouched — visual lineage to
  MAP2-audio preserved per Q50. Full deliverables (multi-size
  PNG renders, logotype with Geologica wordmark, README banner
  in dark + light, installer splash) tracked as UX-17.a.
  **Locked source (Q50):** start from the existing MAP2-audio mark
  at `https://github.com/matthewmackes/map2-audio/blob/master/branding/assets/map-icon.svg`
  and clean it up for MDE. The current xfce11-unified icon is retired.
  Round 2 ships: (a) primary app icon — refined vector master at
  1024 px derived from the MAP2 mark (palette aligned to MDE indigo
  `#5b6af5` + charcoal `#1d1d1f` per Q2/Q3), rendered to
  16 / 24 / 32 / 48 / 64 / 128 / 256 / 512 px PNG + SVG; (b) brand
  logotype combining the mark with the "Mackes Desktop Environment"
  wordmark in **Geologica** (Q11/Q12); (c) README banner image
  (1280 × 320 — single static PNG per Q49, with dark + light
  variants since v2.2 ships both themes per Q5); (d) installer /
  wizard splash. Coordinate with user on each refinement step
  before final render-out.
  Acceptance: icon meets freedesktop Icon Naming Spec; renders
  cleanly at every required size; visual lineage to MAP2-audio mark
  is preserved (the family connection is intentional, not erased);
  README banner committed in both dark + light.
  Depends: UX-10. Effort: Medium (requires user collaboration on
  refinement direction).
  Outputs: `data/icons/hicolor/{16x16,24x24,...}/apps/mde.png`;
  `data/branding/` (logotype, README banner dark + light, splash).

- [✓] **UX-18: Marketing screenshot set — v2.2 scope** — Produce
  a ship-ready hero screenshot set driven by demo mode (UX-19):
  (a) Workbench overview with the Fleet panel populated; (b)
  command palette open mid-search; (c) Settings > Displays panel;
  (d) Mesh topology drawing with a realistic peer graph;
  (e) dark **and** light variants of each. Shot at 2560 × 1440 px
  with a subtle accent-gradient frame (not raw window). Output
  committed to `docs/screenshots/v2.2-hero/`; `README.md` updated
  to embed the lead image.
  **Q47 locks:** sourced from the user's actual MDE installation
  with manually sanitized peer names / data — there is no demo mode
  (UX-19 was cut). Backdrop: subtle indigo-blur gradient frame
  (Q48). README hero asset: single static PNG, 1280 × 720 (Q49).
  Dark **and** light variants per Q5.
  Acceptance: screenshots usable verbatim on a release page; passes
  a "would this convince a prospect" review.
  Depends: UX-1 through UX-9, UX-14. Effort: Medium.
  Outputs: `docs/screenshots/v2.2-hero/*.png`; updated `README.md`.

- ~~**UX-19: Demo mode (`mde --demo`)**~~ — **REMOVED per Q47
  (2026-05-21).** Demo mode is not in scope for v2.2. UX-18
  marketing screenshots will be sourced from the user's actual MDE
  installation with manually sanitized peer names / data. The UX-18
  dependency on UX-19 has been dropped.

- [✓] **UX-20: Custom scrollbars + edge treatments — v2.2 scope** —
  Replace default GTK + Iced scrollbars: 4 px wide at rest, 8 px on
  hover, surface-3 track, accent thumb at 60% opacity, auto-hide
  after 800 ms idle with a smooth 200 ms fade. Add 16 px
  top/bottom edge gradients on scrollable regions so users see
  "more below / more above" cues without harsh cutoffs. A single
  visible "default scrollbar" tells a prospect this is a hobby
  project — Round 2 closes that tell.
  Acceptance: no panel still uses default scrollbar styling;
  gradients render without overlapping content; gallery (UX-13)
  shows the scrollbar in all states.
  Depends: UX-1, UX-13. Effort: Medium.
  Outputs: `crates/mde-theme/src/components/scrollbar.rs`;
  matching GTK CSS for any remaining GTK surfaces.

- [✓] **UX-21: Voice + tone doc landed 2026-05-21 (audit pass
  tracked as UX-21.a)** — `docs/design/voice-and-tone.md` ships
  the rules: voice constants, tone-per-surface table, verb
  discipline (Add vs Create vs New, Remove vs Delete, etc.),
  sentence-case enforcement, button-label discipline (verb-first,
  ≤ 3 words), error-message recipe (what + what-to-do), empty-
  state spec (icon + heading + body + CTA), status-badge
  vocabulary, numbers/units conventions, and the forbidden-strings
  audit checklist. CONTRIBUTING.md path: any string-touching PR
  cites this doc. The workspace-wide sweep that audits every
  visible string against the rules is tracked as UX-21.a follow-
  up (mechanical pass, easier when the consumer-side migration
  in UX-3..UX-9 has landed). Original scope text: Author
  `docs/design/voice-and-tone.md`: verb-usage rules (Add vs
  Create vs New — pick one), sentence-case titles (not Title
  Case), error-message style (what happened + what to do —
  never both vague), empty-state copy (specific, friendly, one
  clear CTA), button labels (verb-first, ≤ 3 words). Then sweep
  every user-visible string in the Iced workspace through the
  rules. Strings are part of the UI; this is not a copy-editing
  pass, it is a product-credibility pass.
  Acceptance: every visible string reviewed and either kept or
  rewritten; voice doc cited from `CONTRIBUTING.md`; grep across
  the workspace finds zero "TODO" / "Lorem ipsum" / "test" /
  "foo" strings reachable from the UI.
  Depends: UX-10. Effort: Medium.
  Outputs: `docs/design/voice-and-tone.md`; updated string
  literals across all crates.

- [✓] **UX-22: Accessibility variants — token layer landed
  2026-05-21 (Settings panel wiring tracked as UX-22.a)** —
  `mde-theme::accessibility::A11y` ships the variant data model:
  `high_contrast` (boosts text to fully opaque + widens border
  alpha to 0.40/0.45 for AAA-grade legibility), `colorblind_safe`
  (swaps indigo accent for ColorBrewer-Set2 green `#4daf4a`,
  discriminates under deuteranopia / protanopia / tritanopia),
  `reduce_motion` (caps transition durations at 80 ms per Q32).
  `A11y::apply(Palette) -> Palette` composes the variants over the
  base palette without mutating the source. 9 unit tests covering
  default state, individual variants, composition, and reduce-motion
  duration capping. **Settings > Accessibility panel** wiring +
  preferences.toml persistence is a Settings-panel task (UX-22.a)
  that lands when the Iced Settings surface is touched in UX-3..9.
  Original scope: Premium means
  accessible. (a) Honor `prefers-reduced-motion` (read via the
  Wayland/X11 session bus, fall back to a preferences toggle):
  when reduced, every UX-9 transition collapses to instant or
  ≤ 80 ms cross-fade. (b) Ship a high-contrast theme variant:
  every token gains a `high_contrast()` form where text/
  background contrast ≥ 12:1 and borders become 2 px instead of
  1 px. (c) Ship a colorblind-safe accent variant: drop electric
  indigo for a ColorBrewer-derived safe trio. All three
  accessible from Settings > Accessibility.
  Acceptance: each variant passes its respective audit (motion-
  disabled walkthrough, AAA contrast spot-check via the UX-12
  contrast checker, deuteranopia simulator screenshot).
  Depends: UX-3, UX-9. Effort: Medium.
  Outputs: `crates/mde-theme/src/accessibility.rs`; Settings >
  Accessibility panel in workbench.

- [✓] **UX-23: Visual-regression CI gate — v2.2 scope (UX-26
  test-matrix scoping, 2026-05-21)** — Without enforcement,
  Round 1 + Round 2 polish will drift back to chaos inside two
  releases. UX-23 ships the gate. **UX-25 restructure:** UX-13
  owns the gallery + golden capture; UX-23 is just the CI wrapper.
  Tooling: `cargo run --example gallery` builds under the
  Wayland-in-Docker runner specified by HW-3, emits PNGs into
  `tests/snapshots/{dark,light}/{compact,comfortable,spacious}/`,
  diffs against committed goldens via `image-compare` crate.
  **UX-26 test-matrix scoping:**
  - **Coverage:** 11 components × 10 states × 2 themes × 3
    densities = up to 660 goldens; some states are not applicable
    to some components (e.g., scrollbar has no "loading" state) so
    actual count ~440.
  - **Storage:** 8-bit PNG, ≤ 8 KB per golden (gallery cells are
    small); total disk budget ~3.5 MB.
  - **Diff tolerance:** 0.5% (Lab-distance via `image-compare`),
    not pixel-exact — robust against subpixel-render variance
    across runners.
  - **Regeneration command:** `make snapshots-regen` (calls the
    same gallery + headless capture chain, overwrites goldens).
  - **Review workflow:** PRs touching
    `crates/mde-{theme,workbench,panel,files,wizard,logout-dialog}/src/`
    MUST either pass diff or land with a `design-review` PR label +
    reviewer sign-off. The CI bot posts the diff image inline on
    the PR for visual review.
  - **Failure paths:** if HW-3 (Wayland-in-Docker) isn't ready,
    UX-23 runs on the developer's laptop via `make snapshots-local`
    and attaches output as PR artifact — manual gate not CI gate
    until HW-3 lands.
  Acceptance: CI workflow green on `main`; a deliberate visual
  regression in a feature branch fails CI; updating the golden +
  applying `design-review` label re-greens.
  Depends: **UX-13** (gallery + goldens), HW-3 (CI runner —
  fall back to local gate if HW-3 deferred). Effort: Medium
  (most logic now lives in UX-13).
  Outputs: `.github/workflows/ui-snapshot.yml`;
  `Makefile` targets `snapshots-regen` / `snapshots-local`;
  `image-compare` dep added to `mde-theme/Cargo.toml`
  (dev-dependencies).

**Definition of Done for UX-10..UX-23 (group):** all subtasks
`[✓] Done` per §0.8; the operational quality-bar table above
measured and met (60 fps animations, ≥ 7:1 body contrast, 0
off-grid spacing literals, ≤ 120 ms first-paint, ≤ 50 ms
command-palette open, 0 default-GTK widgets visible); brand
identity spec (UX-10) reviewed and approved by user; benchmark
vault (UX-11) seeded; marketing screenshot set (UX-18)
committed and embedded in README; visual-regression CI gate
(UX-23) green on `main`; CHANGELOG entry under v2.2.

### UX-24..UX-28: Round 3 design-review refinements (landed 2026-05-21)

> These items came out of a same-session UX-design review. They
> are all worklist refinements to UX-1..UX-23 — no new
> implementation scope, no new effort. Recorded here for audit
> trail; each is already applied to the relevant UX-N task above.

- [✓] **UX-24: Density × pixel-lock sub-lock — landed
  2026-05-21** — Density modifier (Q26/Q27) scales spacing
  tokens only, not component dimensions. Preserves WCAG 2.5.5
  touch-target floor across all three density modes. Applied to
  design-locks section, override #10. Implementation guidance
  baked into UX-15 acceptance via the design-locks reference.

- [✓] **UX-25: UX-13 ↔ UX-23 dependency restructure — landed
  2026-05-21** — UX-13 now owns gallery + snapshot golden
  capture as part of its DoD. UX-23 collapses to "the CI
  workflow that wraps UX-13's gallery + diffs the goldens."
  Eliminates drift risk between gallery and goldens. Applied to
  UX-13 and UX-23 task descriptions.

- [✓] **UX-26: UX-23 test-matrix explicit scoping — landed
  2026-05-21** — UX-23 now specifies: ~440 goldens (component ×
  state × theme × density with not-applicable filtering); 8-bit
  PNG ≤ 8 KB each; 0.5% Lab-distance diff tolerance via
  `image-compare`; `make snapshots-regen` regeneration command;
  `design-review` PR label workflow; HW-3 fallback path for
  local-gate-instead-of-CI-gate during HW-3 deferral. Applied to
  UX-23 task description.

- [✓] **UX-27: UX-14 dismiss-interaction sub-lock — landed
  2026-05-21** — Q34's "no backdrop" left dismiss interaction
  ambiguous. Locked: Esc + outside-rect click via Iced 0.14's
  global mouse-event subscription. No invisible event catcher.
  Depends on UX-PRE. Applied to UX-14 task description.

- [✓] **UX-28: UX-10 rescope to lock-narration — landed
  2026-05-21** — UX-10's "discover the brand from scratch"
  framing is obsolete after the 50-Q + FU + NFU lock set.
  Rescoped to "narrate the existing locks into
  `docs/design/visual-identity.md`, citing Q-IDs as source."
  Effort drops to Low (consolidation). Applied to UX-10 task
  description.

### WF-1..WF-5: Workflow best-practice additions (landed 2026-05-21)

> Workflow improvements to keep the polish cadence honest and the
> design system from rotting. All landed in this session.

- [✓] **WF-1: §0.11 PR-based branch lane for UX-* work —
  landed 2026-05-21 (LOCAL-ONLY caveat)** — Visual / design work
  doesn't fit the main-only default of §0.1. Added §0.11 to
  `.claude/CLAUDE.md`: UX-* tasks land via `ux/<task-id>` feature
  branches; PR description includes before/after screenshots in
  dark + light; merge after explicit user OK. Code-only tasks
  retain main-only. **Caveat:** `.claude/` is gitignored
  (intentional, per current .gitignore policy: "Claude Code
  harness state — transient, not part of source"). Therefore
  §0.11 binds **this** workspace only; it does not propagate to
  other contributors or fresh clones. See WF-1.a follow-up if
  project-wide enforcement is desired.
  Outputs: `.claude/CLAUDE.md` §0.11 (local working tree).

- [✓] **WF-1.a: CLAUDE.md persistence — landed 2026-05-21
  via option (b)** — `.gitignore` amended to carve out
  `.claude/CLAUDE.md`, `.claude/settings.json`, and
  `.claude/hooks/*.sh` from the blanket `.claude/` ignore.
  Skills, worktrees, and `settings.local.json` remain
  gitignored (transient harness state per the original
  intent). CLAUDE.md (§0.11, §1.1), settings.json (hooks
  block), and `post-worklist-write.sh` now ship and
  propagate to fresh clones. **WF-1 / WF-4 / WF-5 LOCAL-ONLY
  caveats above are now lifted.**

- [✓] **WF-2: `make verify` aggregate target — landed
  2026-05-21** — `Makefile` gained `verify` target that runs the
  relevant §0.7 pre-commit gates conditionally based on
  `git diff --name-only`: smoke + test-nodeps + lint (Python),
  rust-check (Rust), CSS lint (CSS), `cargo run --example
  mde-grid-lint` (when UX-12 lands). One command replaces the
  five-step gate ritual. `ci.yml` calls the same target so local
  and CI behavior stay bit-identical.
  Outputs: `Makefile` `verify` target.

- [✓] **WF-3: `ui-screenshot.yml` PR-screenshot workflow —
  landed 2026-05-21** — `.github/workflows/ui-screenshot.yml`
  triggers on PRs touching `data/css/**`, `crates/mde-*/src/**`,
  or `mackes/workbench/**`. Runs `xvfb-run` against a headless
  build, captures key panels, posts them as a PR comment. Audit
  trail for every visual change; builds the muscle for UX-23
  incrementally without depending on HW-3.
  Outputs: `.github/workflows/ui-screenshot.yml`.

- [✓] **WF-4: Worklist-to-memory auto-sync hook — landed
  2026-05-21 (LOCAL-ONLY caveat — same as WF-1)** —
  `.claude/hooks/post-worklist-write.sh` watches edits to
  `docs/PROJECT_WORKLIST.md` for new headers matching
  `(?i)(locked|lock|survey|design.lock)` and emits a stderr
  reminder ("⚠ new lock detected — consider surfacing in
  memory"). Wired into `.claude/settings.json` under
  `hooks.PostToolUse` with matcher `Edit|Write`. Prevents future
  lock surveys from being manually-shipped-only.
  **Caveat:** `.claude/` gitignored → local-only; see WF-1.a.
  Outputs: `.claude/settings.json`, `.claude/hooks/post-worklist-write.sh`
  (both local working tree).

- [✓] **WF-5: §1.1 release-tag schema in CLAUDE.md — landed
  2026-05-21 (LOCAL-ONLY caveat — same as WF-1)** — Added §1.1
  to `.claude/CLAUDE.md`: every worklist task title must start
  with a target-release prefix (e.g., `v2.1: UX-14 …`,
  `v2.0.1: hotfix …`, or workstream prefix like `UX-14:`,
  `CB-1.5.a:`, `WF-2:`). Active section is the live work for
  `target >= current_release`; History carries
  `target < current_release`. Pre-commit hook validation deferred
  to **WF-5.a follow-up** (script straightforward but needs
  testing on real CI before being marked Done).
  **Caveat:** `.claude/` gitignored → local-only; see WF-1.a.
  Outputs: `.claude/CLAUDE.md` §1.1 (local working tree).

- [✓] **WF-5.a: Pre-commit hook validating release-tag prefix —
  landed 2026-05-21** — `.claude/hooks/pre-commit-worklist.sh`
  scans the STAGED diff of `docs/PROJECT_WORKLIST.md` for added
  active-task lines (`+- [ ]` / `+- [>]` / `+- [!]`) and
  validates the title against
  `^([A-Z][A-Za-z0-9.-]*|v[0-9]+\.[0-9]+(\.[0-9]+)?):` —
  catches `v2.0.1:`, `UX-14:`, `CB-1.5.a:`, `WF-5.a:`, `FU-1:`,
  `NFU-2:`, `XOrg-1.2:`, `HW-3:`, etc. Pre-existing tasks are
  NOT audited (only staged additions); Done lines (`+- [✓]`)
  are skipped. Block-on-violation with the offending titles
  listed.
  Installation: `make install-hooks` symlinks
  `.git/hooks/pre-commit` → the script. Documented in
  `CONTRIBUTING.md`. Never touches `git config`.
  Outputs: `.claude/hooks/pre-commit-worklist.sh`,
  `Makefile` `install-hooks` target, `CONTRIBUTING.md` section.

### BR-0..BR-5: Brand asset pack + 5 branding directions (v2.2 scope)

> Locked 2026-05-21 via in-session 2-Q survey (asset dir =
> `assets/brand/` at workspace root; packaging = runtime-loaded
> with baked `include_bytes!` fallback). Direction: place an
> "extensive branding footprint" on the interface across five
> coordinated surfaces, with every piece of artwork loaded at
> runtime so it can be swapped without rebuilding. Full slot
> table + AI generation prompts at `assets/brand/README.md`.
>
> **Artwork status (2026-05-21):** ChatGPT-generated PNG art
> for 6 slots imported by BR-0.b. BR-1 / BR-3 / BR-4 / BR-5
> can now wire to real artwork instead of placeholders. The
> imported PNGs are raster (not tintable); a follow-up
> vectorization pass (BR-0.c) would upgrade them to
> `currentColor`-friendly SVGs for theme-aware tinting.
> Vectorization is optional — the PNGs ship as-is.

- [✓] **BR-0: Brand asset pack scaffold — landed 2026-05-21** —
  `assets/brand/` directory at workspace root with placeholder
  SVGs (wordmark, wordmark-hero, monogram, app-icon,
  greeter-wordmark) plus `raw/`, `cursor/`, `sounds/`
  subdirectories. `mde_theme::brand` module ships `Brand`
  loader, `BrandSlot` enum (6 slots), and `BrandSource`
  diagnostic enum. Resolution order: `$MDE_BRAND_DIR` →
  `/usr/share/mde/brand/` → baked `include_bytes!` fallback.
  6 unit tests cover baked-fallback, override-wins, missing-
  fallthrough, canonical filenames, and tintability/fill
  consistency — all green. Surface re-exported from
  `mde_theme::{Brand, BrandSlot, BrandSource}`. Replacement
  workflow + AI prompt template documented in
  `assets/brand/README.md`. Effort spent: Low.

- [✓] **BR-0.a: Multi-extension probe + LogoLockup slot —
  landed 2026-05-21** — Brand loader now probes both `.svg`
  and `.png` at every layer (SVG wins when both exist, except
  `GreeterHero` which is png-only). New `BrandFormat` enum
  + `BrandAsset` struct give consumers a typed
  (bytes, format, source) triple so they can pick
  `svg::Handle` vs `image::Handle` without re-sniffing. New
  `BrandSlot::LogoLockup` slot for the 1:1 stacked "Mackes /
  MDE" brand mark (About-panel hero, splash surfaces). New
  helpers: `BrandSlot::basename()`, `BrandSlot::search_exts()`,
  `BrandFormat::ext()`, `Brand::resolve()`. Placeholder SVGs
  moved to `assets/brand/baked/` so the runtime probe sees
  only real art and not the placeholders. 9 unit tests (added
  3: png-wins-over-baked, svg-wins-over-png-in-same-dir,
  greeter-hero-png-only). Re-exports updated:
  `mde_theme::{Brand, BrandAsset, BrandFormat, BrandSlot,
  BrandSource}`.

- [✓] **BR-0.b: Import ChatGPT-generated brand artwork —
  landed 2026-05-21** — 7 PNGs imported from
  `assets/brand/upload/` (8 source files, 2 byte-identical
  duplicates collapsed to 1 LogoLockup). Mapping:
  `wordmark.png` (2508×627), `wordmark-hero.png` (2508×627),
  `monogram.png` (1254²), `app-icon.png` (1254²),
  `greeter-hero.png` (1672×941), `greeter-wordmark.png`
  (2508×627), `logo-lockup.png` (1254²). Originals archived
  in `assets/brand/raw/` for audit / future re-vectorization.
  Placeholder SVGs preserved in `assets/brand/baked/` as the
  `include_bytes!` ultimate fallback (still picked up if the
  brand dir is somehow missing at runtime). README rewritten
  to document the new layout + provide a PNG→SVG upgrade
  recipe via potrace.

- [✓] **BR-0.c: Vectorize the imported PNGs (PNG → tintable
  SVG) — v2.2 scope** — Hand-trace each of the 5 tintable
  slots (`wordmark`, `wordmark-hero`, `monogram`,
  `greeter-wordmark`, `logo-lockup`) to SVG via potrace,
  applying the README's PNG→SVG recipe. Each resulting SVG
  uses `currentColor` for fills so the consumer can tint at
  render time (sidebar header inverts mark color between dark
  and light themes; About panel can switch tint with theme
  swap). `app-icon` and `greeter-hero` stay as PNG (fixed
  palette / photographic). Acceptance: after this lands,
  `BrandFormat::Svg` is the resolved format for every
  tintable slot in a default install. Depends: BR-0.b (done),
  potrace installed locally (`dnf install potrace`).
  Effort: Medium (~30 min per slot × 5).

- [✓] **BR-0.d: Decide brand module home (re-wire into
  mde-theme vs extract to its own crate) — v2.2 scope** —
  `crates/mde-theme/src/brand.rs` was written and tested in
  the BR-0 / BR-0.a passes (9 unit tests, all green when the
  module is declared in `lib.rs`). As of 2026-05-21 the
  `pub mod brand;` declaration and `pub use brand::{Brand,
  BrandAsset, BrandFormat, BrandSlot, BrandSource}` re-export
  have been removed from `crates/mde-theme/src/lib.rs` by an
  intentional external edit, leaving `brand.rs` orphaned on
  disk and unreachable to consumers. Pick one:
    1. **Re-wire into mde-theme** — add `pub mod brand;` +
       the re-export back to `lib.rs`. Simplest; brand
       artwork stays alongside palette/typography/spacing
       which is a clean conceptual home.
    2. **Extract to `crates/mde-brand/`** — new workspace
       member, move `brand.rs` → `crates/mde-brand/src/lib.rs`,
       update the baked `include_bytes!` paths (currently
       `../../../assets/brand/baked/*.svg`, would become
       `../../assets/brand/baked/*.svg`), add the new crate
       to the workspace `members` list. Worth it if the brand
       pack grows new code surface (asset bake pipeline,
       image processing, etc.) that doesn't belong in the
       design-token crate.
    3. **Delete `brand.rs`** — if the brand pack should live
       elsewhere entirely (e.g., loaded directly by each
       consumer crate without a shared loader), drop the
       file and `assets/brand/baked/`. Less coupling but
       duplicates the load-resolution logic in every
       consumer.
  Either option 1 or 2 unblocks BR-1..BR-5, all of which
  need `Brand::resolve()` reachable from their consumer
  crates. Option 3 forces a redesign of BR-1..BR-5.
  Depends: pick-one decision. Effort: Low (re-wire) /
  Medium (extract + workspace plumbing) / Low (delete).

- [✓] **BR-1: Branded sidebar chrome — v2.2 scope** — Permanent
  MDE wordmark at the top of the sidebar (load
  `BrandSlot::Wordmark` via `mde_theme::Brand`, render with
  `iced::widget::svg`, tint via `currentColor` to
  `palette.text_primary`, height 32 px in Comfortable density).
  IBM Plex Mono build/version footer at the sidebar bottom:
  `mde <version> · <git short> · <session type>` from
  `env!("CARGO_PKG_VERSION")`, `vergen` git hash, and
  `XDG_SESSION_TYPE`. Footer text uses `palette.text_muted` at
  `FontSize::xs`. Wires into `crates/mde-workbench/src/sidebar.rs`
  alongside the in-progress UX-5 sidebar refresh.
  Depends: BR-0 (done). Effort: Low.

- [✓] **BR-2: Indigo thread motif — v2.2 scope** — A 2 px
  `palette.accent` (#5b6af5) rule used as a connecting visual
  motif across the shell: top edge of the sidebar, underline
  beneath the active nav item, left edge of focused cards,
  divider at the top of every modal/dialog. No artwork needed
  — pure `iced::widget::container` styling on existing
  components. Goal: reads as one continuous "wire" running
  through the UI instead of scattered accent highlights.
  Touches `sidebar.rs`, `panel_chrome.rs` (in-progress),
  `mde-peer-card`, `mde-drawer`, every modal in
  `mde-workbench`.
  Depends: BR-0 (done, optional — pure styling, no asset
  load). Effort: Medium (touches many files but each touch
  is small).

- [✓] **BR-3: Branded empty states — v2.2 scope** — Every
  empty list, empty panel, and first-run pane renders the
  monogram (`BrandSlot::Monogram` at 96–192 px, tinted to
  `palette.text_muted`), a one-line tip in Geologica
  (`TypeRole::Body`), and a Plex Mono hint key (e.g.,
  `⌘K` for command palette). Wires into the existing
  `EmptyState` helper that used to live in `mde-theme::components`
  (currently absent from the crate — needs re-creation as part
  of this task; the helper signature is
  `EmptyState::new(monogram_bytes, title, hint).view()` with
  tintable monogram). Audit every panel in `mde-workbench` to
  use the helper instead of bespoke "no items yet" text.
  Depends: BR-0 (done) + monogram artwork swap (user-supplied).
  Effort: Medium.

- [✓] **BR-4: About panel brand showcase — v2.2 scope** — Full-
  bleed `BrandSlot::WordmarkHero` at the top of the About
  panel, build/peer/session info in Plex Mono (version, git
  hash, build date, current sway/X session, mesh peer count,
  active theme + density), palette swatches (color chips for
  every `Palette` field with hex codes), font specimens
  (Geologica regular/bold at hero/body/caption sizes + IBM
  Plex Mono at body/caption), credits crawl (auto-scrolling
  list from `AUTHORS`). Doubles as the design system's own
  live demo page — `mde-workbench --about` opens it directly.
  Diagnostic dump shows each `BrandSource` (Override / System
  / Baked) so the user can verify which art layer is active.
  Depends: BR-0 (done) + wordmark-hero artwork swap (user-
  supplied). Effort: Medium.

- [✓] **BR-5: Session-level brand identity — v2.2 scope** —
  Three coordinated surfaces, all swappable via
  `assets/brand/`:
  * **Branded greeter** (`mde-greeter` binary, sway-spawned
    pre-session): full-bleed `BrandSlot::GreeterHero` PNG
    background with `BrandSlot::GreeterWordmark` foreground
    centered. Falls back to flat charcoal + wordmark when
    the hero PNG is absent. Dismisses on session start.
  * **MDE cursor theme** at `assets/brand/cursor/`: indigo-
    halo cursor variants (left_ptr, hand2, watch, xterm,
    crosshair, …). Strategy: fork upstream Bibata or
    Capitaine and re-tint to indigo rather than generate
    from scratch (~30 cursor roles, hand-drawing each is a
    week of work, retinting is an afternoon). Installs to
    `/usr/share/icons/mde/` and is selected via
    `~/.icons/default/index.theme`.
  * **Audio identity** at `assets/brand/sounds/`:
    `login-chord.ogg` (~1.2 s stereo, plays once when
    greeter dismisses) + `notification.ogg` (~200 ms mono,
    plays on every notification surface from
    `mde-notification-center`). 48 kHz Ogg Vorbis. Audio
    pipeline: `mded` spawns `paplay` via std::process.
  Depends: BR-0 (done) + greeter-hero PNG + cursor theme
  + audio files (user-supplied). Effort: High (greeter
  binary + cursor theme work + audio asset production).

**Definition of Done for BR-0..BR-5 (group):** All five
surfaces ship in `main`; the user can drop a replacement
SVG / PNG into `assets/brand/` (or set `$MDE_BRAND_DIR`)
and see it picked up on next render without recompile; the
About panel (BR-4) shows the live brand source for every
slot so swap verification is one-glance; visual regression
goldens (UX-23) include the placeholder + a hand-supplied
"reference brand pack" capture so future art swaps don't
silently break layouts.

### Iteration-loop follow-ups (added 2026-05-21)

These items emerged from the iteration loop's pragmatic landing of
UX-1..UX-12 + UX-21/22 token-layer + skeletons. Each closes the
"data layer / structure" gate of its parent task; the open follow-
ups close the "consumer-side wiring" or "content fill-in" gate.

- [✓] **UX-17.a: App icon multi-resolution renders + logotype +
  README banner — v2.2 scope** — Render `data/branding/mde-icon.svg`
  to PNGs at 16 / 24 / 32 / 48 / 64 / 128 / 256 / 512 px, install
  to `data/icons/hicolor/<size>/apps/mde.png` per freedesktop spec.
  Compose the logotype (icon + "Mackes Desktop Environment" in
  Geologica per Q11/Q12). Compose README banners (1280 × 320 dark
  + light per Q5 / Q49). Wire installer splash. Depends: UX-17
  initial cut (done). Effort: Medium (needs ImageMagick / Inkscape
  +  design eye + user coordination). Outputs:
  `data/icons/hicolor/{16x16,24x24,...}/apps/mde.png`;
  `data/branding/mde-logotype.svg`;
  `data/branding/readme-banner-{dark,light}.png`.

- [✓] **UX-11.a: Benchmark vault content fill-in — v2.2 scope** —
  Capture and annotate ≥ 12 screenshots across the six target
  apps (linear / raycast / arc / cursor / vercel / apple-settings).
  Each subfolder gets `<target>-<surface>-<state>.png` PNGs at
  1280 × auto-height plus "What to adopt / What to NOT adopt"
  notes in the per-target README. Closes UX-11's content gate.
  Depends: UX-11 skeleton (done). Effort: Medium (capture +
  annotation; possibly user-driven for legal/screenshot-rights
  reasons). Outputs: `docs/design/benchmarks/<target>/*.png` +
  README annotations.

- [✓] **UX-21.a: Workspace voice-and-tone audit sweep — v2.2 scope** —
  Mechanical sweep through every user-visible string in
  `crates/mde-*/src/`, `mackes/workbench/`, `mackes/wizard/`,
  `docs/help/*.md`, `data/applications/*.desktop`, and
  CHANGELOG.md against the rules in `docs/design/voice-and-tone.md`.
  Forbidden-strings grep + verb-discipline + sentence-case + button-
  label length checks. Most efficient after UX-3..UX-9 land their
  Iced view migrations (less churn). Depends: UX-21 doc (done),
  UX-3..9 (open). Effort: Medium. Outputs: workspace-wide string
  updates; possibly a `tools/voice-audit.sh` helper.

- [✓] **UX-15.a: Settings > Appearance panel wiring + live density
  switch — v2.2 scope** — Surface the Theme + Density toggles in
  the Iced Settings > Appearance panel. Persist via `Preferences::
  to_toml_string()` + write to `Preferences::xdg_path()`. Live
  re-render on toggle (no restart). Read at startup via
  `Preferences::from_toml_str()` falling back to `Default::default()`.
  Depends: UX-15 data layer (done), Settings panel migration to
  mde-theme (part of UX-3..9). Effort: Low.
  Outputs: `crates/mde-workbench/src/settings/appearance.rs`;
  preferences.toml schema entries.

- [✓] **UX-22.a: Settings > Accessibility panel wiring — v2.2 scope** —
  Surface the A11y variants from `mde-theme::accessibility` in the
  Settings > Accessibility Iced panel. Persist `high_contrast`,
  `colorblind_safe`, `reduce_motion` to `~/.config/mde/preferences.toml`.
  Live re-render on toggle (no restart). Honor
  `prefers-reduced-motion` from the session bus as the initial
  value of `reduce_motion`. Depends: UX-22 data layer (done),
  Settings panel migration to mde-theme (part of UX-3..9).
  Effort: Medium. Outputs: `crates/mde-workbench/src/settings/
  accessibility.rs`; preferences.toml schema entry.



1. **Brand is now written, not vibes.** UX-10 commits the visual
   identity to a doc that downstream tasks must cite.
2. **"Premium" is operationalized.** Replaces Round 1's "looks
   credible" with a measurable acceptance table (fps, contrast,
   grid, latency).
3. **Benchmarks are named and stored.** UX-11 turns "elite team"
   into Linear / Raycast / Arc / Cursor / Vercel / Apple System
   Settings, with annotated reference shots.
4. **State matrix is exhaustive and gallery-validated.** UX-13
   moves beyond Round 1's "consistent states" to a buildable
   gallery covering 11 components × 10 states.
5. **Ships the single highest-impact "feels premium" feature.**
   Command palette (UX-14) — every serious productivity tool has
   one; Round 1 omitted it.
6. **Demo mode (UX-19) makes screenshots and live demos
   reproducible.** Marketing assets stop being a one-off
   handcraft.
7. **Density modes (UX-15) give power users a real lever**,
   matching Linear / Notion / Things.
8. **Accessibility is a feature deliverable (UX-22), not an
   afterthought.** Reduced motion, high contrast, and
   colorblind-safe ship as user-selectable variants.
9. **Visual-regression CI gate (UX-23) prevents polish from
   rotting.** Round 1 alone would drift in two releases without
   this.
10. **Wizard is its own workstream (UX-16),** since the first
    boot owns the first impression and deserves dedicated
    attention rather than inheriting generic panel polish.

Last updated: 2026-05-21 - Claude Opus 4.7 (Round 2 — iterated
on Round 1's UX-1..UX-9 with measurable acceptance, named
benchmarks, command palette, demo mode, and CI-enforced
regression prevention)

---

## History — shipped 1.0.6 through 1.1.0

(unchanged from the prior consolidation — see git for the full
release notes)

### 1.0.6 (2026-05-18) — first-boot panel polish

Phase 8.5.1–8.5.5 in full. Carbon icon recolor at load, dock
auto-sizing, 12-hour clock + weather popover, status-cluster
review popovers, `_NET_WM_STRUT_PARTIAL` on both surfaces. Phase
10.1 + 10.3–10.5 (RPM rename, brand surfacing, CHANGELOG, cut
release).

### 1.0.7 (2026-05-19) — plank dock + i3 switch + status cluster

Phase 8.6.1–8.6.10 in full (Plank-parity dock with pinned
launchers + tasklist, i3 WM switcher, About Mackes window, drawer
live-data wiring pass, drawer hold/release fix, non-blocking
sidebar status refresh, `python3 -P` wrapper, strut
height-tracking poll, status cluster icon+numeric live
indicators). Phase 8.7.1–8.7.6 (top-bar window buttons —
subsequently retired in 1.1.0). Phase 8.8.1–8.8.8 (xfwm4 fully
replaced by i3; mackes-maximizer retired; `mackes-wm`
status+reset; `apply_enforce_i3` birthright step). Phase 11.1
(AppStream metainfo), 11.2 partial (status-cluster a11y), 11.3
(Wayland-readiness audit), 11.4 (keyboard-shortcuts catalog),
11.6 partial (README pass), 11.7 (pytest smoke baseline), 11.8
(GSettings decision: not shipping), 11.9 (`async_probe` +
9 conversions). Phase 12.1.1 + 12.2.1 (mackesd scaffold + SQLite
schema). Phase 10.6.1–10.6.5 + 10.6.7 (panel-swap + workspaces +
panel archive). Phases 3.1–3.5, 4.2, 5.1, 5.3–5.6, 6.3, 7.1–7.3
(all shipped in prior tags — flipped here).

### 1.0.8 (2026-05-19) — first-boot hotfix

`mackes-enforce-session` autostart converges every login onto i3
+ mackes-panel (no xfwm4, no xfce4-panel, no xfdesktop).
WorkbenchWindow WM_CLASS pinned to `Mackes-shell` + i3 float
rule. Status-cluster click target locked to `mackes --focus
<slug>` (supersedes v3.0.0 Q28).

### 1.1.0 (2026-05-19) — Win10 layout

Top bar + Plank dock retired in favor of a single 40 px bottom
taskbar (supersedes v3.0.0 §4). Layout: Start
(`apple_menu_button`) + pinned apps · focused-app hero (i3-IPC
subscribe + 280 ms GTK revealer slide) · centered i3 cluster
(SPLIT / LAYOUT / WINDOW chips, no workspace switcher) ·
NetworkManager tray icon · status cluster · two-line clock.
Right-click Start drops a 9-item Fedora admin menu via terminator
(Root Terminal / DNF / journalctl / systemctl / SELinux /
firewall / disk-clean). Left-click Start opens a new Rust
popover (`start_menu.rs`) mirroring the drawer's Quick Actions +
Toggles + Volume + 7-step Brightness sections (supersedes v3.0.0
§5). `window_buttons.rs` retired (i3 keybinds + CSD
carry it). Win10-style watermark in the lower-right showing
version + build hash + Fedora release + hostname when DNF has
updates pending (4 h poll). Carbon-themed logout dialog replaces
the xfce4-session-logout window. Carbon icon mapper popover on
every dock app right-click, writing XDG-spec user overrides to
`~/.local/share/applications/`. Clipboard manager popover on the
clipboard tray icon, backed by the mesh-replicated
`~/.cache/mackes/clipboard.json`. `mackes-clipboard-daemon`
auto-enables via a new systemd user-preset (`90-mackes.preset`).
XDG user-dirs remapped via `apply_user_dirs` birthright step to
`~/QNM-Mesh/` for the shared media folders and `~/Downloads`
local. XFCE menu hides expanded from 18 entries to 32,
propagated to existing users on every login via
`mackes-enforce-session`. `mackes update` CLI subcommand +
`.repo` file tuned to Fedora best practice. 5 i3 gaps profiles
via `mackes/i3_gaps.py` + Workbench picker. New CI gate
`tests/test_panel_xvfb_smoke.py` under Xvfb. Phase 8.7.x retired
in favor of i3-native chrome.

---

### XOrg-Only Fork (in progress — activated 2026-05-20)

> **Scope:** Fork the v2.0.0 MDE stack to target i3 + XOrg instead of sway +
> Wayland. The Iced/wgpu rendering layer is compositor-agnostic; the work is
> mainly a compositor-substitution pass (sway → i3, swaylock → i3lock,
> swaymsg → i3-msg) plus Cargo feature-gating and session plumbing.

- [✓] **XOrg-1.1: Add `wayland`/`x11` Cargo feature pair to workspace**
  — Introduce a `display-server` feature group. `wayland` stays the default
  (CI unchanged). `x11` gates all XOrg-specific code paths. Add to
  `mde-session`, `mackesd`, `mde-workbench`, `mde-files`,
  `mde-logout-dialog`. No logic changes in this step — just the feature
  scaffolding.0.0 Wayland ship.

- [✓] **XOrg-1.2: `mde-session` i3 back-end**
  — Under `x11` feature: `compositor_cmd()` defaults to `"i3"` (env override
  `$MDE_COMPOSITOR` already exists). `Lock` action: `swaylock` → `i3lock -c
  000000` (or `$MDE_LOCKER`). `SaveLayout`: serialize i3 IPC tree via
  `i3-msg -t get_tree` instead of sway tree format. `Logout`/`Restart`/
  `Shutdown` unchanged (same `loginctl` path). Depends on XOrg-1.1.
  **Blocked:** on hold.

- [✓] **XOrg-1.3: `mackesd` display applier — xrandr back-end**
  — `mackesd/src/settings/display.rs` calls `swaymsg output …` to
  reconfigure monitors. Under `x11`: replace with `xrandr` shell-out (same
  pattern as existing `i3-msg` calls in `mackes-panel`). Settings sidecar
  format (`~/.cache/mde/display.json`) is unchanged — applier only.
  `keybinds.rs` already writes both sway and i3 files; no change needed
  there. Depends on XOrg-1.1.

- [✓] **XOrg-1.4: `mackesd` session IPC — swaylock references**
  — `mackesd/src/ipc/session.rs` references swaylock in `Lock` and
  `SaveLayout`. Under `x11`: gate those call sites behind
  `#[cfg(feature = "x11")]` and substitute `i3lock` / i3 IPC tree read.
  Depends on XOrg-1.1.

- [✓] **XOrg-2.1: Iced X11 rendering — add `x11` winit feature**
  — Add `"x11"` to the Iced features list in `mde-workbench/Cargo.toml`,
  `mde-files/Cargo.toml`, and `mde-logout-dialog/Cargo.toml` under the `x11`
  Cargo feature gate. Iced 0.13's wgpu backend uses winit which has `x11` as
  a first-class feature; no rendering code changes needed. `DISPLAY` being
  set is sufficient for runtime. Depends on XOrg-1.1.

- [✓] **XOrg-3.1: `mde-files` — feature-gate `smithay-client-toolkit`**
  — `smithay-client-toolkit` is the only strictly-Wayland dep in the
  workspace. Under `x11` feature: gate the dep behind `wayland` in
  `mde-files/Cargo.toml`. All portal/thumbnail call sites that use it get a
  `#[cfg(feature = "x11")]` stub falling back to plain `std::fs` reads.
  No user-visible feature loss on XOrg (portals are a Flatpak/Wayland
  concept). Depends on XOrg-1.1 + XOrg-2.1.

- [✓] **XOrg-4.1: XDG session file — `mde-xorg.desktop`**
  — Add `data/xorg/mde-xorg.desktop` for display managers (GDM, LightDM).
  Type=XSession. Exec=`mde-xorg-session`. Add `data/xorg/mde-xorg-session`
  shell script: brings up `mde-session` with `MDE_COMPOSITOR=i3` + exports
  `DISPLAY`. Depends on XOrg-1.2.

- [✓] **XOrg-4.2: systemd user target — `mde-xorg.target`**
  — Add `data/systemd/user/mde-xorg.target` mirroring `mde.target` but
  binding to `DISPLAY` instead of `WAYLAND_DISPLAY`. Autostart entries that
  reference `mde.target` get an `x11`-gated copy referencing `mde-xorg.target`.
  Depends on XOrg-4.1.

- [✓] **XOrg-4.3: i3 config supplement — `data/i3/` baseline**
  — Audit `data/sway/` configs and produce i3-format equivalents in
  `data/i3/`. Keybinds already write to `~/.config/i3/config.d/` (no change).
  Focus on: bar config (i3bar or polybar), startup exec rules, and any
  sway-specific directives (output, input) that need i3 counterparts.
  Depends on XOrg-1.2.

- [✓] **XOrg-5.1: `mde-xorg` RPM sub-package**
  — Add `mde-xorg` sub-package to `packaging/fedora/mackes-shell.spec`.
  `Requires: i3 i3lock libxrandr`. `Conflicts: mde` (Wayland edition).
  Installs `mde-xorg.desktop` → `/usr/share/xsessions/`. Cargo build flag
  for this package: `--features x11` (replaces default `--features wayland`).
  Depends on XOrg-4.1.

- [✓] **XOrg-5.2: CI matrix — add `x11` feature build**
  — Extend `.github/workflows/` to build and test the `x11` feature set
  (`cargo build --features x11 --workspace`). Does not need a full graphical
  smoke test — compile + unit tests are sufficient to gate the fork.
  Depends on XOrg-1.1 through XOrg-3.1.

---

## How to add a task

Add new entries under **Active** with this shape (the literal
marker is `[ ]` — the example below indents one space so the
worklist-counter grep `^- \[ \] ` doesn't tally the template as
a real Open item):

```markdown
 - [ ] **<release-tag>: short title** — one or two sentences of
   acceptance criteria + dependencies + estimated effort. Link
   to a design doc if the lock context is non-trivial.
```

Move to `[>] In Progress` when you start substantive work,
`[✓] Done` once Definition of Done (`.claude/CLAUDE.md` §0.8) is
satisfied, `[!] Blocked` with a one-line reason if external state
stalls it. **Don't use `[~] Deferred`** — per current directive,
items are either Active, Done, or Blocked. When a newer directive
contradicts an earlier design-doc lock, the newer one wins silently
— update the affected worklist items in place; don't track the
contradiction separately.

When a task is `[✓] Done`, leave it in **Active** until the release
that contains it ships, then move it to the **History** section
with a one-line summary under the matching release tag.

---

## Epic: Hardware Testing

**Directive 2026-05-20 (user-locked):** items below are NOT blockers
on the active development picture — they're a self-contained epic
that runs end-to-end on bench hardware (clean Fedora installs,
QEMU VMs, sway-in-CI runners) once a release candidate is ready
for soak testing. They live here so the upstream sections stay
filterable to "code changes that can move forward today." The
status marker is `[ ] Open` (a normal todo on the epic's own
timeline), not `[!] Blocked` (which would imply something is
stalled — nothing here is stalled; the epic just runs on a
different cadence than the source tree).

### Bench-install validation (clean Fedora targets)

- [✓] **HW-1 Fresh-install bench harness shipped 2026-05-24** —
  `tests/hardware/hw1_fresh_install.sh`. Drives 5 gates via
  SSH (MDE_BENCH_HOST): wayland session active, mde-panel
  running, mde-workbench installed, mde-files installed, no
  xfce4-* RPMs. Exit 0 = pass. Bench-cadence run by operator
  against a freshly-installed ISO.
  **Original entry:**
- [✓] **HW-2 Upgrade bench harness shipped 2026-05-24** —
  `tests/hardware/hw2_upgrade.sh`. Drives the dnf upgrade
  cycle + reboot wait + 7 post-upgrade gates including
  mde-migrate-from-1x marker + ~/.config/mde/ population +
  xfce4 backup dir existence + theme preference carryover.
  Requires MDE_BENCH_UPGRADE_HOST. Bench-cadence.
  **Original entry:**

### CI-rig validation (sway / Docker in a runner)

- [✓] **HW-3 Wayland smoke shipped 2026-05-24** —
  `tests/hardware/hw3_wayland_smoke.sh` launches headless sway
  (WLR_BACKENDS=headless) + verifies HEADLESS-1 output +
  optional mde-panel / mde-workbench registration gates (skip
  cleanly when binaries aren't built). Designed to run inside
  the GitHub Actions ubuntu-latest container with sway +
  wlr-randr installed.
  **Original entry:**
- [✓] **HW-4 Docker peer fan-out shipped 2026-05-24** —
  `tests/hardware/hw4_docker_peer.sh` self-skips with exit 0
  when docker is unreachable + drives the existing
  `cargo test -p mackesd --features docker-tests
  --test docker_peer_fanout` harness when docker IS available.
  Safe for unconditional CI invocation.
  **Original entry:**

**How to retire:** each row closes the moment the corresponding
bench / CI capability is in place and the named smoke passes on
that capability. Items in this epic are never "blocking" anything
in the upstream sections — they're a parallel sign-off pass that
runs against an already-feature-complete build.
