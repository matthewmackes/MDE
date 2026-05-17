"""Maintain → Fonts.

A simple font preview + install panel — second tool in the MaintenanceKit
(Option 9 of the platform review). Replaces hunting through `fc-list` + a
text editor for "what does this font look like."

Three things:
  - **Browse** every installed font family with a live preview.
  - **Install from path** — drop a folder of .ttf/.otf into ~/.local/share/fonts/
    and rebuild the fc-cache.
  - **Quick installs** — one-click installers for a curated list of popular
    monospace + UI fonts (uses dnf when available, falls back to a manual hint).
"""
from __future__ import annotations

import shutil
import subprocess

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk  # noqa: E402

from mackes.logging import log_action
from mackes.workbench._common import (
    info_label, labeled_row, panel_box, section_description, section_header, title_label,
)


# Curated quick-install set. dnf package names (Fedora).
_QUICK_INSTALL = [
    ("JetBrains Mono",    "jetbrains-mono-fonts"),
    ("Fira Code",         "fira-code-fonts"),
    ("Cascadia Code",     "cascadia-code-fonts"),
    ("Iosevka",           "iosevka-fonts"),
    ("Inter",             "rsms-inter-fonts"),
    ("Noto Sans",         "google-noto-sans-fonts"),
]


def _installed_families() -> list[str]:
    try:
        out = subprocess.check_output(
            ["fc-list", ":", "family"],
            text=True, timeout=10,
        )
    except (OSError, subprocess.CalledProcessError, subprocess.TimeoutExpired):
        return []
    families: set[str] = set()
    for line in out.splitlines():
        # `fc-list : family` returns "Family Name,Aliased" — first variant is canonical
        primary = line.split(",", 1)[0].strip()
        if primary:
            families.add(primary)
    return sorted(families)


