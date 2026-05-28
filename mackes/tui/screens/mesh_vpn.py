"""TUI Mesh VPN screen — peer list + control-node state."""
from __future__ import annotations

from textual.containers import Container
from textual.widgets import Static


def build() -> Container:
    body = Container()
    body.compose_add_child(Static(
        "[b]Mesh VPN[/b]\n"
        "[#c6c6c6]Nebula overlay peers. Cap 8.[/#c6c6c6]\n"
    ))
    try:
        from mackes.mesh_vpn import MeshState, headscale_list_peers, MESH_CAP, tailscale_status
        state = MeshState.load()
        peers = headscale_list_peers()
        ts = tailscale_status()
        n = len(peers)

        if state.is_control:
            status_line = f"[green]●[/]  Control node  ·  {n}/{MESH_CAP} peers"
        elif state.mesh_id:
            status_line = (f"[green]●[/]  Connected  ·  {n}/{MESH_CAP} peers"
                           f"  ·  control: {state.control_peer_id or '?'}")
        else:
            status_line = "[yellow]●[/]  Not joined to any mesh"
        body.compose_add_child(Static(f"{status_line}\n"))
        body.compose_add_child(Static(
            f"[#8d8d8d]Mesh IP:[/] {ts.get('mesh_ip') or '—'}\n"
        ))

        body.compose_add_child(Static("[b]Peers[/b]"))
        lines: list[str] = []
        if not peers:
            lines.append("  [#8d8d8d](no peers visible)[/]")
        else:
            for p in peers:
                dot = "[green]●[/]" if p.online else "[#8d8d8d]○[/]"
                rtt = f"{p.rtt_ms}ms" if p.rtt_ms is not None else "—"
                lines.append(
                    f"  {dot}  {p.name:<20}  "
                    f"[#c6c6c6]{p.mesh_ip or '—':<16}[/]  "
                    f"[#c6c6c6]{p.route or '—':<6}[/]  "
                    f"[#c6c6c6]{rtt:<8}[/]"
                )
        body.compose_add_child(Static("\n".join(lines)))
    except Exception as e:  # noqa: BLE001
        body.compose_add_child(Static(f"[red]Mesh VPN read failed: {e}[/]"))
    return body
