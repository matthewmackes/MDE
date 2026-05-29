//! Clipboard session — Wayland dispatch state and event loop (BUS-5.1/5.2).
//!
//! Connects to the compositor's wlr-data-control interface, watches for
//! clipboard changes, reads the clipboard content via a pipe, and publishes
//! it to the `clipboard/sync` Bus topic (BUS-5.2).

use std::io::Read as _;
use std::os::unix::io::{AsFd as _, OwnedFd};
use std::path::PathBuf;

use anyhow::Context as _;
use tracing::{debug, info, warn};
use wayland_client::{
    globals::{registry_queue_init, GlobalListContents},
    protocol::{wl_registry, wl_seat},
    Connection, Dispatch, QueueHandle,
};

use crate::proto::{
    self, WlSeat, ZwlrDataControlDeviceV1, ZwlrDataControlManagerV1, ZwlrDataControlOfferV1,
};

/// Configuration passed from `main` to [`run`].
pub struct Config {
    /// Root of the bus file tree (`~/.local/share/mde/bus/`).
    pub bus_root: PathBuf,
    /// XDG data home (`~/.local/share/`). Used to derive the blob directory.
    pub data_home: PathBuf,
    /// Hostname of this peer, used as `publisher_peer` in bus messages.
    pub peer_id: String,
}

/// Pending clipboard read: a pipe read-end + the MIME context for BUS-5.2
/// publish. Set in the Selection handler; consumed by the main event loop
/// after flushing the Wayland connection.
struct PendingPub {
    read_fd: OwnedFd,
    mimes: Vec<String>,
    selected_mime: String,
}

/// Full dispatch state for a single clipboard session.
pub struct AppState {
    /// MIME types accumulating for the in-flight offer (reset at each DataOffer).
    pending_mimes: Vec<String>,
    /// Offer proxy introduced by DataOffer — kept alive until Selection/PrimarySelection.
    pending_offer: Option<ZwlrDataControlOfferV1>,
    /// Last regular-clipboard offer; must be destroyed when a new Selection arrives.
    prev_selection: Option<ZwlrDataControlOfferV1>,
    /// Last primary-selection offer; must be destroyed when a new PrimarySelection arrives.
    ///
    /// NOTE: if a compositor shares one offer between Selection + PrimarySelection (rare),
    /// `prev_selection` and `prev_primary` both hold a reference to the same server-side
    /// object. The second `destroy()` call would be a protocol error; ObjectId tracking
    /// to guard this edge case is a BUS-5.x follow-on.
    prev_primary: Option<ZwlrDataControlOfferV1>,
    /// BUS-5.2: pipe read-end waiting to be read + published after the current
    /// dispatch cycle flushes the `receive()` request to the compositor.
    pending_pub: Option<PendingPub>,
    /// Total regular clipboard changes observed this session.
    pub selection_count: u64,
    /// BUS-5.3: count of successful publishes. GC runs every 50 publishes.
    publish_count: u64,
}

impl AppState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            pending_mimes: Vec::new(),
            pending_offer: None,
            prev_selection: None,
            prev_primary: None,
            pending_pub: None,
            selection_count: 0,
            publish_count: 0,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

// ── MIME selection ─────────────────────────────────────────────────────────

/// Pick the best MIME type to read for clipboard publish.
/// Prefers `text/plain` (smallest + most universally useful), then any
/// `text/*`, then the first non-internal MIME in the list.
fn pick_mime(mimes: &[String]) -> Option<String> {
    mimes
        .iter()
        .find(|m| m.starts_with("text/plain"))
        .or_else(|| mimes.iter().find(|m| m.starts_with("text/")))
        .or_else(|| {
            mimes
                .iter()
                .find(|m| !m.starts_with("x-kde-") && !m.starts_with('_'))
        })
        .cloned()
}

/// Create a pipe, send a `receive()` request to the compositor for the
/// selected MIME type, and return the read end of the pipe.
/// The write end is dropped after the request is enqueued — the compositor
/// holds a server-side dup and closes it after writing the data.
fn start_receive(offer: &ZwlrDataControlOfferV1, mime: &str) -> anyhow::Result<OwnedFd> {
    let (read_fd, write_fd) = rustix::pipe::pipe().context("create clipboard pipe")?;
    offer.receive(mime.to_string(), write_fd.as_fd());
    drop(write_fd);
    Ok(read_fd)
}

