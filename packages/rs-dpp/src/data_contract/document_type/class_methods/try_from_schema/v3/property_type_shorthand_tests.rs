//! The property type shorthands under generation 3 (protocol version 14):
//! `"type": "identifier"`, and `"type": "bytes"` with a `size`, parse to
//! exactly what their long form parses to wherever the long form may stand (a
//! top-level property, an object member, the `items` of a typed array, a
//! definition reached by `$ref`), while the document type, and so the stored
//! contract, keeps the schema as sent.

use super::immutable_tests::{
    expect_structure_error, parse_dispatched, parse_dispatched_with_defs,
};
use super::*;
use crate::block::block_info::BlockInfo;
use crate::consensus::basic::BasicError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::document_type::property::ByteArrayPropertySizes;
use crate::data_contract::document_type::property_constraints::DocumentSystemValues;
use crate::data_contract::methods::validate_document::DataContractDocumentValidationMethodsV0;
use crate::data_contract::methods::validate_update::DataContractUpdateValidationMethodsV0;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::data_contract::DataContract;
use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use crate::document::{Document, DocumentV0};
use crate::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted,
    PlatformSerializableWithPlatformVersion,
};
use crate::validation::SimpleConsensusValidationResult;
use platform_value::platform_value;
use platform_version::TryFromPlatformVersioned;

const IDENTIFIER_MEDIA_TYPE: &str = "application/x.dash.dpp.identifier";

/// The long form of `"type": "identifier"`, with `extra` keywords beside it.
fn long_identifier(extra: Value) -> Value {
    with(
        platform_value!({
            "type": "array",
            "byteArray": true,
            "minItems": 32,
            "maxItems": 32,
            "contentMediaType": IDENTIFIER_MEDIA_TYPE
        }),
        extra,
    )
}

/// The long form of `"type": "bytes", "size": size`, with `extra` keywords.
fn long_bytes(size: u16, extra: Value) -> Value {
    with(
        platform_value!({
            "type": "array",
            "byteArray": true,
            "minItems": size,
            "maxItems": size
        }),
        extra,
    )
}

fn with(mut schema: Value, extra: Value) -> Value {
    if let Value::Map(entries) = extra {
        for (key, value) in entries {
            let key = key.to_text().expect("a text key");
            schema.set_value(&key, value).expect("the keyword applies");
        }
    }
    schema
}

/// A `payment` type whose byte arrays are written with `identifier` and
/// `bytes` (`shorthand`) or in full: a recipient that must differ from the
/// owner, a transaction hash a unique index reads, an object holding an
/// identifier and a 20-byte digest, a typed array of identifiers each
/// referring to an identity, a typed array of 20-byte hashes, and a sponsor
/// whose schema is the `sponsor` definition.
fn payment_schema(shorthand: bool) -> Value {
    let identifier = |extra: Value| {
        if shorthand {
            with(platform_value!({ "type": "identifier" }), extra)
        } else {
            long_identifier(extra)
        }
    };
    let bytes = |size: u16, extra: Value| {
        if shorthand {
            with(platform_value!({ "type": "bytes", "size": size }), extra)
        } else {
            long_bytes(size, extra)
        }
    };
    platform_value!({
        "type": "object",
        "properties": {
            "recipientId": identifier(platform_value!({ "distinctFrom": "$ownerId", "position": 0 })),
            "txHash": bytes(32, platform_value!({ "position": 1 })),
            "memo": {
                "type": "object",
                "properties": {
                    "authorId": identifier(platform_value!({ "position": 0 })),
                    "digest": bytes(20, platform_value!({ "position": 1 }))
                },
                "additionalProperties": false,
                "position": 2
            },
            "witnesses": {
                "type": "array",
                "maxItems": 4,
                "items": identifier(platform_value!({ "refersTo": { "type": "identity" } })),
                "position": 3
            },
            "hashes": {
                "type": "array",
                "maxItems": 4,
                "items": bytes(20, platform_value!({})),
                "position": 4
            },
            "sponsorId": { "$ref": "#/$defs/sponsor", "position": 5 }
        },
        "indices": [
            { "name": "byTxHash", "properties": [{ "txHash": "asc" }], "unique": true },
            { "name": "byRecipient", "properties": [{ "recipientId": "asc" }] }
        ],
        "required": ["recipientId", "txHash"],
        "additionalProperties": false
    })
}

