# MDE Voice and Tone

**Authority:** worklist UX-21 (locked 2026-05-21).
**Audience:** anyone writing user-visible strings in `crates/mde-*`,
`mackes/`, the wizard, the help docs, the CHANGELOG, error
messages, or panel labels.

Strings are part of the UI. This document is the lock on how
they read.

---

## Voice (the constant)

MDE speaks like a competent senior engineer running a tight ship.
**Direct. Specific. Calm.** Never apologetic. Never flippant.
Never marketing. The reader is busy and technically literate;
write for them.

| MDE sounds like | MDE does not sound like |
|---|---|
| "Cannot reach 3 of 8 peers. Last seen 4 min ago." | "Oops! Looks like some peers are taking a nap 😴" |
| "Snapshot `home-2026-05-21` restored. 142 files updated." | "Successfully restored your snapshot!" |
| "Run requires sudo. Re-authenticate?" | "We need permission to do this — is that ok?" |
| "Mesh quorum lost. Operations queued; will retry on quorum." | "Something went wrong. Please try again later." |

Avoid:

- **Exclamation marks** (except in error severity prefixes).
- **Emojis**. Use status icons (Material Symbols, per Q43 + EPIC-UI-MATERIAL) instead.
- **"Please"** in routine prompts. Only use it when MDE is
  inconveniencing the user beyond the default contract.
- **"Sorry"**. Lead with what happened and what to do.
- **First-person plural** ("we", "our") — implies a team behind
  the curtain. MDE is software; speak as software.
- **Synonyms in the same surface**. If a button says "Apply" in
  one panel, don't say "Save" in another. Pick one; reuse.

---

## Tone (the variable)

| Surface | Tone |
|---|---|
| Settings panels | Plain-statement, label-then-value. No flavor. |
| Wizard / onboarding | Slightly warmer; second person ("Pick a theme") is fine. |
| Error messages | Diagnostic. State the fact, then the next step. |
| Empty states | Specific + actionable. Always offer one clear CTA. |
| Confirmation dialogs | Stake the consequence before the action verb. |
| Success toasts | One sentence. State what happened, no celebration. |
| Help docs | Tutorial voice — "you do X to get Y." |

---

## Verb discipline

Pick one verb per concept. Never two.

| Concept | Use | Don't use |
|---|---|---|
| Create a new thing | **Add** | New, Create, Make, Insert |
| Persist a draft | **Save** | Apply, Commit, Store, Submit |
| Apply a config change | **Apply** | Save, Confirm, Update, Set |
| Remove a thing | **Remove** | Delete, Trash, Discard, Unset |
| Permanently destroy | **Delete** | Remove, Erase, Drop, Purge |
| Cancel an in-progress op | **Cancel** | Stop, Abort, Quit, Dismiss |
| Dismiss a transient UI | **Dismiss** | Close, Cancel, Hide, X |
| Run a discrete action | **Run** | Execute, Trigger, Launch, Start |

The Add/Remove vs Create/Delete split matters: **Remove takes a
thing out of a working set; Delete destroys it permanently.**
Removing a peer from a fleet doesn't delete its keys. Deleting a
peer does. Wire the verbs accordingly.

---

## Sentence case (not Title Case)

All button labels, menu items, panel titles, section headers,
toast messages, error messages, and dialog titles use **sentence
case** — capitalize only the first word and proper nouns.

| Sentence case ✓ | Title case ✗ |
|---|---|
| "Add peer" | "Add Peer" |
| "Confirm restart" | "Confirm Restart" |
| "Network settings" | "Network Settings" |
| "Restore snapshot" | "Restore Snapshot" |

The one exception: brand names and product names retain their
canonical capitalization. "Mackes Desktop Environment", "MDE",
"i3", "sway", "Fedora" — keep their case.

---

## Button labels

- **Verb-first.** "Restart" before "Restart now."
- **≤ 3 words.** Longer means the underlying action is
  unclear — fix the action, not the label.
