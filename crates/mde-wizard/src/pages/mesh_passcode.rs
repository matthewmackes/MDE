//! Mesh passcode — accept the 16-char shared passcode + enrol
//! via `mded enroll`.
//!
//! The passcode is locked at 16 chars (alphanumeric uppercase
//! per v12.0 enterprise-mesh lock).

/// Locked passcode length.
pub const PASSCODE_LEN: usize = 16;

/// Validate a candidate passcode. Returns `Ok(())` when:
/// - exactly [`PASSCODE_LEN`] chars long
/// - every character is ASCII alphanumeric uppercase
#[allow(clippy::result_unit_err)]
pub fn validate(input: &str) -> Result<(), ValidationError> {
    if input.len() != PASSCODE_LEN {
        return Err(ValidationError::WrongLength {
            got: input.len(),
            want: PASSCODE_LEN,
        });
    }
    if !input
        .chars()
        .all(|c| c.is_ascii_alphanumeric() && c.is_uppercase() || c.is_ascii_digit())
    {
        return Err(ValidationError::InvalidCharacter);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    WrongLength { got: usize, want: usize },
    InvalidCharacter,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::WrongLength { got, want } => {
                write!(f, "passcode must be {want} characters (got {got})")
            }
            ValidationError::InvalidCharacter => {
                write!(f, "passcode must be uppercase letters or digits only")
            }
        }
    }
}

/// Normalize user input — strips whitespace, uppercases letters.
/// Returns the candidate ready for [`validate`].
#[must_use]
pub fn normalize(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(|c| c.to_uppercase())
        .collect()
}

/// Build the argv that invokes `mded enroll --passcode <pc>
/// --json`. The wizard's Apply page runs this once the user
/// confirms.
#[must_use]
pub fn build_enroll_argv(passcode: &str) -> Vec<String> {
    vec![
        "mded".into(),
        "enroll".into(),
        "--passcode".into(),
        passcode.to_string(),
        "--json".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passcode_len_is_16() {
        assert_eq!(PASSCODE_LEN, 16);
    }

    #[test]
    fn validate_accepts_canonical_passcode() {
        assert!(validate("0123456789ABCDEF").is_ok());
        assert!(validate("FACE000000000ACE").is_ok());
    }

    #[test]
    fn validate_rejects_short() {
        let err = validate("ABC").unwrap_err();
        assert!(matches!(err, ValidationError::WrongLength { .. }));
    }

    #[test]
    fn validate_rejects_long() {
        let err = validate("0123456789ABCDEFG").unwrap_err();
        assert!(matches!(err, ValidationError::WrongLength { .. }));
    }

    #[test]
    fn validate_rejects_lowercase() {
        let err = validate("0123456789abcdef").unwrap_err();
        assert!(matches!(err, ValidationError::InvalidCharacter));
    }

    #[test]
    fn validate_rejects_spaces() {
        let err = validate("0123 4567 89AB CDEF").unwrap_err();
        assert!(matches!(err, ValidationError::WrongLength { .. }));
    }

    #[test]
    fn normalize_strips_whitespace_and_uppercases() {
        assert_eq!(normalize("0123 4567 89ab cdef"), "0123456789ABCDEF");
        assert_eq!(normalize("\nFACE\n000000000ACE\n"), "FACE000000000ACE");
    }

    #[test]
    fn enroll_argv_includes_passcode_and_json() {
        let argv = build_enroll_argv("FACE000000000ACE");
        assert_eq!(argv[0], "mded");
        assert_eq!(argv[1], "enroll");
        assert_eq!(argv[2], "--passcode");
        assert_eq!(argv[3], "FACE000000000ACE");
        assert_eq!(argv[4], "--json");
    }

    #[test]
    fn error_display_describes_problem() {
        let s = format!("{}", ValidationError::WrongLength { got: 3, want: 16 });
        assert!(s.contains("16"));
        assert!(s.contains('3'));
        let s = format!("{}", ValidationError::InvalidCharacter);
        assert!(s.to_lowercase().contains("uppercase") || s.to_lowercase().contains("digit"));
    }
}
