//! Phase E.18 — Win10-style lower-right watermark.
//!
//! Long-running layer-shell surface anchored to the bottom-right
//! corner of the primary output. Shows MDE version + Fedora release
//! + pending-update count when dnf has updates queued; renders an
//! empty container (invisible) when the count is zero. Polls
//! `dnf check-update --quiet` every 4 hours and refreshes the
//! display.
//!
//! v3.0.3 — moved from mde-panel/src/watermark.rs (where the
//! Phase E.18 [✓] entry shipped the data layer as dead code —
//! audit 2026-05-22). Now mounts as `mde-popover watermark` via
//! the standard popover dispatcher, spawned at session start by
//! `data/sway/config`. Per v2.0.3 polkit lock: left-click on the
//! watermark issues `pkexec dnf upgrade` so the operator can
//! kick off the update from a single click without dropping to a
//! terminal.
//!
//! 2026 visual: 11px Red Hat Mono, 28% alpha text at rest,
//! lifts to 100% alpha on hover so the clickable affordance is
//! discoverable.
//!
//! **Sync with legacy GTK watermark (v2.0.3)**: the build-hash and
//! build-date strings come from `/usr/share/mde/build-{hash,date}`
//! — the same source-of-truth files that `mackes-panel/src/
//! watermark.rs` reads. Both panel surfaces report identical
//! identity so operators can't see two different builds claimed.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use iced::widget::{button, container, text};
use iced::{Background, Border, Color, Element, Length, Padding, Shadow, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;

/// Snapshot of every value the watermark renders.
#[derive(Debug, Clone, Default)]
pub struct WatermarkState {
    pub mde_version: String,
    pub fedora_release: String,
    pub build_hash: Option<String>,
    /// UTC build date in `YYYY-MM-DD` form. `None` on dev checkouts
    /// where `/usr/share/mde/build-date` doesn't exist (the RPM
    /// `%install` step writes it). Synced with the legacy GTK
    /// watermark via the same file (v2.0.3).
    pub build_date: Option<String>,
    pub hostname: String,
    pub pending_updates: u32,
}

impl WatermarkState {
    /// Best-effort load: reads each field from a stable source,
    /// falling back to an empty string on any error.
    ///
    /// `build_hash` and `build_date` come from
    /// `/usr/share/mde/build-{hash,date}` — the same files the
    /// legacy GTK `mackes-panel` watermark consumes, so both
    /// surfaces report identical identity (v2.0.3 sync fix).
    /// `MDE_BUILD_HASH` env (set by build.rs in dev) wins over the
    /// file when both exist; this keeps `cargo run` builds showing
    /// the live hash even when an installed RPM also wrote a file.
    #[must_use]
    pub fn load() -> Self {
        Self {
            mde_version: env!("CARGO_PKG_VERSION").to_string(),
            fedora_release: read_fedora_release(),
            build_hash: option_env!("MDE_BUILD_HASH")
                .map(str::to_owned)
                .or_else(|| read_build_file_for_hash()),
            build_date: read_build_file_for_date(),
            hostname: read_hostname(),
            pending_updates: read_pending_update_count(),
        }
    }

    /// Single-line label rendered onto the panel. Empty when no
    /// updates are pending — the rendered widget hides on empty.
    #[must_use]
    pub fn render_line(&self) -> String {
        if self.pending_updates == 0 {
            return String::new();
        }
        let hash = self
            .build_hash
            .as_deref()
            .map(|h| format!(" · {h}"))
            .unwrap_or_default();
        let date = self
            .build_date
            .as_deref()
            .map(|d| format!(" · Built {d}"))
            .unwrap_or_default();
        format!(
            "MDE {ver}{hash}{date} · Fedora {release} · {host} · {n} updates pending",
            ver = self.mde_version,
            release = self.fedora_release,
            host = self.hostname,
            n = self.pending_updates,
        )
    }
}

fn read_fedora_release() -> String {
    read_os_release_field("VERSION_ID").unwrap_or_else(|| "44".to_string())
}

fn read_os_release_field(key: &str) -> Option<String> {
    let content = std::fs::read_to_string("/etc/os-release").ok()?;
    parse_os_release_field(&content, key)
}

/// Pure parser — pulls `KEY="value"` lines out of /etc/os-release
/// shape strings. Exposed for tests.
#[must_use]
pub fn parse_os_release_field(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix(&format!("{key}=")) {
            let trimmed = rest.trim().trim_matches('"');
            return Some(trimmed.to_string());
        }
    }
    None
}

