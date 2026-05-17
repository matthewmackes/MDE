"""Top-level argparse router for `mackes <subcommand>`.

Implements the full subcommand surface from §8.12 (Q-HL3) + the CLI
reference doc (docs/help/cli-reference.md).
"""
from __future__ import annotations

import argparse
import os
import sys
from typing import Optional


def _build_parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(
        prog="mackes",
        description="Mackes Shell — XFCE control panel + mesh fabric.",
    )
    root.add_argument("-V", "--version", action="store_true",
                      help="print version and exit")
    root.add_argument("--gui", action="store_true",
                      help="force GTK / GUI mode")
    root.add_argument("--headless", action="store_true",
                      help="force CLI mode")
    sub = root.add_subparsers(dest="cmd")

    # init
    p_init = sub.add_parser("init", help="first-run setup (headless wizard)")
    p_init.add_argument("--preset", default=None)
    p_init.add_argument("--tailscale-authkey", default=None,
                        help="non-interactive Tailscale auth key")
    p_init.add_argument("--enable-on-boot", action=argparse.BooleanOptionalAction,
                        default=None)
    p_init.add_argument("--join", default=None,
                        help="join an existing mesh via a mesh-join:// link")
    p_init.add_argument("--skip-snapshot", action="store_true")
    p_init.add_argument("--yes", action="store_true",
                        help="accept all interactive defaults")

    # join
    p_join = sub.add_parser("join", help="join an existing mesh")
    p_join.add_argument("link", help="mesh-join:// URL")
    p_join.add_argument("--enable-on-boot", action=argparse.BooleanOptionalAction,
                        default=None)

    sub.add_parser("leave", help="leave the mesh (keeps Mackes installed)")

    # status / peers / shares
    sub.add_parser("status", help="current node state")
    p_peers = sub.add_parser("peers", help="list mesh peers")
    p_peers.add_argument("--json", action="store_true")
    sub.add_parser("shares", help="list SSHFS shares (in/out)")

    # snapshot
    p_snap = sub.add_parser("snapshot", help="manage snapshots")
    snap_sub = p_snap.add_subparsers(dest="snap_cmd")
    p_snap_create = snap_sub.add_parser("create")
    p_snap_create.add_argument("label", nargs="?", default="snapshot")
    snap_sub.add_parser("list")
    p_snap_restore = snap_sub.add_parser("restore")
    p_snap_restore.add_argument("name")
    p_snap_delete = snap_sub.add_parser("delete")
    p_snap_delete.add_argument("name")
    p_snap_show = snap_sub.add_parser("show")
    p_snap_show.add_argument("name")

    # maintain
    p_main = sub.add_parser("maintain", help="repair / health / logs")
    main_sub = p_main.add_subparsers(dest="main_cmd")
    main_sub.add_parser("repair")
    main_sub.add_parser("health")
    p_main_logs = main_sub.add_parser("logs")
    p_main_logs.add_argument("n", nargs="?", type=int, default=50)
    p_main_logs.add_argument("--follow", action="store_true")
    main_sub.add_parser("reset")

    # apps
    p_apps = sub.add_parser("apps", help="install / remove / list apps")
    apps_sub = p_apps.add_subparsers(dest="apps_cmd")
    p_install = apps_sub.add_parser("install")
    p_install.add_argument("names", nargs="+")
    p_remove = apps_sub.add_parser("remove")
    p_remove.add_argument("names", nargs="+")
    p_list = apps_sub.add_parser("list")
    p_list.add_argument("--installed-by-mackes", action="store_true")
    apps_sub.add_parser("catalog")

    # preset
    p_pre = sub.add_parser("preset", help="list / apply preset")
    pre_sub = p_pre.add_subparsers(dest="pre_cmd")
    pre_sub.add_parser("list")
    p_pre_apply = pre_sub.add_parser("apply")
    p_pre_apply.add_argument("name")
    p_pre_show = pre_sub.add_parser("show")
    p_pre_show.add_argument("name")
    pre_sub.add_parser("diff")

    # services
    p_svc = sub.add_parser("services", help="mesh services")
    svc_sub = p_svc.add_subparsers(dest="svc_cmd")
    svc_sub.add_parser("list")
    p_svc_launch = svc_sub.add_parser("launch")
    p_svc_launch.add_argument("name")
    p_svc_launch.add_argument("--peer", default=None)
    svc_sub.add_parser("enable-gateway")
    svc_sub.add_parser("disable-gateway")
    svc_sub.add_parser("catalog")

    # ssh
    p_ssh = sub.add_parser("ssh", help="open a mesh SSH session")
    p_ssh.add_argument("peer")
    p_ssh.add_argument("--layer", choices=["auto", "A", "B"], default="auto")
    p_ssh.add_argument("--user", default=None)
    p_ssh.add_argument("rest", nargs=argparse.REMAINDER,
                       help="optional command to run non-interactively")

    # notify
    p_notify = sub.add_parser("notify", help="send a mesh notification")
    p_notify.add_argument("peer", help="peer name or '*' for broadcast")
    p_notify.add_argument("title")
    p_notify.add_argument("--body", default="")
    p_notify.add_argument("--urgency", default="normal",
                          choices=["low", "normal", "critical"])
    p_notify.add_argument("--icon", default="dialog-information")
    p_notify.add_argument("--all", action="store_true",
                          help="broadcast to every peer")

    # mesh
    p_mesh = sub.add_parser("mesh", help="mesh-VPN specifics")
    mesh_sub = p_mesh.add_subparsers(dest="mesh_cmd")
    mesh_sub.add_parser("status")
    mesh_sub.add_parser("add-peer")
    p_mesh_rm = mesh_sub.add_parser("remove-peer")
    p_mesh_rm.add_argument("name")
    mesh_sub.add_parser("elect-control")
    mesh_sub.add_parser("snapshot")

    # daemon
    sub.add_parser("daemon", help="run the mesh-node daemon (used by systemd)")

    # uninstall
    p_un = sub.add_parser("uninstall", help="remove Mackes Shell entirely")
    p_un.add_argument("--yes", action="store_true",
                      help="bypass interactive confirm")
    p_un.add_argument("--keep-snapshots", action="store_true")

    # help
    p_help = sub.add_parser("help", help="print user-guide topics")
    p_help.add_argument("topic", nargs="?", default=None)
    p_help.add_argument("--open", action="store_true",
                        help="open in $PAGER instead of stdout")

    return root


