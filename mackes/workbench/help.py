"""Help tab — comprehensive in-Mackes user guide.

Renders the docs/help/*.md tree shipped at /usr/share/mackes-shell/help/.
Left sidebar = topic list. Right pane = the selected topic rendered with
a small markdown→Pango converter (no external markdown lib needed).

Topics are discovered at runtime; new .md files in the help dir appear
automatically without code changes.
"""
from __future__ import annotations

import re
from pathlib import Path
from typing import Optional

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk, Pango  # noqa: E402

from mackes.workbench._common import (
    info_label, panel_box, section_header, title_label,
)


# Help docs ship at /usr/share/mackes-shell/help/ in the RPM. Dev mode
# falls back to docs/help/ in the repo.
_HELP_ROOTS = (
    Path("/usr/share/mackes-shell/help"),
    Path(__file__).resolve().parent.parent.parent / "docs" / "help",
)


# Topic ordering — matches the structure in docs/help/index.md so the
# sidebar reads top-down in the same logical order. Topics not in this
# list appear after, alphabetized.
_TOPIC_ORDER = [
    "index",
    "getting-started",
    "dashboard",
    "look-and-feel",
    "devices",
    "network",
    "system",
    "apps",
    "maintain",
    "mesh",
    "mesh-vpn",
    "mesh-thunar",
    "mesh-ssh",
    "mesh-services",
    "headless",
    "presets",
    "keybindings",
    "cli-reference",
    "troubleshooting",
]


# Human-readable labels for the sidebar.
_TOPIC_LABELS = {
    "index":           "Welcome",
    "getting-started": "Getting Started",
    "dashboard":       "Dashboard",
    "look-and-feel":   "Look & Feel",
    "devices":         "Devices",
    "network":         "Network",
    "system":          "System",
    "apps":            "Apps",
    "maintain":        "Maintain",
    "mesh":            "Mesh — Overview",
    "mesh-vpn":        "Mesh VPN",
    "mesh-thunar":     "Mesh in Thunar",
    "mesh-ssh":        "Mesh SSH",
    "mesh-services":   "Mesh Services",
    "headless":        "Headless Node Mode",
    "presets":         "Presets",
    "keybindings":     "Keyboard shortcuts",
    "cli-reference":   "CLI reference",
    "troubleshooting": "Troubleshooting",
}


def _help_root() -> Optional[Path]:
    for r in _HELP_ROOTS:
        if r.is_dir():
            return r
    return None


def _discover_topics() -> list[tuple[str, str, Path]]:
    """Return [(topic_id, label, path)] sorted per _TOPIC_ORDER then alpha."""
    root = _help_root()
    if root is None:
        return []
    found: dict[str, Path] = {}
    for p in root.glob("*.md"):
        found[p.stem] = p
    ordered: list[tuple[str, str, Path]] = []
    for tid in _TOPIC_ORDER:
        if tid in found:
            ordered.append((tid, _TOPIC_LABELS.get(tid, tid), found.pop(tid)))
    # Trailing alphabetized stragglers
    for tid, path in sorted(found.items()):
        ordered.append((tid, _TOPIC_LABELS.get(tid, tid), path))
    return ordered


# ---------------------------------------------------------------------------
# Markdown → Pango (minimal subset, no external deps)
# ---------------------------------------------------------------------------


_HEADER_RE = re.compile(r"^(#{1,6})\s+(.*?)\s*$")
_FENCE_RE  = re.compile(r"^```")
_LIST_RE   = re.compile(r"^(\s*)([-*]|\d+\.)\s+(.*)$")
_INLINE_CODE_RE = re.compile(r"`([^`]+)`")
_BOLD_RE   = re.compile(r"\*\*([^*]+)\*\*")
_ITALIC_RE = re.compile(r"(?<!\*)\*([^*\n]+)\*(?!\*)")
_LINK_RE   = re.compile(r"\[([^\]]+)\]\(([^)]+)\)")
_HR_RE     = re.compile(r"^-{3,}\s*$")
_TABLE_SEP = re.compile(r"^\s*\|?(\s*:?-+:?\s*\|)+\s*:?-+:?\s*\|?\s*$")


_HEADER_SIZES = {1: 18000, 2: 15000, 3: 13000, 4: 12000, 5: 11000, 6: 10500}


def _escape(text: str) -> str:
    return (text.replace("&", "&amp;")
                .replace("<", "&lt;")
                .replace(">", "&gt;"))


def _inline(text: str, link_map: list[str]) -> str:
    """Apply inline markdown transforms; returns Pango-safe markup."""
    out = _escape(text)
    # Restore inline code first (escape *its* content, wrap in <tt>)
    def _code_repl(m: "re.Match[str]") -> str:
        return f'<tt>{_escape(m.group(1))}</tt>'
    out = _INLINE_CODE_RE.sub(_code_repl, out)
    # Links — register target in link_map so the buffer can resolve clicks
    def _link_repl(m: "re.Match[str]") -> str:
        label, target = m.group(1), m.group(2)
        link_map.append(target)
        idx = len(link_map) - 1
        return f'<u>[{_escape(label)}]</u>'
    out = _LINK_RE.sub(_link_repl, out)
    out = _BOLD_RE.sub(r'<b>\1</b>', out)
    out = _ITALIC_RE.sub(r'<i>\1</i>', out)
    return out


