//! The `transient` doctype keyword under generation 3: every entry names a
//! top-level property, and no index reads a transient value. Both are
//! registration lints (`full_validation` only), so a stored contract stays
//! readable, and generation 2 (protocol version 13) keeps accepting both.

use super::immutable_tests::{expect_structure_error, parse_dispatched};
use super::*;
use platform_value::platform_value;

/// A type (parsed as `post`) with a short `code`, a long `body` and a required `meta`
/// object around a required `tag`, with `extra` set on top.
fn note_schema_with(extra: Value) -> Value {
    let mut schema = platform_value!({
        "type": "object",
        "properties": {
            "code": { "type": "string", "maxLength": 32, "position": 0 },
            "body": { "type": "string", "maxLength": 500, "position": 1 },
            "meta": {
                "type": "object",
                "position": 2,
                "properties": {
                    "tag": { "type": "string", "maxLength": 30, "position": 0 }
                },
                "required": ["tag"],
                "additionalProperties": false
            }
        },
        "required": ["code", "meta"],
        "additionalProperties": false
    });
    if let Value::Map(entries) = extra {
        for (key, value) in entries {
            let key = key.to_text().expect("a text key");
            schema.set_value(&key, value).expect("doctype key applies");
        }
    }
    schema
}

#[test]
fn should_accept_transient_top_level_properties_and_objects() {
    for transient in [
        platform_value!(["body"]),
        platform_value!(["meta"]),
        platform_value!(["code", "body", "meta"]),
    ] {
        let schema = note_schema_with(platform_value!({ "transient": transient.clone() }));
        parse_dispatched(schema, PlatformVersion::latest(), true)
            .unwrap_or_else(|e| panic!("{transient:?} should register: {e}"));
    }
}

/// Drive drops transient values by top-level name, so an entry naming
/// anything else would be flagged transient and still stored.
#[test]
fn should_refuse_a_transient_entry_that_is_not_a_top_level_property() {
    for entry in ["meta.tag", "ghost", "$ownerId"] {
        let schema = note_schema_with(platform_value!({ "transient": [entry] }));
        expect_structure_error(
            parse_dispatched(schema.clone(), PlatformVersion::latest(), true),
            &format!(
                "document type \"post\" lists \"{entry}\" as transient, but it is not a \
                 top-level property of the document type"
            ),
        );

        // A stored contract is parsed without full validation and stays readable
        parse_dispatched(schema.clone(), PlatformVersion::latest(), false)
            .unwrap_or_else(|e| panic!("{entry}: the stored path should parse: {e}"));

        // Generation 2 predates the rule
        let platform_version_13 = PlatformVersion::get(13).expect("protocol version 13");
        parse_dispatched(schema, platform_version_13, true)
            .unwrap_or_else(|e| panic!("{entry}: protocol version 13 should accept it: {e}"));
    }
}

/// A transient value is never stored, so every document would sit in the
/// index's null branch and a unique index would enforce nothing.
#[test]
fn should_refuse_an_index_reading_a_transient_property_or_one_inside_a_transient_object() {
    for (transient, index_property, unique) in [
        ("code", "code", true),
        ("code", "code", false),
        ("meta", "meta.tag", true),
    ] {
        let index_properties = Value::Array(vec![Value::Map(vec![(
            Value::Text(index_property.to_string()),
            Value::Text("asc".to_string()),
        )])]);
        let schema = note_schema_with(platform_value!({
            "transient": [transient],
            "indices": [{
                "name": "byValue",
                "properties": index_properties,
                "unique": unique
            }]
        }));
        expect_structure_error(
            parse_dispatched(schema.clone(), PlatformVersion::latest(), true),
            &format!(
                "index \"byValue\" of document type \"post\" reads \"{index_property}\", which \
                 is transient or inside a transient object"
            ),
        );
        parse_dispatched(schema.clone(), PlatformVersion::latest(), false)
            .unwrap_or_else(|e| panic!("{index_property}: the stored path should parse: {e}"));
        let platform_version_13 = PlatformVersion::get(13).expect("protocol version 13");
        parse_dispatched(schema, platform_version_13, true).unwrap_or_else(|e| {
            panic!("{index_property}: protocol version 13 should accept it: {e}")
        });
    }

    // The same index over a stored property, beside a transient one, registers
    let schema = note_schema_with(platform_value!({
        "transient": ["body"],
        "indices": [{ "name": "byValue", "properties": [{ "meta.tag": "asc" }], "unique": true }]
    }));
    parse_dispatched(schema, PlatformVersion::latest(), true)
        .expect("an index over a stored property registers");
}
