# tests/acceptance/ — bench-acceptance gate

This directory holds the six bench-acceptance tests locked in
`docs/design/v2.5-nebula-fabric.md` § "Acceptance criteria for v2.5
cut" and tracked as NF-9.1..NF-9.6 in `docs/PROJECT_WORKLIST.md`.

## How to run

The default `pytest tests/` invocation skips the whole module:

```
$ pytest tests/acceptance/test_nebula_fabric.py -v
SKIPPED [6] ... reason: No Nebula bench fleet ...
```

To actually run the gate, an operator points the
`MDE_NEBULA_BENCH_FLEET` env var at a fleet topology JSON file (see
[`docs/help/bench-acceptance.md`](../../docs/help/bench-acceptance.md)
for the schema):

```
$ export MDE_NEBULA_BENCH_FLEET=/etc/mde/bench-001.json
$ pytest tests/acceptance/test_nebula_fabric.py -v
```

The bench fleet is the operator's responsibility — `mackesd` does
not provision it. The six-peer reference fleet locked in the v2.5
design doc is the canonical target.

## Layout

```
tests/acceptance/
├── README.md                      — this file
├── __init__.py
├── test_nebula_fabric.py          — the six pytest functions
└── lib/                           — bash helpers shipped over ssh
    ├── block_udp_egress.sh
    ├── enroll_peer.sh
    ├── link_flap.sh
    ├── restore_udp_egress.sh
    └── stop_mackesd.sh
```

The bash helpers run on the bench hosts themselves. They are bash
on purpose — bench hosts are minimum-footprint Fedora installs and
we don't want to drag a Python runtime onto them just to flip an
interface or stop a unit.

## §0.12 stub-policy exception

CLAUDE.md §0.12 forbids stubs in committed code. The six functions
in `test_nebula_fabric.py` are **not** stubs:

- Each function is the canonical executable form of one bench
  scenario from the design lock — it is the gate, not a placeholder
  for one.
- The functions skip via `pytestmark = pytest.mark.skipif(...)` when
  `MDE_NEBULA_BENCH_FLEET` is unset, which is the only environment
  in which they cannot run. A skip with a clear reason is the
  documented pytest pattern for "this test depends on an external
  resource that the current environment lacks" — `tests/` already
  uses the same pattern for X-server-dependent tests
  (`test_panel_xvfb_smoke.py`) and tailscale-binary-dependent tests
  (legacy `test_mesh_vpn.py`).
- They never lie about success — a skip is reported as skipped, not
  as passed. CI's default jobs do not gate on these tests.

The CI `acceptance` job is gated behind `workflow_dispatch` with a
`bench_fleet_url` input so the gate only runs on operator demand,
against a fleet the operator stood up.

## E2E self-verification recipe

The four checks below verify the scaffolding itself, independent of
any actual bench fleet:

1. `pytest tests/acceptance/test_nebula_fabric.py --collect-only`
   lists the six scenarios.
2. `pytest tests/acceptance/test_nebula_fabric.py -v` skips all six
   (because `MDE_NEBULA_BENCH_FLEET` is unset).
3. `MDE_NEBULA_BENCH_FLEET=/dev/null pytest
   tests/acceptance/test_nebula_fabric.py -v` errors cleanly with a
   "fleet config invalid" message rather than crashing.
4. `make lint` passes.
