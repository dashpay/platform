//! The `anyOf` form of a `refersTo` declaration: the reference holds if at
//! least one of two or more targets holds.
//!
//! Declared in place of a single target (meta-schema v3, protocol version 14),
//! on an identifier property or on the elements of a typed array:
//!
//! ```json
//! "refersTo": {
//!   "anyOf": [
//!     { "type": "identity" },
//!     { "type": "permanentDocument", "documentType": "member" }
//!   ]
//! }
//! ```
//!
//! reads: the value is the id of an identity, or of a `member` document.
//! Each target is an ordinary declaration with its own keys, and only
//! `identity` and `permanentDocument` (by id or through a `lookup`) are
//! allowed: both are existence checks against entities that can never be
//! deleted, so an `anyOf` of them holds for good once it holds, like a single
//! one of them. Consensus checks the targets in declared order when the
//! referring document is written and stops at the first that holds; every
//! read is billed, those of the targets that failed included, and when none
//! holds the write is refused with the error of the last target.

use crate::data_contract::document_type::property::DocumentPropertyReferenceTarget;
use bincode::de::{BorrowDecoder, BorrowUntrustedDecoder, Decoder, UntrustedDecoder};
use bincode::error::DecodeError;
use bincode::{BorrowDecode, BorrowDecodeUntrusted, Decode, DecodeUntrusted, Encode};
use serde::Serialize;
use std::cell::Cell;

/// The `type` values an `anyOf` target may declare.
pub const ANY_OF_REFERENCE_TARGET_TYPES: [&str; 2] = ["identity", "permanentDocument"];

/// The targets of an `anyOf` reference, in declared order: at least two, none
/// of them an `anyOf` itself, each of a type in
/// [`ANY_OF_REFERENCE_TARGET_TYPES`], and no two alike. The parser enforces
/// all of it on every parse; `SystemLimits::max_any_of_reference_targets`
/// caps the count under full validation.
///
/// Decoding refuses an `anyOf` target inside an `anyOf`: the enum is embedded
/// in consensus errors, which clients decode from bytes a node sends, and a
/// nesting the parser can never produce must not let those bytes drive the
/// decoder into unbounded recursion. Encoding needs no guard, since what it
/// encodes was parsed.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Encode)]
#[serde(transparent)]
pub struct AnyOfReferenceTargets(Vec<DocumentPropertyReferenceTarget>);

impl AnyOfReferenceTargets {
    /// Wraps targets the caller has checked against the rules above.
    pub fn new(targets: Vec<DocumentPropertyReferenceTarget>) -> Self {
        Self(targets)
    }

    /// The targets, in declared order.
    pub fn targets(&self) -> &[DocumentPropertyReferenceTarget] {
        &self.0
    }
}

std::thread_local! {
    /// Whether this thread is decoding the targets of an `anyOf`, so that a
    /// target which is itself an `anyOf` is refused before it recurses.
    static DECODING_ANY_OF_TARGETS: Cell<bool> = const { Cell::new(false) };
}

/// Runs `decode`, the decoding of an `anyOf`'s target list, refusing it when
/// this thread is already inside one: a decode reaches here again only
/// through a target that is an `anyOf`.
fn decode_targets(
    decode: impl FnOnce() -> Result<Vec<DocumentPropertyReferenceTarget>, DecodeError>,
) -> Result<AnyOfReferenceTargets, DecodeError> {
    struct LeaveTargets;

    impl Drop for LeaveTargets {
        fn drop(&mut self) {
            DECODING_ANY_OF_TARGETS.with(|decoding| decoding.set(false));
        }
    }

    if DECODING_ANY_OF_TARGETS.with(|decoding| decoding.get()) {
        return Err(DecodeError::OtherString(
            "an anyOf reference target cannot itself be an anyOf".to_string(),
        ));
    }
    DECODING_ANY_OF_TARGETS.with(|decoding| decoding.set(true));
    let _leave = LeaveTargets;
    decode().map(AnyOfReferenceTargets)
}

impl<Context> Decode<Context> for AnyOfReferenceTargets {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        decode_targets(|| Vec::decode(decoder))
    }
}

impl<'de, Context> BorrowDecode<'de, Context> for AnyOfReferenceTargets {
    fn borrow_decode<D: BorrowDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        decode_targets(|| Vec::borrow_decode(decoder))
    }
}

impl<Context> DecodeUntrusted<Context> for AnyOfReferenceTargets {
    fn decode_untrusted<D: UntrustedDecoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        decode_targets(|| Vec::decode_untrusted(decoder))
    }
}

