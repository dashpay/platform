//! The stored form of a document type's rules ([`PropertyConstraint`] and the integer
//! expressions under it): `propertyConstraints`, `deleteConstraints`, `immutable` conditions
//! and `retractedWhen`. Every edge from a rule or an expression to one of its own kind counts
//! against the decoding bound ([`Nested`]).

use super::nested::Nested;
use crate::data_contract::document_type::property_constraints::{
    AffixPosition, AggregateBinding, AggregateKind, AggregateRead, BytesOperand,
    ConstraintComparison, ConstraintExpression, ContainsNeedle, EqualityKind, PropertyConstraint,
    SizeMeasure, SystemProperty, SystemRead, SystemValue, TextOperand, TextProperty,
};
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::Identifier;
use std::collections::{BTreeMap, BTreeSet};

/// [`PropertyConstraint`].
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredRule {
    Compare {
        comparison: StoredComparison,
        left: StoredExpression,
        right: StoredExpression,
    },
    In {
        operand: StoredExpression,
        values: BTreeSet<i128>,
    },
    TextCompare {
        comparison: StoredComparison,
        property: StoredTextProperty,
        value: String,
    },
    TextCompareProperties {
        comparison: StoredComparison,
        left: StoredTextProperty,
        right: StoredTextProperty,
    },
    TextIn {
        property: StoredTextProperty,
        values: BTreeSet<String>,
    },
    IdentifierCompare {
        comparison: StoredComparison,
        path: String,
        value: Identifier,
    },
    IdentifierCompareProperties {
        comparison: StoredComparison,
        left: String,
        right: String,
    },
    IdentifierIn {
        path: String,
        values: BTreeSet<Identifier>,
    },
    TextAffix {
        position: StoredAffixPosition,
        text: StoredTextOperand,
        affix: StoredTextOperand,
    },
    BytesAffix {
        position: StoredAffixPosition,
        bytes: StoredBytesOperand,
        affix: StoredBytesOperand,
    },
    Contains {
        array: String,
        needle: StoredContainsNeedle,
    },
    Present(String),
    Absent(String),
    AnyOf(Nested<Vec<StoredRule>>),
    AllOf(Nested<Vec<StoredRule>>),
    Not(Box<Nested<StoredRule>>),
    IfThen {
        condition: Box<Nested<StoredRule>>,
        then: Box<Nested<StoredRule>>,
        otherwise: Option<Box<Nested<StoredRule>>>,
    },
    NotIn(Box<Nested<StoredRule>>),
}

/// [`ConstraintExpression`].
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredExpression {
    Value(i128),
    Property {
        path: String,
        if_absent: i128,
    },
    Size {
        measure: StoredSizeMeasure,
        path: String,
    },
    CountPresent(Vec<String>),
    ByteAt {
        path: String,
        index: u16,
    },
    System(StoredSystemRead),
    Add(Nested<Vec<StoredExpression>>),
    Multiply(Nested<Vec<StoredExpression>>),
    Subtract(Box<Nested<(StoredExpression, StoredExpression)>>),
    Divide(Box<Nested<(StoredExpression, StoredExpression)>>),
    Modulo(Box<Nested<(StoredExpression, StoredExpression)>>),
    Power(Box<Nested<(StoredExpression, StoredExpression)>>),
    Min(Nested<Vec<StoredExpression>>),
    Max(Nested<Vec<StoredExpression>>),
    Abs(Box<Nested<StoredExpression>>),
    Aggregate(StoredAggregateRead),
}

/// [`ConstraintComparison`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredComparison {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

/// [`SizeMeasure`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredSizeMeasure {
    Length,
    ByteLength,
    Count,
}

/// [`SystemRead`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredSystemRead {
    value: StoredSystemValue,
    stored: bool,
}

/// [`SystemValue`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredSystemValue {
    Property(SystemProperty),
    Revision,
}

/// [`AggregateRead`].
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredAggregateRead {
    kind: StoredAggregateKind,
    document_type: String,
    filter: BTreeMap<String, StoredAggregateBinding>,
    of_own_type: bool,
}

