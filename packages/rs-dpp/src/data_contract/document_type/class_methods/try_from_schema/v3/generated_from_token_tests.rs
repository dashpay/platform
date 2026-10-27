//! `generatedFrom` with a function that reads state (protocol version 15):
//! `sys.tokens.decimals` generates an integer property, the decimals of the
//! token an identifier property names. The parse is `apply_generated_from` 1
//! and the checks against the rest of the type are
//! `validate_generated_from_declarations`. The value is generated when the
//! document is created and kept after (`generate_state_read_properties`), a
//! client leaves it out (`regenerate_generated_properties`), and a proof
//! verifier takes it from the proved document
//! (`adopt_state_generated_properties`).

use super::typed_array_test_helpers::{
    expect_json_schema_error, expect_structure_error, parse_dispatched,
};
use super::*;
use crate::consensus::basic::BasicError;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::methods::DocumentTypeBasicMethods;
use crate::data_contract::document_type::{
    GeneratedFrom, GenerationParam, SystemFunction, TokenFunction,
};
use platform_value::platform_value;
use std::convert::Infallible;

const DECIMALS: &str = "sys.tokens.decimals";

fn identifier_property(position: u64) -> Value {
    platform_value!({
        "type": "array",
        "byteArray": true,
        "minItems": 32,
        "maxItems": 32,
        "contentMediaType": "application/x.dash.dpp.identifier",
        "position": position
    })
}

fn decimals_property(position: u64) -> Value {
    platform_value!({
        "type": "integer",
        "minimum": 0,
        "maximum": 255,
        "generatedFrom": { "function": DECIMALS, "params": ["tokenId"] },
        "position": position
    })
}

/// An `order` type: the `tokenId` it is for, and that token's `decimals`.
fn order_schema() -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "tokenId": identifier_property(0),
            "decimals": decimals_property(1),
            "label": { "type": "string", "maxLength": 32, "position": 2 }
        },
        "required": ["tokenId", "decimals"],
        "additionalProperties": false
    })
}

/// [`order_schema`] with `decimals` replaced by `decimals`.
fn order_schema_with_decimals(decimals: Value) -> Value {
    let mut schema = order_schema();
    schema
        .get_mut("properties")
        .expect("properties")
        .expect("properties are present")
        .insert("decimals".to_string(), decimals)
        .expect("properties are a map");
    schema
}

fn parse(schema: Value) -> DocumentType {
    parse_dispatched(schema, PlatformVersion::latest(), true).expect("the schema parses")
}

fn token(byte: u8) -> Value {
    Value::Identifier([byte; 32])
}

fn data(entries: Vec<(&str, Value)>) -> BTreeMap<String, Value> {
    entries
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect()
}

/// A read of the token's decimals: 6 for token 5, 18 for token 6, none for any other.
fn read_decimals(
    function: TokenFunction,
    token_id: Identifier,
) -> Result<Option<Value>, Infallible> {
    assert_eq!(function, TokenFunction::Decimals);
    Ok(match token_id.to_buffer() {
        [5, ..] => Some(Value::U8(6)),
        [6, ..] => Some(Value::U8(18)),
        _ => None,
    })
}

/// A read that must not happen: the value is kept.
fn no_read(_: TokenFunction, _: Identifier) -> Result<Option<Value>, Infallible> {
    panic!("a kept value is not read from state")
}

// ================================================================
//  Parse
// ================================================================

#[test]
fn should_parse_sys_tokens_decimals_onto_an_integer_property_from_protocol_version_15() {
    let document_type = parse(order_schema());
    assert_eq!(
        document_type
            .as_ref()
            .flattened_properties()
            .get("decimals")
            .expect("decimals is parsed")
            .generated_from,
        Some(GeneratedFrom {
            function: SystemFunction::Token(TokenFunction::Decimals),
            params: vec![GenerationParam::Property("tokenId".to_string())],
        })
    );
    // An identifier carrying a `refersTo` names the token as well
    let mut schema = order_schema();
    schema
        .get_mut("properties")
        .expect("properties")
        .expect("properties are present")
        .get_mut("tokenId")
        .expect("tokenId")
        .expect("tokenId is present")
        .insert("refersTo".to_string(), platform_value!({ "type": "token" }))
        .expect("tokenId is a map");
    parse(schema);
}

