//! `maxBytes`: the property keyword that bounds a string by its UTF-8 length,
//! which plain JSON Schema cannot count (`maxLength` counts characters).
//!
//! The grammar is the v3 document meta-schema's (protocol version 14) and the
//! parse is `apply_max_bytes` 0, which the tables select from protocol version
//! 14 only and which folds the bound into the string's `StringPropertySizes`.
//! The write-time check is `validate_max_bytes_properties`, which
//! `DataContract::validate_document_properties` runs after the JSON schema
//! validation.

use super::typed_array_test_helpers::{
    expect_json_schema_error, expect_structure_error, parse_dispatched,
};
use super::*;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::methods::DocumentTypeBasicMethods;
use crate::data_contract::document_type::StringPropertySizes;
use crate::validation::SimpleConsensusValidationResult;
use platform_value::platform_value;
use rand::rngs::StdRng;
use rand::SeedableRng;

/// A document type with a string `note` (declaring `note_max_bytes` when
/// given), a typed string array `tags` whose elements take at most 8 bytes,
/// and an object `meta` holding a string `meta.tag` of at most 4 bytes.
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
                    "tag": { "type": "string", "maxLength": 16, "maxBytes": 4, "position": 0 }
                },
                "additionalProperties": false
            }
        },
        "additionalProperties": false
    })
}

/// A document type with the one property `value` declared by `value`.
fn schema_with(value: Value) -> Value {
    platform_value!({
        "type": "object",
        "properties": { "value": value },
        "additionalProperties": false
    })
}

fn parse(schema: Value) -> DocumentType {
    parse_dispatched(schema, PlatformVersion::latest(), true).expect("the schema parses")
}

fn property_type(document_type: &DocumentType, path: &str) -> DocumentPropertyType {
    document_type
        .as_ref()
        .flattened_properties()
        .get(path)
        .unwrap_or_else(|| panic!("{path} is parsed"))
        .property_type
        .clone()
}

/// The `maxBytes` a string, or the string elements of a typed array, carry.
fn max_bytes_of(property_type: &DocumentPropertyType) -> Option<u16> {
    match property_type {
        DocumentPropertyType::String(sizes) => sizes.max_bytes,
        DocumentPropertyType::TypedArray(typed_array) => match typed_array.item_type.as_ref() {
            DocumentPropertyType::String(sizes) => sizes.max_bytes,
            other => panic!("expected string elements, got {other:?}"),
        },
        other => panic!("expected a string, got {other:?}"),
    }
}

fn first_basic_error(result: SimpleConsensusValidationResult) -> BasicError {
    match result.errors.into_iter().next() {
        Some(ConsensusError::BasicError(error)) => error,
        other => panic!("expected a basic error, got {other:?}"),
    }
}

// ================================================================
//  Parse
// ================================================================

#[test]
fn should_fold_max_bytes_into_the_sizes_of_strings_and_of_typed_string_elements() {
    let document_type = parse(schema(Some(Value::U64(8))));

    assert_eq!(
        property_type(&document_type, "note"),
        DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: Some(64),
            max_bytes: Some(8),
        })
    );
    assert_eq!(
        max_bytes_of(&property_type(&document_type, "meta.tag")),
        Some(4)
    );
    // Declared on the items, carried by the element type, bounding every element
    assert_eq!(
        max_bytes_of(&property_type(&document_type, "tags")),
        Some(8)
    );

    let without = parse(schema(None));
    assert_eq!(max_bytes_of(&property_type(&without, "note")), None);
}

