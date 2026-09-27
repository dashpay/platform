//! `propertyConstraints` rules: the conditions a document type holds every
//! created or replaced document's properties to, from protocol version 14
//! onward.
//!
//! A rule is a condition: a comparison of integer expressions, a membership
//! test (`in`), a comparison of a string or an identifier property (or
//! `$ownerId`, the document's owner) with constants or with another property
//! of its kind, a presence test (`present`, `absent`), or `anyOf`, `allOf` or
//! `not` over conditions. Consensus evaluates every rule on each create and
//! replace, and the rules reading `$ownerId` on each transfer and purchase,
//! refusing a broken one with `DocumentPropertyConstraintViolatedError`
//! (basic code 10422). What this module adds is *discovery* ("which rules does
//! this document type declare, and what do they read?") and a *pre-check*
//! that evaluates a document against them with the very code consensus runs,
//! so an app can find a broken rule before paying for a refused transition.

use crate::error::{WasmDppError, WasmDppResult};
use crate::serialization::conversions::platform_value_to_json;
use dpp::consensus::basic::document::PropertyConstraintViolation;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::property_constraints::PropertyRead;
use dpp::document::{Document, DocumentV0Getters};
use dpp::platform_value::Value;
use js_sys::{Array, Object, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_PROPERTY_CONSTRAINTS_TS: &'static str = r#"
/**
 * An integer expression of a `propertyConstraints` rule.
 *
 * - a number: an integer value;
 * - a string: the dotted path of an integer or boolean property, whose value
 *   it takes (a boolean reads as 1 for true and 0 for false), 0 when the
 *   document leaves the property out;
 * - `ifAbsent`: a property path and the integer it takes when left out;
 * - `add` and `multiply` over two or more operands, `subtract`, `divide`,
 *   `modulo` and `power` over exactly two. Arithmetic is exact over 128-bit
 *   integers; `divide` and `modulo` are Euclidean.
 */
export type PropertyConstraintExpression =
  | number
  | string
  | { ifAbsent: [path: string, value: number] }
  | { add: PropertyConstraintExpression[] }
  | { multiply: PropertyConstraintExpression[] }
  | { subtract: [PropertyConstraintExpression, PropertyConstraintExpression] }
  | { divide: [PropertyConstraintExpression, PropertyConstraintExpression] }
  | { modulo: [PropertyConstraintExpression, PropertyConstraintExpression] }
  | { power: [PropertyConstraintExpression, PropertyConstraintExpression] };

/**
 * One side of a comparison of strings or identifiers.
 *
 * - a string: the dotted path of a string or an identifier property, or
 *   `"$ownerId"`, the document's owner, an identifier;
 * - `const`: a string constant, or a base58 identifier beside an identifier
 *   property or `$ownerId`;
 * - `ifAbsent`: a string property with the string it takes when left out.
 *
 * A property the document leaves out without a default equals nothing.
 */
export type PropertyConstraintEqualityOperand =
  | string
  | { const: string }
  | { ifAbsent: [path: string, value: string] };

/**
 * A `propertyConstraints` rule, or a condition inside one.
 *
 * - a comparison of two integer expressions; `equal` and `notEqual` also
 *   compare strings or identifiers, which are never ordered;
 * - `in`: an integer expression and two or more distinct integers, or a
 *   string or identifier property (or `$ownerId`) and two or more distinct
 *   strings or base58 identifiers;
 * - `present` / `absent`: whether the document holds a property of any type;
 * - `anyOf` / `allOf` over two or more conditions, `not` over one. Conditions
 *   are checked in order and no further than the outcome needs.
 */
export type PropertyConstraintCondition =
  | { equal: [PropertyConstraintExpression, PropertyConstraintExpression] | [PropertyConstraintEqualityOperand, PropertyConstraintEqualityOperand] }
  | { notEqual: [PropertyConstraintExpression, PropertyConstraintExpression] | [PropertyConstraintEqualityOperand, PropertyConstraintEqualityOperand] }
  | { lessThan: [PropertyConstraintExpression, PropertyConstraintExpression] }
  | { lessThanOrEqual: [PropertyConstraintExpression, PropertyConstraintExpression] }
  | { greaterThan: [PropertyConstraintExpression, PropertyConstraintExpression] }
  | { greaterThanOrEqual: [PropertyConstraintExpression, PropertyConstraintExpression] }
  | { in: [PropertyConstraintExpression, number[]] | [string | { ifAbsent: [path: string, value: string] }, string[]] }
  | { present: string }
  | { absent: string }
  | { anyOf: PropertyConstraintCondition[] }
  | { allOf: PropertyConstraintCondition[] }
  | { not: PropertyConstraintCondition };

/**
 * How a rule reads a property: `value` as an integer operand, `presence` in
 * `present` or `absent`, `text` compared with strings, `identifier` compared
 * with identifiers.
 */
export type PropertyConstraintReadKind = 'value' | 'presence' | 'text' | 'identifier';

/**
 * A single `propertyConstraints` rule of a document type.
 */
export type DocumentPropertyConstraint = {
  /** The rule's name, its key in `propertyConstraints`. Rules are checked in name order. */
  name: string;
  /** The rule as the schema declares it. */
  rule: PropertyConstraintCondition;
  /** Every property the rule reads, in declared order; `$ownerId` is no property and is not listed. */
  reads: Array<{ path: string; kind: PropertyConstraintReadKind }>;
  /** Whether the rule reads `$ownerId`: then a transfer or a purchase is judged against it too. */
  readsOwner: boolean;
};

/**
 * Why a document breaks a rule, the `violation` consensus reports in
 * `DocumentPropertyConstraintViolatedError` (code 10422): `NotMet` when the
 * rule evaluates to false, or the fault met evaluating it.
 */
export type PropertyConstraintViolationKind =
  | 'NotMet'
  | 'Overflow'
  | 'DivisionByZero'
  | 'NegativeExponent'
  | 'NotAnInteger';

/**
 * The first rule a document breaks, as consensus would report it.
 */
export type DocumentPropertyConstraintViolation = {
  /** The broken rule's name. */
  rule: string;
  violation: PropertyConstraintViolationKind;
  /** A readable reason, as in the consensus error's message. */
  message: string;
};
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Array<DocumentPropertyConstraint>")]
    pub type DocumentPropertyConstraintArrayJs;

    #[wasm_bindgen(typescript_type = "Map<string, Array<DocumentPropertyConstraint>>")]
    pub type DocumentPropertyConstraintMapJs;

    #[wasm_bindgen(typescript_type = "DocumentPropertyConstraintViolation | undefined")]
    pub type DocumentPropertyConstraintViolationJs;
}

