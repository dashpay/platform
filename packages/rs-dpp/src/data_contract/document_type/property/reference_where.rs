//! The `where` entries of a document reference that are not an equality with a
//! value of the referring document (meta-schema v3, protocol version 14): a
//! literal the referenced value must equal, a value it must differ from, a list
//! of literals it must be one of, or a bound it must compare with.
//!
//! ```json
//! "where": {
//!   "$ownerId": "pollOwnerId",
//!   "multiChoice": { "const": true },
//!   "status": { "in": ["open", "extended"] },
//!   "endsAt": { "greaterThan": "$createdAt" },
//!   "reservePrice": { "lessThanOrEqual": "amount" },
//!   "$creatorId": { "notEqual": "$ownerId" },
//!   "voters": { "contains": "$ownerId" },
//!   "tags": { "containsAll": "tags" }
//! }
//! ```
//!
//! Each entry is keyed by the referenced side, as an equality entry is: a
//! property of the referenced document type, or its `$ownerId`, `$creatorId`
//! or `$id`. An entry whose value names a property of the referring document
//! type, or `"$ownerId"`, is an equality and stays in the reference's
//! `property_agreement`; every other entry is a [`WhereCondition`]. Whether a
//! literal reads as a string, an identifier or a byte array is decided by the
//! referenced property it is compared with ([`WhereLiteralKind`]), which the
//! check reads beside the referenced document, since the type may be another
//! contract's. What the names resolve to is checked at registration.
//!
//! A typed array is compared as a list: an equality between two typed arrays
//! holds for the same elements in the same order ([`where_lists_equal`]),
//! `contains` for a list holding a value of the referring document or the
//! writer, and `containsAll` for a list holding every element of a typed array
//! of the referring document. A list the document leaves out is the empty one.

use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::property::{
    is_referenced_system_agreement_property, DocumentPropertyType,
};
use crate::data_contract::document_type::property_constraints::{
    spells_bytes, spells_identifier, whole_integer, SystemProperty,
};
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::property_names::OWNER_ID;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::string_encoding::Encoding;
use platform_value::{Identifier, Value};
use serde::Serialize;
use std::borrow::Cow;
use std::collections::BTreeSet;
use std::fmt;

/// The key of a `where` entry's literal: `{ "const": ... }`.
pub const WHERE_CONST: &str = "const";
/// The key of a `where` entry the referenced side must differ from.
pub const WHERE_NOT_EQUAL: &str = "notEqual";
/// The key of a `where` entry listing the literals the referenced side may be.
pub const WHERE_IN: &str = "in";
/// The key of a `where` entry naming a value the referenced typed array must
/// hold.
pub const WHERE_CONTAINS: &str = "contains";
/// The key of a `where` entry naming a typed array whose every element the
/// referenced typed array must hold.
pub const WHERE_CONTAINS_ALL: &str = "containsAll";

/// A literal of a `where` entry, `{ "const": ... }` or a value an `in` lists,
/// as the declaration writes it. A string is a string, a base58 identifier or a
/// byte array in hex, as the referenced property it is compared with decides
/// ([`Self::value_for`]).
// Embedded in `DocumentPropertyReferenceTarget`, which consensus errors carry.
// @append_only
#[derive(
    Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Encode, Decode, DecodeUntrusted,
)]
#[serde(untagged)]
pub enum WhereLiteral {
    /// An integer, or a number with no fractional part.
    Integer(i128),
    /// `true` or `false`.
    Boolean(bool),
    /// A string.
    Text(String),
}

/// What kind of value the referenced side of a `where` entry holds, which
/// decides how a literal compared with it reads.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum WhereLiteralKind {
    /// An integer property: an integer literal.
    Integer,
    /// A boolean property: `true` or `false`.
    Boolean,
    /// A string property: a string literal.
    Text,
    /// An identifier property, or `$ownerId`, `$creatorId` or `$id`: a
    /// string literal, read as a base58 identifier of 32 bytes.
    Identifier,
    /// A byte array property: a string literal, read as hex, two digits a
    /// byte.
    Bytes,
}

impl WhereLiteralKind {
    /// The kind of value a referenced property of `property_type` holds, `None`
    /// for one no literal is compared with: a float, an object or an array of
    /// values.
    pub fn of(property_type: &DocumentPropertyType) -> Option<Self> {
        match property_type {
            DocumentPropertyType::Boolean => Some(WhereLiteralKind::Boolean),
            DocumentPropertyType::String(_) => Some(WhereLiteralKind::Text),
            DocumentPropertyType::ByteArray(_) => Some(WhereLiteralKind::Bytes),
            property_type if property_type.is_identifier() => Some(WhereLiteralKind::Identifier),
            property_type
                if property_type.is_integer()
                    || matches!(
                        property_type,
                        DocumentPropertyType::U128 | DocumentPropertyType::I128
                    ) =>
            {
                Some(WhereLiteralKind::Integer)
            }
            _ => None,
        }
    }

    /// The kind of value the referenced side `referenced_property` of a
    /// `where` entry holds on a document of `referenced`: its `$ownerId`,
    /// `$creatorId` and `$id` are identifiers, and a schema property is of its
    /// type ([`Self::of`]). `None` for a property the type does not have or
    /// that no literal is compared with.
    pub fn of_referenced(referenced: DocumentTypeRef, referenced_property: &str) -> Option<Self> {
        if is_referenced_system_agreement_property(referenced_property) {
            return Some(WhereLiteralKind::Identifier);
        }
        referenced
            .flattened_properties()
            .get(referenced_property)
            .and_then(|property| Self::of(&property.property_type))
    }

    /// What the declaration writes a literal of this kind as, for the errors.
    pub fn describe(self) -> &'static str {
        match self {
            WhereLiteralKind::Integer => "an integer",
            WhereLiteralKind::Boolean => "true or false",
            WhereLiteralKind::Text => "a string",
            WhereLiteralKind::Identifier => "a base58 identifier",
            WhereLiteralKind::Bytes => "a string of hex digits, two a byte",
        }
    }
}

