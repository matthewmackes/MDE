//! BUS-6.5 — cross-topic correlation engine.
//!
//! Synthesizes new topics when multiple source topics fire within a
//! window. Example rule:
//!
//! ```yaml
//! rules:
//!   - name: likely-power-outage
//!     sources: [power/ups/grid-loss, network/wan-down]
//!     window_seconds: 60
//!     emits: incident/likely-power-outage
//!     priority: high
//! ```
//!
//! When BOTH `power/ups/grid-loss` AND `network/wan-down` publish
//! within 60 s of each other, the engine fires a synthesized
//! publish on `incident/likely-power-outage` at high priority.
//!
//! Operator config lives at `~/.config/mde/bus-correlate.yaml`
//! (per the BUS-6.5 design lock — distinct from bus_root which is
//! GFS-mesh-synced state).
//!
//! ## Ships in BUS-6.5.parser
//!
//! - [`CorrelateRule`] schema (deny-unknown-fields YAML)
//! - [`CorrelateConfig`] top-level container
//! - [`SlidingWindow`] per-topic recent-observation tracker
//! - [`evaluate_rule`] pure-fn — given a rule + window + now,
//!   returns `Some(emission)` when every source has fired inside
//!   the window, `None` otherwise.
//! - [`load_default`] reads `~/.config/mde/bus-correlate.yaml`.
//!
//! ## Future
//!
//! BUS-6.5.evaluator wires this into the publish flow: every
//! publish updates the per-topic SlidingWindow, then evaluates
//! every rule whose source-set contains the published topic;
//! firing rules synthesize a fresh `mde-bus publish <emits>`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;

use crate::hooks::config::Priority;

/// Top-level `bus-correlate.yaml` shape.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrelateConfig {
    /// Rules evaluated in declaration order. Each rule is
    /// independent — multiple rules can fire on a single publish
    /// if their predicates overlap.
    #[serde(default)]
    pub rules: Vec<CorrelateRule>,
}

/// One correlation rule.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrelateRule {
    /// Human-readable name for logs + audit.
    pub name: String,
    /// Source topics — ALL must have fired within `window_seconds`
    /// for the rule to emit.
    pub sources: Vec<String>,
    /// Window length, in seconds. A source's last-seen timestamp
    /// older than `now - window_seconds` is treated as not-fired.
    pub window_seconds: u32,
    /// Topic the synthesized publish lands on.
    pub emits: String,
    /// Priority of the synthesized publish.
    #[serde(default)]
    pub priority: Priority,
}

/// Per-topic last-observed timestamp tracker. Operator-process-
/// lifetime in-memory state; doesn't survive a daemon restart
/// (intentional — synthesized incidents over a fleet restart
/// would be noise).
#[derive(Debug, Default, Clone)]
pub struct SlidingWindow {
    /// Topic → wall-clock timestamp (milliseconds since Unix
    /// epoch) of the most recent observation.
    last_seen: BTreeMap<String, i64>,
}

impl SlidingWindow {
    /// Record that `topic` was just observed at `now_unix_ms`.
    pub fn observe(&mut self, topic: &str, now_unix_ms: i64) {
        self.last_seen.insert(topic.to_string(), now_unix_ms);
    }

    /// Return the last-observed timestamp for `topic`, or `None`
    /// when the topic has never been observed (or has aged out).
    /// This helper doesn't age-out by itself — that's the
    /// `evaluate_rule` caller's job to compare against
    /// `window_seconds`.
    #[must_use]
    pub fn last_seen(&self, topic: &str) -> Option<i64> {
        self.last_seen.get(topic).copied()
    }

    /// Count of distinct topics tracked. Used in tests.
    #[must_use]
    pub fn len(&self) -> usize {
        self.last_seen.len()
    }

    /// True when no topic has been observed yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.last_seen.is_empty()
    }
}

/// One synthesized emission from a fired rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynthesizedEmission {
    /// The rule that fired — surfaced in audit.
    pub rule_name: String,
    /// Synthesized topic to publish on.
    pub topic: String,
    /// Priority of the synthesized publish.
    pub priority: Priority,
}

