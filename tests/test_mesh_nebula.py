"""mesh_nebula — overlay-IP parsing, lighthouse extraction,
sshd drop-in writer, toast emitters, firewall preset, and the
canonical-service summary the NF-13.8 workbench panel consumes.

Mirrors the NF-15.5 rename — the pre-v2.5 test_mesh_vpn.py tested
the Headscale/Tailscale shim that NF-5.1 retires. This file
covers the Nebula consumer-side helpers that replace it.
"""
from __future__ import annotations

import json


# ─────────────────────────────────────────────────────────────────
# _extract_lighthouse_hosts (pure helper)
# ─────────────────────────────────────────────────────────────────


def test_extract_lighthouse_hosts_pulls_ips_from_block():
    from mackes.mesh_nebula import _extract_lighthouse_hosts
    body = (
        "pki:\n"
        "  ca: /etc/nebula/ca.crt\n"
        "lighthouse:\n"
        "  am_lighthouse: false\n"
        "  hosts:\n"
        '    - "10.42.0.1"\n'
        '    - "10.42.0.2"\n'
        "listen:\n"
        "  host: 0.0.0.0\n"
    )
    assert _extract_lighthouse_hosts(body) == ["10.42.0.1", "10.42.0.2"]


def test_extract_lighthouse_hosts_handles_unquoted_entries():
    from mackes.mesh_nebula import _extract_lighthouse_hosts
    body = (
        "lighthouse:\n"
        "  hosts:\n"
        "    - 10.42.0.1\n"
        "    - 10.42.0.2\n"
    )
    assert _extract_lighthouse_hosts(body) == ["10.42.0.1", "10.42.0.2"]


def test_extract_lighthouse_hosts_returns_empty_when_no_block():
    from mackes.mesh_nebula import _extract_lighthouse_hosts
    body = "pki:\n  ca: /etc/nebula/ca.crt\n"
    assert _extract_lighthouse_hosts(body) == []


def test_extract_lighthouse_hosts_stops_at_next_key():
    """The hosts list ends when another sibling YAML key starts."""
    from mackes.mesh_nebula import _extract_lighthouse_hosts
    body = (
        "lighthouse:\n"
        "  hosts:\n"
        '    - "10.42.0.1"\n'
        "  am_lighthouse: false\n"
        "  serve_dns: false\n"
        "listen:\n"
        "  host: 0.0.0.0\n"
    )
    # Note: am_lighthouse is itself indented under lighthouse:, so the
    # extractor's "left the list" heuristic kicks in once a non-dash
    # line appears under the hosts block.
    assert _extract_lighthouse_hosts(body) == ["10.42.0.1"]


# ─────────────────────────────────────────────────────────────────
# lighthouse_addresses — file-backed
# ─────────────────────────────────────────────────────────────────


def test_lighthouse_addresses_reads_explicit_path(tmp_path):
    from mackes.mesh_nebula import lighthouse_addresses
    cfg = tmp_path / "lighthouse-config.yaml"
    cfg.write_text(
        "lighthouse:\n"
        "  hosts:\n"
        '    - "10.42.0.7"\n'
    )
    assert lighthouse_addresses(cfg) == ["10.42.0.7"]


def test_lighthouse_addresses_returns_empty_when_path_missing(tmp_path):
    from mackes.mesh_nebula import lighthouse_addresses
    missing = tmp_path / "does-not-exist.yaml"
    # When the explicit path is missing AND the alt /etc/nebula/config.yaml
    # is also absent, the helper returns [].
    assert lighthouse_addresses(missing) == []


# ─────────────────────────────────────────────────────────────────
# current_overlay_ip — monkeypatch the subprocess call
# ─────────────────────────────────────────────────────────────────


def test_current_overlay_ip_parses_ips_line(tmp_path, monkeypatch):
    from mackes import mesh_nebula
    cert = tmp_path / "host.crt"
    cert.write_text("pretend-cert-bytes")

    monkeypatch.setattr(mesh_nebula.shutil, "which", lambda _: "/usr/bin/nebula-cert")
    fake_out = type("R", (), {
        "returncode": 0,
        "stdout": (
            "NebulaCertificate {\n"
            "  Name: laptop-mm\n"
            "  Ips: [10.42.0.5/16]\n"
            "  Groups: [peer]\n"
            "}\n"
        ),
    })()
    monkeypatch.setattr(mesh_nebula.subprocess, "run", lambda *a, **kw: fake_out)
    assert mesh_nebula.current_overlay_ip(cert) == "10.42.0.5"


