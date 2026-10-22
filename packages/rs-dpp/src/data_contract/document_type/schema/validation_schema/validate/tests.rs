use super::super::tests::{
    existing_validator_accepts, generated_schema, generated_values, registered, Cases,
};
use super::*;
use crate::consensus::codes::ErrorWithCode;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::property_constraints::DocumentSystemValues;
use crate::data_contract::methods::validate_document::DataContractDocumentValidationMethodsV0;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::DataContract;
use crate::validation::JsonSchemaValidator;
use platform_value::platform_value;
use platform_version::version::PlatformVersion;

/// A document schema whose one property, `value`, has `schema`.
fn with_value(schema: &JsonValue) -> JsonValue {
    json!({ "type": "object", "properties": { "value": schema } })
}

/// The native validator of `root`, a document schema.
fn validator_of(root: &JsonValue) -> DocumentSchemaValidator {
    let schema = ValidationSchema::try_from_root(&Value::from(root.clone()))
        .expect("expected the schema to build");
    DocumentSchemaValidator::new(schema).expect("expected the patterns to compile")
}

/// Whether the native validator accepts `document` under `root`, asserting the existing
/// validator agrees.
fn agreed_verdict(root: &JsonValue, document: &JsonValue) -> bool {
    let native = validator_of(root)
        .validate(document)
        .expect("expected the check to run")
        .is_empty();
    assert_eq!(
        native,
        existing_validator_accepts(root, document),
        "the validators disagree on {document} under {root}"
    );
    native
}

/// Checks each case, a property schema, a value and the verdict both validators reach.
fn check(cases: Vec<(JsonValue, JsonValue, bool)>) {
    for (schema, value, expected) in cases {
        let root = with_value(&schema);
        assert_eq!(
            agreed_verdict(&root, &json!({ "value": value })),
            expected,
            "{value} under {schema}"
        );
    }
}

#[test]
fn should_read_numbers_as_the_existing_validator_does() {
    check(vec![
        // A float with no fractional part is an integer
        (json!({ "type": "integer" }), json!(3.0), true),
        (json!({ "type": "integer" }), json!(3.5), false),
        (json!({ "type": "integer" }), json!(1e300), true),
        // Numbers compare by value whatever their reading
        (json!({ "enum": [1] }), json!(1.0), true),
        (json!({ "minimum": 1.5 }), json!(1), false),
        (json!({ "minimum": 1.5 }), json!(2), true),
        (json!({ "maximum": -1 }), json!(-1), true),
        (json!({ "maximum": -1 }), json!(0), false),
        (json!({ "exclusiveMaximum": 10 }), json!(10), false),
        (json!({ "exclusiveMaximum": 10 }), json!(9.999), true),
        (json!({ "exclusiveMinimum": -2.5 }), json!(-2), true),
        // A number `const` within `f64::EPSILON`
        (
            json!({ "const": 9007199254740993_u64 }),
            json!(9007199254740992_u64),
            true,
        ),
        (json!({ "const": 0 }), json!(1e-300), true),
        (json!({ "const": 0 }), json!(1e-10), false),
        (json!({ "const": "a" }), json!(1), false),
        // `multipleOf` in f64: integral and fractional divisors
        (json!({ "multipleOf": 2.0 }), json!(4), true),
        (json!({ "multipleOf": 2.0 }), json!(3), false),
        (json!({ "multipleOf": 2 }), json!(4.5), false),
        (json!({ "multipleOf": 0.5 }), json!(1.5), true),
        (json!({ "multipleOf": 0.5 }), json!(-0.3), true),
        (json!({ "multipleOf": 0.5 }), json!(0.3), false),
        // and when the quotient overflows (-2^1023 under 0.3)
        (
            json!({ "multipleOf": 0.3 }),
            json!(-8.98846567431158e307),
            false,
        ),
        (json!({ "multipleOf": 0.1 }), json!(1.5e308), true),
    ]);
}

