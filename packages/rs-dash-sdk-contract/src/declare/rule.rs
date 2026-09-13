//! Rules on ordinary document actions: native bounded guards and read-only
//! WASM predicates.
//!
//! [`ActionScope`] lists the ordinary document actions only. A contested-index
//! award is an internal native action that no guest rule, guard or predicate
//! can scope, veto, delay, redirect or retry; it is therefore unrepresentable
//! here, and the grammar rejects `on = "award"` as an invalid option value.
//! Ordinary action rules still apply to ordinary writes on a collection that
//! also declares a contested index.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use super::DeclarationOrigin;
use crate::identity::{CollectionName, ModuleName, PropertyPath, RuleName};

/// An ordinary document action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActionScope {
    /// Document creation.
    Create,
    /// Document replacement.
    Replace,
    /// Document deletion.
    Delete,
    /// Document transfer.
    Transfer,
    /// Document purchase.
    Purchase,
    /// Price update.
    UpdatePrice,
}

impl ActionScope {
    /// Every ordinary action, in canonical order.
    pub const ALL: &'static [ActionScope] = &[
        ActionScope::Create,
        ActionScope::Replace,
        ActionScope::Delete,
        ActionScope::Transfer,
        ActionScope::Purchase,
        ActionScope::UpdatePrice,
    ];

    /// The grammar spelling.
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionScope::Create => "create",
            ActionScope::Replace => "replace",
            ActionScope::Delete => "delete",
            ActionScope::Transfer => "transfer",
            ActionScope::Purchase => "purchase",
            ActionScope::UpdatePrice => "update_price",
        }
    }
}

impl core::fmt::Display for ActionScope {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Which document a guard field reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FieldContext {
    /// The target immediately before the action.
    Old,
    /// The target as the action would leave it.
    New,
    /// The authenticated action context.
    Context,
}

/// A guard literal.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Literal {
    /// A boolean.
    Bool(bool),
    /// An integer, typed by the compared field's declared width.
    Integer(i128),
    /// A string.
    Text(String),
    /// Bytes.
    Bytes(Vec<u8>),
}

/// The author-facing native guard expression.
///
/// Mirrors the proposed bounded guard node set: no loops, recursion, dynamic
/// traversal or ambient time. Provisional: the canonical guard AST and its
/// evaluator are specified by the guards work; the build tooling translates
/// this form into it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GuardExpr {
    /// A literal.
    Literal(Literal),
    /// A field of the old, new or context document.
    Field(FieldContext, PropertyPath),
    /// Whether the field is present.
    Exists(FieldContext, PropertyPath),
    /// Whether the field is present and null.
    IsNull(FieldContext, PropertyPath),
    /// Equality of two typed operands.
    Eq(Box<GuardExpr>, Box<GuardExpr>),
    /// Inequality.
    Ne(Box<GuardExpr>, Box<GuardExpr>),
    /// Less than.
    Lt(Box<GuardExpr>, Box<GuardExpr>),
    /// Less than or equal.
    Le(Box<GuardExpr>, Box<GuardExpr>),
    /// Greater than.
    Gt(Box<GuardExpr>, Box<GuardExpr>),
    /// Greater than or equal.
    Ge(Box<GuardExpr>, Box<GuardExpr>),
    /// Checked addition.
    Add(Box<GuardExpr>, Box<GuardExpr>),
    /// Checked subtraction.
    Sub(Box<GuardExpr>, Box<GuardExpr>),
    /// Checked multiplication.
    Mul(Box<GuardExpr>, Box<GuardExpr>),
    /// Short-circuit conjunction.
    And(Box<GuardExpr>, Box<GuardExpr>),
    /// Short-circuit disjunction.
    Or(Box<GuardExpr>, Box<GuardExpr>),
    /// Negation.
    Not(Box<GuardExpr>),
    /// Conditional; one branch is evaluated.
    If {
        /// The condition.
        condition: Box<GuardExpr>,
        /// Evaluated when the condition holds.
        then: Box<GuardExpr>,
        /// Evaluated otherwise.
        otherwise: Box<GuardExpr>,
    },
}

impl GuardExpr {
    /// An integer literal.
    pub fn integer(value: i128) -> Self {
        GuardExpr::Literal(Literal::Integer(value))
    }

    /// A boolean literal.
    pub fn boolean(value: bool) -> Self {
        GuardExpr::Literal(Literal::Bool(value))
    }

    /// A field read.
    pub fn field(context: FieldContext, path: PropertyPath) -> Self {
        GuardExpr::Field(context, path)
    }

    /// `self == other`
    pub fn eq(self, other: GuardExpr) -> Self {
        GuardExpr::Eq(Box::new(self), Box::new(other))
    }

    /// `self != other`
    pub fn ne(self, other: GuardExpr) -> Self {
        GuardExpr::Ne(Box::new(self), Box::new(other))
    }

    /// `self < other`
    pub fn lt(self, other: GuardExpr) -> Self {
        GuardExpr::Lt(Box::new(self), Box::new(other))
    }

    /// `self <= other`
    pub fn le(self, other: GuardExpr) -> Self {
        GuardExpr::Le(Box::new(self), Box::new(other))
    }

    /// `self > other`
    pub fn gt(self, other: GuardExpr) -> Self {
        GuardExpr::Gt(Box::new(self), Box::new(other))
    }

    /// `self >= other`
    pub fn ge(self, other: GuardExpr) -> Self {
        GuardExpr::Ge(Box::new(self), Box::new(other))
    }

