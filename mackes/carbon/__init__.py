"""mackes.carbon — Carbon Design System widget library for GTK3.

Implements the locks Q-CB1..Q-CB10:
  - Gray 100 palette (in data/css/tokens.css)
  - Carbon UI Shell layout (header + side nav + content + status bar)
  - IBM Plex Sans / IBM Plex Mono typography
  - Per-preset accent (replaces Carbon blue #0f62fe)
  - Carbon Icons (vendored via /usr/share/icons/Carbon/)
  - Strict 8px spacing grid (--cds-spacing-01..13)
  - Widget set: Button (5-tier) · Tile · DataTable · Accordion · NumberInput
                · MultiSelect · Notification · Toast · Modal · Skeleton

Every widget is a `Gtk.<base>` subclass and exposes a `.add_class("cds-*")`
convention so the CSS in tokens.css can target it. The widgets work
identically inside the workbench tabs, the wizard, the headless wizard
helper, and the mesh-services / mesh-vpn / mesh-ssh panels.
"""
from __future__ import annotations

from mackes.carbon.button       import Button, ButtonKind
from mackes.carbon.tile         import Tile, ClickableTile
from mackes.carbon.data_table   import DataTable, Column
from mackes.carbon.accordion    import Accordion, AccordionItem
from mackes.carbon.number_input import NumberInput
from mackes.carbon.multi_select import MultiSelect
from mackes.carbon.notification import Notification, NotificationKind
from mackes.carbon.toast        import Toast, ToastHost
from mackes.carbon.modal        import Modal, ModalSize
from mackes.carbon.skeleton     import Skeleton, SkeletonLine
from mackes.carbon.ui_shell     import UIShell, SideNavItem

# spacing tokens (Carbon's 8px grid)
SPACING_01 = 2
SPACING_02 = 4
SPACING_03 = 8
SPACING_04 = 12
SPACING_05 = 16
SPACING_06 = 24
SPACING_07 = 32
SPACING_08 = 40
SPACING_09 = 48
SPACING_10 = 64
SPACING_11 = 80
SPACING_12 = 96
SPACING_13 = 160


__all__ = [
    "Button", "ButtonKind",
    "Tile", "ClickableTile",
    "DataTable", "Column",
    "Accordion", "AccordionItem",
    "NumberInput",
    "MultiSelect",
    "Notification", "NotificationKind",
    "Toast", "ToastHost",
    "Modal", "ModalSize",
    "Skeleton", "SkeletonLine",
    "UIShell", "SideNavItem",
    "SPACING_01", "SPACING_02", "SPACING_03", "SPACING_04", "SPACING_05",
    "SPACING_06", "SPACING_07", "SPACING_08", "SPACING_09", "SPACING_10",
    "SPACING_11", "SPACING_12", "SPACING_13",
]
