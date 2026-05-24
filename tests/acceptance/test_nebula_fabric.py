"""NF-9.x — bench-acceptance gate for the v2.5 Nebula fabric rebuild.

Each function below maps 1:1 onto one of the six scenarios locked in
``docs/design/v2.5-nebula-fabric.md`` § "Acceptance criteria for v2.5
cut" and tracked as NF-9.1..NF-9.6 in ``docs/PROJECT_WORKLIST.md``.

Execution model
---------------

The module skips wholesale when ``MDE_NEBULA_BENCH_FLEET`` is unset
or points at an unreadable file. When set, the env var must name a
JSON file describing the bench fleet topology (see
``docs/help/bench-acceptance.md`` for the schema). The
``NebulaBenchFleet`` fixture parses that JSON and exposes ssh-handle
helpers the per-scenario tests use to drive each host.

Per ``tests/acceptance/README.md`` and CLAUDE.md §0.12, the
six functions are **not** stubs — they are the acceptance gate. They
just skip in environments that cannot run them.
"""
from __future__ import annotations

import json
import os
import shutil
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterator

import pytest


# ---- fleet-availability gate ---------------------------------------------


_FLEET_ENV = "MDE_NEBULA_BENCH_FLEET"


def _nebula_fleet_available() -> bool:
    """Return ``True`` only if the operator wired a bench fleet.

    The check is intentionally cheap: presence + non-empty value.
    Schema validation happens inside the fixture so the failure
    surfaces with a clear pytest error rather than a collection-time
    skip with no diagnostic.
    """
    value = os.environ.get(_FLEET_ENV, "").strip()
    return bool(value)


pytestmark = pytest.mark.skipif(
    not _nebula_fleet_available(),
    reason=(
        f"No Nebula bench fleet: set {_FLEET_ENV} to a fleet topology "
        "JSON path (see docs/help/bench-acceptance.md)."
    ),
)


# ---- fleet model ----------------------------------------------------------


@dataclass(frozen=True)
class BenchHost:
    """One row of the fleet topology JSON's ``hosts`` array."""

    node_id: str
    ssh: str  # e.g. "user@10.0.0.10"
    role: str  # "host" | "peer" | "candidate"

    def run(self, cmd: str, *, timeout: float = 30.0) -> subprocess.CompletedProcess:
        """Run ``cmd`` on this host via ssh, capturing stdout/stderr.

        Wraps the shell scripts in ``tests/acceptance/lib/`` and is the
        only sanctioned way for a scenario to mutate a bench host.
        ``-o BatchMode=yes`` keeps us from hanging on a password
        prompt — bench fleets are key-authed by convention.
        """
        ssh_bin = shutil.which("ssh")
        if ssh_bin is None:
            raise RuntimeError("ssh binary not found in PATH")
        argv = [
            ssh_bin,
            "-o",
            "BatchMode=yes",
            "-o",
            "StrictHostKeyChecking=accept-new",
            "-o",
            f"ConnectTimeout={int(min(timeout, 10))}",
            self.ssh,
            cmd,
        ]
        return subprocess.run(
            argv,
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
        )


@dataclass(frozen=True)
class NebulaBenchFleet:
    """Parsed view of the fleet topology JSON."""

    fleet_id: str
    hosts: tuple[BenchHost, ...]

    def by_role(self, role: str) -> tuple[BenchHost, ...]:
        return tuple(h for h in self.hosts if h.role == role)

    def by_node_id(self, node_id: str) -> BenchHost:
        for host in self.hosts:
            if host.node_id == node_id:
                return host
        raise KeyError(f"node_id {node_id!r} not in fleet {self.fleet_id!r}")


# ---- helpers --------------------------------------------------------------