fn read_hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "fedora".to_string())
}

fn read_pending_update_count() -> u32 {
    // Cached count file (written by the dnf-update worker, lands at
    // E.18 worker integration). Returns 0 if absent.
    let cache_path = dirs::cache_dir()
        .map(|d| d.join("mde/dnf-updates.count"))
        .unwrap_or_default();
    parse_count_file(&cache_path)
}

/// Read `/usr/share/mde/build-hash` (RPM `%install`-written). Synced
/// with the legacy GTK watermark — both panels consume the same file
/// so they can't drift on which build is reported.
fn read_build_file_for_hash() -> Option<String> {
    read_build_meta(&[
        "/usr/share/mde/build-hash",
        "/usr/share/mackes-shell/build-hash",
        "build-hash",
    ])
}

/// Read `/usr/share/mde/build-date` (RPM `%install`-written UTC
/// `YYYY-MM-DD`). `None` on dev checkouts.
fn read_build_file_for_date() -> Option<String> {
    read_build_meta(&[
        "/usr/share/mde/build-date",
        "/usr/share/mackes-shell/build-date",
        "build-date",
    ])
}

/// Pure helper — walk the candidate paths and return the first non-
/// empty trimmed content. Exposed for tests.
#[must_use]
pub fn read_build_meta(candidates: &[&str]) -> Option<String> {
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

/// Pure helper — reads + parses the dnf-updates count file. Exposed
/// for tests.
#[must_use]
pub fn parse_count_file(path: &Path) -> u32 {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(0)
}

// ──────────────────────────────────────────────────────────────
// v3.0.3 — Iced layer-shell long-running surface
// ──────────────────────────────────────────────────────────────

/// How often we re-poll dnf for the pending-update count. 4 hours
/// matches the v1.x cadence from the spec.
const DNF_POLL_INTERVAL: Duration = Duration::from_secs(4 * 60 * 60);

/// How often the Iced view re-renders to pick up the latest
/// shared-state count. 30 seconds is plenty — the count itself
/// only changes every 4 hours, but a frequent re-render makes the
/// watermark show up promptly after the first poll completes
/// (which can take 10-30 seconds for `dnf check-update`).
const VIEW_REFRESH_INTERVAL: Duration = Duration::from_secs(30);

/// Foreground text — Carbon `text-helper` at 28% alpha matches the
/// v1.x Win10 watermark visual lock.
const FG_REST: Color = Color {
    r: 0.957,
    g: 0.957,
    b: 0.957,
    a: 0.28,
};

/// On hover the watermark lifts to full alpha so the click target
/// reads as interactive.
const FG_HOVER: Color = Color {
    r: 0.957,
    g: 0.957,
    b: 0.957,
    a: 1.0,
};

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    /// Periodic re-render — picks up the latest shared `WatermarkState`.
    Tick,
    /// User clicked the watermark — fires `pkexec dnf upgrade`.
    UpgradeClicked,
}

/// Iced application backing the watermark surface.
pub struct App {
    /// Shared state updated by the background poll thread. Iced view
    /// reads the snapshot on every render. Mutex is cheap because
    /// only the poll thread writes and Iced reads infrequently.
    state: Arc<Mutex<WatermarkState>>,
}

