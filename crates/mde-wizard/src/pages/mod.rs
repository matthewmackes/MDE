//! Per-page modules for the wizard. Each page's `view` function
//! takes a borrowed `WizardState` and returns the body widget;
//! each page's `update` function takes the typed message + a
//! mutable state ref and returns the next page (or None if the
//! user stays on the current page).
//!
//! The pages are intentionally thin — they're data-form
//! collectors over a known state shape. The actual side effects
//! (mesh enrolment, snapshot creation, birthright apply) run
//! once the user reaches the Apply page; the pre-Apply pages
//! only mutate the in-memory `WizardState`.

pub mod apply;
pub mod legacy_import;
pub mod mesh_passcode;
pub mod network;
pub mod preset;
pub mod re_pair;
pub mod scan;
pub mod snapshot;
pub mod welcome;
