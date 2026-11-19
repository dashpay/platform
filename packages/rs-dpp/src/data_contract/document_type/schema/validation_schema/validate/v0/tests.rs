use super::super::super::tests::{
    existing_validator_accepts, generated_schema, generated_values, Cases,
};
use super::*;
use crate::consensus::basic::json_schema_error::JsonSchemaError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::validation::JsonSchemaValidator;
use platform_version::version::PlatformVersion;

/// A document schema whose one property, `value`, has `schema`.
fn with_value(schema: &JsonValue) -> JsonValue {
    json!({ "type": "object", "properties": { "value": schema } })
}

/// The native validator's first violation of `document`, as a JSON Schema error, under no
/// value-depth limit (the checks that walk a value's depth are covered on their own).
fn first_violation(
    validator: &DocumentSchemaValidator,
    document: &JsonValue,
) -> Option<JsonSchemaError> {
    match validator
        .validate(document, None)
        .expect("expected the check to run")
    {
        None => None,
        Some(ConsensusError::BasicError(BasicError::JsonSchemaError(error))) => Some(error),
        Some(other) => panic!("unexpected consensus error: {other:?}"),
    }
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
    let native = first_violation(&validator_of(root), document).is_none();
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
        // exactly across readings: equal as f64, not as numbers
        (
            json!({ "uniqueItems": true }),
            json!([9007199254740993_u64, 9007199254740992.0]),
            true,
        ),
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
fn should_report_the_first_violation_and_where_it_is() {
    // The value also lacks the required `name`, which the walk does not reach
    let validator = validator_of(&json!({
        "type": "object",
        "properties": {
            "tags": { "type": "array", "items": { "type": "string", "maxLength": 2 } },
        },
        "required": ["name"],
    }));
    let violation = first_violation(&validator, &json!({ "tags": ["ok", "long"] }))
        .expect("expected a violation");
    assert_eq!(
        (
            violation.keyword(),
            violation.instance_path(),
            violation.schema_path(),
            violation.property_name(),
        ),
        (
            "maxLength",
            "/tags/1",
            "/properties/tags/items/maxLength",
            ""
        )
    );
}

#[test]
fn should_report_the_first_violation_in_the_walks_order() {
    // The JSON Schema validator lists `enum` first, as the schema writes it first
    let validator = validator_of(&with_value(&json!({ "enum": ["a"], "type": "string" })));
    let violation =
        first_violation(&validator, &json!({ "value": 1 })).expect("expected a violation");
    assert_eq!(violation.keyword(), "type");
}

/// A rule of protocol version 15: in a `uniqueItems` list, `0.0` and `-0.0` are one value at
/// every length, on both sides of the 15 elements up to which elements compare pairwise.
#[test]
fn should_hold_zero_and_negative_zero_as_one_value_at_every_length() {
    let validator = validator_of(&with_value(&json!({ "uniqueItems": true })));
    // The two numbers given, then distinct integers up to `length` elements
    let list = |length: usize, first: f64, second: f64| {
        let mut items = vec![json!(first), json!(second)];
        items.extend((1..=length - 2).map(|item| json!(item)));
        JsonValue::Array(items)
    };
    for length in [2, 15, 16, 17] {
        let violation = first_violation(&validator, &json!({ "value": list(length, 0.0, -0.0) }));
        assert_eq!(
            violation.as_ref().map(|violation| violation.keyword()),
            Some("uniqueItems"),
            "{length} elements holding 0.0 and -0.0"
        );
        assert!(
            first_violation(&validator, &json!({ "value": list(length, 0.0, 0.5) })).is_none(),
            "{length} distinct elements"
        );
    }
}

#[test]
fn should_refuse_a_cycle_of_references_that_consumes_nothing() {
    // Reached at the value, and through `contains`, whose candidates are only matched
    for value in [
        json!({ "$ref": "#/$defs/a" }),
        json!({ "type": "array", "contains": { "$ref": "#/$defs/a" } }),
    ] {
        let schema = ValidationSchema::try_from_root(&Value::from(json!({
            "type": "object",
            "properties": { "value": value },
            "$defs": { "a": { "$ref": "#/$defs/b" }, "b": { "$ref": "#/$defs/a" } },
        })))
        .expect("expected the schema to build");
        assert!(
            matches!(
                DocumentSchemaValidator::new(schema),
                Err(DataContractError::InvalidContractStructure(_))
            ),
            "{value}"
        );
    }
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

        // Compiled as document validation compiles a contract read from state, on the first
        // document, and reused for the others
        let existing = JsonSchemaValidator::new();
        let Ok(existing_verdicts) = documents
            .iter()
            .map(|document| {
                existing
                    .compile_and_validate(&root, document, platform_version)
                    .map(|result| result.is_valid())
            })
            .collect::<Result<Vec<_>, _>>()
        else {
            continue;
        };
        let Ok(schema) = ValidationSchema::try_from_root(&Value::from(root.clone())) else {
            continue;
        };
        let native =
            DocumentSchemaValidator::new(schema).expect("expected the patterns to compile");
        for (document, existing_accepts) in documents.iter().zip(existing_verdicts) {
            let native_accepts = first_violation(&native, document).is_none();
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

/// The first error of the existing validator on `document` under `root`, as it reports it.
fn existing_validator_first_error(
    root: &JsonValue,
    document: &JsonValue,
) -> Option<JsonSchemaError> {
    let result = JsonSchemaValidator::new()
        .compile_and_validate(root, document, PlatformVersion::latest())
        .expect("expected the schema to compile");
    result.errors.into_iter().next().map(|error| match error {
        ConsensusError::BasicError(BasicError::JsonSchemaError(error)) => error,
        other => panic!("expected a JSON Schema error, got {other:?}"),
    })
}

#[test]
fn should_report_a_violation_with_the_existing_validators_keyword_paths_and_parameters() {
    let identifier = json!({
        "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
        "contentMediaType": "application/x.dash.dpp.identifier",
    });
    let cases: Vec<(&str, JsonValue, JsonValue)> = vec![
        ("type", json!({ "type": "string" }), json!(1)),
        (
            "a union type",
            json!({ "type": ["string", "number"] }),
            json!(true),
        ),
        (
            "a list of one type",
            json!({ "type": ["string"] }),
            json!(1),
        ),
        (
            "a list repeating a type",
            json!({ "type": ["string", "string"] }),
            json!(1),
        ),
        ("enum", json!({ "enum": ["a", "b"] }), json!("c")),
        ("const", json!({ "const": 3 }), json!(4)),
        (
            "minimum",
            json!({ "type": "integer", "minimum": 2 }),
            json!(1),
        ),
        (
            "maximum",
            json!({ "type": "number", "maximum": 2.5 }),
            json!(3),
        ),
        (
            "exclusiveMinimum",
            json!({ "exclusiveMinimum": 2 }),
            json!(2),
        ),
        (
            "exclusiveMaximum",
            json!({ "exclusiveMaximum": 2 }),
            json!(2),
        ),
        ("multipleOf", json!({ "multipleOf": 3 }), json!(4)),
        (
            "minLength",
            json!({ "type": "string", "minLength": 2 }),
            json!("a"),
        ),
        (
            "maxLength",
            json!({ "type": "string", "maxLength": 1 }),
            json!("ab"),
        ),
        (
            "pattern",
            json!({ "type": "string", "pattern": "^a" }),
            json!("b"),
        ),
        (
            "format",
            json!({ "type": "string", "format": "date" }),
            json!("2026-13-01"),
        ),
        (
            "required",
            json!({ "type": "object", "properties": { "a": {} }, "required": ["a"] }),
            json!({}),
        ),
        (
            "additionalProperties",
            json!({ "type": "object", "properties": { "a": {} }, "additionalProperties": false }),
            json!({ "a": 1, "b": 2, "c": 3 }),
        ),
        (
            "minProperties",
            json!({ "type": "object", "minProperties": 1 }),
            json!({}),
        ),
        (
            "maxProperties",
            json!({ "type": "object", "maxProperties": 0 }),
            json!({ "a": 1 }),
        ),
        (
            "dependentRequired",
            json!({ "type": "object", "dependentRequired": { "a": ["b"] } }),
            json!({ "a": 1 }),
        ),
        (
            "additionalProperties false with no properties",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "a": 1, "b": 2 }),
        ),
        (
            "a required name a JSON string escapes",
            json!({ "type": "object", "required": ["a\"b"] }),
            json!({}),
        ),
        (
            "a dependent required name a JSON string escapes",
            json!({ "type": "object", "dependentRequired": { "a": ["b\\c"] } }),
            json!({ "a": 1 }),
        ),
        (
            "an element",
            json!({ "type": "array", "items": { "type": "string" } }),
            json!(["a", 1]),
        ),
        (
            "minItems",
            json!({ "type": "array", "minItems": 2 }),
            json!([1]),
        ),
        (
            "maxItems",
            json!({ "type": "array", "maxItems": 1 }),
            json!([1, 2]),
        ),
        (
            "uniqueItems",
            json!({ "type": "array", "uniqueItems": true }),
            json!([1, 1]),
        ),
        (
            "contains",
            json!({ "type": "array", "contains": { "type": "string" } }),
            json!([1, 2]),
        ),
        (
            "a byte array of no array",
            json!({ "byteArray": true }),
            json!("ab"),
        ),
        (
            "a byte past 255",
            json!({ "type": "array", "byteArray": true }),
            json!([1, 256]),
        ),
        (
            "a byte of no integer",
            json!({ "type": "array", "byteArray": true }),
            json!([1, "a"]),
        ),
        ("an identifier of 31 bytes", identifier, json!(vec![1; 31])),
    ];
    let referenced = |target: JsonValue| {
        json!({
            "type": "object",
            "properties": {
                "value": { "$ref": "#/$defs/target" },
                "nested": { "$ref": "#/$defs/outer" },
            },
            "$defs": {
                "target": target,
                "outer": { "$ref": "#/$defs/target" },
            },
        })
    };
    let mut cases: Vec<(&str, JsonValue, JsonValue)> = cases
        .into_iter()
        .map(|(case, schema, value)| (case, with_value(&schema), json!({ "value": value })))
        .collect();
    cases.extend([
        (
            "a reference",
            referenced(json!({ "type": "string", "maxLength": 1 })),
            json!({ "value": "ab" }),
        ),
        (
            "a reference to a reference",
            referenced(json!({ "type": "string", "maxLength": 1 })),
            json!({ "nested": "ab" }),
        ),
        (
            "a reference to an object",
            referenced(json!({
                "type": "object",
                "properties": { "a": { "type": "integer" } },
                "required": ["b"],
            })),
            json!({ "value": { "a": 1 } }),
        ),
        (
            "a byte array through a reference",
            referenced(json!({ "type": "array", "byteArray": true })),
            json!({ "value": [256] }),
        ),
        (
            "a byte array through a reference to a reference",
            referenced(json!({ "type": "array", "byteArray": true })),
            json!({ "nested": [1, "a"] }),
        ),
        (
            "a byte array inside a referenced object",
            referenced(json!({
                "type": "object",
                "properties": { "bytes": { "type": "array", "byteArray": true } },
            })),
            json!({ "value": { "bytes": [300] } }),
        ),
        (
            "a byte array of no array through a reference",
            referenced(json!({ "byteArray": true })),
            json!({ "value": "ab" }),
        ),
    ]);
    let mut differences = Vec::new();
    for (case, root, document) in cases {
        let expected = existing_validator_first_error(&root, &document)
            .unwrap_or_else(|| panic!("{case}: the existing validator accepts it"));
        let violation = first_violation(&validator_of(&root), &document)
            .unwrap_or_else(|| panic!("{case}: the native validator accepts it"));
        let native = (
            violation.keyword().to_string(),
            violation.instance_path().to_string(),
            violation.schema_path().to_string(),
            violation.params().clone(),
            violation.property_name().to_string(),
        );
        let existing = (
            expected.keyword().to_string(),
            expected.instance_path().to_string(),
            expected.schema_path().to_string(),
            expected.params().clone(),
            expected.property_name().to_string(),
        );
        if native != existing {
            differences.push(format!(
                "{case}:\n  native   {native:?}\n  existing {existing:?}"
            ));
        }
    }
    assert!(differences.is_empty(), "{}", differences.join("\n"));
}

#[test]
fn should_build_no_violation_when_only_asking_whether_a_value_matches() {
    let validator = validator_of(&with_value(&json!({ "type": "string", "enum": ["a"] })));
    let mut walk = Walk::new(&validator, true, None);
    let instance = json!({ "value": 1 });
    walk.node(&validator.schema.root, &instance, 0, 0)
        .expect("expected the check to run");
    assert!(walk.failed);
    assert!(walk.violation.is_none());
}

#[test]
fn should_refuse_a_property_declared_twice() {
    let object = ObjectConstraints {
        properties: Some(vec![
            ("a".to_string(), SchemaNode::default()),
            ("a".to_string(), SchemaNode::default()),
        ]),
        additional_properties: Some(false),
        ..Default::default()
    };
    let schema = ValidationSchema {
        root: SchemaNode {
            object: Some(Box::new(object)),
            ..Default::default()
        },
        references: BTreeMap::new(),
    };
    assert!(matches!(
        DocumentSchemaValidator::new(schema),
        Err(DataContractError::InvalidContractStructure(_))
    ));
}

/// A schema that refers to itself through `items` leads the walk as deep as a value nests.
/// The document validator's own depth check refuses a value past the system maximum before
/// the walk, so this reaches the walk only by calling it directly; the walk then refuses the
/// value at its limit with the same error the depth check reports, rather than recurse as
/// deep as the value.
#[test]
fn should_refuse_a_value_nested_past_the_value_depth_limit() {
    let validator = validator_of(&json!({
        "type": "object",
        "properties": { "p": { "type": "array", "items": { "$ref": "#/properties/p" } } },
    }));
    let mut deep = json!(0);
    for _ in 0..8 {
        deep = JsonValue::Array(vec![deep]);
    }
    let document = json!({ "p": deep });

    let error = validator
        .validate(&document, Some(3))
        .expect("expected the check to run")
        .expect("expected a refusal past the limit");
    let ConsensusError::BasicError(BasicError::ValueError(value_error)) = &error else {
        panic!("expected a value-depth refusal, got {error:?}");
    };
    assert_eq!(
        value_error.to_string(),
        "document value depth 4 exceeds system maximum 3"
    );

    // Under no limit the same value is not refused for its depth; the walk reports its own
    // verdict instead (the innermost value is not an array).
    let unlimited = validator
        .validate(&document, None)
        .expect("expected the check to run");
    assert!(
        !matches!(
            unlimited,
            Some(ConsensusError::BasicError(BasicError::ValueError(_)))
        ),
        "{unlimited:?}"
    );
}

/// The walk's depth bound counts only containers, as the document validator's depth check
/// does, so a value whose container nesting equals the limit and whose leaf is a scalar is
/// not refused for depth under a schema that accepts it.
#[test]
fn should_not_refuse_a_value_whose_container_nesting_equals_the_limit() {
    let validator = validator_of(&json!({
        "type": "object",
        "properties": {
            "p": {
                "type": "array",
                "items": { "type": "array", "items": { "type": "array", "items": { "type": "string" } } }
            }
        },
    }));
    // Three array containers nesting a string: the deepest container is at depth 3.
    let document = json!({ "p": [[["s"]]] });
    assert!(
        validator
            .validate(&document, Some(3))
            .expect("expected the check to run")
            .is_none(),
        "a value at the container limit with a scalar leaf must not be refused for depth"
    );
    // A fourth container exceeds the limit and is refused.
    let too_deep = json!({ "p": [[[["s"]]]] });
    let error = validator
        .validate(&too_deep, Some(3))
        .expect("expected the check to run")
        .expect("expected a refusal past the limit");
    assert!(
        matches!(error, ConsensusError::BasicError(BasicError::ValueError(_))),
        "{error:?}"
    );
}

/// A pattern that parses but that the `regex` crate does not compile under its default size
/// limit.
const PATTERN_BEYOND_THE_SIZE_LIMIT: &str = "^\\p{L}{400}$";

/// Building the validator compiles none of its patterns: it compiles each the first time a
/// value reaches it, and keeps it.
#[test]
fn should_compile_a_pattern_only_when_a_value_first_reaches_it() {
    let validator = validator_of(&json!({
        "type": "object",
        "properties": {
            "a": { "type": "string", "pattern": "^a+$" },
            "b": { "type": "string", "pattern": "^b+$" },
        },
    }));
    assert_eq!(validator.attempted_patterns(), 0);

    assert!(first_violation(&validator, &json!({ "a": "aa" })).is_none());
    assert_eq!(validator.attempted_patterns(), 1);

    let violation = first_violation(&validator, &json!({ "a": "aa", "b": "c" }))
        .expect("expected `b` not to match");
    assert_eq!(violation.keyword(), "pattern");
    assert_eq!(validator.attempted_patterns(), 2);
}

/// A string reaching a pattern the `regex` crate does not compile is refused as the JSON
/// Schema validator refuses a value reaching a `$ref` target that holds one: with a `format`
/// violation naming the pattern as a `regex` it does not compile. A value that never reaches
/// the pattern is checked as usual.
#[test]
fn should_refuse_a_string_reaching_a_pattern_that_does_not_compile() {
    let validator = validator_of(&with_value(
        &json!({ "type": "string", "pattern": PATTERN_BEYOND_THE_SIZE_LIMIT }),
    ));
    assert!(first_violation(&validator, &json!({})).is_none());
    assert_eq!(validator.attempted_patterns(), 0);

    let violation =
        first_violation(&validator, &json!({ "value": "a" })).expect("expected a refusal");
    assert_eq!(violation.keyword(), "format");
    assert_eq!(
        violation.params(),
        &Value::from(json!({ "format": "regex" }))
    );
    assert_eq!(
        violation.error_summary(),
        format!(
            "{} is not a \"regex\"",
            json!(PATTERN_BEYOND_THE_SIZE_LIMIT)
        )
    );
    assert_eq!(violation.schema_path(), "/properties/value/pattern");

    // The result is kept, and refuses the next string reaching the pattern too
    assert_eq!(validator.attempted_patterns(), 1);
    assert_eq!(
        first_violation(&validator, &json!({ "value": "b" })).map(|v| v.keyword().to_string()),
        Some("format".to_string())
    );

    // A value of another type fails `type` before it reaches the pattern
    assert_eq!(
        first_violation(&validator, &json!({ "value": 1 })).map(|v| v.keyword().to_string()),
        Some("type".to_string())
    );
}

/// Matching an element against `contains` only asks whether it matches, so a pattern the
/// `regex` crate does not compile matches no element, as a `$ref` target holding one matches
/// none under the JSON Schema validator: the array fails `contains`, with no other error.
#[test]
fn should_match_no_element_against_a_contains_pattern_that_does_not_compile() {
    let validator = validator_of(&with_value(&json!({
        "type": "array",
        "contains": { "type": "string", "pattern": PATTERN_BEYOND_THE_SIZE_LIMIT },
    })));
    assert!(first_violation(&validator, &json!({})).is_none());
    let violation =
        first_violation(&validator, &json!({ "value": ["a"] })).expect("expected a refusal");
    assert_eq!(violation.keyword(), "contains");
}

/// A pattern is compiled as the JSON Schema validator compiles it, with its ECMA escapes
/// rewritten: `\d` is an ASCII digit and `\cA` the control character U+0001.
#[test]
fn should_compile_a_pattern_with_its_ecma_escapes_rewritten() {
    let validator = validator_of(&json!({
        "type": "object",
        "properties": {
            "digit": { "type": "string", "pattern": "^\\d$" },
            "control": { "type": "string", "pattern": "^\\cA$" },
        },
    }));
    for (document, accepted) in [
        (json!({ "digit": "3" }), true),
        (json!({ "digit": "\u{663}" }), false),
        (json!({ "control": "\u{1}" }), true),
        (json!({ "control": "A" }), false),
    ] {
        assert_eq!(
            first_violation(&validator, &document).is_none(),
            accepted,
            "{document}"
        );
    }
}
