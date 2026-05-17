"""state.json round-trip and hardware probes."""
from __future__ import annotations


def test_state_round_trip(isolated_xdg):
    from mackes.state import MackesState
    s = MackesState.load()
    assert s.provisioned is False
    assert s.active_preset is None

    s.mark_provisioned("hashbang")
    s2 = MackesState.load()
    assert s2.provisioned is True
    assert s2.active_preset == "hashbang"
    assert s2.last_apply  # ISO timestamp string


def test_hardware_summary_keys(isolated_xdg):
    from mackes.state import hardware_summary
    info = hardware_summary()
    for k in ("hostname", "cpu", "ram", "os"):
        assert k in info


def test_service_health_returns_known_states(isolated_xdg):
    from mackes.state import service_health
    for name, status in service_health().items():
        assert status in {"ok", "warn", "fail", "missing"}, name
