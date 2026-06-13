// Contested resource modules
mod queries;
mod transitions;

// Re-export all public functions
pub use queries::*;
pub use transitions::*;

use dash_sdk::dpp::platform_value::Value;

/// Decode a single JSON index-value string into a platform [`Value`] using an
/// **explicit** type tag rather than guessing by content.
///
/// * A string with a `"0x"` prefix is hex-decoded into [`Value::Bytes`] (the
///   remainder after `0x` must be valid, even-length hex).
/// * Any other string is taken verbatim as [`Value::Text`].
///
/// This is the single source of truth shared by the contested-resource write
/// path ([`crate::contested_resource::transitions::cast_vote`]) and the read
/// path query FFIs (`vote_state`, `resources`, `voters_for_identity`) so a poll
/// a caller can read is the same poll it votes on.
///
/// The previous heuristic decoded any even-length all-ASCII-hex string as bytes.
/// That is unsafe on the signing path: a legitimate DPNS text label that happens
/// to be even-length all-hex (e.g. `"abcd"`, `"cafe"`, `"deadbeef"`, `"1234"`)
/// would be silently re-encoded as `Value::Bytes`, targeting a different vote
/// poll than the operator intended. DPNS index values are text labels, so
/// typical callers pass plain text (no `0x`) and correctly get `Value::Text`.
pub(crate) fn parse_index_value(value_str: String) -> Result<Value, String> {
    match value_str.strip_prefix("0x") {
        Some(hex_str) => {
            let bytes = hex::decode(hex_str).map_err(|e| {
                format!(
                    "Index value '{}' has a 0x prefix but is not valid hex: {}",
                    value_str, e
                )
            })?;
            Ok(Value::Bytes(bytes))
        }
        None => Ok(Value::Text(value_str)),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_index_value;
    use dash_sdk::dpp::platform_value::Value;

    #[test]
    fn even_length_all_hex_label_without_prefix_is_text() {
        // A DPNS-style label that is even-length all-hex must stay text on
        // both the read and write paths — the old content heuristic would have
        // re-encoded these as bytes.
        for label in ["abcd", "cafe", "face", "deadbeef", "1234"] {
            assert_eq!(
                parse_index_value(label.to_string()).unwrap(),
                Value::Text(label.to_string()),
                "{label} should decode as Value::Text"
            );
        }
    }

    #[test]
    fn zero_x_prefixed_value_is_bytes() {
        assert_eq!(
            parse_index_value("0xabcd".to_string()).unwrap(),
            Value::Bytes(vec![0xab, 0xcd])
        );
        assert_eq!(
            parse_index_value("0xdeadbeef".to_string()).unwrap(),
            Value::Bytes(vec![0xde, 0xad, 0xbe, 0xef])
        );
    }

    #[test]
    fn plain_text_label_is_text() {
        assert_eq!(
            parse_index_value("dash".to_string()).unwrap(),
            Value::Text("dash".to_string())
        );
        assert_eq!(
            parse_index_value("alice".to_string()).unwrap(),
            Value::Text("alice".to_string())
        );
    }

    #[test]
    fn zero_x_prefixed_invalid_hex_errors() {
        assert!(parse_index_value("0xnothex".to_string()).is_err());
        // odd-length hex after the prefix is rejected too
        assert!(parse_index_value("0xabc".to_string()).is_err());
    }
}
