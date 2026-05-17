"""Headless preset apply — `python3 -m mackes.cli_apply --preset NAME`.

Same pipeline the wizard's Apply page runs, minus the GUI. Useful for:
  - Re-applying a preset over SSH
  - Automation / fleet rollout (run the same command on N machines)
  - Recovery flows where the GUI won't come up

Exits non-zero if any step's `actions` list contains a string starting with
"error", "ERROR", or "failed".
"""
from __future__ import annotations

import argparse
import sys

from mackes.presets import (
    apply_appearance, apply_devices, apply_mesh, apply_network, apply_panel,
    apply_system, list_presets, load_preset,
)
from mackes.state import MackesState


_STEPS = [
    ("appearance", apply_appearance),
    ("devices",    apply_devices),
    ("system",     apply_system),
    ("network",    apply_network),
    ("panel",      apply_panel),
    ("mesh",       apply_mesh),
]


def _is_error_line(line: str) -> bool:
    lower = line.lower()
    return any(lower.startswith(p) for p in ("error", "failed"))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="python -m mackes.cli_apply",
        description="Apply a Mackes preset headlessly.",
    )
    parser.add_argument(
        "--preset", required=False,
        help="preset name (default: state.active_preset, or DEFAULT_PRESET_NAME)",
    )
    parser.add_argument("--list", action="store_true",
                        help="list available presets and exit")
    parser.add_argument("--dry-run", action="store_true",
                        help="show what would happen, no writes (note: today "
                             "each step is its own dry-run gate; this flag "
                             "documents intent only)")
    parser.add_argument("--quiet", action="store_true",
                        help="suppress per-line output; print only summary")
    args = parser.parse_args(argv)

    if args.list:
        for p in list_presets():
            print(f"  {p.name:12s} {p.display_name}")
        return 0

    name = args.preset
    if name is None:
        state = MackesState.load()
        name = state.active_preset
    if not name:
        from mackes.presets import DEFAULT_PRESET_NAME
        name = DEFAULT_PRESET_NAME

    preset = load_preset(name)
    if preset is None:
        print(f"error: no such preset: {name!r}", file=sys.stderr)
        return 2

    if not args.quiet:
        print(f"→ applying preset: {preset.display_name} ({preset.name})")

    errors = 0
    for step_name, fn in _STEPS:
        if not args.quiet:
            print(f"  step: {step_name}")
        try:
            for line in (fn(preset) or []):
                if not args.quiet:
                    print(f"    {line}")
                if _is_error_line(line):
                    errors += 1
        except Exception as e:  # noqa: BLE001
            errors += 1
            print(f"  step {step_name} crashed: {e}", file=sys.stderr)

    if not args.quiet:
        print(f"\ndone. errors={errors}")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
