# Sway-Native Shell + Maximum-Animation Design Lock

**Locked:** 2026-05-28 (150-Q survey: 100 sway-integration/animation +
50 correction/refocus, operator-driven).
**Supersedes:** `docs/design/v6.5-hyprland-compositor.md` (Hyprland
migration **fully removed**), the *universal-flat* clause of
`docs/design/chromeos-classic-spec.md`, `EPIC-UI-WALLPAPER` (Q48
static-wallpaper de-scope), the all-Intel-One-Mono Portal font lock,
and the Roboto-Mono ChromeOS font pairing.
**Authority:** operator directive — the only sanctioned path to amend
the §11 Hyprland roadmap item (`AI_GOVERNANCE.md` §11 item #19 is
removed by this lock). Mirrored in memory `sway-native-reversal`.

---

## 0. Decision

Hyprland is removed from the platform. The compositor is **vanilla
sway**, permanently. The platform integrates sway 100% and pushes
**maximum animation into the MDE-rendered layer-shell surfaces** —
windows snap (sway has no window animation); the shell is the
animation canvas. Verification is **automated + headless + CI**, not
per-machine hardware bench.

Why sway over Hyprland: sway is in Fedora repos (no CI compositor
build, no bundled binaries), needs no custom C++ plugin, and runs
headless trivially (`WLR_BACKENDS=headless`) so the whole cut is
verifiable without special hardware. Hyprland's visual ceiling (blur
/ compositor shadows / window animation) is recovered where it
matters via MDE-drawn surfaces, which iced renders itself.

---

## 1. Foundation (Q1–Q5)

- **Q1 — base:** vanilla sway. No SwayFX, no compositor effects.
- **Q2 — engine:** iced built-in animation (subscription/tick +
  interpolation), one GPU stack (wgpu) across all Rust surfaces;
  aligns with the iced 0.14 + iced_layershell 0.18 migration.
- **Q3 — timing:** keep the locked 100/120/150/200 ms grid. "Maximum
  animation" = COVERAGE (more surfaces animate), not slower motion.
- **Q4 — reduce-motion:** accessibility toggle only (honor
  `prefers-reduced-motion`); **no** battery/hardware auto-throttle.
- **Q5 — verification:** extend the HW-3 headless harness with
  worker-exercise assertions; CI-gated, hardware-agnostic.

## 2. Motion system (Q6, Q8, Q45, Q61, Q79, Q80, Q82)

- **Q6 — vocabulary:** fade + slide + scale (opacity/translate/scale
  interpolations) cover all shell motion.
- **Q8 — exits:** every surface dismisses as the reverse of its
  reveal, dropped one grid tier (reveal 200 → dismiss 150).
- **Q61 — interruptibility:** animations reverse/redirect from their
  current position; never snap or wait. (The premium-feel factor.)
- **Q45 — presets:** one shared motion system; presets differ by
  accent/color only.
- **Q79 — home of truth:** new `mde-motion` crate — the timing grid +
  fade/slide/scale + interruptible-spring primitives, consumed by
  every iced surface.
- **Q80 — tokens:** extend `data/css/motion-vocabulary.css` + add
  motion tokens to `tokens.css`; `mde-motion` reads them.
- **Q82 — frame rate:** 60 fps target, honor display refresh, throttle
  when idle.

## 3. Shell surfaces (Q7, Q9–Q20, Q42, Q44, Q49, Q50)

- **Q7** Portal-compact: slide-up + fade from the panel edge.
- **Q16** Portal-full: horizontal slide between layers (page-like).
- **Q9** Taskbar buttons: **width-expand**, pushing neighbors, on
  open/close.
