//! Reusable building blocks for the Artifact Manager UI — pills, banners, peer
//! cards, file rows, section headers, mime icons. Each function takes the data
//! it needs and returns an `Element<'_, Message>`.

use iced::widget::{button, column, container, row, svg, text, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Theme};

use crate::app::Message;
use crate::icons;
use crate::model::{
    fmt_count, latency_bucket, FileRow, LatencyBucket, Mime, Peer, PeerKind, PeerStatus, Transfer,
    TxDir,
};
use crate::theme as t;

// ─── Generic helpers ───────────────────────────────────────────────────────

/// A coloured square dot — used for status indicators (`.peer-status` in CSS).
pub fn status_dot(color: Color, size: f32) -> Element<'static, Message> {
    container(Space::new(Length::Fixed(size), Length::Fixed(size)))
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(color)),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: (size / 2.0).into(),
            },
            ..container::Style::default()
        })
        .into()
}

/// Wrap an SVG icon (one of `icons::*`) in a fixed-size box, tinted with `color`.
pub fn icon(svg_bytes: &'static [u8], size: f32, color: Color) -> Element<'static, Message> {
    svg(icons::handle(svg_bytes))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |_theme: &Theme, _status: svg::Status| svg::Style { color: Some(color) })
        .into()
}

/// 1-px horizontal divider line (`var(--divider)`).
pub fn hdivider() -> Element<'static, Message> {
    container(Space::new(Length::Fill, Length::Fixed(1.0)))
        .style(|_| container::Style {
            background: Some(Background::Color(t::DIVIDER)),
            ..container::Style::default()
        })
        .into()
}

/// Standard background container that paints `bg` and draws a 1-px border in `border`.
pub fn surface<'a, M: 'a>(bg: Color, border: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// Caps section header (`.fm-section-h h3`) — 11 px, letter-spaced.
pub fn section_h(label: &str, right: Option<&str>) -> Element<'static, Message> {
    let mut r = row![text(label.to_uppercase()).size(11).color(t::FG_DIM),]
        .spacing(8)
        .align_y(iced::alignment::Vertical::Center);

    if let Some(rt) = right {
        r = r.push(Space::with_width(Length::Fill));
        r = r.push(text(rt.to_string()).size(10).color(t::FG_FAINT));
    } else {
        r = r.push(Space::with_width(Length::Fill));
    }

    container(r)
        .padding(Padding {
            top: 22.0,
            right: 0.0,
            bottom: 10.0,
            left: 0.0,
        })
        .into()
}

// ─── Pills ─────────────────────────────────────────────────────────────────

/// Amber mesh-origin pill — `↘ peer.mesh`.
pub fn mesh_pill(peer_host: &str) -> Element<'static, Message> {
    container(
        row![
            text("↘").size(10).color(t::RUST),
            text(peer_host.to_string()).size(10).color(t::ACCENT_HI),
        ]
        .spacing(4),
    )
    .padding(Padding::from([1.0, 6.0]))
    .style(|_| container::Style {
        background: Some(Background::Color(t::MESH_PILL_BG)),
        border: Border {
            color: t::MESH_PILL_BORDER,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    })
    .into()
}

/// Neutral local-origin pill — `local`.
pub fn local_pill() -> Element<'static, Message> {
    container(text("local").size(10).color(t::FG_FAINT))
        .padding(Padding::from([1.0, 6.0]))
        .style(|_| container::Style {
            background: Some(Background::Color(t::LOCAL_PILL_BG)),
            border: Border {
                color: t::LOCAL_PILL_BORDER,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        })
        .into()
}

