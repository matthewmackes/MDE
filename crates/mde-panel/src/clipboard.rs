//! Phase E.5 — clipboard via `wl-clipboard`.
//!
//! Best-choice deviation from the original "wlr-data-control via
//! smithay-client-toolkit" lock: the `wl-clipboard` package (the
//! `wl-copy` + `wl-paste` binaries) is the canonical
//! command-line interface to the wlr-data-control protocol on
//! every Wayland-on-wlroots compositor. It's a 50-line
//! subprocess wrapper instead of 500 lines of SCTK protocol
//! boilerplate.
//!
//! The mesh-replication path (`~/.cache/mde/clipboard.json`)
//! stays unchanged — `mded` writes it on every paste broadcast
//! and reads from it on peer connect. This module owns the
//! local Wayland side; mded owns the mesh side.

use std::io::Write;
use std::process::{Command, Stdio};

/// Read the current clipboard contents (text only). Returns
/// None when no text payload is available or `wl-paste` fails.
#[must_use]
pub fn paste_text() -> Option<String> {
    let out = Command::new("wl-paste")
        .args(["--no-newline", "--type", "text/plain"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Write `text` to the clipboard. Returns `Err` on subprocess
/// failure.
pub fn copy_text(text: &str) -> std::io::Result<()> {
    let mut child = Command::new("wl-copy")
        .args(["--type", "text/plain"])
        .stdin(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }
    let status = child.wait()?;
    if !status.success() {
        return Err(std::io::Error::other(format!(
            "wl-copy exited with {status}"
        )));
    }
    Ok(())
}

/// Read mime-types currently available in the clipboard.
#[must_use]
pub fn available_mime_types() -> Vec<String> {
    let Ok(out) = Command::new("wl-paste").args(["--list-types"]).output() else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    let s = String::from_utf8_lossy(&out.stdout);
    s.lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// One past clipboard entry, as stored in `~/.cache/mde/clipboard.json`
/// by the mesh-clipboard worker. Defined here so test fixtures don't
/// need to import the worker's types.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ClipEntry {
    /// Unix epoch ms when the entry was captured.
    pub captured_at_ms: u64,
    /// Mime type — text/plain is the only one mesh-replicated.
    pub mime: String,
    /// Body (text/plain) or a base64 blob for non-text mimes.
    pub body: String,
    /// Which peer originated the entry (None = local).
    pub origin_peer: Option<String>,
}

/// Pure helper — parse the clipboard.json file. Returns an
/// empty vec if the file's missing/malformed.
#[must_use]
pub fn parse_clipboard_history(json: &str) -> Vec<ClipEntry> {
    serde_json::from_str(json).unwrap_or_default()
}

/// Default location of the mesh-replicated clipboard history.
#[must_use]
pub fn default_history_path() -> std::path::PathBuf {
    dirs::cache_dir()
        .map(|d| d.join("mde/clipboard.json"))
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp/mde-clipboard.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_clipboard_history_handles_empty_string() {
        let out = parse_clipboard_history("");
        assert!(out.is_empty());
    }

    #[test]
    fn parse_clipboard_history_handles_malformed_json() {
        let out = parse_clipboard_history("{not json}");
        assert!(out.is_empty());
    }

    #[test]
    fn parse_clipboard_history_round_trips() {
        let entry = ClipEntry {
            captured_at_ms: 1_700_000_000_000,
            mime: "text/plain".into(),
            body: "hello".into(),
            origin_peer: None,
        };
        let json = serde_json::to_string(&vec![entry.clone()]).unwrap();
        let parsed = parse_clipboard_history(&json);
        assert_eq!(parsed, vec![entry]);
    }

    #[test]
    fn parse_clipboard_history_picks_up_peer_origin() {
        let json =
            r#"[{"captured_at_ms":1,"mime":"text/plain","body":"x","origin_peer":"lab-01"}]"#;
        let parsed = parse_clipboard_history(json);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].origin_peer, Some("lab-01".into()));
    }

    #[test]
    fn default_history_path_ends_with_clipboard_json() {
        let p = default_history_path();
        assert!(p.ends_with("clipboard.json"));
    }

    #[test]
    fn paste_text_does_not_panic_when_wl_paste_absent() {
        // Best-effort: on a Wayland-less host this just returns None.
        let _ = paste_text();
    }

    #[test]
    fn copy_text_does_not_panic_when_wl_copy_absent() {
        // Best-effort: just verify no panic on subprocess failure.
        let _ = copy_text("hello");
    }

    #[test]
    fn available_mime_types_returns_vec_even_on_failure() {
        let _ = available_mime_types();
    }
}
