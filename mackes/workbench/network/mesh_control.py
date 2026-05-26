"""Unified Mesh Control Panel (Phase 12.8.1).

Replaces the six standalone mesh panels (`mesh_health`, `mesh_join`,
`mesh_services`, `mesh_ssh`, `mesh_topology`, `mesh_vpn`) with one
tabbed surface. The legacy panels stay importable so the sidebar's
quick-jump links keep working during the 1.x → 2.x deprecation
window; this panel is the canonical entry point starting in v1.1.x
and the only mesh surface in v2.0.0.

Per `docs/PROJECT_WORKLIST.md` Phase 12.8.1, each tab reads through
`mackesd_core::mesh::*` via `mackes.mackesd_bridge` (shell-out today;
PyO3 in v2.0.0). The bridge transparently falls back to the legacy
in-process probes when the migration flag is off so the panel works
on a fresh install where mackesd hasn't seen its first enrollment yet.
"""
from __future__ import annotations

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402

from mackes.workbench._common import a11y


# Tab definitions: (slug, label, module path, class name). Kept as
# a top-level constant so the wizard's "Mesh setup" deep-link and
# the `mackes --focus mesh.<slug>` CLI route can address tabs by
# slug. The class import is lazy (in `_build_tab`) so a missing GTK
# typelib in one panel doesn't break the whole notebook.
TABS: list[tuple[str, str, str, str]] = [
    ("health",       "Health",        "mackes.workbench.network.mesh_health",       "MeshHealthPanel"),
    ("topology",     "Topology",      "mackes.workbench.network.mesh_topology",     "MeshTopologyPanel"),
    ("services",     "Services",      "mackes.workbench.network.mesh_services",     "MeshServicesPanel"),
    # NF-5.5 (v2.5 Nebula fabric): "VPN" tab dropped along
    # with mackes/workbench/network/mesh_vpn.py. Mesh state
    # surfaces in the Health + Topology tabs (rewritten for
    # Nebula by NF-11.x).
    ("ssh",          "SSH",           "mackes.workbench.network.mesh_ssh",          "MeshSshPanel"),
    ("join",         "Join",          "mackes.workbench.network.mesh_join",         "MeshJoinPanel"),
    ("pending",      "Pending",       "mackes.workbench.network.mesh_pending",      "MeshPendingPanel"),
    ("history",      "History",       "mackes.workbench.network.mesh_history",      "MeshHistoryPanel"),
]


def slug_for_tab(tab_index: int) -> str:
    """Map an integer notebook page index back to its slug."""
    if 0 <= tab_index < len(TABS):
        return TABS[tab_index][0]
    return TABS[0][0]


def tab_index_for_slug(slug: str) -> int:
    """Map a slug back to its notebook page index, defaulting to 0."""
    for i, (s, _, _, _) in enumerate(TABS):
        if s == slug:
            return i
    return 0


def _build_tab(module_path: str, class_name: str) -> Gtk.Widget:
    """Import and instantiate one tab's panel class.

    Returns a placeholder widget when the import fails so a single
    broken panel doesn't break the notebook. Errors are surfaced as a
    Carbon-styled empty state instead of bubbling up.
    """
    try:
        mod = __import__(module_path, fromlist=[class_name])
        cls = getattr(mod, class_name)
        return cls()
    except Exception as exc:  # noqa: BLE001 — boundary import; surface the error
        return _broken_tab(module_path, class_name, exc)


def _broken_tab(module_path: str, class_name: str, exc: BaseException) -> Gtk.Widget:
    box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
    box.set_margin_top(24); box.set_margin_bottom(24)
    box.set_margin_start(24); box.set_margin_end(24)
    title = Gtk.Label(label=f"{class_name} unavailable")
    title.set_xalign(0); title.get_style_context().add_class("mackes-page-title")
    body = Gtk.Label(
        label=(
            f"Failed to load {module_path}.{class_name}: {exc.__class__.__name__}: {exc}"
        )
    )
    body.set_xalign(0); body.set_line_wrap(True)
    body.get_style_context().add_class("mackes-page-subtitle")
    box.pack_start(title, False, False, 0)
    box.pack_start(body, False, False, 0)
    return box


class MeshControlPanel(Gtk.Box):
    """Tabbed mesh control surface (Phase 12.8.1).

    Wraps the 7 existing per-domain mesh panels + the two new 12.8.2
    (pending changes) and 12.8.3 (history + diff viewer) panels into
    one notebook. The sidebar shows a single "Mesh" entry; tabs let
    the user move between concerns without losing place in the
    workbench tree.
    """

    def __init__(self, focus_slug: str | None = None) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)

        self._notebook = Gtk.Notebook()
        self._notebook.set_scrollable(True)
        a11y(self._notebook, "Mesh control tabs", tooltip=None)

        for _slug, label, module_path, class_name in TABS:
            page = _build_tab(module_path, class_name)
            scroller = Gtk.ScrolledWindow()
            scroller.set_policy(Gtk.PolicyType.AUTOMATIC, Gtk.PolicyType.AUTOMATIC)
            scroller.add(page)
            tab_label = Gtk.Label(label=label)
            tab_label.set_xalign(0.5)
            self._notebook.append_page(scroller, tab_label)

        self.pack_start(self._notebook, True, True, 0)

        if focus_slug:
            self._notebook.set_current_page(tab_index_for_slug(focus_slug))

    def focus_slug(self, slug: str) -> None:
        """External hook: jump to a tab by slug. Used by
        `mackes --focus mesh.<slug>` deep-links."""
        self._notebook.set_current_page(tab_index_for_slug(slug))

    def current_slug(self) -> str:
        """Return the slug of the currently-active tab."""
        return slug_for_tab(self._notebook.get_current_page())
