//! Typed arrays: a property that is `type: "array"` with an `items` element
//! schema in place of `byteArray`.
//!
//! The grammar is the v3 document meta-schema's (protocol version 14) and the
//! parse is `parse_typed_array` 0, which the tables select from protocol
//! version 14 only; earlier versions keep refusing an array that is not a
//! byte array. The array is stored inline in the document, so these tests
//! also cover the codec, the document validation and the contract
//! serialization of a type that carries one.

use super::typed_array_test_helpers::{
    expect_json_schema_error, expect_structure_error, parse_dispatched,
};
use super::*;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::array::{ArrayItemConstraints, TypedArrayProperty};
use crate::data_contract::document_type::{
    ByteArrayPropertySizes, DocumentPropertyType, StringPropertySizes,
};
use crate::data_contract::serialized_version::v0::DataContractInSerializationFormatV0;
use crate::data_contract::DataContract;
use platform_value::platform_value;

/// A document type with one property, `list`, declared by `list_schema`.
fn schema_with_list(list_schema: Value) -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "list": list_schema,
        },
        "additionalProperties": false
    })
}

/// The `reasons` declaration of the moderation charters contract: up to 64
/// distinct identifiers.
fn reasons_list() -> Value {
    platform_value!({
        "type": "array",
        "minItems": 0,
        "maxItems": 64,
        "uniqueItems": true,
        "items": {
            "type": "array",
            "byteArray": true,
            "minItems": 32,
            "maxItems": 32,
            "contentMediaType": "application/x.dash.dpp.identifier"
        },
        "position": 0
    })
}

fn list_property_type(document_type: &DocumentType) -> DocumentPropertyType {
    document_type
        .as_ref()
        .flattened_properties()
        .get("list")
        .map(|property| property.property_type.clone())
        .expect("the list property is parsed")
}

#[test]
fn should_parse_a_typed_identifier_array() {
    let document_type = parse_dispatched(
        schema_with_list(reasons_list()),
        PlatformVersion::latest(),
        true,
    )
    .expect("a typed identifier array parses");

    assert_eq!(
        list_property_type(&document_type),
        DocumentPropertyType::TypedArray(TypedArrayProperty {
            item_type: Box::new(DocumentPropertyType::Identifier),
            item_constraints: Default::default(),
            min_items: Some(0),
            max_items: 64,
            unique_items: true,
        })
    );
}

#[test]
fn should_parse_a_typed_integer_array_with_bounds() {
    let platform_version = PlatformVersion::latest();
    let document_type = parse_dispatched(
        schema_with_list(platform_value!({
            "type": "array",
            "minItems": 1,
            "maxItems": 10,
            "items": { "type": "integer", "minimum": 0, "maximum": 100 },
            "position": 0
        })),
        platform_version,
        true,
    )
    .expect("a typed integer array parses");

    let property_type = list_property_type(&document_type);
    assert_eq!(
        property_type,
        DocumentPropertyType::TypedArray(TypedArrayProperty {
            item_type: Box::new(DocumentPropertyType::U8),
            // The bounds keep the value kinds the schema literal gave them
            item_constraints: ArrayItemConstraints {
                allowed_values: None,
                minimum: Some(Value::I32(0)),
                maximum: Some(Value::I32(100)),
            },
            min_items: Some(1),
            max_items: 10,
            unique_items: false,
        })
    );
    // A one-byte element count, then 1 to 10 elements of one byte each: the
    // element takes the width a scalar property bounded 0 to 100 takes
    assert_eq!(
        property_type
            .min_byte_size(platform_version)
            .expect("sized"),
        Some(2)
    );
    assert_eq!(
        property_type
            .max_byte_size(platform_version)
            .expect("sized"),
        Some(11)
    );
}

