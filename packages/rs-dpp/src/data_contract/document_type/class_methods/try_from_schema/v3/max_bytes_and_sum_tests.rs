//! `maxBytes` and `sumOfProperties`: the two property keywords that bound a
//! value by something plain JSON Schema cannot count, the UTF-8 length of a
//! string and the total of an object's integer members.
//!
//! The grammar is the v3 document meta-schema's (protocol version 14) and the
//! parses are `apply_max_bytes` 0 and `apply_sum_of_properties` 0, which the
//! tables select from protocol version 14 only. The write-time checks are
//! `validate_max_bytes_properties` and `validate_sum_of_properties`.

use super::typed_array_test_helpers::{
    expect_json_schema_error, expect_structure_error, parse_dispatched,
};
use super::*;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::methods::DocumentTypeBasicMethods;
use crate::validation::SimpleConsensusValidationResult;
use platform_value::platform_value;

/// A document type with a string `note` (declaring `note_max_bytes` when
/// given), a typed string array `tags`, and an object `meta` holding a string
/// `meta.tag` and the object `meta.split` of two percentages, `a` and `b`.
fn schema(note_max_bytes: Option<Value>) -> Value {
    let mut note = platform_value!({ "type": "string", "maxLength": 64, "position": 0 });
    if let Some(max_bytes) = note_max_bytes {
        note.insert("maxBytes".to_string(), max_bytes)
            .expect("note is a map");
    }
    platform_value!({
        "type": "object",
        "properties": {
            "note": note,
            "tags": {
                "type": "array",
                "maxItems": 4,
                "items": { "type": "string", "maxLength": 16, "maxBytes": 8 },
                "position": 1
            },
            "meta": {
                "type": "object",
                "position": 2,
                "properties": {
                    "tag": { "type": "string", "maxLength": 16, "maxBytes": 4, "position": 0 },
                    "split": split_schema(100)
                },
                "additionalProperties": false
            }
        },
        "additionalProperties": false
    })
}

/// Two required percentages that must add up to `total`.
fn split_schema(total: i64) -> Value {
    platform_value!({
        "type": "object",
        "position": 1,
        "properties": {
            "a": { "type": "integer", "minimum": 0, "maximum": 100, "position": 0 },
            "b": { "type": "integer", "minimum": 0, "maximum": 100, "position": 1 }
        },
        "required": ["a", "b"],
        "additionalProperties": false,
        "sumOfProperties": total
    })
}

/// A document type with the object `split` declared by `split`.
fn schema_with_split(split: Value) -> Value {
    platform_value!({
        "type": "object",
        "properties": { "split": split },
        "additionalProperties": false
    })
}

fn parse(schema: Value) -> DocumentType {
    parse_dispatched(schema, PlatformVersion::latest(), true).expect("the schema parses")
}

fn data(value: Value) -> BTreeMap<String, Value> {
    value.into_btree_string_map().expect("a map of properties")
}

fn first_basic_error(result: SimpleConsensusValidationResult) -> BasicError {
    match result.errors.into_iter().next() {
        Some(ConsensusError::BasicError(error)) => error,
        other => panic!("expected a basic error, got {other:?}"),
    }
}

// ================================================================
//  maxBytes: parse
// ================================================================

#[test]
fn should_parse_max_bytes_on_strings_and_on_the_items_of_a_typed_string_array() {
    let document_type = parse(schema(Some(Value::U64(8))));
    let document_type = document_type.as_ref();
    let flattened = document_type.flattened_properties();

    assert_eq!(flattened.get("note").expect("note").max_bytes, Some(8));
    assert_eq!(
        flattened.get("meta.tag").expect("meta.tag").max_bytes,
        Some(4)
    );
    // Declared on the items, held on the array property, bounding every element
    assert_eq!(flattened.get("tags").expect("tags").max_bytes, Some(8));
    assert_eq!(flattened.get("meta.split.a").expect("a").max_bytes, None);
}

#[test]
fn should_refuse_max_bytes_on_a_property_that_is_not_a_string() {
    let schema = schema_with_split(platform_value!({
        "type": "integer",
        "minimum": 0,
        "maximum": 100,
        "maxBytes": 1,
        "position": 0
    }));
    let error = expect_json_schema_error(parse_dispatched(
        schema.clone(),
        PlatformVersion::latest(),
        true,
    ));
    assert_eq!(error.keyword(), "const", "{error:?}");

    // A parse that skips the meta-schema still refuses it
    expect_structure_error(
        parse_dispatched(schema, PlatformVersion::latest(), false),
        "maxBytes is only allowed on string properties",
    );
}

#[test]
fn should_refuse_max_bytes_on_a_typed_array_itself() {
    let schema = schema_with_split(platform_value!({
        "type": "array",
        "maxItems": 4,
        "maxBytes": 8,
        "items": { "type": "string", "maxLength": 16 },
        "position": 0
    }));
    expect_json_schema_error(parse_dispatched(
        schema.clone(),
        PlatformVersion::latest(),
        true,
    ));
    expect_structure_error(
        parse_dispatched(schema, PlatformVersion::latest(), false),
        "belongs on its items",
    );
}

