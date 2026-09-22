//! Typed scalar arrays: `type: array` declared by an `items` schema instead of
//! `byteArray: true`, generation 3 (protocol version 14).
//!
//! The array is one property stored inline in the document as a count
//! followed by its elements. These tests cover the parse (what is admitted,
//! what is refused and why), the document codec, the JSON schema validation
//! of the element count, uniqueness and item type, random document
//! generation within the bounds, the contract's platform serialization, and
//! the protocol version gate.

use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::array::ArrayItemType;
use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
use crate::data_contract::document_type::random_document::{
    CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
};
use crate::data_contract::document_type::{DocumentPropertyType, TypedArrayProperty};
use crate::data_contract::methods::validate_document::DataContractDocumentValidationMethodsV0;
use crate::data_contract::DataContract;
use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use crate::document::{Document, DocumentV0Getters};
use crate::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted,
    PlatformSerializableWithPlatformVersion,
};
use crate::ProtocolError;
use platform_value::{platform_value, Identifier, Value};
use platform_version::version::PlatformVersion;
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde_json::{json, Value as JsonValue};

const DOCUMENT_TYPE: &str = "charter";

/// A list of up to 64 distinct identifiers.
fn identifier_list_schema(position: u32) -> JsonValue {
    json!({
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
        "position": position
    })
}

/// One to four integers between 0 and 100.
fn bounded_integer_list_schema(position: u32) -> JsonValue {
    json!({
        "type": "array",
        "minItems": 1,
        "maxItems": 4,
        "items": { "type": "integer", "minimum": 0, "maximum": 100 },
        "position": position
    })
}

fn contract_json(properties: JsonValue, required: JsonValue, indices: JsonValue) -> JsonValue {
    let mut document_schema = json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    });
    // The meta-schema refuses an empty `indices` list: declare the key only with content
    if indices
        .as_array()
        .is_some_and(|indices| !indices.is_empty())
    {
        document_schema["indices"] = indices;
    }
    json!({
        "$formatVersion": "1",
        "id": "9k3RE6kHNTsDmyXFwEPpiFQ3ipXfp5FuXGXpQ1rDHDJb",
        "ownerId": "2b994p95akyNFKtkDnDvBRUotDbkH54MHwGbhQLr5gcU",
        "version": 1,
        "documentSchemas": {
            DOCUMENT_TYPE: document_schema
        }
    })
}

/// The contract the document tests share: a required identifier list and a
/// required bounded integer list.
fn lists_contract_json() -> JsonValue {
    contract_json(
        json!({
            "reasons": identifier_list_schema(0),
            "counts": bounded_integer_list_schema(1),
        }),
        json!(["reasons", "counts"]),
        json!([]),
    )
}

fn parse(json: JsonValue) -> Result<DataContract, ProtocolError> {
    DataContract::from_json(json, true, PlatformVersion::latest())
}

fn parse_property(schema: JsonValue) -> Result<DocumentPropertyType, ProtocolError> {
    let contract = parse(contract_json(
        json!({ "list": schema }),
        json!([]),
        json!([]),
    ))?;
    Ok(contract
        .document_type_for_name(DOCUMENT_TYPE)
        .expect("the document type parses")
        .properties()
        .get("list")
        .expect("the property parses")
        .property_type
        .clone())
}

fn refusal(schema: JsonValue) -> String {
    parse_property(schema)
        .expect_err("the schema should be refused")
        .to_string()
}

fn identifier(seed: u8) -> Value {
    Value::Identifier([seed; 32])
}

fn document_with(contract: &DataContract, properties: Value) -> Document {
    contract
        .document_type_for_name(DOCUMENT_TYPE)
        .expect("the document type parses")
        .create_document_from_data(
            properties,
            Identifier::from([1u8; 32]),
            1,
            1,
            [2u8; 32],
            PlatformVersion::latest(),
        )
        .expect("the document should build")
}

fn validate(contract: &DataContract, properties: Value) -> Vec<ConsensusError> {
    let document = document_with(contract, properties);
    contract
        .validate_document(DOCUMENT_TYPE, &document, PlatformVersion::latest())
        .expect("validation should run")
        .errors
}

