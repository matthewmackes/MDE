//! `mde-virtual` — the mesh-aware KVM + Podman compute manager (VIRT-13).
//!
//! A native Iced/Rust app that replaces the original Workbench Compute
//! panel + the Cockpit deep-link. It opens as a standard xdg-toplevel
//! window (not a layer-shell surface) with two tabs:
//!
//! - **Fleet** — every peer's compute, grouped into one collapsible
//!   section per peer, sourced from the `compute/inventory/*` Bus topic
//!   (published by each peer's `mded` `compute_registry` worker every
//!   10 s and mesh-synced as per-peer snapshot files).
//! - **Local** — this peer's compute only.
//!
//! VIRT-13.a (this binary) is the complete read-only viewer: window +
//! tabs + per-peer sections + per-resource rows + the "Mesh unavailable"
//! banner. VIRT-13.b layers on the Local tab's action buttons + the
//! Bus-independent direct libvirt-socket / `podman ps` fallback.
//!
//! Cite: visual-identity.md §1; ref: Apple System Settings.

#![forbid(unsafe_code)]

mod app;
mod sparkline;
mod wizard;

use clap::Parser;

/// CLI surface. `mde-virtual` takes no positional arguments today; the
/// derive gives us `--help` / `--version` for free + a stable place to
/// add `--tab` / `--peer` deep-links as later VIRT tasks land.
#[derive(Parser, Debug)]
#[command(
    name = "mde-virtual",
    about = "MDE Virtual — mesh-aware KVM + Podman compute manager",
    version
)]
struct Args {}

fn main() -> iced::Result {
    let _ = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mde_virtual=info".into()),
        )
        .init();

    tracing::info!("mde-virtual starting");

    iced::application(app::VirtualApp::new, app::VirtualApp::update, app::VirtualApp::view)
        .title("MDE Virtual")
        .subscription(app::VirtualApp::subscription)
        .theme(app::VirtualApp::theme)
        // `id` becomes the Wayland app_id / X11 WM_CLASS, matching the
        // `StartupWMClass=mde-virtual` key the VIRT-10 .desktop file sets
        // so the compositor associates the window with its launcher tile.
        .settings(iced::Settings {
            id: Some("mde-virtual".to_string()),
            ..Default::default()
        })
        // Standard xdg-toplevel window; 800x600 minimum per the spec.
        .window(iced::window::Settings {
            size: iced::Size::new(900.0, 640.0),
            min_size: Some(iced::Size::new(800.0, 600.0)),
            ..Default::default()
        })
        .run()
}

#[cfg(test)]
mod desktop_entry_tests {
    //! VIRT-10.a — validate the shipped
    //! `data/applications/mde-virtual.desktop` the way
    //! `desktop-file-validate` / `gio info` would: every entry line parses
    //! as `key=value`, and the launcher-critical keys carry the values the
    //! spec + the RPM `%files` packaging depend on. The file is embedded at
    //! compile time (`include_str!`) so the test needs no installed copy and
    //! no runtime path resolution; the relative path is repo-root anchored
    //! from `crates/mde-virtual/src/`.

    const DESKTOP: &str =
        include_str!("../../../data/applications/mde-virtual.desktop");

    /// Parse the `[Desktop Entry]` group into `(key, value)` pairs, asserting
    /// every non-comment, non-blank, non-group-header line is well-formed
    /// `key=value`. Panics (fails the test) on a malformed line, mirroring a
    /// validator's parse-error exit.
    fn parse_entry(src: &str) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        let mut in_entry = false;
        for line in src.lines() {
            let line = line.trim_end();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                in_entry = line == "[Desktop Entry]";
                continue;
            }
            if !in_entry {
                continue;
            }
            let (k, v) = line.split_once('=').unwrap_or_else(|| {
                panic!("malformed .desktop line (no '='): {line:?}")
            });
            pairs.push((k.to_string(), v.to_string()));
        }
        pairs
    }

    fn value<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
        pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    #[test]
    fn desktop_entry_parses_and_has_required_keys() {
        let pairs = parse_entry(DESKTOP);
        assert!(!pairs.is_empty(), "no [Desktop Entry] keys parsed");

        // The first meaningful line must open the canonical group.
        let first_meaningful = DESKTOP
            .lines()
            .map(str::trim_end)
            .find(|l| !l.is_empty() && !l.starts_with('#'))
            .expect("file has at least one meaningful line");
        assert_eq!(first_meaningful, "[Desktop Entry]");

        assert_eq!(value(&pairs, "Type"), Some("Application"));
        assert_eq!(value(&pairs, "Name"), Some("Virtual"));
        assert_eq!(value(&pairs, "Icon"), Some("computer"));
        assert_eq!(value(&pairs, "Terminal"), Some("false"));

        let categories =
            value(&pairs, "Categories").expect("Categories key present");
        assert!(
            categories.split(';').any(|c| c == "System"),
            "Categories must include System: {categories:?}"
        );
    }

    #[test]
    fn desktop_exec_matches_binary_and_wmclass_matches_app_id() {
        let pairs = parse_entry(DESKTOP);
        // The launcher contract: `Exec` spawns the same binary name the RPM
        // installs to `%{_bindir}`, and `StartupWMClass` equals it so the
        // compositor maps the window to its launcher (it also equals the
        // `iced::Settings.id` set in `main`).
        assert_eq!(value(&pairs, "Exec"), Some("mde-virtual"));
        assert_eq!(value(&pairs, "StartupWMClass"), Some("mde-virtual"));
    }
}
