//! Typed arrays: a property that is `type: "array"` with an `items` element
//! schema in place of `byteArray`.
//!
//! The grammar is the v3 document meta-schema's (protocol version 14) and the
//! parse is `parse_typed_array` 0, which the tables select from protocol
//! version 14 only; earlier versions keep refusing an array that is not a
//! byte array. The array is stored inline in the document, so these tests
//! also cover the codec, the document validation and the contract
//! serialization of a type that carries one.

use super::*;
use crate::consensus::basic::json_schema_error::JsonSchemaError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::array::{ArrayItemType, TypedArrayProperty};
use crate::data_contract::document_type::DocumentPropertyType;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::serialized_version::v0::DataContractInSerializationFormatV0;
use crate::data_contract::DataContract;
use platform_value::platform_value;

/// Parse through the real dispatcher, which picks the parser generation out
/// of the platform version's `try_from_schema` table value (generation 2 at
/// PV13, generation 3 at PV14).
fn parse_dispatched(
    schema: Value,
    platform_version: &PlatformVersion,
    full_validation: bool,
) -> Result<DocumentType, ProtocolError> {
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available on this platform version");
    DocumentType::try_from_schema(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "charter",
        schema,
        None,
        &BTreeMap::new(),
        &config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

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

/// The error of a validating parse the meta-schema refused.
fn expect_json_schema_error<T: std::fmt::Debug>(
    result: Result<T, ProtocolError>,
) -> JsonSchemaError {
    match result {
        Err(ProtocolError::ConsensusError(boxed)) => match *boxed {
            ConsensusError::BasicError(BasicError::JsonSchemaError(error)) => error,
            other => panic!("expected a JSON schema error, got {other:?}"),
        },
        other => panic!("expected a JSON schema error, got {other:?}"),
    }
}

/// The parser's structure errors surface as `InvalidContractStructure`
/// either directly or, with the `validation` feature on, wrapped as the basic
/// `ContractError`.
fn expect_structure_error<T: std::fmt::Debug>(result: Result<T, ProtocolError>, needle: &str) {
    let message = match result {
        Err(ProtocolError::DataContractError(DataContractError::InvalidContractStructure(
            message,
        ))) => message,
        Err(ProtocolError::ConsensusError(boxed)) => match *boxed {
            ConsensusError::BasicError(BasicError::ContractError(
                DataContractError::InvalidContractStructure(message),
            )) => message,
            other => panic!("expected InvalidContractStructure, got {other:?}"),
        },
        other => panic!("expected InvalidContractStructure, got {other:?}"),
    };
    assert!(
        message.contains(needle),
        "expected {needle:?} in the error, got: {message}"
    );
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
            item_type: ArrayItemType::Identifier,
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
            item_type: ArrayItemType::Integer,
            min_items: Some(1),
            max_items: 10,
            unique_items: false,
        })
    );
    // A one-byte element count, then 1 to 10 elements of 8 bytes each
    assert_eq!(
        property_type
            .min_byte_size(platform_version)
            .expect("sized"),
        Some(9)
    );
    assert_eq!(
        property_type
            .max_byte_size(platform_version)
            .expect("sized"),
        Some(81)
    );
}

#[test]
fn should_parse_every_scalar_element_type() {
    for (items, expected) in [
        (platform_value!({ "type": "number" }), ArrayItemType::Number),
        (
            platform_value!({ "type": "boolean" }),
            ArrayItemType::Boolean,
        ),
        (
            platform_value!({ "type": "string", "minLength": 1, "maxLength": 20 }),
            ArrayItemType::String(Some(1), Some(20)),
        ),
        (
            platform_value!({ "type": "array", "byteArray": true, "minItems": 4, "maxItems": 8 }),
            ArrayItemType::ByteArray(Some(4), Some(8)),
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
        assert_eq!(typed_array.item_type, expected, "{items:?}");
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
    let limit = platform_version.system_limits.max_document_array_items;

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

#[test]
fn should_keep_the_byte_array_form_free_of_items_and_unique_items() {
    for (keyword, value) in [
        ("uniqueItems", Value::Bool(true)),
        ("items", platform_value!({ "type": "integer" })),
    ] {
        let mut list = platform_value!({
            "type": "array",
            "byteArray": true,
            "maxItems": 16,
            "position": 0
        });
        list.set_value(keyword, value).expect("keyword applies");

        let error = expect_json_schema_error(parse_dispatched(
            schema_with_list(list),
            PlatformVersion::latest(),
            true,
        ));
        assert!(
            error.instance_path().ends_with(&format!("/list/{keyword}")),
            "{keyword} on a byte array should be refused, got {}",
            error.instance_path()
        );
    }
}

#[test]
fn should_refuse_refers_to_on_the_elements_of_a_typed_array() {
    let list = platform_value!({
        "type": "array",
        "maxItems": 4,
        "items": {
            "type": "array",
            "byteArray": true,
            "minItems": 32,
            "maxItems": 32,
            "contentMediaType": "application/x.dash.dpp.identifier",
            "refersTo": { "type": "identity" }
        },
        "position": 0
    });

    expect_json_schema_error(parse_dispatched(
        schema_with_list(list.clone()),
        PlatformVersion::latest(),
        true,
    ));
    expect_structure_error(
        parse_dispatched(schema_with_list(list), PlatformVersion::latest(), false),
        "refersTo is not supported on the elements of a typed array",
    );
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
            item_type: ArrayItemType::Identifier,
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
    use crate::document::DocumentV0Getters;
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
