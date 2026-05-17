"""First-run wizard (Gtk.Assistant) — spec §5.

Ten pages: welcome, env scan, preset pick, appearance, shell, hardware,
network, snapshot, review, apply (+ summary closer). Pages share a
WizardContext that the apply step reduces into preset+overrides+actions.
"""
from __future__ import annotations

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk  # noqa: E402

from mackes.presets import list_presets
from mackes.state import MackesState
from mackes.wizard.context import WizardContext
from mackes.wizard.pages import (
    appearance, apply, env_scan, hardware, network, preset_pick, review,
    snapshot, welcome,
)


class WizardWindow(Gtk.Assistant):
    def __init__(self, application: Gtk.Application, state: MackesState) -> None:
        super().__init__(application=application)
        self.set_default_size(900, 680)
        self.set_title("Mackes Shell — Setup")
        self.state = state
        self.ctx = WizardContext()

        self._apply_page = apply.ApplyPage(self.ctx)

        # Q2 lock: when only one GUI-visible preset ships, auto-select it and
        # skip Screen 3 entirely. The headless 'node' preset is filtered out
        # of the GUI wizard (Q-HL4 lock) — it's auto-selected by `mackes init`
        # when no display is present.
        shipped_presets = [p for p in list_presets() if p.name != "node"]
        single_preset = len(shipped_presets) == 1
        if single_preset:
            self.ctx.selected_preset = shipped_presets[0]

        # (page_widget, type, title)
        self._pages: list[tuple[Gtk.Widget, Gtk.AssistantPageType, str]] = [
            (welcome.build(self.ctx),       Gtk.AssistantPageType.INTRO,    "Welcome"),
            (env_scan.build(self.ctx),      Gtk.AssistantPageType.CONTENT,  "Environment Scan"),
        ]
        if not single_preset:
            self._pages.append(
                (preset_pick.build(self.ctx), Gtk.AssistantPageType.CONTENT, "Choose Preset")
            )
        self._pages.extend([
            (appearance.build(self.ctx),    Gtk.AssistantPageType.CONTENT,  "Appearance"),
            (hardware.build(self.ctx),      Gtk.AssistantPageType.CONTENT,  "Hardware"),
            (network.build(self.ctx),       Gtk.AssistantPageType.CONTENT,  "Network"),
            (snapshot.build(self.ctx),      Gtk.AssistantPageType.CONTENT,  "Restore Point"),
            (review.build(self.ctx),        Gtk.AssistantPageType.CONFIRM,  "Review"),
            (self._apply_page,              Gtk.AssistantPageType.PROGRESS, "Apply"),
            (self._build_summary(),         Gtk.AssistantPageType.SUMMARY,  "Welcome to Mackes"),
        ])
        for widget, page_type, title in self._pages:
            scroller = Gtk.ScrolledWindow()
            scroller.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
            scroller.add(widget)
            self.append_page(scroller)
            self.set_page_title(scroller, title)
            self.set_page_type(scroller, page_type)
            # CONTENT/INTRO/CONFIRM/SUMMARY pages are always complete; the
            # PROGRESS page completes when the apply pipeline finishes.
            if page_type != Gtk.AssistantPageType.PROGRESS:
                self.set_page_complete(scroller, True)

        self.connect("prepare", self._on_prepare)
        self.connect("cancel", lambda *_: self.destroy())
        self.connect("apply", self._on_apply)
        self.connect("close", lambda *_: self.destroy())

    # ----- helpers --------------------------------------------------------

    def _build_summary(self) -> Gtk.Widget:
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
        box.set_margin_top(32); box.set_margin_bottom(32)
        box.set_margin_start(40); box.set_margin_end(40)
        title = Gtk.Label(label="Welcome to Mackes")
        title.set_xalign(0); title.get_style_context().add_class("title-1")
        box.pack_start(title, False, False, 0)
        body = Gtk.Label(label=(
            "Your machine is provisioned. Mackes will open the Dashboard next.\n\n"
            "Tips:\n"
            "  • Maintain → Snapshots lets you create restore points before risky changes.\n"
            "  • Maintain → Reset to Preset reapplies the preset if drift accumulates.\n"
            "  • The header gear menu has a link back to this wizard.\n"
        ))
        body.set_xalign(0); body.set_line_wrap(True)
        box.pack_start(body, False, False, 0)
        return box

    # ----- signals --------------------------------------------------------

    def _on_prepare(self, _assistant: "WizardWindow", page: Gtk.Widget) -> None:
        # When the user lands on the Apply page, kick off the apply pipeline,
        # then mark the page complete so they can advance to Summary.
        idx = self.get_current_page()
        widget, page_type, _ = self._pages[idx]
        if page_type == Gtk.AssistantPageType.PROGRESS:
            # Run on idle so the page is realized before we start writing
            def _run():
                self._apply_page.run()
                self.set_page_complete(page, True)
                return False
            GLib.idle_add(_run)

    def _on_apply(self, *_):
        # Gtk.Assistant fires `apply` after the user clicks "Apply" on the
        # CONFIRM page; subsequent navigation lands on the PROGRESS page,
        # which triggers _on_prepare above. Nothing extra to do here.
        pass
