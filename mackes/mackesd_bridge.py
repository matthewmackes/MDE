"""Python-side bridge to the ``mackesd`` control plane (Phase 12.13.3).

This module is the **Cutover** half of the 12.13.3 step: it lets the
Workbench Mesh panels switch from the legacy in-process
``mackes.mesh_*`` probes to data sourced from the new Rust
``mackesd_core`` library. For 1.x compatibility — and to keep the
blast radius small — we cutover via **shell-out** to the ``mackesd``
CLI, not a PyO3 in-process link. PyO3 is the eventual 2.0 path; today
we don't need it.

The lock (`docs/PROJECT_WORKLIST.md`, §12.13):

    "Once the new backend serves a single test mesh end-to-end, the
    Workbench Mesh panels switch to API reads (12.8.1). Legacy probes
    stay during a two-release deprecation window with ``[deprecated]``
    log warnings."

What the bridge gives you
-------------------------

A tiny stable surface — each function corresponds 1:1 to a
``mackesd`` subcommand:

  * :func:`health` → ``mackesd healthz``     (Phase 12.1.3)
  * :func:`peers_why` → ``mackesd peers-why <id>`` (Phase 12.4.4)
  * :func:`audit_verify` → ``mackesd audit-verify`` (Phase 12.10.3)
  * :func:`paired_inventory` → ``mackesd inventory-legacy --json``
    (Phase 12.13.1)

Each call:

  1. Checks the *migration feature flag*
     (``panel.toml::[migration].use_mackesd``).
     Default = ``False`` on 1.1.x for safety; flips to ``True`` on
     2.0.0. Override via ``MACKES_USE_MACKESD=1/0`` for early
     adopters and CI.
  2. If the flag is **on** *and* ``mackesd`` is on ``PATH`` →
     shell-out, parse the structured response, return a typed dataclass.
  3. If the flag is **off** *or* the binary is unavailable → emit a
     single ``[deprecated]`` log warning and return ``None`` so the
     panel can fall back to its legacy probe.

Panels never raise from the bridge — they handle ``None`` gracefully.

The flag
--------

``~/.config/mackes-panel/panel.toml``::

    [migration]
    use_mackesd = true

The flag is *additive* to the panel.toml schema; the importer
(``mackes.legacy_import``) already preserves unknown top-level
tables on re-runs, so flipping the flag survives subsequent legacy
imports.

Environment override
--------------------

``MACKES_USE_MACKESD`` (case-insensitive ``1``/``0``/``true``/``false``)
overrides the panel.toml flag. CI runs use ``0`` to keep the legacy
path under test until the bridge is the default; early adopters set
``1`` to flip a single shell without writing TOML.

Caching
-------

The "binary on PATH" check is memoized at module-import time + on
first call so the hot path doesn't ``shutil.which`` on every render.
Call :func:`_invalidate_availability_cache` from tests that mutate
``PATH``.
"""
from __future__ import annotations

import json
import logging
import os
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import List, Optional


logger = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------


#: Binary name. Resolved against ``$PATH`` (development builds under
#: ``target/release/`` are not auto-discovered — install the RPM or
#: prepend the build dir to ``PATH``).
MACKESD_BINARY = "mackesd"

#: Subprocess timeout in seconds. Every ``mackesd`` subcommand we
#: shell out to is local + read-only and finishes in well under a
#: second on a healthy system; we cap at 5 s so a wedged binary
#: doesn't hang the GTK main thread.
SUBPROCESS_TIMEOUT_S = 5.0

#: TOML table name + key for the feature flag.
PANEL_TOML_MIGRATION_TABLE = "migration"
PANEL_TOML_USE_MACKESD_KEY = "use_mackesd"

#: Environment override. Set to ``1``/``true``/``yes``/``on`` to
#: force-enable; ``0``/``false``/``no``/``off`` to force-disable.
#: Anything else is ignored.
MACKESD_ENV_OVERRIDE = "MACKES_USE_MACKESD"

#: Default value for ``[migration].use_mackesd`` on 1.x. Flips to
#: ``True`` when 2.0.0 cuts.
DEFAULT_USE_MACKESD_FLAG = False