fn assert_json_schema_refusal(errors: Vec<ConsensusError>, fragment: &str) {
    let error = errors
        .first()
        .unwrap_or_else(|| panic!("expected a refusal mentioning {fragment:?}"));
    assert!(
        matches!(
            error,
            ConsensusError::BasicError(BasicError::JsonSchemaError(_))
        ),
        "expected the JSON schema error, got {error:?}"
    );
    assert!(
        error.to_string().contains(fragment),
        "expected a refusal mentioning {fragment:?}, got {error}"
    );
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

#[test]
fn should_parse_a_typed_identifier_array() {
    assert_eq!(
        parse_property(identifier_list_schema(0)).expect("should parse"),
        DocumentPropertyType::TypedArray(TypedArrayProperty {
            items: ArrayItemType::Identifier,
            min_items: Some(0),
            max_items: 64,
            unique_items: true,
        })
    );
}

#[test]
fn should_parse_a_typed_integer_array_with_bounds() {
    assert_eq!(
        parse_property(bounded_integer_list_schema(0)).expect("should parse"),
        DocumentPropertyType::TypedArray(TypedArrayProperty {
            items: ArrayItemType::Integer,
            min_items: Some(1),
            max_items: 4,
            unique_items: false,
        })
    );
}

#[test]
fn should_parse_every_scalar_item_type_with_its_bounds() {
    for (items, expected) in [
        (json!({ "type": "number" }), ArrayItemType::Number),
        (json!({ "type": "boolean" }), ArrayItemType::Boolean),
        (
            json!({ "type": "string", "minLength": 1, "maxLength": 8 }),
            ArrayItemType::String(Some(1), Some(8)),
        ),
        (
            json!({ "type": "array", "byteArray": true, "minItems": 2, "maxItems": 20 }),
            ArrayItemType::ByteArray(Some(2), Some(20)),
        ),
    ] {
        let parsed = parse_property(json!({
            "type": "array",
            "maxItems": 3,
            "items": items,
            "position": 0
        }))
        .unwrap_or_else(|e| panic!("{expected:?} items should parse: {e}"));
        assert_eq!(
            parsed,
            DocumentPropertyType::TypedArray(TypedArrayProperty {
                items: expected,
                min_items: None,
                max_items: 3,
                unique_items: false,
            })
        );
    }
}

#[test]
fn should_keep_parsing_byte_arrays_exactly_as_before() {
    assert_eq!(
        parse_property(json!({
            "type": "array",
            "byteArray": true,
            "minItems": 32,
            "maxItems": 32,
            "contentMediaType": "application/x.dash.dpp.identifier",
            "position": 0
        }))
        .expect("should parse"),
        DocumentPropertyType::Identifier
    );
}

#[test]
fn should_refuse_an_array_of_objects() {
    let error = refusal(json!({
        "type": "array",
        "maxItems": 3,
        "items": {
            "type": "object",
            "properties": { "a": { "type": "string", "position": 0 } },
            "additionalProperties": false
        },
        "position": 0
    }));
    assert!(error.contains("items"), "unexpected refusal: {error}");
}

#[test]
fn should_refuse_an_array_of_arrays() {
    let error = refusal(json!({
        "type": "array",
        "maxItems": 3,
        "items": { "type": "array", "maxItems": 2, "items": { "type": "integer" } },
        "position": 0
    }));
    assert!(error.contains("items"), "unexpected refusal: {error}");
}

#[test]
fn should_refuse_a_typed_array_without_max_items() {
    // The meta-schema refuses it first, as the array form matching neither branch; the
    // parser's own "must declare maxItems" message is pinned on the stored path below
    let error = refusal(json!({
        "type": "array",
        "items": { "type": "integer" },
        "position": 0
    }));
    assert!(error.contains("oneOf"), "unexpected refusal: {error}");
}

#[test]
fn should_refuse_a_typed_array_that_is_also_a_byte_array() {
    let error = refusal(json!({
        "type": "array",
        "byteArray": true,
        "maxItems": 3,
        "items": { "type": "integer" },
        "position": 0
    }));
    assert!(
        error.contains("items") || error.contains("byteArray"),
        "unexpected refusal: {error}"
    );
}

#[test]
fn should_refuse_refers_to_on_the_items_of_a_typed_array() {
    let mut schema = identifier_list_schema(0);
    schema["items"]["refersTo"] = json!({ "type": "identity" });
    let error = refusal(schema);
    assert!(error.contains("refersTo"), "unexpected refusal: {error}");
}

#[test]
fn should_refuse_an_index_on_a_typed_array_property() {
    let error = parse(contract_json(
        json!({ "reasons": identifier_list_schema(0) }),
        json!([]),
        json!([{ "name": "byReasons", "properties": [{ "reasons": "asc" }] }]),
    ))
    .expect_err("an index on an array property should be refused");
    assert!(
        matches!(
            error,
            ProtocolError::ConsensusError(ref boxed)
                if matches!(**boxed, ConsensusError::BasicError(BasicError::InvalidIndexPropertyTypeError(_)))
        ),
        "expected InvalidIndexPropertyTypeError, got {error}"
    );
}

/// The stored path (`full_validation: false`) does not run the meta-schema,
/// so the parser itself must hold every rule.
#[test]
fn should_hold_the_parser_rules_without_the_meta_schema() {
    for (schema, fragment) in [
        (
            json!({ "type": "array", "items": { "type": "integer" }, "position": 0 }),
            "maxItems",
        ),
        (
            json!({
                "type": "array",
                "maxItems": 2,
                "items": { "type": "object", "properties": {}, "additionalProperties": false },
                "position": 0
            }),
            "arrays of objects",
        ),
        (
            json!({
                "type": "array",
                "maxItems": 2,
                "items": { "type": "array", "maxItems": 2, "items": { "type": "integer" } },
                "position": 0
            }),
            "arrays of arrays",
        ),
        (
            json!({
                "type": "array",
                "maxItems": 2,
                "items": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "refersTo": { "type": "identity" }
                },
                "position": 0
            }),
            "refersTo",
        ),
        (
            json!({
                "type": "array",
                "maxItems": 2,
                "minItems": 3,
                "items": { "type": "integer" },
                "position": 0
            }),
            "minItems",
        ),
        (
            json!({
                "type": "array",
                "maxItems": 2,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "items": { "type": "integer" },
                "position": 0
            }),
            "contentMediaType",
        ),
    ] {
        let error = DataContract::from_json(
            contract_json(json!({ "list": schema }), json!([]), json!([])),
            false,
            PlatformVersion::latest(),
        )
        .expect_err("the stored path should refuse it too")
        .to_string();
        assert!(
            error.contains(fragment),
            "expected a refusal mentioning {fragment:?}, got {error}"
        );
    }
}