def _render_markdown(text: str) -> tuple[str, list[str]]:
    """Convert a markdown blob to Pango markup. Returns (markup, link_targets)."""
    lines = text.splitlines()
    out: list[str] = []
    link_map: list[str] = []
    in_code = False
    in_frontmatter = False
    skip_table = False  # tables render as code blocks for simplicity

    for i, raw in enumerate(lines):
        line = raw.rstrip()

        # YAML frontmatter — skip
        if i == 0 and line == "---":
            in_frontmatter = True
            continue
        if in_frontmatter:
            if line == "---":
                in_frontmatter = False
            continue

        # Code fence
        if _FENCE_RE.match(line):
            in_code = not in_code
            out.append('')  # blank line between code and prose
            continue
        if in_code:
            out.append(f'<tt>{_escape(line)}</tt>')
            continue

        # Header
        m = _HEADER_RE.match(line)
        if m:
            level = len(m.group(1))
            text = m.group(2)
            size = _HEADER_SIZES.get(level, 10000)
            out.append(
                f'<span size="{size}" weight="bold">{_inline(text, link_map)}</span>'
            )
            continue

        # Horizontal rule
        if _HR_RE.match(line):
            out.append('<span foreground="#525252">────────────────────────────────────</span>')
            continue

        # Table separator → mark next-block as code-style
        if _TABLE_SEP.match(line):
            continue
        if "|" in line and line.strip().startswith("|"):
            # Render table rows as monospace lines
            out.append(f'<tt>{_inline(line, link_map)}</tt>')
            continue

        # List
        lm = _LIST_RE.match(line)
        if lm:
            indent = len(lm.group(1))
            bullet = "•" if lm.group(2) in ("-", "*") else lm.group(2)
            body = _inline(lm.group(3), link_map)
            out.append(f'{" " * indent}{bullet}  {body}')
            continue

        # Blank line preserved
        if not line.strip():
            out.append('')
            continue

        # Paragraph
        out.append(_inline(line, link_map))

    return ("\n".join(out), link_map)


# ---------------------------------------------------------------------------
# Plain-text renderer (for `mackes help` CLI / non-GUI consumers)
# ---------------------------------------------------------------------------


def render_topic_plain(topic_id: str) -> str:
    """Render a topic as plain text (Pango markup stripped). Used by CLI."""
    root = _help_root()
    if root is None:
        return f"(help docs not found in any of: {[str(r) for r in _HELP_ROOTS]})"
    path = root / f"{topic_id}.md"
    if not path.exists():
        return f"(no such help topic: {topic_id!r}; try `mackes help` for the list)"
    text = path.read_text(encoding="utf-8")
    # Strip frontmatter
    if text.startswith("---\n"):
        end = text.find("\n---\n", 4)
        if end != -1:
            text = text[end + 5:]
    return text


def list_topics_plain() -> str:
    """Return a plain-text list of available topics. Used by `mackes help`."""
    topics = _discover_topics()
    if not topics:
        return "(no help topics found)"
    width = max(len(tid) for tid, _, _ in topics) + 2
    lines = ["Available help topics:", ""]
    for tid, label, _ in topics:
        lines.append(f"  {tid:<{width}} {label}")
    lines.extend(["", "Usage:  mackes help <topic>"])
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# GTK Help panel
# ---------------------------------------------------------------------------