/// The contract's `$defs`: the `sponsor` identifier, written as `payment_schema` writes its own.
fn payment_defs(shorthand: bool) -> BTreeMap<String, Value> {
    let sponsor = if shorthand {
        platform_value!({ "type": "identifier" })
    } else {
        long_identifier(platform_value!({}))
    };
    BTreeMap::from([("sponsor".to_string(), sponsor)])
}

fn parse_payment(
    shorthand: bool,
    platform_version: &PlatformVersion,
    full_validation: bool,
) -> Result<DocumentType, ProtocolError> {
    parse_dispatched_with_defs(
        payment_schema(shorthand),
        Some(&payment_defs(shorthand)),
        platform_version,
        full_validation,
    )
}

fn contract_value(version: u32, schema: Value, defs: BTreeMap<String, Value>) -> Value {
    platform_value!({
        "$formatVersion": "1",
        "id": Value::Identifier([7; 32]),
        "ownerId": Value::Identifier([8; 32]),
        "version": version,
        "schemaDefs": Value::from(defs),
        "documentSchemas": { "payment": schema },
    })
}

fn payment_contract(shorthand: bool) -> DataContract {
    DataContract::from_value(
        contract_value(1, payment_schema(shorthand), payment_defs(shorthand)),
        true,
        PlatformVersion::latest(),
    )
    .expect("the payment contract registers")
}

/// A payment every property of `payment_schema` holds a value of.
fn payment_properties() -> BTreeMap<String, Value> {
    BTreeMap::from([
        ("recipientId".to_string(), Value::Identifier([1; 32])),
        ("txHash".to_string(), Value::Bytes(vec![2; 32])),
        (
            "memo".to_string(),
            platform_value!({
                "authorId": Value::Identifier([3; 32]),
                "digest": Value::Bytes(vec![4; 20])
            }),
        ),
        (
            "witnesses".to_string(),
            Value::Array(vec![Value::Identifier([5; 32]), Value::Identifier([6; 32])]),
        ),
        (
            "hashes".to_string(),
            Value::Array(vec![Value::Bytes(vec![9; 20])]),
        ),
        ("sponsorId".to_string(), Value::Identifier([10; 32])),
    ])
}

fn validate_payment(
    contract: &DataContract,
    properties: BTreeMap<String, Value>,
) -> SimpleConsensusValidationResult {
    contract
        .validate_document_properties(
            "payment",
            Value::from(properties),
            &DocumentSystemValues::owned_by(Identifier::new([11; 32])),
            PlatformVersion::latest(),
        )
        .expect("the document is judged")
}

fn as_v2(document_type: DocumentType) -> DocumentTypeV2 {
    match document_type {
        DocumentType::V2(v2) => v2,
        other => panic!("generation 3 parses to a V2 document type, got {other:?}"),
    }
}

#[test]
fn should_parse_a_shorthand_to_the_document_type_of_its_long_form() {
    for full_validation in [true, false] {
        let mut shorthand = as_v2(
            parse_payment(true, PlatformVersion::latest(), full_validation)
                .expect("the shorthand registers"),
        );
        let long = as_v2(
            parse_payment(false, PlatformVersion::latest(), full_validation)
                .expect("the long form registers"),
        );

        // The document type keeps the schema as sent
        assert_eq!(shorthand.schema, payment_schema(true));
        assert_eq!(long.schema, payment_schema(false));

        // and everything parsed from it is what the long form parses to
        shorthand.schema = long.schema.clone();
        assert_eq!(shorthand, long, "full validation {full_validation}");
        assert_eq!(
            long.flattened_properties
                .get("recipientId")
                .map(|property| &property.property_type),
            Some(&DocumentPropertyType::Identifier)
        );
        assert_eq!(
            long.flattened_properties
                .get("memo.digest")
                .map(|property| &property.property_type),
            Some(&DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
                min_size: Some(20),
                max_size: Some(20),
            }))
        );
    }
}

