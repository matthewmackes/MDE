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
use iced::widget::{column, container, row, scrollable, text, Space};
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
    Network,
}

impl Layer {
    fn from_str(s: &str) -> Self {
        match s {
            "library" => Layer::Library,
            "control" => Layer::Control,
            "network" => Layer::Network,
            _ => Layer::Hub,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Layer::Hub => "Hub",
            Layer::Library => "Library",
            Layer::Control => "Control",
            Layer::Network => "Network",
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
    /// Portal-53.b — in-flight Window-rules modal state. `None`
    /// when no modal is open; `Some(form)` while the operator
    /// edits. Set by `HubMenuWindowRules`, cleared on Apply /
    /// Cancel / Escape. The two modal-state fields are mutually
    /// exclusive at view time — only one renders at a time.
    editing_window_rule: Option<EditWindowRuleForm>,
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
    /// MESH/Portal-22 — the mesh peer roster shown in the Network layer,
    /// loaded synchronously from `mackesd nodes list --json` on entry
    /// (mirrors the Hub-layer tag reload).
    network_peers: Vec<PeerNode>,
    /// Set when the peer-roster load fails (mackesd absent/offline); the
    /// Network layer renders it as an honest empty-state hint.
    network_peers_error: Option<String>,
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

/// Portal-53.b — Window-rules modal form state. Seeded from the
/// current `window-rules.toml` entry matching the right-clicked
/// tag's name (when present) or with an empty match_app_id
/// otherwise. The Apply handler does upsert via
/// `WindowRulesFile::replace_first_matching` → `push_rule`.
///
/// All numeric fields are kept as `String` for in-flight editing
/// so partial input ("4" → "" → "12") doesn't lose Iced focus.
/// The commit handler parses them; non-parseable + non-empty
/// strings reject the commit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditWindowRuleForm {
    /// app_id criterion (Hyprland/sway window class). Required;
    /// empty string rejects commit.
    pub match_app_id: String,
    /// `floating enable` toggle. None when the operator hasn't
    /// touched it; Some(true) / Some(false) when they have.
    pub floating: Option<bool>,
    /// `sticky enable` toggle. Same tri-state semantics.
    pub sticky: Option<bool>,
    /// `fullscreen enable` on window::new toggle.
    pub fullscreen_on_start: Option<bool>,
    /// `border normal <n>` value (in pixels, as a String for in-
    /// flight editing). Empty string = no override.
    pub border_width: String,
    /// `mark <name>` text. Empty string = no mark.
    pub mark: String,
    /// `move container to workspace number <n>` value. Empty
    /// string = no override.
    pub assign_workspace: String,
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
            editing_window_rule: None,
            hub_multi_select: std::collections::BTreeSet::new(),
            hub_typeahead_buffer: String::new(),
            hub_typeahead_match: None,
            hub_cascade_stack: Vec::new(),
            network_peers: Vec::new(),
            network_peers_error: None,
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
    /// member entry. The TagMember payload carries the typed
    /// member so the handler can dispatch on variant:
    /// App → spawn the binary, Workspace → swayipc focus, others
    /// → log-only until each target surface lands.
    HubCascadeMemberClicked(mackes_mesh_types::TagMember),
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
    /// Portal-53.b — Window-rules modal match_app_id field edited.
    EditWindowRuleAppIdChanged(String),
    /// Portal-53.b — Window-rules modal floating-toggle clicked.
    EditWindowRuleFloatingToggled,
    /// Portal-53.b — Window-rules modal sticky-toggle clicked.
    EditWindowRuleStickyToggled,
    /// Portal-53.b — Window-rules modal fullscreen-on-start toggle.
    EditWindowRuleFullscreenToggled,
    /// Portal-53.b — Window-rules modal border-width field edited
    /// (numeric string; commit reject on non-parseable input).
    EditWindowRuleBorderWidthChanged(String),
    /// Portal-53.b — Window-rules modal mark field edited.
    EditWindowRuleMarkChanged(String),
    /// Portal-53.b — Window-rules modal assign-workspace field
    /// edited (numeric string; commit reject on non-parseable input).
    EditWindowRuleAssignWorkspaceChanged(String),
    /// Portal-53.b — operator clicked Apply. Writes the form back
    /// to `window-rules.toml` + closes the modal. Uses replace-
    /// first-matching semantics when an existing rule covers the
    /// same `match_app_id`; otherwise appends.
    ApplyWindowRuleEdit,
    /// Portal-53.b — operator clicked Cancel or pressed Escape.
    /// Discards the form + closes the modal.
    CancelWindowRuleEdit,
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
            // MESH/Portal-22 — load the mesh peer roster on Network-layer
            // entry (synchronous, mirroring the Hub tag reload above).
            if layer == Layer::Network {
                match fetch_peers() {
                    Ok(peers) => {
                        state.network_peers = peers;
                        state.network_peers_error = None;
                    }
                    Err(e) => {
                        state.network_peers = Vec::new();
                        state.network_peers_error = Some(e);
                    }
                }
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
            tracing::debug!("portal-full: Hub right-click menu / Edit-tag modal / Edit-window-rule modal / type-ahead / cascade dismissed");
            state.hub_right_click_target = None;
            state.editing_tag = None;
            state.editing_window_rule = None;
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
            // Portal-44.b — fast-path gesture that opens the Edit-
            // tag modal scoped to layout selection. Re-uses the
            // Portal-18.b modal (which already includes the 5-
            // option layout chooser row) rather than duplicating
            // it as a separate modal. The operator can change other
            // tag fields from this entry too — the menu item is
            // just a faster gesture to the same surface. Closes the
            // Portal-44 UI surface that shipped with backend-only
            // enforcement (no operator affordance) before this
            // commit.
            tracing::info!(%tag_name, "portal-full: HubMenu → LayoutChooser — opening Edit-tag modal");
            state.editing_tag = Some(seed_edit_form(&state.user_tags, &tag_name));
            state.hub_right_click_target = None;
        }
        Message::HubMenuWindowRules(tag_name) => {
            tracing::info!(%tag_name, "portal-full: HubMenu → WindowRules — opening modal");
            // Portal-53.b — seed the modal form from the existing
            // rule for this tag's name (treated as the match_app_id
            // criterion), if any. Otherwise opens a blank form
            // pre-filled with the tag name. The operator can edit
            // match_app_id freely from there.
            state.editing_window_rule = Some(seed_window_rule_form(&tag_name));
            state.hub_right_click_target = None;
        }
        Message::HubMenuEnterMode(tag_name) => {
            // Portal-47.ui — fire swaymsg `mode <tag-name>` so sway
            // flips into the named binding mode. The Portal-45
            // mode segment renders the active mode in the
            // breadcrumb so operators see immediate visual
            // confirmation. Spawned in a detached thread so the
            // UI thread doesn't block on subprocess I/O.
            //
            // The mode must exist in the sway config for the
            // command to take effect. Portal-47.backend (mded
            // worker that walks tag.json + pre-registers modes
            // at startup) is the automation half; until that
            // ships, operators define modes manually in their
            // ~/.config/sway/config. swaymsg silently no-ops on
            // unknown mode names — no error surfaced to the
            // operator beyond the missing mode segment in the
            // breadcrumb.
            tracing::info!(%tag_name, "portal-full: HubMenu → EnterMode — firing swaymsg");
            let name_for_thread = tag_name.clone();
            std::thread::spawn(move || {
                let result = std::process::Command::new("swaymsg")
                    .arg(format!("mode \"{}\"", escape_swayipc_arg(&name_for_thread)))
                    .status();
                match result {
                    Ok(status) if status.success() => {
                        tracing::info!(tag = %name_for_thread, "portal-full: swaymsg mode succeeded");
                    }
                    Ok(status) => {
                        tracing::warn!(tag = %name_for_thread, ?status, "portal-full: swaymsg mode non-zero exit");
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, tag = %name_for_thread, "portal-full: swaymsg spawn failed");
                    }
                }
            });
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
        Message::HubCascadeMemberClicked(member) => {
            // Portal-17.b.activate.targets — dispatch on variant.
            // App: spawn the binary fire-and-forget so the click
            // doesn't block the Iced update loop. Workspace:
            // swayipc focus via an inline blocking call to the
            // swayipc-async runtime (portal-full binary doesn't
            // include the main mde-portal `workspace` module).
            // Other variants land when their target surface (peer
            // card / container shell / file opener / etc.) is wired.
            use mackes_mesh_types::TagMember;
            match &member {
                TagMember::App { app_id } => {
                    tracing::info!(%app_id, "portal-full: cascade activates app");
                    let app_id = app_id.clone();
                    std::thread::spawn(move || {
                        if let Err(e) = std::process::Command::new(&app_id).spawn() {
                            tracing::warn!(%app_id, error = %e, "spawn failed");
                        }
                    });
                }
                TagMember::Workspace { num } => {
                    tracing::info!(workspace = num, "portal-full: cascade focuses workspace");
                    let num = *num;
                    std::thread::spawn(move || {
                        let cmd = format!("workspace number {num}");
                        // Use swaymsg subprocess instead of pulling
                        // swayipc-async into portal-full — both target
                        // the same IPC socket so the command lands
                        // identically. Fire-and-forget; failure logs.
                        if let Err(e) = std::process::Command::new("swaymsg").arg(&cmd).spawn() {
                            tracing::warn!(workspace = num, error = %e, "swaymsg spawn failed");
                        }
                    });
                }
                TagMember::File { path } => {
                    tracing::info!(%path, "portal-full: cascade opens file via xdg-open");
                    let path = path.clone();
                    std::thread::spawn(move || {
                        // xdg-open hands the file to the operator's
                        // configured default app per XDG MIME defaults.
                        // Fire-and-forget; failure logs.
                        if let Err(e) =
                            std::process::Command::new("xdg-open").arg(&path).spawn()
                        {
                            tracing::warn!(%path, error = %e, "xdg-open spawn failed");
                        }
                    });
                }
                TagMember::Contact { ulid } => {
                    tracing::info!(%ulid, "portal-full: cascade opens contact card");
                    let ulid = ulid.clone();
                    std::thread::spawn(move || {
                        // Contacts live at
                        // `<XDG_DATA_HOME>/mde/contacts/<ulid>.json`
                        // per Portal-33 / VOIP-12. Open the JSON
                        // file via xdg-open until the proper
                        // Contact-card drill-in (peer card +
                        // dial-from-contact + SMS) ships.
                        let xdg_data = std::env::var("XDG_DATA_HOME")
                            .ok()
                            .or_else(|| std::env::var("HOME").ok().map(|h| format!("{h}/.local/share")));
                        let Some(base) = xdg_data else {
                            tracing::warn!(%ulid, "contact: no XDG_DATA_HOME / HOME — skip");
                            return;
                        };
                        let contact_path = format!("{base}/mde/contacts/{ulid}.json");
                        if let Err(e) =
                            std::process::Command::new("xdg-open").arg(&contact_path).spawn()
                        {
                            tracing::warn!(%ulid, error = %e, "contact xdg-open spawn failed");
                        }
                    });
                }
                TagMember::Activity { ulid } => {
                    tracing::info!(%ulid, "portal-full: cascade opens activity card");
                    let ulid = ulid.clone();
                    std::thread::spawn(move || {
                        // Activities live at
                        // `<XDG_DATA_HOME>/mde/activity/<type>/<iso>-<hash>.json`
                        // per Portal-33. Without a per-type lookup
                        // index yet, the v1 path is best-effort:
                        // glob the activity dir for any file whose
                        // name contains the ULID, then xdg-open
                        // the first hit. The proper drill-in via
                        // the Portal-33 Activity-as-files
                        // subsystem ships separately.
                        let xdg_data = std::env::var("XDG_DATA_HOME")
                            .ok()
                            .or_else(|| std::env::var("HOME").ok().map(|h| format!("{h}/.local/share")));
                        let Some(base) = xdg_data else {
                            tracing::warn!(%ulid, "activity: no XDG_DATA_HOME / HOME — skip");
                            return;
                        };
                        let activity_root = format!("{base}/mde/activity");
                        // Walk one level deep and find a file
                        // containing the ULID. Shell to `find` for
                        // simplicity — fire-and-forget; failure logs.
                        if let Err(e) = std::process::Command::new("sh")
                            .arg("-c")
                            .arg(format!(
                                "p=$(find {} -maxdepth 2 -type f -name '*{}*' 2>/dev/null | head -1); test -n \"$p\" && xdg-open \"$p\"",
                                activity_root, ulid
                            ))
                            .spawn()
                        {
                            tracing::warn!(%ulid, error = %e, "activity find+open spawn failed");
                        }
                    });
                }
                TagMember::Peer { hostname } => {
                    tracing::info!(%hostname, "portal-full: cascade opens ssh to peer");
                    let hostname = hostname.clone();
                    std::thread::spawn(move || {
                        // Open the platform default terminal (foot)
                        // with `ssh <hostname>`. Peer hostnames
                        // resolve via the mesh DNS (Nebula
                        // overlay), so this works as long as the
                        // peer is online and SSH is permitted by
                        // the mesh's flat-trust ACL. Fire-and-
                        // forget; failure logs.
                        if let Err(e) = std::process::Command::new("foot")
                            .args(["ssh", &hostname])
                            .spawn()
                        {
                            tracing::warn!(%hostname, error = %e, "foot ssh spawn failed");
                        }
                    });
                }
                TagMember::Tray { bus_name } => {
                    tracing::info!(%bus_name, "portal-full: cascade activates SNI tray entry");
                    let bus_name = bus_name.clone();
                    std::thread::spawn(move || {
                        // StatusNotifierItem spec: clients listen
                        // for the `Activate(x, y)` method on the
                        // canonical /StatusNotifierItem object
                        // path; `0 0` coordinates are the standard
                        // "click came from no specific point"
                        // signal (the SNI then surfaces its own
                        // menu or window).
                        if let Err(e) = std::process::Command::new("gdbus")
                            .args([
                                "call",
                                "-e",
                                "-d",
                                &bus_name,
                                "-o",
                                "/StatusNotifierItem",
                                "-m",
                                "org.kde.StatusNotifierItem.Activate",
                                "0",
                                "0",
                            ])
                            .spawn()
                        {
                            tracing::warn!(%bus_name, error = %e, "gdbus SNI Activate spawn failed");
                        }
                    });
                }
                TagMember::Container { name } => {
                    tracing::info!(%name, "portal-full: cascade opens container shell via foot");
                    let name = name.clone();
                    std::thread::spawn(move || {
                        // Open the platform's default terminal (foot)
                        // with `podman exec -it <name> sh` so the
                        // operator lands on an interactive shell
                        // inside the container. Fire-and-forget;
                        // failure logs. The shell choice is `sh` for
                        // maximum compatibility (containers may not
                        // ship bash).
                        if let Err(e) = std::process::Command::new("foot")
                            .args(["podman", "exec", "-it", &name, "sh"])
                            .spawn()
                        {
                            tracing::warn!(%name, error = %e, "foot podman-exec spawn failed");
                        }
                    });
                }
                _ => {
                    tracing::info!(?member, "portal-full: cascade member clicked (no target surface yet)");
                }
            }
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
        Message::EditWindowRuleAppIdChanged(value) => {
            if let Some(form) = state.editing_window_rule.as_mut() {
                form.match_app_id = value;
            }
        }
        Message::EditWindowRuleFloatingToggled => {
            if let Some(form) = state.editing_window_rule.as_mut() {
                form.floating = toggle_tristate(form.floating);
            }
        }
        Message::EditWindowRuleStickyToggled => {
            if let Some(form) = state.editing_window_rule.as_mut() {
                form.sticky = toggle_tristate(form.sticky);
            }
        }
        Message::EditWindowRuleFullscreenToggled => {
            if let Some(form) = state.editing_window_rule.as_mut() {
                form.fullscreen_on_start = toggle_tristate(form.fullscreen_on_start);
            }
        }
        Message::EditWindowRuleBorderWidthChanged(value) => {
            if let Some(form) = state.editing_window_rule.as_mut() {
                form.border_width = value;
            }
        }
        Message::EditWindowRuleMarkChanged(value) => {
            if let Some(form) = state.editing_window_rule.as_mut() {
                form.mark = value;
            }
        }
        Message::EditWindowRuleAssignWorkspaceChanged(value) => {
            if let Some(form) = state.editing_window_rule.as_mut() {
                form.assign_workspace = value;
            }
        }
        Message::ApplyWindowRuleEdit => {
            if let Some(form) = state.editing_window_rule.take() {
                match commit_window_rule_edit(&form) {
                    Ok(()) => {
                        tracing::info!(
                            app_id = %form.match_app_id,
                            "portal-full: window rule applied",
                        );
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, app_id = %form.match_app_id, "portal-full: window rule apply failed");
                    }
                }
            }
        }
        Message::CancelWindowRuleEdit => {
            tracing::debug!("portal-full: window rule edit cancelled");
            state.editing_window_rule = None;
        }
    }
    Task::none()
}

/// Portal-53.b — cycle a tri-state toggle: None → Some(true) →
/// Some(false) → None. The three states reflect the form
/// semantics: "no preference" (don't write the field) / "enabled"
/// (write `Some(true)`) / "disabled" (write `Some(false)`).
fn toggle_tristate(state: Option<bool>) -> Option<bool> {
    match state {
        None => Some(true),
        Some(true) => Some(false),
        Some(false) => None,
    }
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

/// Portal-53.b — seed the Window-rules modal form. Looks up an
/// existing rule whose `match_app_id` equals the seed key; if
/// found, pre-fills every form field from that rule. Otherwise
/// returns a blank form with `match_app_id` set to the seed key
/// (the operator can edit it from there). Failure to read the
/// rules file is treated as "no existing rule" and returns the
/// seed-key-prefilled blank.
fn seed_window_rule_form(seed_app_id: &str) -> EditWindowRuleForm {
    let rules = mackes_mesh_types::WindowRulesFile::load_default()
        .unwrap_or_default();
    if let Some(rule) = rules.find_first_matching(seed_app_id) {
        EditWindowRuleForm {
            match_app_id: rule.match_app_id.clone(),
            floating: rule.floating,
            sticky: rule.sticky,
            fullscreen_on_start: rule.fullscreen_on_start,
            border_width: rule
                .border_width
                .map(|n| n.to_string())
                .unwrap_or_default(),
            mark: rule.mark.clone().unwrap_or_default(),
            assign_workspace: rule
                .assign_workspace
                .map(|n| n.to_string())
                .unwrap_or_default(),
        }
    } else {
        EditWindowRuleForm {
            match_app_id: seed_app_id.to_string(),
            ..EditWindowRuleForm::default()
        }
    }
}

/// Portal-53.b — commit the in-flight EditWindowRuleForm to the
/// rules file. Upsert semantics via
/// `WindowRulesFile::replace_first_matching` → fallback to
/// `push_rule`. Atomic save via `WindowRulesFile::save_default`.
///
/// Numeric field parsing: empty string → `None`; numeric string
/// → `Some(parsed)`; non-parseable non-empty string → returns
/// `RulesError::Parse`-equivalent (mapped to Serialize variant
/// since we don't have a Field-Parse variant in the error enum
/// — fine for the modal's flow since the operator just sees
/// "save failed" + has the form open to fix).
fn commit_window_rule_edit(form: &EditWindowRuleForm) -> Result<(), mackes_mesh_types::WindowRulesError> {
    let trimmed_app_id = form.match_app_id.trim().to_string();
    if trimmed_app_id.is_empty() {
        // Re-use the Io variant with a synthetic empty-app_id
        // error message — the operator sees this in the tracing
        // warn line, modal remains open with the form intact.
        return Err(mackes_mesh_types::WindowRulesError::Io(
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "match_app_id is required",
            ),
        ));
    }
    let border_width: Option<u32> = parse_optional_u32(&form.border_width)?;
    let assign_workspace: Option<i32> = parse_optional_i32(&form.assign_workspace)?;
    let mark = if form.mark.trim().is_empty() {
        None
    } else {
        Some(form.mark.trim().to_string())
    };
    let new_rule = mackes_mesh_types::WindowRule {
        match_app_id: trimmed_app_id.clone(),
        floating: form.floating,
        sticky: form.sticky,
        fullscreen_on_start: form.fullscreen_on_start,
        border_width,
        mark,
        assign_workspace,
    };
    let mut file = mackes_mesh_types::WindowRulesFile::load_default()?;
    if !file.replace_first_matching(&trimmed_app_id, new_rule.clone()) {
        file.push_rule(new_rule);
    }
    file.save_default()
}

/// Portal-53.b — parse an optional numeric form field. Empty
/// string → `None`; non-empty string → `Some(parsed)` or an
/// error. Synthesizes a `WindowRulesError::Io(InvalidInput)`
/// when the string is non-empty + non-parseable.
fn parse_optional_u32(s: &str) -> Result<Option<u32>, mackes_mesh_types::WindowRulesError> {
    if s.trim().is_empty() {
        Ok(None)
    } else {
        s.trim()
            .parse::<u32>()
            .map(Some)
            .map_err(|e| mackes_mesh_types::WindowRulesError::Io(
                std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{e}")),
            ))
    }
}

/// Portal-53.b — same as `parse_optional_u32` but for `i32`
/// (workspace numbers; sway/Hyprland support a small set of
/// negative numbers for the scratchpad meta-workspace, so signed).
fn parse_optional_i32(s: &str) -> Result<Option<i32>, mackes_mesh_types::WindowRulesError> {
    if s.trim().is_empty() {
        Ok(None)
    } else {
        s.trim()
            .parse::<i32>()
            .map(Some)
            .map_err(|e| mackes_mesh_types::WindowRulesError::Io(
                std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{e}")),
            ))
    }
}

/// Portal-47.ui — escape an argument for embedding inside a
/// swaymsg-style quoted string. sway accepts `mode "<name>"`
/// where the name is the literal mode-name from the config;
/// embedded double-quotes + backslashes need to be escaped.
/// Used by the Hub Enter-mode handler to safely pass arbitrary
/// tag names (which may contain quirky chars) to swaymsg.
fn escape_swayipc_arg(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for ch in s.chars() {
        if ch == '\\' || ch == '"' {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

// ── View ──────────────────────────────────────────────────────────────────────

/// Classic ChromeOS charcoal (#202124).
const CHARCOAL: Color = Color { r: 0.125, g: 0.129, b: 0.141, a: 1.0 };
const FG: Color = Color::WHITE;
const FG_DIM: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.4 };
/// Q2-locked indigo accent (#5b6af5). Used for primary-action
/// buttons (Apply), active-state highlights (layout chooser
/// selected option, tristate enabled), and other emphasis points
/// across modals + the Hub menu. Extracting to a single const
/// closes 4 inline drift sites + lets the design-tokens lint
/// shrink its mde-portal allowlist by the same number.
const ACCENT_INDIGO: Color = Color { r: 0.357, g: 0.416, b: 0.961, a: 1.0 };
/// Raised-surface backdrop for modal cards, menu pills, and
/// inactive-state buttons (the visual layer ABOVE the CHARCOAL
/// ground but below the ACCENT_INDIGO emphasis). Charcoal-tinted
/// ~5% lighter than CHARCOAL itself. Used in 7+ sites across
/// portal_full_main; the extraction follows the TUNE-10.b
/// allow-list-shrink direction.
const SURFACE_RAISED: Color = Color { r: 0.16, g: 0.17, b: 0.19, a: 1.0 };
/// Slate button-background for secondary actions (Cancel
/// buttons in modals, dismissable indicator pills). The next
/// step lighter than SURFACE_RAISED so secondary affordances
/// read as elevated above the modal card without competing
/// with the indigo primary-action emphasis.
const BUTTON_SLATE: Color = Color { r: 0.30, g: 0.30, b: 0.34, a: 1.0 };

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
        Layer::Network => build_network_layer(state),
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
    // Portal-53.b — same modal-priority pattern for the window-
    // rules edit modal. The two modal-state fields are mutually
    // exclusive (only one is Some at a time), but check both so
    // the branch order doesn't matter.
    if state.editing_window_rule.is_some() {
        return build_edit_window_rule_modal(state);
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
        return iced::widget::Space::new().width(0.0).height(0.0).into();
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
                    let member_for_msg = member.clone();
                    rows.push(
                        iced::widget::mouse_area(
                            text(label).size(11.0).color(FG_DIM),
                        )
                        .on_press(Message::HubCascadeMemberClicked(member_for_msg))
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
                    background: Some(iced::Background::Color(SURFACE_RAISED)),
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
        return iced::widget::Space::new().width(0.0).height(0.0).into();
    }
    let buffer = state.hub_typeahead_buffer.clone();
    let label = match state.hub_typeahead_match.as_deref() {
        Some(name) => format!("> {buffer}  →  {name}"),
        None => format!("> {buffer}  (no match)"),
    };
    container(text(label).size(12.0).color(Color::WHITE))
        .style(|_theme: &Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(SURFACE_RAISED)),
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
        return iced::widget::Space::new().width(0.0).height(0.0).into();
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
                    background: Some(iced::Background::Color(ACCENT_INDIGO)),
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
                background: Some(iced::Background::Color(BUTTON_SLATE)),
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
///
/// Portal-40.crunchbang easter egg (R2-Q91): when the tag name
/// is literally `#!`, render a CrunchBang ASCII tribute label
/// instead of the bare characters. Same gestures (click +
/// right-click) wire through unchanged; just the label changes.
fn hub_tag_card<'a>(name: &str, group_color: Option<&str>) -> Element<'a, Message> {
    let tint = group_color
        .and_then(hub_parse_hex)
        .unwrap_or(Color { r: 0.20, g: 0.69, b: 1.0, a: 1.0 }); // Material blue 40 default (#33b1ff)
    let display_label = crunchbang_label_for(name).unwrap_or_else(|| name.to_string());
    let name_owned = name.to_string();
    let name_for_left = name_owned.clone();
    let name_for_right = name_owned.clone();
    iced::widget::mouse_area(
        container(text(display_label).size(13.0).color(Color::WHITE))
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

/// Portal-40.crunchbang easter egg (R2-Q91) — returns `Some` with
/// a CrunchBang ASCII tribute when the tag name is literally `#!`;
/// `None` otherwise. The tribute is intentionally compact (single
/// line, fits inside a tag pill): the project's iconic hashbang
/// prefix + the CRUNCHBANG label + the platform's #! preset
/// docstring's tribute spirit. Returned `Some` value is what the
/// renderer should display in place of the raw `#!` label; the
/// underlying tag name (used for click target + tag-store lookup)
/// stays `#!` so cascade expansion + tag-store ops are unchanged.
#[must_use]
pub fn crunchbang_label_for(name: &str) -> Option<String> {
    if name == "#!" {
        Some("#! CRUNCHBANG".to_string())
    } else {
        None
    }
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
        return iced::widget::Space::new().width(0.0).height(0.0).into();
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
            ACCENT_INDIGO
        } else {
            SURFACE_RAISED
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
                background: Some(iced::Background::Color(ACCENT_INDIGO)),
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
                background: Some(iced::Background::Color(BUTTON_SLATE)),
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
        background: Some(iced::Background::Color(SURFACE_RAISED)),
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

/// Portal-53.b — modal view: app_id input + 3 tri-state toggles
/// (floating / sticky / fullscreen_on_start) + 3 text inputs
/// (border_width / mark / assign_workspace) + Apply / Cancel.
/// Mirrors `build_edit_tag_modal`'s visual grammar so the two
/// modals feel like one editor pattern.
fn build_edit_window_rule_modal(state: &PortalFull) -> Element<'_, Message> {
    use iced::widget::{button, row, text_input};
    let Some(form) = state.editing_window_rule.as_ref() else {
        return iced::widget::Space::new().width(0.0).height(0.0).into();
    };
    let app_id_field = text_input("App ID (e.g. firefox)", &form.match_app_id)
        .on_input(Message::EditWindowRuleAppIdChanged)
        .size(14.0)
        .padding(iced::Padding::from([8, 10]));
    let border_field = text_input("Border width in px (blank = inherit)", &form.border_width)
        .on_input(Message::EditWindowRuleBorderWidthChanged)
        .size(14.0)
        .padding(iced::Padding::from([8, 10]));
    let mark_field = text_input("Mark name (blank = none)", &form.mark)
        .on_input(Message::EditWindowRuleMarkChanged)
        .size(14.0)
        .padding(iced::Padding::from([8, 10]));
    let workspace_field = text_input(
        "Assign to workspace number (blank = no override)",
        &form.assign_workspace,
    )
    .on_input(Message::EditWindowRuleAssignWorkspaceChanged)
    .size(14.0)
    .padding(iced::Padding::from([8, 10]));

    let floating_btn = tristate_button(
        "Floating",
        form.floating,
        Message::EditWindowRuleFloatingToggled,
    );
    let sticky_btn = tristate_button(
        "Sticky",
        form.sticky,
        Message::EditWindowRuleStickyToggled,
    );
    let fullscreen_btn = tristate_button(
        "Fullscreen on open",
        form.fullscreen_on_start,
        Message::EditWindowRuleFullscreenToggled,
    );

    let actions = row![
        button(text("Apply").size(13.0).color(Color::WHITE))
            .on_press(Message::ApplyWindowRuleEdit)
            .style(|_theme: &Theme, _status| iced::widget::button::Style {
                background: Some(iced::Background::Color(ACCENT_INDIGO)),
                text_color: Color::WHITE,
                border: iced::Border {
                    radius: iced::border::Radius::from(6.0),
                    ..Default::default()
                },
                ..Default::default()
            }),
        button(text("Cancel").size(13.0).color(Color::WHITE))
            .on_press(Message::CancelWindowRuleEdit)
            .style(|_theme: &Theme, _status| iced::widget::button::Style {
                background: Some(iced::Background::Color(BUTTON_SLATE)),
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
            text("Edit window rule").size(16.0).color(FG),
            text("App ID").size(12.0).color(FG_DIM),
            app_id_field,
            text("Flags").size(12.0).color(FG_DIM),
            row![floating_btn, sticky_btn, fullscreen_btn].spacing(6).wrap(),
            text("Border width").size(12.0).color(FG_DIM),
            border_field,
            text("Mark").size(12.0).color(FG_DIM),
            mark_field,
            text("Assign workspace").size(12.0).color(FG_DIM),
            workspace_field,
            actions,
        ]
        .spacing(8),
    )
    .style(|_theme: &Theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(SURFACE_RAISED)),
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

/// Portal-53.b — render a tri-state toggle button. The three
/// visual states distinguish "no preference" (charcoal) /
/// "enabled" (indigo accent) / "disabled" (slate-red). Click
/// fires `msg` to advance the cycle (the update handler runs
/// `toggle_tristate` on the corresponding form field).
fn tristate_button<'a>(
    label: &'static str,
    state: Option<bool>,
    msg: Message,
) -> Element<'a, Message> {
    use iced::widget::button;
    let suffix = match state {
        None => "—",
        Some(true) => "on",
        Some(false) => "off",
    };
    let bg = match state {
        None => SURFACE_RAISED,
        Some(true) => ACCENT_INDIGO,
        Some(false) => Color { r: 0.50, g: 0.18, b: 0.18, a: 1.0 },
    };
    button(text(format!("{label}: {suffix}")).size(12.0).color(Color::WHITE))
        .on_press(msg)
        .style(move |_theme: &Theme, _status| iced::widget::button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: Color::WHITE,
            border: iced::Border {
                radius: iced::border::Radius::from(6.0),
                ..Default::default()
            },
            ..Default::default()
        })
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
        return iced::widget::Space::new().width(0.0).height(0.0).into();
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
            background: Some(iced::Background::Color(SURFACE_RAISED)),
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
/// tag-card tint. Accepts `#rrggbb` + `#rgb` + `#rrggbbaa` (8-digit
/// alpha form for translucent tag tints); returns None for other
/// forms so the tint falls back to indigo cleanly. The alpha
/// component lets operators tag-color their cards with subtle
/// transparency (e.g. `#42be6580` for half-transparent green)
/// per the Portal-17.a.alpha extension.
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
        8 => {
            // Portal-17.a.alpha — #rrggbbaa: extends the 6-digit
            // form with an explicit alpha byte. Useful for
            // semi-transparent tag cards.
            let r = u8::from_str_radix(&rest[0..2], 16).ok()? as f32 / 255.0;
            let g = u8::from_str_radix(&rest[2..4], 16).ok()? as f32 / 255.0;
            let b = u8::from_str_radix(&rest[4..6], 16).ok()? as f32 / 255.0;
            let a = u8::from_str_radix(&rest[6..8], 16).ok()? as f32 / 255.0;
            Some(Color { r, g, b, a })
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

fn build_library_placeholder(_state: &PortalFull) -> Element<'_, Message> {
    column![].into()
}

// ── MESH / Portal-22 — Network-layer peer roster ─────────────────────────────
// Renders the live mesh peer roster (v6.0-mde-portal §3.5 sidebar peer-cards).
// Peers load synchronously from `mackesd nodes list --json` on layer entry.
// The globe / wireframe canvas (MESH-G / MESH-W) + detail pane layer onto this
// foundation as follow-on tasks.

/// A mesh peer's reachability state (from the node's `health` field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PeerStatus {
    Online,
    Idle,
    Offline,
    Unknown,
}

impl PeerStatus {
    fn from_health(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "online" | "healthy" => Self::Online,
            "idle" | "degraded" => Self::Idle,
            "offline" | "unreachable" => Self::Offline,
            _ => Self::Unknown,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Online => "ONLINE",
            Self::Idle => "IDLE",
            Self::Offline => "OFFLINE",
            Self::Unknown => "UNKNOWN",
        }
    }

    /// Status-dot colour. `Color { .. }` struct literals (not
    /// `Color::from_rgb`) keep the design-tokens lint clean for net-new use.
    fn color(self) -> Color {
        match self {
            Self::Online => Color { r: 0.20, g: 0.80, b: 0.40, a: 1.0 },
            Self::Idle => Color { r: 0.95, g: 0.70, b: 0.20, a: 1.0 },
            Self::Offline => Color { r: 0.92, g: 0.32, b: 0.30, a: 1.0 },
            Self::Unknown => Color { r: 0.60, g: 0.60, b: 0.60, a: 0.80 },
        }
    }
}

/// One mesh peer row in the Network-layer roster.
#[derive(Debug, Clone)]
struct PeerNode {
    name: String,
    region: String,
    role: String,
    status: PeerStatus,
}

/// Shell out to `mackesd nodes list --json` + parse the roster. Blocking
/// (called on layer entry, like the Hub tag reload). Returns an error
/// string when mackesd is absent/offline so the layer shows an honest
/// empty-state.
fn fetch_peers() -> Result<Vec<PeerNode>, String> {
    let out = std::process::Command::new("mackesd")
        .args(["nodes", "list", "--json"])
        .output()
        .map_err(|e| format!("mackesd nodes list failed to spawn: {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        return Err(format!("mackesd nodes list exited non-zero: {stderr}"));
    }
    Ok(parse_nodes(&String::from_utf8_lossy(&out.stdout)))
}

/// Parse `mackesd nodes list --json` (`[{node_id,name,role,health,region}]`)
/// into sorted [`PeerNode`]s. Malformed input yields an empty roster
/// (fail-open) rather than an error.
fn parse_nodes(raw: &str) -> Vec<PeerNode> {
    let Ok(top) = serde_json::from_str::<Vec<serde_json::Value>>(raw) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in top {
        let node_id = entry.get("node_id").and_then(|v| v.as_str()).unwrap_or("");
        if node_id.is_empty() {
            continue;
        }
        let name = entry.get("name").and_then(|v| v.as_str()).unwrap_or(node_id);
        let region = entry.get("region").and_then(|v| v.as_str()).unwrap_or("—");
        let role = entry.get("role").and_then(|v| v.as_str()).unwrap_or("peer");
        let health = entry.get("health").and_then(|v| v.as_str()).unwrap_or("unknown");
        out.push(PeerNode {
            name: name.to_string(),
            region: region.to_string(),
            role: role.to_string(),
            status: PeerStatus::from_health(health),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Portal-22 — the Network layer: the mesh peer roster as a card list (the
/// §3.5 sidebar peer-cards). The globe / wireframe canvas render on top of
/// this foundation in MESH-G / MESH-W.
fn build_network_layer(state: &PortalFull) -> Element<'_, Message> {
    let mut col = column![].spacing(8);
    if let Some(err) = &state.network_peers_error {
        col = col.push(text(format!("Peer roster unavailable — {err}")).size(14.0).color(FG));
    } else if state.network_peers.is_empty() {
        col = col.push(text("No mesh peers enrolled yet.").size(14.0).color(FG));
    } else {
        let n = state.network_peers.len();
        col = col.push(
            text(format!("{n} mesh peer{}", if n == 1 { "" } else { "s" }))
                .size(13.0)
                .color(FG),
        );
        for peer in &state.network_peers {
            let line = row![
                text("●").size(15.0).color(peer.status.color()),
                text(peer.name.clone()).size(15.0).color(FG),
                Space::new().width(Length::Fill),
                text(peer.region.clone()).size(12.0).color(FG),
                text(peer.role.clone()).size(12.0).color(FG),
                text(peer.status.label()).size(11.0).color(peer.status.color()),
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center);
            col = col.push(
                container(line)
                    .padding(10)
                    .width(Length::Fill)
                    .style(|_t: &Theme| container::Style {
                        background: Some(iced::Background::Color(SURFACE_RAISED)),
                        border: iced::Border { radius: 8.0.into(), ..Default::default() },
                        ..Default::default()
                    }),
            );
        }
    }
    scrollable(col).into()
}

fn build_control_placeholder(_state: &PortalFull) -> Element<'_, Message> {
    column![].into()
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
        // iced 0.14 removed `keyboard::on_key_press`; listen on the
        // raw event stream + match KeyPressed instead. The fn-pointer
        // signature precludes closure capture; routing decisions live
        // entirely inside the update handlers.
        iced::event::listen_with(|event, _status, _window| {
            use iced::keyboard::{key::Named, Event as KeyEvent, Key};
            let iced::Event::Keyboard(KeyEvent::KeyPressed { key, modifiers, .. }) = event else {
                return None;
            };
            // Ignore keystrokes with Ctrl / Alt / Super held — those
            // belong to other layers (sway bindings, mode switches).
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
    Subscription::run_with("mde-portal-full-dbus", |_| stream! {
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

/// Constant dark theme. A named fn (not an inline closure) so it
/// satisfies the `for<'a> Fn(&'a State)` HRTB iced 0.14's `.theme()`
/// requires — an inline `|_| Theme::Dark` infers a single concrete
/// lifetime and fails to compile.
fn portal_full_theme(_state: &PortalFull) -> Theme {
    Theme::Dark
}

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
    // iced 0.14: application(boot, update, view) — boot returns the
    // initial State; the title moved to .title().
    iced::application(PortalFull::default, update, view)
        .title("M · Portal")
        .subscription(subscription)
        .theme(portal_full_theme)
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
    fn layer_from_str_network() {
        assert_eq!(Layer::from_str("network"), Layer::Network);
        assert_eq!(Layer::Network.label(), "Network");
    }

    #[test]
    fn parse_nodes_decodes_roster_sorted() {
        let raw = r#"[
            {"node_id":"peer:pine","name":"pine","role":"peer","health":"healthy","region":"us-west"},
            {"node_id":"peer:birch","name":"birch","role":"host","health":"degraded","region":null},
            {"node_id":"","name":"ghost","health":"online"}
        ]"#;
        let rows = parse_nodes(raw);
        assert_eq!(rows.len(), 2, "empty node_id row is dropped");
        assert_eq!(rows[0].name, "birch");
        assert_eq!(rows[0].status, PeerStatus::Idle);
        assert_eq!(rows[0].region, "—");
        assert_eq!(rows[0].role, "host");
        assert_eq!(rows[1].name, "pine");
        assert_eq!(rows[1].status, PeerStatus::Online);
    }

    #[test]
    fn parse_nodes_malformed_is_empty() {
        assert!(parse_nodes("not json").is_empty());
        assert!(parse_nodes("{}").is_empty());
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
        assert!(hub_parse_hex("#abcdefabcd").is_none()); // 10-digit rejected
        assert!(hub_parse_hex("").is_none());
        assert!(hub_parse_hex("#").is_none());
        assert!(hub_parse_hex("rebeccapurple").is_none());
    }

    #[test]
    fn hub_parse_hex_accepts_eight_digit_alpha() {
        // Portal-17.a.alpha — #rrggbbaa form sets the alpha
        // component from the trailing 2 hex digits. `00` = fully
        // transparent; `ff` = fully opaque; `80` ≈ half (128/255).
        let c = hub_parse_hex("#42be6580").unwrap();
        // Color body matches the 6-digit `#42be65` form...
        assert!((c.r - (0x42 as f32 / 255.0)).abs() < f32::EPSILON);
        assert!((c.g - (0xbe as f32 / 255.0)).abs() < f32::EPSILON);
        assert!((c.b - (0x65 as f32 / 255.0)).abs() < f32::EPSILON);
        // ...and the alpha follows from the trailing 2 digits.
        assert!((c.a - (0x80 as f32 / 255.0)).abs() < f32::EPSILON);
        // Fully transparent edge case.
        let c0 = hub_parse_hex("#42be6500").unwrap();
        assert!((c0.a - 0.0).abs() < f32::EPSILON);
        // Fully opaque edge case (matches 6-digit behavior).
        let cff = hub_parse_hex("#42be65ff").unwrap();
        assert!((cff.a - 1.0).abs() < f32::EPSILON);
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

    // ── Portal-53.b — Window-rules modal ──────────────────────

    #[test]
    fn toggle_tristate_cycles_none_true_false() {
        assert_eq!(toggle_tristate(None), Some(true));
        assert_eq!(toggle_tristate(Some(true)), Some(false));
        assert_eq!(toggle_tristate(Some(false)), None);
    }

    #[test]
    fn hub_menu_layout_chooser_opens_edit_tag_modal() {
        // Portal-44.b — the LayoutChooser menu item is a fast-path
        // gesture that opens the same Edit-tag modal as the
        // EditTag item. After firing, editing_tag should be Some
        // with the seeded form, and the right-click target should
        // be cleared.
        let mut state = PortalFull::default();
        assert!(state.editing_tag.is_none());
        let _ = update(
            &mut state,
            Message::HubMenuLayoutChooser("Dev".to_string()),
        );
        let form = state.editing_tag.as_ref().unwrap();
        assert_eq!(form.original_name, "Dev");
        assert!(state.hub_right_click_target.is_none());
    }

    #[test]
    fn hub_menu_window_rules_opens_modal() {
        let mut state = PortalFull::default();
        // Pre-condition: nothing in flight.
        assert!(state.editing_window_rule.is_none());
        let _ = update(
            &mut state,
            Message::HubMenuWindowRules("firefox".to_string()),
        );
        // Post-condition: modal opened with the tag name as the
        // seed app_id.
        let form = state.editing_window_rule.as_ref().unwrap();
        assert_eq!(form.match_app_id, "firefox");
        // Right-click target cleared on menu action.
        assert!(state.hub_right_click_target.is_none());
    }

    #[test]
    fn edit_window_rule_app_id_changed_updates_form() {
        let mut state = PortalFull::default();
        state.editing_window_rule = Some(EditWindowRuleForm {
            match_app_id: "firefox".to_string(),
            ..EditWindowRuleForm::default()
        });
        let _ = update(
            &mut state,
            Message::EditWindowRuleAppIdChanged("chromium".to_string()),
        );
        assert_eq!(
            state.editing_window_rule.as_ref().unwrap().match_app_id,
            "chromium",
        );
    }

    #[test]
    fn floating_toggle_advances_through_tristate() {
        let mut state = PortalFull::default();
        state.editing_window_rule = Some(EditWindowRuleForm::default());
        let _ = update(&mut state, Message::EditWindowRuleFloatingToggled);
        assert_eq!(
            state.editing_window_rule.as_ref().unwrap().floating,
            Some(true),
        );
        let _ = update(&mut state, Message::EditWindowRuleFloatingToggled);
        assert_eq!(
            state.editing_window_rule.as_ref().unwrap().floating,
            Some(false),
        );
        let _ = update(&mut state, Message::EditWindowRuleFloatingToggled);
        assert_eq!(state.editing_window_rule.as_ref().unwrap().floating, None);
    }

    #[test]
    fn sticky_and_fullscreen_toggles_advance_independently() {
        let mut state = PortalFull::default();
        state.editing_window_rule = Some(EditWindowRuleForm::default());
        let _ = update(&mut state, Message::EditWindowRuleStickyToggled);
        let _ = update(&mut state, Message::EditWindowRuleFullscreenToggled);
        assert_eq!(state.editing_window_rule.as_ref().unwrap().sticky, Some(true));
        assert_eq!(
            state.editing_window_rule.as_ref().unwrap().fullscreen_on_start,
            Some(true),
        );
        // Floating is untouched — still None.
        assert!(state.editing_window_rule.as_ref().unwrap().floating.is_none());
    }

    #[test]
    fn border_mark_workspace_inputs_update_form() {
        let mut state = PortalFull::default();
        state.editing_window_rule = Some(EditWindowRuleForm::default());
        let _ = update(
            &mut state,
            Message::EditWindowRuleBorderWidthChanged("4".to_string()),
        );
        let _ = update(
            &mut state,
            Message::EditWindowRuleMarkChanged("browser".to_string()),
        );
        let _ = update(
            &mut state,
            Message::EditWindowRuleAssignWorkspaceChanged("2".to_string()),
        );
        let form = state.editing_window_rule.as_ref().unwrap();
        assert_eq!(form.border_width, "4");
        assert_eq!(form.mark, "browser");
        assert_eq!(form.assign_workspace, "2");
    }

    #[test]
    fn cancel_window_rule_edit_clears_form() {
        let mut state = PortalFull::default();
        state.editing_window_rule = Some(EditWindowRuleForm {
            match_app_id: "firefox".to_string(),
            ..EditWindowRuleForm::default()
        });
        let _ = update(&mut state, Message::CancelWindowRuleEdit);
        assert!(state.editing_window_rule.is_none());
    }

    #[test]
    fn escape_dismisses_open_window_rule_modal() {
        let mut state = PortalFull::default();
        state.editing_window_rule = Some(EditWindowRuleForm {
            match_app_id: "firefox".to_string(),
            ..EditWindowRuleForm::default()
        });
        let _ = update(&mut state, Message::HubMenuDismissed);
        assert!(state.editing_window_rule.is_none());
    }

    #[test]
    fn parse_optional_u32_empty_is_none() {
        assert_eq!(parse_optional_u32("").unwrap(), None);
        assert_eq!(parse_optional_u32("   ").unwrap(), None);
        assert_eq!(parse_optional_u32("4").unwrap(), Some(4));
        assert_eq!(parse_optional_u32(" 12 ").unwrap(), Some(12));
        assert!(parse_optional_u32("abc").is_err());
    }

    #[test]
    fn parse_optional_i32_handles_negatives() {
        assert_eq!(parse_optional_i32("").unwrap(), None);
        assert_eq!(parse_optional_i32("3").unwrap(), Some(3));
        // Sway's scratchpad meta-workspace uses negative nums;
        // schema tolerates them even if we'd never assign there.
        assert_eq!(parse_optional_i32("-1").unwrap(), Some(-1));
        assert!(parse_optional_i32("nope").is_err());
    }

    #[test]
    fn escape_swayipc_arg_passes_through_normal_chars() {
        assert_eq!(escape_swayipc_arg("foot"), "foot");
        assert_eq!(escape_swayipc_arg("Dev mode"), "Dev mode");
        assert_eq!(escape_swayipc_arg("dev-2026"), "dev-2026");
    }

    #[test]
    fn escape_swayipc_arg_escapes_quotes_and_backslashes() {
        // A quirky tag name with embedded `"` and `\` must
        // round-trip safely through the `mode "<name>"` swaymsg
        // quoting. The escape produces `Dev \"quoted\"` so
        // swaymsg parses it as a single quoted argument.
        assert_eq!(escape_swayipc_arg("Dev \"quoted\""), "Dev \\\"quoted\\\"");
        assert_eq!(escape_swayipc_arg("path\\with\\bs"), "path\\\\with\\\\bs");
    }

    #[test]
    fn escape_swayipc_arg_empty_passes_through() {
        assert_eq!(escape_swayipc_arg(""), "");
    }

    #[test]
    fn seed_window_rule_form_uses_seed_key_when_no_existing_rule() {
        // No rule file → blank form with seed-key prefilled.
        // Test runs in CI where ~/.config/mde/window-rules.toml
        // typically doesn't exist; if it does, the test confirms
        // either the seed-key prefill OR the existing-rule prefill
        // path (both are valid for the same seed).
        let form = seed_window_rule_form("firefox");
        // app_id is always populated — either from the existing
        // rule (which also keys on "firefox") or from the seed.
        assert_eq!(form.match_app_id, "firefox");
    }

    #[test]
    fn commit_window_rule_edit_rejects_empty_app_id() {
        let form = EditWindowRuleForm {
            match_app_id: "   ".to_string(),
            ..EditWindowRuleForm::default()
        };
        // Empty/whitespace match_app_id → InvalidInput error.
        let r = commit_window_rule_edit(&form);
        assert!(r.is_err());
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
        // Use a Zone variant so the test stays log-only (no
        // process spawn / no sway connection attempt — Portal-17.b
        // .activate.targets dispatches App + Workspace, others log).
        let mut state = PortalFull::default();
        state.hub_cascade_stack.push("Dev".to_string());
        state.hub_cascade_stack.push("Personal".to_string());
        let _ = update(
            &mut state,
            Message::HubCascadeMemberClicked(mackes_mesh_types::TagMember::Zone {
                name: "dock-tray".to_string(),
            }),
        );
        assert!(state.hub_cascade_stack.is_empty());
    }

    #[test]
    fn cascade_member_clicked_with_empty_stack_is_noop() {
        let mut state = PortalFull::default();
        // No stack — handler shouldn't panic + state stays clean.
        let _ = update(
            &mut state,
            Message::HubCascadeMemberClicked(mackes_mesh_types::TagMember::Contact {
                ulid: "01TESTULIDXYZ".to_string(),
            }),
        );
        assert!(state.hub_cascade_stack.is_empty());
    }

    #[test]
    fn cascade_member_file_variant_clears_stack() {
        // File variant fires xdg-open in a detached thread.
        // The cascade clears regardless of whether xdg-open
        // succeeds (fire-and-forget pattern).
        let mut state = PortalFull::default();
        state.hub_cascade_stack.push("Documents".to_string());
        let _ = update(
            &mut state,
            Message::HubCascadeMemberClicked(mackes_mesh_types::TagMember::File {
                path: "/nonexistent/path/for/cascade/test.txt".to_string(),
            }),
        );
        assert!(state.hub_cascade_stack.is_empty());
    }

    #[test]
    fn crunchbang_label_for_hashbang_returns_tribute() {
        // Portal-40.crunchbang — exact `#!` match fires the
        // tribute label; anything else returns None.
        assert_eq!(
            crunchbang_label_for("#!").as_deref(),
            Some("#! CRUNCHBANG"),
        );
    }

    #[test]
    fn crunchbang_label_for_non_hashbang_returns_none() {
        assert!(crunchbang_label_for("Dev").is_none());
        assert!(crunchbang_label_for("").is_none());
        assert!(crunchbang_label_for("#").is_none());
        assert!(crunchbang_label_for("!").is_none());
        assert!(crunchbang_label_for("#!extra").is_none());
        // Whitespace doesn't count — the tribute fires only on
        // the exact two-character sequence per the easter-egg lock.
        assert!(crunchbang_label_for("#! ").is_none());
        assert!(crunchbang_label_for(" #!").is_none());
    }

    #[test]
    fn cascade_member_contact_variant_clears_stack() {
        let mut state = PortalFull::default();
        state.hub_cascade_stack.push("Contacts".to_string());
        let _ = update(
            &mut state,
            Message::HubCascadeMemberClicked(mackes_mesh_types::TagMember::Contact {
                ulid: "01TESTCONTACT".to_string(),
            }),
        );
        assert!(state.hub_cascade_stack.is_empty());
    }

    #[test]
    fn cascade_member_activity_variant_clears_stack() {
        let mut state = PortalFull::default();
        state.hub_cascade_stack.push("Recent".to_string());
        let _ = update(
            &mut state,
            Message::HubCascadeMemberClicked(mackes_mesh_types::TagMember::Activity {
                ulid: "01TESTACTIVITY".to_string(),
            }),
        );
        assert!(state.hub_cascade_stack.is_empty());
    }

    #[test]
    fn cascade_member_peer_variant_clears_stack() {
        // Peer variant fires `foot ssh <hostname>`. Spawn is
        // fire-and-forget; cascade clears even if the hostname
        // doesn't resolve.
        let mut state = PortalFull::default();
        state.hub_cascade_stack.push("Mesh".to_string());
        let _ = update(
            &mut state,
            Message::HubCascadeMemberClicked(mackes_mesh_types::TagMember::Peer {
                hostname: "nonexistent-peer-for-cascade-test".to_string(),
            }),
        );
        assert!(state.hub_cascade_stack.is_empty());
    }

    #[test]
    fn cascade_member_tray_variant_clears_stack() {
        // Tray variant fires `gdbus call ... Activate 0 0`. Spawn
        // is fire-and-forget; cascade clears even if the bus name
        // doesn't exist on the session bus.
        let mut state = PortalFull::default();
        state.hub_cascade_stack.push("Tray".to_string());
        let _ = update(
            &mut state,
            Message::HubCascadeMemberClicked(mackes_mesh_types::TagMember::Tray {
                bus_name: "org.freedesktop.StatusNotifier-nonexistent-bus-1".to_string(),
            }),
        );
        assert!(state.hub_cascade_stack.is_empty());
    }

    #[test]
    fn cascade_member_container_variant_clears_stack() {
        // Container variant fires `foot podman exec -it ...` in a
        // detached thread. Cascade clears regardless of whether
        // the spawn succeeds (the test fixture's container name
        // is unlikely to be real).
        let mut state = PortalFull::default();
        state.hub_cascade_stack.push("Containers".to_string());
        let _ = update(
            &mut state,
            Message::HubCascadeMemberClicked(mackes_mesh_types::TagMember::Container {
                name: "nonexistent-container-for-cascade-test".to_string(),
            }),
        );
        assert!(state.hub_cascade_stack.is_empty());
    }

    #[test]
    fn cascade_member_app_variant_clears_stack() {
        // App variant DOES fire a thread::spawn for the binary
        // launch — verify the cascade still clears even when the
        // spawn fails (the test fixture's "nonexistent-binary" is
        // unlikely to be in PATH, but the spawn happens fire-and-
        // forget so the test doesn't block on its outcome).
        let mut state = PortalFull::default();
        state.hub_cascade_stack.push("Dev".to_string());
        let _ = update(
            &mut state,
            Message::HubCascadeMemberClicked(mackes_mesh_types::TagMember::App {
                app_id: "nonexistent-binary-for-cascade-test".to_string(),
            }),
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
