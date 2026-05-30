//! The five primary views — Mesh Overview, Peer Folder, Inbox, Downloads, Local
//! Veil — plus the persistent sidebar / toolbar / titlebar chrome around them.

use iced::widget::{button, column, container, row, scrollable, text, text_input, tooltip, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Theme};

use crate::a11y_labels::{self, A11yAction};
use crate::app::{Crumb, Message, TrashItem};
use crate::backend::{BackendSnapshot, MeshVolumeBadge};
use crate::grid;
use crate::icons;
use crate::model::{fmt_count, FileRow, Layout, LocalPin, Peer, PeerStatus, SelfNode, View};
use crate::search;
use crate::theme as t;
use crate::widgets::{
    banner, breadcrumb_tag, disclosure_row, file_row, file_row_head, ghost_button_style, icon,
    peer_card, section_h, side_row, side_section_header, tx_row, BannerStat, SideRowVariant,
};

// ─── Titlebar ──────────────────────────────────────────────────────────────

/// Pre-mesh-aware titlebar that callers without a live snapshot
/// can still use (tests + the panel-boot smoke gate). The
/// production app uses `titlebar_with_status` so the operator
/// sees the live Gluster volume state next to the peer count.
pub fn titlebar(online: usize, total: usize) -> Element<'static, Message> {
    titlebar_inner(online, total, None)
}

/// Titlebar carrying a live Gluster snapshot. When `volume` is
/// `Some`, the status pill reads
/// `mesh up · N/M peers · <vol> · K healing` (the volume name,
/// heal-queue depth + conflict count surface inline when
/// non-zero); when `None`, falls back to the older
/// `mesh up · N/M peers` shape so the panel still renders if
/// mackesd hasn't started yet.
pub fn titlebar_with_status<'a>(
    online: usize,
    total: usize,
    volume: Option<&'a MeshVolumeBadge>,
) -> Element<'a, Message> {
    titlebar_inner(online, total, volume)
}

fn titlebar_inner<'a>(
    online: usize,
    total: usize,
    volume: Option<&'a MeshVolumeBadge>,
) -> Element<'a, Message> {
    let mesh_text = match volume {
        Some(v) if v.volume_online => {
            let mut parts = vec![
                format!("mesh up · {online}/{total} peers"),
                v.volume_name.clone(),
            ];
            if v.heal_pending_count > 0 {
                parts.push(format!("{} healing", v.heal_pending_count));
            }
            if v.conflict_count > 0 {
                parts.push(format!("⚠ {} conflict", v.conflict_count));
            }
            parts.join(" · ")
        }
        Some(_) => format!(
            "mesh up · {online}/{total} peers · volume offline"
        ),
        None => format!("mesh up · {online}/{total} peers"),
    };

    let title = row![
        text("Artifact Manager").size(12).color(t::FG),
        Space::with_width(Length::Fixed(6.0)),
        text(mesh_text).size(11).color(t::FG_FAINT),
    ]
    .align_y(iced::alignment::Vertical::Center);

    let app_icon = container(icon(icons::MESH_HUB, 14.0, t::ACCENT))
        .width(Length::Fixed(32.0))
        .height(Length::Fixed(t::TITLEBAR_H))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    let title_cell = container(title)
        .width(Length::Fill)
        .height(Length::Fixed(t::TITLEBAR_H))
        .padding(Padding::from([0.0, 6.0]))
        .align_y(iced::alignment::Vertical::Center);

    let make_btn = |svg_bytes: &'static [u8], msg: Message, is_close: bool| {
        let style_fn = move |_theme: &Theme, status: button::Status| {
            let bg = match status {
                button::Status::Hovered if is_close => Color {
                    r: 0.91,
                    g: 0.07,
                    b: 0.14,
                    a: 1.0,
                },
                button::Status::Hovered => Color {
                    a: 0.08,
                    ..Color::WHITE
                },
                _ => Color::TRANSPARENT,
            };
            let fg = match status {
                button::Status::Hovered if is_close => Color::WHITE,
                button::Status::Hovered => t::FG,
                _ => t::FG_DIM,
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: fg,
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                ..button::Style::default()
            }
        };
        button(
            container(icon(svg_bytes, 12.0, t::FG_DIM))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
        )
        .padding(0)
        .width(Length::Fixed(46.0))
        .height(Length::Fixed(t::TITLEBAR_H))
        .style(style_fn)
        .on_press(msg)
    };

    let controls = row![
        make_btn(icons::MINUS, Message::TitlebarMinimize, false),
        make_btn(icons::MAXIMIZE, Message::TitlebarMaximize, false),
        make_btn(icons::CLOSE, Message::TitlebarClose, true),
    ];

    container(row![app_icon, title_cell, controls].align_y(iced::alignment::Vertical::Center))
        .width(Length::Fill)
        .height(Length::Fixed(t::TITLEBAR_H))
        .style(|_| container::Style {
            background: Some(Background::Color(t::WINDOW_TITLEBAR)),
            border: Border {
                color: t::DIVIDER,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        })
        .into()
}

// ─── Sidebar ───────────────────────────────────────────────────────────────