#[test]
fn should_serialize_a_document_byte_identically_under_both_spellings() {
    let platform_version = PlatformVersion::latest();
    let shorthand_contract = payment_contract(true);
    let long_contract = payment_contract(false);
    let document = Document::V0(DocumentV0 {
        id: Identifier::new([12; 32]),
        owner_id: Identifier::new([11; 32]),
        properties: payment_properties(),
        revision: Some(1),
        ..Default::default()
    });

    let serialize = |contract: &DataContract| {
        let document_type = contract
            .document_type_for_name("payment")
            .expect("the payment type");
        document
            .serialize(document_type, contract, platform_version)
            .expect("the document serializes")
    };
    let shorthand_bytes = serialize(&shorthand_contract);
    assert_eq!(shorthand_bytes, serialize(&long_contract));

    // Bytes written under one spelling read back alike under the other
    let read_under = |contract: &DataContract| {
        let document_type = contract
            .document_type_for_name("payment")
            .expect("the payment type");
        Document::from_bytes(&shorthand_bytes, document_type, platform_version)
            .expect("the document deserializes")
    };
    assert_eq!(read_under(&shorthand_contract), read_under(&long_contract));
}

#[test]
fn should_round_trip_a_contract_written_with_the_shorthand_unchanged() {
    let platform_version = PlatformVersion::latest();
    let contract = payment_contract(true);

    // Platform serialization: the stored bytes carry the shorthand, and a
    // contract read back from them, without validation as from state,
    // serializes to the same bytes
    let bytes = contract
        .serialize_to_bytes_with_platform_version(platform_version)
        .expect("the contract serializes");
    let stored = DataContract::versioned_deserialize_untrusted(&bytes, false, platform_version)
        .expect("the stored contract deserializes");
    assert_eq!(
        stored
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("the stored contract serializes"),
        bytes
    );
    let stored_schema = stored
        .document_schemas()
        .get("payment")
        .copied()
        .cloned()
        .expect("the payment schema");
    let tx_hash = stored_schema
        .get_value_at_path("properties.txHash")
        .expect("the txHash property");
    assert_eq!(
        tx_hash.get_optional_str("type").ok().flatten(),
        Some("bytes")
    );
    assert_eq!(
        tx_hash.get_optional_integer::<u16>("size").ok().flatten(),
        Some(32)
    );
    assert!(tx_hash
        .get_optional_value("byteArray")
        .ok()
        .flatten()
        .is_none());
    let sponsor = stored
        .schema_defs()
        .and_then(|defs| defs.get("sponsor"))
        .expect("the sponsor definition");
    assert_eq!(
        sponsor.get_optional_str("type").ok().flatten(),
        Some("identifier")
    );
    // and parses to the same document type as the registered contract
    assert_eq!(
        stored
            .document_type_for_name("payment")
            .map(|t| t.properties().clone())
            .ok(),
        contract
            .document_type_for_name("payment")
            .map(|t| t.properties().clone())
            .ok()
    );

    // JSON: the shorthand is written out as sent, and a contract read back
    // from the JSON writes the same JSON
    let to_json = |contract: &DataContract| {
        let format = DataContractInSerializationFormat::try_from_platform_versioned(
            contract.clone(),
            platform_version,
        )
        .expect("the contract converts");
        serde_json::to_value(&format).expect("the contract converts to JSON")
    };
    let json = to_json(&contract);
    assert_eq!(
        json["documentSchemas"]["payment"]["properties"]["txHash"],
        serde_json::json!({ "type": "bytes", "size": 32, "position": 1 })
    );
    assert_eq!(
        json["schemaDefs"]["sponsor"],
        serde_json::json!({ "type": "identifier" })
    );
    let from_json =
        DataContract::from_json(json.clone(), true, platform_version).expect("the JSON registers");
    assert_eq!(to_json(&from_json), json);
}

