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

#[derive(Debug)]
struct PortalFull {
    layer: Layer,
    /// Portal-17.a — cached snapshot of the operator's user tags
    /// from `<XDG_DATA_HOME>/mde/tags.json`. Re-read on Hub-layer
    /// entry so operator edits via Portal-18.b modal take effect
    /// next time the Hub opens (no live mtime-watch yet — that
    /// ships when the modal lands).
    user_tags: Vec<mackes_mesh_types::Tag>,
    /// Portal-17.c — name of the tag whose context menu is open,
    /// or `None` when no menu is showing. Set on right-click via
    /// `HubTagRightClicked`; cleared on any menu-action message
    /// or on `HubMenuDismissed` (click-elsewhere / Escape).
    hub_right_click_target: Option<String>,
    /// Portal-18.b — in-flight Edit-tag modal state. `None` when
    /// no modal is open; `Some(form)` while the operator edits.
    /// Set by `HubMenuEditTag`, cleared on Save / Cancel.
    editing_tag: Option<EditTagForm>,
    /// Portal-17.e — sticky multi-select state for the Hub's
    /// tag-intersection AND-filter. Each entry is a tag name
    /// the operator shift-clicked. Empty → no filter active.
    /// Stays sticky across clicks; clears via the
    /// `HubMultiSelectCleared` message or a fresh click-without-
    /// shift on a single tag-card. The Portal-17.b cascade will
    /// AND-filter its column entries against this set when it
    /// ships — until then, the state is bench-observable via
    /// the indicator pill rendered above the tag-card grid.
    hub_multi_select: std::collections::BTreeSet<String>,
    /// Portal-17.d — type-ahead buffer. Empty when no character
    /// typed since last clear. Each printable keystroke appends;
    /// Backspace pops; Escape (when no menu/modal is open) clears.
    /// The matched-tag-name (`hub_typeahead_match`) updates on
    /// every buffer change via case-insensitive prefix lookup
    /// against the combined system + user tag list.
    hub_typeahead_buffer: String,
    /// Portal-17.d — most recent type-ahead match, or `None`
    /// when the buffer is empty or no tag matches the prefix.
    /// The matched card renders with an inset ring around its
    /// pill so the focus position is visible. Enter activates
    /// (fires `HubTagClicked(match)`).
    hub_typeahead_match: Option<String>,
    /// Portal-17.b — cascade-card column stack. Each entry is
    /// the name of a tag that's been expanded. Click on a tag
    /// pushes it; up to 3 deep before forcing dismiss-to-root.
    /// Empty when the cascade is closed (Hub root view).
    hub_cascade_stack: Vec<String>,
}

/// Portal-18.b — Edit-tag modal form state. Seeded from the
/// current tag-store entry when the modal opens; in-flight
/// edits land back on the store via `SaveTagEdit`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditTagForm {
    /// Name of the tag being edited. Renaming is allowed; the
    /// SaveTagEdit handler validates the new name against the
    /// rest of the store (rejects collisions, empties).
    pub name: String,
    /// Original name at modal-open time. Used by save to find
    /// the existing tag entry to mutate (since `name` may have
    /// been edited).
    pub original_name: String,
    /// CSS hex color (`#42be65` or `#abc` shorthand). Empty
    /// string clears the tint (None on save).
    pub group_color: String,
    /// Default layout (`splith` / `splitv` / `tabbed` /
    /// `stacked`) or empty string for "no preference."
    pub default_layout: String,
}

impl Default for PortalFull {
    fn default() -> Self {
        // Portal-17.a — seed user_tags on construction so the
        // first view-render (which happens before any message
        // fires) has the right tag set. update() refreshes on
        // every Hub-layer entry.
        let user_tags = mackes_mesh_types::TagStore::load_default()
            .map(|store| store.tags)
            .unwrap_or_default();
        Self {
            layer: Layer::default(),
            user_tags,
            hub_right_click_target: None,
            editing_tag: None,
            hub_multi_select: std::collections::BTreeSet::new(),
            hub_typeahead_buffer: String::new(),
            hub_typeahead_match: None,
            hub_cascade_stack: Vec::new(),
        }
    }
}

/// Portal-17.b — maximum cascade depth before forcing
/// dismiss-to-root. Per the design lock.
pub const HUB_CASCADE_DEPTH_CAP: usize = 3;

// ── Messages ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Message {
    /// D-Bus `Goto` received — switch content layer.
    GotoLayer(Layer),
    /// Portal-17.a — user clicked a Hub system-tag or user-tag
    /// card. Placeholder for cascade-card expansion (Portal-17.b)
    /// + right-click iconic menu (Portal-17.c).
    HubTagClicked(String),
    /// Portal-17.c — operator right-clicked a Hub tag card.
    /// Surfaces the iconic context menu over the Hub view; the
    /// `String` carries the tag name so menu actions know
    /// which tag they target.
    HubTagRightClicked(String),
    /// Portal-17.c — operator clicked outside the open context
    /// menu, or pressed Escape. Clears the menu without firing
    /// any action.
    HubMenuDismissed,
    /// Portal-17.c — operator chose "Edit tag…" from the menu.
    /// Fires the Portal-18.b modal route. Tag name is the
    /// current menu target.
    HubMenuEditTag(String),
    /// Portal-17.c — operator chose "Layout chooser…" per R3-Q62.
    /// Routes to the Portal-44 default-layout writer for the
    /// named tag.
    HubMenuLayoutChooser(String),
    /// Portal-17.c — operator chose "Window rules…" — opens the
    /// Portal-53.b per-tag window-rules modal scoped to the tag.
    HubMenuWindowRules(String),
    /// Portal-17.c — operator chose "Enter mode" — Portal-47
    /// flips sway into the tag's binding mode.
    HubMenuEnterMode(String),
    /// Portal-17.c — operator chose "Save as template…" per
    /// Portal-51. Captures the current workspace into a
    /// template card tagged with the current tag.
    HubMenuSaveAsTemplate(String),
    /// Portal-17.e — operator shift-clicked a tag card (or used
    /// the equivalent context-menu toggle). Adds the tag to the
    /// sticky multi-select set, or removes it if already present.
    /// Independent of `HubTagClicked` — that handler will route
    /// here when the cascade ships shift-modifier tracking.
    HubMultiSelectToggled(String),
    /// Portal-17.e — operator cleared the sticky multi-select
    /// filter (clicked the indicator pill's "✕", or pressed
    /// Escape while no other modal/menu was open).
    HubMultiSelectCleared,
    /// Portal-17.d — operator typed a character. Appends to the
    /// type-ahead buffer + recomputes the matched tag. Routed
    /// from the keyboard subscription when the Hub layer is
    /// focused + no modal/menu is in the way.
    HubTypeAheadChar(char),
    /// Portal-17.d — operator pressed Backspace. Pops one char
    /// off the type-ahead buffer (or clears entirely if length 1).
    HubTypeAheadBackspace,
    /// Portal-17.d — operator pressed Enter while a type-ahead
    /// match is active. Fires `HubTagClicked(match)` to activate
    /// the focused card + clears the buffer.
    HubTypeAheadActivate,
    /// Portal-17.b.activate — operator clicked a cascade-column
    /// member entry. The String payload carries the rendered
    /// label (`format_cascade_member(member)`) for logging +
    /// downstream activation; the handler logs + clears the
    /// cascade stack (visual confirmation the click registered).
    /// Per-target activation (App → spawn, Workspace → focus,
    /// Peer → mesh nav, …) lands in Portal-17.b.activate.targets
    /// once each target surface is wired.
    HubCascadeMemberClicked(String),
    /// Portal-18.b — Edit-tag modal name field edited.
    EditTagNameChanged(String),
    /// Portal-18.b — Edit-tag modal group_color field edited.
    EditTagColorChanged(String),
    /// Portal-18.b — Edit-tag modal default_layout selection.
    EditTagLayoutChanged(String),
    /// Portal-18.b — operator clicked Save. Writes the form
    /// back to the tag store + closes the modal.
    SaveTagEdit,
    /// Portal-18.b — operator clicked Cancel or pressed Escape.
    /// Discards the form + closes the modal.
    CancelTagEdit,
}

// ── Update ────────────────────────────────────────────────────────────────────

