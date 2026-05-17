"""Maintain → Health Check.

Combined preflight + validate. Runs through every dependency, every service,
and every Mackes-owned config location, classifying each as ok / warn / fail
and offering a fix link where possible.

Runs synchronously on demand — the checks are cheap (subprocess probes,
file existence). No background polling.
"""
from __future__ import annotations

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402

from mackes.state import (
    HOME, LOG_DIR, SNAPSHOT_DIR, have, service_health,
)
from mackes.workbench._common import (
    info_label, panel_box, section_header, title_label,
)


_SEVERITY_DOT = {"ok": "●", "warn": "●", "fail": "●", "info": "○"}
_SEVERITY_CLASS = {"ok": "success", "warn": "warning", "fail": "error", "info": "dim-label"}


def _check(name: str, severity: str, detail: str) -> tuple[str, str, str, str]:
    return name, severity, detail, ""


def _check_link(name: str, severity: str, detail: str, link_target: str) -> tuple[str, str, str, str]:
    return name, severity, detail, link_target


def _run_all_checks() -> list[tuple[str, str, str, str]]:
    results: list[tuple[str, str, str, str]] = []

    # Core deps
    for cmd in ("xfconf-query", "xfsettingsd", "xfce4-panel", "xfce4-session",
                "xfdesktop", "xfce4-appfinder", "sshd"):
        sev = "ok" if have(cmd) else "fail"
        results.append(_check(f"binary: {cmd}", sev, "found in PATH" if sev == "ok" else "missing"))

    # Network stack
    results.append(_check("binary: nmcli",
                          "ok" if have("nmcli") else "warn",
                          "found" if have("nmcli") else "NetworkManager not installed"))
    results.append(_check("binary: firewall-cmd",
                          "ok" if have("firewall-cmd") else "warn",
                          "found" if have("firewall-cmd") else "firewalld not installed"))
    results.append(_check("binary: timedatectl",
                          "ok" if have("timedatectl") else "fail",
                          "found" if have("timedatectl") else "systemd missing"))

    # Service health (live processes)
    for name, status in service_health().items():
        sev = {"ok": "ok", "warn": "warn", "fail": "fail", "missing": "warn"}[status]
        results.append(_check(f"service: {name}", sev, status))

    # Mackes paths
    results.append(_check_link(
        "log directory", "ok" if LOG_DIR.exists() else "info",
        str(LOG_DIR), "logs",
    ))
    results.append(_check_link(
        "snapshot directory", "ok" if SNAPSHOT_DIR.exists() else "info",
        str(SNAPSHOT_DIR), "snapshots",
    ))

    # Live config dir
    xfce_dir = HOME / ".config" / "xfce4"
    results.append(_check(
        "config: ~/.config/xfce4",
        "ok" if xfce_dir.exists() else "info",
        "present" if xfce_dir.exists() else "not yet created",
    ))

    # xfconf reachability — try a no-op list
    if have("xfconf-query"):
        try:
            import subprocess
            subprocess.check_output(["xfconf-query", "--list"], stderr=subprocess.DEVNULL, timeout=4)
            results.append(_check("xfconf reachable", "ok", "responding"))
        except Exception as e:  # noqa: BLE001
            results.append(_check("xfconf reachable", "fail", str(e)))

    return results


class HealthCheckPanel(Gtk.Box):
    def __init__(self) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self._build()

    def _build(self) -> None:
        box = panel_box()
        box.pack_start(title_label("Health Check"), False, False, 0)
        box.pack_start(info_label(
            "Combined preflight and validate. Each check classifies a "
            "dependency, service or path as ok / warn / fail."
        ), False, False, 0)

        bar = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        run = Gtk.Button(label="Run all checks")
        run.get_style_context().add_class("suggested-action")
        run.connect("clicked", lambda *_: self._run())
        bar.pack_start(run, False, False, 0)
        self._summary = Gtk.Label(label="—"); self._summary.set_xalign(0)
        bar.pack_start(self._summary, True, True, 0)
        box.pack_start(bar, False, False, 0)

        box.pack_start(section_header("Results"), False, False, 0)
        self._results = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        box.pack_start(self._results, False, False, 0)

        self.add(box)
        self._run()

    def _run(self) -> None:
        for child in list(self._results.get_children()):
            self._results.remove(child)
        results = _run_all_checks()

        counts = {"ok": 0, "warn": 0, "fail": 0, "info": 0}
        for _, sev, _, _ in results:
            counts[sev] = counts.get(sev, 0) + 1
        self._summary.set_text(
            f"{counts['ok']} ok · {counts['warn']} warn · {counts['fail']} fail · {counts['info']} info"
        )

        for name, sev, detail, _link in results:
            row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
            dot = Gtk.Label(label=_SEVERITY_DOT.get(sev, "?"))
            dot.get_style_context().add_class(_SEVERITY_CLASS.get(sev, "dim-label"))
            row.pack_start(dot, False, False, 0)

            name_lbl = Gtk.Label(label=name); name_lbl.set_xalign(0); name_lbl.set_size_request(280, -1)
            row.pack_start(name_lbl, False, False, 0)

            detail_lbl = Gtk.Label(label=detail); detail_lbl.set_xalign(0); detail_lbl.set_line_wrap(True)
            detail_lbl.get_style_context().add_class("dim-label")
            row.pack_start(detail_lbl, True, True, 0)

            self._results.pack_start(row, False, False, 0)
        self._results.show_all()
