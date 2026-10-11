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
//!   "$creatorId": { "notEqual": "$ownerId" }
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

use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::property::{
    is_referenced_system_agreement_property, DocumentPropertyType,
};
use crate::data_contract::document_type::property_constraints::SystemProperty;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::property_names::OWNER_ID;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::string_encoding::Encoding;
use platform_value::{Identifier, Value};
use serde::Serialize;
use std::collections::BTreeSet;
use std::fmt;

/// The key of a `where` entry's literal: `{ "const": ... }`.
pub const WHERE_CONST: &str = "const";
/// The key of a `where` entry the referenced side must differ from.
pub const WHERE_NOT_EQUAL: &str = "notEqual";
/// The key of a `where` entry listing the literals the referenced side may be.
pub const WHERE_IN: &str = "in";

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
            // The cast is exact: the float is a whole number inside the range
            Value::Float(float)
                if float.fract() == 0.0
                    && *float >= i128::MIN as f64
                    && *float < i128::MAX as f64 =>
            {
                Some(WhereLiteral::Integer(*float as i128))
            }
            value if value.is_integer() => {
                value.to_integer::<i128>().ok().map(WhereLiteral::Integer)
            }
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
            WhereLiteral::Text(text) => write!(f, "\"{text}\""),
        }
    }
}

/// What the referenced side of a `notEqual` or of an ordering is compared
/// with.
// Embedded in `DocumentPropertyReferenceTarget`, which consensus errors carry.
// @append_only
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Encode, Decode, DecodeUntrusted)]
#[serde(rename_all = "camelCase")]
pub enum WhereBound {
    /// A property of the referring document, by its dotted path.
    Property(String),
    /// `"$ownerId"`: the writer.
    Writer,
    /// `{ "const": ... }`: a literal.
    Constant(WhereLiteral),
    /// A time or height the referring document records (`"$createdAt"`),
    /// which its type lists in `required`: an ordering only.
    System(SystemProperty),
}

impl WhereBound {
    /// The property of the referring document the bound reads, `None` for the
    /// writer, a literal or a time or height.
    pub fn referring_property(&self) -> Option<&str> {
        match self {
            WhereBound::Property(path) => Some(path),
            WhereBound::Writer | WhereBound::Constant(_) | WhereBound::System(_) => None,
        }
    }

    /// Whether the value the bound reads may differ between two writes of the
    /// referring document that change none of its properties: the writer, who
    /// may be a new owner after a transfer or a purchase, and the time and
    /// heights of the last update or transfer. An entry with such a bound is
    /// checked again on every replace, as a writer gate is.
    pub fn changes_without_a_write(&self) -> bool {
        match self {
            WhereBound::Writer => true,
            WhereBound::System(property) => !matches!(
                property,
                SystemProperty::CreatedAt
                    | SystemProperty::CreatedAtBlockHeight
                    | SystemProperty::CreatedAtCoreBlockHeight
            ),
            WhereBound::Property(_) | WhereBound::Constant(_) => false,
        }
    }
}

impl fmt::Display for WhereBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WhereBound::Property(path) => write!(f, "\"{path}\""),
            WhereBound::Writer => write!(f, "\"{OWNER_ID}\""),
            WhereBound::Constant(literal) => write!(f, "{{ \"const\": {literal} }}"),
            WhereBound::System(property) => write!(f, "\"{}\"", property.name()),
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
    NotEqual(WhereBound),
    /// `{ "in": [literal, ...] }`: equal to one of two or more distinct
    /// literals. A referenced side the document leaves out is in none.
    In(BTreeSet<WhereLiteral>),
    /// `{ "greaterThan": bound }` and the other orderings: an integer that
    /// compares with the bound, a property of the referring document, a
    /// literal or a time or height of the referring document, this way. Holds
    /// only when both sides have a value.
    Ordering {
        ordering: WhereOrdering,
        bound: WhereBound,
    },
}

impl WhereCondition {
    /// The key declaring the condition.
    pub fn wire_name(&self) -> &'static str {
        match self {
            WhereCondition::Constant(_) => WHERE_CONST,
            WhereCondition::NotEqual(_) => WHERE_NOT_EQUAL,
            WhereCondition::In(_) => WHERE_IN,
            WhereCondition::Ordering { ordering, .. } => ordering.wire_name(),
        }
    }

    /// The bound a `notEqual` or an ordering compares with, `None` for a
    /// literal or a list of them.
    pub fn bound(&self) -> Option<&WhereBound> {
        match self {
            WhereCondition::NotEqual(bound) | WhereCondition::Ordering { bound, .. } => Some(bound),
            WhereCondition::Constant(_) | WhereCondition::In(_) => None,
        }
    }

    /// The property of the referring document the condition reads, `None`
    /// when it reads none.
    pub fn referring_property(&self) -> Option<&str> {
        self.bound().and_then(WhereBound::referring_property)
    }

    /// Whether the condition is checked again on every replace of the
    /// referring document, as a writer gate is: its bound is the writer or the
    /// time or heights of the last update or transfer
    /// ([`WhereBound::changes_without_a_write`]).
    pub fn is_rechecked_on_every_replace(&self) -> bool {
        self.bound()
            .is_some_and(WhereBound::changes_without_a_write)
    }

    /// Whether the condition holds for `referenced`, the referenced side's
    /// value (`None` when the document leaves it out) of `kind` (`None` for a
    /// property no literal reads as, which registration refuses a literal
    /// for), and `bound`, the value of a bound the referring document reads
    /// (`None` when it leaves it out, and for a literal or a list, which carry
    /// their own): a time or height reads as an integer, the writer as an
    /// identifier.
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
            WhereCondition::NotEqual(WhereBound::Constant(literal)) => !equals_literal(literal),
            // The negation of an equality entry: both absent agree
            WhereCondition::NotEqual(_) => !match (referenced, bound) {
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
                    WhereBound::Constant(WhereLiteral::Integer(integer)) => Some(*integer),
                    WhereBound::Constant(_) => None,
                    WhereBound::Property(_) | WhereBound::Writer | WhereBound::System(_) => {
                        integer(bound)
                    }
                };
                matches!(
                    (integer(referenced), bound),
                    (Some(referenced), Some(bound)) if ordering.holds(referenced, bound)
                )
            }
        }
    }
}

