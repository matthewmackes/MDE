"""Carbon UI Shell — sidebar-grouped workbench window (v1.1.0).

Replaces the v1.0 top-tab Notebook layout with the design's Carbon UI Shell:

    +--------------------------------------------------------------+
    |  brand        | Workbench  Recovery  CLI   ...   user@host  | header (48)
    +---------------+----------------------------------------------+
    | WORKBENCH     |                                              |
    | • Dashboard   |                                              |
    | CONFIGURATION |                                              |
    | • Look & Feel |             content (Gtk.Stack)              |
    | • Devices     |                                              |
    | NETWORK       |                                              |
    | • Mesh VPN    |                                              |
    | • ...         |                                              |
    +---------------+----------------------------------------------+
    | mesh: 5/16   services: 12   sshd ✓  drift: 3   ...           | status (24)
    +--------------------------------------------------------------+

Existing panel widgets are reused as-is — this module owns the chrome and the
content router only.
"""
from __future__ import annotations

from typing import Callable, Dict, List, Optional, Tuple, Union

import gi
gi.require_version("Gtk", "3.0")
gi.require_version("Gdk", "3.0")
from gi.repository import Gdk, GLib, Gtk  # noqa: E402

from mackes.state import MackesState
from mackes import __version__


SIDENAV_WIDTH = 220


# ---------------------------------------------------------------------------
# Navigation model
# ---------------------------------------------------------------------------


class NavItem:
    __slots__ = ("key", "label", "icon", "badge", "builder")

    def __init__(self, key: str, label: str, icon: str,
                 builder: Callable[[], Gtk.Widget], *, badge: str = "") -> None:
        self.key = key
        self.label = label
        self.icon = icon  # Gtk icon-name (symbolic)
        self.badge = badge
        self.builder = builder  # lazily instantiates the panel widget


class NavGroup:
    __slots__ = ("title", "items")

    def __init__(self, title: str, items: List[NavItem]) -> None:
        self.title = title
        self.items = items


# ---------------------------------------------------------------------------
# Lazy panel builders — each closes over the application state.
# Panels are instantiated on first activation, then cached in the Gtk.Stack.
# ---------------------------------------------------------------------------


def _wrap_in_scroller(widget: Gtk.Widget) -> Gtk.Widget:
    scroller = Gtk.ScrolledWindow()
    scroller.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
    scroller.add(widget)
    return scroller


def _build_subnav_container(
    panels: List[Tuple[str, str, "Union[Gtk.Widget, Callable[[], Gtk.Widget]]"]],
) -> Gtk.Widget:
    """Inner sidebar+stack for sections that still have sub-panels.

    Each panel entry is (key, label, widget_or_factory). When the entry
    is a Gtk.Widget it's added immediately (legacy callers). When it's
    a callable, an empty Box placeholder goes in immediately and the
    real widget is packed into that Box on first visibility. This
    keeps a group's first-paint cost to ONE panel (whichever is
    shown initially) instead of N.

    Why-not the standard "Stack.add_titled(factory_result, …)" — that
    forces every panel to construct up-front. Many panels shell out at
    __init__ (xrandr, nmcli, fc-list, rpm -q, …) and the cumulative
    cost is hundreds of ms per group open, freezing the GTK main loop.
    """
    stack = Gtk.Stack()
    stack.set_transition_type(Gtk.StackTransitionType.CROSSFADE)
    stack.set_transition_duration(150)

    # Map: stack child name → (placeholder Box, factory callable)
    pending: dict[str, Tuple[Gtk.Box, Callable[[], Gtk.Widget]]] = {}
    for pid, label, item in panels:
        if callable(item) and not isinstance(item, Gtk.Widget):
            placeholder = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
            stack.add_titled(_wrap_in_scroller(placeholder), pid, label)
            pending[pid] = (placeholder, item)
        else:
            stack.add_titled(_wrap_in_scroller(item), pid, label)

    def _maybe_build(name):
        entry = pending.pop(name, None)
        if entry is None:
            return
        placeholder, factory = entry
        try:
            real = factory()
        except Exception:  # noqa: BLE001
            # Failed-to-build panel — leave the placeholder and let
            # other panels keep working rather than crash the nav.
            import traceback
            traceback.print_exc()
            return
        placeholder.pack_start(real, True, True, 0)
        placeholder.show_all()

    def _on_visible_child_changed(stk, _pspec):
        _maybe_build(stk.get_visible_child_name())

    stack.connect("notify::visible-child", _on_visible_child_changed)

    # Build the first panel up-front so the group opens to real content.
    _maybe_build(stack.get_visible_child_name())

    sidebar = Gtk.StackSidebar()
    sidebar.set_stack(stack)
    sidebar.set_size_request(200, -1)

    pane = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=0)
    pane.pack_start(sidebar, False, False, 0)
    pane.pack_start(Gtk.Separator(orientation=Gtk.Orientation.VERTICAL), False, False, 0)
    pane.pack_start(stack, True, True, 0)
    return pane


