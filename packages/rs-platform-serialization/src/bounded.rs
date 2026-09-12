//! Explicit bounds for allocation-only decoding.
//!
//! The native decoders in this workspace rely on ambient state (a thread-local
//! nesting limit, bincode byte budgets picked per type) and on the operating
//! system for entropy and I/O. Guest code that runs inside a DashVM sandbox has
//! none of that, and it must never allocate more than the caller allowed. This
//! module is the seam both profiles share: a caller supplies one
//! [`CodecBounds`], a [`CodecBudget`] counts against it while a value is
//! decoded, and every variable-length leaf is read through a helper that checks
//! the declared length against the unread input **before** allocating.
//!
//! The invariant the helpers uphold is simple to state: no allocation on the
//! bounded path is ever larger than the bytes still unread. Since the input
//! itself is at most `max_bytes`, total allocation is bounded by the caller.
//!
//! Nothing here depends on `std` or on the native `PlatformVersion` registry.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use bincode::config::{BigEndian, Configuration, NoLimit, Varint};
use bincode::de::read::{BorrowReader, Reader};
use bincode::de::{Decoder, DecoderImpl};
use bincode::error::DecodeError;
use bincode::Decode;
use core::fmt::{self, Display, Formatter};

use crate::BincodeContext;

/// Explicit limits for one bounded decode or encode.
///
/// Supplied by the caller and never read from ambient state. Host code derives
/// the numbers from its protocol-version tables; this crate only enforces them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CodecBounds {
    /// Maximum size of the encoded payload in bytes.
    pub max_bytes: u32,
    /// Maximum container nesting depth. The outermost array or map is depth 1.
    pub max_depth: u16,
    /// Maximum number of container entries across the whole value: one per
    /// array item, two per map entry (key and value), one per string in an
    /// enumeration of strings.
    pub max_elements: u32,
}

/// Why a bounded operation was refused.
///
/// Every field is a fixed-width integer, never `usize`: these errors are mapped
/// into ABI errors that become wire-visible later, so their layout must not
/// depend on the target word size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundsError {
    /// The payload is larger than `max_bytes`.
    BytesExceeded {
        /// Actual payload length.
        len: u64,
        /// The configured maximum.
        max: u32,
    },
    /// A container would nest deeper than `max_depth`.
    DepthExceeded {
        /// The depth that was about to be entered.
        depth: u16,
        /// The configured maximum.
        max: u16,
    },
    /// The running element count would exceed `max_elements`.
    ElementsExceeded {
        /// The count after the refused claim.
        elements: u64,
        /// The configured maximum.
        max: u32,
    },
    /// A length prefix declares more entries or bytes than the unread input
    /// could possibly contain. Reported before any allocation.
    DeclaredLengthExceedsInput {
        /// The declared length.
        declared: u64,
        /// Bytes left unread when the length was seen.
        remaining: u64,
    },
}

impl Display for BoundsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BoundsError::BytesExceeded { len, max } => {
                write!(f, "payload of {len} bytes exceeds the maximum of {max}")
            }
            BoundsError::DepthExceeded { depth, max } => {
                write!(
                    f,
                    "value nesting depth {depth} exceeds the maximum of {max}"
                )
            }
            BoundsError::ElementsExceeded { elements, max } => {
                write!(
                    f,
                    "{elements} container elements exceed the maximum of {max}"
                )
            }
            BoundsError::DeclaredLengthExceedsInput {
                declared,
                remaining,
            } => write!(
                f,
                "declared length {declared} exceeds the {remaining} unread input bytes"
            ),
        }
    }
}

impl core::error::Error for BoundsError {}

/// Running counters against one [`CodecBounds`].
#[derive(Debug)]
pub struct CodecBudget<'a> {
    bounds: &'a CodecBounds,
    depth: u16,
    elements: u64,
}

