local http = require "http"
local shortport = require "shortport"
local stdnse = require "stdnse"
local string = require "string"

description = [[
Fingerprints MDE platform services so a probed host can be recognised
as a Mackes Desktop Environment peer rather than an anonymous box.

EPIC-MESH-PROBE's host inventory wants to distinguish "this is another
MDE peer" from "this is a stranger on the LAN". The clearest signal is
the per-peer Mackes Bus broker, an ntfy-derived service the daemon
supervises on the Nebula overlay (default :8443). This script probes
the broker's health endpoint; a Mackes Bus signature marks the host as
an MDE peer and names the service "mde-bus".

Safe + read-only: a single GET against the broker health path.
]]

author = "Mackes Desktop Environment"
license = "Same as Nmap--See https://nmap.org/book/man-legal.html"
categories = {"discovery", "safe"}

-- The Mackes Bus broker port (per the mde-bus publish default
-- `http://<overlay-ip>:8443`), plus common HTTP fallbacks.
portrule = shortport.port_or_service(
  {8443, 8080, 80},
  {"http", "http-alt", "https"}
)

action = function(host, port)
  -- ntfy (the Bus broker base) answers /v1/health with a small JSON
  -- `{"healthy":true}`; the Mackes Bus build also serves /config with
  -- an `mde-bus` marker. Probe health first (cheapest).
  local resp = http.get(host, port, "/v1/health")
  if not resp or not resp.body then
    return nil
  end
  if not string.find(resp.body, "healthy", 1, true) then
    return nil
  end
  -- Looks like an ntfy/Bus broker; confirm the Mackes Bus build.
  local cfg = http.get(host, port, "/config.js")
  local is_mde = cfg and cfg.body and string.find(cfg.body, "mde-bus", 1, true)

  port.version = port.version or {}
  port.version.name = "mde-bus"
  port.version.product = is_mde and "Mackes Bus broker" or "ntfy (Bus-compatible)"
  nmap.set_port_version(host, port, "hardmatched")
  return stdnse.format_output(true, {
    "mde-service: " .. (is_mde and "mackes-bus-broker" or "ntfy-broker"),
  })
end
