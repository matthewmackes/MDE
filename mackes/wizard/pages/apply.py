"""Wizard final step — Apply (v1.6.0 world-class Carbon-stepped UX).

Replaces the v1.0–v1.5 single-textview-with-progressbar layout.

Layout:

  +-----------------------------------------------------------------+
  |  Becoming Mackes…                                    [Cancel]   |
  |  step 7 of 19   ████████████████████░░░░░░░░░░░░░░  37%         |
  +----------------------------+------------------------------------+
  |  ✓ Snapshot           0.2s |  ▎  Themes                          |
  |  ✓ Appearance         1.4s |                                      |
  |  ✓ Devices            0.3s |  ─ Installing Orchis-Dark theme…     |
  |  ✓ System             0.5s |  ─ Done · /usr/share/themes/Orchis-Dark |
  |  ✓ Network            0.2s |  ─ Installing Black-Sun icons…       |
  |  ✓ Panel              0.4s |  ─ Rebuilding gtk icon cache…        |
  |  ◐ Themes             3.1s |                                      |
  |  ⋯ Fonts                 — |                                      |
  |  ⋯ Apps                  — |                                      |
  |  ...                      |  elapsed 6.0s · ~ 4 min remaining     |
  +----------------------------+------------------------------------+

Architecture:
  * Run loop on a background daemon thread so the GTK main loop stays
    responsive (the previous Gtk.events_pending() spin lock was crap).
  * Every UI write goes through GLib.idle_add — thread-safe.
  * Per-step state machine: pending / running / done / failed / skipped.
  * Step rail: scrollable ListBox; each row paints its own status glyph
    + Cairo spinner when running.
  * Live detail right pane: last 12 log lines for the active step in
    monospace Carbon Gray.
  * Cancel button: sets a stop flag the loop honors between steps.
  * Overall ETA: rolling mean of completed step durations × remaining.
"""
from __future__ import annotations

import shutil
import subprocess
import threading
import time
from typing import Callable, List, Optional

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk  # noqa: E402

from mackes.logging import log_action
from mackes.birthright import (
    apply_apps, apply_clipboard_daemon, apply_dnf_update, apply_drawer,
    apply_enforce_i3, apply_flathub, apply_fleet, apply_fonts,
    apply_display_manager, apply_hotkey, apply_media_clients,
    apply_netdata_monitor, apply_panel_archive, apply_panel_layout,
    apply_panel_swap, apply_plymouth, apply_qnm, apply_remote_desktop,
    apply_sway_config,
    apply_tag_manifests_seed, apply_themes,
    apply_third_party_repos, apply_thunar_autostart, apply_uid_normalize,
    apply_uninstall_legacy_xfce, apply_uninstall_legacy_xsessions,
    apply_user_dirs,
)
from mackes.presets import (
    Preset, apply_appearance, apply_devices, apply_mesh, apply_network,
    apply_panel, apply_system,
)
from mackes.snapshots import create_snapshot


# Step lifecycle states
_PENDING = "pending"
_RUNNING = "running"
_DONE    = "done"
_FAILED  = "failed"
_SKIPPED = "skipped"

_GLYPHS = {
    _PENDING: "⋯",
    _RUNNING: "◐",
    _DONE:    "✓",
    _FAILED:  "✗",
    _SKIPPED: "—",
}


