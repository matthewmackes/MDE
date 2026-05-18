"""Dedicated Headscale setup wizard — v1.6.1.

Surfaced from Network → Mesh VPN → "Setup wizard". Separate from the
first-run birthright wizard so users can re-run mesh setup without
touching the rest of their config.

Three top-level paths (the user picks on screen 1):

  1. SEED      — become the seed/control node for a NEW mesh. Generates
                 a fresh mesh-id, brings up Headscale serve, creates the
                 default user, issues a pre-auth key, runs `tailscale up`
                 against this peer, and prints a join link the user
                 shares with peers.

  2. JOIN      — join an EXISTING mesh by pasting the join link. Validates
                 the link, redeems the pre-auth, runs `tailscale up`,
                 verifies connectivity by pinging the control node.

  3. RECONFIG  — already on a mesh; let the user re-issue keys, update
                 the ACL policy, or re-elect control. Best-effort
                 idempotent path.

UX: same two-pane Carbon stepped layout as the main wizard's Apply
page — left rail of steps with status glyphs, right pane with live
detail of the active step + log tail. Run loop on a daemon thread so
the GTK main loop stays responsive.
"""
from __future__ import annotations

import threading
import time
from typing import Callable, List, Optional

import gi
gi.require_version("Gtk", "3.0")
gi.require_version("Gdk", "3.0")
from gi.repository import GLib, Gtk  # noqa: E402

from mackes.logging import log_action


_PENDING = "pending"
_RUNNING = "running"
_DONE    = "done"
_FAILED  = "failed"
_SKIPPED = "skipped"

_GLYPHS = {
    _PENDING: "⋯", _RUNNING: "◐", _DONE: "✓", _FAILED: "✗", _SKIPPED: "—",
}


# --------------------------------------------------------------------------
# Step record (mirrors wizard/pages/apply.py:_Step but lightweight)
# --------------------------------------------------------------------------


class _Step:
    __slots__ = ("name", "fn", "state", "elapsed", "log",
                 "_row", "_glyph_lbl", "_time_lbl")

    def __init__(self, name: str, fn: Callable[[], List[str]]) -> None:
        self.name = name
        self.fn = fn
        self.state = _PENDING
        self.elapsed = 0.0
        self.log: List[str] = []
        self._row = None
        self._glyph_lbl = None
        self._time_lbl = None

    def build_row(self) -> Gtk.ListBoxRow:
        row = Gtk.ListBoxRow()
        row.set_activatable(False); row.set_selectable(False)
        row.get_style_context().add_class("mackes-side-nav-item")
        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        box.set_margin_top(8); box.set_margin_bottom(8)
        box.set_margin_start(16); box.set_margin_end(16)
        self._glyph_lbl = Gtk.Label(label=_GLYPHS[self.state])
        self._glyph_lbl.set_size_request(20, -1); self._glyph_lbl.set_xalign(0)
        self._glyph_lbl.get_style_context().add_class("mackes-dot")
        self._glyph_lbl.get_style_context().add_class("muted")
        box.pack_start(self._glyph_lbl, False, False, 0)
        name_lbl = Gtk.Label(label=self.name); name_lbl.set_xalign(0)
        box.pack_start(name_lbl, True, True, 0)
        self._time_lbl = Gtk.Label(label="—")
        self._time_lbl.set_xalign(1)
        self._time_lbl.get_style_context().add_class("mackes-section-meta")
        box.pack_end(self._time_lbl, False, False, 0)
        row.add(box)
        self._row = row
        return row

    def set_state(self, state: str) -> None:
        self.state = state
        if self._glyph_lbl is None:
            return
        self._glyph_lbl.set_text(_GLYPHS[state])
        ctx = self._glyph_lbl.get_style_context()
        for v in ("ok", "warn", "fail", "muted", "accent"):
            ctx.remove_class(v)
        ctx.add_class({
            _RUNNING: "accent", _DONE: "ok", _FAILED: "fail",
            _SKIPPED: "muted", _PENDING: "muted",
        }[state])
        if self._row is not None:
            rctx = self._row.get_style_context()
            if state == _RUNNING:
                rctx.add_class("active")
            else:
                rctx.remove_class("active")
        if self._time_lbl is not None:
            self._time_lbl.set_text(
                "—" if state == _PENDING else f"{self.elapsed:.1f}s")


# --------------------------------------------------------------------------
# Window — Gtk.Window because we don't want it to claim the GtkApplication
# slot (the main shell already owns it).
# --------------------------------------------------------------------------


