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
