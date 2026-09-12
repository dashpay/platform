//! Bounded value codec for allocation-only guests.
//!
//! [`Value::decode_bounded`] and [`Value::encode_bounded`] take explicit
//! [`CodecBounds`] instead of the native thread-local depth limit. They run the
//! same iterative decoder state machine as the blanket [`bincode::Decode`] impl
//! and produce the same bytes as the derived [`bincode::Encode`] impl, so guest
//! bytes are native bytes. What differs is how untrusted lengths are handled:
//!
//! - every declared length (container count, byte string, text, string list
//!   and each of its strings) is checked against the unread input and the
//!   budget before any allocation;
//! - byte leaves then allocate exactly the declared length, and containers
//!   start empty and grow by push;
//! - depth and element counts are charged at the header, so an oversized
//!   container is refused before its first entry is decoded.
//!
//! Heap usage is bounded by the three limits together: byte leaves by
//! `max_bytes`, container storage by `max_elements` (plus vector growth
//! slack), and the frame stack by `max_depth`. It is not bounded by
//! `max_bytes` alone; a three byte array header still allocates a frame and an
//! element vector.
//!
//! Bounded decoding is not canonical validation. bincode accepts overlong
//! variable-length integers, so two byte strings can decode to the same value
//! here; canonical bytes are established by re-encoding and comparing, which
//! the ABI layer does.

use alloc::string::String;
use alloc::vec::Vec;
use bincode::de::Decoder;
use bincode::enc::write::{SizeWriter, Writer};
use bincode::enc::{Encode, Encoder, EncoderImpl};
use bincode::error::EncodeError;
use core::fmt::{self, Display, Formatter};
use platform_serialization::bounded::{
    bounded_decode_from_slice, canonical_config, decode_bounded_bytes, decode_bounded_len,
    decode_bounded_string, decode_bounded_string_list, BoundedDecodeError, BoundsError,
    CodecBounds, CodecBudget, RemainingInput,
};

use crate::{
    decode_value_with, Value, ValueLeafReader, ValueMap, VALUE_ARRAY_VARIANT, VALUE_MAP_VARIANT,
};

/// Failure of [`Value::encode_bounded`].
#[derive(Debug)]
pub enum BoundedEncodeError {
    /// A bound from [`CodecBounds`] was hit.
    Bounds(BoundsError),
    /// bincode refused to encode a leaf.
    Encode(EncodeError),
}

impl From<BoundsError> for BoundedEncodeError {
    fn from(error: BoundsError) -> Self {
        BoundedEncodeError::Bounds(error)
    }
}

impl From<EncodeError> for BoundedEncodeError {
    fn from(error: EncodeError) -> Self {
        BoundedEncodeError::Encode(error)
    }
}

impl Display for BoundedEncodeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BoundedEncodeError::Bounds(error) => write!(f, "bounds: {error}"),
            BoundedEncodeError::Encode(error) => write!(f, "encode: {error}"),
        }
    }
}

impl core::error::Error for BoundedEncodeError {}

/// Leaf reader that charges every length to a [`CodecBudget`]. Byte leaves
/// never allocate more than the unread input; containers start empty.
struct BudgetedLeaves<'b, 'a> {
    budget: &'b mut CodecBudget<'a>,
}

impl<D> ValueLeafReader<D> for BudgetedLeaves<'_, '_>
where
    D: Decoder,
    D::R: RemainingInput,
{
    type Error = BoundedDecodeError;

    fn array_header(
        &mut self,
        decoder: &mut D,
        _depth: usize,
    ) -> Result<(usize, Vec<Value>), BoundedDecodeError> {
        self.budget.enter_container()?;
        let len = decode_bounded_len(decoder)?;
        self.budget.claim_elements(len as u64)?;
        Ok((len, Vec::new()))
    }

    fn map_header(
        &mut self,
        decoder: &mut D,
        _depth: usize,
    ) -> Result<(usize, ValueMap), BoundedDecodeError> {
        self.budget.enter_container()?;
        let len = decode_bounded_len(decoder)?;
        // A key and a value per entry.
        self.budget.claim_elements((len as u64).saturating_mul(2))?;
        Ok((len, Vec::new()))
    }

    fn container_end(&mut self) {
        self.budget.exit_container();
    }

    fn bytes(&mut self, decoder: &mut D) -> Result<Vec<u8>, BoundedDecodeError> {
        decode_bounded_bytes(decoder)
    }

    fn text(&mut self, decoder: &mut D) -> Result<String, BoundedDecodeError> {
        decode_bounded_string(decoder)
    }

    fn string_list(&mut self, decoder: &mut D) -> Result<Vec<String>, BoundedDecodeError> {
        decode_bounded_string_list(decoder, self.budget)
    }
}

