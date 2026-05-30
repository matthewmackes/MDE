//! Shared library for the MDE installation manager (`mde-install`) and
//! updater (`mde-update`).
//!
//! INST-3 (v2.7). This is the **INST-3a** slice — the parts buildable
//! today with no upstream gaps:
//!
//! * [`profile`] — the three install profiles + parsing.
//! * [`confirm`] — TTY detection + typed-string confirms + the picker.
//! * [`intent_file`] — mesh-storage upgrade-intent barrier files (INST-10).
//! * [`wipe`] — local MDE-state wipe (config-path scope) + service
//!   control + the installed-profile marker.
//!
//! * [`peers`] — peer registry + version-skew (PEERVER-3, closes
//!   INST-3b). Reads the converged peer-data from the mesh-storage
//!   `<mesh-storage>/peers/` dir (no mackesd/D-Bus/Bus dependency),
//!   per `docs/design/v2.7-peer-data-convergence.md`.
//!
//! Deliberately **not** here yet:
//!
//! * Nebula cert-revoke + LizardFS data teardown (the re-install
//!   half of INST-7) — complete as of MESHFS-18.1 (revoke via
//!   `mackesd ca revoke` CLI; data dir wipe via [`wipe::wipe_meshfs_data`]).

pub mod confirm;
pub mod intent_file;
pub mod peers;
pub mod profile;
pub mod smoke;
pub mod wipe;
