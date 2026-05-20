//! Per-panel views — one module per group leaf. CB-1.x ports
//! land these incrementally; each module ships a state struct,
//! a `Message` variant set, an `update` reducer that returns
//! the parent app's `Message`, and a `view` builder over
//! [`Element<'_, crate::Message>`].

pub mod apps_install;
pub mod apps_installed;
pub mod apps_remove;
pub mod apps_sources;
pub mod datetime;
pub mod default_apps;
pub mod displays;
pub mod firewall;
pub mod fleet_revisions;
pub mod fleet_settings;
pub mod fonts;
pub mod inventory;
pub mod json_helpers;
pub mod logs;
pub mod mesh_history;
pub mod mesh_join;
pub mod notifications;
pub mod playbooks;
pub mod power;
pub mod printers;
pub mod removable;
pub mod repair;
pub mod resources;
pub mod run_history;
pub mod session;
pub mod snapshots;
pub mod sound;
pub mod system_update;
pub mod themes;
pub mod vpn;
pub mod wallpaper;
pub mod wifi;
pub mod window_manager;