def test_current_overlay_ip_returns_none_when_cert_missing(tmp_path):
    from mackes.mesh_nebula import current_overlay_ip
    assert current_overlay_ip(tmp_path / "missing.crt") is None


def test_current_overlay_ip_returns_none_when_nebula_cert_absent(tmp_path, monkeypatch):
    from mackes import mesh_nebula
    cert = tmp_path / "host.crt"
    cert.write_text("pretend-cert-bytes")
    monkeypatch.setattr(mesh_nebula.shutil, "which", lambda _: None)
    assert mesh_nebula.current_overlay_ip(cert) is None


def test_current_overlay_ip_returns_none_on_nonzero_exit(tmp_path, monkeypatch):
    from mackes import mesh_nebula
    cert = tmp_path / "host.crt"
    cert.write_text("garbage")
    monkeypatch.setattr(mesh_nebula.shutil, "which", lambda _: "/usr/bin/nebula-cert")
    fake = type("R", (), {"returncode": 1, "stdout": ""})()
    monkeypatch.setattr(mesh_nebula.subprocess, "run", lambda *a, **kw: fake)
    assert mesh_nebula.current_overlay_ip(cert) is None


# ─────────────────────────────────────────────────────────────────
# write_sshd_overlay_bind — atomic + idempotent
# ─────────────────────────────────────────────────────────────────


def test_write_sshd_overlay_bind_writes_listen_address(tmp_path):
    from mackes.mesh_nebula import write_sshd_overlay_bind
    target = tmp_path / "sshd_config.d" / "mackes-mesh.conf"
    written = write_sshd_overlay_bind("10.42.0.5", dropin_path=target)
    assert written == target
    body = target.read_text()
    assert "ListenAddress 10.42.0.5" in body
    assert body.startswith("# Generated by mackes/mesh_nebula.py")


def test_write_sshd_overlay_bind_overwrites_existing(tmp_path):
    from mackes.mesh_nebula import write_sshd_overlay_bind
    target = tmp_path / "mackes-mesh.conf"
    target.write_text("ListenAddress 10.42.0.1\n")
    write_sshd_overlay_bind("10.42.0.99", dropin_path=target)
    body = target.read_text()
    assert "ListenAddress 10.42.0.99" in body
    assert "10.42.0.1" not in body
    # Temp file shouldn't survive the atomic rename.
    assert not target.with_suffix(target.suffix + ".tmp").exists()


# ─────────────────────────────────────────────────────────────────
# wol_via_lighthouse — fallback handling
# ─────────────────────────────────────────────────────────────────


def test_wol_via_lighthouse_returns_2_when_no_lighthouse(monkeypatch):
    from mackes import mesh_nebula
    monkeypatch.setattr(mesh_nebula, "lighthouse_addresses", lambda: [])
    assert mesh_nebula.wol_via_lighthouse("aa:bb:cc:dd:ee:ff") == 2


def test_wol_via_lighthouse_returns_3_when_wakeonlan_missing(monkeypatch):
    from mackes import mesh_nebula
    monkeypatch.setattr(mesh_nebula, "lighthouse_addresses", lambda: ["10.42.0.1"])
    monkeypatch.setattr(mesh_nebula.shutil, "which", lambda _: None)
    assert mesh_nebula.wol_via_lighthouse("aa:bb:cc:dd:ee:ff") == 3


def test_wol_via_lighthouse_invokes_wakeonlan_with_lighthouse_ip(monkeypatch):
    from mackes import mesh_nebula
    calls = []
    monkeypatch.setattr(mesh_nebula, "lighthouse_addresses", lambda: ["10.42.0.7"])
    monkeypatch.setattr(mesh_nebula.shutil, "which", lambda _: "/usr/bin/wakeonlan")
    monkeypatch.setattr(mesh_nebula.subprocess, "call",
                        lambda argv, **kw: calls.append(argv) or 0)
    assert mesh_nebula.wol_via_lighthouse("aa:bb:cc:dd:ee:ff") == 0
    assert calls == [["wakeonlan", "-i", "10.42.0.7", "aa:bb:cc:dd:ee:ff"]]


