#!/bin/bash
# mde-parity-overlay — sync mackes-shell repo updates onto the running
# installed MDE without cutting a fresh RPM. v4.0.1 (PARITY-1).
#
# Two-phase design:
#   --build  (runs as the developer user mm) — `cargo build --release
#            --workspace` so any Rust crate changes produce fresh
#            binaries in target/release/.
#   --install (runs as root via sudo) — copy newer Python modules into
#            site-packages, newer .desktop files into /usr/share/
#            applications/, newer cargo binaries into /usr/bin/.
#            Refresh desktop database + icon cache. Lock-protected.
#
# Invoked with no arguments (the systemd-user .service path) it runs
# --build first, then re-execs itself via `sudo -n` to enter --install.
# The sudoers drop-in (install-helpers/sudoers-mde-parity) grants the
# operator passwordless exec of EXACTLY this script.
#
# Idempotent: a no-change run prints the timestamp + "summary: py=0
# desktop=0 bin=0" and exits 0. Safe to invoke from a systemd .path
# unit that fires on every .git/refs/heads/main update.

set -euo pipefail

# ---- Configuration ---------------------------------------------------------

REPO=${MDE_PARITY_REPO:-/home/mm/Desktop/files/mackes-shell}
DEVUSER=${MDE_PARITY_USER:-mm}
SITE=${MDE_PARITY_SITE_PACKAGES:-/usr/lib/python3.14/site-packages}
APPS=${MDE_PARITY_APPS:-/usr/share/applications}
BIN=${MDE_PARITY_BIN:-/usr/bin}
LOCK=${MDE_PARITY_LOCK:-/run/mde-parity.lock}
LOG=${MDE_PARITY_LOG:-/var/log/mde-parity.log}

# Whitelist of binary-name prefixes we will install. Anything outside
# this list in target/release/ (e.g. example binaries, build deps) is
# skipped. Keeps the privileged copy path tightly scoped.
BIN_WHITELIST_PREFIXES=("mde-" "mde_" "mded" "mackes-" "mackesd")

# ---- Helpers ---------------------------------------------------------------

log() { printf '%s %s\n' "$(date -Iseconds)" "$*" >&2; }

ensure_log() {
    if [ ! -f "$LOG" ]; then
        install -m 0664 -o root -g root /dev/null "$LOG"
    fi
}

is_whitelisted_bin() {
    local base="$1"
    for p in "${BIN_WHITELIST_PREFIXES[@]}"; do
        case "$base" in "$p"*) return 0 ;; esac
    done
    return 1
}

# ---- Phase 1: build (runs as $DEVUSER) -------------------------------------

phase_build() {
    [ "$(id -un)" = "$DEVUSER" ] || {
        log "ERROR: --build must run as $DEVUSER (currently $(id -un))"
        exit 1
    }
    cd "$REPO"
    log "phase=build cargo build --release --workspace"
    # cargo output goes to the operator's stdout/stderr only. The log
    # file is root-owned and writable only by the install phase; if we
    # tried to tee here from the unprivileged build phase, the failed
    # tee would mask cargo's actual exit status and the install phase
    # would never fire. So: rely on cargo's own progress output, and
    # gate FAILED on cargo's real exit status via PIPESTATUS.
    cargo build --release --workspace
    rc=$?
    if [ "$rc" -ne 0 ]; then
        log "phase=build FAILED (cargo exit=$rc)"
        return 1
    fi
    log "phase=build done"
}

# ---- Phase 2: install (runs as root) ---------------------------------------

