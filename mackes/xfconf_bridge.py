"""xfconf bridge — Mackes panels bind GTK widgets to xfconf properties.

Per Q16 (locked answer A): Mackes panels read/write xfconf keys directly.
`xfsettingsd` watches xfconf and applies changes live, so this is the cheapest
and most architecturally correct way to drive XFCE.

The bridge uses the `xfconf-query` CLI by default — it ships with the xfconf
package, which is already a hard dependency of XFCE. A future optimization is
to switch to `python-xfconf` (GObject Introspection bindings) where available,
which gives free DBus change notifications. The interface below is designed to
accommodate that swap without panel changes.
"""
from __future__ import annotations

import shutil
import subprocess
from typing import Any, Optional


class XfconfError(RuntimeError):
    pass


class XfconfBridge:
    """Thin facade over `xfconf-query`. Stateless; reads happen on demand."""

    def __init__(self) -> None:
        if shutil.which("xfconf-query") is None:
            raise XfconfError(
                "xfconf-query is not installed. Install the 'xfconf' package: "
                "`sudo dnf install xfconf`."
            )

    # ----- Raw access ------------------------------------------------------

    def get(self, channel: str, prop: str, default: Any = None) -> Any:
        """Read a single property. Returns `default` if the key is absent."""
        try:
            out = subprocess.check_output(
                ["xfconf-query", "--channel", channel, "--property", prop],
                stderr=subprocess.PIPE,
                text=True,
            ).strip()
        except subprocess.CalledProcessError:
            return default
        # xfconf-query returns "true"/"false" for booleans and numbers as text
        if out == "true":
            return True
        if out == "false":
            return False
        try:
            if "." in out:
                return float(out)
            return int(out)
        except ValueError:
            return out

    def set(self, channel: str, prop: str, value: Any, type_hint: Optional[str] = None) -> None:
        """Write a single property. Type is inferred from `value` unless hinted."""
        if type_hint is None:
            if isinstance(value, bool):
                type_hint = "bool"
                value = "true" if value else "false"
            elif isinstance(value, int):
                type_hint = "int"
                value = str(int(value))
            elif isinstance(value, float):
                type_hint = "double"
                value = repr(float(value))
            else:
                type_hint = "string"
                value = str(value)
        else:
            value = str(value)
        cmd = [
            "xfconf-query", "--channel", channel, "--property", prop,
            "--create", "--type", type_hint, "--set", value,
        ]
        try:
            subprocess.check_call(cmd, stderr=subprocess.PIPE)
        except subprocess.CalledProcessError as e:
            raise XfconfError(f"xfconf-query failed: {' '.join(cmd)}") from e

    def reset(self, channel: str, prop: str) -> None:
        subprocess.call(
            ["xfconf-query", "--channel", channel, "--property", prop, "--reset"],
            stderr=subprocess.DEVNULL,
        )

    def dump_channel(self, channel: str) -> str:
        """Return the full dump of a channel, suitable for snapshot storage."""
        try:
            return subprocess.check_output(
                ["xfconf-query", "--channel", channel, "--list", "--verbose"],
                text=True,
            )
        except subprocess.CalledProcessError:
            return ""

    # ----- Two-way bind helpers (immediate apply — Q9 lock) ----------------

    def bind_combo(self, combo, channel: str, prop: str, values: list[str], default: str = "") -> None:
        """Bind a Gtk.ComboBoxText to an xfconf property.

        Selecting an entry writes immediately. The initial selection is read
        from xfconf once at bind time.
        """
        current = self.get(channel, prop, default)
        try:
            idx = values.index(current) if current in values else 0
        except ValueError:
            idx = 0
        combo.set_active(idx)

        def on_changed(c):
            txt = c.get_active_text()
            if txt is not None:
                self.set(channel, prop, txt)

        combo.connect("changed", on_changed)

    def bind_switch(self, switch, channel: str, prop: str, default: bool = False) -> None:
        """Bind a Gtk.Switch to a boolean xfconf property."""
        switch.set_active(bool(self.get(channel, prop, default)))

        def on_active(s, _gparam):
            self.set(channel, prop, s.get_active())

        switch.connect("notify::active", on_active)

    def bind_spin(self, spin, channel: str, prop: str, default: int = 0) -> None:
        """Bind a Gtk.SpinButton to an integer xfconf property."""
        spin.set_value(int(self.get(channel, prop, default)))

        def on_changed(s):
            self.set(channel, prop, int(s.get_value()))

        spin.connect("value-changed", on_changed)

    def bind_font(self, font_button, channel: str, prop: str, default: str = "") -> None:
        """Bind a Gtk.FontButton to a string font-name xfconf property."""
        current = self.get(channel, prop, default)
        if current:
            font_button.set_font_name(str(current))

        def on_font_set(b):
            self.set(channel, prop, b.get_font_name())

        font_button.connect("font-set", on_font_set)


# Module-level lazy singleton
_BRIDGE: Optional[XfconfBridge] = None


def get_bridge() -> XfconfBridge:
    global _BRIDGE
    if _BRIDGE is None:
        _BRIDGE = XfconfBridge()
    return _BRIDGE
