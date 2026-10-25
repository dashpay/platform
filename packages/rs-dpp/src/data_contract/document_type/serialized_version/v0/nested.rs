//! The decoding bound of the stored form's recursive values: nested properties, typed array
//! elements, rules and validation schema nodes. Bytes decoded with the untrusted decoder can
//! come from anyone, so each level of nesting counts against [`MAX_STORED_DECODE_DEPTH`] and
//! decoding refuses bytes nested deeper, before the stack runs out.

use bincode::de::{BorrowDecoder, BorrowUntrustedDecoder, Decoder, UntrustedDecoder};
use bincode::error::DecodeError;
use bincode::{BorrowDecode, BorrowDecodeUntrusted, Decode, DecodeUntrusted, Encode};
use std::cell::Cell;

/// The deepest nesting a stored document type may hold, across all its recursive values. A
/// schema registration admits is at most `max_depth` (256) values deep, and every nested stored
/// value sits at least one schema value deeper than the one holding it.
pub(super) const MAX_STORED_DECODE_DEPTH: usize = 256;

std::thread_local! {
    /// How many nested stored values this thread is decoding inside one another.
    static STORED_DECODE_DEPTH: Cell<usize> = const { Cell::new(0) };
}

/// A value of the stored form that can hold values of its own kind: decoding it counts one
/// level of nesting.
#[derive(Debug, Clone, PartialEq, Eq, Encode)]
pub(super) struct Nested<T>(pub(super) T);

/// Runs `decode` one level deeper than the value around it, refusing it past
/// [`MAX_STORED_DECODE_DEPTH`]. The depth is restored however `decode` ends.
fn decode_nested<T>(
    decode: impl FnOnce() -> Result<T, DecodeError>,
) -> Result<Nested<T>, DecodeError> {
    struct Leave;

    impl Drop for Leave {
        fn drop(&mut self) {
            STORED_DECODE_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
        }
    }

    let depth = STORED_DECODE_DEPTH.with(|depth| depth.get()) + 1;
    if depth > MAX_STORED_DECODE_DEPTH {
        return Err(DecodeError::OtherString(format!(
            "stored document type nesting depth {depth} exceeds the maximum of \
             {MAX_STORED_DECODE_DEPTH}"
        )));
    }
    STORED_DECODE_DEPTH.with(|current| current.set(depth));
    let _leave = Leave;
    decode().map(Nested)
}

impl<Context, T: Decode<Context>> Decode<Context> for Nested<T> {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        decode_nested(|| T::decode(decoder))
    }
}

impl<'de, Context, T: BorrowDecode<'de, Context>> BorrowDecode<'de, Context> for Nested<T> {
    fn borrow_decode<D: BorrowDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        decode_nested(|| T::borrow_decode(decoder))
    }
}

impl<Context, T: DecodeUntrusted<Context>> DecodeUntrusted<Context> for Nested<T> {
    fn decode_untrusted<D: UntrustedDecoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        decode_nested(|| T::decode_untrusted(decoder))
    }
}

impl<'de, Context, T: BorrowDecodeUntrusted<'de, Context>> BorrowDecodeUntrusted<'de, Context>
    for Nested<T>
{
    fn borrow_decode_untrusted<D: BorrowUntrustedDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        decode_nested(|| T::borrow_decode_untrusted(decoder))
    }
}