impl fmt::Display for WhereCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WhereCondition::Constant(literal) => write!(f, "{{ \"const\": {literal} }}"),
            WhereCondition::NotEqual(bound) => write!(f, "{{ \"notEqual\": {bound} }}"),
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

    fn ordering(ordering: WhereOrdering, bound: WhereBound) -> WhereCondition {
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
        let other = WhereCondition::NotEqual(WhereBound::Writer);
        let kind = Some(WhereLiteralKind::Identifier);
        let writer = Value::Identifier([1; 32]);
        assert!(!other.holds(Some(&Value::Identifier([1; 32])), kind, Some(&writer)));
        assert!(other.holds(Some(&Value::Identifier([2; 32])), kind, Some(&writer)));

        // Both sides absent are equal, so notEqual does not hold; one absent
        // differs, so it does
        let unlike = WhereCondition::NotEqual(WhereBound::Property("tag".to_string()));
        let kind = Some(WhereLiteralKind::Text);
        assert!(!unlike.holds(None, kind, None));
        assert!(unlike.holds(Some(&Value::Text("a".to_string())), kind, None));
        assert!(unlike.holds(None, kind, Some(&Value::Text("a".to_string()))));

        let not_closed = WhereCondition::NotEqual(WhereBound::Constant(WhereLiteral::Text(
            "closed".to_string(),
        )));
        assert!(not_closed.holds(None, kind, None));
        assert!(!not_closed.holds(Some(&Value::Text("closed".to_string())), kind, None));
    }

    #[test]
    fn should_order_integers_only_when_both_sides_have_a_value() {
        let kind = Some(WhereLiteralKind::Integer);
        let after_creation = ordering(
            WhereOrdering::GreaterThan,
            WhereBound::System(SystemProperty::CreatedAt),
        );
        assert!(after_creation.holds(Some(&Value::U64(10)), kind, Some(&Value::U64(9))));
        assert!(!after_creation.holds(Some(&Value::U64(9)), kind, Some(&Value::U64(9))));
        assert!(!after_creation.holds(None, kind, Some(&Value::U64(9))));
        assert!(!after_creation.holds(Some(&Value::U64(10)), kind, None));

        let at_most_amount = ordering(
            WhereOrdering::LessThanOrEqual,
            WhereBound::Property("amount".to_string()),
        );
        assert!(at_most_amount.holds(Some(&Value::I64(-5)), kind, Some(&Value::U32(0))));
        assert!(!at_most_amount.holds(Some(&Value::I64(1)), kind, Some(&Value::U32(0))));

        let at_least_ten = ordering(
            WhereOrdering::GreaterThanOrEqual,
            WhereBound::Constant(WhereLiteral::Integer(10)),
        );
        assert!(at_least_ten.holds(Some(&Value::U16(10)), kind, None));
        assert!(!at_least_ten.holds(Some(&Value::U16(9)), kind, None));
        // A float is no integer, which an ordering compares
        assert!(!at_least_ten.holds(Some(&Value::Float(11.0)), kind, None));
    }

    #[test]
    fn should_recheck_on_every_replace_only_bounds_that_change_without_a_write() {
        assert!(WhereCondition::NotEqual(WhereBound::Writer).is_rechecked_on_every_replace());
        assert!(ordering(
            WhereOrdering::LessThan,
            WhereBound::System(SystemProperty::UpdatedAt)
        )
        .is_rechecked_on_every_replace());
        assert!(ordering(
            WhereOrdering::LessThan,
            WhereBound::System(SystemProperty::TransferredAtBlockHeight)
        )
        .is_rechecked_on_every_replace());
        assert!(!ordering(
            WhereOrdering::LessThan,
            WhereBound::System(SystemProperty::CreatedAt)
        )
        .is_rechecked_on_every_replace());
        assert!(!ordering(
            WhereOrdering::LessThan,
            WhereBound::Property("amount".to_string())
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
                WhereBound::System(SystemProperty::CreatedAt)
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
            "{ \"in\": [1, 2] }"
        );
    }
}
