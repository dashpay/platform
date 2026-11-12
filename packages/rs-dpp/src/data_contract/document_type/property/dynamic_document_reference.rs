//! A document reference whose target the referring document names (protocol version 15).
//!
//! Every other document reference names its target in the schema: a `contractId` (or the
//! declaring contract) and a `documentType`, so the referenced document type is known when
//! the contract is registered and is checked then. A dynamic reference reads both from the
//! referring document instead (`"contractId": "$self.<property>"`,
//! `"documentType": "$self.<property>"`, `$self` being the document that is written, as
//! `$old` is the stored one in a rule), and the document says, when it is written, which
//! contract and which document type its value points into:
//!
//! ```json
//! "descriptionId": {
//!   "type": "identifier",
//!   "refersTo": {
//!     "type": "moderatedDocument",
//!     "contractId": "$self.descriptionContractId",
//!     "documentType": "$self.descriptionDocumentType",
//!     "documentTypeRequirements": {
//!       "properties": {
//!         "body":      { "name": "$any", "type": "string" },
//!         "bodyBytes": { "name": "$any", "type": "array", "byteArray": true }
//!       },
//!       "require": { "anyOf": ["body", "bodyBytes"] }
//!     }
//!   }
//! }
//! ```
//!
//! Nothing about the target can be checked at registration, so everything is checked when
//! the document is written: that the contract and the document type exist, that the type is
//! of the declared kind, that it meets the `documentTypeRequirements`, and that the document
//! exists. A type's deletion and moderation settings never change, so a target of the
//! declared kind keeps it; the requirements are judged whenever the reference is checked.
//!
//! `type` may list both kinds, `["permanentDocument", "moderatedDocument"]`: the named type
//! is then either, and the reference is checked as one of the kind the type is. A slot may
//! carry a `when` ([`SlotCondition`]) telling the two apart, so that it asks of a moderated
//! type what it does not ask of a permanent one.

use crate::data_contract::config::moderation::ModerationAbility;
use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use crate::data_contract::document_type::property::{
    DocumentProperty, DocumentPropertyType, DocumentReferenceKind, SlotCondition,
    SlotConditionSubject,
};
use crate::data_contract::document_type::DocumentTypeRef;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::Value;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// The name a slot takes for "any property": `"name": "$any"`.
pub const ANY_PROPERTY_NAME: &str = "$any";

/// The prefix through which a `refersTo` reads a value from the referring document, the
/// document being written: `"$self.descriptionContractId"` is its top-level property
/// `descriptionContractId`. A `contractId` and a `documentType` written with it name the
/// target through the document, and a slot's `name` written with it names the property the
/// document says. No contract id, document type or property can take such a name: none
/// holds a `$` or a dot.
pub const REFERRING_DOCUMENT_PREFIX: &str = "$self.";

/// Which kind of document a dynamic reference points at. A `deletableDocument` reference is
/// never dynamic: a dynamic target is trusted for what its type promises about its
/// documents staying, and a deletable type promises nothing.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Encode, Decode, DecodeUntrusted)]
pub enum DynamicDocumentReferenceKind {
    /// `permanentDocument`: the documents never leave state.
    Permanent,
    /// `moderatedDocument`: the documents leave state only through a moderator's recorded
    /// removal.
    Moderated,
    /// `["permanentDocument", "moderatedDocument"]`: either of the two, whichever the named
    /// type is.
    PermanentOrModerated,
}

