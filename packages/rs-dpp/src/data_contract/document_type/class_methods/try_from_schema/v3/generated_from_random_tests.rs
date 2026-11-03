//! `generatedFrom` with a random function (protocol version 15): `sys.random.uniform`
//! draws an integer property from its `minimum` to its `maximum` once the block's transitions
//! are accepted. The parse is `apply_generated_from` 1 and the checks against the rest of the
//! type are `validate_generated_from_declarations`; the rules reading such a property are
//! refused where their reads are checked, and a value of another type read from it is not
//! fixed once written. A create writes the placeholder and a replace keeps the stored value
//! (`generate_random_properties`), a client leaves the value out
//! (`regenerate_generated_properties`), and a proof verifier takes it from the proved document
//! (`adopt_state_generated_properties`).
//!
//! No protocol version admits the function yet (`random_generated_from` is `None` in every
//! table, until block execution draws the values), so these tests parse with the latest
//! platform version with it admitted (`drawing_platform_version`).

use super::refusal_test_support::{assert_refused, CONTRACT_ID};
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

/// The latest platform version with `sys.random.uniform` admitted.
fn drawing_platform_version() -> PlatformVersion {
    let mut platform_version = PlatformVersion::latest().clone();
    platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .random_generated_from = Some(0);
    platform_version
}

fn parse(schema: Value) -> DocumentType {
    parse_dispatched(schema, &drawing_platform_version(), true).expect("the schema parses")
}

/// Both parses refuse the declaration: the validating one natively or with the parser's
/// structure error, the one that skips the schema check with the parser's.
fn expect_refused(schema: Value, needle: &str) {
    let platform_version = drawing_platform_version();
    assert!(
        parse_dispatched(schema.clone(), &platform_version, true).is_err(),
        "a validating parse refuses it ({needle})"
    );
    expect_structure_error(parse_dispatched(schema, &platform_version, false), needle);
}

/// Registration refuses the declaration with the parser's structure error.
fn expect_refused_at_registration(schema: Value, needle: &str) {
    expect_structure_error(
        parse_dispatched(schema, &drawing_platform_version(), true),
        needle,
    );
}

/// The document types of a contract holding `types`, registered.
fn parse_contract(
    types: Vec<(&str, Value)>,
) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
    let platform_version = drawing_platform_version();
    let config = DataContractConfig::default_for_version(&platform_version)
        .expect("default config available");
    DocumentType::create_document_types_from_document_schemas(
        Identifier::new(CONTRACT_ID),
        1,
        config.version(),
        types
            .into_iter()
            .map(|(name, schema)| (name.to_string(), schema))
            .collect(),
        None,
        &BTreeMap::new(),
        &config,
        true,
        false,
        &mut vec![],
        &platform_version,
    )
}

/// An identifier property at `position`, referring as `refers_to` says.
fn identifier(position: u64, refers_to: Value) -> Value {
    platform_value!({
        "type": "array",
        "byteArray": true,
        "minItems": 32,
        "maxItems": 32,
        "contentMediaType": "application/x.dash.dpp.identifier",
        "position": position,
        "refersTo": refers_to
    })
}

/// A card nobody changes or deletes, so a `permanentDocument` reference may point at it, its
/// `rarity` drawn, with `extra` set at the type level.
fn permanent_card(extra: Vec<(&str, Value)>) -> Value {
    let mut card = card_schema_with("documentsMutable", Value::Bool(false));
    card.insert("canBeDeleted".to_string(), Value::Bool(false))
        .expect("the schema is a map");
    for (key, value) in extra {
        card.insert(key.to_string(), value)
            .expect("the schema is a map");
    }
    card
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
        range: RandomRange::new(1, 100),
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
        RandomRange::new(-5, 5)
    );
}

