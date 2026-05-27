//! Portal-48 (v6.0, R12-Q8 + R12-Q10) — auto-mark daemon.
//!
//! Subscribes to sway's `EventType::Window` and, on every
//! `WindowChange::New`, classifies the new window's `app_id` against
//! a static taxonomy table and runs `[con_id=N] mark --add <name>`
//! when a match exists. Five buckets:
//!
//!   * `editor`  — helix, code, vim, emacs, nvim, kakoune
//!   * `web`     — firefox, chromium, librewolf, qutebrowser, brave
//!   * `shell`   — foot, alacritty, kitty, wezterm, ghostty
//!   * `mail`    — thunderbird, geary, evolution, mutt
//!   * `chat`    — discord, element-desktop, signal-desktop,
//!                 telegram-desktop, slack
//!
//! Operator marks (anything already on `node.marks` at `window::new`
//! time) are preserved; the daemon only adds a taxonomy mark when
//! the new window has zero marks, so an operator pre-marking a
//! window via `swaymsg mark <foo>` before launch (rare but valid)
//! still wins.
//!
//! Marks are sway-session-ephemeral: not GFS-synced, not persisted
//! to disk. The downstream Portal-49 running-zone mark-pill render
//! reads them via `swaymsg get_tree` directly (lowest-latency,
//! no IPC).
//!
//! The cross-peer `dev.mackes.MDE.AutoMark.GetMarks()` zbus surface
//! from the original Portal-48 spec is deferred to a Portal-48.b
//! follow-on per CLAUDE.md §0.14 (newer-wins): Q20 + Q96 of the
//! 100-Q tightening survey lock the canonical IPC for MDE-internal
//! control on Mackes Bus, not D-Bus. The local auto-marking half
//! ships here so Portal-49 can render against real data; the
//! cross-peer surface lands on Bus once the broker is fully
//! wired (BUS-1.2+).

#![cfg(feature = "async-services")]

use std::time::Duration;

use futures_util::StreamExt as _;
use swayipc_async::{Connection, EventType};

use super::{ShutdownToken, Worker};

/// Backoff after a swayipc connect failure. Matches the
/// `workspace_namer` worker's cadence (Portal-41) for fleet-wide
/// reconnect lockstep.
const RECONNECT_BACKOFF: Duration = Duration::from_secs(3);

/// Empty-state worker; all state lives on the stack inside `run`.
pub struct AutoMarkWorker;

