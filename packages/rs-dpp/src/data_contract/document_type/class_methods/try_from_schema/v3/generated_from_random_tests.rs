//! `generatedFrom` with a random function (protocol version 15): `sys.random.uniform`
//! draws an integer property from its `minimum` to its `maximum` once the block's transitions
//! are accepted. The parse is `apply_generated_from` 1 and the checks against the rest of the
//! type are `validate_generated_from_declarations`. A create writes the placeholder and a
//! replace keeps the stored value (`generate_random_properties`), a client leaves the value
//! out (`regenerate_generated_properties`), and a proof verifier takes it from the proved
//! document (`adopt_state_generated_properties`).

use super::typed_array_test_helpers::{
    expect_json_schema_error, expect_structure_error, parse_dispatched,
};
use super::*;
use crate::data_contract::document_type::methods::DocumentTypeBasicMethods;
use crate::data_contract::document_type::{
    GeneratedFrom, RandomFunction, RandomRange, SystemFunction,
};
use platform_value::platform_value;

const UNIFORM: &str = "sys.random.uniform";

fn rarity_property() -> Value {
    platform_value!({
        "type": "integer",
        "minimum": 1,
        "maximum": 100,
        "generatedFrom": { "function": UNIFORM },
        "position": 1
    })
}

/// A `card` type: its `name`, and a `rarity` drawn from 1 to 100.
fn card_schema() -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "name": { "type": "string", "maxLength": 32, "position": 0 },
            "rarity": rarity_property()
        },
        "required": ["name", "rarity"],
        "additionalProperties": false
    })
}

/// [`card_schema`] with `rarity` replaced by `rarity`.
fn card_schema_with_rarity(rarity: Value) -> Value {
    let mut schema = card_schema();
    schema
        .get_mut("properties")
        .expect("properties")
        .expect("properties are present")
        .insert("rarity".to_string(), rarity)
        .expect("properties are a map");
    schema
}

/// [`card_schema`] with `key` set to `value` at the type level.
fn card_schema_with(key: &str, value: Value) -> Value {
    let mut schema = card_schema();
    schema
        .insert(key.to_string(), value)
        .expect("the schema is a map");
    schema
}

fn parse(schema: Value) -> DocumentType {
    parse_dispatched(schema, PlatformVersion::latest(), true).expect("the schema parses")
}

/// Both parses refuse the declaration: the validating one natively or with the parser's
/// structure error, the one that skips the schema check with the parser's.
fn expect_refused(schema: Value, needle: &str) {
    assert!(
        parse_dispatched(schema.clone(), PlatformVersion::latest(), true).is_err(),
        "a validating parse refuses it ({needle})"
    );
    expect_structure_error(
        parse_dispatched(schema, PlatformVersion::latest(), false),
        needle,
    );
}

fn data(entries: Vec<(&str, Value)>) -> BTreeMap<String, Value> {
    entries
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect()
}

#[test]
fn should_parse_sys_random_uniform_onto_a_bounded_integer_property_from_protocol_version_15() {
    let expected = Some(GeneratedFrom {
        function: SystemFunction::Random(RandomFunction::Uniform),
        params: vec![],
        range: Some(RandomRange {
            minimum: 1,
            maximum: 100,
        }),
    });
    let rarity = |document_type: DocumentType| {
        document_type
            .as_ref()
            .flattened_properties()
            .get("rarity")
            .expect("rarity is parsed")
            .generated_from
            .clone()
    };
    assert_eq!(rarity(parse(card_schema())), expected);

    // `params` may be written out, empty
    let explicit = card_schema_with_rarity(platform_value!({
        "type": "integer",
        "minimum": 1,
        "maximum": 100,
        "generatedFrom": { "function": UNIFORM, "params": [] },
        "position": 1
    }));
    assert_eq!(rarity(parse(explicit)), expected);

    // A signed range parses too
    let signed = card_schema_with_rarity(platform_value!({
        "type": "integer",
        "minimum": -5,
        "maximum": 5,
        "generatedFrom": { "function": UNIFORM },
        "position": 1
    }));
    assert_eq!(
        parse(signed)
            .as_ref()
            .flattened_properties()
            .get("rarity")
            .expect("rarity is parsed")
            .generated_from
            .as_ref()
            .and_then(|generated_from| generated_from.range),
        Some(RandomRange {
            minimum: -5,
            maximum: 5
        })
    );
}

#[test]
fn should_refuse_sys_random_uniform_before_protocol_version_15() {
    let platform_version = PlatformVersion::get(14).expect("protocol version 14");
    // Protocol version 14's meta-schema refuses the function, and its parser refuses a
    // generated integer property
    expect_json_schema_error(parse_dispatched(card_schema(), platform_version, true));
    expect_structure_error(
        parse_dispatched(card_schema(), platform_version, false),
        "only allowed on string properties",
    );
}