impl<'a> CodecBudget<'a> {
    /// Starts counting from zero against `bounds`.
    pub const fn new(bounds: &'a CodecBounds) -> Self {
        Self {
            bounds,
            depth: 0,
            elements: 0,
        }
    }

    /// The bounds this budget counts against.
    pub const fn bounds(&self) -> &'a CodecBounds {
        self.bounds
    }

    /// The current container nesting depth.
    pub const fn depth(&self) -> u16 {
        self.depth
    }

    /// The number of container elements claimed so far.
    pub const fn elements(&self) -> u64 {
        self.elements
    }

    /// Rejects a payload longer than `max_bytes`. Checked once, before the
    /// first byte is read or written.
    pub fn check_len(&self, len: usize) -> Result<(), BoundsError> {
        let len = len as u64;
        if len > u64::from(self.bounds.max_bytes) {
            return Err(BoundsError::BytesExceeded {
                len,
                max: self.bounds.max_bytes,
            });
        }
        Ok(())
    }

    /// Records entering an array or map. Fails when the new depth would exceed
    /// `max_depth`.
    pub fn enter_container(&mut self) -> Result<(), BoundsError> {
        let depth = self.depth.saturating_add(1);
        if depth > self.bounds.max_depth {
            return Err(BoundsError::DepthExceeded {
                depth,
                max: self.bounds.max_depth,
            });
        }
        self.depth = depth;
        Ok(())
    }

    /// Records leaving the innermost array or map.
    pub fn exit_container(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// Adds `count` container elements to the running total. Fails when the
    /// total would exceed `max_elements`; the addition itself cannot overflow
    /// because it saturates.
    pub fn claim_elements(&mut self, count: u64) -> Result<(), BoundsError> {
        let elements = self.elements.saturating_add(count);
        if elements > u64::from(self.bounds.max_elements) {
            return Err(BoundsError::ElementsExceeded {
                elements,
                max: self.bounds.max_elements,
            });
        }
        self.elements = elements;
        Ok(())
    }
}

/// Rejects a declared length that the unread input cannot contain.
///
/// Every entry of a container, every byte of a byte string and every string of
/// a string list occupies at least one input byte, so a declared length above
/// the unread remainder is malformed. Call this before any allocation sized by
/// a decoded length.
pub fn check_declared_len(declared: u64, remaining: usize) -> Result<(), BoundsError> {
    let remaining = remaining as u64;
    if declared > remaining {
        return Err(BoundsError::DeclaredLengthExceedsInput {
            declared,
            remaining,
        });
    }
    Ok(())
}

/// A reader that can report how much of its input is still unread.
///
/// The bounded leaf decoders need this to check declared lengths against the
/// remaining input before allocating.
pub trait RemainingInput {
    /// Bytes not yet consumed.
    fn remaining(&self) -> usize;
}

/// A bincode reader over a slice that exposes the unread remainder.
///
/// Behaves exactly like bincode's own slice reader; the only addition is
/// [`BoundedSliceReader::remaining`], which lets callers reject declared
/// lengths that cannot fit and detect trailing bytes.
#[derive(Debug)]
pub struct BoundedSliceReader<'a> {
    remaining: &'a [u8],
}

impl<'a> BoundedSliceReader<'a> {
    /// Wraps `bytes`.
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self { remaining: bytes }
    }

    /// Bytes not yet consumed.
    pub const fn remaining(&self) -> usize {
        self.remaining.len()
    }
}

impl RemainingInput for BoundedSliceReader<'_> {
    fn remaining(&self) -> usize {
        self.remaining.len()
    }
}