impl DynamicDocumentReferenceKind {
    /// The `refersTo` `type` the kind is written with: one name, or the two of a list.
    pub const fn wire_names(self) -> &'static [&'static str] {
        match self {
            DynamicDocumentReferenceKind::Permanent => &["permanentDocument"],
            DynamicDocumentReferenceKind::Moderated => &["moderatedDocument"],
            DynamicDocumentReferenceKind::PermanentOrModerated => {
                &["permanentDocument", "moderatedDocument"]
            }
        }
    }

    /// The kind of reference a document pointing into a type of kind `actual` is checked
    /// as. A declaration of one kind is that kind whatever the type is, so a type of
    /// another is refused as it is for a reference naming its target in the schema. A
    /// declaration of either kind is a moderated reference into a moderated type and a
    /// permanent one into any other, which refuses a type whose documents can be deleted.
    pub const fn checked_as(self, actual: DocumentReferenceKind) -> DocumentReferenceKind {
        match (self, actual) {
            (DynamicDocumentReferenceKind::Permanent, _) => DocumentReferenceKind::Permanent,
            (DynamicDocumentReferenceKind::Moderated, _) => DocumentReferenceKind::Moderated,
            (
                DynamicDocumentReferenceKind::PermanentOrModerated,
                DocumentReferenceKind::Moderated,
            ) => DocumentReferenceKind::Moderated,
            (DynamicDocumentReferenceKind::PermanentOrModerated, _) => {
                DocumentReferenceKind::Permanent
            }
        }
    }
}

/// A `refersTo` whose `contractId` and `documentType` are read from the referring document
/// (`"$self.<property>"`): the value is the id of a document of the document type the
/// referring document names.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Encode, Decode, DecodeUntrusted)]
pub struct DynamicDocumentReference {
    /// What the named document type must promise about its documents leaving state.
    pub kind: DynamicDocumentReferenceKind,
    /// The top-level identifier property of the declaring type holding the id of the
    /// contract the referenced document type lives in.
    pub contract_id_property: String,
    /// The top-level string property of the declaring type holding the name of the
    /// referenced document type.
    pub document_type_property: String,
    /// What the named document type must look like (`documentTypeRequirements`); `None`
    /// asks nothing of it beyond its kind.
    pub requirements: Option<DocumentTypeRequirements>,
}

impl DynamicDocumentReference {
    /// `contractId` as a contract schema declares it: `"$self.<the property read>"`.
    pub fn contract_id_schema_value(&self) -> String {
        format!("{REFERRING_DOCUMENT_PREFIX}{}", self.contract_id_property)
    }

    /// `documentType` as a contract schema declares it: `"$self.<the property read>"`.
    pub fn document_type_schema_value(&self) -> String {
        format!("{REFERRING_DOCUMENT_PREFIX}{}", self.document_type_property)
    }

    /// The `refersTo` `type` the declaration is written with: one name, or the two of a
    /// list.
    pub fn wire_names(&self) -> &'static [&'static str] {
        self.kind.wire_names()
    }

    /// The properties of the declaring type the target is read from: the contract id, the
    /// document type name, and every property a slot takes its property name from. A
    /// replace that changes one of them points the reference somewhere else, so it is
    /// checked again.
    pub fn target_properties(&self) -> impl Iterator<Item = &str> {
        [
            self.contract_id_property.as_str(),
            self.document_type_property.as_str(),
        ]
        .into_iter()
        .chain(
            self.requirements
                .iter()
                .flat_map(|requirements| requirements.name_properties()),
        )
    }
}

/// `documentTypeRequirements`: named slots, each describing a top-level property the
/// referenced document type must have, and which of the slots must be filled.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Encode, Decode, DecodeUntrusted)]
pub struct DocumentTypeRequirements {
    /// The slots by name. A slot is filled by one property of the referenced type that
    /// matches its description.
    pub slots: BTreeMap<String, RequiredPropertySlot>,
    /// Which slots must be filled (`require`); `None` means every one of them.
    pub require: Option<SlotRequirement>,
    /// What the referenced type must say of itself, whatever fills the slots (`when`): a
    /// [`SlotCondition`] that reads no `$property`. A type it does not hold for does not
    /// meet the requirements. `None` asks nothing.
    pub when: Option<SlotCondition>,
}

/// One slot of a [`DocumentTypeRequirements`].
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Encode, Decode, DecodeUntrusted)]
pub struct RequiredPropertySlot {
    /// Which property may fill the slot (`name`).
    pub name: SlotPropertyName,
    /// What the property holds (`type`, with `byteArray` for a byte array).
    pub value: SlotValue,
    /// The most bytes a value of the property may ever take (`maxBytes`), for a string or a
    /// byte array slot: the property must declare a bound that is at most this one. `None`
    /// admits a property of any size.
    pub max_bytes: Option<u32>,
    /// The other slots this one must be filled by a different property than
    /// (`distinctFrom`). The relation is symmetric: naming it on one side is enough.
    pub distinct_from: BTreeSet<String>,
    /// What the referenced type must say of the property (`when`): a property the condition
    /// does not hold for does not fill the slot. `None` asks nothing.
    pub when: Option<SlotCondition>,
}

