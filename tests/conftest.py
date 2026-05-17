"""Shared test fixtures.

These tests redirect Mackes' XDG state directories into a tmp_path so they
never touch the developer's real ~/.config or ~/.local/share.
"""
from __future__ import annotations

import importlib
import os
import sys
from pathlib import Path

import pytest


REPO_ROOT = Path(__file__).resolve().parent.parent


@pytest.fixture
def isolated_xdg(tmp_path, monkeypatch):
    """Re-root Mackes' XDG paths into tmp_path and reload the state module.

    Returns a dict with 'home', 'config', 'data', 'snapshots', 'logs'.
    """
    home = tmp_path / "home"
    config = home / ".config"
    data = home / ".local" / "share"
    for d in (home, config, data):
        d.mkdir(parents=True, exist_ok=True)

    monkeypatch.setenv("HOME", str(home))
    monkeypatch.setenv("XDG_CONFIG_HOME", str(config))
    monkeypatch.setenv("XDG_DATA_HOME", str(data))

    # Re-import state-touching modules so their HOME constants pick up the
    # patched environment variables. Drop the entire `mackes.*` namespace so
    # cached package attributes on `mackes` (e.g. mackes.menu_integration)
    # don't keep pointing at the stale module objects after submodule purge.
    for mod_name in [m for m in list(sys.modules) if m == "mackes" or m.startswith("mackes.")]:
        # Keep mackes.app / mackes.workbench / mackes.wizard out of this purge —
        # they import GTK at import time, so reloading them in tests would
        # fail. Only drop the pure-backend modules + the package itself.
        if mod_name in {"mackes", "mackes.state", "mackes.logging",
                        "mackes.snapshots", "mackes.menu_integration",
                        "mackes.presets", "mackes.app_mgmt",
                        "mackes.uninstall", "mackes.xfconf_bridge",
                        "mackes.qnm_bridge"}:
            del sys.modules[mod_name]

    sys.path.insert(0, str(REPO_ROOT))
    import mackes.state
    importlib.reload(mackes.state)
    mackes.state.ensure_dirs()

    # Keep the runtime preset lookups inside the dev tree so an
    # already-installed `/usr/share/mackes-shell/` from a previous RPM build
    # doesn't shadow the local repo's data dirs during tests.
    import mackes.presets as _presets
    monkeypatch.setattr(_presets, "SHIPPED_PRESET_DIRS",
                        [REPO_ROOT / "data" / "presets"])

    yield {
        "home": home,
        "config": mackes.state.CONFIG_DIR,
        "data": mackes.state.DATA_DIR,
        "snapshots": mackes.state.SNAPSHOT_DIR,
        "logs": mackes.state.LOG_DIR,
    }