impl WhereLiteral {
    /// The literal a declaration's JSON value spells: an integer (a number
    /// with no fractional part included), `true` or `false`, or a string.
    /// `None` for anything else.
    pub fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Bool(flag) => Some(WhereLiteral::Boolean(*flag)),
            Value::Text(text) => Some(WhereLiteral::Text(text.clone())),
            Value::Float(_) => whole_integer(value).map(WhereLiteral::Integer),
            value if value.is_integer() => whole_integer(value).map(WhereLiteral::Integer),
            _ => None,
        }
    }

    /// The value a referenced property of `kind` holds when it equals this
    /// literal, compared with it by `Value::same_scalar_data`; `None` when the
    /// literal can not be one: of another kind, an identifier that is not
    /// base58 of 32 bytes, or bytes that are not an even number of hex digits.
    /// Registration refuses such a literal, so a write meets none.
    pub fn value_for(&self, kind: WhereLiteralKind) -> Option<Value> {
        match (self, kind) {
            (WhereLiteral::Integer(integer), WhereLiteralKind::Integer) => {
                Some(Value::I128(*integer))
            }
            (WhereLiteral::Boolean(flag), WhereLiteralKind::Boolean) => Some(Value::Bool(*flag)),
            (WhereLiteral::Text(text), WhereLiteralKind::Text) => Some(Value::Text(text.clone())),
            (WhereLiteral::Text(text), WhereLiteralKind::Identifier) => {
                Identifier::from_string(text, Encoding::Base58)
                    .ok()
                    .map(|identifier| Value::Identifier(identifier.to_buffer()))
            }
            (WhereLiteral::Text(text), WhereLiteralKind::Bytes) => {
                hex::decode(text).ok().map(Value::Bytes)
            }
            _ => None,
        }
    }

    /// Whether `value`, a referenced property of `kind`, equals this literal
    /// as `Value::same_scalar_data` compares it with [`Self::value_for`]'s
    /// value, a string compared in place rather than copied.
    pub fn equals(&self, value: &Value, kind: WhereLiteralKind) -> bool {
        match (self, kind) {
            (WhereLiteral::Text(text), WhereLiteralKind::Text) => {
                matches!(value, Value::Text(held) if held == text)
            }
            // Decoded in place, beside the held bytes
            (WhereLiteral::Text(text), WhereLiteralKind::Identifier) => value
                .as_bytes_slice()
                .ok()
                .and_then(|held| <&[u8; 32]>::try_from(held).ok())
                .is_some_and(|held| spells_identifier(text, held)),
            (WhereLiteral::Text(text), WhereLiteralKind::Bytes) => value
                .as_bytes_slice()
                .is_ok_and(|held| spells_bytes(text, held)),
            _ => self
                .value_for(kind)
                .is_some_and(|literal| value.same_scalar_data(&literal)),
        }
    }
}

impl fmt::Display for WhereLiteral {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WhereLiteral::Integer(integer) => write!(f, "{integer}"),
            WhereLiteral::Boolean(flag) => write!(f, "{flag}"),
            // As a JSON string, quotes and backslashes escaped
            WhereLiteral::Text(text) => match serde_json::to_string(text) {
                Ok(quoted) => f.write_str(&quoted),
                Err(_) => Err(fmt::Error),
            },
        }
    }
}

/// What the referenced side of a `notEqual` must differ from.
// Embedded in `DocumentPropertyReferenceTarget`, which consensus errors carry.
// @append_only
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Encode, Decode, DecodeUntrusted)]
#[serde(rename_all = "camelCase")]
pub enum WhereNotEqualBound {
    /// A property of the referring document, by its dotted path.
    Property(String),
    /// `"$ownerId"`: the writer.
    Writer,
    /// `{ "const": ... }`: a literal.
    Constant(WhereLiteral),
}

impl fmt::Display for WhereNotEqualBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WhereNotEqualBound::Property(path) => write!(f, "\"{path}\""),
            WhereNotEqualBound::Writer => write!(f, "\"{OWNER_ID}\""),
            WhereNotEqualBound::Constant(literal) => write!(f, "{{ \"const\": {literal} }}"),
        }
    }
}

/// What the referenced side of an ordering, an integer, is compared with.
// Embedded in `DocumentPropertyReferenceTarget`, which consensus errors carry.
// @append_only
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Encode, Decode, DecodeUntrusted)]
#[serde(rename_all = "camelCase")]
pub enum WhereOrderingBound {
    /// An integer property of the referring document, by its dotted path.
    Property(String),
    /// `{ "const": ... }`: an integer.
    Constant(i128),
    /// A time or height the referring document records (`"$createdAt"`),
    /// which its type lists in `required`.
    System(SystemProperty),
}

impl fmt::Display for WhereOrderingBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WhereOrderingBound::Property(path) => write!(f, "\"{path}\""),
            WhereOrderingBound::Constant(integer) => write!(f, "{{ \"const\": {integer} }}"),
            WhereOrderingBound::System(property) => write!(f, "\"{}\"", property.name()),
        }
    }
}

/// What the referenced typed array of a `contains` must hold an element equal
/// to.
// Embedded in `DocumentPropertyReferenceTarget`, which consensus errors carry.
// @append_only
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Encode, Decode, DecodeUntrusted)]
#[serde(rename_all = "camelCase")]
pub enum WhereContainsBound {
    /// A property of the referring document holding one value of the element
    /// kind, by its dotted path.
    Property(String),
    /// `"$ownerId"`: the writer, among identifier elements.
    Writer,
}

impl fmt::Display for WhereContainsBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WhereContainsBound::Property(path) => write!(f, "\"{path}\""),
            WhereContainsBound::Writer => write!(f, "\"{OWNER_ID}\""),
        }
    }
}

/// A value of the referring write a `notEqual`, an ordering, a `contains` or a
/// `containsAll` compares the referenced side with
/// ([`WhereCondition::bound_read`]).
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum WhereBoundRead<'a> {
    /// The referring document's property at this dotted path.
    Property(&'a str),
    /// The writer, an identifier.
    Writer,
    /// A time or height the referring document records, an integer.
    System(SystemProperty),
}

