//! `mde-portal-full` — Portal-full scratchpad surface (Portal-16).
//!
//! A regular Iced window (not layer-shell) with XDG app_id
//! `"dev.mackes.MDE.Portal.Full"`.  Sway places it in the scratchpad
//! via a `for_window` rule; the Dock shows/hides it with
//! `swaymsg scratchpad show`.
//!
//! D-Bus interface `dev.mackes.MDE.Portal.Full` exposes `Goto(layer)`
//! so the Dock and external tools can switch the active content layer
//! (hub / library / control).
//!
//! Content layers (Portal-17..Portal-22) render as placeholder text
//! here; each is wired in its own task once the surface is live.

#![forbid(unsafe_code)]

use anyhow::Context as _;
use async_stream::stream;
use iced::widget::{column, container, text};
use iced::{Color, Element, Length, Subscription, Task, Theme};
use std::sync::OnceLock;
use tokio::sync::broadcast;

// ── D-Bus broadcast channel ───────────────────────────────────────────────────
//
// Initialized in `main()` before the Iced runtime starts so the
// subscription stream never blocks on a missing sender.

static DBUS_TX: OnceLock<broadcast::Sender<String>> = OnceLock::new();

fn dbus_sender() -> Option<&'static broadcast::Sender<String>> {
    DBUS_TX.get()
}

// ── D-Bus interface ────────────────────────────────────────────────────────────

mod dbus {
    use anyhow::Context as _;
    use super::dbus_sender;
    use zbus::{interface, Connection};

    struct PortalFullIface;

    #[interface(name = "dev.mackes.MDE.Portal.Full")]
    impl PortalFullIface {
        /// Switch to the named content layer (hub / library / control).
        async fn goto(&self, layer: String) -> zbus::fdo::Result<()> {
            tracing::info!(%layer, "Portal.Full.Goto");
            if let Some(tx) = dbus_sender() {
                let _ = tx.send(layer);
            }
            Ok(())
        }

        /// Smoke-test ping — returns `"pong"`.
        async fn ping(&self) -> zbus::fdo::Result<String> {
            Ok("pong".to_string())
        }
    }

    pub async fn register() -> anyhow::Result<Connection> {
        let conn = Connection::session()
            .await
            .context("connecting to session D-Bus")?;
        conn.object_server()
            .at("/dev/mackes/MDE/Portal/Full", PortalFullIface)
            .await
            .context("registering PortalFullIface")?;
        conn.request_name("dev.mackes.MDE.Portal.Full")
            .await
            .context("requesting dev.mackes.MDE.Portal.Full")?;
        tracing::info!("mde-portal-full: D-Bus registered");
        Ok(conn)
    }
}

// ── Content-layer enum ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Layer {
    #[default]
    Hub,
    Library,
    Control,
}

impl Layer {
    fn from_str(s: &str) -> Self {
        match s {
            "library" => Layer::Library,
            "control" => Layer::Control,
            _ => Layer::Hub,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Layer::Hub => "Hub",
            Layer::Library => "Library",
            Layer::Control => "Control",
        }
    }

    fn breadcrumb(self) -> String {
        format!("M › {}", self.label())
    }
}

// ── Application state ─────────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct PortalFull {
    layer: Layer,
}

// ── Messages ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Message {
    /// D-Bus `Goto` received — switch content layer.
    GotoLayer(Layer),
}

// ── Update ────────────────────────────────────────────────────────────────────

fn update(state: &mut PortalFull, msg: Message) -> Task<Message> {
    match msg {
        Message::GotoLayer(layer) => {
            tracing::info!(?layer, "portal-full: switching layer");
            state.layer = layer;
        }
    }
    Task::none()
}

// ── View ──────────────────────────────────────────────────────────────────────

/// Classic ChromeOS charcoal (#202124).
const CHARCOAL: Color = Color { r: 0.125, g: 0.129, b: 0.141, a: 1.0 };
const FG: Color = Color::WHITE;
const FG_DIM: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.4 };

fn view(state: &PortalFull) -> Element<'_, Message> {
    container(
        column![
            text(state.layer.breadcrumb()).size(22.0).color(FG),
            text(format!(
                "{} layer — wired in Portal-{}",
                state.layer.label(),
                match state.layer {
                    Layer::Hub => "17",
                    Layer::Library => "19",
                    Layer::Control => "20",
                }
            ))
            .size(13.0)
            .color(FG_DIM),
        ]
        .spacing(16),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .style(|_: &Theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(CHARCOAL)),
        ..Default::default()
    })
    .into()
}

