"""Tests for `apply_uninstall_legacy_xfce` (Phase 10.6.6).

Three behavioral guarantees the step has to satisfy:

  (a) panel-swap prerequisite — refuses to call dnf when mackes-panel
      isn't running or apply_panel_swap hasn't written the user-side
      xfce4-panel.desktop autostart override with Hidden=true.

  (b) idempotency — if none of the six canonical packages are installed
      (already removed, or never installed), the step skips the dnf
      call cleanly.

  (c) command shape — when both gates pass and at least one package is
      installed, the step issues exactly ONE `dnf remove -y` invocation
      listing the canonical six packages.

All three tests mock `shutil.which`, `subprocess.run`, and the
AdminSession runner so they never touch the developer's real dnf
or pgrep. No fixtures used — the file is _run_without_pytest-compatible.
"""
from __future__ import annotations

import os
import tempfile
from pathlib import Path
from typing import List
from unittest.mock import patch


# ---------------------------------------------------------------------------
# Test scaffolding
# ---------------------------------------------------------------------------


# 1.1.3 — xfce4-panel dropped from the tuple (the C panel-plugin
# in data/panel-plugins/mackes-clipboard/ still links its library;
# removing the package would break our own RPM's link line).
_LEGACY = (
    "xfdesktop",
    "xfce4-whiskermenu-plugin",
    "xfce4-docklike-plugin",
    "xfce4-pulseaudio-plugin",
    "xfce4-power-manager-plugin",
)


class _FakeProc:
    """subprocess.run() return-value stand-in (returncode-only)."""

    __slots__ = ("returncode", "stdout", "stderr")

    def __init__(self, rc: int = 0, stdout: str = "", stderr: str = "") -> None:
        self.returncode = rc
        self.stdout = stdout
        self.stderr = stderr


def _make_subprocess_run(scenarios: dict):
    """Return a callable suitable to replace `subprocess.run` in the
    birthright module. `scenarios` is a dict of {first-arg: _FakeProc}
    keyed on the command's argv[0] (`pgrep`, `rpm`, etc.). A "default"
    key catches unmatched calls."""
    def run(cmd, *a, **kw):
        # Allow per-cmd[0:2] override (e.g. (rpm,-q,xfce4-panel))
        key = tuple(cmd[:2]) if len(cmd) >= 2 else (cmd[0] if cmd else "default",)
        if key in scenarios:
            return scenarios[key]
        if cmd and cmd[0] in scenarios:
            return scenarios[cmd[0]]
        return scenarios.get("default", _FakeProc(rc=0))
    return run


def _make_run(stdout: str = "", rc: int = 0):
    """Replacement for birthright._run that returns (rc, out)."""
    def fake_run(cmd, *, timeout=60):
        return rc, stdout
    return fake_run


def _make_run_root(captured: List[list]):
    """Replacement for birthright._run_root that appends each call's
    argv into `captured` and returns (0, '')."""
    def fake_run_root(cmd, *, timeout=300):
        captured.append(list(cmd))
        return 0, ""
    return fake_run_root


def _make_run_root_failing(captured: List[list], rc: int = 1,
                            output: str = "Error: nothing to do"):
    def fake_run_root(cmd, *, timeout=300):
        captured.append(list(cmd))
        return rc, output
    return fake_run_root


# ---------------------------------------------------------------------------
# (a) Prerequisite detection
# ---------------------------------------------------------------------------