#[test]
fn should_refuse_sys_random_uniform_on_a_property_without_a_bounded_range() {
    let generated = platform_value!({ "function": UNIFORM });
    let refuse = |rarity: Value, needle: &str| {
        expect_refused(card_schema_with_rarity(rarity), needle);
    };
    refuse(
        platform_value!({ "type": "integer", "maximum": 100, "generatedFrom": generated.clone(), "position": 1 }),
        "must declare both a minimum and a maximum",
    );
    refuse(
        platform_value!({ "type": "integer", "minimum": 1, "generatedFrom": generated.clone(), "position": 1 }),
        "must declare both a minimum and a maximum",
    );
    refuse(
        platform_value!({ "type": "integer", "minimum": 10, "maximum": 1, "generatedFrom": generated.clone(), "position": 1 }),
        "may not be above its maximum",
    );
    for (keyword, value) in [
        ("enum", platform_value!([1, 2, 3])),
        ("multipleOf", platform_value!(2)),
        ("exclusiveMaximum", platform_value!(100)),
    ] {
        let mut rarity = platform_value!({ "type": "integer", "minimum": 1, "maximum": 100, "generatedFrom": generated.clone(), "position": 1 });
        rarity
            .insert(keyword.to_string(), value)
            .expect("rarity is a map");
        refuse(rarity, &format!("may not declare {keyword}"));
    }
    // A string has no range to draw from
    refuse(
        platform_value!({ "type": "string", "maxLength": 8, "generatedFrom": generated, "position": 1 }),
        "only allowed on an integer property of at most 64 bits",
    );
    // The function takes no parameters
    refuse(
        platform_value!({ "type": "integer", "minimum": 1, "maximum": 100, "generatedFrom": { "function": UNIFORM, "params": ["name"] }, "position": 1 }),
        "takes 0 parameter(s)",
    );
}

#[test]
fn should_admit_sys_random_uniform_in_an_ordinary_index_and_refuse_it_in_a_unique_one() {
    let indices = |unique: bool| platform_value!([{ "name": "byRarity", "properties": [{ "rarity": "asc" }], "unique": unique }]);
    let document_type = parse(card_schema_with("indices", indices(false)));
    assert!(document_type.as_ref().indexes().contains_key("byRarity"));

    expect_refused(
        card_schema_with("indices", indices(true)),
        "may not be in unique index \"byRarity\"",
    );
}

#[test]
fn should_refuse_sys_random_uniform_on_a_type_that_keeps_no_document_or_keeps_history() {
    expect_refused(
        card_schema_with("documentsKeepHistory", Value::Bool(true)),
        "would keep the placeholder",
    );

    let index_only = platform_value!({
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "indices": [
            {
                "name": "byRarity",
                "properties": [{ "name": "asc" }, { "rarity": "asc" }],
                "terminal": "$ownerId"
            }
        ],
        "properties": {
            "name": { "type": "string", "maxLength": 32, "position": 0 },
            "rarity": rarity_property()
        },
        "required": ["name", "rarity"],
        "additionalProperties": false
    });
    expect_refused(index_only, "an indexOnly type keeps no document");
}

#[test]
fn should_write_the_placeholder_on_a_create_and_refuse_a_sent_value() {
    let document_type = parse(card_schema());

    let mut created = data(vec![("name", Value::Text("Ace".to_string()))]);
    assert_eq!(
        document_type
            .as_ref()
            .generate_random_properties(&mut created, None),
        None
    );
    // The placeholder is the range's minimum, stored at the property's width
    assert_eq!(created.get("rarity"), Some(&Value::U8(1)));

    // A client cannot pick the value
    let mut sent = data(vec![
        ("name", Value::Text("Ace".to_string())),
        ("rarity", Value::U8(100)),
    ]);
    assert!(document_type
        .as_ref()
        .generate_random_properties(&mut sent, None)
        .is_some());
}

#[test]
fn should_keep_the_stored_value_on_a_replace() {
    let document_type = parse(card_schema());
    let stored = data(vec![
        ("name", Value::Text("Ace".to_string())),
        ("rarity", Value::U8(73)),
    ]);

    // Left out, the stored value is written back
    let mut replaced = data(vec![("name", Value::Text("King".to_string()))]);
    assert_eq!(
        document_type
            .as_ref()
            .generate_random_properties(&mut replaced, Some(&stored)),
        None
    );
    assert_eq!(replaced.get("rarity"), Some(&Value::U8(73)));

    // Sent, it must be the stored value, at any width
    let mut kept = data(vec![
        ("name", Value::Text("King".to_string())),
        ("rarity", Value::U64(73)),
    ]);
    assert_eq!(
        document_type
            .as_ref()
            .generate_random_properties(&mut kept, Some(&stored)),
        None
    );
    let mut rerolled = data(vec![
        ("name", Value::Text("King".to_string())),
        ("rarity", Value::U8(99)),
    ]);
    assert!(document_type
        .as_ref()
        .generate_random_properties(&mut rerolled, Some(&stored))
        .is_some());
}

#[test]
fn should_leave_the_value_to_the_platform_on_a_client() {
    let document_type = parse(card_schema());
    let platform_version = PlatformVersion::latest();

    // The transition builders remove it, whatever the document holds
    let mut properties = data(vec![
        ("name", Value::Text("Ace".to_string())),
        ("rarity", Value::U8(73)),
    ]);
    document_type
        .regenerate_generated_properties(&mut properties, platform_version)
        .expect("the regeneration runs");
    assert_eq!(
        properties,
        data(vec![("name", Value::Text("Ace".to_string()))])
    );

    // The document check does not refuse it left out, nor held
    let check = |properties: BTreeMap<String, Value>| {
        document_type
            .as_ref()
            .validate_generated_from_properties(&Value::from(properties), platform_version)
            .expect("the check runs")
    };
    assert!(check(data(vec![("name", Value::Text("Ace".to_string()))])).is_valid());
    assert!(check(data(vec![
        ("name", Value::Text("Ace".to_string())),
        ("rarity", Value::U8(73))
    ]))
    .is_valid());
}

#[test]
fn should_take_the_proved_value_into_a_rebuilt_document() {
    let document_type = parse(card_schema());
    let proved = data(vec![
        ("name", Value::Text("Ace".to_string())),
        ("rarity", Value::U8(73)),
    ]);

    let mut rebuilt = data(vec![("name", Value::Text("Ace".to_string()))]);
    document_type
        .as_ref()
        .adopt_state_generated_properties(&mut rebuilt, &proved);
    assert_eq!(rebuilt, proved);
}