/// Which property of the referenced type may fill a slot.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Encode, Decode, DecodeUntrusted)]
pub enum SlotPropertyName {
    /// `"$any"`: any property matching the rest of the description.
    Any,
    /// A literal name: the property called this.
    Named(String),
    /// `"$self.<property>"`: the property the referring document names in this top-level
    /// string property of its own.
    FromProperty(String),
}

/// What a property filling a slot holds.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Encode, Decode, DecodeUntrusted)]
pub enum SlotValue {
    /// `"type": "string"`.
    String,
    /// `"type": "array"` with `"byteArray": true`, other than an identifier.
    ByteArray,
    /// `"type": "identifier"`.
    Identifier,
    /// `"type": "integer"`.
    Integer,
    /// `"type": "boolean"`.
    Boolean,
}

impl SlotValue {
    /// The `type` a contract schema declares the slot with; a byte array is an `array`
    /// declared beside `"byteArray": true`.
    pub const fn schema_type(self) -> &'static str {
        match self {
            SlotValue::String => "string",
            SlotValue::ByteArray => "array",
            SlotValue::Identifier => "identifier",
            SlotValue::Integer => "integer",
            SlotValue::Boolean => "boolean",
        }
    }

    /// The `type` the slot is written with, for messages.
    pub const fn wire_name(self) -> &'static str {
        match self {
            SlotValue::String => "string",
            SlotValue::ByteArray => "byte array",
            SlotValue::Identifier => "identifier",
            SlotValue::Integer => "integer",
            SlotValue::Boolean => "boolean",
        }
    }
}

/// `require`: which slots must be filled. One slot, or `anyOf` / `allOf` over terms, a term
/// being a slot or one group of slots joined by the other combinator. Two levels say every
/// condition `anyOf` and `allOf` can say of a set of slots, and keep the type from nesting
/// without bound in a decoder.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Encode, Decode, DecodeUntrusted)]
pub enum SlotRequirement {
    /// The slot must be filled.
    Slot(String),
    /// At least one term must hold; a group holds when all of its slots are filled.
    AnyOf(Vec<SlotRequirementTerm>),
    /// Every term must hold; a group holds when one of its slots is filled.
    AllOf(Vec<SlotRequirementTerm>),
}

/// A term of an `anyOf` or `allOf` [`SlotRequirement`].
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Encode, Decode, DecodeUntrusted)]
pub enum SlotRequirementTerm {
    /// One slot.
    Slot(String),
    /// A nested group of slots, joined by the combinator the enclosing one is not: `allOf`
    /// inside an `anyOf`, `anyOf` inside an `allOf`.
    Group(Vec<String>),
}

/// A schema object: its members in the order given.
fn schema_object(members: Vec<(&str, Value)>) -> Value {
    Value::Map(
        members
            .into_iter()
            .map(|(key, value)| (Value::Text(key.to_string()), value))
            .collect(),
    )
}

/// A schema list of names.
fn schema_names<'a>(names: impl IntoIterator<Item = &'a String>) -> Value {
    Value::Array(names.into_iter().cloned().map(Value::Text).collect())
}

impl SlotRequirement {
    /// `require` as a contract schema declares it: a slot name, or `anyOf` / `allOf` over
    /// slot names and groups, a group written as the other combinator. What the parser
    /// reads back to this requirement.
    pub fn to_schema_value(&self) -> Value {
        let (combinator, group_combinator, terms) = match self {
            SlotRequirement::Slot(slot) => return Value::Text(slot.clone()),
            SlotRequirement::AnyOf(terms) => ("anyOf", "allOf", terms),
            SlotRequirement::AllOf(terms) => ("allOf", "anyOf", terms),
        };
        let terms = terms
            .iter()
            .map(|term| match term {
                SlotRequirementTerm::Slot(slot) => Value::Text(slot.clone()),
                SlotRequirementTerm::Group(slots) => {
                    schema_object(vec![(group_combinator, schema_names(slots))])
                }
            })
            .collect();
        schema_object(vec![(combinator, Value::Array(terms))])
    }