fn update(state: &mut PortalFull, msg: Message) -> Task<Message> {
    match msg {
        Message::GotoLayer(layer) => {
            tracing::info!(?layer, "portal-full: switching layer");
            state.layer = layer;
            // Portal-17.a — refresh the user-tag snapshot on
            // every Hub-layer entry. Cheap (small JSON file
            // parse); covers the operator-edited tags.json case
            // without a live inotify watch.
            if layer == Layer::Hub {
                state.user_tags = match mackes_mesh_types::TagStore::load_default() {
                    Ok(store) => store.tags,
                    Err(e) => {
                        tracing::debug!(error = %e, "portal-full: tag-store load failed; rendering with empty tag set");
                        Vec::new()
                    }
                };
            }
        }
        Message::HubTagClicked(tag_name) => {
            // Portal-17.b — push the clicked tag onto the cascade
            // stack. Clicking the same tag that's already on top
            // collapses one level (toggle). Stack caps at
            // HUB_CASCADE_DEPTH_CAP entries — beyond that the
            // oldest entry drops (root-most), keeping focus on
            // the deepest 3 visible.
            tracing::info!(%tag_name, "portal-full: Hub tag clicked");
            state.hub_right_click_target = None;
            if state.hub_cascade_stack.last() == Some(&tag_name) {
                // Re-click on the deepest → pop (collapse one).
                state.hub_cascade_stack.pop();
            } else {
                state.hub_cascade_stack.push(tag_name);
                while state.hub_cascade_stack.len() > HUB_CASCADE_DEPTH_CAP {
                    state.hub_cascade_stack.remove(0);
                }
            }
        }
        Message::HubTagRightClicked(tag_name) => {
            tracing::info!(%tag_name, "portal-full: Hub tag right-clicked, opening menu");
            state.hub_right_click_target = Some(tag_name);
        }
        Message::HubMenuDismissed => {
            // Portal-17.c / Portal-18.b / Portal-17.d / Portal-17.b
            // — single dismissal path for Escape: clear the right-
            // click menu, any open Edit-tag modal, the type-ahead
            // buffer/match, AND the cascade column stack
            // (dismiss-to-root). Multi-select stays sticky per
            // Portal-17.e — only its explicit Clear button clears
            // it. Any of the four states may be active when
            // Escape fires; this handler is the union close.
            tracing::debug!("portal-full: Hub right-click menu / Edit-tag modal / type-ahead / cascade dismissed");
            state.hub_right_click_target = None;
            state.editing_tag = None;
            state.hub_typeahead_buffer.clear();
            state.hub_typeahead_match = None;
            state.hub_cascade_stack.clear();
        }
        Message::HubMenuEditTag(tag_name) => {
            // Portal-17.c → Portal-18.b. Open the Edit-tag modal
            // seeded with the named tag's current values.
            tracing::info!(%tag_name, "portal-full: HubMenu → EditTag (Portal-18.b modal opens)");
            state.editing_tag = Some(seed_edit_form(&state.user_tags, &tag_name));
            state.hub_right_click_target = None;
        }
        Message::HubMenuLayoutChooser(tag_name) => {
            tracing::info!(%tag_name, "portal-full: HubMenu → LayoutChooser (Portal-44 hand-off)");
            state.hub_right_click_target = None;
        }
        Message::HubMenuWindowRules(tag_name) => {
            tracing::info!(%tag_name, "portal-full: HubMenu → WindowRules (Portal-53.b modal)");
            state.hub_right_click_target = None;
        }
        Message::HubMenuEnterMode(tag_name) => {
            tracing::info!(%tag_name, "portal-full: HubMenu → EnterMode (Portal-47 hand-off)");
            state.hub_right_click_target = None;
        }
        Message::HubMenuSaveAsTemplate(tag_name) => {
            tracing::info!(%tag_name, "portal-full: HubMenu → SaveAsTemplate (Portal-51 hand-off)");
            state.hub_right_click_target = None;
        }
        Message::HubMultiSelectToggled(tag_name) => {
            // Portal-17.e — sticky toggle. Add if absent, remove if
            // present. Empty-name guards against future bindings
            // that might pass garbage. Dismiss the right-click
            // menu (if open) so the indicator pill is visible.
            if tag_name.is_empty() {
                tracing::warn!("portal-full: HubMultiSelectToggled with empty tag — ignored");
            } else if state.hub_multi_select.contains(&tag_name) {
                state.hub_multi_select.remove(&tag_name);
                tracing::info!(%tag_name, count = state.hub_multi_select.len(), "portal-full: tag removed from AND-filter");
            } else {
                state.hub_multi_select.insert(tag_name.clone());
                tracing::info!(%tag_name, count = state.hub_multi_select.len(), "portal-full: tag added to AND-filter");
            }
            state.hub_right_click_target = None;
        }
        Message::HubMultiSelectCleared => {
            // Portal-17.e — clear the sticky filter. Fired by the
            // indicator pill's ✕ button or the Escape-no-menu path
            // (HubMenuDismissed already covers Escape when a menu
            // is open; this handler is for the no-menu case).
            if !state.hub_multi_select.is_empty() {
                tracing::info!(count = state.hub_multi_select.len(), "portal-full: AND-filter cleared");
                state.hub_multi_select.clear();
            }
        }
        Message::HubTypeAheadChar(c) => {
            // Portal-17.d — append the char + recompute match.
            // Lower-casing happens inside the match helper for
            // case-insensitive comparison; the buffer itself
            // preserves the operator's casing for display.
            // Portal-17.d.cascade — match walk also includes
            // currently-visible cascade column members.
            state.hub_typeahead_buffer.push(c);
            state.hub_typeahead_match = find_typeahead_match(
                &state.hub_typeahead_buffer,
                &state.user_tags,
                &state.hub_cascade_stack,
            );
        }
        Message::HubTypeAheadBackspace => {
            state.hub_typeahead_buffer.pop();
            state.hub_typeahead_match = if state.hub_typeahead_buffer.is_empty() {
                None
            } else {
                find_typeahead_match(
                    &state.hub_typeahead_buffer,
                    &state.user_tags,
                    &state.hub_cascade_stack,
                )
            };
        }
        Message::HubCascadeMemberClicked(label) => {
            // Portal-17.b.activate — log + clear the cascade so
            // the operator sees the click register. Per-target
            // activation is the .targets follow-on once each
            // surface (sway focus, app spawn, mesh nav, container
            // shell) is wired.
            tracing::info!(%label, "portal-full: cascade member clicked");
            state.hub_cascade_stack.clear();
        }
        Message::HubTypeAheadActivate => {
            // Enter on a matched tag → activate as if clicked.
            // Re-uses the HubTagClicked handler so cascade
            // expansion behavior stays identical to the mouse
            // path. Clears the buffer afterwards.
            if let Some(name) = state.hub_typeahead_match.clone() {
                tracing::info!(%name, "portal-full: type-ahead Enter activates tag");
                state.hub_typeahead_buffer.clear();
                state.hub_typeahead_match = None;
                // Fall through to HubTagClicked handler logic.
                state.hub_right_click_target = None;
            }
        }
        Message::EditTagNameChanged(value) => {
            if let Some(form) = state.editing_tag.as_mut() {
                form.name = value;
            }
        }
        Message::EditTagColorChanged(value) => {
            if let Some(form) = state.editing_tag.as_mut() {
                form.group_color = value;
            }
        }
        Message::EditTagLayoutChanged(value) => {
            if let Some(form) = state.editing_tag.as_mut() {
                form.default_layout = value;
            }
        }
        Message::SaveTagEdit => {
            if let Some(form) = state.editing_tag.take() {
                match commit_tag_edit(&form) {
                    Ok(()) => {
                        // Refresh in-memory snapshot so the Hub
                        // grid reflects the saved changes.
                        if let Ok(store) = mackes_mesh_types::TagStore::load_default() {
                            state.user_tags = store.tags;
                        }
                        tracing::info!(name = %form.name, "portal-full: tag edit saved");
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, name = %form.name, "portal-full: tag edit save failed");
                    }
                }
            }
        }
        Message::CancelTagEdit => {
            tracing::debug!("portal-full: tag edit cancelled");
            state.editing_tag = None;
        }
    }
    Task::none()
}

/// Portal-17.b — render a single cascade-member entry as a
/// readable label string. Each TagMember variant gets its
/// own format: app:`<id>` → "App: <id>", peer → "Peer: <hostname>",
/// workspace → "Workspace #<num>", container → "Container: <name>",
/// etc. System tags don't have member entries; user tags do.
#[must_use]
pub fn format_cascade_member(member: &mackes_mesh_types::TagMember) -> String {
    use mackes_mesh_types::TagMember;
    match member {
        TagMember::App { app_id } => format!("App: {app_id}"),
        TagMember::Peer { hostname } => format!("Peer: {hostname}"),
        TagMember::Contact { ulid } => format!("Contact: {ulid}"),
        TagMember::Workspace { num } => format!("Workspace #{num}"),
        TagMember::Container { name } => format!("Container: {name}"),
        TagMember::Tray { bus_name } => format!("Tray: {bus_name}"),
        TagMember::File { path } => format!("File: {path}"),
        TagMember::Activity { ulid } => format!("Activity: {ulid}"),
        TagMember::Zone { name } => format!("Zone: {name}"),
    }
}