class HeadscaleSetupWindow(Gtk.Window):
    def __init__(self, parent: Optional[Gtk.Window] = None) -> None:
        super().__init__()
        from mackes.workbench._common import versioned_title
        self.set_title(versioned_title("Mesh VPN — Headscale Setup Wizard"))
        self.set_default_size(1100, 720)
        self.set_modal(False)
        if parent is not None:
            self.set_transient_for(parent)
        self.get_style_context().add_class("mackes-app-window")

        self._steps: List[_Step] = []
        self._cancel = threading.Event()
        self._done = False
        self._start_ts: Optional[float] = None
        self._active_idx = -1
        self._mode: Optional[str] = None    # "seed" / "join" / "reconfig"
        self._join_link: str = ""
        self._mesh_id: str = ""

        self._build_intro_page()

    # ----- Intro: pick mode ------------------------------------------------

    def _build_intro_page(self) -> None:
        for c in list(self.get_children()):
            self.remove(c)
        outer = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        outer.set_margin_top(48); outer.set_margin_bottom(32)
        outer.set_margin_start(56); outer.set_margin_end(56)

        title = Gtk.Label(label="Mesh setup")
        title.set_xalign(0)
        title.get_style_context().add_class("mackes-page-title")
        outer.pack_start(title, False, False, 0)
        sub = Gtk.Label(label=(
            "Join an existing Mackes mesh, or host a new one. "
            "Mackes picks the right tools and configures everything for you — "
            "re-running setup is safe on an already-provisioned peer."
        ))
        sub.set_xalign(0); sub.set_line_wrap(True)
        sub.get_style_context().add_class("mackes-page-subtitle")
        outer.pack_start(sub, False, False, 0)

        # Two outcome-driven cards: Join / Host. "Reconfig" (re-run on a
        # provisioned peer) folds into Host — host_run is idempotent.
        cards = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=16)
        cards.set_margin_top(32)
        for mode, glyph, name, blurb in (
            ("join", "+", "Join an existing mesh",
             "Paste a join link from a peer (or scan one in the clipboard). "
             "Mackes redeems the pre-auth, brings up tailscale, and verifies "
             "the link works."),
            ("seed", "★", "Host a new mesh",
             "Become the first peer of a brand-new mesh. Mackes generates "
             "the mesh-id, brings up Headscale, and gives you a join link "
             "to share with other peers. Safe to re-run."),
        ):
            cards.pack_start(self._make_role_card(mode, glyph, name, blurb),
                             True, True, 0)
        outer.pack_start(cards, False, False, 0)

        # Bottom: Cancel
        bar = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        bar.set_margin_top(40)
        bar.pack_end(self._mk_btn("Cancel", "cds-button-ghost",
                                   lambda *_: self.destroy()),
                     False, False, 0)
        outer.pack_start(bar, False, False, 0)

        self.add(outer)
        self.show_all()

    def _make_role_card(self, mode: str, glyph: str, name: str,
                         blurb: str) -> Gtk.Widget:
        btn = Gtk.Button()
        btn.set_relief(Gtk.ReliefStyle.NONE)
        btn.connect("clicked", lambda *_: self._on_pick_mode(mode))

        card = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        card.get_style_context().add_class("mackes-app-card")
        card.set_size_request(-1, 220)

        head = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        icon = Gtk.Label(label=glyph)
        icon.get_style_context().add_class("mackes-app-icon")
        icon.set_size_request(48, 48)
        head.pack_start(icon, False, False, 0)
        title = Gtk.Label(label=name); title.set_xalign(0)
        title.get_style_context().add_class("mackes-app-name")
        head.pack_start(title, True, True, 0)
        card.pack_start(head, False, False, 0)

        body = Gtk.Label(label=blurb); body.set_xalign(0); body.set_line_wrap(True)
        body.set_max_width_chars(36)
        body.get_style_context().add_class("mackes-app-desc")
        card.pack_start(body, True, True, 0)

        btn.add(card)
        return btn

    @staticmethod
    def _mk_btn(label: str, css_class: str, on_click) -> Gtk.Button:
        b = Gtk.Button(label=label)
        b.get_style_context().add_class(css_class)
        b.connect("clicked", on_click)
        return b

    # ----- Mode selected ---------------------------------------------------

    def _on_pick_mode(self, mode: str) -> None:
        self._mode = mode
        if mode == "join":
            self._show_join_link_input()
        else:
            self._build_run_page()

    # ----- Join: ask for the link ------------------------------------------

    def _show_join_link_input(self) -> None:
        for c in list(self.get_children()):
            self.remove(c)
        outer = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        outer.set_margin_top(48); outer.set_margin_bottom(32)
        outer.set_margin_start(56); outer.set_margin_end(56)

        title = Gtk.Label(label="Paste your join link"); title.set_xalign(0)
        title.get_style_context().add_class("mackes-page-title")
        outer.pack_start(title, False, False, 0)
        sub = Gtk.Label(label=(
            "Open Mesh VPN on the seed peer, click 'Add peer', and copy the "
            "join link it generates. It looks like: "
            "mackes://join/<mesh-id>?key=<token>&control=https://<ip>:8080"
        ))
        sub.set_xalign(0); sub.set_line_wrap(True)
        sub.get_style_context().add_class("mackes-page-subtitle")
        outer.pack_start(sub, False, False, 0)

        entry = Gtk.Entry()
        entry.set_placeholder_text("mackes://join/...")
        entry.set_margin_top(24)
        entry.set_activates_default(True)
        # Pre-fill from clipboard if a mackes:// link is already there —
        # by far the common case is the user just copied it from the
        # peer's "Add peer" button on another machine.
        try:
            from gi.repository import Gdk
            clip = Gtk.Clipboard.get(Gdk.SELECTION_CLIPBOARD)
            text = clip.wait_for_text() if clip is not None else None
            if text and text.strip().startswith("mackes://"):
                entry.set_text(text.strip())
        except Exception:  # noqa: BLE001
            pass
        outer.pack_start(entry, False, False, 0)
        self._join_entry = entry

        bar = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        bar.set_margin_top(24)
        continue_btn = self._mk_btn("Continue", "cds-button-primary",
                                     lambda *_: self._on_join_link_submitted())
        continue_btn.set_can_default(True)
        continue_btn.grab_default()
        bar.pack_end(continue_btn, False, False, 0)
        bar.pack_end(self._mk_btn("Back", "cds-button-ghost",
                                   lambda *_: self._build_intro_page()),
                     False, False, 0)
        outer.pack_start(bar, False, False, 0)

        self.add(outer)
        self.show_all()
        # Focus the entry so the user can hit Ctrl+V → Enter without a
        # mouse click.
        entry.grab_focus_without_selecting()

    def _on_join_link_submitted(self) -> None:
        self._join_link = self._join_entry.get_text().strip()
        if not self._join_link:
            return
        self._build_run_page()

    # ----- Run page: stepped UX -------------------------------------------

    def _build_run_page(self) -> None:
        for c in list(self.get_children()):
            self.remove(c)

        outer = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        outer.set_margin_top(32); outer.set_margin_bottom(24)
        outer.set_margin_start(40); outer.set_margin_end(40)

        # Header
        head = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        title_text = {
            "seed":     "Seeding a new mesh…",
            "join":     "Joining the mesh…",
            "reconfig": "Reconfiguring this peer…",
        }[self._mode]
        self._title = Gtk.Label(label=title_text); self._title.set_xalign(0)
        self._title.get_style_context().add_class("mackes-page-title")
        head.pack_start(self._title, True, True, 0)
        self._cancel_btn = self._mk_btn("Cancel", "cds-button-tertiary",
                                         self._on_cancel)
        head.pack_end(self._cancel_btn, False, False, 0)
        outer.pack_start(head, False, False, 0)

        sub = Gtk.Label(label=(
            "Each step runs in order. You can cancel any time — anything "
            "already applied stays applied. Failed steps stop the wizard "
            "so you can review the log before re-running."
        ))
        sub.set_xalign(0); sub.set_line_wrap(True)
        sub.get_style_context().add_class("mackes-page-subtitle")
        outer.pack_start(sub, False, False, 0)

        # Progress strip
        prog_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        prog_row.set_margin_top(16); prog_row.set_margin_bottom(8)
        self._step_count = Gtk.Label(label="step 0 of 0")
        self._step_count.set_xalign(0); self._step_count.set_size_request(110, -1)
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

        # Body — rail + detail
        body = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=24)
        body.set_margin_top(16)

        rail_wrap = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        rail_wrap.set_size_request(320, -1)
        rail_wrap.get_style_context().add_class("mackes-side-nav")
        hdr = Gtk.Label(label="STEPS")
        hdr.set_xalign(0); hdr.set_margin_top(12); hdr.set_margin_bottom(4)
        hdr.set_margin_start(16); hdr.set_margin_end(16)
        hdr.get_style_context().add_class("mackes-side-nav-group-title")
        rail_wrap.pack_start(hdr, False, False, 0)
        self._rail = Gtk.ListBox()
        self._rail.set_selection_mode(Gtk.SelectionMode.NONE)
        rail_scroller = Gtk.ScrolledWindow()
        rail_scroller.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        rail_scroller.add(self._rail)
        rail_wrap.pack_start(rail_scroller, True, True, 0)
        body.pack_start(rail_wrap, False, False, 0)

        detail = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        self._active_title = Gtk.Label(label="Starting…")
        self._active_title.set_xalign(0)
        self._active_title.get_style_context().add_class("mackes-section-title")
        detail.pack_start(self._active_title, False, False, 0)
        self._active_sub = Gtk.Label(label="")
        self._active_sub.set_xalign(0); self._active_sub.set_line_wrap(True)
        self._active_sub.get_style_context().add_class("mackes-page-subtitle")
        detail.pack_start(self._active_sub, False, False, 0)
        self._log = Gtk.TextView()
        self._log.set_editable(False); self._log.set_monospace(True)
        self._log.set_wrap_mode(Gtk.WrapMode.WORD_CHAR)
        self._log.get_style_context().add_class("mackes-code")
        log_scroller = Gtk.ScrolledWindow()
        log_scroller.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        log_scroller.add(self._log)
        log_scroller.set_margin_top(12)
        detail.pack_start(log_scroller, True, True, 0)
        body.pack_start(detail, True, True, 0)

        outer.pack_start(body, True, True, 0)

        # Footer
        foot = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        foot.set_margin_top(16)
        self._close_btn = self._mk_btn("Close", "cds-button-tertiary",
                                        lambda *_: self.destroy())
        self._close_btn.set_sensitive(False)
        foot.pack_end(self._close_btn, False, False, 0)
        outer.pack_start(foot, False, False, 0)

        self.add(outer)
        self.show_all()

        # Build the steps + start
        self._steps = self._build_steps()
        for s in self._steps:
            self._rail.add(s.build_row())
        self._rail.show_all()
        self._step_count.set_text(f"step 0 of {len(self._steps)}")
        self._start_ts = time.monotonic()
        threading.Thread(target=self._run_loop, daemon=True,
                         name="mackes-headscale-wizard").start()

    # ----- Step lists per mode --------------------------------------------

    def _build_steps(self) -> List[_Step]:
        if self._mode == "seed":
            return [
                _Step("Detect role + mesh-id",   self._s_detect_or_mint),
                _Step("Write Headscale config",  self._s_write_config),
                _Step("Start headscale.service", self._s_start_headscale),
                _Step("Create mesh user",        self._s_create_user),
                _Step("Issue pre-auth key",      self._s_issue_preauth),
                _Step("Run tailscale up",        self._s_tailscale_up),
                _Step("Verify status",           self._s_verify),
                _Step("Generate join link",      self._s_generate_link),
            ]
        if self._mode == "join":
            return [
                _Step("Parse join link",         self._s_parse_link),
                _Step("Join via tailscale up",   self._s_tailscale_join_up),
                _Step("Verify status",           self._s_verify),
                _Step("Confirm peer visibility", self._s_list_peers),
            ]
        # reconfig
        return [
            _Step("Inspect current state",       self._s_inspect),
            _Step("Refresh ACL policy",          self._s_refresh_acl),
            _Step("Maybe take control",          self._s_maybe_take_control),
            _Step("Snapshot mesh state",         self._s_snapshot_state),
        ]

    # ----- Run loop --------------------------------------------------------

    def _run_loop(self) -> None:
        completed: List[float] = []
        any_failed = False
        for i, step in enumerate(self._steps):
            if self._cancel.is_set():
                GLib.idle_add(self._mark_remaining_skipped, i)
                break
            GLib.idle_add(self._start_step, i)
            t = time.monotonic()
            try:
                lines = step.fn() or []
            except Exception as e:  # noqa: BLE001
                lines = [f"ERROR: {e}"]
                step.set_state(_FAILED)
                any_failed = True
                log_action(f"headscale wizard {step.name} failed: {e}")
                step.elapsed = time.monotonic() - t
                step.log = lines
                GLib.idle_add(self._finish_step, i, True, completed)
                # Stop on first failure — caller can re-run after fixing.
                break
            step.elapsed = time.monotonic() - t
            step.log = lines
            completed.append(step.elapsed)
            GLib.idle_add(self._finish_step, i, False, completed)
        GLib.idle_add(self._finalize, any_failed)

    def _start_step(self, idx: int) -> bool:
        self._active_idx = idx
        step = self._steps[idx]
        step.set_state(_RUNNING)
        self._active_title.set_text(step.name)
        self._active_sub.set_text(_SUBTITLES.get(step.name, ""))
        self._log.get_buffer().set_text("")
        return False

    def _finish_step(self, idx: int, failed: bool,
                      completed: list[float]) -> bool:
        step = self._steps[idx]
        if not failed:
            step.set_state(_DONE)
        n = idx + 1
        total = len(self._steps)
        self._progress.set_fraction(n / total)
        self._step_count.set_text(f"step {n} of {total}")
        self._pct.set_text(f"{int(100 * n / total)}%")
        # Stream the log lines into the right pane
        buf = self._log.get_buffer()
        for line in step.log:
            end = buf.get_end_iter()
            buf.insert(end, f"  {line}\n")
            end = buf.get_end_iter()
            self._log.scroll_to_iter(end, 0, False, 0, 1)
        return False

    def _finalize(self, any_failed: bool) -> bool:
        self._done = True
        self._cancel_btn.set_sensitive(False)
        self._close_btn.set_sensitive(True)
        if any_failed:
            self._title.set_text("Mesh setup stopped on error")
            self._active_title.set_text("Stopped on error")
            self._active_sub.set_text(
                "Review the step log above. Most failures are recoverable: "
                "fix the underlying issue (often headscale.service config "
                "or firewall) and re-run this wizard."
            )
        else:
            if self._mode == "seed":
                self._title.set_text("Mesh is up — share the join link")
                # Print the join link prominently in the log
                buf = self._log.get_buffer()
                end = buf.get_end_iter()
                buf.insert(end, "\n----\nJoin link:\n  "
                           f"{self._join_link or '(see Mesh VPN panel)'}\n")
            elif self._mode == "join":
                self._title.set_text("Joined the mesh")
            else:
                self._title.set_text("Mesh reconfigured")
            self._active_title.set_text("Done")
            self._active_sub.set_text(
                "All steps completed. Close this wizard and head over to "
                "Mesh VPN to see the live topology."
            )
        return False

    def _mark_remaining_skipped(self, from_idx: int) -> bool:
        for s in self._steps[from_idx:]:
            s.set_state(_SKIPPED)
        return False

    def _on_cancel(self, *_) -> None:
        self._cancel.set()
        self._cancel_btn.set_label("Cancelling…")
        self._cancel_btn.set_sensitive(False)

    # ----- Steps — SEED ----------------------------------------------------

    def _s_detect_or_mint(self) -> List[str]:
        actions: List[str] = []
        from mackes.mesh_vpn import is_first_peer
        if is_first_peer():
            actions.append("this peer has no mesh state; minting a fresh mesh-id")
        else:
            actions.append("warning: peer already has mesh state; will overwrite")
        import secrets
        self._mesh_id = secrets.token_hex(8)
        actions.append(f"mesh-id: {self._mesh_id}")
        return actions

    def _s_write_config(self) -> List[str]:
        from mackes.mesh_vpn import _ensure_headscale_config
        return _ensure_headscale_config(self._mesh_id)

    def _s_start_headscale(self) -> List[str]:
        from mackes.mesh_vpn import headscale_start_as_control
        return headscale_start_as_control(self._mesh_id)

    def _s_create_user(self) -> List[str]:
        from mackes.mesh_vpn import headscale_create_user
        return headscale_create_user("mesh")

    def _s_issue_preauth(self) -> List[str]:
        from mackes.mesh_vpn import headscale_generate_preauth_key
        return headscale_generate_preauth_key(user="mesh", expiration="24h")

    def _s_tailscale_up(self) -> List[str]:
        from mackes.mesh_vpn import tailscale_up_with_headscale
        return tailscale_up_with_headscale(user="mesh")

    def _s_verify(self) -> List[str]:
        from mackes.mesh_vpn import tailscale_status
        ts = tailscale_status()
        return [
            f"installed: {ts.get('installed')}",
            f"online: {ts.get('online')}",
            f"mesh IP: {ts.get('mesh_ip') or '—'}",
            f"peers: {len(ts.get('peers') or [])}",
        ]

    def _s_generate_link(self) -> List[str]:
        from mackes.mesh_vpn import generate_join_link
        link, actions = generate_join_link(expiration="24h")
        if link:
            self._join_link = link
            actions.append(f"join link: {link}")
        return actions

    # ----- Steps — JOIN ----------------------------------------------------

    def _s_parse_link(self) -> List[str]:
        from mackes.mesh_vpn import parse_join_link
        info = parse_join_link(self._join_link)
        if not info:
            raise RuntimeError(f"invalid join link: {self._join_link!r}")
        return [f"{k}: {v}" for k, v in info.items()]

    def _s_tailscale_join_up(self) -> List[str]:
        from mackes.mesh_vpn import join_with_retry, redeem_join_code
        info = redeem_join_code(self._join_link)
        if not info:
            raise RuntimeError("could not redeem join link")
        # join_with_retry handles the three-attempt auto-heal chain:
        # straight try → restart tailscaled → flush state + re-redeem.
        # The user only sees the structured log; on third failure the
        # step is marked FAILED and the run page surfaces a retry hint.
        success, transcript = join_with_retry(
            headscale_url=info.get("control") or "",
            preauth_key=info.get("key") or "",
            hostname=info.get("user") or None,
        )
        if not success:
            raise RuntimeError("could not join after 3 attempts — "
                               "review the log and try the Join button "
                               "again, or run the Mesh → Advanced "
                               "diagnostics for a deeper look")
        return transcript

    def _s_list_peers(self) -> List[str]:
        from mackes.mesh_vpn import headscale_list_peers, tailscale_status
        try:
            peers = headscale_list_peers()
            return [
                f"{'●' if p.online else '○'} {p.name:<14} {p.mesh_ip or '—'}"
                for p in peers
            ] or ["(no peers visible yet — control node may still be propagating)"]
        except Exception as e:  # noqa: BLE001
            ts = tailscale_status()
            return [f"tailscale online: {ts.get('online')} ({e})"]

    # ----- Steps — RECONFIG -----------------------------------------------

    def _s_inspect(self) -> List[str]:
        from mackes.mesh_vpn import MeshState, tailscale_status
        st = MeshState.load()
        ts = tailscale_status()
        return [
            f"mesh_id: {st.mesh_id or '—'}",
            f"control_peer_id: {st.control_peer_id or '—'}",
            f"is_control: {st.is_control}",
            f"tailscale online: {ts.get('online')}",
        ]

    def _s_refresh_acl(self) -> List[str]:
        from mackes.mesh_ssh import save_policy_yaml, load_policy_yaml
        return save_policy_yaml(load_policy_yaml())

    def _s_maybe_take_control(self) -> List[str]:
        from mackes.mesh_vpn import maybe_take_control
        return maybe_take_control()

    def _s_snapshot_state(self) -> List[str]:
        from mackes.mesh_vpn import snapshot_state
        return snapshot_state()


