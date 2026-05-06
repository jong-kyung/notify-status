//! Pure JSON-parse logic for macOS Assertions.json. Decoupled from filesystem
//! access so it can be unit-tested with fixture strings.
//!
//! The file lives at `~/Library/DoNotDisturb/DB/Assertions.json`. Apple has
//! tweaked the schema across macOS versions (12 - 15), so we parse defensively
//! using `serde_json::Value` rather than a strict typed schema. The signal we
//! want is binary: "is any Focus assertion currently in effect?" — we only
//! need to know whether `data[0].storeAssertionRecords` is a non-empty array.
//!
//! Any parse failure / missing key / unexpected shape collapses to `false`.
//! The library never crashes here; the caller can rely on a boolean result.

use serde_json::Value;

/// Returns `true` iff the parsed JSON contains a non-empty
/// `data[0].storeAssertionRecords` array.
pub fn parse_dnd_active(json: &str) -> bool {
    let value: Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let data = match value.get("data").and_then(Value::as_array) {
        Some(d) => d,
        None => return false,
    };

    let first = match data.first() {
        Some(d) => d,
        None => return false,
    };

    let records = match first.get("storeAssertionRecords").and_then(Value::as_array) {
        Some(r) => r,
        None => return false,
    };

    !records.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_active_focus_assertion() {
        let json = r#"{
            "data": [
                {
                    "storeAssertionRecords": [
                        { "assertionDetailsModeIdentifier": "com.apple.donotdisturb.mode.default" }
                    ]
                }
            ]
        }"#;
        assert!(parse_dnd_active(json));
    }

    #[test]
    fn empty_assertion_records_means_focus_inactive() {
        let json = r#"{ "data": [{ "storeAssertionRecords": [] }] }"#;
        assert!(!parse_dnd_active(json));
    }

    #[test]
    fn missing_data_key_is_inactive_not_panic() {
        assert!(!parse_dnd_active("{}"));
        assert!(!parse_dnd_active(r#"{ "other": [] }"#));
    }

    #[test]
    fn malformed_json_is_inactive_not_panic() {
        assert!(!parse_dnd_active("not json at all"));
        assert!(!parse_dnd_active(""));
        assert!(!parse_dnd_active("{ unclosed"));
    }

    #[test]
    fn empty_data_array_is_inactive() {
        assert!(!parse_dnd_active(r#"{ "data": [] }"#));
    }

    #[test]
    fn missing_store_assertion_records_is_inactive() {
        assert!(!parse_dnd_active(r#"{ "data": [{}] }"#));
        assert!(!parse_dnd_active(r#"{ "data": [{ "other": [] }] }"#));
    }

    #[test]
    fn data_or_records_with_wrong_type_is_inactive() {
        assert!(!parse_dnd_active(r#"{ "data": "not an array" }"#));
        assert!(!parse_dnd_active(r#"{ "data": [{ "storeAssertionRecords": 42 }] }"#));
    }

    #[test]
    fn multiple_records_still_active() {
        let json = r#"{
            "data": [
                {
                    "storeAssertionRecords": [
                        { "assertionDetailsModeIdentifier": "a" },
                        { "assertionDetailsModeIdentifier": "b" }
                    ]
                }
            ]
        }"#;
        assert!(parse_dnd_active(json));
    }
}