def _build_nav(state: MackesState, navigate: Callable[[str], None]) -> List[NavGroup]:
    """Define the sidebar nav model. Lambdas defer panel imports until activated."""

    def _dashboard():
        from mackes.workbench.dashboard import DashboardView
        return DashboardView(state, navigate=navigate)

    def _look_and_feel():
        # Lazy panel construction — only Appearance builds on group open.
        def _appearance():
            from mackes.workbench.look_and_feel.appearance import AppearancePanel
            return AppearancePanel()
        return _build_subnav_container([
            ("appearance", "Appearance", _appearance),
        ])

    def _devices():
        # Each lambda fires only when its sub-tab is clicked the first
        # time — first-paint cost drops from N panels × N shell-outs to
        # one panel.
        def _f_display():
            from mackes.workbench.devices.display import DisplayPanel
            return DisplayPanel()
        def _f_keyboard():
            from mackes.workbench.devices.keyboard import KeyboardPanel
            return KeyboardPanel()
        def _f_mouse():
            from mackes.workbench.devices.mouse import MousePanel
            return MousePanel()
        def _f_sound():
            from mackes.workbench.devices.sound import SoundPanel
            return SoundPanel()
        def _f_power():
            from mackes.workbench.devices.power import PowerPanel
            return PowerPanel()
        return _build_subnav_container([
            ("display", "Display", _f_display),
            ("keyboard", "Keyboard", _f_keyboard),
            ("mouse", "Mouse & Touchpad", _f_mouse),
            ("sound", "Sound", _f_sound),
            ("power", "Power", _f_power),
        ])

    def _system():
        def _f_displays():
            from mackes.workbench.system.displays import DisplaysPanel
            return DisplaysPanel()
        def _f_wm():
            from mackes.workbench.system.window_manager import WindowManagerPanel
            return WindowManagerPanel()
        def _f_ws():
            from mackes.workbench.system.workspaces import WorkspacesPanel
            return WorkspacesPanel()
        def _f_session():
            from mackes.workbench.system.session import SessionPanel
            return SessionPanel()
        def _f_notif():
            from mackes.workbench.system.notifications import NotificationsPanel
            return NotificationsPanel()
        def _f_defapps():
            from mackes.workbench.system.default_apps import DefaultAppsPanel
            return DefaultAppsPanel()
        def _f_remov():
            from mackes.workbench.system.removable import RemovablePanel
            return RemovablePanel()
        def _f_dt():
            from mackes.workbench.system.datetime import DateTimePanel
            return DateTimePanel()
        def _f_boot():
            from mackes.workbench.system.boot_login import BootLoginPanel
            return BootLoginPanel()
        return _build_subnav_container([
            ("displays", "Screens", _f_displays),
            ("boot_login", "Boot & Login", _f_boot),
            ("wm", "Window Manager", _f_wm),
            ("workspaces", "Workspaces", _f_ws),
            ("session", "Session & Startup", _f_session),
            ("notifications", "Notifications", _f_notif),
            ("default_apps", "Default Apps", _f_defapps),
            ("removable", "Removable Media", _f_remov),
            ("datetime", "Date & Time", _f_dt),
        ])

    def _wifi():
        from mackes.workbench.network.wifi import WifiPanel
        return _wrap_in_scroller(WifiPanel())

    def _vpn():
        from mackes.workbench.network.vpn import VpnPanel
        return _wrap_in_scroller(VpnPanel())

    def _qnm():
        from mackes.workbench.network.qnm import QnmPanel
        return _wrap_in_scroller(QnmPanel())

    def _mesh_join():
        from mackes.workbench.network.mesh_join import MeshJoinPanel
        return _wrap_in_scroller(MeshJoinPanel())

    def _mesh_health():
        from mackes.workbench.network.mesh_health import MeshHealthPanel
        return _wrap_in_scroller(MeshHealthPanel())

    # NF-5.5 (v2.5 Nebula fabric): MeshVpnPanel retired with
    # the underlying Tailscale/Headscale Python tree. Mesh
    # state lives in `mesh_control` (which got its Nebula
    # rewrite in NF-11.x) — operators reach it via the
    # primary Network nav rather than the legacy sub-page.

    def _mesh_ssh():
        from mackes.workbench.network.mesh_ssh import MeshSshPanel
        return _wrap_in_scroller(MeshSshPanel())

    def _mesh_services():
        from mackes.workbench.network.mesh_services import MeshServicesPanel
        return _wrap_in_scroller(MeshServicesPanel())

    def _firewall():
        from mackes.workbench.network.firewall import FirewallPanel
        return _wrap_in_scroller(FirewallPanel())

    def _remote_desktop():
        from mackes.workbench.network.remote_desktop import RemoteDesktopPanel
        return _wrap_in_scroller(RemoteDesktopPanel())

    def _network_advanced():
        # Power-user mesh + network controls live behind one Advanced entry
        # so the primary Network surface stays focused on the outcome the
        # user wants: get online. Sub-panels build lazily on first visit.
        def _f_vpn():
            from mackes.workbench.network.vpn import VpnPanel
            return VpnPanel()
        def _f_qnm():
            from mackes.workbench.network.qnm import QnmPanel
            return QnmPanel()
        def _f_health():
            from mackes.workbench.network.mesh_health import MeshHealthPanel
            return MeshHealthPanel()
        # NF-5.5 (v2.5): _f_meshvpn retired — the underlying
        # MeshVpnPanel + its Tailscale/Headscale dependencies
        # retire as part of the wholesale v1.x-Python sweep.
        def _f_meshssh():
            from mackes.workbench.network.mesh_ssh import MeshSshPanel
            return MeshSshPanel()
        def _f_services():
            from mackes.workbench.network.mesh_services import MeshServicesPanel
            return MeshServicesPanel()
        def _f_firewall():
            from mackes.workbench.network.firewall import FirewallPanel
            return FirewallPanel()
        return _build_subnav_container([
            ("mesh_health",      "Mesh Health",      _f_health),
            ("mesh_ssh",         "Mesh SSH",         _f_meshssh),
            ("mesh_services",    "Mesh Services",    _f_services),
            ("firewall",         "Firewall",         _f_firewall),
            ("vpn",              "VPN",              _f_vpn),
            ("qnm",              "QNM",              _f_qnm),
        ])

    def _apps():
        from mackes.workbench.apps.panel import AppsPanel
        return AppsPanel()

    def _app_sources():
        from mackes.workbench.apps.sources import SourcesPanel
        return _wrap_in_scroller(SourcesPanel())

    def _maintain():
        # Hub-plus-sub-panels stack. Sub-panels are constructed lazily —
        # only the hub builds when the group is opened; each sub-panel
        # builds the first time it's navigated to via _go(key). Avoids
        # 13 panel __init__s (each shelling out to dnf/fc-list/journalctl/
        # rpm-q/…) on every "Apps & Maintenance → Maintain" click.
        from mackes.workbench.maintain.hub import MaintainHub

        inner_stack = Gtk.Stack()
        inner_stack.set_transition_type(Gtk.StackTransitionType.CROSSFADE)
        inner_stack.set_transition_duration(120)

        # key → factory (returns the unwrapped sub-panel widget)
        sub_factories: dict[str, Callable[[], Gtk.Widget]] = {
            "snapshots": lambda: __import__(
                "mackes.workbench.maintain.snapshots",
                fromlist=["SnapshotsPanel"]).SnapshotsPanel(state),
            "drift": lambda: __import__(
                "mackes.workbench.maintain.drift",
                fromlist=["DriftPanel"]).DriftPanel(state),
            "update": lambda: __import__(
                "mackes.workbench.maintain.system_update",
                fromlist=["SystemUpdatePanel"]).SystemUpdatePanel(),
            "fonts": lambda: __import__(
                "mackes.workbench.maintain.fonts",
                fromlist=["FontsPanel"]).FontsPanel(),
            "power": lambda: __import__(
                "mackes.workbench.maintain.power",
                fromlist=["PowerPanel"]).PowerPanel(),
            "resources": lambda: __import__(
                "mackes.workbench.maintain.resources",
                fromlist=["ResourcesPanel"]).ResourcesPanel(),
            "health": lambda: __import__(
                "mackes.workbench.maintain.health_check",
                fromlist=["HealthCheckPanel"]).HealthCheckPanel(),
            "deps": lambda: __import__(
                "mackes.workbench.maintain.dependencies",
                fromlist=["DependenciesPanel"]).DependenciesPanel(),
            "logs": lambda: __import__(
                "mackes.workbench.maintain.logs",
                fromlist=["LogsPanel"]).LogsPanel(),
            "repair": lambda: __import__(
                "mackes.workbench.maintain.repair",
                fromlist=["RepairPanel"]).RepairPanel(state),
            "reset": lambda: __import__(
                "mackes.workbench.maintain.reset_to_preset",
                fromlist=["ResetToPresetPanel"]).ResetToPresetPanel(state),
            "uninstall": lambda: __import__(
                "mackes.workbench.maintain.uninstall",
                fromlist=["UninstallPanel"]).UninstallPanel(),
            "debloat": lambda: __import__(
                "mackes.workbench.maintain.debloat",
                fromlist=["DebloatPanel"]).DebloatPanel(),
        }
        # Display labels for the back-link breadcrumb
        sub_labels = {
            "snapshots": "Snapshots", "drift": "Drift",
            "update": "System update", "fonts": "Fonts",
            "power": "Power", "resources": "Resources",
            "health": "Health", "deps": "Dependencies",
            "logs": "Logs", "repair": "Repair",
            "reset": "Reset to Preset", "uninstall": "Uninstall",
            "debloat": "Debloat levels",
        }
        # Map key → placeholder Box (built on first _go to that key)
        placeholders: dict[str, Gtk.Box] = {}

        def _go(key: str) -> None:
            # Lazy-build on first nav to a sub-panel
            if key in sub_factories:
                placeholder = placeholders.get(key)
                if placeholder is None:
                    placeholder = Gtk.Box(orientation=Gtk.Orientation.VERTICAL,
                                          spacing=0)
                    inner_stack.add_named(
                        _wrap_with_back(placeholder, sub_labels[key]), key,
                    )
                    placeholders[key] = placeholder
                if not placeholder.get_children():
                    try:
                        real = sub_factories[key]()
                        placeholder.pack_start(real, True, True, 0)
                        placeholder.show_all()
                    except Exception:  # noqa: BLE001
                        import traceback
                        traceback.print_exc()
            inner_stack.set_visible_child_name(key)

        # Wrap each sub-panel with a back-link header
        def _wrap_with_back(panel: Gtk.Widget, label: str) -> Gtk.Widget:
            box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
            head = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
            head.set_margin_top(16); head.set_margin_start(40)
            head.set_margin_end(40); head.set_margin_bottom(0)
            back = Gtk.Button(label="‹ Back to Maintain")
            back.set_relief(Gtk.ReliefStyle.NONE)
            back.get_style_context().add_class("cds-button-ghost")
            back.connect("clicked", lambda *_: _go("__hub"))
            head.pack_start(back, False, False, 0)
            crumb = Gtk.Label(label=f"Maintain  /  {label}")
            crumb.set_xalign(0)
            crumb.get_style_context().add_class("mackes-breadcrumb")
            head.pack_start(crumb, True, True, 0)
            box.pack_start(head, False, False, 0)
            scroll = Gtk.ScrolledWindow()
            scroll.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
            scroll.add(panel)
            box.pack_start(scroll, True, True, 0)
            return box

        # Build the hub eagerly (it's the landing surface; cheap)
        inner_stack.add_named(MaintainHub(on_open=_go, state=state), "__hub")
        inner_stack.set_visible_child_name("__hub")
        return inner_stack

    def _snapshots():
        from mackes.workbench.maintain.snapshots import SnapshotsPanel
        return _wrap_in_scroller(SnapshotsPanel(state))

    def _fleet_inventory():
        from mackes.workbench.fleet.inventory import FleetInventoryPanel
        return _wrap_in_scroller(FleetInventoryPanel())

    def _fleet_playbooks():
        from mackes.workbench.fleet.playbooks import FleetPlaybooksPanel
        return _wrap_in_scroller(FleetPlaybooksPanel())

    def _fleet_runs():
        from mackes.workbench.fleet.run_history import FleetRunHistoryPanel
        return _wrap_in_scroller(FleetRunHistoryPanel())

    def _help():
        from mackes.workbench.help import HelpPanel
        return HelpPanel()

    return [
        NavGroup("Workbench", [
            NavItem("dashboard", "Dashboard", "view-grid-symbolic", _dashboard),
        ]),
        NavGroup("Configuration", [
            NavItem("look_and_feel", "Look & Feel", "applications-graphics-symbolic", _look_and_feel),
            NavItem("devices", "Devices", "preferences-desktop-peripherals-symbolic", _devices),
            NavItem("system", "System", "computer-symbolic", _system),
        ]),
        NavGroup("Network", [
            NavItem("wifi", "Wi-Fi & Ethernet", "network-wireless-symbolic", _wifi),
            NavItem("mesh_join", "Mesh", "go-jump-symbolic", _mesh_join),
            NavItem("mesh_remote", "Mesh Remote", "video-display-symbolic", _remote_desktop),
            NavItem("network_advanced", "Advanced", "preferences-system-symbolic", _network_advanced),
        ]),
        NavGroup("Fleet", [
            NavItem("fleet_inventory", "Inventory", "view-list-symbolic", _fleet_inventory),
            NavItem("fleet_playbooks", "Playbooks", "text-x-script-symbolic", _fleet_playbooks),
            NavItem("fleet_runs", "Run history", "document-open-recent-symbolic", _fleet_runs),
        ]),
        NavGroup("Apps & Maintenance", [
            NavItem("apps", "Apps", "applications-other-symbolic", _apps),
            NavItem("app_sources", "Sources & Repos", "applications-internet-symbolic", _app_sources),
            NavItem("maintain", "Maintain", "preferences-system-symbolic", _maintain),
            NavItem("snapshots", "Snapshots", "document-revert-symbolic", _snapshots),
        ]),
        NavGroup("Reference", [
            NavItem("help", "Help", "help-browser-symbolic", _help),
        ]),
    ]