    /// `self + other`, checked.
    pub fn plus(self, other: GuardExpr) -> Self {
        GuardExpr::Add(Box::new(self), Box::new(other))
    }

    /// `self - other`, checked.
    pub fn minus(self, other: GuardExpr) -> Self {
        GuardExpr::Sub(Box::new(self), Box::new(other))
    }

    /// `self * other`, checked.
    pub fn times(self, other: GuardExpr) -> Self {
        GuardExpr::Mul(Box::new(self), Box::new(other))
    }

    /// `self && other`
    pub fn and(self, other: GuardExpr) -> Self {
        GuardExpr::And(Box::new(self), Box::new(other))
    }

    /// `self || other`
    pub fn or(self, other: GuardExpr) -> Self {
        GuardExpr::Or(Box::new(self), Box::new(other))
    }

    /// `!self`
    pub fn negate(self) -> Self {
        GuardExpr::Not(Box::new(self))
    }

    /// `if self { then } else { otherwise }`
    pub fn if_else(self, then: GuardExpr, otherwise: GuardExpr) -> Self {
        GuardExpr::If {
            condition: Box::new(self),
            then: Box::new(then),
            otherwise: Box::new(otherwise),
        }
    }

    /// Every field path the expression reads, with its context.
    pub fn field_references(&self) -> Vec<(FieldContext, &PropertyPath)> {
        let mut references = Vec::new();
        self.collect_field_references(&mut references);
        references
    }

    fn collect_field_references<'a>(&'a self, into: &mut Vec<(FieldContext, &'a PropertyPath)>) {
        match self {
            GuardExpr::Literal(_) => {}
            GuardExpr::Field(context, path)
            | GuardExpr::Exists(context, path)
            | GuardExpr::IsNull(context, path) => into.push((*context, path)),
            GuardExpr::Eq(a, b)
            | GuardExpr::Ne(a, b)
            | GuardExpr::Lt(a, b)
            | GuardExpr::Le(a, b)
            | GuardExpr::Gt(a, b)
            | GuardExpr::Ge(a, b)
            | GuardExpr::Add(a, b)
            | GuardExpr::Sub(a, b)
            | GuardExpr::Mul(a, b)
            | GuardExpr::And(a, b)
            | GuardExpr::Or(a, b) => {
                a.collect_field_references(into);
                b.collect_field_references(into);
            }
            GuardExpr::Not(a) => a.collect_field_references(into),
            GuardExpr::If {
                condition,
                then,
                otherwise,
            } => {
                condition.collect_field_references(into);
                then.collect_field_references(into);
                otherwise.collect_field_references(into);
            }
        }
    }
}

/// How a rule decides.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RuleKind {
    /// A native bounded guard expression, evaluated by the host.
    NativeGuard(GuardExpr),
    /// A read-only predicate exported by a WASM module of this contract.
    WasmPredicate {
        /// The module exporting the predicate.
        module: ModuleName,
        /// The export symbol, bound against the module's actual exports at
        /// build time.
        export: String,
    },
}

/// A rule on ordinary actions of one collection. Its identity is
/// `(collection, name)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleSpec {
    /// Where the spec came from.
    pub origin: DeclarationOrigin,
    /// The rule's name, unique within the collection.
    pub name: RuleName,
    /// The guarded collection.
    pub collection: CollectionName,
    /// The guarded actions; stored as a sorted set in the manifest.
    pub actions: Vec<ActionScope>,
    /// How the rule decides.
    pub kind: RuleKind,
}

impl RuleSpec {
    /// A native guard rule.
    pub fn guard(
        collection: CollectionName,
        name: RuleName,
        actions: Vec<ActionScope>,
        guard: GuardExpr,
    ) -> Self {
        RuleSpec {
            origin: DeclarationOrigin::Builder,
            name,
            collection,
            actions,
            kind: RuleKind::NativeGuard(guard),
        }
    }

    /// A WASM predicate rule.
    pub fn predicate(
        collection: CollectionName,
        name: RuleName,
        actions: Vec<ActionScope>,
        module: ModuleName,
        export: impl Into<String>,
    ) -> Self {
        RuleSpec {
            origin: DeclarationOrigin::Builder,
            name,
            collection,
            actions,
            kind: RuleKind::WasmPredicate {
                module,
                export: export.into(),
            },
        }
    }

    /// Records the origin.
    pub fn with_origin(mut self, origin: DeclarationOrigin) -> Self {
        self.origin = origin;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_have_no_award_action() {
        assert!(ActionScope::ALL
            .iter()
            .all(|action| action.as_str() != "award"));
        assert_eq!(ActionScope::ALL.len(), 6);
    }

    #[test]
    fn should_collect_every_field_reference_of_a_guard() {
        let points = PropertyPath::new("points").unwrap();
        let guard = GuardExpr::field(FieldContext::New, points.clone())
            .ge(GuardExpr::field(FieldContext::Old, points.clone()))
            .and(
                GuardExpr::Exists(FieldContext::Old, points.clone())
                    .negate()
                    .if_else(
                        GuardExpr::boolean(true),
                        GuardExpr::integer(1).lt(GuardExpr::integer(2)),
                    ),
            );
        let references = guard.field_references();
        assert_eq!(references.len(), 3);
        assert_eq!(references[0], (FieldContext::New, &points));
        assert_eq!(references[2], (FieldContext::Old, &points));
    }
}