/// Tag chip used in the breadcrumb (`MESH` / `LOCAL`).
pub fn breadcrumb_tag(text_label: &str, is_mesh: bool) -> Element<'static, Message> {
    let (bg, fg, bd) = if is_mesh {
        (t::MESH_PILL_BG, t::ACCENT, t::MESH_PILL_BORDER)
    } else {
        (t::LOCAL_PILL_BG, t::FG_FAINT, t::LOCAL_PILL_BORDER)
    };
    container(text(text_label.to_string()).size(10).color(fg))
        .padding(Padding::from([1.0, 6.0]))
        .style(move |_| container::Style {
            background: Some(Background::Color(bg)),
            border: Border {
                color: bd,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        })
        .into()
}

// ─── Banner (`.fm-banner`) ─────────────────────────────────────────────────

#[derive(Clone)]
pub struct BannerStat {
    pub n: String,
    pub k: String,
}

impl BannerStat {
    #[must_use]
    pub fn new(n: impl Into<String>, k: impl Into<String>) -> Self {
        Self {
            n: n.into(),
            k: k.into(),
        }
    }
}

/// Hero banner with amber gradient + 3 px left border. Used at the top of every view
/// except the local veil.
pub fn banner(
    icon_svg: &'static [u8],
    title: String,
    subtitle: String,
    stats: Vec<BannerStat>,
) -> Element<'static, Message> {
    let mut stats_row = row![].spacing(22);
    for st in stats {
        stats_row = stats_row.push(
            column![
                text(st.n).size(20).color(t::ACCENT_HI),
                text(st.k.to_uppercase()).size(9).color(t::FG_FAINT),
            ]
            .spacing(4)
            .align_x(iced::alignment::Horizontal::Right),
        );
    }

    let layout = row![
        container(icon(icon_svg, 22.0, t::ACCENT_HI))
            .padding(Padding::new(9.0))
            .style(|_| container::Style {
                background: Some(Background::Color(t::MESH_PILL_BG)),
                border: Border {
                    color: t::MESH_PILL_BORDER,
                    width: 1.0,
                    radius: 0.0.into()
                },
                ..container::Style::default()
            }),
        column![
            text(title).size(15).color(t::FG),
            text(subtitle).size(11).color(t::FG_DIM),
        ]
        .spacing(2)
        .width(Length::Fill),
        stats_row,
    ]
    .spacing(16)
    .align_y(iced::alignment::Vertical::Center);

    container(layout)
        .padding(Padding::from([14.0, 18.0]))
        .style(|_| container::Style {
            background: Some(Background::Color(t::PF_BG_200)),
            border: Border {
                color: t::BANNER_BORDER,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        })
        .into()
}

// ─── Peer card (`.fm-peer-card`) ───────────────────────────────────────────

