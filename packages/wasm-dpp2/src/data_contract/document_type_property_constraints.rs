//! `propertyConstraints` rules: the conditions a document type holds every
//! created or replaced document's properties to, from protocol version 14
//! onward.
//!
//! A rule is a condition: a comparison of integer expressions (sizes of
//! strings and arrays included), a membership test (`in`), a comparison of a string or an identifier property (or
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
use dpp::consensus::basic::document::PropertyConstraintViolation;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::methods::DocumentTypeBasicMethods;
use dpp::data_contract::document_type::property_constraints::{
    AggregateKind, DocumentSystemValues, PropertyRead,
};
use dpp::document::{Document, DocumentV0Getters};
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use js_sys::{Array, BigInt, Object, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_PROPERTY_CONSTRAINTS_TS: &'static str = r#"
/**
 * An integer expression of a `propertyConstraints` rule.
 *
 * - a number: an integer value, a `bigint` past `Number.MAX_SAFE_INTEGER`
 *   (rules report such a literal as a `bigint`, exactly);
 * - a string: the dotted path of an integer or boolean property, whose value
 *   it takes (a boolean reads as 1 for true and 0 for false), 0 when the
 *   document leaves the property out; or a system time or height the
 *   document type records by listing it in `required`
 *   (`PropertyConstraintSystemProperty`);
 * - `ifAbsent`: a property path and the integer it takes when left out;
 * - `add`, `multiply`, `min` and `max` over two or more operands,
 *   `subtract`, `divide`, `modulo` and `power` over exactly two, `abs` over
 *   one. Arithmetic is exact over 128-bit
 *   integers; `divide` and `modulo` are Euclidean;
 * - `length` and `byteLength`: the characters (as `maxLength` counts them)
 *   and the UTF-8 bytes of a string property; `count`: the items of an array
 *   property, or the bytes of a byte array property. Each is 0 when the
 *   document leaves the property out;
 * - `countOf` and `sumOf`: a total read from state, how many documents of a
 *   type of the same contract match a filter, or the total of an integer
 *   property over them, as the type's count or sum trees keep it once the
 *   write is done. `checkDocumentPropertyConstraints` does not judge a rule
 *   reading one.
 */
export type PropertyConstraintExpression =
  | number
  | bigint
  | string
  | { ifAbsent: [path: string, value: number | bigint] }
  | { add: PropertyConstraintExpression[] }
  | { multiply: PropertyConstraintExpression[] }
  | { subtract: [PropertyConstraintExpression, PropertyConstraintExpression] }
  | { divide: [PropertyConstraintExpression, PropertyConstraintExpression] }
  | { modulo: [PropertyConstraintExpression, PropertyConstraintExpression] }
  | { power: [PropertyConstraintExpression, PropertyConstraintExpression] }
  | { min: PropertyConstraintExpression[] }
  | { max: PropertyConstraintExpression[] }
  | { abs: PropertyConstraintExpression }
  | { length: string }
  | { byteLength: string }
  | { count: string }
  | { countOf: [documentType: string] | [documentType: string, filter: PropertyConstraintAggregateFilter] }
  | {
    sumOf:
    | [documentType: string, property: string]
    | [documentType: string, property: string, filter: PropertyConstraintAggregateFilter]
  };

/**
 * Which documents a `countOf` or `sumOf` totals: each key, a property path of
 * the counted type or `"$ownerId"`, mapped to the value its documents must
 * take there, read from the document being written: a property path of it,
 * `"$ownerId"`, an integer, or a `const` string or base58 identifier.
 */
export type PropertyConstraintAggregateFilter = Record<
  string,
  string | number | bigint | { const: string }
>;

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
 * - `startsWith` / `endsWith`: two strings, a `const` or a string property
 *   each, the first starting or ending with the second, byte for byte;
 * - `contains`: a typed array property and the value one of its elements must
 *   equal, an integer expression, a string or an identifier operand as its
 *   elements are; an array the document leaves out holds nothing;
 * - `present` / `absent`: whether the document holds a property of any type;
 * - `notIn`: what `in` lists, holding when the operand takes none of the values;
 * - `anyOf` / `allOf` over two or more conditions, `not` over one, `ifThen`
 *   over two (the second must hold when the first does) and `ifThenElse` over
 *   three (the second must hold when the first does, the third when it does
 *   not). Conditions are checked in order and no further than the outcome
 *   needs.
 */
export type PropertyConstraintCondition =
  | { equal: [PropertyConstraintExpression, PropertyConstraintExpression] | [PropertyConstraintEqualityOperand, PropertyConstraintEqualityOperand] }
  | { notEqual: [PropertyConstraintExpression, PropertyConstraintExpression] | [PropertyConstraintEqualityOperand, PropertyConstraintEqualityOperand] }
  | { lessThan: [PropertyConstraintExpression, PropertyConstraintExpression] }
  | { lessThanOrEqual: [PropertyConstraintExpression, PropertyConstraintExpression] }
  | { greaterThan: [PropertyConstraintExpression, PropertyConstraintExpression] }
  | { greaterThanOrEqual: [PropertyConstraintExpression, PropertyConstraintExpression] }
  | { in: [PropertyConstraintExpression, Array<number | bigint>] | [string | { ifAbsent: [path: string, value: string] }, string[]] }
  | { notIn: [PropertyConstraintExpression, Array<number | bigint>] | [string | { ifAbsent: [path: string, value: string] }, string[]] }
  | { startsWith: [PropertyConstraintEqualityOperand, PropertyConstraintEqualityOperand] }
  | { endsWith: [PropertyConstraintEqualityOperand, PropertyConstraintEqualityOperand] }
  | { contains: [path: string, PropertyConstraintExpression | PropertyConstraintEqualityOperand] }
  | { present: string }
  | { absent: string }
  | { anyOf: PropertyConstraintCondition[] }
  | { allOf: PropertyConstraintCondition[] }
  | { not: PropertyConstraintCondition }
  | { ifThen: [PropertyConstraintCondition, PropertyConstraintCondition] }
  | { ifThenElse: [PropertyConstraintCondition, PropertyConstraintCondition, PropertyConstraintCondition] };

/**
 * How a rule reads a property: `value` as an integer operand, `presence` in
 * `present` or `absent`, `text` compared with strings, `identifier` compared
 * with identifiers, `length` by the size of a string (`length` or
 * `byteLength`), `count` by the items of an array or byte array, `elements`
 * by the elements a `contains` looks among.
 */
export type PropertyConstraintReadKind =
  | 'value'
  | 'presence'
  | 'text'
  | 'identifier'
  | 'length'
  | 'count'
  | 'elements';

/**
 * A system time or height a rule reads: the block time in milliseconds
 * (`At`), the Platform block height (`AtBlockHeight`) or the Core block height
 * (`AtCoreBlockHeight`) of the document's creation, its last update (a create,
 * a replace or a price update) and its last transfer (a create, a transfer or
 * a purchase).
 */
export type PropertyConstraintSystemProperty =
  | '$createdAt'
  | '$updatedAt'
  | '$transferredAt'
  | '$createdAtBlockHeight'
  | '$updatedAtBlockHeight'
  | '$transferredAtBlockHeight'
  | '$createdAtCoreBlockHeight'
  | '$updatedAtCoreBlockHeight'
  | '$transferredAtCoreBlockHeight';

/**
 * A `countOf` or `sumOf` total a rule reads: how many documents of
 * `documentType`, a type of the same contract, match the filter, or the total
 * of their integer `property` (a `sumOf` only). `filter` lists the keys the
 * documents are matched by, properties of that type or `$ownerId`; the values
 * they must take are in the rule. The platform reads the total from state when
 * the document is sent; `checkDocumentPropertyConstraints` cannot, and does
 * not judge a rule reading one.
 */
export type PropertyConstraintTotalRead = {
  kind: 'countOf' | 'sumOf';
  documentType: string;
  property?: string;
  filter: string[];
};

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
  /**
   * Whether the rule reads `$ownerId`, or a total that depends on the owner:
   * then a transfer or a purchase is judged against it too.
   */
  readsOwner: boolean;
  /**
   * The system times and heights the rule reads, in declared order. A price
   * update is judged against a rule reading the update's, and a transfer or a
   * purchase against one reading the transfer's.
   */
  readsSystem: PropertyConstraintSystemProperty[];
  /**
   * The `countOf` and `sumOf` totals the rule reads, in declared order, one
   * read twice listed twice. The pre-check does not judge a rule reading one.
   */
  readsTotals: PropertyConstraintTotalRead[];
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
        PropertyRead::Length => "length",
        PropertyRead::Count => "count",
        PropertyRead::Elements(_) => "elements",
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

/// `Number.MAX_SAFE_INTEGER`, the largest integer a JS `number` holds exactly.
const MAX_SAFE_INTEGER: i128 = (1 << 53) - 1;

/// An integer literal of a rule: a `number` while it is exact in JavaScript,
/// a `bigint` past that.
fn integer_to_js(integer: i128) -> JsValue {
    if (-MAX_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&integer) {
        JsValue::from_f64(integer as f64)
    } else {
        BigInt::from(integer).into()
    }
}

/// A declared rule as JS, the JSON it was declared as. Not through
/// `serde_json`: a rule may compare with any 64-bit literal, and the JSON
/// conversion throws on one past `Number.MAX_SAFE_INTEGER`, which would hide
/// every rule of the type.
fn rule_to_js(value: &Value, rule: &str) -> WasmDppResult<JsValue> {
    Ok(match value {
        Value::Text(text) => JsValue::from_str(text),
        Value::Bool(flag) => JsValue::from_bool(*flag),
        Value::Null => JsValue::NULL,
        Value::Float(number) => JsValue::from_f64(*number),
        Value::U8(integer) => integer_to_js((*integer).into()),
        Value::U16(integer) => integer_to_js((*integer).into()),
        Value::U32(integer) => integer_to_js((*integer).into()),
        Value::U64(integer) => integer_to_js((*integer).into()),
        Value::I8(integer) => integer_to_js((*integer).into()),
        Value::I16(integer) => integer_to_js((*integer).into()),
        Value::I32(integer) => integer_to_js((*integer).into()),
        Value::I64(integer) => integer_to_js((*integer).into()),
        Value::I128(integer) => integer_to_js(*integer),
        Value::U128(integer) => match i128::try_from(*integer) {
            Ok(integer) => integer_to_js(integer),
            Err(_) => BigInt::from(*integer).into(),
        },
        Value::Array(items) => {
            let array = Array::new();
            for item in items {
                array.push(&rule_to_js(item, rule)?);
            }
            array.into()
        }
        Value::Map(entries) => {
            let object = Object::new();
            for (key, entry) in entries {
                let key = key.as_text().ok_or_else(|| {
                    WasmDppError::generic(format!(
                        "the propertyConstraints rule '{rule}' has a key that is not a string"
                    ))
                })?;
                set_field(&object, key, &rule_to_js(entry, rule)?, rule)?;
            }
            object.into()
        }
        other => {
            return Err(WasmDppError::generic(format!(
                "the propertyConstraints rule '{rule}' holds {other}, which is not JSON"
            )));
        }
    })
}

/// Collect every `propertyConstraints` rule of one document type, in name
/// order, the order consensus checks them in.
///
/// The parsed rules give the name, the reads and whether the owner is read;
/// the rule itself is the schema's declaration, which is what an app wrote,
/// with integer literals past `Number.MAX_SAFE_INTEGER` as `bigint`.
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
        set_field(&object, "rule", &rule_to_js(declared, name)?, name)?;

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
        let reads_system = Array::new();
        for property in constraint.system_reads() {
            reads_system.push(&JsValue::from_str(property.name()));
        }
        set_field(&object, "readsSystem", &reads_system, name)?;
        let reads_totals = Array::new();
        for read in constraint.aggregate_reads() {
            let entry = Object::new();
            set_field(&entry, "kind", &JsValue::from_str(read.wire_name()), name)?;
            set_field(
                &entry,
                "documentType",
                &JsValue::from_str(&read.document_type),
                name,
            )?;
            if let AggregateKind::Sum { property } = &read.kind {
                set_field(&entry, "property", &JsValue::from_str(property), name)?;
            }
            let filter = Array::new();
            for key in read.filter.keys() {
                filter.push(&JsValue::from_str(key));
            }
            set_field(&entry, "filter", &filter, name)?;
            reads_totals.push(&entry);
        }
        set_field(&object, "readsTotals", &reads_totals, name)?;
        rules.push(&object);
    }

    Ok(rules)
}