# ---------------------------------------------------------------------------
# Typed results
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class HealthReport:
    """Parsed shape of ``mackesd healthz``'s JSON line.

    Mirrors ``mackesd_core::health::HealthReport`` — see
    ``crates/mackesd/src/health.rs``. The Python side keeps the
    schema discriminator so a 2.0 panel can degrade gracefully when a
    newer ``mackesd`` reports a shape it doesn't recognize.
    """

    schema: int
    is_leader: bool
    applied_revision: Optional[str]
    node_count: int
    healthy_nodes: int
    degraded_nodes: int
    unreachable_nodes: int
    audit_chain_intact: bool
    version: str

    @classmethod
    def from_json(cls, raw: str) -> "HealthReport":
        """Parse a single JSON line from ``mackesd healthz``.

        Raises :class:`ValueError` on malformed input — callers are
        expected to wrap this in a try/except that falls back to the
        legacy probe.
        """
        try:
            doc = json.loads(raw)
        except json.JSONDecodeError as exc:
            raise ValueError(f"mackesd healthz: invalid JSON: {exc}") from exc
        if not isinstance(doc, dict):
            raise ValueError(
                f"mackesd healthz: expected JSON object, got {type(doc).__name__}"
            )
        try:
            return cls(
                schema=int(doc["schema"]),
                is_leader=bool(doc["is_leader"]),
                applied_revision=(
                    doc["applied_revision"]
                    if doc.get("applied_revision") is None
                    else str(doc["applied_revision"])
                ),
                node_count=int(doc["node_count"]),
                healthy_nodes=int(doc["healthy_nodes"]),
                degraded_nodes=int(doc["degraded_nodes"]),
                unreachable_nodes=int(doc["unreachable_nodes"]),
                audit_chain_intact=bool(doc["audit_chain_intact"]),
                version=str(doc["version"]),
            )
        except (KeyError, TypeError, ValueError) as exc:
            raise ValueError(
                f"mackesd healthz: missing/wrong-typed field ({exc})"
            ) from exc


@dataclass(frozen=True)
class AuditOutcome:
    """Result of a ``mackesd audit-verify`` run.

    The CLI exits 0 on ``Intact`` or ``Empty`` and 1 on ``Break``.
    The bridge surfaces both the boolean intactness and the raw
    stdout/stderr so the panel can render the message verbatim.
    """

    intact: bool
    exit_code: int
    message: str

    @property
    def is_empty(self) -> bool:
        """True when the audit chain has no events yet."""
        return self.intact and "empty" in self.message.lower()


@dataclass(frozen=True)
class LegacyArtifact:
    """Parsed shape of a single ``mackesd inventory-legacy --json``
    row. Mirrors ``mackesd_core::legacy_inventory::LegacyArtifact``.
    """

    path: Path
    size_bytes: int
    mtime_ms: int
    artifact_kind: str
    mesh_data: bool

    @classmethod
    def from_dict(cls, doc: dict) -> "LegacyArtifact":
        return cls(
            path=Path(str(doc["path"])),
            size_bytes=int(doc["size_bytes"]),
            mtime_ms=int(doc["mtime_ms"]),
            artifact_kind=str(doc["artifact_kind"]),
            mesh_data=bool(doc["mesh_data"]),
        )


# ---------------------------------------------------------------------------
# Availability + feature-flag plumbing
# ---------------------------------------------------------------------------


# Cached resolution of ``shutil.which("mackesd")``. Reset by
# :func:`_invalidate_availability_cache`.
_AVAILABILITY_CACHE: Optional[bool] = None


def _invalidate_availability_cache() -> None:
    """Clear the cached PATH lookup. Tests that mutate ``$PATH``
    must call this between cases."""
    global _AVAILABILITY_CACHE
    _AVAILABILITY_CACHE = None