pub fn peer_card(peer: Peer) -> Element<'static, Message> {
    let kind_icon = icons::svg_for_peer_kind(peer.kind);

    let avatar_color = match peer.status {
        PeerStatus::Online => t::ACCENT_HI,
        PeerStatus::Self_ => t::RUST,
        _ => t::FG_DIM,
    };

    let stripe_color = match peer.status {
        PeerStatus::Online => t::PF_SUCCESS,
        PeerStatus::Idle => t::ACCENT,
        PeerStatus::Offline => t::PF_BORDER,
        PeerStatus::Self_ => t::RUST,
    };

    let head = row![
        container(icon(kind_icon, 20.0, avatar_color))
            .padding(Padding::new(8.0))
            .style(|_| container::Style {
                background: Some(Background::Color(Color {
                    a: 0.04,
                    ..Color::WHITE
                })),
                border: Border {
                    color: Color {
                        a: 0.06,
                        ..Color::WHITE
                    },
                    width: 1.0,
                    radius: 0.0.into()
                },
                ..container::Style::default()
            }),
        column![
            row![
                status_dot(t::peer_status_dot(peer.status), 6.0),
                text(peer.host.to_string()).size(13).color(t::FG),
            ]
            .spacing(6)
            .align_y(iced::alignment::Vertical::Center),
            text(peer.label.to_string()).size(11).color(t::FG_FAINT),
        ]
        .spacing(2)
        .width(Length::Fill),
    ]
    .spacing(10)
    .align_y(iced::alignment::Vertical::Center);

    let num_row = row![
        row![
            text(fmt_count(peer.files)).size(16).color(t::FG),
            text("FILES").size(9).color(t::FG_FAINT),
        ]
        .spacing(4)
        .align_y(iced::alignment::Vertical::Center),
        row![
            text(fmt_count(peer.shared)).size(16).color(t::FG),
            text("SHARED").size(9).color(t::FG_FAINT),
        ]
        .spacing(4)
        .align_y(iced::alignment::Vertical::Center),
    ]
    .spacing(16);

    let meta_row = row![
        text(peer.addr.to_string()).size(10).color(t::FG_FAINT),
        Space::with_width(Length::Fill),
        match peer.latency {
            Some(ms) => {
                let c = match latency_bucket(ms) {
                    LatencyBucket::Good => t::PF_SUCCESS,
                    LatencyBucket::Ok => t::ACCENT,
                    LatencyBucket::Slow => t::FG_FAINT,
                };
                Element::from(text(format!("{ms} ms · {}", peer.derp)).size(10).color(c))
            }
            None => Element::from(
                text(format!("last seen {}", peer.last))
                    .size(10)
                    .color(t::FG_FAINT)
            ),
        },
    ];

    let id = peer.id.clone();
    let actions = row![
        button(text("Browse →").size(11).color(t::ACCENT_HI))
            .padding(Padding::from([4.0, 8.0]))
            .style(|_, _| button::Style {
                background: Some(Background::Color(t::MESH_PILL_BG)),
                text_color: t::ACCENT_HI,
                border: Border {
                    color: t::MESH_PILL_BORDER,
                    width: 1.0,
                    radius: 0.0.into()
                },
                ..button::Style::default()
            })
            .on_press(Message::PeerCardBrowse(id.clone())),
        button(text("Send file").size(11).color(t::FG_DIM))
            .padding(Padding::from([4.0, 8.0]))
            .style(|_, _| ghost_button_style())
            .on_press(Message::PeerCardSend(id)),
        button(icon(icons::MORE, 14.0, t::FG_DIM))
            .padding(Padding::from([4.0, 8.0]))
            .style(|_, _| ghost_button_style())
            .on_press(Message::Noop),
    ]
    .spacing(4);

    let stripe =
        container(Space::new(Length::Fill, Length::Fixed(2.0))).style(move |_| container::Style {
            background: Some(Background::Color(stripe_color)),
            ..container::Style::default()
        });

    let body = column![
        stripe,
        column![
            head,
            num_row,
            meta_row,
            container(hdivider()).padding(Padding {
                top: 8.0,
                right: 0.0,
                bottom: 0.0,
                left: 0.0
            }),
            actions,
        ]
        .padding(Padding {
            top: 14.0,
            right: 14.0,
            bottom: 10.0,
            left: 14.0
        })
        .spacing(12),
    ];

    let card_bg = if matches!(peer.status, PeerStatus::Offline) {
        Color {
            a: 0.55,
            ..t::PF_BG_200
        }
    } else {
        t::PF_BG_200
    };

    container(body)
        .style(move |_| container::Style {
            background: Some(Background::Color(card_bg)),
            border: Border {
                color: t::DIVIDER,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        })
        .width(Length::Fixed(244.0))
        .into()
}

/// Ghost-button style (transparent fill, picks up text colour from caller).
pub fn ghost_button_style() -> button::Style {
    button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        text_color: t::FG_DIM,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..button::Style::default()
    }
}

// ─── File row (`.fm-row`) ──────────────────────────────────────────────────