# ---------------------------------------------------------------------------
# Main window
# ---------------------------------------------------------------------------


class WorkbenchWindow(Gtk.ApplicationWindow):
    """Carbon UI Shell window. Single visual root for the whole app."""

    def __init__(self, application: Gtk.Application, state: MackesState) -> None:
        super().__init__(application=application)
        # 1.0.8 hotfix — Pin a stable, predictable WM_CLASS so i3 can
        # match the workbench in a `for_window … floating enable` rule.
        # Without this, Gtk.Application derives WM_CLASS from
        # `shell.mackes.Mackes` and i3 tiles the workbench full-screen,
        # making `set_default_size` + `WindowPosition.CENTER` no-ops.
        # set_wmclass() is deprecated in GTK3 but still functional and
        # is the only reliable way to set both res_name + res_class.
        try:
            self.set_wmclass("mackes-shell", "Mackes-shell")
        except Exception:  # noqa: BLE001
            pass

        # v1.6.5 — Compact-by-default. Open at a laptop-friendly size
        # (1280x720) regardless of monitor size; let the user maximize
        # themselves if they want full-screen. Previously we forced
        # maximize on every launch which made Mackes feel like an OS
        # rather than an app.
        mon_w, mon_h = _primary_monitor_size()
        target_w = min(1280, mon_w - 40)
        target_h = min(720,  mon_h - 80)
        self.set_default_size(target_w, target_h)
        self.set_position(Gtk.WindowPosition.CENTER)
        from mackes.workbench._common import versioned_title
        # v2.0.0 Phase 0.11 — "Mackes Shell" → just the workbench
        # surface name; versioned_title prepends "MDE <version>".
        self.set_title(versioned_title("Workbench"))
        self.state = state

        # CSS-class root marker so accent files can scope rules to the app
        # window when needed.
        self.get_style_context().add_class("mackes-app-window")
        if state.active_preset:
            self.get_style_context().add_class(f"preset-{state.active_preset}")

        self._nav: List[NavGroup] = _build_nav(state, navigate=self.go_to)
        self._nav_buttons: Dict[str, Gtk.Button] = {}
        self._panel_widgets: Dict[str, Gtk.Widget] = {}
        self._tweaks = _load_tweaks()

        # ---- 3-zone layout (header / body / status) -----------------------
        root = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        root.pack_start(self._build_header(), False, False, 0)
        body = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=0)
        body.pack_start(self._build_sidenav(), False, False, 0)
        self._content = Gtk.Stack()
        self._content.set_transition_type(Gtk.StackTransitionType.CROSSFADE)
        self._content.set_transition_duration(120)
        body.pack_start(self._content, True, True, 0)
        root.pack_start(body, True, True, 0)
        self._status_bar = self._build_status_bar()
        root.pack_start(self._status_bar, False, False, 0)

        # Wrap in Gtk.Overlay so the toast host can float over the
        # whole shell. (Tweaks drawer used to live here too; gone v1.6.5.)
        self._overlay = Gtk.Overlay()
        self._overlay.add(root)
        try:
            from mackes.workbench.shell.toasts import install_host
            install_host(self._overlay)
        except Exception:  # noqa: BLE001
            pass
        self.add(self._overlay)

        # Apply initial tweaks (chrome / status bar visibility / density).
        self._apply_tweaks(self._tweaks)

        # ---- Activate initial panel ---------------------------------------
        initial = self._tweaks.get("active_panel") or "dashboard"
        self.go_to(initial)

        # ---- Live nav-badge refresh (every 30s) ---------------------------
        self._refresh_nav_badges()
        GLib.timeout_add_seconds(30, self._refresh_nav_badges_tick)

        # ---- Auto-lock the admin session on window close ------------------
        self.connect("destroy", self._on_destroy_lock)

    # ----- Build helpers ---------------------------------------------------

    def _build_header(self) -> Gtk.Widget:
        header = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=0)
        header.get_style_context().add_class("mackes-shell-header")
        header.set_size_request(-1, 48)

        # Brand block — same width as sidenav (256), right-bordered
        brand = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        brand.set_size_request(SIDENAV_WIDTH, -1)
        brand.set_margin_start(16); brand.set_margin_end(16)
        # logo dot
        logo = Gtk.Image.new_from_icon_name("preferences-desktop-symbolic", Gtk.IconSize.LARGE_TOOLBAR)
        logo.get_style_context().add_class("mackes-dot")
        logo.get_style_context().add_class("accent")
        brand.pack_start(logo, False, False, 0)
        # text label "MDE" + "Workbench" with two-tone weight
        # (Phase 0.11 — was "Mackes Shell" pre-rebrand).
        text = Gtk.Label()
        text.set_markup(
            '<span weight="600">MDE</span><span weight="400" alpha="80%"> Workbench</span>'
        )
        text.get_style_context().add_class("mackes-brand-text")
        brand.pack_start(text, False, False, 0)
        # Right border via separator
        header.pack_start(brand, False, False, 0)
        header.pack_start(_vsep(), False, False, 0)

        # Mode buttons (Workbench / Recovery / CLI) — only Workbench is
        # active today; the others are visual stubs that surface their
        # planned function on hover via tooltip.
        for label, tip, active in (
            ("Workbench", "The main control plane (active)", True),
            ("Recovery", "Snapshots & restore — see Snapshots", False),
            ("CLI", "Drop to a shell with mesh env preloaded", False),
        ):
            b = Gtk.Button(label=label)
            b.get_style_context().add_class("mackes-header-action")
            b.get_style_context().add_class("mackes-header-action-flush")
            if active:
                b.get_style_context().add_class("checked")
            b.set_tooltip_text(tip)
            b.set_relief(Gtk.ReliefStyle.NONE)
            _ax = b.get_accessible()
            if _ax is not None:
                _ax.set_name(f"Switch to {label} mode — {tip}")
            header.pack_start(b, False, False, 0)

        # Spacer
        spacer = Gtk.Box()
        header.pack_start(spacer, True, True, 0)

        # Right-aligned actions: preset chip, user
        preset_label = (self.state.active_preset or "mackes").title()
        chip = Gtk.Button(label=preset_label)
        chip.get_style_context().add_class("mackes-header-action")
        chip.set_tooltip_text("Active preset — click to switch via the Setup Wizard")
        chip.set_relief(Gtk.ReliefStyle.NONE)
        # v1.6.5 — chip used to open the Tweaks drawer which has been
        # removed. Repoint at the Setup Wizard so the chip is still
        # functional: it's the canonical preset-swap surface now.
        chip.connect("clicked", self._on_open_wizard)
        _ax_chip = chip.get_accessible()
        if _ax_chip is not None:
            _ax_chip.set_name(f"Active preset is {preset_label} — click to open the Setup Wizard")
        header.pack_end(chip, False, False, 0)

        import getpass, socket
        try:
            user = getpass.getuser()
        except Exception:  # noqa: BLE001
            user = "user"
        try:
            host = socket.gethostname()
        except Exception:  # noqa: BLE001
            host = "host"
        ident = Gtk.Button(label=f"{user}@{host}")
        ident.get_style_context().add_class("mackes-header-action")
        ident.set_relief(Gtk.ReliefStyle.NONE)
        ident.set_tooltip_text("Active user @ hostname")
        _ax_id = ident.get_accessible()
        if _ax_id is not None:
            _ax_id.set_name(f"Signed in as {user} on {host}")
        header.pack_end(ident, False, False, 0)

        # Help button (one-click access)
        help_btn = Gtk.Button()
        help_btn.set_image(Gtk.Image.new_from_icon_name("help-browser-symbolic", Gtk.IconSize.BUTTON))
        help_btn.get_style_context().add_class("mackes-header-action")
        help_btn.set_relief(Gtk.ReliefStyle.NONE)
        help_btn.set_tooltip_text("Help")
        help_btn.connect("clicked", lambda *_: self.go_to("help"))
        _ax_help = help_btn.get_accessible()
        if _ax_help is not None:
            _ax_help.set_name("Open Workbench help")
        header.pack_end(help_btn, False, False, 0)

        # v1.4.1 — always-visible Setup Wizard button (next to Help).
        # Users were missing the wizard entry point; the Tweaks drawer's
        # "Re-open Wizard" button is too hidden.
        wiz_btn = Gtk.Button(label="Setup")
        wiz_btn.set_image(
            Gtk.Image.new_from_icon_name("system-run-symbolic", Gtk.IconSize.BUTTON))
        wiz_btn.set_always_show_image(True)
        wiz_btn.get_style_context().add_class("mackes-header-action")
        wiz_btn.set_relief(Gtk.ReliefStyle.NONE)
        wiz_btn.set_tooltip_text("Open the Setup Wizard — re-run birthright")
        wiz_btn.connect("clicked", self._on_open_wizard)
        _ax_wiz = wiz_btn.get_accessible()
        if _ax_wiz is not None:
            _ax_wiz.set_name("Open the Setup Wizard to re-run birthright")
        header.pack_end(wiz_btn, False, False, 0)

        # ---- Admin session lock/unlock button (v1.4.0) -------------------
        from mackes.admin_session import AdminSession
        self._admin = AdminSession.instance()
        self._unlock_btn = Gtk.Button()
        self._unlock_btn.get_style_context().add_class("mackes-header-action")
        self._unlock_btn.set_relief(Gtk.ReliefStyle.NONE)
        self._unlock_btn.connect("clicked", self._on_unlock_clicked)
        _ax_unlock = self._unlock_btn.get_accessible()
        if _ax_unlock is not None:
            # Updated dynamically in _update_unlock_button.
            _ax_unlock.set_name("Toggle admin session — locked or unlocked")
        self._update_unlock_button()
        self._admin.add_listener(lambda _ok: self._update_unlock_button())
        header.pack_end(self._unlock_btn, False, False, 0)

        return header

    def _update_unlock_button(self) -> None:
        if self._admin.is_unlocked():
            icon = "changes-allow-symbolic"
            tip = ("Admin session unlocked — privileged actions won't "
                   "prompt for a password until you lock or close Mackes")
            label = "Locked ▾"
            label = "Unlocked"
        else:
            icon = "changes-prevent-symbolic"
            tip = ("Click to unlock the admin session — authorize once, "
                   "then everything runs without further prompts")
            label = "Unlock"
        img = Gtk.Image.new_from_icon_name(icon, Gtk.IconSize.BUTTON)
        # Replace the existing child
        for c in list(self._unlock_btn.get_children()):
            self._unlock_btn.remove(c)
        row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        row.set_margin_start(8); row.set_margin_end(8)
        row.pack_start(img, False, False, 0)
        row.pack_start(Gtk.Label(label=label), False, False, 0)
        self._unlock_btn.add(row)
        self._unlock_btn.show_all()
        self._unlock_btn.set_tooltip_text(tip)
        _ax = self._unlock_btn.get_accessible()
        if _ax is not None:
            # Match the current state in the accessible name so screen
            # readers don't say "Toggle admin session" forever.
            _ax.set_name(
                "Lock admin session" if self._admin.is_unlocked()
                else "Unlock admin session"
            )

    def _on_unlock_clicked(self, *_) -> None:
        if self._admin.is_unlocked():
            # Confirm-less lock — instant.
            self._admin.lock()
            return
        # Async unlock to avoid blocking the GTK main loop on the password
        # prompt.
        import threading
        def runner() -> None:
            self._admin.unlock()  # listener will refresh the button
        threading.Thread(target=runner, daemon=True).start()

    def _build_sidenav(self) -> Gtk.Widget:
        outer = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        outer.set_size_request(SIDENAV_WIDTH, -1)
        outer.get_style_context().add_class("mackes-side-nav")

        scroller = Gtk.ScrolledWindow()
        scroller.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        inner = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        scroller.add(inner)
        outer.pack_start(scroller, True, True, 0)

        for group in self._nav:
            # group title row
            title = Gtk.Label(label=group.title)
            title.set_xalign(0)
            title.get_style_context().add_class("mackes-side-nav-group-title")
            title.set_margin_top(8)
            inner.pack_start(title, False, False, 0)
            for item in group.items:
                btn = self._make_nav_button(item)
                self._nav_buttons[item.key] = btn
                inner.pack_start(btn, False, False, 0)
            # spacer between groups
            spacer = Gtk.Box()
            spacer.set_size_request(-1, 4)
            inner.pack_start(spacer, False, False, 0)

        return outer

    def _make_nav_button(self, item: NavItem) -> Gtk.Button:
        btn = Gtk.Button()
        btn.set_relief(Gtk.ReliefStyle.NONE)
        btn.get_style_context().add_class("mackes-side-nav-item")
        btn.set_size_request(-1, 40)

        row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        row.set_margin_start(16); row.set_margin_end(16)

        icon = Gtk.Image.new_from_icon_name(item.icon, Gtk.IconSize.MENU)
        row.pack_start(icon, False, False, 0)

        label = Gtk.Label(label=item.label)
        label.set_xalign(0)
        row.pack_start(label, True, True, 0)

        if item.badge:
            badge = Gtk.Label(label=item.badge)
            badge.get_style_context().add_class("mackes-sn-badge")
            row.pack_end(badge, False, False, 0)

        btn.add(row)
        btn.connect("clicked", lambda *_: self.go_to(item.key))
        # Side-nav buttons are the primary navigation surface — every
        # one needs a screen-reader-friendly name. We include the group
        # label implicitly via the item.label ("Wi-Fi & Ethernet" reads
        # better than just "wifi" so we use the visible label).
        btn.set_tooltip_text(f"Open the {item.label} panel")
        _ax = btn.get_accessible()
        if _ax is not None:
            badge_suffix = f" ({item.badge})" if item.badge else ""
            _ax.set_name(f"Open {item.label}{badge_suffix}")
        return btn

    def _build_status_bar(self) -> Gtk.Widget:
        bar = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=24)
        bar.get_style_context().add_class("mackes-status-bar")
        bar.set_size_request(-1, 24)
        bar.set_margin_start(16); bar.set_margin_end(16)

        # 4 live items + 2 fixed right-aligned items. Built once, refreshed
        # in place via _refresh_status_bar() every 30s.
        self._sb_mesh = _status_item("● mesh: …")
        self._sb_services = _status_item("● services: …")
        self._sb_sshd = _status_item("● sshd: …")
        self._sb_drift = _status_item("● drift: …")
        bar.pack_start(self._sb_mesh, False, False, 0)
        bar.pack_start(self._sb_services, False, False, 0)
        bar.pack_start(self._sb_sshd, False, False, 0)
        bar.pack_start(self._sb_drift, False, False, 0)

        right = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=24)
        right.pack_end(_status_item(f"v{__version__}"), False, False, 0)
        right.pack_end(_status_item(
            f"preset: {self.state.active_preset or '—'}"), False, False, 0)
        bar.pack_end(right, True, True, 0)

        # Kick off the 30 s refresh loop. 1.0.7: first refresh runs on
        # a background thread (matches the 30 s tick's pattern) — prior
        # synchronous call blocked __init__ for ~7 s waiting on
        # headscale / fleet / drift probes on top of the nav-badge
        # probes' 10 s.
        import threading
        threading.Thread(target=self._refresh_status_bar, daemon=True).start()
        GLib.timeout_add_seconds(30, self._refresh_status_bar_tick)
        return bar

    def _refresh_status_bar_tick(self) -> bool:
        # v1.5.1 — run the synchronous shell-outs on a background thread
        # so the GTK main loop doesn't freeze every 30s. Results land
        # back on the main thread via GLib.idle_add.
        import threading
        threading.Thread(target=self._refresh_status_bar,
                         daemon=True).start()
        return True   # keep firing

    def _refresh_status_bar(self) -> None:
        """Pull live values from service_health / Headscale / drift detector.

        v1.5.1 — runs on a background thread; UI writes are posted back
        via GLib.idle_add.
        """
        try:
            from mackes.state import service_health
            sh = service_health()
        except Exception:  # noqa: BLE001
            sh = {}
        try:
            from mackes.mesh_vpn import headscale_list_peers, MESH_CAP
            peers = headscale_list_peers()
            len(peers)
            mesh_online = sum(1 for p in peers if p.online)
            mesh_cap = MESH_CAP
        except Exception:  # noqa: BLE001
            mesh_online = mesh_cap = 0
        try:
            from mackes.mesh_services import load_registry
            services_n = len(load_registry())
        except Exception:  # noqa: BLE001
            services_n = 0
        try:
            from mackes.presets import active_preset_drift
            _preset, items = active_preset_drift()
            drift_n = len(items or [])
        except Exception:  # noqa: BLE001
            drift_n = 0

        # All UI writes posted back to the GTK main thread.
        sshd_state = sh.get("sshd", "missing")
        def _apply():
            _set_status_item(
                self._sb_mesh,
                f"● mesh: {mesh_online}/{mesh_cap or 16}",
                "ok" if mesh_online > 0 else "warn",
            )
            _set_status_item(
                self._sb_services, f"● services: {services_n}",
                "ok" if services_n else "warn",
            )
            _set_status_item(
                self._sb_sshd, "● sshd",
                {"ok": "ok", "warn": "warn", "fail": "fail",
                 "missing": "warn"}.get(sshd_state, "warn"),
            )
            _set_status_item(
                self._sb_drift, f"● drift: {drift_n}",
                "warn" if drift_n else "ok",
            )
            return False
        GLib.idle_add(_apply)

    # ----- Navigation ------------------------------------------------------

    def go_to(self, key: str) -> None:
        """Activate the panel with the given nav key, lazily building it."""
        # Find the nav item
        item = self._find_item(key)
        if item is None:
            # Backward-compat: some callers pass legacy panel keys like
            # "appearance" (which now lives under look_and_feel inner stack).
            # Map them to the parent group.
            legacy = _LEGACY_KEY_MAP.get(key)
            if legacy:
                self.go_to(legacy)
            return

        # Lazy-build the panel
        if item.key not in self._panel_widgets:
            try:
                widget = item.builder()
            except Exception as e:  # noqa: BLE001
                widget = _make_error_placeholder(item.label, str(e))
            self._panel_widgets[item.key] = widget
            self._content.add_named(widget, item.key)
            widget.show_all()
        self._content.set_visible_child_name(item.key)

        # Update nav button visual state
        for k, btn in self._nav_buttons.items():
            ctx = btn.get_style_context()
            if k == item.key:
                ctx.add_class("active")
            else:
                ctx.remove_class("active")

        # 1.1.0 — expose the active panel key for the --focus toggle
        # detector in app.py (suggestion #5: second click on the same
        # status-cluster slug closes the workbench).
        self._active_panel_key = item.key

        # Persist active panel — skip the disk write if unchanged.
        if self._tweaks.get("active_panel") != item.key:
            self._tweaks["active_panel"] = item.key
            _save_tweaks(self._tweaks)

    def _find_item(self, key: str) -> Optional[NavItem]:
        for g in self._nav:
            for it in g.items:
                if it.key == key:
                    return it
        return None

    # ----- Tweaks integration ---------------------------------------------

    def _on_tweaks_change(self, tweaks: dict) -> None:
        self._tweaks = tweaks
        _save_tweaks(tweaks)
        self._apply_tweaks(tweaks)

    def _apply_tweaks(self, tweaks: dict) -> None:
        # status bar visibility
        if tweaks.get("show_status_bar", True):
            self._status_bar.show_all()
        else:
            self._status_bar.hide()
        # density — propagates to carbon-layout.css via root style class
        ctx = self.get_style_context()
        for d in ("compact", "cozy", "comfortable"):
            ctx.remove_class(f"mackes-density-{d}")
        density = tweaks.get("density") or "cozy"
        ctx.add_class(f"mackes-density-{density}")
        # preset swap — reload preset CSS, restyle root class.
        # v2.2.0: Conky is gone; the notification drawer reads accent
        # from the live CSS the moment it's reopened, no bounce needed.
        new_preset = tweaks.get("preset")
        if new_preset and new_preset != self.state.active_preset:
            from mackes.app import _install_css
            if self.state.active_preset:
                ctx.remove_class(f"preset-{self.state.active_preset}")
            ctx.add_class(f"preset-{new_preset}")
            self.state.active_preset = new_preset
            self.state.save()
            _install_css(new_preset)

    # ---- live nav badges --------------------------------------------------

    def _refresh_nav_badges_tick(self) -> bool:
        # v1.5.1 — same threaded refactor as the status bar; the
        # underlying queries are slow shell-outs.
        import threading
        threading.Thread(target=self._refresh_nav_badges,
                         daemon=True).start()
        return True

    def _refresh_nav_badges(self) -> None:
        """Update peer/service/fleet/drift counts on the side-nav badges.

        Each probe shells out (headscale, fleet log scan, drift compute);
        run them concurrently so the worst-case tick is bounded by the
        slowest single probe rather than the sum.
        """
        from concurrent.futures import ThreadPoolExecutor

        def _probe_mesh_peers():
            from mackes.mesh_vpn import headscale_list_peers
            online = sum(1 for p in headscale_list_peers() if p.online)
            return ("mesh_vpn", str(online) if online else "")

        def _probe_services():
            from mackes.mesh_services import load_registry
            n = len(load_registry())
            return ("mesh_services", str(n) if n else "")

        def _probe_fleet_failures():
            import time
            from mackes.fleet import list_runs
            recent = list_runs(limit=200, since=time.time() - 86400)
            failures = sum(1 for r in recent if r.exit_code != 0)
            return ("fleet_runs", str(failures) if failures else "")

        def _probe_drift():
            from mackes.presets import active_preset_drift
            _preset, items = active_preset_drift()
            n = len(items or [])
            return ("maintain", str(n) if n else "")

        badges: dict[str, str] = {}
        with ThreadPoolExecutor(max_workers=4,
                                 thread_name_prefix="nav-badges") as ex:
            futures = [ex.submit(p) for p in (_probe_mesh_peers,
                                              _probe_services,
                                              _probe_fleet_failures,
                                              _probe_drift)]
            for fut in futures:
                try:
                    key, val = fut.result(timeout=10)
                    if val:
                        badges[key] = val
                except Exception:  # noqa: BLE001
                    continue

        def _apply():
            for key, badge_text in badges.items():
                btn = self._nav_buttons.get(key)
                if btn is None:
                    continue
                self._set_nav_button_badge(btn, badge_text)
            for key, btn in self._nav_buttons.items():
                if key not in badges:
                    self._set_nav_button_badge(btn, "")
            return False
        GLib.idle_add(_apply)

    @staticmethod
    def _set_nav_button_badge(btn: Gtk.Button, text: str) -> None:
        """Update the trailing badge label on a side-nav button.

        The badge is created once and cached on the button as
        `_mackes_badge` — subsequent calls just .set_text() it, so the
        30s nav-refresh tick doesn't tear down and rebuild a Label
        widget every time when the count hasn't actually changed.
        """
        row = btn.get_child()
        if not isinstance(row, Gtk.Box):
            return
        badge = getattr(btn, "_mackes_badge", None)
        if badge is None:
            badge = Gtk.Label()
            badge.get_style_context().add_class("mackes-sn-badge")
            row.pack_end(badge, False, False, 0)
            btn._mackes_badge = badge   # type: ignore[attr-defined]
        if badge.get_text() == text:
            return
        badge.set_text(text)
        badge.set_visible(bool(text))

    # ---- admin session auto-lock on close ---------------------------------

    def _on_destroy_lock(self, *_) -> None:
        try:
            from mackes.admin_session import AdminSession
            sess = AdminSession.instance()
            if sess.is_unlocked():
                sess.lock()
        except Exception:  # noqa: BLE001
            pass

    def _on_open_wizard(self, *_) -> None:
        """Header → Setup button. Force the wizard regardless of provisioned state."""
        try:
            from mackes.wizard.window import WizardWindow
            from mackes.state import MackesState
            state = MackesState.load()
            w = WizardWindow(application=self.get_application(), state=state)
            w.show_all()
        except Exception as e:  # noqa: BLE001
            try:
                from mackes.workbench.shell.toasts import toast
                toast(f"Could not open wizard: {e}", kind="error")
            except Exception:  # noqa: BLE001
                pass