def _mackesd_available() -> bool:
    """Return ``True`` when the ``mackesd`` binary is on ``PATH``.

    The result is cached for the process lifetime — a redeploy that
    installs ``mackesd`` after the panel is up will not be picked up
    until the next process start. That's fine: the panel re-launches
    on RPM upgrade.
    """
    global _AVAILABILITY_CACHE
    if _AVAILABILITY_CACHE is None:
        _AVAILABILITY_CACHE = shutil.which(MACKESD_BINARY) is not None
    return _AVAILABILITY_CACHE


def _config_home() -> Path:
    """Resolve ``$XDG_CONFIG_HOME`` with the ``~/.config`` fallback."""
    return Path(
        os.environ.get("XDG_CONFIG_HOME")
        or (Path(os.environ.get("HOME", str(Path.home()))) / ".config")
    )


def _panel_toml_path() -> Path:
    """Where the Rust panel keeps its config — same path the importer
    writes to."""
    return _config_home() / "mackes-panel" / "panel.toml"


def _read_use_mackesd_flag() -> bool:
    """Resolve the ``[migration].use_mackesd`` flag.

    Resolution order:

      1. ``MACKES_USE_MACKESD`` env var (case-insensitive parse).
      2. ``[migration].use_mackesd`` in ``~/.config/mackes-panel/panel.toml``.
      3. :data:`DEFAULT_USE_MACKESD_FLAG`.

    Never raises — a malformed panel.toml falls through to the
    default with a single ``logger.warning`` line.
    """
    # ---- 1. env override ------------------------------------------------
    override = os.environ.get(MACKESD_ENV_OVERRIDE)
    if override is not None:
        normalized = override.strip().lower()
        if normalized in {"1", "true", "yes", "on"}:
            return True
        if normalized in {"0", "false", "no", "off"}:
            return False
        # Anything else: log + fall through to TOML.
        logger.warning(
            "mackesd_bridge: ignoring unrecognized %s=%r",
            MACKESD_ENV_OVERRIDE,
            override,
        )

    # ---- 2. panel.toml --------------------------------------------------
    path = _panel_toml_path()
    if not path.is_file():
        return DEFAULT_USE_MACKESD_FLAG

    try:
        import tomllib  # py311+
        with path.open("rb") as fp:
            parsed = tomllib.load(fp)
    except (OSError, ValueError) as exc:
        # ValueError covers tomllib.TOMLDecodeError without forcing
        # the import to succeed for the error class.
        logger.warning(
            "mackesd_bridge: panel.toml unreadable (%s) — using default flag",
            exc,
        )
        return DEFAULT_USE_MACKESD_FLAG

    table = parsed.get(PANEL_TOML_MIGRATION_TABLE)
    if isinstance(table, dict):
        value = table.get(PANEL_TOML_USE_MACKESD_KEY)
        if isinstance(value, bool):
            return value
        if value is not None:
            logger.warning(
                "mackesd_bridge: [%s].%s should be a boolean, got %r — "
                "using default",
                PANEL_TOML_MIGRATION_TABLE,
                PANEL_TOML_USE_MACKESD_KEY,
                value,
            )
    return DEFAULT_USE_MACKESD_FLAG


def set_use_mackesd_flag(value: bool) -> Path:
    """Persist ``[migration].use_mackesd = <value>`` into
    ``panel.toml``. Returns the file path written.

    The writer reuses :mod:`mackes.legacy_import`'s panel.toml read +
    serializer so the on-disk file stays byte-for-byte equivalent to
    what the importer produces. Unknown tables (e.g. ``[top_bar]``,
    ``[dock]``) are preserved verbatim.
    """
    from mackes import legacy_import  # local import to avoid cycle

    cfg = legacy_import._read_panel_toml_or_default()  # noqa: SLF001
    migration = cfg.setdefault(PANEL_TOML_MIGRATION_TABLE, {})
    if not isinstance(migration, dict):
        migration = {}
        cfg[PANEL_TOML_MIGRATION_TABLE] = migration
    migration[PANEL_TOML_USE_MACKESD_KEY] = bool(value)
    legacy_import._write_panel_toml(cfg)  # noqa: SLF001
    return _panel_toml_path()


# ---------------------------------------------------------------------------
# Deprecation log helper
# ---------------------------------------------------------------------------


