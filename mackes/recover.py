"""TTY-driven recovery for when the GUI won't come up.

Run as `python3 -m mackes.recover`. Lists snapshots and lets the user pick
one to restore. The recovery path is usable from anywhere a terminal works,
including a single-user systemd target with no display.

The systemd `mackes-recovery.target` (data/systemd/) drops the user at a
console with `mackes-recover` in PATH. The GRUB submenu entry
(data/grub/40_mackes_recovery) lets the user boot directly into that target
when the desktop is broken.
"""
from __future__ import annotations

import argparse
import sys

from mackes.snapshots import Snapshot, list_snapshots, restore_snapshot


_BANNER = """
================================================================
  Mackes Recovery
  Restore a previous snapshot. Ctrl-C exits without changes.
================================================================
"""


def _prompt_choice(snaps: list[Snapshot]) -> Snapshot | None:
    print(_BANNER)
    if not snaps:
        print("No snapshots found.")
        return None
    snaps_sorted = sorted(snaps, key=lambda s: s.created, reverse=True)
    for i, s in enumerate(snaps_sorted, 1):
        print(f"  [{i:2d}]  {s.display_label()}")
    print()
    while True:
        try:
            raw = input("Number to restore (or 'q' to quit): ").strip().lower()
        except (EOFError, KeyboardInterrupt):
            print()
            return None
        if raw in ("q", "quit", "exit", ""):
            return None
        if raw.isdigit():
            idx = int(raw)
            if 1 <= idx <= len(snaps_sorted):
                return snaps_sorted[idx - 1]
        print(f"  ?  no such snapshot: {raw!r}")


def _confirm(snap: Snapshot) -> bool:
    print()
    print(f"About to restore snapshot:  {snap.display_label()}")
    print("This overwrites the current xfconf channels and ~/.config/xfce4.")
    try:
        raw = input("Type YES to proceed: ").strip()
    except (EOFError, KeyboardInterrupt):
        return False
    return raw == "YES"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="mackes-recover",
        description="TTY-driven snapshot restore.",
    )
    parser.add_argument(
        "--latest", action="store_true",
        help="restore the most recent snapshot without prompting",
    )
    parser.add_argument("--list", action="store_true", help="list snapshots and exit")
    args = parser.parse_args(argv)

    snaps = list_snapshots()

    if args.list:
        print(_BANNER)
        if not snaps:
            print("No snapshots found.")
            return 0
        for s in sorted(snaps, key=lambda x: x.created, reverse=True):
            print(f"  {s.display_label()}")
        return 0

    if args.latest:
        if not snaps:
            print("No snapshots found; nothing to restore.")
            return 1
        snap = max(snaps, key=lambda s: s.created)
    else:
        snap = _prompt_choice(snaps)
        if snap is None:
            print("(no choice — exiting)")
            return 0
        if not _confirm(snap):
            print("(not confirmed — exiting)")
            return 0

    print(f"\nRestoring {snap.display_label()}…")
    for line in restore_snapshot(snap):
        print(f"  {line}")
    print("\nDone. Reboot or log out for changes to take full effect.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