- **Q10** Focus indicator: slides between taskbar buttons.
- **Q11** Mark pills: pop-in (scale+fade) + slide on reorder.
- **Q12** Notifications: slide-in from edge + stack-shift on dismiss.
- **Q13** OSD (volume/brightness): bar fills smoothly + slide-in.
- **Q14** Launcher: slide-up + staggered item reveal.
- **Q15** List/grid policy: capped ~8-item stagger everywhere, rest
  instant (long lists don't crawl).
- **Q17** Hover/press: subtle scale + brightness; **stays flat** on
  the element (transform + color only, no shadow/depth).
- **Q18** Selection/focus: highlight slides to target (all focusables).
- **Q19** Loading: skeleton shimmer → crossfade to content.
- **Q20** Urgent: bounded pulse/glow (~2 cycles) then steady accent.
- **Q42** Command palette: drop-in + live result reflow (filter/
  reorder as you type).
- **Q44** Context menus: grow from cursor + staggered items.
- **Q49** Settings nav: crossfade + slight slide + content stagger.
- **Q50** File manager: full operation motion + drag ghost + drop
  highlight.

## 4. Window / workspace experience on vanilla sway (Q21–Q25, Q46, Q55, Q56)

Windows snap; the shell supplies the motion.

- **Q21** Workspace switch: panel indicator slides + brief centered
  name OSD (suppressed under reduce-motion).
- **Q22** Workspace label change: **character morph / typewriter**.
- **Q23** Fullscreen: panel slides away + back.
- **Q24** Wallpaper: crossfade on change + per-workspace parallax
  shift (see §7 for the EtherApe wallpaper).
- **Q25** Login → desktop: sequenced reveal (greeter fades, wallpaper
  in, panel slides up, tray/pills stagger).
- **Q46** Overview: data-driven workspace switcher (cards = name +
  app icons + pills; **no** wlr-screencopy thumbnails).
- **Q55** Sway modes: mode pill in panel + which-key binding overlay.
- **Q56** App launch: clicked icon pulses until the window appears.

## 5. Visual identity — "flat-but-elevated" (Q26–Q33)

Supersedes the *universal-flat* clause of chromeos-classic-spec.md;
keeps its palette + density.

- **Q26** Windows: **borderless, gaps only**.
- **Q27** Focus cue: thin 1–2 px accent line on the focused window
  only (unfocused border width 0).
- **Q28** Tag color: focus line + that window's mark pill render in
  the tag (app-taxonomy) accent color.
- **Q29** Shadows: subtle soft shadows on **floating MDE surfaces**
  (Portal, menus, notifications, OSDs) — iced-drawn, free on sway;
  windows + docked elements stay flat.
- **Q30** Corner radius: **mixed elevation scale 4 / 8 / 12 px**
  (inline / menus-popovers / Portal-full-modals). Windows square.
- **Q31** Fonts: **Roboto (UI) + Intel One Mono (code/IDs/logs/
  terminal)**.
- **Q32** Icons: Material Symbols fill-morph (outline→filled) on
  active/selected via the variable-font fill axis.
- **Q33** Theme/preset switch: shell-wide color-token crossfade
  (~200 ms).

## 6. Feature surfaces (Q34–Q41, Q48, Q57–Q60)

- **Q34** Hub drag-to-tile: drag ghost + region highlight + drop
  flash.
- **Q35** Breadcrumb: segments slide + crossfade (directional).
- **Q36** Template apply: toast + cascading taskbar-button entries.
- **Q37** Resize: live dimension OSD + subtle scene dim.
- **Q38** Status indicators: animate state + tray enter/leave;
  transient states animate (network pulses connecting, sync spins).
- **Q39** Toggles: thumb slide + track color + panel state echo.
- **Q40** Logout/shutdown: graceful shell-out (mirror of login).
- **Q41** Mesh peers: animate enter/leave + connection-state (online
  width-expand, offline fade+desaturate, connecting pulse).
- **Q48** Lock screen: crossfade lock/unlock.
- **Q57** VoIP: ringing slide-in + looping pulse → active-call pill →
  hangup animate-out.
- **Q58** Monitoring: live sparklines + animated gauges + alert pulse.
- **Q59** Birthright wizard: horizontal step slide + progress +
  content stagger.
- **Q60** Screenshot: flash + shrink-to-corner thumbnail.

### Clipboard — signature interaction (Q43, operator-confirmed)

**Content-morph chip → fly-to-peers.**
1. On copy a chip appears and morphs its icon to the content type
   (text glyph / live thumbnail / color swatch / link / file icon)
   via the Material fill-morph. ~150 ms — *what you copied*.
2. The chip then splits into one ghost per online peer, each arcing
   to that peer's panel/Portal indicator and dissolving on arrival.
   ~200 ms — *the mesh sync, made visible*.
3. No peers online → morph chip only. Reduce-motion → static chip.

## 7. Mesh-wallpaper — EtherApe network-activity wireframe (Q76)

**Supersedes EPIC-UI-WALLPAPER (Q48 static-image de-scope).** Revives
Portal-22/23/24 + MESH-W-* + MESH-WP-* as the rendering source.

- The wallpaper renders the **MESH-W** wireframe graph: mesh peers as
  nodes, traffic as animated edges (thickness = bandwidth, color =
  protocol, animated flow = packet rate) — EtherApe-style.
- **"Least load, most animation":** vector wireframe is cheap to
  draw, yet traffic-driven flow is continuous and *meaningful*. Runs
  at **MESH-WP** lower fidelity — reduced force-solver iterations,
  no heavy particles (cheap animated edge-flow instead), 12 s
  (6–60 s configurable) update, **auto-pause when fully obscured**
  (MESH-WP-2, the battery concession), reduce-motion → static frame.
- Flagship **default** wallpaper; static image stays selectable for
  weak peers (MESH-WP-5 + varied-hardware standard). View-only on the
  wallpaper layer; interaction happens in Portal-compact (Portal-23).

## 8. Integration & packaging (Q51–Q54)

- **Q51** Installer pulls sway from Fedora repos + seeds the MDE sway
  config + mded autostart. No CI compositor build, no bundle.
- **Q52** `mde-config` generates the sway config from MDE settings;
  the file lives in mesh-home and GFS-replicates to every peer
  (configure once, mesh converges). Settings UI stays authoritative.
- **Q53** Shared config + a small per-peer hardware overlay keyed by
  hostname/EDID (output layout, scale, input quirks).
- **Q54** mded watches the config (inotify on the GFS file); on
  change → `swaymsg reload` live + affected surfaces crossfade. No
  logout.

## 9. Q62–Q100 — recommended-path locks (abbreviated)

Invalid action = shake + red pulse (Q62); idle → gradual dim →
crossfade-lock (Q63); empty states = animated icon + fade copy (Q64);
cross-peer file drop = fly-to-peer choreography (Q65); notification
center = slide panel + cascade "clear all" (Q66); **sound silent by
default**, optional pack later (Q67); scratchpad = mark-pill pulse on
summon (Q68); audio mixer = staggered rows + animated fill + live VU
(Q69); calendar/clock popover = slide-up + sliding month transitions
(Q70); wifi/BT/network pickers follow the global list+connect+icon-
morph policy (Q71); IME/layout switch = pill morph + OSD (Q72);
power-profile toggle follows Q39 (Q73); mded cold start = sequenced
reveal (Q74); default sway cursor, no custom cursor animation (Q75);
tags **are** named workspaces, one concept (Q77); apps theme
themselves, MDE does not animate app interiors (Q78); voice-tone
**never exposes "sway"/"i3"** in UI — compositor is an implementation
detail, platform is "MDE" (Q84); notification grouping expand/collapse
slide (Q95); FLIP drag-reorder for pinned apps/panel (Q96); toast
action buttons inline-expand on hover/focus (Q97); hold/long-press =
radial progress fill (Q98); reduce-motion CI asserts static render
(Q99); no forced first-run motion showcase — login + onboarding
already showcase (Q100).

## 10. Retune mechanics (Q85–Q94) — execution checklist

- [ ] This doc is canonical; archive `v6.5-hyprland-compositor.md`
  with a SUPERSEDED banner (Q85).
- [ ] Retire HYP-1..33 in the worklist; lift animation locks into new
  `ANIM-*` tasks; fold surviving sway-integration into `SWAY-*`;
  un-defer Portal-22/23/24 + MESH-W/WP (Q86).
- [ ] Scope = **1.0** (the whole backlog); no separate version (Q87).
- [ ] Discard the `hyprland-cut` worktree + branch (Q88) — its 6
  commits are pushed to `mde-x` and recoverable; salvage the
  compositor-agnostic logic first (Q94): marks pure fns + Bus
  responder (revert IPC hyprland-rs → swayipc, Q89); template emitter
  retargeted `hyprctl --batch` → `swaymsg` (Q90); `CardKind::Template`
  kept.
- [ ] Remove `AI_GOVERNANCE.md` §11 roadmap item #19 (Hyprland);
  sway-native shell + animation is the compositor line (Q91).
- [ ] Record the operator lock-lift in `.claude/CLAUDE.md` §0.16 (Q92).
- [ ] Drop planned Hyprland-specific lint gates; keep/extend the
  voice-tone lint for "sway as implementation detail"; add a
  low-effort "motion uses `mde-motion` tokens, no ad-hoc durations"
  lint (Q93).

## 11. Operator meta-directives (this survey)

- **Low-effort options only** — but the operator will choose
  flashy-when-achievable (width-expand, character-morph, EtherApe
  wallpaper); exclude only heavy engineering (custom compositor
  patches, screencopy-heavy features, new daemons).
- **Recommended path for all questions**; surface only genuine
  no-clear-path forks.
- Verification standard: automated/headless/CI, not single-machine
  bench.