impl WhereBoundRead<'_> {
    /// Whether the value may differ between two writes of the referring
    /// document that change none of its properties: the writer, who may be a
    /// new owner after a transfer or a purchase, and the time and heights of
    /// the last update or transfer. An entry reading one is checked again on
    /// every replace, as a writer gate is.
    pub fn changes_without_a_write(self) -> bool {
        match self {
            WhereBoundRead::Writer => true,
            WhereBoundRead::System(property) => !matches!(
                property,
                SystemProperty::CreatedAt
                    | SystemProperty::CreatedAtBlockHeight
                    | SystemProperty::CreatedAtCoreBlockHeight
            ),
            WhereBoundRead::Property(_) => false,
        }
    }
}

/// How the referenced side of an ordering compares with its bound.
// Embedded in `DocumentPropertyReferenceTarget`, which consensus errors carry.
// @append_only
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Encode, Decode, DecodeUntrusted)]
#[serde(rename_all = "camelCase")]
pub enum WhereOrdering {
    /// `greaterThan`
    GreaterThan,
    /// `greaterThanOrEqual`
    GreaterThanOrEqual,
    /// `lessThan`
    LessThan,
    /// `lessThanOrEqual`
    LessThanOrEqual,
}

impl WhereOrdering {
    /// Every ordering.
    pub const ALL: [WhereOrdering; 4] = [
        WhereOrdering::GreaterThan,
        WhereOrdering::GreaterThanOrEqual,
        WhereOrdering::LessThan,
        WhereOrdering::LessThanOrEqual,
    ];

    /// The key declaring it.
    pub fn wire_name(self) -> &'static str {
        match self {
            WhereOrdering::GreaterThan => "greaterThan",
            WhereOrdering::GreaterThanOrEqual => "greaterThanOrEqual",
            WhereOrdering::LessThan => "lessThan",
            WhereOrdering::LessThanOrEqual => "lessThanOrEqual",
        }
    }

    /// Whether `referenced` compares with `bound` this way.
    pub fn holds(self, referenced: i128, bound: i128) -> bool {
        match self {
            WhereOrdering::GreaterThan => referenced > bound,
            WhereOrdering::GreaterThanOrEqual => referenced >= bound,
            WhereOrdering::LessThan => referenced < bound,
            WhereOrdering::LessThanOrEqual => referenced <= bound,
        }
    }
}

/// A `where` entry that is not an equality with a value of the referring
/// document: what the referenced value, the entry's key, must be.
// Embedded in `DocumentPropertyReferenceTarget`, which consensus errors carry.
// @append_only
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Encode, Decode, DecodeUntrusted)]
#[serde(rename_all = "camelCase")]
pub enum WhereCondition {
    /// `{ "const": literal }`: equal to the literal. A referenced side the
    /// document leaves out equals no literal.
    Constant(WhereLiteral),
    /// `{ "notEqual": bound }`: not equal to the bound, a property of the
    /// referring document, the writer or a literal; the negation of an
    /// equality, so two absent sides are equal and one absent side differs.
    NotEqual(WhereNotEqualBound),
    /// `{ "in": [literal, ...] }`: equal to one of two or more distinct
    /// literals, in the order the declaration lists them. A referenced side
    /// the document leaves out is in none.
    In(Vec<WhereLiteral>),
    /// `{ "greaterThan": bound }` and the other orderings: an integer that
    /// compares with the bound, an integer property of the referring document,
    /// an integer or a time or height of the referring document, this way.
    /// Holds only when both sides have a value.
    Ordering {
        ordering: WhereOrdering,
        bound: WhereOrderingBound,
    },
    /// `{ "contains": bound }`: a typed array holding an element equal to the
    /// bound, a property of the referring document of the element kind or the
    /// writer, as [`where_lists_equal`] compares two elements. A list the
    /// document leaves out holds nothing, and a bound the write leaves out is
    /// held by no list.
    Contains(WhereContainsBound),
    /// `{ "containsAll": path }`: a typed array holding every element of the
    /// typed array of the referring document at `path`, each as
    /// [`where_lists_equal`] compares two elements, in any order. Every list
    /// holds all of an empty one, and a list the write leaves out is empty.
    ContainsAll(String),
}