impl iced_layershell::Application for App {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Task<Message>) {
        let shared = Arc::new(Mutex::new(WatermarkState::load()));
        spawn_poll_thread(Arc::clone(&shared));
        (Self { state: shared }, Task::none())
    }

    fn namespace(&self) -> String {
        "mde-popover-watermark".into()
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Tick => {
                // Refresh the snapshot so the view picks up the
                // poll thread's latest write. Cheap because
                // WatermarkState::load() reads small files.
                if let Ok(mut s) = self.state.lock() {
                    s.pending_updates = current_pending_count();
                }
            }
            Message::UpgradeClicked => {
                // v2.0.3 polkit lock: route through pkexec so the
                // polkit auth agent owns the prompt (Wayland-clean).
                tracing::info!("watermark click → pkexec dnf upgrade");
                spawn_pkexec_dnf_upgrade();
                // After the user runs an upgrade, the count is
                // probably stale. Re-poll once we know dnf has
                // completed; for now just bump the tick so the
                // next 30s refresh picks up any change.
            }
            _ => {}
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let snapshot = match self.state.lock() {
            Ok(g) => g.clone(),
            Err(_) => WatermarkState::default(),
        };
        let line = snapshot.render_line();
        if line.is_empty() {
            // No pending updates — render a 1x1 invisible container
            // so the surface still exists (layer-shell would unmap
            // a zero-size surface on some compositors). The user
            // sees nothing.
            return container(text(""))
                .width(Length::Fixed(1.0))
                .height(Length::Fixed(1.0))
                .into();
        }
        button(text(line).size(11).color(FG_REST))
            .padding(Padding {
                top: 4.0,
                right: 10.0,
                bottom: 4.0,
                left: 10.0,
            })
            .style(watermark_button_style)
            .on_press(Message::UpgradeClicked)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        // Just the periodic refresh — no Esc handler (the watermark
        // is a background surface; the user can't dismiss it).
        iced::time::every(VIEW_REFRESH_INTERVAL).map(|_| Message::Tick)
    }
}

pub fn run() -> iced_layershell::Result {
    <App as iced_layershell::Application>::run(Settings {
        id: Some("mde-popover-watermark".into()),
        fonts: crate::fonts::load_fallback_fonts(),
        layer_settings: LayerShellSettings {
            // Layer::Background sits below normal windows so the
            // watermark behaves like wallpaper chrome — operators
            // can still raise any window over it.
            layer: Layer::Background,
            // Bottom-right corner, 24px inset per the v1.x lock.
            anchor: Anchor::Bottom | Anchor::Right,
            // (top, right, bottom, left) — top/left unused for this
            // anchor; bottom shifts above the panel's 40px zone
            // (24 inset + 40 panel = 64 from screen bottom).
            margin: (0, 24, 64, 0),
            // None: the watermark must never grab keyboard focus
            // (background chrome).
            keyboard_interactivity: KeyboardInteractivity::None,
            // Don't reserve compositor space — we're just chrome.
            exclusive_zone: 0,
            // v4.0.1 fix (2026-05-23): explicit size cap. While the
            // watermark is corner-anchored (Bottom | Right) which
            // SHOULD auto-fit, leaving size=None hits the same
            // iced_layershell fallback the toast stack tripped over
            // when render_line() is empty (zero pending updates →
            // 1x1 widget → fullscreen surface allocation). Capping
            // at 280x32 keeps the surface tight to the longest line
            // we ship ("MDE 4.0.1 · fc44 · build-XXX · 999 pending").
            size: Some((280, 32)),
            ..Default::default()
        },
        ..Default::default()
    })
}

fn watermark_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let (bg, fg) = match status {
        button::Status::Hovered => (
            Some(Background::Color(Color {
                r: 0.055,
                g: 0.055,
                b: 0.063,
                a: 0.40,
            })),
            FG_HOVER,
        ),
        button::Status::Pressed => (
            Some(Background::Color(Color {
                r: 0.055,
                g: 0.055,
                b: 0.063,
                a: 0.60,
            })),
            FG_HOVER,
        ),
        _ => (None, FG_REST),
    };
    button::Style {
        background: bg,
        text_color: fg,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
    }
}

