"""Mackes Shell — slide-out popover (v1.6.2 redesign).

10-question survey lock 2026-05-17:
  Q1 model:    XFCE panel plugin → popover (Gtk.Window POPUP)
  Q2 nav:      Tabbed top bar
  Q3 tabs:     Glance · Mesh · Tools · Manage · Help
  Q4 size:     420×600 fixed, close on focus-out
  Q5 lists:    GtkTreeView (sortable + filterable)
  Q6 type:     Carbon Productive Type Scale
  Q7 color:    Adaptive (follows XFCE dark/light)
  Q8 trigger:  Panel button + tray + Super+M
  Q9 full:     Wizard, Logs, Snapshots, Mesh topology escape
  Q10 cut:     Merge close-cousin panels
  + Nerd Font glyphs everywhere they fit

Entry point: `mackes --popover` (handled in mackes.app:main).
"""
from __future__ import annotations
