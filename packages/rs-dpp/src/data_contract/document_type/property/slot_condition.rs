//! The `when` condition of a `documentTypeRequirements` slot (protocol version 15).
//!
//! A slot describes a property the referenced document type must have by its name, its
//! value and its size. `when` adds a condition on what the type says about the property,
//! in the vocabulary of `propertyConstraints` conditions, read against the referenced
//! type's schema instead of a document:
//!
//! ```json
//! "body": {
//!   "name": "$any",
//!   "type": "string",
//!   "when": {
//!     "ifThen": [
//!       { "equal": ["$kind", { "const": "moderatedDocument" }] },
//!       { "allOf": [
//!         { "not": { "contains": ["$moderatorAbilities.deleteKeepsFields", "$self"] } },
//!         { "not": { "contains": ["$moderatorAbilities.changeFields", "$self"] } }
//!       ] }
//!     ]
//!   }
//! }
//! ```
//!
//! reads: when the type is moderated, the property filling the slot is one a moderator's
//! removal takes away and one its author writes. A property the condition does not hold for
//! does not fill the slot.
//!
//! The operands are `$kind` (the kind of the referenced type, by the `refersTo` `type` it
//! is referred to with), `$self` (the name of the property tried for the slot),
//! `$moderatorAbilities.deleteKeepsFields` and `$moderatorAbilities.changeFields` (the
//! type's lists, empty where it declares none) and `{ "const": <string> }`.

use crate::data_contract::document_type::property::DocumentReferenceKind;
use bincode::de::{BorrowDecoder, BorrowUntrustedDecoder, Decoder, UntrustedDecoder};
use bincode::error::DecodeError;
use bincode::{BorrowDecode, BorrowDecodeUntrusted, Decode, DecodeUntrusted, Encode};
use serde::Serialize;
use std::cell::Cell;
use std::collections::BTreeSet;

/// The operand reading the kind of the referenced document type.
pub const SLOT_CONDITION_KIND: &str = "$kind";
/// The operand reading the name of the property tried for the slot.
pub const SLOT_CONDITION_SELF: &str = "$self";
/// The operand reading the referenced type's `moderatorAbilities.deleteKeepsFields`.
pub const SLOT_CONDITION_DELETE_KEEPS_FIELDS: &str = "$moderatorAbilities.deleteKeepsFields";
/// The operand reading the referenced type's `moderatorAbilities.changeFields`.
pub const SLOT_CONDITION_CHANGE_FIELDS: &str = "$moderatorAbilities.changeFields";

/// The deepest a decoder accepts conditions inside one another. It only keeps crafted bytes
/// from driving the decoder, and the evaluation of what it decodes, into unbounded
/// recursion: a registered condition has at most
/// `SystemLimits::max_document_type_requirement_condition_nodes` conditions, so it nests
/// less deep than that (a test holds every protocol version's limit to it), and every
/// declaration a contract can carry decodes. A constant rather than a limit, like
/// `MAX_REFERENCE_EXPRESSION_DECODE_DEPTH`, so that no change to a limit can make a stored
/// contract undecodable.
pub const MAX_SLOT_CONDITION_DECODE_DEPTH: usize = 16;

/// A string a slot condition compares.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Encode, Decode, DecodeUntrusted)]
pub enum SlotConditionValue {
    /// `$kind`: `permanentDocument`, `moderatedDocument` or `deletableDocument`, as the
    /// referenced type's documents leave state.
    Kind,
    /// `$self`: the name of the property tried for the slot.
    PropertyName,
    /// `{ "const": <string> }`.
    Constant(String),
}

/// A list of property paths of the referenced type a slot condition looks into.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Encode, Decode, DecodeUntrusted)]
pub enum SlotConditionList {
    /// `$moderatorAbilities.deleteKeepsFields`: what a moderator's removal keeps public.
    DeletionKeptFields,
    /// `$moderatorAbilities.changeFields`: what only moderators write.
    ModeratorChangeableFields,
}

impl SlotConditionList {
    /// The operand the list is written as.
    pub const fn wire_name(self) -> &'static str {
        match self {
            SlotConditionList::DeletionKeptFields => SLOT_CONDITION_DELETE_KEEPS_FIELDS,
            SlotConditionList::ModeratorChangeableFields => SLOT_CONDITION_CHANGE_FIELDS,
        }
    }
}

