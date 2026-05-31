//! VIRT-15.a — the 4-step VM creation wizard.
//!
//! A self-contained sub-component: `WizardState` holds the form, `update`
//! folds a `WizardMsg` and returns a [`WizardAction`] the parent acts on
//! (close, or publish the built `CreateRequest`), and `view` renders the
//! current step. The parent (`app`) owns the `Option<WizardState>`, the
//! `[+ New VM]` entry point, and the publish to
//! `compute/create/<local-peer-nebula-addr>`.
//!
//! Cite: visual-identity.md §1; ref: Apple System Settings.

use std::path::{Path, PathBuf};

use iced::widget::{button, checkbox, column, container, row, text, text_input, Space};
use iced::{Background, Border, Color, Element, Length};
use mde_theme::{Rgba, Tokens, TypeRole};
use serde::{Deserialize, Serialize};

/// Directory scanned for installer / cloud-image ISOs (§13 M-table).
const ISO_DIR: &str = "/var/lib/mde-vms/isos";

/// Create-request payload published to `compute/create/<peer>` (serialize
/// mirror of `compute_provision::CreateRequest`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CreateRequest {
    pub request_ulid: String,
    pub name: String,
    pub vcpus: u32,
    pub ram_mb: u64,
    pub disk_gb: u64,
    pub iso_path: Option<String>,
    pub share_meshfs: bool,
}

/// Messages the wizard emits (wrapped by the parent as `Message::Wizard`).
#[derive(Debug, Clone)]
pub enum WizardMsg {
    NameInput(String),
    /// Pre-fill from the saved template at this index.
    ApplyTemplate(usize),
    /// Delete the saved template at this index (removes its file).
    DeleteTemplate(usize),
    VcpusDelta(i64),
    RamDelta(i64),
    DiskDelta(i64),
    SelectIso(Option<String>),
    CustomIsoInput(String),
    ToggleMeshfs,
    Next,
    Back,
    Cancel,
    Create,
}

/// What the parent should do after folding a [`WizardMsg`].
#[derive(Debug, Clone, PartialEq)]
pub enum WizardAction {
    /// Stay open; nothing for the parent to do.
    None,
    /// Close the wizard (cancelled).
    Cancel,
    /// Publish this create request, then close.
    Create(CreateRequest),
}

/// The wizard's form state.
#[derive(Debug, Clone)]
pub struct WizardState {
    /// 1-4.
    step: u8,
    name: String,
    vcpus: u32,
    ram_mb: u64,
    disk_gb: u64,
    /// Selected ISO from the list (None = no installer ISO).
    iso: Option<String>,
    /// Free-form ISO path (takes precedence over `iso` when non-empty).
    custom_iso: String,
    share_meshfs: bool,
    /// ISOs discovered under [`ISO_DIR`] at open time.
    isos: Vec<String>,
    /// Saved templates `(path, template)` for the step-1 picker (15.c/16.b).
    templates: Vec<(PathBuf, Template)>,
}

impl WizardState {
    /// Open a fresh wizard with the spec defaults (2 vCPU / 2048 MB /
    /// 20 GB / MeshFS on).
    pub fn new() -> Self {
        Self {
            step: 1,
            name: String::new(),
            vcpus: 2,
            ram_mb: 2048,
            disk_gb: 20,
            iso: None,
            custom_iso: String::new(),
            share_meshfs: true,
            isos: list_isos(),
            templates: list_templates(),
        }
    }

