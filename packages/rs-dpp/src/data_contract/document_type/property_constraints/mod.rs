//! The doctype-level `propertyConstraints` keyword (meta-schema v3, protocol
//! version 14): named rules every document of the type must meet, each a
//! condition on the document's properties: a comparison of two integer
//! expressions, a test of whether the document holds a property (`present`,
//! `absent`), or `anyOf`, `allOf` or `not` over conditions.
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
//!   }
//! }
//! ```
//!
//! An operand is an integer value, the dotted path of an integer property, or
//! an object with one key: an arithmetic operator over its operands, or
//! `ifAbsent`, a property with the value it takes when the document leaves it
//! out. A property named on its own takes 0 when absent. How the arithmetic
//! treats overflow, division and powers is set out on
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

#[cfg(test)]
mod tests;

use crate::consensus::basic::document::PropertyConstraintViolation;
use crate::data_contract::document_type::property_names;
use crate::data_contract::errors::DataContractError;
use platform_value::{Value, ValueMapHelper};
use std::collections::BTreeMap;
use std::fmt::Write;

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
const PRESENT: &str = "present";
const ABSENT: &str = "absent";

/// Every key an operand object may hold, for the errors.
const OPERAND_KEYS: &str = "add, subtract, multiply, divide, modulo, power or ifAbsent";

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

/// An integer expression, one side of a rule or an operand inside one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintExpression {
    /// An integer value.
    Value(i128),
    /// The value of the integer property at the dotted `path`, or `if_absent`
    /// when the document leaves it out: 0 for a path on its own, the declared
    /// value for an `ifAbsent` operand.
    Property { path: String, if_absent: i128 },
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
}

impl ConstraintExpression {
    /// The value of the expression for a document whose properties are `data`.
    ///
    /// Exact arithmetic over `i128`. Operands are evaluated left to right and
    /// the first fault is returned; every intermediate result must fit:
    ///
    /// * a property the document leaves out takes its `if_absent` value; one it
    ///   holds must be an integer ([`PropertyConstraintViolation::NotAnInteger`]
    ///   otherwise: the schema validation running first admits a float with no
    ///   fractional part as an integer, which the document could not be stored
    ///   with anyway) that fits an `i128`
    ///   ([`PropertyConstraintViolation::Overflow`] otherwise);
    /// * `add` and `multiply` fold their operands from the left, so an overflow
    ///   on the way is a fault even when a later operand would bring the result
    ///   back in range;
    /// * `divide` and `modulo` are Euclidean: the remainder is never negative
    ///   and the quotient is the one that goes with it (`-7` by `2` is `-4`
    ///   remainder `1`), which for operands that are not negative is ordinary
    ///   integer division. A divisor of 0 is a
    ///   [`PropertyConstraintViolation::DivisionByZero`];
    /// * `power` refuses a negative exponent
    ///   ([`PropertyConstraintViolation::NegativeExponent`]), which has no
    ///   integer result, and takes `0` to the power `0` as `1`.
    pub fn evaluate(&self, data: &Value) -> Result<i128, PropertyConstraintViolation> {
        match self {
            ConstraintExpression::Value(value) => Ok(*value),
            ConstraintExpression::Property { path, if_absent } => {
                property_value(data, path, *if_absent)
            }
            ConstraintExpression::Add(operands) => {
                operands.iter().try_fold(0i128, |sum, operand| {
                    sum.checked_add(operand.evaluate(data)?)
                        .ok_or(PropertyConstraintViolation::Overflow)
                })
            }
            ConstraintExpression::Multiply(operands) => {
                operands.iter().try_fold(1i128, |product, operand| {
                    product
                        .checked_mul(operand.evaluate(data)?)
                        .ok_or(PropertyConstraintViolation::Overflow)
                })
            }
            ConstraintExpression::Subtract(left, right) => {
                let (left, right) = (left.evaluate(data)?, right.evaluate(data)?);
                left.checked_sub(right)
                    .ok_or(PropertyConstraintViolation::Overflow)
            }
            ConstraintExpression::Divide(left, right) => {
                let (dividend, divisor) = (left.evaluate(data)?, right.evaluate(data)?);
                if divisor == 0 {
                    return Err(PropertyConstraintViolation::DivisionByZero);
                }
                // `i128::MIN` by -1 is the one quotient that does not fit
                dividend
                    .checked_div_euclid(divisor)
                    .ok_or(PropertyConstraintViolation::Overflow)
            }
            ConstraintExpression::Modulo(left, right) => {
                let (dividend, divisor) = (left.evaluate(data)?, right.evaluate(data)?);
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
                let (base, exponent) = (left.evaluate(data)?, right.evaluate(data)?);
                power(base, exponent)
            }
        }
    }

