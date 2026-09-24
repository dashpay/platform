//! Reference expressions: a `refersTo` declaration combining targets with
//! `anyOf` (at least one operand holds) and `allOf` (every operand holds),
//! nested up to a registration limit.
//!
//! Declared in place of a single target (meta-schema v3, protocol version 14),
//! on an identifier property or on the elements of a typed array:
//!
//! ```json
//! "refersTo": {
//!   "anyOf": [
//!     { "type": "permanentDocument", "documentType": "addedModerator", "lookup": { ... } },
//!     { "allOf": [
//!       { "type": "permanentDocument", "documentType": "joinRequest", "lookup": { ... } },
//!       { "type": "identity" }
//!     ] }
//!   ]
//! }
//! ```
//!
//! reads: the value was added as a moderator, or it both asked to join and is
//! an identity. Every leaf is an ordinary declaration with its own keys, and
//! only `identity` and `permanentDocument` (by id or through a `lookup`) are
//! allowed: both are existence checks against entities that can never be
//! deleted, so an expression of them holds for good once it holds, like a
//! single one of them. Consensus evaluates an expression when the referring
//! document is written: an `anyOf` checks its operands in declared order and
//! stops at the first that holds, refusing with the error of the last when none
//! does; an `allOf` checks them in declared order and stops at the first that
//! fails, refusing with that operand's error. Every read is billed, those of
//! the operands that failed included.

use crate::data_contract::document_type::property::DocumentPropertyReferenceTarget;
use bincode::de::{BorrowDecoder, BorrowUntrustedDecoder, Decoder, UntrustedDecoder};
use bincode::error::DecodeError;
use bincode::{BorrowDecode, BorrowDecodeUntrusted, Decode, DecodeUntrusted, Encode};
use serde::Serialize;
use std::cell::Cell;

/// The `type` values a leaf of a reference expression may declare: existence
/// checks against entities that are never deleted (`listElement` reads a
/// list that never changes on a document that is never deleted, so it holds
/// for good once it holds, as the other two do).
///
/// Read by `apply_property_reference` 0 (protocol version 14). Admitting
/// another type once that version is released takes a new generation of the
/// parser (and a new meta-schema), not an edit here.
pub const COMBINABLE_REFERENCE_TARGET_TYPES: [&str; 3] =
    ["identity", "permanentDocument", "listElement"];

/// The deepest nesting of `anyOf` and `allOf` a decoder accepts. It only keeps
/// crafted bytes from driving the decoder into unbounded recursion: the
/// registration limit, `SystemLimits::max_reference_expression_depth`, is far
/// below it (a test holds every protocol version's limit to it), so every
/// declaration a contract can carry decodes.
pub const MAX_REFERENCE_EXPRESSION_DECODE_DEPTH: usize = 16;

/// How the operands of a reference expression combine.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ReferenceCombinator {
    /// `anyOf`: the expression holds if at least one operand holds.
    AnyOf,
    /// `allOf`: the expression holds if every operand holds.
    AllOf,
}

impl ReferenceCombinator {
    /// Both combinators, in the order the parser looks for their keys.
    pub const ALL: [ReferenceCombinator; 2] =
        [ReferenceCombinator::AnyOf, ReferenceCombinator::AllOf];

    /// The schema key declaring it.
    pub fn wire_name(self) -> &'static str {
        match self {
            ReferenceCombinator::AnyOf => "anyOf",
            ReferenceCombinator::AllOf => "allOf",
        }
    }
}

/// The operands of an `anyOf` or `allOf` reference expression, in declared
/// order: at least two, each a leaf of a type in
/// [`COMBINABLE_REFERENCE_TARGET_TYPES`] or an expression of the other
/// combinator (an `anyOf` directly inside an `anyOf` says what one flat list
/// says). The parser enforces those on every parse; the most operands one list
/// may hold (`SystemLimits::max_reference_operands`), the deepest nesting
/// (`SystemLimits::max_reference_expression_depth`) and that no two operands of
/// a list are alike are checked under full validation.
///
/// Decoding refuses a nesting deeper than
/// [`MAX_REFERENCE_EXPRESSION_DECODE_DEPTH`]: the enum is embedded in consensus
/// errors, which clients decode from bytes a node sends, and those bytes must
/// not drive the decoder into unbounded recursion. Encoding needs no guard,
/// since what it encodes was parsed.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Encode)]
#[serde(transparent)]
pub struct ReferenceOperands(Vec<DocumentPropertyReferenceTarget>);