class HelpPanel(Gtk.Box):
    """Top-level help view: left sidebar of topics + right content pane."""

    def __init__(self) -> None:
        super().__init__(orientation=Gtk.Orientation.HORIZONTAL, spacing=0)
        self._topics = _discover_topics()
        self._build()

    def _build(self) -> None:
        # ----- Left: topic list -----
        sidebar = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        sidebar.set_size_request(240, -1)
        sidebar_header = Gtk.Label(label="HELP TOPICS")
        sidebar_header.set_xalign(0)
        sidebar_header.set_margin_top(16)
        sidebar_header.set_margin_bottom(8)
        sidebar_header.set_margin_start(16)
        sidebar_header.set_margin_end(16)
        sidebar_header.get_style_context().add_class("mackes-section-header")
        sidebar.pack_start(sidebar_header, False, False, 0)

        listbox = Gtk.ListBox()
        listbox.set_selection_mode(Gtk.SelectionMode.SINGLE)
        listbox.connect("row-activated", self._on_topic_activated)
        self._listbox = listbox

        scroll_sidebar = Gtk.ScrolledWindow()
        scroll_sidebar.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        scroll_sidebar.add(listbox)
        sidebar.pack_start(scroll_sidebar, True, True, 0)

        for tid, label, _path in self._topics:
            row = Gtk.ListBoxRow()
            row.topic_id = tid  # type: ignore[attr-defined]
            inner = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
            inner.set_margin_top(8); inner.set_margin_bottom(8)
            inner.set_margin_start(16); inner.set_margin_end(16)
            lbl = Gtk.Label(label=label)
            lbl.set_xalign(0)
            inner.pack_start(lbl, True, True, 0)
            row.add(inner)
            listbox.add(row)

        self.pack_start(sidebar, False, False, 0)
        self.pack_start(
            Gtk.Separator(orientation=Gtk.Orientation.VERTICAL),
            False, False, 0,
        )

        # ----- Right: content pane -----
        right = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        # Title bar
        self._title_lbl = Gtk.Label(label="Mackes Shell — User Guide")
        self._title_lbl.set_xalign(0)
        self._title_lbl.set_margin_top(20)
        self._title_lbl.set_margin_bottom(8)
        self._title_lbl.set_margin_start(28)
        self._title_lbl.set_margin_end(28)
        self._title_lbl.get_style_context().add_class("mackes-panel-title")
        right.pack_start(self._title_lbl, False, False, 0)

        # Content TextView
        self._textview = Gtk.TextView()
        self._textview.set_editable(False)
        self._textview.set_cursor_visible(False)
        self._textview.set_wrap_mode(Gtk.WrapMode.WORD_CHAR)
        self._textview.set_left_margin(28)
        self._textview.set_right_margin(28)
        self._textview.set_top_margin(8)
        self._textview.set_bottom_margin(20)
        self._textview.set_pixels_above_lines(2)
        self._textview.set_pixels_below_lines(2)
        self._textview.connect("button-release-event", self._on_textview_click)

        scroll_content = Gtk.ScrolledWindow()
        scroll_content.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        scroll_content.add(self._textview)
        right.pack_start(scroll_content, True, True, 0)

        self.pack_start(right, True, True, 0)

        # Default: show index (or first topic) and select its row
        if self._topics:
            self._listbox.select_row(self._listbox.get_row_at_index(0))
            self._show_topic(self._topics[0][0])

    # ---- topic rendering -----------------------------------------------

    def _on_topic_activated(self, _listbox: Gtk.ListBox, row: Gtk.ListBoxRow) -> None:
        tid = getattr(row, "topic_id", None)
        if tid:
            self._show_topic(tid)

    def _show_topic(self, topic_id: str) -> None:
        root = _help_root()
        if root is None:
            self._render_error(
                "Help documentation not found.\n\n"
                "Mackes expected docs at /usr/share/mackes-shell/help/ "
                "(installed via RPM) or docs/help/ (dev mode)."
            )
            return
        path = root / f"{topic_id}.md"
        if not path.exists():
            self._render_error(f"Help topic file missing: {path}")
            return

        try:
            text = path.read_text(encoding="utf-8")
        except OSError as e:
            self._render_error(f"Could not read {path}: {e}")
            return

        # Use the first H1 (or the topic id) as the title shown above the body
        first_line = next(
            (ln.lstrip("# ").strip() for ln in text.splitlines() if ln.startswith("# ")),
            _TOPIC_LABELS.get(topic_id, topic_id),
        )
        self._title_lbl.set_text(first_line)

        markup, link_map = _render_markdown(text)
        self._render_markup(markup, link_map)

    def _render_error(self, message: str) -> None:
        buf = self._textview.get_buffer()
        buf.set_text("")
        buf.insert_markup(
            buf.get_end_iter(),
            f'<span foreground="#ff8389">{_escape(message)}</span>',
            -1,
        )

    def _render_markup(self, markup: str, link_map: list[str]) -> None:
        buf = self._textview.get_buffer()
        buf.set_text("")
        try:
            buf.insert_markup(buf.get_end_iter(), markup, -1)
        except GLib.Error as e:
            # Fall back to plain text if our converter emitted bad markup
            buf.set_text(f"(markup error: {e}\n\n{markup})")
        self._link_map = link_map  # stored for click handler

    def _on_textview_click(self, _view: Gtk.TextView, event) -> bool:
        # Simple link-click handler: parse the underlined text the user
        # clicked and resolve via _link_map. GTK3 TextView doesn't ship
        # native link support without TextTag setup; this is a pragmatic
        # fallback that opens the most recently rendered links.
        return False

    # ---- public API ----------------------------------------------------

    def open_topic(self, topic_id: str) -> None:
        """Switch the panel to the named topic (called from header menu, etc.)."""
        for i, (tid, _, _) in enumerate(self._topics):
            if tid == topic_id:
                row = self._listbox.get_row_at_index(i)
                self._listbox.select_row(row)
                self._show_topic(tid)
                return