    /// The nodes of the expression: this one, and those of its operands.
    pub fn node_count(&self) -> usize {
        1 + match self {
            ConstraintExpression::Value(_) | ConstraintExpression::Property { .. } => 0,
            ConstraintExpression::Add(operands) | ConstraintExpression::Multiply(operands) => {
                operands.iter().map(ConstraintExpression::node_count).sum()
            }
            ConstraintExpression::Subtract(left, right)
            | ConstraintExpression::Divide(left, right)
            | ConstraintExpression::Modulo(left, right)
            | ConstraintExpression::Power(left, right) => left.node_count() + right.node_count(),
        }
    }

    /// Whether the expression reads at least one property.
    fn reads_property(&self) -> bool {
        match self {
            ConstraintExpression::Value(_) => false,
            ConstraintExpression::Property { .. } => true,
            ConstraintExpression::Add(operands) | ConstraintExpression::Multiply(operands) => {
                operands.iter().any(ConstraintExpression::reads_property)
            }
            ConstraintExpression::Subtract(left, right)
            | ConstraintExpression::Divide(left, right)
            | ConstraintExpression::Modulo(left, right)
            | ConstraintExpression::Power(left, right) => {
                left.reads_property() || right.reads_property()
            }
        }
    }

    /// Appends the properties the expression reads, each by its value, to
    /// `reads`, in the order it reads them.
    fn collect_property_reads<'a>(&'a self, reads: &mut Vec<(&'a str, PropertyRead)>) {
        match self {
            ConstraintExpression::Value(_) => {}
            ConstraintExpression::Property { path, .. } => reads.push((path, PropertyRead::Value)),
            ConstraintExpression::Add(operands) | ConstraintExpression::Multiply(operands) => {
                for operand in operands {
                    operand.collect_property_reads(reads);
                }
            }
            ConstraintExpression::Subtract(left, right)
            | ConstraintExpression::Divide(left, right)
            | ConstraintExpression::Modulo(left, right)
            | ConstraintExpression::Power(left, right) => {
                left.collect_property_reads(reads);
                right.collect_property_reads(reads);
            }
        }
    }
}

/// How a rule reads a property, which decides the properties it may name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyRead {
    /// By its value, as an operand: an integer property.
    Value,
    /// Only whether the document holds it, in a `present` or `absent`: a
    /// property of any type, an object included.
    Presence,
}

/// A rule of `propertyConstraints`, or a condition inside one: a comparison of
/// two integer expressions, a test of whether the document holds a property,
/// or `anyOf`, `allOf` or `not` over conditions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyConstraint {
    /// A comparison: the two sides must compare as `comparison` says.
    Compare {
        comparison: ConstraintComparison,
        left: ConstraintExpression,
        right: ConstraintExpression,
    },
    /// `present`: the document holds the property at the dotted path. One it
    /// leaves out, or sets to null, is absent, as it is for an operand. Unlike
    /// an operand, it tells a property left out from one set to 0, and it may
    /// name a property of any type.
    Present(String),
    /// `absent`: the document leaves the property at the dotted path out.
    Absent(String),
    /// `anyOf`: at least one of two or more conditions holds.
    AnyOf(Vec<PropertyConstraint>),
    /// `allOf`: every one of two or more conditions holds.
    AllOf(Vec<PropertyConstraint>),
    /// `not`: the condition does not hold.
    Not(Box<PropertyConstraint>),
}

