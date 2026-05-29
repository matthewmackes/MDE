---
name: audit
description: Bidirectional codebase sweep for incomplete & unwired ideas — finds orphaned / half-wired / stub / dead code where intent and implementation diverge. Five finding categories (DECLARED-UNREACHED / REACHED-NOOP / HALF-WIRED / PROMISED-ABSENT / ABANDONED-SCAFFOLD) crossed with eight failure-mode passes; every finding gets a FINISH-or-REMOVE verdict and is lifted into the canonical worklist. Use when the user says "audit the codebase", "find incomplete / unwired / dead / half-implemented code", "find stubs", "find orphaned features", "bidirectional sweep", "what's unfinished", or "find false-done tasks". The manual catch standing behind the §0.12 no-stubs + §0.8 runtime-reachability + §0.13 continuous-retirement lints — it finds the drift those lints can't grep for. Sister skills: `plan` (rescue-pass + lift findings into the worklist), `ship` (drain the resulting queue), `release` (cut once green).
---

# Audit

**Codebase Audit Directive — Incomplete & Unwired Ideas.**

A systematic sweep for the gap between *intent* and *implementation*:
code that was declared but never reached, wired but left empty,
connected one-way, promised but absent, or scaffolded and abandoned.
This is the manual catch behind the mechanical lints — it finds the
drift `grep` can't, the exact failure mode the v3.x dead-module audit
(2026-05-22) surfaced by hand: 13 of 18 panel modules marked
`[✓] shipped` while sitting dead at runtime, costing four user-visible
bugs. See [`docs/V3_RUNTIME_INTEGRATION_AUDIT.md`](../../../docs/V3_RUNTIME_INTEGRATION_AUDIT.md).

## Triggers

- "Audit the codebase" / "run the audit" / "bidirectional sweep"
- "Find incomplete / unwired / dead / half-implemented code"
- "Find stubs" / "find orphaned features" / "find false-done tasks"
- "What's unfinished?" / "what's marked done but isn't wired?"
- Before a `cut release` — a pre-cut integrity pass over the core epics
- As the deeper version of the `plan` worklist-rescue pass

## Mission

A **bidirectional sweep**, both directions required:

1. **Intent → implementation:** every intended feature (worklist task,
   design-doc action, declared symbol, UI affordance) is fully
   implemented and reachable.
2. **Implementation → intent:** every implemented piece (module,
   handler, state key, flag, abstraction) serves a documented,
   reachable purpose.

A thing passes only when both directions close. Half a connection is a
finding.

## Finding categories

Tag every finding with exactly one:

1. **DECLARED-UNREACHED** — defined but nothing calls or triggers it.
2. **REACHED-NOOP** — wired, but the body is a stub: empty,
   early-return, `TODO`, throws "not implemented."
3. **HALF-WIRED** — one-directional: read without write, write without
   read, event emitted with no listener.
4. **PROMISED-ABSENT** — references something that doesn't exist: a
   named target with no definition, a route/handler pointing nowhere.
5. **ABANDONED-SCAFFOLD** — commented-out blocks, flags gating nothing,
   single-empty-impl abstractions, leftover experiment branches.

## Eight failure-mode passes

Run each pass across the audited scope. Each targets a distinct way
intent and implementation diverge:

1. **Trigger wiring** — every interactive element / event source / hook
   has a connected, non-empty handler.
2. **State round-trip symmetry** — every persisted or shared state key
   is both written and read.
3. **Named-reference resolution** — every identifier that names a thing
   resolves to a real definition.
4. **Paired-operation coverage** — for every operation pair
   (serialize/deserialize, save/load), the two halves cover the
   identical set.
5. **Monitoring / validation completeness** — a managed item excluded
   from the check is a blind spot.
6. **Lifecycle commit points** — every step that collects a value has a
   commit/finish path that actually persists it.
7. **Invariant violations** — anything that exists despite a stated
   design constraint.
8. **Build / package / deploy claims** — every step a build script
   claims to perform has a real implementation.

## Reporting format

One row per finding:

```
Location | Category | Evidence | Confidence | Verdict
```

- **Location** — `path:line` (use `file_path:line_number` so the
  operator can click through).
- **Category** — one of the five tags above.
- **Evidence** — the concrete grep/read result that proves the gap
  (the missing caller, the empty body, the unread key). Not a guess.
- **Confidence** — high / medium / low. Low-confidence findings are
  fine; flag them as such rather than dropping them.
- **Verdict** — binary: **FINISH** (build the missing half) or
  **REMOVE** (delete the orphan), with a one-line justification.

## False-positive safeguards

Do **not** flag without first verifying the wiring layer. Exclude:

- **Declaratively-wired handlers** — registered via macro / attribute /
  config rather than a direct call (e.g. `#[interface]` methods, Tera
  template hooks, `.desktop` actions, swayipc `for_window` rules).
- **Public API calls** — exported for external callers; absence of an
  in-repo caller is expected.
