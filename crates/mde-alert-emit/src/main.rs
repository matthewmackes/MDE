//! MON-3 (v2.6) — `mde-alert-emit` binary.
//!
//! Reads Netdata's `NETDATA_ALARM_*` env vars (set by the
//! `health_alarm_notify.conf` custom-sender hook) + translates
//! the alert state change into a deterministic-ULID-named
//! JSON event under `~/.local/share/mde/alerts/<ulid>.json`.
//!
//! Idempotent. The ULID is derived deterministically from
//! `NETDATA_ALARM_UNIQUE_ID + NETDATA_ALARM_WHEN`, so
//! invoking the binary twice with the same env produces the
//! same filename + a single file (atomic-rename hides the
//! tempfile so inotify watchers see one event, not two).
//!
//! Wired by `health_alarm_notify.conf` (operator-owned via
//! MON-1's birthright pipeline once MON-1.b lands the stream
//! block). Consumed by `mackesd::workers::alert_relay`
//! (MON-4) which forwards events to the FDO notification
//! daemon.
//!
//! Lives under `/usr/libexec/mde/alert-emit` (FHS-correct
//! location for helper binaries the user never invokes
//! directly).

use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::Parser;
use serde::{Deserialize, Serialize};

/// Wire shape of the alert event written under
/// `~/.local/share/mde/alerts/<ulid>.json`. Locked 2026-05-24
/// per the in-session MON-3 design AskUserQuestion.
///
/// Future schema bumps are backward-compatible additions only
/// (new optional fields). Field names match the worklist
/// body's locked spec verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertEvent {
    /// Deterministic ULID (also the filename's stem).
    pub id: String,
    /// Unix-epoch seconds when Netdata fired the alarm
    /// (`NETDATA_ALARM_WHEN`).
    pub ts: i64,
    /// Severity from `NETDATA_ALARM_STATUS` — typically
    /// `WARNING`, `CRITICAL`, `CLEAR`, etc.
    pub severity: String,
    /// Category (`NETDATA_ALARM_CHART_CONTEXT`) — e.g.
    /// `nebula.process` / `meshfs.chunkserver`.
    pub category: String,
    /// Alert name (`NETDATA_ALARM_NAME`).
    pub alert: String,
    /// Host that fired the alarm (`NETDATA_ALARM_HOSTNAME`).
    pub host: String,
    /// One-line human summary (`NETDATA_ALARM_INFO`).
    pub summary: String,
    /// Observed value at the time of the state change
    /// (`NETDATA_ALARM_VALUE`).
    pub value: String,
    /// Threshold that triggered the state change
    /// (`NETDATA_ALARM_THRESHOLD`).
    pub threshold: String,
    /// Deep-link URL into the aggregator's Netdata web UI
    /// (`NETDATA_ALARM_CHART_URL` or a synthesized
    /// `https://<host>:19999/#menu_<chart>`).
    pub chart_url: String,
    /// Process identifier that wrote this event — set
    /// to `"mde-alert-emit"` constant so downstream consumers
    /// can distinguish events written by this helper from
    /// future event-emitting paths.
    pub fired_by: String,
    /// Per-peer ack list — starts empty. The mde-alert-relay
    /// worker on each peer appends its node-id once it has
    /// surfaced the event to the FDO notification daemon, so
    /// any peer rendering an alert view can show "seen by 4
    /// of 8 peers" without re-querying (Q3 lock 2026-05-25;
    /// was 16-peer cap).
    pub seen_by: Vec<String>,
}

/// CLI args. The expected invocation is from Netdata's
/// `health_alarm_notify.conf` custom-sender hook; the env
/// vars do all the heavy lifting + the args just override
/// the output dir + opt into dry-run mode for tests.
#[derive(Debug, Parser)]
#[command(version, about = "MON-3 — translate a Netdata alert env block into a JSON event under ~/.local/share/mde/alerts/")]
struct Args {
    /// Output directory for the JSON event. Defaults to
    /// `$XDG_DATA_HOME/mde/alerts/` or
    /// `$HOME/.local/share/mde/alerts/`.
    #[clap(long)]
    output_dir: Option<PathBuf>,