pub fn file_row(row_data: FileRow, show_src: bool) -> Element<'static, Message> {
    // Extract borrowed data before any field moves.
    let is_mesh = row_data.is_mesh();
    let has_conflict = row_data.has_conflict;
    let syncing = row_data.syncing;
    let origin_host: Option<String> = row_data.origin().map(str::to_owned);

    let FileRow { name, conflict_sibling, mime, size, age, .. } = row_data;
    let sibling = conflict_sibling.unwrap_or_default();

    let bg = if is_mesh { t::MESH_ROW_BG } else { Color::TRANSPARENT };
    let mime_color = if is_mesh { t::ACCENT } else { t::FG_FAINT };

    let src_cell: Element<'static, Message> = if show_src {
        match origin_host.as_deref() {
            Some(host) => mesh_pill(host),
            None => local_pill(),
        }
    } else {
        Space::with_width(Length::Fixed(0.0)).into()
    };

    // MESHFS-11.1: yellow clickable chip when a .conflict-* sibling exists.
    let conflict_chip: Element<'static, Message> = if has_conflict {
        let orig_name = name.clone();
        button(
            container(
                row![
                    text("⚠").size(10).color(t::ACCENT_HI),
                    text("conflict").size(10).color(t::ACCENT_HI),
                ]
                .spacing(3)
                .align_y(iced::alignment::Vertical::Center),
            )
            .padding(Padding::from([1.0, 6.0])),
        )
        .padding(0)
        .style(|_, status| {
            let bg = match status {
                button::Status::Hovered => t::PRIMARY_AMBER_BG_HOVER,
                _ => t::PRIMARY_AMBER_BG,
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: t::ACCENT_HI,
                border: Border {
                    color: t::PRIMARY_AMBER_BORDER,
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..button::Style::default()
            }
        })
        .on_press(Message::ConflictResolve(orig_name, sibling))
        .into()
    } else {
        Space::with_width(Length::Fixed(0.0)).into()
    };

    // MESHFS-11.1: ⟳ badge on mesh rows while the fleet is healing.
    let sync_badge: Element<'static, Message> = if syncing {
        container(text("⟳").size(10).color(t::FG_FAINT))
            .padding(Padding::from([1.0, 4.0]))
            .into()
    } else {
        Space::with_width(Length::Fixed(0.0)).into()
    };

    let layout = row![
        container(icon(icons::svg_for_mime(mime), 16.0, mime_color))
            .width(Length::Fixed(22.0))
            .align_x(iced::alignment::Horizontal::Left),
        container(text(name).size(13).color(t::FG)).width(Length::Fill),
        container(conflict_chip).width(Length::Shrink),
        container(sync_badge).width(Length::Shrink),
        container(src_cell).width(Length::Shrink),
        container(text(size).size(11).color(t::FG_DIM))
            .width(Length::Fixed(120.0))
            .align_x(iced::alignment::Horizontal::Right),
        container(text(age).size(11).color(t::FG_DIM))
            .width(Length::Fixed(100.0))
            .align_x(iced::alignment::Horizontal::Right),
    ]
    .spacing(12)
    .align_y(iced::alignment::Vertical::Center);

    container(layout)
        .padding(Padding::from([7.0, 8.0]))
        .style(move |_| container::Style {
            background: Some(Background::Color(bg)),
            border: Border {
                color: t::ROW_DIVIDER,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        })
        .into()
}

/// File-list head row (caps, dim).
pub fn file_row_head(src_label: &str) -> Element<'static, Message> {
    let layout = row![
        Space::with_width(Length::Fixed(22.0)),
        container(text("Name".to_string()).size(10).color(t::FG_FAINT)).width(Length::Fill),
        container(text(src_label.to_string()).size(10).color(t::FG_FAINT)).width(Length::Shrink),
        container(text("Size".to_string()).size(10).color(t::FG_FAINT))
            .width(Length::Fixed(120.0))
            .align_x(iced::alignment::Horizontal::Right),
        container(text("Modified".to_string()).size(10).color(t::FG_FAINT))
            .width(Length::Fixed(100.0))
            .align_x(iced::alignment::Horizontal::Right),
    ]
    .spacing(12);

    container(layout)
        .padding(Padding::from([6.0, 8.0]))
        .style(|_| container::Style {
            background: Some(Background::Color(Color {
                a: 0.02,
                ..Color::WHITE
            })),
            border: Border {
                color: t::DIVIDER,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        })
        .into()
}

