mod contested_resource_identity_votes;
mod contested_resource_vote_state;
mod contested_resource_voters_for_identity;
mod contested_resources;
mod vote_polls_by_end_date_query;

use crate::error::query::QueryError;
use dpp::platform_value::Value;

/// Maximum number of encoded bytes accepted for a single caller-supplied,
/// `bincode`-encoded `platform_value::Value` used as an index or cursor value
/// in the public contested-resource voting queries.
///
/// These queries accept opaque, caller-controlled cursors. Decoding them with
/// an unlimited `bincode` configuration lets a tiny input declare an enormous
/// container length (e.g. a ten-byte `Value::Bytes` payload declaring a
/// `u64::MAX` length). `bincode` would attempt to reserve that capacity before
/// noticing the body is missing, panicking with `capacity overflow`. Because
/// the process-wide panic hook cancels the token shared by the query and
/// consensus servers, such a panic on a public, unauthenticated query would
/// take the whole node down. Bounding both the raw input length and the
/// decoder's allocation budget keeps decoding safe. Legitimate index/cursor
/// values are far smaller than this ceiling.
pub(crate) const MAX_ENCODED_INDEX_VALUE_BYTES: usize = 64 * 1024;

/// Decode a single caller-supplied, `bincode`-encoded `platform_value::Value`
/// (an index or cursor value) under a finite budget.
///
/// The decoder is configured with a byte limit so that an oversized declared
/// container length is rejected with `DecodeError::LimitExceeded` *before* any
/// allocation is attempted, rather than panicking. The raw input length is
/// checked up front, and trailing bytes are rejected so each encoding is
/// canonical.
///
/// `invalid_argument` produces the error message used for every rejection
/// reason so callers keep their field- and position-specific wording.
pub(crate) fn decode_serialized_index_value<F>(
    serialized_value: &[u8],
    invalid_argument: F,
) -> Result<Value, QueryError>
where
    F: FnOnce() -> String,
{
    fn try_decode(serialized_value: &[u8]) -> Option<Value> {
        if serialized_value.len() > MAX_ENCODED_INDEX_VALUE_BYTES {
            return None;
        }

        let config = bincode::config::standard()
            .with_big_endian()
            .with_limit::<MAX_ENCODED_INDEX_VALUE_BYTES>();

        let (value, consumed) =
            bincode::decode_from_slice::<Value, _>(serialized_value, config).ok()?;

        // Reject trailing bytes so each encoded cursor value is canonical.
        if consumed != serialized_value.len() {
            return None;
        }

        Some(value)
    }

    try_decode(serialized_value).ok_or_else(|| QueryError::InvalidArgument(invalid_argument()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exact ten-byte `Value::Bytes` payload from the advisory: enum
    /// discriminant 10, variable-integer marker `0xfd`, then a big-endian
    /// `u64::MAX` declared length with no body.
    const CAPACITY_OVERFLOW_PAYLOAD: [u8; 10] =
        [0x0a, 0xfd, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];

    #[test]
    fn rejects_unbounded_length_payload_without_panicking() {
        let result =
            decode_serialized_index_value(&CAPACITY_OVERFLOW_PAYLOAD, || "invalid".to_string());
        assert!(matches!(result, Err(QueryError::InvalidArgument(_))));
    }

    #[test]
    fn rejects_input_over_the_byte_ceiling() {
        let oversized = vec![0u8; MAX_ENCODED_INDEX_VALUE_BYTES + 1];
        let result = decode_serialized_index_value(&oversized, || "invalid".to_string());
        assert!(matches!(result, Err(QueryError::InvalidArgument(_))));
    }

    #[test]
    fn rejects_trailing_bytes() {
        // A valid encoding of Value::U8(1) is `[0x08, 0x01]`; append a stray byte.
        let mut encoded =
            bincode::encode_to_vec(Value::U8(1), bincode::config::standard().with_big_endian())
                .expect("encode");
        encoded.push(0x00);
        let result = decode_serialized_index_value(&encoded, || "invalid".to_string());
        assert!(matches!(result, Err(QueryError::InvalidArgument(_))));
    }

    #[test]
    fn round_trips_a_valid_value() {
        let encoded = bincode::encode_to_vec(
            Value::Text("hello".to_string()),
            bincode::config::standard().with_big_endian(),
        )
        .expect("encode");
        let decoded =
            decode_serialized_index_value(&encoded, || "invalid".to_string()).expect("decode");
        assert_eq!(decoded, Value::Text("hello".to_string()));
    }
}