# ─────────────────────────────────────────────────────────────────
# CANONICAL_SERVICES + published_services_summary (NF-13.8 data layer)
# ─────────────────────────────────────────────────────────────────


def test_canonical_services_contains_expected_ids():
    from mackes.mesh_nebula import CANONICAL_SERVICES
    ids = {row[0] for row in CANONICAL_SERVICES}
    assert ids == {"ssh", "nats", "fs", "media", "sync", "wol", "av"}


def test_published_services_summary_marks_publishable_when_overlay_present(monkeypatch):
    from mackes import mesh_nebula
    monkeypatch.setattr(mesh_nebula, "current_overlay_ip", lambda: "10.42.0.5")
    rows = mesh_nebula.published_services_summary()
    assert len(rows) == 7
    assert all(r["is_publishable"] for r in rows)
    assert all(r["overlay_ip"] == "10.42.0.5" for r in rows)
    ssh = next(r for r in rows if r["id"] == "ssh")
    assert ssh["port"] == 22
    assert ssh["proto"] == "tcp"


def test_published_services_summary_unpublishable_when_no_overlay(monkeypatch):
    from mackes import mesh_nebula
    monkeypatch.setattr(mesh_nebula, "current_overlay_ip", lambda: None)
    rows = mesh_nebula.published_services_summary()
    assert all(r["overlay_ip"] is None for r in rows)
    assert all(r["is_publishable"] is False for r in rows)


def test_bind_target_for_returns_overlay_ip(monkeypatch):
    from mackes import mesh_nebula
    monkeypatch.setattr(mesh_nebula, "current_overlay_ip", lambda: "10.42.0.5")
    assert mesh_nebula.bind_target_for("ssh") == "10.42.0.5"
    assert mesh_nebula.bind_target_for("nats") == "10.42.0.5"


def test_bind_target_for_returns_none_pre_enrollment(monkeypatch):
    from mackes import mesh_nebula
    monkeypatch.setattr(mesh_nebula, "current_overlay_ip", lambda: None)
    assert mesh_nebula.bind_target_for("ssh") is None


# ─────────────────────────────────────────────────────────────────
# NF-16 toast emitters
# ─────────────────────────────────────────────────────────────────


def test_emit_lighthouse_event_promoted_writes_info_toast(tmp_path, monkeypatch):
    from mackes import mesh_nebula
    toast_path = tmp_path / "toasts.jsonl"
    monkeypatch.setattr(mesh_nebula, "TOASTS_PATH", toast_path)
    assert mesh_nebula.emit_lighthouse_event(promoted=True) is True
    line = toast_path.read_text().strip()
    payload = json.loads(line)
    assert payload["kind"] == "info"
    assert payload["title"] == "Lighthouse active"


def test_emit_lighthouse_event_demoted_writes_info_toast(tmp_path, monkeypatch):
    from mackes import mesh_nebula
    toast_path = tmp_path / "toasts.jsonl"
    monkeypatch.setattr(mesh_nebula, "TOASTS_PATH", toast_path)
    mesh_nebula.emit_lighthouse_event(promoted=False)
    payload = json.loads(toast_path.read_text().strip())
    assert payload["kind"] == "info"
    assert "stepped down" in payload["title"]


def test_emit_ca_rotation_success_writes_info(tmp_path, monkeypatch):
    from mackes import mesh_nebula
    toast_path = tmp_path / "toasts.jsonl"
    monkeypatch.setattr(mesh_nebula, "TOASTS_PATH", toast_path)
    mesh_nebula.emit_ca_rotation(success=True)
    payload = json.loads(toast_path.read_text().strip())
    assert payload["kind"] == "info"
    assert "rotated" in payload["title"]


def test_emit_ca_rotation_failure_writes_error_with_recovery_pointer(tmp_path, monkeypatch):
    from mackes import mesh_nebula
    toast_path = tmp_path / "toasts.jsonl"
    monkeypatch.setattr(mesh_nebula, "TOASTS_PATH", toast_path)
    mesh_nebula.emit_ca_rotation(success=False, error_detail="permission denied")
    payload = json.loads(toast_path.read_text().strip())
    assert payload["kind"] == "error"
    assert "permission denied" in payload["body"]
    assert "mesh-recovery.md" in payload["body"]