/// A condition a property must meet to fill a slot.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Encode, Decode, DecodeUntrusted)]
pub enum SlotCondition {
    /// `equal`: the two strings are the same.
    Equal(SlotConditionValue, SlotConditionValue),
    /// `notEqual`: the two strings differ.
    NotEqual(SlotConditionValue, SlotConditionValue),
    /// `in`: the string is one of the listed ones.
    In(SlotConditionValue, BTreeSet<String>),
    /// `notIn`: the string is none of the listed ones.
    NotIn(SlotConditionValue, BTreeSet<String>),
    /// `contains`: the list holds the string, as a whole entry.
    Contains(SlotConditionList, SlotConditionValue),
    /// `not`: the one condition does not hold.
    Not(SlotConditions),
    /// `anyOf`: at least one of the conditions holds.
    AnyOf(SlotConditions),
    /// `allOf`: every one of the conditions holds.
    AllOf(SlotConditions),
    /// `ifThen`: two conditions; the second holds whenever the first does.
    IfThen(SlotConditions),
    /// `ifThenElse`: three conditions; the second holds when the first does, the third
    /// when it does not.
    IfThenElse(SlotConditions),
}

/// What a slot condition is judged against: the referenced document type, and the one of
/// its properties tried for the slot.
#[derive(Debug, Clone, Copy)]
pub struct SlotConditionSubject<'a> {
    /// How the referenced type's documents leave state.
    pub kind: DocumentReferenceKind,
    /// The name of the property tried for the slot.
    pub property_name: &'a str,
    /// The referenced type's `moderatorAbilities.deleteKeepsFields`.
    pub deletion_kept_fields: &'a BTreeSet<String>,
    /// The referenced type's `moderatorAbilities.changeFields`.
    pub moderator_changeable_fields: &'a BTreeSet<String>,
}

impl SlotConditionValue {
    fn read<'a>(&'a self, subject: &SlotConditionSubject<'a>) -> &'a str {
        match self {
            SlotConditionValue::Kind => subject.kind.wire_name(),
            SlotConditionValue::PropertyName => subject.property_name,
            SlotConditionValue::Constant(constant) => constant,
        }
    }
}

impl SlotCondition {
    /// Whether the condition holds for `subject`.
    ///
    /// A `not`, `ifThen` or `ifThenElse` holding another number of conditions than it takes
    /// is one no parse produces; it holds for nothing.
    pub fn holds(&self, subject: &SlotConditionSubject) -> bool {
        match self {
            SlotCondition::Equal(left, right) => left.read(subject) == right.read(subject),
            SlotCondition::NotEqual(left, right) => left.read(subject) != right.read(subject),
            SlotCondition::In(value, among) => among.contains(value.read(subject)),
            SlotCondition::NotIn(value, among) => !among.contains(value.read(subject)),
            SlotCondition::Contains(list, value) => {
                let list = match list {
                    SlotConditionList::DeletionKeptFields => subject.deletion_kept_fields,
                    SlotConditionList::ModeratorChangeableFields => {
                        subject.moderator_changeable_fields
                    }
                };
                list.contains(value.read(subject))
            }
            SlotCondition::Not(conditions) => match conditions.conditions() {
                [condition] => !condition.holds(subject),
                _ => false,
            },
            SlotCondition::AnyOf(conditions) => conditions
                .conditions()
                .iter()
                .any(|condition| condition.holds(subject)),
            SlotCondition::AllOf(conditions) => conditions
                .conditions()
                .iter()
                .all(|condition| condition.holds(subject)),
            SlotCondition::IfThen(conditions) => match conditions.conditions() {
                [condition, then] => !condition.holds(subject) || then.holds(subject),
                _ => false,
            },
            SlotCondition::IfThenElse(conditions) => match conditions.conditions() {
                [condition, then, otherwise] => {
                    if condition.holds(subject) {
                        then.holds(subject)
                    } else {
                        otherwise.holds(subject)
                    }
                }
                _ => false,
            },
        }
    }