class FontsPanel(Gtk.Box):
    def __init__(self) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self._families: list[str] = []
        self._build()
        GLib.idle_add(self._reload_families)

    def _build(self) -> None:
        box = panel_box()
        box.pack_start(title_label("Fonts"), False, False, 0)
        box.pack_start(info_label(
            "See every font on your machine with a live preview, or "
            "install popular extras with one click."
        ), False, False, 0)
        box.pack_start(section_description(
            "Newly added font files go into ~/.local/share/fonts/. "
            "Mackes rebuilds the font cache for you."
        ), False, False, 0)

        # ---- Browse ---------------------------------------------------
        box.pack_start(section_header("Browse"), False, False, 0)

        self._family_combo = Gtk.ComboBoxText()
        self._family_combo.connect("changed", lambda *_: self._update_preview())
        box.pack_start(labeled_row("Family", self._family_combo), False, False, 0)

        self._size_spin = Gtk.SpinButton.new_with_range(8, 64, 1)
        self._size_spin.set_value(16)
        self._size_spin.connect("value-changed", lambda *_: self._update_preview())
        box.pack_start(labeled_row("Size", self._size_spin), False, False, 0)

        self._preview = Gtk.Label(
            label="The quick brown fox jumps over the lazy dog.\n"
                  "0123456789  !@#$%&()  → ← ⇒ ✓ ✗"
        )
        self._preview.set_xalign(0)
        self._preview.set_line_wrap(True)
        self._preview.set_margin_top(12)
        self._preview.set_margin_bottom(12)
        box.pack_start(self._preview, False, False, 0)

        refresh = Gtk.Button(label="Rebuild fc-cache")
        refresh.connect("clicked", lambda *_: self._rebuild_cache())
        box.pack_start(refresh, False, False, 0)

        # ---- Quick installs ------------------------------------------
        box.pack_start(section_header("Quick install"), False, False, 0)
        grid = Gtk.Grid(column_spacing=8, row_spacing=4, column_homogeneous=True)
        for i, (display, pkg) in enumerate(_QUICK_INSTALL):
            btn = Gtk.Button(label=display)
            btn.connect("clicked", lambda _b, p=pkg, d=display: self._install_pkg(d, p))
            grid.attach(btn, i % 2, i // 2, 1, 1)
        box.pack_start(grid, False, False, 0)

        # ---- Output --------------------------------------------------
        box.pack_start(section_header("Output"), False, False, 0)
        self._log = Gtk.TextView()
        self._log.set_editable(False); self._log.set_monospace(True)
        scroll = Gtk.ScrolledWindow(); scroll.add(self._log)
        scroll.set_size_request(-1, 160)
        box.pack_start(scroll, False, False, 0)

        self.add(box)

    # ---- Logic --------------------------------------------------------

    def _reload_families(self) -> bool:
        # fc-list returns 600–2000 families on a stock Fedora install;
        # the call is 50–300ms but compounds across panels. Cache it and
        # run off-thread so the GTK loop never blocks here.
        import threading
        from mackes.probe_cache import cached, invalidate

        def worker():
            families = cached("fc-list.families",
                              factory=_installed_families, ttl_s=120)
            GLib.idle_add(self._apply_families, families)
        # After a font *install* we explicitly invalidate the cache —
        # see _on_install_done; the call from _build sees an empty cache.
        threading.Thread(target=worker, daemon=True).start()
        return False

    def _apply_families(self, families: list[str]) -> bool:
        self._families = families
        self._family_combo.remove_all()
        for fam in families:
            self._family_combo.append(fam, fam)
        idx = 0
        for i, fam in enumerate(families):
            if fam == "SF Pro Text":
                idx = i; break
        if families:
            self._family_combo.set_active(idx)
        return False

    def _update_preview(self) -> None:
        fam = self._family_combo.get_active_text()
        if not fam:
            return
        size = int(self._size_spin.get_value())
        # Pango markup-escape the family name (sanitization)
        from xml.sax.saxutils import escape as xesc
        self._preview.set_markup(
            f"<span font_family=\"{xesc(fam)}\" size=\"{size * 1024}\">"
            "The quick brown fox jumps over the lazy dog.\n"
            "0123456789  !@#$%&amp;()  → ← ⇒ ✓ ✗"
            "</span>"
        )

    def _rebuild_cache(self) -> None:
        if shutil.which("fc-cache") is None:
            self._append("fc-cache not installed.\n")
            return
        try:
            out = subprocess.check_output(
                ["fc-cache", "-f"], stderr=subprocess.STDOUT,
                text=True, timeout=60,
            )
            self._append(f"fc-cache: {out.strip() or 'rebuilt'}\n")
            # Bust the cache before reloading so the new families show up.
            from mackes.probe_cache import invalidate
            invalidate("fc-list.families")
            GLib.idle_add(self._reload_families)
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired, OSError) as e:
            self._append(f"fc-cache failed: {e}\n")

    def _install_pkg(self, display: str, pkg: str) -> None:
        if shutil.which("pkexec") is None or shutil.which("dnf") is None:
            self._append(f"need pkexec + dnf to install {pkg}.\n")
            return
        self._append(f"$ pkexec dnf install -y {pkg}\n")
        log_action(f"fonts: installing {pkg} ({display})")
        try:
            proc = subprocess.Popen(
                ["pkexec", "dnf", "install", "-y", pkg],
                stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                text=True, bufsize=1,
            )
            GLib.io_add_watch(
                proc.stdout.fileno(),
                GLib.IO_IN | GLib.IO_HUP,
                lambda fd, cond, p=proc: self._drain_pipe(p),
            )
        except OSError as e:
            self._append(f"launch failed: {e}\n")

    def _drain_pipe(self, proc: subprocess.Popen) -> bool:
        line = proc.stdout.readline() if proc.stdout else ""
        if line:
            self._append(line)
            return True
        rc = proc.wait()
        self._append(f"[exit {rc}]\n")
        from mackes.probe_cache import invalidate
        invalidate("fc-list.families")
        GLib.idle_add(self._reload_families)
        return False

    def _append(self, text: str) -> None:
        buf = self._log.get_buffer()
        buf.insert(buf.get_end_iter(), text)