#[test]
fn should_refuse_max_bytes_on_elements_that_are_not_strings() {
    let schema = schema_with_split(platform_value!({
        "type": "array",
        "maxItems": 4,
        "items": { "type": "integer", "minimum": 0, "maximum": 9, "maxBytes": 1 },
        "position": 0
    }));
    expect_json_schema_error(parse_dispatched(
        schema.clone(),
        PlatformVersion::latest(),
        true,
    ));
    expect_structure_error(
        parse_dispatched(schema, PlatformVersion::latest(), false),
        "only allowed on string elements",
    );
}

#[test]
fn should_refuse_a_max_bytes_of_zero_or_below_min_length() {
    let error = expect_json_schema_error(parse_dispatched(
        schema(Some(Value::U64(0))),
        PlatformVersion::latest(),
        true,
    ));
    assert_eq!(error.keyword(), "minimum", "{error:?}");

    // Two characters are at least two bytes, so a one-byte bound refuses every value
    expect_structure_error(
        parse_dispatched(
            schema_with_split(platform_value!({
                "type": "string",
                "minLength": 2,
                "maxLength": 8,
                "maxBytes": 1,
                "position": 0
            })),
            PlatformVersion::latest(),
            true,
        ),
        "maxBytes 1 is below minLength 2",
    );
}

#[test]
fn should_ignore_max_bytes_before_protocol_version_14() {
    // Protocol version 13's meta-schema refuses the keyword; a parse that skips it
    // (a contract read back from state) ignores it, as it always did
    let platform_version = PlatformVersion::get(13).expect("protocol version 13");
    let document_type = parse_dispatched(
        schema_with_split(platform_value!({
            "type": "string",
            "maxLength": 8,
            "maxBytes": 4,
            "position": 0
        })),
        platform_version,
        false,
    )
    .expect("a parse predating maxBytes ignores it");
    assert_eq!(
        document_type
            .as_ref()
            .flattened_properties()
            .get("split")
            .expect("split")
            .max_bytes,
        None
    );
}

// ================================================================
//  maxBytes: write time
// ================================================================

#[test]
fn should_refuse_a_string_over_its_max_bytes_by_its_utf8_length() {
    let document_type = parse(schema(Some(Value::U64(8))));
    let platform_version = PlatformVersion::latest();

    // Eight bytes in one-byte or two-byte characters fit; the same four characters with
    // a fifth two-byte one do not, though every one of them is well within maxLength
    for note in ["abcdefgh", "éééé"] {
        let result = document_type
            .validate_max_bytes_properties(
                &data(platform_value!({ "note": note })),
                platform_version,
            )
            .expect("validation executes");
        assert!(result.is_valid(), "{note}: {:?}", result.errors);
    }
    let result = document_type
        .validate_max_bytes_properties(
            &data(platform_value!({ "note": "ééééé" })),
            platform_version,
        )
        .expect("validation executes");
    assert!(matches!(
        first_basic_error(result),
        BasicError::DocumentPropertyMaxBytesExceededError(e)
            if e.property() == "note" && e.byte_length() == 10 && e.max_bytes() == 8
    ));

    // A nested string is named by its dotted path
    let result = document_type
        .validate_max_bytes_properties(
            &data(platform_value!({ "meta": { "tag": "ééé" } })),
            platform_version,
        )
        .expect("validation executes");
    assert!(matches!(
        first_basic_error(result),
        BasicError::DocumentPropertyMaxBytesExceededError(e)
            if e.property() == "meta.tag" && e.byte_length() == 6 && e.max_bytes() == 4
    ));

    // An absent property is not checked
    assert!(document_type
        .validate_max_bytes_properties(&BTreeMap::new(), platform_version)
        .expect("validation executes")
        .is_valid());
}

#[test]
fn should_name_the_element_of_a_typed_array_over_its_max_bytes() {
    let document_type = parse(schema(None));
    let result = document_type
        .validate_max_bytes_properties(
            &data(platform_value!({ "tags": ["short", "ééééé", "ok"] })),
            PlatformVersion::latest(),
        )
        .expect("validation executes");
    assert!(matches!(
        first_basic_error(result),
        BasicError::DocumentPropertyMaxBytesExceededError(e)
            if e.property() == "tags[1]" && e.byte_length() == 10 && e.max_bytes() == 8
    ));
}

// ================================================================
//  sumOfProperties: parse
// ================================================================

#[test]
fn should_parse_sum_of_properties_onto_the_object() {
    let document_type = parse(schema(None));
    let document_type = document_type.as_ref();
    let DocumentPropertyType::Object(meta) = &document_type
        .properties()
        .get("meta")
        .expect("meta")
        .property_type
    else {
        panic!("meta is an object");
    };
    assert_eq!(
        meta.get("split").expect("split").sum_of_properties,
        Some(100)
    );
    assert_eq!(meta.get("tag").expect("tag").sum_of_properties, None);
}