impl Reader for BoundedSliceReader<'_> {
    #[inline]
    fn read(&mut self, bytes: &mut [u8]) -> Result<(), DecodeError> {
        if bytes.len() > self.remaining.len() {
            return Err(DecodeError::UnexpectedEnd {
                additional: bytes.len() - self.remaining.len(),
            });
        }
        let (head, tail) = self.remaining.split_at(bytes.len());
        bytes.copy_from_slice(head);
        self.remaining = tail;
        Ok(())
    }

    #[inline]
    fn peek_read(&mut self, n: usize) -> Option<&[u8]> {
        self.remaining.get(..n)
    }

    #[inline]
    fn consume(&mut self, n: usize) {
        self.remaining = self.remaining.get(n..).unwrap_or_default();
    }
}

impl<'de> BorrowReader<'de> for BoundedSliceReader<'de> {
    #[inline]
    fn take_bytes(&mut self, length: usize) -> Result<&'de [u8], DecodeError> {
        if length > self.remaining.len() {
            return Err(DecodeError::UnexpectedEnd {
                additional: length - self.remaining.len(),
            });
        }
        let (head, tail) = self.remaining.split_at(length);
        self.remaining = tail;
        Ok(head)
    }
}

/// The bincode configuration used for canonical wire bytes: standard layout,
/// big endian, variable-length integers, no byte limit (the caller's
/// [`CodecBounds`] is the limit). Identical to the native platform config.
pub const fn canonical_config() -> Configuration<BigEndian, Varint, NoLimit> {
    bincode::config::standard()
        .with_big_endian()
        .with_no_limit()
}

/// The decoder type used by [`bounded_decode_from_slice`].
pub type BoundedDecoder<'a> =
    DecoderImpl<BoundedSliceReader<'a>, Configuration<BigEndian, Varint, NoLimit>, BincodeContext>;

/// Failure of a bounded decode.
#[derive(Debug)]
pub enum BoundedDecodeError {
    /// A bound from [`CodecBounds`] was hit.
    Bounds(BoundsError),
    /// The bytes are not a valid encoding.
    Decode(DecodeError),
    /// The value ended before the input did.
    TrailingBytes {
        /// Bytes left after the value.
        remaining: u64,
    },
}

impl From<BoundsError> for BoundedDecodeError {
    fn from(error: BoundsError) -> Self {
        BoundedDecodeError::Bounds(error)
    }
}

impl From<DecodeError> for BoundedDecodeError {
    fn from(error: DecodeError) -> Self {
        BoundedDecodeError::Decode(error)
    }
}

impl Display for BoundedDecodeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BoundedDecodeError::Bounds(error) => write!(f, "bounds: {error}"),
            BoundedDecodeError::Decode(error) => write!(f, "decode: {error}"),
            BoundedDecodeError::TrailingBytes { remaining } => {
                write!(f, "{remaining} trailing bytes after the value")
            }
        }
    }
}

impl core::error::Error for BoundedDecodeError {}

/// Reads a bincode length prefix and returns it once the unread input could
/// contain that many entries. No allocation happens here.
pub fn decode_bounded_len<D>(decoder: &mut D) -> Result<usize, BoundedDecodeError>
where
    D: Decoder,
    D::R: RemainingInput,
{
    let declared = <u64 as Decode<D::Context>>::decode(decoder)?;
    check_declared_len(declared, decoder.reader().remaining())?;
    // `declared` is at most the unread length, which is a `usize`, so this
    // conversion only fails on a platform bincode itself does not support.
    usize::try_from(declared).map_err(|_| DecodeError::OutsideUsizeRange(declared).into())
}

/// Reads a length-prefixed byte string, allocating exactly the declared length
/// and only after that length has been checked against the unread input.
///
/// bincode's own `Vec<u8>` decoder allocates from the declared length before
/// reading the payload, so it is never used on the bounded path.
pub fn decode_bounded_bytes<D>(decoder: &mut D) -> Result<Vec<u8>, BoundedDecodeError>
where
    D: Decoder,
    D::R: RemainingInput,
{
    let len = decode_bounded_len(decoder)?;
    decoder.claim_bytes_read(len)?;
    let mut bytes = vec![0u8; len];
    decoder.reader().read(&mut bytes)?;
    Ok(bytes)
}

