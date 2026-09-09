use crate::error::query::QueryError;
use bincode::de::{Decode, Decoder};
use bincode::error::DecodeError;
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
/// encoded values are far smaller than this ceiling. The protocol limits a
/// complete index key to 255 bytes, so 4 KiB leaves ample room for bincode
/// overhead.
pub(crate) const MAX_ENCODED_INDEX_VALUE_BYTES: usize = 4 * 1024;

/// Maximum total encoded index/cursor bytes accepted across one query.
///
/// Keep this separate from the per-value ceiling so either boundary can be
/// tightened without silently changing the other.
const MAX_AGGREGATE_ENCODED_INDEX_VALUES_BYTES: usize = 4 * 1024;

/// Maximum decoded-memory budget for one value. This remains deliberately
/// larger than the encoded budget to accommodate valid nested Values while
/// preventing declared container lengths from causing unreasonable allocation.
const MAX_DECODED_INDEX_VALUE_BYTES: usize = 64 * 1024;

/// Maximum nesting depth for caller-supplied Array and Map values.
///
/// Bincode's allocation limit does not bound recursive decoder frames. A
/// canonical 4 KiB payload can otherwise contain more than 2,000 nested
/// single-element arrays and overflow Drive's 8 MiB dev-profile worker stack.
const MAX_INDEX_VALUE_NESTING_DEPTH: usize = 64;

struct DepthLimitedValue(Value);

impl Decode<()> for DepthLimitedValue {
    fn decode<D: Decoder<Context = ()>>(decoder: &mut D) -> Result<Self, DecodeError> {
        decode_value_at_depth(decoder, 0).map(Self)
    }
}

fn decode_value_at_depth<D: Decoder<Context = ()>>(
    decoder: &mut D,
    depth: usize,
) -> Result<Value, DecodeError> {
    if depth > MAX_INDEX_VALUE_NESTING_DEPTH {
        return Err(DecodeError::Other(
            "maximum platform Value nesting depth exceeded",
        ));
    }

    Ok(match u32::decode(decoder)? {
        0 => Value::U128(u128::decode(decoder)?),
        1 => Value::I128(i128::decode(decoder)?),
        2 => Value::U64(u64::decode(decoder)?),
        3 => Value::I64(i64::decode(decoder)?),
        4 => Value::U32(u32::decode(decoder)?),
        5 => Value::I32(i32::decode(decoder)?),
        6 => Value::U16(u16::decode(decoder)?),
        7 => Value::I16(i16::decode(decoder)?),
        8 => Value::U8(u8::decode(decoder)?),
        9 => Value::I8(i8::decode(decoder)?),
        10 => Value::Bytes(Vec::<u8>::decode(decoder)?),
        11 => Value::Bytes20(<[u8; 20]>::decode(decoder)?),
        12 => Value::Bytes32(<[u8; 32]>::decode(decoder)?),
        13 => Value::Bytes36(<[u8; 36]>::decode(decoder)?),
        14 => Value::EnumU8(Vec::<u8>::decode(decoder)?),
        15 => Value::EnumString(Vec::<String>::decode(decoder)?),
        16 => Value::Identifier(<[u8; 32]>::decode(decoder)?),
        17 => Value::Float(f64::decode(decoder)?),
        18 => Value::Text(String::decode(decoder)?),
        19 => Value::Bool(bool::decode(decoder)?),
        20 => Value::Null,
        21 => Value::Array(decode_array_at_depth(decoder, depth)?),
        22 => Value::Map(decode_map_at_depth(decoder, depth)?),
        _ => return Err(DecodeError::Other("unexpected platform Value variant")),
    })
}

fn decode_array_at_depth<D: Decoder<Context = ()>>(
    decoder: &mut D,
    depth: usize,
) -> Result<Vec<Value>, DecodeError> {
    let len = usize::decode(decoder)?;
    decoder.claim_container_read::<Value>(len)?;

    let mut values = Vec::with_capacity(len);
    for _ in 0..len {
        decoder.unclaim_bytes_read(std::mem::size_of::<Value>());
        values.push(decode_value_at_depth(decoder, depth + 1)?);
    }
    Ok(values)
}

fn decode_map_at_depth<D: Decoder<Context = ()>>(
    decoder: &mut D,
    depth: usize,
) -> Result<Vec<(Value, Value)>, DecodeError> {
    let len = usize::decode(decoder)?;
    decoder.claim_container_read::<(Value, Value)>(len)?;

    let mut values = Vec::with_capacity(len);
    for _ in 0..len {
        decoder.unclaim_bytes_read(std::mem::size_of::<(Value, Value)>());
        values.push((
            decode_value_at_depth(decoder, depth + 1)?,
            decode_value_at_depth(decoder, depth + 1)?,
        ));
    }
    Ok(values)
}