#[test]
fn should_parse_every_scalar_element_type() {
    for (items, expected) in [
        (
            platform_value!({ "type": "number" }),
            DocumentPropertyType::F64,
        ),
        (
            platform_value!({ "type": "boolean" }),
            DocumentPropertyType::Boolean,
        ),
        (
            platform_value!({ "type": "string", "minLength": 1, "maxLength": 20 }),
            DocumentPropertyType::String(StringPropertySizes {
                min_length: Some(1),
                max_length: Some(20),
                max_bytes: None,
            }),
        ),
        (
            platform_value!({ "type": "array", "byteArray": true, "minItems": 4, "maxItems": 8 }),
            DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
                min_size: Some(4),
                max_size: Some(8),
            }),
        ),
        (
            platform_value!({ "type": "integer", "minimum": -5, "maximum": 5 }),
            DocumentPropertyType::I8,
        ),
        (
            platform_value!({ "type": "integer", "minimum": 0, "maximum": 70000 }),
            DocumentPropertyType::U32,
        ),
    ] {
        let document_type = parse_dispatched(
            schema_with_list(platform_value!({
                "type": "array",
                "maxItems": 4,
                "items": items.clone(),
                "position": 0
            })),
            PlatformVersion::latest(),
            true,
        )
        .unwrap_or_else(|e| panic!("{items:?} elements should parse: {e}"));

        let DocumentPropertyType::TypedArray(typed_array) = list_property_type(&document_type)
        else {
            panic!("{items:?} should parse to a typed array");
        };
        assert_eq!(*typed_array.item_type, expected, "{items:?}");
    }
}

#[test]
fn should_refuse_arrays_of_objects() {
    let list = platform_value!({
        "type": "array",
        "maxItems": 4,
        "items": {
            "type": "object",
            "properties": { "name": { "type": "string", "maxLength": 10, "position": 0 } },
            "additionalProperties": false
        },
        "position": 0
    });

    let error = expect_json_schema_error(parse_dispatched(
        schema_with_list(list.clone()),
        PlatformVersion::latest(),
        true,
    ));
    assert!(
        error.instance_path().ends_with("/list/items/type")
            || error.instance_path().contains("/list/items"),
        "the meta-schema should point at the element schema, got {}",
        error.instance_path()
    );

    // A parse that skips the meta-schema refuses it with the same rule
    expect_structure_error(
        parse_dispatched(schema_with_list(list), PlatformVersion::latest(), false),
        "arrays of objects are not supported",
    );
}

#[test]
fn should_refuse_arrays_of_arrays() {
    for items in [
        platform_value!({ "type": "array" }),
        platform_value!({ "type": "array", "items": { "type": "integer" }, "maxItems": 2 }),
    ] {
        let list = platform_value!({
            "type": "array",
            "maxItems": 4,
            "items": items.clone(),
            "position": 0
        });

        expect_json_schema_error(parse_dispatched(
            schema_with_list(list.clone()),
            PlatformVersion::latest(),
            true,
        ));
        expect_structure_error(
            parse_dispatched(schema_with_list(list), PlatformVersion::latest(), false),
            "arrays of arrays are not supported",
        );
    }
}

#[test]
fn should_refuse_an_index_on_a_typed_array_property() {
    let mut schema = schema_with_list(reasons_list());
    schema
        .set_value(
            "indices",
            platform_value!([{ "name": "byList", "properties": [{ "list": "asc" }] }]),
        )
        .expect("indices apply");

    let result = parse_dispatched(schema, PlatformVersion::latest(), true);

    assert!(
        matches!(
            &result,
            Err(ProtocolError::ConsensusError(boxed))
                if matches!(
                    **boxed,
                    ConsensusError::BasicError(BasicError::InvalidIndexPropertyTypeError(_))
                )
        ),
        "an index on a typed array should be refused as an invalid index property type, got \
         {result:?}"
    );
}

