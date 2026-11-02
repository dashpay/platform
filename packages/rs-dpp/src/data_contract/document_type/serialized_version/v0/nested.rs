//! The decoding bounds of the stored form's recursive values. Bytes decoded with the untrusted
//! decoder can come from anyone, so each level of nesting counts against a bound and decoding
//! refuses bytes nested deeper, before the stack runs out: nested properties, typed array
//! elements and validation schema nodes against [`MAX_STORED_DECODE_DEPTH`] ([`Nested`]), and
//! rules and the expressions under them against the rule grammar's own bound
//! ([`RuleNested`]).
//!
//! Neither wrapper changes the bytes: each encodes as the value it holds.

use crate::data_contract::document_type::property_constraints::MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH;
use bincode::de::{BorrowDecoder, BorrowUntrustedDecoder, Decoder, UntrustedDecoder};
use bincode::error::DecodeError;
use bincode::{BorrowDecode, BorrowDecodeUntrusted, Decode, DecodeUntrusted, Encode};
use std::cell::Cell;
use std::thread::LocalKey;

/// The deepest nesting of properties, typed array elements and validation schema nodes a stored
/// document type may hold, across all of them. A schema registration admits is at most
/// `max_depth` values deep, and each of these sits at least one schema value deeper than the one
/// holding it (a property under `properties`, an element under `items`, a schema node under
/// `properties`, `items` or `contains`), so a registrable schema stays within the bound while
/// `max_depth` does (a test holds every protocol version to it).
pub(super) const MAX_STORED_DECODE_DEPTH: usize = 256;

/// The deepest an edge from a rule or an expression to one of its own kind may sit: the rule
/// parser's bound, [`MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH`]. The parser places each condition and
/// each operand one level deeper than what holds it, and each stored edge is one of those levels
/// at most, so every rule a parse admits decodes, and nothing deeper than a parse admits does.
pub(super) const MAX_STORED_RULE_DECODE_DEPTH: usize = MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH;

std::thread_local! {
    /// How many nested properties, elements and schema nodes this thread is decoding inside one
    /// another.
    static STORED_DECODE_DEPTH: Cell<usize> = const { Cell::new(0) };
    /// How many nested rules and expressions this thread is decoding inside one another.
    static STORED_RULE_DECODE_DEPTH: Cell<usize> = const { Cell::new(0) };
}

/// A property, element or schema node of the stored form that can hold values of its own kind:
/// decoding it counts one level against [`MAX_STORED_DECODE_DEPTH`].
#[derive(Debug, Clone, PartialEq, Eq, Encode)]
pub(super) struct Nested<T>(pub(super) T);

/// A rule or an expression of the stored form that can hold values of its own kind: decoding it
/// counts one level against [`MAX_STORED_RULE_DECODE_DEPTH`].
#[derive(Debug, Clone, PartialEq, Eq, Encode)]
pub(super) struct RuleNested<T>(pub(super) T);

/// Runs `decode` one level deeper on `counter` than the value around it, refusing it past
/// `bound`. The depth is restored however `decode` ends.
fn decode_nested<T>(
    counter: &'static LocalKey<Cell<usize>>,
    bound: usize,
    what: &str,
    decode: impl FnOnce() -> Result<T, DecodeError>,
) -> Result<T, DecodeError> {
    struct Leave(&'static LocalKey<Cell<usize>>);

    impl Drop for Leave {
        fn drop(&mut self) {
            self.0
                .with(|depth| depth.set(depth.get().saturating_sub(1)));
        }
    }

    let depth = counter.with(|depth| depth.get()) + 1;
    if depth > bound {
        return Err(DecodeError::OtherString(format!(
            "stored document type {what} nesting depth {depth} exceeds the maximum of {bound}"
        )));
    }
    counter.with(|current| current.set(depth));
    let _leave = Leave(counter);
    decode()
}

/// Implements the four decoders for a nesting wrapper, each counting one level on `$counter`
/// against `$bound`.
macro_rules! nesting_decoders {
    ($wrapper:ident, $counter:expr, $bound:expr, $what:expr) => {
        impl<Context, T: Decode<Context>> Decode<Context> for $wrapper<T> {
            fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
                decode_nested(&$counter, $bound, $what, || T::decode(decoder)).map($wrapper)
            }
        }

        impl<'de, Context, T: BorrowDecode<'de, Context>> BorrowDecode<'de, Context>
            for $wrapper<T>
        {
            fn borrow_decode<D: BorrowDecoder<'de, Context = Context>>(
                decoder: &mut D,
            ) -> Result<Self, DecodeError> {
                decode_nested(&$counter, $bound, $what, || T::borrow_decode(decoder)).map($wrapper)
            }
        }

        impl<Context, T: DecodeUntrusted<Context>> DecodeUntrusted<Context> for $wrapper<T> {
            fn decode_untrusted<D: UntrustedDecoder<Context = Context>>(
                decoder: &mut D,
            ) -> Result<Self, DecodeError> {
                decode_nested(&$counter, $bound, $what, || T::decode_untrusted(decoder))
                    .map($wrapper)
            }
        }

        impl<'de, Context, T: BorrowDecodeUntrusted<'de, Context>>
            BorrowDecodeUntrusted<'de, Context> for $wrapper<T>
        {
            fn borrow_decode_untrusted<D: BorrowUntrustedDecoder<'de, Context = Context>>(
                decoder: &mut D,
            ) -> Result<Self, DecodeError> {
                decode_nested(&$counter, $bound, $what, || {
                    T::borrow_decode_untrusted(decoder)
                })
                .map($wrapper)
            }
        }
    };
}