    /// How many conditions this one is made of, itself included: what
    /// `SystemLimits::max_document_type_requirement_condition_nodes` bounds.
    pub fn node_count(&self) -> usize {
        match self {
            SlotCondition::Equal(..)
            | SlotCondition::NotEqual(..)
            | SlotCondition::In(..)
            | SlotCondition::NotIn(..)
            | SlotCondition::Contains(..) => 1,
            SlotCondition::Not(conditions)
            | SlotCondition::AnyOf(conditions)
            | SlotCondition::AllOf(conditions)
            | SlotCondition::IfThen(conditions)
            | SlotCondition::IfThenElse(conditions) => conditions
                .conditions()
                .iter()
                .map(SlotCondition::node_count)
                .fold(1usize, usize::saturating_add),
        }
    }
}

/// The conditions a [`SlotCondition`] holds inside it.
///
/// Decoding refuses conditions inside one another deeper than
/// [`MAX_SLOT_CONDITION_DECODE_DEPTH`]: the condition is embedded in a reference target,
/// which consensus errors carry and clients decode from bytes a node sends, and those bytes
/// must not drive the decoder into unbounded recursion. Encoding needs no guard, since what
/// it encodes was parsed.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Encode)]
#[serde(transparent)]
pub struct SlotConditions(Vec<SlotCondition>);

impl SlotConditions {
    /// Wraps conditions the caller has checked: as many as the condition holding them takes.
    pub fn new(conditions: Vec<SlotCondition>) -> Self {
        Self(conditions)
    }

    /// The conditions, in declared order.
    pub fn conditions(&self) -> &[SlotCondition] {
        &self.0
    }
}

std::thread_local! {
    /// How many condition lists this thread is decoding inside one another.
    static CONDITION_LIST_DECODE_DEPTH: Cell<usize> = const { Cell::new(0) };
}

/// Runs `decode`, the decoding of one list of conditions, one level deeper than the list
/// around it, refusing it past [`MAX_SLOT_CONDITION_DECODE_DEPTH`].
fn decode_conditions(
    decode: impl FnOnce() -> Result<Vec<SlotCondition>, DecodeError>,
) -> Result<SlotConditions, DecodeError> {
    struct LeaveList;

    impl Drop for LeaveList {
        fn drop(&mut self) {
            CONDITION_LIST_DECODE_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
        }
    }

    let depth = CONDITION_LIST_DECODE_DEPTH.with(|depth| depth.get()) + 1;
    if depth > MAX_SLOT_CONDITION_DECODE_DEPTH {
        return Err(DecodeError::OtherString(format!(
            "slot condition nesting depth {depth} exceeds the maximum of \
             {MAX_SLOT_CONDITION_DECODE_DEPTH}"
        )));
    }
    CONDITION_LIST_DECODE_DEPTH.with(|current| current.set(depth));
    let _leave = LeaveList;
    decode().map(SlotConditions)
}

impl<Context> Decode<Context> for SlotConditions {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        decode_conditions(|| Vec::decode(decoder))
    }
}

impl<'de, Context> BorrowDecode<'de, Context> for SlotConditions {
    fn borrow_decode<D: BorrowDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        decode_conditions(|| Vec::borrow_decode(decoder))
    }
}

impl<Context> DecodeUntrusted<Context> for SlotConditions {
    fn decode_untrusted<D: UntrustedDecoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        decode_conditions(|| Vec::decode_untrusted(decoder))
    }
}

impl<'de, Context> BorrowDecodeUntrusted<'de, Context> for SlotConditions {
    fn borrow_decode_untrusted<D: BorrowUntrustedDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        decode_conditions(|| Vec::borrow_decode_untrusted(decoder))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_version::version::PLATFORM_VERSIONS;