/// The ranked key-length check runs before the index property-type check and
/// would otherwise size the whole list as a key, so it skips a typed array:
/// the type error is the one reported.
#[test]
fn should_refuse_a_typed_array_in_an_index_with_a_ranked_axis_as_an_invalid_index_type() {
    let mut schema = schema_with_list(reasons_list());
    schema
        .set_value(
            "indices",
            platform_value!([{
                "name": "byList",
                "properties": [{ "list": "asc" }],
                "rangeCountable": true,
                "rankedCountable": true
            }]),
        )
        .expect("indices apply");

    let result = parse_dispatched(schema, PlatformVersion::latest(), true);

    assert!(
        matches!(
            &result,
            Err(ProtocolError::ConsensusError(boxed))
                if matches!(
                    **boxed,
                    ConsensusError::BasicError(BasicError::InvalidIndexPropertyTypeError(_))
                )
        ),
        "a ranked index on a typed array should be refused as an invalid index property type, \
         got {result:?}"
    );
}

#[test]
fn should_refuse_a_typed_array_below_protocol_version_14_and_accept_it_at_14() {
    let schema = schema_with_list(reasons_list());
    let platform_version_13 = PlatformVersion::get(13).expect("protocol version 13 should exist");

    // Meta-schema v2 has no typed arrays
    expect_json_schema_error(parse_dispatched(schema.clone(), platform_version_13, true));
    // and without it the shipped parse refuses an array that is not a byte
    // array, exactly as it always did
    expect_structure_error(
        parse_dispatched(schema.clone(), platform_version_13, false),
        "only byte arrays are supported now",
    );

    parse_dispatched(schema, PlatformVersion::latest(), true)
        .expect("protocol version 14 parses typed arrays");
}

#[test]
fn should_require_max_items_on_a_typed_array_within_the_system_limit() {
    let platform_version = PlatformVersion::latest();
    let limit = platform_version.system_limits.max_typed_array_items;

    // maxItems is the shape of the declaration: required on every parse
    let without_max_items = schema_with_list(platform_value!({
        "type": "array",
        "items": { "type": "boolean" },
        "position": 0
    }));
    let error = expect_json_schema_error(parse_dispatched(
        without_max_items.clone(),
        platform_version,
        true,
    ));
    assert_eq!(error.keyword(), "required");
    expect_structure_error(
        parse_dispatched(without_max_items, platform_version, false),
        "a typed array must declare maxItems",
    );

    // The cap is a registration limit: a stored contract is read as declared
    let over_the_limit = schema_with_list(platform_value!({
        "type": "array",
        "maxItems": u64::from(limit) + 1,
        "items": { "type": "boolean" },
        "position": 0
    }));
    expect_structure_error(
        parse_dispatched(over_the_limit.clone(), platform_version, true),
        &format!("above the maximum of {limit}"),
    );
    parse_dispatched(over_the_limit, platform_version, false)
        .expect("the non-validating parse reads the declaration as it is");

    let at_the_limit = schema_with_list(platform_value!({
        "type": "array",
        "maxItems": limit,
        "items": { "type": "boolean" },
        "position": 0
    }));
    parse_dispatched(at_the_limit, platform_version, true).expect("maxItems at the limit parses");
}

#[test]
fn should_hold_the_shape_rules_of_a_typed_array_without_the_meta_schema() {
    for (list, needle) in [
        (
            platform_value!({
                "type": "array",
                "minItems": 5,
                "maxItems": 4,
                "items": { "type": "integer" },
                "position": 0
            }),
            "minItems may not exceed its maxItems",
        ),
        (
            platform_value!({
                "type": "array",
                "maxItems": 4,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "items": { "type": "integer" },
                "position": 0
            }),
            "contentMediaType belongs on the items",
        ),
    ] {
        for full_validation in [true, false] {
            expect_structure_error_or_json_schema_error(
                parse_dispatched(
                    schema_with_list(list.clone()),
                    PlatformVersion::latest(),
                    full_validation,
                ),
                full_validation,
                needle,
            );
        }
    }
}

/// On the validating path the meta-schema may refuse a declaration before the
/// parser sees it; without it the parser has to refuse it itself.
fn expect_structure_error_or_json_schema_error<T: std::fmt::Debug>(
    result: Result<T, ProtocolError>,
    full_validation: bool,
    needle: &str,
) {
    match result {
        Err(ProtocolError::ConsensusError(boxed))
            if full_validation
                && matches!(
                    *boxed,
                    ConsensusError::BasicError(BasicError::JsonSchemaError(_))
                ) => {}
        result => expect_structure_error(result, needle),
    }
}