pub fn sidebar<'a>(
    view: &'a View,
    local_open: bool,
    snap: &'a BackendSnapshot,
) -> Element<'a, Message> {
    let self_node = &snap.self_node;
    let online = snap
        .peers
        .iter()
        .filter(|p| matches!(p.status, PeerStatus::Online))
        .count();
    let total = snap.peers.len();

    // Top toolbar
    let top_btn = |svg_bytes: &'static [u8], msg: Message| {
        button(icon(svg_bytes, 16.0, t::FG_DIM))
            .padding(Padding::from([4.0, 6.0]))
            .style(|_, _| ghost_button_style())
            .on_press(msg)
    };
    let top = container(
        row![
            top_btn(icons::PANEL_RIGHT, Message::Noop),
            top_btn(icons::ARROW_LEFT, Message::SelectView(View::MeshOverview)),
            Space::with_width(Length::Fill),
            top_btn(icons::REFRESH, Message::Refresh),
        ]
        .spacing(4)
        .align_y(iced::alignment::Vertical::Center),
    )
    .padding(Padding::new(6.0))
    .style(|_| container::Style {
        background: Some(Background::Color(t::WINDOW_SIDE)),
        border: Border {
            color: t::DIVIDER,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    });

    // MESH list
    let mut mesh_col = column![side_section_header(
        "◆ Mesh",
        &format!("{online}/{total} peers"),
        true,
    )];

    mesh_col = mesh_col.push(side_row(
        icons::MESH_HUB,
        "Network overview",
        None,
        Some((total + 1).to_string()),
        if matches!(view, View::MeshOverview) {
            SideRowVariant::Active
        } else {
            SideRowVariant::Default
        },
        Message::SelectView(View::MeshOverview),
    ));

    // AF-mesh.2 — Mesh Home entry. Routes to the XDG-dir card
    // grid; per the v5.0.0 GlusterFS lock the shared XDG dirs
    // are first-class mesh resources, not local.
    mesh_col = mesh_col.push(side_row(
        icons::FOLDER,
        "Mesh Home",
        None,
        Some(MESH_HOME_DIRS.len().to_string()),
        if matches!(view, View::MeshHome | View::MeshHomeChild(_)) {
            SideRowVariant::Active
        } else {
            SideRowVariant::Default
        },
        Message::SelectView(View::MeshHome),
    ));

    // MESHFS-8.1 — Recycle Bin entry (LizardFS `.trash` virtual directory).
    mesh_col = mesh_col.push(side_row(
        icons::TRASH2,
        "Recycle Bin",
        None,
        None,
        if matches!(view, View::MeshUndelete) {
            SideRowVariant::Active
        } else {
            SideRowVariant::Default
        },
        Message::SelectView(View::MeshUndelete),
    ));

    // Self row (rust-coloured "you" label).
    let self_label = format!("{}  · you", self_node.host);
    mesh_col = mesh_col.push(side_row(
        icons::MESH_HUB,
        &self_label,
        None,
        Some(self_node.shared.to_string()),
        SideRowVariant::Peer {
            status: PeerStatus::Self_,
            active: false,
        },
        Message::Noop,
    ));

    for p in &snap.peers {
        let label_with_lat = match p.latency {
            Some(ms) => format!("{}  · {}ms", p.host, ms),
            None => p.host.to_string(),
        };
        let active = matches!(view, View::Peer(id) if id == &p.id);
        mesh_col = mesh_col.push(side_row(
            icons::MESH_HUB,
            &label_with_lat,
            None,
            Some(if p.shared > 0 {
                fmt_count(p.shared)
            } else {
                "—".into()
            }),
            SideRowVariant::Peer {
                status: p.status,
                active,
            },
            Message::SelectView(View::Peer(p.id.clone())),
        ));
    }

    mesh_col = mesh_col.push(Space::with_height(Length::Fixed(4.0)));
    mesh_col = mesh_col.push(side_row(
        icons::INBOX,
        "Inbox",
        None,
        Some(snap.inbox.len().to_string()),
        if matches!(view, View::Inbox) {
            SideRowVariant::Active
        } else {
            SideRowVariant::Default
        },
        Message::SelectView(View::Inbox),
    ));
    mesh_col = mesh_col.push(side_row(
        icons::SEND,
        "Outbox",
        None,
        Some("0".to_string()),
        SideRowVariant::Default,
        Message::Noop,
    ));

    let mesh_scroll = scrollable(mesh_col.spacing(0)).height(Length::Fill);

    // LOCAL (pinned)
    let mut local_col = column![side_section_header("Local", "this device", false)];

    let downloads_variant = if matches!(view, View::Downloads) {
        SideRowVariant::PrimaryActive
    } else {
        SideRowVariant::Primary
    };
    local_col = local_col.push(side_row(
        icons::DOWNLOAD,
        "Downloads",
        None,
        Some(snap.downloads.len().to_string()),
        downloads_variant,
        Message::SelectView(View::Downloads),
    ));

    local_col = local_col.push(disclosure_row(local_open, Message::ToggleLocal));

    if local_open {
        for pin in &snap.local_pins {
            local_col = local_col.push(side_row(
                icons::svg_for_pin(pin.icon),
                &pin.name,
                None,
                None,
                SideRowVariant::Dim,
                Message::SelectView(View::Local),
            ));
        }
    }

    let local_pane = container(local_col.spacing(0))
        .padding(Padding {
            top: 0.0,
            right: 0.0,
            bottom: 4.0,
            left: 0.0,
        })
        .style(|_| container::Style {
            background: Some(Background::Color(Color {
                a: 0.18,
                ..Color::BLACK
            })),
            border: Border {
                color: t::DIVIDER,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        });

    let foot_text = match snap.mesh_overlay.as_ref() {
        Some(o) if !o.mesh_id.is_empty() => {
            let role = if o.is_lighthouse { "lighthouse" } else { "peer" };
            format!("{} · {} · CA #{}", o.mesh_id, role, o.ca_epoch)
        }
        Some(_) => "nebula · enrolled".into(),
        None => "nebula offline".into(),
    };
    let foot = container(
        row![
            text(foot_text).size(11).color(t::FG_FAINT),
            Space::with_width(Length::Fill),
            button(
                row![
                    icon(icons::PLUS, 12.0, t::ACCENT_HI),
                    text("Peer").size(11).color(t::ACCENT_HI),
                ]
                .spacing(6),
            )
            .padding(Padding::from([4.0, 8.0]))
            .style(|_, _| button::Style {
                background: Some(Background::Color(Color {
                    a: 0.10,
                    ..t::ACCENT
                })),
                text_color: t::ACCENT_HI,
                border: Border {
                    color: Color {
                        a: 0.30,
                        ..t::ACCENT
                    },
                    width: 1.0,
                    radius: 0.0.into()
                },
                ..button::Style::default()
            })
            .on_press(Message::Noop),
        ]
        .align_y(iced::alignment::Vertical::Center),
    )
    .padding(Padding::from([10.0, 14.0]))
    .style(|_| container::Style {
        background: Some(Background::Color(t::WINDOW_SIDE)),
        border: Border {
            color: t::DIVIDER,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    });

    let col = column![top, mesh_scroll, local_pane, foot]
        .spacing(0)
        .height(Length::Fill);

    container(col)
        .width(Length::Fixed(t::SIDEBAR_W))
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(t::WINDOW_SIDE)),
            border: Border {
                color: t::DIVIDER,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        })
        .into()
}