/// The system values a create or a replace of `document` will have, as far as
/// a client can tell before its block: the owner, and the stored times and
/// heights the write keeps, with the device clock standing in for the block
/// time it records (its update, and its creation and transfer when the
/// document has none yet). The block heights it records are unknown until the
/// block, so a rule reading one is not judged, and so is a rule reading a
/// `countOf` or `sumOf` total, which no client reads from state here.
fn system_values_for_write(document: &Document) -> DocumentSystemValues {
    // Milliseconds since the epoch, a whole number well inside a `u64`
    let now = js_sys::Date::now() as u64;
    let stored = DocumentSystemValues::of_document(document);
    DocumentSystemValues {
        created_at: stored.created_at.or(Some(now)),
        updated_at: Some(now),
        transferred_at: stored.transferred_at.or(Some(now)),
        updated_at_block_height: None,
        updated_at_core_block_height: None,
        ..stored
    }
}

/// The first rule of `document_type`'s `propertyConstraints` that `document`
/// breaks, in name order, as consensus judges a create or replace: its
/// properties with every `generatedFrom` property generated from its params,
/// as the transition builders send them, its owner for `$ownerId`, and its
/// system times and heights as [`system_values_for_write`] estimates them.
/// `undefined` when it meets them all.
pub(crate) fn check_property_constraints(
    document_type: DocumentTypeRef<'_>,
    document: &Document,
) -> WasmDppResult<JsValue> {
    let constraints = document_type.property_constraints();
    if constraints.is_empty() {
        return Ok(JsValue::UNDEFINED);
    }
    let mut properties = document.properties().clone();
    document_type.regenerate_generated_properties(&mut properties, PlatformVersion::desired())?;
    let data = Value::from(properties);
    let system = system_values_for_write(document);
    for (name, constraint) in constraints {
        if let Some(violation) = constraint.violation(&data, &system) {
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
