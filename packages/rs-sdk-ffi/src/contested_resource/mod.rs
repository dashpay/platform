// Contested resource modules
pub(crate) mod contender_document;
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

/// Render a platform [`Value`] as an index-value string, the inverse of
/// [`parse_index_value`], so a value the read path returns can be passed back
/// into a query or a vote unchanged.
///
/// * [`Value::Text`] is returned verbatim.
/// * Byte values ([`Value::Bytes`], the fixed-size byte variants and
///   [`Value::Identifier`]) become `"0x"` followed by lowercase hex.
/// * Any other kind uses its plain string representation. Contested indexes
///   hold text or bytes, so this is for display only and does not parse back
///   to the same kind.
///
/// A text value that itself starts with `"0x"` parses back as bytes; that is
/// the limit of the `"0x"` convention in [`parse_index_value`].
pub(crate) fn render_index_value(value: &Value) -> String {
    match value {
        Value::Text(text) => text.clone(),
        other => match other.as_bytes_slice() {
            Ok(bytes) => format!("0x{}", hex::encode(bytes)),
            Err(_) => other.non_qualified_string_representation(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_index_value, render_index_value};
    use dash_sdk::dpp::platform_value::Value;

    #[test]
    fn should_render_text_label_verbatim_and_parse_it_back() {
        // "a11ce0" holds '0', which is outside the base58 alphabet.
        let value = Value::Text("a11ce0".to_string());
        let rendered = render_index_value(&value);
        assert_eq!(rendered, "a11ce0");
        assert_eq!(parse_index_value(rendered).unwrap(), value);
    }

    #[test]
    fn should_render_bytes_as_0x_hex_and_parse_them_back() {
        let value = Value::Bytes(vec![0x00, 0xab, 0xcd]);
        let rendered = render_index_value(&value);
        assert_eq!(rendered, "0x00abcd");
        assert_eq!(parse_index_value(rendered).unwrap(), value);
    }

    #[test]
    fn should_render_identifier_as_0x_hex() {
        let value = Value::Identifier([7u8; 32]);
        assert_eq!(render_index_value(&value), format!("0x{}", "07".repeat(32)));
    }

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