/// [`AggregateKind`].
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredAggregateKind {
    Count,
    Sum { property: String },
}

/// [`AggregateBinding`].
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredAggregateBinding {
    Property {
        path: String,
        kind: Option<StoredEqualityKind>,
    },
    Owner,
    Id,
    Integer(i128),
    Constant(String),
}

/// [`EqualityKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredEqualityKind {
    Text,
    Identifier,
    Bytes,
}

/// [`TextProperty`].
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredTextProperty {
    path: String,
    if_absent: Option<String>,
}

/// [`AffixPosition`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredAffixPosition {
    Start,
    End,
}

/// [`TextOperand`].
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredTextOperand {
    Constant(String),
    Property(StoredTextProperty),
}

/// [`BytesOperand`].
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredBytesOperand {
    Constant(Vec<u8>),
    Property(String),
}

/// [`ContainsNeedle`].
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredContainsNeedle {
    Integer(StoredExpression),
    TextConstant(String),
    TextProperty(StoredTextProperty),
    IdentifierConstant(Identifier),
    IdentifierProperty(String),
}

impl From<&PropertyConstraint> for StoredRule {
    fn from(rule: &PropertyConstraint) -> Self {
        let nested = |rule: &PropertyConstraint| Box::new(Nested(StoredRule::from(rule)));
        let list =
            |rules: &[PropertyConstraint]| Nested(rules.iter().map(StoredRule::from).collect());
        match rule {
            PropertyConstraint::Compare {
                comparison,
                left,
                right,
            } => StoredRule::Compare {
                comparison: (*comparison).into(),
                left: left.into(),
                right: right.into(),
            },
            PropertyConstraint::In { operand, values } => StoredRule::In {
                operand: operand.into(),
                values: values.clone(),
            },
            PropertyConstraint::TextCompare {
                comparison,
                property,
                value,
            } => StoredRule::TextCompare {
                comparison: (*comparison).into(),
                property: property.into(),
                value: value.clone(),
            },
            PropertyConstraint::TextCompareProperties {
                comparison,
                left,
                right,
            } => StoredRule::TextCompareProperties {
                comparison: (*comparison).into(),
                left: left.into(),
                right: right.into(),
            },
            PropertyConstraint::TextIn { property, values } => StoredRule::TextIn {
                property: property.into(),
                values: values.clone(),
            },
            PropertyConstraint::IdentifierCompare {
                comparison,
                path,
                value,
            } => StoredRule::IdentifierCompare {
                comparison: (*comparison).into(),
                path: path.clone(),
                value: *value,
            },
            PropertyConstraint::IdentifierCompareProperties {
                comparison,
                left,
                right,
            } => StoredRule::IdentifierCompareProperties {
                comparison: (*comparison).into(),
                left: left.clone(),
                right: right.clone(),
            },
            PropertyConstraint::IdentifierIn { path, values } => StoredRule::IdentifierIn {
                path: path.clone(),
                values: values.clone(),
            },
            PropertyConstraint::TextAffix {
                position,
                text,
                affix,
            } => StoredRule::TextAffix {
                position: (*position).into(),
                text: text.into(),
                affix: affix.into(),
            },
            PropertyConstraint::BytesAffix {
                position,
                bytes,
                affix,
            } => StoredRule::BytesAffix {
                position: (*position).into(),
                bytes: bytes.into(),
                affix: affix.into(),
            },
            PropertyConstraint::Contains { array, needle } => StoredRule::Contains {
                array: array.clone(),
                needle: needle.into(),
            },
            PropertyConstraint::Present(path) => StoredRule::Present(path.clone()),
            PropertyConstraint::Absent(path) => StoredRule::Absent(path.clone()),
            PropertyConstraint::AnyOf(rules) => StoredRule::AnyOf(list(rules)),
            PropertyConstraint::AllOf(rules) => StoredRule::AllOf(list(rules)),
            PropertyConstraint::Not(rule) => StoredRule::Not(nested(rule)),
            PropertyConstraint::IfThen {
                condition,
                then,
                otherwise,
            } => StoredRule::IfThen {
                condition: nested(condition),
                then: nested(then),
                otherwise: otherwise.as_deref().map(nested),
            },
            PropertyConstraint::NotIn(rule) => StoredRule::NotIn(nested(rule)),
        }
    }
}

