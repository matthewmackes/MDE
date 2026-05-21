//! Reusable layout primitives — the canonical data shapes every
//! Iced panel pulls from. UX-6 introduces this module so panel
//! authors have one place to look for "what does a polished
//! empty-state / card / status badge actually look like."
//!
//! `mde-theme` keeps the data forms (struct definitions, tier
//! constants). The Iced widget builders live in the
//! consumer-side `crates/mde-workbench/src/panel_chrome.rs` so
//! the toolkit dep doesn't leak into this crate.

pub mod empty_state;

pub use empty_state::{
    EmptyState, BODY_CTA_GAP, EMPTY_ICON_SIZE, HEADING_BODY_GAP, VERTICAL_PADDING,
};
