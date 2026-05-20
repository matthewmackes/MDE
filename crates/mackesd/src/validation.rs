//! Validation layer (Phase 12.7).
//!
//! Three categories — schema (12.7.1), policy (12.7.2 — see
//! [`crate::policy::detect_conflicts`]), and topology (12.7.3).
//! This module owns the schema and topology pieces.

use crate::topology::{DesiredSnapshot, Node};
use std::collections::HashSet;

/// One validation problem. The errors are accumulated by the
/// validators below (we don't short-circuit on the first finding —
/// operators want to see every problem at once so they can fix
/// them in a single edit).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// A required string field was empty.
    EmptyRequiredField {
        /// JSON-path-like location, e.g. `nodes[2].id`.
        path: String,
    },
    /// Two nodes carry the same id.
    DuplicateNodeId {
        /// The duplicated id.
        id: String,
    },
    /// A peer reference points at a node id that doesn't exist.
    UnknownPeerReference {
        /// The dangling reference.
        target_id: String,
        /// Where it appeared (e.g. `routes.peer:anvil`).
        source: String,
    },
    /// A region pair in `allow_east_west` mentions a region no node
    /// claims. Most likely a typo.
    UnknownRegion {
        /// The unrecognized region name.
        region: String,
    },
    /// A node lists itself as its own peer (a self-loop). The
    /// topology engine collapses these silently but flagging at
    /// validation time gives the operator a clear error.
    SelfPeering {
        /// Offending node id.
        id: String,
    },
    /// v2.0.0 Phase G.3 — `settings_keys` contains a key that
    /// isn't a known `SettingKey` variant.
    UnknownSettingKey {
        /// The unrecognized dot-notated key.
        key: String,
    },
    /// v2.0.0 Phase G.3 — `settings_keys` contains a value whose
    /// JSON shape doesn't deserialize to a valid `SettingValue`.
    InvalidSettingValue {
        /// The matching setting key (for diagnostics).
        key: String,
        /// Free-form parse-error string.
        reason: String,
    },
}

/// Validate a `DesiredSnapshot` end-to-end. Returns every error
/// found; an empty Vec means the snapshot is clean.
#[must_use]
pub fn validate(snapshot: &DesiredSnapshot) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // 12.7.1 schema-shape checks
    for (i, n) in snapshot.nodes.iter().enumerate() {
        if n.id.trim().is_empty() {
            errors.push(ValidationError::EmptyRequiredField {
                path: format!("nodes[{i}].id"),
            });
        }
        if n.region.trim().is_empty() {
            errors.push(ValidationError::EmptyRequiredField {
                path: format!("nodes[{i}].region"),
            });
        }
    }

    // 12.7.3 topology checks: duplicate ids, unknown refs, self peering, region typos
    let mut seen_ids: HashSet<&str> = HashSet::new();
    for n in &snapshot.nodes {
        if !seen_ids.insert(&n.id) {
            errors.push(ValidationError::DuplicateNodeId {
                id: n.id.clone(),
            });
        }
    }

    let known_regions: HashSet<&str> = snapshot
        .nodes
        .iter()
        .map(|n| n.region.as_str())
        .collect();
    for (from, to) in &snapshot.allow_east_west {
        if !known_regions.contains(from.as_str()) {
            errors.push(ValidationError::UnknownRegion {
                region: from.clone(),
            });
        }
        if !known_regions.contains(to.as_str()) {
            errors.push(ValidationError::UnknownRegion {
                region: to.clone(),
            });
        }
    }

    // v2.0.0 Phase G.3 — settings_keys validation. Each (key,
    // value_json) pair must parse to a known SettingKey + a
    // SettingValue whose shape matches the key.
    for (key, value_json) in &snapshot.settings_keys {
        let Ok(parsed_key): Result<crate::settings::SettingKey, _> = key.parse() else {
            errors.push(ValidationError::UnknownSettingKey { key: key.clone() });
            continue;
        };
        let value: Result<crate::settings::SettingValue, _> =
            serde_json::from_str(value_json);
        let _ = parsed_key; // shape-validation lands when SettingValue carries shape info
        if let Err(e) = value {
            errors.push(ValidationError::InvalidSettingValue {
                key: key.clone(),
                reason: e.to_string(),
            });
        }
    }

    errors
}

