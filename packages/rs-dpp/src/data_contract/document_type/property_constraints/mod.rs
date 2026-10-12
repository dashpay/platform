//! The doctype-level `propertyConstraints` keyword (meta-schema v3, protocol
//! version 14): named rules every document of the type must meet, each a
//! condition on the document's properties: a comparison of two integer
//! expressions, a test of whether an integer expression takes one of listed
//! values (`in`), a comparison of a string or an identifier property with
//! constants (`equal`, `notEqual`, `in`) or with another property of its kind
//! (`equal`, `notEqual`), a test of whether a string, or a byte array, starts
//! or ends with another (`startsWith`, `endsWith`), a test of whether an array
//! property holds a value (`contains`), a test of whether the document holds a
//! property (`present`, `absent`), or `anyOf`, `allOf`, `not`, `ifThen` or
//! `ifThenElse` over conditions; `notIn` is an `in` negated.
//!
//! ```json
//! "propertyConstraints": {
//!   "depositCoversOrder": {
//!     "lessThanOrEqual": [
//!       { "multiply": [{ "add": ["price", "fee"] }, "quantity"] },
//!       "deposit"
//!     ]
//!   },
//!   "wholeLots": { "equal": [{ "modulo": ["quantity", 10] }, 0] },
//!   "minimumOrder": {
//!     "greaterThanOrEqual": [{ "multiply": ["price", { "ifAbsent": ["quantity", 1] }] }, 100]
//!   },
//!   "feeWaivedOrAtLeastTen": {
//!     "anyOf": [{ "equal": ["fee", 0] }, { "greaterThanOrEqual": ["fee", 10] }]
//!   },
//!   "discountGivenAboveZero": {
//!     "anyOf": [{ "absent": "discount" }, { "greaterThan": ["discount", 0] }]
//!   },
//!   "tieredFee": { "in": ["fee", [0, 10, 25, 50]] },
//!   "closedNeedsClosedAt": {
//!     "anyOf": [{ "notEqual": ["status", { "const": "closed" }] }, { "present": "closedAt" }]
//!   }
//! }
//! ```
//!
//! An operand is an integer value, the dotted path of an integer or boolean
//! property (a boolean reads as 1 for true and 0 for false), or an object with
//! one key: an arithmetic operator over its operands (`min`, `max` and `abs`
//! included), `ifAbsent`, a property
//! with the value it takes when the document leaves it out, or a size:
//! `length` and `byteLength`, the characters and the UTF-8 bytes of a string
//! property, and `count`, the items of an array or byte array property. A
//! property named on its own takes 0 when absent, and so does the size of one
//! (`{ "lessThanOrEqual": [{ "count": "tags" }, "maxTags"] }`).
//! `countPresent` is how many of two or more properties the document holds,
//! each as `present` tests it, so a rule bounds how many of a group are set:
//! `{ "equal": [{ "countPresent": ["email", "phone", "handle"] }, 1] }` holds
//! a document to exactly one, and an `in` listing `[1, 2]` to one or two.
//! `byteAt` reads one byte of a byte array property, 0 to 255, at a literal
//! index (`{ "in": [{ "byteAt": ["address", 0] }, [0, 1]] }`), and a byte the
//! array does not hold, or one of an array the document leaves out, reads as
//! 0, as a property left out does. A string constant is written
//! `{ "const": "closed" }`, since a string on its own is a path; `equal` and
//! `notEqual` compare one with a string property, or two bare paths naming
//! string properties with each other, and an `in` whose values are strings
//! lists them bare; `{ "ifAbsent": ["status", "open"] }` gives a string
//! property compared with strings a default. An identifier property compares
//! the same ways, its constants written base58, without defaults, and so does
//! `$ownerId`, the document's owner, which a transfer or a purchase changes.
//! A byte array property is tested only by `startsWith` and `endsWith`, against
//! a constant written in hex (`{ "const": "0001" }`) or another byte array
//! property, without defaults: one the document leaves out takes no bytes.
//! The block time and heights of the document's creation, last update and last
//! transfer are integer operands too (`"$createdAt"`, `"$updatedAtBlockHeight"`,
//! [`SystemProperty`]), on a document type that records them, so a price update
//! and a transfer or a purchase, which change some of them, are judged against
//! the rules reading those; and so is `$revision` ([`SystemValue::Revision`]),
//! on a document type that keeps a revision.
//! Under `$old.` ([`STORED_DOCUMENT_PREFIX`]) a path, `$ownerId`, a time, a
//! height or `$revision` reads the stored version of the document a write
//! replaces instead ([`DocumentSystemValues::stored`]), so a rule compares a
//! replace with what it replaces:
//! `{ "greaterThanOrEqual": ["counter", { "ifAbsent": ["$old.counter", 0] }] }`
//! keeps a counter from going down. A create has none, so every such read is
//! absent there ([`StoredVersion::Absent`]).
//! `countOf` and `sumOf` are integer operands read from state, [`AggregateRead`]:
//! how many documents of a type of the same contract match, or the total of an
//! integer property over them, from the count and sum trees their indexes keep
//! (`{ "countOf": ["listing", { "$ownerId": "$ownerId" }] }`), as the total will
//! be once the write is done. A filter may match by the document's own id
//! (`{ "countOf": ["vote", { "pollId": "$id" }] }`, the votes pointing at it).
//! Consensus reads them before judging the rules
//! ([`DocumentSystemValues::aggregates`]), and a total missing there is an
//! error; a client, which reads none, does not judge a rule reading one.
//!
//! The same grammar declares `deleteConstraints` ([`parse_delete_constraints`]):
//! named rules the stored document must meet for its owner to delete it, so
//! that a poll may be deleted only while no vote points at it.
//! How the arithmetic treats overflow, division and powers is set out on
//! [`ConstraintExpression::evaluate`], and how conditions combine on
//! [`PropertyConstraint::holds`].
//!
//! [`parse_property_constraints`] checks the declaration's shape on every
//! parse, [`MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH`] included. Which properties a
//! rule may read is checked against the parsed document type by parser
//! generation 3, and the limits on the rules, and that no `anyOf` or `allOf`
//! repeats a condition, under full validation only.
//! Nothing here is serialized: a document type rebuilds its rules from its
//! stored schema whenever the contract is loaded.

mod aggregate;
#[cfg(test)]
mod tests;

use crate::block::block_info::BlockInfo;
use crate::consensus::basic::document::PropertyConstraintViolation;
use crate::data_contract::document_type::property::is_referenced_system_agreement_property;
use crate::data_contract::document_type::property_names;
use crate::data_contract::errors::DataContractError;
use crate::document::property_names::{
    CREATED_AT, CREATED_AT_BLOCK_HEIGHT, CREATED_AT_CORE_BLOCK_HEIGHT, CREATOR_ID,
    ID as DOCUMENT_ID, OWNER_ID, REVISION, TRANSFERRED_AT, TRANSFERRED_AT_BLOCK_HEIGHT,
    TRANSFERRED_AT_CORE_BLOCK_HEIGHT, UPDATED_AT, UPDATED_AT_BLOCK_HEIGHT,
    UPDATED_AT_CORE_BLOCK_HEIGHT,
};
use crate::document::{Document, DocumentV0Getters, INITIAL_REVISION};
use crate::prelude::{BlockHeight, CoreBlockHeight, Revision, TimestampMillis};
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use platform_value::string_encoding::Encoding;
use platform_value::{Identifier, Value, ValueMapHelper};
use serde::{Serialize, Serializer};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::ops::ControlFlow;

/// The operand key naming a property together with the value it takes when
/// the document leaves it out: `{ "ifAbsent": ["quantity", 1] }`.
pub const IF_ABSENT: &str = "ifAbsent";
const ADD: &str = "add";
const SUBTRACT: &str = "subtract";
const MULTIPLY: &str = "multiply";
const DIVIDE: &str = "divide";
const MODULO: &str = "modulo";
const POWER: &str = "power";
const ANY_OF: &str = "anyOf";
const ALL_OF: &str = "allOf";
const NOT: &str = "not";
const IF_THEN: &str = "ifThen";
const IF_THEN_ELSE: &str = "ifThenElse";
const NOT_IN: &str = "notIn";
const MIN: &str = "min";
const MAX: &str = "max";
const ABS: &str = "abs";
const COUNT_OF: &str = "countOf";
const SUM_OF: &str = "sumOf";
const PRESENT: &str = "present";
const ABSENT: &str = "absent";
const IN: &str = "in";
const CONTAINS: &str = "contains";
const STARTS_WITH: &str = "startsWith";
const ENDS_WITH: &str = "endsWith";
const LENGTH: &str = "length";
const BYTE_LENGTH: &str = "byteLength";
const COUNT: &str = "count";
const COUNT_PRESENT: &str = "countPresent";
const BYTE_AT: &str = "byteAt";
/// The operand key of a string constant: `{ "const": "closed" }`.
const CONST: &str = "const";

/// Every key an operand object may hold, for the errors.
const OPERAND_KEYS: &str = "add, subtract, multiply, divide, modulo, power, min, max, abs, \
                            ifAbsent, length, byteLength, count, countPresent, byteAt, countOf, \
                            sumOf or referenced";

/// The deepest a condition or an operand may sit in its rule: the rule's own
/// condition at depth 0, and each operand of a comparison, and each condition
/// under `anyOf`, `allOf` or `not`, one level deeper than what holds it.
/// Checked on every parse, stored contracts included, so that a declaration
/// handed to a parse without full validation cannot drive the parser, or the
/// evaluation of what it builds, into unbounded recursion. A registrable rule
/// stays far below it: it has at most
/// `SystemLimits::max_property_constraint_nodes` nodes, so it is never deeper
/// than that (a test holds every protocol version's limit to it). A constant
/// rather than a limit, like `MAX_REFERENCE_EXPRESSION_DECODE_DEPTH`, so that
/// no change to a limit can make a stored contract unparseable.
pub const MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH: usize = 64;

/// The prefix through which a rule, or the condition of an `immutable` entry or
/// of `retractedWhen`, reads the stored version of the document a write
/// replaces instead of the version it writes: `$old.status` is the stored
/// `status`, `$old.$ownerId` ([`STORED_OWNER_ID`]) the stored owner, and
/// `$old.$updatedAt` or `$old.$revision` ([`SystemRead`]) a stored system
/// value. A rule is judged against the written document's properties with the
/// stored ones under [`STORED_DOCUMENT_KEY`] ([`DocumentSystemValues::judged_data`]),
/// and the stored owner, times, heights and revision in
/// [`DocumentSystemValues::stored`]. A create has no stored version, so there
/// every such read is absent ([`StoredVersion::Absent`]). A `deleteConstraints`
/// rule, judged on the stored document itself, reads none.
pub const STORED_DOCUMENT_PREFIX: &str = "$old.";

/// The key the stored document's properties sit under in the data a rule or a
/// condition is judged against, so that a path through
/// [`STORED_DOCUMENT_PREFIX`] reads them. No property can take the name: a
/// property name is letters, digits and underscores.
pub const STORED_DOCUMENT_KEY: &str = "$old";

/// `$old.$ownerId`: the owner of the stored version of the document, which a
/// comparison of identifiers reads as it reads `$ownerId`. It differs from
/// `$ownerId` only on a transfer or a purchase, where it is the previous owner.
pub const STORED_OWNER_ID: &str = "$old.$ownerId";

/// What `path` reads of the stored version, the part after
/// [`STORED_DOCUMENT_PREFIX`], when it reads the stored version: `price` for
/// `$old.price`, `$ownerId` for `$old.$ownerId`. Every reading of a path's
/// version goes through it.
pub fn stored_version_path(path: &str) -> Option<&str> {
    path.strip_prefix(STORED_DOCUMENT_PREFIX)
}

/// `path` as a path of the document, whichever version it reads: what it reads
/// of the stored version ([`stored_version_path`]), or `path` itself.
pub fn either_version_path(path: &str) -> &str {
    stored_version_path(path).unwrap_or(path)
}
/// The operand key of a value of the document a reference of the type points
/// at: `{ "referenced": ["pollId", "endsAt"] }` is the `endsAt` of the
/// document the property `pollId` refers to. It stands wherever a property
/// path does, and is read as the path `$referenced.pollId.$.endsAt`
/// ([`referenced_path`]).
pub const REFERENCED: &str = "referenced";

/// The key the values of the documents the type's references point at sit
/// under in the data a `propertyConstraints` rule reading one is judged
/// against, by the reference property, so that a path through
/// [`REFERENCED_DOCUMENT_PREFIX`] reads them. Consensus puts them there once
/// the references are checked, from the documents that check read; a rule
/// reading one is not judged without them
/// ([`DocumentSystemValues::referenced_documents`]).
pub const REFERENCED_DOCUMENT_KEY: &str = "$referenced";

/// The prefix of the path a `referenced` operand is read through:
/// `$referenced.<reference property>.$.<field>`.
pub const REFERENCED_DOCUMENT_PREFIX: &str = "$referenced.";

/// The segment between the reference property, which may be a dotted path
/// (`meta.pollId`), and the field of the document it points at, which may be
/// one too, in the path a `referenced` operand is read through: no property is
/// named `$`.
const REFERENCED_FIELD_SEPARATOR: &str = ".$.";

/// The reference property and the field of the document it points at that the
/// path a `referenced` operand is read through names: `("meta.pollId",
/// "endsAt")` for `$referenced.meta.pollId.$.endsAt`. `None` for any other
/// path.
pub fn referenced_read(path: &str) -> Option<(&str, &str)> {
    path.strip_prefix(REFERENCED_DOCUMENT_PREFIX)?
        .split_once(REFERENCED_FIELD_SEPARATOR)
}

/// The path a `referenced` operand reading `field` through `reference` is read
/// through ([`REFERENCED_DOCUMENT_PREFIX`]).
pub fn referenced_path(reference: &str, field: &str) -> String {
    format!("{REFERENCED_DOCUMENT_PREFIX}{reference}{REFERENCED_FIELD_SEPARATOR}{field}")
}

/// What `rules` read of the documents the type's references point at: by
/// reference property, the fields read of the document it points at. Empty when
/// no rule reads one. A reference listed here is checked again on every replace,
/// since every replace judges the rules.
pub fn referenced_fields(
    rules: &BTreeMap<String, PropertyConstraint>,
) -> BTreeMap<&str, BTreeSet<&str>> {
    let mut fields: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for rule in rules.values() {
        for (reference, field, _) in rule.referenced_reads() {
            fields.entry(reference).or_default().insert(field);
        }
    }
    fields
}

/// The values `fields` names of `document`, a document a reference points at,
/// as a map a rule reads through [`REFERENCED_DOCUMENT_PREFIX`] once it is put
/// under the reference property ([`data_with_referenced_documents`]): a schema
/// property at its dotted path, `$id`, `$ownerId` and `$creatorId` as
/// identifiers, a time or height as an integer. A field the document does not
/// hold is left out, so a rule reads it as absent.
pub fn referenced_document_values<'a>(
    document: &Document,
    fields: impl IntoIterator<Item = &'a str>,
) -> Result<Value, platform_value::Error> {
    let mut values = Value::Map(Vec::new());
    for field in fields {
        if let Some(value) = referenced_document_value(document, field)? {
            values.set_value_at_full_path(field, value.into_owned())?;
        }
    }
    Ok(values)
}

/// The value `field` names of `document`, a document a reference points at,
/// as a rule or a `where` entry reads it: a schema property at its dotted path,
/// `$id`, `$ownerId` and `$creatorId` as identifiers, a time or height as an
/// integer. `None` when the document holds none there; an error when a value
/// on the way down a dotted path is no object.
pub fn referenced_document_value<'a>(
    document: &'a Document,
    field: &str,
) -> Result<Option<Cow<'a, Value>>, platform_value::Error> {
    Ok(match field {
        DOCUMENT_ID => Some(Cow::Owned(Value::Identifier(document.id().to_buffer()))),
        OWNER_ID => Some(Cow::Owned(Value::Identifier(
            document.owner_id().to_buffer(),
        ))),
        CREATOR_ID => document
            .creator_id()
            .map(|creator_id| Cow::Owned(Value::Identifier(creator_id.to_buffer()))),
        field => match SystemProperty::from_name(field) {
            Some(property) => DocumentSystemValues::of_document(document)
                .value(property)
                .map(|value| Cow::Owned(Value::I128(value))),
            None => document
                .properties()
                .get_optional_at_path(field)?
                .map(Cow::Borrowed),
        },
    })
}

/// The data a rule reading a referenced document is judged against: the
/// document's properties, `data`, with `referenced`, each reference property and
/// the values its rules read of the document it points at
/// ([`referenced_document_values`]), where [`referenced_path`] reads them, under
/// [`REFERENCED_DOCUMENT_KEY`]. A reference the document leaves out has no
/// entry, and reads as absent.
pub fn data_with_referenced_documents(
    data: &BTreeMap<String, Value>,
    referenced: BTreeMap<String, Value>,
) -> Result<Value, platform_value::Error> {
    let mut documents = Value::Map(Vec::new());
    for (reference, values) in referenced {
        // The path of the separator segment, without the leading `$referenced.`
        let at = format!("{reference}{REFERENCED_FIELD_SEPARATOR}");
        documents.set_value_at_full_path(at.trim_end_matches('.'), values)?;
    }
    Ok(with_reserved_entry(
        data.iter()
            .map(|(property, value)| (Value::Text(property.clone()), value.clone())),
        REFERENCED_DOCUMENT_KEY,
        documents,
    ))
}

/// The data a rule reading the stored version's properties is judged against:
/// the document's properties, `data` (a map), with `stored`, the stored
/// version's, under [`STORED_DOCUMENT_KEY`], where a path through
/// [`STORED_DOCUMENT_PREFIX`] reads them.
pub fn data_with_stored_properties(data: Value, stored: Value) -> Value {
    let entries = match data {
        Value::Map(entries) => entries,
        _ => Vec::new(),
    };
    with_reserved_entry(entries.into_iter(), STORED_DOCUMENT_KEY, stored)
}

/// The map of `entries` with `value` under the reserved key `key`, which an
/// entry of `entries` holding the key itself does not keep: no document
/// property takes a name starting with `$`, and the schema validation refuses
/// one a type does not declare, so such an entry is never read as what the key
/// holds.
fn with_reserved_entry(
    entries: impl Iterator<Item = (Value, Value)>,
    key: &str,
    value: Value,
) -> Value {
    let mut entries: Vec<(Value, Value)> = entries
        .filter(|(entry_key, _)| entry_key.as_text() != Some(key))
        .collect();
    entries.push((Value::Text(key.to_string()), value));
    Value::Map(entries)
}

/// Whether `text` spells `identifier` in base58, as a constant compared with an
/// identifier is written: decoded in place, 32 bytes exactly.
pub(crate) fn spells_identifier(text: &str, identifier: &[u8; 32]) -> bool {
    let mut decoded = [0u8; 32];
    matches!(bs58::decode(text).onto(&mut decoded), Ok(32)) && decoded == *identifier
}

/// Whether `text` spells `bytes` in hex, two digits of either case a byte, as a
/// constant compared with a byte array is written: read beside the bytes.
pub(crate) fn spells_bytes(text: &str, bytes: &[u8]) -> bool {
    text.len() == 2 * bytes.len()
        && bytes.iter().enumerate().all(|(index, byte)| {
            text.get(2 * index..2 * index + 2)
                .filter(|digits| digits.bytes().all(|digit| digit.is_ascii_hexdigit()))
                .and_then(|digits| u8::from_str_radix(digits, 16).ok())
                == Some(*byte)
        })
}

/// Whether `value`, a value of a referenced document a rule reads, is the one
/// the string `constant` spells: a string itself, an identifier in base58, a
/// byte array in hex. The parse of a rule does not know what a referenced
/// field holds (its type may be another contract's), so a string constant
/// compared with one reads as the field's value says; registration checks the
/// constant spells a value of the field's type. Consensus reads the value from
/// a stored document or removal record, decoded under the referenced type: an
/// identifier as `Value::Identifier`, a byte array as one of the bytes values,
/// never the other.
fn spelled_by(value: &Value, constant: &str) -> bool {
    match value {
        Value::Text(text) => text == constant,
        Value::Identifier(identifier) => spells_identifier(constant, identifier),
        value => value
            .as_bytes_slice()
            .is_ok_and(|bytes| spells_bytes(constant, bytes)),
    }
}

/// How the two sides of a comparison must compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintComparison {
    /// `equal`: the two sides are the same number.
    Equal,
    /// `notEqual`: the two sides differ.
    NotEqual,
    /// `lessThan`: the left side is below the right one.
    LessThan,
    /// `lessThanOrEqual`: the left side is not above the right one.
    LessThanOrEqual,
    /// `greaterThan`: the left side is above the right one.
    GreaterThan,
    /// `greaterThanOrEqual`: the left side is not below the right one.
    GreaterThanOrEqual,
}

impl ConstraintComparison {
    /// Every comparison.
    pub const ALL: [ConstraintComparison; 6] = [
        ConstraintComparison::Equal,
        ConstraintComparison::NotEqual,
        ConstraintComparison::LessThan,
        ConstraintComparison::LessThanOrEqual,
        ConstraintComparison::GreaterThan,
        ConstraintComparison::GreaterThanOrEqual,
    ];

    /// The schema key declaring it.
    pub fn wire_name(self) -> &'static str {
        match self {
            ConstraintComparison::Equal => "equal",
            ConstraintComparison::NotEqual => "notEqual",
            ConstraintComparison::LessThan => "lessThan",
            ConstraintComparison::LessThanOrEqual => "lessThanOrEqual",
            ConstraintComparison::GreaterThan => "greaterThan",
            ConstraintComparison::GreaterThanOrEqual => "greaterThanOrEqual",
        }
    }

    /// Whether `left` and `right` compare this way.
    pub fn holds(self, left: i128, right: i128) -> bool {
        match self {
            ConstraintComparison::Equal => left == right,
            ConstraintComparison::NotEqual => left != right,
            ConstraintComparison::LessThan => left < right,
            ConstraintComparison::LessThanOrEqual => left <= right,
            ConstraintComparison::GreaterThan => left > right,
            ConstraintComparison::GreaterThanOrEqual => left >= right,
        }
    }
}

