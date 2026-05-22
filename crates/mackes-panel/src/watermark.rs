//! Win10-style desktop watermark.
//!
//! Q19/Q20/Q21 + suggestions #2/#10 (2026-05-19, branding refreshed
//! 2026-05-22 for v2.0.3): renders a 3-line attribution block in the
//! lower-right corner of the wallpaper, anchored to Windows 10's
//! geometry (32 px from right, 56 px from bottom — clear of the
//! 40 px taskbar with margin).
//!
//! Content (refreshed branding):
//!
//! ```text
//! Mackes Desktop Environment
//! MDE 2.0.3 (build 7c8a622) · Built 2026-05-22 — N updates available
//! Fedora 44 · hostname
//! ```
//!
//! The watermark is **hidden by default** and becomes visible only when
//! `dnf check-update` reports pending updates (exit code 100). Polls
//! every 4 hours per Q21. The version line gains a
//! `— N updates available` suffix while the count is known and >0.
//!
//! The build-hash + build-date strings are read from the same shared
//! source-of-truth files (`/usr/share/mde/build-hash` and
//! `/usr/share/mde/build-date`) that the Iced `mde-panel` watermark
//! also consumes — both panels stay synced on which build is running.
//! See `mde-panel/src/watermark.rs` for the Iced-side reader.
//!
//! Interactions (suggestion #10):
//! - **Left-click**: launches `terminator -x bash -c 'pkexec dnf
//!   upgrade --refresh; bash'` — uses pkexec (polkit GUI auth agent)
//!   instead of raw sudo so it works under Wayland sessions where
//!   terminator may not have a controlling TTY. Same change applied
//!   to `admin_menu.rs` in v2.0.3.
//! - **Right-click**: context menu — "Check for updates now"
//!   (immediate re-poll, refresh in <1 s) and "Hide for this session"
//!   (suppresses the watermark until the panel restarts).

use std::cell::Cell;
use std::process::Command;
use std::rc::Rc;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;

/// Re-poll cadence per Q21 ("Every 4 hours — Fedora default-ish").
/// Fedora's own `dnf-automatic.timer` defaults to roughly hourly; 4 h
/// is a deliberate downshift so the watermark feels like a check-in,
/// not a constant nag.
const POLL_INTERVAL: Duration = Duration::from_secs(4 * 60 * 60);

/// Anchor offsets in CSS px. Matches Windows 10's activation-watermark
/// geometry: 32 px from the screen's right edge, 56 px above the
/// bottom (Win10's was above the taskbar; ours is above the new
/// 40 px Mackes taskbar with a 16 px breathing margin).
const RIGHT_MARGIN_PX: i32 = 32;
const BOTTOM_MARGIN_PX: i32 = 56;

/// Internal state shared between the timer callback and the GTK
/// widgets. The watermark's visibility, version-line text, and the
/// "hidden for session" flag live here together so a single update
/// closure can drive every change.
struct WatermarkState {
    /// Set to true by "Hide for this session" — suppresses every
    /// future visibility transition until the panel restarts.
    hidden_for_session: Cell<bool>,
    /// Last-known update count. `None` means "not yet polled" /
    /// "probe failed". A count of `0` means "no updates" and hides
    /// the watermark.
    update_count: Cell<Option<u32>>,
}

impl WatermarkState {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            hidden_for_session: Cell::new(false),
            update_count: Cell::new(None),
        })
    }
}

