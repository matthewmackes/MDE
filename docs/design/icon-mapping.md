# Icon mapping — Carbon → Material Symbols migration

**Locked:** 2026-05-26 via 8-Q survey (`/plan The Icon-mapping
Sruvey`) across 2 rounds.

**Scope:** locks the design forks that make
`EPIC-UI-MATERIAL.svg-swap` shippable. Mapping table itself is
generated at implementation time per the heuristic policy locked
in Round 2 Q4; this doc captures **policies**, not the per-icon
table.

**Cross-refs:** [[project_ux_polish_locks]] (the Q43 lock that
made this migration mandatory), `docs/PROJECT_WORKLIST.md`
EPIC-UI-MATERIAL + its `.svg-swap` / `.lint-scope` sub-tasks.

---

## 0. Axiom

The 64 Carbon SVGs currently baked into `crates/mde-theme/src/icons.rs`
via 49 `include_bytes!` references are the **only** runtime icon
source in the modern MDE Rust stack. Migrating them — and the
`Icon::carbon_name()` API + `assets/icons/carbon/` directory —
delivers the actual Q43 ChromeOS-Classic visual lock that the
prior bundle's doc/memory/lint hygiene only described.

---

## 1. Locked decisions

| # | Decision | Lock |
|---|---|---|
| 1 | Material Symbols variant | **Outlined** |
| 2 | Fill rule | **Status indicators + notification bell + active-state tab/sidebar icons** (broader than Q38 Carbon; selection state now drives outlined↔filled swap) |
| 3 | SVG asset source | **Download 64 base icons × 3 sizes = 192 SVGs from Google Fonts** (Apache-2.0) via a fetch script |
| 4 | Stroke weight | **400** (Material default) |
| 5 | Sizing strategy | **Material optical-size variants** — bundle 20 / 24 / 40 px SVGs per icon; `IconSize` enum maps `Inline → 20`, `Nav → 24`, `Panel → 40` |
| 6 | Bake mechanism | **`include_bytes!` continues** — path swap to `assets/icons/material-symbols/<name>--{20,24,40}.svg` (+ `--fill` variant when active) |
| 7 | API rename | **`Icon::material_name()`** (was `Icon::carbon_name()`) |
| 8 | Per-icon mapping | **Heuristic policy** — at `.svg-swap` implementation time, Claude matches each Carbon symbolic name to its closest Material Symbol via Google's official catalog (https://fonts.google.com/icons). No operator pre-review; per-mapping disagreements get reverted in follow-on commits |

---

## 2. Asset layout

```
assets/icons/material-symbols/
├── dashboard_20px.svg                  # outlined, 20 px optical, weight 400
├── dashboard_24px.svg
├── dashboard_40px.svg
├── dashboard_fill1_20px.svg            # filled variant (only for fill-eligible icons)
├── dashboard_fill1_24px.svg
├── dashboard_fill1_40px.svg
├── ... (one set per mapped icon)
```

**File naming convention:** `<material_snake_name>[_fill1]_<size>px.svg`.
This matches Google's upstream filename convention verbatim, so the
fetch script's `mv` lands the file at the path the resolver expects
without renaming. Material's catalog uses `snake_case` names
(`notifications`, `network_check`); the `_fill1` infix is appended
only for fill-eligible icons.

**Total asset count (actual):** 180 unique SVGs on disk — 46 unique
Material names × 3 sizes = 138 outlined, plus 14 fill-eligible
unique names × 3 sizes = 42 filled. (The original 222 estimate
included `help` counted twice and `palette`/`settings`/`build`
counted per Carbon-name; deduplication brings the actual count
down. Several Icon variants share the same Material SVG —
`Icon::System` + `Icon::Settings` both render `settings_*.svg`,
`Icon::Maintain` + `Icon::Repair` both render `build_*.svg`, etc.)

---

## 3. Fetch script

`install-helpers/fetch-material-symbols.sh` — Bourne shell,
single `curl` dep.

The mapping table lives inline in the script as a here-document
(46 rows of `carbon_name:material_name:fill_mode` — much simpler
than a separate manifest file). For each row, builds the Google
material-design-icons GitHub raw URL:

```
https://raw.githubusercontent.com/google/material-design-icons/
    master/symbols/web/<material_name>/materialsymbolsoutlined/
    <material_name>[_fill1]_<size>px.svg
```

(The `_fill1` infix is appended for fill-eligible icons —
`fill_mode = always` or `fill_mode = active`.)

**During the implementation pass on 2026-05-26 the originally-
documented `fonts.gstatic.com/s/i/short-term/release/...` URL
pattern proved to return 404; the GitHub raw URL pattern above is
what actually serves the SVGs. The design lock changed; the doc
caught up here.**

Re-runnable. Idempotent — `[ -s "$target" ]` skip when the file
is already present and non-empty. Lives outside the build path so
`cargo build` doesn't depend on network access; operators (or a
future `make icons` rule) run it whenever the mapping gains or
loses a row.

The script's exit code is a hard fail when any download produces a
non-SVG response (checked via `head -c 200 | grep '<svg'`) —
guards against silent 404 HTML pages getting baked in.

---

## 4. API surface