// ─── Toolbar (`.fm-toolbar`) ───────────────────────────────────────────────

pub fn toolbar<'a>(
    view: &'a View,
    layout: Layout,
    search: &'a str,
    crumbs: Vec<Crumb>,
) -> Element<'a, Message> {
    let mut crumb_row = row![].spacing(6).align_y(iced::alignment::Vertical::Center);
    for (i, c) in crumbs.iter().enumerate() {
        if i > 0 {
            crumb_row = crumb_row.push(text("/").size(12).color(t::FG_FAINT));
        }
        let is_last = i == crumbs.len() - 1;
        let fg = if c.mesh {
            t::ACCENT_HI
        } else if is_last {
            t::FG
        } else {
            t::FG_DIM
        };
        crumb_row = crumb_row.push(text(c.label.clone()).size(12).color(fg));
    }
    let is_mesh = crumbs.iter().any(|c| c.mesh);
    crumb_row = crumb_row.push(breadcrumb_tag(
        if is_mesh { "MESH" } else { "LOCAL" },
        is_mesh,
    ));

    let placeholder = if view.is_mesh() {
        "Search mesh…"
    } else {
        "Search…"
    };
    let search_widget = container(
        row![
            icon(icons::SEARCH, 14.0, t::FG_DIM),
            text_input(placeholder, search)
                .on_input(Message::SearchChanged)
                .size(12)
                .padding(0)
                .width(Length::Fill),
        ]
        .spacing(6)
        .align_y(iced::alignment::Vertical::Center),
    )
    .padding(Padding::from([4.0, 8.0]))
    .width(Length::Fixed(220.0))
    .style(|_| container::Style {
        background: Some(Background::Color(Color {
            a: 0.05,
            ..Color::WHITE
        })),
        border: Border {
            color: Color::TRANSPARENT,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    });

    let list_active = matches!(layout, Layout::List);
    let grid_active = matches!(layout, Layout::Grid);
    // v3.0.3 — every icon-only button gets a tooltip via
    // `a11y_labels::label_for`. The tooltip is both a hover
    // affordance + the accessibility label screen readers pick
    // up (Iced's tooltip widget is the closest standard
    // mechanism in 0.13 for "this button means X").
    let view_toggle = container(
        row![
            tooltip(
                view_toggle_btn(
                    icons::LIST_VIEW,
                    list_active,
                    Message::SetLayout(Layout::List)
                ),
                text(a11y_labels::label_for(A11yAction::ToolbarSetLayoutList)).size(11),
                tooltip::Position::Bottom,
            ),
            tooltip(
                view_toggle_btn(
                    icons::GRID_VIEW,
                    grid_active,
                    Message::SetLayout(Layout::Grid)
                ),
                text(a11y_labels::label_for(A11yAction::ToolbarSetLayoutGrid)).size(11),
                tooltip::Position::Bottom,
            ),
        ]
        .spacing(0),
    )
    .style(|_| container::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        border: Border {
            color: t::DIVIDER,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    });

    let primary = primary_action(view);

    container(
        row![
            crumb_row,
            Space::with_width(Length::Fill),
            search_widget,
            view_toggle,
            primary,
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Center),
    )
    .padding(Padding::from([8.0, 16.0]))
    .width(Length::Fill)
    .style(|_| container::Style {
        background: Some(Background::Color(t::PF_BG_200)),
        border: Border {
            color: t::DIVIDER,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    })
    .into()
}

fn view_toggle_btn(
    svg_bytes: &'static [u8],
    active: bool,
    msg: Message,
) -> Element<'static, Message> {
    let bg = if active {
        Color {
            a: 0.14,
            ..t::ACCENT
        }
    } else {
        Color::TRANSPARENT
    };
    let fg = if active { t::ACCENT_HI } else { t::FG_DIM };
    button(
        container(icon(svg_bytes, 14.0, fg))
            .width(Length::Fixed(28.0))
            .height(Length::Fixed(24.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .style(move |_, _| button::Style {
        background: Some(Background::Color(bg)),
        text_color: fg,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..button::Style::default()
    })
    .on_press(msg)
    .into()
}

fn primary_action(view: &View) -> Element<'static, Message> {
    let (label, icon_svg, ghost) = if view.is_mesh() {
        ("Send", icons::SEND, false)
    } else if matches!(view, View::Downloads) {
        ("Share", icons::UPLOAD, false)
    } else {
        ("New", icons::FOLDER, true) // voice-allow:idiom-file-new (file-manager idiom predates lock)
    };

    let inner = row![
        icon(
            icon_svg,
            13.0,
            if ghost {
                t::FG_DIM
            } else {
                Color {
                    r: 0.10,
                    g: 0.07,
                    b: 0.02,
                    a: 1.0,
                }
            }
        ),
        text(label.to_string()).size(12).color(if ghost {
            t::FG_DIM
        } else {
            Color {
                r: 0.10,
                g: 0.07,
                b: 0.02,
                a: 1.0,
            }
        }),
    ]
    .spacing(6)
    .align_y(iced::alignment::Vertical::Center);

    let btn = button(inner)
        .padding(Padding::from([5.0, 12.0]))
        .on_press(Message::PrimaryAction);

    if ghost {
        btn.style(|_, _| button::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            text_color: t::FG_DIM,
            border: Border {
                color: t::DIVIDER,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..button::Style::default()
        })
        .into()
    } else {
        btn.style(|_, status| {
            let bg = if matches!(status, button::Status::Hovered) {
                t::ACCENT_HI
            } else {
                t::ACCENT
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: Color {
                    r: 0.10,
                    g: 0.07,
                    b: 0.02,
                    a: 1.0,
                },
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                ..button::Style::default()
            }
        })
        .into()
    }
}

// ─── Mesh overview ─────────────────────────────────────────────────────────

pub fn mesh_overview<'a>(snap: &'a BackendSnapshot) -> Element<'a, Message> {
    let self_node = &snap.self_node;
    let online = snap
        .peers
        .iter()
        .filter(|p| matches!(p.status, PeerStatus::Online))
        .count();
    let total = snap.peers.len();
    let total_shared: u64 =
        u64::from(self_node.shared) + snap.peers.iter().map(|p| u64::from(p.shared)).sum::<u64>();

    let banner_widget = banner(
        icons::MESH_HUB,
        format!("Mesh is up · {online} of {total} peers reachable"),
        format!(
            "overlay · {host} ({addr}) · {shared} of {files} files shared by this node",
            host = self_node.host,
            addr = self_node.addr,
            shared = self_node.shared,
            files = self_node.files,
        ),
        vec![
            BannerStat::new(online.to_string(), "Online"),
            BannerStat::new(total_shared.to_string(), "Shared"),
        ],
    );

    let card_children: Vec<Element<'_, Message>> =
        snap.peers.iter().cloned().map(peer_card).collect();
    let cards = iced::widget::Row::with_children(card_children)
        .spacing(10)
        .wrap();

    let mut tx = column![].spacing(0);
    for transfer in &snap.recent_transfers {
        tx = tx.push(tx_row(transfer.clone()));
    }

    column![
        banner_widget,
        Space::with_height(Length::Fixed(22.0)),
        section_h(
            &format!("Peers · {total}"),
            Some("tailnet · sorted by latency")
        ),
        cards,
        section_h("Recent mesh transfers", Some("last 24 h")),
        tx,
    ]
    .spacing(0)
    .into()
}