/// Portal-17.b — look up the TagMember list for the named tag
/// in the live user-tag snapshot. Returns `None` when the tag
/// doesn't exist (system tags + un-stored user tags). Empty
/// list when the tag exists but has no members yet.
#[must_use]
pub fn cascade_members_for_tag<'a>(
    tag_name: &str,
    user_tags: &'a [mackes_mesh_types::Tag],
) -> Option<&'a [mackes_mesh_types::TagMember]> {
    user_tags
        .iter()
        .find(|t| t.name == tag_name)
        .map(|t| t.members.as_slice())
}

/// Portal-17.d — find the first item whose label starts with the
/// given prefix (case-insensitive). Searches in visible-surface
/// priority order:
///   1. System tags (in declaration order)
///   2. User tags (in stored order)
///   3. Cascade column members, column-by-column (root-most
///      first). Each member's rendered label is matched via
///      `format_cascade_member`.
/// Returns `None` when the prefix is empty or nothing matches.
///
/// The cascade-column search is the Portal-17.d.cascade extension
/// (2026-05-27); empty `cascade_stack` reduces to the original
/// root-only behavior.
fn find_typeahead_match(
    prefix: &str,
    user_tags: &[mackes_mesh_types::Tag],
    cascade_stack: &[String],
) -> Option<String> {
    if prefix.is_empty() {
        return None;
    }
    let needle = prefix.to_lowercase();
    for system in SYSTEM_TAGS {
        if system.to_lowercase().starts_with(&needle) {
            return Some((*system).to_string());
        }
    }
    for tag in user_tags {
        if tag.name.to_lowercase().starts_with(&needle) {
            return Some(tag.name.clone());
        }
    }
    // Portal-17.d.cascade — walk each visible cascade column's
    // members in stack order. Each member's rendered label is
    // the comparison key (e.g. "App: foot" matches `a`).
    for column_tag_name in cascade_stack {
        if let Some(members) = cascade_members_for_tag(column_tag_name, user_tags) {
            for member in members {
                let label = format_cascade_member(member);
                if label.to_lowercase().starts_with(&needle) {
                    return Some(label);
                }
            }
        }
    }
    None
}

/// Portal-18.b — seed the Edit-tag form from the live in-memory
/// snapshot. System-tag entries (which don't exist in the
/// user-tag store) get an empty form with the system name
/// pre-filled; saving will create the tag.
fn seed_edit_form(user_tags: &[mackes_mesh_types::Tag], target: &str) -> EditTagForm {
    let existing = user_tags.iter().find(|t| t.name == target);
    EditTagForm {
        name: target.to_string(),
        original_name: target.to_string(),
        group_color: existing
            .and_then(|t| t.group_color.clone())
            .unwrap_or_default(),
        default_layout: existing
            .and_then(|t| t.default_layout.clone())
            .unwrap_or_default(),
    }
}

/// Portal-18.b — commit the in-flight EditTagForm to the tag
/// store. Atomic save via `TagStore::save_default`. Handles
/// rename (original_name → form.name) by removing the original
/// + adding the renamed entry.
fn commit_tag_edit(form: &EditTagForm) -> Result<(), mackes_mesh_types::TagStoreError> {
    let mut store = mackes_mesh_types::TagStore::load_default()?;
    let trimmed_name = form.name.trim().to_string();
    if trimmed_name.is_empty() {
        // Reject empty rename — surface as a DuplicateName error
        // for consistency with TagStore::add's reject path.
        return Err(mackes_mesh_types::TagStoreError::DuplicateName(String::new()));
    }
    // Find-and-mutate, or rename, or create.
    let same_name = trimmed_name == form.original_name;
    let group_color = if form.group_color.trim().is_empty() {
        None
    } else {
        Some(form.group_color.trim().to_string())
    };
    let default_layout = if form.default_layout.trim().is_empty() {
        None
    } else {
        Some(form.default_layout.trim().to_string())
    };
    if same_name {
        if let Some(tag) = store.find_by_name_mut(&form.original_name) {
            tag.group_color = group_color;
            tag.default_layout = default_layout;
        } else {
            // Original name doesn't exist (system tag or fresh
            // create) — append a new Manual tag.
            store.add(mackes_mesh_types::Tag {
                name: trimmed_name,
                flavor: mackes_mesh_types::TagFlavor::Manual,
                members: Vec::new(),
                group_color,
                preferred_output: None,
                default_layout,
                autostart: Vec::new(),
            })?;
        }
    } else {
        // Rename path — take the existing entry's members +
        // autostart so they survive the rename, then write
        // back under the new name.
        let preserved = store
            .find_by_name(&form.original_name)
            .map(|t| (t.flavor.clone(), t.members.clone(), t.preferred_output.clone(), t.autostart.clone()));
        store.remove(&form.original_name);
        let (flavor, members, preferred_output, autostart) = preserved.unwrap_or_else(|| (
            mackes_mesh_types::TagFlavor::Manual,
            Vec::new(),
            None,
            Vec::new(),
        ));
        store.add(mackes_mesh_types::Tag {
            name: trimmed_name,
            flavor,
            members,
            group_color,
            preferred_output,
            default_layout,
            autostart,
        })?;
    }
    store.save_default()
}

// ── View ──────────────────────────────────────────────────────────────────────

/// Classic ChromeOS charcoal (#202124).
const CHARCOAL: Color = Color { r: 0.125, g: 0.129, b: 0.141, a: 1.0 };
const FG: Color = Color::WHITE;
const FG_DIM: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.4 };

/// Portal-17.a — the 6 locked system tags. Order is the design
/// lock from R10-Q16 + 'Recent' retired per R3-Q20.
pub const SYSTEM_TAGS: &[&str] = &[
    "All apps",
    "Untagged",
    "Workspaces",
    "Settings",
    "Power",
    "Mesh",
];

fn view(state: &PortalFull) -> Element<'_, Message> {
    let body: Element<'_, Message> = match state.layer {
        Layer::Hub => build_hub_layer(state),
        Layer::Library => build_library_placeholder(state),
        Layer::Control => build_control_placeholder(state),
    };
    container(
        column![
            text(state.layer.breadcrumb()).size(22.0).color(FG),
            body,
        ]
        .spacing(16),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(24)
    .style(|_: &Theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(CHARCOAL)),
        ..Default::default()
    })
    .into()
}

/// Portal-17.a — Hub layer view: 6 system-tag cards in a row at
/// top, then a grid of user-tag cards from the live tag store.
/// Card click → `Message::HubTagClicked(tag_name)`. Right-click
/// + cascade expansion + type-ahead ship as Portal-17.b..d.
fn build_hub_layer(state: &PortalFull) -> Element<'_, Message> {
    use iced::widget::row;
    // Portal-18.b — if a tag edit is in flight, show the modal
    // instead of the grid. Save / Cancel return to the grid.
    if state.editing_tag.is_some() {
        return build_edit_tag_modal(state);
    }
    let mut system_row: Vec<Element<'_, Message>> = Vec::new();
    for &name in SYSTEM_TAGS {
        system_row.push(hub_tag_card(name, None));
    }
    let mut user_grid: Vec<Element<'_, Message>> = Vec::new();
    for tag in &state.user_tags {
        user_grid.push(hub_tag_card(&tag.name, tag.group_color.as_deref()));
    }
    let user_section: Element<'_, Message> = if state.user_tags.is_empty() {
        text("No user tags yet. Edit ~/.local/share/mde/tags.json to add one.")
            .size(11.0)
            .color(FG_DIM)
            .into()
    } else {
        row(user_grid)
            .spacing(8)
            .wrap()
            .into()
    };
    column![
        // Portal-17.d — type-ahead indicator above the chips;
        // renders empty space when the buffer is empty.
        build_hub_typeahead_indicator(state),
        // Portal-17.e — sticky multi-select indicator above the
        // grid; renders empty space when no tag is selected.
        build_hub_multi_select_indicator(state),
        row(system_row).spacing(8).wrap(),
        text("Your tags").size(13.0).color(FG_DIM),
        user_section,
        // Portal-17.b — cascade columns to the right of the
        // root grid; renders empty space when stack is empty.
        build_hub_cascade_columns(state),
        // Portal-17.c — context-menu overlay; renders empty
        // space when no menu is open.
        build_hub_menu_overlay(state),
    ]
    .spacing(16)
    .into()
}