impl From<StoredRule> for PropertyConstraint {
    fn from(stored: StoredRule) -> Self {
        let nested = |rule: Box<Nested<StoredRule>>| Box::new(PropertyConstraint::from(rule.0));
        let list = |rules: Nested<Vec<StoredRule>>| {
            rules.0.into_iter().map(PropertyConstraint::from).collect()
        };
        match stored {
            StoredRule::Compare {
                comparison,
                left,
                right,
            } => PropertyConstraint::Compare {
                comparison: comparison.into(),
                left: left.into(),
                right: right.into(),
            },
            StoredRule::In { operand, values } => PropertyConstraint::In {
                operand: operand.into(),
                values,
            },
            StoredRule::TextCompare {
                comparison,
                property,
                value,
            } => PropertyConstraint::TextCompare {
                comparison: comparison.into(),
                property: property.into(),
                value,
            },
            StoredRule::TextCompareProperties {
                comparison,
                left,
                right,
            } => PropertyConstraint::TextCompareProperties {
                comparison: comparison.into(),
                left: left.into(),
                right: right.into(),
            },
            StoredRule::TextIn { property, values } => PropertyConstraint::TextIn {
                property: property.into(),
                values,
            },
            StoredRule::IdentifierCompare {
                comparison,
                path,
                value,
            } => PropertyConstraint::IdentifierCompare {
                comparison: comparison.into(),
                path,
                value,
            },
            StoredRule::IdentifierCompareProperties {
                comparison,
                left,
                right,
            } => PropertyConstraint::IdentifierCompareProperties {
                comparison: comparison.into(),
                left,
                right,
            },
            StoredRule::IdentifierIn { path, values } => {
                PropertyConstraint::IdentifierIn { path, values }
            }
            StoredRule::TextAffix {
                position,
                text,
                affix,
            } => PropertyConstraint::TextAffix {
                position: position.into(),
                text: text.into(),
                affix: affix.into(),
            },
            StoredRule::BytesAffix {
                position,
                bytes,
                affix,
            } => PropertyConstraint::BytesAffix {
                position: position.into(),
                bytes: bytes.into(),
                affix: affix.into(),
            },
            StoredRule::Contains { array, needle } => PropertyConstraint::Contains {
                array,
                needle: needle.into(),
            },
            StoredRule::Present(path) => PropertyConstraint::Present(path),
            StoredRule::Absent(path) => PropertyConstraint::Absent(path),
            StoredRule::AnyOf(rules) => PropertyConstraint::AnyOf(list(rules)),
            StoredRule::AllOf(rules) => PropertyConstraint::AllOf(list(rules)),
            StoredRule::Not(rule) => PropertyConstraint::Not(nested(rule)),
            StoredRule::IfThen {
                condition,
                then,
                otherwise,
            } => PropertyConstraint::IfThen {
                condition: nested(condition),
                then: nested(then),
                otherwise: otherwise.map(nested),
            },
            StoredRule::NotIn(rule) => PropertyConstraint::NotIn(nested(rule)),
        }
    }
}