# Emit the ``[deprecated]`` log line at most once per process per
# (function, reason) pair so a panel that re-renders every 15 s
# doesn't flood the log.
_DEPRECATION_LOGGED: set[tuple[str, str]] = set()


def _log_deprecated(fn_name: str, reason: str) -> None:
    """Emit a single ``[deprecated]`` log line for ``fn_name``.

    The two-release deprecation window (see
    ``docs/MIGRATION_TO_MACKESD.md``) requires every legacy fallback
    path to log at WARN. We dedupe to one line per (fn, reason).
    """
    key = (fn_name, reason)
    if key in _DEPRECATION_LOGGED:
        return
    _DEPRECATION_LOGGED.add(key)
    logger.warning(
        "[deprecated] mackesd_bridge.%s: %s — falling back to legacy "
        "mackes.mesh_* probe. This path is removed in 2.0.0; see "
        "docs/MIGRATION_TO_MACKESD.md.",
        fn_name,
        reason,
    )


def _reset_deprecation_log_for_tests() -> None:
    """Drop the dedupe set so tests can re-assert the log line. Not
    part of the public API."""
    _DEPRECATION_LOGGED.clear()


# ---------------------------------------------------------------------------
# Subprocess helper
# ---------------------------------------------------------------------------


def _run_mackesd(args: List[str]) -> subprocess.CompletedProcess:
    """Invoke ``mackesd <args>`` and return the completed process.

    Raises :class:`FileNotFoundError` when the binary is missing,
    :class:`subprocess.TimeoutExpired` when the call exceeds
    :data:`SUBPROCESS_TIMEOUT_S`, and :class:`OSError` for everything
    else. The bridge's public functions catch all three and route to
    the legacy fallback.
    """
    return subprocess.run(  # noqa: S603 — args is a fixed list
        [MACKESD_BINARY, *args],
        capture_output=True,
        text=True,
        timeout=SUBPROCESS_TIMEOUT_S,
        check=False,
    )


# ---------------------------------------------------------------------------
# Public surface
# ---------------------------------------------------------------------------


def health() -> Optional[HealthReport]:
    """Return the ``mackesd_core::health::HealthReport`` for this peer.

    Returns ``None`` when:

      * ``[migration].use_mackesd`` is false (default on 1.x), OR
      * ``mackesd`` is not on ``$PATH``, OR
      * the subprocess exits non-zero or its stdout is malformed.

    On ``None`` the bridge has already logged a ``[deprecated]``
    warning and the caller should fall back to its legacy probe.
    """
    if not _read_use_mackesd_flag():
        _log_deprecated("health", "[migration].use_mackesd = false")
        return None
    if not _mackesd_available():
        _log_deprecated("health", "mackesd not on PATH")
        return None
    try:
        proc = _run_mackesd(["healthz"])
    except (FileNotFoundError, subprocess.TimeoutExpired, OSError) as exc:
        _log_deprecated("health", f"mackesd healthz failed: {exc}")
        return None
    if proc.returncode != 0:
        _log_deprecated(
            "health",
            f"mackesd healthz exit={proc.returncode}: "
            f"{proc.stderr.strip()[:200]}",
        )
        return None
    try:
        return HealthReport.from_json(proc.stdout.strip())
    except ValueError as exc:
        _log_deprecated("health", str(exc))
        return None


def peers_why(node_id: str) -> Optional[str]:
    """Return ``mackesd peers-why <node_id>``'s stdout as a string.

    ``None`` semantics match :func:`health`.
    """
    if not _read_use_mackesd_flag():
        _log_deprecated("peers_why", "[migration].use_mackesd = false")
        return None
    if not _mackesd_available():
        _log_deprecated("peers_why", "mackesd not on PATH")
        return None
    try:
        proc = _run_mackesd(["peers-why", node_id])
    except (FileNotFoundError, subprocess.TimeoutExpired, OSError) as exc:
        _log_deprecated("peers_why", f"mackesd peers-why failed: {exc}")
        return None
    if proc.returncode != 0:
        _log_deprecated(
            "peers_why",
            f"mackesd peers-why exit={proc.returncode}",
        )
        return None
    return proc.stdout.rstrip("\n")


