"""Look & Feel → Appearance.

Reference panel for the architecture. Shows:
  - Q13 lock: one unified Appearance panel with internal sections
  - Q16 lock: panels bind directly to xfconf keys via xfconf_bridge
  - Q9  lock: immediate apply — every widget change writes through

xfconf channels used:
  xsettings  /Net/ThemeName        GTK theme name
  xsettings  /Net/IconThemeName    Icon theme
  xsettings  /Gtk/CursorThemeName  Cursor theme
  xsettings  /Gtk/CursorThemeSize  Cursor size
  xsettings  /Gtk/FontName         UI font (e.g. "Droid Sans 10")
  xsettings  /Gtk/MonospaceFontName  Monospace font

Wallpaper lives in the xfce4-desktop channel; per-monitor and per-workspace
properties make the key path dynamic (see _wallpaper_section).

Theme discovery scans /usr/share/themes (and ~/.themes) for entries with a
gtk-3.0 subdirectory, /usr/share/icons (and ~/.icons) for entries with an
index.theme file, and /usr/share/icons for entries with a cursors subdir.

11.9 reliability sweep: theme + monitor discovery + every xfconf read in
__init__ used to add up to ~600 ms on a typical machine. They now happen
off-main-thread via `mackes.workbench._async.async_probe`. The panel
renders a "Loading…" placeholder synchronously; sections fill in as the
probe lands.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402

from mackes.workbench._async import async_probe
from mackes.xfconf_bridge import get_bridge, XfconfError
from mackes.workbench._common import (
    a11y, error_label, info_label, labeled_row, section_header,
)


def _discover_gtk_themes() -> list[str]:
    seen: set[str] = set()
    for root in (Path("/usr/share/themes"), Path("/usr/local/share/themes"),
                 Path.home() / ".themes"):
        if not root.is_dir():
            continue
        for entry in root.iterdir():
            if (entry / "gtk-3.0").is_dir():
                seen.add(entry.name)
    return sorted(seen) or ["Adwaita"]


def _discover_icon_themes() -> list[str]:
    seen: set[str] = set()
    for root in (Path("/usr/share/icons"), Path("/usr/local/share/icons"),
                 Path.home() / ".icons", Path.home() / ".local/share/icons"):
        if not root.is_dir():
            continue
        for entry in root.iterdir():
            if (entry / "index.theme").exists():
                seen.add(entry.name)
    return sorted(seen) or ["Adwaita"]


def _discover_cursor_themes() -> list[str]:
    seen: set[str] = set()
    for root in (Path("/usr/share/icons"), Path("/usr/local/share/icons"),
                 Path.home() / ".icons", Path.home() / ".local/share/icons"):
        if not root.is_dir():
            continue
        for entry in root.iterdir():
            if (entry / "cursors").is_dir() or (entry / "cursor.theme").exists():
                seen.add(entry.name)
    return sorted(seen) or ["Adwaita"]


def _list_monitors() -> list[str]:
    """Connected monitor names. Prefers `mackes.displays.list_outputs()`
    (xfsettings displays channel — instant; no shell-out), falling back
    to xrandr only if the channel is unreadable."""
    from mackes.probe_cache import cached

    def _probe() -> list[str]:
        try:
            from mackes.displays import xrandr_outputs_for_conky
            outs = xrandr_outputs_for_conky()
            names = [o["name"] for o in outs]
            if names:
                return names
        except Exception:  # noqa: BLE001
            pass
        # Fallback: xrandr CLI (rarely installed on minimal Fedora).
        import subprocess
        try:
            out = subprocess.check_output(["xrandr", "--query"], text=True,
                                          stderr=subprocess.DEVNULL, timeout=4)
        except (FileNotFoundError, subprocess.CalledProcessError,
                subprocess.TimeoutExpired):
            return ["monitor0"]
        mons = []
        for line in out.splitlines():
            if " connected" in line:
                mons.append(line.split(" ", 1)[0])
        return mons or ["monitor0"]

    return cached("appearance.monitors", factory=_probe, ttl_s=60)


# ---- Carbon refresh helpers (v1.1.1) -------------------------------------


def _appearance_breadcrumb() -> Gtk.Widget:
    bc = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=4)
    bc.get_style_context().add_class("mackes-breadcrumb")
    for i, p in enumerate(("MDE", "Look & Feel", "Appearance")):
        lab = Gtk.Label(label=p); lab.set_xalign(0)
        bc.pack_start(lab, False, False, 0)
        if i != 2:
            sep = Gtk.Label(label="/"); sep.set_xalign(0)
            sep.get_style_context().add_class("mackes-dot")
            bc.pack_start(sep, False, False, 0)
    return bc


def _ap_section_title(box: Gtk.Box, text: str, *, meta: str = "") -> None:
    row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
    row.set_margin_top(20); row.set_margin_bottom(8)
    t = Gtk.Label(label=text); t.set_xalign(0)
    t.get_style_context().add_class("mackes-section-title")
    row.pack_start(t, True, True, 0)
    if meta:
        m = Gtk.Label(label=meta); m.set_xalign(1)
        m.get_style_context().add_class("mackes-section-meta")
        row.pack_end(m, False, False, 0)
    box.pack_start(row, False, False, 0)


def _design_lock_notification() -> Gtk.Widget:
    from mackes.carbon import Notification, NotificationKind
    return Notification(
        "Design System locks",
        body=("Q-CB1 Gray 100 palette · Q-CB3 Red Hat typography · Q-CB5 "
              "Material Symbols (per Q43, supersedes legacy Carbon). "
              "Per-preset accent replaces the legacy blue but everything "
              "else is fixed."),
        kind=NotificationKind.INFO,
        dismissible=False,
    )


def _draw_accent_swatch(_w, cr) -> bool:
    """Draw the active preset's accent as a solid 56x56 swatch."""
    alloc = _w.get_allocation()
    w, h = alloc.width, alloc.height
    # Pull accent from the active style context
    ctx = _w.get_style_context()
    ok, rgba = ctx.lookup_color("mackes_accent")
    if ok:
        cr.set_source_rgb(rgba.red, rgba.green, rgba.blue)
    else:
        cr.set_source_rgb(0xf1 / 255, 0x85 / 255, 0x3d / 255)
    cr.rectangle(0, 0, w, h)
    cr.fill()
    return False