def test_emit_https_fallback_state_active_warns(tmp_path, monkeypatch):
    from mackes import mesh_nebula
    toast_path = tmp_path / "toasts.jsonl"
    monkeypatch.setattr(mesh_nebula, "TOASTS_PATH", toast_path)
    mesh_nebula.emit_https_fallback_state(active=True)
    payload = json.loads(toast_path.read_text().strip())
    assert payload["kind"] == "warn"
    assert payload["title"] == "Mesh in firewall mode"


def test_emit_https_fallback_state_inactive_info(tmp_path, monkeypatch):
    from mackes import mesh_nebula
    toast_path = tmp_path / "toasts.jsonl"
    monkeypatch.setattr(mesh_nebula, "TOASTS_PATH", toast_path)
    mesh_nebula.emit_https_fallback_state(active=False)
    payload = json.loads(toast_path.read_text().strip())
    assert payload["kind"] == "info"
    assert "Direct UDP" in payload["title"]


def test_emit_cert_expiry_warning_expired_is_persistent_error(tmp_path, monkeypatch):
    from mackes import mesh_nebula
    toast_path = tmp_path / "toasts.jsonl"
    monkeypatch.setattr(mesh_nebula, "TOASTS_PATH", toast_path)
    mesh_nebula.emit_cert_expiry_warning("birch", days_remaining=0)
    payload = json.loads(toast_path.read_text().strip())
    assert payload["kind"] == "error"
    assert payload["visible_ms"] == 0  # persistent per the applet convention


def test_emit_cert_expiry_warning_within_7d_warns(tmp_path, monkeypatch):
    from mackes import mesh_nebula
    toast_path = tmp_path / "toasts.jsonl"
    monkeypatch.setattr(mesh_nebula, "TOASTS_PATH", toast_path)
    mesh_nebula.emit_cert_expiry_warning("oak", days_remaining=3)
    payload = json.loads(toast_path.read_text().strip())
    assert payload["kind"] == "warn"
    assert "3d" in payload["title"]


def test_emit_cert_expiry_warning_beyond_7d_noop(tmp_path, monkeypatch):
    from mackes import mesh_nebula
    toast_path = tmp_path / "toasts.jsonl"
    monkeypatch.setattr(mesh_nebula, "TOASTS_PATH", toast_path)
    assert mesh_nebula.emit_cert_expiry_warning("pine", days_remaining=30) is False
    assert not toast_path.exists()


# ─────────────────────────────────────────────────────────────────
# NF-17 firewall preset
# ─────────────────────────────────────────────────────────────────


def test_apply_nebula_firewall_preset_returns_1_when_firewall_cmd_missing(monkeypatch):
    from mackes import mesh_nebula
    monkeypatch.setattr(mesh_nebula.shutil, "which", lambda _: None)
    assert mesh_nebula.apply_nebula_firewall_preset() == 1


def test_apply_nebula_firewall_preset_invokes_all_ports(monkeypatch):
    from mackes import mesh_nebula
    invocations = []
    monkeypatch.setattr(mesh_nebula.shutil, "which", lambda _: "/usr/bin/firewall-cmd")
    monkeypatch.setattr(mesh_nebula.subprocess, "call",
                        lambda argv, **kw: invocations.append(argv) or 0)
    rc = mesh_nebula.apply_nebula_firewall_preset()
    assert rc == 0
    add_calls = [c for c in invocations if "--add-port" in c]
    assert any("4242/udp" in c for c in add_calls)
    assert any("443/tcp" in c for c in add_calls)
    assert any("--reload" in c for c in invocations)


def test_nebula_firewall_ports_pins_4242_udp_and_443_tcp():
    from mackes.mesh_nebula import NEBULA_FIREWALL_PORTS
    assert (4242, "udp") in NEBULA_FIREWALL_PORTS
    assert (443, "tcp") in NEBULA_FIREWALL_PORTS


# ─────────────────────────────────────────────────────────────────
# nebula_peer_ips — D-Bus consumer
# ─────────────────────────────────────────────────────────────────


def test_nebula_peer_ips_returns_empty_when_dbus_send_missing(monkeypatch):
    from mackes import mesh_nebula
    monkeypatch.setattr(mesh_nebula.shutil, "which", lambda _: None)
    assert mesh_nebula.nebula_peer_ips() == []


