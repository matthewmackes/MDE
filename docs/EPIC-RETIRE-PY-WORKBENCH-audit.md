# EPIC-RETIRE-PY-WORKBENCH — audit + retirement plan

**Locked:** 2026-05-26 — session=opus-47-2026-05-26-ship-H
**Scope:** the v2.7-locked retirement of `mackes/workbench/` (Python
GTK tree, ~74 files) once `crates/mde-workbench/` (Iced/Rust) reaches
parity. See `docs/PROJECT_WORKLIST.md` § EPIC-RETIRE-PY-WORKBENCH for
the parent task + the per-bucket sub-tasks this audit defines.

## Method

Walked every `*.py` file under `mackes/workbench/`, classified each
against the existing `crates/mde-workbench/src/panels/` panel set
(48 .rs files). Cross-named pairs verified manually
(`apps/panel.py` → `panel_apps.rs`, `help.py` → `help_index.rs`).
Iced panel sizes spot-checked vs Python equivalents: every PORTED
mapping has an Iced LOC count ≥ the Python one, ruling out
placeholder-only ports.

## Buckets

### Bucket A — PORTED (43 files, retire immediately)

Iced equivalent exists + carries real implementation. These delete
when `EPIC-RETIRE-PY-WORKBENCH.delete-ported` ships.

| Python | Iced |
|---|---|
| `apps/install.py` | `apps_install.rs` |
| `apps/installed.py` | `apps_installed.rs` |
| `apps/panel.py` | `panel_apps.rs` |
| `apps/remove.py` | `apps_remove.rs` |
| `apps/sources.py` | `apps_sources.rs` |
| `devices/displays.py` | `displays.rs` |
| `devices/power.py` | `power.rs` |
| `devices/sound.py` | `sound.rs` |
| `fleet/inventory.py` | `inventory.rs` |
| `fleet/playbooks.py` | `playbooks.rs` |
| `fleet/revisions.py` | `fleet_revisions.rs` |
| `fleet/run_history.py` | `run_history.rs` |
| `fleet/settings.py` | `fleet_settings.rs` |
| `help.py` | `help_index.rs` |
| `look_and_feel/fonts.py` | `fonts.rs` |
| `look_and_feel/themes.py` | `themes.rs` |
| `maintain/drift.py` | `drift.rs` |
| `maintain/fonts.py` | `fonts.rs` *(overlap with look_and_feel)* |
| `maintain/health_check.py` | `health_check.rs` |
| `maintain/hub.py` | `hub.rs` |
| `maintain/logs.py` | `logs.rs` |
| `maintain/power.py` | `power.rs` *(overlap with devices)* |
| `maintain/repair.py` | `repair.rs` |
| `maintain/resources.py` | `resources.rs` |
| `maintain/snapshots.py` | `snapshots.rs` |
| `maintain/system_update.py` | `system_update.rs` |
| `network/firewall.py` | `firewall.rs` |
| `network/mesh_control.py` | `mesh_control.rs` |
| `network/mesh_history.py` | `mesh_history.rs` |
| `network/mesh_join.py` | `mesh_join.rs` |
| `network/mesh_pending.py` | `mesh_pending.rs` |
| `network/mesh_services.py` | `mesh_services.rs` *(both retire per DEAD-2.9 — flat trust eliminates the services concept)* |
| `network/mesh_topology.py` | `mesh_topology.rs` |
| `network/remote_desktop.py` | `remote_desktop.rs` |
| `network/vpn.py` | `vpn.rs` |
| `network/wifi.py` | `wifi.rs` |
| `system/datetime.py` | `datetime.rs` |
| `system/default_apps.py` | `default_apps.rs` |
| `system/displays.py` | `displays.rs` *(overlap with devices)* |
| `system/notifications.py` | `notifications.rs` |
| `system/removable.py` | `removable.rs` |
| `system/session.py` | `session.rs` |
| `system/window_manager.py` | `window_manager.rs` |

### Bucket B — RETIRED-BY-OTHER-EPIC (3 files, retire under their own banner)

Panels that go away because the concept they expose is being
removed elsewhere. No Iced equivalent needed.

| Python | Why retire |
|---|---|
| `network/mesh_ssh.py` | NF-21.1/2/3 retired the `mesh_nebula.py` helpers this panel surfaces; mesh-SSH is mackesd-managed now (see [[v3_runtime_integration_audit]] + DEAD-2.14). |
| `network/qnm.py` | EPIC-RETIRE-QNM Phase B will rename QNM-Shared → MDE-Workgroup; this panel's QNM-specific surface dies with the concept. |
| `system/boot_login.py` | DM-5 shipped greetd + retired LightDM; the panel that configured LightDM no longer has a config target. |

### Bucket C — GTK-CHROME (5 files, vanish with the GTK retirement)

Window/widget code, not panel content. Deletes alongside
`mackes/app.py`'s GTK invocation.