    /// Whether the requirement holds when exactly the slots `filled` reports are filled.
    pub fn holds(&self, filled: impl Fn(&str) -> bool) -> bool {
        match self {
            SlotRequirement::Slot(slot) => filled(slot),
            SlotRequirement::AnyOf(terms) => terms.iter().any(|term| match term {
                SlotRequirementTerm::Slot(slot) => filled(slot),
                SlotRequirementTerm::Group(slots) => slots.iter().all(|slot| filled(slot)),
            }),
            SlotRequirement::AllOf(terms) => terms.iter().all(|term| match term {
                SlotRequirementTerm::Slot(slot) => filled(slot),
                SlotRequirementTerm::Group(slots) => slots.iter().any(|slot| filled(slot)),
            }),
        }
    }

    /// Every slot the requirement names, a slot named twice given twice.
    pub fn slots(&self) -> Vec<&str> {
        match self {
            SlotRequirement::Slot(slot) => vec![slot.as_str()],
            SlotRequirement::AnyOf(terms) | SlotRequirement::AllOf(terms) => terms
                .iter()
                .flat_map(|term| match term {
                    SlotRequirementTerm::Slot(slot) => vec![slot.as_str()],
                    SlotRequirementTerm::Group(slots) => slots.iter().map(String::as_str).collect(),
                })
                .collect(),
        }
    }
}

impl RequiredPropertySlot {
    /// The slot as a contract schema declares it: `name`, `type`, `byteArray` on a byte
    /// array, and `maxBytes`, `distinctFrom` (always a list) and `when` when it has them.
    /// What the parser reads back to this slot.
    pub fn to_schema_value(&self) -> Value {
        let name = match &self.name {
            SlotPropertyName::Any => Value::Text(ANY_PROPERTY_NAME.to_string()),
            SlotPropertyName::Named(name) => Value::Text(name.clone()),
            SlotPropertyName::FromProperty(property) => {
                Value::Text(format!("{REFERRING_DOCUMENT_PREFIX}{property}"))
            }
        };
        let mut members = vec![
            ("name", name),
            ("type", Value::Text(self.value.schema_type().to_string())),
        ];
        if self.value == SlotValue::ByteArray {
            members.push(("byteArray", Value::Bool(true)));
        }
        if let Some(max_bytes) = self.max_bytes {
            members.push(("maxBytes", Value::U32(max_bytes)));
        }
        if !self.distinct_from.is_empty() {
            members.push(("distinctFrom", schema_names(&self.distinct_from)));
        }
        if let Some(when) = &self.when {
            members.push(("when", when.to_schema_value()));
        }
        schema_object(members)
    }

    /// Whether `property`, a top-level property of the referenced type, matches the slot's
    /// description, its name and its `when` aside. A transient property is never stored, so
    /// it fills nothing.
    pub fn admits(&self, property: &DocumentProperty) -> bool {
        if property.transient {
            return false;
        }
        let within = |declared: Option<u32>| {
            self.max_bytes
                .is_none_or(|bound| declared.is_some_and(|declared| declared <= bound))
        };
        match (self.value, &property.property_type) {
            // A string holds at most its `maxBytes`, and at most four bytes a character of
            // its `maxLength`: the smaller of the two when it declares both
            (SlotValue::String, DocumentPropertyType::String(sizes)) => {
                let by_bytes = sizes.max_bytes.map(u32::from);
                let by_length = sizes
                    .max_length
                    .map(|characters| u32::from(characters).saturating_mul(4));
                within(match (by_bytes, by_length) {
                    (Some(by_bytes), Some(by_length)) => Some(by_bytes.min(by_length)),
                    (by_bytes, by_length) => by_bytes.or(by_length),
                })
            }
            (SlotValue::ByteArray, DocumentPropertyType::ByteArray(sizes)) => {
                within(sizes.max_size.map(u32::from))
            }
            (
                SlotValue::Identifier,
                DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_),
            ) => true,
            (SlotValue::Integer, property_type) => property_type.is_integer(),
            (SlotValue::Boolean, DocumentPropertyType::Boolean) => true,
            _ => false,
        }
    }
}

