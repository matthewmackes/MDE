"""mDNS-SD bridge for mesh service discovery (#4).

Replaces polling-based `mesh_services.probe_all()` with push: each
peer announces its services via `avahi-publish-service` over the
mesh interface; listeners use python-zeroconf to receive in real-time.

Open-source dependencies (all on Fedora 44 main repos):
  * avahi             — multicast DNS responder + publisher
  * avahi-tools       — avahi-publish-service CLI
  * python3-zeroconf  — async listener (`pip install zeroconf`)

The service-discovery latency drops from a 30 s scan window to
sub-second push events. Falls back to TCP probe (the legacy path)
when zeroconf isn't installed.

Public API:

  is_available()                  → bool   (zeroconf + avahi present)
  announce(service: ServiceDef)   → AvahiHandle | None
  start_listener(callback)        → MDNSListener
  scan_once(timeout=3.0)          → list[ServiceHit]
  install_announcer_units()       → list[str]
"""
from __future__ import annotations

import shutil
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Optional


# ---------------------------------------------------------------------------
# Capability probes
# ---------------------------------------------------------------------------


def has_zeroconf() -> bool:
    try:
        import zeroconf  # noqa: F401
        return True
    except ImportError:
        return False


def has_avahi_publish() -> bool:
    return shutil.which("avahi-publish-service") is not None


def is_available() -> bool:
    """True iff we can both publish (avahi-publish-service) AND listen
    (python-zeroconf)."""
    return has_zeroconf() and has_avahi_publish()


# ---------------------------------------------------------------------------
# Announcer — each peer publishes its services over mDNS
# ---------------------------------------------------------------------------


@dataclass
class AvahiHandle:
    """A live avahi-publish-service subprocess. Call .stop() to retract."""
    proc: subprocess.Popen
    service_name: str
    service_type: str
    port: int

    def stop(self) -> None:
        try:
            self.proc.terminate()
            self.proc.wait(timeout=3)
        except (OSError, subprocess.TimeoutExpired):
            try:
                self.proc.kill()
            except OSError:
                pass


def announce(*, service_name: str, service_type: str, port: int,
             txt_records: Optional[dict[str, str]] = None,
             interface: str = "") -> Optional[AvahiHandle]:
    """Publish a service via avahi-publish-service.

    service_type follows the standard "_<proto>._tcp" form
    (e.g. "_jellyfin._tcp", "_ssh._tcp", "_rdp._tcp").
    interface — restrict to one network device (e.g. "tailscale0")
    so we don't bleed mDNS onto random LAN segments.
    """
    if not has_avahi_publish():
        return None
    args = ["avahi-publish-service"]
    if interface:
        args.extend(["--interface", interface])
    args.extend([service_name, service_type, str(port)])
    if txt_records:
        for k, v in txt_records.items():
            args.append(f"{k}={v}")
    try:
        proc = subprocess.Popen(args, stdout=subprocess.DEVNULL,
                                stderr=subprocess.DEVNULL,
                                start_new_session=True)
        return AvahiHandle(proc, service_name, service_type, port)
    except OSError:
        return None


# ---------------------------------------------------------------------------
# Listener — receive announces in real-time
# ---------------------------------------------------------------------------


@dataclass
class MDNSHit:
    peer:         str   # hostname.local
    service_name: str
    service_type: str
    port:         int
    ip:           str
    txt:          dict[str, str]


class MDNSListener:
    """python-zeroconf-based listener that calls `callback(MDNSHit)`
    every time a new service is announced or removed.

    Service types we listen for are the same set mesh_services.load_catalog()
    knows about, prefixed with `_mackes-mesh-` so we don't see random
    LAN devices."""

    SERVICE_TYPES = (
        "_mackes-mesh._tcp.local.",
        # Standard service types we still want to surface
        "_jellyfin._tcp.local.",
        "_ssh._tcp.local.",
        "_rdp._tcp.local.",
        "_http._tcp.local.",
    )

    def __init__(self, callback: Callable[[MDNSHit, str], None]) -> None:
        if not has_zeroconf():
            raise RuntimeError("python-zeroconf not installed")
        from zeroconf import ServiceBrowser, Zeroconf
        self._cb = callback
        self._zc = Zeroconf()
        self._browsers: list = []
        # Listener handler that adapts zeroconf's API to our callback
        listener = _ZeroconfListener(self._zc, callback)
        for st in self.SERVICE_TYPES:
            self._browsers.append(
                ServiceBrowser(self._zc, st, listener=listener)
            )

    def stop(self) -> None:
        try:
            self._zc.close()
        except Exception:  # noqa: BLE001
            pass