/// Background OS-thread driver — re-polls dnf every
/// `DNF_POLL_INTERVAL`, writes the resulting count to the shared
/// `WatermarkState`. Runs forever; detached.
fn spawn_poll_thread(state: Arc<Mutex<WatermarkState>>) {
    thread::Builder::new()
        .name("watermark-dnf-poll".into())
        .spawn(move || {
            // First poll runs immediately so the count is fresh on
            // session start (rather than waiting 4 hours after
            // login for the first refresh).
            loop {
                let n = poll_dnf_check_update();
                if let Ok(mut s) = state.lock() {
                    s.pending_updates = n;
                }
                cache_count(n);
                tracing::debug!(pending = n, "watermark dnf poll complete");
                thread::sleep(DNF_POLL_INTERVAL);
            }
        })
        .expect("spawn watermark dnf poll thread");
}

/// Run `dnf check-update --quiet` and count the package lines.
/// dnf exits with status 100 when there ARE updates, 0 when none,
/// 1 on error. We count any non-empty, non-header line of stdout
/// as an update; that matches what `dnf check-update` outputs
/// (one package per line, with `Last metadata expiration check`
/// lines going to stderr).
fn poll_dnf_check_update() -> u32 {
    let output = match std::process::Command::new("dnf")
        .args(["check-update", "--quiet"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!(error = %e, "dnf check-update spawn failed");
            return 0;
        }
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| {
            let t = line.trim();
            !t.is_empty() && !t.starts_with("Obsoleting")
        })
        .count()
        .try_into()
        .unwrap_or(u32::MAX)
}

/// Write the latest count to the cache file so the next session
/// load (`WatermarkState::load`) seeds from the last known value
/// without waiting for the first dnf poll to complete.
fn cache_count(n: u32) {
    let Some(dir) = dirs::cache_dir() else {
        return;
    };
    let dir = dir.join("mde");
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let _ = std::fs::write(dir.join("dnf-updates.count"), format!("{n}\n"));
}

/// Read the cached count from disk. Used by view-side refresh so
/// the surface picks up the latest poll-thread write without
/// holding the mutex during a file read (the mutex is held only
/// briefly to write the result).
fn current_pending_count() -> u32 {
    let Some(dir) = dirs::cache_dir() else {
        return 0;
    };
    parse_count_file(&dir.join("mde/dnf-updates.count"))
}