@dataclass(frozen=True)
class _AppearanceState:
    """Snapshot of every slow probe (theme discovery + xfconf reads)
    gathered off the main thread. The on-result handler hands these
    cached values to the widget builders so they never block."""

    gtk_themes: list[str]
    icon_themes: list[str]
    cursor_themes: list[str]
    monitors: list[str]
    # xfconf scalar reads cached at probe time
    prefer_dark: bool
    aa_enabled: bool
    # wallpaper init values (key -> value) — keyed by the dynamic
    # /backdrop/screen0/<monitor>/workspace0/<prop> path.
    wallpaper_values: dict[str, object] = field(default_factory=dict)


def _gather_appearance_state(monitors_only: bool = False) -> _AppearanceState:
    """Off-main-thread probe. Pre-reads every value the panel needs so
    the GTK-thread builders never shell out.

    Theme + monitor discovery is the dominant cost (filesystem walk +
    optional `xrandr`). The xfconf scalars are quick individually but
    they add up — pre-reading lets the panel construct in ~50 ms.
    """
    gtk_themes = _discover_gtk_themes()
    icon_themes = _discover_icon_themes()
    cursor_themes = _discover_cursor_themes()
    monitors = _list_monitors() if monitors_only is False else _list_monitors()

    # Pre-read the booleans the dark / AA switches need at init.
    try:
        xf = get_bridge()
    except XfconfError:
        return _AppearanceState(
            gtk_themes=gtk_themes, icon_themes=icon_themes,
            cursor_themes=cursor_themes, monitors=monitors,
            prefer_dark=False, aa_enabled=True, wallpaper_values={},
        )

    prefer_dark = bool(xf.get("xsettings", "/Gtk/ApplicationPreferDarkTheme", False))
    aa_enabled = bool(xf.get("xsettings", "/Xft/Antialias", 1))

    # Pre-read wallpaper state for every monitor so the wallpaper section
    # doesn't shell out at build time. The combo's `changed` handler still
    # does an on-demand read for the *newly* selected monitor — that's a
    # post-construction event and won't block panel switching.
    wallpaper_values: dict[str, object] = {}
    for mon in monitors:
        for prop in ("last-image", "image-style"):
            key = f"/backdrop/screen0/{mon}/workspace0/{prop}"
            default = 5 if prop == "image-style" else ""
            wallpaper_values[key] = xf.get("xfce4-desktop", key, default)

    return _AppearanceState(
        gtk_themes=gtk_themes, icon_themes=icon_themes,
        cursor_themes=cursor_themes, monitors=monitors,
        prefer_dark=prefer_dark, aa_enabled=aa_enabled,
        wallpaper_values=wallpaper_values,
    )