```rust
// crates/mde-theme/src/icons.rs

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconState {
    Idle,    // outlined render
    Active,  // filled render, if the icon is fill-eligible
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconSize {
    Inline,  // 20 px optical
    Nav,     // 24 px optical
    Panel,   // 40 px optical
}

impl Icon {
    /// Material Symbol's `snake_case` semantic name (e.g.
    /// `"network_check"`). Successor to the retired
    /// `carbon_name()`.
    pub const fn material_name(self) -> &'static str { … }

    /// `true` if this icon participates in the active-state
    /// outlined↔filled swap (status indicators, the
    /// notification bell, and the active-state tab/sidebar
    /// set). Drives the `IconState::Active → --fill.svg`
    /// resolution.
    pub const fn is_fill_eligible(self) -> bool { … }

    /// Bytes for the requested (size, state) pair.
    /// `IconState::Active` falls back to the outlined variant
    /// for icons where `is_fill_eligible == false`.
    pub fn svg_bytes(self, size: IconSize, state: IconState) -> &'static [u8] { … }
}
```

Callsites that don't care about state pass `IconState::Idle`. The
sidebar / tab-strip render path threads its own selection state
into `svg_bytes` for the rows in the fill-eligible set.

---

## 5. Implications + scope

- **192 → ~222 SVGs** to fetch, bake, and ship. The Cargo binary
  size grows; profile with `cargo bloat` after the swap.
- **`IconState` plumbing** is the meaningful new surface area
  outside `mde-theme`. Every consumer that renders a selectable
  icon (`crates/mde-iced-components/src/lib.rs`,
  `crates/mde-workbench/src/sidebar.rs`, the dock breadcrumb's
  nav buttons in `crates/mde-portal/`) needs to thread its
  selection state through. The "no API regression" stance from
  the prior icons.rs doc-comment doesn't survive this lock — the
  active-state fill rule is the migration's net-new behavior.
- **Mapping table at implementation time** — Claude builds the
  `manifest.toml` against Google's catalog when `.svg-swap`
  runs. Operator review happens in the commit message + PR /
  CI gate, not pre-survey. If specific mappings disappoint
  (e.g. `firewall-classic`, `workspace-dot`, `recently-viewed`),
  follow-on commits replace single rows.

---

## 6. Acceptance (translates to worklist bullets on `.svg-swap`)

- `assets/icons/material-symbols/manifest.toml` lists all 64
  Carbon → Material mappings.
- `install-helpers/fetch-material-symbols.sh` runs clean; ~222
  SVG files land on disk.
- `crates/mde-theme/src/icons.rs` updated: `carbon_name → material_name`,
  `include_bytes!` paths swapped, `IconSize` mapped to 20/24/40
  optical sizes, `IconState` enum added, `svg_bytes(size, state)`
  resolver landed.
- Every `mde-theme` consumer (Iced, workbench sidebar, portal dock)
  recompiles; selection-state callsites updated to thread
  `IconState::Active` where they were already in an active context.
- 116 / 116 `mde-theme` tests pass; test name
  `every_action_carries_a_carbon_symbolic_icon` →
  `every_action_carries_a_material_symbolic_icon`.
- `grep -rln "carbon" crates/mde-theme/src/icons.rs assets/icons/` → 0 hits.
- `assets/icons/carbon/` directory deleted.
- `install-helpers/lint-material-symbols.sh` allow-list trimmed:
  `crates/mde-theme/src/icons.rs` row removed (the swap closes
  the regression-vector that line existed for).

---

## 7. Out of scope

- Visual snapshot regression tests — HW carve-out per the
  existing UX-23 deferral. Local `make snapshots-local` runs
  ad-hoc during the swap; mesh-wide bench-tests come later.
- Per-icon design re-judgement (e.g. "should `firewall-classic`
  map to `security` or `firewall`?") — follow-on commits if
  operator disagrees with the heuristic match. Not a survey.
- Migration of the Mackes-Carbon GTK icon theme
  (`data/icons/Mackes-Carbon/scalable/apps/*.svg`) — that
  theme ships to system-installed locations + serves XFCE-era
  consumers under retirement; not load-bearing for any v2.5+
  Rust crate. Retirement tracked separately under DEAD-*.
- The Carbon design tokens (`data/css/tokens.css`,
  `data/css/carbon-layout.css`) — those drive layout / spacing
  not iconography; CSS-side retirement tracks separately.

---

## 8. Risks

- **Google Fonts URL stability** — the `materialsymbolsoutlined/<name>/wght400/<size>px.svg`
  pattern is undocumented as a long-term API. Mitigation: the
  fetch script vendors the SVGs into the repo on first run; the
  bake is reproducible without re-fetching. If Google's URL
  pattern breaks later, only the *refresh* breaks, not the
  build.
- **Mapping mismatch** — Google's Material catalog doesn't have
  a perfect equivalent for every Carbon icon (e.g. `mesh`,
  `firewall-classic`). Heuristic policy means Claude picks the
  closest match; some will be controversial. Mitigation: every
  questionable mapping gets called out in the `.svg-swap`
  commit body so the operator can revert single rows.
- **`IconState` thread-through churn** — every consumer in
  `crates/mde-iced-components/`, `crates/mde-workbench/`,
  `crates/mde-portal/` that renders icons in a selectable
  context needs an `IconState::Active` argument added. Mitigation:
  default `IconState::Idle` on every callsite at swap time; only
  surface that *currently* implements selection state explicitly
  threads the `Active` value. Subsequent commits widen the
  active-state coverage as needed.

---

## 9. Companion doc

`docs/design/voice-and-tone.md` line 30 already cites Material
Symbols (per Q43 + EPIC-UI-MATERIAL). Memory
[[project_ux_polish_locks]] line 67 carries the supersession
marker. This doc is the implementation-detail companion that
those two reference.
