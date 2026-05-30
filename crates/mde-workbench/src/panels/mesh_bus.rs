//! BUS-7.2 — Network → Mackes Bus panel.
//!
//! 5-tab panel: Topics / Subscriptions / Hooks / Audit / DND.
//! BUS-7.2 ships the skeleton + DND tab real content (BUS-7.6).
//! BUS-7.3..BUS-7.5 fill in the remaining content tabs.
//!
//! Cite: docs/design/v6.x-mackes-bus.md §7 (operator surfaces);
//! ref: Linear (notification-settings tab bar).

use std::path::PathBuf;

use iced::widget::button::Status as ButtonStatus;
use iced::widget::{button, column, row, text, text_input, Space};
use iced::{alignment, Background, Border, Color, Element, Length, Task};
use mde_theme::{Density, EmptyState, FontSize, Icon, Palette, Radii, TypeRole};

use crate::panel_chrome::{empty_state, panel_container};

// ─── local mirror types ──────────────────────────────────────────────────────
// These shadow the mde-bus structs so the workbench crate does not
// need the full mde-bus dep. Same serde field names → same YAML/JSON.

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct DndStatusJson {
    #[serde(default)]
    active: bool,
    #[serde(default)]
    since_unix_ms: i64,
    #[serde(default)]
    set_by_peer: String,
    #[serde(default)]
    snoozes: Vec<SnoozeJson>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct SnoozeJson {
    topic: String,
    until_unix_ms: i64,
    #[serde(default)]
    set_by_peer: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct SubsYaml {
    #[serde(default)]
    topics: Vec<String>,
    #[serde(default)]
    mute: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quiet_hours: Option<QuietHoursYaml>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct QuietHoursYaml {
    start: String,
    end: String,
}

// ─── Subscriptions tab state ─────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct SubsTabState {
    /// Subscribed topic patterns (from `subs.yaml topics:`).
    pub topics: Vec<String>,
    /// Muted patterns (from `subs.yaml mute:`).
    pub muted: Vec<String>,
    pub new_topic: String,
    pub peer_input: String,
    pub loading: bool,
    pub error: Option<String>,
    pub loaded: bool,
}

// ─── DND tab state ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct DndTabState {
    pub status: Option<DndStatusJson>,
    pub quiet_start: String,
    pub quiet_end: String,
    pub loading: bool,
    pub saving: bool,
    pub error: Option<String>,
    pub loaded: bool,
}

// ─── Tab enum ────────────────────────────────────────────────────────────────

/// The five Bus panel tabs, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Topics,
    Subscriptions,
    Hooks,
    Audit,
    Dnd,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Self::Topics => "Topics",
            Self::Subscriptions => "Subscriptions",
            Self::Hooks => "Hooks",
            Self::Audit => "Audit",
            Self::Dnd => "DND",
        }
    }

    const ALL: [Tab; 5] = [
        Tab::Topics,
        Tab::Subscriptions,
        Tab::Hooks,
        Tab::Audit,
        Tab::Dnd,
    ];
}

// ─── Panel struct ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct MeshBusPanel {
    pub active_tab: Tab,
    pub subs: SubsTabState,
    pub dnd: DndTabState,
}

// ─── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    SelectTab(Tab),
    // Subscriptions tab
    SubsLoaded(Result<(Vec<String>, Vec<String>), String>),
    SubsNewTopicChanged(String),
    SubsAddClicked,
    SubsAddDone(Result<(), String>),
    SubsRemoveClicked(String),
    SubsRemoveDone(Result<(), String>),
    SubsPeerInputChanged(String),
    SubsMatchPeerClicked,
    SubsMatchPeerDone(Result<Vec<String>, String>),
    SubsRefreshClicked,
    // DND tab
    DndLoaded(Result<(DndStatusJson, String, String), String>),
    DndToggleClicked,
    DndToggleDone(Result<(), String>),
    DndRefreshClicked,
    DndQuietStartChanged(String),
    DndQuietEndChanged(String),
    DndSaveClicked,
    DndSaveDone(Result<(), String>),
}

// ─── Async helpers ────────────────────────────────────────────────────────────

fn bus_root() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|h| PathBuf::from(h).join(".local").join("share"))
        })
        .map(|d| d.join("mde").join("bus"))
}

async fn load_subs() -> Result<(Vec<String>, Vec<String>), String> {
    let root = bus_root().ok_or_else(|| "no data dir".to_string())?;
    let path = root.join("subs.yaml");
    let txt = tokio::fs::read_to_string(&path)
        .await
        .unwrap_or_default();
    let manifest: SubsYaml = serde_yaml::from_str(&txt).unwrap_or_default();
    Ok((manifest.topics, manifest.mute))
}