/// A system property of a document a rule may read as an integer operand, by
/// its name (`"$createdAt"`): the block time, in milliseconds, the Platform
/// block height or the Core chain block height of the document's creation, of
/// its last update (a create, a replace or a price update) or of its last
/// transfer (a create, a transfer or a purchase). A rule may read one only on a
/// document type that records it, by listing it in `required`, so every stored
/// document of the type holds it. A `where` entry of a `refersTo` may bound an
/// ordering by one too (`WhereOrderingBound::System`), which embeds it in the reference
/// errors, so it encodes.
// @append_only
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode, DecodeUntrusted,
)]
pub enum SystemProperty {
    /// `$createdAt`
    CreatedAt,
    /// `$updatedAt`
    UpdatedAt,
    /// `$transferredAt`
    TransferredAt,
    /// `$createdAtBlockHeight`
    CreatedAtBlockHeight,
    /// `$updatedAtBlockHeight`
    UpdatedAtBlockHeight,
    /// `$transferredAtBlockHeight`
    TransferredAtBlockHeight,
    /// `$createdAtCoreBlockHeight`
    CreatedAtCoreBlockHeight,
    /// `$updatedAtCoreBlockHeight`
    UpdatedAtCoreBlockHeight,
    /// `$transferredAtCoreBlockHeight`
    TransferredAtCoreBlockHeight,
}

impl SystemProperty {
    /// Every system property a rule may read.
    pub const ALL: [SystemProperty; 9] = [
        SystemProperty::CreatedAt,
        SystemProperty::UpdatedAt,
        SystemProperty::TransferredAt,
        SystemProperty::CreatedAtBlockHeight,
        SystemProperty::UpdatedAtBlockHeight,
        SystemProperty::TransferredAtBlockHeight,
        SystemProperty::CreatedAtCoreBlockHeight,
        SystemProperty::UpdatedAtCoreBlockHeight,
        SystemProperty::TransferredAtCoreBlockHeight,
    ];

    /// Its name, as a rule and `required` write it.
    pub fn name(self) -> &'static str {
        match self {
            SystemProperty::CreatedAt => CREATED_AT,
            SystemProperty::UpdatedAt => UPDATED_AT,
            SystemProperty::TransferredAt => TRANSFERRED_AT,
            SystemProperty::CreatedAtBlockHeight => CREATED_AT_BLOCK_HEIGHT,
            SystemProperty::UpdatedAtBlockHeight => UPDATED_AT_BLOCK_HEIGHT,
            SystemProperty::TransferredAtBlockHeight => TRANSFERRED_AT_BLOCK_HEIGHT,
            SystemProperty::CreatedAtCoreBlockHeight => CREATED_AT_CORE_BLOCK_HEIGHT,
            SystemProperty::UpdatedAtCoreBlockHeight => UPDATED_AT_CORE_BLOCK_HEIGHT,
            SystemProperty::TransferredAtCoreBlockHeight => TRANSFERRED_AT_CORE_BLOCK_HEIGHT,
        }
    }

    /// The system property a rule names `name`, if any.
    pub fn from_name(name: &str) -> Option<SystemProperty> {
        SystemProperty::ALL
            .into_iter()
            .find(|property| property.name() == name)
    }

    /// Whether `change` sets it.
    pub fn changed_by(self, change: SystemChange) -> bool {
        match change {
            SystemChange::Transfer => matches!(
                self,
                SystemProperty::TransferredAt
                    | SystemProperty::TransferredAtBlockHeight
                    | SystemProperty::TransferredAtCoreBlockHeight
            ),
            SystemChange::PriceUpdate => matches!(
                self,
                SystemProperty::UpdatedAt
                    | SystemProperty::UpdatedAtBlockHeight
                    | SystemProperty::UpdatedAtCoreBlockHeight
            ),
            SystemChange::Restore => false,
        }
    }
}

/// Serialized as a rule writes it, `"$createdAt"`.
impl Serialize for SystemProperty {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

/// A write that changes none of a stored document's properties, so only the
/// rules reading what it does change can break: the others held when the
/// document was last written and still do, since a type's rules never change
/// once it is registered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemChange {
    /// A transfer or a purchase: a new owner, the time and heights of the last
    /// transfer, and the next revision.
    Transfer,
    /// A price update: the time and heights of the last update, and the next
    /// revision.
    PriceUpdate,
    /// A moderator's restore of a document its moderators deleted: the document
    /// comes back as it was, owner, times, heights and revision included, but
    /// the `countOf` and `sumOf` totals around it may have moved while it was
    /// gone.
    Restore,
}

/// A system value of a document a rule may read as an integer operand: one of
/// the times and heights [`SystemProperty`] names, or `$revision`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SystemValue {
    /// A time or a height, on a document type that records it.
    Property(SystemProperty),
    /// `$revision`: 1 for a created document, and one more for each later write
    /// of it (a replace, a transfer, a purchase, a price update, a moderator's
    /// change of its fields), on a document type that keeps a revision: one
    /// whose documents are mutable, transferable, sold at a price their owner
    /// sets, or keep fields for their moderators.
    Revision,
}

impl SystemValue {
    /// Its name, as a rule writes it.
    pub fn name(self) -> &'static str {
        match self {
            SystemValue::Property(property) => property.name(),
            SystemValue::Revision => REVISION,
        }
    }

    /// The system value a rule names `name`, if any.
    pub fn from_name(name: &str) -> Option<SystemValue> {
        if name == REVISION {
            return Some(SystemValue::Revision);
        }
        SystemProperty::from_name(name).map(SystemValue::Property)
    }

    /// Whether `change` sets it.
    pub fn changed_by(self, change: SystemChange) -> bool {
        match self {
            SystemValue::Property(property) => property.changed_by(change),
            SystemValue::Revision => {
                matches!(change, SystemChange::Transfer | SystemChange::PriceUpdate)
            }
        }
    }
}

/// A system value a rule reads: of the document version the write stores
/// (`"$updatedAt"`), or, with `stored`, of the stored version it replaces,
/// through [`STORED_DOCUMENT_PREFIX`] (`"$old.$updatedAt"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SystemRead {
    pub value: SystemValue,
    pub stored: bool,
}

impl From<SystemProperty> for SystemRead {
    /// `property` of the document version written.
    fn from(property: SystemProperty) -> Self {
        SystemRead {
            value: SystemValue::Property(property),
            stored: false,
        }
    }
}

impl SystemRead {
    /// Its name, as a rule writes it: `$updatedAt`, `$old.$revision`.
    pub fn name(self) -> String {
        if self.stored {
            format!("{STORED_DOCUMENT_PREFIX}{}", self.value.name())
        } else {
            self.value.name().to_string()
        }
    }

    /// The system value a rule names `name`, of either version, if any.
    pub fn from_name(name: &str) -> Option<SystemRead> {
        match stored_version_path(name) {
            Some(stored) => SystemValue::from_name(stored).map(|value| SystemRead {
                value,
                stored: true,
            }),
            None => SystemValue::from_name(name).map(|value| SystemRead {
                value,
                stored: false,
            }),
        }
    }
}

/// The system values of the document version a rule is judged against: its
/// owner, which `$ownerId` reads, the times and heights [`SystemProperty`]
/// names and its revision, and what a rule reads beyond the document: the
/// totals of other documents and the stored version the write replaces.
/// Consensus passes every one the document type records; a client passes
/// those it knows. `$ownerId` equals no identifier when the owner is unknown,
/// and [`PropertyConstraint::violation`] does not judge a rule reading a time,
/// a height, a revision, an aggregate or a stored version it is not given.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DocumentSystemValues {
    pub owner_id: Option<Identifier>,
    pub created_at: Option<TimestampMillis>,
    pub updated_at: Option<TimestampMillis>,
    pub transferred_at: Option<TimestampMillis>,
    pub created_at_block_height: Option<BlockHeight>,
    pub updated_at_block_height: Option<BlockHeight>,
    pub transferred_at_block_height: Option<BlockHeight>,
    pub created_at_core_block_height: Option<CoreBlockHeight>,
    pub updated_at_core_block_height: Option<CoreBlockHeight>,
    pub transferred_at_core_block_height: Option<CoreBlockHeight>,
    /// `$revision` ([`SystemValue::Revision`]).
    pub revision: Option<Revision>,
    /// The `countOf` and `sumOf` totals the rules read, each as it will be once
    /// the write is done ([`AggregateRead`]). `None` for a client, which reads
    /// no state: a rule reading a total is then not judged. Consensus reads
    /// every total the rules judging the write read ([`Self::consensus`]).
    pub aggregates: Option<BTreeMap<AggregateRead, i128>>,
    /// Whether consensus judges the write, and so gives every total and the
    /// stored version the rules judging it read: one missing is then a fault in
    /// the code building the write, which the judging reports as an error
    /// rather than skip the rule ([`PropertyConstraint::unread_aggregate`],
    /// [`PropertyConstraint::unread_stored_version`]). `false` for a client,
    /// which skips a rule reading what it does not know.
    pub consensus: bool,
    /// Whether the data the rules are judged against holds, under
    /// [`REFERENCED_DOCUMENT_KEY`], the values they read of the documents the
    /// type's references point at. Consensus sets it once the references are
    /// checked, and judges then the rules reading one; anywhere else a rule
    /// reading one is not judged ([`PropertyConstraint::violation`]).
    pub referenced_documents: bool,
    /// The version of the document the write replaces, which a rule reads
    /// through [`STORED_DOCUMENT_PREFIX`]: [`StoredVersion::Absent`] for a
    /// create, the stored version for any other write. `None` for a client not
    /// given it: a rule reading it is then not judged. Consensus gives it
    /// whenever a rule judging the write reads it ([`Self::consensus`]).
    pub stored: Option<StoredVersion>,
    /// `Some` for a write that changes only these top-level properties of a
    /// stored document and nothing else a rule reads but `$revision`: a
    /// moderator's change of a document's fields. It is judged only against
    /// the rules reading one of them, directly or through `$old.`, `$revision`
    /// or a total ([`PropertyConstraint::reads_changed_fields`]); every other
    /// rule reads what the document held when it was last written, and gave
    /// its verdict then. `None` for every other write, judged against every
    /// rule.
    pub changed_fields: Option<BTreeSet<String>>,
}

/// The stored version of the document a write replaces, as a rule reads it
/// through [`STORED_DOCUMENT_PREFIX`].
#[derive(Debug, Clone, PartialEq)]
pub enum StoredVersion {
    /// There is none: the write is a create. A path through `$old.` reads as a
    /// property the document leaves out does (0, or its `ifAbsent` value;
    /// absent to `present`; equal to no string or identifier), `$old.$ownerId`
    /// equals no identifier, and a stored time, height or revision reads 0.
    Absent,
    /// The version the write replaces.
    Present(Box<StoredDocument>),
}

impl StoredVersion {
    /// `document`, the version a write replaces.
    pub fn of_document(document: &Document) -> Self {
        StoredVersion::Present(Box::new(StoredDocument::of_document(document)))
    }

    /// The version a write replaces, given by its owner, times, heights and
    /// revision, `system`, alone ([`StoredDocument::without_properties`]).
    pub fn without_properties(system: StoredSystemValues) -> Self {
        StoredVersion::Present(Box::new(StoredDocument::without_properties(system)))
    }
}

/// A stored version of a document, as a rule reads it through `$old.`.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredDocument {
    /// Its properties, which `$old.<path>` reads. `None` when not given: no
    /// rule judged with it reads one, which the judging checks
    /// ([`DocumentSystemValues::stored_properties_given`]), or the write changes
    /// no property (a transfer, a purchase, a price update, a restore), so that
    /// they are the properties written, which
    /// `validate_property_constraints_for_system_change` reads instead.
    pub properties: Option<BTreeMap<String, Value>>,
    /// Its owner, times, heights and revision, which `$old.$ownerId`,
    /// `$old.$createdAt` and the rest read.
    pub system: StoredSystemValues,
}

impl StoredDocument {
    /// What `document` holds.
    pub fn of_document(document: &Document) -> Self {
        StoredDocument {
            properties: Some(document.properties().clone()),
            system: StoredSystemValues::of_document(document),
        }
    }

    /// What `document` holds, its properties taken rather than copied.
    pub fn of_owned_document(document: Document) -> Self {
        let system = StoredSystemValues::of_document(&document);
        StoredDocument {
            properties: Some(document.properties_consumed()),
            system,
        }
    }

    /// Its owner, times, heights and revision, `system`, without its
    /// properties ([`Self::properties`]).
    pub fn without_properties(system: StoredSystemValues) -> Self {
        StoredDocument {
            properties: None,
            system,
        }
    }
}

/// The owner, times, heights and revision of a stored version of a document,
/// as `$old.$ownerId`, `$old.$createdAt` and the rest read them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StoredSystemValues {
    pub owner_id: Identifier,
    pub created_at: Option<TimestampMillis>,
    pub updated_at: Option<TimestampMillis>,
    pub transferred_at: Option<TimestampMillis>,
    pub created_at_block_height: Option<BlockHeight>,
    pub updated_at_block_height: Option<BlockHeight>,
    pub transferred_at_block_height: Option<BlockHeight>,
    pub created_at_core_block_height: Option<CoreBlockHeight>,
    pub updated_at_core_block_height: Option<CoreBlockHeight>,
    pub transferred_at_core_block_height: Option<CoreBlockHeight>,
    pub revision: Option<Revision>,
}

impl StoredSystemValues {
    /// What `document` holds.
    pub fn of_document(document: &Document) -> Self {
        StoredSystemValues {
            owner_id: document.owner_id(),
            created_at: document.created_at(),
            updated_at: document.updated_at(),
            transferred_at: document.transferred_at(),
            created_at_block_height: document.created_at_block_height(),
            updated_at_block_height: document.updated_at_block_height(),
            transferred_at_block_height: document.transferred_at_block_height(),
            created_at_core_block_height: document.created_at_core_block_height(),
            updated_at_core_block_height: document.updated_at_core_block_height(),
            transferred_at_core_block_height: document.transferred_at_core_block_height(),
            revision: document.revision(),
        }
    }

    /// The system values of a written version holding them: the same owner,
    /// times, heights and revision, and nothing a rule reads beyond the
    /// document.
    pub fn as_written(&self) -> DocumentSystemValues {
        DocumentSystemValues {
            owner_id: Some(self.owner_id),
            created_at: self.created_at,
            updated_at: self.updated_at,
            transferred_at: self.transferred_at,
            created_at_block_height: self.created_at_block_height,
            updated_at_block_height: self.updated_at_block_height,
            transferred_at_block_height: self.transferred_at_block_height,
            created_at_core_block_height: self.created_at_core_block_height,
            updated_at_core_block_height: self.updated_at_core_block_height,
            transferred_at_core_block_height: self.transferred_at_core_block_height,
            revision: self.revision,
            ..Default::default()
        }
    }
}

impl DocumentSystemValues {
    /// The owner alone, no time, height or revision.
    pub fn owned_by(owner_id: Identifier) -> Self {
        DocumentSystemValues {
            owner_id: Some(owner_id),
            ..Default::default()
        }
    }

    /// A document created by `owner_id` in the block `block_info` describes:
    /// its creation, last update and last transfer all at that block, as a
    /// create records every one its type requires, its first revision, which
    /// a rule reads only on a type keeping one, and no stored version.
    pub fn created_in_block(owner_id: Identifier, block_info: &BlockInfo) -> Self {
        DocumentSystemValues {
            owner_id: Some(owner_id),
            created_at: Some(block_info.time_ms),
            updated_at: Some(block_info.time_ms),
            transferred_at: Some(block_info.time_ms),
            created_at_block_height: Some(block_info.height),
            updated_at_block_height: Some(block_info.height),
            transferred_at_block_height: Some(block_info.height),
            created_at_core_block_height: Some(block_info.core_height),
            updated_at_core_block_height: Some(block_info.core_height),
            transferred_at_core_block_height: Some(block_info.core_height),
            revision: Some(INITIAL_REVISION),
            aggregates: None,
            consensus: false,
            stored: Some(StoredVersion::Absent),
            changed_fields: None,
            referenced_documents: false,
        }
    }

    /// The values `document` holds; no stored version.
    pub fn of_document(document: &Document) -> Self {
        DocumentSystemValues {
            stored: None,
            ..StoredSystemValues::of_document(document).as_written()
        }
    }

    /// The value `read` reads, `None` when not given: one of the stored
    /// version is 0 on a create, and not given when the stored version is not.
    pub fn read(&self, read: SystemRead) -> Option<i128> {
        if read.stored {
            return match &self.stored {
                Some(StoredVersion::Present(stored)) => {
                    stored.system.as_written().version_value(read.value)
                }
                Some(StoredVersion::Absent) => Some(0),
                None => None,
            };
        }
        self.version_value(read.value)
    }

    /// The value of `value` of this version, `None` when not given.
    fn version_value(&self, value: SystemValue) -> Option<i128> {
        match value {
            SystemValue::Property(property) => self.value(property),
            SystemValue::Revision => self.revision.map(i128::from),
        }
    }

    /// Whether the stored version's properties are given whenever the stored
    /// version is: `false` only for a stored version given without them
    /// ([`StoredDocument::without_properties`]).
    pub fn stored_properties_given(&self) -> bool {
        !matches!(
            &self.stored,
            Some(StoredVersion::Present(stored)) if stored.properties.is_none()
        )
    }

    /// The owner of the stored version, which `$old.$ownerId` reads: `None` on
    /// a create, and when the stored version is not given.
    pub fn stored_owner_id(&self) -> Option<Identifier> {
        match &self.stored {
            Some(StoredVersion::Present(stored)) => Some(stored.system.owner_id),
            Some(StoredVersion::Absent) | None => None,
        }
    }

    /// The data a rule is judged against for a document whose properties are
    /// `data` (a map): `data`, with the stored version's properties under
    /// [`STORED_DOCUMENT_KEY`] when they are given
    /// ([`data_with_stored_properties`]), so that a path through `$old.` reads
    /// them, and with nothing there otherwise. A `$old` key `data` carries
    /// itself, which no document holds, is dropped either way, so that it is
    /// never read as the stored version. `data` itself when there is nothing to
    /// add or drop.
    pub fn judged_data<'a>(&self, data: &'a Value) -> Cow<'a, Value> {
        if let Some(StoredVersion::Present(stored)) = &self.stored {
            if let Some(properties) = &stored.properties {
                return Cow::Owned(data_with_stored_properties(
                    data.clone(),
                    Value::from(properties.clone()),
                ));
            }
        }
        match data {
            Value::Map(entries)
                if entries
                    .iter()
                    .any(|(key, _)| key.as_text() == Some(STORED_DOCUMENT_KEY)) =>
            {
                Cow::Owned(Value::Map(
                    entries
                        .iter()
                        .filter(|(key, _)| key.as_text() != Some(STORED_DOCUMENT_KEY))
                        .cloned()
                        .collect(),
                ))
            }
            _ => Cow::Borrowed(data),
        }
    }

    /// The value of `property`, `None` when not given.
    pub fn value(&self, property: SystemProperty) -> Option<i128> {
        match property {
            SystemProperty::CreatedAt => self.created_at.map(i128::from),
            SystemProperty::UpdatedAt => self.updated_at.map(i128::from),
            SystemProperty::TransferredAt => self.transferred_at.map(i128::from),
            SystemProperty::CreatedAtBlockHeight => self.created_at_block_height.map(i128::from),
            SystemProperty::UpdatedAtBlockHeight => self.updated_at_block_height.map(i128::from),
            SystemProperty::TransferredAtBlockHeight => {
                self.transferred_at_block_height.map(i128::from)
            }
            SystemProperty::CreatedAtCoreBlockHeight => {
                self.created_at_core_block_height.map(i128::from)
            }
            SystemProperty::UpdatedAtCoreBlockHeight => {
                self.updated_at_core_block_height.map(i128::from)
            }
            SystemProperty::TransferredAtCoreBlockHeight => {
                self.transferred_at_core_block_height.map(i128::from)
            }
        }
    }
}

/// What a size operand measures of the property it names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeMeasure {
    /// `length`: the characters of a string property, as `maxLength` counts
    /// them.
    Length,
    /// `byteLength`: the UTF-8 bytes of a string property, as `maxBytes`
    /// counts them.
    ByteLength,
    /// `count`: the items of an array property, or the bytes of a byte array
    /// property, as `maxItems` counts them.
    Count,
}

impl SizeMeasure {
    /// The operand key declaring it.
    pub fn wire_name(self) -> &'static str {
        match self {
            SizeMeasure::Length => LENGTH,
            SizeMeasure::ByteLength => BYTE_LENGTH,
            SizeMeasure::Count => COUNT,
        }
    }

    /// How an operand of this measure reads the property it names.
    fn read(self) -> PropertyRead {
        match self {
            SizeMeasure::Length | SizeMeasure::ByteLength => PropertyRead::Length,
            SizeMeasure::Count => PropertyRead::Count,
        }
    }
}

/// What an aggregate operand totals over the documents it matches.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AggregateKind {
    /// `countOf`: how many there are.
    Count,
    /// `sumOf`: the total of the integer property at `property` over them.
    Sum { property: String },
}

/// The value a key of an aggregate's filter must take, read from the document
/// being written, or, for a rule of `deleteConstraints`, deleted.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AggregateBinding {
    /// The document's property at the dotted `path`; `kind` is how it
    /// compares, `None` for an integer.
    Property {
        path: String,
        kind: Option<EqualityKind>,
    },
    /// `"$ownerId"`: the document's owner.
    Owner,
    /// `"$id"`: the document's id, so that a total counts the documents
    /// pointing at it (`{ "countOf": ["vote", { "pollId": "$id" }] }`). It never
    /// changes, so a transfer, a purchase or a price update reads it as the
    /// write before did.
    Id,
    /// An integer.
    Integer(i128),
    /// `{ "const": ... }`: a string, or a base58 identifier where the key is
    /// an identifier.
    Constant(String),
}