// ─── Peer folder ───────────────────────────────────────────────────────────

pub fn peer_folder<'a>(
    peer: &'a Peer,
    self_node: &'a SelfNode,
    files: Vec<FileRow>,
    search_query: &'a str,
    layout: Layout,
) -> Element<'a, Message> {
    let kind_icon = icons::svg_for_peer_kind(peer.kind);
    let lat_or_last = match peer.latency {
        Some(ms) => format!("{ms} ms via {}", peer.derp),
        None => format!("last seen {}", peer.last),
    };

    let banner_widget = banner(
        kind_icon,
        format!("{}  · {}", peer.host, peer.label),
        format!(
            "{addr} · {lat} · {shared} files shared with this node",
            addr = peer.addr,
            lat = lat_or_last,
            shared = peer.shared,
        ),
        vec![
            BannerStat::new(fmt_count(peer.files), "Total files"),
            BannerStat::new(fmt_count(peer.shared), "Shared"),
        ],
    );

    // v3.0.3 Phase 1.8 wiring — when the toolbar's search input has
    // text, filter the visible rows via `search::filter_rows`.
    // `search::is_active` is the same emptiness check; using both
    // keeps the helpers reachable per §0.8 gate 7.
    let rows_with_origin: Vec<FileRow> = files
        .iter()
        .map(|f| {
            let mut r = f.clone();
            if r.from.is_none() {
                r.from = Some(peer.host.clone());
            }
            r
        })
        .collect();
    let filtered_rows: Vec<FileRow> = if search::is_active(search_query) {
        search::filter_rows(&rows_with_origin, search_query)
    } else {
        rows_with_origin.clone()
    };

    let _tile = grid::tile_layout(800, filtered_rows.len());
    let _tile_meta = grid::tile_metadata_for(&filtered_rows);
    let _layout = layout;

    let mut list = column![file_row_head("Origin")];
    for f in &filtered_rows {
        list = list.push(file_row(f.clone(), true));
    }

    let count_label = if search::is_active(search_query) {
        format!(
            "{} of {} items match \"{}\"",
            filtered_rows.len(),
            files.len(),
            search_query
        )
    } else {
        format!("{} items", filtered_rows.len())
    };

    let _ = self_node;
    column![
        banner_widget,
        Space::with_height(Length::Fixed(22.0)),
        section_h("Shared with this node", Some(&count_label)),
        list,
    ]
    .spacing(0)
    .into()
}

