//! Per-panel views — one module per group leaf. CB-1.x ports
//! land these incrementally; each module ships a state struct,
//! a `Message` variant set, an `update` reducer that returns
//! the parent app's `Message`, and a `view` builder over
//! [`Element<'_, crate::Message>`].

pub mod displays;
pub mod fleet_revisions;
pub mod fleet_settings;
pub mod fonts;
pub mod inventory;
pub mod json_helpers;
pub mod notifications;
pub mod power;
pub mod printers;
pub mod removable;
pub mod session;
pub mod sound;
pub mod themes;
pub mod wallpaper;
