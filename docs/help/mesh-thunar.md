# Mesh in Thunar — DEPRECATED 2026-05-25

> **This doc is HISTORICAL.** The `mesh:///` URI scheme + the
> `gvfsd-mesh` GVFS backend + the `~/QNM-Mesh/` bookmark layout
> are all **RETIRED** per DEAD-2.11 of the 100-Q tightening
> survey (Q14 + Q77 lock 2026-05-25). The mesh is now natively
> reachable through your normal file manager: gluster FUSE-mounts
> `~/Documents/`, `~/Pictures/`, `~/Music/`, `~/Videos/`, and
> `~/Downloads/` directly onto the mesh-home volume per GF-4.1.
> No URI scheme, no bookmarks, no per-peer SSHFS mounts —
> every peer holds every file, and your XDG dirs ARE the mesh.
> (The underlying filesystem is LizardFS mesh-storage per MESHFS-4.1 —
> was GlusterFS mesh-home per GF-4.1.)
>
> The pre-DEAD-2.11 mesh:// behavior below is retained only
> for archeology + cross-referencing legacy code paths still
> being retired.

---

# Mesh in Thunar (legacy, pre-DEAD-2.11)

Mackes ships a `gvfsd-mesh` GVFS backend that exposes the mesh as a
browsable filesystem under the URI scheme `mesh:///`. Works inside Thunar,
gvfs-mount, the GTK file picker, and any other GVFS-aware app.

## Entry points

Three ways to land on `mesh:///`:

1. **Sidebar entry** — "Mesh" under the Network section of Thunar's
   sidebar. Always one click away.
2. **Location bar URI** — type `mesh:///` in Thunar's location bar.
3. **Bookmarks** — `~/QNM-Mesh/`, `~/QNM-Clipboard/`, `~/QNM-Notifications/`,
   `~/QNM-Drop/` shortcuts in `~/.config/gtk-3.0/bookmarks`.

## Layout

```
mesh:///
├── Peers/
│   ├── peer-A/        live SSHFS mount of peer-A's ~/QNM-Shared/
│   ├── peer-B/        (greyed + "offline since 14:32" if offline)
│   └── …
├── Clipboard/
│   ├── mine/          your last 100 clipboard items
│   │   ├── 2026-05-16T14-32-08_a3f9.png
│   │   ├── 2026-05-16T14-31-44_c712.txt
│   │   └── Saved/     pinned items (uncapped)
│   └── peer-A/
│       ├── …          peer A's last 100 items
│       └── Saved/
├── Notifications/
│   ├── mine/  …_<id>.md   bold = unread
│   └── peer-A/ …
└── Object Store/
    ├── Themes/        versioned blobs (right-click → Show versions)
    ├── Snapshots/
    ├── Presets/
    └── Drop/          generic file drop
```

## Live updates

Open Thunar windows refresh in real time. qnmd subscribes to NATS events
(`clipboard.new`, `notification.new`, `peer.up`, `peer.down`,
`object.put`) and triggers FUSE invalidation on every event. Typical
latency: ~ms.

## Clipboard

- Each item is a real file with its native MIME extension (`.png`, `.txt`,
  `.html`, `.bin`).
- Filename = `<ISO-timestamp>_<short-hash>.<ext>`.
- Drag-out copies the content into other Thunar windows or apps.
- Double-click opens with system default app.
- Right-click → Copy to local clipboard.
- Right-click → Pin moves the item into `Saved/`.
- The 100-item ring rolls oldest-first; pinned items survive forever.

## Notifications

Each notification is a markdown file:

```markdown
---
peer: laptop-mm
timestamp: 2026-05-16T14:32:08
urgency: normal
app: org.mozilla.firefox
icon: web-browser
---

# Download complete
"linux-mint-22.iso" finished downloading to ~/Downloads/.
```

Attachments (screenshots, files) are sibling files with the same prefix.
Unread notifications render with bold filename + a dot badge in Thunar.
First view marks read. Manual Delete propagates to the originating peer.

## Object Store

NATS Object Store buckets. Each bucket maps to a folder under
`Object Store/`. Files inside are versioned — right-click → "Show
versions…" lists prior revisions with size/timestamp/uploader-peer, with
Restore and Open buttons.

Drag any file into a bucket folder → uploaded, replicated to all peers,
visible everywhere within ~ms.

### Conflict resolution

Two peers write the same key → last-write-wins; older write preserved as
a prior revision (visible via "Show versions…").

## Drop targets

Drag a file from your desktop onto `mesh:///` or its sidebar entry → a
destination picker pops up (bucket + optional target peer). No silent
defaults.

xfdesktop also gets a right-click "Drop on mesh…" item that opens the
same picker without needing to open Thunar first.

## Right-click menu

Available on any mesh item:

- **Copy to local clipboard** (clipboard items)
- **Send to peer…** (any file)
- **Pin / Unpin** (clipboard, notifications)
- **Delete from mesh** (with confirm)
- **Save as File** (export to a chosen local path)
- **View** (preview without opening/copying)
- **Open** (with system default app)
- **Show versions…** (Object Store files)

## Search

The Mesh root has a search box at the top — queries every peer's
clipboard + notifications + Object Store. Results show with peer-of-origin
badges. Per-folder search still uses native Thunar Ctrl+F.

## Per-peer offline behavior

Offline peers remain in the tree, greyed out, with a "offline (since HH:MM)"
badge. Clicking shows an empty pane with a Reconnect button. No
last-known-state cache (avoids stale-state surprises).

## Permissions

- All four subtrees are read-write by default.
- Per-item RO via right-click → Make Read-Only.
- Per-peer overrides in Mackes → Network → QNM → Mesh Filesystem.

## Thumbnailer

Mackes ships a Tumbler thumbnailer plugin that renders rich previews for:
- Clipboard items (images scaled, text/HTML previews, audio waveforms)
- Notification `.md` files (Carbon-styled card preview in Thunar's
  preview pane)