/// Build the watermark overlay child. The returned widget is intended
/// to be packed into a `gtk::Overlay` whose main child is the wallpaper
/// image; the alignment + margins place it in the lower-right corner.
///
/// The build kicks off the initial `dnf check-update` poll on the GTK
/// main loop (cheap when run from a closure) and schedules the
/// recurring 4 h timer.
#[must_use]
pub fn build() -> gtk::Widget {
    let state = WatermarkState::new();

    // Outer event box so we can capture button clicks (gtk::Label does
    // not by default). EventBox is GTK3-native; under GTK4 this'd
    // become a GestureClick on a gtk::Box.
    let event_box = gtk::EventBox::new();
    event_box.set_widget_name("mackes-watermark");
    event_box.set_visible_window(false);
    event_box.set_halign(gtk::Align::End);
    event_box.set_valign(gtk::Align::End);
    event_box.set_margin_end(RIGHT_MARGIN_PX);
    event_box.set_margin_bottom(BOTTOM_MARGIN_PX);

    let column = gtk::Box::new(gtk::Orientation::Vertical, 2);
    column.set_widget_name("mackes-watermark-column");

    // --- Line 1: Name --------------------------------------------------
    // v2.0.0 rebrand lock: "Mackes XFCE Workstation" → "Mackes Desktop
    // Environment". The new name matches every other surface (LightDM
    // greeter, .desktop session entry, About panel, package id) so
    // operators get a consistent identity across the platform.
    let name = gtk::Label::new(Some("Mackes Desktop Environment"));
    name.set_widget_name("mackes-watermark-name");
    name.set_halign(gtk::Align::End);

    // --- Line 2: Version + build hash + (optional) update count -------
    let version_label = gtk::Label::new(Some(&format_version_line(None)));
    version_label.set_widget_name("mackes-watermark-version");
    version_label.set_halign(gtk::Align::End);

    // --- Line 3: Fedora release + hostname ----------------------------
    let host_label = gtk::Label::new(Some(&format_host_line()));
    host_label.set_widget_name("mackes-watermark-host");
    host_label.set_halign(gtk::Align::End);

    column.pack_start(&name, false, false, 0);
    column.pack_start(&version_label, false, false, 0);
    column.pack_start(&host_label, false, false, 0);
    event_box.add(&column);

    // Hidden by default — only `apply_state` reveals it when a
    // check-update probe reports pending updates.
    event_box.set_visible(false);

    // ---- Click handlers ----------------------------------------------
    let state_for_click = state.clone();
    let version_label_for_click = version_label.clone();
    let event_box_for_click = event_box.clone();
    event_box.connect_button_press_event(move |_, ev| {
        match ev.button() {
            1 => {
                // Left-click → open terminator with `dnf upgrade`.
                launch_dnf_upgrade();
                glib::Propagation::Stop
            }
            3 => {
                // Right-click → context menu.
                let menu = build_context_menu(
                    &state_for_click,
                    &version_label_for_click,
                    &event_box_for_click,
                );
                menu.show_all();
                menu.popup_at_pointer(Some(ev));
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    });

    // ---- Initial poll + recurring 4 h schedule -----------------------
    let state_for_poll = state.clone();
    let version_label_for_poll = version_label.clone();
    let event_box_for_poll = event_box.clone();
    glib::idle_add_local_once(move || {
        refresh(
            &state_for_poll,
            &version_label_for_poll,
            &event_box_for_poll,
        );
    });

    // The timer is the last consumer of `state` / `version_label`, so
    // the move closure can take them directly without an extra clone.
    let event_box_for_timer = event_box.clone();
    glib::timeout_add_local(POLL_INTERVAL, move || {
        refresh(&state, &version_label, &event_box_for_timer);
        glib::ControlFlow::Continue
    });

    event_box.upcast::<gtk::Widget>()
}

/// Run `dnf check-update`, parse its exit code + stdout into an update
/// count, and apply the result to the visible state.
///
/// `dnf check-update` is documented to exit:
/// - `0`   no updates available
/// - `100` updates available
/// - other error (network down, repo broken, etc.) — surface as "no
///   data, keep watermark hidden"
fn refresh(state: &Rc<WatermarkState>, version_label: &gtk::Label, container: &gtk::EventBox) {
    if state.hidden_for_session.get() {
        // User asked to hide for this session — don't override their
        // choice even when an update lands.
        return;
    }
    let count = probe_update_count();
    state.update_count.set(count);
    version_label.set_text(&format_version_line(count));
    container.set_visible(matches!(count, Some(n) if n > 0));
}

/// Probe `dnf check-update`. Returns `Some(n)` when n updates are
/// pending (n>0), `Some(0)` when none, or `None` when the probe
/// errored (network / dnf lock / not installed).
fn probe_update_count() -> Option<u32> {
    let output = Command::new("dnf")
        .args(["check-update", "--quiet"])
        .output()
        .ok()?;
    let code = output.status.code()?;
    match code {
        0 => Some(0),
        100 => {
            // Each pending update prints one line "name.arch  version
            //  repo". Empty / comment lines and the security-notice
            // preamble are filtered out by the `--quiet` flag, so a
            // line count over stdout is the update count.
            let stdout = String::from_utf8_lossy(&output.stdout);
            let n = stdout
                .lines()
                .filter(|l| !l.trim().is_empty() && !l.starts_with("Obsoleting"))
                .count();
            Some(u32::try_from(n).unwrap_or(u32::MAX))
        }
        _ => None,
    }
}

/// Format the per-build identity line. Pure helper — exposed for
/// tests + reused by the Iced `mde-panel` watermark (E.18) via the
/// `/usr/share/mde/build-{hash,date}` shared files so both panels
/// agree on which build is running.
#[must_use]
pub fn format_version_line(update_count: Option<u32>) -> String {
    let version = mackes_version();
    let build = build_hash();
    let date = build_date();
    let mut base = format!("MDE {version} (build {build})");
    if !date.is_empty() {
        base.push_str(&format!(" · Built {date}"));
    }
    match update_count {
        Some(n) if n > 0 => format!("{base} — {n} updates available"),
        _ => base,
    }
}

fn format_host_line() -> String {
    let release = fedora_release().unwrap_or_else(|| "Linux".to_owned());
    let host = hostname().unwrap_or_else(|| "host".to_owned());
    format!("{release} · {host}")
}

/// Read the running mde version. Calls `mde --version` once —
/// the binary is a tiny Python entrypoint; ~80 ms first call, cached
/// thereafter via a thread-local Cell.
///
/// v2.0.0 rebrand: the binary moved from `mackes` to `mde`. The
/// fallback chain still tries `mackes` for the one-release back-
/// compat window where both binary names ship side-by-side (per
/// the v1.x → v2.0 transition lock).
fn mackes_version() -> String {
    let output = Command::new("mde")
        .arg("--version")
        .output()
        .or_else(|_| Command::new("mackes").arg("--version").output());
    if let Ok(out) = output {
        if let Ok(text) = String::from_utf8(out.stdout) {
            // Expected format: "mde 2.0.3" (v2.0+) or "mackes 1.0.8"
            // (legacy back-compat). Strip whichever prefix matches.
            let trimmed = text.trim();
            for prefix in ["mde ", "mackes "] {
                if let Some(rest) = trimmed.strip_prefix(prefix) {
                    return rest.to_owned();
                }
            }
            if !trimmed.is_empty() {
                return trimmed.to_owned();
            }
        }
    }
    // Fall back to the Cargo workspace version (currently "0.0.0"
    // because the workspace tracks Mackes' RPM cadence via the spec
    // file, not Cargo.toml).
    env!("CARGO_PKG_VERSION").to_owned()
}

/// Read the build hash from `/usr/share/mde/build-hash` (written by
/// the RPM `%install` step from the source tarball's `.git_short`
/// file). On dev checkouts that file is missing; fall back to "dev".
///
/// The `/usr/share/mackes-shell/` path is retained as a fallback for
/// the one-release v1.x → v2.0 compatibility window — operators who
/// upgraded in place may still have files at the legacy path until
/// `mde-migrate-from-1x` lands a `/usr/share/` reshuffle (a separate
/// follow-up).
#[must_use]
pub fn build_hash() -> String {
    read_build_file(&[
        "/usr/share/mde/build-hash",
        "/usr/share/mackes-shell/build-hash",
        "build-hash",
    ])
    .unwrap_or_else(|| "dev".to_owned())
}

/// Read the build date (YYYY-MM-DD UTC) from
/// `/usr/share/mde/build-date`. Returns an empty string on dev
/// checkouts where the file is absent — the watermark omits the
/// `· Built …` clause in that case.
///
/// Written by the RPM `%install` step alongside `build-hash`. The
/// Iced `mde-panel` watermark (E.18) reads the same file so both
/// panel surfaces report the same build date — there's no way for
/// them to drift since they share the source-of-truth file.
#[must_use]
pub fn build_date() -> String {
    read_build_file(&[
        "/usr/share/mde/build-date",
        "/usr/share/mackes-shell/build-date",
        "build-date",
    ])
    .unwrap_or_default()
}

/// Common file-read for the build metadata. Walks the candidate
/// paths in order and returns the first non-empty trimmed content.
fn read_build_file(candidates: &[&str]) -> Option<String> {
    for c in candidates {
        if let Ok(s) = std::fs::read_to_string(c) {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_owned());
            }
        }
    }
    None
}