impl ReferenceOperands {
    /// Wraps operands the caller has checked against the rules above.
    pub fn new(operands: Vec<DocumentPropertyReferenceTarget>) -> Self {
        Self(operands)
    }

    /// The operands, in declared order.
    pub fn operands(&self) -> &[DocumentPropertyReferenceTarget] {
        &self.0
    }
}

std::thread_local! {
    /// How many operand lists this thread is decoding inside one another.
    static OPERAND_LIST_DECODE_DEPTH: Cell<usize> = const { Cell::new(0) };
}

/// Runs `decode`, the decoding of one operand list, one level deeper than the
/// list around it, refusing it past [`MAX_REFERENCE_EXPRESSION_DECODE_DEPTH`].
fn decode_operands(
    decode: impl FnOnce() -> Result<Vec<DocumentPropertyReferenceTarget>, DecodeError>,
) -> Result<ReferenceOperands, DecodeError> {
    struct LeaveList;

    impl Drop for LeaveList {
        fn drop(&mut self) {
            OPERAND_LIST_DECODE_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
        }
    }

    let depth = OPERAND_LIST_DECODE_DEPTH.with(|depth| depth.get()) + 1;
    if depth > MAX_REFERENCE_EXPRESSION_DECODE_DEPTH {
        return Err(DecodeError::OtherString(format!(
            "reference expression nesting depth {depth} exceeds the maximum of \
             {MAX_REFERENCE_EXPRESSION_DECODE_DEPTH}"
        )));
    }
    OPERAND_LIST_DECODE_DEPTH.with(|current| current.set(depth));
    let _leave = LeaveList;
    decode().map(ReferenceOperands)
}

impl<Context> Decode<Context> for ReferenceOperands {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        decode_operands(|| Vec::decode(decoder))
    }
}

impl<'de, Context> BorrowDecode<'de, Context> for ReferenceOperands {
    fn borrow_decode<D: BorrowDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        decode_operands(|| Vec::borrow_decode(decoder))
    }
}

impl<Context> DecodeUntrusted<Context> for ReferenceOperands {
    fn decode_untrusted<D: UntrustedDecoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        decode_operands(|| Vec::decode_untrusted(decoder))
    }
}

impl<'de, Context> BorrowDecodeUntrusted<'de, Context> for ReferenceOperands {
    fn borrow_decode_untrusted<D: BorrowUntrustedDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        decode_operands(|| Vec::borrow_decode_untrusted(decoder))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::document_type::{DocumentReferenceLookup, LookupKeySource};
    use platform_version::version::PLATFORM_VERSIONS;
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

    fn any_of(operands: Vec<DocumentPropertyReferenceTarget>) -> DocumentPropertyReferenceTarget {
        DocumentPropertyReferenceTarget::AnyOf(ReferenceOperands::new(operands))
    }

    fn all_of(operands: Vec<DocumentPropertyReferenceTarget>) -> DocumentPropertyReferenceTarget {
        DocumentPropertyReferenceTarget::AllOf(ReferenceOperands::new(operands))
    }

    /// `anyOf(member, allOf(identity, member))`
    fn nested() -> DocumentPropertyReferenceTarget {
        any_of(vec![
            member_lookup(),
            all_of(vec![
                DocumentPropertyReferenceTarget::Identity,
                member_lookup(),
            ]),
        ])
    }

    /// An expression `depth` combinators deep, alternating from `anyOf`.
    fn nested_to(depth: usize) -> DocumentPropertyReferenceTarget {
        let mut expression = DocumentPropertyReferenceTarget::Identity;
        for level in 0..depth {
            let operands = vec![member_lookup(), expression];
            expression = if level % 2 == 0 {
                all_of(operands)
            } else {
                any_of(operands)
            };
        }
        expression
    }

    fn decode_every_way(bytes: &[u8]) -> [Result<DocumentPropertyReferenceTarget, DecodeError>; 3] {
        let config = bincode::config::standard();
        [
            bincode::decode_from_slice(bytes, config).map(|(target, _)| target),
            bincode::borrow_decode_from_slice(bytes, config).map(|(target, _)| target),
            bincode::decode_from_slice_untrusted(bytes, config).map(|(target, _)| target),
        ]
    }