#[test]
fn should_refuse_sys_tokens_decimals_before_protocol_version_15() {
    let platform_version = PlatformVersion::get(14).expect("protocol version 14");
    // Protocol version 14's meta-schema refuses the function, and its parser refuses a
    // generated integer property
    expect_json_schema_error(parse_dispatched(order_schema(), platform_version, true));
    expect_structure_error(
        parse_dispatched(order_schema(), platform_version, false),
        "only allowed on string properties",
    );
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

#[test]
fn should_refuse_sys_tokens_decimals_on_a_property_that_cannot_hold_every_decimals() {
    let generated = platform_value!({ "function": DECIMALS, "params": ["tokenId"] });
    for (decimals, needle) in [
        (
            platform_value!({ "type": "string", "maxLength": 8, "generatedFrom": generated.clone(), "position": 1 }),
            "only allowed on an integer property that admits every value from 0 to 255",
        ),
        (
            platform_value!({ "type": "integer", "minimum": 0, "maximum": 100, "generatedFrom": generated.clone(), "position": 1 }),
            "only allowed on an integer property that admits every value from 0 to 255",
        ),
        (
            platform_value!({ "type": "integer", "minimum": 1, "maximum": 255, "generatedFrom": generated.clone(), "position": 1 }),
            "only allowed on an integer property that admits every value from 0 to 255",
        ),
        (
            platform_value!({ "type": "integer", "enum": [0, 2, 8], "generatedFrom": generated.clone(), "position": 1 }),
            "only allowed on an integer property that admits every value from 0 to 255",
        ),
        (
            platform_value!({ "type": "integer", "minimum": 0, "maximum": 255, "generatedFrom": { "function": DECIMALS, "params": ["tokenId", "label"] }, "position": 1 }),
            "takes 1 parameter(s), but params lists 2",
        ),
        (
            platform_value!({ "type": "integer", "minimum": 0, "maximum": 255, "generatedFrom": { "function": DECIMALS, "params": ["label"] }, "position": 1 }),
            "param \"label\" has type string, not identifier",
        ),
        (
            platform_value!({ "type": "integer", "minimum": 0, "maximum": 255, "generatedFrom": { "function": "sys.stringTransformations.lowercase", "params": ["label"] }, "position": 1 }),
            "only allowed on string properties for function sys.stringTransformations.lowercase",
        ),
    ] {
        expect_refused(order_schema_with_decimals(decimals), needle);
    }
    // An unbounded integer admits every decimals
    parse(order_schema_with_decimals(platform_value!({
        "type": "integer",
        "generatedFrom": generated,
        "position": 1
    })));
}

#[test]
fn should_refuse_sys_tokens_decimals_on_an_index_only_type() {
    let schema = platform_value!({
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "indices": [
            {
                "name": "byToken",
                "properties": [{ "tokenId": "asc" }, { "decimals": "asc" }],
                "terminal": "$ownerId"
            }
        ],
        "properties": {
            "tokenId": identifier_property(0),
            "decimals": decimals_property(1)
        },
        "required": ["tokenId", "decimals"],
        "additionalProperties": false
    });
    expect_refused(schema, "an indexOnly type keeps no document");
}

#[test]
fn should_refuse_sys_tokens_decimals_in_a_contested_index() {
    let schema = platform_value!({
        "type": "object",
        "documentsMutable": false,
        "canBeDeleted": false,
        "indices": [
            {
                "name": "byLabel",
                "properties": [{ "label": "asc" }, { "decimals": "asc" }],
                "unique": true,
                "contested": {
                    "fieldMatches": [{ "field": "label", "regexPattern": "^[a-z]{3,5}$" }],
                    "resolution": 0
                }
            }
        ],
        "properties": {
            "tokenId": identifier_property(0),
            "decimals": decimals_property(1),
            "label": { "type": "string", "maxLength": 20, "position": 2 }
        },
        "required": ["tokenId", "decimals", "label"],
        "additionalProperties": false
    });
    expect_refused(schema, "may not be in contested index \"byLabel\"");
}

// ================================================================
//  Generation from state
// ================================================================

#[test]
fn should_write_the_token_decimals_on_a_create_that_leaves_them_out() {
    let document_type = parse(order_schema());
    let mut properties = data(vec![("tokenId", token(5))]);
    let refusal = document_type
        .generate_state_read_properties(&mut properties, None, read_decimals)
        .expect("the reads succeed");
    assert_eq!(refusal, None);
    assert_eq!(
        properties,
        data(vec![("tokenId", token(5)), ("decimals", Value::U8(6))])
    );
}

#[test]
fn should_check_decimals_a_create_sends() {
    let document_type = parse(order_schema());

    // The right value, at any integer width, passes as sent
    let mut properties = data(vec![("tokenId", token(5)), ("decimals", Value::I64(6))]);
    let refusal = document_type
        .generate_state_read_properties(&mut properties, None, read_decimals)
        .expect("the reads succeed");
    assert_eq!(refusal, None);
    assert_eq!(properties.get("decimals"), Some(&Value::I64(6)));

    // Another value is refused
    let mut properties = data(vec![("tokenId", token(5)), ("decimals", Value::U8(2))]);
    let refusal = document_type
        .generate_state_read_properties(&mut properties, None, read_decimals)
        .expect("the reads succeed");
    match refusal {
        Some(ConsensusError::BasicError(BasicError::DocumentPropertyNotGeneratedError(error))) => {
            assert_eq!(error.property(), "decimals");
            assert_eq!(error.function(), DECIMALS);
        }
        other => panic!("expected the property to be refused, got {other:?}"),
    }
}

#[test]
fn should_refuse_a_token_that_does_not_exist() {
    let document_type = parse(order_schema());
    let mut properties = data(vec![("tokenId", token(9))]);
    let refusal = document_type
        .generate_state_read_properties(&mut properties, None, read_decimals)
        .expect("the reads succeed");
    match refusal {
        Some(ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(error))) => {
            assert_eq!(error.entity_id(), &Identifier::new([9; 32]));
            assert_eq!(error.path(), "tokenId");
        }
        other => panic!("expected the token to be refused, got {other:?}"),
    }
}