#[test]
fn should_refuse_a_typed_array_over_the_element_cap() {
    let cap = PlatformVersion::latest()
        .system_limits
        .max_typed_array_items
        .expect("protocol version 14 caps typed arrays");
    let error = refusal(json!({
        "type": "array",
        "maxItems": cap + 1,
        "items": { "type": "boolean" },
        "position": 0
    }));
    assert!(
        error.contains("exceeds the maximum"),
        "unexpected refusal: {error}"
    );

    parse_property(json!({
        "type": "array",
        "maxItems": cap,
        "items": { "type": "boolean" },
        "position": 0
    }))
    .expect("the cap itself is admitted");
}

// ---------------------------------------------------------------------------
// The document codec
// ---------------------------------------------------------------------------

#[test]
fn should_round_trip_a_document_with_typed_arrays_through_the_codec() {
    let contract = parse(lists_contract_json()).expect("should parse");
    let document_type = contract
        .document_type_for_name(DOCUMENT_TYPE)
        .expect("the document type parses");
    let properties = platform_value!({
        "reasons": [identifier(3), identifier(4)],
        "counts": [7i64, 0i64, 100i64],
    });
    let document = document_with(&contract, properties);

    let bytes = document
        .serialize(document_type, &contract, PlatformVersion::latest())
        .expect("should serialize");
    let restored = Document::from_bytes(&bytes, document_type, PlatformVersion::latest())
        .expect("should deserialize");

    assert_eq!(
        restored.properties().get("reasons"),
        Some(&Value::Array(vec![identifier(3), identifier(4)]))
    );
    // Integer items always decode as i64
    assert_eq!(
        restored.properties().get("counts"),
        Some(&Value::Array(vec![
            Value::I64(7),
            Value::I64(0),
            Value::I64(100)
        ]))
    );
    assert_eq!(restored.properties(), document.properties());
}

