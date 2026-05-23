//! v4.0.1 BUG-13 — Carbon icon bytes baked into the panel binary.
//!
//! Previously every chip / button on the panel rendered Unicode
//! fallback glyphs (`◯`, `≡`, `−`, `□`, `×`, `📋`, `○`, "M") because
//! the Iced consumer side never wired real Carbon SVG bytes. The
//! semantic surface in `mde_theme::Icon` returns the symbolic name
//! + a Unicode fallback today, with the SVG swap explicitly
//! deferred to UX-8.a. This module is the v4.0.1 partial close —
//! the 12 SVGs the panel directly draws are now baked here via
//! `include_bytes!`, ready to feed `iced::widget::svg::Handle`.
//!
//! Adding a new icon: drop the SVG into `assets/icons/carbon/<name>.svg`
//! (the convention is one freedesktop-ish name per file), add a
//! variant to [`PanelIcon`], extend [`PanelIcon::bytes`] with the
//! matching `include_bytes!`, and one new test in `tests` below
//! asserting the variant's bytes start with `<svg` so a future
//! editor swapping the file with a placeholder ZIP can't ship.

use iced::widget::svg;

/// Every Carbon SVG glyph the panel renders directly. Names are
/// the *semantic* role on the panel (e.g. `Start`, `Audio`,
/// `WindowClose`), not the Carbon glyph string. The mapping from
/// fdo icon names → Carbon glyph names lives in
/// [`crate::icon_mapper`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelIcon {
    /// Start-menu button (the "M" key on Win10).
    Start,
    /// Audio tray chip.
    Audio,
    /// Network tray chip.
    Network,
    /// Mesh-status tray chip.
    Mesh,
    /// Status-cluster tray chip (battery / system).
    Status,
    /// Clipboard tray button.
    Clipboard,
    /// Notification-bell tray button.
    Bell,
    /// Window-manage minimize button.
    WindowMinimize,
    /// Window-manage maximize / restore button.
    WindowMaximize,
    /// Window-manage close button.
    WindowClose,
    /// Files pinned start-menu tile.
    Files,
    /// Workbench pinned start-menu tile.
    Workbench,
    /// Desktop Layout — single (1 fullscreen). v4.0.1 BUG-16.
    LayoutSingle,
    /// Desktop Layout — vsplit (2 side-by-side). v4.0.1 BUG-16.
    LayoutVsplit,
    /// Desktop Layout — grid (2x2). v4.0.1 BUG-16.
    LayoutGrid,
    /// Desktop Layout — main + sidebar (60/40). v4.0.1 BUG-16.
    LayoutMainSidebar,
    /// Desktop Layout — tabbed. v4.0.1 BUG-16.
    LayoutTabbed,
    /// Workspace 1 chip. v4.0.1 WM-1.
    Workspace1,
    /// Workspace 2 chip. v4.0.1 WM-1.
    Workspace2,
    /// Workspace 3 chip. v4.0.1 WM-1.
    Workspace3,
    /// Workspace 4 chip. v4.0.1 WM-1.
    Workspace4,
    /// Tiny indicator dot next to a workspace chip when that
    /// workspace has at least one window. v4.0.1 WM-1.
    WorkspaceDot,
}