/// A `countOf` or `sumOf` operand: the documents of the type
/// `document_type`, of the same contract, whose values at the keys of
/// `filter` (a property path of that type, or `$ownerId`) equal the values
/// the bindings read from the document being written, counted or with an
/// integer property totalled; every document of the type when the filter is
/// empty. The total is the one a count or sum tree of that type keeps, as it
/// will be once the write is done: when the type is the writer's own, the
/// document being written counts as it will be stored, and no longer as it
/// was. A registered rule reads only totals a tree keeps: `documentsCountable`
/// or `documentsSummable` for a whole type, and otherwise an index whose
/// properties are exactly the filter's keys ([`Self::answering_index`]).
///
/// `{ "countOf": ["listing", { "$ownerId": "$ownerId" }] }` counts the
/// writer's listings; `{ "sumOf": ["pledge", "amount", { "campaignId": "campaignId" }] }`
/// totals the pledges to the document's campaign.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AggregateRead {
    pub kind: AggregateKind,
    pub document_type: String,
    pub filter: BTreeMap<String, AggregateBinding>,
    /// Whether `document_type` is the type declaring the rule, so that the
    /// document being written is among those it totals.
    pub of_own_type: bool,
}

impl AggregateRead {
    /// The operand key declaring it.
    pub fn wire_name(&self) -> &'static str {
        match self.kind {
            AggregateKind::Count => COUNT_OF,
            AggregateKind::Sum { .. } => SUM_OF,
        }
    }

    /// Whether the total depends on the document's owner: a binding reads
    /// `$ownerId`, or the type is the writer's own and a key is `$ownerId`, so
    /// that the document counts toward another owner once it changes hands.
    pub fn reads_owner(&self) -> bool {
        self.filter
            .values()
            .any(|binding| *binding == AggregateBinding::Owner)
            || (self.of_own_type && self.filter.contains_key(OWNER_ID))
    }

    /// The filter's keys when the total is over the type declaring the rule:
    /// a write changing one of them on a document decides whether that
    /// document counts, moving the total without the rule reading the property
    /// itself. None for another type's documents, which a write of this type
    /// leaves alone. The property a `sumOf` adds up moves the total too, but
    /// a summable index makes it required, so it is written with the document.
    pub fn own_type_keys(&self) -> impl Iterator<Item = &str> + '_ {
        self.of_own_type
            .then(|| self.filter.keys().map(String::as_str))
            .into_iter()
            .flatten()
    }
}

/// An integer expression, one side of a rule or an operand inside one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintExpression {
    /// An integer value.
    Value(i128),
    /// The value of the integer or boolean property at the dotted `path` (1
    /// for true, 0 for false), or `if_absent` when the document leaves it out:
    /// 0 for a path on its own, the declared value for an `ifAbsent` operand.
    Property { path: String, if_absent: i128 },
    /// `length`, `byteLength` or `count`: the size of the property at the
    /// dotted `path`, as `measure` counts it, or 0 when the document leaves it
    /// out.
    Size { measure: SizeMeasure, path: String },
    /// `countPresent`: how many of the properties at the dotted `paths`, two
    /// or more and no two alike, the document holds, each as
    /// [`PropertyConstraint::Present`] tests it.
    CountPresent(Vec<String>),
    /// `byteAt`: the byte, 0 to 255, at `index` (counted from 0) of the byte
    /// array property at the dotted `path`, `{ "byteAt": ["address", 0] }`, or
    /// 0 when the array does not hold that byte or the document leaves it out.
    ByteAt { path: String, index: u16 },
    /// A system value, `"$createdAt"` or `"$revision"` say, of the document
    /// version written or, through `$old.`, of the stored version it replaces:
    /// its value in the [`DocumentSystemValues`] the rule is judged with.
    System(SystemRead),
    /// `add`: the sum of two or more operands.
    Add(Vec<ConstraintExpression>),
    /// `multiply`: the product of two or more operands.
    Multiply(Vec<ConstraintExpression>),
    /// `subtract`: the left operand less the right one.
    Subtract(Box<ConstraintExpression>, Box<ConstraintExpression>),
    /// `divide`: the Euclidean quotient of the left operand by the right one.
    Divide(Box<ConstraintExpression>, Box<ConstraintExpression>),
    /// `modulo`: the Euclidean remainder of the left operand by the right one,
    /// never negative.
    Modulo(Box<ConstraintExpression>, Box<ConstraintExpression>),
    /// `power`: the left operand raised to the right one.
    Power(Box<ConstraintExpression>, Box<ConstraintExpression>),
    /// `min`: the least of two or more operands.
    Min(Vec<ConstraintExpression>),
    /// `max`: the greatest of two or more operands.
    Max(Vec<ConstraintExpression>),
    /// `abs`: the absolute value of its one operand.
    Abs(Box<ConstraintExpression>),
    /// `countOf` or `sumOf`: a total read from state, its value in the
    /// [`DocumentSystemValues`] the rule is judged with.
    Aggregate(AggregateRead),
}

impl ConstraintExpression {
    /// The value of the expression for a document whose properties are `data`.
    ///
    /// Exact arithmetic over `i128`. Operands are evaluated left to right and
    /// the first fault is returned; every intermediate result must fit:
    ///
    /// * a property the document leaves out takes its `if_absent` value; a
    ///   boolean it holds reads as 1 for true and 0 for false; any other value
    ///   must be an integer ([`PropertyConstraintViolation::NotAnInteger`]
    ///   otherwise: the schema validation running first admits a float with no
    ///   fractional part as an integer, which the document could not be stored
    ///   with anyway) that fits an `i128`
    ///   ([`PropertyConstraintViolation::Overflow`] otherwise);
    /// * a size is never a fault: a property the document leaves out, or sets
    ///   to null, has size 0, and so does a value of another type than the
    ///   one measured, which the schema validation reported first refuses;
    /// * a `countPresent` is never a fault: it counts the properties that are
    ///   present, whatever their type;
    /// * nor is a byte: one the array does not hold, of an array the document
    ///   leaves out or sets to null, reads as 0, and so does one of a value
    ///   that is no byte array, which the schema validation refuses first;
    /// * a system property takes its value in `system`, 0 when not given
    ///   ([`PropertyConstraint::violation`] does not judge a rule reading one it
    ///   is not given), and so does an aggregate;
    /// * `add` and `multiply` fold their operands from the left, so an overflow
    ///   on the way is a fault even when a later operand would bring the result
    ///   back in range;
    /// * `divide` and `modulo` are Euclidean: the remainder is never negative
    ///   and the quotient is the one that goes with it (`-7` by `2` is `-4`
    ///   remainder `1`), which for operands that are not negative is ordinary
    ///   integer division. A divisor of 0 is a
    ///   [`PropertyConstraintViolation::DivisionByZero`];
    /// * `min` and `max` evaluate every operand, so a fault in any breaks the
    ///   rule; `abs` of `i128::MIN` does not fit
    ///   ([`PropertyConstraintViolation::Overflow`]);
    /// * `power` refuses a negative exponent
    ///   ([`PropertyConstraintViolation::NegativeExponent`]), which has no
    ///   integer result, and takes `0` to the power `0` as `1`.
    pub fn evaluate(
        &self,
        data: &Value,
        system: &DocumentSystemValues,
    ) -> Result<i128, PropertyConstraintViolation> {
        match self {
            ConstraintExpression::Value(value) => Ok(*value),
            ConstraintExpression::System(read) => Ok(system.read(*read).unwrap_or(0)),
            ConstraintExpression::Aggregate(read) => Ok(system
                .aggregates
                .as_ref()
                .and_then(|aggregates| aggregates.get(read))
                .copied()
                .unwrap_or(0)),
            ConstraintExpression::Property { path, if_absent } => {
                property_value(data, path, *if_absent)
            }
            ConstraintExpression::Size { measure, path } => {
                // A size fits a `usize`, which always fits an `i128`
                i128::try_from(property_size(data, path, *measure))
                    .map_err(|_| PropertyConstraintViolation::Overflow)
            }
            ConstraintExpression::CountPresent(paths) => {
                // At most the number of paths, which always fits an `i128`
                i128::try_from(paths.iter().filter(|path| is_present(data, path)).count())
                    .map_err(|_| PropertyConstraintViolation::Overflow)
            }
            ConstraintExpression::ByteAt { path, index } => Ok(i128::from(
                byte_at(data, path, usize::from(*index)).unwrap_or(0),
            )),
            ConstraintExpression::Add(operands) => {
                operands.iter().try_fold(0i128, |sum, operand| {
                    sum.checked_add(operand.evaluate(data, system)?)
                        .ok_or(PropertyConstraintViolation::Overflow)
                })
            }
            ConstraintExpression::Multiply(operands) => {
                operands.iter().try_fold(1i128, |product, operand| {
                    product
                        .checked_mul(operand.evaluate(data, system)?)
                        .ok_or(PropertyConstraintViolation::Overflow)
                })
            }
            ConstraintExpression::Subtract(left, right) => {
                let (left, right) = (left.evaluate(data, system)?, right.evaluate(data, system)?);
                left.checked_sub(right)
                    .ok_or(PropertyConstraintViolation::Overflow)
            }
            ConstraintExpression::Divide(left, right) => {
                let (dividend, divisor) =
                    (left.evaluate(data, system)?, right.evaluate(data, system)?);
                if divisor == 0 {
                    return Err(PropertyConstraintViolation::DivisionByZero);
                }
                // `i128::MIN` by -1 is the one quotient that does not fit
                dividend
                    .checked_div_euclid(divisor)
                    .ok_or(PropertyConstraintViolation::Overflow)
            }
            ConstraintExpression::Modulo(left, right) => {
                let (dividend, divisor) =
                    (left.evaluate(data, system)?, right.evaluate(data, system)?);
                match divisor {
                    0 => Err(PropertyConstraintViolation::DivisionByZero),
                    // Every integer is a multiple of -1. `checked_rem_euclid` refuses
                    // `i128::MIN` by -1 because the quotient does not fit, but the
                    // remainder does
                    -1 => Ok(0),
                    _ => dividend
                        .checked_rem_euclid(divisor)
                        .ok_or(PropertyConstraintViolation::Overflow),
                }
            }
            ConstraintExpression::Power(left, right) => {
                let (base, exponent) =
                    (left.evaluate(data, system)?, right.evaluate(data, system)?);
                power(base, exponent)
            }
            // Folded from their identities, as add and multiply are
            ConstraintExpression::Min(operands) => {
                operands.iter().try_fold(i128::MAX, |least, operand| {
                    Ok(least.min(operand.evaluate(data, system)?))
                })
            }
            ConstraintExpression::Max(operands) => {
                operands.iter().try_fold(i128::MIN, |greatest, operand| {
                    Ok(greatest.max(operand.evaluate(data, system)?))
                })
            }
            ConstraintExpression::Abs(operand) => operand
                .evaluate(data, system)?
                .checked_abs()
                .ok_or(PropertyConstraintViolation::Overflow),
        }
    }

    /// The nodes of the expression: this one, and those of its operands.
    pub fn node_count(&self) -> usize {
        1 + match self {
            ConstraintExpression::Value(_)
            | ConstraintExpression::Property { .. }
            | ConstraintExpression::Size { .. }
            | ConstraintExpression::ByteAt { .. }
            | ConstraintExpression::System(_) => 0,
            // One for each key and the value it takes
            ConstraintExpression::Aggregate(read) => read.filter.len(),
            // One for each property it tests, as a `present` is one
            ConstraintExpression::CountPresent(paths) => paths.len(),
            ConstraintExpression::Add(operands)
            | ConstraintExpression::Multiply(operands)
            | ConstraintExpression::Min(operands)
            | ConstraintExpression::Max(operands) => {
                operands.iter().map(ConstraintExpression::node_count).sum()
            }
            ConstraintExpression::Abs(operand) => operand.node_count(),
            ConstraintExpression::Subtract(left, right)
            | ConstraintExpression::Divide(left, right)
            | ConstraintExpression::Modulo(left, right)
            | ConstraintExpression::Power(left, right) => left.node_count() + right.node_count(),
        }
    }

    /// Whether the expression reads at least one property, or a value read
    /// from state, so that it is no constant.
    fn reads_property(&self) -> bool {
        match self {
            ConstraintExpression::Value(_) => false,
            ConstraintExpression::Property { .. }
            | ConstraintExpression::Size { .. }
            | ConstraintExpression::CountPresent(_)
            | ConstraintExpression::ByteAt { .. }
            | ConstraintExpression::System(_)
            | ConstraintExpression::Aggregate(_) => true,
            ConstraintExpression::Add(operands)
            | ConstraintExpression::Multiply(operands)
            | ConstraintExpression::Min(operands)
            | ConstraintExpression::Max(operands) => {
                operands.iter().any(ConstraintExpression::reads_property)
            }
            ConstraintExpression::Abs(operand) => operand.reads_property(),
            ConstraintExpression::Subtract(left, right)
            | ConstraintExpression::Divide(left, right)
            | ConstraintExpression::Modulo(left, right)
            | ConstraintExpression::Power(left, right) => {
                left.reads_property() || right.reads_property()
            }
        }
    }

    /// Hands `visit` the properties the expression reads, each by its value or
    /// its size, in the order it reads them, stopping at the first `Break`.
    fn visit_property_reads<'a>(
        &'a self,
        visit: &mut dyn FnMut(&'a str, PropertyRead) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        match self {
            ConstraintExpression::Value(_) | ConstraintExpression::System(_) => {}
            ConstraintExpression::Property { path, .. } => visit(path, PropertyRead::Value)?,
            // The properties of the document being written its filter reads
            ConstraintExpression::Aggregate(read) => {
                for binding in read.filter.values() {
                    if let AggregateBinding::Property { path, kind } = binding {
                        let read = match kind {
                            // Never bytes: the parse reads a byte array as an
                            // integer here, which the parsed type refuses
                            None | Some(EqualityKind::Bytes) => PropertyRead::Value,
                            Some(EqualityKind::Text) => PropertyRead::Text,
                            Some(EqualityKind::Identifier) => PropertyRead::Identifier,
                        };
                        visit(path, read)?;
                    }
                }
            }
            ConstraintExpression::Size { measure, path } => visit(path, measure.read())?,
            ConstraintExpression::CountPresent(paths) => {
                for path in paths {
                    visit(path, PropertyRead::Presence)?;
                }
            }
            // Byte `index` is there only in an array of `index + 1` bytes or more
            ConstraintExpression::ByteAt { path, index } => visit(
                path,
                PropertyRead::Bytes {
                    extent: usize::from(*index) + 1,
                },
            )?,
            ConstraintExpression::Add(operands)
            | ConstraintExpression::Multiply(operands)
            | ConstraintExpression::Min(operands)
            | ConstraintExpression::Max(operands) => {
                for operand in operands {
                    operand.visit_property_reads(visit)?;
                }
            }
            ConstraintExpression::Abs(operand) => operand.visit_property_reads(visit)?,
            ConstraintExpression::Subtract(left, right)
            | ConstraintExpression::Divide(left, right)
            | ConstraintExpression::Modulo(left, right)
            | ConstraintExpression::Power(left, right) => {
                left.visit_property_reads(visit)?;
                right.visit_property_reads(visit)?;
            }
        }
        ControlFlow::Continue(())
    }

    /// Hands `visit` the system values the expression reads, in the order it
    /// reads them, stopping at the first `Break`.
    fn visit_system_reads(
        &self,
        visit: &mut dyn FnMut(SystemRead) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        match self {
            ConstraintExpression::System(read) => visit(*read)?,
            ConstraintExpression::Value(_)
            | ConstraintExpression::Property { .. }
            | ConstraintExpression::Size { .. }
            | ConstraintExpression::CountPresent(_)
            | ConstraintExpression::ByteAt { .. }
            | ConstraintExpression::Aggregate(_) => {}
            ConstraintExpression::Add(operands)
            | ConstraintExpression::Multiply(operands)
            | ConstraintExpression::Min(operands)
            | ConstraintExpression::Max(operands) => {
                for operand in operands {
                    operand.visit_system_reads(visit)?;
                }
            }
            ConstraintExpression::Abs(operand) => operand.visit_system_reads(visit)?,
            ConstraintExpression::Subtract(left, right)
            | ConstraintExpression::Divide(left, right)
            | ConstraintExpression::Modulo(left, right)
            | ConstraintExpression::Power(left, right) => {
                left.visit_system_reads(visit)?;
                right.visit_system_reads(visit)?;
            }
        }
        ControlFlow::Continue(())
    }

    /// Appends the aggregates the expression reads to `reads`, in the order it
    /// reads them.
    fn collect_aggregate_reads<'a>(&'a self, reads: &mut Vec<&'a AggregateRead>) {
        match self {
            ConstraintExpression::Aggregate(read) => reads.push(read),
            ConstraintExpression::Value(_)
            | ConstraintExpression::Property { .. }
            | ConstraintExpression::Size { .. }
            | ConstraintExpression::CountPresent(_)
            | ConstraintExpression::ByteAt { .. }
            | ConstraintExpression::System(_) => {}
            ConstraintExpression::Add(operands)
            | ConstraintExpression::Multiply(operands)
            | ConstraintExpression::Min(operands)
            | ConstraintExpression::Max(operands) => {
                for operand in operands {
                    operand.collect_aggregate_reads(reads);
                }
            }
            ConstraintExpression::Abs(operand) => operand.collect_aggregate_reads(reads),
            ConstraintExpression::Subtract(left, right)
            | ConstraintExpression::Divide(left, right)
            | ConstraintExpression::Modulo(left, right)
            | ConstraintExpression::Power(left, right) => {
                left.collect_aggregate_reads(reads);
                right.collect_aggregate_reads(reads);
            }
        }
    }

    /// The first aggregate the expression reads, in the order it reads them,
    /// that `matches`, the walk stopping there.
    fn find_aggregate<'a>(
        &'a self,
        matches: &dyn Fn(&AggregateRead) -> bool,
    ) -> Option<&'a AggregateRead> {
        match self {
            ConstraintExpression::Aggregate(read) => matches(read).then_some(read),
            ConstraintExpression::Value(_)
            | ConstraintExpression::Property { .. }
            | ConstraintExpression::Size { .. }
            | ConstraintExpression::CountPresent(_)
            | ConstraintExpression::ByteAt { .. }
            | ConstraintExpression::System(_) => None,
            ConstraintExpression::Add(operands)
            | ConstraintExpression::Multiply(operands)
            | ConstraintExpression::Min(operands)
            | ConstraintExpression::Max(operands) => operands
                .iter()
                .find_map(|operand| operand.find_aggregate(matches)),
            ConstraintExpression::Abs(operand) => operand.find_aggregate(matches),
            ConstraintExpression::Subtract(left, right)
            | ConstraintExpression::Divide(left, right)
            | ConstraintExpression::Modulo(left, right)
            | ConstraintExpression::Power(left, right) => left
                .find_aggregate(matches)
                .or_else(|| right.find_aggregate(matches)),
        }
    }
}

/// How a rule reads a property, which decides the properties it may name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyRead {
    /// By its value, as an operand: an integer or boolean property.
    Value,
    /// Only whether the document holds it, in a `present`, `absent` or
    /// `countPresent`: a property of any type, an object included.
    Presence,
    /// By its value, compared with string constants: a string property.
    Text,
    /// By its value, compared with identifier constants: an identifier
    /// property.
    Identifier,
    /// By its size, in a `length` or `byteLength` operand: a string property.
    Length,
    /// By its size, in a `count` operand: an array or byte array property.
    Count,
    /// By its elements, which a `contains` looks among for a value of the
    /// kind given: a typed array property with elements of that kind.
    Elements(ElementKind),
    /// By its bytes, in a `byteAt` operand or a `startsWith` or `endsWith` of
    /// byte arrays: a byte array property. `extent` is how long the array must
    /// be for the read to find what it looks for: one past the index a
    /// `byteAt` reads, the length of a constant the array must start or end
    /// with, and 0 for any other side of a `startsWith` or `endsWith`. A type
    /// whose `maxItems` keeps the array shorter is refused, since the read
    /// could never find what it looks for.
    Bytes { extent: usize },
}

/// What a `contains` looks for among an array's elements, which decides the
/// elements the array must have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementKind {
    /// An integer, the value of an integer expression.
    Integer,
    /// A string.
    Text,
    /// An identifier.
    Identifier,
}

/// What a comparison of equality compares when it is not integers: strings or
/// identifiers, or what a `startsWith` or `endsWith` tests: strings or byte
/// arrays. [`parse_property_constraints`] asks it of every bare path on either
/// side of an `equal`, a `notEqual`, a `startsWith` or an `endsWith`, of an
/// `in`'s operand, and of the array a `contains` looks in (the kind of its
/// elements), since the declaration alone does not tell a string property, an
/// identifier property, a byte array property or an integer one apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EqualityKind {
    /// A string property.
    Text,
    /// An identifier property.
    Identifier,
    /// A byte array property, which only a `startsWith` or an `endsWith`
    /// tests, against a constant written in hex or another byte array
    /// property: `equal`, `notEqual`, `in` and `contains` never compare one.
    Bytes,
}

/// A string property a string comparison reads: its dotted path, and the
/// string it takes when the document leaves it out, from an `ifAbsent` with a
/// string default (`{ "ifAbsent": ["status", "open"] }`). A bare path has no
/// default: a property the document leaves out then equals no string, not even
/// another one it leaves out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextProperty {
    pub path: String,
    pub if_absent: Option<String>,
}

impl TextProperty {
    /// The value a document whose properties are `data` holds at a path a
    /// `referenced` operand reads when it is no string: an identifier, a byte
    /// array, an integer or a boolean of the referenced document, compared as
    /// what it is ([`spelled_by`], `Value::same_scalar_data`). `None` for a
    /// string, a value left out, and every property of the type's own.
    fn referenced_other<'a>(&'a self, data: &'a Value) -> Option<&'a Value> {
        if !self.path.starts_with(REFERENCED_DOCUMENT_PREFIX) {
            return None;
        }
        match data.get_optional_value_at_path(&self.path) {
            Ok(Some(Value::Text(_) | Value::Null)) | Ok(None) | Err(_) => None,
            Ok(Some(value)) => Some(value),
        }
    }

