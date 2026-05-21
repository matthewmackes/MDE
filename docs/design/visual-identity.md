# MDE Visual Identity

**Status:** Lock (2026-05-21, via the 50-Q survey + FU + NFU rounds).
**Authority:** `docs/PROJECT_WORKLIST.md` § UX Design Locks.
**Audience:** anyone designing or implementing for MDE.

This document narrates the design system that was decided across
three rounds of locks in a single 2026-05-21 session. It does not
re-litigate decisions; every section cites the survey Q-IDs that
locked it. Where two locks interact (e.g., density × component
size), the resolved interpretation is documented inline.

---

## 1. Brand vision

**MDE is calm enterprise.** Q1 locked the metaphor as **Apple
System Settings minimalism** — neutral surfaces, generous spacing,
a single restrained accent, soft elevation. We are not building
a terminal. We are not building a cyberpunk console. We are
building the desktop equivalent of a high-end settings panel: a
tool that gets out of the way until the user asks for something,
and then answers without flourish.

**Reference targets** (cited from the Round 2 brief and UX-11):
Linear, Raycast, Arc, Cursor, Vercel dashboard, Apple macOS
Sonoma System Settings.

**What MDE explicitly is not:**

- Not playful. Animations are decisive, not bouncy. No springs
  with overshoot. No mascots.
- Not glassmorphic. We use one blur effect (modal backdrop,
  Q44) — everywhere else, opaque surfaces with hairline borders.
- Not skeuomorphic. No textures, no faux materials, no shadows
  pretending to be physical lighting.
- Not maximalist. Information density is high but visual noise
  is low. Whitespace is structural, not decorative.
- Not terminal / cyberpunk. The Round 2 "deep-night terminal +
  command room" direction was rejected at Q1.

---

## 2. Palette

| Token | Dark | Light | Locked at |
|---|---|---|---|
| Background  | `#1d1d1f` | `#f5f5f7` | Q3 (dark only; light derived) |
| Surface     | `#2a2a2c` | `#ffffff` | Q4 (4-tier elevation) |
| Raised      | `#38383a` | `#f0f0f2` | Q4 |
| Overlay     | `#48484a` | `#e5e5e7` | Q4 |
| Accent      | `#5b6af5` (indigo) | same | Q2 |
| Border (dark)  | `rgba(255,255,255,0.08)` hairline | `rgba(0,0,0,0.12)` 1 px solid | Q7 (adaptive) |
| Text primary   | `rgba(255,255,255,0.92)` | `rgba(0,0,0,0.88)` | derived for AAA contrast |
| Text muted     | `rgba(255,255,255,0.55)` | `rgba(0,0,0,0.55)` | derived |

**Both themes ship together in v2.2** (Q5 + FU-2 full parity).
**First-launch theme prompt** is part of the wizard step set (Q6 +
UX-16).

### Why a single accent

The Round 2 proposal explored secondary / tertiary accents (status
greens, warning yellows). The locks rejected those: status reads
through icon shape (filled vs line, Q38), not color, and the only
non-neutral color in the system is the indigo accent. This
constraint forces hierarchy via type and spacing rather than color
volume — which is what Apple System Settings does, and why it
reads as "calm" rather than "decorated."

### Border philosophy (Q7)

Hairlines in dark mode (`rgba(255,255,255,0.08)`) are barely
visible at rest, but they prevent surfaces from blurring together
when they're at adjacent elevation tiers. In light mode, hairlines
disappear against the bright field, so we use a 1 px solid border
at the standard text-muted color. The same component spec produces
two visually-correct results in the two themes.

---

## 3. Typography

**Single-family system** — Geologica for both display and body
(Q11, Q12). Geologica is variable, so the entire scale (and a
slight optical-size shift between display and body) is one font
file. Less network / disk weight than a Display + Body pairing,
and a stronger brand signature.

| Role | Size (sp) | Weight | Tracking | Locked at |
|---|---|---|---|---|
| Display     | 28 | 500 | tight (-1.5%) | Q11, Q14, Q15 |
| Heading     | 20 | 500 | tight (-1%)   | Q14 |
| Subheading  | 17 | 500 | default       | Q14 |
| Body        | 14 | 400 | default       | Q12, Q14 |
| Caption     | 12 | 500 | default       | Q14 |
| Mono        | 13 | 400 | default       | Q13 |

**Type scale** is **1.2 minor third** (Q14): 12 / 14 / 17 / 20 /
24 / 28 sp. Calm progression — Apple System Settings sits near
this ratio. Larger scales (1.25 major third, 1.333 perfect fourth)
were rejected for being too "editorial."

**Monospace** is **IBM Plex Mono** (Q13) for paths, IDs, peer
addresses, and any code-like content. Pairs visually with
Geologica's geometric character.