impl From<&ConstraintExpression> for StoredExpression {
    fn from(expression: &ConstraintExpression) -> Self {
        let list = |operands: &[ConstraintExpression]| {
            Nested(operands.iter().map(StoredExpression::from).collect())
        };
        let pair = |left: &ConstraintExpression, right: &ConstraintExpression| {
            Box::new(Nested((left.into(), right.into())))
        };
        match expression {
            ConstraintExpression::Value(value) => StoredExpression::Value(*value),
            ConstraintExpression::Property { path, if_absent } => StoredExpression::Property {
                path: path.clone(),
                if_absent: *if_absent,
            },
            ConstraintExpression::Size { measure, path } => StoredExpression::Size {
                measure: match measure {
                    SizeMeasure::Length => StoredSizeMeasure::Length,
                    SizeMeasure::ByteLength => StoredSizeMeasure::ByteLength,
                    SizeMeasure::Count => StoredSizeMeasure::Count,
                },
                path: path.clone(),
            },
            ConstraintExpression::CountPresent(paths) => {
                StoredExpression::CountPresent(paths.clone())
            }
            ConstraintExpression::ByteAt { path, index } => StoredExpression::ByteAt {
                path: path.clone(),
                index: *index,
            },
            ConstraintExpression::System(read) => StoredExpression::System(StoredSystemRead {
                value: match read.value {
                    SystemValue::Property(property) => StoredSystemValue::Property(property),
                    SystemValue::Revision => StoredSystemValue::Revision,
                },
                stored: read.stored,
            }),
            ConstraintExpression::Add(operands) => StoredExpression::Add(list(operands)),
            ConstraintExpression::Multiply(operands) => StoredExpression::Multiply(list(operands)),
            ConstraintExpression::Subtract(left, right) => {
                StoredExpression::Subtract(pair(left, right))
            }
            ConstraintExpression::Divide(left, right) => {
                StoredExpression::Divide(pair(left, right))
            }
            ConstraintExpression::Modulo(left, right) => {
                StoredExpression::Modulo(pair(left, right))
            }
            ConstraintExpression::Power(left, right) => StoredExpression::Power(pair(left, right)),
            ConstraintExpression::Min(operands) => StoredExpression::Min(list(operands)),
            ConstraintExpression::Max(operands) => StoredExpression::Max(list(operands)),
            ConstraintExpression::Abs(operand) => {
                StoredExpression::Abs(Box::new(Nested(operand.as_ref().into())))
            }
            ConstraintExpression::Aggregate(read) => StoredExpression::Aggregate(read.into()),
        }
    }
}

impl From<StoredExpression> for ConstraintExpression {
    fn from(stored: StoredExpression) -> Self {
        let list = |operands: Nested<Vec<StoredExpression>>| {
            operands
                .0
                .into_iter()
                .map(ConstraintExpression::from)
                .collect()
        };
        let pair = |operands: Box<Nested<(StoredExpression, StoredExpression)>>| {
            let (left, right) = operands.0;
            (Box::new(left.into()), Box::new(right.into()))
        };
        match stored {
            StoredExpression::Value(value) => ConstraintExpression::Value(value),
            StoredExpression::Property { path, if_absent } => {
                ConstraintExpression::Property { path, if_absent }
            }
            StoredExpression::Size { measure, path } => ConstraintExpression::Size {
                measure: match measure {
                    StoredSizeMeasure::Length => SizeMeasure::Length,
                    StoredSizeMeasure::ByteLength => SizeMeasure::ByteLength,
                    StoredSizeMeasure::Count => SizeMeasure::Count,
                },
                path,
            },
            StoredExpression::CountPresent(paths) => ConstraintExpression::CountPresent(paths),
            StoredExpression::ByteAt { path, index } => {
                ConstraintExpression::ByteAt { path, index }
            }
            StoredExpression::System(read) => ConstraintExpression::System(SystemRead {
                value: match read.value {
                    StoredSystemValue::Property(property) => SystemValue::Property(property),
                    StoredSystemValue::Revision => SystemValue::Revision,
                },
                stored: read.stored,
            }),
            StoredExpression::Add(operands) => ConstraintExpression::Add(list(operands)),
            StoredExpression::Multiply(operands) => ConstraintExpression::Multiply(list(operands)),
            StoredExpression::Subtract(operands) => {
                let (left, right) = pair(operands);
                ConstraintExpression::Subtract(left, right)
            }
            StoredExpression::Divide(operands) => {
                let (left, right) = pair(operands);
                ConstraintExpression::Divide(left, right)
            }
            StoredExpression::Modulo(operands) => {
                let (left, right) = pair(operands);
                ConstraintExpression::Modulo(left, right)
            }
            StoredExpression::Power(operands) => {
                let (left, right) = pair(operands);
                ConstraintExpression::Power(left, right)
            }
            StoredExpression::Min(operands) => ConstraintExpression::Min(list(operands)),
            StoredExpression::Max(operands) => ConstraintExpression::Max(list(operands)),
            StoredExpression::Abs(operand) => ConstraintExpression::Abs(Box::new(operand.0.into())),
            StoredExpression::Aggregate(read) => ConstraintExpression::Aggregate(read.into()),
        }
    }
}