- **Reflection / plugins** — dispatched dynamically.
- **Framework lifecycle callbacks** — `update()` / `view()` / `Drop` /
  `Default` and the Iced/zbus/serde machinery that calls them.
- **Build-required symbols** — referenced only by the build/package
  layer.
- **Test-only code** — fixtures, mocks, `#[cfg(test)]`, `*_tests.rs`,
  `tests/`.

A symbol that *looks* orphaned but resolves through one of these layers
is not a finding. Verify the layer before writing the row.

## How it lands in mackes-shell

This directive is portable; here is how it binds to this repo.

### Project-specific recipes per pass

- **Runtime reachability (DECLARED-UNREACHED)** — the §0.8 gate-7 test:
  for a Rust module `foo`, `grep -rln "foo::" --include='*.rs'
  crates/<crate>` must return a file other than `foo.rs` itself, or a
  `pub use foo::*` re-export must exist. For Python, at least one
  `import`/`from … import` from outside the module's own file. The
  `install-helpers/lint-runtime-reachability.sh` lint automates the
  module-level case; this pass extends it to functions and handlers the
  lint can't see.
- **REACHED-NOOP** — `install-helpers/lint-no-stubs.sh` catches
  `todo!()` / `unimplemented!()` / `panic!("not yet …")`; the
  voice-tone lint catches the user-visible "coming soon" / "TBD" / "WIP"
  side. This pass catches the silent stubs they miss: empty `Ok(())`
  arms, `tracing::info!("… not yet implemented")` branches, match arms
  that log-and-exit-0 (see the grandfathered `Kind::Network` stub in
  `crates/mde-popover/src/main.rs`).
- **HALF-WIRED** — Bus topics published with no subscriber (or
  subscribed with no publisher); GFS state files written by one worker,
  read by none; D-Bus signals emitted into the void.
- **PROMISED-ABSENT** — worklist tasks marked `[✓]` whose named file /
  symbol / behavior doesn't exist or isn't committed (the **false-done**
  case — the heartbeat-fix-uncommitted-but-marked-done pattern); design-
  doc actions never lifted into the worklist; `[[memory-link]]` targets
  with no file.
- **INVARIANT violations (pass 7)** — the locked platform constraints
  are the design ones to check against: net-new public ports outside
  UDP/4242 + TCP/443 (§0.7 #10), net-new `#[interface]` D-Bus blocks
  (§0.7 #8), Carbon icon refs (§0.7 #9), legacy mesh vocab (§0.7 #7).
  Each has a lint; this pass is the cross-check that the lints' allow-
  lists haven't quietly grown.
- **Build claims (pass 8)** — `Makefile`, `packaging/fedora/*.spec`
  (`%files` / `%posttrans` / comps group), `.github/workflows/*.yml`,
  the `install-helpers/*.sh`. INST-1's headless-ISO bug (comps group
  pulled base `mde-core` but never `mde-desktop`) is the canonical
  build-claim finding.

### Findings become worklist tasks

Per CLAUDE.md §1 (single worklist), every actionable finding is lifted
into `docs/PROJECT_WORKLIST.md` — never a side tracker, never left only
in the report. Shape (per [[feedback_no_stubs]] + §0.12):

- **REMOVE** verdict → a `DEAD-N` retirement sub-task (the §0.13
  continuous-retirement queue).
- **FINISH** verdict → a normal user-story task with bench-observable
  acceptance bullets, prefixed per §1.1.
- A false-done finding (PROMISED-ABSENT on a `[✓]` task) → flip the
  task back to `[>]`/`[ ]` with a note on the unmet §0.8 gate, **don't**
  open a duplicate.

Small, non-colliding REMOVE findings may ship in the same `/ship` cycle
per §0.13 layer 2. Larger FINISH findings queue for `/ship`.

### Verdict discipline

Default to **REMOVE** for ABANDONED-SCAFFOLD and DECLARED-UNREACHED
that no locked design doc calls for — dead code is debt, and §0.12 says
unreachable code shouldn't be in `main`. Default to **FINISH** only when
a locked task / design-doc action / user-visible affordance depends on
the missing half. When unsure which, record the finding at low
confidence and surface the fork rather than guessing — but per
[[feedback_make_recommended_choice]], lead with the recommended verdict.

## Scope control

- **Whole-repo audit** is expensive (the runtime-reachability lint alone
  is ~60–120 s). Prefer a scoped sweep: one crate, one epic's touched
  files, or the §11.1 core epics before a cut.
- Parallelize the read/grep passes (independent crates, independent
  passes) per the `ship` parallel-bundle standard.
- This skill **finds and reports**; it does not auto-fix. Fixing is a
  `/ship` action against the lifted tasks, under the standing commit/push
  auth ([[feedback_push_commit_auth]]).

## Companion skills

- `plan` — its worklist-rescue pass is the lightweight version of this;
  use `plan` to lift a batch of findings into a structured epic.
- `ship` — drains the FINISH/REMOVE tasks this audit produces.
- `release` — a pre-cut `audit` over the §11.1 core is the integrity
  gate before `cut release`.