#[test]
fn should_keep_the_stored_decimals_on_a_replace_that_keeps_the_token() {
    let document_type = parse(order_schema());
    // Stored when the token had 2 decimals; it has 6 now
    let stored = data(vec![("tokenId", token(5)), ("decimals", Value::U8(2))]);

    let mut properties = data(vec![
        ("tokenId", token(5)),
        ("label", Value::Text("x".into())),
    ]);
    let refusal = document_type
        .generate_state_read_properties(&mut properties, Some(&stored), no_read)
        .expect("nothing is read");
    assert_eq!(refusal, None);
    assert_eq!(properties.get("decimals"), Some(&Value::U8(2)));

    let mut properties = data(vec![("tokenId", token(5)), ("decimals", Value::U8(2))]);
    let refusal = document_type
        .generate_state_read_properties(&mut properties, Some(&stored), no_read)
        .expect("nothing is read");
    assert_eq!(refusal, None);

    // The token's current decimals are not the kept value
    let mut properties = data(vec![("tokenId", token(5)), ("decimals", Value::U8(6))]);
    let refusal = document_type
        .generate_state_read_properties(&mut properties, Some(&stored), no_read)
        .expect("nothing is read");
    assert!(matches!(
        refusal,
        Some(ConsensusError::BasicError(
            BasicError::DocumentPropertyNotGeneratedError(_)
        ))
    ));
}