nesting_decoders!(
    Nested,
    STORED_DECODE_DEPTH,
    MAX_STORED_DECODE_DEPTH,
    "value"
);
nesting_decoders!(
    RuleNested,
    STORED_RULE_DECODE_DEPTH,
    MAX_STORED_RULE_DECODE_DEPTH,
    "rule"
);

#[cfg(test)]
mod tests {
    use super::*;
    use platform_serialization::PLATFORM_BINCODE_CONFIG;
    use platform_version::version::PLATFORM_VERSIONS;

    /// A value holding one of its own kind, as a nested property or schema node does.
    #[derive(Debug, PartialEq, Encode, Decode, DecodeUntrusted)]
    struct Chain(Option<Box<Nested<Chain>>>);

    /// A chain `depth` values deep.
    fn chain(depth: usize) -> Chain {
        let mut chain = Chain(None);
        for _ in 0..depth {
            chain = Chain(Some(Box::new(Nested(chain))));
        }
        chain
    }

    /// A chain `depth` values deep, encoded and decoded with the untrusted decoder.
    fn round_trip(depth: usize) -> Result<Chain, DecodeError> {
        let bytes = bincode::encode_to_vec(chain(depth), PLATFORM_BINCODE_CONFIG).expect("encodes");
        bincode::borrow_decode_from_slice_untrusted::<Chain, _>(&bytes, PLATFORM_BINCODE_CONFIG)
            .map(|(chain, _)| chain)
    }

    /// A release build decodes and drops values nested to the bound within a 2 MiB stack; an
    /// unoptimized build does not, so the test runs on a larger one.
    #[test]
    fn should_refuse_a_stored_value_nested_past_the_decoding_bound() {
        std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(|| {
                assert_eq!(
                    round_trip(MAX_STORED_DECODE_DEPTH).expect("expected it to decode"),
                    chain(MAX_STORED_DECODE_DEPTH)
                );
                let error = round_trip(MAX_STORED_DECODE_DEPTH + 1)
                    .expect_err("expected decoding to refuse it");
                assert!(error.to_string().contains("value nesting depth"), "{error}");
                // The depth is back to zero: the next decode on this thread works
                assert!(round_trip(MAX_STORED_DECODE_DEPTH).is_ok());
            })
            .expect("expected the thread to start")
            .join()
            .expect("expected the checks to pass");
    }

    /// Every schema a contract can register stores within the decoder's bound: the registration
    /// depth limit is within it at every protocol version.
    #[test]
    fn should_keep_every_registrable_schema_depth_within_the_stored_decode_bound() {
        for platform_version in PLATFORM_VERSIONS {
            assert!(
                usize::from(
                    platform_version
                        .dpp
                        .contract_versions
                        .document_type_versions
                        .schema
                        .max_depth
                ) <= MAX_STORED_DECODE_DEPTH,
                "protocol version {} registers schemas deeper than a stored document type \
                 decodes",
                platform_version.protocol_version
            );
        }
    }
}