/// One pending container while encoding without recursion.
enum EncodeFrame<'v> {
    Array(core::slice::Iter<'v, Value>),
    Map {
        entries: core::slice::Iter<'v, (Value, Value)>,
        /// The value of the entry whose key is currently being written. Keys
        /// are leaves in every canonical value, but a native value may carry a
        /// container key, so the value waits until the key's subtree closes.
        pending_value: Option<&'v Value>,
    },
}

/// Writes `value` with the same layout as the derived [`Encode`] impl, walking
/// containers with an explicit stack and charging depth and element counts to
/// `budget` at each container header.
fn encode_value_with<E: Encoder>(
    value: &Value,
    encoder: &mut E,
    budget: &mut CodecBudget<'_>,
) -> Result<(), BoundedEncodeError> {
    let mut frames = Vec::<EncodeFrame<'_>>::new();
    let mut next = Some(value);

    loop {
        if let Some(value) = next.take() {
            encode_leaf_or_push(value, encoder, budget, &mut frames)?;
            continue;
        }

        let Some(frame) = frames.last_mut() else {
            return Ok(());
        };
        match frame {
            EncodeFrame::Array(items) => match items.next() {
                Some(item) => next = Some(item),
                None => {
                    frames.pop();
                    budget.exit_container();
                }
            },
            EncodeFrame::Map {
                entries,
                pending_value,
            } => {
                if let Some(value) = pending_value.take() {
                    next = Some(value);
                } else if let Some((key, value)) = entries.next() {
                    *pending_value = Some(value);
                    next = Some(key);
                } else {
                    frames.pop();
                    budget.exit_container();
                }
            }
        }
    }
}

fn encode_leaf_or_push<'v, E: Encoder>(
    value: &'v Value,
    encoder: &mut E,
    budget: &mut CodecBudget<'_>,
    frames: &mut Vec<EncodeFrame<'v>>,
) -> Result<(), BoundedEncodeError> {
    match value {
        Value::Array(items) => {
            budget.enter_container()?;
            budget.claim_elements(items.len() as u64)?;
            VALUE_ARRAY_VARIANT.encode(encoder)?;
            (items.len() as u64).encode(encoder)?;
            frames.push(EncodeFrame::Array(items.iter()));
        }
        Value::Map(entries) => {
            budget.enter_container()?;
            budget.claim_elements((entries.len() as u64).saturating_mul(2))?;
            VALUE_MAP_VARIANT.encode(encoder)?;
            (entries.len() as u64).encode(encoder)?;
            frames.push(EncodeFrame::Map {
                entries: entries.iter(),
                pending_value: None,
            });
        }
        Value::EnumString(strings) => {
            budget.claim_elements(strings.len() as u64)?;
            value.encode(encoder)?;
        }
        leaf => leaf.encode(encoder)?,
    }
    Ok(())
}

impl Value {
    /// Decodes one value from exactly `bytes` under explicit `bounds`.
    ///
    /// Rejects input longer than `max_bytes` before reading, nesting deeper
    /// than `max_depth` and more container entries than `max_elements` at the
    /// container header, any declared length the unread input cannot contain
    /// before allocating, and trailing bytes after the value.
    ///
    /// Accepted inputs decode to the same value as the plain [`bincode::Decode`]
    /// impl with the native big-endian configuration.
    pub fn decode_bounded(bytes: &[u8], bounds: &CodecBounds) -> Result<Value, BoundedDecodeError> {
        bounded_decode_from_slice(bytes, bounds, |decoder, budget| {
            decode_value_with(decoder, &mut BudgetedLeaves { budget })
        })
    }