/// The `uniqueItems` refusal on identifiers is safe for every stored
/// contract: a census on 2026-09-23 of every data contract create and update
/// transition on mainnet (72) and testnet (4593), decoded from the raw bytes
/// with dpp (see `reference_mainnet_explorer_transition_audit` for the
/// method), found no `uniqueItems` on any property of any contract, and the
/// only `items` keywords on 22 testnet creates that were refused. Nothing a
/// node ever stored has to drop a keyword to update at protocol version 14.
#[test]
fn should_refuse_items_on_a_byte_array_and_unique_items_on_an_identifier() {
    let byte_array = platform_value!({
        "type": "array",
        "byteArray": true,
        "maxItems": 16,
        "position": 0
    });
    let identifier = platform_value!({
        "type": "array",
        "byteArray": true,
        "minItems": 32,
        "maxItems": 32,
        "contentMediaType": "application/x.dash.dpp.identifier",
        "position": 0
    });

    for (mut list, keyword, value) in [
        (
            byte_array.clone(),
            "items",
            platform_value!({ "type": "integer" }),
        ),
        // An identifier is one value: "no repeated byte" would refuse most
        // identifiers
        (identifier.clone(), "uniqueItems", Value::Bool(true)),
    ] {
        list.set_value(keyword, value).expect("keyword applies");

        let error = expect_json_schema_error(parse_dispatched(
            schema_with_list(list),
            PlatformVersion::latest(),
            true,
        ));
        assert!(
            error.instance_path().ends_with(&format!("/list/{keyword}")),
            "{keyword} should be refused, got {}",
            error.instance_path()
        );
    }

    // On a plain byte array uniqueItems keeps its meaning, no repeated byte
    let mut list = byte_array;
    list.set_value("uniqueItems", Value::Bool(true))
        .expect("keyword applies");
    parse_dispatched(schema_with_list(list), PlatformVersion::latest(), true)
        .expect("uniqueItems on a plain byte array parses");
}

/// An element may be limited to allowed values with `enum`, but takes no
/// `const` (a list of one repeated value carries only its length, and a
/// one-value `enum` does the same while an update can still widen it) and no
/// `examples`.
#[test]
fn should_accept_enum_and_refuse_const_and_examples_on_the_elements_of_a_typed_array() {
    let list_with_items = |items: Value| {
        schema_with_list(platform_value!({
            "type": "array",
            "maxItems": 4,
            "items": items,
            "position": 0
        }))
    };

    parse_dispatched(
        list_with_items(platform_value!({
            "type": "string",
            "maxLength": 20,
            "enum": ["spam", "abuse", "offTopic"]
        })),
        PlatformVersion::latest(),
        true,
    )
    .expect("enum on an element parses");

    for items in [
        platform_value!({ "type": "integer", "const": 1 }),
        platform_value!({ "type": "integer", "examples": [1] }),
    ] {
        let error = expect_json_schema_error(parse_dispatched(
            list_with_items(items.clone()),
            PlatformVersion::latest(),
            true,
        ));
        assert!(
            error.instance_path().ends_with("/list/items"),
            "{items:?} should be refused on an element, got {}",
            error.instance_path()
        );
    }
}