#[test]
fn should_compare_array_elements_as_the_existing_validator_does() {
    let sixteen = |first: JsonValue, second: JsonValue| {
        let mut elements = vec![first, second];
        elements.extend((3..17).map(|element| json!(element)));
        JsonValue::Array(elements)
    };
    check(vec![
        // Up to 15 elements, by value, and objects key by key in written order
        (json!({ "uniqueItems": true }), json!([1, 1.0]), false),
        (
            json!({ "uniqueItems": true }),
            json!([{ "a": 1, "b": 2 }, { "b": 2, "a": 1 }, 3]),
            true,
        ),
        // More, by JSON value: `1` and `1.0` differ, objects equal in any key order
        (
            json!({ "uniqueItems": true }),
            sixteen(json!(1), json!(1.0)),
            true,
        ),
        (
            json!({ "uniqueItems": true }),
            sixteen(json!({ "a": 1, "b": 2 }), json!({ "b": 2, "a": 1 })),
            false,
        ),
        (
            json!({ "uniqueItems": true }),
            sixteen(json!("x"), json!("x")),
            false,
        ),
        (json!({ "uniqueItems": false }), json!([1, 1]), true),
        (
            json!({ "contains": { "type": "string" } }),
            json!([1, "a"]),
            true,
        ),
        (
            json!({ "contains": { "type": "string" } }),
            json!([1]),
            false,
        ),
        (json!({ "items": { "maximum": 3 } }), json!([1, 4]), false),
        (json!({ "minItems": 2 }), json!([1]), false),
        (json!({ "maxItems": 1 }), json!([1, 2]), false),
        // `byteArray`: unsigned integers up to 255, refusing a float and any other value
        (json!({ "byteArray": true }), json!([0, 255]), true),
        (json!({ "byteArray": true }), json!([256]), false),
        (json!({ "byteArray": true }), json!([1.0]), false),
        (json!({ "byteArray": true }), json!([-1]), false),
        (json!({ "byteArray": true }), json!("ab"), false),
    ]);
}

#[test]
fn should_read_strings_as_the_existing_validator_does() {
    check(vec![
        // Lengths count characters
        (json!({ "minLength": 2 }), json!("é"), false),
        (json!({ "maxLength": 1 }), json!("é"), true),
        // A pattern is searched for, its classes ASCII
        (json!({ "pattern": "b" }), json!("abc"), true),
        (json!({ "pattern": "^b" }), json!("abc"), false),
        (json!({ "pattern": "^\\d+$" }), json!("123"), true),
        (json!({ "pattern": "^\\d+$" }), json!("٣"), false),
        (json!({ "pattern": "\\w" }), json!("é"), false),
        // Formats
        (json!({ "format": "date" }), json!("2026-02-28"), true),
        (json!({ "format": "date" }), json!("2026-02-30"), false),
        (json!({ "format": "date" }), json!("2026-1-01"), false),
        (json!({ "format": "date" }), json!(5), true),
        (
            json!({ "format": "date-time" }),
            json!("2026-10-22T10:00:00Z"),
            true,
        ),
        (
            json!({ "format": "date-time" }),
            json!("2026-10-22 10:00:00"),
            false,
        ),
        (json!({ "format": "email" }), json!("a@b"), true),
        (json!({ "format": "email" }), json!(".a@b"), false),
        (json!({ "format": "email" }), json!("a..b@c"), false),
        (json!({ "format": "idn-email" }), json!("ab"), false),
        (json!({ "format": "hostname" }), json!("a-b.c"), true),
        (json!({ "format": "hostname" }), json!("-a"), false),
        (json!({ "format": "hostname" }), json!("a_b"), false),
        (json!({ "format": "ipv4" }), json!("1.2.3.4"), true),
        (json!({ "format": "ipv4" }), json!("01.2.3.4"), false),
        (json!({ "format": "ipv4" }), json!("::1"), false),
        (json!({ "format": "ipv6" }), json!("::1"), true),
        (json!({ "format": "ipv6" }), json!("1.2.3.4"), false),
        (json!({ "format": "regex" }), json!("a+"), true),
        (json!({ "format": "regex" }), json!("("), false),
        (json!({ "format": "regex" }), json!("(?=a)"), false),
        (json!({ "format": "time" }), json!("12:00:00Z"), true),
        (json!({ "format": "time" }), json!("24:00:00Z"), false),
        (json!({ "format": "time" }), json!("12:00:00"), false),
        (json!({ "format": "uri" }), json!("http://x"), true),
        (json!({ "format": "uri" }), json!("x"), false),
    ]);
}