    /// Encodes the value with the canonical configuration under explicit
    /// `bounds`.
    ///
    /// Depth and element counts are checked during a size pass whose only
    /// allocation is the traversal stack (one frame per open container, so at
    /// most `max_depth` frames); the output is then refused if it would exceed
    /// `max_bytes`, and only afterwards is the output buffer allocated, sized
    /// exactly once. The bytes are identical to the derived
    /// [`bincode::Encode`] output.
    pub fn encode_bounded(&self, bounds: &CodecBounds) -> Result<Vec<u8>, BoundedEncodeError> {
        let size = {
            let mut budget = CodecBudget::new(bounds);
            let mut sizer = EncoderImpl::new(SizeWriter::default(), canonical_config());
            encode_value_with(self, &mut sizer, &mut budget)?;
            sizer.into_writer().bytes_written
        };
        CodecBudget::new(bounds).check_len(size)?;

        let mut budget = CodecBudget::new(bounds);
        let mut encoder = EncoderImpl::new(ExactWriter::with_capacity(size), canonical_config());
        encode_value_with(self, &mut encoder, &mut budget)?;
        Ok(encoder.into_writer().bytes)
    }
}

/// A writer into a `Vec<u8>` sized once, up front, from the size pass.
struct ExactWriter {
    bytes: Vec<u8>,
}

impl ExactWriter {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(capacity),
        }
    }
}

impl Writer for ExactWriter {
    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_value;
    use bincode::error::DecodeError;

    const BOUNDS: CodecBounds = CodecBounds {
        max_bytes: 128,
        max_depth: 4,
        max_elements: 16,
    };

    fn native_encode(value: &Value) -> Vec<u8> {
        bincode::encode_to_vec(value, canonical_config()).expect("native encode")
    }

    fn native_decode(bytes: &[u8]) -> Value {
        let (value, consumed): (Value, usize) =
            bincode::decode_from_slice(bytes, canonical_config()).expect("native decode");
        assert_eq!(consumed, bytes.len());
        value
    }

    fn sample_values() -> Vec<Value> {
        vec![
            Value::Null,
            Value::Bool(true),
            Value::U8(250),
            Value::I8(-128),
            Value::U16(251),
            Value::U32(70_000),
            Value::U64(u64::MAX),
            Value::I64(i64::MIN),
            Value::U128(u128::MAX),
            Value::I128(i128::MIN),
            Value::Float(-0.0),
            Value::Bytes(vec![]),
            Value::Bytes(vec![1, 2, 3]),
            Value::Bytes20([7; 20]),
            Value::Bytes32([8; 32]),
            Value::Bytes36([9; 36]),
            Value::EnumU8(vec![1, 2]),
            Value::EnumString(vec!["a".into(), "bc".into()]),
            Value::Identifier([3; 32]),
            Value::Text("héllo".into()),
            Value::Array(vec![]),
            Value::Map(vec![]),
            platform_value!({ "a": [1, { "b": null }], "c": "d" }),
            Value::Array(vec![Value::Array(vec![Value::Array(vec![Value::Null])])]),
        ]
    }

    #[test]
    fn should_encode_identically_to_the_derived_encode_impl() {
        for value in sample_values() {
            let bounded = value.encode_bounded(&BOUNDS).expect("within bounds");
            assert_eq!(bounded, native_encode(&value), "{value:?}");
        }
    }

    #[test]
    fn should_decode_identically_to_the_native_decode_impl() {
        for value in sample_values() {
            let bytes = native_encode(&value);
            let bounded = Value::decode_bounded(&bytes, &BOUNDS).expect("within bounds");
            assert_eq!(bounded, native_decode(&bytes));
            assert_eq!(bounded, value);
        }
    }