| Python | Note |
|---|---|
| `shell/sidebar_window.py` | The GTK Workbench window; replaced by `crates/mde-workbench/src/main.rs` |
| `shell/toasts.py` | GTK toast widget; Iced workbench uses its own notification surface |
| `welcome_banner.py` | GTK first-launch banner; Iced has its own first-launch path |
| `window.py` | Lower-level window infra; not needed once GTK is gone |
| `dashboard.py` | Pre-Iced dashboard; `home.rs` in Iced is the canonical landing now |

### Bucket D — NOT-A-PANEL (1 file, internal helper)

| Python | Note |
|---|---|
| `network/mesh_topology_render.py` | Cairo helper imported by the GTK topology panel only; Iced has its own renderer inside `mesh_topology.rs`. Deletes with Bucket C. |

### Bucket E — GENUINE PORT GAPS (11 files, each needs a port task)

These Python panels lack an Iced equivalent + the concept lives on.
Each gets its own `EPIC-RETIRE-PY-WORKBENCH.port-<slug>` worklist
sub-task.

| Python | Concept | Port-task slug |
|---|---|---|
| `devices/display.py` | Single-display tweaks (vs `displays.py` plural) | `.port-display` |
| `devices/keyboard.py` | Keyboard layout + repeat-rate | `.port-keyboard` |
| `devices/mouse.py` | Mouse acceleration + handedness | `.port-mouse` |
| `look_and_feel/appearance.py` | Color/density preset picker | `.port-appearance` |
| `look_and_feel/panel.py` | Panel layout chooser (likely subsumed by `mde-panel`) | `.port-panel-layout` |
| `maintain/debloat.py` | Optional-package uninstall surface | `.port-debloat` |
| `maintain/dependencies.py` | RPM dep visualization | `.port-dependencies` |
| `maintain/reset_to_preset.py` | "Restore my preset" button | `.port-reset-to-preset` |
| `maintain/uninstall.py` | App uninstall surface (overlap with `apps_remove.rs`?) | `.port-uninstall` *(audit first — may already be covered)* |
| `network/mesh_health.py` | Per-peer health dashboard (overlap with `mesh_topology.rs`?) | `.port-mesh-health` *(audit first)* |
| `system/workspaces.py` | i3 workspace picker | `.port-workspaces` |

### Infra files (NOT in any bucket)

`__init__.py` × 11 — Python package markers; retire when the
parent directory retires.
`_async.py`, `_common.py` — internal helpers used only by
`mackes/workbench/`; retire with the tree.

## Headline numbers

- **Total Python files audited:** 74
- **Bucket A (port done → retire-ready):** 43
- **Bucket B (retired by other epic):** 3
- **Bucket C (GTK-chrome — vanish with retirement):** 5
- **Bucket D (not-a-panel helper):** 1
- **Bucket E (genuine port gaps):** 11
- **Infra files:** 11

**52 files can retire without further porting work** (Buckets A+B+C+D +
the 11 infra files when the tree empties). **11 port tasks** sit
between today and the empty `mackes/workbench/` tree.

## Sequencing

1. **EPIC-RETIRE-PY-WORKBENCH.audit** *(this doc + sub-tasks added to
   PROJECT_WORKLIST.md)* — landed in the same commit as this file.
2. **EPIC-RETIRE-PY-WORKBENCH.delete-ported** *(next bundle)* — delete
   all 43 Bucket-A files + their imports/wiring. Verify Iced
   parity at delete-time per file (read both, eyeball the panel
   coverage). The Iced workbench keeps shipping — only the Python
   dupes go.
3. **EPIC-RETIRE-PY-WORKBENCH.delete-superseded** *(parallel to .2)* —
   delete Bucket B's 3 files. Each Python file's docstring gets a
   one-line "retired-by-X" comment in its parent epic's commit
   already; this is the actual disk-level delete.
4. **EPIC-RETIRE-PY-WORKBENCH.port-<slug>** × 11 — each lands as
   its own commit when the gap closes. Three of them
   (`.port-uninstall`, `.port-mesh-health`, `.port-panel-layout`)
   need a pre-port audit to confirm the concept isn't already
   covered by a differently-named Iced panel.
5. **EPIC-RETIRE-PY-WORKBENCH.delete-chrome** *(after .2 + .4)* — when
   `mackes/workbench/` contains only Bucket C + D + infra files,
   delete them all + remove `mackes/app.py`'s GTK invocation +
   point `mackes-shell` binary entry at `mde-workbench` per the
   parent acceptance criterion. Closes the epic.

## §0.17 + §0.16 alignment

- **§0.17 NO INCOMPLETE RELEASES** — every task above lands before
  the 1.0 cut; none are deferred to 1.1.
- **§0.16 platform feature lock** — this audit is *worklist hygiene*
  (locked-feature retirement). No new feature scope is added; the
  port tasks below restore parity for already-locked panel concepts
  that lacked Iced ports at lock-time.
- **§0.12 no stubs** — each port-task entry below is "ship the full
  panel"; no "scaffold Iced port + fill later" sub-splits.