def test_skips_when_mackes_panel_not_running():
    """Gate (a): pgrep -x mackes-panel returns rc=1 → step is a no-op."""
    from mackes import birthright

    # pgrep -x mackes-panel → not found (rc=1). dnf is present so we
    # get past the first early-return.
    sp_scenarios = {
        ("pgrep", "-x"): _FakeProc(rc=1),
        "default":      _FakeProc(rc=0),
    }
    captured: List[list] = []
    with patch.object(birthright.shutil, "which", side_effect=lambda c: f"/usr/bin/{c}"):
        with patch.object(birthright.subprocess, "run",
                          side_effect=_make_subprocess_run(sp_scenarios)):
            with patch.object(birthright, "_run_root",
                              side_effect=_make_run_root(captured)):
                with patch.object(birthright, "_run",
                                  side_effect=_make_run()):
                    actions = birthright.apply_uninstall_legacy_xfce(None)

    assert captured == [], (
        f"expected no _run_root calls when mackes-panel not running; got {captured}"
    )
    joined = "\n".join(actions)
    assert "panel-swap prerequisite not met" in joined, joined
    assert "mackes-panel is not running" in joined, joined


def test_skips_when_autostart_override_missing():
    """Gate (a) part 2: even with mackes-panel running, no autostart
    override (~/.config/autostart/xfce4-panel.desktop) means the swap
    didn't finish on this account."""
    from mackes import birthright

    sp_scenarios = {
        ("pgrep", "-x"): _FakeProc(rc=0),  # mackes-panel IS running
        "default":      _FakeProc(rc=0),
    }
    captured: List[list] = []
    # Point HOME at an empty tmpdir so the autostart file doesn't exist
    # AND xfce4-panel exists at /usr/bin so we don't short-circuit on
    # "no xfce4-panel ever installed".
    with tempfile.TemporaryDirectory() as td:
        old_home = os.environ.get("HOME")
        os.environ["HOME"] = td
        try:
            def which(cmd):
                # Pretend everything is available, including xfce4-panel.
                return f"/usr/bin/{cmd}"
            with patch.object(birthright.shutil, "which", side_effect=which):
                with patch.object(birthright.subprocess, "run",
                                  side_effect=_make_subprocess_run(sp_scenarios)):
                    with patch.object(birthright, "_run_root",
                                      side_effect=_make_run_root(captured)):
                        with patch.object(birthright, "_run",
                                          side_effect=_make_run()):
                            actions = birthright.apply_uninstall_legacy_xfce(None)
        finally:
            if old_home is not None:
                os.environ["HOME"] = old_home
            else:
                del os.environ["HOME"]

    assert captured == [], (
        f"expected no _run_root call when autostart override missing; got {captured}"
    )
    joined = "\n".join(actions)
    assert "panel-swap prerequisite not met" in joined, joined
    assert "apply_panel_swap" in joined or "hasn't been run" in joined, joined


# ---------------------------------------------------------------------------
# (b) Idempotency — no packages installed → no dnf call
# ---------------------------------------------------------------------------


def test_idempotent_when_no_packages_installed():
    """Gate (b): apply_panel_swap has succeeded AND none of the six
    legacy packages are present → still no-op. Re-runs after the first
    cleanup should never re-shell out to dnf."""
    from mackes import birthright

    sp_scenarios = {
        ("pgrep", "-x"): _FakeProc(rc=0),  # mackes-panel running
        "default":      _FakeProc(rc=0),
    }
    captured_root: List[list] = []
    captured_run: List[list] = []

    def fake_run(cmd, *, timeout=60):
        captured_run.append(list(cmd))
        # `rpm -q <pkg>` always reports not-installed (rc=1).
        if cmd and cmd[0] == "rpm" and cmd[1:2] == ["-q"]:
            return 1, "package not installed"
        return 0, ""

    with tempfile.TemporaryDirectory() as td:
        # Stage the autostart override so the panel-swap gate passes.
        autostart = Path(td) / ".config" / "autostart" / "xfce4-panel.desktop"
        autostart.parent.mkdir(parents=True, exist_ok=True)
        autostart.write_text(
            "[Desktop Entry]\nType=Application\nHidden=true\n"
            "X-XFCE-Autostart-enabled=false\n", encoding="utf-8")
        old_home = os.environ.get("HOME")
        os.environ["HOME"] = td
        try:
            with patch.object(birthright.shutil, "which",
                              side_effect=lambda c: f"/usr/bin/{c}"):
                with patch.object(birthright.subprocess, "run",
                                  side_effect=_make_subprocess_run(sp_scenarios)):
                    with patch.object(birthright, "_run_root",
                                      side_effect=_make_run_root(captured_root)):
                        with patch.object(birthright, "_run", side_effect=fake_run):
                            actions = birthright.apply_uninstall_legacy_xfce(None)
        finally:
            if old_home is not None:
                os.environ["HOME"] = old_home
            else:
                del os.environ["HOME"]

    # The step should have rpm-queried all six packages (idempotency probe).
    rpm_q_calls = [c for c in captured_run if c[:2] == ["rpm", "-q"]]
    assert len(rpm_q_calls) == len(_LEGACY), (
        f"expected an rpm -q probe for each of {len(_LEGACY)} legacy "
        f"packages; got {len(rpm_q_calls)} ({rpm_q_calls})"
    )
    # …and skipped the dnf call entirely.
    assert captured_root == [], (
        f"expected no dnf call when no packages installed; got {captured_root}"
    )
    joined = "\n".join(actions)
    assert "no legacy XFCE packages installed" in joined, joined


