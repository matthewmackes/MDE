# Modal pattern — canonical lock

**Locked:** 2026-05-22
**Status:** Design lock. Supersedes any prior ad-hoc per-surface
modal chrome. Every new modal surface in the codebase must follow
the pattern documented below.
**Reference implementation:**
`crates/mackes-panel/src/notification_center.rs` (GTK,
shipped 2026-05-19).
**Iced port target:**
`crates/mde-workbench/src/panel_chrome.rs::modal` builder
(MOD-1.b, v2.3 scope) — extends the existing `dialog`
(confirm) builder rather than replacing it.

---

## 0. Why this lock exists

Until 2026-05-22 MDE shipped two unrelated modal chromes:

| Surface | Chrome | Size | Use case |
|---|---|---|---|
| `panel_chrome::dialog` (UX-9, shipped) | Iced confirm shell | **480 px max-width** | Yes/no confirms (snapshot-restore, "Are you sure?") |
| `notification_center::open()` (shipped 2026-05-19) | GTK toplevel | **960 × 640** | Content-rich inspector (list + tree + per-row actions) |

Future surfaces (Tray Icons Live, Peer Connection Card, future
inspectors) drifted toward inventing a third chrome each time
("a 360 px drawer, but with a hero...", "a 720 px master-detail
panel..."). The lock kills the drift: there are exactly **two**
modal sizes in MDE, and both come from one design doc.

---

## 1. The two-size lock

| Size | Tier | Use case | Builder | Max width | Layout |
|---|---|---|---|---|---|
| **S** | Confirm | Single-question dialogs ("Discard changes?"), one-shot prompts, error toasts that escalate | `panel_chrome::dialog` | **480 px** | Single column. Body + 1–3 actions. |
| **L** | Content-rich | Inspectors, lists, trees, hero cards, anything with > 1 logical section | `panel_chrome::modal` (MOD-1.b) | **960 × 640 fixed** | Header → optional summary strip → scrolling body → optional sticky footer. |

There is no M tier. Surfaces that feel "too big for S, too small
for L" should pick L and use whitespace; the dimensional
inconsistency cost outweighs the wasted pixels.

---

## 2. The L-tier (Notification Center) pattern

This is the pattern every new content-rich modal must follow. The
section numbers below match the GTK reference in
`crates/mackes-panel/src/notification_center.rs`.

### 2.1 Window properties

| Property | Value | Source |
|---|---|---|
| Default size | **960 × 640** | NC `set_default_size(960, 640)` |
| Position | **Centered on the active output** | NC `WindowPosition::Center` |
| Keep above | **true** | NC `set_keep_above(true)` |
| Skip taskbar | **true** | NC `set_skip_taskbar_hint(true)` |
| Resizable | **true** | NC `set_resizable(true)` (down to ~720 × 480, up to ~1280 × 800) |
| Decorations | **client-side (none from compositor)** | NC `set_decorated(false)` — modal draws its own header |
| Type hint | `WindowTypeHint::Dialog` | NC line 133 |
| Backdrop | **50% black full-fill overlay** | matches UX-9 `dialog_tokens::BACKDROP_OPACITY` |
| Backdrop click | **dismisses** | per UX-27 lock (auto-mark-read-on-close fires on the way out) |
| Esc key | **dismisses** | NC `connect_key_press_event` |
| Close button | **× in the header, far right, no relief** | NC `mackes-nc-close` |

The 50% backdrop + click-outside-dismiss is the single
behavioral lock that makes a surface "modal" in MDE. Surfaces
that don't dim the parent are *not* modals — they're drawers
(PC-1 peer card) or popovers (clock, weather).

### 2.2 Outer chrome

```
┌──────────────────────────────────────────────────────────┐
│  Title             count-meta        [Action]  [×]       │  ← header, 28 px margins
├──────────────────────────────────────────────────────────┤
│                                                          │
│  ┌─ LATEST ──────────────────────────────────────┐       │  ← optional "hero strip"
│  │ card · card · card                            │       │     (top-3 by recency or
│  └───────────────────────────────────────────────┘       │      priority, max 3)
│                                                          │
│  ALL ITEMS                                               │  ← section header,
│  ▸ group-a   3 unread / 12 total                         │     all-caps, muted
│      card                                                │
│      card                                                │
│  ▸ group-b   0 unread / 5 total                          │     ← tree grouping,
│      card                                                │       per-group meta
│      card                                                │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

Outer container is a vertical `Box` with three children:
1. **Header** (`pack_start`, `expand=false`) — non-scrolling,
   stays pinned during scroll.
2. **ScrolledWindow** (`pack_start`, `expand=true, fill=true`) —
   the body. Vertical scrolling only (no horizontal). Adjustable
   policy: `Never × Automatic`.
3. **Footer** (optional, `pack_end`, `expand=false`) — sticky.
   Currently unused by Notification Center; reserved for future
   modals that need a "selected items: 3 · Apply" affordance.

### 2.3 Header

| Slot | Content | Style |
|---|---|---|
| **Left** | Title text + count meta (`"5 unread · 12 total"`) | Title: `mackes-nc-title` (20 sp medium, palette.text). Count: `mackes-nc-count` (14 sp regular, palette.text-muted), 12 px gap from title. |
| **Right** | One or more action buttons + close × | `ReliefStyle::None` (no button background until hover). Hover tints with `palette.surface-2`. Close × hover-tints with `palette.danger`. AT-SPI names mandatory. |

The header has **20 px top margin**, **8 px bottom margin**,
**28 px left + right margins**. These are absolute pixels, not
density-aware — the L-tier modal has its own spacing scale
because density-aware spacing inside a fixed 960 px shell looks
visually inconsistent across compact / comfortable / spacious.

### 2.4 Body (scrolling list)

The body has **12 px top margin**, **20 px bottom margin**,
**28 px left + right margins** (matching the header). Vertical
gap between children is **12 px**.

Content pattern (NC reference, but generalizes):

1. **Optional hero strip ("LATEST" section).** Top 3 most-recent
   or highest-priority items, rendered as full-width cards.
   Header label: all-caps 11 sp medium (`mackes-nc-section`,
   palette.text-muted, 0.08em letter-spacing).
2. **Tree grouping ("ALL ITEMS" section).** Items grouped by a
   primary axis (node, app source, category). Each group has a
   header with `▸ group-name  —  meta` and per-group counters.
   Children indent by **20 px from the left margin** (NC line
   307 — `set_margin_start(20)` when `in_tree = true`).

Cards inside the tree are visually identical to cards in the
LATEST section except for the indent. Same chrome, same actions,
same density.

### 2.5 Card chrome

```
┌──────────────────────────────────────────────────┐
│ subhead  ·  meta                  ┌──┬──┬──┐     │
│ Title                             │✓ │⧉ │🗑 │     │
│ Body text (line-wrapped)          └──┴──┴──┘     │
└──────────────────────────────────────────────────┘
```

| Slot | Content | Style |
|---|---|---|
| **Subhead** | App / source / category name + relative time meta | 12 sp medium, palette.text-muted |
| **Title** | One-line summary | 14 sp medium, palette.text |
| **Body** | Optional long-form, line-wrapped | 14 sp regular, palette.text, max ~4 lines before truncation |
| **Action strip** | 3 affordances by default: confirm · copy · dismiss. Each is `ReliefStyle::None`, hover-tints with surface-2. AT-SPI name mandatory. | Right-aligned, vertically centered. |

Cards have **8 px top + bottom internal padding** and **12 px
left + right internal padding**. Unread / attention state adds
a `unread` CSS class that the theme adapts to the accent
treatment (NC: subtle left-edge accent stripe, palette.accent at
60 % opacity, 2 px wide).

### 2.6 Live refresh

Modals that surface data from a mesh-replicated cache file
(`~/.cache/mackes/notifications.json`,
`~/.cache/mde/peers/*.json`, `~/.cache/mde/tray/*.json`, ...)
**must** re-read the file every **2 seconds** while the modal is
visible and re-render the body. NC reference:
`glib::timeout_add_local(Duration::from_secs(2), ...)` with a
`window.is_visible()` guard to break the loop on close.

Live refresh is what makes mesh-pushed state surface without the
user reopening the modal. It's not optional — a "frozen modal"
on data that may be updated by another peer is a bug in this
pattern.

### 2.7 Dismissal and side effects

| Dismissal path | Side effect |
|---|---|
| Esc key | Plain close. No mutation. |
| Click outside (backdrop) | Plain close. No mutation. |
| Close button (×) | **Auto-mutate-on-close** is allowed (NC marks all-read on close). Document this per surface — surfaces without it leave it off. |
| Action button mutation (Clear all, Mute all, etc) | Mutates immediately; rerender the list. Does not close the modal. |

The auto-mutate-on-close rule is explicit per surface in its own
design doc, never inherited silently. NC's auto-mark-read is
locked in the handoff bundle and called out in `open()`'s
docstring at line 206.

### 2.8 Empty state

When the data source is empty, the body renders a single
centered label with **40 px top + bottom margins** and copy that
hints at where the data comes from
(NC: `"(no notifications — mesh history syncs here)"`).

The empty state does **not** show the LATEST / ALL ITEMS
section headers — those would be lying about there being content.

---

## 3. The S-tier (confirm) pattern

For completeness — this is the pattern for confirm dialogs.
Documented for clarity; no changes to the existing UX-9 lock.

| Property | Value | Source |
|---|---|---|
| Max width | **480 px** | `dialog_tokens::MAX_WIDTH` |
| Width | `Length::Shrink` (sizes to content) | `panel_chrome::dialog` |
| Padding | **SPACE_24 inner**, density-aware | `MdeSpace::for_density` |
| Corners | **16 px** (`Radii::modal`) | `Radii::defaults()` |
| Shadow | `Shadow::modal()` | `MdeShadow::modal()` |
| Background | `palette.raised` | NC + dialog match |
| Border | 1 px `palette.border` | dialog only |
| Backdrop | 50% black, `dialog_tokens::BACKDROP_OPACITY` | UX-9 lock |
| Esc + click-outside | dismiss | Same as L-tier |

S-tier surfaces are used **only** for:
- Yes/no confirms with no body content beyond a short prompt.
- Single-input forms with no list / tree / table.
- Error escalations that exceed a toast.

If a surface has multiple sections, a list, a tree, or scrolling
content, it is **not** S-tier. Promote to L.

---

## 4. When to use each pattern (decision table)

| Surface type | Tier | Example |
|---|---|---|
| Yes / no confirm | S | "Discard unsaved changes?" |
| Single-input form | S | "Rename profile to:" |
| Error escalation (no remediation list) | S | "Mesh fabric unreachable. Retry?" |
| Inspector with multiple sections | **L** | Tray Icons Live, Notification Center, Peer Connection Card |
| List / tree / table viewer | **L** | Snapshot history, Run history, Fleet revisions detail |
| Settings panel | Neither — use Workbench | (Modal settings are an anti-pattern; route to Workbench) |
| Tooltip / popover | Neither — use `tooltip` chrome | Status cluster hover, clock popover |
| Slide-in side panel | Neither — use `mde-drawer` | Notification drawer, side-panel inspectors |

The drawer pattern (`mde-drawer`, 360 px slide-in) is its own
chrome, separate from modals. Drawers attach to a screen edge
and don't dim the parent; modals center and dim. The Peer
Connection Card (PC-1) is a drawer, not a modal — its
360 px slide-in chrome stays as locked.

---

## 5. Accessibility lock

Every L-tier modal must ship:

1. **AT-SPI metadata on every interactive widget.** Window title
   maps to the AT-SPI accessible name. Action buttons carry
   action-verb names ("Clear all notifications", not "Clear
   all"). Per-row action buttons carry per-row context ("Mark
   notification 'Build failed' as read", NC line 341).
2. **Full keyboard navigation.** Tab cycles through the action
   strip in the header, then into the body in document order.
   Arrow keys navigate inside the body. Enter activates the
   focused card's primary action. Esc dismisses.
3. **Screen-reader friendly section headers.** "LATEST" and
   "ALL ITEMS" headers expose their section role to AT-SPI so
   Orca announces the grouping.
4. **Reduced motion compliance.** The 2-second live refresh
   skips the animation tween when `prefers-reduced-motion`
   is set; instead, content swaps instantly.
5. **High-contrast variant.** The card's `unread` accent stripe
   uses `palette.accent_high_contrast` when the high-contrast
   variant is active (UX-22, shipped data layer).

These five points are MDM-9 (a11y workstream) deliverables but
locked here so future modal authors can't claim ignorance.

---

## 6. Iced port — `panel_chrome::modal` builder (MOD-1.b)

The GTK reference implementation lives in
`crates/mackes-panel/src/notification_center.rs`. The Iced port
ships as a builder in `panel_chrome.rs` alongside the existing
`dialog` builder. Shape:

```rust
pub struct ModalConfig<'a, Message: 'a> {
    /// Window title shown in the header left.
    pub title: &'a str,
    /// Optional count meta beside the title (e.g. "5 unread · 12 total").
    pub meta: Option<&'a str>,
    /// Header right-side action buttons (label + verb).
    /// Renders right-to-left, with `×` always last.
    pub header_actions: Vec<ModalHeaderAction<Message>>,
    /// Optional hero strip (top-3 cards, rendered above the tree).
    pub hero: Option<Element<'a, Message>>,
    /// Tree body. Section headers + groups + cards.
    pub body: Element<'a, Message>,
    /// Optional sticky footer.
    pub footer: Option<Element<'a, Message>>,
    /// Esc / click-outside dismiss message.
    pub on_dismiss: Message,
    /// Optional live-refresh tick (None = static modal).
    pub refresh_every: Option<std::time::Duration>,
}

pub fn modal<'a, Message: 'a + Clone>(
    cfg: ModalConfig<'a, Message>,
    palette: Palette,
) -> Element<'a, Message> { ... }
```

The builder enforces:
- 960 × 640 default size.
- Centered on the active output via layer-shell anchoring.
- 50% backdrop overlay via `dialog_backdrop()` (re-used).
- Esc + click-outside both fire `on_dismiss`.
- AT-SPI metadata for the title and every header action.
- Live-refresh subscription wired automatically when
  `refresh_every` is `Some`.

Surfaces that need to break the pattern (custom shape, custom
backdrop opacity, etc) **must** open a follow-up task to amend
this doc rather than bypassing the builder. Drift is how design
systems die.

---

## 7. Surfaces that adopt this pattern (worklist track)

| Surface | Workstream | Status |
|---|---|---|
| Notification Center (GTK) | shipped 2026-05-19 | **Reference implementation** |
| `panel_chrome::modal` Iced builder | MOD-1.b (v2.3 scope) | Open |
| Notification Center (Iced port) | MOD-1.c (v2.3 scope) | Open |
| Tray Icons Live | TR-3 (v2.3 scope) | Open |
| Snapshot history viewer | (future, MDM-3 follow-up) | Open |
| Run history detail | (existing `run_history` panel — *consider* promotion from panel to modal) | TBD |
| Fleet revisions detail | (existing `fleet_revisions` panel — *consider* promotion) | TBD |

When proposing a new modal surface, the design lock cites this
document by section number and declares any deviation. Surfaces
that don't cite are reviewed against this doc on PR and bounced
until they do.

---

## 8. Anti-patterns (don't do these)

1. **Custom modal sizes per surface.** "A 720 × 480 modal feels
   right for this one" — no. Pick S or L.
2. **Modals without backdrops.** A surface that doesn't dim the
   parent is not a modal. Use a drawer or popover.
3. **Modals that don't dismiss on Esc / click-outside.** Both
   are non-negotiable. The only exception is destructive-action
   confirms that block click-outside to prevent accidental
   dismissal — and even those must respect Esc.
4. **Multi-step modals (wizards inside modals).** Wizards belong
   in the full-screen wizard surface (`crates/mde-wizard/`),
   not crammed into a 960 × 640 box.
5. **Modals as settings panels.** Settings live in Workbench.
   If a workflow needs more than a confirm + one input, it's a
   Workbench panel, not a modal.
6. **Suppressing the live refresh.** If the data source is
   mesh-replicated, refresh every 2 s. Surfaces that "freeze on
   open" lie to users about what they see.
7. **Hiding the close button.** The × is mandatory. Esc and
   click-outside are not enough on their own — they're
   discoverable only after the user already knows them.

---

## 9. Changelog

| Date | Change |
|---|---|
| 2026-05-22 | Initial lock. Locks the 2-tier pattern, locks 960 × 640 L-tier, locks the live-refresh + AT-SPI + auto-mark-on-close behaviors. Opens MOD-1 + MOD-1.b + MOD-1.c follow-up tasks in the worklist. |
