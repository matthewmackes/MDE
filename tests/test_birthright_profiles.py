"""Tests for INST-8 — profile-aware birthright CLI in ``mackes/birthright.py``.

INST-8 adds ``--profile=lighthouse|headless|full`` gating + the
``apply`` CLI dispatch (which fixes the previously-dead
``python3 -m mackes.birthright apply <steps>`` contract that
``mde-wizard`` builds).

Acceptance covered here (each from the INST-8 task body):

  1. Each profile's step list is exactly what the locked profile
     matrix says — lighthouse ⊂ headless ⊂ full (no extra, no missing).
  2. Every step-id mapped into a profile resolves to a real
     ``apply_*`` function (no dangling ids).
  3. No profile has an empty step set (a programming error).
  4. ``--profile`` is required when no explicit steps are given (the
     module refuses to guess) and unknown profiles are rejected.
  5. ``list`` exits 0; a dry-run profile plan exits 0 and runs nothing.
"""
from __future__ import annotations

import mackes.birthright as b


def test_profiles_constant() -> None:
    assert b.PROFILES == ("lighthouse", "headless", "full")


def test_matrix_nesting_lighthouse_subset_headless_subset_full() -> None:
    light = set(b.PROFILE_STEPS["lighthouse"])
    headless = set(b.PROFILE_STEPS["headless"])
    full = set(b.PROFILE_STEPS["full"])
    assert light < headless, "lighthouse must be a strict subset of headless"
    assert headless < full, "headless must be a strict subset of full"


def test_exact_step_sets_per_matrix() -> None:
    # Mesh substrate runs on every profile.
    assert b.PROFILE_STEPS["lighthouse"] == b._MESH_STEPS
    # Headless adds fleet ansible-pull + monitoring + clipboard, no desktop.
    assert b.PROFILE_STEPS["headless"] == b._MESH_STEPS + b._HEADLESS_EXTRA
    # Full adds the Wayland desktop on top of headless.
    assert (
        b.PROFILE_STEPS["full"]
        == b._MESH_STEPS + b._HEADLESS_EXTRA + b._DESKTOP_EXTRA
    )


def test_no_desktop_steps_on_non_desktop_profiles() -> None:
    desktop = set(b._DESKTOP_EXTRA)
    assert not (set(b.PROFILE_STEPS["lighthouse"]) & desktop)
    assert not (set(b.PROFILE_STEPS["headless"]) & desktop)


def test_every_mapped_step_has_a_real_function() -> None:
    for prof, ids in b.PROFILE_STEPS.items():
        for sid in ids:
            assert sid in b.STEP_FUNCS, f"{prof}: step '{sid}' has no apply_* function"
            assert callable(b.STEP_FUNCS[sid])


def test_no_profile_is_empty() -> None:
    for prof, ids in b.PROFILE_STEPS.items():
        assert ids, f"profile '{prof}' has an empty step set (programming error)"


def test_retired_hyprland_step_not_mapped() -> None:
    # Hyprland was retired 2026-05-28; its baseline-conf step must not
    # run under any profile.
    all_ids = {sid for ids in b.PROFILE_STEPS.values() for sid in ids}
    for sid in all_ids:
        assert "hyprland" not in b.STEP_FUNCS[sid].__name__


def test_steps_for_profile_rejects_unknown() -> None:
    import pytest

    with pytest.raises(ValueError):
        b.steps_for_profile("nope")


def test_steps_for_profile_returns_matrix() -> None:
    assert b.steps_for_profile("headless") == list(b.PROFILE_STEPS["headless"])


def test_main_requires_profile_without_explicit_steps() -> None:
    # No subcommand, no --profile: refuse to guess.
    assert b.main([]) == 2


def test_main_rejects_unknown_explicit_step() -> None:
    assert b.main(["apply", "definitely-not-a-step"]) == 2


def test_main_list_exits_zero() -> None:
    assert b.main(["list"]) == 0


def test_main_dry_run_profile_runs_nothing(capsys) -> None:  # type: ignore[no-untyped-def]
    rc = b.main(["--profile=lighthouse", "--dry-run"])
    assert rc == 0
    out = capsys.readouterr().out
    # Dry-run announces each lighthouse step and nothing else executes.
    for sid in b.PROFILE_STEPS["lighthouse"]:
        assert sid in out
    assert "[dry-run]" in out