impl From<ConstraintComparison> for StoredComparison {
    fn from(comparison: ConstraintComparison) -> Self {
        match comparison {
            ConstraintComparison::Equal => StoredComparison::Equal,
            ConstraintComparison::NotEqual => StoredComparison::NotEqual,
            ConstraintComparison::LessThan => StoredComparison::LessThan,
            ConstraintComparison::LessThanOrEqual => StoredComparison::LessThanOrEqual,
            ConstraintComparison::GreaterThan => StoredComparison::GreaterThan,
            ConstraintComparison::GreaterThanOrEqual => StoredComparison::GreaterThanOrEqual,
        }
    }
}

impl From<StoredComparison> for ConstraintComparison {
    fn from(stored: StoredComparison) -> Self {
        match stored {
            StoredComparison::Equal => ConstraintComparison::Equal,
            StoredComparison::NotEqual => ConstraintComparison::NotEqual,
            StoredComparison::LessThan => ConstraintComparison::LessThan,
            StoredComparison::LessThanOrEqual => ConstraintComparison::LessThanOrEqual,
            StoredComparison::GreaterThan => ConstraintComparison::GreaterThan,
            StoredComparison::GreaterThanOrEqual => ConstraintComparison::GreaterThanOrEqual,
        }
    }
}

impl From<AffixPosition> for StoredAffixPosition {
    fn from(position: AffixPosition) -> Self {
        match position {
            AffixPosition::Start => StoredAffixPosition::Start,
            AffixPosition::End => StoredAffixPosition::End,
        }
    }
}

impl From<StoredAffixPosition> for AffixPosition {
    fn from(stored: StoredAffixPosition) -> Self {
        match stored {
            StoredAffixPosition::Start => AffixPosition::Start,
            StoredAffixPosition::End => AffixPosition::End,
        }
    }
}

impl From<&TextProperty> for StoredTextProperty {
    fn from(property: &TextProperty) -> Self {
        StoredTextProperty {
            path: property.path.clone(),
            if_absent: property.if_absent.clone(),
        }
    }
}

impl From<StoredTextProperty> for TextProperty {
    fn from(stored: StoredTextProperty) -> Self {
        TextProperty {
            path: stored.path,
            if_absent: stored.if_absent,
        }
    }
}

impl From<&TextOperand> for StoredTextOperand {
    fn from(operand: &TextOperand) -> Self {
        match operand {
            TextOperand::Constant(text) => StoredTextOperand::Constant(text.clone()),
            TextOperand::Property(property) => StoredTextOperand::Property(property.into()),
        }
    }
}

impl From<StoredTextOperand> for TextOperand {
    fn from(stored: StoredTextOperand) -> Self {
        match stored {
            StoredTextOperand::Constant(text) => TextOperand::Constant(text),
            StoredTextOperand::Property(property) => TextOperand::Property(property.into()),
        }
    }
}

impl From<&BytesOperand> for StoredBytesOperand {
    fn from(operand: &BytesOperand) -> Self {
        match operand {
            BytesOperand::Constant(bytes) => StoredBytesOperand::Constant(bytes.clone()),
            BytesOperand::Property(path) => StoredBytesOperand::Property(path.clone()),
        }
    }
}

impl From<StoredBytesOperand> for BytesOperand {
    fn from(stored: StoredBytesOperand) -> Self {
        match stored {
            StoredBytesOperand::Constant(bytes) => BytesOperand::Constant(bytes),
            StoredBytesOperand::Property(path) => BytesOperand::Property(path),
        }
    }
}

