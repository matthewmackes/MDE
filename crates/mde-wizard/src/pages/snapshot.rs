//! Snapshot — pre-apply snapshot via mackesd.
//!
//! Before the Apply page mutates the user's environment, we
//! snapshot every known state file so the user can roll back
//! via `mde recover --apply <snapshot>`.

/// Default snapshot tag — timestamp-prefixed.
#[must_use]
pub fn default_tag() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("pre-wizard-{epoch}")
}

/// Validate a user-supplied tag.
#[allow(clippy::result_unit_err)]
pub fn validate_tag(tag: &str) -> Result<(), TagError> {
    if tag.is_empty() {
        return Err(TagError::Empty);
    }
    if tag.len() > 64 {
        return Err(TagError::TooLong(tag.len()));
    }
    if !tag
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(TagError::IllegalCharacter);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagError {
    Empty,
    TooLong(usize),
    IllegalCharacter,
}

/// Build the argv for `mded snapshot create --tag <tag> --json`.
#[must_use]
pub fn build_create_argv(tag: &str) -> Vec<String> {
    vec![
        "mded".into(),
        "snapshot".into(),
        "create".into(),
        "--tag".into(),
        tag.into(),
        "--json".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_tag_starts_with_prefix() {
        assert!(default_tag().starts_with("pre-wizard-"));
    }

    #[test]
    fn validate_accepts_canonical_tag() {
        assert!(validate_tag("pre-wizard-1700000000").is_ok());
        assert!(validate_tag("manual_setup.v1").is_ok());
    }

    #[test]
    fn validate_rejects_empty() {
        assert_eq!(validate_tag(""), Err(TagError::Empty));
    }

    #[test]
    fn validate_rejects_too_long() {
        let s = "a".repeat(100);
        assert!(matches!(validate_tag(&s), Err(TagError::TooLong(_))));
    }

    #[test]
    fn validate_rejects_illegal_chars() {
        assert_eq!(validate_tag("has space"), Err(TagError::IllegalCharacter));
        assert_eq!(
            validate_tag("slash/in/name"),
            Err(TagError::IllegalCharacter)
        );
    }

    #[test]
    fn create_argv_uses_snapshot_create() {
        let argv = build_create_argv("test-tag");
        assert_eq!(argv[0], "mded");
        assert_eq!(argv[1], "snapshot");
        assert_eq!(argv[2], "create");
        assert_eq!(argv[3], "--tag");
        assert_eq!(argv[4], "test-tag");
        assert_eq!(argv[5], "--json");
    }
}