/// A contract whose `charter` type carries a typed array of every element
/// type, `reasons` and `counts` required and the rest optional.
fn charter_contract(platform_version: &PlatformVersion) -> DataContract {
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available on this platform version");
    let charter = platform_value!({
        "type": "object",
        "properties": {
            "reasons": reasons_list(),
            "counts": {
                "type": "array",
                "minItems": 1,
                "maxItems": 5,
                "items": { "type": "integer" },
                "position": 1
            },
            "labels": {
                "type": "array",
                "maxItems": 5,
                "items": { "type": "string", "minLength": 1, "maxLength": 20 },
                "position": 2
            },
            "flags": {
                "type": "array",
                "maxItems": 2,
                "uniqueItems": true,
                "items": { "type": "boolean" },
                "position": 3
            },
            "digests": {
                "type": "array",
                "maxItems": 3,
                "items": { "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32 },
                "position": 4
            },
            "weights": {
                "type": "array",
                "maxItems": 3,
                "items": { "type": "number" },
                "position": 5
            },
            "tags": {
                "type": "array",
                "maxItems": 3,
                "uniqueItems": true,
                "items": { "type": "string", "maxLength": 20, "enum": ["spam", "abuse", "offTopic"] },
                "position": 6
            },
            "scores": {
                "type": "array",
                "minItems": 1,
                "maxItems": 4,
                "items": { "type": "integer", "minimum": 0, "maximum": 100 },
                "position": 7
            },
            "ratios": {
                "type": "array",
                "maxItems": 2,
                "items": { "type": "number", "minimum": 0, "maximum": 1 },
                "position": 8
            }
        },
        "required": ["reasons", "counts"],
        "additionalProperties": false
    });

    DataContract::try_from_platform_versioned(
        DataContractInSerializationFormatV0 {
            id: Identifier::new([7; 32]),
            config,
            version: 1,
            owner_id: Identifier::new([8; 32]),
            schema_defs: None,
            document_schemas: BTreeMap::from([("charter".to_string(), charter)]),
        }
        .into(),
        true,
        &mut vec![],
        platform_version,
    )
    .expect("the charter contract registers")
}

/// A contract whose `note` type nests typed arrays inside an object: a list
/// of identifiers and a list of byte arrays under `team`.
fn nested_lists_contract(platform_version: &PlatformVersion) -> DataContract {
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available on this platform version");
    let note = platform_value!({
        "type": "object",
        "properties": {
            "reasons": reasons_list(),
            "team": {
                "type": "object",
                "position": 1,
                "properties": {
                    "leads": {
                        "type": "array",
                        "maxItems": 4,
                        "items": {
                            "type": "array",
                            "byteArray": true,
                            "minItems": 32,
                            "maxItems": 32,
                            "contentMediaType": "application/x.dash.dpp.identifier"
                        },
                        "position": 0
                    },
                    "digests": {
                        "type": "array",
                        "maxItems": 3,
                        "items": { "type": "array", "byteArray": true, "minItems": 4, "maxItems": 8 },
                        "position": 1
                    }
                },
                "additionalProperties": false
            }
        },
        "required": ["reasons"],
        "additionalProperties": false
    });

    DataContract::try_from_platform_versioned(
        DataContractInSerializationFormatV0 {
            id: Identifier::new([7; 32]),
            config,
            version: 1,
            owner_id: Identifier::new([8; 32]),
            schema_defs: None,
            document_schemas: BTreeMap::from([("note".to_string(), note)]),
        }
        .into(),
        true,
        &mut vec![],
        platform_version,
    )
    .expect("the nested lists contract registers")
}

/// Built by hand: `platform_value!` would store the identifiers as bytes,
/// not as the identifiers the decoder reads back.
fn charter_properties() -> Value {
    Value::Map(vec![
        (
            Value::Text("reasons".to_string()),
            Value::Array(vec![Value::Identifier([3; 32]), Value::Identifier([4; 32])]),
        ),
        (
            Value::Text("counts".to_string()),
            Value::Array(vec![Value::I64(-5), Value::I64(0), Value::I64(i64::MAX)]),
        ),
        (
            Value::Text("labels".to_string()),
            Value::Array(vec![
                Value::Text("spam".to_string()),
                Value::Text("abuse".to_string()),
            ]),
        ),
        (
            Value::Text("flags".to_string()),
            Value::Array(vec![Value::Bool(true)]),
        ),
        (
            Value::Text("digests".to_string()),
            Value::Array(vec![Value::Bytes32([9; 32])]),
        ),
        (
            Value::Text("weights".to_string()),
            Value::Array(vec![Value::Float(0.5)]),
        ),
    ])
}

#[test]
fn should_round_trip_a_contract_with_typed_arrays_through_platform_serialization() {
    use crate::serialization::{
        PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted,
        PlatformSerializableWithPlatformVersion,
    };

    let platform_version = PlatformVersion::latest();
    let contract = charter_contract(platform_version);

    let bytes = contract
        .serialize_to_bytes_with_platform_version(platform_version)
        .expect("the contract serializes");
    let restored = DataContract::versioned_deserialize_untrusted(&bytes, true, platform_version)
        .expect("the contract deserializes with full validation");

    assert_eq!(restored, contract);
    let reasons = restored
        .document_type_for_name("charter")
        .expect("charter type")
        .flattened_properties()
        .get("reasons")
        .map(|property| property.property_type.clone());
    assert_eq!(
        reasons,
        Some(DocumentPropertyType::TypedArray(TypedArrayProperty {
            item_type: Box::new(DocumentPropertyType::Identifier),
            item_constraints: Default::default(),
            min_items: Some(0),
            max_items: 64,
            unique_items: true,
        }))
    );
}

#[test]
fn should_round_trip_a_document_with_typed_arrays_through_serialization() {
    use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use crate::document::{Document, DocumentV0, DocumentV0Getters};

    let platform_version = PlatformVersion::latest();
    let contract = charter_contract(platform_version);
    let document_type = contract
        .document_type_for_name("charter")
        .expect("charter type");

    let full = charter_properties()
        .into_btree_string_map()
        .expect("properties are a map");
    // The required arrays alone, one of them empty
    let required_only = BTreeMap::from([
        ("reasons".to_string(), Value::Array(vec![])),
        ("counts".to_string(), Value::Array(vec![Value::I64(7)])),
    ]);

    for properties in [full, required_only] {
        let document: Document = DocumentV0 {
            contract_version: None,
            id: Identifier::new([1; 32]),
            owner_id: Identifier::new([2; 32]),
            properties,
            revision: Some(1),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        }
        .into();

        let bytes = document
            .serialize(document_type, &contract, platform_version)
            .expect("the document serializes");
        let restored = Document::from_bytes(&bytes, document_type, platform_version)
            .expect("the document deserializes");

        assert_eq!(restored.properties(), document.properties());
    }
}

/// The identifier and byte array elements are conversion paths (`reasons[]`,
/// `digests[]`), so a document built from data carrying base58 strings and
/// plain bytes holds identifiers and bytes, as it does for scalar properties.
#[test]
fn should_convert_the_elements_of_typed_arrays_when_creating_a_document_from_data() {
    use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
    use crate::data_contract::methods::validate_document::DataContractDocumentValidationMethodsV0;
    use crate::document::DocumentV0Getters;
    use platform_value::btreemap_extensions::BTreeValueMapPathHelper;
    use platform_value::string_encoding::Encoding;

    let platform_version = PlatformVersion::latest();
    let contract = charter_contract(platform_version);
    let document_type = contract
        .document_type_for_name("charter")
        .expect("charter type");

    assert!(document_type.identifier_paths().contains("reasons[]"));
    assert!(document_type.binary_paths().contains("digests[]"));

    let reason = Identifier::new([5; 32]);
    let data = Value::Map(vec![
        (
            Value::Text("reasons".to_string()),
            Value::Array(vec![Value::Text(reason.to_string(Encoding::Base58))]),
        ),
        (
            Value::Text("counts".to_string()),
            Value::Array(vec![Value::I64(1)]),
        ),
    ]);
    let document = document_type
        .create_document_from_data(
            data,
            Identifier::new([2; 32]),
            1,
            1,
            [3; 32],
            platform_version,
        )
        .expect("the document is created");

    assert_eq!(
        document.properties().get("reasons"),
        Some(&Value::Array(vec![Value::Identifier([5; 32])]))
    );
    let result = contract
        .validate_document("charter", &document, platform_version)
        .expect("validation runs");
    assert!(result.is_valid(), "{result:?}");

    // A list nested in an object converts through its dotted list path
    let contract = nested_lists_contract(platform_version);
    let document_type = contract.document_type_for_name("note").expect("note type");
    assert!(document_type.identifier_paths().contains("team.leads[]"));
    assert!(document_type.binary_paths().contains("team.digests[]"));
    let lead = Identifier::new([9; 32]);
    let data = Value::Map(vec![
        (
            Value::Text("reasons".to_string()),
            Value::Array(vec![Value::Text(reason.to_string(Encoding::Base58))]),
        ),
        (
            Value::Text("team".to_string()),
            Value::Map(vec![
                (
                    Value::Text("leads".to_string()),
                    Value::Array(vec![Value::Text(lead.to_string(Encoding::Base58))]),
                ),
                (
                    Value::Text("digests".to_string()),
                    Value::Array(vec![Value::Bytes(vec![1, 2, 3, 4])]),
                ),
            ]),
        ),
    ]);
    let document = document_type
        .create_document_from_data(
            data,
            Identifier::new([2; 32]),
            1,
            1,
            [3; 32],
            platform_version,
        )
        .expect("the document is created");
    assert_eq!(
        document
            .properties()
            .get_optional_at_path("team.leads")
            .expect("a nested list is reachable"),
        Some(&Value::Array(vec![Value::Identifier([9; 32])]))
    );
    let result = contract
        .validate_document("note", &document, platform_version)
        .expect("validation runs");
    assert!(result.is_valid(), "{result:?}");
}

#[test]
fn should_convert_the_members_of_a_typed_array_set_on_an_extended_document() {
    use crate::data_contract::document_type::random_document::CreateRandomDocument;
    use crate::document::extended_document::v0::ExtendedDocumentV0;
    use crate::document::DocumentV0Getters;
    use platform_value::string_encoding::Encoding;

    let platform_version = PlatformVersion::latest();
    let contract = charter_contract(platform_version);
    let document = contract
        .document_type_for_name("charter")
        .expect("charter type")
        .random_document(Some(21), platform_version)
        .expect("a random document");
    let mut extended = ExtendedDocumentV0::from_document_with_additional_info(
        document,
        contract,
        "charter".to_string(),
        None,
    );

    let reason = Identifier::new([7; 32]);
    extended
        .set_untrusted(
            "reasons",
            Value::Array(vec![Value::Text(reason.to_string(Encoding::Base58))]),
        )
        .expect("a list of base58 identifiers is set");
    assert_eq!(
        extended.document.properties().get("reasons"),
        Some(&Value::Array(vec![Value::Identifier([7; 32])]))
    );

    extended
        .set_untrusted(
            "digests",
            Value::Array(vec![Value::Text("AQIDBA==".to_string())]),
        )
        .expect("a list of base64 byte arrays is set");
    assert_eq!(
        extended.document.properties().get("digests"),
        Some(&Value::Array(vec![Value::Bytes(vec![1, 2, 3, 4])]))
    );

    assert!(extended
        .set_untrusted("reasons", Value::Text("not a list".to_string()))
        .is_err());
}

#[test]
fn should_refuse_element_constraints_no_element_could_satisfy_on_both_paths() {
    for (items, needle) in [
        (
            platform_value!({ "type": "integer", "enum": ["a"] }),
            "must be a",
        ),
        (
            platform_value!({ "type": "string", "enum": [] }),
            "at least one value",
        ),
        (
            platform_value!({
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "enum": [[1]]
            }),
            "not supported on byte array or identifier",
        ),
        (
            platform_value!({ "type": "integer", "minimum": 5, "maximum": 4 }),
            "may not exceed their maximum",
        ),
        (
            platform_value!({ "type": "number", "minimum": "low" }),
            "minimum of a typed array's elements must be a",
        ),
    ] {
        let list = platform_value!({
            "type": "array",
            "maxItems": 4,
            "items": items,
            "position": 0
        });
        for full_validation in [true, false] {
            expect_structure_error_or_json_schema_error(
                parse_dispatched(
                    schema_with_list(list.clone()),
                    PlatformVersion::latest(),
                    full_validation,
                ),
                full_validation,
                needle,
            );
        }
    }
}

#[test]
fn should_refuse_a_document_whose_typed_array_breaks_its_schema() {
    use crate::data_contract::methods::validate_document::DataContractDocumentValidationMethodsV0;

    let platform_version = PlatformVersion::latest();
    let contract = charter_contract(platform_version);

    contract
        .validate_document_properties("charter", charter_properties(), platform_version)
        .map(|result| assert!(result.is_valid(), "the base document is valid: {result:?}"))
        .expect("validation runs");

    let too_many_counts: Vec<Value> = (0..6).map(Value::I64).collect();
    for (property, value, keyword) in [
        ("counts", Value::Array(too_many_counts), "maxItems"),
        ("counts", Value::Array(vec![]), "minItems"),
        (
            "reasons",
            Value::Array(vec![Value::Identifier([3; 32]), Value::Identifier([3; 32])]),
            "uniqueItems",
        ),
        ("counts", platform_value!([Value::I64(1), "two"]), "type"),
        (
            "reasons",
            platform_value!([Value::Bytes(vec![1; 31])]),
            "minItems",
        ),
        // The element schema's own bounds
        (
            "labels",
            Value::Array(vec![Value::Text("x".repeat(21))]),
            "maxLength",
        ),
        (
            "labels",
            Value::Array(vec![Value::Text(String::new())]),
            "minLength",
        ),
    ] {
        let mut properties = charter_properties();
        properties
            .set_value(property, value.clone())
            .expect("property applies");

        let result = contract
            .validate_document_properties("charter", properties, platform_version)
            .expect("validation returns a consensus result, never an error");

        let Some(ConsensusError::BasicError(BasicError::JsonSchemaError(error))) =
            result.first_error()
        else {
            panic!("{property} = {value:?} should be refused by the JSON schema, got {result:?}");
        };
        assert_eq!(
            error.keyword(),
            keyword,
            "{property} = {value:?} should break {keyword}, got {error:?}"
        );
    }
}

#[test]
fn should_generate_random_documents_that_validate_against_their_own_schema() {
    use crate::data_contract::document_type::random_document::{
        CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
    };
    use crate::data_contract::methods::validate_document::DataContractDocumentValidationMethodsV0;
    use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use crate::document::{Document, DocumentV0Getters};
    use platform_value::Bytes32;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    let platform_version = PlatformVersion::latest();
    let contract = charter_contract(platform_version);
    let document_type = contract
        .document_type_for_name("charter")
        .expect("charter type");
    let mut rng = StdRng::seed_from_u64(14);

    let mut documents: Vec<(String, Document)> = (0..8)
        .map(|seed| {
            let document = document_type
                .random_document(Some(seed), platform_version)
                .expect("a random document");
            ("random_document".to_string(), document)
        })
        .collect();
    for fill_size in [
        DocumentFieldFillSize::MinDocumentFillSize,
        DocumentFieldFillSize::MaxDocumentFillSize,
        DocumentFieldFillSize::AnyDocumentFillSize,
    ] {
        for _ in 0..8 {
            let owner_id = Identifier::random_with_rng(&mut rng);
            let entropy = Bytes32::random_with_rng(&mut rng);
            let document = document_type
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    owner_id,
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    fill_size,
                    platform_version,
                )
                .expect("a random document");
            documents.push((format!("{fill_size:?}"), document));
        }
    }

    for (generator, document) in documents {
        let result = contract
            .validate_document("charter", &document, platform_version)
            .expect("validation runs");
        assert!(
            result.is_valid(),
            "a {generator} random document should satisfy its own schema: {result:?} for {:?}",
            document.properties()
        );

        let bytes = document
            .serialize(document_type, &contract, platform_version)
            .expect("the random document serializes");
        let restored = Document::from_bytes(&bytes, document_type, platform_version)
            .expect("the random document deserializes");
        assert_eq!(restored.properties(), document.properties());
    }
}
