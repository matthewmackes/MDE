//! MDE Files — mesh-first "Artifact Manager" for the Mackes Desktop Environment.
//!
//! Implementation contract: `docs/design/v2.0.0-mde-files/design-spec.md`.
//! Prototype: `docs/design/v2.0.0-mde-files/upstream-bundle/Artifact-Manager.html`.

pub mod app;
pub mod backend;
pub mod demo_data;
pub mod icons;
pub mod model;
pub mod panels;
pub mod search;
pub mod selection;
pub mod theme;
pub mod views;
pub mod widgets;

pub use app::{Message, MdeFiles};
pub use backend::{
    AuditEntry, Backend, BackendError, ConflictPolicy,
    DemoBackend, Destination, OpId, SendMode,
};
pub use model::{FileRow, Layout, Mime, Peer, PeerKind, PeerStatus, SelfNode, View};