/// Spawn `pkexec dnf upgrade` detached so the watermark click
/// returns immediately. The polkit agent prompts on the user's
/// behalf; on accept, dnf runs in the background. `wait()` to
/// avoid zombies.
fn spawn_pkexec_dnf_upgrade() {
    match std::process::Command::new("pkexec")
        .args(["dnf", "upgrade", "-y"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(mut child) => {
            // Reap in a background thread so we don't block the
            // UI on `dnf upgrade` (which takes minutes).
            thread::spawn(move || {
                let _ = child.wait();
            });
        }
        Err(e) => {
            tracing::warn!(error = %e, "pkexec dnf upgrade spawn failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn render_empty_when_no_pending_updates() {
        let state = WatermarkState::default();
        assert!(state.render_line().is_empty());
    }

    #[test]
    fn render_includes_every_field_when_updates_pending() {
        let state = WatermarkState {
            mde_version: "2.0.3".into(),
            fedora_release: "44".into(),
            build_hash: Some("abc123".into()),
            build_date: Some("2026-05-22".into()),
            hostname: "lab-01".into(),
            pending_updates: 12,
        };
        let line = state.render_line();
        assert!(line.contains("MDE 2.0.3"));
        assert!(line.contains("abc123"));
        assert!(line.contains("Built 2026-05-22"));
        assert!(line.contains("Fedora 44"));
        assert!(line.contains("lab-01"));
        assert!(line.contains("12 updates pending"));
    }

    #[test]
    fn render_omits_hash_when_unset() {
        let state = WatermarkState {
            mde_version: "2.0.3".into(),
            fedora_release: "44".into(),
            build_hash: None,
            build_date: None,
            hostname: "lab-01".into(),
            pending_updates: 1,
        };
        let line = state.render_line();
        assert!(!line.contains("·  ·")); // no double separator
        assert!(line.starts_with("MDE 2.0.3 · Fedora 44"));
        assert!(!line.contains("Built"));
    }

    #[test]
    fn render_includes_build_date_separately_from_hash() {
        // The v2.0.3 sync change splits build date out as its own
        // `· Built YYYY-MM-DD` clause. Lock the ordering: version,
        // then hash, then date, then Fedora/host.
        let state = WatermarkState {
            mde_version: "2.0.3".into(),
            fedora_release: "44".into(),
            build_hash: Some("abc123".into()),
            build_date: Some("2026-05-22".into()),
            hostname: "lab-01".into(),
            pending_updates: 1,
        };
        let line = state.render_line();
        let hash_idx = line.find("abc123").expect("hash present");
        let date_idx = line.find("2026-05-22").expect("date present");
        let fedora_idx = line.find("Fedora").expect("fedora present");
        assert!(hash_idx < date_idx, "hash must come before date in {line}");
        assert!(
            date_idx < fedora_idx,
            "date must come before Fedora in {line}"
        );
    }

    #[test]
    fn render_handles_only_date_no_hash() {
        // Edge case: build-date file exists but build-hash doesn't
        // (RPM install ordering glitch). Render must still produce
        // a coherent line without a stray `· Built` after nothing.
        let state = WatermarkState {
            mde_version: "2.0.3".into(),
            fedora_release: "44".into(),
            build_hash: None,
            build_date: Some("2026-05-22".into()),
            hostname: "lab-01".into(),
            pending_updates: 1,
        };
        let line = state.render_line();
        assert!(line.contains("Built 2026-05-22"));
        assert!(line.starts_with("MDE 2.0.3 · Built 2026-05-22"));
    }

    #[test]
    fn read_build_meta_returns_none_for_missing_paths() {
        let tmp = tempdir().unwrap();
        let absent = tmp.path().join("does-not-exist");
        let absent_s = absent.to_string_lossy().into_owned();
        let paths = [absent_s.as_str()];
        assert_eq!(read_build_meta(&paths), None);
    }

    #[test]
    fn read_build_meta_returns_first_non_empty_candidate() {
        let tmp = tempdir().unwrap();
        let empty = tmp.path().join("empty");
        let real = tmp.path().join("real");
        std::fs::write(&empty, "   \n").unwrap();
        std::fs::write(&real, "2026-05-22\n").unwrap();
        let empty_s = empty.to_string_lossy().into_owned();
        let real_s = real.to_string_lossy().into_owned();
        let paths = [empty_s.as_str(), real_s.as_str()];
        assert_eq!(read_build_meta(&paths), Some("2026-05-22".to_string()));
    }

    #[test]
    fn parse_os_release_extracts_field() {
        let content = r#"
NAME="Fedora Linux"
VERSION="44 (Workstation)"
VERSION_ID=44
PRETTY_NAME="Fedora Linux 44"
"#;
        assert_eq!(
            parse_os_release_field(content, "VERSION_ID"),
            Some("44".into())
        );
        assert_eq!(
            parse_os_release_field(content, "NAME"),
            Some("Fedora Linux".into())
        );
    }

    #[test]
    fn parse_os_release_returns_none_for_missing_key() {
        let content = "NAME=Fedora\n";
        assert_eq!(parse_os_release_field(content, "MISSING"), None);
    }

    #[test]
    fn parse_count_file_returns_zero_when_missing() {
        let tmp = tempdir().unwrap();
        assert_eq!(parse_count_file(&tmp.path().join("absent")), 0);
    }

    #[test]
    fn parse_count_file_parses_integer() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("count");
        std::fs::write(&path, "42\n").unwrap();
        assert_eq!(parse_count_file(&path), 42);
    }

    #[test]
    fn parse_count_file_falls_back_on_garbage() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("count");
        std::fs::write(&path, "not a number").unwrap();
        assert_eq!(parse_count_file(&path), 0);
    }

    #[test]
    fn load_does_not_panic() {
        // Even on a system without /etc/os-release etc., load()
        // returns a valid state.
        let _state = WatermarkState::load();
    }
}