impl WhereCondition {
    /// The key declaring the condition.
    pub fn wire_name(&self) -> &'static str {
        match self {
            WhereCondition::Constant(_) => WHERE_CONST,
            WhereCondition::NotEqual(_) => WHERE_NOT_EQUAL,
            WhereCondition::In(_) => WHERE_IN,
            WhereCondition::Ordering { ordering, .. } => ordering.wire_name(),
            WhereCondition::Contains(_) => WHERE_CONTAINS,
            WhereCondition::ContainsAll(_) => WHERE_CONTAINS_ALL,
        }
    }

    /// Whether the condition compares the referenced side as a list, a typed
    /// array (`contains` and `containsAll`), rather than as one value.
    pub fn reads_a_list(&self) -> bool {
        matches!(
            self,
            WhereCondition::Contains(_) | WhereCondition::ContainsAll(_)
        )
    }

    /// The value of the referring write the condition compares with, `None`
    /// for one that carries its own: a literal or a list of them.
    pub fn bound_read(&self) -> Option<WhereBoundRead<'_>> {
        match self {
            WhereCondition::NotEqual(WhereNotEqualBound::Property(path))
            | WhereCondition::Ordering {
                bound: WhereOrderingBound::Property(path),
                ..
            }
            | WhereCondition::Contains(WhereContainsBound::Property(path))
            | WhereCondition::ContainsAll(path) => Some(WhereBoundRead::Property(path)),
            WhereCondition::NotEqual(WhereNotEqualBound::Writer)
            | WhereCondition::Contains(WhereContainsBound::Writer) => Some(WhereBoundRead::Writer),
            WhereCondition::Ordering {
                bound: WhereOrderingBound::System(property),
                ..
            } => Some(WhereBoundRead::System(*property)),
            WhereCondition::Constant(_)
            | WhereCondition::In(_)
            | WhereCondition::NotEqual(WhereNotEqualBound::Constant(_))
            | WhereCondition::Ordering {
                bound: WhereOrderingBound::Constant(_),
                ..
            } => None,
        }
    }

    /// The property of the referring document the condition reads, `None`
    /// when it reads none.
    pub fn referring_property(&self) -> Option<&str> {
        match self.bound_read() {
            Some(WhereBoundRead::Property(path)) => Some(path),
            _ => None,
        }
    }

    /// Whether the condition is checked again on every replace of the
    /// referring document, as a writer gate is: it reads the writer or the
    /// time or heights of the last update or transfer
    /// ([`WhereBoundRead::changes_without_a_write`]).
    pub fn is_rechecked_on_every_replace(&self) -> bool {
        self.bound_read()
            .is_some_and(WhereBoundRead::changes_without_a_write)
    }

    /// How many list elements [`Self::holds`] reads for `referenced` and
    /// `bound`: the referenced list's for a `contains`, and the bound list's
    /// too for a `containsAll`; none for a condition comparing one value. What
    /// document reference validation bills before checking the condition.
    pub fn list_elements_read(&self, referenced: Option<&Value>, bound: Option<&Value>) -> usize {
        match self {
            WhereCondition::Contains(_) => where_list_len(referenced),
            WhereCondition::ContainsAll(_) => {
                where_list_len(referenced).saturating_add(where_list_len(bound))
            }
            _ => 0,
        }
    }

    /// Whether the condition holds for `referenced`, the referenced side's
    /// value (`None` when the document leaves it out) of `kind` (`None` for a
    /// property no literal reads as, which registration refuses a literal
    /// for, a typed array among them), and `bound`, the value
    /// [`Self::bound_read`] names (`None` when the write leaves it out, and for
    /// a literal or a list, which carry their own): a time or height reads as
    /// an integer, the writer as an identifier. A `contains` or a `containsAll`
    /// reads `referenced` as a list and needs no `kind`.
    pub fn holds(
        &self,
        referenced: Option<&Value>,
        kind: Option<WhereLiteralKind>,
        bound: Option<&Value>,
    ) -> bool {
        // An absent value equals no literal, which is then not read at all
        let equals_literal = |literal: &WhereLiteral| {
            matches!(
                (referenced, kind),
                (Some(referenced), Some(kind)) if literal.equals(referenced, kind)
            )
        };
        match self {
            WhereCondition::Constant(literal) => equals_literal(literal),
            WhereCondition::In(literals) => literals.iter().any(equals_literal),
            WhereCondition::NotEqual(WhereNotEqualBound::Constant(literal)) => {
                !equals_literal(literal)
            }
            // The negation of an equality entry: both absent agree
            WhereCondition::NotEqual(
                WhereNotEqualBound::Property(_) | WhereNotEqualBound::Writer,
            ) => !match (referenced, bound) {
                (Some(referenced), Some(bound)) => referenced.same_scalar_data(bound),
                (None, None) => true,
                (Some(_), None) | (None, Some(_)) => false,
            },
            WhereCondition::Ordering {
                ordering,
                bound: declared,
            } => {
                let integer = |value: Option<&Value>| {
                    value
                        .filter(|value| value.is_integer())
                        .and_then(|value| value.to_integer::<i128>().ok())
                };
                let bound = match declared {
                    WhereOrderingBound::Constant(integer) => Some(*integer),
                    WhereOrderingBound::Property(_) | WhereOrderingBound::System(_) => {
                        integer(bound)
                    }
                };
                matches!(
                    (integer(referenced), bound),
                    (Some(referenced), Some(bound)) if ordering.holds(referenced, bound)
                )
            }
            // An absent bound is no element of any list
            WhereCondition::Contains(_) => match (where_list_elements(referenced), bound) {
                (Some(elements), Some(bound)) => elements
                    .iter()
                    .any(|element| element.same_scalar_data(bound)),
                _ => false,
            },
            WhereCondition::ContainsAll(_) => {
                match (where_list_elements(referenced), where_list_elements(bound)) {
                    (Some(held), Some(wanted)) => list_holds_all(held, wanted),
                    _ => false,
                }
            }
        }
    }
}

/// Whether `held` has, for every element of `wanted`, an element equal to it
/// as `Value::same_scalar_data` compares two single values, each wanted one
/// looked up in a set of the held ones, so two lists of up to
/// `max_typed_array_items` elements each are compared in time near their
/// length, not its square, whatever they hold. Every value but a float is
/// looked up by the order [`ListElement`] gives it. A float equals a number,
/// a float or an integer, exactly when the two read as the same `f64` bits
/// (`Value::as_float`), so a wanted float is looked up among the bits of every
/// held number, and a wanted integer, beside its order, among those of every
/// held float. A map or a list of anything but bytes equals nothing.
fn list_holds_all(held: &[Value], wanted: &[Value]) -> bool {
    let keyed: BTreeSet<ListElement> = held.iter().filter_map(ListElement::of).collect();
    let number_bits: BTreeSet<u64> = held
        .iter()
        .filter_map(|element| element.as_float().map(f64::to_bits))
        .collect();
    let float_bits: BTreeSet<u64> = held
        .iter()
        .filter_map(|element| match element {
            Value::Float(float) => Some(float.to_bits()),
            _ => None,
        })
        .collect();
    wanted.iter().all(|element| match element {
        Value::Float(float) => number_bits.contains(&float.to_bits()),
        element => ListElement::of(element).is_some_and(|key| {
            keyed.contains(&key)
                || element
                    .as_float()
                    .is_some_and(|number| float_bits.contains(&number.to_bits()))
        }),
    })
}

/// The elements of a typed array a `where` entry over lists reads: a list the
/// document leaves out, or holds as null, is the empty one, as an indexed typed
/// array without elements is absent. `None` for a value that is no list, which
/// no stored typed array is and which then holds nothing and equals no list.
pub fn where_list_elements(value: Option<&Value>) -> Option<&[Value]> {
    match value {
        None | Some(Value::Null) => Some(&[]),
        Some(Value::Array(elements)) => Some(elements),
        Some(_) => None,
    }
}

/// How many elements a `where` entry reads of the typed array `value`: its
/// length, none for a list left out or a value that is no list.
fn where_list_len(value: Option<&Value>) -> usize {
    where_list_elements(value).map_or(0, <[Value]>::len)
}

/// How many elements [`where_lists_equal`] reads of `a` and `b`, both lists':
/// what document reference validation bills before comparing them.
pub fn where_lists_elements_read(a: Option<&Value>, b: Option<&Value>) -> usize {
    where_list_len(a).saturating_add(where_list_len(b))
}

