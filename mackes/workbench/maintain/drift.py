"""Maintain → Drift.

A first-class surface for preset drift (Option 6 of the platform review).
Today the dashboard shows a one-line "N keys diverged" card. This panel
breaks each divergence into its own row with three actions:

  - **Revert to preset** — write the preset's value back to xfconf or the
    shell stack, taking the live state back to the preset's intent.
  - **Adopt into preset** — write the live value into the active preset's
    user-override YAML, making the divergence the new contract.
  - **Ignore** — purely UI: hide this row for this session.

Without a panel like this, drift accumulates silently and the preset becomes
a one-shot apply that goes stale. With it, the preset becomes a living
agreement between the user and Mackes.
"""
from __future__ import annotations

from typing import Optional

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk  # noqa: E402

from mackes.logging import log_action
from mackes.presets import (
    APPEARANCE_KEYS, DriftItem, Preset, active_preset_drift, detect_drift, load_preset,
)
from mackes.state import MackesState
from mackes.workbench._common import (
    info_label, panel_box, section_header, title_label,
)
from mackes.xfconf_bridge import XfconfError, get_bridge


class DriftPanel(Gtk.Box):
    def __init__(self, state: MackesState) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.state = state
        self._ignored: set[tuple[str, str]] = set()  # session-only mute
        self._rows_container: Optional[Gtk.Box] = None
        self._build()
        GLib.idle_add(self._refresh)

    # ---- UI ------------------------------------------------------------

    def _build(self) -> None:
        box = panel_box()
        box.pack_start(title_label("Drift"), False, False, 0)
        box.pack_start(info_label(
            "Where your live system differs from the active preset. Use the "
            "buttons to bring them back into alignment, or to record a "
            "deliberate change as the new contract."
        ), False, False, 0)

        box.pack_start(section_header("Divergences"), False, False, 0)
        self._rows_container = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        box.pack_start(self._rows_container, False, False, 0)

        actions = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        refresh = Gtk.Button(label="Refresh")
        refresh.connect("clicked", lambda *_: self._refresh())
        actions.pack_start(refresh, False, False, 0)

        revert_all = Gtk.Button(label="Revert ALL to preset")
        revert_all.connect("clicked", lambda *_: self._revert_all())
        actions.pack_start(revert_all, False, False, 0)
        box.pack_start(actions, False, False, 0)

        self._status = Gtk.Label(label="")
        self._status.set_xalign(0)
        self._status.get_style_context().add_class("dim-label")
        box.pack_start(self._status, False, False, 0)

        self.add(box)

    # ---- Refresh -------------------------------------------------------

    def _refresh(self) -> bool:
        if self._rows_container is None:
            return False
        for child in list(self._rows_container.get_children()):
            self._rows_container.remove(child)
        preset, items = active_preset_drift()
        if preset is None:
            self._rows_container.add(info_label(
                "No active preset on this system yet — drift detection needs a preset to compare against."
            ))
            self._rows_container.show_all()
            return False

        items = [it for it in items if (it.section, it.field) not in self._ignored]
        if not items:
            self._rows_container.add(info_label(
                f"In sync with preset {preset.display_name!r}. No drift detected."
            ))
            self._rows_container.show_all()
            self._status.set_text("")
            return False

        for it in items:
            self._rows_container.add(self._build_row(preset, it))
        self._rows_container.show_all()
        self._status.set_text(f"{len(items)} divergence(s) against {preset.display_name!r}.")
        return False

    def _build_row(self, preset: Preset, it: DriftItem) -> Gtk.Widget:
        frame = Gtk.Frame()
        frame.get_style_context().add_class("view")

        outer = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        outer.set_margin_top(8); outer.set_margin_bottom(8)
        outer.set_margin_start(12); outer.set_margin_end(12)

        title = Gtk.Label()
        title.set_xalign(0)
        title.set_markup(f"<b>{it.section}.{it.field}</b>")
        outer.pack_start(title, False, False, 0)

        detail = Gtk.Label(label=f"preset → {it.expected!r}   live → {it.actual!r}")
        detail.set_xalign(0)
        detail.get_style_context().add_class("dim-label")
        outer.pack_start(detail, False, False, 0)

        btns = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        b_revert = Gtk.Button(label="Revert to preset")
        b_revert.connect("clicked", lambda *_: self._revert(preset, it))
        b_adopt = Gtk.Button(label="Adopt into preset")
        b_adopt.set_sensitive(False)  # writing user-override YAML is Phase 2 of drift work
        b_adopt.set_tooltip_text("Writing user-override YAML is not implemented yet.")
        b_ignore = Gtk.Button(label="Ignore")
        b_ignore.connect("clicked", lambda *_: self._ignore(it))
        btns.pack_start(b_revert, False, False, 0)
        btns.pack_start(b_adopt, False, False, 0)
        btns.pack_start(b_ignore, False, False, 0)
        outer.pack_start(btns, False, False, 0)

        frame.add(outer)
        return frame

    # ---- Actions -------------------------------------------------------

    def _revert(self, preset: Preset, it: DriftItem) -> None:
        try:
            n = self._write_back(preset, it)
            log_action(f"drift: revert {it.section}.{it.field} -> {it.expected!r} ({n} key(s))")
            self._status.set_text(f"reverted {it.section}.{it.field}")
        except Exception as e:  # noqa: BLE001
            self._status.set_text(f"revert failed: {e}")
        GLib.idle_add(self._refresh)

    def _revert_all(self) -> None:
        preset, items = active_preset_drift()
        if preset is None:
            return
        n = 0
        for it in items:
            try:
                n += self._write_back(preset, it)
            except Exception:  # noqa: BLE001
                pass
        log_action(f"drift: revert ALL ({n} key(s) written)")
        self._status.set_text(f"reverted {n} key(s)")
        GLib.idle_add(self._refresh)

    def _ignore(self, it: DriftItem) -> None:
        self._ignored.add((it.section, it.field))
        self._refresh()

    # ---- Write-back ----------------------------------------------------
    # Only appearance/system keys flow through xfconf directly. Any heavier
    # drift can be reverted via "Reset to Preset" in the sibling panel.

    def _write_back(self, preset: Preset, it: DriftItem) -> int:
        if it.section == "appearance":
            binding = APPEARANCE_KEYS.get(it.field)
            if binding is None:
                return 0
            channel, prop = binding
            xf = get_bridge()
            xf.set(channel, prop, it.expected)
            return 1
        if it.section == "system":
            xf = get_bridge()
            if it.field == "workspace_count":
                xf.set("xfwm4", "/general/workspace_count", int(it.expected))
                return 1
            if it.field == "window_manager_theme":
                xf.set("xfwm4", "/general/theme", str(it.expected), type_hint="string")
                return 1
        return 0