// ─── Inbox ─────────────────────────────────────────────────────────────────

pub fn inbox<'a>(snap: &'a BackendSnapshot) -> Element<'a, Message> {
    let self_node = &snap.self_node;
    let unique_senders = {
        let mut hosts: Vec<&str> = snap.inbox.iter().filter_map(|f| f.from.as_deref()).collect();
        hosts.sort_unstable();
        hosts.dedup();
        hosts.len()
    };

    let banner_widget = banner(
        icons::INBOX,
        "Mesh inbox".to_string(),
        format!(
            "files peers sent to {} · auto-routed to ~/mesh/inbox/",
            self_node.host
        ),
        vec![
            BannerStat::new(snap.inbox.len().to_string(), "Items"),
            BannerStat::new(unique_senders.to_string(), "From peers"),
        ],
    );

    let mut list = column![file_row_head("From")];
    for f in &snap.inbox {
        list = list.push(file_row(f.clone(), true));
    }

    column![banner_widget, Space::with_height(Length::Fixed(22.0)), list,]
        .spacing(0)
        .into()
}

// ─── Downloads ─────────────────────────────────────────────────────────────

pub fn downloads<'a>(snap: &'a BackendSnapshot) -> Element<'a, Message> {
    let mesh_count = snap.downloads.iter().filter(|d| d.mesh.is_some()).count();

    let banner_widget = banner(
        icons::DOWNLOAD,
        "Downloads  · ~/Downloads".to_string(),
        format!(
            "local downloads · {mesh_count} item{plural} arrived via mesh transfer",
            plural = if mesh_count == 1 { "" } else { "s" }
        ),
        vec![
            BannerStat::new(snap.downloads.len().to_string(), "Items"),
            BannerStat::new(mesh_count.to_string(), "From mesh"),
        ],
    );

    let mut list = column![file_row_head("Origin")];
    for f in &snap.downloads {
        list = list.push(file_row(f.clone(), true));
    }

    column![banner_widget, Space::with_height(Length::Fixed(22.0)), list,]
        .spacing(0)
        .into()
}

// ─── Local veil ────────────────────────────────────────────────────────────

pub fn local_veil<'a>(snap: &'a BackendSnapshot) -> Element<'a, Message> {
    let self_node = &snap.self_node;
    let title_row = row![
        icon(icons::HDD, 18.0, t::FG),
        text("Local filesystem").size(15).color(t::FG),
        container(
            text(format!("private to {}", self_node.host))
                .size(10)
                .color(t::FG_FAINT)
        )
        .padding(Padding::from([1.0, 6.0]))
        .style(|_| container::Style {
            background: Some(Background::Color(Color {
                a: 0.04,
                ..Color::WHITE
            })),
            border: Border {
                color: Color {
                    a: 0.08,
                    ..Color::WHITE
                },
                width: 1.0,
                radius: 0.0.into()
            },
            ..container::Style::default()
        }),
    ]
    .spacing(8)
    .align_y(iced::alignment::Vertical::Center);

    let body_text = format!(
        "This is the unsynced filesystem on {host}. Nothing here is visible to other peers. \
         To share, move a file into ~/mesh or drag it onto a peer in the sidebar.",
        host = self_node.host,
    );

    let pin_children: Vec<Element<'_, Message>> = snap
        .local_pins
        .iter()
        .cloned()
        .map(local_pin_tile)
        .collect();
    let pin_grid = iced::widget::Row::with_children(pin_children)
        .spacing(6)
        .wrap();

    let veil = container(
        column![
            title_row,
            text(body_text).size(12).color(t::FG_DIM),
            pin_grid,
        ]
        .spacing(14),
    )
    .padding(Padding::from([20.0, 22.0]))
    .style(|_| container::Style {
        background: Some(Background::Color(t::PF_BG_200)),
        border: Border {
            color: t::DIVIDER,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    });

    let mut recent = column![file_row_head("Where")];
    for f in &snap.local_recent {
        recent = recent.push(file_row(f.clone(), true));
    }

    column![
        veil,
        Space::with_height(Length::Fixed(20.0)),
        section_h("Recent locally-modified", Some("~/ · last 24 h")),
        recent,
    ]
    .spacing(0)
    .into()
}

// ─── Mesh Home (AF-mesh.2) ────────────────────────────────────────────────