// ─── Transfer-log row (`.fm-tx`) ───────────────────────────────────────────

pub fn tx_row(tx: Transfer) -> Element<'static, Message> {
    let (dir_label, dir_fg, dir_bg, dir_bd) = match tx.dir {
        TxDir::In => (
            "↘ IN",
            Color::from_rgb(0x6f as f32 / 255.0, 0xb1 as f32 / 255.0, 1.0),
            Color {
                a: 0.08,
                ..t::PF_INFO
            },
            Color {
                a: 0.40,
                ..t::PF_INFO
            },
        ),
        TxDir::Out => ("↗ OUT", t::ACCENT_HI, t::MESH_PILL_BG, t::MESH_PILL_BORDER),
    };

    let layout = row![
        container(text(dir_label.to_string()).size(10).color(dir_fg))
            .padding(Padding::from([1.0, 6.0]))
            .style(move |_| container::Style {
                background: Some(Background::Color(dir_bg)),
                border: Border {
                    color: dir_bd,
                    width: 1.0,
                    radius: 0.0.into()
                },
                ..container::Style::default()
            }),
        container(text(tx.name.to_string()).size(12).color(t::FG)).width(Length::Fill),
        text(tx.peer.to_string()).size(11).color(t::ACCENT),
        text(format!("{} · {}", tx.size, tx.age))
            .size(11)
            .color(t::FG_FAINT),
    ]
    .spacing(10)
    .align_y(iced::alignment::Vertical::Center);

    container(layout).padding(Padding::from([8.0, 10.0])).into()
}

// ─── Sidebar row ───────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub enum SideRowVariant {
    Default,
    Active,
    Primary,
    PrimaryActive,
    Peer { status: PeerStatus, active: bool },
    Dim,
}