    /// Whether the property, for a document whose properties are `data`, is the
    /// string `constant` spells ([`Self::value`], or [`Self::referenced_other`]
    /// read as [`spelled_by`] says).
    fn equals_constant(&self, data: &Value, constant: &str) -> bool {
        self.value(data) == Some(constant)
            || self
                .referenced_other(data)
                .is_some_and(|value| spelled_by(value, constant))
    }

    /// The string a document whose properties are `data` gives the property:
    /// the one it holds, or `if_absent` when it leaves the property out or
    /// sets it to null (where an operand takes its `ifAbsent` value). `None`
    /// for a property left out without a default, or holding anything but a
    /// string, which the schema validation running first refuses for a string
    /// property.
    fn value<'a>(&'a self, data: &'a Value) -> Option<&'a str> {
        match data.get_optional_value_at_path(&self.path) {
            Ok(Some(Value::Text(text))) => Some(text),
            Ok(Some(Value::Null)) | Ok(None) | Err(_) => self.if_absent.as_deref(),
            Ok(Some(_)) => None,
        }
    }
}

/// Where `startsWith` and `endsWith` look for their second string in their
/// first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AffixPosition {
    /// `startsWith`: at the start.
    Start,
    /// `endsWith`: at the end.
    End,
}

impl AffixPosition {
    /// The condition key declaring it.
    pub fn wire_name(self) -> &'static str {
        match self {
            AffixPosition::Start => STARTS_WITH,
            AffixPosition::End => ENDS_WITH,
        }
    }

    /// Whether `text` starts or ends with `affix`, byte for byte: no case
    /// folding or normalization, and every string starts and ends with the
    /// empty one.
    pub fn holds(self, text: &str, affix: &str) -> bool {
        self.holds_bytes(text.as_bytes(), affix.as_bytes())
    }

    /// Whether `bytes` starts or ends with `affix`; every byte array starts
    /// and ends with the empty one.
    pub fn holds_bytes(self, bytes: &[u8], affix: &[u8]) -> bool {
        match self {
            AffixPosition::Start => bytes.starts_with(affix),
            AffixPosition::End => bytes.ends_with(affix),
        }
    }
}

/// A side of a `startsWith` or `endsWith`: a string constant, or a string
/// property with or without an `ifAbsent` default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextOperand {
    /// A `{ "const": string }`.
    Constant(String),
    /// A string property.
    Property(TextProperty),
}

impl TextOperand {
    /// The string it takes for a document whose properties are `data`, `None`
    /// for a property left out without a default.
    fn value<'a>(&'a self, data: &'a Value) -> Option<&'a str> {
        match self {
            TextOperand::Constant(value) => Some(value),
            TextOperand::Property(property) => property.value(data),
        }
    }

    /// The bytes it takes for a document whose properties are `data` where a
    /// byte array of a referenced document is tested: a constant read as hex,
    /// a byte array a `referenced` operand reads ([`TextProperty::referenced_other`]).
    /// `None` for anything else.
    fn bytes<'a>(&'a self, data: &'a Value) -> Option<Cow<'a, [u8]>> {
        match self {
            TextOperand::Constant(value) => hex::decode(value).ok().map(Cow::Owned),
            TextOperand::Property(property) => match property.referenced_other(data)? {
                Value::Identifier(_) => None,
                value => value.as_bytes_slice().ok().map(Cow::Borrowed),
            },
        }
    }
}

/// A side of a `startsWith` or `endsWith` of byte arrays: a constant, written
/// in hex, or a byte array property, which takes no default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BytesOperand {
    /// A `{ "const": hex }`.
    Constant(Vec<u8>),
    /// The byte array property at the dotted path.
    Property(String),
}

impl BytesOperand {
    /// The bytes it takes for a document whose properties are `data`, `None`
    /// for a property left out ([`byte_array_value`]).
    fn value<'a>(&'a self, data: &'a Value) -> Option<Cow<'a, [u8]>> {
        match self {
            BytesOperand::Constant(bytes) => Some(Cow::Borrowed(bytes)),
            BytesOperand::Property(path) => byte_array_value(data, path),
        }
    }
}

/// What a `contains` looks for among an array property's elements, of the
/// kind of its elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainsNeedle {
    /// The value of an integer expression, among integers.
    Integer(ConstraintExpression),
    /// A string constant, among strings.
    TextConstant(String),
    /// The string a string property holds, or its `ifAbsent` default, among
    /// strings; one the document leaves out without a default is among none.
    TextProperty(TextProperty),
    /// An identifier constant, among identifiers.
    IdentifierConstant(Identifier),
    /// The identifier at the dotted path, or `$ownerId`, among identifiers;
    /// one the document leaves out is among none.
    IdentifierProperty(String),
}

/// A rule of `propertyConstraints`, or a condition inside one: a comparison of
/// two integer expressions, a test of whether an integer expression takes one
/// of listed values, a comparison of a string property with string constants,
/// a test of whether the document holds a property, or `anyOf`, `allOf` or
/// `not` over conditions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyConstraint {
    /// A comparison: the two sides must compare as `comparison` says.
    Compare {
        comparison: ConstraintComparison,
        left: ConstraintExpression,
        right: ConstraintExpression,
    },
    /// `in`: the expression takes one of two or more distinct integer values,
    /// what an `anyOf` of `equal`s says in far fewer nodes.
    In {
        operand: ConstraintExpression,
        values: BTreeSet<i128>,
    },
    /// `equal` or `notEqual` between a string property and a string constant,
    /// `{ "equal": ["status", { "const": "closed" }] }`, written either way
    /// round. `comparison` is `Equal` or `NotEqual`.
    TextCompare {
        comparison: ConstraintComparison,
        property: TextProperty,
        value: String,
    },
    /// `equal` or `notEqual` between two string properties,
    /// `{ "notEqual": ["fromCurrency", "toCurrency"] }`: two bare paths that
    /// both name string properties, or an `ifAbsent` with a string default on
    /// either side. `comparison` is `Equal` or `NotEqual`.
    TextCompareProperties {
        comparison: ConstraintComparison,
        left: TextProperty,
        right: TextProperty,
    },
    /// `in` over strings: the string property holds one of two or more
    /// distinct string constants, `{ "in": ["status", ["open", "pending"]] }`.
    TextIn {
        property: TextProperty,
        values: BTreeSet<String>,
    },
    /// `equal` or `notEqual` between the identifier property at the dotted path
    /// and an identifier constant, written base58,
    /// `{ "equal": ["paymentToken", { "const": "<base58>" }] }`, either way
    /// round. `comparison` is `Equal` or `NotEqual`. An identifier property the
    /// document leaves out equals no identifier.
    IdentifierCompare {
        comparison: ConstraintComparison,
        path: String,
        value: Identifier,
    },
    /// `equal` or `notEqual` between two identifier properties: two bare paths
    /// that both name identifier properties. One the document leaves out equals
    /// no identifier, not even another one it leaves out.
    IdentifierCompareProperties {
        comparison: ConstraintComparison,
        left: String,
        right: String,
    },
    /// `in` over identifiers: the identifier property at the dotted path holds
    /// one of two or more distinct identifiers, listed base58.
    IdentifierIn {
        path: String,
        values: BTreeSet<Identifier>,
    },
    /// `startsWith` or `endsWith`: the string `text` takes starts or ends, as
    /// `position` says, with the one `affix` takes,
    /// `{ "startsWith": ["url", { "const": "https://" }] }`. A string property
    /// the document leaves out without a default takes no string, and the
    /// condition does not hold for it.
    TextAffix {
        position: AffixPosition,
        text: TextOperand,
        affix: TextOperand,
    },
    /// `startsWith` or `endsWith` of byte arrays: the bytes `bytes` takes
    /// start or end, as `position` says, with those `affix` takes,
    /// `{ "startsWith": ["address", { "const": "00" }] }`. A byte array
    /// property the document leaves out takes no bytes, and the condition does
    /// not hold for it.
    BytesAffix {
        position: AffixPosition,
        bytes: BytesOperand,
        affix: BytesOperand,
    },
    /// `contains`: the typed array property at the dotted path `array` holds
    /// an element equal to `needle`, `{ "contains": ["tags", { "const": "sale" }] }`.
    /// An array the document leaves out holds nothing.
    Contains {
        array: String,
        needle: ContainsNeedle,
    },
    /// `present`: the document holds the property at the dotted path. One it
    /// leaves out, or sets to null, is absent, as it is for an operand, and so
    /// is an object none of whose members is present (`{}`), which a stored
    /// document does not keep. Unlike an operand, it tells a property left out
    /// from one set to 0, and it may name a property of any type.
    Present(String),
    /// `absent`: the document leaves the property at the dotted path out.
    Absent(String),
    /// `anyOf`: at least one of two or more conditions holds.
    AnyOf(Vec<PropertyConstraint>),
    /// `allOf`: every one of two or more conditions holds.
    AllOf(Vec<PropertyConstraint>),
    /// `not`: the condition does not hold.
    Not(Box<PropertyConstraint>),
    /// `ifThen`: `then` holds whenever `condition` does,
    /// `{ "ifThen": [{ "equal": ["status", { "const": "closed" }] }, { "present": "closedAt" }] }`;
    /// or `ifThenElse`, with `otherwise` given: `then` when `condition` holds,
    /// `otherwise` when it does not. Only the branch taken is evaluated.
    IfThen {
        condition: Box<PropertyConstraint>,
        then: Box<PropertyConstraint>,
        otherwise: Option<Box<PropertyConstraint>>,
    },
    /// `notIn`: an `in` ([`Self::In`], [`Self::TextIn`] or
    /// [`Self::IdentifierIn`]) that does not hold, the operand taking none of
    /// the listed values. It costs what the `in` costs.
    NotIn(Box<PropertyConstraint>),
}

impl PropertyConstraint {
    /// Whether a document whose properties are `data` and whose system values
    /// are `system` meets the condition. `$ownerId` reads `system.owner_id`,
    /// equalling no identifier when it is `None`, and a system value reads its
    /// value there, 0 when not given. A path through `$old.` reads the stored
    /// version's properties `data` holds under [`STORED_DOCUMENT_KEY`]
    /// ([`DocumentSystemValues::judged_data`]), and `$old.$ownerId` and a
    /// stored system value read [`DocumentSystemValues::stored`].
    ///
    /// Evaluated left to right, and no further than the outcome needs: a
    /// comparison evaluates its left side, then its right one; `anyOf` checks
    /// its conditions in declared order and holds at the first that holds;
    /// `allOf` fails at the first that fails; `not` inverts its condition;
    /// `ifThen` and `ifThenElse` evaluate their condition, then only the
    /// branch it selects (an `ifThen` holding when the condition does not); a
    /// string comparison, `present` or `absent` never faults. The first fault
    /// an evaluated expression meets ([`ConstraintExpression::evaluate`]) is
    /// returned whatever the conditions left unevaluated would say, and `not`
    /// never turns a fault into a pass. So an earlier condition guards a later
    /// one: `anyOf: [{ equal: ["b", 0] }, { equal: [{ divide: ["a", "b"] }, 2] }]`
    /// holds for a `b` of 0 without dividing by it, while the same two
    /// conditions the other way round divide by zero.
    pub fn holds(
        &self,
        data: &Value,
        system: &DocumentSystemValues,
    ) -> Result<bool, PropertyConstraintViolation> {
        match self {
            PropertyConstraint::Compare {
                comparison,
                left,
                right,
            } => {
                let (left, right) = (left.evaluate(data, system)?, right.evaluate(data, system)?);
                Ok(comparison.holds(left, right))
            }
            PropertyConstraint::In { operand, values } => {
                Ok(values.contains(&operand.evaluate(data, system)?))
            }
            PropertyConstraint::TextCompare {
                comparison,
                property,
                value,
            } => {
                let equal = property.equals_constant(data, value);
                Ok(equal == (*comparison == ConstraintComparison::Equal))
            }
            PropertyConstraint::TextCompareProperties {
                comparison,
                left,
                right,
            } => {
                // Two strings, or two values of referenced documents that are no
                // strings, compared as what they are
                let equal = match (left.value(data), right.value(data)) {
                    (Some(left), Some(right)) => left == right,
                    _ => matches!(
                        (left.referenced_other(data), right.referenced_other(data)),
                        (Some(left), Some(right)) if left.same_scalar_data(right)
                    ),
                };
                Ok(equal == (*comparison == ConstraintComparison::Equal))
            }
            PropertyConstraint::TextIn { property, values } => Ok(match property.value(data) {
                Some(text) => values.contains(text),
                None => property
                    .referenced_other(data)
                    .is_some_and(|value| values.iter().any(|constant| spelled_by(value, constant))),
            }),
            PropertyConstraint::IdentifierCompare {
                comparison,
                path,
                value,
            } => {
                let equal = identifier_value(data, system, path) == Some(*value);
                Ok(equal == (*comparison == ConstraintComparison::Equal))
            }
            PropertyConstraint::IdentifierCompareProperties {
                comparison,
                left,
                right,
            } => {
                let equal = matches!(
                    (
                        identifier_value(data, system, left),
                        identifier_value(data, system, right)
                    ),
                    (Some(left), Some(right)) if left == right
                );
                Ok(equal == (*comparison == ConstraintComparison::Equal))
            }
            PropertyConstraint::IdentifierIn { path, values } => {
                Ok(identifier_value(data, system, path)
                    .is_some_and(|value| values.contains(&value)))
            }
            PropertyConstraint::TextAffix {
                position,
                text,
                affix,
            } => Ok(match (text.value(data), affix.value(data)) {
                (Some(text), Some(affix)) => position.holds(text, affix),
                // A byte array of a referenced document, its affix in hex
                _ => matches!(
                    (text.bytes(data), affix.bytes(data)),
                    (Some(bytes), Some(affix)) if position.holds_bytes(&bytes, &affix)
                ),
            }),
            PropertyConstraint::BytesAffix {
                position,
                bytes,
                affix,
            } => Ok(matches!(
                (bytes.value(data), affix.value(data)),
                (Some(bytes), Some(affix)) if position.holds_bytes(&bytes, &affix)
            )),
            PropertyConstraint::Contains { array, needle } => {
                let elements = match data.get_optional_value_at_path(array) {
                    Ok(Some(Value::Array(elements))) => elements.as_slice(),
                    _ => &[],
                };
                Ok(match needle {
                    ContainsNeedle::Integer(expression) => {
                        let value = expression.evaluate(data, system)?;
                        elements.iter().any(|element| {
                            element.is_integer() && element.to_integer::<i128>().ok() == Some(value)
                        })
                    }
                    // Among the identifiers or byte arrays of a referenced
                    // document as the constant spells them
                    ContainsNeedle::TextConstant(value) => {
                        let referenced = array.starts_with(REFERENCED_DOCUMENT_PREFIX);
                        elements.iter().any(|element| {
                            element.as_text() == Some(value.as_str())
                                || (referenced && spelled_by(element, value))
                        })
                    }
                    ContainsNeedle::TextProperty(property) => {
                        property.value(data).is_some_and(|value| {
                            elements
                                .iter()
                                .any(|element| element.as_text() == Some(value))
                        })
                    }
                    ContainsNeedle::IdentifierConstant(value) => elements
                        .iter()
                        .any(|element| element.to_identifier().ok() == Some(*value)),
                    ContainsNeedle::IdentifierProperty(path) => {
                        identifier_value(data, system, path).is_some_and(|value| {
                            elements
                                .iter()
                                .any(|element| element.to_identifier().ok() == Some(value))
                        })
                    }
                })
            }
            PropertyConstraint::Present(path) => Ok(is_present(data, path)),
            PropertyConstraint::Absent(path) => Ok(!is_present(data, path)),
            PropertyConstraint::AnyOf(conditions) => {
                for condition in conditions {
                    if condition.holds(data, system)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            PropertyConstraint::AllOf(conditions) => {
                for condition in conditions {
                    if !condition.holds(data, system)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            PropertyConstraint::Not(condition) => Ok(!condition.holds(data, system)?),
            PropertyConstraint::IfThen {
                condition,
                then,
                otherwise,
            } => {
                if condition.holds(data, system)? {
                    then.holds(data, system)
                } else {
                    otherwise
                        .as_ref()
                        .map_or(Ok(true), |otherwise| otherwise.holds(data, system))
                }
            }
            PropertyConstraint::NotIn(condition) => Ok(!condition.holds(data, system)?),
        }
    }

    /// Whether a write judging rules with `system` judges this one: a rule
    /// reading a referenced document only where `system` says the data holds
    /// the values it reads ([`DocumentSystemValues::referenced_documents`]), a
    /// create or a replace once its references hold. Consensus reads no total
    /// of a rule the write does not judge.
    pub fn is_judged_with(&self, system: &DocumentSystemValues) -> bool {
        system.referenced_documents || !self.reads_referenced()
    }

    /// Why a document whose properties are `data` and whose system values are
    /// `system` breaks the rule, `None` when it meets it: the first fault met
    /// on the way ([`Self::holds`]), or [`PropertyConstraintViolation::NotMet`]
    /// when the rule evaluates to false. A rule reading a system value
    /// `system` does not give is not judged: consensus gives every one the
    /// document type records, the only ones a rule may read, so only a client
    /// that does not know one skips the rule. So is a rule reading the stored
    /// version when `system` does not say what it is
    /// ([`DocumentSystemValues::stored`]), and a rule reading an aggregate
    /// `system` does not give: consensus reads every one before judging the
    /// write, and a client, which cannot read state here, gives none. And so
    /// is a rule reading a referenced document unless `system` says `data`
    /// holds the values it reads ([`DocumentSystemValues::referenced_documents`]).
    /// `data` holds the stored version's properties under
    /// [`STORED_DOCUMENT_KEY`] when there is one ([`DocumentSystemValues::judged_data`]).
    pub fn violation(
        &self,
        data: &Value,
        system: &DocumentSystemValues,
    ) -> Option<PropertyConstraintViolation> {
        if !self.is_judged_with(system)
            || self.reads_system(&|read| system.read(read).is_none())
            || (system.stored.is_none() && self.reads_stored())
            || self
                .find_aggregate(&|read| {
                    !system
                        .aggregates
                        .as_ref()
                        .is_some_and(|aggregates| aggregates.contains_key(read))
                })
                .is_some()
        {
            return None;
        }
        match self.holds(data, system) {
            Ok(true) => None,
            Ok(false) => Some(PropertyConstraintViolation::NotMet),
            Err(violation) => Some(violation),
        }
    }

    /// The nodes of the rule, counted against
    /// `SystemLimits::max_property_constraint_nodes`: every comparison and
    /// logical operator, every `in` and each value it lists, every string
    /// constant, every `contains` with its array and what it looks for, every
    /// `present` or `absent` with the property it names, every
    /// arithmetic operator and every operand (an integer value, a property with
    /// or without `ifAbsent`, a size or a system property), and every
    /// `countPresent` with one more for each property it tests.
    pub fn node_count(&self) -> usize {
        1 + match self {
            PropertyConstraint::Compare { left, right, .. } => {
                left.node_count() + right.node_count()
            }
            PropertyConstraint::In { operand, values } => operand.node_count() + values.len(),
            // The property and the constant, as a comparison of a path with a value
            PropertyConstraint::TextCompare { .. }
            | PropertyConstraint::TextCompareProperties { .. }
            | PropertyConstraint::TextAffix { .. }
            | PropertyConstraint::BytesAffix { .. }
            | PropertyConstraint::IdentifierCompare { .. }
            | PropertyConstraint::IdentifierCompareProperties { .. } => 2,
            PropertyConstraint::TextIn { values, .. } => 1 + values.len(),
            PropertyConstraint::IdentifierIn { values, .. } => 1 + values.len(),
            // The array, and what is looked for among its elements
            PropertyConstraint::Contains { needle, .. } => {
                1 + match needle {
                    ContainsNeedle::Integer(expression) => expression.node_count(),
                    ContainsNeedle::TextConstant(_)
                    | ContainsNeedle::TextProperty(_)
                    | ContainsNeedle::IdentifierConstant(_)
                    | ContainsNeedle::IdentifierProperty(_) => 1,
                }
            }
            PropertyConstraint::Present(_) | PropertyConstraint::Absent(_) => 0,
            PropertyConstraint::AnyOf(conditions) | PropertyConstraint::AllOf(conditions) => {
                conditions.iter().map(PropertyConstraint::node_count).sum()
            }
            PropertyConstraint::Not(condition) => condition.node_count(),
            PropertyConstraint::IfThen {
                condition,
                then,
                otherwise,
            } => {
                condition.node_count()
                    + then.node_count()
                    + otherwise
                        .as_ref()
                        .map_or(0, |otherwise| otherwise.node_count())
            }
            // The `in`'s own nodes, the negation adding none
            PropertyConstraint::NotIn(condition) => condition.node_count() - 1,
        }
    }

    /// The dotted paths of the properties the rule reads, in declared order, a
    /// path read twice listed twice.
    pub fn property_paths(&self) -> Vec<&str> {
        self.property_reads()
            .into_iter()
            .map(|(path, _)| path)
            .collect()
    }

    /// The properties the rule reads, each with how it reads it, in declared
    /// order, a property read twice listed twice.
    pub fn property_reads(&self) -> Vec<(&str, PropertyRead)> {
        let mut reads = Vec::new();
        let _ = self.visit_property_reads(&mut |path, read| {
            reads.push((path, read));
            ControlFlow::Continue(())
        });
        reads
    }

    /// The properties of the document type itself the rule reads, each with how
    /// it reads it, in declared order: [`Self::property_reads`] without the
    /// values of referenced documents, which [`Self::referenced_reads`] lists.
    pub fn own_property_reads(&self) -> Vec<(&str, PropertyRead)> {
        let mut reads = Vec::new();
        let _ = self.visit_property_reads(&mut |path, read| {
            if !path.starts_with(REFERENCED_DOCUMENT_PREFIX) {
                reads.push((path, read));
            }
            ControlFlow::Continue(())
        });
        reads
    }

    /// What the rule reads of the documents the type's references point at,
    /// each as the reference property, the field of the referenced document
    /// and how it is read, in declared order, a read made twice listed twice.
    pub fn referenced_reads(&self) -> Vec<(&str, &str, PropertyRead)> {
        self.property_reads()
            .into_iter()
            .filter_map(|(path, read)| {
                referenced_read(path).map(|(reference, field)| (reference, field, read))
            })
            .collect()
    }

    /// Whether the rule reads a value of a document one of the type's
    /// references points at (`{ "referenced": [...] }`), the walk stopping at
    /// the first such read rather than collecting every one.
    pub fn reads_referenced(&self) -> bool {
        self.reads_property(&|path| path.starts_with(REFERENCED_DOCUMENT_PREFIX))
    }

    /// Whether the rule compares the document's owner, `$ownerId`, or the
    /// stored version's, `$old.$ownerId`, or reads an aggregate depending on it
    /// ([`AggregateRead::reads_owner`]): then a transfer or a purchase, which
    /// changes the owner and no property, is judged against it too.
    pub fn reads_owner(&self) -> bool {
        self.compares_identifier(&is_owner_path)
            || self.find_aggregate(&AggregateRead::reads_owner).is_some()
    }

    /// Whether the rule reads the stored version of the document the write
    /// replaces, through [`STORED_DOCUMENT_PREFIX`]: one of its properties, its
    /// owner or one of its system values, each walk stopping at the first.
    pub fn reads_stored(&self) -> bool {
        self.reads_stored_properties()
            || self.compares_identifier(&|path| path == STORED_OWNER_ID)
            || self.reads_system(&|read| read.stored)
    }

    /// Whether the rule reads a property of the stored version
    /// (`$old.<path>`), which the data it is judged against holds under
    /// [`STORED_DOCUMENT_KEY`] ([`DocumentSystemValues::judged_data`]); its
    /// owner and system values, which `system` gives, are not properties.
    pub fn reads_stored_properties(&self) -> bool {
        self.reads_property(&|path| stored_version_path(path).is_some())
    }

    /// Whether the rule compares an identifier, or looks for one among an
    /// array's elements, at a path `matches` accepts.
    fn compares_identifier(&self, matches: &dyn Fn(&str) -> bool) -> bool {
        match self {
            PropertyConstraint::IdentifierCompare { path, .. }
            | PropertyConstraint::IdentifierIn { path, .. }
            | PropertyConstraint::Contains {
                needle: ContainsNeedle::IdentifierProperty(path),
                ..
            } => matches(path),
            PropertyConstraint::IdentifierCompareProperties { left, right, .. } => {
                matches(left) || matches(right)
            }
            PropertyConstraint::AnyOf(conditions) | PropertyConstraint::AllOf(conditions) => {
                conditions
                    .iter()
                    .any(|condition| condition.compares_identifier(matches))
            }
            PropertyConstraint::Not(condition) | PropertyConstraint::NotIn(condition) => {
                condition.compares_identifier(matches)
            }
            PropertyConstraint::IfThen {
                condition,
                then,
                otherwise,
            } => [Some(condition), Some(then), otherwise.as_ref()]
                .into_iter()
                .flatten()
                .any(|part| part.compares_identifier(matches)),
            PropertyConstraint::Compare { .. }
            | PropertyConstraint::In { .. }
            | PropertyConstraint::TextCompare { .. }
            | PropertyConstraint::TextCompareProperties { .. }
            | PropertyConstraint::TextIn { .. }
            | PropertyConstraint::TextAffix { .. }
            | PropertyConstraint::BytesAffix { .. }
            | PropertyConstraint::Contains { .. }
            | PropertyConstraint::Present(_)
            | PropertyConstraint::Absent(_) => false,
        }
    }

    /// Whether consensus judges the write ([`DocumentSystemValues::consensus`])
    /// but `system` does not say what the stored version is while the rule
    /// reads it. Every write consensus judges gives the stored version to the
    /// rules reading it, so one missing is a fault in the code building the
    /// write, not a rule to skip.
    pub fn unread_stored_version(&self, system: &DocumentSystemValues) -> bool {
        system.consensus && system.stored.is_none() && self.reads_stored()
    }

    /// Whether a write changing the top-level properties `fields` of a stored
    /// document, `$revision`, and nothing else a rule reads (a moderator's
    /// change of its fields) can break the rule: it reads one of `fields`,
    /// directly or through `$old.`, `$revision` of either version, or a total,
    /// which may have moved since the document was last written.
    pub fn reads_changed_fields(&self, fields: &BTreeSet<String>) -> bool {
        self.find_aggregate(&|_| true).is_some()
            || self.reads_system(&|read| read.value == SystemValue::Revision)
            || self.reads_property(&|path| {
                let path = either_version_path(path);
                fields.contains(path.split('.').next().unwrap_or(path))
            })
    }

    /// A total the rule reads that consensus did not read: `None` unless
    /// consensus judges the write ([`DocumentSystemValues::consensus`]) and
    /// `system` lacks a total the rule reads. Every write consensus judges reads
    /// the totals of the rules it judges, so one missing is a fault in the code
    /// building the write, not a rule to skip.
    pub fn unread_aggregate<'a>(
        &'a self,
        system: &DocumentSystemValues,
    ) -> Option<&'a AggregateRead> {
        if !system.consensus {
            return None;
        }
        self.find_aggregate(&|read| {
            !system
                .aggregates
                .as_ref()
                .is_some_and(|aggregates| aggregates.contains_key(read))
        })
    }

    /// The first aggregate the rule reads, in declared order, that `matches`,
    /// the walk stopping there rather than collecting every one.
    fn find_aggregate<'a>(
        &'a self,
        matches: &dyn Fn(&AggregateRead) -> bool,
    ) -> Option<&'a AggregateRead> {
        match self {
            PropertyConstraint::Compare { left, right, .. } => left
                .find_aggregate(matches)
                .or_else(|| right.find_aggregate(matches)),
            PropertyConstraint::In { operand, .. } => operand.find_aggregate(matches),
            PropertyConstraint::Contains {
                needle: ContainsNeedle::Integer(expression),
                ..
            } => expression.find_aggregate(matches),
            PropertyConstraint::AnyOf(conditions) | PropertyConstraint::AllOf(conditions) => {
                conditions
                    .iter()
                    .find_map(|condition| condition.find_aggregate(matches))
            }
            PropertyConstraint::Not(condition) | PropertyConstraint::NotIn(condition) => {
                condition.find_aggregate(matches)
            }
            PropertyConstraint::IfThen {
                condition,
                then,
                otherwise,
            } => [Some(condition), Some(then), otherwise.as_ref()]
                .into_iter()
                .flatten()
                .find_map(|part| part.find_aggregate(matches)),
            PropertyConstraint::TextCompare { .. }
            | PropertyConstraint::TextCompareProperties { .. }
            | PropertyConstraint::TextIn { .. }
            | PropertyConstraint::TextAffix { .. }
            | PropertyConstraint::BytesAffix { .. }
            | PropertyConstraint::IdentifierCompare { .. }
            | PropertyConstraint::IdentifierCompareProperties { .. }
            | PropertyConstraint::IdentifierIn { .. }
            | PropertyConstraint::Contains { .. }
            | PropertyConstraint::Present(_)
            | PropertyConstraint::Absent(_) => None,
        }
    }