#[test]
fn should_refuse_sys_random_uniform_while_no_protocol_version_draws_the_values() {
    // `random_generated_from` is `None` at the latest protocol version: nothing draws the
    // values yet, so a document would keep its placeholder
    let platform_version = PlatformVersion::latest();
    expect_structure_error(
        parse_dispatched(card_schema(), platform_version, true),
        "is not active at this protocol version",
    );
    expect_structure_error(
        parse_dispatched(card_schema(), platform_version, false),
        "is not active at this protocol version",
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
    let platform_version = &drawing_platform_version();

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

#[test]
fn should_refuse_sys_random_uniform_on_a_type_with_a_contested_index() {
    // The contested index is on another property: the document a contest awards would still
    // keep the placeholder
    let contested = platform_value!([{
        "name": "byName",
        "properties": [{ "name": "asc" }],
        "unique": true,
        "contested": {
            "fieldMatches": [{ "field": "name", "regexPattern": "^[a-z]{3,5}$" }],
            "resolution": 0
        }
    }]);
    expect_refused(
        permanent_card(vec![("indices", contested)]),
        "has contested index \"byName\"",
    );
}

#[test]
fn should_refuse_summing_or_averaging_a_property_drawn_at_random() {
    expect_refused(
        card_schema_with("documentsSummable", Value::Text("rarity".to_string())),
        "as documentsSummable does",
    );
    for keyword in ["summable", "averageable"] {
        let mut index = platform_value!({ "name": "byName", "properties": [{ "name": "asc" }] });
        index
            .insert(keyword.to_string(), Value::Text("rarity".to_string()))
            .expect("the index is a map");
        expect_refused(
            card_schema_with("indices", Value::Array(vec![index])),
            "as index \"byName\" does",
        );
    }
}

#[test]
fn should_refuse_a_reference_of_the_type_reading_a_property_drawn_at_random() {
    let with_deck = |where_clause: Value| {
        let mut schema = card_schema();
        schema
            .get_mut("properties")
            .expect("properties")
            .expect("properties are present")
            .insert(
                "deckId".to_string(),
                identifier(
                    2,
                    platform_value!({
                        "type": "permanentDocument",
                        "documentType": "deck",
                        "where": where_clause
                    }),
                ),
            )
            .expect("properties are a map");
        schema
    };
    // The deck's level must equal the card's rarity, or be above it
    for where_clause in [
        platform_value!({ "level": "rarity" }),
        platform_value!({ "level": { "greaterThan": "rarity" } }),
    ] {
        expect_refused(
            with_deck(where_clause),
            "a reference declared on the type reads the property",
        );
    }
}

#[test]
fn should_refuse_an_encrypted_for_key_drawn_at_random() {
    let mut schema = card_schema();
    let properties = schema
        .get_mut("properties")
        .expect("properties")
        .expect("properties are present");
    properties
        .insert(
            "senderKeyId".to_string(),
            platform_value!({ "type": "integer", "minimum": 0, "maximum": 4294967295_u64, "position": 2 }),
        )
        .expect("properties are a map");
    properties
        .insert(
            "secret".to_string(),
            platform_value!({
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 1040,
                "position": 3,
                "encryptedFor": {
                    "recipient": "$ownerId",
                    "recipientKey": "rarity",
                    "senderKey": "senderKeyId",
                    "scheme": "ecdh-secp256k1-aes256-cbc"
                }
            }),
        )
        .expect("properties are a map");
    expect_refused(schema, "names the property as a key");
}

#[test]
fn should_refuse_a_rule_reading_a_property_drawn_at_random() {
    let drawn = "which the platform draws at random once the block's transitions are accepted";
    // A rule judging a create or a replace, reading the document written or the stored one
    for rule in [
        platform_value!({ "greaterThan": ["rarity", 50] }),
        platform_value!({ "greaterThan": ["$old.rarity", 50] }),
        platform_value!({ "anyOf": [{ "absent": "name" }, { "equal": ["rarity", 1] }] }),
    ] {
        expect_refused(
            card_schema_with("propertyConstraints", platform_value!({ "rare": rule })),
            drawn,
        );
    }
    // A rule judging the owner's delete, which a delete in the block that created the
    // document would judge against the placeholder
    expect_refused(
        card_schema_with(
            "deleteConstraints",
            platform_value!({ "common": { "lessThan": ["rarity", 50] } }),
        ),
        drawn,
    );
    // The condition an immutable entry is frozen under
    expect_refused_at_registration(
        card_schema_with(
            "immutable",
            platform_value!([{ "property": "name", "when": { "greaterThan": ["rarity", 50] } }]),
        ),
        drawn,
    );
}

#[test]
fn should_refuse_a_rule_of_another_type_reading_a_property_drawn_at_random() {
    let binder = |rules: Value| {
        platform_value!({
            "type": "object",
            "properties": {
                "cardId": identifier(0, platform_value!({
                    "type": "permanentDocument",
                    "documentType": "card"
                })),
                "label": { "type": "string", "maxLength": 32, "position": 1 }
            },
            "required": ["cardId"],
            "propertyConstraints": rules,
            "additionalProperties": false
        })
    };
    // Through the reference to the card
    assert_refused(
        parse_contract(vec![
            ("card", permanent_card(vec![])),
            (
                "binder",
                binder(platform_value!({
                    "rareOnly": { "greaterThan": [{ "referenced": ["cardId", "rarity"] }, 50] }
                })),
            ),
        ]),
        "draws at random",
    );
    // A count of the cards keyed by their drawn rarity, which a countable index keeps
    let counted_card = permanent_card(vec![(
        "indices",
        platform_value!([{
            "name": "byRarity",
            "properties": [{ "rarity": "asc" }],
            "countable": "countable"
        }]),
    )]);
    assert_refused(
        parse_contract(vec![
            ("card", counted_card),
            (
                "binder",
                binder(platform_value!({
                    "fewLegends": { "lessThan": [{ "countOf": ["card", { "rarity": 100 }] }, 10] }
                })),
            ),
        ]),
        "\"card\" draws at random",
    );
}

#[test]
fn should_refuse_a_derived_index_property_reading_a_property_drawn_at_random() {
    // The card is immutable and never deleted, so its rarity would be fixed once written but
    // for the draw, which moves no entry another type keyed by the placeholder
    let binder = |index_property: &str| {
        platform_value!({
            "type": "object",
            "documentsMutable": false,
            "properties": {
                "cardId": identifier(0, platform_value!({
                    "type": "permanentDocument",
                    "documentType": "card"
                })),
                "label": { "type": "string", "maxLength": 32, "position": 1 }
            },
            "indices": [{
                "name": "byDerived",
                "properties": [{ index_property: "asc" }, { "$createdAt": "asc" }]
            }],
            "required": ["cardId", "$createdAt"],
            "additionalProperties": false
        })
    };
    parse_contract(vec![
        ("card", permanent_card(vec![])),
        ("binder", binder("cardId.name")),
    ])
    .expect("a derived index property reading the card's name registers");
    assert_refused(
        parse_contract(vec![
            ("card", permanent_card(vec![])),
            ("binder", binder("cardId.rarity")),
        ]),
        "is drawn at random once the block's transitions are accepted",
    );
}