    #[test]
    fn should_accept_depth_exactly_at_the_bound_and_reject_one_more() {
        let mut value = Value::Null;
        for _ in 0..4 {
            value = Value::Array(vec![value]);
        }
        let bytes = native_encode(&value);
        assert_eq!(Value::decode_bounded(&bytes, &BOUNDS).unwrap(), value);
        assert_eq!(value.encode_bounded(&BOUNDS).unwrap(), bytes);

        let deeper = Value::Array(vec![value]);
        let bytes = native_encode(&deeper);
        assert!(matches!(
            Value::decode_bounded(&bytes, &BOUNDS),
            Err(BoundedDecodeError::Bounds(BoundsError::DepthExceeded {
                depth: 5,
                max: 4
            }))
        ));
        assert!(matches!(
            deeper.encode_bounded(&BOUNDS),
            Err(BoundedEncodeError::Bounds(BoundsError::DepthExceeded {
                depth: 5,
                max: 4
            }))
        ));
    }

    #[test]
    fn should_count_map_entries_twice_and_reject_one_element_over_the_bound() {
        // 8 entries = 16 elements, exactly the bound.
        let entries: ValueMap = (0..8).map(|i| (Value::U8(i), Value::Null)).collect();
        let value = Value::Map(entries.clone());
        let bytes = native_encode(&value);
        assert_eq!(Value::decode_bounded(&bytes, &BOUNDS).unwrap(), value);

        let mut over = entries;
        over.push((Value::U8(8), Value::Null));
        let over = Value::Map(over);
        let bytes = native_encode(&over);
        assert!(matches!(
            Value::decode_bounded(&bytes, &BOUNDS),
            Err(BoundedDecodeError::Bounds(BoundsError::ElementsExceeded {
                elements: 18,
                max: 16
            }))
        ));
        assert!(matches!(
            over.encode_bounded(&BOUNDS),
            Err(BoundedEncodeError::Bounds(BoundsError::ElementsExceeded {
                elements: 18,
                max: 16
            }))
        ));
    }

    #[test]
    fn should_charge_string_list_entries_to_the_element_budget() {
        let value = Value::Array(vec![Value::EnumString(vec![String::new(); 16])]);
        let bytes = native_encode(&value);
        assert!(matches!(
            Value::decode_bounded(&bytes, &BOUNDS),
            Err(BoundedDecodeError::Bounds(BoundsError::ElementsExceeded {
                elements: 17,
                max: 16
            }))
        ));
        assert!(matches!(
            value.encode_bounded(&BOUNDS),
            Err(BoundedEncodeError::Bounds(BoundsError::ElementsExceeded {
                elements: 17,
                max: 16
            }))
        ));
    }

    #[test]
    fn should_reject_output_over_max_bytes_without_allocating_it() {
        let value = Value::Bytes(vec![0; 200]);
        assert!(matches!(
            value.encode_bounded(&BOUNDS),
            Err(BoundedEncodeError::Bounds(BoundsError::BytesExceeded {
                len: 202,
                max: 128
            }))
        ));
        let bytes = native_encode(&value);
        assert!(matches!(
            Value::decode_bounded(&bytes, &BOUNDS),
            Err(BoundedDecodeError::Bounds(BoundsError::BytesExceeded {
                len: 202,
                max: 128
            }))
        ));
    }

    #[test]
    fn should_reject_declared_array_length_beyond_the_input() {
        // Array header claiming 100 items, then nothing.
        let bytes = [21u8, 100];
        assert!(matches!(
            Value::decode_bounded(&bytes, &BOUNDS),
            Err(BoundedDecodeError::Bounds(
                BoundsError::DeclaredLengthExceedsInput {
                    declared: 100,
                    remaining: 0
                }
            ))
        ));
    }

    #[test]
    fn should_reject_declared_map_length_beyond_the_input() {
        let bytes = [22u8, 3, 20, 20];
        assert!(matches!(
            Value::decode_bounded(&bytes, &BOUNDS),
            Err(BoundedDecodeError::Bounds(
                BoundsError::DeclaredLengthExceedsInput {
                    declared: 3,
                    remaining: 2
                }
            ))
        ));
    }