**Optical sizing** (Q15) — tracking tightens on display sizes,
defaults on body. Implemented via Geologica's `opsz` variable
axis. Body copy gets no tracking adjustment; reading rhythm wins
over visual rhythm at 14 sp.

---

## 4. Iconography

**Carbon icons** (Q24) — the platform requirement that overrode
Round 2's Lucide/Phosphor proposal. Carbon's 1 px line set is the
canonical source.

| Size | Use |
|---|---|
| 16 px | Inline-with-text (button glyphs, list-row leading icons) |
| 20 px | Sidebar nav |
| 24 px | Panel section headers |
| 32 px | Empty states |
| 48 px | Wizard hero / onboarding |

**Stroke weight** is **1 px** (Q39) — Carbon's standard. We do not
diverge to 1.5 px or 2 px even at small sizes; if 16 px reads as
wispy, the fix is to bump the line weight UX-locally via a Carbon
"compact" variant, not to override the standard.

**Style** is **mostly line, filled only for status dots and
notifications** (Q38). Active nav items keep the line variant —
the active state is signaled by background fill (§ 5), not by
swapping to a filled icon. Filled icons are reserved for binary
state indicators where shape recognition is the load-bearing cue
(online/offline dot, unread/read bell, dirty/clean snapshot).

---

## 5. Window chrome and navigation

### Window decorations (Q16, Q17, Q18, Q19, Q20)

**Hybrid CSD/SSD:** when the window is floating, MDE draws its own
header (client-side decorations); when tiled under i3/sway, the
compositor draws the title bar (server-side decorations).
Detection: `xdg-decoration-unstable-v1` on Wayland;
`con.floating` via i3-IPC on XOrg.

CSD header:
- **44 px tall** (Q17 — Apple compact).
- **20 px MDE icon on the left** (Q19), no wordmark text.
- **Window controls** (min/max/close) **hover-revealed** on the
  right (Q18 — Arc-style). At rest, the header is clean; on cursor
  approach within 32 px of the right edge, the trio fades in over
  120 ms.
- **Layered shadow** — 1 px hairline ring (matches the adaptive
  border from § 2) + 16 px ambient drop shadow (Q20).

### Sidebar (Q21, Q22, Q23, Q25)

- **240 px wide** (Q21).
- **32 px nav rows** (Q25 — compact, VS Code-style). Component
  height is invariant across densities (UX-24 sub-lock).
- **Active row** uses the **inset/sunken** treatment (Q22): the
  row's background drops one elevation tier (surface → background)
  rather than rising. This is the trick Apple System Settings
  uses for selected list items.
- **Section dividers** are all-caps muted labels at 11 sp, no rule
  lines (Q23). Gap rhythm: 4 px above, 8 px below the label.

---

## 6. Interactive states

### Hover (Q8)

`Indigo @ 8% opacity` translucent wash. Linear-style. Same recipe
for buttons, nav items, list rows, and dropdown items. The
opacity rises to 12% on `:active` (mouse-down).

### Focus-visible (Q9)

`1 px accent ring + 2 px outer halo at low opacity` — Stripe and
Vercel use this layered approach. Crisp inner ring for keyboard
navigation legibility; soft outer halo prevents the ring from
feeling harsh. Only shows on keyboard focus (`:focus-visible`),
not on mouse-down focus.

### Disabled (Q10)

`Desaturated + 60% opacity` with `cursor-default`. The component
keeps its layout footprint but loses color saturation. Apple uses
this; it reads as "not now," not "broken."

### Loading (Q43)

**Combined:** skeleton shimmer for content blocks, 1 px progress
bar at the panel top for navigation transitions. Skeleton uses
the surface tier with a 1.5 s linear shimmer in indigo @ 4%
opacity. Progress bar is indeterminate, 180 ms cycle.

---

## 7. Buttons and inputs

### Buttons (Q40, Q41)

- **Primary** = outline + accent text + transparent fill at rest;
  fills to `indigo @ 12%` on hover, `indigo @ 100%` (solid) on
  active. This is the lock that surprises most reviewers: most
  design systems use solid-accent for primary CTAs. MDE's
  outline-first locks the calm direction — primary is *defined*,
  not *loud*.
- **Secondary** = no border, muted text, hover → surface +1
  elevation.
- **Ghost** = text-only, hover → indigo @ 8%.
- **Radius** = **8 px** (Q41). Matches the broader Apple corner
  family.
- **Height** = 36 px (standard), 28 px (compact), 44 px (hero /
  wizard).

### Inputs (Q42)

- **1 px hairline border** at rest (adaptive per Q7).
- **Focus** = accent border + subtle inset shadow (Apple-style),
  not a halo glow.
