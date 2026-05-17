"""Standalone entry point for the mDNS relay (started by systemd)."""
from __future__ import annotations

import signal
import sys
import time

from mackes.logging import log_action
from mackes.mdns_relay import loop_once

_RUNNING = True


def _sigterm(_a, _b):
    global _RUNNING
    _RUNNING = False


def main() -> int:
    signal.signal(signal.SIGTERM, _sigterm)
    signal.signal(signal.SIGINT,  _sigterm)
    log_action("mdns-relay: starting")
    while _RUNNING:
        try:
            for line in loop_once():
                log_action(f"mdns-relay: {line}")
        except Exception as e:  # noqa: BLE001
            log_action(f"mdns-relay: loop crashed: {e}")
        for _ in range(30):
            if not _RUNNING:
                break
            time.sleep(1)
    log_action("mdns-relay: stopped")
    return 0


if __name__ == "__main__":
    sys.exit(main())