    /// The aggregates the rule reads (`countOf`, `sumOf`), in declared order,
    /// one read twice listed twice.
    pub fn aggregate_reads(&self) -> Vec<&AggregateRead> {
        let mut reads = Vec::new();
        self.collect_aggregate_reads(&mut reads);
        reads
    }

    fn collect_aggregate_reads<'a>(&'a self, reads: &mut Vec<&'a AggregateRead>) {
        match self {
            PropertyConstraint::Compare { left, right, .. } => {
                left.collect_aggregate_reads(reads);
                right.collect_aggregate_reads(reads);
            }
            PropertyConstraint::In { operand, .. } => operand.collect_aggregate_reads(reads),
            PropertyConstraint::Contains {
                needle: ContainsNeedle::Integer(expression),
                ..
            } => expression.collect_aggregate_reads(reads),
            PropertyConstraint::AnyOf(conditions) | PropertyConstraint::AllOf(conditions) => {
                for condition in conditions {
                    condition.collect_aggregate_reads(reads);
                }
            }
            PropertyConstraint::Not(condition) | PropertyConstraint::NotIn(condition) => {
                condition.collect_aggregate_reads(reads)
            }
            PropertyConstraint::IfThen {
                condition,
                then,
                otherwise,
            } => {
                for part in [Some(condition), Some(then), otherwise.as_ref()]
                    .into_iter()
                    .flatten()
                {
                    part.collect_aggregate_reads(reads);
                }
            }
            PropertyConstraint::TextCompare { .. }
            | PropertyConstraint::TextCompareProperties { .. }
            | PropertyConstraint::TextIn { .. }
            | PropertyConstraint::TextAffix { .. }
            | PropertyConstraint::BytesAffix { .. }
            | PropertyConstraint::IdentifierCompare { .. }
            | PropertyConstraint::IdentifierCompareProperties { .. }
            | PropertyConstraint::IdentifierIn { .. }
            | PropertyConstraint::Contains { .. }
            | PropertyConstraint::Present(_)
            | PropertyConstraint::Absent(_) => {}
        }
    }

    /// The system values the rule reads (`"$createdAt"`, `"$revision"`,
    /// `"$old.$updatedAt"`, ...), in declared order, one read twice listed
    /// twice. `$ownerId` and `$old.$ownerId` are [`Self::reads_owner`]'s.
    pub fn system_reads(&self) -> Vec<SystemRead> {
        let mut reads = Vec::new();
        let _ = self.visit_system_reads(&mut |read| {
            reads.push(read);
            ControlFlow::Continue(())
        });
        reads
    }

    /// Whether `change` can break the rule: for a transfer or a purchase, it
    /// reads the owner, the time and heights of the last transfer or
    /// `$revision`; for a price update, the time and heights of the last update
    /// or `$revision`; either of the document written or, through `$old.`, of
    /// the stored version, which the change makes the version before it. For a
    /// restore, it reads a total. Such a write changes those and no property,
    /// so a rule reading none of them held when the document was last written
    /// and still does: one comparing properties with their stored values only
    /// would compare each with itself.
    pub fn reads_change(&self, change: SystemChange) -> bool {
        match change {
            SystemChange::Restore => self.find_aggregate(&|_| true).is_some(),
            SystemChange::Transfer | SystemChange::PriceUpdate => {
                (change == SystemChange::Transfer && self.reads_owner())
                    || self.reads_system(&|read| read.value.changed_by(change))
            }
        }
    }

    /// Hands `visit` the system values the rule reads, in declared order,
    /// stopping at the first `Break`.
    fn visit_system_reads(
        &self,
        visit: &mut dyn FnMut(SystemRead) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        match self {
            PropertyConstraint::Compare { left, right, .. } => {
                left.visit_system_reads(visit)?;
                right.visit_system_reads(visit)?;
            }
            PropertyConstraint::In { operand, .. } => operand.visit_system_reads(visit)?,
            PropertyConstraint::Contains {
                needle: ContainsNeedle::Integer(expression),
                ..
            } => expression.visit_system_reads(visit)?,
            PropertyConstraint::AnyOf(conditions) | PropertyConstraint::AllOf(conditions) => {
                for condition in conditions {
                    condition.visit_system_reads(visit)?;
                }
            }
            PropertyConstraint::Not(condition) | PropertyConstraint::NotIn(condition) => {
                condition.visit_system_reads(visit)?
            }
            PropertyConstraint::IfThen {
                condition,
                then,
                otherwise,
            } => {
                for part in [Some(condition), Some(then), otherwise.as_ref()]
                    .into_iter()
                    .flatten()
                {
                    part.visit_system_reads(visit)?;
                }
            }
            PropertyConstraint::TextCompare { .. }
            | PropertyConstraint::TextCompareProperties { .. }
            | PropertyConstraint::TextIn { .. }
            | PropertyConstraint::TextAffix { .. }
            | PropertyConstraint::BytesAffix { .. }
            | PropertyConstraint::IdentifierCompare { .. }
            | PropertyConstraint::IdentifierCompareProperties { .. }
            | PropertyConstraint::IdentifierIn { .. }
            | PropertyConstraint::Contains { .. }
            | PropertyConstraint::Present(_)
            | PropertyConstraint::Absent(_) => {}
        }
        ControlFlow::Continue(())
    }

    /// Whether the rule reads a system value `matches` accepts, the walk
    /// stopping at the first.
    fn reads_system(&self, matches: &dyn Fn(SystemRead) -> bool) -> bool {
        self.visit_system_reads(&mut |read| {
            if matches(read) {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })
        .is_break()
    }

    /// Whether the rule reads a property at a path `matches` accepts, the walk
    /// stopping at the first.
    fn reads_property(&self, matches: &dyn Fn(&str) -> bool) -> bool {
        self.visit_property_reads(&mut |path, _| {
            if matches(path) {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })
        .is_break()
    }

    /// Every string constant the rule compares a property with, as the
    /// property's dotted path and the constant, in declared order.
    pub fn text_constants(&self) -> Vec<(&str, &str)> {
        let mut constants = Vec::new();
        self.collect_text_constants(&mut constants);
        constants
    }

    /// Every string default an `ifAbsent` gives a string property, as the
    /// property's dotted path and the default, in declared order.
    pub fn text_defaults(&self) -> Vec<(&str, &str)> {
        self.text_properties()
            .into_iter()
            .filter_map(|property| {
                property
                    .if_absent
                    .as_deref()
                    .map(|default| (property.path.as_str(), default))
            })
            .collect()
    }

    /// Every string constant a `startsWith` or `endsWith` looks for in a string
    /// property, with the property's path and where it is looked for, in
    /// declared order: a property that declares an `enum` must have a value
    /// the constant could start or end, or the condition would never hold.
    pub fn text_affixes(&self) -> Vec<(&str, &str, AffixPosition)> {
        let mut affixes = Vec::new();
        self.collect_text_affixes(&mut affixes);
        affixes
    }

    /// Every pair of properties the rule compares with each other as strings,
    /// in declared order: the two sides of an `equal` or `notEqual` of string
    /// properties, of a `startsWith` or `endsWith` of two, and an array and the
    /// property a `contains` looks for in it. A field of a referenced document
    /// compared so is read as what it holds, which must be what its peer holds.
    pub fn text_peers(&self) -> Vec<(&str, &str)> {
        let mut peers = Vec::new();
        self.collect_text_peers(&mut peers);
        peers
    }

    fn collect_text_peers<'a>(&'a self, peers: &mut Vec<(&'a str, &'a str)>) {
        match self {
            PropertyConstraint::TextCompareProperties { left, right, .. } => {
                peers.push((&left.path, &right.path))
            }
            PropertyConstraint::TextAffix {
                text: TextOperand::Property(text),
                affix: TextOperand::Property(affix),
                ..
            } => peers.push((&text.path, &affix.path)),
            PropertyConstraint::Contains {
                array,
                needle: ContainsNeedle::TextProperty(needle),
            } => peers.push((array, &needle.path)),
            PropertyConstraint::AnyOf(conditions) | PropertyConstraint::AllOf(conditions) => {
                for condition in conditions {
                    condition.collect_text_peers(peers);
                }
            }
            PropertyConstraint::Not(condition) | PropertyConstraint::NotIn(condition) => {
                condition.collect_text_peers(peers)
            }
            PropertyConstraint::IfThen {
                condition,
                then,
                otherwise,
            } => {
                for part in [Some(condition), Some(then), otherwise.as_ref()]
                    .into_iter()
                    .flatten()
                {
                    part.collect_text_peers(peers);
                }
            }
            _ => {}
        }
    }

    fn collect_text_affixes<'a>(&'a self, affixes: &mut Vec<(&'a str, &'a str, AffixPosition)>) {
        match self {
            PropertyConstraint::TextAffix {
                position,
                text: TextOperand::Property(property),
                affix: TextOperand::Constant(value),
            } => affixes.push((&property.path, value, *position)),
            PropertyConstraint::AnyOf(conditions) | PropertyConstraint::AllOf(conditions) => {
                for condition in conditions {
                    condition.collect_text_affixes(affixes);
                }
            }
            PropertyConstraint::Not(condition) | PropertyConstraint::NotIn(condition) => {
                condition.collect_text_affixes(affixes)
            }
            PropertyConstraint::IfThen {
                condition,
                then,
                otherwise,
            } => {
                for part in [Some(condition), Some(then), otherwise.as_ref()]
                    .into_iter()
                    .flatten()
                {
                    part.collect_text_affixes(affixes);
                }
            }
            _ => {}
        }
    }

    /// Every string property the rule's string comparisons read, in declared
    /// order.
    fn text_properties(&self) -> Vec<&TextProperty> {
        let mut properties = Vec::new();
        self.collect_text_properties(&mut properties);
        properties
    }

    fn collect_text_properties<'a>(&'a self, properties: &mut Vec<&'a TextProperty>) {
        match self {
            PropertyConstraint::TextCompare { property, .. }
            | PropertyConstraint::TextIn { property, .. } => properties.push(property),
            PropertyConstraint::TextCompareProperties { left, right, .. } => {
                properties.push(left);
                properties.push(right);
            }
            PropertyConstraint::AnyOf(conditions) | PropertyConstraint::AllOf(conditions) => {
                for condition in conditions {
                    condition.collect_text_properties(properties);
                }
            }
            PropertyConstraint::Not(condition) | PropertyConstraint::NotIn(condition) => {
                condition.collect_text_properties(properties)
            }
            PropertyConstraint::IfThen {
                condition,
                then,
                otherwise,
            } => {
                for part in [Some(condition), Some(then), otherwise.as_ref()]
                    .into_iter()
                    .flatten()
                {
                    part.collect_text_properties(properties);
                }
            }
            PropertyConstraint::TextAffix { text, affix, .. } => {
                for side in [text, affix] {
                    if let TextOperand::Property(property) = side {
                        properties.push(property);
                    }
                }
            }
            PropertyConstraint::Contains {
                needle: ContainsNeedle::TextProperty(property),
                ..
            } => properties.push(property),
            PropertyConstraint::Compare { .. }
            | PropertyConstraint::In { .. }
            | PropertyConstraint::BytesAffix { .. }
            | PropertyConstraint::IdentifierCompare { .. }
            | PropertyConstraint::IdentifierCompareProperties { .. }
            | PropertyConstraint::IdentifierIn { .. }
            | PropertyConstraint::Contains { .. }
            | PropertyConstraint::Present(_)
            | PropertyConstraint::Absent(_) => {}
        }
    }

    fn collect_text_constants<'a>(&'a self, constants: &mut Vec<(&'a str, &'a str)>) {
        match self {
            PropertyConstraint::TextCompare {
                property, value, ..
            } => constants.push((&property.path, value)),
            PropertyConstraint::TextIn { property, values } => constants.extend(
                values
                    .iter()
                    .map(|value| (property.path.as_str(), value.as_str())),
            ),
            PropertyConstraint::AnyOf(conditions) | PropertyConstraint::AllOf(conditions) => {
                for condition in conditions {
                    condition.collect_text_constants(constants);
                }
            }
            PropertyConstraint::Not(condition) | PropertyConstraint::NotIn(condition) => {
                condition.collect_text_constants(constants)
            }
            PropertyConstraint::IfThen {
                condition,
                then,
                otherwise,
            } => {
                for part in [Some(condition), Some(then), otherwise.as_ref()]
                    .into_iter()
                    .flatten()
                {
                    part.collect_text_constants(constants);
                }
            }
            // Checked against the enum of the array's elements
            PropertyConstraint::Contains {
                array,
                needle: ContainsNeedle::TextConstant(value),
            } => constants.push((array, value)),
            PropertyConstraint::Compare { .. }
            | PropertyConstraint::In { .. }
            | PropertyConstraint::TextCompareProperties { .. }
            | PropertyConstraint::TextAffix { .. }
            | PropertyConstraint::BytesAffix { .. }
            | PropertyConstraint::IdentifierCompare { .. }
            | PropertyConstraint::IdentifierCompareProperties { .. }
            | PropertyConstraint::IdentifierIn { .. }
            | PropertyConstraint::Contains { .. }
            | PropertyConstraint::Present(_)
            | PropertyConstraint::Absent(_) => {}
        }
    }

    /// Where an `anyOf` or `allOf` of the rule lists the same condition twice,
    /// or an `ifThen` or `ifThenElse` holds two alike (a then-branch equal to
    /// the condition, or two equal branches, says what a simpler rule says):
    /// the repeat's place and the earlier one's (`anyOf[2]` and `anyOf[0]`),
    /// the first found in declared order, `None` when no list does. Conditions
    /// are alike when they parse alike, so `1` and `1.0` are the same value,
    /// `"price"` and `{ "ifAbsent": ["price", 0] }` the same operand, and two
    /// `in`s listing the same values in another order the same condition.
    /// Checked under full validation with the limits, which bound the lists it
    /// compares; a stored rule was checked when its contract registered.
    pub fn repeated_condition(&self) -> Option<(String, String)> {
        self.find_repeated_condition(&mut String::new())
    }

    /// [`Self::repeated_condition`] for the condition at `at` (empty for the
    /// rule's own), which is extended as the walk descends and trimmed back
    /// when it returns `None`.
    fn find_repeated_condition(&self, at: &mut String) -> Option<(String, String)> {
        let (key, conditions) = match self {
            PropertyConstraint::Compare { .. }
            | PropertyConstraint::In { .. }
            | PropertyConstraint::TextCompare { .. }
            | PropertyConstraint::TextCompareProperties { .. }
            | PropertyConstraint::TextIn { .. }
            | PropertyConstraint::TextAffix { .. }
            | PropertyConstraint::BytesAffix { .. }
            | PropertyConstraint::IdentifierCompare { .. }
            | PropertyConstraint::IdentifierCompareProperties { .. }
            | PropertyConstraint::IdentifierIn { .. }
            | PropertyConstraint::Contains { .. }
            | PropertyConstraint::Present(_)
            | PropertyConstraint::Absent(_)
            | PropertyConstraint::NotIn(_) => return None,
            PropertyConstraint::AnyOf(conditions) => (ANY_OF, conditions),
            PropertyConstraint::AllOf(conditions) => (ALL_OF, conditions),
            PropertyConstraint::Not(condition) => {
                let parent = enter(at, NOT);
                let found = condition.find_repeated_condition(at);
                at.truncate(parent);
                return found;
            }
            PropertyConstraint::IfThen {
                condition,
                then,
                otherwise,
            } => {
                let key = if otherwise.is_some() {
                    IF_THEN_ELSE
                } else {
                    IF_THEN
                };
                let parent = enter(at, key);
                let parts: Vec<&PropertyConstraint> =
                    [Some(condition), Some(then), otherwise.as_ref()]
                        .into_iter()
                        .flatten()
                        .map(|part| part.as_ref())
                        .collect();
                let base = at.len();
                for (index, part) in parts.iter().enumerate() {
                    if let Some(earlier) = parts[..index].iter().position(|earlier| earlier == part)
                    {
                        return Some((format!("{at}[{index}]"), format!("{at}[{earlier}]")));
                    }
                    // Writing to a `String` cannot fail
                    let _ = write!(at, "[{index}]");
                    if let Some(found) = part.find_repeated_condition(at) {
                        return Some(found);
                    }
                    at.truncate(base);
                }
                at.truncate(parent);
                return None;
            }
        };
        let parent = enter(at, key);
        let base = at.len();
        for (index, condition) in conditions.iter().enumerate() {
            if let Some(earlier) = conditions[..index]
                .iter()
                .position(|earlier| earlier == condition)
            {
                return Some((format!("{at}[{index}]"), format!("{at}[{earlier}]")));
            }
            // Writing to a `String` cannot fail
            let _ = write!(at, "[{index}]");
            if let Some(found) = condition.find_repeated_condition(at) {
                return Some(found);
            }
            at.truncate(base);
        }
        at.truncate(parent);
        None
    }

    /// Hands `visit` the properties the rule reads, each with how it reads
    /// it, in declared order, stopping at the first `Break`.
    fn visit_property_reads<'a>(
        &'a self,
        visit: &mut dyn FnMut(&'a str, PropertyRead) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        match self {
            PropertyConstraint::Compare { left, right, .. } => {
                left.visit_property_reads(visit)?;
                right.visit_property_reads(visit)?;
            }
            PropertyConstraint::In { operand, .. } => operand.visit_property_reads(visit)?,
            PropertyConstraint::TextCompare { property, .. }
            | PropertyConstraint::TextIn { property, .. } => {
                visit(&property.path, PropertyRead::Text)?
            }
            PropertyConstraint::TextCompareProperties { left, right, .. } => {
                visit(&left.path, PropertyRead::Text)?;
                visit(&right.path, PropertyRead::Text)?;
            }
            // `$ownerId` and `$old.$ownerId` are owners, no properties
            PropertyConstraint::IdentifierCompare { path, .. }
            | PropertyConstraint::IdentifierIn { path, .. } => {
                if !is_owner_path(path) {
                    visit(path, PropertyRead::Identifier)?
                }
            }
            PropertyConstraint::IdentifierCompareProperties { left, right, .. } => {
                for path in [left, right] {
                    if !is_owner_path(path) {
                        visit(path, PropertyRead::Identifier)?;
                    }
                }
            }
            PropertyConstraint::TextAffix { text, affix, .. } => {
                for side in [text, affix] {
                    if let TextOperand::Property(property) = side {
                        visit(&property.path, PropertyRead::Text)?;
                    }
                }
            }
            PropertyConstraint::BytesAffix { bytes, affix, .. } => {
                // A constant the array must start or end with is as long as
                // the array must be
                if let BytesOperand::Property(path) = bytes {
                    let extent = match affix {
                        BytesOperand::Constant(constant) => constant.len(),
                        BytesOperand::Property(_) => 0,
                    };
                    visit(path, PropertyRead::Bytes { extent })?;
                }
                if let BytesOperand::Property(path) = affix {
                    visit(path, PropertyRead::Bytes { extent: 0 })?;
                }
            }
            PropertyConstraint::Contains { array, needle } => match needle {
                ContainsNeedle::Integer(expression) => {
                    visit(array, PropertyRead::Elements(ElementKind::Integer))?;
                    expression.visit_property_reads(visit)?;
                }
                ContainsNeedle::TextConstant(_) => {
                    visit(array, PropertyRead::Elements(ElementKind::Text))?
                }
                ContainsNeedle::TextProperty(property) => {
                    visit(array, PropertyRead::Elements(ElementKind::Text))?;
                    visit(&property.path, PropertyRead::Text)?;
                }
                ContainsNeedle::IdentifierConstant(_) => {
                    visit(array, PropertyRead::Elements(ElementKind::Identifier))?
                }
                ContainsNeedle::IdentifierProperty(path) => {
                    visit(array, PropertyRead::Elements(ElementKind::Identifier))?;
                    // `$ownerId` and `$old.$ownerId` are owners, no properties
                    if !is_owner_path(path) {
                        visit(path, PropertyRead::Identifier)?;
                    }
                }
            },
            PropertyConstraint::Present(path) | PropertyConstraint::Absent(path) => {
                visit(path, PropertyRead::Presence)?
            }
            PropertyConstraint::AnyOf(conditions) | PropertyConstraint::AllOf(conditions) => {
                for condition in conditions {
                    condition.visit_property_reads(visit)?;
                }
            }
            PropertyConstraint::Not(condition) | PropertyConstraint::NotIn(condition) => {
                condition.visit_property_reads(visit)?
            }
            PropertyConstraint::IfThen {
                condition,
                then,
                otherwise,
            } => {
                for part in [Some(condition), Some(then), otherwise.as_ref()]
                    .into_iter()
                    .flatten()
                {
                    part.visit_property_reads(visit)?;
                }
            }
        }
        ControlFlow::Continue(())
    }
}

