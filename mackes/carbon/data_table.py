"""Carbon DataTable — tabular data with columns, sorting, and filtering.

Backed by Gtk.TreeView + ListStore so it gets native keyboard nav and
selection behavior. Carbon's visual styling comes from tokens.css.

Used by:
  - Mesh VPN peer list
  - Mesh SSH audit log
  - Apps → Installed
  - Maintain → Snapshots
"""
from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Callable, Iterable, Optional

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import GObject, Gtk  # noqa: E402


@dataclass
class Column:
    name: str
    title: str
    width: int = -1
    sortable: bool = True
    monospace: bool = False
    formatter: Optional[Callable[[Any], str]] = None  # value -> display string


class DataTable(Gtk.Box):
    """Carbon-styled DataTable.

    Construct with column definitions, then call `set_rows(...)` repeatedly.
    Each row is a dict keyed by column name.
    """

    def __init__(
        self,
        columns: Iterable[Column],
        *,
        searchable: bool = True,
        on_row_activate: Optional[Callable[[dict], None]] = None,
    ) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.get_style_context().add_class("cds-data-table")
        self._columns = list(columns)
        self._on_row_activate = on_row_activate
        self._all_rows: list[dict[str, Any]] = []
        self._filter_text = ""

        # Search bar
        if searchable:
            search_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
            search_box.set_margin_top(8); search_box.set_margin_bottom(8)
            search_box.set_margin_start(0); search_box.set_margin_end(0)
            self._search = Gtk.SearchEntry()
            self._search.set_placeholder_text("Filter…")
            self._search.connect("search-changed", self._on_search_changed)
            search_box.pack_start(self._search, True, True, 0)
            self.pack_start(search_box, False, False, 0)

        # TreeView
        # Every column is stored as a string for display simplicity.
        store_types = [str] * len(self._columns)
        self._store = Gtk.ListStore(*store_types)
        self._filter = self._store.filter_new()
        self._filter.set_visible_func(self._row_visible)

        self._view = Gtk.TreeView(model=self._filter)
        self._view.set_enable_search(False)
        self._view.set_headers_visible(True)
        if on_row_activate is not None:
            self._view.connect("row-activated", self._on_row_activated)

        for idx, col in enumerate(self._columns):
            renderer = Gtk.CellRendererText()
            if col.monospace:
                renderer.set_property("family", "IBM Plex Mono")
            tv_col = Gtk.TreeViewColumn(col.title, renderer, text=idx)
            if col.width > 0:
                tv_col.set_fixed_width(col.width)
                tv_col.set_sizing(Gtk.TreeViewColumnSizing.FIXED)
            if col.sortable:
                tv_col.set_sort_column_id(idx)
            tv_col.set_resizable(True)
            self._view.append_column(tv_col)

        scroll = Gtk.ScrolledWindow()
        scroll.set_policy(Gtk.PolicyType.AUTOMATIC, Gtk.PolicyType.AUTOMATIC)
        scroll.add(self._view)
        self.pack_start(scroll, True, True, 0)

    # ---- public API -----------------------------------------------------

    def set_rows(self, rows: Iterable[dict[str, Any]]) -> None:
        self._all_rows = list(rows)
        self._rebuild_store()

    def add_row(self, row: dict[str, Any]) -> None:
        self._all_rows.append(row)
        self._append_to_store(row)

    def clear(self) -> None:
        self._all_rows = []
        self._store.clear()

    def selected_row(self) -> Optional[dict[str, Any]]:
        sel = self._view.get_selection()
        model, iter_ = sel.get_selected()
        if iter_ is None:
            return None
        # Translate filter iter back to source path index
        source_iter = self._filter.convert_iter_to_child_iter(iter_)
        path = self._store.get_path(source_iter)
        idx = path.get_indices()[0]
        return self._all_rows[idx]

    # ---- internals ------------------------------------------------------

    def _rebuild_store(self) -> None:
        self._store.clear()
        for row in self._all_rows:
            self._append_to_store(row)

    def _append_to_store(self, row: dict[str, Any]) -> None:
        values = []
        for col in self._columns:
            v = row.get(col.name, "")
            if col.formatter is not None:
                v = col.formatter(v)
            else:
                v = "" if v is None else str(v)
            values.append(v)
        self._store.append(values)

    def _row_visible(self, model: Gtk.ListStore, iter_: Gtk.TreeIter, _data: Any) -> bool:
        if not self._filter_text:
            return True
        needle = self._filter_text.lower()
        for i in range(len(self._columns)):
            val = model.get_value(iter_, i) or ""
            if needle in val.lower():
                return True
        return False

    def _on_search_changed(self, entry: Gtk.SearchEntry) -> None:
        self._filter_text = entry.get_text().strip()
        self._filter.refilter()

    def _on_row_activated(
        self,
        _view: Gtk.TreeView,
        path: Gtk.TreePath,
        _column: Gtk.TreeViewColumn,
    ) -> None:
        if self._on_row_activate is None:
            return
        idx = path.get_indices()[0]
        # path is into the filter model — translate to source
        filter_iter = self._filter.get_iter(path)
        source_iter = self._filter.convert_iter_to_child_iter(filter_iter)
        source_idx = self._store.get_path(source_iter).get_indices()[0]
        if 0 <= source_idx < len(self._all_rows):
            self._on_row_activate(self._all_rows[source_idx])