# ---------------------------------------------------------------------------
# (c) The right dnf remove invocation
# ---------------------------------------------------------------------------


def test_emits_canonical_dnf_remove_command():
    """Gate (c): when both gates pass and at least one of the six
    packages is installed, the step calls `_run_root` exactly once
    with the canonical `dnf remove -y <six packages>` argv."""
    from mackes import birthright

    sp_scenarios = {
        ("pgrep", "-x"): _FakeProc(rc=0),
        "default":      _FakeProc(rc=0),
    }
    captured_root: List[list] = []

    def fake_run(cmd, *, timeout=60):
        # Pretend xfdesktop + xfce4-whiskermenu-plugin are installed
        # (xfce4-panel is no longer in the canonical tuple — see the
        # 1.1.3 changelog).
        if cmd[:2] == ["rpm", "-q"]:
            if cmd[2] in ("xfdesktop", "xfce4-whiskermenu-plugin"):
                return 0, f"{cmd[2]}-4.x"
            return 1, "package not installed"
        return 0, ""

    with tempfile.TemporaryDirectory() as td:
        autostart = Path(td) / ".config" / "autostart" / "xfce4-panel.desktop"
        autostart.parent.mkdir(parents=True, exist_ok=True)
        autostart.write_text(
            "[Desktop Entry]\nType=Application\nHidden=true\n",
            encoding="utf-8")
        old_home = os.environ.get("HOME")
        os.environ["HOME"] = td
        try:
            with patch.object(birthright.shutil, "which",
                              side_effect=lambda c: f"/usr/bin/{c}"):
                with patch.object(birthright.subprocess, "run",
                                  side_effect=_make_subprocess_run(sp_scenarios)):
                    with patch.object(birthright, "_run_root",
                                      side_effect=_make_run_root(captured_root)):
                        with patch.object(birthright, "_run", side_effect=fake_run):
                            actions = birthright.apply_uninstall_legacy_xfce(None)
        finally:
            if old_home is not None:
                os.environ["HOME"] = old_home
            else:
                del os.environ["HOME"]

    assert len(captured_root) == 1, (
        f"expected exactly one dnf remove call; got {len(captured_root)} "
        f"({captured_root})"
    )
    cmd = captured_root[0]
    assert cmd[0] == "dnf", f"argv[0] should be dnf, got {cmd!r}"
    assert cmd[1] == "remove", f"argv[1] should be remove, got {cmd!r}"
    assert cmd[2] == "-y", f"argv[2] should be -y, got {cmd!r}"
    # Every canonical package must appear, in the locked order.
    assert tuple(cmd[3:]) == _LEGACY, (
        f"argv tail should be the six legacy packages in lock order;\n"
        f"  got      {cmd[3:]!r}\n  expected {list(_LEGACY)!r}"
    )
    joined = "\n".join(actions)
    assert "uninstall-legacy-xfce: removed" in joined, joined
    # The action log should mention what was actually installed (not the
    # whole canonical list) so the user sees what dnf actually dropped.
    assert "xfdesktop" in joined and "xfce4-whiskermenu-plugin" in joined, joined