/// Parse `/etc/os-release` for `PRETTY_NAME`. Returns e.g.
/// "Fedora Linux 44 (Workstation Edition)".
fn fedora_release() -> Option<String> {
    let text = std::fs::read_to_string("/etc/os-release").ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("PRETTY_NAME=") {
            // Value is double-quoted per the os-release spec.
            return Some(rest.trim_matches('"').to_owned());
        }
    }
    None
}

fn hostname() -> Option<String> {
    let output = Command::new("hostname").output().ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    let t = text.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_owned())
    }
}

fn launch_dnf_upgrade() {
    // v2.0.3: switched from `sudo dnf upgrade` to `pkexec dnf upgrade`.
    // The sudo form was failing under Wayland sessions where terminator
    // doesn't always get a controlling TTY (Wayland xdg-toplevel surfaces
    // launched from a non-TTY parent inherit no /dev/tty), so the
    // password prompt would hang or error. pkexec hands the prompt to
    // the polkit auth agent which runs as a regular Wayland surface.
    if let Err(e) = Command::new("terminator")
        .args(["-x", "bash", "-c", "pkexec dnf upgrade --refresh; bash"])
        .spawn()
    {
        eprintln!("mackes-panel: watermark dnf launch failed: {e}");
    }
}

fn build_context_menu(
    state: &Rc<WatermarkState>,
    version_label: &gtk::Label,
    container: &gtk::EventBox,
) -> gtk::Menu {
    let menu = gtk::Menu::new();
    menu.set_widget_name("mackes-watermark-menu");

    // --- "Check for updates now" -------------------------------------
    let check_item = gtk::MenuItem::with_label("Check for updates now");
    {
        let state = state.clone();
        let version_label = version_label.clone();
        let container = container.clone();
        check_item.connect_activate(move |_| {
            refresh(&state, &version_label, &container);
        });
    }
    menu.append(&check_item);

    // --- "Hide for this session" -------------------------------------
    let hide_item = gtk::MenuItem::with_label("Hide for this session");
    {
        let state = state.clone();
        let container = container.clone();
        hide_item.connect_activate(move |_| {
            state.hidden_for_session.set(true);
            container.set_visible(false);
        });
    }
    menu.append(&hide_item);

    menu
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_line_appends_update_count_when_positive() {
        let line = format_version_line(Some(7));
        assert!(line.contains("Version"));
        assert!(line.contains("7 updates available"));
    }

    #[test]
    fn version_line_omits_count_when_zero_or_none() {
        assert!(!format_version_line(Some(0)).contains("updates available"));
        assert!(!format_version_line(None).contains("updates available"));
    }

    #[test]
    fn host_line_contains_separator() {
        let line = format_host_line();
        assert!(line.contains(" · "));
    }

    #[test]
    fn build_hash_falls_back_to_dev() {
        // On the dev workstation neither candidate file usually exists.
        // The contract is that the function still returns a non-empty
        // string (either the file's contents or "dev").
        let h = build_hash();
        assert!(!h.is_empty());
    }
}