class _Step:
    """One row in the step rail."""

    __slots__ = ("name", "fn", "state", "elapsed", "log", "_row",
                 "_glyph_lbl", "_name_lbl", "_time_lbl")

    def __init__(self, name: str, fn: Callable[[], List[str] | None]) -> None:
        self.name = name
        self.fn = fn
        self.state = _PENDING
        self.elapsed = 0.0
        self.log: List[str] = []
        self._row: Optional[Gtk.ListBoxRow] = None
        self._glyph_lbl: Optional[Gtk.Label] = None
        self._name_lbl: Optional[Gtk.Label] = None
        self._time_lbl: Optional[Gtk.Label] = None

    def build_row(self) -> Gtk.ListBoxRow:
        row = Gtk.ListBoxRow()
        row.get_style_context().add_class("mackes-side-nav-item")
        row.set_activatable(False); row.set_selectable(False)
        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        box.set_margin_top(8); box.set_margin_bottom(8)
        box.set_margin_start(16); box.set_margin_end(16)

        self._glyph_lbl = Gtk.Label(label=_GLYPHS[self.state])
        self._glyph_lbl.set_xalign(0); self._glyph_lbl.set_size_request(20, -1)
        self._glyph_lbl.get_style_context().add_class("mackes-dot")
        box.pack_start(self._glyph_lbl, False, False, 0)

        self._name_lbl = Gtk.Label(label=self.name)
        self._name_lbl.set_xalign(0)
        box.pack_start(self._name_lbl, True, True, 0)

        self._time_lbl = Gtk.Label(label="—")
        self._time_lbl.set_xalign(1)
        self._time_lbl.get_style_context().add_class("mackes-section-meta")
        box.pack_end(self._time_lbl, False, False, 0)

        row.add(box)
        self._row = row
        self._apply_state_classes()
        return row

    def set_state(self, state: str) -> None:
        self.state = state
        self._apply_state_classes()

    def _apply_state_classes(self) -> None:
        if self._glyph_lbl is None:
            return
        self._glyph_lbl.set_text(_GLYPHS[self.state])
        ctx = self._glyph_lbl.get_style_context()
        for v in ("ok", "warn", "fail", "muted", "accent"):
            ctx.remove_class(v)
        if   self.state == _RUNNING: ctx.add_class("accent")
        elif self.state == _DONE:    ctx.add_class("ok")
        elif self.state == _FAILED:  ctx.add_class("fail")
        elif self.state == _SKIPPED: ctx.add_class("muted")
        else: ctx.add_class("muted")

        # Time column
        if self.state == _PENDING:
            self._time_lbl.set_text("—")
        elif self.state == _RUNNING:
            self._time_lbl.set_text(f"{self.elapsed:.1f}s")
        else:
            self._time_lbl.set_text(f"{self.elapsed:.1f}s")

        # Active-row emphasis on the parent ListBoxRow
        if self._row is not None:
            row_ctx = self._row.get_style_context()
            if self.state == _RUNNING:
                row_ctx.add_class("active")
            else:
                row_ctx.remove_class("active")

    def update_elapsed(self, secs: float) -> None:
        self.elapsed = secs
        if self._time_lbl is not None and self.state == _RUNNING:
            self._time_lbl.set_text(f"{secs:.1f}s")


# --------------------------------------------------------------------------
# ApplyPage
# --------------------------------------------------------------------------