# --------------------------------------------------------------------------
# Per-step subtitles
# --------------------------------------------------------------------------

_SUBTITLES = {
    "Detect role + mesh-id":   "Generate a fresh mesh-id (16 hex chars) for the new mesh.",
    "Write Headscale config":  "Write /etc/headscale/config.yaml (listen + key paths).",
    "Start headscale.service": "Enable + start headscale via systemd (NOPASSWD via sudoers).",
    "Create mesh user":        "headscale users create mesh.",
    "Issue pre-auth key":      "headscale preauthkeys create --user mesh --expiration 24h.",
    "Run tailscale up":        "tailscale up against the local Headscale + redeem the pre-auth.",
    "Verify status":           "tailscale status — confirm we're online with a mesh IP.",
    "Generate join link":      "Build a mackes://join/… URL to share with other peers.",

    "Parse join link":         "Decode the mackes:// URL — mesh-id, control URL, pre-auth.",
    "Join via tailscale up":   "tailscale up --login-server=… --authkey=… --advertise-routes=.",
    "Confirm peer visibility": "Ask Headscale for the live peer list to confirm registration.",

    "Inspect current state":   "Read MeshState + tailscale status to learn what's running.",
    "Refresh ACL policy":      "Re-push /etc/headscale/acls.hujson to Headscale.",
    "Maybe take control":      "If we're eligible + control is absent > 120s, take the role.",
    "Snapshot mesh state":     "Save the current MeshState to ~/.local/share/mackes-shell/mesh.json.",
}