- **Height** = 36 px.
- **Radius** = 6 px (one notch tighter than buttons to read as
  "data entry" rather than "action").

---

## 8. Modals and dialogs (Q44, Q45, Q46)

| Property | Value | Locked at |
|---|---|---|
| Backdrop | `4 px gaussian blur, no tint` | Q44 |
| Corner radius | `16 px` | Q45 |
| Max-width | `640 px` | Q46 |
| Shadow | `SHADOW_3` (24 px blur, 30% opacity, 8 px y-offset) | derived |
| Dismiss | Esc + click outside content rect | UX-27 sub-lock |

The blurred backdrop with no tint is the *one* glass-like effect
in the system. Reserved for modal moments — confirm dialogs,
logout dialog, the notification center modal. The command palette
(Q34) uses a different chrome (semi-transparent, **no backdrop**)
and is not considered a modal in the dialog sense.

---

## 9. Motion (Q29, Q30, Q31, Q32)

**Personality:** calm + decisive (Q29). Nothing bounces. Nothing
overshoots. Motion happens, then stops cleanly.

**Standard duration:** 180 ms (Q30). Tiers:

| Tier | Duration | Use |
|---|---|---|
| Micro | 120 ms | Hover state shifts, tooltip fade-in |
| Standard | 180 ms | Panel mounts, modal open, palette open |
| Hero | 280 ms | Wizard step transitions, full-screen sheets |

**Easing** (Q31): per-direction.

- Enter → `cubic-bezier(0.22, 1, 0.36, 1)` (ease-out, decelerate to
  stop).
- Exit → `cubic-bezier(0.65, 0, 0.35, 0)` (ease-in, accelerate
  away).

This is the iOS HIG approach — distinct curves for "arrival" vs
"departure." Saves 1–2 % of perceived sluggishness compared to a
single ease-in-out everywhere.

**Reduced motion** (Q32): when `prefers-reduced-motion` is set,
every transition collapses to an 80 ms cross-fade. Spatial motion
(translate, scale) is replaced by opacity. Vestibular safety
preserved without going fully instant.

---

## 10. Density and spacing

### Spacing scale (NFU-1)

Locked at **`4 / 6 / 8 / 10 / 14 / 17 / 20 / 24 / 28 / 34 / 40 /
48 px`** — 12-step modular set derived from the 1.2 type scale
(NFU-1). UX-12's grid lint enforces this list exactly; no off-list
literal values appear in `Length::Fixed(n)`, `padding(n)`, or
`spacing(n)` calls anywhere in `crates/mde-*`.

### Density modes (Q26, Q27, UX-24)

Three user-selectable modes via Settings > Appearance:

| Mode | Spacing multiplier |
|---|---|
| Compact | 0.75× |
| **Comfortable** (default) | 1.0× |
| Spacious | 1.25× |

**UX-24 sub-lock:** density multiplies **spacing tokens only**,
never **component dimensions**. The nav row stays 32 px in all
three modes; the gaps between rows flex. Buttons stay 36 px;
their internal padding flexes. This preserves the WCAG 2.5.5
touch-target floor (24 px) at every density.

---

## 11. Command palette (Q33, Q34, Q35, Q36, UX-27)

A first-class fixture of the system, not a power-user easter egg.
Triggered by **Ctrl+K** (Q33). Spotlight-style chrome (Q34):
centered, semi-transparent, **no backdrop** (the one place we
break from the modal blur of § 8). Responsive 640 → 800 px width
(Q35) — grows to fit the longest visible result. Default view is
**category tabs** (Q36): Commands / Peers / Files / Settings.
Tab cycles tabs; arrows cycle within the active tab; Enter
activates; Esc dismisses; click outside the palette rect also
dismisses (UX-27).

---

## 12. Iconography and asset lineage

The MDE app icon is being refined from the **MAP2-audio mark**
(Q50): a 4-square grid on a rounded blue square. The source SVG
is in-tree at
`docs/design/v2.2-icon-source/map-icon.svg`. UX-17 evolves it for
MDE — same family geometry, recolored to MDE indigo + charcoal.
The visual lineage to MAP2 is intentional and preserved, not
erased.

---

## 13. Authorities

This document is a narration. The decisions live in
`docs/PROJECT_WORKLIST.md`:

- **Q1..Q50** — sequential survey, 2026-05-21.
- **FU-1..FU-4** — follow-up clarifications, same session.
- **NFU-1..NFU-4** — next-batch locks, same session.
- **UX-24..UX-28** — Round 3 design review, same session.

Where this document and the worklist disagree, the worklist wins.
This document follows the worklist; it does not lead it.