    #[test]
    fn should_round_trip_a_nested_expression_through_every_decoder() {
        let target = nested();
        let bytes = bincode::encode_to_vec(&target, bincode::config::standard()).expect("encodes");
        // Appended after the seven single targets: anyOf 7, allOf 8
        assert_eq!(bytes[0], 7);
        assert_eq!(
            bincode::encode_to_vec(
                all_of(vec![
                    DocumentPropertyReferenceTarget::Identity,
                    member_lookup()
                ]),
                bincode::config::standard()
            )
            .expect("encodes")[0],
            8
        );

        for decoded in decode_every_way(&bytes) {
            assert_eq!(decoded.expect("decodes"), target);
        }
        // The depth is back to zero: a second decode on this thread works
        for decoded in decode_every_way(&bytes) {
            assert_eq!(decoded.expect("decodes again"), target);
        }
    }

    /// Nesting within the decode bound decodes; one level past it is refused,
    /// however deep the bytes claim to go, so a consensus error carrying the
    /// target cannot drive a client's decoder into unbounded recursion. A
    /// refused decode leaves the depth where it found it.
    #[test]
    fn should_refuse_to_decode_a_nesting_past_the_decode_bound() {
        let deepest = nested_to(MAX_REFERENCE_EXPRESSION_DECODE_DEPTH);
        let bytes = bincode::encode_to_vec(&deepest, bincode::config::standard()).expect("encodes");
        for decoded in decode_every_way(&bytes) {
            assert_eq!(decoded.expect("the bound itself decodes"), deepest);
        }

        let too_deep = nested_to(MAX_REFERENCE_EXPRESSION_DECODE_DEPTH + 1);
        let bytes =
            bincode::encode_to_vec(&too_deep, bincode::config::standard()).expect("encodes");
        for decoded in decode_every_way(&bytes) {
            let error = decoded.expect_err("one level past the bound is refused");
            assert!(
                error.to_string().contains(&format!(
                    "nesting depth {} exceeds the maximum of {MAX_REFERENCE_EXPRESSION_DECODE_DEPTH}",
                    MAX_REFERENCE_EXPRESSION_DECODE_DEPTH + 1
                )),
                "unexpected error: {error}"
            );
        }

        // Nesting claimed by hand far past the bound: variant 7, one operand, ...
        let mut deep = Vec::new();
        for _ in 0..100_000 {
            deep.extend_from_slice(&[7, 1]);
        }
        for decoded in decode_every_way(&deep) {
            let error = decoded.expect_err("claimed nesting is refused at the bound");
            assert!(
                error
                    .to_string()
                    .contains("reference expression nesting depth"),
                "unexpected error: {error}"
            );
        }

        let bytes = bincode::encode_to_vec(nested(), bincode::config::standard()).expect("encodes");
        for decoded in decode_every_way(&bytes) {
            assert_eq!(decoded.expect("decodes after a refusal"), nested());
        }
    }

    /// Every declaration a contract can register decodes: the registration
    /// limit is within the decoder's bound at every protocol version.
    #[test]
    fn should_keep_every_registrable_depth_within_the_decode_bound() {
        for platform_version in PLATFORM_VERSIONS {
            assert!(
                usize::from(
                    platform_version
                        .system_limits
                        .max_reference_expression_depth
                ) <= MAX_REFERENCE_EXPRESSION_DECODE_DEPTH,
                "protocol version {} registers reference expressions deeper than a decoder \
                 accepts",
                platform_version.protocol_version
            );
        }
    }

    #[test]
    fn should_serialize_an_expression_under_its_schema_keys() {
        assert_eq!(
            serde_json::to_value(nested()).expect("serializes"),
            serde_json::json!({
                "anyOf": [
                    {
                        "permanentDocument": {
                            "contract_id": null,
                            "document_type_name": "addedModerator",
                            "lookup": {
                                "index": "byModerator",
                                "keys": { "moderatorId": "." }
                            }
                        }
                    },
                    {
                        "allOf": [
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
                    }
                ]
            })
        );
    }
}