impl PanelIcon {
    /// Raw SVG bytes. Compiled in via `include_bytes!` so the panel
    /// works on systems where the Mackes-Carbon icon theme isn't
    /// installed (e.g. dev `cargo run` builds against a stock
    /// hicolor theme).
    #[must_use]
    pub fn bytes(self) -> &'static [u8] {
        match self {
            PanelIcon::Start => {
                include_bytes!("../../../assets/icons/carbon/start.svg")
            }
            PanelIcon::Audio => {
                include_bytes!("../../../assets/icons/carbon/audio.svg")
            }
            PanelIcon::Network => {
                include_bytes!("../../../assets/icons/carbon/network.svg")
            }
            PanelIcon::Mesh => {
                include_bytes!("../../../assets/icons/carbon/mesh.svg")
            }
            PanelIcon::Status => {
                include_bytes!("../../../assets/icons/carbon/status.svg")
            }
            PanelIcon::Clipboard => {
                include_bytes!("../../../assets/icons/carbon/clipboard.svg")
            }
            PanelIcon::Bell => {
                include_bytes!("../../../assets/icons/carbon/bell.svg")
            }
            PanelIcon::WindowMinimize => {
                include_bytes!("../../../assets/icons/carbon/minimize.svg")
            }
            PanelIcon::WindowMaximize => {
                include_bytes!("../../../assets/icons/carbon/maximize.svg")
            }
            PanelIcon::WindowClose => {
                include_bytes!("../../../assets/icons/carbon/close.svg")
            }
            PanelIcon::Files => {
                include_bytes!("../../../assets/icons/carbon/files.svg")
            }
            PanelIcon::Workbench => {
                include_bytes!("../../../assets/icons/carbon/workbench.svg")
            }
            PanelIcon::LayoutSingle => {
                include_bytes!("../../../assets/icons/carbon/layout-single.svg")
            }
            PanelIcon::LayoutVsplit => {
                include_bytes!("../../../assets/icons/carbon/layout-vsplit.svg")
            }
            PanelIcon::LayoutGrid => {
                include_bytes!("../../../assets/icons/carbon/layout-grid.svg")
            }
            PanelIcon::LayoutMainSidebar => {
                include_bytes!("../../../assets/icons/carbon/layout-main-sidebar.svg")
            }
            PanelIcon::LayoutTabbed => {
                include_bytes!("../../../assets/icons/carbon/layout-tabbed.svg")
            }
            PanelIcon::Workspace1 => {
                include_bytes!("../../../assets/icons/carbon/workspace-1.svg")
            }
            PanelIcon::Workspace2 => {
                include_bytes!("../../../assets/icons/carbon/workspace-2.svg")
            }
            PanelIcon::Workspace3 => {
                include_bytes!("../../../assets/icons/carbon/workspace-3.svg")
            }
            PanelIcon::Workspace4 => {
                include_bytes!("../../../assets/icons/carbon/workspace-4.svg")
            }
            PanelIcon::WorkspaceDot => {
                include_bytes!("../../../assets/icons/carbon/workspace-dot.svg")
            }
        }
    }

    /// Construct an `iced::widget::svg::Handle` from the baked
    /// bytes. The handle is cheap to clone — Iced reuses the
    /// inner image across redraws.
    #[must_use]
    pub fn handle(self) -> svg::Handle {
        svg::Handle::from_memory(self.bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every baked SVG must actually be an SVG payload — a build-
    /// time placeholder swap (e.g. someone replacing the file with
    /// a ZIP) gets caught here rather than at render time.
    #[test]
    fn every_panel_icon_starts_with_svg_header() {
        for icon in [
            PanelIcon::Start,
            PanelIcon::Audio,
            PanelIcon::Network,
            PanelIcon::Mesh,
            PanelIcon::Status,
            PanelIcon::Clipboard,
            PanelIcon::Bell,
            PanelIcon::WindowMinimize,
            PanelIcon::WindowMaximize,
            PanelIcon::WindowClose,
            PanelIcon::Files,
            PanelIcon::Workbench,
            PanelIcon::LayoutSingle,
            PanelIcon::LayoutVsplit,
            PanelIcon::LayoutGrid,
            PanelIcon::LayoutMainSidebar,
            PanelIcon::LayoutTabbed,
            PanelIcon::Workspace1,
            PanelIcon::Workspace2,
            PanelIcon::Workspace3,
            PanelIcon::Workspace4,
            PanelIcon::WorkspaceDot,
        ] {
            let bytes = icon.bytes();
            assert!(bytes.len() > 32, "{icon:?} bytes too small: {}", bytes.len());
            // Carbon SVGs sometimes lead with a doctype or
            // `<?xml ...?>` declaration before the `<svg`. Just
            // assert the substring appears somewhere in the first
            // 256 bytes.
            let header = std::str::from_utf8(&bytes[..bytes.len().min(256)])
                .unwrap_or("");
            assert!(header.contains("<svg"), "{icon:?} missing <svg tag");
        }
    }
}