/// Landing card grid for the five shared XDG dirs. Per the
/// v5.0.0 GlusterFS lock these dirs are full-mesh-replicated
/// over Nebula, so they're first-class mesh resources — not
/// local files. The page is the operator's primary entry into
/// the shared file plane.
pub fn mesh_home<'a>(snap: &'a BackendSnapshot) -> Element<'a, Message> {
    let (vol_summary, mount_subtitle) =
        match (snap.mesh_volume.as_ref(), snap.mesh_mount.as_ref()) {
            (Some(v), Some(m)) if v.volume_online && m.is_mounted => (
                vec![
                    BannerStat::new(fmt_bytes_u64(v.total_bytes), "Total"),
                    BannerStat::new(fmt_bytes_u64(v.free_bytes), "Free"),
                    BannerStat::new(v.bricks_count.to_string(), "Bricks"),
                ],
                format!("GlusterFS mounted at {}", m.mount_point),
            ),
            (Some(v), _) if v.volume_online => (
                vec![
                    BannerStat::new(fmt_bytes_u64(v.total_bytes), "Total"),
                    BannerStat::new(fmt_bytes_u64(v.free_bytes), "Free"),
                ],
                "GlusterFS volume online; FUSE mount pending".into(),
            ),
            _ => (
                vec![BannerStat::new("—".to_string(), "No mesh".to_string())],
                "Local-only mode · gluster bootstrap pending".into(),
            ),
        };

    let banner_widget = banner(
        icons::MESH_HUB,
        "Mesh Home".to_string(),
        mount_subtitle,
        vol_summary,
    );

    // Five cards — Documents · Pictures · Music · Videos · Downloads.
    // Each card routes to MeshHomeChild(slug).
    let cards: Vec<Element<'_, Message>> = MESH_HOME_DIRS
        .iter()
        .map(|(slug, label, pin_icon)| mesh_home_card(slug, label, *pin_icon))
        .collect();
    let card_grid = iced::widget::Row::with_children(cards)
        .spacing(10)
        .wrap();

    column![
        banner_widget,
        Space::with_height(Length::Fixed(22.0)),
        section_h(
            "Shared directories",
            Some("auto-synced across every peer in the mesh")
        ),
        card_grid,
    ]
    .spacing(0)
    .into()
}

/// File listing inside one of the shared XDG dirs. Reads
/// from `local:<slug>` via the backend (which today is the
/// `LocalFsBackend` path) — once GlusterFS is FUSE-mounted at
/// the XDG dirs the listing is the same disk read but the
/// content reflects mesh-replicated state.
///
/// AF-mesh.3 — subdirectory navigation. When `path` is non-
/// empty the page shows a parent-link affordance ("↑ <prev>")
/// and folder rows render as clickable buttons that dispatch
/// `Message::MeshFolderEnter`. File rows stay non-clickable;
/// future commits add per-file actions.
pub fn mesh_home_child<'a>(
    slug: &'a str,
    files: Vec<FileRow>,
    search: &'a str,
    _layout: Layout,
    path: &'a [String],
) -> Element<'a, Message> {
    let label = crate::app::mesh_home_label(slug);
    let filtered: Vec<FileRow> = if search::is_active(search) {
        search::filter_rows(&files, search)
    } else {
        files
    };
    let count = filtered.len();
    let sub = path.join("/");
    let banner_subtitle = if path.is_empty() {
        format!(
            "{count} item{plural} · mesh-replicated via GlusterFS",
            plural = if count == 1 { "" } else { "s" }
        )
    } else {
        format!(
            "{count} item{plural} · ~/{label}/{sub}",
            plural = if count == 1 { "" } else { "s" }
        )
    };
    let banner_title = if path.is_empty() {
        format!("Mesh Home · {label}")
    } else {
        format!("Mesh Home · {label}/{sub}")
    };
    let banner_widget = banner(
        icons::FOLDER,
        banner_title,
        banner_subtitle,
        vec![BannerStat::new(count.to_string(), "Items")],
    );

    let mut list = column![file_row_head("Modified")];
    // Parent-link row when descended at least one level.
    if !path.is_empty() {
        list = list.push(parent_link_row());
    }
    for f in filtered {
        let is_folder = f.name.ends_with('/') || matches!(f.mime, crate::model::Mime::Folder);
        if is_folder {
            // Clickable folder row. Strip trailing `/` for the
            // message payload so the reducer compares clean
            // names against the path stack.
            list = list.push(folder_row_button(f));
        } else {
            list = list.push(file_row(f, false));
        }
    }

    column![
        banner_widget,
        Space::with_height(Length::Fixed(22.0)),
        list.spacing(0),
    ]
    .spacing(0)
    .into()
}

/// Parent-link row for nested Mesh Home navigation. Mirrors the
/// shape of `file_row` so the list looks continuous; clicking
/// dispatches `Message::MeshFolderUp`.
fn parent_link_row() -> Element<'static, Message> {
    button(
        container(
            row![
                icon(icons::ARROW_LEFT, 14.0, t::FG_DIM),
                Space::with_width(Length::Fixed(8.0)),
                text("..").size(12).color(t::FG_DIM),
                Space::with_width(Length::Fill),
                text("parent folder").size(10).color(t::FG_FAINT),
            ]
            .align_y(iced::alignment::Vertical::Center),
        )
        .padding(Padding::from([6.0, 12.0]))
        .width(Length::Fill),
    )
    .padding(0)
    .style(|_, _| ghost_button_style())
    .on_press(Message::MeshFolderUp)
    .into()
}

