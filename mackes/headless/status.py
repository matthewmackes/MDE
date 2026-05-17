"""mackes status / peers / shares — terminal-friendly formatters."""
from __future__ import annotations

import os
import socket

from mackes.state import MackesState, hardware_summary, service_health, last_snapshot


_BOLD = "\033[1m"
_DIM  = "\033[2m"
_RST  = "\033[0m"


def _color(text: str, status: str) -> str:
    if not os.isatty(1):
        return text
    color = {
        "ok":      "\033[32m",
        "warn":    "\033[33m",
        "fail":    "\033[31m",
        "missing": "\033[37m",
    }.get(status, "")
    return f"{color}{text}{_RST}" if color else text


def status() -> int:
    state = MackesState.load()
    hw = hardware_summary()
    svc = service_health()
    snap = last_snapshot()

    print(f"{_BOLD}Mackes Shell{_RST}")
    print(f"  hostname:        {socket.gethostname()}")
    print(f"  preset:          {state.active_preset or '(none)'}")
    print(f"  provisioned:     {state.provisioned}")
    print(f"  last apply:      {state.last_apply or '(never)'}")
    if snap:
        name, when = snap
        print(f"  last snapshot:   {when:%Y-%m-%d %H:%M}  ({name})")
    else:
        print(f"  last snapshot:   (none yet)")
    print()
    print(f"{_BOLD}Hardware{_RST}")
    for k in ("hostname", "os", "cpu", "ram"):
        print(f"  {k:15s} {hw.get(k, '?')}")
    print()
    print(f"{_BOLD}Services{_RST}")
    for name, s in svc.items():
        dot = "●" if s == "ok" else ("○" if s == "missing" else "●")
        print(f"  {_color(dot, s)}  {name:15s} {s}")
    print()

    # Mesh status
    try:
        from mackes.mesh_vpn import MeshState as MV, headscale_list_peers, tailscale_status
        mv = MV.load()
        if mv.mesh_id:
            peers = headscale_list_peers()
            online = sum(1 for p in peers if p.online)
            ctrl_flag = " (control)" if mv.is_control else ""
            print(f"{_BOLD}Mesh{_RST}")
            print(f"  mesh-id:         {mv.mesh_id}")
            print(f"  peers:           {online}/{len(peers)} online{ctrl_flag}")
            ts = tailscale_status()
            print(f"  this peer:       {ts.get('mesh_ip', '(no mesh IP)')}")
    except Exception:  # noqa: BLE001
        pass
    return 0


def peers(json_out: bool = False) -> int:
    try:
        from mackes.mesh_vpn import headscale_list_peers
        ps = headscale_list_peers()
    except Exception as e:  # noqa: BLE001
        print(f"(mesh not reachable: {e})")
        return 1
    if json_out:
        import json as _j
        from dataclasses import asdict
        print(_j.dumps([asdict(p) for p in ps], indent=2))
        return 0
    if not ps:
        print("(no mesh peers)")
        return 0
    print(f"{_BOLD}{'NAME':20s} {'MESH-IP':16s} {'ROUTE':8s} {'STATUS'}{_RST}")
    for p in ps:
        s = "online" if p.online else "offline"
        print(f"{p.name:20s} {p.mesh_ip:16s} {p.route:8s} {s}")
    return 0


def shares() -> int:
    from mackes.mesh_fs import QNM_MESH, QNM_SHARED, is_mounted
    print(f"{_BOLD}Shares served from this peer{_RST}")
    print(f"  {QNM_SHARED}  ({'exists' if QNM_SHARED.exists() else 'missing'})")
    print()
    print(f"{_BOLD}Peer filesystems mounted here{_RST}")
    if not QNM_MESH.exists() or not list(QNM_MESH.iterdir()):
        print("  (none)")
    else:
        for d in sorted(QNM_MESH.iterdir()):
            if d.is_dir():
                mark = "MOUNTED" if is_mounted(d.name) else "stale"
                print(f"  {d.name:20s} {mark}")
    return 0