# ---------------------------------------------------------------------------
# (d) Failure path — dnf returned non-zero
# ---------------------------------------------------------------------------


def test_dnf_failure_is_reported_not_raised():
    """Birthright steps never raise — failures land in the action log."""
    from mackes import birthright

    sp_scenarios = {
        ("pgrep", "-x"): _FakeProc(rc=0),
        "default":      _FakeProc(rc=0),
    }
    captured: List[list] = []

    def fake_run(cmd, *, timeout=60):
        if cmd[:2] == ["rpm", "-q"] and cmd[2] == "xfdesktop":
            return 0, "xfdesktop-4.x"
        if cmd[:2] == ["rpm", "-q"]:
            return 1, ""
        return 0, ""

    with tempfile.TemporaryDirectory() as td:
        autostart = Path(td) / ".config" / "autostart" / "xfce4-panel.desktop"
        autostart.parent.mkdir(parents=True, exist_ok=True)
        autostart.write_text("[Desktop Entry]\nHidden=true\n", encoding="utf-8")
        old_home = os.environ.get("HOME")
        os.environ["HOME"] = td
        try:
            with patch.object(birthright.shutil, "which",
                              side_effect=lambda c: f"/usr/bin/{c}"):
                with patch.object(birthright.subprocess, "run",
                                  side_effect=_make_subprocess_run(sp_scenarios)):
                    with patch.object(birthright, "_run_root",
                                      side_effect=_make_run_root_failing(
                                          captured, rc=1,
                                          output="Error: yum lock held")):
                        with patch.object(birthright, "_run", side_effect=fake_run):
                            actions = birthright.apply_uninstall_legacy_xfce(None)
        finally:
            if old_home is not None:
                os.environ["HOME"] = old_home
            else:
                del os.environ["HOME"]

    joined = "\n".join(actions)
    assert "dnf remove failed" in joined, joined
    assert "yum lock held" in joined, joined


# ---------------------------------------------------------------------------
# (e) Spec audit: every legacy package is Obsoleted by the renamed RPM
# ---------------------------------------------------------------------------


def test_spec_does_not_obsolete_legacy_xfce_packages():
    """1.1.4 — spec must NOT Obsolete the legacy XFCE packages.
    libdnf5 ≤ 5.2.x trips an internal assertion when our install
    transaction carries 4+ implicit erases. Cleanup happens
    at runtime via apply_uninstall_legacy_xfce instead."""
    repo_root = Path(__file__).resolve().parent.parent
    spec_path = repo_root / "packaging" / "fedora" / "mackes-shell.spec"
    text = spec_path.read_text(encoding="utf-8")
    forbidden = [
        "xfce4-panel",
        "xfdesktop",
        "xfce4-whiskermenu-plugin",
        "xfce4-docklike-plugin",
        "xfce4-pulseaudio-plugin",
        "xfce4-power-manager-plugin",
    ]
    leaks = []
    for line in text.splitlines():
        stripped = line.lstrip()
        if not stripped.startswith("Obsoletes:"):
            continue
        rhs = stripped.split(":", 1)[1].strip()
        head = rhs.split(None, 1)[0]
        if head in forbidden:
            leaks.append(head)
    assert not leaks, (
        f"spec ships Obsoletes: lines for {leaks} — these trigger the "
        "dnf5 implicit_ts_elements assertion on install. Remove them; "
        "apply_uninstall_legacy_xfce handles the runtime cleanup."
    )