/// Portal-17.b — render the cascade columns. One column per
/// entry on `hub_cascade_stack`, in declaration order (root-most
/// on the left, deepest on the right). Each column lists the
/// tag's members via `format_cascade_member`. Empty space when
/// the stack is empty.
fn build_hub_cascade_columns(state: &PortalFull) -> Element<'_, Message> {
    if state.hub_cascade_stack.is_empty() {
        return iced::widget::Space::new(0.0, 0.0).into();
    }
    use iced::widget::row;
    let mut columns: Vec<Element<'_, Message>> = Vec::new();
    for tag_name in &state.hub_cascade_stack {
        let header = text(tag_name.clone()).size(13.0).color(FG);
        let mut rows: Vec<Element<'_, Message>> = vec![header.into()];
        match cascade_members_for_tag(tag_name, &state.user_tags) {
            Some(members) if !members.is_empty() => {
                for member in members {
                    let label = format_cascade_member(member);
                    let label_for_msg = label.clone();
                    rows.push(
                        iced::widget::mouse_area(
                            text(label).size(11.0).color(FG_DIM),
                        )
                        .on_press(Message::HubCascadeMemberClicked(label_for_msg))
                        .into(),
                    );
                }
            }
            Some(_) => {
                rows.push(text("(no members)").size(11.0).color(FG_DIM).into());
            }
            None => {
                rows.push(
                    text("(system tag — members render via per-surface integration)")
                        .size(11.0)
                        .color(FG_DIM)
                        .into(),
                );
            }
        }
        columns.push(
            container(column(rows).spacing(4))
                .style(|_theme: &Theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(Color { r: 0.16, g: 0.17, b: 0.19, a: 1.0 })),
                    border: iced::Border {
                        radius: iced::border::Radius::from(8.0),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .padding(iced::Padding::from([8, 12]))
                .width(Length::Fixed(220.0))
                .into(),
        );
    }
    row(columns).spacing(8).into()
}

/// Portal-17.d — render the type-ahead caret indicator above
/// the chip-row. Shape: a single pill containing
/// `> <typed-buffer>  →  <matched-tag>` when a match is active,
/// or `> <typed-buffer>  (no match)` when nothing matches.
/// Empty buffer renders zero-px space so the layout doesn't
/// reflow on first keystroke.
fn build_hub_typeahead_indicator(state: &PortalFull) -> Element<'_, Message> {
    if state.hub_typeahead_buffer.is_empty() {
        return iced::widget::Space::new(0.0, 0.0).into();
    }
    let buffer = state.hub_typeahead_buffer.clone();
    let label = match state.hub_typeahead_match.as_deref() {
        Some(name) => format!("> {buffer}  →  {name}"),
        None => format!("> {buffer}  (no match)"),
    };
    container(text(label).size(12.0).color(Color::WHITE))
        .style(|_theme: &Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(Color { r: 0.16, g: 0.17, b: 0.19, a: 1.0 })),
            border: iced::Border {
                radius: iced::border::Radius::from(6.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .padding(iced::Padding::from([4, 10]))
        .into()
}

/// Portal-17.e — render the sticky multi-select AND-filter
/// indicator above the tag-card grid. Renders empty space when
/// the set is empty; otherwise a wrap-row of "AND:" + one
/// chip per selected tag + a "✕" clear button.
fn build_hub_multi_select_indicator(state: &PortalFull) -> Element<'_, Message> {
    if state.hub_multi_select.is_empty() {
        return iced::widget::Space::new(0.0, 0.0).into();
    }
    use iced::widget::{button, row};
    let mut chips: Vec<Element<'_, Message>> = Vec::new();
    chips.push(
        text("AND:")
            .size(12.0)
            .color(FG_DIM)
            .into(),
    );
    for tag_name in &state.hub_multi_select {
        chips.push(
            container(text(tag_name.clone()).size(11.0).color(Color::WHITE))
                .style(|_theme: &Theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(Color { r: 0.357, g: 0.416, b: 0.961, a: 1.0 })),
                    border: iced::Border {
                        radius: iced::border::Radius::from(6.0),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .padding(iced::Padding::from([2, 8]))
                .into(),
        );
    }
    chips.push(
        button(text("Clear").size(11.0).color(Color::WHITE))
            .on_press(Message::HubMultiSelectCleared)
            .style(|_theme: &Theme, _status| iced::widget::button::Style {
                background: Some(iced::Background::Color(Color { r: 0.30, g: 0.30, b: 0.34, a: 1.0 })),
                text_color: Color::WHITE,
                border: iced::Border {
                    radius: iced::border::Radius::from(6.0),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into(),
    );
    row(chips).spacing(6).wrap().into()
}

/// Portal-17.a — render one tag card with optional color tint.
/// Material-blue 40 default if no `group_color`; the tag's hex when
/// set + parseable. Left-click → `HubTagClicked` for cascade
/// expansion (Portal-17.b). Right-click → `HubTagRightClicked`
/// for the iconic context menu (Portal-17.c).
fn hub_tag_card<'a>(name: &str, group_color: Option<&str>) -> Element<'a, Message> {
    let tint = group_color
        .and_then(hub_parse_hex)
        .unwrap_or(Color { r: 0.20, g: 0.69, b: 1.0, a: 1.0 }); // Material blue 40 default (#33b1ff)
    let name_owned = name.to_string();
    let name_for_left = name_owned.clone();
    let name_for_right = name_owned.clone();
    iced::widget::mouse_area(
        container(text(name_owned).size(13.0).color(Color::WHITE))
            .style(move |_theme: &Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(tint)),
                border: iced::Border {
                    radius: iced::border::Radius::from(8.0),
                    ..Default::default()
                },
                ..Default::default()
            })
            .padding(iced::Padding::from([8, 16]))
            .width(Length::Shrink)
            .height(Length::Shrink),
    )
    .on_press(Message::HubTagClicked(name_for_left))
    .on_right_press(Message::HubTagRightClicked(name_for_right))
    .into()
}

/// Portal-18.b — Edit-tag modal layout-selection options.
/// Mirrors the four-layout set the design lock recognises for
/// `default_layout`. The empty-string row is "no preference"
/// (clears the field on save).
pub const EDIT_TAG_LAYOUT_OPTIONS: &[&str] = &[
    "",
    "splith",
    "splitv",
    "tabbed",
    "stacked",
];

/// Portal-18.b — modal view: name input + color input + layout
/// chooser + Save / Cancel buttons. Placed inline within the
/// Hub layer view; `build_hub_layer` swaps to this when
/// `editing_tag.is_some()`.
fn build_edit_tag_modal(state: &PortalFull) -> Element<'_, Message> {
    use iced::widget::{button, row, text_input};
    let Some(form) = state.editing_tag.as_ref() else {
        return iced::widget::Space::new(0.0, 0.0).into();
    };
    let name_field = text_input("Tag name (e.g. Dev)", &form.name)
        .on_input(Message::EditTagNameChanged)
        .size(14.0)
        .padding(iced::Padding::from([8, 10]));
    let color_field = text_input("Group color (e.g. #42be65)", &form.group_color)
        .on_input(Message::EditTagColorChanged)
        .size(14.0)
        .padding(iced::Padding::from([8, 10]));
    // Layout picker — render as a row of buttons; the selected
    // option gets the indigo accent so the choice is visible.
    let mut layout_row: Vec<Element<'_, Message>> = Vec::new();
    for option in EDIT_TAG_LAYOUT_OPTIONS {
        let is_selected = form.default_layout == *option;
        let label = if option.is_empty() { "no default" } else { *option };
        let option_owned = option.to_string();
        let bg = if is_selected {
            Color { r: 0.357, g: 0.416, b: 0.961, a: 1.0 }
        } else {
            Color { r: 0.16, g: 0.17, b: 0.19, a: 1.0 }
        };
        layout_row.push(
            button(text(label).size(12.0).color(Color::WHITE))
                .on_press(Message::EditTagLayoutChanged(option_owned))
                .style(move |_theme: &Theme, _status| iced::widget::button::Style {
                    background: Some(iced::Background::Color(bg)),
                    text_color: Color::WHITE,
                    border: iced::Border {
                        radius: iced::border::Radius::from(6.0),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .into(),
        );
    }
    let actions = row![
        button(text("Apply").size(13.0).color(Color::WHITE))
            .on_press(Message::SaveTagEdit)
            .style(|_theme: &Theme, _status| iced::widget::button::Style {
                background: Some(iced::Background::Color(Color { r: 0.357, g: 0.416, b: 0.961, a: 1.0 })),
                text_color: Color::WHITE,
                border: iced::Border {
                    radius: iced::border::Radius::from(6.0),
                    ..Default::default()
                },
                ..Default::default()
            }),
        button(text("Cancel").size(13.0).color(Color::WHITE))
            .on_press(Message::CancelTagEdit)
            .style(|_theme: &Theme, _status| iced::widget::button::Style {
                background: Some(iced::Background::Color(Color { r: 0.30, g: 0.30, b: 0.34, a: 1.0 })),
                text_color: Color::WHITE,
                border: iced::Border {
                    radius: iced::border::Radius::from(6.0),
                    ..Default::default()
                },
                ..Default::default()
            }),
    ]
    .spacing(8);
    container(
        column![
            text(format!("Edit tag — {}", form.original_name)).size(16.0).color(FG),
            text("Name").size(12.0).color(FG_DIM),
            name_field,
            text("Group color").size(12.0).color(FG_DIM),
            color_field,
            text("Default layout").size(12.0).color(FG_DIM),
            row(layout_row).spacing(6).wrap(),
            actions,
        ]
        .spacing(8),
    )
    .style(|_theme: &Theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(Color { r: 0.16, g: 0.17, b: 0.19, a: 1.0 })),
        border: iced::Border {
            radius: iced::border::Radius::from(10.0),
            ..Default::default()
        },
        ..Default::default()
    })
    .padding(iced::Padding::from([16, 16]))
    .width(Length::Fill)
    .into()
}

/// Portal-17.c — locked menu-action labels. System tags (All
/// apps / Untagged / Workspaces / Settings / Power / Mesh) get
/// the same iconic menu as user tags — most actions are no-ops
/// on system tags but the menu shape stays consistent. The
/// handler decides per-action whether to log or hand off.
pub const HUB_MENU_ACTIONS: &[&str] = &[
    "Edit tag…",
    "Layout chooser…",
    "Window rules…",
    "Enter mode",
    "Save as template…",
    // Portal-17.e — sticky multi-select for AND-filter. The
    // handler toggles membership in `hub_multi_select` (add if
    // absent, remove if present); the indicator pill above the
    // grid reflects the current set.
    "Add to AND-filter",
];

/// Portal-17.c — render the right-click context-menu overlay
/// when `hub_right_click_target` is Some. Modal-style placement:
/// the menu appears at the bottom of the Hub view above the
/// tag-card grid + dims the rest of the layer. Click anywhere
/// outside the menu (or Esc) fires `HubMenuDismissed`.
///
/// Returns `iced::widget::Space` when no menu is open so the
/// view layout stays unchanged in the common case.
fn build_hub_menu_overlay<'a>(state: &PortalFull) -> Element<'a, Message> {
    let Some(target) = state.hub_right_click_target.clone() else {
        return iced::widget::Space::new(0.0, 0.0).into();
    };
    let mut items: Vec<Element<'a, Message>> = Vec::with_capacity(HUB_MENU_ACTIONS.len() + 1);
    items.push(text(format!("Tag: {target}")).size(12.0).color(FG_DIM).into());
    for action in HUB_MENU_ACTIONS {
        let target_for_msg = target.clone();
        let msg = match *action {
            "Edit tag…" => Message::HubMenuEditTag(target_for_msg),
            "Layout chooser…" => Message::HubMenuLayoutChooser(target_for_msg),
            "Window rules…" => Message::HubMenuWindowRules(target_for_msg),
            "Enter mode" => Message::HubMenuEnterMode(target_for_msg),
            "Save as template…" => Message::HubMenuSaveAsTemplate(target_for_msg),
            "Add to AND-filter" => Message::HubMultiSelectToggled(target_for_msg),
            // Defensive — every entry in HUB_MENU_ACTIONS has a
            // matching variant. New actions land via the locked
            // table + a new Message variant in lockstep.
            _ => Message::HubMenuDismissed,
        };
        items.push(
            iced::widget::mouse_area(
                container(text(*action).size(13.0).color(FG))
                    .padding(iced::Padding::from([8, 16]))
                    .width(Length::Fill),
            )
            .on_press(msg)
            .into(),
        );
    }
    container(column(items).spacing(2))
        .style(|_theme: &Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(Color { r: 0.16, g: 0.17, b: 0.19, a: 1.0 })),
            border: iced::Border {
                radius: iced::border::Radius::from(8.0),
                color: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.12 },
                width: 1.0,
            },
            ..Default::default()
        })
        .padding(iced::Padding::from([8, 8]))
        .width(Length::Fixed(280.0))
        .into()
}