- **No icon-only buttons** except for established affordances
  (close ×, expand chevron, sort arrows). Always pair an icon
  with a text label or `aria-label`.
- **Destructive actions** get a color shift (the destructive
  red, when added — currently it's the standard secondary), not
  a different shape. Confirmation dialogs carry the staking
  ("Delete `home`? This cannot be undone.").

---

## Error messages

Two-part recipe: **what happened + what to do next.** Either
half alone is incomplete.

| Bad ✗ | Better ✓ |
|---|---|
| "Connection failed." | "Cannot reach `peer-3`. Retry in 30 s or check the network." |
| "Invalid input." | "Hostname must be 1–63 ASCII characters." |
| "Permission denied." | "`mded` cannot read `/etc/wireguard/`. Re-run with sudo." |
| "An error occurred." | This entire phrase is forbidden. State the fact. |

Long error blobs get a one-line summary + an "Details" disclosure
showing the underlying log line. Never dump a stack trace into
the user's face.

---

## Empty states

Three required elements: **icon + heading + body + CTA**. (The
CTA is the make-or-break; an empty state without a clear action
fails the spec.)

```
   [icon, 32 px, line variant]

   No snapshots yet.
   Snapshots capture the state of selected directories.
   You can restore from one in seconds.

   [ Take first snapshot ]
```

Heading: 14 sp medium. Body: 13 sp regular, ≤ 2 sentences. CTA:
verb-first, single button. No "Get started" copy paths — name
the specific action.

---

## Status badges

Three states, three colors:

| State | Label | Color token | Icon |
|---|---|---|---|
| Healthy | "Online" / "Synced" / "Ready" | text-primary | filled dot, accent |
| Warning | "Degraded" / "Behind" / "Idle" | text-muted | filled dot, text-muted |
| Failed | "Offline" / "Failed" / "Quorum lost" | accent | filled dot, accent |

Status labels never use punctuation (no "Synced!", no
"Offline."). They are nouns or noun-phrases. Time qualifiers
("4 min ago") sit beside the label, not inside it.

---

## Numbers and units

- **No thousands separators** under 1,000. After 1,000, use
  comma separators ("1,024 files", not "1024 files").
- **Bytes**: KiB / MiB / GiB / TiB (binary). Show one decimal
  for ≥ 10 (e.g. "12.4 GiB"); two decimals for < 10
  ("2.34 GiB").
- **Times**: relative when ≤ 24 h ("4 min ago", "2 h ago");
  absolute thereafter ("2026-05-19 14:23").
- **Counts**: prefer specific over vague ("8 peers" not "several
  peers").

---

## Forbidden strings

Audit (`grep`) every release for these. If found, fix before
shipping:

- `TODO`, `FIXME`, `XXX`, `HACK` reachable from the UI
- `Lorem ipsum`, `dolor`, `consectetur`
- `foo`, `bar`, `baz`, `qux` as visible strings
- `test`, `testing`, `test123` as default values
- `placeholder` as a placeholder string
- `error` as the entirety of an error message
- `Oops`, `Whoops`, `Yikes`

---

## Where this doc applies

Every user-visible string in:

- `crates/mde-*/src/` (Iced views, panel labels, error text)
- `mackes/workbench/` (residual GTK surfaces during the v2.0.0
  retirement window)
- `mackes/wizard/` (onboarding copy)
- `docs/help/*.md` (in-app help)
- `data/applications/*.desktop` (app launchers)
- The `CHANGELOG.md` user-facing summary lines
- GitHub Release notes
- README.md feature copy

Internal-only strings (debug logs, panic messages, dev assertions,
test fixtures) are out of scope.

---

## Process

PRs touching user-visible strings should cite this doc in the
description or the commit body. UX-21's acceptance includes a
sweep that audits the workspace for the forbidden strings above
and corrects every violation found. After that initial sweep,
the pre-commit hook from WF-5.a could be extended to also grep
for the forbidden patterns; that's a v2.3 follow-up if needed.