def _load_fleet_json(path: Path) -> dict[str, Any]:
    """Read + JSON-parse ``path``; raise a pytest-friendly error otherwise.

    The bench-acceptance recipe explicitly verifies that pointing the
    env var at an invalid file (e.g. ``/dev/null``) surfaces this
    error cleanly rather than as an unhandled exception. We use
    ``pytest.fail`` so the message lands in the collection output.
    """
    try:
        raw = path.read_text(encoding="utf-8")
    except OSError as exc:
        pytest.fail(f"fleet config unreadable at {path}: {exc}")
    if not raw.strip():
        pytest.fail(
            f"fleet config invalid: {path} is empty; see "
            "docs/help/bench-acceptance.md for the schema"
        )
    try:
        return json.loads(raw)
    except json.JSONDecodeError as exc:
        pytest.fail(
            f"fleet config invalid: {path} is not valid JSON ({exc}); "
            "see docs/help/bench-acceptance.md for the schema"
        )
    return {}  # unreachable, pytest.fail raises


def _coerce_fleet(payload: dict[str, Any]) -> NebulaBenchFleet:
    fleet_id = payload.get("fleet_id")
    hosts_raw = payload.get("hosts")
    if not isinstance(fleet_id, str) or not fleet_id:
        pytest.fail("fleet config invalid: missing string `fleet_id`")
    if not isinstance(hosts_raw, list) or not hosts_raw:
        pytest.fail("fleet config invalid: `hosts` must be a non-empty array")
    hosts: list[BenchHost] = []
    for idx, entry in enumerate(hosts_raw):
        if not isinstance(entry, dict):
            pytest.fail(f"fleet config invalid: hosts[{idx}] must be an object")
        try:
            hosts.append(
                BenchHost(
                    node_id=str(entry["node_id"]),
                    ssh=str(entry["ssh"]),
                    role=str(entry.get("role", "peer")),
                )
            )
        except KeyError as exc:
            pytest.fail(
                f"fleet config invalid: hosts[{idx}] missing key {exc.args[0]!r}"
            )
    return NebulaBenchFleet(fleet_id=fleet_id, hosts=tuple(hosts))


def _lib_script(name: str) -> Path:
    """Resolve a helper script under ``tests/acceptance/lib/``."""
    here = Path(__file__).resolve().parent
    path = here / "lib" / name
    if not path.exists():
        pytest.fail(f"acceptance helper script missing: {path}")
    return path


def _scp_run(host: BenchHost, script: Path, *args: str, timeout: float = 60.0) -> subprocess.CompletedProcess:
    """Copy ``script`` to ``host`` and exec it with ``args``.

    Bench hosts are minimum-footprint Fedora installs — we keep the
    helpers in bash so we don't drag a Python runtime onto them.
    """
    scp_bin = shutil.which("scp")
    ssh_bin = shutil.which("ssh")
    if scp_bin is None or ssh_bin is None:
        raise RuntimeError("scp/ssh binaries not found in PATH")
    # Drop the script under a pid-scoped name so parallel pytest
    # runs (or interleaved scenarios sharing the same bench host)
    # don't clobber each other's helpers.
    remote = f"/tmp/mde-acceptance-{os.getpid()}-{script.name}"
    push = subprocess.run(
        [
            scp_bin,
            "-o",
            "BatchMode=yes",
            "-o",
            "StrictHostKeyChecking=accept-new",
            str(script),
            f"{host.ssh}:{remote}",
        ],
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )
    if push.returncode != 0:
        pytest.fail(f"scp {script.name} → {host.node_id} failed: {push.stderr}")
    # Single-quote-escape every arg: any literal `'` is rewritten as
    # `'\''` (close-quote, escaped-quote, reopen-quote). Safe against
    # join tokens or node ids that happen to contain a quote.
    def _shquote(s: str) -> str:
        return "'" + s.replace("'", "'\\''") + "'"

    quoted_args = " ".join(_shquote(a) for a in args)
    return host.run(f"bash {remote} {quoted_args}", timeout=timeout)


def _wait_until(predicate, *, timeout: float, interval: float = 0.5) -> bool:
    """Poll ``predicate()`` until truthy or ``timeout`` seconds elapse.

    Returns ``True`` if the predicate satisfied within the budget. We
    use this everywhere we have an SLO ("under 5 s", "within 30 s",
    "within 10 s") so the assertion failure carries the actual wait
    budget.
    """
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return True
        time.sleep(interval)
    return False