impl PropertyConstraint {
    /// Whether a document whose properties are `data` meets the condition.
    ///
    /// Evaluated left to right, and no further than the outcome needs: a
    /// comparison evaluates its left side, then its right one; `anyOf` checks
    /// its conditions in declared order and holds at the first that holds;
    /// `allOf` fails at the first that fails; `not` inverts its condition; a
    /// `present` or `absent` never faults. The first fault an evaluated
    /// expression meets ([`ConstraintExpression::evaluate`]) is returned
    /// whatever the conditions left unevaluated would say, and `not` never
    /// turns a fault into a pass. So an earlier condition guards a later
    /// one: `anyOf: [{ equal: ["b", 0] }, { equal: [{ divide: ["a", "b"] }, 2] }]`
    /// holds for a `b` of 0 without dividing by it, while the same two
    /// conditions the other way round divide by zero.
    pub fn holds(&self, data: &Value) -> Result<bool, PropertyConstraintViolation> {
        match self {
            PropertyConstraint::Compare {
                comparison,
                left,
                right,
            } => {
                let (left, right) = (left.evaluate(data)?, right.evaluate(data)?);
                Ok(comparison.holds(left, right))
            }
            PropertyConstraint::Present(path) => Ok(is_present(data, path)),
            PropertyConstraint::Absent(path) => Ok(!is_present(data, path)),
            PropertyConstraint::AnyOf(conditions) => {
                for condition in conditions {
                    if condition.holds(data)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            PropertyConstraint::AllOf(conditions) => {
                for condition in conditions {
                    if !condition.holds(data)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            PropertyConstraint::Not(condition) => Ok(!condition.holds(data)?),
        }
    }

    /// Why a document whose properties are `data` breaks the rule, `None` when
    /// it meets it: the first fault met on the way ([`Self::holds`]), or
    /// [`PropertyConstraintViolation::NotMet`] when the rule evaluates to false.
    pub fn violation(&self, data: &Value) -> Option<PropertyConstraintViolation> {
        match self.holds(data) {
            Ok(true) => None,
            Ok(false) => Some(PropertyConstraintViolation::NotMet),
            Err(violation) => Some(violation),
        }
    }

    /// The nodes of the rule, counted against
    /// `SystemLimits::max_property_constraint_nodes`: every comparison and
    /// logical operator, every `present` or `absent` with the property it
    /// names, every arithmetic operator and every operand (an integer value, or
    /// a property with or without `ifAbsent`).
    pub fn node_count(&self) -> usize {
        1 + match self {
            PropertyConstraint::Compare { left, right, .. } => {
                left.node_count() + right.node_count()
            }
            PropertyConstraint::Present(_) | PropertyConstraint::Absent(_) => 0,
            PropertyConstraint::AnyOf(conditions) | PropertyConstraint::AllOf(conditions) => {
                conditions.iter().map(PropertyConstraint::node_count).sum()
            }
            PropertyConstraint::Not(condition) => condition.node_count(),
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
        self.collect_property_reads(&mut reads);
        reads
    }

    /// Where an `anyOf` or `allOf` of the rule lists the same condition twice:
    /// the repeat's place and the earlier one's (`anyOf[2]` and `anyOf[0]`), the
    /// first found in declared order, `None` when no list does. Conditions are
    /// alike when they parse alike, so `1` and `1.0` are the same value, and so
    /// are `"price"` and `{ "ifAbsent": ["price", 0] }`. Checked under full
    /// validation with the limits, which bound the lists it compares; a stored
    /// rule was checked when its contract registered.
    pub fn repeated_condition(&self) -> Option<(String, String)> {
        self.find_repeated_condition(&mut String::new())
    }

    /// [`Self::repeated_condition`] for the condition at `at` (empty for the
    /// rule's own), which is extended as the walk descends and trimmed back
    /// when it returns `None`.
    fn find_repeated_condition(&self, at: &mut String) -> Option<(String, String)> {
        let (key, conditions) = match self {
            PropertyConstraint::Compare { .. }
            | PropertyConstraint::Present(_)
            | PropertyConstraint::Absent(_) => return None,
            PropertyConstraint::AnyOf(conditions) => (ANY_OF, conditions),
            PropertyConstraint::AllOf(conditions) => (ALL_OF, conditions),
            PropertyConstraint::Not(condition) => {
                let parent = enter(at, NOT);
                let found = condition.find_repeated_condition(at);
                at.truncate(parent);
                return found;
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

    fn collect_property_reads<'a>(&'a self, reads: &mut Vec<(&'a str, PropertyRead)>) {
        match self {
            PropertyConstraint::Compare { left, right, .. } => {
                left.collect_property_reads(reads);
                right.collect_property_reads(reads);
            }
            PropertyConstraint::Present(path) | PropertyConstraint::Absent(path) => {
                reads.push((path, PropertyRead::Presence))
            }
            PropertyConstraint::AnyOf(conditions) | PropertyConstraint::AllOf(conditions) => {
                for condition in conditions {
                    condition.collect_property_reads(reads);
                }
            }
            PropertyConstraint::Not(condition) => condition.collect_property_reads(reads),
        }
    }
}

/// Reads the `propertyConstraints` keyword of a document type's `schema`:
/// every rule by its name, in name order, the order a document is checked
/// against them. Empty when the schema declares none.
///
/// The rules of the declaration's shape are checked here, on every parse: an
/// object of one or more rules, each named with 1 to 64 letters, digits or
/// underscores and holding one condition. A condition is an object with one
/// key: a comparison of exactly two operands, `present` or `absent` with a
/// property path, `anyOf` or `allOf` with two or more conditions, none of them directly the same operator (it says what one
/// flat list says), or `not` with one condition that is not directly another
/// `not`. An operand is an integer value, a property path, or
/// an object with one key: `ifAbsent` with a path and an integer value, `add`
/// or `multiply` with two or more operands, or `subtract`, `divide`, `modulo`
/// or `power` with exactly two. An integer value may be spelled as a float with
/// no fractional part, as the meta-schema's `integer` type admits one. A
/// literal 0 divisor, a literal negative exponent, a comparison that reads no
/// property, which would hold for every document or for none, and a condition
/// or operand deeper than [`MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH`] are refused.
/// What the paths name is checked against the parsed document type, and the
/// limits and that no list repeats a condition under full validation, by
/// parser generation 3.
pub fn parse_property_constraints(
    schema: &Value,
    document_type_name: &str,
) -> Result<BTreeMap<String, PropertyConstraint>, DataContractError> {
    let structure_error = |message: String| {
        DataContractError::InvalidContractStructure(format!(
            "document type \"{document_type_name}\" propertyConstraints {message}"
        ))
    };
    // A schema that is not an object carries no keyword: the core parser
    // refuses it, and a value error here must not replace that refusal
    let Ok(schema_map) = schema.to_map() else {
        return Ok(BTreeMap::new());
    };
    let Some(declaration) = schema_map.get_optional_key(property_names::PROPERTY_CONSTRAINTS)
    else {
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
        let constraint = parse_condition(rule, &mut String::new(), 0)
            .map_err(|message| structure_error(format!("rule \"{name}\" {message}")))?;
        if constraints.insert(name.to_string(), constraint).is_some() {
            return Err(structure_error(format!("declares rule \"{name}\" twice")));
        }
    }
    Ok(constraints)
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
        "a comparison ({}), present, absent, anyOf, allOf or not",
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

/// A condition at `at` (`anyOf[1]`, empty for the rule's own), where the
/// errors place it, `depth` levels into its rule: an object whose one key is a
/// comparison listing its two sides, `present` or `absent` naming a property,
/// or `anyOf`, `allOf` or `not`. The error is
/// the rest of a message naming the rule. `at` is extended for what the
/// condition holds and trimmed back before a successful return.
fn parse_condition(
    value: &Value,
    at: &mut String,
    depth: usize,
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
    let condition = match key {
        ANY_OF => PropertyConstraint::AnyOf(condition_list(body, key, at, depth + 1)?),
        ALL_OF => PropertyConstraint::AllOf(condition_list(body, key, at, depth + 1)?),
        NOT => {
            if single_entry(body).is_some_and(|(inner, _)| inner == NOT) {
                return Err(format!(
                    "at {at}.{NOT} is a not directly inside a not, which says what the \
                     condition inside it says: declare that condition"
                ));
            }
            PropertyConstraint::Not(Box::new(parse_condition(body, at, depth + 1)?))
        }
        // What the path names is checked against the parsed document type
        PRESENT | ABSENT => {
            let Some(path) = body.as_text() else {
                return Err(format!("at {at} must name a property path"));
            };
            if key == PRESENT {
                PropertyConstraint::Present(path.to_string())
            } else {
                PropertyConstraint::Absent(path.to_string())
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
            let (left, right) = operand_pair(body, at, depth + 1)?;
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
        parsed.push(parse_condition(value, at, depth)?);
        at.truncate(base);
    }
    Ok(parsed)
}

/// An operand at `at` (`lessThan[0].add[1]`), where the errors place it,
/// `depth` levels into its rule. `at` is extended for the operands of an
/// operator and trimmed back before a successful return.
fn parse_expression(
    value: &Value,
    at: &mut String,
    depth: usize,
) -> Result<ConstraintExpression, String> {
    if depth > MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH {
        return Err(format!(
            "at {at} nests deeper than {MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH} levels"
        ));
    }
    if let Some(path) = value.as_text() {
        return Ok(ConstraintExpression::Property {
            path: path.to_string(),
            if_absent: 0,
        });
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
            let Some(path) = path.as_text() else {
                return Err(format!("at {at} must name a property path first"));
            };
            if !is_number(if_absent) {
                return Err(format!("at {at} must give an integer value second"));
            }
            at.push_str("[1]");
            ConstraintExpression::Property {
                path: path.to_string(),
                if_absent: integer_value(if_absent, at)?,
            }
        }
        ADD => ConstraintExpression::Add(operand_list(operands, at, depth + 1)?),
        MULTIPLY => ConstraintExpression::Multiply(operand_list(operands, at, depth + 1)?),
        SUBTRACT => {
            let (left, right) = operand_pair(operands, at, depth + 1)?;
            ConstraintExpression::Subtract(Box::new(left), Box::new(right))
        }
        DIVIDE | MODULO => {
            let (dividend, divisor) = operand_pair(operands, at, depth + 1)?;
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
            let (base, exponent) = operand_pair(operands, at, depth + 1)?;
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

/// The exactly two operands listed at `at`, `depth` levels into their rule.
fn operand_pair(
    operands: &Value,
    at: &mut String,
    depth: usize,
) -> Result<(ConstraintExpression, ConstraintExpression), String> {
    let Some([left, right]) = operands.as_array().map(Vec::as_slice) else {
        return Err(format!("at {at} must list exactly two operands"));
    };
    let base = at.len();
    at.push_str("[0]");
    let left = parse_expression(left, at, depth)?;
    at.truncate(base);
    at.push_str("[1]");
    let right = parse_expression(right, at, depth)?;
    at.truncate(base);
    Ok((left, right))
}

/// The two or more operands listed at `at`, `depth` levels into their rule.
fn operand_list(
    operands: &Value,
    at: &mut String,
    depth: usize,
) -> Result<Vec<ConstraintExpression>, String> {
    let Some(values) = operands.as_array().filter(|values| values.len() >= 2) else {
        return Err(format!("at {at} must list two or more operands"));
    };
    let base = at.len();
    let mut expressions = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        // Writing to a `String` cannot fail
        let _ = write!(at, "[{index}]");
        expressions.push(parse_expression(value, at, depth)?);
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

/// An integer literal at `at`: an integer, or a float with no fractional part,
/// in the range of an `i128`.
fn integer_value(value: &Value, at: &str) -> Result<i128, String> {
    let integer = match value {
        // The cast is exact: the float is a whole number inside the range
        Value::Float(float)
            if float.fract() == 0.0 && *float >= i128::MIN as f64 && *float < i128::MAX as f64 =>
        {
            Some(*float as i128)
        }
        Value::Float(_) => None,
        _ => value.to_integer::<i128>().ok(),
    };
    integer.ok_or_else(|| {
        format!(
            "at {at} holds {}, which is not an integer in the range of a 128-bit signed \
             integer",
            value.non_qualified_string_representation()
        )
    })
}

/// Whether `data` holds the property at `path`: absent exactly where
/// [`property_value`] would take the `if_absent` value.
fn is_present(data: &Value, path: &str) -> bool {
    matches!(
        data.get_optional_value_at_path(path),
        Ok(Some(value)) if !matches!(value, Value::Null)
    )
}

/// The value of the property at `path` in `data`, or `if_absent` when the
/// document leaves it out. An intermediate that is not an object reads as
/// absent: the schema validation that runs first refuses such a document.
fn property_value(
    data: &Value,
    path: &str,
    if_absent: i128,
) -> Result<i128, PropertyConstraintViolation> {
    match data.get_optional_value_at_path(path) {
        Ok(Some(Value::Null)) | Ok(None) | Err(_) => Ok(if_absent),
        Ok(Some(value)) if value.is_integer() => value
            .to_integer::<i128>()
            .map_err(|_| PropertyConstraintViolation::Overflow),
        Ok(Some(_)) => Err(PropertyConstraintViolation::NotAnInteger),
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