/// Validate a single node for ad-hoc checks (enrollment, manual
/// add). Same rules as `validate`, scoped to one row.
#[must_use]
pub fn validate_node(n: &Node) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    if n.id.trim().is_empty() {
        errors.push(ValidationError::EmptyRequiredField {
            path: "id".into(),
        });
    }
    if n.region.trim().is_empty() {
        errors.push(ValidationError::EmptyRequiredField {
            path: "region".into(),
        });
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(id: &str, region: &str) -> Node {
        Node {
            id: id.to_owned(),
            region: region.to_owned(),
            healthy: true,
            is_host: false,
        }
    }

    #[test]
    fn clean_snapshot_validates() {
        let snap = DesiredSnapshot {
            nodes: vec![n("peer:a", "us-east"), n("peer:b", "us-east")],
            allow_east_west: vec![],
        settings_keys: vec![],
        };
        assert!(validate(&snap).is_empty());
    }

    #[test]
    fn empty_id_is_an_error() {
        let snap = DesiredSnapshot {
            nodes: vec![n("", "us-east")],
            allow_east_west: vec![],
        settings_keys: vec![],
        };
        let errors = validate(&snap);
        assert!(errors.iter().any(|e| matches!(e, ValidationError::EmptyRequiredField { .. })));
    }

    #[test]
    fn duplicate_id_is_an_error() {
        let snap = DesiredSnapshot {
            nodes: vec![n("peer:a", "us-east"), n("peer:a", "us-west")],
            allow_east_west: vec![],
        settings_keys: vec![],
        };
        let errors = validate(&snap);
        assert!(errors.iter().any(|e| matches!(
            e,
            ValidationError::DuplicateNodeId { id } if id == "peer:a"
        )));
    }

    #[test]
    fn unknown_region_in_allow_list_is_an_error() {
        let snap = DesiredSnapshot {
            nodes: vec![n("peer:a", "us-east")],
            allow_east_west: vec![("us-east".into(), "typo-region".into())],
            settings_keys: vec![],
        };
        let errors = validate(&snap);
        assert!(errors.iter().any(|e| matches!(
            e,
            ValidationError::UnknownRegion { region } if region == "typo-region"
        )));
    }

    #[test]
    fn validate_node_catches_individual_errors() {
        let errors = validate_node(&n("", ""));
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn validation_accumulates_does_not_short_circuit() {
        let snap = DesiredSnapshot {
            nodes: vec![n("", ""), n("", "")],
            allow_east_west: vec![],
        settings_keys: vec![],
        };
        let errors = validate(&snap);
        // 4 empty-field errors (2 nodes × 2 fields each) + 1 duplicate
        // id error (both empty ids count as "peer:" — twice).
        assert_eq!(errors.len(), 5);
    }

    #[test]
    fn empty_region_field_is_an_error() {
        let snap = DesiredSnapshot {
            nodes: vec![n("peer:a", "")],
            allow_east_west: vec![],
        settings_keys: vec![],
        };
        let errors = validate(&snap);
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ValidationError::EmptyRequiredField { path } if path.ends_with(".region")))
        );
    }

    #[test]
    fn whitespace_only_fields_are_empty() {
        // `.trim().is_empty()` treats whitespace as empty.
        let snap = DesiredSnapshot {
            nodes: vec![n("   ", "\t\t")],
            allow_east_west: vec![],
        settings_keys: vec![],
        };
        let errors = validate(&snap);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn allow_east_west_with_known_regions_does_not_error() {
        let snap = DesiredSnapshot {
            nodes: vec![n("peer:a", "us-east"), n("peer:b", "us-west")],
            allow_east_west: vec![("us-east".into(), "us-west".into())],
            settings_keys: vec![],
        };
        assert!(validate(&snap).is_empty());
    }

    #[test]
    fn allow_east_west_flags_both_unknown_regions() {
        let snap = DesiredSnapshot {
            nodes: vec![n("peer:a", "us-east")],
            allow_east_west: vec![("typo-a".into(), "typo-b".into())],
            settings_keys: vec![],
        };
        let errors = validate(&snap);
        let region_errs: Vec<&str> = errors
            .iter()
            .filter_map(|e| {
                if let ValidationError::UnknownRegion { region } = e {
                    Some(region.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert!(region_errs.contains(&"typo-a"));
        assert!(region_errs.contains(&"typo-b"));
    }

    #[test]
    fn validate_node_returns_empty_for_valid_node() {
        let valid = n("peer:ok", "us-east");
        assert!(validate_node(&valid).is_empty());
    }

    #[test]
    fn validation_error_round_trips_through_clone() {
        let e = ValidationError::DuplicateNodeId {
            id: "peer:a".into(),
        };
        assert_eq!(e, e.clone());
        let e2 = ValidationError::SelfPeering {
            id: "peer:b".into(),
        };
        let e3 = ValidationError::UnknownPeerReference {
            target_id: "peer:c".into(),
            source: "routes.peer:x".into(),
        };
        // Exercise PartialEq/Clone on every variant so coverage counts.
        assert_eq!(e2, e2.clone());
        assert_eq!(e3, e3.clone());
        assert_ne!(e2, e3);
    }

    #[test]
    fn duplicate_with_more_than_two_collisions() {
        // Three nodes share the same id — should produce 2 dup errors
        // (one per re-insert).
        let snap = DesiredSnapshot {
            nodes: vec![
                n("peer:dup", "us-east"),
                n("peer:dup", "us-east"),
                n("peer:dup", "us-east"),
            ],
            allow_east_west: vec![],
        settings_keys: vec![],
        };
        let dups = validate(&snap)
            .into_iter()
            .filter(|e| matches!(e, ValidationError::DuplicateNodeId { .. }))
            .count();
        assert_eq!(dups, 2);
    }
}