#[test]
fn should_refuse_max_bytes_on_a_property_that_is_not_a_string() {
    let schema = schema_with(platform_value!({
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
    let schema = schema_with(platform_value!({
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
        "maxBytes on a typed array belongs on its items, where it bounds every element",
    );
}

#[test]
fn should_refuse_max_bytes_on_elements_that_are_not_strings() {
    let schema = schema_with(platform_value!({
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
            schema_with(platform_value!({
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
        schema_with(platform_value!({
            "type": "string",
            "maxLength": 8,
            "maxBytes": 4,
            "position": 0
        })),
        platform_version,
        false,
    )
    .expect("a parse predating maxBytes ignores it");
    assert_eq!(max_bytes_of(&property_type(&document_type, "value")), None);
}

// ================================================================
//  Sizes
// ================================================================

/// `maxLength` alone allows four bytes a character; a declared `maxBytes` is
/// the real bound, for size estimates and for the characters that fit.
#[test]
fn should_bound_the_sizes_of_a_string_by_its_max_bytes() {
    let platform_version = PlatformVersion::latest();
    let string = |max_length: Option<u16>, max_bytes: Option<u16>| {
        DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length,
            max_bytes,
        })
    };

    for (max_length, max_bytes, byte_size, size) in [
        (Some(64), None, 256, 64),
        (Some(64), Some(16), 16, 16),
        (Some(4), Some(100), 16, 4),
        (None, Some(16), 16, 16),
        (None, None, u16::MAX, 16383),
    ] {
        let property_type = string(max_length, max_bytes);
        assert_eq!(
            property_type
                .max_byte_size(platform_version)
                .expect("a byte size"),
            Some(byte_size),
            "{max_length:?} / {max_bytes:?}"
        );
        assert_eq!(
            property_type.max_size(),
            Some(size),
            "{max_length:?} / {max_bytes:?}"
        );
    }
}

/// Random documents of a type with a byte cap stay within it, so strategy
/// tests and random fixtures produce documents consensus accepts.
#[test]
fn should_generate_random_strings_within_their_max_bytes() {
    let document_type = parse(schema(Some(Value::U64(8))));
    let mut rng = StdRng::seed_from_u64(7);
    for path in ["note", "meta.tag", "tags"] {
        let property_type = property_type(&document_type, path);
        let max_bytes = max_bytes_of(&property_type).expect("a byte cap") as usize;
        for _ in 0..50 {
            let values = match property_type.random_value(&mut rng) {
                Value::Array(elements) => elements,
                value => vec![value],
            };
            for value in values {
                let text = value.as_text().expect("a string");
                assert!(text.len() <= max_bytes, "{path}: {text:?}");
            }
        }
    }
}

// ================================================================
//  Write time
// ================================================================

#[test]
fn should_refuse_a_string_over_its_max_bytes_by_its_utf8_length() {
    let document_type = parse(schema(Some(Value::U64(8))));
    let platform_version = PlatformVersion::latest();

    // Eight bytes in one-byte or two-byte characters fit; five two-byte characters do
    // not, though they are well within maxLength
    for note in ["abcdefgh", "éééé"] {
        let result = document_type
            .validate_max_bytes_properties(&platform_value!({ "note": note }), platform_version)
            .expect("validation executes");
        assert!(result.is_valid(), "{note}: {:?}", result.errors);
    }
    let result = document_type
        .validate_max_bytes_properties(&platform_value!({ "note": "ééééé" }), platform_version)
        .expect("validation executes");
    assert!(matches!(
        first_basic_error(result),
        BasicError::DocumentPropertyMaxBytesExceededError(e)
            if e.property() == "note" && e.byte_length() == 10 && e.max_bytes() == 8
    ));

    // A nested string is named by its dotted path
    let result = document_type
        .validate_max_bytes_properties(
            &platform_value!({ "meta": { "tag": "ééé" } }),
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
        .validate_max_bytes_properties(&platform_value!({}), platform_version)
        .expect("validation executes")
        .is_valid());
}

#[test]
fn should_name_the_element_of_a_typed_array_over_its_max_bytes() {
    let document_type = parse(schema(None));
    let result = document_type
        .validate_max_bytes_properties(
            &platform_value!({ "tags": ["short", "ééééé", "ok"] }),
            PlatformVersion::latest(),
        )
        .expect("validation executes");
    assert!(matches!(
        first_basic_error(result),
        BasicError::DocumentPropertyMaxBytesExceededError(e)
            if e.property() == "tags[1]" && e.byte_length() == 10 && e.max_bytes() == 8
    ));
}

#[test]
fn should_check_nothing_before_protocol_version_14() {
    // A type parsed with the keyword, judged under protocol version 13's method table
    let document_type = parse(schema(Some(Value::U64(1))));
    let platform_version = PlatformVersion::get(13).expect("protocol version 13");
    assert!(document_type
        .validate_max_bytes_properties(&platform_value!({ "note": "too long" }), platform_version)
        .expect("validation executes")
        .is_valid());
}