async fn sub_add(topic: String) -> Result<(), String> {
    let out = tokio::process::Command::new("mde-bus")
        .args(["sub", "add", &topic])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}

async fn sub_remove(topic: String) -> Result<(), String> {
    let out = tokio::process::Command::new("mde-bus")
        .args(["sub", "remove", &topic])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}

/// Copy a peer's subscriptions from the mesh-storage mount.
/// Looks for the peer's subs.yaml at
/// `~/.mde-mesh/<peer>/.local/share/mde/bus/subs.yaml`
/// (LizardFS per-peer home per MESHFS-4.1 mount layout).
async fn match_peer_subs(peer: String) -> Result<Vec<String>, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "no $HOME".to_string())?;
    let peer_subs = home
        .join(".mde-mesh")
        .join(&peer)
        .join(".local")
        .join("share")
        .join("mde")
        .join("bus")
        .join("subs.yaml");
    let txt = tokio::fs::read_to_string(&peer_subs)
        .await
        .map_err(|e| format!("peer @{peer} not reachable via mesh storage: {e}"))?;
    let manifest: SubsYaml = serde_yaml::from_str(&txt)
        .map_err(|e| e.to_string())?;
    // Merge into local subs.yaml — add any topic not yet subscribed.
    let root = bus_root().ok_or_else(|| "no data dir".to_string())?;
    let local_path = root.join("subs.yaml");
    let local_txt = tokio::fs::read_to_string(&local_path)
        .await
        .unwrap_or_default();
    let mut local: SubsYaml = serde_yaml::from_str(&local_txt).unwrap_or_default();
    let mut added = Vec::new();
    for t in &manifest.topics {
        if !local.topics.contains(t) {
            local.topics.push(t.clone());
            added.push(t.clone());
        }
    }
    if !added.is_empty() {
        let yaml = serde_yaml::to_string(&local).map_err(|e| e.to_string())?;
        tokio::fs::write(&local_path, yaml)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(added)
}

async fn load_dnd() -> Result<(DndStatusJson, String, String), String> {
    // Read DND state via mde-bus CLI (JSON output).
    let out = tokio::process::Command::new("mde-bus")
        .args(["dnd", "status", "--json"])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    let status: DndStatusJson = if out.status.success() && !out.stdout.is_empty() {
        serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?
    } else {
        DndStatusJson::default()
    };

    // Read quiet hours from subs.yaml.
    let (qs, qe) = if let Some(root) = bus_root() {
        let path = root.join("subs.yaml");
        match tokio::fs::read_to_string(&path).await {
            Ok(txt) => {
                let manifest: SubsYaml =
                    serde_yaml::from_str(&txt).unwrap_or_default();
                if let Some(qh) = manifest.quiet_hours {
                    (qh.start, qh.end)
                } else {
                    (String::new(), String::new())
                }
            }
            Err(_) => (String::new(), String::new()),
        }
    } else {
        (String::new(), String::new())
    };

    Ok((status, qs, qe))
}

async fn toggle_dnd() -> Result<(), String> {
    let out = tokio::process::Command::new("mde-bus")
        .args(["dnd", "toggle"])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}

async fn save_quiet_hours(start: String, end: String) -> Result<(), String> {
    let root = bus_root().ok_or_else(|| "no data dir".to_string())?;
    let path = root.join("subs.yaml");

    let mut manifest: SubsYaml = match tokio::fs::read_to_string(&path).await {
        Ok(txt) => serde_yaml::from_str(&txt).unwrap_or_default(),
        Err(_) => SubsYaml::default(),
    };

    if start.is_empty() && end.is_empty() {
        manifest.quiet_hours = None;
    } else {
        manifest.quiet_hours = Some(QuietHoursYaml { start, end });
    }

    let yaml = serde_yaml::to_string(&manifest).map_err(|e| e.to_string())?;
    tokio::fs::write(&path, yaml)
        .await
        .map_err(|e| e.to_string())
}

// ─── Panel impl ───────────────────────────────────────────────────────────────