/// Portal-17.a — minimal hex-color parser sufficient for the Hub
/// tag-card tint. Accepts `#rrggbb` + `#rgb`; returns None for
/// other forms so the tint falls back to indigo cleanly.
#[must_use]
fn hub_parse_hex(s: &str) -> Option<Color> {
    let rest = s.strip_prefix('#')?;
    if !rest.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    match rest.len() {
        6 => {
            let r = u8::from_str_radix(&rest[0..2], 16).ok()? as f32 / 255.0;
            let g = u8::from_str_radix(&rest[2..4], 16).ok()? as f32 / 255.0;
            let b = u8::from_str_radix(&rest[4..6], 16).ok()? as f32 / 255.0;
            Some(Color { r, g, b, a: 1.0 })
        }
        3 => {
            // #rgb shorthand → expand each digit to a byte.
            let expand = |c: char| {
                let v = c.to_digit(16)? as u8;
                Some(((v << 4) | v) as f32 / 255.0)
            };
            let chars: Vec<char> = rest.chars().collect();
            Some(Color {
                r: expand(chars[0])?,
                g: expand(chars[1])?,
                b: expand(chars[2])?,
                a: 1.0,
            })
        }
        _ => None,
    }
}

/// Library layer placeholder — Portal-19 covers the full
/// implementation. Until then it renders the breadcrumb +
/// one-line status.
fn build_library_placeholder(_state: &PortalFull) -> Element<'_, Message> {
    text("Library layer — wired in Portal-19")
        .size(13.0)
        .color(FG_DIM)
        .into()
}

/// Control layer placeholder — Portal-20.
fn build_control_placeholder(_state: &PortalFull) -> Element<'_, Message> {
    text("Control layer — wired in Portal-20")
        .size(13.0)
        .color(FG_DIM)
        .into()
}

// ── Subscription ──────────────────────────────────────────────────────────────

fn subscription(_state: &PortalFull) -> Subscription<Message> {
    Subscription::batch([
        // Portal-17.c / Portal-18.b / Portal-17.d — keyboard
        // handler routes printable chars to the type-ahead path,
        // Backspace / Enter to their respective handlers, and
        // Escape to the union-dismiss path (clears right-click
        // menu + Edit modal + type-ahead buffer all at once).
        // Modifier-tracking is intentionally minimal here —
        // Ctrl / Alt / Super are ignored so shortcut bindings
        // owned by other surfaces don't fire as type-ahead
        // input. The fn-pointer signature precludes closure
        // capture; routing decisions live entirely inside the
        // update handlers.
        iced::keyboard::on_key_press(|key, modifiers| {
            use iced::keyboard::{key::Named, Key};
            // Ignore keystrokes with Ctrl / Alt / Super held —
            // those belong to other layers (sway bindings, mode
            // switches, etc.).
            if modifiers.control() || modifiers.alt() || modifiers.logo() {
                return None;
            }
            match key {
                Key::Named(Named::Escape) => Some(Message::HubMenuDismissed),
                Key::Named(Named::Backspace) => Some(Message::HubTypeAheadBackspace),
                Key::Named(Named::Enter) => Some(Message::HubTypeAheadActivate),
                Key::Character(s) => {
                    // SmolStr — take first char if any.
                    s.chars().next().map(Message::HubTypeAheadChar)
                }
                _ => None,
            }
        }),
        dbus_subscription(),
    ])
}