#[test]
fn should_read_objects_and_references_as_the_existing_validator_does() {
    let object = json!({
        "type": "object",
        "properties": { "a": { "type": "string" } },
        "required": ["a"],
        "additionalProperties": false,
    });
    check(vec![
        (object.clone(), json!({ "a": "x" }), true),
        (object.clone(), json!({}), false),
        (object.clone(), json!({ "a": "x", "b": 1 }), false),
        (object, json!({ "a": 1 }), false),
        (
            json!({ "dependentRequired": { "a": ["b"] } }),
            json!({ "a": 1 }),
            false,
        ),
        (
            json!({ "dependentRequired": { "a": ["b"] } }),
            json!({ "a": 1, "b": 2 }),
            true,
        ),
        (
            json!({ "dependentRequired": { "a": ["b"] } }),
            json!({ "b": 1 }),
            true,
        ),
        (json!({ "minProperties": 1 }), json!({}), false),
        (
            json!({ "maxProperties": 1 }),
            json!({ "a": 1, "b": 2 }),
            false,
        ),
    ]);

    // A reference, one keeping keywords beside it, and one inside `contains`
    let root = json!({
        "type": "object",
        "properties": {
            "label": { "$ref": "#/$defs/label" },
            "short": { "$ref": "#/$defs/label", "maxLength": 1 },
            "labels": { "type": "array", "contains": { "$ref": "#/$defs/label" } },
        },
        "$defs": { "label": { "type": "string", "maxLength": 3 } },
    });
    for (document, expected) in [
        (json!({ "label": "abc" }), true),
        (json!({ "label": "abcd" }), false),
        (json!({ "short": "ab" }), false),
        (json!({ "labels": [1, "abc"] }), true),
        (json!({ "labels": [1, "abcd"] }), false),
    ] {
        assert_eq!(agreed_verdict(&root, &document), expected, "{document}");
    }
}

#[test]
fn should_report_where_a_value_fails() {
    let violations = validator_of(&json!({
        "type": "object",
        "properties": {
            "tags": { "type": "array", "items": { "type": "string", "maxLength": 2 } },
        },
        "required": ["name"],
    }))
    .validate(&json!({ "tags": ["ok", "long"] }))
    .expect("expected the check to run");
    let found: Vec<_> = violations
        .iter()
        .map(|violation| {
            (
                violation.keyword,
                violation.instance_path.as_str(),
                violation.schema_path.as_str(),
                violation.property_name.as_str(),
            )
        })
        .collect();
    assert_eq!(
        found,
        vec![
            (
                "maxLength",
                "/tags/1",
                "/properties/tags/items/maxLength",
                ""
            ),
            ("required", "", "/required", "name"),
        ]
    );
}

#[test]
fn should_refuse_a_cycle_of_references_that_consumes_nothing() {
    let validator = validator_of(&json!({
        "type": "object",
        "properties": { "value": { "$ref": "#/$defs/a" } },
        "$defs": { "a": { "$ref": "#/$defs/b" }, "b": { "$ref": "#/$defs/a" } },
    }));
    assert!(matches!(
        validator.validate(&json!({ "value": 1 })),
        Err(ProtocolError::CorruptedCodeExecution(_))
    ));
}

/// Generated schemas and documents: wherever the builder builds a schema the existing
/// validator compiles, the native validator reaches the existing validator's verdict on
/// every generated document.
#[test]
fn should_reach_the_existing_validators_verdict_on_generated_schemas() {
    let platform_version = PlatformVersion::latest();
    let values = generated_values();
    let documents: Vec<JsonValue> = values
        .iter()
        .map(|value| json!({ "value": value }))
        .chain([json!({})])
        .collect();

    let mut cases = Cases(0x0dd5_eed5_1234_abcd);
    let mut checked = 0;
    for case in 0..800 {
        let definitions = cases.below(3);
        let defs: JsonMap<String, JsonValue> = (0..definitions)
            .map(|index| {
                (
                    format!("d{index}"),
                    generated_schema(&mut cases, &values, 2, index),
                )
            })
            .collect();
        let mut root = with_value(&generated_schema(&mut cases, &values, 2, definitions));
        if definitions > 0 {
            root["$defs"] = JsonValue::Object(defs);
        }

        let existing = JsonSchemaValidator::new();
        if existing.compile(&root, platform_version).is_err() {
            continue;
        }
        let Ok(schema) = ValidationSchema::try_from_root(&Value::from(root.clone())) else {
            continue;
        };
        let native =
            DocumentSchemaValidator::new(schema).expect("expected the patterns to compile");
        for document in &documents {
            let existing_accepts = existing
                .validate(document, platform_version)
                .expect("expected the existing validator to run")
                .is_valid();
            let native_accepts = native
                .validate(document)
                .expect("expected the check to run")
                .is_empty();
            assert_eq!(
                native_accepts, existing_accepts,
                "case {case}: {document} under {root}"
            );
        }
        checked += 1;
    }
    assert!(
        checked >= 600,
        "expected most generated schemas to build, {checked} did"
    );
}