impl DocumentTypeRequirements {
    /// `documentTypeRequirements` as a contract schema declares it: the slots under
    /// `properties`, each as [`RequiredPropertySlot::to_schema_value`] gives it, and
    /// `require` when not every slot is required. The one place the declared shape is
    /// written out of the parsed one; the parser reads it back to these requirements.
    pub fn to_schema_value(&self) -> Value {
        let slots = Value::Map(
            self.slots
                .iter()
                .map(|(name, slot)| (Value::Text(name.clone()), slot.to_schema_value()))
                .collect(),
        );
        let mut members = vec![("properties", slots)];
        if let Some(require) = &self.require {
            members.push(("require", require.to_schema_value()));
        }
        if let Some(when) = &self.when {
            members.push(("when", when.to_schema_value()));
        }
        schema_object(members)
    }

    /// The properties of the declaring type a slot takes its property name from.
    pub fn name_properties(&self) -> impl Iterator<Item = &str> {
        self.slots.values().filter_map(|slot| match &slot.name {
            SlotPropertyName::FromProperty(property) => Some(property.as_str()),
            SlotPropertyName::Any | SlotPropertyName::Named(_) => None,
        })
    }

    /// How many comparisons judging `document_type` against the requirements takes, at
    /// most: what a write that checks the reference is billed for, before it is judged.
    ///
    /// Each slot tries every top-level property of the type when it takes any property
    /// (`"$any"`), and one when it names it. Trying a property is one comparison of it with
    /// the slot's description, and one more for each node of the slot's `when`
    /// ([`SlotCondition::node_count`]), whether or not the description admits it: the
    /// count depends on the declaration and the size of the type alone, not on what the
    /// type's properties are. The requirements' own `when`, judged once for the type, adds
    /// its nodes. The search for an assignment that follows is over at most
    /// `max_document_type_requirement_slots` slots and is not counted.
    pub fn comparisons(&self, document_type: DocumentTypeRef) -> u64 {
        let properties = document_type.properties().len() as u64;
        let of_the_type = self
            .when
            .as_ref()
            .map_or(0, |when| when.node_count() as u64);
        self.slots
            .values()
            .map(|slot| {
                let tried = match slot.name {
                    SlotPropertyName::Any => properties,
                    SlotPropertyName::Named(_) | SlotPropertyName::FromProperty(_) => 1,
                };
                let per_property = slot
                    .when
                    .as_ref()
                    .map_or(0, |when| when.node_count() as u64)
                    .saturating_add(1);
                tried.saturating_mul(per_property)
            })
            .fold(of_the_type, u64::saturating_add)
    }

