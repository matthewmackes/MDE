//! Mackes Desktop Environment (MDE) Workbench — Iced rewrite of
//! the v1.x GTK3 Python Workbench.
//!
//! **CB-1.1 scaffold** (`docs/PROJECT_WORKLIST.md`): exposes
//! [`App`], [`Message`], and [`View`] mirroring the v1.x sidebar
//! group structure; [`single_instance`] guards against duplicate
//! processes via the `dev.mackes.MDE.Workbench` bus name.
//!
//! **CB-1.2 nav layer** ports `_build_nav` and `_common.py` helpers
//! into a pure-Rust sidebar/breadcrumb model (see [`model::nav_model`]
//! + [`patternfly`]).
//!
//! The crate stays Iced-only at the public surface — the live
//! `Backend::DBus` impls (zbus calls into `dev.mackes.MDE.Shell.*`)
//! are routed through `mded` (CB-1.13) rather than open-coded here.

pub mod app;
pub mod dbus;
pub mod keyboard;
pub mod model;
pub mod patternfly;
pub mod sidebar;
pub mod single_instance;

pub use app::{App, Message};
pub use dbus::{INTERFACE_NAME, METHOD_FOCUS, PendingFocus, WorkbenchService};
pub use model::{Group, NavEntry, Panel, View, nav_model};
pub use single_instance::{
    BUS_NAME, OBJECT_PATH, PrimaryStatus, decide_primary_status,
};
