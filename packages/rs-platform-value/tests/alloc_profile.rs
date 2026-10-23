//! Integration test for the allocation-only profile.
//!
//! Run with `cargo test -p platform-value --no-default-features --test alloc_profile`.
//! The test binary links `std` itself, but the library under test is built
//! without `std`, `random` and `platform-version`, so everything exercised here
//! is what a DashVM guest can reach. Under default features the same test runs
//! against the native profile, which proves the two profiles agree.

use platform_serialization::bounded::{BoundedDecodeError, BoundsError, CodecBounds};
use platform_value::guest_bounds::BoundedEncodeError;
use platform_value::{
    from_value, platform_value, to_value, Identifier, Value, DEFAULT_MAX_VALUE_DECODE_DEPTH,
};
use serde::{Deserialize, Serialize};

const BOUNDS: CodecBounds = CodecBounds {
    max_bytes: 1024,
    max_depth: 8,
    max_elements: 64,
};

fn native_config() -> bincode::config::Configuration<bincode::config::BigEndian> {
    bincode::config::standard().with_big_endian()
}

fn native_encode(value: &Value) -> Vec<u8> {
    bincode::encode_to_vec(value, native_config()).expect("native encode")
}

fn native_decode(bytes: &[u8]) -> Result<Value, bincode::error::DecodeError> {
    bincode::decode_from_slice::<Value, _>(bytes, native_config()).map(|(value, _)| value)
}

fn nested_arrays(depth: usize) -> Value {
    let mut value = Value::Null;
    for _ in 0..depth {
        value = Value::Array(vec![value]);
    }
    value
}

#[test]
fn should_build_values_with_the_macro_and_round_trip_them_through_serde() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Document {
        owner: Identifier,
        tags: Vec<String>,
        score: u32,
    }

    let document = Document {
        owner: Identifier::new([7; 32]),
        tags: vec!["a".into(), "b".into()],
        score: 3,
    };
    let value = to_value(&document).expect("serialize");
    let expected = platform_value!({
        "owner": Identifier::new([7; 32]),
        "tags": ["a", "b"],
        "score": 3u32,
    });
    assert_eq!(value, expected);
    let recovered: Document = from_value(value).expect("deserialize");
    assert_eq!(recovered, document);
}

#[test]
fn should_produce_native_bytes_from_the_bounded_encoder() {
    let mut value = platform_value!({
        "id": Identifier::new([1; 32]),
        "bytes": Value::Bytes(vec![1, 2, 3]),
        "list": [1u8, 2u8, { "deep": null }],
        "text": "héllo",
    });
    // The serializer behind the macro has no enumeration support, so the
    // enumeration leaves are attached by hand.
    if let Value::Map(entries) = &mut value {
        entries.push((
            Value::Text("enum".into()),
            Value::EnumString(vec!["x".into()]),
        ));
        entries.push((Value::Text("enum_u8".into()), Value::EnumU8(vec![4, 5])));
    }
    let bounded = value.encode_bounded(&BOUNDS).expect("within bounds");
    assert_eq!(bounded, native_encode(&value));
    assert_eq!(Value::decode_bounded(&bounded, &BOUNDS).unwrap(), value);
    assert_eq!(native_decode(&bounded).unwrap(), value);
}

#[test]
fn should_keep_the_identifier_encoding_unchanged() {
    let id = Identifier::new([9; 32]);
    let bytes = bincode::encode_to_vec(id, native_config()).expect("encode");
    assert_eq!(bytes, vec![9; 32]);
    let (decoded, consumed): (Identifier, usize) =
        bincode::decode_from_slice(&bytes, native_config()).expect("decode");
    assert_eq!(decoded, id);
    assert_eq!(consumed, 32);
}

#[test]
fn should_apply_the_default_depth_limit_through_the_plain_decode_impl() {
    let at_limit = native_encode(&nested_arrays(DEFAULT_MAX_VALUE_DECODE_DEPTH));
    assert!(native_decode(&at_limit).is_ok());

    let over = native_encode(&nested_arrays(DEFAULT_MAX_VALUE_DECODE_DEPTH + 1));
    let error = native_decode(&over).expect_err("one level over the limit");
    assert!(error
        .to_string()
        .contains("value nesting depth 257 exceeds maximum 256"));
}

#[test]
fn should_reject_depth_over_the_explicit_bound_on_both_paths() {
    let value = nested_arrays(9);
    let bytes = native_encode(&value);
    assert!(matches!(
        Value::decode_bounded(&bytes, &BOUNDS),
        Err(BoundedDecodeError::Bounds(BoundsError::DepthExceeded {
            depth: 9,
            max: 8
        }))
    ));
    assert!(matches!(
        value.encode_bounded(&BOUNDS),
        Err(BoundedEncodeError::Bounds(BoundsError::DepthExceeded {
            depth: 9,
            max: 8
        }))
    ));
    let inside = nested_arrays(8);
    let bytes = native_encode(&inside);
    assert_eq!(Value::decode_bounded(&bytes, &BOUNDS).unwrap(), inside);
}

#[test]
fn should_reject_declared_lengths_beyond_the_input_before_allocating() {
    // Array header declaring 2^32 items with a three byte tail.
    let bytes = [21u8, 253, 0, 0, 0, 1, 0, 0, 0, 0, 20, 20, 20];
    assert!(matches!(
        Value::decode_bounded(&bytes, &BOUNDS),
        Err(BoundedDecodeError::Bounds(
            BoundsError::DeclaredLengthExceedsInput {
                declared: 4_294_967_296,
                remaining: 3
            }
        ))
    ));
    // Bytes declaring u64::MAX with no payload.
    let bytes = [10u8, 253, 255, 255, 255, 255, 255, 255, 255, 255];
    assert!(matches!(
        Value::decode_bounded(&bytes, &BOUNDS),
        Err(BoundedDecodeError::Bounds(
            BoundsError::DeclaredLengthExceedsInput {
                declared: u64::MAX,
                remaining: 0
            }
        ))
    ));
}

#[test]
fn should_reject_input_over_max_bytes_elements_over_the_bound_and_trailing_bytes() {
    // One variant byte, a three byte length prefix and the payload.
    let big = Value::Bytes(vec![0; 2000]);
    let bytes = native_encode(&big);
    assert_eq!(bytes.len(), 2004);
    assert!(matches!(
        Value::decode_bounded(&bytes, &BOUNDS),
        Err(BoundedDecodeError::Bounds(BoundsError::BytesExceeded {
            len: 2004,
            max: 1024
        }))
    ));
    assert!(matches!(
        big.encode_bounded(&BOUNDS),
        Err(BoundedEncodeError::Bounds(BoundsError::BytesExceeded {
            len: 2004,
            max: 1024
        }))
    ));

    let wide = Value::Array(vec![Value::Null; 65]);
    let bytes = native_encode(&wide);
    assert!(matches!(
        Value::decode_bounded(&bytes, &BOUNDS),
        Err(BoundedDecodeError::Bounds(BoundsError::ElementsExceeded {
            elements: 65,
            max: 64
        }))
    ));

    let mut bytes = native_encode(&Value::U8(1));
    bytes.push(0);
    assert!(matches!(
        Value::decode_bounded(&bytes, &BOUNDS),
        Err(BoundedDecodeError::TrailingBytes { remaining: 1 })
    ));
}
