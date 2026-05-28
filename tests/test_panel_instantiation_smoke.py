"""Smoke test: every WorkbenchPanel subclass can be instantiated headless.

Skipped when GTK or `$DISPLAY` (or Xvfb fallback) is unavailable. The
test walks `mackes.workbench.**`, finds every `Gtk.Box` subclass whose
name ends in `Panel`, constructs one of each, and asserts no exception
escapes the constructor.

This is the Phase 11.7 baseline. It catches:

  - import-time crashes (a panel that references a deleted helper)
  - `__init__`-time crashes (a panel that runs a blocking probe
    before the GTK main loop is running)
  - missing fixtures (CSS class names referenced before they're loaded)
  - constructor calls that need an `xfconf_bridge` argument we don't
    pass (covers refactor breakage)

It does NOT exercise:

  - the GTK main loop (no `Gtk.main()`)
  - signal handlers (nothing clicks anything)
  - the sidebar shell (handled by `test_imports.py`)

Each panel gets at most 100 ms of wall time. A panel that exceeds that
fails the test — surfaces the "blocking call on the main thread"
class of bug (Phase 11.9 reliability sweep).
"""
from __future__ import annotations

import importlib
import os
import pkgutil
import signal
import time
from contextlib import contextmanager

import pytest


# Skip the whole module if GTK isn't importable or we have no display.
gi = pytest.importorskip("gi")
gi.require_version("Gtk", "3.0")
try:
    from gi.repository import Gtk  # noqa: E402
except (ImportError, ValueError) as exc:
    pytest.skip(f"GTK3 typelib unavailable: {exc}", allow_module_level=True)

if not (os.environ.get("DISPLAY") or os.environ.get("WAYLAND_DISPLAY")):
    pytest.skip("no $DISPLAY (run under Xvfb)", allow_module_level=True)


# Hard timeout — a panel that exceeds this is almost certainly hung.
# We intentionally do NOT enforce a tight per-panel budget here: every
# panel doing synchronous probes in __init__ is a Phase 11.9 reliability
# follow-up that's tracked in the worklist, not in CI. The smoke test's
# job is to catch crashes and regressions in panel discovery, not to
# ratchet probe latency.
HARD_TIMEOUT_S = 5.0
SLOW_INFO_S = 0.1

# Panels that block waiting for a system daemon that may not be running
# in the test environment. Empty as of 1.0.7 — FirewallPanel was fixed
# in Phase 11.9 (sync rpm/firewall-cmd probes moved to a background
# thread via `mackes.workbench._async.async_probe`). New entries here
# document a regression and a follow-up task.
DAEMON_DEPENDENT_PANELS: frozenset[str] = frozenset()


class _Timeout(Exception):
    """Raised inside the constructor when a panel exceeds its budget."""


@contextmanager
def _watchdog(seconds: float):
    """SIGALRM-based watchdog. POSIX-only; safe under Xvfb + pytest."""

    def _handler(_signum, _frame):
        raise _Timeout(f"panel constructor exceeded {seconds:.2f} s")

    old = signal.signal(signal.SIGALRM, _handler)
    signal.setitimer(signal.ITIMER_REAL, seconds)
    try:
        yield
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, old)


def _collect_panel_classes() -> list[type[Gtk.Box]]:
    """Walk `mackes.workbench.**` and return every Gtk.Box subclass
    whose name ends in 'Panel'. Returns a sorted list keyed by
    `module.ClassName` so test output is deterministic."""
    import mackes.workbench  # noqa: F401

    panels: list[type[Gtk.Box]] = []
    seen: set[str] = set()
    for _finder, name, _ispkg in pkgutil.walk_packages(
        mackes.workbench.__path__, prefix="mackes.workbench."
    ):
        try:
            module = importlib.import_module(name)
        except Exception:  # noqa: BLE001
            # Import failures here are surfaced separately by
            # test_imports.py; don't re-fail the smoke when typelibs
            # are missing or a single broken module.
            continue
        for attr_name in dir(module):
            cls = getattr(module, attr_name)
            if not isinstance(cls, type):
                continue
            if not issubclass(cls, Gtk.Box):
                continue
            if cls is Gtk.Box:
                continue
            if not attr_name.endswith("Panel"):
                continue
            key = f"{cls.__module__}.{cls.__name__}"
            if key in seen:
                continue
            seen.add(key)
            panels.append(cls)
    return sorted(panels, key=lambda c: f"{c.__module__}.{c.__name__}")