# ---- fixtures -------------------------------------------------------------


@pytest.fixture(scope="module")
def fleet() -> Iterator[NebulaBenchFleet]:
    """Module-scoped fleet handle parsed from ``MDE_NEBULA_BENCH_FLEET``.

    Module-scoped because the six scenarios share a continuously-
    running mesh: e.g. NF-9.2's enroll runs against the CA NF-9.1
    minted, NF-9.3 expects the overlay NF-9.2 brought up, etc. Tests
    are written to run in declaration order for that reason.
    """
    raw_value = os.environ.get(_FLEET_ENV, "").strip()
    if not raw_value:
        pytest.skip(f"{_FLEET_ENV} not set")
    path = Path(raw_value)
    payload = _load_fleet_json(path)
    yield _coerce_fleet(payload)


# ---- scenario tests -------------------------------------------------------


def test_nf9_1_mesh_init_smoke(fleet: NebulaBenchFleet) -> None:
    """NF-9.1 — ``mackesd mesh init`` mints CA + brings up lighthouse.

    Runs on the first host of the fleet (the convention is that the
    fleet JSON lists hosts in topology order). Asserts:

    * ``mackesd mesh init --mesh-id <fleet_id>`` exits 0.
    * The CA certificate appears at
      ``/var/lib/mackesd/nebula-ca/ca.crt`` with mode 0644.
    """
    if not fleet.hosts:
        pytest.fail("fleet has no hosts; cannot run NF-9.1")
    host = fleet.hosts[0]

    init = host.run(
        f"sudo mackesd mesh init --mesh-id {fleet.fleet_id}",
        timeout=60.0,
    )
    assert init.returncode == 0, (
        f"mackesd mesh init failed on {host.node_id}: "
        f"stdout={init.stdout!r} stderr={init.stderr!r}"
    )

    ca_path = "/var/lib/mackesd/nebula-ca/ca.crt"
    stat = host.run(f"sudo stat -c '%a %n' {ca_path}", timeout=10.0)
    assert stat.returncode == 0, (
        f"CA cert missing on {host.node_id}: {stat.stderr!r}"
    )
    mode, _, name = stat.stdout.strip().partition(" ")
    assert mode == "644", f"CA cert mode {mode!r} != 644 on {host.node_id}"
    assert name == ca_path


def test_nf9_2_two_peer_enroll_ping(fleet: NebulaBenchFleet) -> None:
    """NF-9.2 — second host enrolls + both ping each other within 30 s."""
    if len(fleet.hosts) < 2:
        pytest.skip("NF-9.2 requires >= 2 hosts in the fleet")
    host_a, host_b = fleet.hosts[0], fleet.hosts[1]

    token_proc = host_a.run("sudo mackesd mesh show-join-token", timeout=15.0)
    assert token_proc.returncode == 0, (
        f"show-join-token failed on {host_a.node_id}: {token_proc.stderr!r}"
    )
    token = token_proc.stdout.strip()
    assert token, "join token was empty"

    enroll = _scp_run(host_b, _lib_script("enroll_peer.sh"), token, timeout=90.0)
    assert enroll.returncode == 0, (
        f"enroll on {host_b.node_id} failed: {enroll.stdout!r} / {enroll.stderr!r}"
    )

    # Each peer should resolve the other on the overlay within 30 s.
    def _both_ping() -> bool:
        ping_ab = host_a.run(
            f"sudo mackesd mesh ping {host_b.node_id}", timeout=10.0
        )
        ping_ba = host_b.run(
            f"sudo mackesd mesh ping {host_a.node_id}", timeout=10.0
        )
        return ping_ab.returncode == 0 and ping_ba.returncode == 0

    assert _wait_until(_both_ping, timeout=30.0), (
        f"peers did not converge: {host_a.node_id} <-> {host_b.node_id}"
    )