def main(argv: Optional[list[str]] = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv if argv is not None else sys.argv[1:])

    if args.version:
        from mackes import __version__
        print(f"mackes {__version__}")
        return 0

    cmd = args.cmd

    if cmd is None:
        parser.print_help()
        return 2

    # ---- init / join / leave ----
    if cmd == "init":
        from mackes.headless.wizard import run as wizard_run
        return wizard_run(
            preset=args.preset,
            tailscale_authkey=args.tailscale_authkey,
            enable_on_boot=args.enable_on_boot,
            join_link=args.join,
            skip_snapshot=args.skip_snapshot,
            yes_to_all=args.yes,
        )

    if cmd == "join":
        from mackes.headless.wizard import join as wizard_join
        return wizard_join(args.link)

    if cmd == "leave":
        import subprocess
        rc = subprocess.call(["tailscale", "down"])
        print(f"tailscale down rc={rc}")
        return rc

    # ---- status / peers / shares ----
    from mackes.headless import status as st
    if cmd == "status":
        return st.status()
    if cmd == "peers":
        return st.peers(json_out=args.json)
    if cmd == "shares":
        return st.shares()

    # ---- snapshot ----
    if cmd == "snapshot":
        from mackes.snapshots import create_snapshot, list_snapshots, restore_snapshot, delete_snapshot
        if args.snap_cmd == "create":
            snap = create_snapshot(args.label)
            print(f"created: {snap.name}")
            return 0
        if args.snap_cmd == "list":
            for s in list_snapshots():
                print(s.display_label())
            return 0
        if args.snap_cmd == "restore":
            target = next((s for s in list_snapshots() if s.name == args.name), None)
            if target is None:
                print(f"no such snapshot: {args.name}", file=sys.stderr)
                return 1
            for line in restore_snapshot(target):
                print(line)
            return 0
        if args.snap_cmd == "delete":
            target = next((s for s in list_snapshots() if s.name == args.name), None)
            if target is None:
                print(f"no such snapshot: {args.name}", file=sys.stderr)
                return 1
            delete_snapshot(target)
            print(f"deleted: {args.name}")
            return 0
        if args.snap_cmd == "show":
            target = next((s for s in list_snapshots() if s.name == args.name), None)
            if target is None:
                print(f"no such snapshot: {args.name}", file=sys.stderr)
                return 1
            import json as _j
            print(_j.dumps(target.manifest(), indent=2))
            return 0
        print("snapshot: subcommand required (create/list/restore/delete/show)",
              file=sys.stderr)
        return 2

    # ---- maintain ----
    if cmd == "maintain":
        if args.main_cmd == "repair":
            from mackes.presets import apply_preset, load_preset
            from mackes.state import MackesState
            st_ = MackesState.load()
            if not st_.active_preset:
                print("no active preset set in state.json", file=sys.stderr)
                return 1
            p = load_preset(st_.active_preset)
            if p is None:
                print(f"no such preset: {st_.active_preset}", file=sys.stderr)
                return 1
            for line in apply_preset(p):
                print(line)
            return 0
        if args.main_cmd == "health":
            from mackes.state import service_health, hardware_summary
            ok = True
            for n, s in service_health().items():
                print(f"  {s:>7s}  {n}")
                if s == "fail":
                    ok = False
            return 0 if ok else 1
        if args.main_cmd == "logs":
            from mackes.state import LOG_DIR
            log = LOG_DIR / "mackes.log"
            if not log.exists():
                print("(no log)")
                return 0
            if args.follow:
                import subprocess
                rc = subprocess.call(["tail", "-F", str(log)])
                return rc
            text = log.read_text(encoding="utf-8")
            for ln in text.splitlines()[-args.n:]:
                print(ln)
            return 0
        if args.main_cmd == "reset":
            from mackes.presets import apply_preset, load_preset
            from mackes.state import MackesState
            st_ = MackesState.load()
            if not st_.active_preset:
                return 1
            p = load_preset(st_.active_preset)
            if p:
                for line in apply_preset(p):
                    print(line)
            return 0

    # ---- apps ----
    if cmd == "apps":
        from mackes.app_mgmt import install_app, remove_packages, list_installed_packages, CATALOG
        if args.apps_cmd == "install":
            for name in args.names:
                for line in install_app(name):
                    print(f"  · {line}")
            return 0
        if args.apps_cmd == "remove":
            for line in remove_packages(args.names, category="manual"):
                print(f"  · {line}")
            return 0
        if args.apps_cmd == "list":
            for n, v in list_installed_packages():
                print(f"  {n}  {v}")
            return 0
        if args.apps_cmd == "catalog":
            for name, defn in CATALOG.items():
                print(f"  {name:25s}  ({defn.backend})  {defn.description}")
            return 0

    # ---- preset ----
    if cmd == "preset":
        from mackes.presets import list_presets, load_preset, apply_preset, detect_drift
        if args.pre_cmd == "list":
            for p in list_presets():
                print(f"  {p.name:10s} {p.display_name}")
            return 0
        if args.pre_cmd == "apply":
            p = load_preset(args.name)
            if p is None:
                print(f"no such preset: {args.name}", file=sys.stderr)
                return 1
            for line in apply_preset(p):
                print(line)
            return 0
        if args.pre_cmd == "show":
            p = load_preset(args.name)
            if p is None:
                return 1
            import json as _j
            from dataclasses import asdict
            print(_j.dumps(asdict(p), indent=2, default=str))
            return 0
        if args.pre_cmd == "diff":
            from mackes.state import MackesState
            st_ = MackesState.load()
            if not st_.active_preset:
                return 1
            p = load_preset(st_.active_preset)
            for d in detect_drift(p):
                print(f"  {d.section}.{d.field}: preset={d.expected!r} live={d.actual!r}")
            return 0

    # ---- services ----
    if cmd == "services":
        from mackes.mesh_services import (
            load_catalog, load_registry, probe_all, url_for, launch,
            cheatsheet_lines,
        )
        if args.svc_cmd == "list":
            for line in cheatsheet_lines():
                print(line)
            return 0
        if args.svc_cmd == "launch":
            hits = load_registry()
            match = [h for h in hits if h.service == args.name
                     and (args.peer is None or h.peer == args.peer)]
            if not match:
                print(f"(no matching service: {args.name})", file=sys.stderr)
                return 1
            for line in launch(match[0]):
                print(line)
            return 0
        if args.svc_cmd == "enable-gateway":
            from mackes.caddy_gateway import enable_gateway
            for line in enable_gateway():
                print(line)
            return 0
        if args.svc_cmd == "disable-gateway":
            from mackes.caddy_gateway import disable_gateway
            for line in disable_gateway():
                print(line)
            return 0
        if args.svc_cmd == "catalog":
            for d in load_catalog():
                p = d.port if d.port else "—"
                print(f"  {d.name:25s}  port={p!s:>5s}  {d.description}")
            return 0

    # ---- ssh ----
    if cmd == "ssh":
        from mackes.mesh_ssh import open_session
        return open_session(args.peer, layer=args.layer, user=args.user)

    # ---- notify ----
    if cmd == "notify":
        from mackes.mesh_notifications import send
        target = "*" if args.all else args.peer
        for line in send(target, args.title, body=args.body,
                         urgency=args.urgency, icon=args.icon):
            print(f"  · {line}")
        return 0

    # ---- mesh ----
    if cmd == "mesh":
        from mackes.mesh_vpn import (
            MeshState, generate_join_link, snapshot_state, maybe_take_control,
        )
        if args.mesh_cmd == "status":
            return st.status()
        if args.mesh_cmd == "add-peer":
            link, actions = generate_join_link()
            for a in actions:
                print(f"  · {a}")
            return 0 if link else 1
        if args.mesh_cmd == "remove-peer":
            import subprocess
            from mackes.mesh_vpn import _pkexec_run
            rc, out, err = _pkexec_run(
                ["headscale", "nodes", "delete", "--identifier", args.name],
                timeout=10,
            )
            print((out + err).strip() or f"rc={rc}")
            return rc
        if args.mesh_cmd == "elect-control":
            for line in maybe_take_control():
                print(line)
            return 0
        if args.mesh_cmd == "snapshot":
            for line in snapshot_state():
                print(line)
            return 0

    # ---- daemon ----
    if cmd == "daemon":
        from mackes.headless.daemon import run as daemon_run
        return daemon_run()

    # ---- uninstall ----
    if cmd == "uninstall":
        from mackes.uninstall import run_uninstall, schedule_logout
        if not args.yes:
            try:
                ans = input("Type 'UNINSTALL' to confirm: ").strip()
            except (EOFError, KeyboardInterrupt):
                return 0
            if ans != "UNINSTALL":
                print("(not confirmed)")
                return 0
        report = run_uninstall(progress=lambda s: print(s))
        return 0 if report.failed_count == 0 else 1

    # ---- help ----
    if cmd == "help":
        from mackes.workbench.help import (
            list_topics_plain, render_topic_plain,
        )
        if args.topic is None:
            print(list_topics_plain())
            return 0
        text = render_topic_plain(args.topic)
        if args.open:
            import subprocess
            pager = os.environ.get("PAGER", "less")
            proc = subprocess.Popen([pager], stdin=subprocess.PIPE, text=True)
            try:
                if proc.stdin:
                    proc.stdin.write(text)
                    proc.stdin.close()
                proc.wait()
            except OSError:
                print(text)
            return 0
        print(text)
        return 0

    parser.print_help()
    return 2
