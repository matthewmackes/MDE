# Motion Language — MackesDE for Workgroups

**Locked:** 2026-05-25 via Q47 of the 100-Q tightening survey
**Status:** Functional + subtle decorative motion
**Authority:** `docs/AI_GOVERNANCE.md` §6 + this doc

## 0. Principle

Motion **serves information.** A transition exists to communicate a
state change (focus shift, surface activation, content arrival).
Decorative motion is allowed when subtle; expressive/bouncy/springy
animation is not.

> *"Subtle decorative motion, 150 ms ease-out."*

## 1. Canonical timing

- **Standard transition:** `150ms ease-out`
- **Active feedback (button press, toggle):** `100ms ease-out`
- **Information arrival (toast, popover open, tray reveal):**
  `200ms ease-out`
- **Information dismissal (toast close, popover close):**
  `120ms ease-in`

Cubic-bezier helpers (CSS):
- `ease-out` → `cubic-bezier(0.0, 0.0, 0.2, 1.0)` (Material standard)
- `ease-in` → `cubic-bezier(0.4, 0.0, 1.0, 1.0)`

## 2. Approved motion patterns

The platform ships these motion patterns; new surfaces should reach
for one of these before inventing a new shape.

### 2.1 Typewriter reveal (Portal Breadcrumb, BUS-2.2)

Characters reveal at **60 chars/sec**, no easing on the per-character
appearance. Used for Bus notification segments + Breadcrumb context
strings. Pauses at the end for `2000ms`, then either holds (urgent)
or fades (per BUS-2.2 + Round 19 TTL).

### 2.2 Marquee scroll (Portal Breadcrumb, overflow text)

Scrolls at **50 px/sec** when text overflows the segment. No ease;
linear. Pauses at start + end for `800ms` per loop.

### 2.3 Tray + popover reveal

Slides up `8px` while fading in over `200ms ease-out`. Reverses for
dismiss over `120ms ease-in`.

### 2.4 Hover focus transition

`150ms ease-out` on background-color, border-color, and outline.
Cards + list rows + buttons all share the same hover transition.

### 2.5 Layout shift

When a section expands/collapses (e.g., Workbench Bus subpage tab
content), `200ms ease-out` on height + opacity. No scale/skew.

### 2.6 Sidebar hover-expand (Classic ChromeOS lock)

Per `docs/design/chromeos-classic-spec.md`: sidebar expands
`56 → 256 px` over `200ms ease-out` on hover. Indigo selection
underlay slides into place over `150ms ease-out`.

### 2.7 Mode cycle (Portal status-zone focus glyph, deferred to BUS-2.8)

When operator clicks the DND toggle (per BUS-2.8) → glyph swap
fades cross-fade over `120ms ease-in` (out) + `120ms ease-out`
(in). No rotation/scale.

### 2.8 Stagger list reveal (ANIM-4, Q15)

When a list or grid of items enters, items reveal individually with a
staggered delay. Cap: **8 items** max stagger; items at or beyond the
cap appear at the same maximum delay (`7 × 20ms = 140ms`) so long
lists don't crawl.

- Per-item delay step: **20ms**
- Each item's reveal: **120ms ease-out** fade-in
- Items ≥ cap: appear at the cap's delay (instant relative to each
  other — they all start at 140ms)

Implementation: `mde_iced_components::motion::stagger_delay_ms(index)`
+ `motion::fade_in_alpha(elapsed_ms - stagger_delay_ms(i), 120, reduce)`.

### 2.9 Skeleton shimmer → content crossfade (ANIM-4, Q19)

While content is loading, a placeholder rectangle oscillates between
the `raised` surface tier and a lighter band to indicate activity.

- Shimmer: sine-wave alpha in **[0.15, 0.50]** over a **1200ms**
  loop period — sweeps once per 1.2 s.
- Corner radius: **4 px** (matches Inline elevation tier).
- On content arrival: crossfade from shimmer to real content over
  **150ms ease-out**.
- Reduced motion: shimmer holds a flat **0.15** alpha (visible but
  static); crossfade remains at 150ms (functional state change).

Implementation: `mde_iced_components::skeleton_shimmer(width, height,
motion::shimmer_alpha(now_ms, reduce), palette)`.

## 3. Forbidden motion (voice-and-tone lint)

The following motion verbs + descriptors must not appear in
user-visible strings or in design-doc spec text:

- "bounce" / "bouncy" / "springy" / "elastic"
- "wiggle" / "shake" / "jiggle"
- "zoom" / "swoosh" / "whoosh"
- "rotate" (except for clock hands, mesh-globe spin)
- "pop" / "punch" / "snap" (except as button-press feedback noun)
- "twirl" / "swirl"

The lint catches additions to the list as a regression detector.
Existing surfaces using forbidden verbs should be audited + corrected
under EPIC-UI-MOTION.cleanup (separate task, deferred).

## 4. Reduced-motion accessibility

When `@media (prefers-reduced-motion: reduce)` is set:
- All decorative transitions reduce to `0ms` (instant)
- Functional transitions stay (focus feedback, state change) but
  reduce duration to `60ms`
- Typewriter + marquee in BUS-2.2 + Breadcrumb fall back to
  static text (full text shown at once)

Per-surface implementation: every CSS transition declaration that
matters for motion should be wrapped:

```css
.surface-foo {
  transition: background-color 150ms ease-out;
}
@media (prefers-reduced-motion: reduce) {
  .surface-foo {
    transition: background-color 60ms ease-out;
  }
}
```

## 5. Master rule alignment

Per `docs/AI_GOVERNANCE.md` §0: when in doubt, motion choices fall
back to **"Secure, Simple, Centerless Workgroup."**

- *Simple* — fewer motion patterns; reuse the 5 approved shapes
- *Centerless* — no single peer is "the source of motion"; every
  peer renders its own transitions locally
- *Secure / Workgroup* — motion never carries semantic information
  the operator could miss (a critical alert can't be conveyed by
  animation alone; it always has a strong visual + text + sound
  via the BUS surface mapping)

## 6. Future work

- **EPIC-UI-MOTION.lint:** add motion-vocabulary lint to
  `install-helpers/lint-voice.sh` once the forbidden list is
  exercised against the codebase.
- **EPIC-UI-MOTION.cleanup:** audit existing CSS for transitions
  outside the 100/120/150/200 ms grid + bring into compliance.
- **EPIC-UI-MOTION.reduced:** add the `prefers-reduced-motion`
  wrapper to every existing transition declaration in `data/css/`.

These follow-on tasks land separately as the operator picks them up.