#[test]
fn should_validate_documents_against_the_long_form_of_a_shorthand() {
    let platform_version = PlatformVersion::latest();
    let registered = payment_contract(true);
    // A contract read back from state compiles its validator on first use,
    // from the schema as sent
    let stored = DataContract::versioned_deserialize_untrusted(
        &registered
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("the contract serializes"),
        false,
        platform_version,
    )
    .expect("the stored contract deserializes");

    for contract in [&registered, &stored] {
        let result = validate_payment(contract, payment_properties());
        assert!(result.is_valid(), "{:?}", result.errors);

        for (path, value) in [
            ("recipientId", Value::Bytes(vec![1; 31])),
            ("txHash", Value::Bytes(vec![2; 33])),
            (
                "memo",
                platform_value!({ "authorId": Value::Identifier([3; 32]), "digest": Value::Bytes(vec![4; 19]) }),
            ),
            ("hashes", Value::Array(vec![Value::Bytes(vec![9; 21])])),
            ("sponsorId", Value::Bytes(vec![10; 33])),
        ] {
            let mut properties = payment_properties();
            properties.insert(path.to_string(), value);
            let result = validate_payment(contract, properties);
            assert!(
                matches!(
                    result.errors.first(),
                    Some(ConsensusError::BasicError(BasicError::JsonSchemaError(_)))
                ),
                "{path}: {:?}",
                result.errors
            );
        }
    }
}

#[test]
fn should_read_a_definition_reached_by_ref_as_its_long_form() {
    let schema = platform_value!({
        "type": "object",
        "properties": {
            "ownerRef": { "$ref": "#/$defs/owner", "position": 0 },
            "hashRef": { "$ref": "#/$defs/hash", "position": 1 }
        },
        "additionalProperties": false
    });
    let defs = BTreeMap::from([
        (
            "owner".to_string(),
            platform_value!({ "type": "identifier" }),
        ),
        (
            "hash".to_string(),
            platform_value!({ "type": "bytes", "size": 32 }),
        ),
    ]);
    for full_validation in [true, false] {
        let document_type = parse_dispatched_with_defs(
            schema.clone(),
            Some(&defs),
            PlatformVersion::latest(),
            full_validation,
        )
        .expect("the definitions register");
        let properties = document_type.flattened_properties();
        assert_eq!(
            properties
                .get("ownerRef")
                .map(|property| &property.property_type),
            Some(&DocumentPropertyType::Identifier)
        );
        assert_eq!(
            properties
                .get("hashRef")
                .map(|property| &property.property_type),
            Some(&DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
                min_size: Some(32),
                max_size: Some(32),
            }))
        );
        assert!(document_type.identifier_paths().contains("ownerRef"));
        assert!(document_type.binary_paths().contains("hashRef"));
    }
}

#[test]
fn should_refuse_a_long_form_keyword_beside_a_shorthand() {
    for (keyword, value) in [
        ("byteArray", Value::Bool(true)),
        ("minItems", Value::U64(32)),
        ("maxItems", Value::U64(32)),
        (
            "contentMediaType",
            Value::Text(IDENTIFIER_MEDIA_TYPE.into()),
        ),
    ] {
        for shorthand in [
            platform_value!({ "type": "identifier", "position": 0 }),
            platform_value!({ "type": "bytes", "size": 32, "position": 0 }),
        ] {
            let mut property = shorthand;
            property
                .set_value(keyword, value.clone())
                .expect("the keyword applies");
            let schema = platform_value!({
                "type": "object",
                "properties": { "value": property },
                "additionalProperties": false
            });
            for full_validation in [true, false] {
                expect_structure_error(
                    parse_dispatched(schema.clone(), PlatformVersion::latest(), full_validation),
                    &format!("writes its own {keyword}"),
                );
            }
        }
    }
}