/// Clickable folder row used inside `mesh_home_child`. Renders
/// the same shape as `file_row` but the whole row is a button
/// that dispatches `Message::MeshFolderEnter(name)`.
/// CR-4 — folder navigation row renders as a CardSize::Small
/// Object Card per docs/design/chromeos-classic-spec.md §Object
/// Cards. Title: folder name; subtitle: `<size> · <age>` (size +
/// last-modified condensed into the one-line compact-shape slot
/// per the round-4 re-ask 2026-05-24). Wrapped in a button so the
/// card is the click target for `MeshFolderEnter`.
///
/// File-row retrofit (per-view file enumeration through
/// `widgets::file_row`) tracked as CR-4.b — share this same
/// `mde_iced_components::object_card` call once the file-row
/// data shape (name + size + mtime + selection state) maps
/// cleanly onto the Card schema.
fn folder_row_button(f: FileRow) -> Element<'static, Message> {
    let name_payload = f.name.clone();
    let display = f.name.trim_end_matches('/').to_owned();
    let subtitle = match (f.size.is_empty(), f.age.is_empty()) {
        (true, true) => String::new(),
        (true, false) => f.age.clone(),
        (false, true) => f.size.clone(),
        (false, false) => format!("{} · {}", f.size, f.age),
    };
    let palette = t::mde_files_palette();
    let mut card = mde_theme::ObjectCard::small(
        mde_theme::Icon::Fleet,
        format!("{display}/"),
    );
    if !subtitle.is_empty() {
        card = card.with_subtitle(subtitle);
    }
    button(mde_iced_components::object_card(card, palette))
        .padding(0)
        .style(|_, _| ghost_button_style())
        .on_press(Message::MeshFolderEnter(name_payload))
        .into()
}

/// The five mesh-home shortcut slugs the sidebar + the
/// MeshHome card grid both consume. Stays a single source of
/// truth so adding a sixth directory means changing one
/// constant.
pub const MESH_HOME_DIRS: &[(&str, &str, crate::model::PinIcon)] = &[
    ("docs", "Documents", crate::model::PinIcon::Doc2),
    ("pics", "Pictures", crate::model::PinIcon::Image),
    ("music", "Music", crate::model::PinIcon::Doc),
    ("videos", "Videos", crate::model::PinIcon::Player),
    ("downloads", "Downloads", crate::model::PinIcon::Home),
];

fn mesh_home_card(
    slug: &'static str,
    label: &'static str,
    pin_icon: crate::model::PinIcon,
) -> Element<'static, Message> {
    let inner = container(
        column![
            row![
                icon(icons::svg_for_pin(pin_icon), 20.0, t::ACCENT),
                Space::with_width(Length::Fill),
                text("shared")
                    .size(9)
                    .color(t::ACCENT),
            ]
            .align_y(iced::alignment::Vertical::Center),
            Space::with_height(Length::Fixed(12.0)),
            text(label).size(14).color(t::FG),
            text(format!("~/{label}"))
                .size(10)
                .color(t::FG_FAINT),
        ]
        .spacing(2),
    )
    .padding(Padding::from([14.0, 16.0]))
    .width(Length::Fixed(180.0))
    .height(Length::Fixed(110.0))
    .style(|_| container::Style {
        background: Some(Background::Color(Color {
            a: 0.04,
            ..Color::WHITE
        })),
        border: Border {
            color: Color {
                a: 0.12,
                ..Color::WHITE
            },
            width: 1.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    });

    button(inner)
        .padding(0)
        .on_press(Message::SelectView(crate::model::View::MeshHomeChild(
            slug.into(),
        )))
        .style(|_, _| button::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            text_color: t::FG,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..button::Style::default()
        })
        .into()
}

// ── MESHFS-11.1: Conflict resolve dialog ────────────────────────────────────