    pub fn update(&mut self, msg: WizardMsg) -> WizardAction {
        match msg {
            WizardMsg::NameInput(s) => {
                // Sanitize as typed: alphanumeric + hyphen only.
                self.name = s
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                    .collect();
                WizardAction::None
            }
            WizardMsg::ApplyTemplate(i) => {
                if let Some(t) = self.templates.get(i).map(|(_, t)| t.clone()) {
                    self.name = t
                        .name
                        .chars()
                        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                        .collect();
                    self.vcpus = t.vcpus.clamp(1, 16);
                    self.ram_mb = t.ram_mb.clamp(512, 65536);
                    self.disk_gb = t.disk_gb.clamp(10, 500);
                    self.share_meshfs = t.share_meshfs;
                }
                WizardAction::None
            }
            WizardMsg::DeleteTemplate(i) => {
                if let Some((path, _)) = self.templates.get(i) {
                    let _ = std::fs::remove_file(path);
                }
                self.templates = list_templates();
                WizardAction::None
            }
            WizardMsg::VcpusDelta(d) => {
                self.vcpus = clamp_i64(i64::from(self.vcpus) + d, 1, 16) as u32;
                WizardAction::None
            }
            WizardMsg::RamDelta(d) => {
                self.ram_mb = clamp_i64(self.ram_mb as i64 + d, 512, 65536) as u64;
                WizardAction::None
            }
            WizardMsg::DiskDelta(d) => {
                self.disk_gb = clamp_i64(self.disk_gb as i64 + d, 10, 500) as u64;
                WizardAction::None
            }
            WizardMsg::SelectIso(o) => {
                self.iso = o;
                WizardAction::None
            }
            WizardMsg::CustomIsoInput(s) => {
                self.custom_iso = s;
                WizardAction::None
            }
            WizardMsg::ToggleMeshfs => {
                self.share_meshfs = !self.share_meshfs;
                WizardAction::None
            }
            WizardMsg::Next => {
                if self.step < 4 && self.can_advance() {
                    self.step += 1;
                }
                WizardAction::None
            }
            WizardMsg::Back => {
                if self.step > 1 {
                    self.step -= 1;
                }
                WizardAction::None
            }
            WizardMsg::Cancel => WizardAction::Cancel,
            WizardMsg::Create => {
                if self.step == 4 && name_valid(&self.name) {
                    WizardAction::Create(self.build_request())
                } else {
                    WizardAction::None
                }
            }
        }
    }

    /// Whether the current step's inputs allow advancing.
    fn can_advance(&self) -> bool {
        if self.step == 1 {
            name_valid(&self.name)
        } else {
            true
        }
    }

    /// Resolve the effective ISO path: the custom field wins when set.
    fn effective_iso(&self) -> Option<String> {
        let trimmed = self.custom_iso.trim();
        if trimmed.is_empty() {
            self.iso.clone()
        } else {
            Some(trimmed.to_string())
        }
    }

    /// Build the create request. The libvirt name gets a short ULID suffix
    /// for cross-mesh uniqueness (§13 — "ULID suffix auto-appended").
    fn build_request(&self) -> CreateRequest {
        let ulid = ulid::Ulid::new().to_string();
        let suffix: String = ulid.chars().take(8).collect::<String>().to_ascii_lowercase();
        CreateRequest {
            request_ulid: ulid,
            name: format!("{}-{}", self.name, suffix),
            vcpus: self.vcpus,
            ram_mb: self.ram_mb,
            disk_gb: self.disk_gb,
            iso_path: self.effective_iso(),
            share_meshfs: self.share_meshfs,
        }
    }

    pub fn view<'a>(&'a self, tokens: &'a Tokens) -> Element<'a, WizardMsg> {
        let palette = tokens.palette;
        let space = tokens.space;
        let radius = f32::from(tokens.radii.md);

        let title = text(format!("New VM — step {} of 4", self.step))
            .size(TypeRole::Subheading.size_in(tokens.font_size))
            .color(rgba(palette.text));

        let body: Element<'a, WizardMsg> = match self.step {
            1 => self.step_name(tokens),
            2 => self.step_resources(tokens),
            3 => self.step_disk_iso(tokens),
            _ => self.step_review(tokens),
        };

