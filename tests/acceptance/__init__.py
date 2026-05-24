"""Bench-acceptance test package for the v2.5 Nebula fabric rebuild.

These tests are the executable form of the six bench scenarios locked
in ``docs/design/v2.5-nebula-fabric.md`` ("Acceptance criteria for v2.5
cut") and tracked as NF-9.1..NF-9.6 in ``docs/PROJECT_WORKLIST.md``.

They are **not** stubs:

* Each test is the canonical pass/fail gate for its scenario — the
  cut is not ``[✓]`` until all six pass on the 6-peer test fleet over
  a 7-day window.
* They skip cleanly when ``MDE_NEBULA_BENCH_FLEET`` does not point at
  an operator-supplied fleet topology JSON, so they never noise the
  default ``pytest tests/`` run.

See ``tests/acceptance/README.md`` for the §0.12 stub-policy
exception and the fleet topology JSON schema.
"""