/// What the parse of one document type's rules knows of the type: its name,
/// which tells an aggregate of the type's own documents from one of another
/// type's, and `property_kind` ([`parse_property_constraints`]).
struct ParseContext<'a> {
    document_type_name: &'a str,
    property_kind: &'a dyn Fn(&str) -> Option<EqualityKind>,
}

impl ParseContext<'_> {
    /// How a bare path compares: `$ownerId` and `$old.$ownerId` as
    /// identifiers, and a property, of either version, as `property_kind`
    /// says of its path in the type.
    fn kind_of(&self, path: &str) -> Option<EqualityKind> {
        if is_owner_path(path) {
            return Some(EqualityKind::Identifier);
        }
        (self.property_kind)(either_version_path(path))
    }
}

/// Reads the `propertyConstraints` keyword of a document type's `schema`:
/// every rule by its name, in name order, the order a document is checked
/// against them. Empty when the schema declares none. `property_kind` tells
/// which dotted paths name string or identifier properties of the document
/// type: an `equal` or `notEqual` with a `const`, or of two bare paths naming
/// such properties, compares strings or identifiers as they decide, as does
/// an `in` listing strings; any other comparison of two expressions compares
/// integers.
///
/// The rules of the declaration's shape are checked here, on every parse: an
/// object of one or more rules, each named with 1 to 64 letters, digits or
/// underscores and holding one condition. A condition is an object with one
/// key: a comparison of exactly two operands, `in` with an operand and a list
/// of two or more distinct integer values, `equal` or `notEqual` of a property
/// and a `{ "const": ... }` (a string, or a base58 identifier for an
/// identifier property) or of two string or two identifier properties, `in`
/// with a property and two or more distinct strings or base58 identifiers,
/// `present` or `absent` with a property path, `anyOf` or `allOf` with two or
/// more conditions, none of them directly the same operator (it says what one
/// flat list says), or `not` with one condition that is not directly another
/// `not`. An operand is an integer value, a property path, or an object with
/// one key: `ifAbsent` with a path and an integer value, `add` or `multiply`
/// with two or more operands, `subtract`, `divide`, `modulo` or `power`
/// with exactly two, `countPresent` with two or more distinct property paths,
/// or `byteAt` with a path and the index of a byte, an integer from 0 to
/// 65535; a string property may take an `ifAbsent` with a string default
/// instead. A `startsWith` or `endsWith` with a bare path naming a byte array
/// property on either side tests byte arrays: each side such a path or a
/// `const` of hex digits, two a byte. An integer value may be spelled as a
/// float with no fractional part, as the meta-schema's `integer` type admits
/// one. A literal
/// 0 divisor, a literal negative exponent, a comparison or `in` that reads no
/// property, which would hold for every document or for none, an ordering
/// comparison of strings or identifiers, and a condition or operand deeper
/// than [`MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH`] are refused. What the paths
/// name is checked against the parsed document type, and the limits and that
/// no list repeats a condition under full validation, by parser generation 3.
pub fn parse_property_constraints(
    schema: &Value,
    document_type_name: &str,
    property_kind: &dyn Fn(&str) -> Option<EqualityKind>,
) -> Result<BTreeMap<String, PropertyConstraint>, DataContractError> {
    parse_named_rules(
        schema,
        property_names::PROPERTY_CONSTRAINTS,
        document_type_name,
        property_kind,
    )
}

/// Reads the `deleteConstraints` keyword of a document type's `schema`: the
/// rules, by name and in name order, the stored document must meet for its
/// owner to delete it. Their grammar and the rules of their shape are those of
/// `propertyConstraints` ([`parse_property_constraints`]); a rule is judged on
/// the stored document, and a `countOf` or `sumOf` it reads as the total will
/// be once the document is gone. Empty when the schema declares none. Which
/// type may declare them, and what their paths name, is checked against the
/// parsed document type by parser generation 3.
pub fn parse_delete_constraints(
    schema: &Value,
    document_type_name: &str,
    property_kind: &dyn Fn(&str) -> Option<EqualityKind>,
) -> Result<BTreeMap<String, PropertyConstraint>, DataContractError> {
    parse_named_rules(
        schema,
        property_names::DELETE_CONSTRAINTS,
        document_type_name,
        property_kind,
    )
}

/// Reads the rules `keyword` of `schema` declares by name
/// ([`parse_property_constraints`]).
fn parse_named_rules(
    schema: &Value,
    keyword: &str,
    document_type_name: &str,
    property_kind: &dyn Fn(&str) -> Option<EqualityKind>,
) -> Result<BTreeMap<String, PropertyConstraint>, DataContractError> {
    let structure_error = |message: String| {
        DataContractError::InvalidContractStructure(format!(
            "document type \"{document_type_name}\" {keyword} {message}"
        ))
    };
    // A schema that is not an object carries no keyword: the core parser
    // refuses it, and a value error here must not replace that refusal
    let Ok(schema_map) = schema.to_map() else {
        return Ok(BTreeMap::new());
    };
    let Some(declaration) = schema_map.get_optional_key(keyword) else {
        return Ok(BTreeMap::new());
    };
    let Value::Map(rules) = declaration else {
        return Err(structure_error(
            "must be an object of rules by name".to_string(),
        ));
    };
    if rules.is_empty() {
        return Err(structure_error(
            "must declare at least one rule".to_string(),
        ));
    }

    let context = ParseContext {
        document_type_name,
        property_kind,
    };
    let mut constraints = BTreeMap::new();
    for (name, rule) in rules {
        let Some(name) = name.as_text().filter(|name| is_rule_name(name)) else {
            return Err(structure_error(format!(
                "names a rule \"{}\", but a rule name is 1 to 64 letters, digits or underscores",
                name.non_qualified_string_representation()
            )));
        };
        // Where a condition or an operand sits in the rule (`anyOf[1].lessThan[0]`),
        // grown and trimmed in place as the parse descends and only read into an error
        let constraint = parse_condition(rule, &mut String::new(), 0, &context)
            .map_err(|message| structure_error(format!("rule \"{name}\" {message}")))?;
        if constraints.insert(name.to_string(), constraint).is_some() {
            return Err(structure_error(format!("declares rule \"{name}\" twice")));
        }
    }
    Ok(constraints)
}

/// Reads one condition, the `when` of an `immutable` entry, with the grammar
/// and the shape rules of a `propertyConstraints` rule ([`parse_property_constraints`]).
/// `property_kind` is as there: the parse hands it the path a condition names
/// through [`STORED_DOCUMENT_PREFIX`] as the path after it
/// ([`either_version_path`]). The error is the rest of a message
/// naming the condition. What the paths name is checked against the parsed
/// document type by parser generation 3.
pub fn parse_property_constraint_condition(
    condition: &Value,
    document_type_name: &str,
    property_kind: &dyn Fn(&str) -> Option<EqualityKind>,
) -> Result<PropertyConstraint, String> {
    let context = ParseContext {
        document_type_name,
        property_kind,
    };
    parse_condition(condition, &mut String::new(), 0, &context)
}

/// The path `value` names where a property path stands: a bare string, or a
/// `referenced` operand, `{ "referenced": [reference, field] }`, read through
/// [`REFERENCED_DOCUMENT_PREFIX`]. `None` for anything else, `Some(Err)` for a
/// `referenced` operand that is malformed, the error placing it at `at`. A
/// bare string naming the path a `referenced` operand is read through is
/// refused: there is one way to write the operand.
fn named_path(value: &Value, at: &str) -> Option<Result<String, String>> {
    if let Some(path) = value.as_text() {
        return Some(bare_path(path, at));
    }
    let (REFERENCED, operands) = single_entry(value)? else {
        return None;
    };
    Some(referenced_operand(operands, at))
}

/// A path written as a bare string at `at`, refused when it names the path a
/// `referenced` operand is read through, which has one spelling.
fn bare_path(path: &str, at: &str) -> Result<String, String> {
    if path.starts_with(REFERENCED_DOCUMENT_KEY) {
        return Err(format!(
            "{}names \"{path}\": a value of the document a reference points at is written \
             {{ \"{REFERENCED}\": [reference, field] }}",
            located(at)
        ));
    }
    Ok(path.to_string())
}

/// The path a `referenced` operand at `at` listing `operands` is read through:
/// exactly two strings, the dotted path of a property of the type (the
/// reference, a nested one through the objects around it, whose declaration is
/// checked against the parsed document type), then a field of the document it
/// points at: a dotted property path, its `$id`, `$ownerId` or `$creatorId`, or
/// one of the nine times and heights.
fn referenced_operand(operands: &Value, at: &str) -> Result<String, String> {
    let at = if at.is_empty() {
        REFERENCED.to_string()
    } else {
        format!("{at}.{REFERENCED}")
    };
    let Some([Value::Text(reference), Value::Text(field)]) = operands.as_array().map(Vec::as_slice)
    else {
        return Err(format!(
            "at {at} must list a reference property of the document type, then a field of the \
             document it refers to"
        ));
    };
    if reference.is_empty() || !reference.split('.').all(is_rule_name) {
        return Err(format!(
            "at {at}[0] names \"{reference}\", but a reference property is the dotted path \
             of a property of the type, each name of 1 to 64 letters, digits or underscores"
        ));
    }
    let is_field = is_referenced_system_agreement_property(field)
        || SystemProperty::from_name(field).is_some()
        || (!field.is_empty() && field.split('.').all(is_rule_name));
    if !is_field {
        return Err(format!(
            "at {at}[1] names \"{field}\", but a field of the referenced document is a \
             property path of its type, its $id, $ownerId or $creatorId, or a time or height \
             it records"
        ));
    }
    Ok(referenced_path(reference, field))
}

/// How a condition compares the value at `path`: `$ownerId`, `$old.$ownerId`
/// and the identifiers of a referenced document as identifiers, a property of
/// the document type, of either version, as `property_kind` says
/// ([`ParseContext::kind_of`]), and any other value of a referenced document
/// not at all ([`has_open_kind`]).
fn path_kind(context: &ParseContext, path: &str) -> Option<EqualityKind> {
    if let Some((_, field)) = referenced_read(path) {
        return is_referenced_system_agreement_property(field).then_some(EqualityKind::Identifier);
    }
    context.kind_of(path)
}

/// Whether the value at `path` is a schema property of a referenced document,
/// whose type the parse of the declaring type does not know (it may be another
/// contract's): a comparison then reads it as what it is compared with says, a
/// string or an identifier beside a property of that kind, a string beside a
/// `const` or in an `in` listing strings, an integer otherwise, and the read is
/// checked against the referenced type at registration.
fn has_open_kind(path: &str) -> bool {
    referenced_read(path).is_some_and(|(_, field)| !field.starts_with('$'))
}

/// The kinds the two sides of a comparison, `left` and `right`, compare as
/// ([`path_kind`]), a side of an open kind ([`has_open_kind`]) taking the
/// other side's.
fn side_kinds(
    context: &ParseContext,
    left: &Value,
    right: &Value,
) -> (Option<EqualityKind>, Option<EqualityKind>) {
    let path = |value: &Value| named_path(value, "").and_then(Result::ok);
    let (left_path, right_path) = (path(left), path(right));
    let kind = |path: &Option<String>| path.as_deref().and_then(|path| path_kind(context, path));
    let open = |path: &Option<String>| path.as_deref().is_some_and(has_open_kind);
    let (left_kind, right_kind) = (kind(&left_path), kind(&right_path));
    (
        left_kind.or(if open(&left_path) { right_kind } else { None }),
        right_kind.or(if open(&right_path) { left_kind } else { None }),
    )
}

