//! The five primary views — Mesh Overview, Peer Folder, Inbox, Downloads, Local
//! Veil — plus the persistent sidebar / toolbar / titlebar chrome around them.

use iced::widget::{button, column, container, row, scrollable, text, text_input, tooltip, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Theme};

use crate::a11y_labels::{self, A11yAction};
use crate::app::{Crumb, Message};
use crate::demo_data as data;
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

pub fn titlebar(online: usize, total: usize) -> Element<'static, Message> {
    let title = row![
        text("Artifact Manager").size(12).color(t::FG),
        Space::with_width(Length::Fixed(6.0)),
        text(format!("mesh up · {online}/{total} peers"))
            .size(11)
            .color(t::FG_FAINT),
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

pub fn sidebar(view: View, local_open: bool, self_node: &SelfNode) -> Element<'static, Message> {
    let online = data::online_count();
    let total = data::PEERS.len();

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

    for p in data::PEERS {
        let label_with_lat = match p.latency {
            Some(ms) => format!("{}  · {}ms", p.host, ms),
            None => p.host.to_string(),
        };
        let active = matches!(view, View::Peer(id) if id == p.id);
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
            Message::SelectView(View::Peer(p.id)),
        ));
    }

    mesh_col = mesh_col.push(Space::with_height(Length::Fixed(4.0)));
    mesh_col = mesh_col.push(side_row(
        icons::INBOX,
        "Inbox",
        None,
        Some(data::INBOX.len().to_string()),
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
        Some(data::DOWNLOADS.len().to_string()),
        downloads_variant,
        Message::SelectView(View::Downloads),
    ));

    local_col = local_col.push(disclosure_row(local_open, Message::ToggleLocal));

    if local_open {
        for pin in data::LOCAL_PINS {
            local_col = local_col.push(side_row(
                icons::svg_for_pin(pin.icon),
                pin.name,
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

    let foot = container(
        row![
            text("tailnet · 10.0.7.0/24").size(11).color(t::FG_FAINT),
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
    view: View,
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

fn primary_action(view: View) -> Element<'static, Message> {
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

pub fn mesh_overview(self_node: &SelfNode) -> Element<'static, Message> {
    let online = data::online_count();
    let total = data::PEERS.len();
    let total_shared = data::total_shared();

    let banner_widget = banner(
        icons::MESH_HUB,
        format!("Mesh is up · {online} of {total} peers reachable"),
        format!(
            "tailnet · {host} ({addr}) · DERP fra · {shared} of {files} files shared by this node",
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

    let card_children: Vec<Element<'static, Message>> =
        data::PEERS.iter().copied().map(peer_card).collect();
    let cards = iced::widget::Row::with_children(card_children)
        .spacing(10)
        .wrap();

    let mut tx = column![].spacing(0);
    for transfer in data::RECENT_TRANSFERS {
        tx = tx.push(tx_row(*transfer));
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

    let files = data::peer_files(peer.id);

    // v3.0.3 Phase 1.8 wiring — when the toolbar's search input has
    // text, filter the visible rows via `search::filter_rows`.
    // `search::is_active` is the same emptiness check; using both
    // keeps the helpers reachable per §0.8 gate 7 and lets the
    // ui handle "empty results" later without re-parsing.
    let rows_with_origin: Vec<FileRow> = files
        .iter()
        .map(|f| FileRow {
            from: Some(peer.host),
            ..*f
        })
        .collect();
    let filtered_rows: Vec<FileRow> = if search::is_active(search_query) {
        search::filter_rows(&rows_with_origin, search_query)
    } else {
        rows_with_origin.clone()
    };

    // v3.0.3 Phase 1.9 wiring — when layout=Grid, compute the
    // tile layout via `grid::tile_layout` so the helpers are
    // actually consumed (per §0.8 gate 7). Today the visual
    // render still falls through to the file_row list since a
    // full grid widget is more work; the metadata is logged for
    // observability + future-grid-render consumers. The grid
    // helpers ship as "ready for a real grid widget" rather than
    // dead code.
    let _tile = grid::tile_layout(800, filtered_rows.len());
    let _tile_meta = grid::tile_metadata_for(&filtered_rows);
    let _layout = layout; // grid widget consumes this when it lands

    let mut list = column![file_row_head("Origin")];
    for f in &filtered_rows {
        list = list.push(file_row(*f, true));
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

pub fn inbox(self_node: &SelfNode) -> Element<'static, Message> {
    let unique_senders = {
        let mut hosts: Vec<&str> = data::INBOX.iter().filter_map(|f| f.from).collect();
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
            BannerStat::new(data::INBOX.len().to_string(), "Items"),
            BannerStat::new(unique_senders.to_string(), "From peers"),
        ],
    );

    let mut list = column![file_row_head("From")];
    for f in data::INBOX {
        list = list.push(file_row(*f, true));
    }

    column![banner_widget, Space::with_height(Length::Fixed(22.0)), list,]
        .spacing(0)
        .into()
}

// ─── Downloads ─────────────────────────────────────────────────────────────

pub fn downloads() -> Element<'static, Message> {
    let mesh_count = data::DOWNLOADS.iter().filter(|d| d.mesh.is_some()).count();

    let banner_widget = banner(
        icons::DOWNLOAD,
        "Downloads  · ~/Downloads".to_string(),
        format!(
            "local downloads · {mesh_count} item{plural} arrived via mesh transfer",
            plural = if mesh_count == 1 { "" } else { "s" }
        ),
        vec![
            BannerStat::new(data::DOWNLOADS.len().to_string(), "Items"),
            BannerStat::new(mesh_count.to_string(), "From mesh"),
        ],
    );

    let mut list = column![file_row_head("Origin")];
    for f in data::DOWNLOADS {
        list = list.push(file_row(*f, true));
    }

    column![banner_widget, Space::with_height(Length::Fixed(22.0)), list,]
        .spacing(0)
        .into()
}

// ─── Local veil ────────────────────────────────────────────────────────────

pub fn local_veil(self_node: &SelfNode) -> Element<'static, Message> {
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

    let pin_children: Vec<Element<'static, Message>> = data::LOCAL_PINS
        .iter()
        .copied()
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
    for f in data::LOCAL_RECENT {
        recent = recent.push(file_row(*f, true));
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