/// Whether two typed array values are equal as a `where` equality between two
/// typed arrays compares them: the same elements in the same order, a list the
/// document leaves out the empty one. Two elements are compared as a scalar
/// equality compares two single values (`Value::same_scalar_data`): an integer
/// whatever width it is held at, the bytes of an identifier or a byte array
/// whichever variant holds them (an identifier and its 32 bytes are equal), a
/// string as a string, a boolean as a boolean.
pub fn where_lists_equal(a: Option<&Value>, b: Option<&Value>) -> bool {
    match (where_list_elements(a), where_list_elements(b)) {
        (Some(a), Some(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.same_scalar_data(b))
        }
        _ => false,
    }
}

/// A single value other than a float as `Value::same_scalar_data` compares it:
/// two such values are equal exactly when they read as the same `ListElement`,
/// which orders them, so a `containsAll` looks each element up in a set. A
/// float compares with an integer as the float the integer rounds to, which is
/// no order, so it reads as none and is compared one by one.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ListElement<'a> {
    Null,
    Boolean(bool),
    Integer(i128),
    /// An unsigned integer past `i128::MAX`.
    LargeUnsigned(u128),
    Text(&'a str),
    Bytes(Cow<'a, [u8]>),
    EnumU8(&'a [u8]),
    EnumString(&'a [String]),
}

impl<'a> ListElement<'a> {
    /// The element `value` reads as, `None` for a float and for a value
    /// `same_scalar_data` finds equal to nothing: a map, or a list of anything
    /// but bytes.
    fn of(value: &'a Value) -> Option<Self> {
        Some(match value {
            Value::Null => ListElement::Null,
            Value::Bool(flag) => ListElement::Boolean(*flag),
            Value::Text(text) => ListElement::Text(text),
            Value::U128(integer) => match i128::try_from(*integer) {
                Ok(integer) => ListElement::Integer(integer),
                Err(_) => ListElement::LargeUnsigned(*integer),
            },
            Value::I128(integer) => ListElement::Integer(*integer),
            Value::U64(integer) => ListElement::Integer(i128::from(*integer)),
            Value::I64(integer) => ListElement::Integer(i128::from(*integer)),
            Value::U32(integer) => ListElement::Integer(i128::from(*integer)),
            Value::I32(integer) => ListElement::Integer(i128::from(*integer)),
            Value::U16(integer) => ListElement::Integer(i128::from(*integer)),
            Value::I16(integer) => ListElement::Integer(i128::from(*integer)),
            Value::U8(integer) => ListElement::Integer(i128::from(*integer)),
            Value::I8(integer) => ListElement::Integer(i128::from(*integer)),
            Value::EnumU8(values) => ListElement::EnumU8(values),
            Value::EnumString(values) => ListElement::EnumString(values),
            Value::Bytes(bytes) => ListElement::Bytes(Cow::Borrowed(bytes)),
            Value::Bytes20(bytes) => ListElement::Bytes(Cow::Borrowed(bytes)),
            Value::Bytes32(bytes) => ListElement::Bytes(Cow::Borrowed(bytes)),
            Value::Bytes36(bytes) => ListElement::Bytes(Cow::Borrowed(bytes)),
            Value::Identifier(bytes) => ListElement::Bytes(Cow::Borrowed(bytes)),
            // A transition may carry an identifier or a byte array as the bytes
            // it lists
            Value::Array(items) => ListElement::Bytes(Cow::Owned(
                items
                    .iter()
                    .map(|item| match item {
                        Value::U8(byte) => Some(*byte),
                        _ => None,
                    })
                    .collect::<Option<Vec<u8>>>()?,
            )),
            _ => return None,
        })
    }
}

impl fmt::Display for WhereCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WhereCondition::Constant(literal) => write!(f, "{{ \"const\": {literal} }}"),
            WhereCondition::NotEqual(bound) => write!(f, "{{ \"notEqual\": {bound} }}"),
            WhereCondition::Contains(bound) => write!(f, "{{ \"contains\": {bound} }}"),
            WhereCondition::ContainsAll(path) => write!(f, "{{ \"containsAll\": \"{path}\" }}"),
            WhereCondition::In(literals) => {
                write!(f, "{{ \"in\": [")?;
                for (index, literal) in literals.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{literal}")?;
                }
                write!(f, "] }}")
            }
            WhereCondition::Ordering { ordering, bound } => {
                write!(f, "{{ \"{}\": {bound} }}", ordering.wire_name())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ordering(ordering: WhereOrdering, bound: WhereOrderingBound) -> WhereCondition {
        WhereCondition::Ordering { ordering, bound }
    }

    #[test]
    fn should_read_a_literal_as_the_kind_of_the_referenced_property() {
        let text = WhereLiteral::Text("delivered".to_string());
        assert_eq!(
            text.value_for(WhereLiteralKind::Text),
            Some(Value::Text("delivered".to_string()))
        );
        assert_eq!(text.value_for(WhereLiteralKind::Integer), None);
        assert_eq!(text.value_for(WhereLiteralKind::Identifier), None);
        assert_eq!(text.value_for(WhereLiteralKind::Bytes), None);

        let hex = WhereLiteral::Text("cafe".to_string());
        assert_eq!(
            hex.value_for(WhereLiteralKind::Bytes),
            Some(Value::Bytes(vec![0xca, 0xfe]))
        );

        let id = Identifier::new([7; 32]);
        let base58 = WhereLiteral::Text(id.to_string(Encoding::Base58));
        assert_eq!(
            base58.value_for(WhereLiteralKind::Identifier),
            Some(Value::Identifier([7; 32]))
        );
        assert_eq!(
            WhereLiteral::Boolean(true).value_for(WhereLiteralKind::Boolean),
            Some(Value::Bool(true))
        );
        assert_eq!(
            WhereLiteral::Boolean(true).value_for(WhereLiteralKind::Integer),
            None
        );
    }

    #[test]
    fn should_spell_a_literal_from_a_whole_number_float() {
        assert_eq!(
            WhereLiteral::from_value(&Value::Float(5.0)),
            Some(WhereLiteral::Integer(5))
        );
        assert_eq!(WhereLiteral::from_value(&Value::Float(5.5)), None);
        assert_eq!(
            WhereLiteral::from_value(&Value::U64(9)),
            Some(WhereLiteral::Integer(9))
        );
        assert_eq!(WhereLiteral::from_value(&Value::Null), None);
    }

    #[test]
    fn should_hold_a_constant_only_for_an_equal_present_value() {
        let delivered = WhereCondition::Constant(WhereLiteral::Text("delivered".to_string()));
        let kind = Some(WhereLiteralKind::Text);
        assert!(delivered.holds(Some(&Value::Text("delivered".to_string())), kind, None));
        assert!(!delivered.holds(Some(&Value::Text("refunded".to_string())), kind, None));
        assert!(!delivered.holds(None, kind, None));

        // An integer constant equals the value whatever width it is stored at
        let three = WhereCondition::Constant(WhereLiteral::Integer(3));
        assert!(three.holds(Some(&Value::U8(3)), Some(WhereLiteralKind::Integer), None));
        assert!(!three.holds(Some(&Value::U8(4)), Some(WhereLiteralKind::Integer), None));
    }

    #[test]
    fn should_hold_an_in_for_any_listed_value() {
        let open = WhereCondition::In(
            ["open", "extended"]
                .into_iter()
                .map(|text| WhereLiteral::Text(text.to_string()))
                .collect(),
        );
        let kind = Some(WhereLiteralKind::Text);
        assert!(open.holds(Some(&Value::Text("extended".to_string())), kind, None));
        assert!(!open.holds(Some(&Value::Text("closed".to_string())), kind, None));
        assert!(!open.holds(None, kind, None));
    }

    #[test]
    fn should_negate_equality_for_not_equal() {
        let other = WhereCondition::NotEqual(WhereNotEqualBound::Writer);
        let kind = Some(WhereLiteralKind::Identifier);
        let writer = Value::Identifier([1; 32]);
        assert!(!other.holds(Some(&Value::Identifier([1; 32])), kind, Some(&writer)));
        assert!(other.holds(Some(&Value::Identifier([2; 32])), kind, Some(&writer)));

        // Both sides absent are equal, so notEqual does not hold; one absent
        // differs, so it does
        let unlike = WhereCondition::NotEqual(WhereNotEqualBound::Property("tag".to_string()));
        let kind = Some(WhereLiteralKind::Text);
        assert!(!unlike.holds(None, kind, None));
        assert!(unlike.holds(Some(&Value::Text("a".to_string())), kind, None));
        assert!(unlike.holds(None, kind, Some(&Value::Text("a".to_string()))));

        let not_closed = WhereCondition::NotEqual(WhereNotEqualBound::Constant(
            WhereLiteral::Text("closed".to_string()),
        ));
        assert!(not_closed.holds(None, kind, None));
        assert!(!not_closed.holds(Some(&Value::Text("closed".to_string())), kind, None));
    }

    #[test]
    fn should_order_integers_only_when_both_sides_have_a_value() {
        let kind = Some(WhereLiteralKind::Integer);
        let after_creation = ordering(
            WhereOrdering::GreaterThan,
            WhereOrderingBound::System(SystemProperty::CreatedAt),
        );
        assert!(after_creation.holds(Some(&Value::U64(10)), kind, Some(&Value::U64(9))));
        assert!(!after_creation.holds(Some(&Value::U64(9)), kind, Some(&Value::U64(9))));
        assert!(!after_creation.holds(None, kind, Some(&Value::U64(9))));
        assert!(!after_creation.holds(Some(&Value::U64(10)), kind, None));

        let at_most_amount = ordering(
            WhereOrdering::LessThanOrEqual,
            WhereOrderingBound::Property("amount".to_string()),
        );
        assert!(at_most_amount.holds(Some(&Value::I64(-5)), kind, Some(&Value::U32(0))));
        assert!(!at_most_amount.holds(Some(&Value::I64(1)), kind, Some(&Value::U32(0))));

        let at_least_ten = ordering(
            WhereOrdering::GreaterThanOrEqual,
            WhereOrderingBound::Constant(10),
        );
        assert!(at_least_ten.holds(Some(&Value::U16(10)), kind, None));
        assert!(!at_least_ten.holds(Some(&Value::U16(9)), kind, None));
        // A float is no integer, which an ordering compares
        assert!(!at_least_ten.holds(Some(&Value::Float(11.0)), kind, None));
    }

    #[test]
    fn should_recheck_on_every_replace_only_bounds_that_change_without_a_write() {
        assert!(
            WhereCondition::NotEqual(WhereNotEqualBound::Writer).is_rechecked_on_every_replace()
        );
        assert!(ordering(
            WhereOrdering::LessThan,
            WhereOrderingBound::System(SystemProperty::UpdatedAt)
        )
        .is_rechecked_on_every_replace());
        assert!(ordering(
            WhereOrdering::LessThan,
            WhereOrderingBound::System(SystemProperty::TransferredAtBlockHeight)
        )
        .is_rechecked_on_every_replace());
        assert!(!ordering(
            WhereOrdering::LessThan,
            WhereOrderingBound::System(SystemProperty::CreatedAt)
        )
        .is_rechecked_on_every_replace());
        assert!(!ordering(
            WhereOrdering::LessThan,
            WhereOrderingBound::Property("amount".to_string())
        )
        .is_rechecked_on_every_replace());
        assert!(
            !WhereCondition::Constant(WhereLiteral::Boolean(true)).is_rechecked_on_every_replace()
        );
    }

    #[test]
    fn should_display_the_condition_as_written() {
        assert_eq!(
            ordering(
                WhereOrdering::GreaterThan,
                WhereOrderingBound::System(SystemProperty::CreatedAt)
            )
            .to_string(),
            "{ \"greaterThan\": \"$createdAt\" }"
        );
        assert_eq!(
            WhereCondition::Constant(WhereLiteral::Boolean(true)).to_string(),
            "{ \"const\": true }"
        );
        assert_eq!(
            WhereCondition::In(
                [WhereLiteral::Integer(2), WhereLiteral::Integer(1)]
                    .into_iter()
                    .collect()
            )
            .to_string(),
            "{ \"in\": [2, 1] }"
        );
        // A string as JSON writes it, quotes and backslashes escaped
        assert_eq!(
            WhereCondition::Constant(WhereLiteral::Text("say \"hi\" \\o/".to_string())).to_string(),
            r#"{ "const": "say \"hi\" \\o/" }"#
        );
        assert_eq!(
            WhereCondition::Contains(WhereContainsBound::Writer).to_string(),
            "{ \"contains\": \"$ownerId\" }"
        );
        assert_eq!(
            WhereCondition::Contains(WhereContainsBound::Property("tag".to_string())).to_string(),
            "{ \"contains\": \"tag\" }"
        );
        assert_eq!(
            WhereCondition::ContainsAll("tags".to_string()).to_string(),
            "{ \"containsAll\": \"tags\" }"
        );
    }

    fn texts(texts: &[&str]) -> Value {
        Value::Array(
            texts
                .iter()
                .map(|text| Value::Text(text.to_string()))
                .collect(),
        )
    }

    #[test]
    fn should_equal_a_list_with_the_same_elements_in_the_same_order() {
        let ab = texts(&["a", "b"]);
        assert!(where_lists_equal(Some(&ab), Some(&texts(&["a", "b"]))));
        assert!(!where_lists_equal(Some(&ab), Some(&texts(&["b", "a"]))));
        assert!(!where_lists_equal(Some(&ab), Some(&texts(&["a"]))));
        assert!(!where_lists_equal(
            Some(&ab),
            Some(&texts(&["a", "b", "b"]))
        ));
        // A list left out, or null, is the empty one
        assert!(where_lists_equal(None, None));
        assert!(where_lists_equal(None, Some(&texts(&[]))));
        assert!(where_lists_equal(Some(&Value::Null), Some(&texts(&[]))));
        assert!(!where_lists_equal(None, Some(&ab)));
        // A value that is no list equals no list
        assert!(!where_lists_equal(
            Some(&Value::Text("a".to_string())),
            None
        ));
    }

    #[test]
    fn should_compare_list_elements_by_value_not_by_encoding() {
        let id = [7; 32];
        let held = Value::Array(vec![Value::Identifier(id), Value::Identifier([8; 32])]);
        // An identifier equals its 32 bytes, however a transition carries them
        let carried = Value::Array(vec![
            Value::Bytes(id.to_vec()),
            Value::Array(vec![Value::U8(8); 32]),
        ]);
        assert!(where_lists_equal(Some(&held), Some(&carried)));
        // An integer at any width, and a whole number carried as a float
        let held = Value::Array(vec![Value::U16(3), Value::I64(-4)]);
        let carried = Value::Array(vec![Value::U64(3), Value::Float(-4.0)]);
        assert!(where_lists_equal(Some(&held), Some(&carried)));
        assert!(!where_lists_equal(
            Some(&held),
            Some(&Value::Array(vec![Value::U64(3), Value::Float(-4.5)]))
        ));
        // A string never equals bytes, and an element of no list kind equals
        // nothing, not even itself
        assert!(!where_lists_equal(
            Some(&texts(&["a"])),
            Some(&Value::Array(vec![Value::Bytes(b"a".to_vec())]))
        ));
        let maps = Value::Array(vec![Value::Map(vec![])]);
        assert!(!where_lists_equal(Some(&maps), Some(&maps)));
        // An unsigned integer past i128::MAX equals only itself
        let large = Value::Array(vec![Value::U128(u128::MAX)]);
        assert!(where_lists_equal(Some(&large), Some(&large)));
        assert!(!where_lists_equal(
            Some(&large),
            Some(&Value::Array(vec![Value::I128(-1)]))
        ));
    }

    /// The three list forms compare two elements exactly as a scalar equality
    /// compares two single values, floats included: `-0.0` is not `0`, and an
    /// integer past 2^53 equals the float it rounds to.
    #[test]
    fn should_compare_list_elements_as_a_scalar_equality_does() {
        let sample = [
            Value::Null,
            Value::Bool(true),
            Value::Bool(false),
            Value::U8(0),
            Value::I8(0),
            Value::I8(-4),
            Value::U16(3),
            Value::I16(3),
            Value::U32(3),
            Value::I32(-4),
            Value::I64(0),
            Value::U64(3),
            Value::U128(3),
            Value::I128(-4),
            Value::U128(u128::MAX),
            Value::I128(i128::MAX),
            Value::Float(0.0),
            Value::Float(-0.0),
            Value::Float(3.0),
            Value::Float(3.5),
            Value::Float(f64::NAN),
            Value::Float(9007199254740992.0),
            Value::U64(9007199254740993),
            Value::Text("a".to_string()),
            Value::Text(String::new()),
            Value::Bytes(vec![7; 32]),
            Value::Bytes(vec![7; 20]),
            Value::Bytes(vec![7; 36]),
            Value::Bytes(vec![]),
            Value::Bytes20([7; 20]),
            Value::Bytes32([7; 32]),
            Value::Bytes36([7; 36]),
            Value::Identifier([7; 32]),
            Value::Array(vec![Value::U8(7); 20]),
            Value::Array(vec![Value::U8(7); 32]),
            Value::Array(vec![Value::U8(7); 36]),
            Value::Array(vec![]),
            Value::Array(vec![Value::Text("a".to_string())]),
            Value::Map(vec![]),
            Value::EnumU8(vec![1]),
            Value::EnumU8(vec![7; 32]),
            Value::EnumString(vec!["a".to_string()]),
            Value::EnumString(vec![]),
        ];
        let all_tags = WhereCondition::ContainsAll("tags".to_string());
        let tag = WhereCondition::Contains(WhereContainsBound::Property("tag".to_string()));
        for held in &sample {
            for wanted in &sample {
                let scalar = held.same_scalar_data(wanted);
                let held_list = Value::Array(vec![held.clone()]);
                let wanted_list = Value::Array(vec![wanted.clone()]);
                let pair = format!("{held:?} against {wanted:?}");
                assert_eq!(
                    where_lists_equal(Some(&held_list), Some(&wanted_list)),
                    scalar,
                    "equality of {pair}"
                );
                assert_eq!(
                    tag.holds(Some(&held_list), None, Some(wanted)),
                    scalar,
                    "contains of {pair}"
                );
                assert_eq!(
                    all_tags.holds(Some(&held_list), None, Some(&wanted_list)),
                    scalar,
                    "containsAll of {pair}"
                );
            }
        }
        // Every value held at once, floats among them, each looked up in the set
        let everything = Value::Array(sample.to_vec());
        for wanted in &sample {
            assert_eq!(
                all_tags.holds(
                    Some(&everything),
                    None,
                    Some(&Value::Array(vec![wanted.clone()]))
                ),
                sample.iter().any(|held| held.same_scalar_data(wanted)),
                "containsAll of {wanted:?} among the whole sample"
            );
        }
    }

    /// What document reference validation bills before comparing: every
    /// element of both lists for an equality and a `containsAll`, the held
    /// list's for a `contains`, a list left out none, and nothing for an entry
    /// comparing single values.
    #[test]
    fn should_count_the_list_elements_an_entry_reads() {
        let three = texts(&["a", "b", "c"]);
        let two = texts(&["a", "b"]);
        assert_eq!(where_lists_elements_read(Some(&three), Some(&two)), 5);
        assert_eq!(where_lists_elements_read(None, Some(&two)), 2);
        let all_tags = WhereCondition::ContainsAll("tags".to_string());
        assert_eq!(all_tags.list_elements_read(Some(&three), Some(&two)), 5);
        assert_eq!(all_tags.list_elements_read(Some(&three), None), 3);
        let tag = WhereCondition::Contains(WhereContainsBound::Property("tag".to_string()));
        assert_eq!(
            tag.list_elements_read(Some(&three), Some(&Value::Text("a".to_string()))),
            3
        );
        let constant = WhereCondition::Constant(WhereLiteral::Text("a".to_string()));
        assert_eq!(constant.list_elements_read(Some(&three), None), 0);
    }

    #[test]
    fn should_hold_contains_only_for_a_held_element() {
        let writer = WhereCondition::Contains(WhereContainsBound::Writer);
        let members = Value::Array(vec![Value::Identifier([1; 32]), Value::Identifier([2; 32])]);
        assert!(writer.holds(Some(&members), None, Some(&Value::Identifier([2; 32]))));
        assert!(writer.holds(Some(&members), None, Some(&Value::Bytes(vec![1; 32]))));
        assert!(!writer.holds(Some(&members), None, Some(&Value::Identifier([3; 32]))));
        // An absent list holds nothing, and an absent value is held by no list
        assert!(!writer.holds(None, None, Some(&Value::Identifier([1; 32]))));
        let tag = WhereCondition::Contains(WhereContainsBound::Property("tag".to_string()));
        assert!(!tag.holds(Some(&texts(&["a"])), None, None));
        let b = Value::Text("b".to_string());
        assert!(tag.holds(Some(&texts(&["a", "b"])), None, Some(&b)));
        assert!(!tag.holds(None, None, Some(&b)));
    }

    #[test]
    fn should_hold_contains_all_for_every_element_in_any_order() {
        let all_tags = WhereCondition::ContainsAll("tags".to_string());
        let held = texts(&["a", "b", "c"]);
        assert!(all_tags.holds(Some(&held), None, Some(&texts(&["c", "a"]))));
        assert!(all_tags.holds(Some(&held), None, Some(&texts(&["a", "a"]))));
        assert!(!all_tags.holds(Some(&held), None, Some(&texts(&["a", "d"]))));
        // Every list holds all of an empty one, a list left out among them
        assert!(all_tags.holds(Some(&held), None, Some(&texts(&[]))));
        assert!(all_tags.holds(Some(&held), None, None));
        assert!(all_tags.holds(None, None, None));
        assert!(!all_tags.holds(None, None, Some(&texts(&["a"]))));
        // By value, as the equality compares
        let held = Value::Array(vec![Value::Identifier([1; 32]), Value::Identifier([2; 32])]);
        let wanted = Value::Array(vec![Value::Array(vec![Value::U8(2); 32])]);
        assert!(all_tags.holds(Some(&held), None, Some(&wanted)));
    }

    #[test]
    fn should_recheck_a_contains_of_the_writer_on_every_replace() {
        assert!(
            WhereCondition::Contains(WhereContainsBound::Writer).is_rechecked_on_every_replace()
        );
        assert!(
            !WhereCondition::Contains(WhereContainsBound::Property("tag".to_string()))
                .is_rechecked_on_every_replace()
        );
        assert!(!WhereCondition::ContainsAll("tags".to_string()).is_rechecked_on_every_replace());
        assert_eq!(
            WhereCondition::ContainsAll("tags".to_string()).referring_property(),
            Some("tags")
        );
        assert_eq!(
            WhereCondition::Contains(WhereContainsBound::Writer).referring_property(),
            None
        );
    }

    #[test]
    fn should_compare_a_literal_as_the_value_it_reads_as() {
        let id = Identifier::new([7; 32]);
        let base58 = WhereLiteral::Text(id.to_string(Encoding::Base58));
        let kind = WhereLiteralKind::Identifier;
        assert!(base58.equals(&Value::Identifier([7; 32]), kind));
        assert!(base58.equals(&Value::Bytes(vec![7; 32]), kind));
        assert!(!base58.equals(&Value::Identifier([8; 32]), kind));
        assert!(!WhereLiteral::Text("not base58 0OIl".to_string())
            .equals(&Value::Identifier([7; 32]), kind));

        let kind = WhereLiteralKind::Bytes;
        for hex in ["cafe", "CAFE", "CaFe"] {
            assert!(
                WhereLiteral::Text(hex.to_string()).equals(&Value::Bytes(vec![0xca, 0xfe]), kind)
            );
        }
        assert!(!WhereLiteral::Text("cafe".to_string()).equals(&Value::Bytes(vec![0xca]), kind));
        assert!(
            !WhereLiteral::Text("+a+e".to_string()).equals(&Value::Bytes(vec![0x0a, 0x0e]), kind)
        );
        assert!(
            !WhereLiteral::Text("caf".to_string()).equals(&Value::Bytes(vec![0xca, 0x0f]), kind)
        );
    }
}
