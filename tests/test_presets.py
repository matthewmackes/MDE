"""Preset discovery and loading.

The roster as of the 2026-05-16 redesign:
  hashbang (default, '#!') · mackes · daylight · vanilla
"""
from __future__ import annotations

import pytest

yaml = pytest.importorskip("yaml")


def test_list_presets_ships_four(isolated_xdg):
    from mackes.presets import list_presets
    names = [p.name for p in list_presets()]
    assert set(names) == {"hashbang", "mackes", "daylight", "vanilla"}, names


def test_default_preset_is_hashbang(isolated_xdg):
    from mackes.presets import default_preset, DEFAULT_PRESET_NAME
    p = default_preset()
    assert p is not None
    assert p.name == DEFAULT_PRESET_NAME == "hashbang"


def test_default_preset_sorted_first(isolated_xdg):
    from mackes.presets import list_presets, DEFAULT_PRESET_NAME
    names = [p.name for p in list_presets()]
    assert names[0] == DEFAULT_PRESET_NAME


def test_hashbang_has_minimal_modern_apps(isolated_xdg):
    from mackes.presets import load_preset
    p = load_preset("hashbang")
    assert p is not None
    install = p.apps.get("install", [])
    # Spirit-of-CrunchBang: alacritty/neovim/firefox/mpv/conky
    assert "alacritty" in install
    assert "neovim" in install
    # XFCE extras get removed via the combined Bloat list (Q15)
    bloat = p.apps.get("remove_bloat", [])
    assert "asunder" in bloat
    assert "xfburn" in bloat


def test_vanilla_is_invisible(isolated_xdg):
    from mackes.presets import load_preset
    p = load_preset("vanilla")
    assert p is not None
    # Vanilla means "don't touch": empty install, empty bloat-removal.
    assert p.apps.get("install", []) == []
    assert p.apps.get("remove_bloat", []) == []


def test_each_preset_has_required_top_level_keys(isolated_xdg):
    from mackes.presets import list_presets
    for p in list_presets():
        assert p.name, f"{p.source_path}: blank name"
        assert p.display_name, f"{p.source_path}: blank display_name"
        # description allowed to be empty
        # appearance is a dict (possibly empty)
        assert isinstance(p.appearance, dict)


def test_user_preset_overrides_shipped(isolated_xdg):
    from mackes.presets import list_presets
    user_dir = isolated_xdg["config"] / "presets"
    user_dir.mkdir(parents=True, exist_ok=True)
    (user_dir / "hashbang.yaml").write_text(
        "name: hashbang\n"
        "display_name: '#! (USER OVERRIDE)'\n"
        "description: overridden\n",
        encoding="utf-8",
    )
    presets = {p.name: p for p in list_presets()}
    assert "hashbang" in presets
    assert presets["hashbang"].display_name == "#! (USER OVERRIDE)"