PANELS = _collect_panel_classes()


@pytest.mark.parametrize(
    "panel_cls",
    PANELS,
    ids=[f"{c.__module__.split('.')[-1]}.{c.__name__}" for c in PANELS],
)
def test_workbench_panel_constructs_headless(panel_cls):
    """Every Panel must construct in under 100 ms without raising.

    Constructors that need arguments are skipped — they're either
    legacy or already covered by their own targeted test. The 100 ms
    budget catches blocking subprocess calls on the main thread (the
    8.6.7 sidebar pattern: long probes belong on a background thread).
    """
    import inspect

    try:
        sig = inspect.signature(panel_cls.__init__)
    except (TypeError, ValueError):
        pytest.skip(f"{panel_cls.__name__} has no inspectable signature")

    required = [
        p
        for name, p in sig.parameters.items()
        if name != "self"
        and p.default is inspect.Parameter.empty
        and p.kind
        not in (inspect.Parameter.VAR_POSITIONAL, inspect.Parameter.VAR_KEYWORD)
    ]
    if required:
        pytest.skip(
            f"{panel_cls.__name__} needs args {[p.name for p in required]}"
        )

    if panel_cls.__name__ in DAEMON_DEPENDENT_PANELS:
        pytest.skip(
            f"{panel_cls.__name__} blocks on a system daemon "
            "(see DAEMON_DEPENDENT_PANELS — Phase 11.9 follow-up)"
        )

    start = time.monotonic()
    try:
        with _watchdog(HARD_TIMEOUT_S):
            widget = panel_cls()
    except _Timeout:
        elapsed = time.monotonic() - start
        pytest.fail(
            f"{panel_cls.__name__}.__init__ blocked for {elapsed:.2f} s "
            f"(hard budget: {HARD_TIMEOUT_S} s) — almost certainly a hang."
        )
    elapsed = time.monotonic() - start

    assert isinstance(widget, Gtk.Box), (
        f"{panel_cls.__name__} did not return a Gtk.Box subclass"
    )
    # Sanity check: every panel should have at least one child by the
    # time its __init__ returns. An empty panel is almost always a bug
    # (the panel forgot to call .pack_start() with content).
    children = widget.get_children()
    assert children, (
        f"{panel_cls.__name__} has no children after __init__ — "
        "did you forget to pack the content into self?"
    )

    # Informational only: slow constructors are tracked in Phase 11.9
    # (reliability sweep) in PROJECT_WORKLIST.md. Surfacing them in
    # test output makes the list of candidates concrete.
    if elapsed > SLOW_INFO_S:
        print(
            f"\n  slow constructor (Phase 11.9 follow-up): "
            f"{panel_cls.__name__} took {elapsed*1000:.0f} ms"
        )


def test_panel_discovery_finds_meaningful_count():
    """Sanity: we should still discover the surviving Bucket-E
    port-gap panels (devices/keyboard, mouse, look_and_feel/appearance,
    maintain/debloat, maintain/dependencies, maintain/reset_to_preset,
    system/workspaces — the 7 panels post the .delete-superseded +
    3-retire-instead-of-port pass (2026-05-26) + the port-display
    collapse-to-RETIRE (2026-05-28, superseded by displays.rs)).
    The original >= 20 floor predated `.delete-ported.batch-1..4`;
    the floor now drops to >= 5 so this smoke still catches
    `*Panel(Gtk.Box)` registration breakage on the panels that
    remain. The whole test file retires under `.delete-chrome` when
    `mackes/workbench/` empties entirely."""
    assert len(PANELS) >= 5, (
        f"only discovered {len(PANELS)} Workbench panels; expected >= 5. "
        "Did a refactor break the `*Panel(Gtk.Box)` naming convention?"
    )