/// Modal overlay for resolving a `.conflict-*` file pair. Renders as a
/// semi-transparent full-screen backdrop (click to dismiss) with a
/// centered card showing the two resolution options. Compose via
/// `iced::widget::Stack::with_children(vec![base_view, resolve_conflict_dialog(...)])`.
pub fn resolve_conflict_dialog<'a>(
    original: &'a str,
    sibling: &'a str,
) -> Element<'a, Message> {
    let orig = original.to_owned();
    let sib = sibling.to_owned();

    let dismiss_backdrop = button(Space::new(Length::Fill, Length::Fill))
        .padding(0)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_, _| button::Style {
            background: Some(Background::Color(Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.55,
            })),
            text_color: Color::TRANSPARENT,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..button::Style::default()
        })
        .on_press(Message::DismissConflictDialog);

    let dialog_card = container(
        column![
            text("Merge conflict").size(14).color(t::FG),
            Space::with_height(Length::Fixed(4.0)),
            text(format!("File: {original}")).size(12).color(t::FG_DIM),
            Space::with_height(Length::Fixed(16.0)),
            button(
                column![
                    text("Keep original").size(12).color(t::FG),
                    text(format!("Archive: {sibling}"))
                        .size(10)
                        .color(t::FG_FAINT),
                ]
                .spacing(2),
            )
            .padding(Padding::from([10.0, 14.0]))
            .width(Length::Fill)
            .style(|_, status| {
                let bg = match status {
                    button::Status::Hovered => t::PRIMARY_AMBER_BG_HOVER,
                    _ => t::PRIMARY_AMBER_BG,
                };
                button::Style {
                    background: Some(Background::Color(bg)),
                    text_color: t::FG,
                    border: Border {
                        color: t::PRIMARY_AMBER_BORDER,
                        width: 1.0,
                        radius: 0.0.into(),
                    },
                    ..button::Style::default()
                }
            })
            .on_press(Message::ArchiveConflictFile(sib)),
            Space::with_height(Length::Fixed(8.0)),
            button(
                column![
                    text("Keep conflict copy").size(12).color(t::FG),
                    text(format!("Archive: {original}"))
                        .size(10)
                        .color(t::FG_FAINT),
                ]
                .spacing(2),
            )
            .padding(Padding::from([10.0, 14.0]))
            .width(Length::Fill)
            .style(|_, status| {
                let bg = match status {
                    button::Status::Hovered => Color {
                        a: 0.08,
                        ..Color::WHITE
                    },
                    _ => Color {
                        a: 0.04,
                        ..Color::WHITE
                    },
                };
                button::Style {
                    background: Some(Background::Color(bg)),
                    text_color: t::FG,
                    border: Border {
                        color: t::DIVIDER,
                        width: 1.0,
                        radius: 0.0.into(),
                    },
                    ..button::Style::default()
                }
            })
            .on_press(Message::ArchiveConflictFile(orig)),
            Space::with_height(Length::Fixed(12.0)),
            button(text("Dismiss").size(11).color(t::FG_FAINT))
                .padding(Padding::from([4.0, 10.0]))
                .style(|_, _| button::Style {
                    background: Some(Background::Color(Color::TRANSPARENT)),
                    text_color: t::FG_FAINT,
                    border: Border {
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius: 0.0.into(),
                    },
                    ..button::Style::default()
                })
                .on_press(Message::DismissConflictDialog),
        ]
        .spacing(0),
    )
    .width(Length::Fixed(420.0))
    .padding(Padding::from([24.0, 28.0]))
    .style(|_| container::Style {
        background: Some(Background::Color(t::PF_BG_200)),
        border: Border {
            color: t::PRIMARY_AMBER_BORDER,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    });

    let centered_dialog = container(dialog_card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    iced::widget::Stack::with_children(vec![
        dismiss_backdrop.into(),
        centered_dialog.into(),
    ])
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn fmt_bytes_u64(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;
    if n >= TB {
        format!("{:.1} TB", n as f64 / TB as f64)
    } else if n >= GB {
        format!("{:.1} GB", n as f64 / GB as f64)
    } else if n >= MB {
        format!("{:.1} MB", n as f64 / MB as f64)
    } else if n >= KB {
        format!("{} KB", n / KB)
    } else {
        format!("{n} B")
    }
}

fn local_pin_tile(pin: LocalPin) -> Element<'static, Message> {
    container(
        row![
            icon(icons::svg_for_pin(pin.icon), 16.0, t::FG_DIM),
            text(pin.name.to_string()).size(12).color(t::FG_DIM),
            Space::with_width(Length::Fill),
            text(pin.path.to_string()).size(10).color(t::FG_FAINT),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Center),
    )
    .padding(Padding::from([8.0, 10.0]))
    .width(Length::Fixed(180.0))
    .style(|_| container::Style {
        background: Some(Background::Color(Color {
            a: 0.02,
            ..Color::WHITE
        })),
        border: Border {
            color: Color {
                a: 0.05,
                ..Color::WHITE
            },
            width: 1.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    })
    .into()
}

// ── MESHFS-8.1: Recycle Bin view ────────────────────────────────────────────

/// Render the LizardFS trash listing. Shows items recoverable within the
/// configured retention window (default 48 h) with a "Restore" button per
/// row. Displays a loading/error state when busy or on error.
pub fn mesh_undelete<'a>(
    items: &'a [TrashItem],
    busy: bool,
    error: Option<&'a str>,
) -> Element<'a, Message> {
    let header = row![
        text("Recycle Bin").size(13).color(t::FG),
        Space::with_width(Length::Fill),
        text(if busy { "Loading…" } else { "" })
            .size(11)
            .color(t::FG_FAINT),
    ]
    .align_y(iced::alignment::Vertical::Center);

    let body: Element<'a, Message> = if let Some(err) = error {
        text(format!("Error: {err}"))
            .size(12)
            .color(Color {
                r: 1.0,
                g: 0.35,
                b: 0.35,
                a: 1.0,
            })
            .into()
    } else if items.is_empty() && !busy {
        text("Recycle Bin is empty — no files recoverable.")
            .size(12)
            .color(t::FG_FAINT)
            .into()
    } else {
        let rows: Vec<Element<'a, Message>> = items
            .iter()
            .map(|item| trash_row(item))
            .collect();
        scrollable(column(rows).spacing(2)).into()
    };

    column![header, Space::with_height(12), body]
        .spacing(4)
        .into()
}

fn trash_row(item: &TrashItem) -> Element<'_, Message> {
    let path = item.trash_path.clone();
    let restore_btn = button(text("Restore").size(11).color(t::FG))
        .padding(Padding::from([4.0, 10.0]))
        .style(|_, status: iced::widget::button::Status| {
            let bg = match status {
                iced::widget::button::Status::Hovered => Color {
                    a: 0.12,
                    ..Color::WHITE
                },
                _ => Color {
                    a: 0.07,
                    ..Color::WHITE
                },
            };
            iced::widget::button::Style {
                background: Some(Background::Color(bg)),
                text_color: t::FG,
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 4.0.into(),
                },
                shadow: iced::Shadow::default(),
            }
        })
        .on_press(Message::RestoreTrashItem(path));

    row![
        icon(icons::TRASH2, 14.0, t::FG_FAINT),
        text(item.name.clone())
            .size(12)
            .color(t::FG)
            .width(Length::Fill),
        restore_btn,
    ]
    .spacing(8)
    .align_y(iced::alignment::Vertical::Center)
    .padding(Padding::from([4.0, 0.0]))
    .into()
}