# ---------------------------------------------------------------------------
# Tweaks persistence — light JSON in ~/.config/mackes-shell/tweaks.json
# ---------------------------------------------------------------------------


def _tweaks_path():
    from mackes.state import CONFIG_DIR
    return CONFIG_DIR / "tweaks.json"


def _load_tweaks() -> dict:
    import json
    p = _tweaks_path()
    defaults = {
        "preset": None,             # picked up from MackesState
        "density": "cozy",          # compact / cozy / comfortable (shell GUI)
        "show_status_bar": True,
        "show_xfce_frame": True,    # client-side; XFCE frame is OS-managed but we keep the flag
        "show_conky": True,         # v1.4.0: birthright Conky HUD on by default
        "conky_density": "standard",  # v1.6.2: compact / standard / full (HUD)
        "conky_monitor": None,        # v1.6.2: xrandr output name (None = primary)
        "active_panel": "dashboard",
    }
    if not p.exists():
        return defaults
    try:
        loaded = json.loads(p.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return defaults
    defaults.update({k: v for k, v in loaded.items() if k in defaults})
    return defaults


def _save_tweaks(tweaks: dict) -> None:
    import json
    p = _tweaks_path()
    try:
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(json.dumps(tweaks, indent=2, sort_keys=True), encoding="utf-8")
    except OSError:
        pass


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _vsep() -> Gtk.Widget:
    s = Gtk.Separator(orientation=Gtk.Orientation.VERTICAL)
    return s


def _primary_monitor_size() -> tuple[int, int]:
    """Detect the primary monitor's pixel size via GdkDisplay.

    Falls back to 1280×800 if GdkDisplay isn't available yet (very
    rare; typically only during early test imports).
    """
    try:
        display = Gdk.Display.get_default()
        if display is None:
            return (1280, 800)
        mon = display.get_primary_monitor() or display.get_monitor(0)
        geom = mon.get_geometry()
        return (max(1024, geom.width), max(700, geom.height))
    except Exception:  # noqa: BLE001
        return (1280, 800)


def _status_item(text: str, kind: Optional[str] = None) -> Gtk.Widget:
    """Build a single .mackes-status-bar-item Label."""
    lab = Gtk.Label(label=text)
    lab.set_xalign(0)
    lab.get_style_context().add_class("mackes-status-bar-item")
    if kind:
        lab.get_style_context().add_class(kind)
    return lab


def _set_status_item(widget: Gtk.Label, text: str, kind: Optional[str]) -> None:
    """Update an existing status item's text + class (called every 30s)."""
    widget.set_text(text)
    ctx = widget.get_style_context()
    for k in ("ok", "warn", "fail"):
        ctx.remove_class(k)
    if kind:
        ctx.add_class(kind)


def _make_error_placeholder(name: str, err: str) -> Gtk.Widget:
    box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
    box.set_margin_top(40); box.set_margin_start(40)
    box.set_margin_end(40); box.set_margin_bottom(40)
    title = Gtk.Label(label=f"{name} failed to load")
    title.set_xalign(0)
    title.get_style_context().add_class("mackes-page-title")
    body = Gtk.Label(label=err)
    body.set_xalign(0)
    body.set_line_wrap(True)
    body.get_style_context().add_class("mackes-page-subtitle")
    box.pack_start(title, False, False, 0)
    box.pack_start(body, False, False, 0)
    return box


# Legacy nav keys → new nav keys (so Dashboard quick-action links keep working).
_LEGACY_KEY_MAP = {
    "appearance": "look_and_feel",
    "display": "devices", "keyboard": "devices", "mouse": "devices",
    "sound": "devices", "power": "devices",
    "wm": "system", "workspaces": "system", "session": "system",
    "notifications": "system", "default_apps": "system", "removable": "system",
    "datetime": "system", "displays": "system",
    "boot_login": "system",
    "apps_install": "apps", "apps_remove": "apps", "apps_installed": "apps",
    "app_sources": "app_sources",
    "mesh_health": "mesh_health",
    "drift": "maintain", "update": "maintain", "fonts": "maintain",
    "resources": "maintain", "health": "maintain", "deps": "maintain",
    "logs": "maintain", "repair": "maintain", "reset": "maintain",
    "uninstall": "maintain",
}