#[test]
fn should_refuse_size_on_an_identifier_and_bytes_without_a_size() {
    for (property, needle) in [
        (
            platform_value!({ "type": "identifier", "size": 32, "position": 0 }),
            "takes no size",
        ),
        (
            platform_value!({ "type": "bytes", "position": 0 }),
            "without a size",
        ),
        (
            platform_value!({ "type": "bytes", "size": 0, "position": 0 }),
            "not an integer from 1",
        ),
        (
            platform_value!({ "type": "bytes", "size": "32", "position": 0 }),
            "not an integer from 1",
        ),
    ] {
        let schema = platform_value!({
            "type": "object",
            "properties": { "value": property },
            "additionalProperties": false
        });
        for full_validation in [true, false] {
            expect_structure_error(
                parse_dispatched(schema.clone(), PlatformVersion::latest(), full_validation),
                needle,
            );
        }
    }
}

/// A byte array larger than the most bytes a document field may hold could
/// never be filled. The bound is read from the tables, so it is a
/// registration rule: a stored contract keeps parsing.
#[test]
fn should_refuse_a_size_past_the_largest_field_value_at_registration_only() {
    let max_field_value_size = PlatformVersion::latest().system_limits.max_field_value_size;
    let schema = |size: u32| {
        platform_value!({
            "type": "object",
            "properties": {
                "blob": { "type": "bytes", "size": size, "position": 0 }
            },
            "additionalProperties": false
        })
    };
    parse_dispatched(
        schema(max_field_value_size),
        PlatformVersion::latest(),
        true,
    )
    .expect("the largest field value is the largest size");
    expect_structure_error(
        parse_dispatched(
            schema(max_field_value_size + 1),
            PlatformVersion::latest(),
            true,
        ),
        &format!("not an integer from 1 to {max_field_value_size}"),
    );
    parse_dispatched(
        schema(max_field_value_size + 1),
        PlatformVersion::latest(),
        false,
    )
    .expect("a stored contract keeps parsing");
}

#[test]
fn should_refuse_the_shorthands_at_protocol_version_13() {
    let platform_version_13 = PlatformVersion::get(13).expect("protocol version 13");
    for property in [
        platform_value!({ "type": "identifier", "position": 0 }),
        platform_value!({ "type": "bytes", "size": 32, "position": 0 }),
    ] {
        let schema = platform_value!({
            "type": "object",
            "properties": { "value": property },
            "additionalProperties": false
        });
        // Meta-schema v2 knows no such JSON Schema type
        match parse_dispatched(schema.clone(), platform_version_13, true) {
            Err(ProtocolError::ConsensusError(error)) => assert!(
                matches!(
                    *error,
                    ConsensusError::BasicError(BasicError::JsonSchemaError(_))
                ),
                "{error:?}"
            ),
            other => panic!("expected the meta-schema to refuse it, got {other:?}"),
        }
        // and the generation 2 parser no such property type
        let error = parse_dispatched(schema, platform_version_13, false)
            .expect_err("generation 2 refuses it");
        assert!(
            error.to_string().contains("unsupported property type"),
            "{error}"
        );
    }

    // A definition in the shorthand too
    let schema = platform_value!({
        "type": "object",
        "properties": { "value": { "$ref": "#/$defs/owner", "position": 0 } },
        "additionalProperties": false
    });
    let defs = BTreeMap::from([(
        "owner".to_string(),
        platform_value!({ "type": "identifier" }),
    )]);
    parse_dispatched_with_defs(schema, Some(&defs), platform_version_13, true)
        .expect_err("meta-schema v2 refuses the definition");
}

