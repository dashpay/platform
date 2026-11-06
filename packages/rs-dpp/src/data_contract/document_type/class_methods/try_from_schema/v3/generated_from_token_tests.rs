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
use crate::data_contract::config::v1::DataContractConfigSettersV1;
use crate::data_contract::document_type::methods::DocumentTypeBasicMethods;
use crate::data_contract::document_type::property_constraints::DocumentSystemValues;
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
            range: None,
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
            platform_value!({ "type": "integer", "const": 6, "generatedFrom": generated.clone(), "position": 1 }),
            "only allowed on an integer property that admits every value from 0 to 255",
        ),
        (
            platform_value!({ "type": "integer", "minimum": 0, "maximum": 255, "exclusiveMinimum": 0, "generatedFrom": generated.clone(), "position": 1 }),
            "only allowed on an integer property that admits every value from 0 to 255",
        ),
        (
            platform_value!({ "type": "integer", "minimum": 0, "maximum": 255, "exclusiveMaximum": 200, "generatedFrom": generated.clone(), "position": 1 }),
            "only allowed on an integer property that admits every value from 0 to 255",
        ),
        (
            platform_value!({ "type": "integer", "minimum": 0, "maximum": 255, "multipleOf": 2, "generatedFrom": generated.clone(), "position": 1 }),
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

/// A contract that does not size its integers makes the property an `i64` whatever its
/// bounds, so a `maximum` past `i64::MAX` reaches the check, which admits it.
#[test]
fn should_admit_sys_tokens_decimals_under_a_maximum_past_i64_in_an_unsized_contract() {
    let mut config = DataContractConfig::default_for_version(PlatformVersion::latest())
        .expect("default config available");
    config.set_sized_integer_types_enabled(false);
    let maximum = u64::MAX;
    let schema = order_schema_with_decimals(platform_value!({
        "type": "integer",
        "minimum": 0,
        "maximum": maximum,
        "generatedFrom": { "function": DECIMALS, "params": ["tokenId"] },
        "position": 1
    }));
    for full_validation in [true, false] {
        DocumentType::try_from_schema(
            Identifier::new([1; 32]),
            1,
            config.version(),
            "charter",
            schema.clone(),
            None,
            &BTreeMap::new(),
            &config,
            full_validation,
            &mut vec![],
            PlatformVersion::latest(),
        )
        .unwrap_or_else(|error| {
            panic!("the schema parses (full validation {full_validation}): {error:?}")
        });
    }
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

/// A `quote` object holding the token and its generated decimals, with `extra` keywords on
/// the object.
fn quote_schema(extra: Vec<(&str, Value)>) -> Value {
    let mut quote = platform_value!({
        "type": "object",
        "properties": {
            "tokenId": identifier_property(0),
            "decimals": decimals_with_params(&["quote.tokenId"])
        },
        "required": ["tokenId"],
        "additionalProperties": false,
        "position": 0
    });
    for (keyword, value) in extra {
        quote
            .insert(keyword.to_string(), value)
            .expect("the object schema is a map");
    }
    platform_value!({
        "type": "object",
        "properties": { "quote": quote },
        "additionalProperties": false
    })
}

/// An object's `const` or `enum` compares the whole object, the decimals included, which a
/// client leaving them out cannot know: a `sys.tokens` property inside one is refused. The
/// same object without them is admitted.
#[test]
fn should_refuse_sys_tokens_decimals_inside_an_object_declaring_const_or_enum() {
    let fixed = platform_value!({ "tokenId": token(5), "decimals": 6 });
    parse(quote_schema(vec![]));
    expect_refused(
        quote_schema(vec![("const", fixed.clone())]),
        "may not sit inside an object declaring const, as \"quote\" does",
    );
    expect_refused(
        quote_schema(vec![("enum", Value::Array(vec![fixed]))]),
        "may not sit inside an object declaring enum, as \"quote\" does",
    );
}

/// A token function's param errors name identifier properties, where a string
/// transformation's name string ones.
#[test]
fn should_name_identifier_properties_in_a_token_function_param_error() {
    let mut schema = quote_schema(vec![]);
    schema
        .get_mut("properties")
        .expect("properties")
        .expect("properties are present")
        .insert(
            "decimals".to_string(),
            platform_value!({
                "type": "integer",
                "minimum": 0,
                "maximum": 255,
                "generatedFrom": { "function": DECIMALS, "params": ["quote"] },
                "position": 1
            }),
        )
        .expect("properties are a map");
    expect_refused(
        schema,
        "param \"quote\" is an object, not an identifier property: name one of its identifier \
         members",
    );
    expect_refused(
        order_schema_with_decimals(decimals_with_params(&["decimals"])),
        "reads the property itself: name other identifier properties of the document type",
    );
}

fn decimals_with_params(params: &[&str]) -> Value {
    let params: Vec<Value> = params
        .iter()
        .map(|param| Value::Text(param.to_string()))
        .collect();
    platform_value!({
        "type": "integer",
        "minimum": 0,
        "maximum": 255,
        "generatedFrom": { "function": DECIMALS, "params": params },
        "position": 1
    })
}

/// `schema` parsed at the latest protocol version in a contract whose `$defs` are `defs`.
fn parse_with_defs(
    schema: Value,
    defs: &BTreeMap<String, Value>,
    full_validation: bool,
) -> Result<DocumentType, ProtocolError> {
    let platform_version = PlatformVersion::latest();
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available");
    DocumentType::try_from_schema(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "charter",
        schema,
        Some(defs),
        &BTreeMap::new(),
        &config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

/// Both parses of `schema` with `defs` refuse it, the one that skips the schema check with
/// the parser's structure error.
fn expect_refused_with_defs(schema: Value, defs: &BTreeMap<String, Value>, needle: &str) {
    assert!(
        parse_with_defs(schema.clone(), defs, true).is_err(),
        "a validating parse refuses it ({needle})"
    );
    expect_structure_error(parse_with_defs(schema, defs, false), needle);
}

/// A document type whose `quote` is declared with a `$ref` to the `quote` definition, a token
/// and its generated decimals, with `beside` written beside the `$ref`.
fn referenced_quote(beside: Vec<(&str, Value)>) -> (Value, BTreeMap<String, Value>) {
    let mut quote = platform_value!({ "$ref": "#/$defs/quote", "position": 0 });
    for (keyword, value) in beside {
        quote
            .insert(keyword.to_string(), value)
            .expect("the property schema is a map");
    }
    let defs = BTreeMap::from([(
        "quote".to_string(),
        platform_value!({
            "type": "object",
            "properties": {
                "tokenId": identifier_property(0),
                "decimals": decimals_with_params(&["quote.tokenId"])
            },
            "required": ["tokenId"],
            "additionalProperties": false
        }),
    )]);
    let schema = platform_value!({
        "type": "object",
        "properties": { "quote": quote },
        "additionalProperties": false
    });
    (schema, defs)
}

/// A client's schema check reads a stand-in for decimals it leaves out, which only the
/// property's own schema, read at registration, is known to admit. A `$ref` brings in schemas
/// registration does not judge (the definition, the keywords beside the `$ref`, and the
/// definition's own references), so an object holding token decimals declared with one is
/// refused, however plain: a `const` beside it, a `const` behind a further reference, or a
/// `properties` beside it fixing the decimals would each refuse the stand-in.
#[test]
fn should_refuse_sys_tokens_decimals_inside_an_object_declared_with_a_ref() {
    let refused = "must be declared in place, and \"quote\" is declared with a $ref";
    let fixed = platform_value!({ "tokenId": token(5), "decimals": 6 });

    let (schema, defs) = referenced_quote(vec![]);
    expect_refused_with_defs(schema, &defs, refused);
    let (schema, defs) = referenced_quote(vec![("const", fixed.clone())]);
    expect_refused_with_defs(schema, &defs, refused);
    let (schema, defs) = referenced_quote(vec![("enum", Value::Array(vec![fixed.clone()]))]);
    expect_refused_with_defs(schema, &defs, refused);
    // A `properties` beside the `$ref`, fixing the decimals
    let (schema, defs) = referenced_quote(vec![(
        "properties",
        platform_value!({ "decimals": { "type": "integer", "const": 6 } }),
    )]);
    expect_refused_with_defs(schema, &defs, refused);
    // A definition referring further, to one fixing the whole quote
    let (schema, mut defs) = referenced_quote(vec![]);
    defs.get_mut("quote")
        .expect("the quote definition")
        .insert(
            "$ref".to_string(),
            Value::Text("#/$defs/fixedQuote".to_string()),
        )
        .expect("the definition is a map");
    defs.insert(
        "fixedQuote".to_string(),
        platform_value!({
            "type": "object",
            "properties": {
                "tokenId": identifier_property(0),
                "decimals": { "type": "integer", "minimum": 0, "maximum": 255, "position": 1 }
            },
            "const": fixed
        }),
    );
    expect_refused_with_defs(schema, &defs, refused);
}

/// An `order` whose `decimals` are declared with a `$ref` to a generated decimals definition,
/// with `beside` written beside the `$ref`.
fn referenced_decimals(beside: Vec<(&str, Value)>) -> (Value, BTreeMap<String, Value>) {
    let mut decimals = platform_value!({ "$ref": "#/$defs/generatedDecimals", "position": 1 });
    for (keyword, value) in beside {
        decimals
            .insert(keyword.to_string(), value)
            .expect("the property schema is a map");
    }
    let defs = BTreeMap::from([(
        "generatedDecimals".to_string(),
        platform_value!({
            "type": "integer",
            "minimum": 0,
            "maximum": 255,
            "generatedFrom": { "function": DECIMALS, "params": ["tokenId"] }
        }),
    )]);
    (order_schema_with_decimals(decimals), defs)
}

/// A token decimals property declared with a `$ref` is refused, plain or narrowed beside it:
/// the keywords beside the `$ref` apply to a document too, a `type` no integer meets
/// included.
#[test]
fn should_refuse_sys_tokens_decimals_declared_with_a_ref() {
    let refused = "must be declared in place, and \"decimals\" is declared with a $ref";
    let (schema, defs) = referenced_decimals(vec![]);
    expect_refused_with_defs(schema, &defs, refused);
    for beside in [
        ("type", platform_value!("string")),
        ("multipleOf", platform_value!(2)),
        ("minimum", platform_value!(1)),
        ("const", platform_value!(6)),
        ("enum", platform_value!([6, 18])),
    ] {
        let (schema, defs) = referenced_decimals(vec![beside]);
        expect_refused_with_defs(schema, &defs, refused);
    }
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

/// A token named in base58 text is read as its identifier; a text longer than any identifier's
/// is left to the schema validation without being decoded.
#[test]
fn should_read_a_base58_token_and_leave_a_text_too_long_for_one() {
    let document_type = parse(order_schema());
    let text_token = Value::Text(
        Identifier::new([5; 32]).to_string(platform_value::string_encoding::Encoding::Base58),
    );
    let mut properties = data(vec![("tokenId", text_token.clone())]);
    let refusal = document_type
        .generate_state_read_properties(&mut properties, None, read_decimals)
        .expect("the reads succeed");
    assert_eq!(refusal, None);
    assert_eq!(
        properties,
        data(vec![("tokenId", text_token), ("decimals", Value::U8(6))])
    );

    let too_long = Value::Text("2".repeat(19 * 1024));
    let mut properties = data(vec![("tokenId", too_long.clone())]);
    let refusal = document_type
        .generate_state_read_properties(
            &mut properties,
            None,
            |_, _| -> Result<Option<Value>, Infallible> {
                panic!("expected no read for a text too long to be an identifier")
            },
        )
        .expect("nothing is read");
    assert_eq!(refusal, None);
    assert_eq!(properties, data(vec![("tokenId", too_long)]));
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

/// A client, which leaves the decimals out, does not know them: its check of the
/// `propertyConstraints` rules does not judge a rule reading them, and still judges every
/// other rule, and one reading decimals the document holds or leaves out without its token.
/// Consensus, which writes them before judging, judges every rule.
#[test]
fn should_leave_a_rule_reading_state_generated_decimals_unjudged_on_a_client() {
    let mut schema = order_schema();
    schema
        .insert(
            "propertyConstraints".to_string(),
            platform_value!({
                "decimalsPresent": { "present": "decimals" },
                "decimalsUpTo18": { "lessThanOrEqual": ["decimals", 18] },
                "shortLabel": { "lessThanOrEqual": [{ "length": "label" }, 8] }
            }),
        )
        .expect("the schema is a map");
    schema
        .insert("required".to_string(), platform_value!(["tokenId"]))
        .expect("the schema is a map");
    let document_type = parse(schema);
    let platform_version = PlatformVersion::latest();
    let broken = |properties: BTreeMap<String, Value>, system: &DocumentSystemValues| {
        let result = document_type
            .as_ref()
            .validate_property_constraints(&Value::from(properties), system, platform_version)
            .expect("the check runs");
        result.errors.first().map(|error| match error {
            ConsensusError::BasicError(BasicError::DocumentPropertyConstraintViolatedError(
                error,
            )) => error.constraint().to_string(),
            other => panic!("expected a broken rule, got {other:?}"),
        })
    };
    let client = DocumentSystemValues::default();
    let short = || Value::Text("short".to_string());

    // The decimals rules wait for the platform; the label rule is judged
    assert_eq!(
        broken(
            data(vec![("tokenId", token(5)), ("label", short())]),
            &client
        ),
        None
    );
    assert_eq!(
        broken(
            data(vec![
                ("tokenId", token(5)),
                ("label", Value::Text("a long label".to_string()))
            ]),
            &client
        ),
        Some("shortLabel".to_string())
    );
    // Decimals the document holds are judged
    assert_eq!(
        broken(
            data(vec![
                ("tokenId", token(5)),
                ("decimals", Value::U8(20)),
                ("label", short())
            ]),
            &client
        ),
        Some("decimalsUpTo18".to_string())
    );
    // Without its token the property stays absent, on a client as in consensus
    assert_eq!(
        broken(data(vec![("label", short())]), &client),
        Some("decimalsPresent".to_string())
    );
    // Consensus judges every rule
    let consensus = DocumentSystemValues {
        consensus: true,
        ..DocumentSystemValues::default()
    };
    assert_eq!(
        broken(
            data(vec![("tokenId", token(5)), ("label", short())]),
            &consensus
        ),
        Some("decimalsPresent".to_string())
    );
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