impl MeshBusPanel {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, msg: Message) -> Task<crate::Message> {
        match msg {
            Message::SelectTab(tab) => {
                self.active_tab = tab;
                if tab == Tab::Subscriptions
                    && !self.subs.loaded
                    && !self.subs.loading
                {
                    self.subs.loading = true;
                    return Task::perform(load_subs(), |r| {
                        crate::Message::MeshBus(Message::SubsLoaded(r))
                    });
                }
                if tab == Tab::Dnd && !self.dnd.loaded && !self.dnd.loading {
                    self.dnd.loading = true;
                    return Task::perform(load_dnd(), |r| {
                        crate::Message::MeshBus(Message::DndLoaded(r))
                    });
                }
                Task::none()
            }

            Message::SubsLoaded(result) => {
                self.subs.loading = false;
                self.subs.loaded = true;
                match result {
                    Ok((topics, muted)) => {
                        self.subs.topics = topics;
                        self.subs.muted = muted;
                        self.subs.error = None;
                    }
                    Err(e) => self.subs.error = Some(e),
                }
                Task::none()
            }

            Message::SubsNewTopicChanged(s) => {
                self.subs.new_topic = s;
                Task::none()
            }

            Message::SubsAddClicked => {
                let topic = self.subs.new_topic.trim().to_string();
                if topic.is_empty() {
                    return Task::none();
                }
                self.subs.new_topic.clear();
                Task::perform(sub_add(topic), |r| {
                    crate::Message::MeshBus(Message::SubsAddDone(r))
                })
            }

            Message::SubsAddDone(result) => {
                match result {
                    Ok(()) => {
                        self.subs.loaded = false;
                        self.subs.loading = true;
                        Task::perform(load_subs(), |r| {
                            crate::Message::MeshBus(Message::SubsLoaded(r))
                        })
                    }
                    Err(e) => {
                        self.subs.error = Some(e);
                        Task::none()
                    }
                }
            }

            Message::SubsRemoveClicked(topic) => {
                Task::perform(sub_remove(topic), |r| {
                    crate::Message::MeshBus(Message::SubsRemoveDone(r))
                })
            }

            Message::SubsRemoveDone(result) => match result {
                Ok(()) => {
                    self.subs.loaded = false;
                    self.subs.loading = true;
                    Task::perform(load_subs(), |r| {
                        crate::Message::MeshBus(Message::SubsLoaded(r))
                    })
                }
                Err(e) => {
                    self.subs.error = Some(e);
                    Task::none()
                }
            },

            Message::SubsPeerInputChanged(s) => {
                self.subs.peer_input = s;
                Task::none()
            }

            Message::SubsMatchPeerClicked => {
                let peer = self.subs.peer_input.trim().to_string();
                if peer.is_empty() {
                    return Task::none();
                }
                Task::perform(match_peer_subs(peer), |r| {
                    crate::Message::MeshBus(Message::SubsMatchPeerDone(r))
                })
            }

            Message::SubsMatchPeerDone(result) => match result {
                Ok(added) => {
                    self.subs.loaded = false;
                    self.subs.loading = true;
                    if added.is_empty() {
                        self.subs.error = Some("No new topics from that peer.".to_string());
                        self.subs.loading = false;
                        self.subs.loaded = true;
                        return Task::none();
                    }
                    Task::perform(load_subs(), |r| {
                        crate::Message::MeshBus(Message::SubsLoaded(r))
                    })
                }
                Err(e) => {
                    self.subs.error = Some(e);
                    Task::none()
                }
            },

            Message::SubsRefreshClicked => {
                self.subs.loaded = false;
                self.subs.loading = true;
                Task::perform(load_subs(), |r| {
                    crate::Message::MeshBus(Message::SubsLoaded(r))
                })
            }

            Message::DndLoaded(result) => {
                self.dnd.loading = false;
                self.dnd.loaded = true;
                match result {
                    Ok((status, qs, qe)) => {
                        self.dnd.quiet_start = qs.clone();
                        self.dnd.quiet_end = qe.clone();
                        self.dnd.quiet_start = qs;
                        self.dnd.quiet_end = qe;
                        self.dnd.status = Some(status);
                        self.dnd.error = None;
                    }
                    Err(e) => {
                        self.dnd.error = Some(e);
                    }
                }
                Task::none()
            }

            Message::DndToggleClicked => {
                self.dnd.saving = true;
                Task::perform(toggle_dnd(), |r| {
                    crate::Message::MeshBus(Message::DndToggleDone(r))
                })
            }

            Message::DndToggleDone(result) => {
                self.dnd.saving = false;
                match result {
                    Ok(()) => {
                        self.dnd.loaded = false;
                        self.dnd.loading = true;
                        Task::perform(load_dnd(), |r| {
                            crate::Message::MeshBus(Message::DndLoaded(r))
                        })
                    }
                    Err(e) => {
                        self.dnd.error = Some(e);
                        Task::none()
                    }
                }
            }

            Message::DndRefreshClicked => {
                self.dnd.loaded = false;
                self.dnd.loading = true;
                Task::perform(load_dnd(), |r| {
                    crate::Message::MeshBus(Message::DndLoaded(r))
                })
            }

            Message::DndQuietStartChanged(s) => {
                self.dnd.quiet_start = s;
                Task::none()
            }

            Message::DndQuietEndChanged(s) => {
                self.dnd.quiet_end = s;
                Task::none()
            }

            Message::DndSaveClicked => {
                self.dnd.saving = true;
                let qs = self.dnd.quiet_start.clone();
                let qe = self.dnd.quiet_end.clone();
                Task::perform(save_quiet_hours(qs, qe), |r| {
                    crate::Message::MeshBus(Message::DndSaveDone(r))
                })
            }

            Message::DndSaveDone(result) => {
                self.dnd.saving = false;
                if let Err(e) = result {
                    self.dnd.error = Some(e);
                }
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, crate::Message> {
        let palette = Palette::dark();
        let density = Density::Comfortable;
        let sizes = FontSize::defaults();
        let radii = Radii::defaults();
        let accent = palette.accent.into_iced_color();
        let raised = palette.raised.into_iced_color();

        let title = text("Mackes Bus")
            .size(TypeRole::Display.size_in(sizes))
            .color(palette.text.into_iced_color());

        let subtitle = text("Per-peer notification distribution · ntfy over Nebula")
            .size(TypeRole::Body.size_in(sizes))
            .color(palette.text_muted.into_iced_color());

        let tab_bar: Element<'_, crate::Message> = {
            let r = f32::from(radii.sm);
            let buttons: Vec<Element<'_, crate::Message>> = Tab::ALL
                .iter()
                .map(|&tab| {
                    let is_active = tab == self.active_tab;
                    let (bg, fg) = if is_active {
                        (accent, Color::WHITE)
                    } else {
                        (Color::TRANSPARENT, palette.text.into_iced_color())
                    };
                    button(
                        text(tab.label())
                            .size(TypeRole::Body.size_in(sizes))
                            .color(fg),
                    )
                    .padding([6u16, 14u16])
                    .style(move |_t, status: ButtonStatus| {
                        let fill = match (is_active, status) {
                            (true, _) => bg,
                            (false, ButtonStatus::Hovered) => Color {
                                r: accent.r,
                                g: accent.g,
                                b: accent.b,
                                a: 0.08,
                            },
                            _ => bg,
                        };
                        button::Style {
                            background: Some(Background::Color(fill)),
                            text_color: fg,
                            border: Border {
                                color: Color::TRANSPARENT,
                                width: 0.0,
                                radius: r.into(),
                            },
                            shadow: iced::Shadow::default(),
                        }
                    })
                    .on_press(crate::Message::MeshBus(Message::SelectTab(tab)))
                    .into()
                })
                .collect();

            row(buttons).spacing(4).into()
        };

        let tab_separator = {
            use iced::widget::container;
            container(Space::new(Length::Fill, Length::Fixed(1.0)))
                .style(move |_t: &iced::Theme| iced::widget::container::Style {
                    background: Some(Background::Color(raised)),
                    ..Default::default()
                })
                .width(Length::Fill)
                .height(Length::Fixed(1.0))
        };

        let body: Element<'_, crate::Message> = match self.active_tab {
            Tab::Topics => empty_state(
                EmptyState::info(
                    "No topics active yet",
                    "Publish a message or start a webhook to create the first topic.",
                )
                .with_icon(Icon::Notification),
                palette,
                || crate::Message::Noop,
            ),
            Tab::Subscriptions => self.view_subscriptions_tab(palette, sizes),
            Tab::Hooks => empty_state(
                EmptyState::info(
                    "No webhook rules configured",
                    "Add a rule to bus-hooks.yaml to route incoming webhook events to topics.",
                )
                .with_icon(Icon::Settings),
                palette,
                || crate::Message::Noop,
            ),
            Tab::Audit => empty_state(
                EmptyState::info(
                    "No audit events recorded",
                    "Bus activity will appear here as messages flow through the broker.",
                )
                .with_icon(Icon::History),
                palette,
                || crate::Message::Noop,
            ),
            Tab::Dnd => self.view_dnd_tab(palette, sizes),
        };

        let header = column![title, subtitle].spacing(4);

        let content = column![
            header,
            Space::with_height(12),
            tab_bar,
            tab_separator,
            Space::with_height(16),
            body,
        ]
        .spacing(0)
        .align_x(alignment::Horizontal::Left);

        panel_container(content.into(), density)
    }

