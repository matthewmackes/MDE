"""Workbench main window.

Implements the navigation from Q3 (two-level hybrid) — top tabs route to
object-level Stack/Sidebar pairs.

    Dashboard
    ├── Look & Feel
    ├── Devices
    ├── Network
    ├── System
    ├── Apps
    └── Maintain

The Shell tab — formerly Polybar/Plank/Rofi/Panel-Visibility — was retired
in the v1.0 XFCE-provisioner pivot.
"""
from __future__ import annotations

import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk  # noqa: E402

from mackes.state import MackesState
from mackes.workbench.dashboard import DashboardView


# ---------------------------------------------------------------------------
# Tab body assembly — each is a Gtk.StackSidebar + Gtk.Stack of panels (object
# level under task-level tabs, per Q3).
# ---------------------------------------------------------------------------


def _build_tab(panels: list[tuple[str, str, Gtk.Widget]]) -> Gtk.Widget:
    stack = Gtk.Stack()
    stack.set_transition_type(Gtk.StackTransitionType.CROSSFADE)
    stack.set_transition_duration(150)
    for pid, label, widget in panels:
        scroller = Gtk.ScrolledWindow()
        scroller.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        scroller.add(widget)
        stack.add_titled(scroller, pid, label)

    sidebar = Gtk.StackSidebar()
    sidebar.set_stack(stack)
    sidebar.set_size_request(180, -1)

    pane = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=0)
    pane.pack_start(sidebar, False, False, 0)
    pane.pack_start(Gtk.Separator(orientation=Gtk.Orientation.VERTICAL), False, False, 0)
    pane.pack_start(stack, True, True, 0)
    return pane


def _look_and_feel_tab() -> Gtk.Widget:
    from mackes.workbench.look_and_feel.appearance import AppearancePanel
    return _build_tab([
        ("appearance", "Appearance", AppearancePanel()),
    ])


def _devices_tab() -> Gtk.Widget:
    from mackes.workbench.devices.display import DisplayPanel
    from mackes.workbench.devices.keyboard import KeyboardPanel
    from mackes.workbench.devices.mouse import MousePanel
    from mackes.workbench.devices.sound import SoundPanel
    from mackes.workbench.devices.power import PowerPanel
    return _build_tab([
        ("display", "Display", DisplayPanel()),
        ("keyboard", "Keyboard", KeyboardPanel()),
        ("mouse", "Mouse & Touchpad", MousePanel()),
        ("sound", "Sound", SoundPanel()),
        ("power", "Power", PowerPanel()),
    ])


def _network_tab() -> Gtk.Widget:
    from mackes.workbench.network.wifi import WifiPanel
    from mackes.workbench.network.vpn import VpnPanel
    from mackes.workbench.network.qnm import QnmPanel
    from mackes.workbench.network.firewall import FirewallPanel
    from mackes.workbench.network.mesh_vpn import MeshVpnPanel
    from mackes.workbench.network.mesh_ssh import MeshSshPanel
    from mackes.workbench.network.mesh_services import MeshServicesPanel
    return _build_tab([
        ("wifi", "Wi-Fi & Ethernet", WifiPanel()),
        ("vpn", "VPN", VpnPanel()),
        ("qnm", "Quick Network Mesh", QnmPanel()),
        ("mesh_vpn", "Mesh VPN", MeshVpnPanel()),
        ("mesh_ssh", "Mesh SSH", MeshSshPanel()),
        ("mesh_services", "Mesh Services", MeshServicesPanel()),
        ("firewall", "Firewall", FirewallPanel()),
    ])


def _system_tab() -> Gtk.Widget:
    from mackes.workbench.system.window_manager import WindowManagerPanel
    from mackes.workbench.system.workspaces import WorkspacesPanel
    from mackes.workbench.system.session import SessionPanel
    from mackes.workbench.system.notifications import NotificationsPanel
    from mackes.workbench.system.default_apps import DefaultAppsPanel
    from mackes.workbench.system.removable import RemovablePanel
    from mackes.workbench.system.datetime import DateTimePanel
    return _build_tab([
        ("wm", "Window Manager", WindowManagerPanel()),
        ("workspaces", "Workspaces", WorkspacesPanel()),
        ("session", "Session & Startup", SessionPanel()),
        ("notifications", "Notifications", NotificationsPanel()),
        ("default_apps", "Default Apps", DefaultAppsPanel()),
        ("removable", "Removable Media", RemovablePanel()),
        ("datetime", "Date & Time", DateTimePanel()),
    ])