        // Nav row: Cancel | Back | Next/Create.
        let mut nav = row![btn(tokens, "Cancel", Some(WizardMsg::Cancel))]
            .spacing(f32::from(space.sm));
        if self.step > 1 {
            nav = nav.push(btn(tokens, "Back", Some(WizardMsg::Back)));
        }
        nav = nav.push(Space::new().width(Length::Fill));
        if self.step < 4 {
            let next = self.can_advance().then_some(WizardMsg::Next);
            nav = nav.push(btn(tokens, "Next", next));
        } else {
            let create = name_valid(&self.name).then_some(WizardMsg::Create);
            nav = nav.push(btn(tokens, "Create", create));
        }

        let col = column![title, body, nav]
            .spacing(f32::from(space.md))
            .width(Length::Fill);
        container(col)
            .width(Length::FillPortion(2))
            .height(Length::Fill)
            .padding([space.md, space.lg2])
            .style(move |_t| container::Style {
                snap: false,
                background: Some(Background::Color(rgba(palette.surface))),
                border: Border {
                    color: rgba(palette.border),
                    width: 1.0,
                    radius: radius.into(),
                },
                ..container::Style::default()
            })
            .into()
    }

    fn step_name<'a>(&'a self, tokens: &'a Tokens) -> Element<'a, WizardMsg> {
        let space = tokens.space;
        let mut col = column![
            label(tokens, "VM name"),
            text_input("my-vm", &self.name)
                .on_input(WizardMsg::NameInput)
                .padding(f32::from(space.xs))
                .size(TypeRole::Body.size_in(tokens.font_size)),
        ]
        .spacing(f32::from(space.xs))
        .width(Length::Fill);
        if !name_valid(&self.name) {
            col = col.push(muted(tokens, "Name must be non-empty (letters, digits, hyphens)."));
        }
        if !self.templates.is_empty() {
            col = col.push(label(tokens, "Start from a template"));
            for (i, (_, t)) in self.templates.iter().enumerate() {
                let lbl = format!(
                    "{} — {} vCPU / {} MB / {} GB",
                    t.name, t.vcpus, t.ram_mb, t.disk_gb
                );
                col = col.push(
                    row![
                        btn(tokens, &lbl, Some(WizardMsg::ApplyTemplate(i))),
                        Space::new().width(Length::Fill),
                        btn(tokens, "Delete", Some(WizardMsg::DeleteTemplate(i))),
                    ]
                    .spacing(f32::from(tokens.space.sm))
                    .align_y(iced::alignment::Vertical::Center),
                );
            }
        }
        col.into()
    }

    fn step_resources<'a>(&'a self, tokens: &'a Tokens) -> Element<'a, WizardMsg> {
        let space = tokens.space;
        column![
            stepper(
                tokens,
                "vCPUs",
                &self.vcpus.to_string(),
                WizardMsg::VcpusDelta(-1),
                WizardMsg::VcpusDelta(1),
            ),
            stepper(
                tokens,
                "RAM (MB)",
                &self.ram_mb.to_string(),
                WizardMsg::RamDelta(-512),
                WizardMsg::RamDelta(512),
            ),
        ]
        .spacing(f32::from(space.sm))
        .width(Length::Fill)
        .into()
    }

    fn step_disk_iso<'a>(&'a self, tokens: &'a Tokens) -> Element<'a, WizardMsg> {
        let space = tokens.space;
        let mut col = column![
            stepper(
                tokens,
                "Disk (GB)",
                &self.disk_gb.to_string(),
                WizardMsg::DiskDelta(-1),
                WizardMsg::DiskDelta(1),
            ),
            label(tokens, "Installer ISO"),
        ]
        .spacing(f32::from(space.sm))
        .width(Length::Fill);

        // "None" + each discovered ISO as a selectable row.
        col = col.push(iso_choice(tokens, "None", self.iso.is_none(), WizardMsg::SelectIso(None)));
        for iso in &self.isos {
            let selected = self.iso.as_deref() == Some(iso.as_str());
            let label_txt = Path::new(iso)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(iso.as_str())
                .to_string();
            col = col.push(iso_choice(
                tokens,
                &label_txt,
                selected,
                WizardMsg::SelectIso(Some(iso.clone())),
            ));
        }
        col = col.push(
            text_input("…or a custom ISO path", &self.custom_iso)
                .on_input(WizardMsg::CustomIsoInput)
                .padding(f32::from(space.xs))
                .size(TypeRole::Caption.size_in(tokens.font_size)),
        );
        col = col.push(
            checkbox(self.share_meshfs)
                .label("Share MeshFS")
                .on_toggle(|_| WizardMsg::ToggleMeshfs),
        );
        col.into()
    }

    fn step_review<'a>(&'a self, tokens: &'a Tokens) -> Element<'a, WizardMsg> {
        let space = tokens.space;
        let iso = self
            .effective_iso()
            .unwrap_or_else(|| "none".to_string());
        column![
            kv(tokens, "Name", &self.name),
            kv(tokens, "vCPUs", &self.vcpus.to_string()),
            kv(tokens, "RAM", &format!("{} MB", self.ram_mb)),
            kv(tokens, "Disk", &format!("{} GB", self.disk_gb)),
            kv(tokens, "ISO", &iso),
            kv(
                tokens,
                "MeshFS",
                if self.share_meshfs { "shared" } else { "off" },
            ),
        ]
        .spacing(f32::from(space.xs))
        .width(Length::Fill)
        .into()
    }
}