pub fn side_row(
    icon_svg: &'static [u8],
    label: &str,
    secondary: Option<&str>,
    meta: Option<String>,
    variant: SideRowVariant,
    msg: Message,
) -> Element<'static, Message> {
    let (bg, fg, border_color) = match variant {
        SideRowVariant::Default => (Color::TRANSPARENT, t::FG_DIM, Color::TRANSPARENT),
        SideRowVariant::Active => (t::ACTIVE_RUST_BG, Color::WHITE, t::ACTIVE_RUST_BORDER),
        SideRowVariant::Primary => (t::PRIMARY_AMBER_BG, t::FG, t::PRIMARY_AMBER_BORDER),
        SideRowVariant::PrimaryActive => (t::PRIMARY_AMBER_BG_ACTIVE, t::FG, t::ACCENT_HI),
        SideRowVariant::Peer { status, active } => {
            if active {
                (t::ACTIVE_RUST_BG, Color::WHITE, t::ACTIVE_RUST_BORDER)
            } else if matches!(status, PeerStatus::Offline) {
                (Color::TRANSPARENT, t::FG_FAINT, Color::TRANSPARENT)
            } else {
                (Color::TRANSPARENT, t::FG_DIM, Color::TRANSPARENT)
            }
        }
        SideRowVariant::Dim => (Color::TRANSPARENT, t::FG_FAINT, Color::TRANSPARENT),
    };

    let icon_color = match variant {
        SideRowVariant::Active | SideRowVariant::PrimaryActive => t::ACCENT_HI,
        SideRowVariant::Primary => t::ACCENT_HI,
        _ => fg,
    };

    let mut name_row = row![text(label.to_string()).size(13).color(fg)].spacing(6);
    if let Some(s) = secondary {
        name_row = name_row.push(text(s.to_string()).size(10).color(t::FG_FAINT));
    }

    let mut grid = row![
        container(icon(icon_svg, 16.0, icon_color))
            .width(Length::Fixed(18.0))
            .align_x(iced::alignment::Horizontal::Center),
        container(name_row).width(Length::Fill),
    ]
    .spacing(t::SIDE_ROW_GAP)
    .align_y(iced::alignment::Vertical::Center);

    if let Some(m) = meta {
        grid = grid.push(text(m).size(10).color(t::FG_FAINT));
    }

    let inner = container(grid)
        .padding(Padding {
            top: t::SIDE_ROW_PAD_Y,
            right: t::SIDE_ROW_PAD_X,
            bottom: t::SIDE_ROW_PAD_Y,
            // leave room for the 2-px active border
            left: t::SIDE_ROW_PAD_X - 2.0,
        })
        .style(move |_| container::Style {
            background: Some(Background::Color(bg)),
            border: Border {
                color: border_color,
                width: 2.0,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        });

    button(inner)
        .padding(0)
        .style(|_, _| button::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            text_color: t::FG_DIM,
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

/// Sidebar caps section header (`◆ Mesh` or `Local`).
pub fn side_section_header(
    label: &str,
    meta: &str,
    mesh_tinted: bool,
) -> Element<'static, Message> {
    let fg = if mesh_tinted {
        t::ACCENT_HI
    } else {
        t::FG_FAINT
    };
    container(
        row![
            text(label.to_uppercase()).size(10).color(fg),
            Space::with_width(Length::Fill),
            text(meta.to_string()).size(10).color(t::FG_FAINT),
        ]
        .align_y(iced::alignment::Vertical::Center),
    )
    .padding(Padding {
        top: 10.0,
        right: 14.0,
        bottom: 4.0,
        left: 14.0,
    })
    .into()
}

/// "Browse filesystem…" dashed disclosure row at the bottom of the sidebar.
pub fn disclosure_row(open: bool, msg: Message) -> Element<'static, Message> {
    let chevron = if open {
        icons::CHEVRON_DOWN
    } else {
        icons::CHEVRON_RIGHT
    };
    let inner = container(
        row![
            icon(chevron, 14.0, t::FG_FAINT),
            text("Browse filesystem…").size(12).color(t::FG_DIM),
            Space::with_width(Length::Fill),
            text("/").size(10).color(t::FG_FAINT),
        ]
        .spacing(8)
        .align_y(iced::alignment::Vertical::Center),
    )
    .padding(Padding::from([8.0, 12.0]))
    .style(move |_| container::Style {
        background: Some(Background::Color(if open {
            Color {
                a: 0.04,
                ..Color::WHITE
            }
        } else {
            Color {
                a: 0.02,
                ..Color::WHITE
            }
        })),
        border: Border {
            color: Color {
                a: if open { 0.18 } else { 0.10 },
                ..Color::WHITE
            },
            width: 1.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    });

    container(
        button(inner)
            .padding(0)
            .style(|_, _| ghost_button_style())
            .on_press(msg),
    )
    .padding(Padding {
        top: 8.0,
        right: 12.0,
        bottom: 4.0,
        left: 12.0,
    })
    .into()
}

// ─── Mime helper ───────────────────────────────────────────────────────────

#[must_use]
pub const fn mime_label(mime: Mime) -> &'static str {
    match mime {
        Mime::Folder => "folder",
        Mime::Doc => "doc",
        Mime::Image => "image",
        Mime::Pdf => "pdf",
        Mime::Archive => "archive",
        Mime::Disk => "disk",
    }
}

#[must_use]
pub const fn peer_kind_label(kind: PeerKind) -> &'static str {
    match kind {
        PeerKind::Desktop => "desktop",
        PeerKind::Server => "server",
        PeerKind::Phone => "phone",
        PeerKind::Ci => "ci",
    }
}
