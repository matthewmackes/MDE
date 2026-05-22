#!/bin/sh
# mde-installer-launch — post-RPM TTY installer launcher.
#
# Invoked by mde-installer@tty1.service when
# /var/lib/mde/installer/firstboot.needed exists. Job:
#
#   1. Tear down any live graphical session so the framebuffer is
#      ours. We do this unconditionally — by the time the
#      installer.target has isolated, display-manager.service is
#      already Conflicted out, but a re-run from a live session
#      (the unsupported manual `systemctl start mde-installer@tty1`
#      path) needs an explicit kill.
#   2. Switch to /dev/tty1 (chvt 1).
#   3. Clamp the framebuffer to HD (1920×1080) — the user
#      requirement is "at most HD, never higher." Best-effort;
#      KMS drivers often refuse runtime mode-changes, in which
#      case the kernel `video=1920x1080@60` parameter
#      (mde-installer.target's bootloader stanza) carries the
#      cap from boot. Console font is bumped so the text is
#      legible at that resolution.
#   4. exec mde-wizard --frontend=tui. The wizard's auto-detect
#      collapses to tui because $DISPLAY / $WAYLAND_DISPLAY are
#      unset in the unit environment.
#
# Idempotent: if the flag file is gone (installer already ran),
# the script exits 0 immediately so re-firing the unit is a
# no-op.

set -eu

FLAG_DIR=/var/lib/mde/installer
FLAG=${FLAG_DIR}/firstboot.needed
LOG_DIR=/var/log/mde
LOG=${LOG_DIR}/installer-launch.log

# Log to file; the wizard's about to take over the screen
# anyway, so duplicating to the tty here adds nothing. POSIX
# sh, no process substitution.
mkdir -p "$LOG_DIR" "$FLAG_DIR"
exec >> "$LOG" 2>&1
printf '\n=== mde-installer-launch %s ===\n' "$(date -Iseconds)"

if [ ! -e "$FLAG" ]; then
    echo "no firstboot flag at $FLAG — nothing to do"
    exit 0
fi

# --- 1. Tear down live graphical session (compositor + DM). ---
#
# Order matters: stop the DM first so it doesn't immediately
# respawn the session we're about to kill.
if command -v systemctl >/dev/null 2>&1; then
    if systemctl is-active --quiet display-manager.service 2>/dev/null; then
        echo "stopping display-manager.service"
        systemctl stop display-manager.service || true
    fi
    # Active user sessions on graphical seats — terminate so
    # the wizard owns the seat.
    if command -v loginctl >/dev/null 2>&1; then
        for sid in $(loginctl list-sessions --no-legend 2>/dev/null \
                     | awk '{print $1}'); do
            type=$(loginctl show-session "$sid" -p Type --value 2>/dev/null || echo "")
            case "$type" in
                wayland|x11)
                    echo "terminating session $sid (type=$type)"
                    loginctl terminate-session "$sid" || true
                    ;;
            esac
        done
    fi
fi

# --- 2. Switch to tty1. ---
if command -v chvt >/dev/null 2>&1; then
    chvt 1 || echo "warning: chvt 1 failed (no permissions or no kbd subsystem)"
fi

# --- 3. Clamp framebuffer + bump console font. ---
#
# The cap is 1920×1080 ("HD or lower, never higher"). Anything
# smaller is left alone. fbset's exit status isn't reliable
# across drivers (Nouveau in particular returns 0 on a no-op),
# so we read back the geometry after the call and log it for
# the installer log even when the call silently failed.
if command -v fbset >/dev/null 2>&1; then
    cur=$(fbset 2>/dev/null \
          | awk '/^[[:space:]]*geometry/ {print $2 "x" $3; exit}' \
          || true)
    echo "current framebuffer geometry: ${cur:-unknown}"
    case "${cur:-}" in
        ""|0x0)
            echo "framebuffer geometry unreadable — skipping clamp (kernel video= line carries it)"
            ;;
        *)
            # Parse "WxH" and compare to 1920x1080.
            w=$(echo "$cur" | cut -dx -f1)
            h=$(echo "$cur" | cut -dx -f2)
            if [ "${w:-0}" -gt 1920 ] || [ "${h:-0}" -gt 1080 ]; then
                echo "clamping ${w}x${h} → 1920x1080"
                fbset -g 1920 1080 1920 1080 32 \
                    || echo "warning: fbset clamp failed (driver may reject runtime mode change)"
            else
                echo "framebuffer ${w}x${h} already ≤ HD — no clamp"
            fi
            ;;
    esac
else
    echo "fbset not present — relying on kernel video= line"
fi

# Larger console font so 1080p panels are legible. We try
# terminus first (most installs have it via the kbd package),
# then fall back to whatever default the kernel ships.
if command -v setfont >/dev/null 2>&1; then
    setfont ter-132n 2>/dev/null \
        || setfont Lat15-Terminus32x16 2>/dev/null \
        || setfont ter-v32n 2>/dev/null \
        || echo "no Terminus font available — keeping default"
fi

# --- 4. Hand off to the wizard. ---
#
# tput clear gives the wizard a clean canvas. We don't need to
# clear the log file (the wizard writes its own /var/log/mde/
# entries). exec replaces this shell so the wizard owns the tty.
if command -v tput >/dev/null 2>&1; then
    tput clear 2>/dev/null || true
fi

echo "launching mde-wizard --frontend=tui"
exec /usr/bin/mde-wizard --frontend=tui