#[test]
fn should_refuse_sum_of_properties_on_a_property_that_is_not_an_object() {
    let schema = schema_with_split(platform_value!({
        "type": "integer",
        "minimum": 0,
        "maximum": 100,
        "sumOfProperties": 100,
        "position": 0
    }));
    expect_json_schema_error(parse_dispatched(
        schema.clone(),
        PlatformVersion::latest(),
        true,
    ));
    expect_structure_error(
        parse_dispatched(schema, PlatformVersion::latest(), false),
        "only allowed on object properties",
    );
}

#[test]
fn should_refuse_sum_of_properties_over_members_that_are_not_required_integers() {
    for (members, required, needle) in [
        (
            platform_value!({
                "a": { "type": "integer", "minimum": 0, "maximum": 100, "position": 0 },
                "b": { "type": "string", "maxLength": 3, "position": 1 }
            }),
            platform_value!(["a", "b"]),
            "member \"b\" is not one",
        ),
        (
            platform_value!({
                "a": { "type": "integer", "minimum": 0, "maximum": 100, "position": 0 },
                "b": { "type": "integer", "minimum": 0, "maximum": 100, "position": 1 }
            }),
            platform_value!(["a"]),
            "member \"b\" must be required",
        ),
    ] {
        expect_structure_error(
            parse_dispatched(
                schema_with_split(platform_value!({
                    "type": "object",
                    "position": 0,
                    "properties": members,
                    "required": required,
                    "additionalProperties": false,
                    "sumOfProperties": 100
                })),
                PlatformVersion::latest(),
                true,
            ),
            needle,
        );
    }
}

#[test]
fn should_refuse_a_total_the_members_cannot_reach() {
    // Two members of 0 to 100 add up to 0 through 200
    for total in [-1, 201] {
        let mut split = split_schema(total);
        split
            .insert("position".to_string(), Value::U64(0))
            .expect("split is a map");
        expect_structure_error(
            parse_dispatched(schema_with_split(split), PlatformVersion::latest(), true),
            "the members add up to between 0 and 200",
        );
    }
    for total in [0, 200] {
        let mut split = split_schema(total);
        split
            .insert("position".to_string(), Value::U64(0))
            .expect("split is a map");
        parse_dispatched(schema_with_split(split), PlatformVersion::latest(), true)
            .unwrap_or_else(|e| panic!("{total} is reachable: {e:?}"));
    }
}

#[test]
fn should_ignore_sum_of_properties_before_protocol_version_14() {
    let platform_version = PlatformVersion::get(13).expect("protocol version 13");
    let mut split = split_schema(100);
    split
        .insert("position".to_string(), Value::U64(0))
        .expect("split is a map");
    let document_type = parse_dispatched(schema_with_split(split), platform_version, false)
        .expect("a parse predating sumOfProperties ignores it");
    assert_eq!(
        document_type
            .as_ref()
            .properties()
            .get("split")
            .expect("split")
            .sum_of_properties,
        None
    );
}

// ================================================================
//  sumOfProperties: write time
// ================================================================

#[test]
fn should_refuse_an_object_whose_members_miss_the_total() {
    let document_type = parse(schema(None));
    let platform_version = PlatformVersion::latest();

    let result = document_type
        .validate_sum_of_properties(
            &data(platform_value!({ "meta": { "split": { "a": 30, "b": 70 } } })),
            platform_version,
        )
        .expect("validation executes");
    assert!(result.is_valid(), "{:?}", result.errors);

    for (a, b) in [(30, 60), (0, 0), (100, 100)] {
        let result = document_type
            .validate_sum_of_properties(
                &data(platform_value!({ "meta": { "split": { "a": a, "b": b } } })),
                platform_version,
            )
            .expect("validation executes");
        assert!(
            matches!(
                first_basic_error(result),
                BasicError::DocumentPropertySumMismatchError(e)
                    if e.property() == "meta.split"
                        && e.expected_sum() == 100
                        && e.actual_sum() == a + b
            ),
            "{a} + {b}"
        );
    }

    // An absent object is not checked; whether it may be absent is `required`'s
    for absent in [
        platform_value!({}),
        platform_value!({ "meta": { "tag": "x" } }),
    ] {
        assert!(document_type
            .validate_sum_of_properties(&data(absent), platform_version)
            .expect("validation executes")
            .is_valid());
    }
}

#[test]
fn should_check_nothing_before_protocol_version_14() {
    // A type parsed with the keywords, judged under protocol version 13's method table
    let document_type = parse(schema(Some(Value::U64(1))));
    let platform_version = PlatformVersion::get(13).expect("protocol version 13");
    let properties = data(platform_value!({
        "note": "too long",
        "meta": { "split": { "a": 1, "b": 1 } }
    }));
    assert!(document_type
        .validate_max_bytes_properties(&properties, platform_version)
        .expect("validation executes")
        .is_valid());
    assert!(document_type
        .validate_sum_of_properties(&properties, platform_version)
        .expect("validation executes")
        .is_valid());
}