/// Pure-fn — evaluate one rule against the live sliding window.
/// Returns `Some(emission)` when every `source` topic has fired
/// at or after `now_unix_ms - window_seconds * 1000`. Empty
/// `sources` returns `None` (an always-firing rule is a config
/// error). Missing observations for any source return `None`.
#[must_use]
pub fn evaluate_rule(
    rule: &CorrelateRule,
    window: &SlidingWindow,
    now_unix_ms: i64,
) -> Option<SynthesizedEmission> {
    if rule.sources.is_empty() {
        return None;
    }
    let cutoff_ms = now_unix_ms - i64::from(rule.window_seconds) * 1000;
    for src in &rule.sources {
        match window.last_seen(src) {
            Some(ts) if ts >= cutoff_ms => continue,
            _ => return None,
        }
    }
    Some(SynthesizedEmission {
        rule_name: rule.name.clone(),
        topic: rule.emits.clone(),
        priority: rule.priority,
    })
}

/// Default operator-config path:
/// `$XDG_CONFIG_HOME/mde/bus-correlate.yaml` (falls back to
/// `$HOME/.config/mde/bus-correlate.yaml`).
#[must_use]
pub fn default_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("mde").join("bus-correlate.yaml"))
}

/// Load the correlation config from `path`. Returns:
/// - `Ok(config)` on successful parse.
/// - `Ok(default)` when the file is missing (operators may not
///   have configured correlation rules; that's not an error).
/// - `Err(CorrelateLoadError)` on read or parse failure.
///
/// # Errors
/// Returns [`CorrelateLoadError::Read`] when the file exists but
/// cannot be read; [`CorrelateLoadError::Parse`] when the YAML is
/// malformed.
pub fn load_default(path: &std::path::Path) -> Result<CorrelateConfig, CorrelateLoadError> {
    if !path.exists() {
        return Ok(CorrelateConfig::default());
    }
    let body = std::fs::read_to_string(path)
        .map_err(|e| CorrelateLoadError::Read(format!("{}: {e}", path.display())))?;
    serde_yaml::from_str(&body).map_err(|e| CorrelateLoadError::Parse(e.to_string()))
}

/// Errors loading the correlation config.
#[derive(Debug)]
pub enum CorrelateLoadError {
    /// Filesystem read failed (permission, encoding, etc.).
    Read(String),
    /// YAML parse failed.
    Parse(String),
}

impl std::fmt::Display for CorrelateLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(e) => write!(f, "correlate config read: {e}"),
            Self::Parse(e) => write!(f, "correlate config parse: {e}"),
        }
    }
}