def test_nebula_peer_ips_parses_json_reply(monkeypatch):
    from mackes import mesh_nebula
    fake = type("R", (), {
        "returncode": 0,
        "stdout": json.dumps([
            {"name": "birch", "overlay_ip": "10.42.0.2"},
            {"name": "oak", "overlay_ip": "10.42.0.3"},
        ]),
    })()
    monkeypatch.setattr(mesh_nebula.shutil, "which", lambda _: "/usr/bin/dbus-send")
    monkeypatch.setattr(mesh_nebula.subprocess, "run", lambda *a, **kw: fake)
    assert mesh_nebula.nebula_peer_ips() == [
        ("birch", "10.42.0.2"),
        ("oak", "10.42.0.3"),
    ]


def test_nebula_peer_ips_skips_rows_without_name_or_ip(monkeypatch):
    from mackes import mesh_nebula
    fake = type("R", (), {
        "returncode": 0,
        "stdout": json.dumps([
            {"name": "birch", "overlay_ip": "10.42.0.2"},
            {"name": "broken"},                      # missing overlay_ip
            {"overlay_ip": "10.42.0.4"},             # missing name
            "not a dict",                            # invalid row
        ]),
    })()
    monkeypatch.setattr(mesh_nebula.shutil, "which", lambda _: "/usr/bin/dbus-send")
    monkeypatch.setattr(mesh_nebula.subprocess, "run", lambda *a, **kw: fake)
    assert mesh_nebula.nebula_peer_ips() == [("birch", "10.42.0.2")]


def test_nebula_peer_ips_handles_garbage_json(monkeypatch):
    from mackes import mesh_nebula
    fake = type("R", (), {"returncode": 0, "stdout": "{not valid json"})()
    monkeypatch.setattr(mesh_nebula.shutil, "which", lambda _: "/usr/bin/dbus-send")
    monkeypatch.setattr(mesh_nebula.subprocess, "run", lambda *a, **kw: fake)
    assert mesh_nebula.nebula_peer_ips() == []


def test_nebula_peer_ips_handles_nonzero_exit(monkeypatch):
    from mackes import mesh_nebula
    fake = type("R", (), {"returncode": 1, "stdout": ""})()
    monkeypatch.setattr(mesh_nebula.shutil, "which", lambda _: "/usr/bin/dbus-send")
    monkeypatch.setattr(mesh_nebula.subprocess, "run", lambda *a, **kw: fake)
    assert mesh_nebula.nebula_peer_ips() == []


# ─────────────────────────────────────────────────────────────────
# NF-7.2 join-token parser (lives in mackes.wizard.pages.mesh_passcode
# but is part of the v2.5 enrollment surface this file exercises).
# ─────────────────────────────────────────────────────────────────


def test_parse_join_token_round_trip():
    from mackes.wizard.pages.mesh_passcode import (
        JoinToken, parse_join_token, join_token_is_valid,
    )
    raw = "mesh:mesh-001@10.0.0.5:4242#dGVzdC1iZWFyZXItYWJjZGVm"
    tok = parse_join_token(raw)
    assert isinstance(tok, JoinToken)
    assert tok.mesh_id == "mesh-001"
    assert tok.lighthouse == "10.0.0.5"
    assert tok.port == 4242
    assert tok.bearer == "dGVzdC1iZWFyZXItYWJjZGVm"
    assert tok.encode() == raw
    assert join_token_is_valid(raw) is True


def test_parse_join_token_rejects_wrong_scheme():
    from mackes.wizard.pages.mesh_passcode import (
        parse_join_token, join_token_is_valid,
    )
    assert parse_join_token("not-a-token") is None
    assert parse_join_token("") is None
    assert join_token_is_valid("not-a-token") is False


def test_parse_join_token_rejects_invalid_port():
    from mackes.wizard.pages.mesh_passcode import join_token_is_valid
    assert join_token_is_valid(
        "mesh:m@10.0.0.5:99999#bearer"
    ) is False


def test_parse_join_token_rejects_non_ipv4_lighthouse():
    from mackes.wizard.pages.mesh_passcode import join_token_is_valid
    # IPv6 + hostname both rejected per the v2.5 lock (Q5: IPv4-only).
    assert join_token_is_valid(
        "mesh:m@fe80::1:4242#bearer"
    ) is False
    assert join_token_is_valid(
        "mesh:m@lighthouse.example.com:4242#bearer"
    ) is False