#[test]
fn should_round_trip_an_empty_typed_array_and_an_absent_optional_one() {
    let contract = parse(contract_json(
        json!({
            "reasons": identifier_list_schema(0),
            "tags": {
                "type": "array",
                "maxItems": 3,
                "items": { "type": "string", "maxLength": 8 },
                "position": 1
            },
        }),
        json!(["reasons"]),
        json!([]),
    ))
    .expect("should parse");
    let document_type = contract
        .document_type_for_name(DOCUMENT_TYPE)
        .expect("the document type parses");
    let document = document_with(&contract, platform_value!({ "reasons": [] }));

    let bytes = document
        .serialize(document_type, &contract, PlatformVersion::latest())
        .expect("should serialize");
    let restored = Document::from_bytes(&bytes, document_type, PlatformVersion::latest())
        .expect("should deserialize");

    assert_eq!(
        restored.properties().get("reasons"),
        Some(&Value::Array(vec![]))
    );
    assert_eq!(restored.properties().get("tags"), None);
}

#[test]
fn should_convert_base58_identifier_items_when_creating_a_document_from_data() {
    let contract = parse(lists_contract_json()).expect("should parse");
    let id = Identifier::from([9u8; 32]);
    let document = document_with(
        &contract,
        platform_value!({
            "reasons": [id.to_string(platform_value::string_encoding::Encoding::Base58)],
            "counts": [1u64],
        }),
    );
    assert_eq!(
        document.properties().get("reasons"),
        Some(&Value::Array(vec![identifier(9)]))
    );
}

// ---------------------------------------------------------------------------
// Document validation
// ---------------------------------------------------------------------------

#[test]
fn should_accept_a_document_within_the_bounds() {
    let contract = parse(lists_contract_json()).expect("should parse");
    let errors = validate(
        &contract,
        platform_value!({
            "reasons": [identifier(1), identifier(2)],
            "counts": [0u64, 50u64, 100u64, 100u64],
        }),
    );
    assert!(errors.is_empty(), "unexpected refusal: {errors:?}");
}

#[test]
fn should_refuse_a_document_over_max_items() {
    let contract = parse(lists_contract_json()).expect("should parse");
    let errors = validate(
        &contract,
        platform_value!({
            "reasons": [],
            "counts": [1u64, 2u64, 3u64, 4u64, 5u64],
        }),
    );
    assert_json_schema_refusal(errors, "counts");
}

#[test]
fn should_refuse_a_document_under_min_items() {
    let contract = parse(lists_contract_json()).expect("should parse");
    let errors = validate(
        &contract,
        platform_value!({
            "reasons": [],
            "counts": [],
        }),
    );
    assert_json_schema_refusal(errors, "counts");
}

#[test]
fn should_refuse_a_repeated_element_under_unique_items() {
    let contract = parse(lists_contract_json()).expect("should parse");
    let errors = validate(
        &contract,
        platform_value!({
            "reasons": [identifier(1), identifier(1)],
            "counts": [1u64],
        }),
    );
    assert_json_schema_refusal(errors, "reasons");
}

#[test]
fn should_refuse_a_wrong_typed_element() {
    let contract = parse(lists_contract_json()).expect("should parse");
    let errors = validate(
        &contract,
        platform_value!({
            "reasons": [],
            "counts": ["one"],
        }),
    );
    assert_json_schema_refusal(errors, "counts");
}

#[test]
fn should_refuse_an_element_outside_the_item_bounds() {
    let contract = parse(lists_contract_json()).expect("should parse");
    let errors = validate(
        &contract,
        platform_value!({
            "reasons": [],
            "counts": [101u64],
        }),
    );
    assert_json_schema_refusal(errors, "counts");
}

// ---------------------------------------------------------------------------
// Random documents
// ---------------------------------------------------------------------------

/// A contract whose every typed array a random generator can satisfy: the
/// integer items carry no `minimum` / `maximum`, which the generator does
/// not read for any integer property.
fn random_lists_contract() -> DataContract {
    parse(contract_json(
        json!({
            "reasons": identifier_list_schema(0),
            "counts": {
                "type": "array",
                "minItems": 1,
                "maxItems": 4,
                "items": { "type": "integer" },
                "position": 1
            },
            "tags": {
                "type": "array",
                "minItems": 2,
                "maxItems": 5,
                "uniqueItems": true,
                "items": { "type": "string", "minLength": 3, "maxLength": 8 },
                "position": 2
            },
            "flags": {
                "type": "array",
                "maxItems": 2,
                "uniqueItems": true,
                "items": { "type": "boolean" },
                "position": 3
            },
        }),
        json!(["reasons", "counts", "tags", "flags"]),
        json!([]),
    ))
    .expect("should parse")
}