class AppearancePanel(Gtk.Box):
    def __init__(self) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=0)

        try:
            self.xf = get_bridge()
        except XfconfError as e:
            self.pack_start(error_label(str(e)), False, False, 0)
            return

        self._build_skeleton()
        async_probe(_gather_appearance_state, self._apply_state)

    # ---- skeleton (sync; cheap) ------------------------------------------

    def _build_skeleton(self) -> None:
        """Render the page chrome immediately. Slots that need probe data
        are kept as `None` here and filled by `_apply_state`."""
        outer = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        outer.set_margin_top(12); outer.set_margin_bottom(12)
        outer.set_margin_start(16); outer.set_margin_end(16)

        outer.pack_start(_appearance_breadcrumb(), False, False, 0)
        page_title = Gtk.Label(label="Appearance")
        page_title.set_xalign(0)
        page_title.get_style_context().add_class("mackes-page-title")
        outer.pack_start(page_title, False, False, 0)

        page_sub = Gtk.Label(label=(
            "Change how your desktop looks: theme colors, icons, "
            "cursor, fonts, and wallpaper. Changes show up the moment "
            "you make them."
        ))
        page_sub.set_xalign(0); page_sub.set_line_wrap(True)
        page_sub.get_style_context().add_class("mackes-page-subtitle")
        outer.pack_start(page_sub, False, False, 0)
        page_desc = Gtk.Label(label=(
            "Watch the Live preview panel on the right to see your "
            "tweaks before you commit to them."
        ))
        page_desc.set_xalign(0); page_desc.set_line_wrap(True)
        page_desc.get_style_context().add_class("mackes-section-description")
        outer.pack_start(page_desc, False, False, 0)

        # Probe-pending placeholder. Removed by `_apply_state` when the
        # background gather completes.
        self._loading = info_label("Loading themes, fonts, and wallpaper settings…")
        outer.pack_start(self._loading, False, False, 0)

        # Two-column layout (settings | live preview) — same structure as
        # the synchronous version, but `_settings_col` is empty until the
        # probe lands.
        self._grid = Gtk.Grid(column_spacing=32, row_spacing=0)
        self._grid.set_column_homogeneous(False)
        self._grid.set_margin_top(16)

        self._settings_col = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        self._settings_col.set_hexpand(True)
        self._grid.attach(self._settings_col, 0, 0, 3, 1)

        right_col = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        right_col.set_hexpand(False)
        right_col.set_size_request(360, -1)

        _ap_section_title(right_col, "Live preview", meta="re-renders on changes")
        right_col.pack_start(self._live_preview_tile(), False, False, 0)

        _ap_section_title(right_col, "Active accent")
        right_col.pack_start(self._active_accent_tile(), False, False, 0)

        _ap_section_title(right_col, "Locked by design system")
        right_col.pack_start(_design_lock_notification(), False, False, 0)
        self._grid.attach(right_col, 3, 0, 1, 1)

        outer.pack_start(self._grid, True, True, 0)
        self._outer = outer

        scroller = Gtk.ScrolledWindow()
        scroller.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        scroller.add(outer)
        self.pack_start(scroller, True, True, 0)

    # ---- apply (main thread) ---------------------------------------------

    def _apply_state(self, state: _AppearanceState) -> None:
        """Builds the left-column setting sections now that the probe
        has the discovery lists + cached xfconf scalars in hand."""
        self._state = state

        # Remove the "loading…" placeholder.
        if self._loading is not None and self._loading.get_parent() is not None:
            self._outer.remove(self._loading)
            self._loading = None

        self._settings_col.pack_start(self._theme_section(),         False, False, 0)
        self._settings_col.pack_start(self._icons_section(),         False, False, 0)
        self._settings_col.pack_start(self._cursor_section(),        False, False, 0)
        self._settings_col.pack_start(self._fonts_section(),         False, False, 0)
        self._settings_col.pack_start(self._antialiasing_section(),  False, False, 0)
        self._settings_col.pack_start(self._wallpaper_section(),     False, False, 0)
        self._settings_col.show_all()

    # ---- Live preview tile ------------------------------------------------

    def _live_preview_tile(self) -> Gtk.Widget:
        # A miniature window frame with sample text + buttons. Updates
        # implicitly via the global xfconf cascade (xsettings → GTK).
        tile = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        tile.get_style_context().add_class("mackes-stat-tile")
        tile.set_margin_top(0)

        # Fake titlebar
        title = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        title.set_margin_bottom(4)
        path = Gtk.Label(label="~/Documents")
        path.set_xalign(0)
        path.get_style_context().add_class("mackes-section-meta")
        title.pack_start(path, True, True, 0)
        for c in ("muted", "muted", "accent"):
            dot = Gtk.Label(label="●")
            dot.get_style_context().add_class("mackes-dot")
            dot.get_style_context().add_class(c)
            title.pack_end(dot, False, False, 0)
        tile.pack_start(title, False, False, 0)
        tile.pack_start(Gtk.Separator(), False, False, 0)

        # Sample text
        sample = Gtk.Label(label="The quick brown fox")
        sample.set_xalign(0); sample.set_margin_top(8)
        sample.get_style_context().add_class("mackes-section-title")
        tile.pack_start(sample, False, False, 0)
        line2 = Gtk.Label(label="jumps over the lazy dog · 0123456789")
        line2.set_xalign(0)
        line2.get_style_context().add_class("mackes-page-subtitle")
        tile.pack_start(line2, False, False, 0)
        line3 = Gtk.Label(label="$ mackes preset apply mackes")
        line3.set_xalign(0); line3.set_margin_top(8)
        line3.get_style_context().add_class("mackes-code")
        tile.pack_start(line3, False, False, 0)

        # Sample buttons
        btn_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        btn_row.set_margin_top(12)
        for label, klass in (("Primary", "suggested-action"),
                             ("Tertiary", "cds-button-tertiary"),
                             ("Ghost", "cds-button-ghost")):
            b = Gtk.Button(label=label)
            b.get_style_context().add_class(klass)
            btn_row.pack_start(b, False, False, 0)
        tile.pack_start(btn_row, False, False, 0)

        return tile

    def _active_accent_tile(self) -> Gtk.Widget:
        tile = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=16)
        tile.get_style_context().add_class("mackes-stat-tile")
        # Accent swatch
        swatch = Gtk.DrawingArea()
        swatch.set_size_request(56, 56)
        swatch.connect("draw", _draw_accent_swatch)
        tile.pack_start(swatch, False, False, 0)
        # Right: preset label + accent hex
        right = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
        from mackes.state import MackesState
        try:
            state = MackesState.load()
            preset_name = (state.active_preset or "mackes").title()
        except Exception:  # noqa: BLE001
            preset_name = "Mackes"
        title = Gtk.Label(label=preset_name); title.set_xalign(0)
        title.get_style_context().add_class("mackes-section-title")
        right.pack_start(title, False, False, 0)
        meta = Gtk.Label(label="from active preset")
        meta.set_xalign(0)
        meta.get_style_context().add_class("mackes-section-meta")
        right.pack_start(meta, False, False, 0)
        tile.pack_start(right, True, True, 0)
        return tile

    # ---- Theme ------------------------------------------------------------

    def _theme_section(self) -> Gtk.Widget:
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        box.pack_start(section_header("Theme"), False, False, 0)

        themes = self._state.gtk_themes
        combo = Gtk.ComboBoxText()
        for t in themes:
            combo.append_text(t)
        self.xf.bind_combo(combo, "xsettings", "/Net/ThemeName", themes,
                           "Adwaita" if "Adwaita" in themes else themes[0])
        a11y(combo, name="Choose GTK widget theme",
             tooltip="Select the GTK theme used by every app on the desktop")
        box.pack_start(labeled_row("GTK theme", combo), False, False, 0)

        # Dark variant switch — use the value already cached in state so
        # the bind doesn't shell out a second time.
        dark = Gtk.Switch()
        dark.set_active(self._state.prefer_dark)
        def on_dark(s, _g):
            self.xf.set("xsettings", "/Gtk/ApplicationPreferDarkTheme", s.get_active())
        dark.connect("notify::active", on_dark)
        a11y(dark, name="Prefer the dark variant of the GTK theme",
             tooltip="Hint apps to use a dark colour scheme where supported")
        box.pack_start(labeled_row("Prefer dark variant", dark), False, False, 0)

        return box

    # ---- Icons ------------------------------------------------------------

    def _icons_section(self) -> Gtk.Widget:
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        box.pack_start(section_header("Icons"), False, False, 0)

        icons = self._state.icon_themes
        combo = Gtk.ComboBoxText()
        for t in icons:
            combo.append_text(t)
        self.xf.bind_combo(combo, "xsettings", "/Net/IconThemeName", icons,
                           "Adwaita" if "Adwaita" in icons else icons[0])
        a11y(combo, name="Choose icon theme",
             tooltip="Select the icon theme used throughout the desktop")
        box.pack_start(labeled_row("Icon theme", combo), False, False, 0)
        return box

    # ---- Cursor -----------------------------------------------------------

    def _cursor_section(self) -> Gtk.Widget:
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        box.pack_start(section_header("Cursor"), False, False, 0)

        cursors = self._state.cursor_themes
        combo = Gtk.ComboBoxText()
        for t in cursors:
            combo.append_text(t)
        self.xf.bind_combo(combo, "xsettings", "/Gtk/CursorThemeName", cursors,
                           "Adwaita" if "Adwaita" in cursors else cursors[0])
        a11y(combo, name="Choose mouse-cursor theme",
             tooltip="Select the cursor theme used by all applications")
        box.pack_start(labeled_row("Cursor theme", combo), False, False, 0)

        spin = Gtk.SpinButton.new_with_range(16, 96, 4)
        self.xf.bind_spin(spin, "xsettings", "/Gtk/CursorThemeSize", 24)
        a11y(spin, name="Cursor size in pixels",
             tooltip="Pixel size of the mouse-cursor sprite (16–96)")
        box.pack_start(labeled_row("Cursor size", spin), False, False, 0)
        return box

    # ---- Fonts ------------------------------------------------------------

    def _fonts_section(self) -> Gtk.Widget:
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        box.pack_start(section_header("Fonts"), False, False, 0)

        ui = Gtk.FontButton()
        self.xf.bind_font(ui, "xsettings", "/Gtk/FontName", "Droid Sans 10")
        a11y(ui, name="Choose interface font",
             tooltip="Pick the font used throughout the GTK interface")
        box.pack_start(labeled_row("Interface", ui), False, False, 0)

        mono = Gtk.FontButton()
        mono.set_filter_func(lambda family, _face: family.is_monospace())
        self.xf.bind_font(mono, "xsettings", "/Gtk/MonospaceFontName", "JetBrains Mono 10")
        a11y(mono, name="Choose monospace font (terminal / code)",
             tooltip="Pick the monospace font used by terminals and code views")
        box.pack_start(labeled_row("Monospace", mono), False, False, 0)

        return box

    # ---- Antialiasing / hinting ------------------------------------------

    def _antialiasing_section(self) -> Gtk.Widget:
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        box.pack_start(section_header("Font rendering"), False, False, 0)

        aa = Gtk.Switch()
        aa.set_active(self._state.aa_enabled)
        aa.connect("notify::active",
                   lambda s, _g: self.xf.set("xsettings", "/Xft/Antialias",
                                              1 if s.get_active() else 0))
        a11y(aa, name="Enable font antialiasing",
             tooltip="Smooth font edges (Xft/Antialias)")
        box.pack_start(labeled_row("Antialiasing", aa), False, False, 0)

        HINTING = ["none", "slight", "medium", "full"]
        hinting = Gtk.ComboBoxText()
        for h in HINTING:
            hinting.append_text(h)
        self.xf.bind_combo(hinting, "xsettings", "/Xft/HintStyle", HINTING, "slight")
        a11y(hinting, name="Font hint style",
             tooltip="How aggressively glyphs are snapped to pixels")
        box.pack_start(labeled_row("Hinting", hinting), False, False, 0)

        RGBA = ["none", "rgb", "bgr", "vrgb", "vbgr"]
        rgba = Gtk.ComboBoxText()
        for r in RGBA:
            rgba.append_text(r)
        self.xf.bind_combo(rgba, "xsettings", "/Xft/RGBA", RGBA, "rgb")
        a11y(rgba, name="Sub-pixel order for font rendering",
             tooltip="Match the colour-order of your monitor's sub-pixels")
        box.pack_start(labeled_row("Sub-pixel order", rgba), False, False, 0)

        return box

    # ---- Wallpaper --------------------------------------------------------

    def _wallpaper_section(self) -> Gtk.Widget:
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        box.pack_start(section_header("Wallpaper"), False, False, 0)

        monitors = self._state.monitors
        monitor_combo = Gtk.ComboBoxText()
        for m in monitors:
            monitor_combo.append_text(m)
        monitor_combo.set_active(0)
        a11y(monitor_combo, name="Monitor whose wallpaper is being edited",
             tooltip="Pick which monitor's wallpaper this section configures")
        box.pack_start(labeled_row("Monitor", monitor_combo), False, False, 0)

        chooser = Gtk.FileChooserButton(title="Wallpaper", action=Gtk.FileChooserAction.OPEN)
        filt = Gtk.FileFilter()
        filt.set_name("Images")
        for ext in ("png", "jpg", "jpeg", "webp", "svg"):
            filt.add_pattern(f"*.{ext}")
        chooser.add_filter(filt)
        a11y(chooser, name="Choose wallpaper image",
             tooltip="Pick an image file to use as the desktop background")

        STYLES = ["0 — None", "1 — Centered", "2 — Tiled",
                  "3 — Stretched", "4 — Scaled", "5 — Zoomed"]
        style_combo = Gtk.ComboBoxText()
        for s in STYLES:
            style_combo.append_text(s)
        a11y(style_combo, name="Wallpaper scaling style",
             tooltip="How the image fits the monitor (centered, tiled, stretched, scaled, zoomed)")

        def key_for(prop: str) -> str:
            idx = monitor_combo.get_active()
            mon = monitors[idx] if idx >= 0 else "monitor0"
            return f"/backdrop/screen0/{mon}/workspace0/{prop}"

        def refresh_from_state() -> None:
            """Pull cached values from `_state.wallpaper_values` if present,
            falling back to a live read if the user switches to a monitor
            not pre-cached. The fallback is rare and post-construction so
            it never blocks panel switching."""
            cur_key = key_for("last-image")
            if cur_key in self._state.wallpaper_values:
                current = self._state.wallpaper_values[cur_key]
            else:
                current = self.xf.get("xfce4-desktop", cur_key, "")
            if current and Path(str(current)).exists():
                chooser.set_filename(str(current))

            style_key = key_for("image-style")
            if style_key in self._state.wallpaper_values:
                style_raw = self._state.wallpaper_values[style_key]
            else:
                style_raw = self.xf.get("xfce4-desktop", style_key, 5)
            try:
                style = int(style_raw or 5)
            except (TypeError, ValueError):
                style = 5
            style_combo.set_active(min(max(style, 0), len(STYLES) - 1))

        def on_monitor_changed(_):
            refresh_from_state()
        monitor_combo.connect("changed", on_monitor_changed)
        refresh_from_state()

        def on_set(b):
            f = b.get_filename()
            if f:
                self.xf.set("xfce4-desktop", key_for("last-image"), f, type_hint="string")
        chooser.connect("file-set", on_set)
        box.pack_start(labeled_row("Image", chooser), False, False, 0)

        def on_style(c):
            i = c.get_active()
            if i >= 0:
                self.xf.set("xfce4-desktop", key_for("image-style"), int(i))
        style_combo.connect("changed", on_style)
        box.pack_start(labeled_row("Style", style_combo), False, False, 0)

        return box