def _maintain_tab(state: MackesState) -> Gtk.Widget:
    from mackes.workbench.maintain.snapshots import SnapshotsPanel
    from mackes.workbench.maintain.drift import DriftPanel
    from mackes.workbench.maintain.fonts import FontsPanel
    from mackes.workbench.maintain.power import PowerPanel
    from mackes.workbench.maintain.resources import ResourcesPanel
    from mackes.workbench.maintain.health_check import HealthCheckPanel
    from mackes.workbench.maintain.dependencies import DependenciesPanel
    from mackes.workbench.maintain.logs import LogsPanel
    from mackes.workbench.maintain.repair import RepairPanel
    from mackes.workbench.maintain.reset_to_preset import ResetToPresetPanel
    from mackes.workbench.maintain.system_update import SystemUpdatePanel
    from mackes.workbench.maintain.uninstall import UninstallPanel
    return _build_tab([
        ("snapshots", "Snapshots", SnapshotsPanel(state)),
        ("drift", "Drift", DriftPanel(state)),
        ("update", "System Update", SystemUpdatePanel()),
        ("fonts", "Fonts", FontsPanel()),
        ("power", "Power", PowerPanel()),
        ("resources", "Resources", ResourcesPanel()),
        ("health", "Health Check", HealthCheckPanel()),
        ("deps", "Dependencies", DependenciesPanel()),
        ("logs", "Logs", LogsPanel()),
        ("repair", "Repair", RepairPanel(state)),
        ("reset", "Reset to Preset", ResetToPresetPanel(state)),
        ("uninstall", "Uninstall", UninstallPanel()),
    ])


def _apps_tab() -> Gtk.Widget:
    from mackes.workbench.apps.install import AppsInstallPanel
    from mackes.workbench.apps.remove import AppsRemovePanel
    from mackes.workbench.apps.installed import AppsInstalledPanel
    return _build_tab([
        ("apps_install", "Install", AppsInstallPanel()),
        ("apps_remove", "Remove", AppsRemovePanel()),
        ("apps_installed", "Installed", AppsInstalledPanel()),
    ])


def _help_tab() -> Gtk.Widget:
    # Help is a single full-width pane (no sub-panel sidebar via _build_tab).
    # The Help panel has its own internal topic sidebar.
    from mackes.workbench.help import HelpPanel
    return HelpPanel()


# ---------------------------------------------------------------------------
# Main window
# ---------------------------------------------------------------------------