class _ZeroconfListener:
    """Adapter that implements the zeroconf.ServiceListener interface."""

    def __init__(self, zc, cb: Callable[[MDNSHit, str], None]) -> None:
        self._zc = zc
        self._cb = cb

    def add_service(self, zc, type_, name):
        self._emit(name, type_, "add")

    def remove_service(self, zc, type_, name):
        self._emit(name, type_, "remove")

    def update_service(self, zc, type_, name):
        self._emit(name, type_, "update")

    def _emit(self, name: str, type_: str, kind: str) -> None:
        try:
            info = self._zc.get_service_info(type_, name, timeout=2000)
            if info is None:
                return
            import socket as _socket
            ip = _socket.inet_ntoa(info.addresses[0]) if info.addresses else ""
            txt = {}
            if info.properties:
                for k, v in info.properties.items():
                    try:
                        k_s = k.decode("utf-8") if isinstance(k, bytes) else str(k)
                        v_s = v.decode("utf-8") if isinstance(v, bytes) else (
                            "" if v is None else str(v)
                        )
                        txt[k_s] = v_s
                    except UnicodeDecodeError:
                        pass
            hit = MDNSHit(
                peer=info.server.rstrip("."), service_name=name,
                service_type=type_, port=info.port or 0,
                ip=ip, txt=txt,
            )
            self._cb(hit, kind)
        except Exception:  # noqa: BLE001
            pass


def scan_once(timeout: float = 3.0) -> list[MDNSHit]:
    """One-shot mDNS scan: open a listener, collect everything that
    answers within `timeout` seconds, close.

    Useful as a fallback when callers want a static snapshot rather
    than a streaming listener."""
    if not has_zeroconf():
        return []
    hits: list[MDNSHit] = []
    def cb(hit: MDNSHit, kind: str) -> None:
        if kind in ("add", "update"):
            hits.append(hit)
    listener = MDNSListener(cb)
    try:
        time.sleep(timeout)
    finally:
        listener.stop()
    return hits


# ---------------------------------------------------------------------------
# Birthright wiring — install announcer units for each Mackes service
# ---------------------------------------------------------------------------


USER_UNITDIR = Path.home() / ".config/systemd/user"


def install_announcer_units(services: list[dict]) -> list[str]:
    """For each (name, type, port) tuple, write a user systemd unit
    that runs `avahi-publish-service`. Idempotent."""
    if not has_avahi_publish():
        return ["avahi-publish-service not installed; cannot announce"]
    USER_UNITDIR.mkdir(parents=True, exist_ok=True)
    actions: list[str] = []
    for s in services:
        name = s["name"]; stype = s["type"]; port = s["port"]
        unit = USER_UNITDIR / f"mackes-mdns-{name}.service"
        unit.write_text(
            f"[Unit]\n"
            f"Description=Mackes mDNS announce: {name}\n"
            f"After=network-online.target\n"
            f"\n"
            f"[Service]\n"
            f"Type=simple\n"
            f"ExecStart=/usr/bin/avahi-publish-service "
            f"--interface tailscale0 {name} {stype} {port}\n"
            f"Restart=on-failure\n"
            f"RestartSec=5\n"
            f"\n"
            f"[Install]\n"
            f"WantedBy=default.target\n",
            encoding="utf-8",
        )
        subprocess.run(["systemctl", "--user", "enable", "--now",
                        f"mackes-mdns-{name}.service"],
                       capture_output=True, timeout=5)
        actions.append(f"mdns announce: {name} ({stype} on :{port})")
    subprocess.run(["systemctl", "--user", "daemon-reload"],
                   capture_output=True, timeout=5)
    return actions


__all__ = [
    "AvahiHandle", "MDNSHit", "MDNSListener",
    "announce", "scan_once", "is_available",
    "has_zeroconf", "has_avahi_publish",
    "install_announcer_units",
]