impl From<&ContainsNeedle> for StoredContainsNeedle {
    fn from(needle: &ContainsNeedle) -> Self {
        match needle {
            ContainsNeedle::Integer(expression) => StoredContainsNeedle::Integer(expression.into()),
            ContainsNeedle::TextConstant(text) => StoredContainsNeedle::TextConstant(text.clone()),
            ContainsNeedle::TextProperty(property) => {
                StoredContainsNeedle::TextProperty(property.into())
            }
            ContainsNeedle::IdentifierConstant(identifier) => {
                StoredContainsNeedle::IdentifierConstant(*identifier)
            }
            ContainsNeedle::IdentifierProperty(path) => {
                StoredContainsNeedle::IdentifierProperty(path.clone())
            }
        }
    }
}

impl From<StoredContainsNeedle> for ContainsNeedle {
    fn from(stored: StoredContainsNeedle) -> Self {
        match stored {
            StoredContainsNeedle::Integer(expression) => ContainsNeedle::Integer(expression.into()),
            StoredContainsNeedle::TextConstant(text) => ContainsNeedle::TextConstant(text),
            StoredContainsNeedle::TextProperty(property) => {
                ContainsNeedle::TextProperty(property.into())
            }
            StoredContainsNeedle::IdentifierConstant(identifier) => {
                ContainsNeedle::IdentifierConstant(identifier)
            }
            StoredContainsNeedle::IdentifierProperty(path) => {
                ContainsNeedle::IdentifierProperty(path)
            }
        }
    }
}

impl From<&AggregateRead> for StoredAggregateRead {
    fn from(read: &AggregateRead) -> Self {
        let AggregateRead {
            kind,
            document_type,
            filter,
            of_own_type,
        } = read;
        StoredAggregateRead {
            kind: match kind {
                AggregateKind::Count => StoredAggregateKind::Count,
                AggregateKind::Sum { property } => StoredAggregateKind::Sum {
                    property: property.clone(),
                },
            },
            document_type: document_type.clone(),
            filter: filter
                .iter()
                .map(|(name, binding)| {
                    (
                        name.clone(),
                        match binding {
                            AggregateBinding::Property { path, kind } => {
                                StoredAggregateBinding::Property {
                                    path: path.clone(),
                                    kind: kind.map(|kind| match kind {
                                        EqualityKind::Text => StoredEqualityKind::Text,
                                        EqualityKind::Identifier => StoredEqualityKind::Identifier,
                                        EqualityKind::Bytes => StoredEqualityKind::Bytes,
                                    }),
                                }
                            }
                            AggregateBinding::Owner => StoredAggregateBinding::Owner,
                            AggregateBinding::Id => StoredAggregateBinding::Id,
                            AggregateBinding::Integer(value) => {
                                StoredAggregateBinding::Integer(*value)
                            }
                            AggregateBinding::Constant(text) => {
                                StoredAggregateBinding::Constant(text.clone())
                            }
                        },
                    )
                })
                .collect(),
            of_own_type: *of_own_type,
        }
    }
}

impl From<StoredAggregateRead> for AggregateRead {
    fn from(stored: StoredAggregateRead) -> Self {
        let StoredAggregateRead {
            kind,
            document_type,
            filter,
            of_own_type,
        } = stored;
        AggregateRead {
            kind: match kind {
                StoredAggregateKind::Count => AggregateKind::Count,
                StoredAggregateKind::Sum { property } => AggregateKind::Sum { property },
            },
            document_type,
            filter: filter
                .into_iter()
                .map(|(name, binding)| {
                    (
                        name,
                        match binding {
                            StoredAggregateBinding::Property { path, kind } => {
                                AggregateBinding::Property {
                                    path,
                                    kind: kind.map(|kind| match kind {
                                        StoredEqualityKind::Text => EqualityKind::Text,
                                        StoredEqualityKind::Identifier => EqualityKind::Identifier,
                                        StoredEqualityKind::Bytes => EqualityKind::Bytes,
                                    }),
                                }
                            }
                            StoredAggregateBinding::Owner => AggregateBinding::Owner,
                            StoredAggregateBinding::Id => AggregateBinding::Id,
                            StoredAggregateBinding::Integer(value) => {
                                AggregateBinding::Integer(value)
                            }
                            StoredAggregateBinding::Constant(text) => {
                                AggregateBinding::Constant(text)
                            }
                        },
                    )
                })
                .collect(),
            of_own_type,
        }
    }
}