impl<'de, Context> BorrowDecodeUntrusted<'de, Context> for AnyOfReferenceTargets {
    fn borrow_decode_untrusted<D: BorrowUntrustedDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        decode_targets(|| Vec::borrow_decode_untrusted(decoder))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::document_type::{DocumentReferenceLookup, LookupKeySource};
    use std::collections::BTreeMap;

    fn member_lookup() -> DocumentPropertyReferenceTarget {
        DocumentPropertyReferenceTarget::PermanentDocumentLookup {
            contract_id: None,
            document_type_name: "addedModerator".to_string(),
            property_agreement: BTreeMap::new(),
            lookup: DocumentReferenceLookup {
                index: "byModerator".to_string(),
                keys: [("moderatorId".to_string(), LookupKeySource::ReferenceValue)].into(),
            },
        }
    }

    fn any_of() -> DocumentPropertyReferenceTarget {
        DocumentPropertyReferenceTarget::AnyOf(AnyOfReferenceTargets::new(vec![
            DocumentPropertyReferenceTarget::Identity,
            member_lookup(),
        ]))
    }

    #[test]
    fn should_round_trip_an_any_of_target_through_every_decoder() {
        let target = any_of();
        let bytes = bincode::encode_to_vec(&target, bincode::config::standard()).expect("encodes");
        // Appended after the seven single targets
        assert_eq!(bytes[0], 7);

        let (decoded, read): (DocumentPropertyReferenceTarget, usize) =
            bincode::decode_from_slice(&bytes, bincode::config::standard()).expect("decodes");
        assert_eq!((decoded, read), (target.clone(), bytes.len()));
        let (decoded, _): (DocumentPropertyReferenceTarget, usize) =
            bincode::borrow_decode_from_slice(&bytes, bincode::config::standard())
                .expect("borrow decodes");
        assert_eq!(decoded, target);
        let (decoded, _): (DocumentPropertyReferenceTarget, usize) =
            bincode::decode_from_slice_untrusted(&bytes, bincode::config::standard())
                .expect("decodes untrusted");
        assert_eq!(decoded, target);

        // The guard is left on no thread: a second decode on this one works
        let (decoded, _): (DocumentPropertyReferenceTarget, usize) =
            bincode::decode_from_slice_untrusted(&bytes, bincode::config::standard())
                .expect("decodes again");
        assert_eq!(decoded, target);
    }

    /// The parser never produces an `anyOf` inside an `anyOf`, and bytes that
    /// claim one are refused at the second level, however deep they nest: a
    /// consensus error carrying the target cannot drive a client's decoder
    /// into unbounded recursion.
    #[test]
    fn should_refuse_to_decode_an_any_of_nested_in_an_any_of() {
        let nested = DocumentPropertyReferenceTarget::AnyOf(AnyOfReferenceTargets::new(vec![
            DocumentPropertyReferenceTarget::Identity,
            any_of(),
        ]));
        let bytes = bincode::encode_to_vec(&nested, bincode::config::standard()).expect("encodes");

        let trusted: Result<(DocumentPropertyReferenceTarget, usize), _> =
            bincode::decode_from_slice(&bytes, bincode::config::standard());
        let untrusted: Result<(DocumentPropertyReferenceTarget, usize), _> =
            bincode::decode_from_slice_untrusted(&bytes, bincode::config::standard());
        for result in [trusted, untrusted] {
            let error = result.expect_err("a nested anyOf is refused");
            assert!(
                error.to_string().contains("cannot itself be an anyOf"),
                "unexpected error: {error}"
            );
        }

        // Deep nesting claimed by hand: variant 7, one target, variant 7, ...
        let mut deep = Vec::new();
        for _ in 0..100_000 {
            deep.extend_from_slice(&[7, 1]);
        }
        let result: Result<(DocumentPropertyReferenceTarget, usize), _> =
            bincode::decode_from_slice_untrusted(&deep, bincode::config::standard());
        assert!(result.is_err());

        // A refused decode leaves the guard off
        let bytes = bincode::encode_to_vec(any_of(), bincode::config::standard()).expect("encodes");
        let (decoded, _): (DocumentPropertyReferenceTarget, usize) =
            bincode::decode_from_slice_untrusted(&bytes, bincode::config::standard())
                .expect("decodes after a refusal");
        assert_eq!(decoded, any_of());
    }

    #[test]
    fn should_serialize_an_any_of_target_under_its_schema_key() {
        assert_eq!(
            serde_json::to_value(any_of()).expect("serializes"),
            serde_json::json!({
                "anyOf": [
                    "identity",
                    {
                        "permanentDocument": {
                            "contract_id": null,
                            "document_type_name": "addedModerator",
                            "lookup": {
                                "index": "byModerator",
                                "keys": { "moderatorId": "." }
                            }
                        }
                    }
                ]
            })
        );
    }
}