impl AutoMarkWorker {
    /// Construct a fresh worker. No configuration — taxonomy is
    /// compile-time-locked.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for AutoMarkWorker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Worker for AutoMarkWorker {
    fn name(&self) -> &'static str {
        "auto_mark"
    }

    async fn run(&mut self, mut shutdown: ShutdownToken) -> anyhow::Result<()> {
        loop {
            if shutdown.is_shutdown() {
                return Ok(());
            }
            let mut cmd_conn = match Connection::new().await {
                Ok(c) => c,
                Err(e) => {
                    tracing::debug!(error = %e, "auto_mark cmd-conn connect failed; backing off");
                    sleep_or_shutdown(RECONNECT_BACKOFF, &mut shutdown).await;
                    continue;
                }
            };
            let event_conn = match Connection::new().await {
                Ok(c) => c,
                Err(e) => {
                    tracing::debug!(error = %e, "auto_mark event-conn connect failed; backing off");
                    sleep_or_shutdown(RECONNECT_BACKOFF, &mut shutdown).await;
                    continue;
                }
            };
            let mut events = match event_conn.subscribe([EventType::Window]).await {
                Ok(stream) => stream,
                Err(e) => {
                    tracing::debug!(error = %e, "auto_mark subscribe failed; backing off");
                    sleep_or_shutdown(RECONNECT_BACKOFF, &mut shutdown).await;
                    continue;
                }
            };
            loop {
                tokio::select! {
                    biased;
                    _ = shutdown.wait() => return Ok(()),
                    next = events.next() => {
                        match next {
                            Some(Ok(swayipc_async::Event::Window(win_ev))) => {
                                if win_ev.change == swayipc_async::WindowChange::New {
                                    handle_new_window(&mut cmd_conn, &win_ev.container).await;
                                }
                            }
                            Some(Ok(_)) => {}
                            Some(Err(e)) => {
                                tracing::debug!(error = %e, "auto_mark event stream errored; reconnecting");
                                break;
                            }
                            None => {
                                tracing::debug!("auto_mark event stream ended; reconnecting");
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn sleep_or_shutdown(dur: Duration, shutdown: &mut ShutdownToken) {
    tokio::select! {
        _ = shutdown.wait() => {}
        _ = tokio::time::sleep(dur) => {}
    }
}

/// Handle a `WindowChange::New` event. Inspects the container's
/// `app_id` + `marks`; if the taxonomy table matches AND marks is
/// empty, fires `[con_id=N] mark --add <taxonomy>`. Existing
/// operator marks are preserved (the daemon never overwrites).
async fn handle_new_window(conn: &mut Connection, container: &swayipc_async::Node) {
    let Some(taxonomy) = decide_mark(container.app_id.as_deref(), &container.marks) else {
        return;
    };
    let con_id = container.id;
    let cmd = format!("[con_id={con_id}] mark --add {taxonomy}");
    match conn.run_command(&cmd).await {
        Ok(_) => tracing::debug!(con_id, app_id = ?container.app_id, %taxonomy, "auto_mark applied"),
        Err(e) => tracing::warn!(con_id, %taxonomy, error = %e, "auto_mark command failed"),
    }
}

// ── Pure helpers (testable without a sway connection) ───────────────────

/// Compile-time taxonomy table. Returns `Some(category)` if `app_id`
/// matches one of the five buckets, `None` otherwise. The mapping is
/// frozen at compile time per the Portal-48 design lock — any new
/// entries land via a new commit, not via config.
#[must_use]
pub fn taxonomy_for_app_id(app_id: &str) -> Option<&'static str> {
    match app_id {
        "helix" | "code" | "vim" | "emacs" | "nvim" | "kakoune" => Some("editor"),
        "firefox" | "chromium" | "librewolf" | "qutebrowser" | "brave" => Some("web"),
        "foot" | "alacritty" | "kitty" | "wezterm" | "ghostty" => Some("shell"),
        "thunderbird" | "geary" | "evolution" | "mutt" => Some("mail"),
        "discord" | "element-desktop" | "signal-desktop" | "telegram-desktop" | "slack" => {
            Some("chat")
        }
        _ => None,
    }
}

/// Decide whether to apply a taxonomy mark. Returns `Some(taxonomy)`
/// if `app_id` matches the taxonomy AND `existing_marks` is empty;
/// `None` otherwise (no app_id, unknown app, or marks already
/// present). Operator marks always win — the daemon never overwrites.
#[must_use]
pub fn decide_mark(app_id: Option<&str>, existing_marks: &[String]) -> Option<&'static str> {
    if !existing_marks.is_empty() {
        return None;
    }
    let app_id = app_id?;
    if app_id.is_empty() {
        return None;
    }
    taxonomy_for_app_id(app_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Five canonical app_ids — one per bucket — round-trip
    /// through `taxonomy_for_app_id` to their bucket name.
    #[test]
    fn taxonomy_table_matches_canonical_app_ids() {
        assert_eq!(taxonomy_for_app_id("firefox"), Some("web"));
        assert_eq!(taxonomy_for_app_id("helix"), Some("editor"));
        assert_eq!(taxonomy_for_app_id("foot"), Some("shell"));
        assert_eq!(taxonomy_for_app_id("thunderbird"), Some("mail"));
        assert_eq!(taxonomy_for_app_id("discord"), Some("chat"));
    }

    /// All 25 taxonomy entries land in the table.
    #[test]
    fn taxonomy_table_covers_all_buckets() {
        // editor (6)
        for app in ["helix", "code", "vim", "emacs", "nvim", "kakoune"] {
            assert_eq!(taxonomy_for_app_id(app), Some("editor"), "{app}");
        }
        // web (5)
        for app in ["firefox", "chromium", "librewolf", "qutebrowser", "brave"] {
            assert_eq!(taxonomy_for_app_id(app), Some("web"), "{app}");
        }
        // shell (5)
        for app in ["foot", "alacritty", "kitty", "wezterm", "ghostty"] {
            assert_eq!(taxonomy_for_app_id(app), Some("shell"), "{app}");
        }
        // mail (4)
        for app in ["thunderbird", "geary", "evolution", "mutt"] {
            assert_eq!(taxonomy_for_app_id(app), Some("mail"), "{app}");
        }
        // chat (5)
        for app in [
            "discord", "element-desktop", "signal-desktop",
            "telegram-desktop", "slack",
        ] {
            assert_eq!(taxonomy_for_app_id(app), Some("chat"), "{app}");
        }
    }

    /// Unknown app_ids return None — no auto-mark, no error.
    #[test]
    fn unknown_app_ids_are_passthrough() {
        assert_eq!(taxonomy_for_app_id("unknown-app"), None);
        assert_eq!(taxonomy_for_app_id("org.mozilla.Firefox"), None); // exact-match only
        assert_eq!(taxonomy_for_app_id("FIREFOX"), None); // case-sensitive
        assert_eq!(taxonomy_for_app_id(""), None);
    }

    /// Operator marks block auto-marking — if any existing mark is
    /// present on the new window's container, the daemon skips.
    #[test]
    fn existing_marks_block_auto_mark() {
        // Operator already marked firefox with a custom name.
        let existing = vec!["work".to_string()];
        assert_eq!(decide_mark(Some("firefox"), &existing), None);
    }

    /// Empty marks + matching taxonomy = mark gets applied.
    #[test]
    fn empty_marks_with_matching_taxonomy_applies() {
        let no_marks: Vec<String> = Vec::new();
        assert_eq!(decide_mark(Some("firefox"), &no_marks), Some("web"));
        assert_eq!(decide_mark(Some("foot"), &no_marks), Some("shell"));
    }

    /// Empty app_id never marks — covers the xwayland case where
    /// app_id is None.
    #[test]
    fn empty_or_missing_app_id_skips() {
        let no_marks: Vec<String> = Vec::new();
        assert_eq!(decide_mark(None, &no_marks), None);
        assert_eq!(decide_mark(Some(""), &no_marks), None);
    }

    /// Unknown app_id + empty marks = still skip. The daemon doesn't
    /// invent marks for apps outside the taxonomy.
    #[test]
    fn unknown_app_with_empty_marks_skips() {
        let no_marks: Vec<String> = Vec::new();
        assert_eq!(decide_mark(Some("custom-app"), &no_marks), None);
    }
}