def test_nf9_3_lan_cable_replug(fleet: NebulaBenchFleet) -> None:
    """NF-9.3 — link-down + link-up; reconnect under 5 s.

    Drives the host's primary interface down for 2 s then back up via
    the ``link_flap.sh`` helper. We poll the panel-state surface
    (``mackesd mesh status --json``) for the ``LinkWatchWorker``'s
    CameUp transition.
    """
    if not fleet.hosts:
        pytest.fail("fleet has no hosts; cannot run NF-9.3")
    host = fleet.hosts[0]

    iface_proc = host.run(
        "ip -o -4 route show default | awk '{print $5}' | head -n1",
        timeout=10.0,
    )
    iface = iface_proc.stdout.strip()
    assert iface, f"could not resolve default iface on {host.node_id}"

    flap = _scp_run(host, _lib_script("link_flap.sh"), iface, timeout=30.0)
    assert flap.returncode == 0, f"link flap failed: {flap.stderr!r}"
    t_up = time.monotonic()

    def _reconnected() -> bool:
        status = host.run("sudo mackesd mesh status --json", timeout=5.0)
        if status.returncode != 0:
            return False
        try:
            payload = json.loads(status.stdout)
        except json.JSONDecodeError:
            return False
        return payload.get("link_state") == "up" and payload.get("overlay_up") is True

    assert _wait_until(_reconnected, timeout=5.0, interval=0.25), (
        f"overlay did not reconverge within 5 s on {host.node_id} "
        f"(elapsed {time.monotonic() - t_up:.2f}s)"
    )


def test_nf9_4_udp_block(fleet: NebulaBenchFleet) -> None:
    """NF-9.4 — block UDP egress; verify TCP/443 fallback within 30 s."""
    if not fleet.hosts:
        pytest.fail("fleet has no hosts; cannot run NF-9.4")
    host = fleet.hosts[0]

    block = _scp_run(host, _lib_script("block_udp_egress.sh"), timeout=30.0)
    assert block.returncode == 0, f"udp block failed: {block.stderr!r}"

    def _fellback_to_https443() -> bool:
        status = host.run(
            "sudo busctl call dev.mackes.MDE /dev/mackes/MDE/Nebula "
            "dev.mackes.MDE.Nebula.Status GetTransport",
            timeout=5.0,
        )
        if status.returncode != 0:
            return False
        return "nebula_https443" in status.stdout

    try:
        assert _wait_until(_fellback_to_https443, timeout=30.0, interval=1.0), (
            f"transport did not flip to nebula_https443 on {host.node_id}"
        )
    finally:
        # Always restore egress — leaving the rule in place would
        # poison every subsequent scenario.
        restore = _scp_run(
            host, _lib_script("restore_udp_egress.sh"), timeout=15.0
        )
        assert restore.returncode == 0, (
            f"udp restore failed on {host.node_id}: {restore.stderr!r}"
        )


def test_nf9_5_host_role_promotion(fleet: NebulaBenchFleet) -> None:
    """NF-9.5 — promote a third host to Host role; verify lighthouse roster."""
    if len(fleet.hosts) < 3:
        pytest.skip("NF-9.5 requires >= 3 hosts in the fleet")
    leader = fleet.hosts[0]
    # Prefer an explicit `role: candidate` host if the operator
    # tagged one (this is the convention documented in
    # docs/help/bench-acceptance.md). Fall back to hosts[2] so
    # legacy fleet topologies without role tags still work.
    role_tagged = fleet.by_role("candidate")
    candidate = role_tagged[0] if role_tagged else fleet.hosts[2]

    promote = leader.run(
        f"sudo mackesd promote {candidate.node_id}", timeout=15.0
    )
    assert promote.returncode == 0, (
        f"promote {candidate.node_id} failed: {promote.stderr!r}"
    )

    def _in_every_roster() -> bool:
        for peer in fleet.hosts:
            roster = peer.run(
                "sudo mackesd mesh lighthouses --json", timeout=5.0
            )
            if roster.returncode != 0:
                return False
            try:
                payload = json.loads(roster.stdout)
            except json.JSONDecodeError:
                return False
            ids = {entry.get("node_id") for entry in payload.get("hosts", [])}
            if candidate.node_id not in ids:
                return False
        return True

    assert _wait_until(_in_every_roster, timeout=10.0, interval=0.5), (
        f"{candidate.node_id} did not appear in every peer's lighthouse "
        "roster within 10 s"
    )

    demote = leader.run(
        f"sudo mackesd demote {candidate.node_id}", timeout=15.0
    )
    assert demote.returncode == 0, (
        f"demote {candidate.node_id} failed: {demote.stderr!r}"
    )

    def _removed_from_every_roster() -> bool:
        for peer in fleet.hosts:
            roster = peer.run(
                "sudo mackesd mesh lighthouses --json", timeout=5.0
            )
            if roster.returncode != 0:
                return False
            try:
                payload = json.loads(roster.stdout)
            except json.JSONDecodeError:
                return False
            ids = {entry.get("node_id") for entry in payload.get("hosts", [])}
            if candidate.node_id in ids:
                return False
        return True

    assert _wait_until(_removed_from_every_roster, timeout=5.0, interval=0.25), (
        f"{candidate.node_id} did not drop from rosters within 5 s of demote"
    )