/// Validate a VM name: non-empty, ASCII alphanumeric + hyphens only.
pub fn name_valid(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// List `*.iso` files under [`ISO_DIR`] (sorted; empty when absent).
fn list_isos() -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(ISO_DIR) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) == Some("iso") {
                if let Some(s) = p.to_str() {
                    out.push(s.to_string());
                }
            }
        }
    }
    out.sort();
    out
}

/// A saved VM config template (the VIRT-16 store; loaded for the step-1
/// picker). Stored at `~/.local/share/mde/vm-templates/<ulid>.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Template {
    pub name: String,
    pub vcpus: u32,
    pub ram_mb: u64,
    pub disk_gb: u64,
    #[serde(default)]
    pub share_meshfs: bool,
}

/// The MeshFS-backed template store dir (`~/.local/share/mde/vm-templates`).
pub fn templates_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("mde").join("vm-templates"))
}

/// Load all saved templates as `(path, template)` (sorted by name; empty
/// when the dir is absent). The path lets the picker delete the file.
fn list_templates() -> Vec<(PathBuf, Template)> {
    let Some(dir) = templates_dir() else {
        return vec![];
    };
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) == Some("json") {
                if let Ok(body) = std::fs::read_to_string(&p) {
                    if let Ok(t) = serde_json::from_str::<Template>(&body) {
                        out.push((p, t));
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| a.1.name.cmp(&b.1.name));
    out
}

fn clamp_i64(v: i64, lo: i64, hi: i64) -> i64 {
    v.max(lo).min(hi)
}

// ── small view helpers ──────────────────────────────────────────────────

fn rgba(c: Rgba) -> Color {
    c.into_iced_color()
}

fn label<'a>(tokens: &Tokens, t: &str) -> Element<'a, WizardMsg> {
    text(t.to_string())
        .size(TypeRole::Body.size_in(tokens.font_size))
        .color(rgba(tokens.palette.text))
        .into()
}

fn muted<'a>(tokens: &Tokens, t: &str) -> Element<'a, WizardMsg> {
    text(t.to_string())
        .size(TypeRole::Caption.size_in(tokens.font_size))
        .color(rgba(tokens.palette.text_muted))
        .into()
}

fn kv<'a>(tokens: &Tokens, k: &str, v: &str) -> Element<'a, WizardMsg> {
    let palette = tokens.palette;
    row![
        text(k.to_string())
            .size(TypeRole::Caption.size_in(tokens.font_size))
            .color(rgba(palette.text_muted))
            .width(Length::FillPortion(2)),
        text(v.to_string())
            .size(TypeRole::Body.size_in(tokens.font_size))
            .color(rgba(palette.text))
            .width(Length::FillPortion(3)),
    ]
    .spacing(f32::from(tokens.space.sm))
    .into()
}

/// A button; `msg = None` renders it disabled (greyed, no `on_press`).
fn btn<'a>(tokens: &Tokens, lbl: &str, msg: Option<WizardMsg>) -> Element<'a, WizardMsg> {
    let palette = tokens.palette;
    let space = tokens.space;
    let radius = f32::from(tokens.radii.sm);
    let enabled = msg.is_some();
    let mut b = button(text(lbl.to_string()).size(TypeRole::Caption.size_in(tokens.font_size)))
        .padding([space.xs2, space.xs])
        .style(move |_t, _s| button::Style {
            background: Some(Background::Color(rgba(palette.surface))),
            text_color: if enabled {
                rgba(palette.accent)
            } else {
                rgba(palette.text_muted)
            },
            border: Border {
                color: rgba(palette.border),
                width: 1.0,
                radius: radius.into(),
            },
            ..button::Style::default()
        });
    if let Some(m) = msg {
        b = b.on_press(m);
    }
    b.into()
}

/// `label  [-] value [+]` numeric stepper.
fn stepper<'a>(
    tokens: &Tokens,
    lbl: &str,
    value: &str,
    dec: WizardMsg,
    inc: WizardMsg,
) -> Element<'a, WizardMsg> {
    let palette = tokens.palette;
    let space = tokens.space;
    row![
        text(lbl.to_string())
            .size(TypeRole::Body.size_in(tokens.font_size))
            .color(rgba(palette.text))
            .width(Length::Fill),
        btn(tokens, "-", Some(dec)),
        text(value.to_string())
            .size(TypeRole::Body.size_in(tokens.font_size))
            .color(rgba(palette.text)),
        btn(tokens, "+", Some(inc)),
    ]
    .spacing(f32::from(space.sm))
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