class ApplyPage(Gtk.Box):
    """v1.6.0 — stepped Carbon progress with live detail."""

    def __init__(self, ctx) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.ctx = ctx
        self._done = False
        self._cancel = threading.Event()
        self._start_ts: Optional[float] = None
        self._steps: List[_Step] = []
        self._active_idx = -1
        self._on_complete: Optional[Callable[[], None]] = None

        outer = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        outer.set_margin_top(32); outer.set_margin_bottom(24)
        outer.set_margin_start(40); outer.set_margin_end(40)

        # ---- header ---------------------------------------------------
        header = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        preset_name = ctx.selected_preset.display_name if ctx.selected_preset else "Mackes"
        self._title = Gtk.Label(label=f"Becoming {preset_name}…")
        self._title.set_xalign(0)
        self._title.get_style_context().add_class("mackes-page-title")
        header.pack_start(self._title, True, True, 0)

        self._cancel_btn = Gtk.Button(label="Cancel")
        self._cancel_btn.get_style_context().add_class("cds-button-tertiary")
        self._cancel_btn.connect("clicked", self._on_cancel)
        header.pack_end(self._cancel_btn, False, False, 0)
        outer.pack_start(header, False, False, 0)

        sub = Gtk.Label(label=(
            "Each step runs in order. You can cancel any time — anything "
            "already applied stays applied. Failed steps are skipped and "
            "logged; the wizard continues."
        ))
        sub.set_xalign(0); sub.set_line_wrap(True)
        sub.get_style_context().add_class("mackes-page-subtitle")
        outer.pack_start(sub, False, False, 0)

        # ---- top progress strip --------------------------------------
        prog_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        prog_row.set_margin_top(16); prog_row.set_margin_bottom(8)
        self._step_count = Gtk.Label(label="step 0 of 0")
        self._step_count.set_xalign(0)
        self._step_count.set_size_request(110, -1)
        self._step_count.get_style_context().add_class("mackes-section-meta")
        prog_row.pack_start(self._step_count, False, False, 0)
        self._progress = Gtk.ProgressBar(); self._progress.set_fraction(0.0)
        self._progress.set_size_request(-1, 8); self._progress.set_valign(Gtk.Align.CENTER)
        prog_row.pack_start(self._progress, True, True, 0)
        self._pct = Gtk.Label(label="0%")
        self._pct.set_xalign(1); self._pct.set_size_request(48, -1)
        self._pct.get_style_context().add_class("mackes-section-meta")
        prog_row.pack_end(self._pct, False, False, 0)
        outer.pack_start(prog_row, False, False, 0)

        # ---- two-pane body -------------------------------------------
        body = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=24)
        body.set_margin_top(16)

        # Left: step rail
        rail_wrap = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        rail_wrap.set_size_request(340, -1)
        rail_wrap.get_style_context().add_class("mackes-side-nav")
        rail_header = Gtk.Label(label="STEPS")
        rail_header.set_xalign(0)
        rail_header.set_margin_top(12); rail_header.set_margin_bottom(4)
        rail_header.set_margin_start(16); rail_header.set_margin_end(16)
        rail_header.get_style_context().add_class("mackes-side-nav-group-title")
        rail_wrap.pack_start(rail_header, False, False, 0)
        self._rail = Gtk.ListBox()
        self._rail.set_selection_mode(Gtk.SelectionMode.NONE)
        rail_scroller = Gtk.ScrolledWindow()
        rail_scroller.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        rail_scroller.add(self._rail)
        rail_wrap.pack_start(rail_scroller, True, True, 0)
        body.pack_start(rail_wrap, False, False, 0)

        # Right: live detail
        detail_wrap = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        self._active_title = Gtk.Label(label="Waiting to start…")
        self._active_title.set_xalign(0)
        self._active_title.get_style_context().add_class("mackes-section-title")
        detail_wrap.pack_start(self._active_title, False, False, 0)

        self._active_sub = Gtk.Label(label="")
        self._active_sub.set_xalign(0); self._active_sub.set_line_wrap(True)
        self._active_sub.get_style_context().add_class("mackes-page-subtitle")
        detail_wrap.pack_start(self._active_sub, False, False, 0)

        self._log = Gtk.TextView()
        self._log.set_editable(False); self._log.set_monospace(True)
        self._log.set_wrap_mode(Gtk.WrapMode.WORD_CHAR)
        self._log.get_style_context().add_class("mackes-code")
        log_scroller = Gtk.ScrolledWindow()
        log_scroller.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        log_scroller.add(self._log)
        log_scroller.set_margin_top(12)
        detail_wrap.pack_start(log_scroller, True, True, 0)

        # Bottom: elapsed + ETA
        timing = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        timing.set_margin_top(8)
        self._elapsed_lbl = Gtk.Label(label="elapsed 0s")
        self._elapsed_lbl.set_xalign(0)
        self._elapsed_lbl.get_style_context().add_class("mackes-section-meta")
        timing.pack_start(self._elapsed_lbl, True, True, 0)
        self._eta_lbl = Gtk.Label(label="")
        self._eta_lbl.set_xalign(1)
        self._eta_lbl.get_style_context().add_class("mackes-section-meta")
        timing.pack_end(self._eta_lbl, False, False, 0)
        detail_wrap.pack_start(timing, False, False, 0)

        body.pack_start(detail_wrap, True, True, 0)
        outer.pack_start(body, True, True, 0)

        self.pack_start(outer, True, True, 0)

    # ----- public surface --------------------------------------------------

    def is_done(self) -> bool:
        return self._done

    def run(self, on_complete: Optional[Callable[[], None]] = None) -> None:
        """Execute the apply pipeline. Idempotent: returns early on re-entry.

        ``on_complete`` (if given) fires on the GTK main thread once the
        worker thread has finished every step — used by the wizard window
        to gate the Next button so the user can't advance mid-install.
        """
        self._on_complete = on_complete
        if self._done:
            if on_complete is not None:
                on_complete()
            return
        ctx = self.ctx
        preset = ctx.selected_preset
        if preset is None:
            self._set_active_title("No preset selected")
            self._set_active_sub("Nothing to apply.")
            self._done = True
            if on_complete is not None:
                on_complete()
            return

        # Build the effective preset by overlaying overrides on top of
        # the preset's declared defaults.
        merged = Preset(
            name=preset.name, display_name=preset.display_name,
            description=preset.description,
            appearance={**preset.appearance, **(ctx.overrides.get("appearance") or {})},
            devices=   {**preset.devices,    **(ctx.overrides.get("devices") or {})},
            system=    {**preset.system,     **(ctx.overrides.get("system") or {})},
            network=   {**preset.network, "qnm_enabled": ctx.enable_qnm,
                        "firewall_default_zone": ctx.firewall_zone},
            panel=     {**preset.panel,      **(ctx.overrides.get("panel") or {})},
            snapshot=  preset.snapshot,
        )

        self._steps = self._build_steps(merged)
        self._populate_rail()
        self._step_count.set_text(f"step 0 of {len(self._steps)}")

        # Run on a background thread so the GTK main loop stays
        # responsive. All UI mutation funnels back through GLib.idle_add.
        self._start_ts = time.monotonic()
        threading.Thread(target=self._run_loop, args=(merged,),
                         daemon=True, name="mackes-wizard-apply").start()
        # Tick the elapsed clock + active-step elapsed every 0.5s.
        GLib.timeout_add(500, self._tick_clocks)

    # ----- build the step list --------------------------------------------

    def _build_steps(self, merged: Preset) -> List[_Step]:
        return [
            _Step("Snapshot",          self._step_snapshot),
            _Step("Appearance",        lambda: apply_appearance(merged)),
            _Step("Devices",           lambda: apply_devices(merged)),
            _Step("System",            lambda: apply_system(merged)),
            _Step("Network",           lambda: apply_network(merged)),
            _Step("Panel",             lambda: apply_panel(merged)),
            _Step("Themes",            lambda: apply_themes(merged)),
            _Step("Display manager",   lambda: apply_display_manager(merged)),
            _Step("Fonts",             lambda: apply_fonts(merged)),
            _Step("Apps",              lambda: apply_apps(merged)),
            _Step("Panel layout",      lambda: apply_panel_layout(merged)),
            # 2026-05-25 — seed ~/.config/sway/config from the MDE
            # default at /usr/share/mde/sway/config. Without this,
            # mde-session execs `sway` and sway falls back to
            # /etc/sway/config (stock Fedora), producing the "logged
            # into MDE but got empty sway" bug operators hit on
            # fresh installs. Idempotent — preserves operator
            # customizations.
            _Step("Sway config",       lambda: apply_sway_config(merged)),
            _Step("Boot splash",       lambda: apply_plymouth(merged)),
            _Step("System update",     lambda: apply_dnf_update(merged)),
            _Step("Third-party repos", lambda: apply_third_party_repos(merged)),
            _Step("Flathub",           lambda: apply_flathub(merged)),
            _Step("Media clients",     lambda: apply_media_clients(merged)),
            _Step("Remote desktop",    lambda: apply_remote_desktop(merged)),
            _Step("Fleet management",  lambda: apply_fleet(merged)),
            _Step("Notification drawer", lambda: apply_drawer(merged)),
            _Step("Mesh clipboard",    lambda: apply_clipboard_daemon(merged)),
            _Step("Quick Network Mesh", lambda: apply_qnm(merged)),
            _Step("Thunar on login",   lambda: apply_thunar_autostart(merged)),
            # GF-3.1 / MESHFS-5.1 (v5.0.0) — pin the primary login
            # account to uid:gid 1000:1000 so LizardFS mesh-storage
            # mounts hand out consistent file ownership across all
            # peers. Idempotent + collision-safe (refuses with a
            # log line when uid 1000 is held by a different user
            # rather than silently chowning their data). FS-agnostic.
            _Step("Normalize UID",     lambda: apply_uid_normalize(merged)),
            # MON-1 (v2.6) — write the locked Netdata baseline
            # config + reload. Fail-soft per the 2026-05-24
            # design lock: each peer self-parents with 7d
            # local dbengine retention. Future MON-1.b
            # mackesd publisher rewrites the stream block on
            # leader-flip so children stream to the elected
            # aggregator.
            _Step("Netdata monitoring", lambda: apply_netdata_monitor(merged)),
            # HYP-8.5.birthright (v6.5) — seed the operator's
            # ~/.config/mde/tags/ from the 6 system tag manifests
            # shipped at /usr/share/mde/tag-manifests/. Idempotent:
            # existing destination files are preserved so operator
            # edits survive re-runs.
            _Step("Tag manifest seed", lambda: apply_tag_manifests_seed(merged)),
            _Step("XDG user dirs",     lambda: apply_user_dirs(merged)),
            _Step("Super+M hotkey",    lambda: apply_hotkey(merged)),
            # Phase 10.6.1-4: archive the user's pre-1.0 xfce4-panel state,
            # then start mackes-panel and retire xfce4-panel + xfdesktop.
            _Step("Archive legacy panel", lambda: apply_panel_archive(merged)),
            _Step("Panel swap",        lambda: apply_panel_swap(merged)),
            # Phase 8.8: i3 is now the only window manager. Migrate
            # upgraded 1.0.6 installs that still have xfwm4 running.
            _Step("Enforce i3",        lambda: apply_enforce_i3(merged)),
            # Phase 10.6.6: dnf-remove the six legacy XFCE packages
            # mackes-panel has supplanted. Hard-gated on panel-swap.
            _Step("Uninstall legacy XFCE",
                                       lambda: apply_uninstall_legacy_xfce(merged)),
            # v2.0.1 hotfix: sweep orphan xsession entries from the
            # v1.x xfce11-unified era so LightDM only shows the
            # Wayland MDE session.
            _Step("Uninstall legacy xsessions",
                                       lambda: apply_uninstall_legacy_xsessions(merged)),
            _Step("Mesh",              lambda: apply_mesh(merged)),
            _Step("VPN import",        self._step_vpn),
            _Step("Menu",              self._step_menu),
            _Step("Finalize",          lambda: self._step_finalize(merged)),
        ]

    def _populate_rail(self) -> None:
        for child in list(self._rail.get_children()):
            self._rail.remove(child)
        for step in self._steps:
            self._rail.add(step.build_row())
        self._rail.show_all()

    # ----- run loop (background thread) -----------------------------------

    def _run_loop(self, merged: Preset) -> None:
        completed: List[float] = []
        for i, step in enumerate(self._steps):
            if self._cancel.is_set():
                GLib.idle_add(self._mark_remaining_skipped, i)
                break

            GLib.idle_add(self._start_step, i)
            step_start = time.monotonic()
            failed = False
            try:
                result = step.fn() or []
                lines = list(result)
            except Exception as e:  # noqa: BLE001
                lines = [f"ERROR: {e}"]
                log_action(f"wizard apply {step.name} failed: {e}")
                failed = True
            step.elapsed = time.monotonic() - step_start
            step.log = lines
            completed.append(step.elapsed)

            GLib.idle_add(self._finish_step, i, failed, completed)

        GLib.idle_add(self._finalize_run, merged)

    # ----- step lifecycle (UI thread) -------------------------------------

    def _start_step(self, idx: int) -> bool:
        self._active_idx = idx
        step = self._steps[idx]
        step.set_state(_RUNNING)
        self._set_active_title(step.name)
        self._set_active_sub(self._sub_for_step(step.name))
        self._log.get_buffer().set_text("")
        # Scroll the rail to keep the active step visible
        row = step._row
        if row is not None:
            GLib.idle_add(self._scroll_rail_to, row)
        return False

    def _finish_step(self, idx: int, failed: bool,
                      completed: list[float]) -> bool:
        step = self._steps[idx]
        step.set_state(_FAILED if failed else _DONE)
        # Stream the step's log into the right pane
        buf = self._log.get_buffer()
        for line in step.log:
            self._append_log(buf, f"  {line}")
        # Update overall progress + step counter
        n = idx + 1
        total = len(self._steps)
        self._progress.set_fraction(n / total)
        self._step_count.set_text(f"step {n} of {total}")
        self._pct.set_text(f"{int(100 * n / total)}%")
        # ETA — mean completed step duration × remaining
        if completed:
            mean = sum(completed) / len(completed)
            remaining = max(0, total - n)
            secs = mean * remaining
            self._eta_lbl.set_text(self._format_remaining(secs))
        return False

    def _finalize_run(self, merged: Preset) -> bool:
        self._done = True
        self._cancel_btn.set_sensitive(False)
        if self._on_complete is not None:
            try:
                self._on_complete()
            except Exception:  # noqa: BLE001
                pass
        ok_n = sum(1 for s in self._steps if s.state == _DONE)
        fail_n = sum(1 for s in self._steps if s.state == _FAILED)
        skip_n = sum(1 for s in self._steps if s.state == _SKIPPED)
        if fail_n == 0 and skip_n == 0:
            self._title.set_text(f"You are now {merged.display_name}.")
            self._set_active_title("Done")
            self._set_active_sub(
                f"All {ok_n} steps completed successfully. "
                "Welcome to Mackes."
            )
        elif skip_n > 0 and fail_n == 0:
            self._title.set_text(f"You are now {merged.display_name} (partial).")
            self._set_active_title("Done with cancellations")
            self._set_active_sub(
                f"{ok_n} steps completed · {skip_n} cancelled before running. "
                "Re-run the wizard later to finish the rest."
            )
        else:
            self._title.set_text(f"You are now {merged.display_name} (with errors).")
            self._set_active_title("Done with errors")
            self._set_active_sub(
                f"{ok_n} ok · {fail_n} failed · {skip_n} cancelled. "
                "See the step log above for details. Most failures are "
                "recoverable — open the relevant Maintain panel."
            )
        self._eta_lbl.set_text("")
        return False

    def _mark_remaining_skipped(self, from_idx: int) -> bool:
        for step in self._steps[from_idx:]:
            step.set_state(_SKIPPED)
        return False

    # ----- timing + helpers ------------------------------------------------

    def _tick_clocks(self) -> bool:
        if self._done or self._start_ts is None:
            return False
        elapsed = time.monotonic() - self._start_ts
        self._elapsed_lbl.set_text(f"elapsed {self._format_remaining(elapsed)}")
        # Update active step's elapsed
        if 0 <= self._active_idx < len(self._steps):
            step = self._steps[self._active_idx]
            if step.state == _RUNNING:
                # Approximate — runner thread owns the precise number
                step_start_offset = sum(
                    s.elapsed for s in self._steps[:self._active_idx]
                )
                step.update_elapsed(max(0.0, elapsed - step_start_offset))
        return True

    @staticmethod
    def _format_remaining(secs: float) -> str:
        if secs < 0:
            return "—"
        if secs < 60:
            return f"{int(secs)}s"
        m, s = divmod(int(secs), 60)
        if m < 60:
            return f"{m}m {s:02d}s"
        h, m = divmod(m, 60)
        return f"{h}h {m:02d}m"

    def _set_active_title(self, text: str) -> None:
        self._active_title.set_text(text)

    def _set_active_sub(self, text: str) -> None:
        self._active_sub.set_text(text)

    @staticmethod
    def _sub_for_step(name: str) -> str:
        """Human-friendly per-step subtitle. Kept terse — full detail goes
        into the log lines below."""
        return _STEP_SUBTITLES.get(name,
            "Applying configuration changes for this step…")

    def _append_log(self, buf: Gtk.TextBuffer, line: str) -> None:
        end = buf.get_end_iter()
        buf.insert(end, line + "\n")
        end = buf.get_end_iter()
        self._log.scroll_to_iter(end, 0, False, 0, 1)

    def _scroll_rail_to(self, row: Gtk.ListBoxRow) -> bool:
        try:
            adj = self._rail.get_parent().get_vadjustment()
            alloc = row.get_allocation()
            if alloc.height > 0:
                target = max(0, alloc.y - 80)
                adj.set_value(min(target, adj.get_upper() - adj.get_page_size()))
        except Exception:  # noqa: BLE001
            pass
        return False

    def _on_cancel(self, *_) -> None:
        self._cancel.set()
        self._cancel_btn.set_label("Cancelling…")
        self._cancel_btn.set_sensitive(False)

    # ----- individual steps -----------------------------------------------

    def _step_snapshot(self):
        if not self.ctx.create_initial_snapshot:
            return ["skipped (per wizard choice)"]
        snap = create_snapshot(label=self.ctx.snapshot_label,
                               source_preset=self.ctx.selected_preset.name)
        return [f"created {snap.name}"]

    def _step_vpn(self):
        path = self.ctx.imported_vpn_path
        if not path:
            return ["no VPN to import"]
        if shutil.which("nmcli") is None:
            return ["nmcli not installed; skipping"]
        suffix = path.rsplit(".", 1)[-1].lower()
        vpn_type = "openvpn" if suffix == "ovpn" else "wireguard"
        try:
            out = subprocess.check_output(
                ["nmcli", "connection", "import", "type", vpn_type, "file", path],
                text=True, stderr=subprocess.STDOUT, timeout=10,
            )
            return [out.strip() or f"imported {path}"]
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as e:
            return [f"import failed: {getattr(e, 'output', e)}"]

    def _step_menu(self):
        """v2.0.0 Phase F.10 — mackes.menu_integration retired.
        The .desktop entry is now package-owned (data/applications/
        mde.desktop installed by the RPM); XFCE settings entries
        no longer need hiding because XFCE is gone on v2.0.0."""
        return [
            "Menu integration: no-op on v2.0.0 (entry is package-owned; "
            "XFCE settings panels no longer installed)."
        ]

    def _step_finalize(self, merged):
        from mackes.state import MackesState
        state = MackesState.load()
        state.mark_provisioned(merged.name)
        return [f"state.json marked provisioned with preset={merged.name}"]