/// Reads a length-prefixed UTF-8 string through [`decode_bounded_bytes`] and
/// validates the encoding after the bounded read.
pub fn decode_bounded_string<D>(decoder: &mut D) -> Result<String, BoundedDecodeError>
where
    D: Decoder,
    D::R: RemainingInput,
{
    let bytes = decode_bounded_bytes(decoder)?;
    String::from_utf8(bytes).map_err(|error| {
        DecodeError::Utf8 {
            inner: error.utf8_error(),
        }
        .into()
    })
}

/// Reads a length-prefixed list of strings. The count is charged to the
/// element budget and checked against the unread input (each entry needs at
/// least its own length byte) before the list starts; the list then grows by
/// push and each string goes through [`decode_bounded_string`].
pub fn decode_bounded_string_list<D>(
    decoder: &mut D,
    budget: &mut CodecBudget<'_>,
) -> Result<Vec<String>, BoundedDecodeError>
where
    D: Decoder,
    D::R: RemainingInput,
{
    let count = decode_bounded_len(decoder)?;
    budget.claim_elements(count as u64)?;
    let mut strings = Vec::new();
    for _ in 0..count {
        strings.push(decode_bounded_string(decoder)?);
    }
    Ok(strings)
}

/// Runs `decode` over exactly `bytes` under `bounds`.
///
/// Checks the input length first, hands the closure a decoder over the whole
/// input plus a fresh [`CodecBudget`], and finally requires that every byte was
/// consumed: trailing bytes are an error, so one value has exactly one
/// accepted encoding of a given length.
pub fn bounded_decode_from_slice<'a, T, F>(
    bytes: &'a [u8],
    bounds: &CodecBounds,
    decode: F,
) -> Result<T, BoundedDecodeError>
where
    F: FnOnce(&mut BoundedDecoder<'a>, &mut CodecBudget<'_>) -> Result<T, BoundedDecodeError>,
{
    let mut budget = CodecBudget::new(bounds);
    budget.check_len(bytes.len())?;
    let mut decoder = DecoderImpl::new(BoundedSliceReader::new(bytes), canonical_config(), ());
    let value = decode(&mut decoder, &mut budget)?;
    let remaining = decoder.reader().remaining();
    if remaining != 0 {
        return Err(BoundedDecodeError::TrailingBytes {
            remaining: remaining as u64,
        });
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode::Encode;

    const BOUNDS: CodecBounds = CodecBounds {
        max_bytes: 64,
        max_depth: 3,
        max_elements: 8,
    };

    fn encode<T: Encode>(value: T) -> Vec<u8> {
        bincode::encode_to_vec(value, canonical_config()).expect("encoding cannot fail")
    }

    fn decode_all<T, F>(bytes: &[u8], decode: F) -> Result<T, BoundedDecodeError>
    where
        F: FnOnce(&mut BoundedDecoder<'_>, &mut CodecBudget<'_>) -> Result<T, BoundedDecodeError>,
    {
        bounded_decode_from_slice(bytes, &BOUNDS, decode)
    }

    #[test]
    fn should_accept_input_exactly_at_the_byte_bound() {
        let budget = CodecBudget::new(&BOUNDS);
        assert_eq!(budget.check_len(64), Ok(()));
    }

    #[test]
    fn should_reject_input_one_byte_over_the_bound() {
        let budget = CodecBudget::new(&BOUNDS);
        assert_eq!(
            budget.check_len(65),
            Err(BoundsError::BytesExceeded { len: 65, max: 64 })
        );
    }

    #[test]
    fn should_accept_depth_exactly_at_the_bound_and_reject_one_more() {
        let mut budget = CodecBudget::new(&BOUNDS);
        for _ in 0..3 {
            assert_eq!(budget.enter_container(), Ok(()));
        }
        assert_eq!(budget.depth(), 3);
        assert_eq!(
            budget.enter_container(),
            Err(BoundsError::DepthExceeded { depth: 4, max: 3 })
        );
        budget.exit_container();
        assert_eq!(budget.enter_container(), Ok(()));
    }

    #[test]
    fn should_not_underflow_depth_on_extra_exit() {
        let mut budget = CodecBudget::new(&BOUNDS);
        budget.exit_container();
        assert_eq!(budget.depth(), 0);
    }

    #[test]
    fn should_accept_elements_exactly_at_the_bound_and_reject_one_more() {
        let mut budget = CodecBudget::new(&BOUNDS);
        assert_eq!(budget.claim_elements(5), Ok(()));
        assert_eq!(budget.claim_elements(3), Ok(()));
        assert_eq!(budget.elements(), 8);
        assert_eq!(
            budget.claim_elements(1),
            Err(BoundsError::ElementsExceeded {
                elements: 9,
                max: 8
            })
        );
    }

    #[test]
    fn should_reject_element_claims_that_would_overflow_the_counter() {
        let mut budget = CodecBudget::new(&BOUNDS);
        assert_eq!(budget.claim_elements(4), Ok(()));
        assert_eq!(
            budget.claim_elements(u64::MAX),
            Err(BoundsError::ElementsExceeded {
                elements: u64::MAX,
                max: 8
            })
        );
    }

    #[test]
    fn should_accept_declared_length_equal_to_remaining_and_reject_one_more() {
        assert_eq!(check_declared_len(10, 10), Ok(()));
        assert_eq!(
            check_declared_len(11, 10),
            Err(BoundsError::DeclaredLengthExceedsInput {
                declared: 11,
                remaining: 10
            })
        );
    }

    #[test]
    fn should_read_bytes_whose_declared_length_equals_the_remaining_input() {
        let bytes = encode(vec![7u8, 8, 9]);
        let decoded = decode_all(&bytes, |decoder, _| decode_bounded_bytes(decoder)).unwrap();
        assert_eq!(decoded, vec![7, 8, 9]);
    }

    #[test]
    fn should_reject_bytes_declaring_one_more_than_the_remaining_input() {
        let mut bytes = encode(vec![7u8, 8, 9]);
        bytes.truncate(bytes.len() - 1);
        match decode_all(&bytes, |decoder, _| decode_bounded_bytes(decoder)) {
            Err(BoundedDecodeError::Bounds(BoundsError::DeclaredLengthExceedsInput {
                declared: 3,
                remaining: 2,
            })) => {}
            other => panic!("expected declared length rejection, got {other:?}"),
        }
    }

    #[test]
    fn should_reject_bytes_declaring_u64_max_without_allocating() {
        let bytes = encode(u64::MAX);
        match decode_all(&bytes, |decoder, _| decode_bounded_bytes(decoder)) {
            Err(BoundedDecodeError::Bounds(BoundsError::DeclaredLengthExceedsInput {
                declared: u64::MAX,
                remaining: 0,
            })) => {}
            other => panic!("expected declared length rejection, got {other:?}"),
        }
    }

    #[test]
    fn should_validate_utf8_after_the_bounded_read() {
        let bytes = encode(vec![0xffu8, 0xfe]);
        match decode_all(&bytes, |decoder, _| decode_bounded_string(decoder)) {
            Err(BoundedDecodeError::Decode(DecodeError::Utf8 { .. })) => {}
            other => panic!("expected utf8 rejection, got {other:?}"),
        }
        let bytes = encode("héllo");
        let decoded = decode_all(&bytes, |decoder, _| decode_bounded_string(decoder)).unwrap();
        assert_eq!(decoded, "héllo");
    }

    #[test]
    fn should_read_a_string_list_and_charge_its_count_to_the_element_budget() {
        let bytes = encode(vec!["a", "bc", ""]);
        let (strings, elements) = decode_all(&bytes, |decoder, budget| {
            let strings = decode_bounded_string_list(decoder, budget)?;
            Ok((strings, budget.elements()))
        })
        .unwrap();
        assert_eq!(strings, vec!["a", "bc", ""]);
        assert_eq!(elements, 3);
    }

    #[test]
    fn should_reject_a_string_list_count_beyond_the_remaining_input() {
        // Count 5 with only two bytes of payload after the prefix.
        let bytes = [5u8, 0, 0];
        match decode_all(&bytes, |decoder, budget| {
            decode_bounded_string_list(decoder, budget)
        }) {
            Err(BoundedDecodeError::Bounds(BoundsError::DeclaredLengthExceedsInput {
                declared: 5,
                remaining: 2,
            })) => {}
            other => panic!("expected declared length rejection, got {other:?}"),
        }
    }

    #[test]
    fn should_reject_a_string_list_count_beyond_the_element_budget() {
        let bytes = encode(vec![""; 9]);
        match decode_all(&bytes, |decoder, budget| {
            decode_bounded_string_list(decoder, budget)
        }) {
            Err(BoundedDecodeError::Bounds(BoundsError::ElementsExceeded {
                elements: 9,
                max: 8,
            })) => {}
            other => panic!("expected element budget rejection, got {other:?}"),
        }
    }

    #[test]
    fn should_reject_an_inner_string_declaring_beyond_the_remaining_input() {
        // Count 1, then a string claiming 200 bytes with one byte present.
        let bytes = [1u8, 200, 0];
        match decode_all(&bytes, |decoder, budget| {
            decode_bounded_string_list(decoder, budget)
        }) {
            Err(BoundedDecodeError::Bounds(BoundsError::DeclaredLengthExceedsInput {
                declared: 200,
                remaining: 1,
            })) => {}
            other => panic!("expected declared length rejection, got {other:?}"),
        }
    }

    #[test]
    fn should_reject_trailing_bytes() {
        let mut bytes = encode(42u32);
        bytes.push(0);
        match decode_all(&bytes, |decoder, _| {
            <u32 as Decode<()>>::decode(decoder).map_err(Into::into)
        }) {
            Err(BoundedDecodeError::TrailingBytes { remaining: 1 }) => {}
            other => panic!("expected trailing byte rejection, got {other:?}"),
        }
    }

    #[test]
    fn should_reject_input_longer_than_max_bytes_before_decoding() {
        let bytes = vec![0u8; 65];
        match decode_all::<(), _>(&bytes, |_, _| panic!("the closure must not run")) {
            Err(BoundedDecodeError::Bounds(BoundsError::BytesExceeded { len: 65, max: 64 })) => {}
            other => panic!("expected byte bound rejection, got {other:?}"),
        }
    }

    #[test]
    fn should_expose_the_unread_remainder_through_the_reader() {
        let mut reader = BoundedSliceReader::new(&[1, 2, 3, 4]);
        assert_eq!(reader.remaining(), 4);
        let mut two = [0u8; 2];
        reader.read(&mut two).unwrap();
        assert_eq!(two, [1, 2]);
        assert_eq!(reader.remaining(), 2);
        assert_eq!(reader.peek_read(1), Some(&[3][..]));
        reader.consume(1);
        assert_eq!(reader.take_bytes(1).unwrap(), &[4]);
        assert_eq!(reader.remaining(), 0);
        assert!(matches!(
            reader.read(&mut two),
            Err(DecodeError::UnexpectedEnd { additional: 2 })
        ));
        assert!(matches!(
            reader.take_bytes(1),
            Err(DecodeError::UnexpectedEnd { additional: 1 })
        ));
    }

    #[test]
    fn should_use_the_native_big_endian_varint_configuration() {
        let native = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        assert_eq!(
            bincode::encode_to_vec(300u16, canonical_config()).unwrap(),
            bincode::encode_to_vec(300u16, native).unwrap()
        );
    }
}