    #[test]
    fn should_reject_byte_leaves_declaring_beyond_the_input() {
        // Bytes claiming u64::MAX with no payload.
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
        // EnumU8 claiming 2^32 with three payload bytes.
        let bytes = [14u8, 253, 0, 0, 0, 1, 0, 0, 0, 0, 1, 2, 3];
        assert!(matches!(
            Value::decode_bounded(&bytes, &BOUNDS),
            Err(BoundedDecodeError::Bounds(
                BoundsError::DeclaredLengthExceedsInput {
                    declared: 4_294_967_296,
                    remaining: 3
                }
            ))
        ));
        // Text claiming 5 with 4 bytes present.
        let bytes = [18u8, 5, b'a', b'b', b'c', b'd'];
        assert!(matches!(
            Value::decode_bounded(&bytes, &BOUNDS),
            Err(BoundedDecodeError::Bounds(
                BoundsError::DeclaredLengthExceedsInput {
                    declared: 5,
                    remaining: 4
                }
            ))
        ));
    }

    #[test]
    fn should_reject_string_list_counts_and_inner_lengths_beyond_the_input() {
        let bytes = [15u8, 9, 0, 0];
        assert!(matches!(
            Value::decode_bounded(&bytes, &BOUNDS),
            Err(BoundedDecodeError::Bounds(
                BoundsError::DeclaredLengthExceedsInput {
                    declared: 9,
                    remaining: 2
                }
            ))
        ));
        let bytes = [15u8, 1, 7, b'x'];
        assert!(matches!(
            Value::decode_bounded(&bytes, &BOUNDS),
            Err(BoundedDecodeError::Bounds(
                BoundsError::DeclaredLengthExceedsInput {
                    declared: 7,
                    remaining: 1
                }
            ))
        ));
    }

    #[test]
    fn should_reject_trailing_bytes_and_unknown_variants() {
        let mut bytes = native_encode(&Value::Null);
        bytes.push(0);
        assert!(matches!(
            Value::decode_bounded(&bytes, &BOUNDS),
            Err(BoundedDecodeError::TrailingBytes { remaining: 1 })
        ));
        assert!(matches!(
            Value::decode_bounded(&[23u8], &BOUNDS),
            Err(BoundedDecodeError::Decode(DecodeError::UnexpectedVariant {
                found: 23,
                ..
            }))
        ));
    }

    #[test]
    fn should_accept_overlong_varints_so_canonicality_needs_a_re_encode_check() {
        // `U16(0)` as the minimal bytes and as two overlong forms: one on the
        // payload, one on the variant index. All decode; only the minimal form
        // survives a re-encode comparison, which is the ABI layer's job.
        let minimal = [6u8, 0];
        let overlong_payload = [6u8, 251, 0, 0];
        let overlong_variant = [251u8, 0, 6, 0];
        for bytes in [&minimal[..], &overlong_payload[..], &overlong_variant[..]] {
            assert_eq!(
                Value::decode_bounded(bytes, &BOUNDS).unwrap(),
                Value::U16(0)
            );
        }
        assert_eq!(Value::U16(0).encode_bounded(&BOUNDS).unwrap(), minimal);
    }

    #[test]
    fn should_reject_invalid_utf8_text_after_the_bounded_read() {
        let bytes = [18u8, 2, 0xff, 0xfe];
        assert!(matches!(
            Value::decode_bounded(&bytes, &BOUNDS),
            Err(BoundedDecodeError::Decode(DecodeError::Utf8 { .. }))
        ));
    }

    #[test]
    fn should_encode_container_map_keys_like_the_derived_impl() {
        let value = Value::Map(vec![
            (
                Value::Array(vec![Value::U8(1)]),
                Value::Map(vec![(Value::Null, Value::Null)]),
            ),
            (Value::Text("k".into()), Value::Array(vec![])),
        ]);
        let bytes = native_encode(&value);
        assert_eq!(value.encode_bounded(&BOUNDS).unwrap(), bytes);
        assert_eq!(Value::decode_bounded(&bytes, &BOUNDS).unwrap(), value);
    }

    #[test]
    fn should_release_depth_when_a_container_closes() {
        // Two sibling containers at depth 4 must both be accepted: depth is
        // released on close, not accumulated.
        let leaf = Value::Array(vec![Value::Array(vec![Value::Array(vec![Value::Null])])]);
        let value = Value::Array(vec![leaf.clone(), leaf]);
        let bytes = native_encode(&value);
        assert_eq!(Value::decode_bounded(&bytes, &BOUNDS).unwrap(), value);
        assert_eq!(value.encode_bounded(&BOUNDS).unwrap(), bytes);
    }
}