impl std::error::Error for CorrelateLoadError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config_parses() {
        let cfg: CorrelateConfig = serde_yaml::from_str("rules: []").unwrap();
        assert_eq!(cfg.rules.len(), 0);
    }

    #[test]
    fn sample_rule_round_trips() {
        let yaml = r#"
rules:
  - name: likely-power-outage
    sources:
      - power/ups/grid-loss
      - network/wan-down
    window_seconds: 60
    emits: incident/likely-power-outage
    priority: high
"#;
        let cfg: CorrelateConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.rules.len(), 1);
        let rule = &cfg.rules[0];
        assert_eq!(rule.name, "likely-power-outage");
        assert_eq!(rule.sources.len(), 2);
        assert_eq!(rule.window_seconds, 60);
        assert_eq!(rule.emits, "incident/likely-power-outage");
    }

    #[test]
    fn rejects_unknown_fields() {
        let yaml = r#"
rules:
  - name: r
    sources: [a]
    window_seconds: 60
    emits: x
    unexpected_field: oops
"#;
        let err = serde_yaml::from_str::<CorrelateConfig>(yaml).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("unknown field") || msg.contains("unexpected_field"));
    }

    #[test]
    fn sliding_window_observe_and_lookup() {
        let mut w = SlidingWindow::default();
        assert!(w.is_empty());
        w.observe("a", 100);
        w.observe("b", 200);
        assert_eq!(w.last_seen("a"), Some(100));
        assert_eq!(w.last_seen("b"), Some(200));
        assert!(w.last_seen("c").is_none());
        assert_eq!(w.len(), 2);
    }

    #[test]
    fn sliding_window_observe_updates_existing_entry() {
        let mut w = SlidingWindow::default();
        w.observe("a", 100);
        w.observe("a", 250);
        // Updated value wins.
        assert_eq!(w.last_seen("a"), Some(250));
        assert_eq!(w.len(), 1);
    }

    fn sample_rule() -> CorrelateRule {
        CorrelateRule {
            name: "likely-power-outage".to_string(),
            sources: vec!["power/ups/grid-loss".to_string(), "network/wan-down".to_string()],
            window_seconds: 60,
            emits: "incident/likely-power-outage".to_string(),
            priority: Priority::High,
        }
    }

    #[test]
    fn evaluate_rule_fires_when_both_sources_within_window() {
        let rule = sample_rule();
        let mut w = SlidingWindow::default();
        let now = 1_700_000_000_000;
        // Both sources fired 30 s ago — inside the 60 s window.
        w.observe("power/ups/grid-loss", now - 30_000);
        w.observe("network/wan-down", now - 15_000);
        let emission = evaluate_rule(&rule, &w, now).expect("fires");
        assert_eq!(emission.rule_name, "likely-power-outage");
        assert_eq!(emission.topic, "incident/likely-power-outage");
        assert_eq!(emission.priority, Priority::High);
    }

    #[test]
    fn evaluate_rule_no_fire_when_source_outside_window() {
        let rule = sample_rule();
        let mut w = SlidingWindow::default();
        let now = 1_700_000_000_000;
        w.observe("power/ups/grid-loss", now - 30_000);
        // network/wan-down fired 90 s ago — beyond the 60 s window.
        w.observe("network/wan-down", now - 90_000);
        assert!(evaluate_rule(&rule, &w, now).is_none());
    }

    #[test]
    fn evaluate_rule_no_fire_when_source_missing() {
        let rule = sample_rule();
        let mut w = SlidingWindow::default();
        let now = 1_700_000_000_000;
        // Only one of the two sources observed.
        w.observe("power/ups/grid-loss", now - 30_000);
        assert!(evaluate_rule(&rule, &w, now).is_none());
    }

    #[test]
    fn evaluate_rule_no_fire_on_empty_sources() {
        let rule = CorrelateRule {
            name: "always-fire-bug".to_string(),
            sources: vec![],
            window_seconds: 60,
            emits: "x".to_string(),
            priority: Priority::Default,
        };
        let w = SlidingWindow::default();
        // Empty source set returns None — an always-firing rule
        // is a config error, not a feature.
        assert!(evaluate_rule(&rule, &w, 0).is_none());
    }

    #[test]
    fn evaluate_rule_zero_window_requires_exact_now() {
        let rule = CorrelateRule {
            name: "instant".to_string(),
            sources: vec!["a".to_string()],
            window_seconds: 0,
            emits: "x".to_string(),
            priority: Priority::Default,
        };
        let mut w = SlidingWindow::default();
        let now = 1_700_000_000_000;
        // Observation at exactly `now` → cutoff = now - 0 = now;
        // ts >= cutoff → fires.
        w.observe("a", now);
        assert!(evaluate_rule(&rule, &w, now).is_some());
        // Observation 1 ms ago → cutoff still now; ts < cutoff
        // → no fire.
        let mut w2 = SlidingWindow::default();
        w2.observe("a", now - 1);
        assert!(evaluate_rule(&rule, &w2, now).is_none());
    }

    #[test]
    fn load_default_missing_file_returns_default() {
        let p = std::path::Path::new("/nonexistent/path/bus-correlate.yaml");
        let cfg = load_default(p).unwrap();
        assert!(cfg.rules.is_empty());
    }

    #[test]
    fn load_default_round_trips() {
        let tmp = std::env::temp_dir().join(format!("mde-bus-correlate-load-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("bus-correlate.yaml");
        std::fs::write(
            &path,
            "rules:\n  - name: r\n    sources: [a, b]\n    window_seconds: 60\n    emits: x\n",
        )
        .unwrap();
        let cfg = load_default(&path).unwrap();
        assert_eq!(cfg.rules.len(), 1);
        assert_eq!(cfg.rules[0].sources, vec!["a".to_string(), "b".to_string()]);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