/// A selectable ISO row (accent text when selected).
fn iso_choice<'a>(
    tokens: &Tokens,
    lbl: &str,
    selected: bool,
    msg: WizardMsg,
) -> Element<'a, WizardMsg> {
    let palette = tokens.palette;
    let space = tokens.space;
    let fg = if selected {
        palette.accent
    } else {
        palette.text
    };
    button(text(lbl.to_string()).size(TypeRole::Caption.size_in(tokens.font_size)).color(rgba(fg)))
        .on_press(msg)
        .padding([space.xs2, space.xs])
        .style(move |_t, _s| button::Style {
            background: None,
            text_color: rgba(fg),
            ..button::Style::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_validation() {
        assert!(name_valid("web-01"));
        assert!(name_valid("db"));
        assert!(!name_valid(""));
        assert!(!name_valid("bad name")); // space
        assert!(!name_valid("under_score"));
    }

    #[test]
    fn name_input_sanitizes() {
        let mut w = WizardState::new();
        w.update(WizardMsg::NameInput("my vm!_01".to_string()));
        assert_eq!(w.name, "myvm01"); // space, !, _ stripped
    }

    #[test]
    fn next_blocked_until_name_valid() {
        let mut w = WizardState::new();
        assert_eq!(w.step, 1);
        w.update(WizardMsg::Next); // empty name → blocked
        assert_eq!(w.step, 1);
        w.update(WizardMsg::NameInput("web".to_string()));
        w.update(WizardMsg::Next);
        assert_eq!(w.step, 2);
    }

    #[test]
    fn steppers_clamp_to_range() {
        let mut w = WizardState::new();
        for _ in 0..30 {
            w.update(WizardMsg::VcpusDelta(1));
        }
        assert_eq!(w.vcpus, 16); // capped
        for _ in 0..30 {
            w.update(WizardMsg::VcpusDelta(-1));
        }
        assert_eq!(w.vcpus, 1); // floored
        w.update(WizardMsg::RamDelta(-512));
        assert_eq!(w.ram_mb, 1536); // 2048 - 512
        for _ in 0..10 {
            w.update(WizardMsg::DiskDelta(-100));
        }
        assert_eq!(w.disk_gb, 10); // floored at 10
    }

    #[test]
    fn cancel_from_any_step_returns_cancel() {
        let mut w = WizardState::new();
        w.update(WizardMsg::NameInput("web".into()));
        w.update(WizardMsg::Next);
        assert_eq!(w.step, 2);
        assert_eq!(w.update(WizardMsg::Cancel), WizardAction::Cancel);
    }

    #[test]
    fn create_only_on_step_4_with_valid_name() {
        let mut w = WizardState::new();
        // Not on step 4 → no create.
        assert_eq!(w.update(WizardMsg::Create), WizardAction::None);
        w.update(WizardMsg::NameInput("web".into()));
        w.update(WizardMsg::Next); // 2
        w.update(WizardMsg::Next); // 3
        w.update(WizardMsg::Next); // 4
        assert_eq!(w.step, 4);
        let action = w.update(WizardMsg::Create);
        match action {
            WizardAction::Create(req) => {
                assert!(req.name.starts_with("web-")); // ULID suffix appended
                assert_eq!(req.vcpus, 2);
                assert_eq!(req.ram_mb, 2048);
                assert_eq!(req.disk_gb, 20);
                assert!(req.share_meshfs);
                assert!(!req.request_ulid.is_empty());
            }
            other => panic!("expected Create, got {other:?}"),
        }
    }

    #[test]
    fn custom_iso_overrides_selection() {
        let mut w = WizardState::new();
        w.iso = Some("/var/lib/mde-vms/isos/a.iso".into());
        w.custom_iso = "  /tmp/custom.iso  ".into();
        assert_eq!(w.effective_iso(), Some("/tmp/custom.iso".to_string()));
        w.custom_iso = "   ".into();
        assert_eq!(w.effective_iso(), Some("/var/lib/mde-vms/isos/a.iso".to_string()));
    }

    #[test]
    fn template_round_trips_json() {
        let t = Template {
            name: "web".into(),
            vcpus: 4,
            ram_mb: 4096,
            disk_gb: 40,
            share_meshfs: true,
        };
        let json = serde_json::to_string(&t).unwrap();
        let back: Template = serde_json::from_str(&json).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn apply_template_prefills_and_clamps() {
        let mut w = WizardState::new();
        w.templates = vec![(
            PathBuf::from("/tmp/x.json"),
            Template {
                name: "big_box!".into(), // sanitized on apply
                vcpus: 99,               // clamped to 16
                ram_mb: 8192,
                disk_gb: 5, // clamped up to 10
                share_meshfs: false,
            },
        )];
        w.update(WizardMsg::ApplyTemplate(0));
        assert_eq!(w.name, "bigbox"); // '!' + '_' stripped
        assert_eq!(w.vcpus, 16);
        assert_eq!(w.ram_mb, 8192);
        assert_eq!(w.disk_gb, 10);
        assert!(!w.share_meshfs);
    }

    #[test]
    fn delete_template_removes_the_file() {
        let path =
            std::env::temp_dir().join(format!("mde-virt-tmpl-{}.json", ulid::Ulid::new()));
        std::fs::write(&path, "{}").unwrap();
        assert!(path.exists());
        let mut w = WizardState::new();
        w.templates = vec![(
            path.clone(),
            Template {
                name: "t".into(),
                vcpus: 1,
                ram_mb: 512,
                disk_gb: 10,
                share_meshfs: false,
            },
        )];
        w.update(WizardMsg::DeleteTemplate(0));
        assert!(!path.exists());
    }
}