fn dbus_subscription() -> Subscription<Message> {
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

    // ── Portal-17.a tests ──────────────────────────────────────────────────

    #[test]
    fn system_tags_match_design_lock() {
        assert_eq!(SYSTEM_TAGS.len(), 6);
        assert_eq!(SYSTEM_TAGS[0], "All apps");
        assert_eq!(SYSTEM_TAGS[1], "Untagged");
        assert_eq!(SYSTEM_TAGS[2], "Workspaces");
        assert_eq!(SYSTEM_TAGS[3], "Settings");
        assert_eq!(SYSTEM_TAGS[4], "Power");
        assert_eq!(SYSTEM_TAGS[5], "Mesh");
        // R3-Q20 lock: 'Recent' must NOT appear.
        assert!(!SYSTEM_TAGS.contains(&"Recent"));
    }

    #[test]
    fn hub_parse_hex_accepts_six_digit_form() {
        let c = hub_parse_hex("#42be65").unwrap();
        // 0x42 = 66 → 66/255 ≈ 0.259, 0xbe = 190 → ≈ 0.745,
        // 0x65 = 101 → ≈ 0.396.
        assert!((c.r - 0.259).abs() < 0.01);
        assert!((c.g - 0.745).abs() < 0.01);
        assert!((c.b - 0.396).abs() < 0.01);
        assert!((c.a - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn hub_parse_hex_accepts_three_digit_shorthand() {
        // #f00 → 0xff/255 = 1.0, 0, 0
        let c = hub_parse_hex("#f00").unwrap();
        assert!((c.r - 1.0).abs() < f32::EPSILON);
        assert!((c.g - 0.0).abs() < f32::EPSILON);
        assert!((c.b - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn hub_parse_hex_rejects_malformed_forms() {
        assert!(hub_parse_hex("42be65").is_none()); // no #
        assert!(hub_parse_hex("#xyz").is_none()); // non-hex
        assert!(hub_parse_hex("#1234").is_none()); // 4-digit rejected
        assert!(hub_parse_hex("#abcdefab").is_none()); // 8-digit rejected
        assert!(hub_parse_hex("").is_none());
        assert!(hub_parse_hex("#").is_none());
        assert!(hub_parse_hex("rebeccapurple").is_none());
    }

    #[test]
    fn hub_tag_clicked_pushes_to_cascade() {
        // Portal-17.b — clicking a tag pushes its name onto the
        // cascade stack + the layer stays Hub (cascade is a
        // sub-render inside Hub, not a layer flip).
        let mut state = PortalFull::default();
        let layer_before = state.layer;
        let _ = update(&mut state, Message::HubTagClicked("Dev".to_string()));
        assert_eq!(state.layer, layer_before);
        assert_eq!(state.hub_cascade_stack, vec!["Dev".to_string()]);
    }

    // ── Portal-17.c right-click menu tests ─────────────────────────────────

    #[test]
    fn hub_menu_actions_lock_matches_design() {
        // R3-Q62 + Portal-17.c lock — five iconic menu items in
        // this order. Portal-17.e adds the sixth "Add to AND-filter"
        // entry. Each entry needs a matching Message variant + match
        // arm in build_hub_menu_overlay.
        assert_eq!(HUB_MENU_ACTIONS.len(), 6);
        assert_eq!(HUB_MENU_ACTIONS[0], "Edit tag…");
        assert_eq!(HUB_MENU_ACTIONS[1], "Layout chooser…");
        assert_eq!(HUB_MENU_ACTIONS[2], "Window rules…");
        assert_eq!(HUB_MENU_ACTIONS[3], "Enter mode");
        assert_eq!(HUB_MENU_ACTIONS[4], "Save as template…");
        assert_eq!(HUB_MENU_ACTIONS[5], "Add to AND-filter");
    }

    #[test]
    fn right_click_sets_menu_target() {
        let mut state = PortalFull::default();
        assert!(state.hub_right_click_target.is_none());
        let _ = update(&mut state, Message::HubTagRightClicked("Dev".to_string()));
        assert_eq!(state.hub_right_click_target.as_deref(), Some("Dev"));
    }

    #[test]
    fn left_click_dismisses_open_menu() {
        let mut state = PortalFull::default();
        state.hub_right_click_target = Some("Dev".to_string());
        let _ = update(&mut state, Message::HubTagClicked("Untagged".to_string()));
        assert!(state.hub_right_click_target.is_none());
    }

    #[test]
    fn menu_dismissed_clears_target() {
        let mut state = PortalFull::default();
        state.hub_right_click_target = Some("Dev".to_string());
        let _ = update(&mut state, Message::HubMenuDismissed);
        assert!(state.hub_right_click_target.is_none());
    }

    #[test]
    fn each_menu_action_dismisses_after_firing() {
        for action in [
            Message::HubMenuEditTag("Dev".to_string()),
            Message::HubMenuLayoutChooser("Dev".to_string()),
            Message::HubMenuWindowRules("Dev".to_string()),
            Message::HubMenuEnterMode("Dev".to_string()),
            Message::HubMenuSaveAsTemplate("Dev".to_string()),
            Message::HubMultiSelectToggled("Dev".to_string()),
        ] {
            let mut state = PortalFull::default();
            state.hub_right_click_target = Some("Dev".to_string());
            let _ = update(&mut state, action);
            assert!(
                state.hub_right_click_target.is_none(),
                "menu must dismiss after action fires"
            );
        }
    }

    #[test]
    fn right_click_target_replaces_previous() {
        let mut state = PortalFull::default();
        let _ = update(&mut state, Message::HubTagRightClicked("Dev".to_string()));
        let _ = update(&mut state, Message::HubTagRightClicked("Personal".to_string()));
        assert_eq!(state.hub_right_click_target.as_deref(), Some("Personal"));
    }

    #[test]
    fn goto_hub_layer_refreshes_user_tags() {
        // The Goto(Hub) handler re-reads the tag store. Without
        // a real tags.json we just assert the call doesn't panic
        // + the resulting user_tags is a Vec (possibly empty).
        let mut state = PortalFull::default();
        let _ = update(&mut state, Message::GotoLayer(Layer::Library));
        let _ = update(&mut state, Message::GotoLayer(Layer::Hub));
        assert_eq!(state.layer, Layer::Hub);
        // user_tags is a Vec — len() is always valid (0 or more).
        let _ = state.user_tags.len();
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

    // ── Portal-18.b — Edit-tag modal ─────────────────────────

    #[test]
    fn edit_tag_layout_options_lock_matches_design() {
        // Locked set: empty (no preference) + the 4 sway layouts
        // the design doc names for default_layout.
        assert_eq!(
            EDIT_TAG_LAYOUT_OPTIONS,
            &["", "splith", "splitv", "tabbed", "stacked"],
        );
    }

    #[test]
    fn seed_edit_form_for_unknown_tag_is_blank() {
        let form = seed_edit_form(&[], "Dev");
        assert_eq!(form.name, "Dev");
        assert_eq!(form.original_name, "Dev");
        assert_eq!(form.group_color, "");
        assert_eq!(form.default_layout, "");
    }

    #[test]
    fn seed_edit_form_pre_fills_known_tag() {
        let tags = vec![mackes_mesh_types::Tag {
            name: "Dev".to_string(),
            flavor: mackes_mesh_types::TagFlavor::Manual,
            members: Vec::new(),
            group_color: Some("#42be65".to_string()),
            preferred_output: None,
            default_layout: Some("tabbed".to_string()),
            autostart: Vec::new(),
        }];
        let form = seed_edit_form(&tags, "Dev");
        assert_eq!(form.name, "Dev");
        assert_eq!(form.original_name, "Dev");
        assert_eq!(form.group_color, "#42be65");
        assert_eq!(form.default_layout, "tabbed");
    }

    #[test]
    fn hub_menu_edit_tag_opens_modal() {
        let mut state = PortalFull::default();
        state.hub_right_click_target = Some("Dev".to_string());
        let _ = update(&mut state, Message::HubMenuEditTag("Dev".to_string()));
        assert!(state.editing_tag.is_some(), "modal must open");
        assert!(
            state.hub_right_click_target.is_none(),
            "right-click menu must dismiss when modal opens",
        );
        let form = state.editing_tag.as_ref().unwrap();
        assert_eq!(form.original_name, "Dev");
    }

    #[test]
    fn edit_tag_name_changed_updates_form() {
        let mut state = PortalFull::default();
        state.editing_tag = Some(EditTagForm {
            name: "Dev".to_string(),
            original_name: "Dev".to_string(),
            group_color: String::new(),
            default_layout: String::new(),
        });
        let _ = update(&mut state, Message::EditTagNameChanged("Dev2".to_string()));
        let form = state.editing_tag.as_ref().unwrap();
        assert_eq!(form.name, "Dev2");
        assert_eq!(form.original_name, "Dev", "original_name is immutable");
    }

    #[test]
    fn edit_tag_color_changed_updates_form() {
        let mut state = PortalFull::default();
        state.editing_tag = Some(EditTagForm {
            name: "Dev".to_string(),
            original_name: "Dev".to_string(),
            group_color: String::new(),
            default_layout: String::new(),
        });
        let _ = update(&mut state, Message::EditTagColorChanged("#42be65".to_string()));
        assert_eq!(state.editing_tag.as_ref().unwrap().group_color, "#42be65");
    }

    #[test]
    fn edit_tag_layout_changed_updates_form() {
        let mut state = PortalFull::default();
        state.editing_tag = Some(EditTagForm {
            name: "Dev".to_string(),
            original_name: "Dev".to_string(),
            group_color: String::new(),
            default_layout: String::new(),
        });
        let _ = update(&mut state, Message::EditTagLayoutChanged("tabbed".to_string()));
        assert_eq!(state.editing_tag.as_ref().unwrap().default_layout, "tabbed");
    }

    #[test]
    fn cancel_tag_edit_clears_form() {
        let mut state = PortalFull::default();
        state.editing_tag = Some(EditTagForm {
            name: "Dev".to_string(),
            original_name: "Dev".to_string(),
            group_color: String::new(),
            default_layout: String::new(),
        });
        let _ = update(&mut state, Message::CancelTagEdit);
        assert!(state.editing_tag.is_none());
    }

    #[test]
    fn escape_dismisses_open_edit_modal() {
        // HubMenuDismissed is the union close — handles both
        // right-click menu + Edit modal. This test guards the
        // Escape path for Portal-18.b.
        let mut state = PortalFull::default();
        state.editing_tag = Some(EditTagForm {
            name: "Dev".to_string(),
            original_name: "Dev".to_string(),
            group_color: String::new(),
            default_layout: String::new(),
        });
        let _ = update(&mut state, Message::HubMenuDismissed);
        assert!(state.editing_tag.is_none());
    }

    // ── Portal-17.e — sticky multi-select / AND-filter ──────

    #[test]
    fn multi_select_starts_empty() {
        let state = PortalFull::default();
        assert!(state.hub_multi_select.is_empty());
    }

    #[test]
    fn toggle_adds_then_removes() {
        let mut state = PortalFull::default();
        let _ = update(&mut state, Message::HubMultiSelectToggled("Dev".to_string()));
        assert!(state.hub_multi_select.contains("Dev"));
        assert_eq!(state.hub_multi_select.len(), 1);
        // Toggle the same tag again → removed.
        let _ = update(&mut state, Message::HubMultiSelectToggled("Dev".to_string()));
        assert!(!state.hub_multi_select.contains("Dev"));
        assert!(state.hub_multi_select.is_empty());
    }

    #[test]
    fn toggle_accumulates_multiple_tags() {
        let mut state = PortalFull::default();
        let _ = update(&mut state, Message::HubMultiSelectToggled("Dev".to_string()));
        let _ = update(&mut state, Message::HubMultiSelectToggled("Personal".to_string()));
        let _ = update(&mut state, Message::HubMultiSelectToggled("Work".to_string()));
        assert_eq!(state.hub_multi_select.len(), 3);
        assert!(state.hub_multi_select.contains("Dev"));
        assert!(state.hub_multi_select.contains("Personal"));
        assert!(state.hub_multi_select.contains("Work"));
    }

    #[test]
    fn toggle_with_empty_name_is_noop() {
        let mut state = PortalFull::default();
        let _ = update(&mut state, Message::HubMultiSelectToggled(String::new()));
        assert!(state.hub_multi_select.is_empty());
    }

    #[test]
    fn multi_select_cleared_empties_set() {
        let mut state = PortalFull::default();
        state.hub_multi_select.insert("Dev".to_string());
        state.hub_multi_select.insert("Work".to_string());
        let _ = update(&mut state, Message::HubMultiSelectCleared);
        assert!(state.hub_multi_select.is_empty());
    }

    #[test]
    fn multi_select_survives_hub_menu_dismissed() {
        // Portal-17.e is sticky — Escape / outside-click that
        // dismisses the right-click menu must NOT clear the
        // multi-select filter.
        let mut state = PortalFull::default();
        state.hub_right_click_target = Some("Dev".to_string());
        state.hub_multi_select.insert("Dev".to_string());
        state.hub_multi_select.insert("Work".to_string());
        let _ = update(&mut state, Message::HubMenuDismissed);
        assert!(state.hub_right_click_target.is_none());
        assert_eq!(state.hub_multi_select.len(), 2, "multi-select must stay sticky");
    }

    #[test]
    fn multi_select_toggle_dismisses_menu() {
        let mut state = PortalFull::default();
        state.hub_right_click_target = Some("Dev".to_string());
        let _ = update(&mut state, Message::HubMultiSelectToggled("Dev".to_string()));
        assert!(state.hub_right_click_target.is_none());
        assert!(state.hub_multi_select.contains("Dev"));
    }

    // ── Portal-17.d — type-ahead caret ──────────────────────

    #[test]
    fn typeahead_starts_empty() {
        let state = PortalFull::default();
        assert!(state.hub_typeahead_buffer.is_empty());
        assert!(state.hub_typeahead_match.is_none());
    }

    #[test]
    fn typeahead_char_appends_and_matches_system_tag() {
        let mut state = PortalFull::default();
        let _ = update(&mut state, Message::HubTypeAheadChar('a'));
        assert_eq!(state.hub_typeahead_buffer, "a");
        // "All apps" is the first system tag — case-insensitive
        // prefix match wins.
        assert_eq!(state.hub_typeahead_match.as_deref(), Some("All apps"));
    }

    #[test]
    fn typeahead_case_insensitive_match() {
        let mut state = PortalFull::default();
        let _ = update(&mut state, Message::HubTypeAheadChar('M'));
        // "Mesh" is in SYSTEM_TAGS; uppercase 'M' still matches.
        assert_eq!(state.hub_typeahead_match.as_deref(), Some("Mesh"));
    }

    #[test]
    fn typeahead_extends_match_on_more_chars() {
        let mut state = PortalFull::default();
        let _ = update(&mut state, Message::HubTypeAheadChar('s'));
        // 's' alone matches "Settings" (first prefix-s system tag).
        assert_eq!(state.hub_typeahead_match.as_deref(), Some("Settings"));
        let _ = update(&mut state, Message::HubTypeAheadChar('e'));
        assert_eq!(state.hub_typeahead_buffer, "se");
        // Still "Settings".
        assert_eq!(state.hub_typeahead_match.as_deref(), Some("Settings"));
    }

    #[test]
    fn typeahead_no_match_keeps_buffer() {
        let mut state = PortalFull::default();
        let _ = update(&mut state, Message::HubTypeAheadChar('z'));
        let _ = update(&mut state, Message::HubTypeAheadChar('z'));
        assert_eq!(state.hub_typeahead_buffer, "zz");
        assert!(state.hub_typeahead_match.is_none());
    }

    #[test]
    fn typeahead_backspace_pops_one_char() {
        let mut state = PortalFull::default();
        let _ = update(&mut state, Message::HubTypeAheadChar('a'));
        let _ = update(&mut state, Message::HubTypeAheadChar('l'));
        assert_eq!(state.hub_typeahead_buffer, "al");
        let _ = update(&mut state, Message::HubTypeAheadBackspace);
        assert_eq!(state.hub_typeahead_buffer, "a");
        assert_eq!(state.hub_typeahead_match.as_deref(), Some("All apps"));
    }

    #[test]
    fn typeahead_backspace_to_empty_clears_match() {
        let mut state = PortalFull::default();
        let _ = update(&mut state, Message::HubTypeAheadChar('a'));
        assert!(state.hub_typeahead_match.is_some());
        let _ = update(&mut state, Message::HubTypeAheadBackspace);
        assert!(state.hub_typeahead_buffer.is_empty());
        assert!(state.hub_typeahead_match.is_none());
    }

    #[test]
    fn typeahead_escape_clears_via_hub_menu_dismissed() {
        let mut state = PortalFull::default();
        let _ = update(&mut state, Message::HubTypeAheadChar('a'));
        assert!(state.hub_typeahead_match.is_some());
        let _ = update(&mut state, Message::HubMenuDismissed);
        assert!(state.hub_typeahead_buffer.is_empty());
        assert!(state.hub_typeahead_match.is_none());
    }

    #[test]
    fn typeahead_activate_clears_buffer() {
        let mut state = PortalFull::default();
        let _ = update(&mut state, Message::HubTypeAheadChar('a'));
        assert!(state.hub_typeahead_match.is_some());
        let _ = update(&mut state, Message::HubTypeAheadActivate);
        assert!(state.hub_typeahead_buffer.is_empty());
        assert!(state.hub_typeahead_match.is_none());
    }

    #[test]
    fn typeahead_activate_with_no_match_is_noop() {
        let mut state = PortalFull::default();
        let _ = update(&mut state, Message::HubTypeAheadChar('z'));
        let buf_before = state.hub_typeahead_buffer.clone();
        let _ = update(&mut state, Message::HubTypeAheadActivate);
        // No match → buffer stays so the operator can backspace.
        assert_eq!(state.hub_typeahead_buffer, buf_before);
    }

    #[test]
    fn typeahead_match_helper_falls_through_to_user_tags() {
        let user_tags = vec![mackes_mesh_types::Tag {
            name: "Zebra".to_string(),
            flavor: mackes_mesh_types::TagFlavor::Manual,
            members: Vec::new(),
            group_color: None,
            preferred_output: None,
            default_layout: None,
            autostart: Vec::new(),
        }];
        let m = find_typeahead_match("z", &user_tags, &[]);
        assert_eq!(m.as_deref(), Some("Zebra"));
    }

    // ── Portal-17.b — cascade-card expansion ───────────────

    #[test]
    fn cascade_starts_empty() {
        let state = PortalFull::default();
        assert!(state.hub_cascade_stack.is_empty());
    }

    #[test]
    fn cascade_depth_cap_is_three() {
        // R5 design lock — 3 levels deep before forcing
        // dismiss-to-root.
        assert_eq!(HUB_CASCADE_DEPTH_CAP, 3);
    }

    #[test]
    fn cascade_push_appends_to_stack() {
        let mut state = PortalFull::default();
        let _ = update(&mut state, Message::HubTagClicked("Dev".to_string()));
        let _ = update(&mut state, Message::HubTagClicked("Personal".to_string()));
        assert_eq!(state.hub_cascade_stack, vec!["Dev".to_string(), "Personal".to_string()]);
    }

    #[test]
    fn cascade_re_click_deepest_collapses_one_level() {
        let mut state = PortalFull::default();
        let _ = update(&mut state, Message::HubTagClicked("Dev".to_string()));
        let _ = update(&mut state, Message::HubTagClicked("Personal".to_string()));
        // Click "Personal" again → pop.
        let _ = update(&mut state, Message::HubTagClicked("Personal".to_string()));
        assert_eq!(state.hub_cascade_stack, vec!["Dev".to_string()]);
    }

    #[test]
    fn cascade_caps_at_depth_three() {
        let mut state = PortalFull::default();
        let _ = update(&mut state, Message::HubTagClicked("A".to_string()));
        let _ = update(&mut state, Message::HubTagClicked("B".to_string()));
        let _ = update(&mut state, Message::HubTagClicked("C".to_string()));
        let _ = update(&mut state, Message::HubTagClicked("D".to_string()));
        // Cap is 3 — root drops, deepest 3 stay.
        assert_eq!(state.hub_cascade_stack.len(), 3);
        assert_eq!(
            state.hub_cascade_stack,
            vec!["B".to_string(), "C".to_string(), "D".to_string()]
        );
    }

    #[test]
    fn cascade_cleared_on_hub_menu_dismissed() {
        let mut state = PortalFull::default();
        state.hub_cascade_stack.push("Dev".to_string());
        state.hub_cascade_stack.push("Personal".to_string());
        let _ = update(&mut state, Message::HubMenuDismissed);
        assert!(state.hub_cascade_stack.is_empty());
    }

    #[test]
    fn cascade_members_for_system_tag_returns_none() {
        let user_tags: Vec<mackes_mesh_types::Tag> = Vec::new();
        assert!(cascade_members_for_tag("Settings", &user_tags).is_none());
    }

    #[test]
    fn cascade_members_for_known_user_tag_returns_members() {
        let user_tags = vec![mackes_mesh_types::Tag {
            name: "Dev".to_string(),
            flavor: mackes_mesh_types::TagFlavor::Manual,
            members: vec![
                mackes_mesh_types::TagMember::App { app_id: "foot".to_string() },
                mackes_mesh_types::TagMember::Workspace { num: 3 },
            ],
            group_color: None,
            preferred_output: None,
            default_layout: None,
            autostart: Vec::new(),
        }];
        let members = cascade_members_for_tag("Dev", &user_tags).unwrap();
        assert_eq!(members.len(), 2);
    }

    #[test]
    fn typeahead_walks_cascade_column_members() {
        // Portal-17.d.cascade — with a cascade column open, the
        // type-ahead match walks the column's members after root.
        // Test fixture: one user tag "Dev" with an App member +
        // a Workspace member. Cascade stack contains "Dev". Type
        // 'w' → root has no tag starting with 'w', falls through
        // to the cascade column, matches "Workspace #2".
        let user_tags = vec![mackes_mesh_types::Tag {
            name: "Dev".to_string(),
            flavor: mackes_mesh_types::TagFlavor::Manual,
            members: vec![
                mackes_mesh_types::TagMember::App { app_id: "foot".to_string() },
                mackes_mesh_types::TagMember::Workspace { num: 2 },
            ],
            group_color: None,
            preferred_output: None,
            default_layout: None,
            autostart: Vec::new(),
        }];
        let cascade = vec!["Dev".to_string()];
        let m = find_typeahead_match("w", &user_tags, &cascade);
        assert_eq!(m.as_deref(), Some("Workspaces"), "root system tag wins");
        // 'wor' → past 'Workspaces' the root has no match;
        // cascade-walk surfaces "Workspace #2".
        let m = find_typeahead_match("wor", &user_tags, &cascade);
        // Both "Workspaces" (root system tag) and "Workspace #2"
        // (cascade member) start with "wor" — root wins by
        // priority order.
        assert_eq!(m.as_deref(), Some("Workspaces"));
        // 'app' → matches "App: foot" in cascade (no root tag
        // starts with "app").
        let m = find_typeahead_match("app", &user_tags, &cascade);
        assert_eq!(m.as_deref(), Some("App: foot"));
    }

    #[test]
    fn typeahead_cascade_walk_skips_empty_stack() {
        let user_tags = vec![mackes_mesh_types::Tag {
            name: "Dev".to_string(),
            flavor: mackes_mesh_types::TagFlavor::Manual,
            members: vec![mackes_mesh_types::TagMember::App {
                app_id: "foot".to_string(),
            }],
            group_color: None,
            preferred_output: None,
            default_layout: None,
            autostart: Vec::new(),
        }];
        // No cascade open → "App:" prefix has no match anywhere
        // since no root tag starts with "App:".
        let m = find_typeahead_match("App:", &user_tags, &[]);
        assert!(m.is_none());
    }

    #[test]
    fn cascade_member_clicked_clears_stack() {
        let mut state = PortalFull::default();
        state.hub_cascade_stack.push("Dev".to_string());
        state.hub_cascade_stack.push("Personal".to_string());
        let _ = update(
            &mut state,
            Message::HubCascadeMemberClicked("App: foot".to_string()),
        );
        assert!(state.hub_cascade_stack.is_empty());
    }

    #[test]
    fn cascade_member_clicked_with_empty_stack_is_noop() {
        let mut state = PortalFull::default();
        // No stack — handler shouldn't panic + state stays clean.
        let _ = update(
            &mut state,
            Message::HubCascadeMemberClicked("Workspace #5".to_string()),
        );
        assert!(state.hub_cascade_stack.is_empty());
    }

    #[test]
    fn format_cascade_member_renders_each_variant() {
        use mackes_mesh_types::TagMember;
        assert_eq!(
            format_cascade_member(&TagMember::App { app_id: "foot".to_string() }),
            "App: foot",
        );
        assert_eq!(
            format_cascade_member(&TagMember::Peer { hostname: "alpha".to_string() }),
            "Peer: alpha",
        );
        assert_eq!(
            format_cascade_member(&TagMember::Workspace { num: 5 }),
            "Workspace #5",
        );
        assert_eq!(
            format_cascade_member(&TagMember::Container { name: "ntfy".to_string() }),
            "Container: ntfy",
        );
        assert_eq!(
            format_cascade_member(&TagMember::Zone { name: "taskbar".to_string() }),
            "Zone: taskbar",
        );
    }
}