/// `Reflect::set` with the collection-getter error convention the other
/// document type accessors use.
fn set_field(target: &Object, key: &str, value: &JsValue, rule: &str) -> WasmDppResult<()> {
    Reflect::set(target, &JsValue::from_str(key), value).map_err(|_| {
        WasmDppError::generic(format!(
            "unable to serialize the `{key}` field of the propertyConstraints rule '{rule}'"
        ))
    })?;
    Ok(())
}

/// The name a read kind goes by in `PropertyConstraintReadKind`.
fn read_kind_name(read: PropertyRead) -> &'static str {
    match read {
        PropertyRead::Value => "value",
        PropertyRead::Presence => "presence",
        PropertyRead::Text => "text",
        PropertyRead::Identifier => "identifier",
    }
}

/// The name a violation goes by in `PropertyConstraintViolationKind`, the
/// variant's own.
fn violation_name(violation: PropertyConstraintViolation) -> &'static str {
    match violation {
        PropertyConstraintViolation::NotMet => "NotMet",
        PropertyConstraintViolation::Overflow => "Overflow",
        PropertyConstraintViolation::DivisionByZero => "DivisionByZero",
        PropertyConstraintViolation::NegativeExponent => "NegativeExponent",
        PropertyConstraintViolation::NotAnInteger => "NotAnInteger",
    }
}

/// Collect every `propertyConstraints` rule of one document type, in name
/// order, the order consensus checks them in.
///
/// The parsed rules give the name, the reads and whether the owner is read;
/// the rule itself is the schema's declaration, which is what an app wrote
/// and what `toJSON()` shows.
pub(crate) fn property_constraints_for_document_type(
    document_type: DocumentTypeRef<'_>,
) -> WasmDppResult<Array> {
    let rules = Array::new();
    let declarations = document_type
        .schema()
        .get_optional_value("propertyConstraints")
        .ok()
        .flatten();

    for (name, constraint) in document_type.property_constraints() {
        let object = Object::new();
        set_field(&object, "name", &JsValue::from_str(name), name)?;

        let declared = declarations
            .and_then(|declarations| declarations.get_optional_value(name).ok().flatten())
            .ok_or_else(|| {
                WasmDppError::generic(format!(
                    "the propertyConstraints rule '{name}' is missing from the document type's \
                     schema"
                ))
            })?;
        set_field(&object, "rule", &platform_value_to_json(declared)?, name)?;

        let reads = Array::new();
        for (path, read) in constraint.property_reads() {
            let entry = Object::new();
            set_field(&entry, "path", &JsValue::from_str(path), name)?;
            set_field(
                &entry,
                "kind",
                &JsValue::from_str(read_kind_name(read)),
                name,
            )?;
            reads.push(&entry);
        }
        set_field(&object, "reads", &reads, name)?;
        set_field(
            &object,
            "readsOwner",
            &JsValue::from_bool(constraint.reads_owner()),
            name,
        )?;
        rules.push(&object);
    }

    Ok(rules)
}

/// The first rule of `document_type`'s `propertyConstraints` that `document`
/// breaks, in name order, as consensus judges a create or replace: its
/// properties, and its owner for `$ownerId`. `undefined` when it meets them
/// all.
pub(crate) fn check_property_constraints(
    document_type: DocumentTypeRef<'_>,
    document: &Document,
) -> WasmDppResult<JsValue> {
    let constraints = document_type.property_constraints();
    if constraints.is_empty() {
        return Ok(JsValue::UNDEFINED);
    }
    let data = Value::from(document.properties().clone());
    for (name, constraint) in constraints {
        if let Some(violation) = constraint.violation(&data, Some(document.owner_id())) {
            let object = Object::new();
            set_field(&object, "rule", &JsValue::from_str(name), name)?;
            set_field(
                &object,
                "violation",
                &JsValue::from_str(violation_name(violation)),
                name,
            )?;
            set_field(
                &object,
                "message",
                &JsValue::from_str(&violation.to_string()),
                name,
            )?;
            return Ok(object.into());
        }
    }
    Ok(JsValue::UNDEFINED)
}