#[test]
fn should_read_the_decimals_again_on_a_replace_that_names_another_token() {
    let document_type = parse(order_schema());
    let stored = data(vec![("tokenId", token(5)), ("decimals", Value::U8(6))]);
    let mut properties = data(vec![("tokenId", token(6))]);
    let refusal = document_type
        .generate_state_read_properties(&mut properties, Some(&stored), read_decimals)
        .expect("the reads succeed");
    assert_eq!(refusal, None);
    assert_eq!(properties.get("decimals"), Some(&Value::U8(18)));

    // The stored token's decimals are refused for the new one
    let mut properties = data(vec![("tokenId", token(6)), ("decimals", Value::U8(6))]);
    let refusal = document_type
        .generate_state_read_properties(&mut properties, Some(&stored), read_decimals)
        .expect("the reads succeed");
    assert!(refusal.is_some());
}

#[test]
fn should_leave_a_document_without_its_token_alone() {
    let document_type = parse(order_schema());
    for sent in [
        data(vec![]),
        data(vec![("decimals", Value::U8(6))]),
        // Not an identifier: the schema validation refuses it
        data(vec![("tokenId", Value::Text("token".into()))]),
    ] {
        let mut properties = sent.clone();
        let refusal = document_type
            .generate_state_read_properties(&mut properties, None, no_read)
            .expect("nothing is read");
        assert_eq!(refusal, None);
        assert_eq!(properties, sent);
    }
}

// ================================================================
//  Clients and proofs
// ================================================================

#[test]
fn should_leave_the_value_to_the_platform_on_a_client() {
    let document_type = parse(order_schema());
    let platform_version = PlatformVersion::latest();

    // The transition builders remove it, whatever the document holds
    let mut properties = data(vec![("tokenId", token(5)), ("decimals", Value::U8(6))]);
    document_type
        .regenerate_generated_properties(&mut properties, platform_version)
        .expect("the regeneration runs");
    assert_eq!(properties, data(vec![("tokenId", token(5))]));

    // The fill on arrival leaves it to the state read
    document_type
        .fill_generated_properties(&mut properties, platform_version)
        .expect("the fill runs");
    assert_eq!(properties, data(vec![("tokenId", token(5))]));

    // The document check does not refuse it left out, and still refuses it without its token
    let check = |properties: BTreeMap<String, Value>| {
        document_type
            .as_ref()
            .validate_generated_from_properties(&Value::from(properties), platform_version)
            .expect("the check runs")
    };
    assert!(check(data(vec![("tokenId", token(5))])).is_valid());
    assert!(check(data(vec![
        ("tokenId", token(5)),
        ("decimals", Value::U8(6))
    ]))
    .is_valid());
    assert!(!check(data(vec![("decimals", Value::U8(6))])).is_valid());
}

#[test]
fn should_take_the_proved_value_into_a_rebuilt_document() {
    let document_type = parse(order_schema());
    let proved = data(vec![("tokenId", token(5)), ("decimals", Value::U8(6))]);

    let mut rebuilt = data(vec![("tokenId", token(5))]);
    document_type.adopt_state_generated_properties(&mut rebuilt, &proved);
    assert_eq!(rebuilt, proved);

    // A value the transition sent stays, so a proved document holding another fails the
    // comparison
    let mut rebuilt = data(vec![("tokenId", token(5)), ("decimals", Value::U8(2))]);
    document_type.adopt_state_generated_properties(&mut rebuilt, &proved);
    assert_eq!(rebuilt.get("decimals"), Some(&Value::U8(2)));

    // Without its token nothing is taken
    let mut rebuilt = data(vec![]);
    document_type.adopt_state_generated_properties(&mut rebuilt, &proved);
    assert!(rebuilt.is_empty());
}