def test_nf9_6_leader_kill_ca_epoch_bump(fleet: NebulaBenchFleet) -> None:
    """NF-9.6 — kill the leader; new Host wins lease, CA epoch bumps.

    Stops ``mackesd`` on the current leader, waits for the election
    lease to expire, then checks:

    * Some peer reports itself as the new leader.
    * The CA epoch (read from the mackesd SQL store on the new
      leader) is strictly greater than what the killed leader held.
    * Every peer reports a fresh ``cert_bundle_epoch`` matching the
      new CA epoch.
    """
    if len(fleet.hosts) < 2:
        pytest.skip("NF-9.6 requires >= 2 hosts to survive a leader kill")
    leader = fleet.hosts[0]

    epoch_before_proc = leader.run(
        "sudo mackesd ca show-epoch", timeout=10.0
    )
    assert epoch_before_proc.returncode == 0, (
        f"ca show-epoch failed pre-kill: {epoch_before_proc.stderr!r}"
    )
    epoch_before = int(epoch_before_proc.stdout.strip())

    stop = _scp_run(leader, _lib_script("stop_mackesd.sh"), timeout=30.0)
    assert stop.returncode == 0, f"systemctl stop mackesd failed: {stop.stderr!r}"

    survivors = fleet.hosts[1:]

    def _new_leader_elected() -> bool:
        for peer in survivors:
            who = peer.run("sudo mackesd ca show-leader", timeout=5.0)
            if who.returncode == 0 and who.stdout.strip() == peer.node_id:
                return True
        return False

    # Lease TTL is operator-tunable; 60 s is the upper bound the
    # design lock commits to (Q26).
    assert _wait_until(_new_leader_elected, timeout=60.0, interval=1.0), (
        "no survivor claimed leader within the 60 s lease TTL"
    )

    new_leader: BenchHost | None = None
    for peer in survivors:
        who = peer.run("sudo mackesd ca show-leader", timeout=5.0)
        if who.returncode == 0 and who.stdout.strip() == peer.node_id:
            new_leader = peer
            break
    assert new_leader is not None, "could not resolve new leader after election"

    epoch_after_proc = new_leader.run(
        "sudo mackesd ca show-epoch", timeout=10.0
    )
    assert epoch_after_proc.returncode == 0
    epoch_after = int(epoch_after_proc.stdout.strip())
    assert epoch_after > epoch_before, (
        f"CA epoch did not bump after leader kill: {epoch_before} -> {epoch_after}"
    )

    def _all_peers_re_enrolled() -> bool:
        for peer in survivors:
            bundle = peer.run(
                "sudo mackesd ca cert-bundle-epoch", timeout=5.0
            )
            if bundle.returncode != 0:
                return False
            try:
                if int(bundle.stdout.strip()) != epoch_after:
                    return False
            except ValueError:
                return False
        return True

    assert _wait_until(_all_peers_re_enrolled, timeout=60.0, interval=2.0), (
        "not every surviving peer received the fresh cert bundle"
    )