    fn view_subscriptions_tab(
        &self,
        palette: Palette,
        sizes: FontSize,
    ) -> Element<'_, crate::Message> {
        if self.subs.loading {
            return text("Loading…")
                .size(TypeRole::Body.size_in(sizes))
                .color(palette.text_muted.into_iced_color())
                .into();
        }

        if !self.subs.loaded {
            return empty_state(
                EmptyState::info(
                    "No subscriptions configured",
                    "Add a subscription to start receiving messages on this peer.",
                )
                .with_icon(Icon::Network),
                palette,
                || crate::Message::Noop,
            );
        }

        let accent = palette.accent.into_iced_color();
        let radii = Radii::defaults();
        let r = f32::from(radii.sm);

        // — Topic list —
        let list_label = text(format!("Subscriptions ({})", self.subs.topics.len()))
            .size(TypeRole::Subheading.size_in(sizes))
            .color(palette.text.into_iced_color());

        let topic_rows: Vec<Element<'_, crate::Message>> = if self.subs.topics.is_empty() {
            vec![text("No topics subscribed yet.")
                .size(TypeRole::Caption.size_in(sizes))
                .color(palette.text_muted.into_iced_color())
                .into()]
        } else {
            self.subs
                .topics
                .iter()
                .map(|t| {
                    let topic = t.clone();
                    let is_muted = self.subs.muted.contains(&topic);
                    let label_color = if is_muted {
                        palette.text_muted.into_iced_color()
                    } else {
                        palette.text.into_iced_color()
                    };
                    let mute_note: Option<Element<'_, crate::Message>> = if is_muted {
                        Some(
                            text("muted")
                                .size(TypeRole::Caption.size_in(sizes))
                                .color(palette.text_muted.into_iced_color())
                                .into(),
                        )
                    } else {
                        None
                    };
                    let remove_topic = topic.clone();
                    let remove_btn: Element<'_, crate::Message> = button(
                        text("Remove")
                            .size(TypeRole::Caption.size_in(sizes))
                            .color(label_color),
                    )
                    .padding([2u16, 8u16])
                    .style(move |_t, _s: ButtonStatus| button::Style {
                        background: Some(Background::Color(Color {
                            r: 0.8,
                            g: 0.1,
                            b: 0.1,
                            a: 0.12,
                        })),
                        text_color: Color { r: 0.9, g: 0.2, b: 0.2, a: 1.0 },
                        border: Border {
                            color: Color::TRANSPARENT,
                            width: 0.0,
                            radius: r.into(),
                        },
                        shadow: iced::Shadow::default(),
                    })
                    .on_press(crate::Message::MeshBus(Message::SubsRemoveClicked(
                        remove_topic,
                    )))
                    .into();

                    let mut row_items: Vec<Element<'_, crate::Message>> = vec![
                        text(t.as_str())
                            .size(TypeRole::Body.size_in(sizes))
                            .color(label_color)
                            .into(),
                    ];
                    if let Some(mn) = mute_note {
                        row_items.push(Space::with_width(8).into());
                        row_items.push(mn);
                    }
                    row_items.push(Space::with_width(Length::Fill).into());
                    row_items.push(remove_btn);

                    row(row_items)
                        .align_y(iced::Alignment::Center)
                        .into()
                })
                .collect()
        };

        let topic_list: Element<'_, crate::Message> = column(topic_rows).spacing(6).into();

        // — Add topic row —
        let add_input: Element<'_, crate::Message> =
            text_input("fleet/# or mon/+/alerts", &self.subs.new_topic)
                .on_input(|s| crate::Message::MeshBus(Message::SubsNewTopicChanged(s)))
                .on_submit(crate::Message::MeshBus(Message::SubsAddClicked))
                .width(Length::Fixed(240.0))
                .into();

        let add_btn: Element<'_, crate::Message> = button(
            text("Subscribe")
                .size(TypeRole::Body.size_in(sizes))
                .color(Color::WHITE),
        )
        .padding([6u16, 14u16])
        .style(move |_t, _s: ButtonStatus| button::Style {
            background: Some(Background::Color(accent)),
            text_color: Color::WHITE,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: r.into(),
            },
            shadow: iced::Shadow::default(),
        })
        .on_press(crate::Message::MeshBus(Message::SubsAddClicked))
        .into();

        let add_row: Element<'_, crate::Message> = row![
            add_input,
            Space::with_width(8),
            add_btn,
        ]
        .align_y(iced::Alignment::Center)
        .into();

        // — Match @peer section —
        let peer_label = text("Copy from peer")
            .size(TypeRole::Subheading.size_in(sizes))
            .color(palette.text.into_iced_color());

        let peer_hint = text(
            "Copies all subscriptions from another peer's subs.yaml via mesh storage.",
        )
        .size(TypeRole::Caption.size_in(sizes))
        .color(palette.text_muted.into_iced_color());

        let peer_input: Element<'_, crate::Message> =
            text_input("hostname", &self.subs.peer_input)
                .on_input(|s| crate::Message::MeshBus(Message::SubsPeerInputChanged(s)))
                .on_submit(crate::Message::MeshBus(Message::SubsMatchPeerClicked))
                .width(Length::Fixed(160.0))
                .into();

        let match_btn: Element<'_, crate::Message> = button(
            text("Match @peer")
                .size(TypeRole::Body.size_in(sizes))
                .color(palette.text.into_iced_color()),
        )
        .padding([6u16, 14u16])
        .style(move |_t, _s: ButtonStatus| button::Style {
            background: Some(Background::Color(palette.raised.into_iced_color())),
            text_color: palette.text.into_iced_color(),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: r.into(),
            },
            shadow: iced::Shadow::default(),
        })
        .on_press(crate::Message::MeshBus(Message::SubsMatchPeerClicked))
        .into();

        let peer_row: Element<'_, crate::Message> = row![
            peer_input,
            Space::with_width(8),
            match_btn,
        ]
        .align_y(iced::Alignment::Center)
        .into();

        // — Error display —
        let error_row: Option<Element<'_, crate::Message>> =
            self.subs.error.as_deref().map(|e| {
                text(format!("Error: {e}"))
                    .size(TypeRole::Caption.size_in(sizes))
                    .color(Color { r: 0.9, g: 0.2, b: 0.2, a: 1.0 })
                    .into()
            });

        let mut col = column![
            list_label,
            Space::with_height(8),
            topic_list,
            Space::with_height(16),
            add_row,
            Space::with_height(28),
            peer_label,
            Space::with_height(4),
            peer_hint,
            Space::with_height(8),
            peer_row,
        ]
        .spacing(0);

        if let Some(err) = error_row {
            col = col.push(Space::with_height(12)).push(err);
        }

        col.into()
    }

    fn view_dnd_tab(
        &self,
        palette: Palette,
        sizes: FontSize,
    ) -> Element<'_, crate::Message> {
        if self.dnd.loading {
            return text("Loading…")
                .size(TypeRole::Body.size_in(sizes))
                .color(palette.text_muted.into_iced_color())
                .into();
        }

        let accent = palette.accent.into_iced_color();
        let radii = Radii::defaults();
        let r = f32::from(radii.sm);

        // — DND master toggle —
        let (active, since_label, peer_label) = match &self.dnd.status {
            Some(s) => {
                let since = if s.since_unix_ms > 0 {
                    format!("since {}", s.since_unix_ms / 1000)
                } else {
                    String::new()
                };
                let by = if s.set_by_peer.is_empty() {
                    String::new()
                } else {
                    format!("by @{}", s.set_by_peer)
                };
                (s.active, since, by)
            }
            None => (false, String::new(), String::new()),
        };

        let toggle_label = if active { "DND On" } else { "DND Off" };
        let toggle_bg = if active { accent } else { palette.raised.into_iced_color() };
        let toggle_fg = if active { Color::WHITE } else { palette.text.into_iced_color() };

        let toggle_btn: Element<'_, crate::Message> = button(
            text(toggle_label)
                .size(TypeRole::Body.size_in(sizes))
                .color(toggle_fg),
        )
        .padding([8u16, 20u16])
        .style(move |_t, _s: ButtonStatus| button::Style {
            background: Some(Background::Color(toggle_bg)),
            text_color: toggle_fg,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: r.into(),
            },
            shadow: iced::Shadow::default(),
        })
        .on_press(crate::Message::MeshBus(Message::DndToggleClicked))
        .into();

        let meta_parts: Vec<&str> = [since_label.as_str(), peer_label.as_str()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect();
        let meta_str = meta_parts.join(" · ");

        let toggle_row: Element<'_, crate::Message> = if meta_str.is_empty() {
            toggle_btn
        } else {
            row![
                toggle_btn,
                Space::with_width(12),
                text(meta_str)
                    .size(TypeRole::Caption.size_in(sizes))
                    .color(palette.text_muted.into_iced_color()),
            ]
            .align_y(iced::Alignment::Center)
            .into()
        };

        // — Quiet hours editor —
        let quiet_label = text("Global quiet window")
            .size(TypeRole::Subheading.size_in(sizes))
            .color(palette.text.into_iced_color());

        let quiet_hint = text("Messages delivered outside this window only. Leave blank to deliver around the clock.")
            .size(TypeRole::Caption.size_in(sizes))
            .color(palette.text_muted.into_iced_color());

        let start_input: Element<'_, crate::Message> = text_input("22:00", &self.dnd.quiet_start)
            .on_input(|s| crate::Message::MeshBus(Message::DndQuietStartChanged(s)))
            .width(Length::Fixed(80.0))
            .into();

        let end_input: Element<'_, crate::Message> = text_input("07:00", &self.dnd.quiet_end)
            .on_input(|s| crate::Message::MeshBus(Message::DndQuietEndChanged(s)))
            .width(Length::Fixed(80.0))
            .into();

        let save_bg = accent;
        let save_fg = Color::WHITE;
        let save_btn: Element<'_, crate::Message> = button(
            text(if self.dnd.saving { "Applying…" } else { "Apply" })
                .size(TypeRole::Body.size_in(sizes))
                .color(save_fg),
        )
        .padding([6u16, 16u16])
        .style(move |_t, _s: ButtonStatus| button::Style {
            background: Some(Background::Color(save_bg)),
            text_color: save_fg,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: r.into(),
            },
            shadow: iced::Shadow::default(),
        })
        .on_press(crate::Message::MeshBus(Message::DndSaveClicked))
        .into();

        let quiet_row: Element<'_, crate::Message> = row![
            start_input,
            Space::with_width(8),
            text("→")
                .size(TypeRole::Body.size_in(sizes))
                .color(palette.text_muted.into_iced_color()),
            Space::with_width(8),
            end_input,
            Space::with_width(12),
            save_btn,
        ]
        .align_y(iced::Alignment::Center)
        .into();

        // — Active snoozes —
        let snooze_count = self
            .dnd
            .status
            .as_ref()
            .map(|s| s.snoozes.len())
            .unwrap_or(0);

        let snooze_label = text(format!("Active fleet snoozes ({snooze_count})"))
            .size(TypeRole::Subheading.size_in(sizes))
            .color(palette.text.into_iced_color());

        let snooze_body: Element<'_, crate::Message> = if snooze_count == 0 {
            text("No active snoozes — use `mde-bus mute <topic> --duration <D>` to snooze.")
                .size(TypeRole::Caption.size_in(sizes))
                .color(palette.text_muted.into_iced_color())
                .into()
        } else {
            let rows: Vec<Element<'_, crate::Message>> = self
                .dnd
                .status
                .as_ref()
                .map(|s| &s.snoozes)
                .unwrap_or(&vec![])
                .iter()
                .map(|sn| {
                    let by = if sn.set_by_peer.is_empty() {
                        String::new()
                    } else {
                        format!(" (by @{})", sn.set_by_peer)
                    };
                    text(format!("{}{}", sn.topic, by))
                        .size(TypeRole::Caption.size_in(sizes))
                        .color(palette.text.into_iced_color())
                        .into()
                })
                .collect();
            column(rows).spacing(4).into()
        };

        // — Error display —
        let error_row: Option<Element<'_, crate::Message>> =
            self.dnd.error.as_deref().map(|e| {
                text(format!("Error: {e}"))
                    .size(TypeRole::Caption.size_in(sizes))
                    .color(Color { r: 0.9, g: 0.2, b: 0.2, a: 1.0 })
                    .into()
            });

        let mut col = column![
            toggle_row,
            Space::with_height(20),
            quiet_label,
            Space::with_height(4),
            quiet_hint,
            Space::with_height(8),
            quiet_row,
            Space::with_height(24),
            snooze_label,
            Space::with_height(8),
            snooze_body,
        ]
        .spacing(0);

        if let Some(err) = error_row {
            col = col.push(Space::with_height(12)).push(err);
        }

        col.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_tab_is_topics() {
        let panel = MeshBusPanel::new();
        assert_eq!(panel.active_tab, Tab::Topics);
    }

    #[test]
    fn select_tab_updates_active() {
        let mut panel = MeshBusPanel::new();
        let _ = panel.update(Message::SelectTab(Tab::Subscriptions));
        assert_eq!(panel.active_tab, Tab::Subscriptions);
        let _ = panel.update(Message::SelectTab(Tab::Dnd));
        assert_eq!(panel.active_tab, Tab::Dnd);
    }

    #[test]
    fn all_tabs_cycle_without_panic() {
        let mut panel = MeshBusPanel::new();
        for tab in Tab::ALL {
            let _ = panel.update(Message::SelectTab(tab));
            assert_eq!(panel.active_tab, tab);
        }
    }

    #[test]
    fn tab_labels_are_non_empty() {
        for tab in Tab::ALL {
            assert!(!tab.label().is_empty());
        }
    }

    #[test]
    fn five_tabs_declared() {
        assert_eq!(Tab::ALL.len(), 5);
    }

    #[test]
    fn dnd_not_loaded_on_topics_tab() {
        let mut panel = MeshBusPanel::new();
        // Switching to Topics does not trigger a DND load.
        let _ = panel.update(Message::SelectTab(Tab::Topics));
        assert!(!panel.dnd.loaded);
        assert!(!panel.dnd.loading);
    }

    #[test]
    fn dnd_loading_set_on_dnd_tab_switch() {
        let mut panel = MeshBusPanel::new();
        // Switching to DND triggers loading (returns a Task::perform).
        let _ = panel.update(Message::SelectTab(Tab::Dnd));
        assert!(panel.dnd.loading);
        assert!(!panel.dnd.loaded);
    }

    #[test]
    fn dnd_loaded_ok_populates_state() {
        let mut panel = MeshBusPanel::new();
        let status = DndStatusJson {
            active: true,
            since_unix_ms: 1_700_000_000_000,
            set_by_peer: "desktop-2".to_string(),
            snoozes: vec![],
        };
        let _ = panel.update(Message::DndLoaded(Ok((
            status,
            "22:00".to_string(),
            "07:00".to_string(),
        ))));
        let s = panel.dnd.status.as_ref().unwrap();
        assert!(s.active);
        assert_eq!(s.set_by_peer, "desktop-2");
        assert_eq!(panel.dnd.quiet_start, "22:00");
        assert_eq!(panel.dnd.quiet_end, "07:00");
        assert!(panel.dnd.loaded);
        assert!(!panel.dnd.loading);
        assert!(panel.dnd.error.is_none());
    }

    #[test]
    fn dnd_loaded_err_sets_error() {
        let mut panel = MeshBusPanel::new();
        let _ =
            panel.update(Message::DndLoaded(Err("mde-bus not found".to_string())));
        assert!(panel.dnd.error.is_some());
        assert!(panel.dnd.status.is_none());
        assert!(panel.dnd.loaded);
    }

    #[test]
    fn quiet_hours_inputs_update_state() {
        let mut panel = MeshBusPanel::new();
        let _ = panel.update(Message::DndQuietStartChanged("23:00".to_string()));
        assert_eq!(panel.dnd.quiet_start, "23:00");
        let _ = panel.update(Message::DndQuietEndChanged("06:00".to_string()));
        assert_eq!(panel.dnd.quiet_end, "06:00");
    }

    #[test]
    fn toggle_sets_saving_flag() {
        let mut panel = MeshBusPanel::new();
        let _ = panel.update(Message::DndToggleClicked);
        assert!(panel.dnd.saving);
    }

    // ─── Subscriptions tab tests ──────────────────────────────────────────────

    #[test]
    fn subs_loading_set_on_subscriptions_tab_switch() {
        let mut panel = MeshBusPanel::new();
        let _ = panel.update(Message::SelectTab(Tab::Subscriptions));
        assert!(panel.subs.loading);
        assert!(!panel.subs.loaded);
    }

    #[test]
    fn subs_not_loaded_on_topics_tab() {
        let mut panel = MeshBusPanel::new();
        let _ = panel.update(Message::SelectTab(Tab::Topics));
        assert!(!panel.subs.loaded);
        assert!(!panel.subs.loading);
    }

    #[test]
    fn subs_loaded_ok_populates_state() {
        let mut panel = MeshBusPanel::new();
        let topics = vec!["fleet/#".to_string(), "mon/+/alerts".to_string()];
        let muted = vec!["fleet/debug".to_string()];
        let _ = panel.update(Message::SubsLoaded(Ok((topics.clone(), muted.clone()))));
        assert_eq!(panel.subs.topics, topics);
        assert_eq!(panel.subs.muted, muted);
        assert!(panel.subs.loaded);
        assert!(!panel.subs.loading);
        assert!(panel.subs.error.is_none());
    }

    #[test]
    fn subs_loaded_err_sets_error() {
        let mut panel = MeshBusPanel::new();
        let _ = panel.update(Message::SubsLoaded(Err("no data dir".to_string())));
        assert!(panel.subs.error.is_some());
        assert!(panel.subs.topics.is_empty());
        assert!(panel.subs.loaded);
    }

    #[test]
    fn subs_new_topic_changed_updates_state() {
        let mut panel = MeshBusPanel::new();
        let _ = panel.update(Message::SubsNewTopicChanged("gh/#".to_string()));
        assert_eq!(panel.subs.new_topic, "gh/#");
    }

    #[test]
    fn subs_add_clears_input_and_triggers_task() {
        let mut panel = MeshBusPanel::new();
        panel.subs.new_topic = "gh/#".to_string();
        let _ = panel.update(Message::SubsAddClicked);
        // Input cleared immediately.
        assert!(panel.subs.new_topic.is_empty());
    }

    #[test]
    fn subs_add_noop_on_empty_input() {
        let mut panel = MeshBusPanel::new();
        panel.subs.new_topic = String::new();
        let _ = panel.update(Message::SubsAddClicked);
        // No state change — still not loading.
        assert!(!panel.subs.loading);
    }

    #[test]
    fn subs_peer_input_changed_updates_state() {
        let mut panel = MeshBusPanel::new();
        let _ = panel.update(Message::SubsPeerInputChanged("desktop-2".to_string()));
        assert_eq!(panel.subs.peer_input, "desktop-2");
    }

    #[test]
    fn subs_remove_done_error_sets_error() {
        let mut panel = MeshBusPanel::new();
        let _ = panel.update(Message::SubsRemoveDone(Err("failed".to_string())));
        assert!(panel.subs.error.is_some());
    }

    #[test]
    fn subs_match_peer_done_empty_sets_info_error() {
        let mut panel = MeshBusPanel::new();
        let _ = panel.update(Message::SubsMatchPeerDone(Ok(vec![])));
        assert!(panel.subs.error.is_some());
        assert!(panel.subs.loaded);
    }
}
