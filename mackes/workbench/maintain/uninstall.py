"""Maintain → Uninstall (Q8 lock — 7th sub-panel).

Single-checkbox confirmation (Q23) + streaming log + progress bar (Q24).
Best-effort sequencing handled by mackes.uninstall.run_uninstall (Q26).
Post-uninstall logout countdown dialog with opt-out (Q25).
"""
from __future__ import annotations

import threading

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk  # noqa: E402

from mackes.uninstall import run_uninstall, schedule_logout
from mackes.workbench._common import (
    info_label, panel_box, section_header, title_label,
)


_PLAN_LINES = [
    "1. Create pre-uninstall snapshot and tarball it to ~/Desktop/.",
    "2. Reset xfconf channels to XFCE distribution defaults and signal xfsettingsd.",
    "3. Run install-helpers/restore-xfce-settings.sh to un-hide xfce4-settings menus.",
    "4. Delete ~/.config/mackes-shell and ~/.local/share/mackes-shell (snapshots, logs).",
    "5. Remove xfce11-unified v2.2 leftovers from known paths (QNM preserved).",
    "6. Remove mackes-shell via dnf / pip / (git checkout left in place).",
    "7. Write the uninstall log to ~/Desktop/mackes-shell-uninstall-<ts>.log.",
]


class UninstallPanel(Gtk.Box):
    def __init__(self) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self._build()

    def _build(self) -> None:
        box = panel_box()
        box.pack_start(title_label("Uninstall Mackes Shell"), False, False, 0)
        warn = info_label(
            "This removes Mackes Shell and every change it made to your "
            "system. A pre-uninstall snapshot will be written to "
            "~/Desktop/ as a tarball — that is the only artifact you can "
            "restore from. The standard XFCE shell (xfce4-panel, xfdesktop, "
            "Whisker Menu, etc.) keeps running afterwards."
        )
        warn.get_style_context().add_class("warning")
        box.pack_start(warn, False, False, 0)

        box.pack_start(section_header("Plan"), False, False, 0)
        for line in _PLAN_LINES:
            lbl = Gtk.Label(label=line); lbl.set_xalign(0); lbl.set_line_wrap(True)
            lbl.get_style_context().add_class("dim-label")
            box.pack_start(lbl, False, False, 0)

        self._consent = Gtk.CheckButton(label="I understand this removes Mackes Shell and all its files.")
        box.pack_start(self._consent, False, False, 0)

        bar = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        self._run_btn = Gtk.Button(label="Uninstall Mackes Shell")
        self._run_btn.get_style_context().add_class("destructive-action")
        self._run_btn.set_sensitive(False)
        self._consent.connect("toggled",
                              lambda c: self._run_btn.set_sensitive(c.get_active()))
        self._run_btn.connect("clicked", lambda *_: self._run())
        bar.pack_start(self._run_btn, False, False, 0)
        box.pack_start(bar, False, False, 0)

        self._progress = Gtk.ProgressBar()
        self._progress.set_show_text(True)
        box.pack_start(self._progress, False, False, 0)

        box.pack_start(section_header("Live log"), False, False, 0)
        self._log = Gtk.TextView(); self._log.set_editable(False)
        self._log.set_monospace(True); self._log.set_wrap_mode(Gtk.WrapMode.WORD_CHAR)
        sw = Gtk.ScrolledWindow(); sw.set_min_content_height(260); sw.add(self._log)
        box.pack_start(sw, True, True, 0)

        self.add(box)
        self._step_count = 0
        # Rough estimate so the progress bar moves; underestimated rather than
        # overestimated, since the user sees individual lines either way.
        self._expected_steps = 30

    def _append(self, line: str) -> None:
        buf = self._log.get_buffer()
        buf.insert(buf.get_end_iter(), line + "\n")
        # auto-scroll
        mark = buf.get_insert()
        self._log.scroll_to_mark(mark, 0.0, True, 0.5, 1.0)
        self._step_count += 1
        frac = min(1.0, self._step_count / self._expected_steps)
        self._progress.set_fraction(frac)
        self._progress.set_text(f"{self._step_count} step(s)")

    def _on_progress(self, line: str) -> None:
        GLib.idle_add(self._append, line)

    def _run(self) -> None:
        self._run_btn.set_sensitive(False)
        self._consent.set_sensitive(False)
        self._append("--- uninstall starting ---")

        def worker() -> None:
            try:
                report = run_uninstall(progress=self._on_progress)
            except Exception as e:  # noqa: BLE001
                GLib.idle_add(self._append, f"FATAL: {e}")
                return
            GLib.idle_add(self._on_done, report)

        threading.Thread(target=worker, daemon=True).start()

    def _on_done(self, report) -> bool:
        self._progress.set_fraction(1.0)
        self._progress.set_text(f"Done — {report.failed_count} failure(s)")
        self._append("")
        self._append(f"Failed steps: {report.failed_count} of {len(report.steps)}")
        if report.log_path is not None:
            self._append(f"Full log: {report.log_path}")
        if report.desktop_tarball is not None:
            self._append(f"Final snapshot: {report.desktop_tarball}")
        self._show_logout_dialog()
        return False

    def _show_logout_dialog(self) -> None:
        """Q25 lock: 10-second countdown logout with Stay-logged-in opt-out."""
        dialog = Gtk.Dialog(
            title="Uninstall complete",
            transient_for=self.get_toplevel(), modal=True,
        )
        stay = dialog.add_button("Stay logged in", Gtk.ResponseType.CANCEL)
        out = dialog.add_button("Log out now", Gtk.ResponseType.OK)
        content = dialog.get_content_area()
        content.set_margin_top(16); content.set_margin_bottom(16)
        content.set_margin_start(20); content.set_margin_end(20)
        body = Gtk.Label(label=(
            "Mackes Shell has been uninstalled.\n\n"
            "Log out in 10s for a clean desktop session — xfce4-panel will "
            "take over and your appearance will reset to XFCE defaults."
        ))
        body.set_line_wrap(True); body.set_xalign(0)
        content.add(body)
        dialog.show_all()
        # Auto-fire logout after 10s unless cancelled.
        countdown = {"sec": 10}
        def tick() -> bool:
            countdown["sec"] -= 1
            if countdown["sec"] <= 0:
                dialog.response(Gtk.ResponseType.OK)
                return False
            out.set_label(f"Log out now ({countdown['sec']}s)")
            return True
        timer = GLib.timeout_add_seconds(1, tick)
        response = dialog.run()
        GLib.source_remove(timer)
        dialog.destroy()
        if response == Gtk.ResponseType.OK:
            schedule_logout(1)