def audit_verify() -> Optional[AuditOutcome]:
    """Return the structured outcome of ``mackesd audit-verify``.

    ``mackesd`` exits 0 on ``Intact`` / ``Empty`` and 1 on ``Break``;
    both outcomes return a populated :class:`AuditOutcome` (the
    panel renders the message). ``None`` means the flag is off or
    the binary is unreachable — caller falls back to legacy.
    """
    if not _read_use_mackesd_flag():
        _log_deprecated("audit_verify", "[migration].use_mackesd = false")
        return None
    if not _mackesd_available():
        _log_deprecated("audit_verify", "mackesd not on PATH")
        return None
    try:
        proc = _run_mackesd(["audit-verify"])
    except (FileNotFoundError, subprocess.TimeoutExpired, OSError) as exc:
        _log_deprecated("audit_verify", f"mackesd audit-verify failed: {exc}")
        return None
    # Treat exit 0 as intact (Empty or Intact) and exit 1 as break.
    # Any other exit code is "unknown" and triggers fallback.
    if proc.returncode == 0:
        message = proc.stdout.strip() or "audit chain ok"
        return AuditOutcome(intact=True, exit_code=0, message=message)
    if proc.returncode == 1:
        message = (proc.stderr.strip() or proc.stdout.strip()
                   or "audit chain break")
        return AuditOutcome(intact=False, exit_code=1, message=message)
    _log_deprecated(
        "audit_verify",
        f"mackesd audit-verify unrecognized exit={proc.returncode}",
    )
    return None


def paired_inventory() -> Optional[List[LegacyArtifact]]:
    """Return the JSON inventory of legacy on-disk state.

    Wraps ``mackesd inventory-legacy --json``. ``None`` on
    flag-off or binary-missing.
    """
    if not _read_use_mackesd_flag():
        _log_deprecated("paired_inventory", "[migration].use_mackesd = false")
        return None
    if not _mackesd_available():
        _log_deprecated("paired_inventory", "mackesd not on PATH")
        return None
    try:
        proc = _run_mackesd(["inventory-legacy", "--json"])
    except (FileNotFoundError, subprocess.TimeoutExpired, OSError) as exc:
        _log_deprecated(
            "paired_inventory",
            f"mackesd inventory-legacy failed: {exc}",
        )
        return None
    if proc.returncode != 0:
        _log_deprecated(
            "paired_inventory",
            f"mackesd inventory-legacy exit={proc.returncode}",
        )
        return None
    try:
        raw = json.loads(proc.stdout or "[]")
    except json.JSONDecodeError as exc:
        _log_deprecated(
            "paired_inventory",
            f"mackesd inventory-legacy returned invalid JSON: {exc}",
        )
        return None
    if not isinstance(raw, list):
        _log_deprecated(
            "paired_inventory",
            "mackesd inventory-legacy returned non-array",
        )
        return None
    out: List[LegacyArtifact] = []
    for entry in raw:
        if not isinstance(entry, dict):
            continue
        try:
            out.append(LegacyArtifact.from_dict(entry))
        except (KeyError, TypeError, ValueError) as exc:
            logger.warning(
                "mackesd_bridge.paired_inventory: skipping malformed "
                "entry (%s): %r",
                exc,
                entry,
            )
            continue
    return out


__all__ = [
    "AuditOutcome",
    "DEFAULT_USE_MACKESD_FLAG",
    "HealthReport",
    "LegacyArtifact",
    "MACKESD_BINARY",
    "MACKESD_ENV_OVERRIDE",
    "PANEL_TOML_MIGRATION_TABLE",
    "PANEL_TOML_USE_MACKESD_KEY",
    "SUBPROCESS_TIMEOUT_S",
    "_invalidate_availability_cache",
    "_mackesd_available",
    "_read_use_mackesd_flag",
    "_reset_deprecation_log_for_tests",
    "audit_verify",
    "health",
    "paired_inventory",
    "peers_why",
    "set_use_mackesd_flag",
]
