//! Phase E.17 follow-up — weather popover.
//!
//! Bottom-bar clock-zone popover showing a 4-line column:
//!
//! ```text
//!   ┌──────────────────────┐
//!   │ Toronto, ON          │   ← location
//!   │ 12° · Partly cloudy  │   ← temp + condition
//!   │ ↑18°  ↓7°  · 8 km/h  │   ← high/low + wind
//!   │ Updated 4 min ago    │   ← freshness
//!   ├──────────────────────┤
//!   │ wttr.in              │   ← attribution footer
//!   └──────────────────────┘
//! ```
//!
//! Backed by the public `wttr.in` JSON endpoint
//! (`https://wttr.in/<city>?format=j1`). Polled every 30 min,
//! cached to `$XDG_CACHE_HOME/mde/weather.json` so panel reboots
//! don't lose the last value.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Default poll cadence — 30 minutes.
pub const POLL_INTERVAL_SECS: u64 = 1800;

/// Cached snapshot displayed in the popover.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct WeatherSnapshot {
    pub location: String,
    pub condition: String,
    pub temp_c: i16,
    pub high_c: i16,
    pub low_c: i16,
    pub wind_kmh: u16,
    /// Unix epoch ms of the last successful fetch.
    pub fetched_at_ms: u64,
}

impl WeatherSnapshot {
    /// Rendered popover lines (excluding the attribution footer).
    #[must_use]
    pub fn render_lines(&self) -> Vec<String> {
        vec![
            self.location.clone(),
            format!("{}° · {}", self.temp_c, self.condition),
            format!(
                "↑{}°  ↓{}°  · {} km/h",
                self.high_c, self.low_c, self.wind_kmh
            ),
            self.freshness_label(),
        ]
    }

    /// Human-readable "Updated N min ago" line.
    #[must_use]
    pub fn freshness_label(&self) -> String {
        freshness_label(self.fetched_at_ms, current_epoch_ms())
    }

    /// Footer attribution (always wttr.in for now).
    #[must_use]
    pub const fn attribution() -> &'static str {
        "wttr.in"
    }
}

/// Pure helper — compute the freshness label given a fetch-time
/// + current-time pair.
#[must_use]
pub fn freshness_label(fetched_ms: u64, now_ms: u64) -> String {
    if fetched_ms == 0 {
        return "(never updated)".into();
    }
    let delta_secs = now_ms.saturating_sub(fetched_ms) / 1000;
    if delta_secs < 60 {
        "Updated just now".into()
    } else if delta_secs < 3600 {
        format!("Updated {} min ago", delta_secs / 60)
    } else if delta_secs < 86_400 {
        format!("Updated {} hr ago", delta_secs / 3600)
    } else {
        format!("Updated {} day(s) ago", delta_secs / 86_400)
    }
}

/// Default cache file path — `$XDG_CACHE_HOME/mde/weather.json`.
#[must_use]
pub fn default_cache_path() -> PathBuf {
    dirs::cache_dir()
        .map(|d| d.join("mde/weather.json"))
        .unwrap_or_else(|| PathBuf::from("/tmp/mde-weather.json"))
}

/// Load the cached snapshot, returning an empty default on any
/// read / deserialize error. Uses direct serde deserialization
/// because the cache is OUR format (`save_cached`), not wttr.in.
#[must_use]
pub fn load_cached(path: &Path) -> WeatherSnapshot {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<WeatherSnapshot>(&s).ok())
        .unwrap_or_default()
}

/// Pure JSON parser — wttr.in's `j1` format is verbose; we map
/// it onto our 4-field snapshot. Returns `Err` on parse failure.
pub fn parse(json: &str) -> Result<WeatherSnapshot, serde_json::Error> {
    let raw: WttrJ1 = serde_json::from_str(json)?;
    let current = raw.current_condition.into_iter().next().unwrap_or_default();
    let today = raw.weather.into_iter().next().unwrap_or_default();
    let area = raw.nearest_area.into_iter().next().unwrap_or_default();
    let area_name = area.area_name.into_iter().next().unwrap_or_default().value;
    let region_name = area.region.into_iter().next().unwrap_or_default().value;
    let location = if region_name.is_empty() {
        area_name
    } else {
        format!("{area_name}, {region_name}")
    };

    Ok(WeatherSnapshot {
        location,
        condition: current
            .weather_desc
            .into_iter()
            .next()
            .unwrap_or_default()
            .value,
        temp_c: current.temp_c.parse().unwrap_or(0),
        high_c: today.maxtemp_c.parse().unwrap_or(0),
        low_c: today.mintemp_c.parse().unwrap_or(0),
        wind_kmh: current.windspeed_kmph.parse().unwrap_or(0),
        fetched_at_ms: current_epoch_ms(),
    })
}

/// Persist the snapshot to the cache file.
pub fn save_cached(path: &Path, snapshot: &WeatherSnapshot) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string(snapshot).map_err(std::io::Error::other)?;
    std::fs::write(path, json)
}

#[must_use]
fn current_epoch_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default()
}

// ──────────────────────────────────────────────────────────────
// Wttr.in `j1` shape — minimal subset we consume
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
struct WttrJ1 {
    #[serde(default)]
    current_condition: Vec<CurrentCondition>,
    #[serde(default)]
    weather: Vec<WeatherDay>,
    #[serde(default)]
    nearest_area: Vec<NearestArea>,
}