/// An update rewriting a property, or a definition, from one spelling to the
/// other changes nothing, in either direction; a real change is still one.
#[test]
fn should_accept_an_update_between_the_two_spellings_as_no_change() {
    let platform_version = PlatformVersion::latest();
    let contract = |version: u32, schema: Value, defs: BTreeMap<String, Value>| {
        DataContract::from_value(
            contract_value(version, schema, defs),
            true,
            platform_version,
        )
        .expect("the contract registers")
    };

    for (old_shorthand, new_shorthand) in [(false, true), (true, false), (true, true)] {
        let old = contract(
            1,
            payment_schema(old_shorthand),
            payment_defs(old_shorthand),
        );
        let new = contract(
            2,
            payment_schema(new_shorthand),
            payment_defs(new_shorthand),
        );
        let result = old
            .validate_update(&new, &BlockInfo::default(), platform_version)
            .expect("the update is judged");
        assert!(
            result.is_valid(),
            "{old_shorthand} -> {new_shorthand}: {:?}",
            result.errors
        );
    }

    let old = contract(1, payment_schema(true), payment_defs(true));
    let changed = |path: &str, replacement: Value| {
        let mut schema = payment_schema(true);
        schema
            .set_value_at_full_path(path, replacement)
            .expect("the path exists");
        contract(2, schema, payment_defs(true))
    };
    for (path, replacement) in [
        // An identifier is no plain 32-byte array
        (
            "properties.recipientId",
            platform_value!({ "type": "bytes", "size": 32, "position": 0 }),
        ),
        // nor is a fixed size another
        (
            "properties.memo.properties.digest",
            platform_value!({ "type": "bytes", "size": 32, "position": 1 }),
        ),
    ] {
        let result = old
            .validate_update(
                &changed(path, replacement),
                &BlockInfo::default(),
                platform_version,
            )
            .expect("the update is judged");
        assert!(!result.is_valid(), "{path} changed and was accepted");
    }

    // A definition changed from an identifier to plain bytes
    let new = contract(
        2,
        payment_schema(true),
        BTreeMap::from([(
            "sponsor".to_string(),
            platform_value!({ "type": "bytes", "size": 32 }),
        )]),
    );
    let result = old
        .validate_update(&new, &BlockInfo::default(), platform_version)
        .expect("the update is judged");
    assert!(
        !result.is_valid(),
        "the definition changed and was accepted"
    );
}

/// The update comparison helpers the shorthands reach are shared with the
/// shipped generation 0 of `validate_update`: at protocol version 13 the
/// expansion rewrites nothing, and the long form is judged as before.
#[test]
fn should_judge_updates_at_protocol_version_13_as_before() {
    let platform_version_13 = PlatformVersion::get(13).expect("protocol version 13");
    let contract = |version: u32, schema: Value, defs: BTreeMap<String, Value>| {
        DataContract::from_value(
            contract_value(version, schema, defs),
            true,
            platform_version_13,
        )
        .expect("the long form registers at protocol version 13")
    };
    let schema = |recipient: Value| {
        platform_value!({
            "type": "object",
            "properties": {
                "recipientId": recipient,
                "sponsorId": { "$ref": "#/$defs/sponsor", "position": 1 }
            },
            "additionalProperties": false
        })
    };
    let sponsor = |definition: Value| BTreeMap::from([("sponsor".to_string(), definition)]);
    let old = contract(
        1,
        schema(long_identifier(platform_value!({ "position": 0 }))),
        sponsor(long_identifier(platform_value!({}))),
    );
    let judge = |new: &DataContract| {
        old.validate_update(new, &BlockInfo::default(), platform_version_13)
            .expect("the update is judged")
    };

    // A note added to the property and to the definition is compatible
    let described = contract(
        2,
        schema(long_identifier(
            platform_value!({ "position": 0, "description": "who is paid" }),
        )),
        sponsor(long_identifier(
            platform_value!({ "description": "who pays" }),
        )),
    );
    let result = judge(&described);
    assert!(result.is_valid(), "{:?}", result.errors);

    // An identifier turned into plain bytes is not, as a property or as a definition
    let property_changed = contract(
        2,
        schema(long_bytes(32, platform_value!({ "position": 0 }))),
        sponsor(long_identifier(platform_value!({}))),
    );
    assert!(!judge(&property_changed).is_valid());
    let definition_changed = contract(
        2,
        schema(long_identifier(platform_value!({ "position": 0 }))),
        sponsor(long_bytes(32, platform_value!({}))),
    );
    assert!(!judge(&definition_changed).is_valid());
}