    /// Print the event JSON to stdout instead of writing to
    /// disk. Used by the `--dry-run-from-env` bench gate.
    #[clap(long)]
    dry_run_from_env: bool,

    /// BUS-4.3 — skip the Bus dual-write. Default behavior is
    /// to also shell out to `mde-bus publish mon/<class> ...`
    /// after the JSONL write, so MON alerts surface through
    /// Bus subscribers AND the legacy `~/.local/share/mde/alerts/`
    /// JSONL consumers. `--no-bus` disables the dual-write for
    /// debugging or for environments where `mde-bus` isn't
    /// installed yet (pre-enrollment peer).
    #[clap(long)]
    no_bus: bool,
}

/// MON-3 — deterministic ULID derived from
/// `NETDATA_ALARM_UNIQUE_ID + NETDATA_ALARM_WHEN`. The
/// timestamp portion comes from `when`; the randomness
/// portion is a simple FNV-1a fold of `unique_id` so the
/// same input yields the same ULID across invocations.
///
/// Not crypto-secure — the goal is `at-most-once` write
/// dedup, not unguessability. Output shape: 26-char
/// Crockford-base32 string (ULID-format-compatible).
#[must_use]
pub fn make_ulid(when_unix_s: i64, unique_id: &str) -> String {
    // 48 bits of timestamp (milliseconds since epoch).
    let ms: u64 = (when_unix_s as u64).saturating_mul(1000);
    let mut bytes = [0u8; 16];
    let ms_be = ms.to_be_bytes();
    // ms_be is 8 bytes; the 48-bit timestamp lives in the
    // low 6 bytes (high 2 bytes are zero for any timestamp
    // pre-year-10895 which is fine for our lifetime).
    bytes[0..6].copy_from_slice(&ms_be[2..8]);

    // 80 bits of "randomness" derived from a FNV-1a fold of
    // unique_id + length. Hash twice with different primes
    // to fill 10 bytes.
    let h1 = fnv1a64(unique_id.as_bytes(), 0xcbf2_9ce4_8422_2325);
    let h2 = fnv1a64(unique_id.as_bytes(), 0x84222325_cbf29ce4_u64.swap_bytes());
    bytes[6..14].copy_from_slice(&h1.to_be_bytes());
    bytes[14..16].copy_from_slice(&h2.to_be_bytes()[0..2]);

    crockford_base32_16(&bytes)
}

fn fnv1a64(input: &[u8], seed: u64) -> u64 {
    let prime: u64 = 0x100000001b3;
    let mut h = seed;
    for &b in input {
        h ^= u64::from(b);
        h = h.wrapping_mul(prime);
    }
    h
}

/// Crockford-base32 encode 16 bytes → 26 ASCII chars (ULID
/// format). Uses the Crockford alphabet
/// `0123456789ABCDEFGHJKMNPQRSTVWXYZ` (no I/L/O/U to avoid
/// ambiguity with digits).
fn crockford_base32_16(bytes: &[u8; 16]) -> String {
    const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    // Treat the 16 bytes as a 128-bit big-endian integer; we
    // output 26 base32 digits (130 bits, top 2 bits zero by
    // construction since 26*5 = 130).
    let mut buf = [0u8; 26];
    // Use u128 arithmetic — std-only, no external dep.
    let mut n = u128::from_be_bytes(*bytes);
    for slot in buf.iter_mut().rev() {
        *slot = ALPHABET[(n & 0x1f) as usize];
        n >>= 5;
    }
    String::from_utf8(buf.to_vec()).expect("ALPHABET is ASCII")
}

