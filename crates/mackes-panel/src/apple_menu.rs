// Apple-menu API consumed by Phase 3.3 (the dropdown chrome).
#![allow(dead_code)]

//! Apple-menu category bucketing.
//!
//! The Apple menu's `Applications →` submenu groups installed `.desktop`
//! entries by their `Categories` field. Per Q24 / Q25 of the design lock
//! the menu is the ONLY browse path for non-pinned apps, so the grouping
//! has to cover every common freedesktop category cleanly.
//!
//! Phase 3.2 ships the pure-data layer — a function that takes a slice
//! of `DesktopEntry` and returns a `Vec<Category>` ready for Phase 3.3
//! to render as nested `gtk::Menu` items.

use crate::desktop_files::DesktopEntry;

/// One submenu in the `Applications →` fanout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    /// Display label.
    pub label: &'static str,
    /// Mackes-Carbon icon name used for the submenu glyph.
    pub icon: &'static str,
    /// Entries sorted by `name` (case-insensitive). Each `.desktop`
    /// lands in the first category it qualifies for.
    pub entries: Vec<DesktopEntry>,
}

/// Canonical freedesktop main categories we surface, in render order.
///
/// `(label, icon, tags)` — `tags` are matched case-insensitively against
/// `Categories=` entries. The first tag that hits decides the bucket.
const CATEGORY_RULES: &[(&str, &str, &[&str])] = &[
    (
        "Internet",
        "applications-internet-symbolic",
        &["Network", "WebBrowser", "Email"],
    ),
    (
        "Multimedia",
        "applications-multimedia-symbolic",
        &["AudioVideo", "Audio", "Video", "Player"],
    ),
    (
        "Graphics",
        "applications-graphics-symbolic",
        &[
            "Graphics",
            "Photography",
            "RasterGraphics",
            "VectorGraphics",
        ],
    ),
    (
        "Office",
        "applications-office-symbolic",
        &["Office", "TextEditor", "Spreadsheet", "Presentation"],
    ),
    (
        "Development",
        "applications-development-symbolic",
        &["Development", "IDE", "Building", "Debugger"],
    ),
    ("Games", "applications-games-symbolic", &["Game"]),
    (
        "System",
        "applications-system-symbolic",
        &["System", "Settings", "Monitor", "Security"],
    ),
    (
        "Utilities",
        "applications-utilities-symbolic",
        &["Utility", "Accessibility", "TextTools", "FileTools"],
    ),
];

/// Fallback bucket for entries whose `Categories` don't match any rule.
const OTHER: (&str, &str) = ("Other", "applications-other-symbolic");

/// Group every `DesktopEntry` into categories. Returns the rule order
/// in `CATEGORY_RULES`, plus an `Other` bucket at the end if anything
/// fell through. Empty categories are dropped from the result.
#[must_use]
pub fn build(entries: &[DesktopEntry]) -> Vec<Category> {
    let mut buckets: Vec<Vec<DesktopEntry>> = CATEGORY_RULES.iter().map(|_| Vec::new()).collect();
    let mut other: Vec<DesktopEntry> = Vec::new();

    for entry in entries {
        match classify(&entry.categories) {
            Some(idx) => buckets[idx].push(entry.clone()),
            None => other.push(entry.clone()),
        }
    }

    let mut out: Vec<Category> = CATEGORY_RULES
        .iter()
        .zip(buckets)
        .filter(|(_, b)| !b.is_empty())
        .map(|((label, icon, _), mut bucket)| {
            bucket.sort_by_key(|e| e.name.to_lowercase());
            Category {
                label,
                icon,
                entries: bucket,
            }
        })
        .collect();

    if !other.is_empty() {
        other.sort_by_key(|e| e.name.to_lowercase());
        out.push(Category {
            label: OTHER.0,
            icon: OTHER.1,
            entries: other,
        });
    }

    out
}

fn classify(categories: &[String]) -> Option<usize> {
    for cat in categories {
        let lc = cat.to_ascii_lowercase();
        for (idx, (_, _, tags)) in CATEGORY_RULES.iter().enumerate() {
            if tags.iter().any(|t| t.eq_ignore_ascii_case(&lc)) {
                return Some(idx);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, categories: &[&str]) -> DesktopEntry {
        DesktopEntry {
            id: format!("{name}.desktop"),
            name: name.to_owned(),
            exec: name.to_lowercase(),
            icon: None,
            categories: categories.iter().map(|s| (*s).to_owned()).collect(),
            terminal: false,
        }
    }

    #[test]
    fn entries_bucket_by_first_matching_category() {
        let v = vec![
            entry("Firefox", &["Network", "WebBrowser"]),
            entry("Krita", &["Graphics"]),
            entry("htop", &["System", "Monitor"]),
        ];
        let cats = build(&v);
        let by_label: std::collections::HashMap<_, _> =
            cats.iter().map(|c| (c.label, c.entries.len())).collect();
        assert_eq!(by_label.get("Internet"), Some(&1));
        assert_eq!(by_label.get("Graphics"), Some(&1));
        assert_eq!(by_label.get("System"), Some(&1));
    }

    #[test]
    fn unclassified_entries_land_in_other() {
        let v = vec![entry("Weirdo", &["NotARealCategory"])];
        let cats = build(&v);
        assert_eq!(cats.len(), 1);
        assert_eq!(cats[0].label, "Other");
        assert_eq!(cats[0].entries[0].name, "Weirdo");
    }

    #[test]
    fn empty_input_yields_empty_categories() {
        let cats = build(&[]);
        assert!(cats.is_empty());
    }

    #[test]
    fn categories_dedupe_by_first_match_order() {
        // "WebBrowser" hits Internet (idx 0) — "Audio" doesn't get a
        // second chance even though it'd hit Multimedia.
        let v = vec![entry("Browser", &["WebBrowser", "Audio"])];
        let cats = build(&v);
        assert_eq!(cats.len(), 1);
        assert_eq!(cats[0].label, "Internet");
    }

    #[test]
    fn entries_sorted_case_insensitive() {
        let v = vec![
            entry("zoom", &["Network"]),
            entry("Aria", &["Network"]),
            entry("blender", &["Network"]),
        ];
        let cats = build(&v);
        let names: Vec<&str> = cats[0].entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["Aria", "blender", "zoom"]);
    }
}