#[derive(Debug, Deserialize, Default)]
struct CurrentCondition {
    #[serde(default, rename = "temp_C")]
    temp_c: String,
    #[serde(default)]
    windspeed_kmph: String,
    #[serde(default, rename = "weatherDesc")]
    weather_desc: Vec<NameValue>,
}

#[derive(Debug, Deserialize, Default)]
struct WeatherDay {
    #[serde(default, rename = "maxtempC")]
    maxtemp_c: String,
    #[serde(default, rename = "mintempC")]
    mintemp_c: String,
}

#[derive(Debug, Deserialize, Default)]
struct NearestArea {
    #[serde(default, rename = "areaName")]
    area_name: Vec<NameValue>,
    #[serde(default)]
    region: Vec<NameValue>,
}

#[derive(Debug, Deserialize, Default)]
struct NameValue {
    #[serde(default)]
    value: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn poll_interval_is_30_minutes() {
        assert_eq!(POLL_INTERVAL_SECS, 1800);
    }

    #[test]
    fn render_lines_has_four_lines() {
        let snap = WeatherSnapshot {
            location: "Toronto, ON".into(),
            condition: "Partly cloudy".into(),
            temp_c: 12,
            high_c: 18,
            low_c: 7,
            wind_kmh: 8,
            fetched_at_ms: current_epoch_ms(),
        };
        let lines = snap.render_lines();
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], "Toronto, ON");
        assert!(lines[1].contains("12°"));
        assert!(lines[1].contains("Partly cloudy"));
        assert!(lines[2].contains("↑18°"));
        assert!(lines[2].contains("↓7°"));
        assert!(lines[2].contains("8 km/h"));
        assert!(lines[3].starts_with("Updated"));
    }

    #[test]
    fn freshness_label_just_now_under_a_minute() {
        let now = 1_700_000_000_000;
        let fetched = now - 30_000; // 30 seconds ago
        assert_eq!(freshness_label(fetched, now), "Updated just now");
    }

    #[test]
    fn freshness_label_minutes() {
        let now = 1_700_000_000_000;
        let fetched = now - 5 * 60_000; // 5 minutes ago
        assert_eq!(freshness_label(fetched, now), "Updated 5 min ago");
    }

    #[test]
    fn freshness_label_hours() {
        let now = 1_700_000_000_000;
        let fetched = now - 2 * 3_600_000; // 2 hours ago
        assert_eq!(freshness_label(fetched, now), "Updated 2 hr ago");
    }

    #[test]
    fn freshness_label_days() {
        let now = 1_700_000_000_000;
        let fetched = now - 3 * 86_400_000; // 3 days ago
        assert_eq!(freshness_label(fetched, now), "Updated 3 day(s) ago");
    }

    #[test]
    fn freshness_label_never() {
        assert_eq!(freshness_label(0, 1_700_000_000_000), "(never updated)");
    }

    #[test]
    fn attribution_is_wttr_in() {
        assert_eq!(WeatherSnapshot::attribution(), "wttr.in");
    }

    #[test]
    fn parse_extracts_fields_from_wttr_json() {
        let json = r#"{
            "current_condition": [{
                "temp_C": "12",
                "windspeed_kmph": "8",
                "weatherDesc": [{"value": "Partly cloudy"}]
            }],
            "weather": [{
                "maxtempC": "18",
                "mintempC": "7"
            }],
            "nearest_area": [{
                "areaName": [{"value": "Toronto"}],
                "region": [{"value": "Ontario"}]
            }]
        }"#;
        let snap = parse(json).unwrap();
        assert_eq!(snap.location, "Toronto, Ontario");
        assert_eq!(snap.condition, "Partly cloudy");
        assert_eq!(snap.temp_c, 12);
        assert_eq!(snap.high_c, 18);
        assert_eq!(snap.low_c, 7);
        assert_eq!(snap.wind_kmh, 8);
    }

    #[test]
    fn parse_handles_missing_region() {
        let json = r#"{
            "current_condition": [{"temp_C": "5", "windspeed_kmph": "10", "weatherDesc": [{"value": "Sunny"}]}],
            "weather": [{"maxtempC": "8", "mintempC": "1"}],
            "nearest_area": [{"areaName": [{"value": "Reykjavik"}]}]
        }"#;
        let snap = parse(json).unwrap();
        assert_eq!(snap.location, "Reykjavik");
    }

    #[test]
    fn parse_returns_default_on_garbage() {
        assert!(parse("not json").is_err());
    }

    #[test]
    fn load_cached_returns_default_when_missing() {
        let tmp = tempdir().unwrap();
        let snap = load_cached(&tmp.path().join("absent.json"));
        assert_eq!(snap, WeatherSnapshot::default());
    }

    #[test]
    fn save_then_load_round_trips() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("weather.json");
        let snap = WeatherSnapshot {
            location: "Berlin".into(),
            condition: "Clear".into(),
            temp_c: 5,
            high_c: 9,
            low_c: 1,
            wind_kmh: 12,
            fetched_at_ms: 1_700_000_000_000,
        };
        save_cached(&path, &snap).unwrap();
        let loaded = load_cached(&path);
        assert_eq!(loaded, snap);
    }

    #[test]
    fn default_cache_path_ends_with_weather_json() {
        let p = default_cache_path();
        assert!(p.ends_with("weather.json"));
    }
}