/// The document type `item` of a contract registered with `properties`.
fn item_contract(properties: Value) -> DataContract {
    registered(properties, platform_value!({}))
}

/// Document validation's verdict on `properties` as an `item`, at `platform_version`.
fn document_errors(
    contract: &DataContract,
    properties: Value,
    platform_version: &PlatformVersion,
) -> Vec<ConsensusError> {
    contract
        .validate_document_properties(
            "item",
            properties,
            &DocumentSystemValues::default(),
            platform_version,
        )
        .expect("expected document validation to run")
        .errors
}

#[test]
fn should_validate_documents_at_protocol_version_15_as_at_14() {
    let contract = item_contract(platform_value!({
        "name": { "type": "string", "maxLength": 4, "pattern": "^[a-z]+$", "position": 0 },
        "count": { "type": "integer", "minimum": 0, "maximum": 10, "position": 1 },
        "kind": { "type": "string", "enum": ["a", "b"], "position": 2 },
        "tags": {
            "type": "array",
            "items": { "type": "string", "maxLength": 3 },
            "maxItems": 3,
            "uniqueItems": true,
            "position": 3,
        },
        "data": {
            "type": "array",
            "byteArray": true,
            "maxItems": 4,
            "position": 4,
        },
        "when": { "type": "string", "format": "date", "maxLength": 10, "position": 5 },
    }));
    let latest = PlatformVersion::latest();
    let protocol_version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
    for properties in [
        platform_value!({ "name": "abc", "count": 3, "kind": "a" }),
        platform_value!({ "name": "abcde" }),
        platform_value!({ "name": "ABC" }),
        platform_value!({ "count": 11 }),
        platform_value!({ "count": 3.0 }),
        platform_value!({ "kind": "c" }),
        platform_value!({ "tags": ["a", "a"] }),
        platform_value!({ "tags": ["abcd"] }),
        platform_value!({ "data": [1, 2] }),
        platform_value!({ "data": [1, 300] }),
        platform_value!({ "when": "2026-10-22" }),
        platform_value!({ "when": "2026-13-01" }),
        platform_value!({ "other": 1 }),
    ] {
        let native = document_errors(&contract, properties.clone(), latest);
        let existing = document_errors(&contract, properties.clone(), protocol_version_14);
        assert_eq!(native.is_empty(), existing.is_empty(), "{properties:?}");
        // Consensus depends on the code of the first error, not on its message
        assert_eq!(
            native.first().map(ErrorWithCode::code),
            existing.first().map(ErrorWithCode::code),
            "{properties:?}"
        );
    }
}

#[test]
fn should_keep_the_existing_check_for_a_schema_without_a_validation_schema() {
    let platform_version = PlatformVersion::latest();
    let mut contract = item_contract(platform_value!({
        "name": { "type": "string", "position": 0 },
    }));
    // `not` has no place in the validation schema; the document meta-schemas refuse it, so
    // the type is set without full validation
    contract
        .set_document_schema(
            "item",
            platform_value!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "not": { "const": "x" }, "position": 0 },
                },
                "additionalProperties": false,
            }),
            false,
            &mut Vec::new(),
            platform_version,
        )
        .expect("expected the document type to set");
    assert!(contract
        .document_type_validation_schema("item", platform_version)
        .is_err());
    assert!(document_errors(
        &contract,
        platform_value!({ "name": "y" }),
        platform_version
    )
    .is_empty());
    assert!(!document_errors(
        &contract,
        platform_value!({ "name": "x" }),
        platform_version
    )
    .is_empty());
}