# ---------------------------------------------------------------------------
# (f) v2.0.1 hotfix — apply_uninstall_legacy_xsessions
# ---------------------------------------------------------------------------


def test_uninstall_legacy_xsessions_is_noop_when_nothing_present():
    """Idempotent: when none of _LEGACY_XSESSIONS exist on disk the
    step logs a skip and never invokes _run_root."""
    from mackes import birthright

    captured: List[list] = []

    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        # Point all entries at a tmp dir that doesn't carry any of
        # the canonical orphan filenames.
        fake_set = tuple(str(tmp_path / Path(p).name)
                          for p in birthright._LEGACY_XSESSIONS)
        with patch.object(birthright, "_LEGACY_XSESSIONS", fake_set):
            with patch.object(birthright, "_run_root",
                              side_effect=_make_run_root(captured)):
                actions = birthright.apply_uninstall_legacy_xsessions(None)

    joined = "\n".join(actions)
    assert "nothing to remove" in joined, joined
    assert captured == [], f"expected no _run_root calls, got {captured}"


def test_uninstall_legacy_xsessions_removes_present_files():
    """When ANY orphan xsession file is on disk, the step issues a
    single `rm -f` listing exactly those files (not the whole tuple)."""
    from mackes import birthright

    captured: List[list] = []

    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        # Materialize TWO of the three canonical orphans.
        present_names = (
            "xfce11-i3-plank.desktop",
            "xfce11.desktop",
        )
        # All three names are tracked in the allow-list, but only
        # two exist on disk; the third stays missing to prove the
        # rm call uses the *present* subset, not the whole tuple.
        all_names = present_names + ("mackes.desktop",)
        fake_set = tuple(str(tmp_path / n) for n in all_names)
        for name in present_names:
            (tmp_path / name).write_text(
                "[Desktop Entry]\nName=Legacy\n", encoding="utf-8")

        with patch.object(birthright, "_LEGACY_XSESSIONS", fake_set):
            with patch.object(birthright, "_run_root",
                              side_effect=_make_run_root(captured)):
                actions = birthright.apply_uninstall_legacy_xsessions(None)

    assert len(captured) == 1, f"expected exactly 1 rm call, got {captured}"
    rm_argv = captured[0]
    assert rm_argv[:2] == ["rm", "-f"], rm_argv
    rm_targets = set(rm_argv[2:])
    expected = {str(tmp_path / n) for n in present_names}
    assert rm_targets == expected, (rm_targets, expected)

    joined = "\n".join(actions)
    assert "removed " in joined, joined
    for n in present_names:
        assert n in joined, (n, joined)


def test_uninstall_legacy_xsessions_reports_rm_failure():
    """A failing `rm` call is reported in the action log (not raised)."""
    from mackes import birthright

    captured: List[list] = []

    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        target = tmp_path / "xfce11-i3-plank.desktop"
        target.write_text("[Desktop Entry]\nName=Legacy\n", encoding="utf-8")
        fake_set = (str(target),)

        with patch.object(birthright, "_LEGACY_XSESSIONS", fake_set):
            with patch.object(birthright, "_run_root",
                              side_effect=_make_run_root_failing(
                                  captured, rc=1,
                                  output="rm: cannot remove: Permission denied")):
                actions = birthright.apply_uninstall_legacy_xsessions(None)

    joined = "\n".join(actions)
    assert "rm failed" in joined, joined
    assert "Permission denied" in joined, joined


def test_uninstall_legacy_xsessions_allowlist_contains_canonical_entry():
    """Spec audit — the v1.x xfce11-i3-plank xsession (the one
    LightDM picks up on the bug-report box) must be in the
    allow-list so birthright actually sweeps it."""
    from mackes import birthright
    assert "/usr/share/xsessions/xfce11-i3-plank.desktop" \
        in birthright._LEGACY_XSESSIONS