/// Read the Netdata env block + assemble the alert event.
/// Returns `None` when required env vars are missing — the
/// caller exits cleanly with a diagnostic.
#[must_use]
pub fn assemble_from_env(env: &BTreeMap<String, String>) -> Option<AlertEvent> {
    let when_str = env.get("NETDATA_ALARM_WHEN")?;
    let when_unix_s: i64 = when_str.parse().ok()?;
    let unique_id = env.get("NETDATA_ALARM_UNIQUE_ID")?.clone();
    let id = make_ulid(when_unix_s, &unique_id);
    Some(AlertEvent {
        id,
        ts: when_unix_s,
        severity: env
            .get("NETDATA_ALARM_STATUS")
            .cloned()
            .unwrap_or_else(|| "UNKNOWN".into()),
        category: env
            .get("NETDATA_ALARM_CHART_CONTEXT")
            .cloned()
            .unwrap_or_default(),
        alert: env
            .get("NETDATA_ALARM_NAME")
            .cloned()
            .unwrap_or_default(),
        host: env
            .get("NETDATA_ALARM_HOSTNAME")
            .cloned()
            .unwrap_or_default(),
        summary: env
            .get("NETDATA_ALARM_INFO")
            .cloned()
            .unwrap_or_default(),
        value: env
            .get("NETDATA_ALARM_VALUE")
            .cloned()
            .unwrap_or_default(),
        threshold: env
            .get("NETDATA_ALARM_THRESHOLD")
            .cloned()
            .unwrap_or_default(),
        chart_url: env
            .get("NETDATA_ALARM_CHART_URL")
            .cloned()
            .unwrap_or_default(),
        fired_by: "mde-alert-emit".into(),
        seen_by: Vec::new(),
    })
}

/// Default output dir resolution. Honors `$XDG_DATA_HOME`
/// first, falls back to `$HOME/.local/share/`.
fn default_output_dir() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
        return Some(PathBuf::from(xdg).join("mde").join("alerts"));
    }
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".local").join("share").join("mde").join("alerts"))
}

/// Atomic-write the event JSON to `<output_dir>/<id>.json`.
/// Returns the final path. Tempfile lives in the same dir
/// + gets fsync'd before rename so inotify watchers see one
/// event, not two.
fn write_event(event: &AlertEvent, output_dir: &std::path::Path) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(output_dir)?;
    let final_path = output_dir.join(format!("{}.json", event.id));
    let tmp_path = output_dir.join(format!("{}.json.tmp", event.id));
    let body = serde_json::to_string_pretty(event)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("encode: {e}")))?;
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp_path)?;
        f.write_all(body.as_bytes())?;
        f.write_all(b"\n")?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp_path, &final_path)?;
    Ok(final_path)
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let env: BTreeMap<String, String> = std::env::vars()
        .filter(|(k, _)| k.starts_with("NETDATA_ALARM_"))
        .collect();

    let Some(event) = assemble_from_env(&env) else {
        eprintln!(
            "mde-alert-emit: NETDATA_ALARM_UNIQUE_ID + NETDATA_ALARM_WHEN required (env block missing)"
        );
        std::process::exit(2);
    };

    if args.dry_run_from_env {
        let body = serde_json::to_string_pretty(&event)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("encode: {e}")))?;
        println!("{body}");
        return Ok(());
    }

    let output_dir = args
        .output_dir
        .or_else(default_output_dir)
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "$XDG_DATA_HOME / $HOME unset; pass --output-dir",
            )
        })?;
    let path = write_event(&event, &output_dir)?;
    println!("{}", path.display());

    // BUS-4.3 — dual-write to the Bus so MON alerts surface
    // through the priority-tiered surface dispatcher (status
    // strip on `high`, Theater on `urgent`) in addition to the
    // legacy alerts/ JSONL consumers. Best-effort: if `mde-bus`
    // isn't on PATH (pre-enrollment / dev box without the RPM
    // installed) we log + continue. The JSONL is durably stored
    // regardless, and a later `mde-alert-emit` run on the same
    // unique_id is a no-op anyway (deterministic ULID).
    if !args.no_bus {
        publish_to_bus(&event);
    }

    Ok(())
}