/// Whether `name` can name a rule: 1 to 64 letters, digits or underscores, as
/// a property name.
fn is_rule_name(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// The one key of an object and its value: `None` when `value` is not an
/// object with exactly one text key.
fn single_entry(value: &Value) -> Option<(&str, &Value)> {
    let Value::Map(entries) = value else {
        return None;
    };
    let [(key, value)] = entries.as_slice() else {
        return None;
    };
    Some((key.as_text()?, value))
}

/// Every key a condition object may hold, for the errors.
fn condition_keys() -> String {
    format!(
        "a comparison ({}), in, notIn, startsWith, endsWith, contains, present, absent, \
         anyOf, allOf, not, ifThen or ifThenElse",
        ConstraintComparison::ALL
            .map(ConstraintComparison::wire_name)
            .join(", ")
    )
}

/// `at ` followed by where something sits in its rule, nothing for the rule's
/// own condition, to open the rest of an error naming the rule.
fn located(at: &str) -> String {
    if at.is_empty() {
        String::new()
    } else {
        format!("at {at} ")
    }
}

/// Extends `at`, where a condition sits in its rule (empty for the rule's
/// own), to the place of its `key`, returning the length to trim it back to.
fn enter(at: &mut String, key: &str) -> usize {
    let parent = at.len();
    if parent > 0 {
        at.push('.');
    }
    at.push_str(key);
    parent
}

/// A condition at `at` (`anyOf[1]`, empty for the rule's own), where the errors
/// place it, `depth` levels into its rule: an object whose one key is a
/// comparison listing its two sides, `in` listing an operand and its values,
/// `present` or `absent` naming a property, or `anyOf`, `allOf` or `not`. The
/// error is the rest of a message naming the rule. `at` is extended for what
/// the condition holds and trimmed back before a successful return.
fn parse_condition(
    value: &Value,
    at: &mut String,
    depth: usize,
    context: &ParseContext,
) -> Result<PropertyConstraint, String> {
    if depth > MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH {
        return Err(format!(
            "{}nests deeper than {MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH} levels",
            located(at)
        ));
    }
    let Some((key, body)) = single_entry(value) else {
        return Err(format!(
            "{}must be an object with one key: {}",
            located(at),
            condition_keys()
        ));
    };
    let parent = enter(at, key);
    // `$ownerId` and `$old.$ownerId` compare as identifier properties, and so do
    // the identifiers of a referenced document
    let kind_of = |path: &str| path_kind(context, path);
    let condition = match key {
        ANY_OF => PropertyConstraint::AnyOf(condition_list(body, key, at, depth + 1, context)?),
        ALL_OF => PropertyConstraint::AllOf(condition_list(body, key, at, depth + 1, context)?),
        NOT => {
            if single_entry(body).is_some_and(|(inner, _)| inner == NOT) {
                return Err(format!(
                    "at {at}.{NOT} is a not directly inside a not, which says what the \
                     condition inside it says: declare that condition"
                ));
            }
            if single_entry(body).is_some_and(|(inner, _)| inner == NOT_IN) {
                return Err(format!(
                    "at {at}.{NOT_IN} is a notIn directly inside a not, which says what an in \
                     of the same values says: declare that in"
                ));
            }
            PropertyConstraint::Not(Box::new(parse_condition(body, at, depth + 1, context)?))
        }
        IF_THEN | IF_THEN_ELSE => {
            let base = at.len();
            let mut part = |index: usize, value: &Value| {
                // Writing to a `String` cannot fail
                let _ = write!(at, "[{index}]");
                let parsed = parse_condition(value, at, depth + 1, context);
                at.truncate(base);
                parsed.map(Box::new)
            };
            match (key, body.as_array().map(Vec::as_slice)) {
                (IF_THEN, Some([condition, then])) => PropertyConstraint::IfThen {
                    condition: part(0, condition)?,
                    then: part(1, then)?,
                    otherwise: None,
                },
                (IF_THEN_ELSE, Some([condition, then, otherwise])) => PropertyConstraint::IfThen {
                    condition: part(0, condition)?,
                    then: part(1, then)?,
                    otherwise: Some(part(2, otherwise)?),
                },
                (IF_THEN, _) => {
                    return Err(format!(
                        "at {at} must list two conditions: the condition, then the one that \
                         must hold when it does"
                    ));
                }
                _ => {
                    return Err(format!(
                        "at {at} must list three conditions: the condition, the one that must \
                         hold when it does, and the one that must hold when it does not"
                    ));
                }
            }
        }
        IN | NOT_IN => {
            let Some([operand, values]) = body.as_array().map(Vec::as_slice) else {
                return Err(format!(
                    "at {at} must list an integer expression and the values it may {}take",
                    if key == NOT_IN { "not " } else { "" }
                ));
            };
            let base = at.len();
            // The values are literals, so a string among them is a string rather
            // than a path, and the first one decides what the in compares
            let over_strings = values
                .as_array()
                .and_then(|values| values.first())
                .is_some_and(|first| first.as_text().is_some());
            // Strings listed for an identifier property are its identifiers, base58
            let identifier_path = match named_path(operand, &format!("{at}[0]")) {
                Some(Err(error)) => return Err(error),
                Some(Ok(path))
                    if over_strings && kind_of(&path) == Some(EqualityKind::Identifier) =>
                {
                    Some(path)
                }
                _ => None,
            };
            let listed = if let Some(path) = identifier_path {
                at.push_str("[1]");
                let values = in_identifier_values(values, at)?;
                at.truncate(base);
                PropertyConstraint::IdentifierIn { path, values }
            } else if over_strings {
                let Ok(TextSide::Property(property)) = text_side(operand, &format!("{at}[0]"))
                else {
                    return Err(format!(
                        "at {at}[0] must be the path of a string property or an ifAbsent giving \
                         one a string default: {} over strings reads a string property",
                        if key == NOT_IN { "a notIn" } else { "an in" }
                    ));
                };
                at.push_str("[1]");
                let values = in_text_values(values, at)?;
                at.truncate(base);
                PropertyConstraint::TextIn { property, values }
            } else {
                at.push_str("[0]");
                let operand = parse_expression(operand, at, depth + 1, context)?;
                at.truncate(base);
                if !operand.reads_property() {
                    at.truncate(parent);
                    return Err(format!(
                        "{}reads no property, so it would hold for every document or for none",
                        located(at)
                    ));
                }
                at.push_str("[1]");
                let values = in_values(values, at)?;
                at.truncate(base);
                PropertyConstraint::In { operand, values }
            };
            if key == NOT_IN {
                PropertyConstraint::NotIn(Box::new(listed))
            } else {
                listed
            }
        }
        STARTS_WITH | ENDS_WITH => {
            let Some([text, affix]) = body.as_array().map(Vec::as_slice) else {
                return Err(format!(
                    "at {at} must list two strings: the one tested, then the one it must {} with",
                    if key == STARTS_WITH { "start" } else { "end" }
                ));
            };
            let position = if key == STARTS_WITH {
                AffixPosition::Start
            } else {
                AffixPosition::End
            };
            // A bare path naming a byte array property on either side tests byte
            // arrays, and a const beside it is written in hex
            match side_kinds(context, text, affix) {
                (Some(EqualityKind::Bytes), None | Some(EqualityKind::Bytes))
                | (None, Some(EqualityKind::Bytes)) => {
                    let bytes = bytes_operand(text, &format!("{at}[0]"))?;
                    let affix = bytes_operand(affix, &format!("{at}[1]"))?;
                    if let (BytesOperand::Property(bytes), BytesOperand::Property(affix)) =
                        (&bytes, &affix)
                    {
                        if bytes == affix {
                            return Err(format!(
                                "at {at} tests \"{bytes}\" against itself, so it would hold for \
                                 every document or for none"
                            ));
                        }
                    }
                    at.truncate(parent);
                    return Ok(PropertyConstraint::BytesAffix {
                        position,
                        bytes,
                        affix,
                    });
                }
                (Some(EqualityKind::Bytes), Some(_)) | (Some(_), Some(EqualityKind::Bytes)) => {
                    return Err(format!(
                        "at {at} tests a byte array property against a property that is not \
                         one: a byte array starts or ends with a byte array"
                    ));
                }
                _ => {}
            }
            let text = text_operand(text, &format!("{at}[0]"))?;
            let affix = text_operand(affix, &format!("{at}[1]"))?;
            match (&text, &affix) {
                (TextOperand::Constant(_), TextOperand::Constant(_)) => {
                    at.truncate(parent);
                    return Err(format!(
                        "{}reads no property, so it would hold for every document or for none",
                        located(at)
                    ));
                }
                (TextOperand::Property(text), TextOperand::Property(affix))
                    if text.path == affix.path =>
                {
                    return Err(format!(
                        "at {at} tests \"{}\" against itself, so it would hold for every \
                         document or for none",
                        text.path
                    ));
                }
                _ => {}
            }
            PropertyConstraint::TextAffix {
                position,
                text,
                affix,
            }
        }
        CONTAINS => {
            let Some([array, needle]) = body.as_array().map(Vec::as_slice) else {
                return Err(format!(
                    "at {at} must list an array property path and the value looked for among \
                     its elements"
                ));
            };
            // `$ownerId` and the system values are values, never arrays; an
            // array of the stored version is named through `$old.`
            let array = match named_path(array, &format!("{at}[0]")) {
                Some(Err(error)) => return Err(error),
                Some(Ok(path))
                    if !either_version_path(&path).starts_with('$')
                        || path.starts_with(REFERENCED_DOCUMENT_PREFIX) =>
                {
                    path
                }
                _ => return Err(format!("at {at}[0] must name an array property path")),
            };
            let base = at.len();
            at.push_str("[1]");
            // The kind of the array's elements decides what a const spells, and
            // what is checked against the parsed document type. An array of a
            // referenced document is of the kind of what is looked for in it: a
            // string for a const or a string default, the kind of a path
            let array_kind = context.kind_of(&array).or_else(|| {
                if !has_open_kind(&array) {
                    return None;
                }
                if is_const(needle) || is_text_if_absent(needle) {
                    return Some(EqualityKind::Text);
                }
                named_path(needle, "")
                    .and_then(Result::ok)
                    .and_then(|path| path_kind(context, &path))
            });
            let needle = match array_kind {
                Some(EqualityKind::Text) => match text_side(needle, at)? {
                    TextSide::Constant(value) => ContainsNeedle::TextConstant(value),
                    TextSide::Property(property) => ContainsNeedle::TextProperty(property),
                },
                Some(EqualityKind::Identifier) => match identifier_side(needle, at)? {
                    IdentifierSide::Constant(value) => ContainsNeedle::IdentifierConstant(value),
                    IdentifierSide::Property(path) => ContainsNeedle::IdentifierProperty(path),
                },
                // A byte array is no array of elements: looked in for an integer, as
                // an array of integers is, it is refused against the parsed type
                Some(EqualityKind::Bytes) | None if is_const(needle) => {
                    return Err(format!(
                        "at {at} is a const, but {array} holds no strings or identifiers: an \
                         integer is written as itself"
                    ));
                }
                Some(EqualityKind::Bytes) | None => {
                    ContainsNeedle::Integer(parse_expression(needle, at, depth + 1, context)?)
                }
            };
            at.truncate(base);
            PropertyConstraint::Contains { array, needle }
        }
        // What the path names is checked against the parsed document type
        PRESENT | ABSENT => {
            let path = match named_path(body, at) {
                Some(path) => path?,
                None => return Err(format!("at {at} must name a property path")),
            };
            if key == PRESENT {
                PropertyConstraint::Present(path)
            } else {
                PropertyConstraint::Absent(path)
            }
        }
        _ => {
            let Some(comparison) = ConstraintComparison::ALL
                .into_iter()
                .find(|comparison| comparison.wire_name() == key)
            else {
                at.truncate(parent);
                return Err(format!(
                    "{}names \"{key}\", which is not {}",
                    located(at),
                    condition_keys()
                ));
            };
            if let Some([left, right]) = body.as_array().map(Vec::as_slice) {
                // A const or an ifAbsent with a string default on either side, or
                // two bare paths naming string or identifier properties, compare
                // strings or identifiers, as the properties named decide
                let (left_kind, right_kind) = side_kinds(context, left, right);
                // Two fields of referenced documents, whose types the parse does
                // not know, compared with each other by value, whatever they hold
                let open = |side: &Value| {
                    named_path(side, "")
                        .and_then(Result::ok)
                        .is_some_and(|path| has_open_kind(&path))
                };
                let open_pair = matches!(
                    comparison,
                    ConstraintComparison::Equal | ConstraintComparison::NotEqual
                ) && open(left)
                    && open(right);
                if is_const(left)
                    || is_const(right)
                    || is_text_if_absent(left)
                    || is_text_if_absent(right)
                    || (left_kind.is_some() && right_kind.is_some())
                    || open_pair
                {
                    let compare = match (left_kind, right_kind) {
                        (Some(EqualityKind::Bytes), _) | (_, Some(EqualityKind::Bytes)) => {
                            return Err(format!(
                                "at {at} compares a byte array property, which only startsWith \
                                 and endsWith test, against a {{ \"const\": hex }} or another \
                                 byte array property: byteAt reads one of its bytes"
                            ));
                        }
                        (Some(left_kind), Some(right_kind)) if left_kind != right_kind => {
                            return Err(format!(
                                "at {at} compares a string property with an identifier \
                                 property"
                            ));
                        }
                        (Some(EqualityKind::Identifier), _)
                        | (_, Some(EqualityKind::Identifier)) => {
                            identifier_comparison(comparison, left, right, at)?
                        }
                        _ => text_comparison(comparison, left, right, at)?,
                    };
                    at.truncate(parent);
                    return compare.ok_or_else(|| {
                        format!(
                            "{}reads no property, so it would hold for every document or for \
                             none",
                            located(at)
                        )
                    });
                }
            }
            let (left, right) = operand_pair(body, at, depth + 1, context)?;
            if !left.reads_property() && !right.reads_property() {
                at.truncate(parent);
                return Err(format!(
                    "{}reads no property, so it would hold for every document or for none",
                    located(at)
                ));
            }
            PropertyConstraint::Compare {
                comparison,
                left,
                right,
            }
        }
    };
    at.truncate(parent);
    Ok(condition)
}

/// The two or more conditions the `anyOf` or `allOf` named `key` lists at
/// `at`, `depth` levels into their rule, none of them directly another `key`,
/// which says what one flat list says. That no two are alike is checked under
/// full validation ([`PropertyConstraint::repeated_condition`]).
fn condition_list(
    conditions: &Value,
    key: &str,
    at: &mut String,
    depth: usize,
    context: &ParseContext,
) -> Result<Vec<PropertyConstraint>, String> {
    let Some(values) = conditions.as_array().filter(|values| values.len() >= 2) else {
        return Err(format!("at {at} must list two or more conditions"));
    };
    let base = at.len();
    let mut parsed = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        // Writing to a `String` cannot fail
        let _ = write!(at, "[{index}]");
        if single_entry(value).is_some_and(|(inner, _)| inner == key) {
            return Err(format!(
                "at {at} is an {key} directly inside an {key}, which says what one flat list \
                 says: list its conditions in the outer {key}"
            ));
        }
        parsed.push(parse_condition(value, at, depth, context)?);
        at.truncate(base);
    }
    Ok(parsed)
}

/// The values an `in` lists at `at` (`in[1]`): two or more integer literals,
/// no two alike, `1` and `1.0` being the same value. A duplicate is refused on
/// every parse: unlike a repeated condition it costs a set insertion to find.
fn in_values(values: &Value, at: &mut String) -> Result<BTreeSet<i128>, String> {
    let Some(values) = values.as_array().filter(|values| values.len() >= 2) else {
        return Err(format!("at {at} must list two or more integer values"));
    };
    let base = at.len();
    // Each value with the index it first appears at, for the errors
    let mut seen = BTreeMap::new();
    for (index, value) in values.iter().enumerate() {
        // Writing to a `String` cannot fail
        let _ = write!(at, "[{index}]");
        if !is_number(value) {
            return Err(format!("at {at} must be an integer value"));
        }
        let integer = integer_value(value, at)?;
        if let Some(earlier) = seen.insert(integer, index) {
            return Err(format!(
                "at {at} repeats the value at {}[{earlier}]",
                &at[..base]
            ));
        }
        at.truncate(base);
    }
    Ok(seen.into_keys().collect())
}

/// The values an `in` over strings lists at `at` (`in[1]`): two or more
/// strings, no two alike.
fn in_text_values(values: &Value, at: &mut String) -> Result<BTreeSet<String>, String> {
    let Some(values) = values.as_array().filter(|values| values.len() >= 2) else {
        return Err(format!("at {at} must list two or more string values"));
    };
    let base = at.len();
    // Each value with the index it first appears at, for the errors
    let mut seen = BTreeMap::new();
    for (index, value) in values.iter().enumerate() {
        // Writing to a `String` cannot fail
        let _ = write!(at, "[{index}]");
        let Some(text) = value.as_text() else {
            return Err(format!("at {at} must be a string, as the first value is"));
        };
        if let Some(earlier) = seen.insert(text, index) {
            return Err(format!(
                "at {at} repeats the value at {}[{earlier}]",
                &at[..base]
            ));
        }
        at.truncate(base);
    }
    Ok(seen.into_keys().map(str::to_string).collect())
}

/// Whether `value` is a string constant operand, `{ "const": ... }`.
fn is_const(value: &Value) -> bool {
    single_entry(value).is_some_and(|(key, _)| key == CONST)
}

/// The values an `in` over identifiers lists at `at` (`in[1]`): two or more
/// base58 identifiers, no two alike.
fn in_identifier_values(values: &Value, at: &mut String) -> Result<BTreeSet<Identifier>, String> {
    let Some(values) = values.as_array().filter(|values| values.len() >= 2) else {
        return Err(format!("at {at} must list two or more identifiers"));
    };
    let base = at.len();
    // Each value with the index it first appears at, for the errors
    let mut seen = BTreeMap::new();
    for (index, value) in values.iter().enumerate() {
        // Writing to a `String` cannot fail
        let _ = write!(at, "[{index}]");
        let identifier = identifier_constant(value, at)?;
        if let Some(earlier) = seen.insert(identifier, index) {
            return Err(format!(
                "at {at} repeats the value at {}[{earlier}]",
                &at[..base]
            ));
        }
        at.truncate(base);
    }
    Ok(seen.into_keys().collect())
}

/// The identifier a base58 string at `at` spells.
fn identifier_constant(value: &Value, at: &str) -> Result<Identifier, String> {
    let Some(text) = value.as_text() else {
        return Err(format!("at {at} must be an identifier, written base58"));
    };
    Identifier::from_string(text, Encoding::Base58).map_err(|_| {
        format!("at {at} holds \"{text}\", which is not a base58 identifier of 32 bytes")
    })
}

/// One side of a comparison of identifiers.
enum IdentifierSide {
    /// A `{ "const": base58 }`.
    Constant(Identifier),
    /// The dotted path of an identifier property.
    Property(String),
}

/// The side at `at` (`equal[1]`) of a comparison of identifiers: a `const`
/// identifier, written base58, or a bare path. What a path names is checked
/// against the parsed document type.
fn identifier_side(value: &Value, at: &str) -> Result<IdentifierSide, String> {
    if is_const(value) {
        let constant = single_entry(value).map_or(value, |(_, constant)| constant);
        return identifier_constant(constant, &format!("{at}.{CONST}"))
            .map(IdentifierSide::Constant);
    }
    if let Some(path) = named_path(value, at) {
        return path.map(IdentifierSide::Property);
    }
    if is_text_if_absent(value) {
        return Err(format!(
            "at {at} gives an identifier property a default, which identifiers do not take"
        ));
    }
    Err(format!(
        "at {at} must be the path of an identifier property or a const: identifiers are \
         compared with identifiers"
    ))
}

/// The comparison at `at` (`equal`) of `left` and `right`, a comparison of
/// identifiers, one side at least naming an identifier property: only `equal`
/// and `notEqual` compare them, and each side is a `const` identifier or an
/// identifier property ([`identifier_side`]).
fn identifier_comparison(
    comparison: ConstraintComparison,
    left: &Value,
    right: &Value,
    at: &str,
) -> Result<Option<PropertyConstraint>, String> {
    if !matches!(
        comparison,
        ConstraintComparison::Equal | ConstraintComparison::NotEqual
    ) {
        return Err(format!(
            "at {at} compares identifiers, which only equal and notEqual do"
        ));
    }
    let left = identifier_side(left, &format!("{at}[0]"))?;
    let right = identifier_side(right, &format!("{at}[1]"))?;
    Ok(match (left, right) {
        (IdentifierSide::Constant(_), IdentifierSide::Constant(_)) => None,
        (IdentifierSide::Property(path), IdentifierSide::Constant(value))
        | (IdentifierSide::Constant(value), IdentifierSide::Property(path)) => {
            Some(PropertyConstraint::IdentifierCompare {
                comparison,
                path,
                value,
            })
        }
        (IdentifierSide::Property(left), IdentifierSide::Property(right)) if left == right => {
            return Err(format!(
                "at {at} compares \"{left}\" with itself, so it would hold for every document or \
                 for none"
            ));
        }
        (IdentifierSide::Property(left), IdentifierSide::Property(right)) => {
            Some(PropertyConstraint::IdentifierCompareProperties {
                comparison,
                left,
                right,
            })
        }
    })
}

/// Whether `value` is `{ "ifAbsent": [path, string] }`: a string property with
/// the string it takes when the document leaves it out.
fn is_text_if_absent(value: &Value) -> bool {
    single_entry(value).is_some_and(|(key, operands)| {
        key == IF_ABSENT
            && matches!(
                operands.as_array().map(Vec::as_slice),
                Some([_, Value::Text(_)])
            )
    })
}

/// One side of a comparison of strings.
enum TextSide {
    /// A `{ "const": string }`.
    Constant(String),
    /// A string property: a bare path, or an `ifAbsent` with a string default.
    Property(TextProperty),
}

/// The side at `at` (`equal[1]`) of a comparison of strings: a `const`
/// string, a bare path, or an `ifAbsent` with a string default. What a path
/// names is checked against the parsed document type.
fn text_side(value: &Value, at: &str) -> Result<TextSide, String> {
    if is_const(value) {
        return single_entry(value)
            .and_then(|(_, constant)| constant.as_text())
            .map(|constant| TextSide::Constant(constant.to_string()))
            .ok_or_else(|| {
                format!("at {at}.{CONST} must be a string: an integer is written as itself")
            });
    }
    if let Some(path) = named_path(value, at) {
        return Ok(TextSide::Property(TextProperty {
            path: path?,
            if_absent: None,
        }));
    }
    if let Some((IF_ABSENT, operands)) = single_entry(value) {
        if let Some([path, Value::Text(default)]) = operands.as_array().map(Vec::as_slice) {
            let Some(path) = named_path(path, &format!("{at}.{IF_ABSENT}[0]")) else {
                return Err(format!(
                    "at {at}.{IF_ABSENT} must name a property path first"
                ));
            };
            return Ok(TextSide::Property(TextProperty {
                path: path?,
                if_absent: Some(default.clone()),
            }));
        }
    }
    Err(format!(
        "at {at} must be the path of a string property, an ifAbsent giving one a string \
         default, or a const: strings are compared with strings"
    ))
}

/// A side at `at` (`startsWith[1]`) of a `startsWith` or `endsWith`: a `const`
/// string or a string property, as [`text_side`] reads them.
fn text_operand(value: &Value, at: &str) -> Result<TextOperand, String> {
    Ok(match text_side(value, at)? {
        TextSide::Constant(value) => TextOperand::Constant(value),
        TextSide::Property(property) => TextOperand::Property(property),
    })
}

/// A side at `at` (`startsWith[1]`) of a `startsWith` or `endsWith` of byte
/// arrays: a `const` written in hex, an even number of hex digits (either
/// case), or a bare path. What a path names is checked against the parsed
/// document type.
fn bytes_operand(value: &Value, at: &str) -> Result<BytesOperand, String> {
    if is_const(value) {
        let constant = single_entry(value).map_or(value, |(_, constant)| constant);
        let Some(hex) = constant.as_text() else {
            return Err(format!(
                "at {at}.{CONST} must be a string of hex digits: a byte array constant is \
                 written in hex"
            ));
        };
        return hex::decode(hex).map(BytesOperand::Constant).map_err(|_| {
            format!(
                "at {at}.{CONST} holds \"{hex}\", which is not an even number of hex digits: \
                 a byte array constant is written in hex, two digits a byte"
            )
        });
    }
    if let Some(path) = named_path(value, at) {
        return path.map(BytesOperand::Property);
    }
    if single_entry(value).is_some_and(|(key, _)| key == IF_ABSENT) {
        return Err(format!(
            "at {at} gives a byte array property a default, which byte arrays do not take"
        ));
    }
    Err(format!(
        "at {at} must be the path of a byte array property or a {{ \"const\": hex }}: a byte \
         array starts or ends with a byte array"
    ))
}

/// The comparison at `at` (`equal`) of `left` and `right`, a comparison of
/// strings: only `equal` and `notEqual` compare them, and each side is a
/// `const` string or a string property ([`text_side`]). `None` when both are
/// constants, a comparison that reads no property.
fn text_comparison(
    comparison: ConstraintComparison,
    left: &Value,
    right: &Value,
    at: &str,
) -> Result<Option<PropertyConstraint>, String> {
    if !matches!(
        comparison,
        ConstraintComparison::Equal | ConstraintComparison::NotEqual
    ) {
        return Err(format!(
            "at {at} compares strings, which only equal and notEqual do"
        ));
    }
    let left = text_side(left, &format!("{at}[0]"))?;
    let right = text_side(right, &format!("{at}[1]"))?;
    Ok(match (left, right) {
        (TextSide::Constant(_), TextSide::Constant(_)) => None,
        (TextSide::Property(property), TextSide::Constant(value))
        | (TextSide::Constant(value), TextSide::Property(property)) => {
            Some(PropertyConstraint::TextCompare {
                comparison,
                property,
                value,
            })
        }
        (TextSide::Property(left), TextSide::Property(right)) if left == right => {
            return Err(format!(
                "at {at} compares \"{}\" with itself, so it would hold for every document or for \
                 none",
                left.path
            ));
        }
        (TextSide::Property(left), TextSide::Property(right)) => {
            Some(PropertyConstraint::TextCompareProperties {
                comparison,
                left,
                right,
            })
        }
    })
}