/// Validate the aggregate cursor budget before decoding any individual value.
///
/// A per-value bincode limit alone is insufficient for protobuf repeated
/// fields because its budget resets for every element. Bounding both the
/// number of values and their total encoded size prevents a request from
/// multiplying many individually bounded allocations.
pub(crate) fn validate_serialized_index_values<'a, I, F>(
    serialized_values: I,
    max_values: usize,
    invalid_argument: F,
) -> Result<(), QueryError>
where
    I: IntoIterator<Item = &'a [u8]>,
    F: FnOnce() -> String,
{
    let mut count = 0usize;
    let mut total_bytes = 0usize;

    for serialized_value in serialized_values {
        count = count.saturating_add(1);
        total_bytes = total_bytes.saturating_add(serialized_value.len());

        if count > max_values || total_bytes > MAX_AGGREGATE_ENCODED_INDEX_VALUES_BYTES {
            return Err(QueryError::InvalidArgument(invalid_argument()));
        }
    }

    Ok(())
}

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
            .with_limit::<MAX_DECODED_INDEX_VALUE_BYTES>();

        let (DepthLimitedValue(value), consumed) =
            bincode::decode_from_slice::<DepthLimitedValue, _>(serialized_value, config).ok()?;

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

    fn nested_array_payload(levels: usize) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(levels * 2 + 1);
        for _ in 0..levels {
            encoded.extend_from_slice(&[0x15, 0x01]);
        }
        encoded.push(0x14);
        encoded
    }

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
    fn rejects_too_many_serialized_values() {
        let values = [vec![0u8], vec![1u8]];
        let result = validate_serialized_index_values(values.iter().map(Vec::as_slice), 1, || {
            "invalid".to_string()
        });
        assert!(matches!(result, Err(QueryError::InvalidArgument(_))));
    }

    #[test]
    fn rejects_aggregate_input_over_the_byte_ceiling() {
        let values = [
            vec![0u8; MAX_AGGREGATE_ENCODED_INDEX_VALUES_BYTES / 2 + 1],
            vec![0u8; MAX_AGGREGATE_ENCODED_INDEX_VALUES_BYTES / 2],
        ];
        let result = validate_serialized_index_values(
            values.iter().map(Vec::as_slice),
            values.len(),
            || "invalid".to_string(),
        );
        assert!(matches!(result, Err(QueryError::InvalidArgument(_))));
    }

    #[test]
    fn accepts_values_within_the_aggregate_budget() {
        let values = [vec![0u8; 32], vec![1u8; 32]];
        validate_serialized_index_values(values.iter().map(Vec::as_slice), values.len(), || {
            "invalid".to_string()
        })
        .expect("values should be within the aggregate budget");
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

    #[test]
    fn round_trips_a_valid_nested_value() {
        let value = Value::Array(vec![Value::U8(1); 512]);
        let encoded =
            bincode::encode_to_vec(value.clone(), bincode::config::standard().with_big_endian())
                .expect("encode");
        assert!(encoded.len() < MAX_ENCODED_INDEX_VALUE_BYTES);

        let decoded =
            decode_serialized_index_value(&encoded, || "invalid".to_string()).expect("decode");
        assert_eq!(decoded, value);
    }

    #[test]
    fn round_trips_every_value_variant() {
        let values = vec![
            Value::U128(u128::MAX),
            Value::I128(i128::MIN),
            Value::U64(u64::MAX),
            Value::I64(i64::MIN),
            Value::U32(u32::MAX),
            Value::I32(i32::MIN),
            Value::U16(u16::MAX),
            Value::I16(i16::MIN),
            Value::U8(u8::MAX),
            Value::I8(i8::MIN),
            Value::Bytes(vec![1, 2, 3]),
            Value::Bytes20([1; 20]),
            Value::Bytes32([2; 32]),
            Value::Bytes36([3; 36]),
            Value::EnumU8(vec![1, 2]),
            Value::EnumString(vec!["one".to_string(), "two".to_string()]),
            Value::Identifier([4; 32]),
            Value::Float(1.5),
            Value::Text("text".to_string()),
            Value::Bool(true),
            Value::Null,
            Value::Array(vec![Value::U8(1)]),
            Value::Map(vec![(Value::Text("key".to_string()), Value::U8(1))]),
        ];

        for value in values {
            let encoded = bincode::encode_to_vec(
                value.clone(),
                bincode::config::standard().with_big_endian(),
            )
            .expect("encode");
            let decoded =
                decode_serialized_index_value(&encoded, || "invalid".to_string()).expect("decode");
            assert_eq!(decoded, value);
        }
    }

    #[test]
    fn accepts_the_maximum_nesting_depth() {
        let encoded = nested_array_payload(MAX_INDEX_VALUE_NESTING_DEPTH);
        decode_serialized_index_value(&encoded, || "invalid".to_string())
            .expect("depth at the limit should decode");
    }

    #[test]
    fn rejects_excessively_nested_maps() {
        let levels = MAX_INDEX_VALUE_NESTING_DEPTH + 1;
        let mut encoded = Vec::with_capacity(levels * 3 + 1);
        for _ in 0..levels {
            encoded.extend_from_slice(&[0x16, 0x01, 0x14]);
        }
        encoded.push(0x14);

        let result = decode_serialized_index_value(&encoded, || "invalid".to_string());
        assert!(matches!(result, Err(QueryError::InvalidArgument(_))));
    }

    #[test]
    fn rejects_excessive_nesting_without_overflowing_the_drive_stack() {
        const NESTING_LEVELS: usize = 2047;
        let encoded = nested_array_payload(NESTING_LEVELS);
        assert_eq!(encoded.len(), MAX_ENCODED_INDEX_VALUE_BYTES - 1);

        let result = std::thread::Builder::new()
            .stack_size(8 * 1024 * 1024)
            .spawn(move || decode_serialized_index_value(&encoded, || "invalid".to_string()))
            .expect("spawn")
            .join();

        assert!(matches!(result, Ok(Err(QueryError::InvalidArgument(_)))));
    }
}
