// Contested resource modules
pub(crate) mod contender_document;
mod queries;
mod transitions;

// Re-export all public functions
pub use queries::*;
pub use transitions::*;

use crate::{DashSDKError, DashSDKErrorCode};
use dash_sdk::dpp::platform_value::Value;

/// Marks an index-value string as text whatever follows it. See
/// [`parse_index_value`].
const TEXT_ESCAPE: char = '\\';

/// Decode a single JSON index-value string into a platform [`Value`] using an
/// **explicit** type tag rather than guessing by content.
///
/// * A string starting with a backslash is [`Value::Text`]: that one
///   backslash is removed and the rest is taken verbatim. This is how a text
///   value that itself starts with `"0x"` or a backslash is passed, for
///   example the DPNS label "oxford", which normalizes to `"0xf0rd"` and is
///   passed as `"\\0xf0rd"`.
/// * Otherwise a string with a `"0x"` prefix is hex-decoded into
///   [`Value::Bytes`] (the remainder after `0x` must be valid, even-length
///   hex).
/// * Any other string is taken verbatim as [`Value::Text`].
///
/// [`render_index_value`] is the exact inverse for text and byte values.
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
    if let Some(text) = value_str.strip_prefix(TEXT_ESCAPE) {
        return Ok(Value::Text(text.to_string()));
    }
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

/// Decode a JSON array of index-value strings, each with
/// [`parse_index_value`]. Every contested-resource FFI that takes index
/// values, read or vote, goes through here.
pub(crate) fn parse_index_values_json(json: &str) -> Result<Vec<Value>, String> {
    let strings: Vec<String> = serde_json::from_str(json)
        .map_err(|e| format!("Failed to parse index values JSON: {}", e))?;
    strings.into_iter().map(parse_index_value).collect()
}

/// The Drive request limit for a contested-resource query's `count`: `None`
/// for 0 (no limit is sent, so the node applies its default page size),
/// otherwise `count` itself. Drive limits are `u16`, so a `count` above
/// `u16::MAX` is refused with `InvalidParameter` rather than wrapped.
pub(crate) fn request_limit(count: u32) -> Result<Option<u16>, DashSDKError> {
    if count == 0 {
        return Ok(None);
    }
    u16::try_from(count).map(Some).map_err(|_| {
        DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!("count {} exceeds the maximum of {}", count, u16::MAX),
        )
    })
}

/// Render a platform [`Value`] as an index-value string, the inverse of
/// [`parse_index_value`], so a value the read path returns can be passed back
/// into a query or a vote unchanged.
///
/// * [`Value::Text`] is returned verbatim, unless it starts with `"0x"` or a
///   backslash: then one backslash is put in front so it parses back as text.
/// * Byte values ([`Value::Bytes`], the fixed-size byte variants and
///   [`Value::Identifier`]) become `"0x"` followed by lowercase hex.
/// * Any other kind uses its plain string representation. Contested indexes
///   hold text or bytes, so this is for display only and does not parse back
///   to the same kind.
pub(crate) fn render_index_value(value: &Value) -> String {
    match value {
        Value::Text(text) if text.starts_with("0x") || text.starts_with(TEXT_ESCAPE) => {
            format!("{TEXT_ESCAPE}{text}")
        }
        Value::Text(text) => text.clone(),
        other => match other.as_bytes_slice() {
            Ok(bytes) => format!("0x{}", hex::encode(bytes)),
            Err(_) => other.non_qualified_string_representation(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_index_value, parse_index_values_json, render_index_value, request_limit};
    use crate::{dash_sdk_error_free, DashSDKErrorCode};
    use dash_sdk::dpp::platform_value::Value;

    #[test]
    fn should_send_no_limit_for_count_zero_and_the_count_up_to_u16_max() {
        assert_eq!(request_limit(0).ok(), Some(None));
        assert_eq!(request_limit(1).ok(), Some(Some(1)));
        assert_eq!(
            request_limit(u32::from(u16::MAX)).ok(),
            Some(Some(u16::MAX))
        );
    }

    #[test]
    fn should_refuse_a_count_above_u16_max_instead_of_wrapping() {
        let error = request_limit(u32::from(u16::MAX) + 1).expect_err("count above u16::MAX");
        assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
        // SAFETY: the error was allocated by `DashSDKError::new` and is freed once.
        unsafe { dash_sdk_error_free(Box::into_raw(Box::new(error))) };
    }

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
    fn should_round_trip_a_normalized_dpns_label_starting_with_0x_as_text() {
        // DPNS normalization turns "oxford" into "0xf0rd". "0xface" is valid
        // hex, so without the escape it would address bytes, not the label.
        for label in ["0xf0rd", "0xface"] {
            let value = Value::Text(label.to_string());
            let rendered = render_index_value(&value);
            assert_eq!(rendered, format!("\\{label}"));
            assert_eq!(parse_index_value(rendered).unwrap(), value);
        }
    }

    #[test]
    fn should_round_trip_text_starting_with_a_backslash() {
        for text in ["\\", "\\0xab", "\\\\x"] {
            let value = Value::Text(text.to_string());
            let rendered = render_index_value(&value);
            assert_eq!(rendered, format!("\\{text}"));
            assert_eq!(parse_index_value(rendered).unwrap(), value);
        }
    }

    #[test]
    fn should_read_a_backslash_escaped_value_as_text_without_interpreting_the_rest() {
        assert_eq!(
            parse_index_value("\\0xf0rd".to_string()).unwrap(),
            Value::Text("0xf0rd".to_string())
        );
        assert_eq!(
            parse_index_value("\\dash".to_string()).unwrap(),
            Value::Text("dash".to_string())
        );
    }

    #[test]
    fn should_parse_a_json_array_mixing_escaped_text_plain_text_and_bytes() {
        assert_eq!(
            parse_index_values_json(r#"["dash", "\\0xf0rd", "0x0a"]"#).unwrap(),
            vec![
                Value::Text("dash".to_string()),
                Value::Text("0xf0rd".to_string()),
                Value::Bytes(vec![0x0a]),
            ]
        );
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