// ── Subscription ──────────────────────────────────────────────────────────────

fn subscription(_state: &PortalFull) -> Subscription<Message> {
    Subscription::run_with_id("mde-portal-full-dbus", stream! {
        // The sender is set in main() before iced starts, but subscription
        // streams are spawned by iced's runtime potentially very quickly.
        // Poll briefly until the OnceLock is populated.
        let tx = loop {
            if let Some(tx) = DBUS_TX.get() {
                break tx;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        };
        let mut rx = tx.subscribe();
        loop {
            match rx.recv().await {
                Ok(layer_str) => yield Message::GotoLayer(Layer::from_str(&layer_str)),
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
            }
        }
    })
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("MDE_PORTAL_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("mde_portal=info,warn")),
        )
        .json()
        .init();

    // Initialize D-Bus → Iced channel before the Iced runtime starts so the
    // subscription stream always finds the sender in the OnceLock.
    let (tx, _rx) = broadcast::channel::<String>(32);
    DBUS_TX.set(tx).expect("DBUS_TX initialized once in main");

    // D-Bus registration runs in a dedicated multi-thread runtime so zbus
    // dispatch doesn't contend with the Iced render thread.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("building tokio runtime for D-Bus")?;
    let _conn = rt
        .block_on(dbus::register())
        .context("registering dev.mackes.MDE.Portal.Full")?;
    let _rt_thread = std::thread::spawn(move || rt.block_on(std::future::pending::<()>()));

    // Run the Portal-full Iced window.
    // - `decorations: false` removes the window border (sway draws none for scratchpad).
    // - `resizable: false` prevents manual resize; sway rules handle sizing.
    // - `application_id` must match sway's `for_window` rule.
    iced::application("M · Portal", update, view)
        .subscription(subscription)
        .theme(|_| Theme::Dark)
        .window(iced::window::Settings {
            size: iced::Size::new(1280.0, 720.0),
            platform_specific: iced::window::settings::PlatformSpecific {
                application_id: "dev.mackes.MDE.Portal.Full".to_string(),
                ..Default::default()
            },
            decorations: false,
            resizable: false,
            ..Default::default()
        })
        .run()
        .map_err(|e| anyhow::anyhow!("mde-portal-full: {e}"))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_from_str_hub_is_default() {
        assert_eq!(Layer::from_str("hub"), Layer::Hub);
        assert_eq!(Layer::from_str("unknown"), Layer::Hub);
        assert_eq!(Layer::from_str(""), Layer::Hub);
    }

    #[test]
    fn layer_from_str_library() {
        assert_eq!(Layer::from_str("library"), Layer::Library);
    }

    #[test]
    fn layer_from_str_control() {
        assert_eq!(Layer::from_str("control"), Layer::Control);
    }

    #[test]
    fn layer_breadcrumb_contains_m_prefix() {
        assert!(Layer::Hub.breadcrumb().starts_with("M › "));
        assert!(Layer::Library.breadcrumb().contains("Library"));
        assert!(Layer::Control.breadcrumb().contains("Control"));
    }

    #[test]
    fn layer_label_matches_expected() {
        assert_eq!(Layer::Hub.label(), "Hub");
        assert_eq!(Layer::Library.label(), "Library");
        assert_eq!(Layer::Control.label(), "Control");
    }

    #[test]
    fn portal_full_default_layer_is_hub() {
        let state = PortalFull::default();
        assert_eq!(state.layer, Layer::Hub);
    }

    #[test]
    fn update_goto_layer_changes_state() {
        let mut state = PortalFull::default();
        let _ = update(&mut state, Message::GotoLayer(Layer::Library));
        assert_eq!(state.layer, Layer::Library);

        let _ = update(&mut state, Message::GotoLayer(Layer::Control));
        assert_eq!(state.layer, Layer::Control);

        let _ = update(&mut state, Message::GotoLayer(Layer::Hub));
        assert_eq!(state.layer, Layer::Hub);
    }

    #[test]
    fn charcoal_is_chromeos_lock() {
        let r = (CHARCOAL.r * 255.0).round() as u8;
        let g = (CHARCOAL.g * 255.0).round() as u8;
        let b = (CHARCOAL.b * 255.0).round() as u8;
        assert_eq!((r, g, b), (32, 33, 36), "#202124 charcoal");
    }
}
