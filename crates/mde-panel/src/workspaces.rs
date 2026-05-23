//! v4.0.1 WM-1 — Workspace switcher.
//!
//! Pure-fn parser over `swaymsg -t get_workspaces` JSON + render
//! helper for the panel's workspace chip row. Lives on the panel
//! crate (not as a separate applet binary) so the existing Tick
//! subscription can drive the 2 s poll without needing an
//! applet-host child process.
//!
//! Sway's `get_workspaces` reply shape (per `man 7 sway-ipc`):
//!
//! ```json
//! [
//!   {"num": 1, "name": "1", "focused": true, "visible": true,
//!    "urgent": false, "representation": "H[firefox foot]"},
//!   {"num": 2, "name": "2", "focused": false, ...}
//! ]
//! ```
//!
//! The interesting fields for the chip row are `num` (display
//! label), `focused` (paint the chip in Q2 indigo), and the
//! `representation` non-empty signal (has-windows indicator dot).

use serde::Deserialize;

/// One workspace's parsed state — what the chip row renders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceState {
    /// 1-indexed workspace number. sway emits these in
    /// `name`/`num` form; we surface the integer for the
    /// chip-renderer.
    pub num: i32,
    /// True when this workspace owns the keyboard focus on
    /// at least one of its outputs.
    pub focused: bool,
    /// True when the workspace has ≥1 toplevel window —
    /// drives the indicator dot.
    pub has_windows: bool,
}

#[derive(Debug, Deserialize)]
struct RawWorkspace {
    #[serde(default)]
    num: i32,
    #[serde(default)]
    focused: bool,
    #[serde(default)]
    representation: String,
}

/// Parse a `swaymsg -t get_workspaces` JSON payload. Empty input
/// (or sway not running) yields an empty Vec — the panel renders
/// no chips in that case, matching the BUG-11 "missing process =
/// invisible widget" pattern.
#[must_use]
pub fn parse_workspaces(raw: &str) -> Vec<WorkspaceState> {
    let Ok(workspaces) = serde_json::from_str::<Vec<RawWorkspace>>(raw) else {
        return Vec::new();
    };
    workspaces
        .into_iter()
        .filter(|w| w.num >= 1) // ignore scratch (-1) + zero
        .map(|w| WorkspaceState {
            num: w.num,
            focused: w.focused,
            has_windows: !w.representation.is_empty()
                && w.representation != "[]"
                && !is_empty_representation(&w.representation),
        })
        .collect()
}

/// sway's `representation` field carries the workspace's container
/// tree — `H[]` / `V[]` / `T[]` / `S[]` with bracket contents
/// listing each app_id. A wholly-empty workspace has
/// `representation = "H[]"` (or just empty); a populated one
/// has names inside the brackets. We classify by stripping the
/// bracket pair + checking for non-whitespace content.
fn is_empty_representation(rep: &str) -> bool {
    let trimmed = rep.trim();
    if trimmed.is_empty() {
        return true;
    }
    if let Some(inner) = trimmed
        .strip_prefix('H')
        .or_else(|| trimmed.strip_prefix('V'))
        .or_else(|| trimmed.strip_prefix('T'))
        .or_else(|| trimmed.strip_prefix('S'))
    {
        let payload = inner.trim_start_matches('[').trim_end_matches(']');
        return payload.trim().is_empty();
    }
    false
}

/// Fold the workspace list into the fixed 4-slot row the panel
/// renders. Any workspace number outside 1..=4 is collapsed onto
/// the nearest in-range chip (or dropped if no chip exists).
/// MDE locks 4 persistent workspaces per
/// [[project_v1_1_0_win10_layout]], so this is the right shape.
#[must_use]
pub fn fixed_four_slots(workspaces: &[WorkspaceState]) -> [Option<WorkspaceState>; 4] {
    let mut slots: [Option<WorkspaceState>; 4] = [None, None, None, None];
    for w in workspaces {
        if (1..=4).contains(&w.num) {
            slots[(w.num - 1) as usize] = Some(w.clone());
        }
    }
    slots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_json_yields_empty_vec() {
        assert!(parse_workspaces("").is_empty());
        assert!(parse_workspaces("[]").is_empty());
        assert!(parse_workspaces("garbage").is_empty());
    }

    #[test]
    fn parse_extracts_focused_and_num() {
        let raw = r#"[
            {"num": 1, "focused": true,  "representation": "H[firefox foot]"},
            {"num": 2, "focused": false, "representation": ""}
        ]"#;
        let parsed = parse_workspaces(raw);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].num, 1);
        assert!(parsed[0].focused);
        assert!(parsed[0].has_windows);
        assert_eq!(parsed[1].num, 2);
        assert!(!parsed[1].focused);
        assert!(!parsed[1].has_windows);
    }

    #[test]
    fn parse_skips_scratch_workspace() {
        // sway exposes the scratchpad as a workspace with num=-1;
        // the chip row only shows positive-num workspaces.
        let raw = r#"[
            {"num": -1, "focused": false, "representation": ""},
            {"num":  1, "focused": true,  "representation": "H[foot]"}
        ]"#;
        let parsed = parse_workspaces(raw);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].num, 1);
    }

    #[test]
    fn is_empty_representation_handles_bracket_pair() {
        assert!(is_empty_representation(""));
        assert!(is_empty_representation("H[]"));
        assert!(is_empty_representation("V[ ]"));
        assert!(!is_empty_representation("H[firefox]"));
        assert!(!is_empty_representation("V[foot kitty]"));
    }

    #[test]
    fn fixed_four_slots_indexes_by_num() {
        let workspaces = vec![
            WorkspaceState { num: 1, focused: true, has_windows: true },
            WorkspaceState { num: 3, focused: false, has_windows: false },
        ];
        let slots = fixed_four_slots(&workspaces);
        assert!(slots[0].is_some()); // ws 1
        assert!(slots[1].is_none()); // ws 2
        assert!(slots[2].is_some()); // ws 3
        assert!(slots[3].is_none()); // ws 4
        assert_eq!(slots[0].as_ref().unwrap().num, 1);
        assert!(slots[0].as_ref().unwrap().focused);
    }

    #[test]
    fn fixed_four_slots_drops_out_of_range() {
        let workspaces = vec![
            WorkspaceState { num: 7, focused: false, has_windows: false },
            WorkspaceState { num: 2, focused: true, has_windows: true },
        ];
        let slots = fixed_four_slots(&workspaces);
        assert!(slots[0].is_none());
        assert_eq!(slots[1].as_ref().unwrap().num, 2);
        assert!(slots[2].is_none());
        assert!(slots[3].is_none());
    }
}
