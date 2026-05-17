"""Wizard screen 10 — Apply (progress bar + streaming actions)."""
from __future__ import annotations

import shutil
import subprocess

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk  # noqa: E402

from mackes.logging import log_action
from mackes.presets import (
    Preset, apply_appearance, apply_devices, apply_mesh, apply_network,
    apply_panel, apply_system,
)
from mackes.snapshots import create_snapshot


class ApplyPage(Gtk.Box):
    """A self-contained widget that runs the apply pipeline when triggered."""

    def __init__(self, ctx) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=14)
        self.set_margin_top(40); self.set_margin_bottom(32)
        self.set_margin_start(56); self.set_margin_end(56)
        self.ctx = ctx
        self._done = False

        preset_name = ctx.selected_preset.display_name if ctx.selected_preset else "Mackes"
        self._title = Gtk.Label(label=f"Becoming {preset_name}…")
        self._title.set_xalign(0); self._title.get_style_context().add_class("title-1")
        self.pack_start(self._title, False, False, 0)

        sub = Gtk.Label()
        sub.set_xalign(0); sub.set_line_wrap(True)
        sub.set_markup(
            "<span size='medium'>Each step is logged below. "
            "Cancel any time — anything already applied stays applied.</span>"
        )
        sub.get_style_context().add_class("dim-label")
        self.pack_start(sub, False, False, 0)

        self._progress = Gtk.ProgressBar(); self._progress.set_fraction(0.0)
        self._progress.set_margin_top(8); self._progress.set_margin_bottom(4)
        self.pack_start(self._progress, False, False, 0)

        self._output = Gtk.TextView()
        self._output.set_editable(False); self._output.set_monospace(True)
        scroll = Gtk.ScrolledWindow(); scroll.add(self._output)
        scroll.set_size_request(-1, 320)
        self.pack_start(scroll, True, True, 0)

    def is_done(self) -> bool:
        return self._done

    def append(self, line: str) -> None:
        buf = self._output.get_buffer()
        end = buf.get_end_iter()
        buf.insert(end, line + "\n")
        end = buf.get_end_iter()
        self._output.scroll_to_iter(end, 0, False, 0, 1)
        while Gtk.events_pending():
            Gtk.main_iteration_do(False)

    def _set_progress(self, frac: float) -> None:
        self._progress.set_fraction(min(1.0, max(0.0, frac)))
        while Gtk.events_pending():
            Gtk.main_iteration_do(False)

    def run(self) -> None:
        """Execute the apply pipeline. Idempotent: returns early on re-entry."""
        if self._done:
            return
        ctx = self.ctx
        preset = ctx.selected_preset
        if preset is None:
            self.append("No preset selected. Nothing to apply.")
            self._done = True
            return

        # Build the effective preset by overlaying overrides on top of the
        # preset's declared defaults.
        merged = Preset(
            name=preset.name, display_name=preset.display_name, description=preset.description,
            appearance={**preset.appearance, **(ctx.overrides.get("appearance") or {})},
            devices=   {**preset.devices,    **(ctx.overrides.get("devices") or {})},
            system=    {**preset.system,     **(ctx.overrides.get("system") or {})},
            network=   {**preset.network, "qnm_enabled": ctx.enable_qnm,
                        "firewall_default_zone": ctx.firewall_zone},
            panel=     {**preset.panel,      **(ctx.overrides.get("panel") or {})},
            snapshot=  preset.snapshot,
        )

        steps = [
            ("Snapshot",   self._step_snapshot),
            ("Appearance", lambda: apply_appearance(merged)),
            ("Devices",    lambda: apply_devices(merged)),
            ("System",     lambda: apply_system(merged)),
            ("Network",    lambda: apply_network(merged)),
            ("Panel",      lambda: apply_panel(merged)),
            ("Mesh",       lambda: apply_mesh(merged)),
            ("VPN import", self._step_vpn),
            ("Menu",       self._step_menu),
            ("Finalize",   lambda: self._step_finalize(merged)),
        ]
        total = len(steps)
        for i, (name, fn) in enumerate(steps, start=1):
            self.append(f"→  {name}")
            try:
                for line in fn() or []:
                    self.append(f"   {line}")
            except Exception as e:  # noqa: BLE001
                self.append(f"   ERROR: {e}")
                log_action(f"wizard apply {name} failed: {e}")
            self._set_progress(i / total)

        self._title.set_text(f"You are now {merged.display_name}.")
        self.append("")
        self.append(f"Done. Welcome to {merged.display_name}.")
        self._done = True

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
        from mackes.menu_integration import (
            hide_xfce_settings_entries, install_mackes_menu_entry,
        )
        from pathlib import Path
        out = hide_xfce_settings_entries()
        for c in (Path("/usr/share/applications/mackes-shell.desktop"),
                  Path(__file__).resolve().parent.parent.parent.parent
                  / "data" / "applications" / "mackes-shell.desktop"):
            if c.exists():
                out.extend(install_mackes_menu_entry(c))
                break
        return out

    def _step_finalize(self, merged):
        from mackes.state import MackesState
        state = MackesState.load()
        state.mark_provisioned(merged.name)
        return [f"state.json marked provisioned with preset={merged.name}"]