phase_install() {
    [ "$EUID" -eq 0 ] || {
        log "ERROR: --install must run as root (currently uid $EUID)"
        exit 1
    }
    ensure_log
    exec 9>"$LOCK"
    if ! flock -n 9; then
        log "another overlay holds $LOCK; exiting"
        exit 0
    fi

    {
        log "==== phase=install starting ===="

        # 1) Python overlay
        local py_changed=0
        while IFS= read -r f; do
            local rel="${f#mackes/}"
            local src="$REPO/$f"
            local dst="$SITE/mackes/$rel"
            if [ ! -f "$dst" ] || [ "$src" -nt "$dst" ]; then
                install -D -m 0644 "$src" "$dst"
                log "py: $rel"
                py_changed=$((py_changed + 1))
            fi
        done < <(cd "$REPO" && find mackes -type f -name '*.py')
        if [ "$py_changed" -gt 0 ]; then
            find "$SITE/mackes" -name '__pycache__' -prune -exec rm -rf {} + 2>/dev/null || true
        fi

        # 2) .desktop files
        local desktop_changed=0
        if [ -d "$REPO/data/applications" ]; then
            for f in "$REPO"/data/applications/*.desktop; do
                [ -f "$f" ] || continue
                local dst="$APPS/$(basename "$f")"
                if [ ! -f "$dst" ] || [ "$f" -nt "$dst" ]; then
                    install -m 0644 "$f" "$dst"
                    log "desktop: $(basename "$f")"
                    desktop_changed=$((desktop_changed + 1))
                fi
            done
        fi
        if [ "$desktop_changed" -gt 0 ] && command -v update-desktop-database >/dev/null; then
            update-desktop-database "$APPS" 2>/dev/null || true
        fi

        # 3) Rust binaries from target/release/
        local bin_changed=0
        if [ -d "$REPO/target/release" ]; then
            for src in "$REPO"/target/release/*; do
                [ -f "$src" ] || continue
                [ -x "$src" ] || continue
                local base
                base="$(basename "$src")"
                is_whitelisted_bin "$base" || continue
                local dst="$BIN/$base"
                if [ ! -f "$dst" ] || [ "$src" -nt "$dst" ]; then
                    install -m 0755 "$src" "$dst"
                    log "bin: $base"
                    bin_changed=$((bin_changed + 1))
                fi
            done
        fi

        # 4) sway config.d drop-ins. v4.0.1 WM-6 (2026-05-23) —
        #    new *.conf files in data/sway/config.d/ need to land
        #    in every operator's ~/.config/sway/config.d/. The
        #    mde-shell-migrate-v2 first-boot path seeds only when
        #    ~/.config/sway/ is empty; for in-place upgrades the
        #    parity overlay rsyncs the dir on every tick. The user
        #    home is derived from $SUDO_USER (overlay re-execs
        #    itself as root for install phase).
        local sway_changed=0
        local devhome
        if [ -n "${SUDO_USER:-}" ]; then
            devhome="$(getent passwd "$SUDO_USER" | cut -d: -f6)"
        else
            devhome=""
        fi
        if [ -n "$devhome" ] && [ -d "$REPO/data/sway/config.d" ]; then
            install -d -o "$SUDO_USER" -g "$SUDO_USER" \
                "$devhome/.config/sway/config.d"
            for f in "$REPO"/data/sway/config.d/*.conf; do
                [ -f "$f" ] || continue
                local dst="$devhome/.config/sway/config.d/$(basename "$f")"
                if [ ! -f "$dst" ] || [ "$f" -nt "$dst" ]; then
                    install -m 0644 -o "$SUDO_USER" -g "$SUDO_USER" "$f" "$dst"
                    log "sway-config.d: $(basename "$f")"
                    sway_changed=$((sway_changed + 1))
                fi
            done
        fi

        # 5) Restart the panel + popover stack when a panel-stack
        #    binary actually changed this tick. v4.0.1 PARITY-6
        #    (2026-05-23) — without this, the running mde-panel
        #    keeps its old code in memory after parity ticks
        #    until the operator manually kills it. Helper runs
        #    as $SUDO_USER (needs the user's WAYLAND_DISPLAY /
        #    DBUS_SESSION_BUS_ADDRESS to spawn into the live
        #    sway session).
        local restart_log="skipped"
        # The bin: log lines just emitted name everything that
        # landed this tick. Grep for the panel-stack subset to
        # decide whether the helper needs to fire. Workbench /
        # files / mackesd updates don't need a restart (the
        # workbench / files window is the operator's to reopen;
        # mackesd has its own systemd unit).
        local need_restart=0
        if [ "$bin_changed" -gt 0 ] && [ -n "$devhome" ]; then
            if tail -n 30 "$LOG" 2>/dev/null \
                | grep -E 'bin:\s+(mde-panel|mde-popover|mde-applet-)' \
                  >/dev/null 2>&1; then
                need_restart=1
            fi
        fi
        if [ "$need_restart" -eq 1 ] \
            && [ -x "$REPO/install-helpers/restart-panel-stack.sh" ]; then
            log "restart-panel-stack: panel-stack binary changed, respawning"
            local uid
            uid="$(id -u "$SUDO_USER" 2>/dev/null)"
            if [ -n "$uid" ] && [ -d "/run/user/$uid" ]; then
                if sudo -u "$SUDO_USER" \
                    XDG_RUNTIME_DIR="/run/user/$uid" \
                    WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-1}" \
                    DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$uid/bus" \
                    "$REPO/install-helpers/restart-panel-stack.sh" all \
                    >>"$LOG" 2>&1; then
                    restart_log="ok"
                else
                    restart_log="not-in-session"
                fi
            else
                restart_log="no-active-session"
            fi
        fi

        log "summary: py=$py_changed desktop=$desktop_changed bin=$bin_changed sway=$sway_changed restart=$restart_log"
        log "==== phase=install done ===="
    } 2>&1 | tee -a "$LOG"
}

# ---- Default entry: build then install -------------------------------------

main() {
    case "${1:-}" in
        --build)   phase_build ;;
        --install) phase_install ;;
        "")
            # Build as the current user, then re-exec via sudo for install.
            "$0" --build
            log "re-exec via sudo -n for install phase"
            exec sudo -n "$0" --install
            ;;
        *)
            cat >&2 <<USAGE
usage: $0 [--build|--install]
  --build    (run as $DEVUSER) cargo build --release --workspace
  --install  (run as root)     overlay-install changed files
  (no args)  build then sudo-exec install
USAGE
            exit 2
            ;;
    esac
}

main "$@"