class WorkbenchWindow(Gtk.ApplicationWindow):
    def __init__(self, application: Gtk.Application, state: MackesState) -> None:
        super().__init__(application=application)
        self.set_default_size(1180, 740)
        self.set_title("Mackes Shell")
        self.state = state

        header = Gtk.HeaderBar()
        header.set_show_close_button(True)
        header.set_title("Mackes Shell")
        if state.active_preset:
            header.set_subtitle(f"Preset: {state.active_preset}")
        self.set_titlebar(header)

        menu_button = Gtk.MenuButton()
        menu_button.set_image(Gtk.Image.new_from_icon_name("open-menu-symbolic", Gtk.IconSize.BUTTON))
        menu = Gtk.Menu()
        for label, callback in [
            ("Run First-Run Wizard…", self._on_run_wizard),
            ("Help / User Guide", self._on_help),
            ("Open Log", self._on_open_log),
            ("About Mackes Shell", self._on_about),
        ]:
            item = Gtk.MenuItem(label=label)
            item.connect("activate", callback)
            menu.append(item)
        menu.show_all()
        menu_button.set_popup(menu)
        header.pack_end(menu_button)

        # Standalone Help button on the header for one-click access.
        help_button = Gtk.Button()
        help_button.set_image(Gtk.Image.new_from_icon_name("help-browser-symbolic", Gtk.IconSize.BUTTON))
        help_button.set_tooltip_text("Help / User Guide")
        help_button.connect("clicked", self._on_help)
        header.pack_end(help_button)

        self._notebook = Gtk.Notebook()
        self._notebook.set_tab_pos(Gtk.PositionType.TOP)

        self._dashboard = DashboardView(state, navigate=self.go_to)
        self._notebook.append_page(self._dashboard, Gtk.Label(label="Dashboard"))

        for label, builder in [
            ("Look & Feel", _look_and_feel_tab),
            ("Devices", _devices_tab),
            ("Network", _network_tab),
            ("System", _system_tab),
            ("Apps", _apps_tab),
            ("Maintain", lambda: _maintain_tab(state)),
            ("Help", _help_tab),
        ]:
            self._notebook.append_page(builder(), Gtk.Label(label=label))

        self.add(self._notebook)

    # ---- Cross-panel navigation (used by Dashboard quick actions) --------

    # Tab indices: 0 Dashboard, 1 L&F, 2 Devices, 3 Network, 4 System, 5 Apps,
    # 6 Maintain, 7 Help
    _TAB_INDEX = {
        "dashboard": 0,
        "look_and_feel": 1, "appearance": (1, "appearance"),
        "devices": 2, "display": (2, "display"), "keyboard": (2, "keyboard"),
        "mouse": (2, "mouse"), "sound": (2, "sound"), "power": (2, "power"),
        "network": 3, "wifi": (3, "wifi"), "vpn": (3, "vpn"),
        "qnm": (3, "qnm"), "mesh_vpn": (3, "mesh_vpn"),
        "mesh_ssh": (3, "mesh_ssh"), "mesh_services": (3, "mesh_services"),
        "firewall": (3, "firewall"),
        "system": 4, "wm": (4, "wm"), "workspaces": (4, "workspaces"),
        "session": (4, "session"), "notifications": (4, "notifications"),
        "default_apps": (4, "default_apps"), "removable": (4, "removable"),
        "datetime": (4, "datetime"),
        "apps": 5, "apps_install": (5, "apps_install"),
        "apps_remove": (5, "apps_remove"), "apps_installed": (5, "apps_installed"),
        "maintain": 6, "snapshots": (6, "snapshots"), "health": (6, "health"),
        "deps": (6, "deps"), "logs": (6, "logs"),
        "repair": (6, "repair"), "reset": (6, "reset"),
        "uninstall": (6, "uninstall"),
        "help": 7,
    }

    def go_to(self, target: str) -> None:
        entry = self._TAB_INDEX.get(target)
        if entry is None:
            return
        if isinstance(entry, tuple):
            tab_idx, panel_id = entry
            self._notebook.set_current_page(tab_idx)
            page = self._notebook.get_nth_page(tab_idx)
            stack = self._find_stack(page)
            if stack is not None:
                stack.set_visible_child_name(panel_id)
        else:
            self._notebook.set_current_page(entry)

    @staticmethod
    def _find_stack(widget: Gtk.Widget) -> Gtk.Stack | None:
        if isinstance(widget, Gtk.Stack):
            return widget
        if isinstance(widget, Gtk.Container):
            for child in widget.get_children():
                found = WorkbenchWindow._find_stack(child)
                if found is not None:
                    return found
        return None

    # ---- Header menu callbacks ------------------------------------------

    def _on_run_wizard(self, *_):
        from mackes.wizard.window import WizardWindow
        w = WizardWindow(application=self.get_application(), state=self.state)
        w.show_all()

    def _on_help(self, *_):
        self.go_to("help")

    def _on_open_log(self, *_):
        import subprocess
        from mackes.state import LOG_DIR
        log = LOG_DIR / "mackes.log"
        if log.exists():
            subprocess.Popen(["xdg-open", str(log)])

    def _on_about(self, *_):
        from mackes import __version__
        from mackes.workbench.dashboard import _hero_logo_path
        d = Gtk.AboutDialog(transient_for=self, modal=True)
        d.set_program_name("Mackes Shell")
        d.set_version(__version__)
        d.set_comments("A single control panel for XFCE on Fedora.")
        d.set_license_type(Gtk.License.GPL_3_0)
        logo_path = _hero_logo_path()
        if logo_path is not None:
            try:
                from gi.repository import GdkPixbuf
                pixbuf = GdkPixbuf.Pixbuf.new_from_file_at_scale(
                    str(logo_path), width=200, height=-1, preserve_aspect_ratio=True,
                )
                d.set_logo(pixbuf)
            except Exception:  # noqa: BLE001
                pass
        d.run()
        d.destroy()