# --------------------------------------------------------------------------
# Per-step subtitles — short human-readable descriptions.
# --------------------------------------------------------------------------

_STEP_SUBTITLES = {
    "Snapshot":         "Capturing a restore point of your current config.",
    "Appearance":       "Writing GTK theme · icons · cursor · fonts · wallpaper to xfconf.",
    "Devices":          "Power profile · audio sink — xfconf devices block.",
    "System":           "Workspace count · WM theme · notifications toggle.",
    "Network":          "QNM toggle · firewall zone hint.",
    "Panel":            "Clock plugin format · font · layout.",
    "Themes":           "Copying Orchis-Dark + Shiki-Statler + Black-Sun + Mackes-Carbon to /usr/share/{themes,icons}.",
    "LightDM greeter":  "Mackes-themed login screen — wallpaper / theme / icons / fonts.",
    "Fonts":            "Installing Red Hat Text + Mono via dnf.",
    "Apps":             "Installing preset.apps.install + removing apps.remove_bloat.",
    "Panel layout":     "Writing Mackes default xfce4-panel layout (Whisker + Docklike + clock).",
    "Boot splash":      "Installing + activating the Mackes Plymouth theme (regenerates initrd — slow).",
    "System update":    "dnf upgrade -y --refresh. This is the heaviest step — can take minutes.",
    "Third-party repos": "fedora-workstation-repositories + RPM Fusion free + nonfree.",
    "Flathub":          "Adding the per-user Flathub flatpak remote.",
    "Remote desktop":   "xrdp + x11vnc + guacd + Tomcat + Guacamole. Fetches guacamole.war from Apache.",
    "Fleet management": "ansible-core + 7 curated playbooks + 30-min ansible-pull timer.",
    "Notification drawer": "Mackes notification drawer panel applet — pill + slide-in status drawer.",
    "Maximize windows": "mackes-maximizer.service — every new top-level window starts maximized.",
    "Mesh clipboard":   "Bidirectional XA_CLIPBOARD ↔ QNM-Shared sync with secret-filter.",
    "Quick Network Mesh": "dnf install qnm + qnmctl init + qnm.service.",
    "Thunar on login":  "Open Thunar at ~/QNM-Mesh every graphical login (XDG autostart).",
    "Archive legacy panel": "Copying pre-1.0 ~/.config/xfce4/panel/ to ~/.config/mackes-panel/legacy-xfce-panel/.",
    "Panel swap":       "Starting mackes-panel · stopping xfce4-panel + xfdesktop · rebinding the Whisker Super-key.",
    "Enforce i3":       "Making i3 the active window manager and retiring mackes-maximizer.",
    "Uninstall legacy XFCE": "dnf remove xfce4-panel + xfdesktop + whisker + docklike + pulseaudio + power-manager plugins.",
    "Mesh":             "Headscale + Tailscale keypair + QNM-Shared bucket dirs.",
    "VPN import":       "Optional — imports the .ovpn / .conf you picked earlier.",
    "Menu":             "Hides xfce4-settings entries · installs mackes-shell.desktop.",
    "Finalize":         "Marks state.json provisioned. The Workbench will open next.",
}
