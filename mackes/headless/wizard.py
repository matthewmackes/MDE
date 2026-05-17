"""mackes init — headless first-run wizard (Q-HL2: pure stdin prompts)."""
from __future__ import annotations

import os
import socket
from typing import Optional

from mackes.logging import log_action
from mackes.state import MackesState, ensure_dirs, hardware_summary


_BOLD = "\033[1m"
_GREEN = "\033[32m"
_RED   = "\033[31m"
_DIM   = "\033[2m"
_RST   = "\033[0m"


def _bold(s: str) -> str:
    return f"{_BOLD}{s}{_RST}" if os.isatty(1) else s


def _ok(s: str) -> str:
    return f"{_GREEN}{s}{_RST}" if os.isatty(1) else s


def _err(s: str) -> str:
    return f"{_RED}{s}{_RST}" if os.isatty(1) else s


def _ask(prompt: str, default: Optional[str] = None) -> str:
    if default is not None:
        suffix = f" [{default}]"
    else:
        suffix = ""
    while True:
        try:
            ans = input(f"{prompt}{suffix}: ").strip()
        except (EOFError, KeyboardInterrupt):
            print()
            return ""
        if ans:
            return ans
        if default is not None:
            return default


def _ask_yes(prompt: str, *, default: bool = True) -> bool:
    suffix = " [Y/n]" if default else " [y/N]"
    while True:
        try:
            ans = input(f"{prompt}{suffix}: ").strip().lower()
        except (EOFError, KeyboardInterrupt):
            print()
            return default
        if not ans:
            return default
        if ans in ("y", "yes"):
            return True
        if ans in ("n", "no"):
            return False


def run(*, preset: Optional[str] = None,
        tailscale_authkey: Optional[str] = None,
        enable_on_boot: Optional[bool] = None,
        join_link: Optional[str] = None,
        skip_snapshot: bool = False,
        yes_to_all: bool = False) -> int:
    """Run the headless wizard. Returns 0 on success.

    Flags map to `mackes init --preset / --tailscale-authkey /
    --enable-on-boot / --join / --skip-snapshot / --yes`.
    """
    ensure_dirs()
    state = MackesState.load()

    print(_bold("Mackes Shell 1.0.0 — first-run setup"))
    print()

    # ---- environment ----
    hw = hardware_summary()
    print(_bold("Environment"))
    for k in ("hostname", "os", "cpu", "ram"):
        print(f"  {k:9s} {hw.get(k, '?')}")
    print()

    # ---- preset choice ----
    if preset is None:
        # Headless defaults to 'node'; ask user to confirm
        preset = "node"
        if not yes_to_all and not _ask_yes(
                f"Apply preset {_bold(preset)} (recommended for headless)?",
                default=True):
            from mackes.presets import list_presets
            available = [p.name for p in list_presets()]
            print(f"  available: {', '.join(available)}")
            preset = _ask("Preset to apply", default="node")
    print(f"  preset: {_ok(preset)}")
    print()

    # ---- join vs. seed ----
    if join_link:
        from mackes.mesh_vpn import join_existing_mesh
        ok, actions = join_existing_mesh(join_link)
        for a in actions:
            print(f"  · {a}")
        if not ok:
            return 1
    elif state.provisioned and state.active_preset:
        print(f"  state.json already provisioned with preset='{state.active_preset}'. "
              f"Re-applying.")
    else:
        # Seed-peer path
        from mackes.mesh_vpn import bootstrap_seed_peer, is_first_peer
        if is_first_peer():
            def _print_url(url: str) -> None:
                print()
                print(_bold("→ Tailscale device-auth"))
                print(f"  Open this URL on any device, sign in, then press Enter here:")
                print(f"    {_ok(url)}")
                try:
                    input("  Press Enter when authenticated: ")
                except (EOFError, KeyboardInterrupt):
                    pass
            ok, actions = bootstrap_seed_peer(
                tailscale_authkey=tailscale_authkey,
                interactive_login_callback=None if tailscale_authkey else _print_url,
            )
            for a in actions:
                print(f"  · {a}")
            if not ok:
                return 1

    # ---- preset apply ----
    from mackes.presets import load_preset, apply_preset
    p = load_preset(preset)
    if p is None:
        print(_err(f"  ERROR: preset '{preset}' not found"))
        return 2
    print()
    print(_bold(f"Applying preset {preset}…"))
    for line in apply_preset(p):
        print(f"  · {line}")

    # ---- snapshot ----
    if not skip_snapshot:
        from mackes.snapshots import create_snapshot
        snap = create_snapshot(label=f"{preset}-baseline", source_preset=preset)
        print(f"  · snapshot created: {snap.name}")

    # ---- mesh-ssh keypair (Q-HL7 — backend services participation) ----
    from mackes.mesh_ssh import ensure_mesh_keypair, publish_my_pubkey
    for a in ensure_mesh_keypair():
        print(f"  · {a}")
    for a in publish_my_pubkey():
        print(f"  · {a}")

    # ---- enable on boot? ----
    if enable_on_boot is None:
        enable_on_boot = yes_to_all or _ask_yes(
            "Auto-start mesh node on boot?", default=True,
        )
    if enable_on_boot:
        import shutil, subprocess
        pkexec = shutil.which("pkexec") or shutil.which("sudo") or ""
        if pkexec:
            rc = subprocess.call([pkexec, "systemctl", "enable", "--now",
                                  "mackes-node.service"])
            print(f"  · systemctl enable --now mackes-node rc={rc}")
        else:
            print("  · pkexec/sudo missing; run `systemctl enable --now mackes-node` manually")

    # ---- finalize ----
    state.mark_provisioned(preset)
    log_action(f"headless init: preset={preset} enable-on-boot={enable_on_boot}")
    print()
    print(_ok(_bold(f"Done. Welcome to {preset}.")))
    return 0


def join(link: str) -> int:
    """`mackes join <link>` — non-interactive join."""
    from mackes.mesh_vpn import join_existing_mesh
    ok, actions = join_existing_mesh(link)
    for a in actions:
        print(f"  · {a}")
    return 0 if ok else 1