/// Map a Netdata `category` (`NETDATA_ALARM_CHART_CONTEXT`)
/// to a Bus topic under the `mon/*` namespace. Per BUS-1.6's
/// 12-topic seed lock, the four canonical mon topics are
/// `mon/cpu` / `mon/memory` / `mon/disk` / `mon/network`;
/// Nebula + meshfs contexts route to dedicated `mon/nebula`
/// / `mon/meshfs` topics that auto-create on first publish
/// (Round 3 self-serve creation lock).
///
/// Pure helper — exposed for unit tests.
#[must_use]
pub fn bus_topic_for(category: &str) -> String {
    let mut parts = category.split('.');
    let prefix = parts.next().filter(|s| !s.is_empty()).unwrap_or("system");
    match prefix {
        "nebula" => "mon/nebula".to_string(),
        "meshfs" | "lizardfs" => "mon/meshfs".to_string(),
        "mackesd" => "mon/mackesd".to_string(),
        "system" => {
            let sub = parts.next().unwrap_or("misc");
            match sub {
                "cpu" => "mon/cpu".to_string(),
                "ram" | "memory" | "mem" => "mon/memory".to_string(),
                "disk" | "io" => "mon/disk".to_string(),
                "net" | "network" | "interface" => "mon/network".to_string(),
                _ => "mon/system".to_string(),
            }
        }
        other => format!("mon/{other}"),
    }
}

/// Map Netdata's `NETDATA_ALARM_STATUS` severity to a Bus
/// priority string. `CRITICAL` → urgent (Theater takeover),
/// `WARNING` → high (status-strip + sound), `CLEAR` → default
/// (just the tray entry — the resolve is informational).
/// Anything unrecognized → default (safe fallback).
#[must_use]
pub fn bus_priority_for(severity: &str) -> &'static str {
    match severity.to_ascii_uppercase().as_str() {
        "CRITICAL" | "CRIT" | "EMERGENCY" => "urgent",
        "WARNING" | "WARN" => "high",
        "CLEAR" | "OK" | "INFO" => "default",
        _ => "default",
    }
}

