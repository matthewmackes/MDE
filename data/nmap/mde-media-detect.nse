local http = require "http"
local shortport = require "shortport"
local stdnse = require "stdnse"
local string = require "string"

description = [[
Detects mesh media servers behind generic "http" service banners.

Stock `nmap -sV` reports Airsonic, Navidrome and Jellyfin all as plain
"http" because they ride ordinary HTTP ports. EPIC-MESH-PROBE needs a
precise service-kind so the media-config sync (EPIC-SYNC-APP-CONFIG)
can point Sublime Music / Delfin at the right peers. This script probes
two distinctive endpoints:

  * Subsonic API `/rest/ping.view` -> any Subsonic-compatible server
    (Airsonic / Navidrome); the `Server:` header distinguishes
    Navidrome when present.
  * Jellyfin `/System/Info/Public` -> a JSON document whose
    `ProductName` is "Jellyfin".

Output is the detected media kind ("airsonic" / "navidrome" /
"jellyfin"), which the probe parser folds into the Service card's
service_kind.
]]

author = "Mackes Desktop Environment"
license = "Same as Nmap--See https://nmap.org/book/man-legal.html"
categories = {"discovery", "safe"}

-- The media ports the curated probe set scans, plus the generic HTTP
-- ports a media server might ride.
portrule = shortport.port_or_service(
  {4040, 4533, 8096, 8080, 80, 443},
  {"http", "http-alt", "https"}
)

-- Probe the Subsonic ping endpoint. Returns "navidrome" / "airsonic"
-- when the response looks Subsonic-compatible, else nil.
local function detect_subsonic(host, port)
  local resp = http.get(host, port, "/rest/ping.view?c=mde&v=1.16.1&f=json")
  if not resp or not resp.body then
    return nil
  end
  if string.find(resp.body, "subsonic%-response", 1, false)
      or string.find(resp.body, '"status"', 1, true) then
    local server = resp.header and resp.header["server"]
    if server and string.find(server:lower(), "navidrome", 1, true) then
      return "navidrome"
    end
    return "airsonic"
  end
  return nil
end

-- Probe the Jellyfin public-info endpoint. Returns "jellyfin" when the
-- ProductName names Jellyfin, else nil.
local function detect_jellyfin(host, port)
  local resp = http.get(host, port, "/System/Info/Public")
  if not resp or not resp.body then
    return nil
  end
  if string.find(resp.body, "Jellyfin", 1, true) then
    return "jellyfin"
  end
  return nil
end

action = function(host, port)
  local kind = detect_jellyfin(host, port) or detect_subsonic(host, port)
  if not kind then
    return nil
  end
  -- Tag the port's service so -sV output + the probe parser pick up
  -- the precise kind.
  port.version = port.version or {}
  port.version.name = kind
  port.version.product = "MDE mesh media (" .. kind .. ")"
  nmap.set_port_version(host, port, "hardmatched")
  return stdnse.format_output(true, { "mde-media: " .. kind })
end