    /// Whether `document_type` meets the requirements.
    ///
    /// A slot is filled by one top-level property of the type that its description admits
    /// and its `when` holds for: any such property for `"$any"`, the named one for a
    /// literal name, and for a name read from the referring document the property
    /// `name_from_property` gives for that referring property (`None` when the document
    /// leaves it out, which fills nothing).
    /// Slots that are `distinctFrom` each other are filled by different properties. The
    /// requirements are met when some choice of properties fills a set of slots `require`
    /// holds for, which is every slot when there is no `require`.
    ///
    /// The search tries the fullest sets of slots first and, within a set, the slot with the
    /// fewest candidates first. A set that cannot be filled has a group of slots with fewer
    /// candidates between them than slots, so with the number of slots held to
    /// `max_document_type_requirement_slots` the search stays small whatever the type
    /// declares.
    ///
    /// `elected_abilities` are the abilities the type's contract gives its elected
    /// moderation team on the type (`ElectedModerators::moderated_document_types`), `None`
    /// when the contract elects no team or leaves the type out: what `$electedAbilities`
    /// reads. The requirements' own `when` is judged first, for the type as a whole.
    pub fn met_by(
        &self,
        document_type: DocumentTypeRef,
        elected_abilities: Option<&BTreeSet<ModerationAbility>>,
        name_from_property: impl Fn(&str) -> Option<String>,
    ) -> bool {
        let properties = document_type.properties();
        let kind = document_type.document_reference_kind();
        let deletion_kept_fields = document_type.moderator_deletion_kept_fields();
        let moderator_changeable_fields = document_type.moderator_changeable_fields();
        let elected_abilities: BTreeSet<String> = elected_abilities
            .into_iter()
            .flatten()
            .map(|ability| ability.as_str().to_string())
            .collect();
        let settled_deletion = document_type.moderator_settled_deletion();
        // The type as a whole: no property is in hand, and its condition reads none
        let of_the_type = SlotConditionSubject {
            kind,
            property_name: "",
            deletion_kept_fields,
            moderator_changeable_fields,
            elected_abilities: &elected_abilities,
            deletion_window: document_type
                .documents_can_be_deleted_by_moderators_for()
                .is_some(),
            settled_deletion: settled_deletion.is_some(),
            settled_approvers_predate_document: settled_deletion
                .is_some_and(|rule| rule.approvers_predate_document),
        };
        if self
            .when
            .as_ref()
            .is_some_and(|when| !when.holds(&of_the_type))
        {
            return false;
        }
        let slots: Vec<(&str, &RequiredPropertySlot, Vec<&str>)> = self
            .slots
            .iter()
            .map(|(slot_name, slot)| {
                let fills = |name: &str, property: &DocumentProperty| {
                    slot.admits(property)
                        && slot.when.as_ref().is_none_or(|when| {
                            when.holds(&SlotConditionSubject {
                                property_name: name,
                                ..of_the_type
                            })
                        })
                };
                let named = |name: &str| {
                    properties
                        .get_key_value(name)
                        .filter(|(name, property)| fills(name, property))
                        .map(|(name, _)| vec![name.as_str()])
                        .unwrap_or_default()
                };
                let candidates = match &slot.name {
                    SlotPropertyName::Any => properties
                        .iter()
                        .filter(|(name, property)| fills(name, property))
                        .map(|(name, _)| name.as_str())
                        .collect(),
                    SlotPropertyName::Named(name) => named(name),
                    SlotPropertyName::FromProperty(referring_property) => {
                        name_from_property(referring_property)
                            .map(|name| named(&name))
                            .unwrap_or_default()
                    }
                };
                (slot_name.as_str(), slot, candidates)
            })
            .collect();

        // Slot sets as bit masks, the fullest first: `require` only ever asks for slots to
        // be filled, so a fuller set never does worse
        let slot_count = slots.len();
        let mut sets: Vec<u32> = (0..(1u32 << slot_count)).collect();
        sets.sort_by_key(|set| std::cmp::Reverse(set.count_ones()));
        sets.into_iter().any(|set| {
            let in_set = |index: usize| set & (1 << index) != 0;
            let required = match &self.require {
                None => set.count_ones() as usize == slot_count,
                Some(require) => require.holds(|slot_name| {
                    slots
                        .iter()
                        .position(|(name, _, _)| *name == slot_name)
                        .is_some_and(in_set)
                }),
            };
            if !required {
                return false;
            }
            let mut order: Vec<usize> = (0..slot_count).filter(|index| in_set(*index)).collect();
            order.sort_by_key(|index| slots[*index].2.len());
            fill(&slots, &order, &mut Vec::new())
        })
    }
}

/// Fills the slots `order` lists, in that order, with properties no slot distinct from them
/// holds; `filled` holds the slots filled so far with the property each took.
fn fill<'a>(
    slots: &[(&'a str, &'a RequiredPropertySlot, Vec<&'a str>)],
    order: &[usize],
    filled: &mut Vec<(usize, &'a str)>,
) -> bool {
    let Some((index, rest)) = order.split_first() else {
        return true;
    };
    let (slot_name, slot, candidates) = &slots[*index];
    for candidate in candidates {
        let taken = filled.iter().any(|(other, property)| {
            let (other_name, other_slot, _) = &slots[*other];
            property == candidate
                && (slot.distinct_from.contains(*other_name)
                    || other_slot.distinct_from.contains(*slot_name))
        });
        if taken {
            continue;
        }
        filled.push((*index, candidate));
        if fill(slots, rest, filled) {
            return true;
        }
        filled.pop();
    }
    false
}