/// Shell out to `mde-bus publish` to surface this alert
/// through the Bus surface dispatcher. Best-effort: every
/// failure path logs to stderr + returns Ok-equivalent so
/// the JSONL write is the authoritative outcome.
fn publish_to_bus(event: &AlertEvent) {
    let topic = bus_topic_for(&event.category);
    let priority = bus_priority_for(&event.severity);
    let title = format!("{}: {}", event.host, event.alert);
    let body = if event.chart_url.is_empty() {
        event.summary.clone()
    } else {
        format!("{}\n\n{}", event.summary, event.chart_url)
    };
    let result = std::process::Command::new("mde-bus")
        .args([
            "publish",
            &topic,
            "--priority",
            priority,
            "--title",
            &title,
            "--body-flag",
            &body,
            // --no-broker means "persist + audit only, don't
            // shell to ntfy". The bus daemon's broker may not
            // be up yet (pre-enrollment); persistence still
            // catches the event so the surface dispatcher
            // sees it through the file tree.
            "--no-broker",
        ])
        .status();
    match result {
        Ok(status) if status.success() => {}
        Ok(status) => {
            eprintln!(
                "mde-alert-emit: mde-bus publish to {topic} exited {} — alert recorded in JSONL only",
                status
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "?".to_string())
            );
        }
        Err(e) => {
            eprintln!(
                "mde-alert-emit: mde-bus binary not invocable ({e}) — alert recorded in JSONL only"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn make_ulid_is_deterministic_for_same_inputs() {
        let a = make_ulid(1_716_000_000, "unique-id-001");
        let b = make_ulid(1_716_000_000, "unique-id-001");
        assert_eq!(a, b);
        assert_eq!(a.len(), 26);
    }

    #[test]
    fn make_ulid_changes_on_different_unique_id() {
        let a = make_ulid(1_716_000_000, "unique-id-001");
        let b = make_ulid(1_716_000_000, "unique-id-002");
        assert_ne!(a, b);
    }

    #[test]
    fn make_ulid_changes_on_different_timestamp() {
        let a = make_ulid(1_716_000_000, "unique-id-001");
        let b = make_ulid(1_716_000_001, "unique-id-001");
        assert_ne!(a, b);
    }

    #[test]
    fn make_ulid_uses_crockford_alphabet_only() {
        let id = make_ulid(1_716_000_000, "some-unique-id");
        for c in id.chars() {
            assert!(
                "0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(c),
                "non-Crockford char {c} in ULID {id}",
            );
        }
    }

    #[test]
    fn make_ulid_timestamp_prefix_is_sortable() {
        // ULIDs are sortable by their timestamp prefix; later
        // ms sorts higher.
        let earlier = make_ulid(1_716_000_000, "uid-A");
        let later = make_ulid(1_716_999_999, "uid-A");
        assert!(later > earlier);
    }

    #[test]
    fn assemble_from_env_requires_unique_id_and_when() {
        // Missing both → None.
        assert!(assemble_from_env(&env(&[])).is_none());
        // Missing WHEN → None.
        assert!(assemble_from_env(&env(&[("NETDATA_ALARM_UNIQUE_ID", "u")])).is_none());
        // Missing UNIQUE_ID → None.
        assert!(assemble_from_env(&env(&[("NETDATA_ALARM_WHEN", "1716000000")])).is_none());
        // Both present → Some.
        let e = env(&[
            ("NETDATA_ALARM_UNIQUE_ID", "u"),
            ("NETDATA_ALARM_WHEN", "1716000000"),
        ]);
        assert!(assemble_from_env(&e).is_some());
    }

    #[test]
    fn assemble_from_env_carries_every_locked_field() {
        let e = env(&[
            ("NETDATA_ALARM_UNIQUE_ID", "uid-1"),
            ("NETDATA_ALARM_WHEN", "1716000000"),
            ("NETDATA_ALARM_STATUS", "CRITICAL"),
            ("NETDATA_ALARM_CHART_CONTEXT", "nebula.process"),
            ("NETDATA_ALARM_NAME", "nebula_process_down"),
            ("NETDATA_ALARM_HOSTNAME", "peer:alice"),
            ("NETDATA_ALARM_INFO", "Nebula process inactive for 45s"),
            ("NETDATA_ALARM_VALUE", "inactive"),
            ("NETDATA_ALARM_THRESHOLD", "active"),
            ("NETDATA_ALARM_CHART_URL", "https://peer:alice:19999/#menu_nebula"),
        ]);
        let ev = assemble_from_env(&e).expect("complete env → event");
        assert_eq!(ev.ts, 1_716_000_000);
        assert_eq!(ev.severity, "CRITICAL");
        assert_eq!(ev.category, "nebula.process");
        assert_eq!(ev.alert, "nebula_process_down");
        assert_eq!(ev.host, "peer:alice");
        assert_eq!(ev.summary, "Nebula process inactive for 45s");
        assert_eq!(ev.value, "inactive");
        assert_eq!(ev.threshold, "active");
        assert_eq!(ev.chart_url, "https://peer:alice:19999/#menu_nebula");
        assert_eq!(ev.fired_by, "mde-alert-emit");
        assert!(ev.seen_by.is_empty());
        // ULID is deterministic from the (unique_id, when) pair.
        assert_eq!(ev.id, make_ulid(1_716_000_000, "uid-1"));
    }

    #[test]
    fn assemble_from_env_substitutes_unknown_severity_when_status_missing() {
        let e = env(&[
            ("NETDATA_ALARM_UNIQUE_ID", "u"),
            ("NETDATA_ALARM_WHEN", "1716000000"),
        ]);
        let ev = assemble_from_env(&e).unwrap();
        assert_eq!(ev.severity, "UNKNOWN");
    }

    #[test]
    fn write_event_lands_at_id_dot_json_atomically() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let event = AlertEvent {
            id: "01H8XYZABC0123456789DEFGHJ".to_string(),
            ts: 1_716_000_000,
            severity: "WARNING".into(),
            category: "test.cat".into(),
            alert: "test_alert".into(),
            host: "peer:test".into(),
            summary: "test summary".into(),
            value: "42".into(),
            threshold: "10".into(),
            chart_url: "https://example/chart".into(),
            fired_by: "mde-alert-emit".into(),
            seen_by: vec![],
        };
        let path = write_event(&event, tmp.path()).expect("write");
        assert_eq!(path, tmp.path().join("01H8XYZABC0123456789DEFGHJ.json"));
        assert!(path.exists());
        let tmp_artifact = tmp.path().join("01H8XYZABC0123456789DEFGHJ.json.tmp");
        assert!(!tmp_artifact.exists(), "tempfile must rename away");
        // Round-trip parse.
        let body = std::fs::read_to_string(&path).expect("read");
        let parsed: AlertEvent = serde_json::from_str(&body).expect("parse");
        assert_eq!(parsed, event);
    }

    // BUS-4.3 — Bus topic + priority mapping helpers.

    #[test]
    fn bus_topic_for_routes_system_subcategories() {
        assert_eq!(bus_topic_for("system.cpu"), "mon/cpu");
        assert_eq!(bus_topic_for("system.ram"), "mon/memory");
        assert_eq!(bus_topic_for("system.memory"), "mon/memory");
        assert_eq!(bus_topic_for("system.disk"), "mon/disk");
        assert_eq!(bus_topic_for("system.io"), "mon/disk");
        assert_eq!(bus_topic_for("system.network"), "mon/network");
        assert_eq!(bus_topic_for("system.interface"), "mon/network");
        assert_eq!(bus_topic_for("system.misc"), "mon/system");
    }

    #[test]
    fn bus_topic_for_routes_dedicated_prefixes() {
        assert_eq!(bus_topic_for("nebula.process"), "mon/nebula");
        assert_eq!(bus_topic_for("meshfs.chunkserver"), "mon/meshfs");
        assert_eq!(bus_topic_for("lizardfs.heal"), "mon/meshfs");
        assert_eq!(bus_topic_for("mackesd.workers"), "mon/mackesd");
    }

    #[test]
    fn bus_topic_for_unknown_prefix_falls_to_mon_other() {
        assert_eq!(bus_topic_for("kdc.session"), "mon/kdc");
        assert_eq!(bus_topic_for(""), "mon/system");
    }

    #[test]
    fn bus_priority_for_maps_severity_classes() {
        assert_eq!(bus_priority_for("CRITICAL"), "urgent");
        assert_eq!(bus_priority_for("critical"), "urgent");
        assert_eq!(bus_priority_for("crit"), "urgent");
        assert_eq!(bus_priority_for("WARNING"), "high");
        assert_eq!(bus_priority_for("warn"), "high");
        assert_eq!(bus_priority_for("CLEAR"), "default");
        assert_eq!(bus_priority_for("OK"), "default");
        assert_eq!(bus_priority_for("INFO"), "default");
        // Unknown → default (safe fallback)
        assert_eq!(bus_priority_for("MAGENTA"), "default");
        assert_eq!(bus_priority_for(""), "default");
    }

    #[test]
    fn write_event_is_idempotent_on_repeat_invocation() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let event = AlertEvent {
            id: "01H8XYZABC0000000000000000".to_string(),
            ts: 1,
            severity: "WARNING".into(),
            category: "".into(),
            alert: "".into(),
            host: "".into(),
            summary: "".into(),
            value: "".into(),
            threshold: "".into(),
            chart_url: "".into(),
            fired_by: "mde-alert-emit".into(),
            seen_by: vec![],
        };
        let p1 = write_event(&event, tmp.path()).expect("first");
        let p2 = write_event(&event, tmp.path()).expect("second");
        assert_eq!(p1, p2);
        // Only one file in the dir.
        let entries: Vec<_> = std::fs::read_dir(tmp.path()).unwrap().collect();
        assert_eq!(entries.len(), 1);
    }
}