// ── Registry (dynamic global changes — observed but not acted upon) ────────────

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for AppState {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

// ── Seat (capabilities events — not needed for clipboard watching) ─────────────

impl Dispatch<WlSeat, ()> for AppState {
    fn event(
        _: &mut Self,
        _: &WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

// ── Data control manager (no events) ──────────────────────────────────────────

impl Dispatch<ZwlrDataControlManagerV1, ()> for AppState {
    fn event(
        _: &mut Self,
        _: &ZwlrDataControlManagerV1,
        _event: proto::zwlr_data_control_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // Manager has no events in the protocol spec.
    }
}

// ── Data control device ───────────────────────────────────────────────────────

impl Dispatch<ZwlrDataControlDeviceV1, ()> for AppState {
    fn event(
        state: &mut Self,
        _: &ZwlrDataControlDeviceV1,
        event: proto::zwlr_data_control_device_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use proto::zwlr_data_control_device_v1::Event;
        match event {
            Event::DataOffer { id } => {
                // New offer arriving. Reset MIME accumulation and keep the
                // proxy alive so Selection/PrimarySelection can reference it.
                state.pending_mimes.clear();
                state.pending_offer = Some(id);
                debug!("clipboard: data_offer — new offer introduced");
            }
            Event::Selection { id } => {
                match &id {
                    Some(_) => {
                        state.selection_count += 1;
                        info!(
                            count = state.selection_count,
                            mime_types = ?state.pending_mimes,
                            "clipboard: selection changed"
                        );
                        // BUS-5.2: request the clipboard content before the offer
                        // moves to prev_selection. The compositor writes to the
                        // pipe write end (now server-side) after we flush.
                        // BUS-5.5: if Super+Shift+C was pressed within the last
                        // 500 ms, skip publishing this copy (local-only).
                        if crate::skip_sync::should_skip_and_clear() {
                            debug!("clipboard: skip-sync flag set — local-only copy");
                        } else if let Some(offer) = &state.pending_offer {
                            if let Some(selected) = pick_mime(&state.pending_mimes) {
                                match start_receive(offer, &selected) {
                                    Ok(read_fd) => {
                                        state.pending_pub = Some(PendingPub {
                                            read_fd,
                                            mimes: state.pending_mimes.clone(),
                                            selected_mime: selected,
                                        });
                                    }
                                    Err(e) => {
                                        warn!(error = %e, "clipboard: start_receive failed — skipping publish");
                                    }
                                }
                            } else {
                                debug!("clipboard: no usable MIME type — skipping publish");
                            }
                        }
                    }
                    None => {
                        info!("clipboard: selection cleared");
                    }
                }
                // `id` is a duplicate reference to `pending_offer`. Drop it — we
                // use `pending_offer` as the canonical holder.
                drop(id);
                // Destroy the PREVIOUS cycle's offer (required by the protocol spec).
                if let Some(old) = state.prev_selection.take() {
                    old.destroy();
                }
                // Park the current offer as "previous" for the next Selection cycle.
                state.prev_selection = state.pending_offer.take();
            }
            Event::Finished => {
                // The seat was destroyed. The supervisor will restart us.
                warn!("clipboard: data-control device finished (seat gone) — exiting");
                std::process::exit(0);
            }
            Event::PrimarySelection { id } => {
                debug!("clipboard: primary selection changed");
                // Destroy the previous primary-selection offer.
                if let Some(old) = state.prev_primary.take() {
                    old.destroy();
                }
                // `id` is an independent reference even if the same server-side
                // object is referenced by prev_selection. Store it as prev_primary
                // and destroy it on the next PrimarySelection event.
                state.prev_primary = id;
            }
            _ => {}
        }
    }
}

// ── Data control offer (MIME type announcements) ──────────────────────────────

impl Dispatch<ZwlrDataControlOfferV1, ()> for AppState {
    fn event(
        state: &mut Self,
        _: &ZwlrDataControlOfferV1,
        event: proto::zwlr_data_control_offer_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let proto::zwlr_data_control_offer_v1::Event::Offer { mime_type } = event {
            debug!(?mime_type, "clipboard: offer mime type");
            state.pending_mimes.push(mime_type);
        }
    }
}

// ── Session entry point ───────────────────────────────────────────────────────

/// Connect to the Wayland display, discover the data-control globals, and
/// watch for clipboard events until the compositor disconnects or the daemon
/// is killed. Each selection change is read via a pipe and published to the
/// `clipboard/sync` bus topic (BUS-5.2). The mded `clipd_supervisor` worker
/// restarts this process on exit.
pub fn run(conn: &Connection, config: &Config) -> anyhow::Result<()> {
    let (globals, mut queue) = registry_queue_init::<AppState>(conn)
        .context("failed to initialise Wayland registry")?;
    let qh = queue.handle();
    let mut state = AppState::new();

    let seat: WlSeat = globals
        .bind(&qh, 1..=9, ())
        .context("compositor does not advertise wl_seat — is a seat attached?")?;

    let mgr: ZwlrDataControlManagerV1 = globals
        .bind(&qh, 1..=2, ())
        .context("compositor does not support zwlr_data_control_manager_v1 (sway ≥ 1.2 required)")?;

    // Subscribe to all clipboard events for this seat.
    let _device = mgr.get_data_device(&seat, &qh, ());

    // Flush our subscribe request and receive the initial selection event.
    queue
        .roundtrip(&mut state)
        .context("Wayland roundtrip failed")?;

    info!(
        initial_mimes = state.pending_mimes.len(),
        "mde-clipd: watching clipboard (BUS-5.2 publish + BUS-5.3 GC + BUS-5.4 subscriber enabled)"
    );

    // BUS-5.3: initial GC pass to clean up blobs from any previous run.
    if let Err(e) = crate::blobstore::gc_orphaned_blobs(&config.bus_root, &config.data_home) {
        warn!(error = %e, "clipboard: startup GC failed — continuing");
    }

    // BUS-5.4: launch the inbound subscriber thread.
    crate::subscribe::spawn_subscriber(config.bus_root.clone(), config.peer_id.clone());

    loop {
        queue
            .blocking_dispatch(&mut state)
            .context("Wayland dispatch error")?;

        // BUS-5.2: if the Selection handler queued a pipe receive, flush the
        // Wayland connection now so the compositor receives our receive() request,
        // then read the data and publish it to the bus.
        if let Some(pending) = state.pending_pub.take() {
            // Flush: compositor receives receive() request and writes to pipe.
            queue.flush().context("Wayland flush after receive()")?;

            // Read all clipboard data (blocks until compositor closes write end).
            let mut data = Vec::new();
            std::fs::File::from(pending.read_fd)
                .read_to_end(&mut data)
                .context("read clipboard pipe")?;

            if data.is_empty() {
                debug!(
                    mime = %pending.selected_mime,
                    "clipboard: compositor wrote no data — skipping publish"
                );
            } else {
                match crate::publish::publish_clipboard(
                    &config.bus_root,
                    &config.data_home,
                    &config.peer_id,
                    &pending.mimes,
                    &pending.selected_mime,
                    &data,
                ) {
                    Ok(()) => {
                        state.publish_count += 1;
                        // BUS-5.3: periodic GC every 50 publishes.
                        if state.publish_count % 50 == 0 {
                            if let Err(e) = crate::blobstore::gc_orphaned_blobs(
                                &config.bus_root,
                                &config.data_home,
                            ) {
                                warn!(error = %e, "clipboard: periodic GC failed");
                            }
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "clipboard: publish to bus failed");
                    }
                }
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_is_empty() {
        let s = AppState::new();
        assert_eq!(s.selection_count, 0);
        assert_eq!(s.publish_count, 0);
        assert!(s.pending_mimes.is_empty());
        assert!(s.pending_offer.is_none());
        assert!(s.prev_selection.is_none());
        assert!(s.prev_primary.is_none());
        assert!(s.pending_pub.is_none());
    }

    #[test]
    fn default_equals_new() {
        let s = AppState::default();
        assert_eq!(s.selection_count, 0);
        assert!(s.pending_mimes.is_empty());
    }

    #[test]
    fn pending_mimes_accumulate() {
        let mut s = AppState::new();
        s.pending_mimes.push("text/plain".to_string());
        s.pending_mimes.push("text/html".to_string());
        assert_eq!(s.pending_mimes.len(), 2);
        assert_eq!(s.pending_mimes[0], "text/plain");
    }

    #[test]
    fn pending_mimes_clear_simulates_data_offer() {
        let mut s = AppState::new();
        s.pending_mimes.push("text/plain".to_string());
        // Simulates the DataOffer handler clearing the slate.
        s.pending_mimes.clear();
        assert!(s.pending_mimes.is_empty());
    }

    #[test]
    fn selection_count_increments() {
        let mut s = AppState::new();
        s.selection_count += 1;
        s.selection_count += 1;
        assert_eq!(s.selection_count, 2);
    }

    #[test]
    fn selection_count_starts_at_zero() {
        let s = AppState::new();
        assert_eq!(s.selection_count, 0);
    }
}
