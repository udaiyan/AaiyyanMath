//! Validation of user-chosen names (math labels, plot variables).
//!
//! Names flow into two sinks that must agree: the Rust binding table and the
//! generated Python script, where they become assignment targets. Anything
//! that is not a plain ASCII identifier is rejected, as are Python keywords
//! and names the generated script reserves for itself. This stops labels like
//! `x'; import os; #` from being injected into the script, and labels like
//! `my var` from silently producing broken Python.

/// Python 3 hard keywords — never legal assignment targets.
/// Soft keywords (`match`, `case`, `type`, `_`) are legal identifiers.
pub const PYTHON_KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while",
    "with", "yield",
];

/// Names the generated Python script uses for its own machinery.
pub const RESERVED_SCRIPT_NAMES: &[&str] = &[
    "np",
    "plt",
    "math",
    "pint",
    "matplotlib",
    "ureg",
    "print",
    "_deriv",
    "_integ",
];

/// A non-empty, Python-safe identifier.
pub fn validate_identifier(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("name cannot be empty".to_string());
    }
    let first = name.chars().next().unwrap();
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(format!("'{name}' must start with a letter or underscore"));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(format!(
            "'{name}' may only contain letters, digits and underscores (no spaces or punctuation)"
        ));
    }
    if PYTHON_KEYWORDS.contains(&name) {
        return Err(format!("'{name}' is a Python keyword"));
    }
    if RESERVED_SCRIPT_NAMES.contains(&name) {
        return Err(format!(
            "'{name}' is reserved by the generated Python script"
        ));
    }
    Ok(())
}

/// A math-region label: like an identifier, but empty means "anonymous".
pub fn validate_label(label: &str) -> Result<(), String> {
    if label.is_empty() {
        Ok(())
    } else {
        validate_identifier(label)
    }
}

pub fn is_valid_label(label: &str) -> bool {
    validate_label(label).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_identifiers() {
        for name in [
            "a",
            "x1",
            "_x",
            "_",
            "CamelCase",
            "v_2",
            "a123",
            "Class",
            "none",
        ] {
            assert!(validate_identifier(name).is_ok(), "rejected {name}");
        }
    }

    #[test]
    fn empty_label_is_anonymous_but_empty_identifier_is_not() {
        assert!(validate_label("").is_ok());
        assert!(validate_identifier("").is_err());
    }

    #[test]
    fn rejects_leading_digit() {
        assert!(validate_identifier("2x").is_err());
        assert!(validate_identifier("1").is_err());
    }

    #[test]
    fn rejects_spaces_and_punctuation() {
        for name in [
            "my var", "a b", "a-b", "a.b", "a;b", "a,b", "a'b", "a\"b", "a/b", "a\\b", "a(b)",
            "a[0]", "a+b", "a=b", "a#", "a\nb", "a\tb", "a:b",
        ] {
            assert!(validate_identifier(name).is_err(), "accepted {name:?}");
        }
    }

    #[test]
    fn rejects_injection_payloads() {
        for name in [
            "x'; import os; #",
            "a\nimport os",
            "x\"; __import__('os').system('calc'); #",
            "a = 1; evil",
        ] {
            assert!(validate_label(name).is_err(), "accepted {name:?}");
        }
    }

    #[test]
    fn rejects_every_python_keyword() {
        assert_eq!(PYTHON_KEYWORDS.len(), 35);
        for kw in PYTHON_KEYWORDS {
            assert!(validate_identifier(kw).is_err(), "accepted keyword {kw}");
            assert!(validate_label(kw).is_err());
        }
    }

    #[test]
    fn keyword_check_is_case_sensitive() {
        assert!(validate_identifier("class").is_err());
        assert!(validate_identifier("Class").is_ok());
        assert!(validate_identifier("None").is_err());
        assert!(validate_identifier("none").is_ok());
    }

    #[test]
    fn rejects_reserved_script_names() {
        for name in RESERVED_SCRIPT_NAMES {
            assert!(
                validate_identifier(name).is_err(),
                "accepted reserved {name}"
            );
        }
    }

    #[test]
    fn allows_soft_keywords() {
        for name in ["match", "case", "type", "_"] {
            assert!(
                validate_identifier(name).is_ok(),
                "rejected soft keyword {name}"
            );
        }
    }

    #[test]
    fn rejects_non_ascii() {
        // exmex and the Python generator must agree; keep the alphabet ASCII.
        for name in ["café", "λ", "x²", "ｍ"] {
            assert!(validate_identifier(name).is_err(), "accepted {name}");
        }
    }

    #[test]
    fn unit_names_are_valid_labels() {
        // Shadowing a unit is the user's choice (Mathcad allows it); not our bug.
        for name in ["m", "s", "kg", "pi", "e", "A"] {
            assert!(validate_label(name).is_ok());
        }
    }

    #[test]
    fn error_messages_are_actionable() {
        assert!(validate_identifier("my var")
            .unwrap_err()
            .contains("letters, digits"));
        assert!(validate_identifier("2x")
            .unwrap_err()
            .contains("start with"));
        assert!(validate_identifier("class")
            .unwrap_err()
            .contains("keyword"));
        assert!(validate_identifier("np").unwrap_err().contains("reserved"));
    }

    #[test]
    fn is_valid_label_matches_validate() {
        assert!(is_valid_label(""));
        assert!(is_valid_label("good_name"));
        assert!(!is_valid_label("bad name"));
        assert!(!is_valid_label("for"));
    }
}