/// An operand at `at` (`lessThan[0].add[1]`), where the errors place it,
/// `depth` levels into its rule. `at` is extended for the operands of an
/// operator and trimmed back before a successful return.
fn parse_expression(
    value: &Value,
    at: &mut String,
    depth: usize,
    context: &ParseContext,
) -> Result<ConstraintExpression, String> {
    if depth > MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH {
        return Err(format!(
            "at {at} nests deeper than {MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH} levels"
        ));
    }
    if let Some(path) = value.as_text() {
        let path = bare_path(path, at)?;
        if let Some(read) = SystemRead::from_name(&path) {
            return Ok(ConstraintExpression::System(read));
        }
        return Ok(ConstraintExpression::Property { path, if_absent: 0 });
    }
    if is_number(value) {
        return integer_value(value, at).map(ConstraintExpression::Value);
    }
    let Some((key, operands)) = single_entry(value) else {
        return Err(format!(
            "at {at} must be an integer, a property path or an object with one key: \
             {OPERAND_KEYS}"
        ));
    };
    let parent = at.len();
    at.push('.');
    at.push_str(key);
    let expression = match key {
        IF_ABSENT => {
            let Some([path, if_absent]) = operands.as_array().map(Vec::as_slice) else {
                return Err(format!(
                    "at {at} must list a property path and the integer value it takes when \
                     the document leaves it out"
                ));
            };
            let path = match named_path(path, &format!("{at}[0]")) {
                Some(path) => path?,
                None => return Err(format!("at {at} must name a property path first")),
            };
            match SystemRead::from_name(&path) {
                Some(SystemRead { stored: false, .. }) => {
                    return Err(format!(
                        "at {at} gives {path} a default, but a system property a rule reads is \
                         always set: name it on its own"
                    ));
                }
                Some(SystemRead { stored: true, .. }) => {
                    return Err(format!(
                        "at {at} gives {path} a default, but a system value of the stored \
                         version is always set, and reads 0 on a create, which has none: name \
                         it on its own"
                    ));
                }
                None => {}
            }
            if if_absent.as_text().is_some() {
                return Err(format!(
                    "at {at} gives a string default, which only a comparison of strings \
                     takes, never an integer expression"
                ));
            }
            if !is_number(if_absent) {
                return Err(format!("at {at} must give an integer value second"));
            }
            at.push_str("[1]");
            ConstraintExpression::Property {
                path,
                if_absent: integer_value(if_absent, at)?,
            }
        }
        ADD => ConstraintExpression::Add(operand_list(operands, at, depth + 1, context)?),
        MIN => ConstraintExpression::Min(operand_list(operands, at, depth + 1, context)?),
        MAX => ConstraintExpression::Max(operand_list(operands, at, depth + 1, context)?),
        ABS => {
            if operands.as_array().is_some() {
                return Err(format!(
                    "at {at} must be one operand, not a list: abs takes a single operand"
                ));
            }
            ConstraintExpression::Abs(Box::new(parse_expression(
                operands,
                at,
                depth + 1,
                context,
            )?))
        }
        MULTIPLY => ConstraintExpression::Multiply(operand_list(operands, at, depth + 1, context)?),
        SUBTRACT => {
            let (left, right) = operand_pair(operands, at, depth + 1, context)?;
            ConstraintExpression::Subtract(Box::new(left), Box::new(right))
        }
        DIVIDE | MODULO => {
            let (dividend, divisor) = operand_pair(operands, at, depth + 1, context)?;
            if divisor == ConstraintExpression::Value(0) {
                return Err(format!("at {at} divides by 0"));
            }
            if key == DIVIDE {
                ConstraintExpression::Divide(Box::new(dividend), Box::new(divisor))
            } else {
                ConstraintExpression::Modulo(Box::new(dividend), Box::new(divisor))
            }
        }
        POWER => {
            let (base, exponent) = operand_pair(operands, at, depth + 1, context)?;
            if let ConstraintExpression::Value(exponent) = exponent {
                if exponent < 0 {
                    return Err(format!(
                        "at {at} raises to the negative power {exponent}, which has no \
                         integer result"
                    ));
                }
            }
            ConstraintExpression::Power(Box::new(base), Box::new(exponent))
        }
        // What the path names is checked against the parsed document type
        LENGTH | BYTE_LENGTH | COUNT => {
            let path = match named_path(operands, at) {
                Some(path) => path?,
                None => return Err(format!("at {at} must name a property path")),
            };
            let measure = match key {
                LENGTH => SizeMeasure::Length,
                BYTE_LENGTH => SizeMeasure::ByteLength,
                _ => SizeMeasure::Count,
            };
            ConstraintExpression::Size { measure, path }
        }
        // What the paths name is checked against the parsed document type
        COUNT_PRESENT => ConstraintExpression::CountPresent(count_present_paths(operands, at)?),
        // What the path names, and that the index is inside its maxItems, is
        // checked against the parsed document type
        BYTE_AT => {
            let Some([path, index]) = operands.as_array().map(Vec::as_slice) else {
                return Err(format!(
                    "at {at} must list a byte array property path and the index of the byte \
                     it reads, counted from 0"
                ));
            };
            let path = match named_path(path, &format!("{at}[0]")) {
                Some(path) => path?,
                None => return Err(format!("at {at} must name a property path first")),
            };
            if is_owner_path(&path) || SystemRead::from_name(&path).is_some() {
                return Err(format!(
                    "at {at} names {path}, but byteAt reads a byte array property"
                ));
            }
            at.push_str("[1]");
            if !is_number(index) {
                return Err(format!(
                    "at {at} must be the index of the byte, an integer from 0 to {}",
                    u16::MAX
                ));
            }
            let index = integer_value(index, at)?;
            let Ok(index) = u16::try_from(index) else {
                return Err(format!(
                    "at {at} holds {index}, but the index of a byte is an integer from 0 to {}, \
                     as a byte array's maxItems is",
                    u16::MAX
                ));
            };
            ConstraintExpression::ByteAt { path, index }
        }
        // Which document type it names, and what its keys name, is checked
        // once every document type of the contract is parsed
        COUNT_OF | SUM_OF => {
            ConstraintExpression::Aggregate(parse_aggregate(key == SUM_OF, operands, at, context)?)
        }
        // A value of the document a reference points at, read as a property is
        REFERENCED => {
            at.truncate(parent);
            ConstraintExpression::Property {
                path: referenced_operand(operands, at)?,
                if_absent: 0,
            }
        }
        CONST => {
            at.truncate(parent);
            return Err(format!(
                "at {at} is a string constant, which only equal and notEqual compare, with a \
                 string property, never inside an integer expression"
            ));
        }
        other => {
            at.truncate(parent);
            return Err(format!(
                "at {at} names \"{other}\", which is not one of {OPERAND_KEYS}"
            ));
        }
    };
    at.truncate(parent);
    Ok(expression)
}

/// The paths a `countPresent` lists at `at`: two or more property paths, no
/// two alike, in declared order. A repeat is refused on every parse, as a
/// value an `in` lists twice is: it would count one property twice.
fn count_present_paths(paths: &Value, at: &mut String) -> Result<Vec<String>, String> {
    let Some(paths) = paths.as_array().filter(|paths| paths.len() >= 2) else {
        return Err(format!("at {at} must list two or more property paths"));
    };
    let base = at.len();
    let mut parsed = Vec::with_capacity(paths.len());
    // Each path with the index it first appears at, for the errors
    let mut seen = BTreeMap::new();
    for (index, path) in paths.iter().enumerate() {
        // Writing to a `String` cannot fail
        let _ = write!(at, "[{index}]");
        let path = match named_path(path, at) {
            Some(path) => path?,
            None => return Err(format!("at {at} must name a property path")),
        };
        if let Some(earlier) = seen.insert(path.clone(), index) {
            return Err(format!(
                "at {at} repeats the path at {}[{earlier}]",
                &at[..base]
            ));
        }
        parsed.push(path);
        at.truncate(base);
    }
    Ok(parsed)
}

/// A `countOf` (`sum` false) or `sumOf` at `at`, listing a document type,
/// for a `sumOf` the integer property to total, and optionally the filter its
/// documents must match: an object of one or more keys, each a property path
/// of that type or `$ownerId`, and the value it must take, a property path of
/// the document being written, `$ownerId`, `$id`, an integer or a
/// `{ "const": ... }`.
fn parse_aggregate(
    sum: bool,
    operands: &Value,
    at: &mut String,
    context: &ParseContext,
) -> Result<AggregateRead, String> {
    let parts = operands.as_array().map(Vec::as_slice).unwrap_or_default();
    let (document_type, property, filter) = match (sum, parts) {
        (false, [document_type]) => (document_type, None, None),
        (false, [document_type, filter]) => (document_type, None, Some(filter)),
        (true, [document_type, property]) => (document_type, Some(property), None),
        (true, [document_type, property, filter]) => (document_type, Some(property), Some(filter)),
        (false, _) => {
            return Err(format!(
                "at {at} must list the document type to count, then optionally the values \
                 its documents must match: [type] or [type, {{ key: value, ... }}]"
            ))
        }
        (true, _) => {
            return Err(format!(
                "at {at} must list the document type, the integer property of it to total, \
                 then optionally the values its documents must match: [type, property] or \
                 [type, property, {{ key: value, ... }}]"
            ))
        }
    };
    let Some(document_type) = document_type.as_text().filter(|name| !name.is_empty()) else {
        return Err(format!("at {at} must name a document type first"));
    };
    let kind = match property {
        None => AggregateKind::Count,
        Some(property) => {
            let Some(property) = property.as_text().filter(|path| !path.is_empty()) else {
                return Err(format!("at {at} must name the property to total second"));
            };
            AggregateKind::Sum {
                property: property.to_string(),
            }
        }
    };
    let mut bindings = BTreeMap::new();
    if let Some(filter) = filter {
        let base = at.len();
        // Writing to a `String` cannot fail
        let _ = write!(at, "[{}]", if sum { 2 } else { 1 });
        let entries = match filter {
            Value::Map(entries) if !entries.is_empty() => entries,
            _ => {
                return Err(format!(
                    "at {at} must match its documents by one or more keys: {{ key: value, ... }}"
                ))
            }
        };
        for (key, binding) in entries {
            let Some(key) = key.as_text().filter(|key| !key.is_empty()) else {
                return Err(format!(
                    "at {at} holds the key {}, but a key is a property path of the type or \
                     $ownerId",
                    key.non_qualified_string_representation()
                ));
            };
            if key.starts_with('$') && key != OWNER_ID {
                return Err(format!(
                    "at {at} matches by {key}, but the one system value a key names is $ownerId"
                ));
            }
            let binding = match binding {
                Value::Text(path) if path == OWNER_ID => AggregateBinding::Owner,
                Value::Text(path) if path == DOCUMENT_ID => AggregateBinding::Id,
                Value::Text(path) if stored_version_path(path).is_some() => {
                    return Err(format!(
                        "at {at}.{key} takes {path}, but a total matches its documents by the \
                         document being written, never the stored version it replaces"
                    ))
                }
                Value::Text(path) if path.starts_with('$') => {
                    return Err(format!(
                        "at {at}.{key} takes {path}, but the system values a key takes are \
                         $ownerId and $id"
                    ))
                }
                // A byte array matches no key: it reads as an integer, which the
                // parsed document type refuses
                Value::Text(path) => AggregateBinding::Property {
                    kind: (context.property_kind)(path).filter(|kind| *kind != EqualityKind::Bytes),
                    path: path.clone(),
                },
                binding if is_number(binding) => {
                    AggregateBinding::Integer(integer_value(binding, &format!("{at}.{key}"))?)
                }
                binding => match single_entry(binding) {
                    Some((CONST, Value::Text(constant))) => {
                        AggregateBinding::Constant(constant.clone())
                    }
                    _ => {
                        return Err(format!(
                            "at {at}.{key} must be a property path of the document, $ownerId, \
                             $id, an integer or a {{ \"const\": ... }} string or base58 \
                             identifier"
                        ))
                    }
                },
            };
            if bindings.insert(key.to_string(), binding).is_some() {
                return Err(format!("at {at} matches by {key} twice"));
            }
        }
        at.truncate(base);
    }
    Ok(AggregateRead {
        kind,
        of_own_type: document_type == context.document_type_name,
        document_type: document_type.to_string(),
        filter: bindings,
    })
}

/// The exactly two operands listed at `at`, `depth` levels into their rule.
fn operand_pair(
    operands: &Value,
    at: &mut String,
    depth: usize,
    context: &ParseContext,
) -> Result<(ConstraintExpression, ConstraintExpression), String> {
    let Some([left, right]) = operands.as_array().map(Vec::as_slice) else {
        return Err(format!("at {at} must list exactly two operands"));
    };
    let base = at.len();
    at.push_str("[0]");
    let left = parse_expression(left, at, depth, context)?;
    at.truncate(base);
    at.push_str("[1]");
    let right = parse_expression(right, at, depth, context)?;
    at.truncate(base);
    Ok((left, right))
}

/// The two or more operands listed at `at`, `depth` levels into their rule.
fn operand_list(
    operands: &Value,
    at: &mut String,
    depth: usize,
    context: &ParseContext,
) -> Result<Vec<ConstraintExpression>, String> {
    let Some(values) = operands.as_array().filter(|values| values.len() >= 2) else {
        return Err(format!("at {at} must list two or more operands"));
    };
    let base = at.len();
    let mut expressions = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        // Writing to a `String` cannot fail
        let _ = write!(at, "[{index}]");
        expressions.push(parse_expression(value, at, depth, context)?);
        at.truncate(base);
    }
    Ok(expressions)
}

/// Whether `value` is a number: an integer, or a float, which the meta-schema's
/// `integer` type admits when it has no fractional part (JSON does not tell
/// `100` from `100.0`).
fn is_number(value: &Value) -> bool {
    value.is_integer() || matches!(value, Value::Float(_))
}

/// The integer a declaration's number spells: an integer, or a float with no
/// fractional part inside the range of a 128-bit signed integer, the cast then
/// exact. `None` for any other value.
pub(crate) fn whole_integer(value: &Value) -> Option<i128> {
    match value {
        Value::Float(float)
            if float.fract() == 0.0 && *float >= i128::MIN as f64 && *float < i128::MAX as f64 =>
        {
            Some(*float as i128)
        }
        Value::Float(_) => None,
        _ => value.to_integer::<i128>().ok(),
    }
}

/// An integer literal at `at`: an integer, or a float with no fractional part,
/// in the range of an `i128`.
fn integer_value(value: &Value, at: &str) -> Result<i128, String> {
    whole_integer(value).ok_or_else(|| {
        format!(
            "at {at} holds {}, which is not an integer in the range of a 128-bit signed \
             integer",
            value.non_qualified_string_representation()
        )
    })
}

/// Whether `path` names an owner rather than a property: `$ownerId`, or the
/// stored version's, `$old.$ownerId`.
fn is_owner_path(path: &str) -> bool {
    either_version_path(path) == OWNER_ID
}

/// The identifier `data` holds at `path`, in any of the forms a document's
/// identifier takes, or the owner `system` gives for `$ownerId`, and the
/// stored version's for `$old.$ownerId`; `None` when the document leaves the
/// property out or holds something that is no identifier there, which the
/// schema validation running first refuses for an identifier property.
fn identifier_value(data: &Value, system: &DocumentSystemValues, path: &str) -> Option<Identifier> {
    if path == OWNER_ID {
        return system.owner_id;
    }
    if path == STORED_OWNER_ID {
        return system.stored_owner_id();
    }
    match data.get_optional_value_at_path(path) {
        Ok(Some(value)) => value.to_identifier().ok(),
        _ => None,
    }
}

/// Whether `data` holds the property at `path`: absent where
/// [`property_value`] would take the `if_absent` value, and where it holds an
/// object a stored document does not keep ([`is_kept_in_storage`]).
fn is_present(data: &Value, path: &str) -> bool {
    matches!(
        data.get_optional_value_at_path(path),
        Ok(Some(value)) if is_kept_in_storage(value)
    )
}

/// Whether a stored document keeps `value`: anything but null, and an object
/// only when it holds a member it keeps. A document's encoding reads an object
/// with no member back as no object at all, so `{}`, and `{ "inner": {} }`
/// around it, are absent from the stored document. A create or a replace
/// judges the data it carries, and a transfer, a purchase or a price update
/// the stored document, so all of them must see such an object as absent.
/// Null and every value but an object answer at once; only an object's members
/// are walked. A create's data is walked before its schema validation is
/// reported, so the walk is iterative, like the other walks over a document's
/// values, and takes no stack however deep the object nests.
fn is_kept_in_storage(value: &Value) -> bool {
    let Value::Map(members) = value else {
        return !value.is_null();
    };
    let mut pending: Vec<&Value> = members.iter().map(|(_, member)| member).collect();
    while let Some(value) = pending.pop() {
        match value {
            Value::Null => {}
            Value::Map(members) => pending.extend(members.iter().map(|(_, member)| member)),
            _ => return true,
        }
    }
    false
}

/// The value of the property at `path` in `data`, 1 or 0 for a boolean, or
/// `if_absent` when the document leaves it out. An intermediate that is not an object reads as
/// absent: the schema validation that runs first refuses such a document.
fn property_value(
    data: &Value,
    path: &str,
    if_absent: i128,
) -> Result<i128, PropertyConstraintViolation> {
    match data.get_optional_value_at_path(path) {
        Ok(Some(Value::Null)) | Ok(None) | Err(_) => Ok(if_absent),
        Ok(Some(Value::Bool(flag))) => Ok(i128::from(*flag)),
        Ok(Some(value)) if value.is_integer() => value
            .to_integer::<i128>()
            .map_err(|_| PropertyConstraintViolation::Overflow),
        Ok(Some(_)) => Err(PropertyConstraintViolation::NotAnInteger),
    }
}

/// The size of the property at `path` in `data`, as `measure` counts it: 0
/// when the document leaves it out or sets it to null, and for a value of
/// another type than `measure` reads, which the schema validation reported
/// before the rules refuses. A byte array counts its bytes, whichever form the
/// document gives them in.
fn property_size(data: &Value, path: &str, measure: SizeMeasure) -> usize {
    let Ok(Some(value)) = data.get_optional_value_at_path(path) else {
        return 0;
    };
    match (measure, value) {
        (SizeMeasure::Length, Value::Text(text)) => text.chars().count(),
        (SizeMeasure::ByteLength, Value::Text(text)) => text.len(),
        (SizeMeasure::Count, Value::Array(items)) => items.len(),
        (SizeMeasure::Count, Value::Bytes(bytes)) => bytes.len(),
        (SizeMeasure::Count, Value::Bytes20(_)) => 20,
        (SizeMeasure::Count, Value::Bytes32(_) | Value::Identifier(_)) => 32,
        (SizeMeasure::Count, Value::Bytes36(_)) => 36,
        _ => 0,
    }
}

/// The bytes of the byte array property at `path` in `data`, in whichever form
/// the document gives them: `None` when it leaves the property out or sets it
/// to null, and for a value that is no byte array, which the schema validation
/// running first refuses. An array of integers, each 0 to 255, is the bytes it
/// lists, as `count` reads it, so that a create carrying one is judged as the
/// stored document, which holds the bytes, is.
fn byte_array_value<'a>(data: &'a Value, path: &'a str) -> Option<Cow<'a, [u8]>> {
    let Ok(Some(value)) = data.get_optional_value_at_path(path) else {
        return None;
    };
    match value {
        Value::Array(items) => items
            .iter()
            .map(byte_item)
            .collect::<Option<Vec<u8>>>()
            .map(Cow::Owned),
        value => byte_slice(value).map(Cow::Borrowed),
    }
}

/// The byte at `index` of the byte array property at `path` in `data`, as
/// [`byte_array_value`] reads the array, without collecting an array of
/// integers into bytes: `None` when the array does not hold that byte, and
/// when [`byte_array_value`] reads no array. Every element of an array of
/// integers is checked, not only the one at `index`, so that an array holding
/// anything but bytes reads no byte whichever one is asked for.
fn byte_at(data: &Value, path: &str, index: usize) -> Option<u8> {
    let Ok(Some(value)) = data.get_optional_value_at_path(path) else {
        return None;
    };
    match value {
        Value::Array(items) => {
            let mut found = None;
            for (position, item) in items.iter().enumerate() {
                let byte = byte_item(item)?;
                if position == index {
                    found = Some(byte);
                }
            }
            found
        }
        value => byte_slice(value)?.get(index).copied(),
    }
}

/// The bytes a value holding them directly holds: `Value::Bytes` and the
/// fixed-size forms; `None` for any other value.
fn byte_slice(value: &Value) -> Option<&[u8]> {
    match value {
        Value::Bytes(bytes) => Some(bytes),
        Value::Bytes20(bytes) => Some(bytes),
        Value::Bytes32(bytes) | Value::Identifier(bytes) => Some(bytes),
        Value::Bytes36(bytes) => Some(bytes),
        _ => None,
    }
}

/// The byte an element of an array of integers is: `None` for an element
/// that is not an integer from 0 to 255.
fn byte_item(item: &Value) -> Option<u8> {
    if item.is_integer() {
        item.to_integer::<u8>().ok()
    } else {
        None
    }
}

/// `base` to the power `exponent`, exactly. An exponent too large for
/// `checked_pow` leaves only the bases 0, 1 and -1 in range.
fn power(base: i128, exponent: i128) -> Result<i128, PropertyConstraintViolation> {
    if exponent < 0 {
        return Err(PropertyConstraintViolation::NegativeExponent);
    }
    match u32::try_from(exponent) {
        Ok(exponent) => base
            .checked_pow(exponent)
            .ok_or(PropertyConstraintViolation::Overflow),
        Err(_) => match base {
            0 | 1 => Ok(base),
            -1 => Ok(if exponent % 2 == 0 { 1 } else { -1 }),
            _ => Err(PropertyConstraintViolation::Overflow),
        },
    }
}