#[test]
fn should_generate_random_documents_that_validate_against_their_own_schema() {
    let contract = random_lists_contract();
    let document_type = contract
        .document_type_for_name(DOCUMENT_TYPE)
        .expect("the document type parses");
    let platform_version = PlatformVersion::latest();

    for seed in 0..16u64 {
        let document = document_type
            .random_document(Some(seed), platform_version)
            .expect("a random document should build");
        let result = contract
            .validate_document(DOCUMENT_TYPE, &document, platform_version)
            .expect("validation should run");
        assert!(
            result.is_valid(),
            "seed {seed}: {:?} refused with {:?}",
            document.properties(),
            result.errors
        );
    }

    for fill_size in [
        DocumentFieldFillSize::MinDocumentFillSize,
        DocumentFieldFillSize::MaxDocumentFillSize,
    ] {
        let mut rng = StdRng::seed_from_u64(42);
        let document = document_type
            .random_document_with_params(
                Identifier::random_with_rng(&mut rng),
                platform_value::Bytes32::random_with_rng(&mut rng),
                None,
                None,
                None,
                DocumentFieldFillType::FillIfNotRequired,
                fill_size,
                &mut rng,
                platform_version,
            )
            .expect("a random document should build");
        let result = contract
            .validate_document(DOCUMENT_TYPE, &document, platform_version)
            .expect("validation should run");
        assert!(
            result.is_valid(),
            "{fill_size:?}: {:?} refused with {:?}",
            document.properties(),
            result.errors
        );
    }
}

// ---------------------------------------------------------------------------
// Fee sizing
// ---------------------------------------------------------------------------

#[test]
fn should_size_a_typed_array_by_its_bounds() {
    let platform_version = PlatformVersion::latest();
    let identifiers = parse_property(identifier_list_schema(0)).expect("should parse");
    // count prefix (1 byte for 0 and for 64) plus 33 bytes an identifier
    assert_eq!(
        identifiers.min_byte_size(platform_version).unwrap(),
        Some(1)
    );
    assert_eq!(
        identifiers.max_byte_size(platform_version).unwrap(),
        Some(1 + 64 * 33)
    );

    let integers = parse_property(bounded_integer_list_schema(0)).expect("should parse");
    assert_eq!(
        integers.min_byte_size(platform_version).unwrap(),
        Some(1 + 8)
    );
    assert_eq!(
        integers.max_byte_size(platform_version).unwrap(),
        Some(1 + 4 * 8)
    );

    let unbounded_strings = parse_property(json!({
        "type": "array",
        "maxItems": 2,
        "items": { "type": "string" },
        "position": 0
    }))
    .expect("should parse");
    assert_eq!(
        unbounded_strings.max_byte_size(platform_version).unwrap(),
        Some(u16::MAX)
    );
}

// ---------------------------------------------------------------------------
// Contract serialization and the version gate
// ---------------------------------------------------------------------------

#[test]
fn should_round_trip_the_contract_through_platform_serialization() {
    let platform_version = PlatformVersion::latest();
    let contract = parse(lists_contract_json()).expect("should parse");
    let bytes = contract
        .serialize_to_bytes_with_platform_version(platform_version)
        .expect("should serialize");
    let restored = DataContract::versioned_deserialize_untrusted(&bytes, true, platform_version)
        .expect("should deserialize");
    assert_eq!(
        restored
            .document_type_for_name(DOCUMENT_TYPE)
            .expect("the document type parses")
            .properties(),
        contract
            .document_type_for_name(DOCUMENT_TYPE)
            .expect("the document type parses")
            .properties()
    );
    assert_eq!(restored, contract);
}

#[test]
fn should_refuse_a_typed_array_below_protocol_version_14_and_accept_it_at_14() {
    let json = lists_contract_json();
    let v13 = PlatformVersion::get(13).expect("protocol version 13 exists");

    let validating = DataContract::from_json(json.clone(), true, v13)
        .expect_err("meta-schema v2 admits only byte arrays");
    assert!(
        matches!(validating, ProtocolError::ConsensusError(_)),
        "expected a consensus refusal at 13, got {validating}"
    );
    let stored = DataContract::from_json(json.clone(), false, v13)
        .expect_err("the generation 2 parser admits only byte arrays")
        .to_string();
    assert!(
        stored.contains("only byte arrays"),
        "expected the historical refusal at 13, got {stored}"
    );

    DataContract::from_json(json, true, PlatformVersion::latest())
        .expect("protocol version 14 admits typed arrays");
}