    fn kept(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    fn not(condition: SlotCondition) -> SlotCondition {
        SlotCondition::Not(SlotConditions::new(vec![condition]))
    }

    fn is_moderated() -> SlotCondition {
        SlotCondition::Equal(
            SlotConditionValue::Kind,
            SlotConditionValue::Constant("moderatedDocument".to_string()),
        )
    }

    fn listed(list: SlotConditionList) -> SlotCondition {
        SlotCondition::Contains(list, SlotConditionValue::PropertyName)
    }

    /// "When the type is moderated, a removal takes the property and its author writes it."
    fn removed_with_the_document() -> SlotCondition {
        SlotCondition::IfThen(SlotConditions::new(vec![
            is_moderated(),
            SlotCondition::AllOf(SlotConditions::new(vec![
                not(listed(SlotConditionList::DeletionKeptFields)),
                not(listed(SlotConditionList::ModeratorChangeableFields)),
            ])),
        ]))
    }

    /// `depth` lists of conditions inside one another.
    fn nested_to(depth: usize) -> SlotCondition {
        let mut condition = is_moderated();
        for _ in 0..depth {
            condition = not(condition);
        }
        condition
    }

    fn decode_every_way(bytes: &[u8]) -> [Result<SlotCondition, DecodeError>; 3] {
        let config = bincode::config::standard();
        [
            bincode::decode_from_slice(bytes, config).map(|(condition, _)| condition),
            bincode::borrow_decode_from_slice(bytes, config).map(|(condition, _)| condition),
            bincode::decode_from_slice_untrusted(bytes, config).map(|(condition, _)| condition),
        ]
    }

    #[test]
    fn should_judge_a_property_by_what_its_type_says_of_it() {
        let condition = removed_with_the_document();
        let kept_fields = kept(&["tags", "$createdAt"]);
        let changeable_fields = kept(&["label"]);
        let none = BTreeSet::new();
        let holds =
            |kind, property_name, kept: &BTreeSet<String>, changeable: &BTreeSet<String>| {
                condition.holds(&SlotConditionSubject {
                    kind,
                    property_name,
                    deletion_kept_fields: kept,
                    moderator_changeable_fields: changeable,
                })
            };

        // A moderated type: only a property neither list names
        let moderated = DocumentReferenceKind::Moderated;
        assert!(holds(
            moderated,
            "content",
            &kept_fields,
            &changeable_fields
        ));
        assert!(!holds(moderated, "tags", &kept_fields, &changeable_fields));
        assert!(!holds(moderated, "label", &kept_fields, &changeable_fields));
        assert!(holds(moderated, "tags", &none, &none));
        // A whole entry, not a part of one
        assert!(holds(moderated, "tag", &kept_fields, &changeable_fields));

        // A permanent type: the condition asks nothing of it
        let permanent = DocumentReferenceKind::Permanent;
        assert!(holds(permanent, "tags", &kept_fields, &changeable_fields));
        assert!(holds(permanent, "label", &kept_fields, &changeable_fields));

        assert_eq!(condition.node_count(), 7);
    }

    #[test]
    fn should_judge_every_form() {
        let empty = BTreeSet::new();
        let subject = SlotConditionSubject {
            kind: DocumentReferenceKind::Permanent,
            property_name: "body",
            deletion_kept_fields: &empty,
            moderator_changeable_fields: &empty,
        };
        let name_is = |name: &str| {
            SlotCondition::Equal(
                SlotConditionValue::PropertyName,
                SlotConditionValue::Constant(name.to_string()),
            )
        };
        let list = |conditions: Vec<SlotCondition>| SlotConditions::new(conditions);

        assert!(name_is("body").holds(&subject));
        assert!(!name_is("title").holds(&subject));
        assert!(SlotCondition::NotEqual(
            SlotConditionValue::PropertyName,
            SlotConditionValue::Constant("title".to_string())
        )
        .holds(&subject));
        assert!(
            SlotCondition::In(SlotConditionValue::PropertyName, kept(&["body", "text"]))
                .holds(&subject)
        );
        assert!(
            SlotCondition::NotIn(SlotConditionValue::PropertyName, kept(&["title", "text"]))
                .holds(&subject)
        );
        assert!(
            SlotCondition::In(SlotConditionValue::Kind, kept(&["permanentDocument"]))
                .holds(&subject)
        );
        assert!(
            SlotCondition::AnyOf(list(vec![name_is("title"), name_is("body")])).holds(&subject)
        );
        assert!(
            !SlotCondition::AllOf(list(vec![name_is("title"), name_is("body")])).holds(&subject)
        );
        assert!(not(name_is("title")).holds(&subject));
        // The first holds, so the second decides; the first fails, so the third does
        assert!(!SlotCondition::IfThenElse(list(vec![
            name_is("body"),
            name_is("title"),
            name_is("body")
        ]))
        .holds(&subject));
        assert!(SlotCondition::IfThenElse(list(vec![
            name_is("title"),
            name_is("title"),
            name_is("body")
        ]))
        .holds(&subject));

        // A count of conditions no parse produces holds for nothing
        assert!(!SlotCondition::Not(list(vec![])).holds(&subject));
        assert!(
            !SlotCondition::Not(list(vec![name_is("title"), name_is("title")])).holds(&subject)
        );
        assert!(!SlotCondition::IfThen(list(vec![name_is("title")])).holds(&subject));
        assert!(
            !SlotCondition::IfThenElse(list(vec![name_is("body"), name_is("body")]))
                .holds(&subject)
        );
    }

    #[test]
    fn should_round_trip_through_every_decoder() {
        let condition = removed_with_the_document();
        let bytes =
            bincode::encode_to_vec(&condition, bincode::config::standard()).expect("encodes");
        for decoded in decode_every_way(&bytes) {
            assert_eq!(decoded.expect("decodes"), condition);
        }
        // The depth is back to zero: a second decode on this thread works
        for decoded in decode_every_way(&bytes) {
            assert_eq!(decoded.expect("decodes again"), condition);
        }
    }

    /// Nesting within the decode bound decodes; one level past it is refused, however deep
    /// the bytes claim to go. A refused decode leaves the depth where it found it.
    #[test]
    fn should_refuse_to_decode_a_nesting_past_the_decode_bound() {
        let deepest = nested_to(MAX_SLOT_CONDITION_DECODE_DEPTH);
        let bytes = bincode::encode_to_vec(&deepest, bincode::config::standard()).expect("encodes");
        for decoded in decode_every_way(&bytes) {
            assert_eq!(decoded.expect("the bound itself decodes"), deepest);
        }

        let too_deep = nested_to(MAX_SLOT_CONDITION_DECODE_DEPTH + 1);
        let bytes =
            bincode::encode_to_vec(&too_deep, bincode::config::standard()).expect("encodes");
        for decoded in decode_every_way(&bytes) {
            let error = decoded.expect_err("one level past the bound is refused");
            assert!(
                error.to_string().contains(&format!(
                    "nesting depth {} exceeds the maximum of {MAX_SLOT_CONDITION_DECODE_DEPTH}",
                    MAX_SLOT_CONDITION_DECODE_DEPTH + 1
                )),
                "unexpected error: {error}"
            );
        }

        // Nesting claimed by hand far past the bound: variant 5 (`not`), one condition, ...
        assert_eq!(
            bincode::encode_to_vec(not(is_moderated()), bincode::config::standard())
                .expect("encodes")[..2],
            [5, 1]
        );
        let mut deep = Vec::new();
        for _ in 0..100_000 {
            deep.extend_from_slice(&[5, 1]);
        }
        for decoded in decode_every_way(&deep) {
            let error = decoded.expect_err("claimed nesting is refused at the bound");
            assert!(
                error.to_string().contains("slot condition nesting depth"),
                "unexpected error: {error}"
            );
        }

        for decoded in decode_every_way(
            &bincode::encode_to_vec(&deepest, bincode::config::standard()).expect("encodes"),
        ) {
            assert_eq!(decoded.expect("decodes after a refusal"), deepest);
        }
    }

    /// A condition of `n` conditions nests at most `n - 1` lists deep, so every condition
    /// a contract can register decodes.
    #[test]
    fn should_keep_every_registration_limit_within_the_decode_bound() {
        for platform_version in PLATFORM_VERSIONS.iter() {
            assert!(
                usize::from(
                    platform_version
                        .system_limits
                        .max_document_type_requirement_condition_nodes
                ) <= MAX_SLOT_CONDITION_DECODE_DEPTH,
                "protocol version {} admits a slot condition a decoder refuses",
                platform_version.protocol_version
            );
        }
    }
}
