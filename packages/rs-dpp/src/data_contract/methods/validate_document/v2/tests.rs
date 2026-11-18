//! Generation 2 of document validation, at protocol version 15 against generation 1 at 14.

use crate::consensus::basic::json_schema_error::JsonSchemaError;
use crate::consensus::basic::BasicError;
use crate::consensus::codes::ErrorWithCode;
use crate::consensus::ConsensusError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::property_constraints::DocumentSystemValues;
use crate::data_contract::document_type::schema::validation_schema::DocumentSchemaValidator;
use crate::data_contract::document_type::validator::LoadedValidationSchema;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::methods::validate_document::{
    is_unreadable_byte_array_refusal, DataContractDocumentValidationMethodsV0,
};
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::DataContract;
use crate::tests::fixtures::get_data_contract_fixture;
use crate::ProtocolError;
use platform_value::{platform_value, Value};
use platform_version::version::PlatformVersion;

/// The document type `item` of a contract registered with `properties`.
fn item_contract(properties: Value) -> DataContract {
    let platform_version = PlatformVersion::latest();
    let mut contract =
        get_data_contract_fixture(None, 0, platform_version.protocol_version).data_contract_owned();
    contract
        .set_document_schema(
            "item",
            platform_value!({
                "type": "object",
                "properties": properties,
                "additionalProperties": false,
            }),
            true,
            &mut Vec::new(),
            platform_version,
        )
        .expect("expected the document type to register");
    contract
}

/// `contract` read anew from its bytes without validation, as a node reads a contract it
/// stores in format version 1: no validator built yet, as registration keeps the one it builds.
fn read_anew(contract: &DataContract) -> DataContract {
    use crate::serialization::{
        PlatformDeserializableWithPotentialValidationFromVersionedStructureTrusted,
        PlatformSerializableWithPlatformVersion,
    };
    let latest = PlatformVersion::latest();
    let bytes = contract
        .serialize_to_bytes_with_platform_version(latest)
        .expect("expected the contract to serialize");
    DataContract::versioned_deserialize_trusted(&bytes, false, latest)
        .expect("expected the contract to read back")
}

/// What document validation has built of `item`'s native validator with builder generation 0,
/// the one protocol version 15 selects: nothing yet (`None`),
/// the validator (`Some(true)`), or the refusal of a schema without a validation schema
/// (`Some(false)`).
fn native_validator_state(contract: &DataContract) -> Option<bool> {
    contract
        .document_type_for_name("item")
        .expect("expected the document type")
        .json_schema_validator_ref()
        .native(0)
        .and_then(|slot| slot.get())
        .map(Result::is_ok)
}

/// Registration keeps the native validator it builds of a type's schema, so document
/// validation and the type's stored form use it rather than build it again, and a clone of the
/// contract keeps it too.
#[test]
fn should_keep_the_native_validator_registration_builds() {
    let contract = item_contract(platform_value!({
        "name": { "type": "string", "maxLength": 4, "position": 0 },
    }));
    assert_eq!(native_validator_state(&contract), Some(true));
    assert_eq!(native_validator_state(&contract.clone()), Some(true));
    assert_eq!(native_validator_state(&read_anew(&contract)), None);
}

/// Document validation's verdict on `properties` as an `item`, at `platform_version`.
fn document_errors(
    contract: &DataContract,
    properties: Value,
    platform_version: &PlatformVersion,
) -> Vec<ConsensusError> {
    contract
        .validate_document_properties(
            "item",
            properties,
            &DocumentSystemValues::default(),
            platform_version,
        )
        .expect("expected document validation to run")
        .errors
}

#[test]
fn should_validate_documents_at_protocol_version_15_as_at_14() {
    let contract = item_contract(platform_value!({
        "name": { "type": "string", "maxLength": 4, "pattern": "^[a-z]+$", "position": 0 },
        "count": { "type": "integer", "minimum": 0, "maximum": 10, "position": 1 },
        "kind": { "type": "string", "enum": ["a", "b"], "position": 2 },
        "tags": {
            "type": "array",
            "items": { "type": "string", "maxLength": 3 },
            "maxItems": 3,
            "uniqueItems": true,
            "position": 3,
        },
        "data": {
            "type": "array",
            "byteArray": true,
            "maxItems": 4,
            "position": 4,
        },
        "when": { "type": "string", "format": "date", "maxLength": 10, "position": 5 },
    }));
    let latest = PlatformVersion::latest();
    let protocol_version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
    for properties in [
        platform_value!({ "name": "abc", "count": 3, "kind": "a" }),
        platform_value!({ "name": "abcde" }),
        platform_value!({ "name": "ABC" }),
        platform_value!({ "count": 11 }),
        platform_value!({ "count": 3.0 }),
        platform_value!({ "kind": "c" }),
        platform_value!({ "tags": ["a", "a"] }),
        platform_value!({ "tags": ["abcd"] }),
        // Written as bytes: a byte array of other integers is refused from 15 only
        platform_value!({ "data": [Value::U8(1), Value::U8(2)] }),
        platform_value!({ "data": [1, 300] }),
        platform_value!({ "when": "2026-10-22" }),
        platform_value!({ "when": "2026-13-01" }),
        platform_value!({ "other": 1 }),
    ] {
        // Read anew, the contract holds neither validator built
        let contract = read_anew(&contract);
        let existing = document_errors(&contract, properties.clone(), protocol_version_14);
        // Protocol version 14 never builds the native validator
        assert_eq!(native_validator_state(&contract), None, "{properties:?}");
        let native = document_errors(&contract, properties.clone(), latest);
        assert_eq!(
            native_validator_state(&contract),
            Some(true),
            "{properties:?}"
        );
        assert_eq!(native.is_empty(), existing.is_empty(), "{properties:?}");
        // Consensus depends on the code of the first error, not on its message
        assert_eq!(
            native.first().map(ErrorWithCode::code),
            existing.first().map(ErrorWithCode::code),
            "{properties:?}"
        );
    }
}

#[test]
fn should_refuse_the_documents_of_a_type_without_a_validation_schema() {
    let platform_version = PlatformVersion::latest();
    let mut contract = item_contract(platform_value!({
        "name": { "type": "string", "position": 0 },
    }));
    // `not` has no place in the validation schema; the document meta-schemas refuse it, so
    // the type is set without full validation
    contract
        .set_document_schema(
            "item",
            platform_value!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "not": { "const": "x" }, "position": 0 },
                },
                "additionalProperties": false,
            }),
            false,
            &mut Vec::new(),
            platform_version,
        )
        .expect("expected the document type to set");

    // Protocol version 14 checks the documents through the `jsonschema` crate
    let protocol_version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
    assert!(document_errors(
        &contract,
        platform_value!({ "name": "y" }),
        protocol_version_14
    )
    .is_empty());
    assert!(!document_errors(
        &contract,
        platform_value!({ "name": "x" }),
        protocol_version_14
    )
    .is_empty());
    assert_eq!(native_validator_state(&contract), None);

    // Protocol version 15 refuses every document of the type, valid under the schema or not
    for name in ["y", "x"] {
        let errors = document_errors(
            &contract,
            platform_value!({ "name": name }),
            platform_version,
        );
        assert!(
            matches!(
                errors.as_slice(),
                [ConsensusError::BasicError(BasicError::ContractError(
                    DataContractError::InvalidContractStructure(_)
                ))]
            ),
            "{name}: {errors:?}"
        );
    }
    // The refusal is kept with the document type, as a validator would be
    assert_eq!(native_validator_state(&contract), Some(false));
    assert!(contract
        .document_type_validation_schema("item", platform_version)
        .is_err());
}

#[test]
fn should_refuse_the_documents_of_a_type_whose_references_cycle() {
    let platform_version = PlatformVersion::latest();
    let mut contract = item_contract(platform_value!({
        "name": { "type": "string", "position": 0 },
    }));
    // The definitions and the type are set without full validation
    let mut defs = contract.schema_defs().cloned().unwrap_or_default();
    defs.insert("a".to_string(), platform_value!({ "$ref": "#/$defs/b" }));
    defs.insert("b".to_string(), platform_value!({ "$ref": "#/$defs/a" }));
    contract
        .set_schema_defs(Some(defs), false, &mut Vec::new(), platform_version)
        .expect("expected the definitions to set");
    contract
        .set_document_schema(
            "item",
            platform_value!({
                "type": "object",
                "properties": {
                    "tags": {
                        "type": "array",
                        "position": 0,
                        "maxItems": 4,
                        "items": { "type": "integer" },
                        "contains": { "$ref": "#/$defs/a" },
                    },
                },
                "additionalProperties": false,
            }),
            false,
            &mut Vec::new(),
            platform_version,
        )
        .expect("expected the document type to set");

    let errors = document_errors(
        &contract,
        platform_value!({ "tags": [1] }),
        platform_version,
    );
    assert!(
        matches!(
            errors.as_slice(),
            [ConsensusError::BasicError(BasicError::ContractError(
                DataContractError::InvalidContractStructure(_)
            ))]
        ),
        "{errors:?}"
    );
    assert_eq!(native_validator_state(&contract), Some(false));
}

/// At protocol version 15, `uniqueItems` holds `0.0` and `-0.0` as one value however long the
/// list: a 16-element list holding both is refused, while 16 distinct numbers, the control,
/// are accepted.
#[test]
fn should_hold_zero_and_negative_zero_as_one_unique_item_in_a_long_list() {
    let platform_version = PlatformVersion::latest();
    let contract = item_contract(platform_value!({
        "scores": {
            "type": "array",
            "items": { "type": "number" },
            "maxItems": 20,
            "uniqueItems": true,
            "position": 0,
        },
    }));
    let distinct: Vec<Value> = (0..16)
        .map(|number| Value::Float(f64::from(number)))
        .collect();
    let mut signed_zeros = distinct.clone();
    signed_zeros[15] = Value::Float(-0.0);

    let errors = document_errors(
        &contract,
        platform_value!({ "scores": Value::Array(distinct) }),
        platform_version,
    );
    assert!(errors.is_empty(), "{errors:?}");
    let errors = document_errors(
        &contract,
        platform_value!({ "scores": Value::Array(signed_zeros) }),
        platform_version,
    );
    assert!(
        matches!(
            errors.as_slice(),
            [ConsensusError::BasicError(BasicError::JsonSchemaError(error))]
                if error.keyword() == "uniqueItems"
        ),
        "{errors:?}"
    );
    assert_eq!(native_validator_state(&contract), Some(true));
}

/// A document type read back from its stored form is checked against the typed validation
/// schema stored with it, by the generation that built it, with the verdicts of the same type
/// parsed from its schema.
#[test]
fn should_check_the_documents_of_a_type_read_back_from_its_stored_form_against_its_stored_schema() {
    use crate::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructureTrusted;

    let latest = PlatformVersion::latest();
    let contract = item_contract(platform_value!({
        "name": { "type": "string", "maxLength": 4, "pattern": "^[a-z]+$", "position": 0 },
        "count": { "type": "integer", "minimum": 0, "maximum": 10, "position": 1 },
        "tags": {
            "type": "array",
            "items": { "type": "string", "maxLength": 3 },
            "maxItems": 3,
            "uniqueItems": true,
            "position": 2,
        },
    }));
    let bytes = contract
        .serialize_to_storage_bytes(latest)
        .expect("expected the contract to compile");
    let read_back = DataContract::versioned_deserialize_trusted(&bytes, false, latest)
        .expect("expected the contract to read back");

    let document_type = read_back
        .document_type_for_name("item")
        .expect("expected the document type");
    let Some(LoadedValidationSchema::Built { generation, schema }) = document_type
        .json_schema_validator_ref()
        .loaded_validation_schema()
    else {
        panic!("expected the stored validation schema");
    };
    assert_eq!(*generation, 0);
    assert_eq!(
        schema
            .decode()
            .expect("expected the stored schema to decode"),
        contract
            .document_type_validation_schema("item", latest)
            .expect("expected the validation schema")
    );
    // Read back, the schema stays encoded until a document is checked
    assert_eq!(native_validator_state(&read_back), None);

    for properties in [
        platform_value!({ "name": "abc", "count": 3 }),
        platform_value!({ "name": "abcde" }),
        platform_value!({ "name": "ABC" }),
        platform_value!({ "count": 11 }),
        platform_value!({ "tags": ["a", "a"] }),
        platform_value!({ "other": 1 }),
    ] {
        assert_eq!(
            document_errors(&read_back, properties.clone(), latest),
            document_errors(&contract, properties.clone(), latest),
            "{properties:?}"
        );
    }
    assert_eq!(native_validator_state(&read_back), Some(true));
}

/// A document type stored without a validation schema, as the version that stored it built
/// none, keeps its documents refused where the builder of the version reading it would accept
/// its schema, and is stored again without one.
#[test]
fn should_keep_refusing_the_documents_of_a_type_stored_without_a_validation_schema() {
    use crate::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructureTrusted;

    let latest = PlatformVersion::latest();
    let contract = item_contract(platform_value!({
        "name": { "type": "string", "maxLength": 4, "position": 0 },
    }));
    let mut storing_without_builder = latest.clone();
    storing_without_builder
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .validation_schema = None;
    let bytes = contract
        .serialize_to_storage_bytes(&storing_without_builder)
        .expect("expected the contract to compile");
    let read_back = DataContract::versioned_deserialize_trusted(&bytes, false, latest)
        .expect("expected the contract to read back");

    let document = platform_value!({ "name": "abc" });
    let errors = document_errors(&contract, document.clone(), latest);
    assert!(errors.is_empty(), "{errors:?}");
    let refused = |contract: &DataContract| {
        let errors = document_errors(contract, document.clone(), latest);
        assert!(
            matches!(
                errors.as_slice(),
                [ConsensusError::BasicError(BasicError::ContractError(
                    DataContractError::InvalidContractStructure(message)
                ))] if message.contains("selected no validation schema builder")
            ),
            "{errors:?}"
        );
        assert!(contract
            .document_type_validation_schema("item", latest)
            .is_err());
    };
    refused(&read_back);

    let stored_again = read_back
        .serialize_to_storage_bytes(latest)
        .expect("expected the contract to compile again");
    refused(
        &DataContract::versioned_deserialize_trusted(&stored_again, false, latest)
            .expect("expected the contract to read back again"),
    );
}

/// An `order` contract whose `decimals` are generated from the token `tokenId` names
/// (`sys.tokens.decimals`, protocol version 15), both required.
fn order_contract() -> DataContract {
    let platform_version = PlatformVersion::latest();
    let mut contract =
        get_data_contract_fixture(None, 0, platform_version.protocol_version).data_contract_owned();
    contract
        .set_document_schema(
            "order",
            platform_value!({
                "type": "object",
                "properties": {
                    "tokenId": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "position": 0
                    },
                    "decimals": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 255,
                        "generatedFrom": { "function": "sys.tokens.decimals", "params": ["tokenId"] },
                        "position": 1
                    }
                },
                "required": ["tokenId", "decimals"],
                "additionalProperties": false,
            }),
            true,
            &mut Vec::new(),
            platform_version,
        )
        .expect("expected the document type to register");
    contract
}

/// A client leaves out decimals generated from state, which the platform writes before it
/// checks the document: the client's full check passes such a document although its type
/// requires them, and still checks decimals it holds, and refuses a document without its
/// token. Consensus, which checks the document once the value is written, refuses it without.
#[test]
fn should_pass_a_client_document_leaving_out_required_state_generated_decimals() {
    use crate::document::{Document, DocumentV0};
    let contract = order_contract();
    let latest = PlatformVersion::latest();
    let token = Value::Identifier([5; 32]);
    let errors = |properties: Value, system: &DocumentSystemValues| {
        contract
            .validate_document_properties("order", properties, system, latest)
            .expect("expected document validation to run")
            .errors
    };
    let client = DocumentSystemValues::default();
    let missing = |errors: &[ConsensusError], property: &str| {
        matches!(
            errors,
            [ConsensusError::BasicError(BasicError::JsonSchemaError(error))]
                if error.keyword() == "required" && error.property_name() == property
        )
    };

    let left_out = errors(platform_value!({ "tokenId": token.clone() }), &client);
    assert!(left_out.is_empty(), "{left_out:?}");
    let held = errors(
        platform_value!({ "tokenId": token.clone(), "decimals": 6 }),
        &client,
    );
    assert!(held.is_empty(), "{held:?}");
    let out_of_bounds = errors(
        platform_value!({ "tokenId": token.clone(), "decimals": 300 }),
        &client,
    );
    assert!(
        matches!(
            out_of_bounds.as_slice(),
            [ConsensusError::BasicError(BasicError::JsonSchemaError(error))]
                if error.keyword() == "maximum"
        ),
        "{out_of_bounds:?}"
    );
    let without_token = errors(platform_value!({}), &client);
    assert!(missing(&without_token, "tokenId"), "{without_token:?}");

    // The whole-document check a client calls reads the document the same way
    let document = Document::V0(DocumentV0 {
        properties: [("tokenId".to_string(), token.clone())].into(),
        ..Default::default()
    });
    let whole = contract
        .validate_document("order", &document, latest)
        .expect("expected document validation to run");
    assert!(whole.is_valid(), "{:?}", whole.errors);

    let consensus = DocumentSystemValues {
        consensus: true,
        ..DocumentSystemValues::default()
    };
    let in_consensus = errors(platform_value!({ "tokenId": token }), &consensus);
    assert!(missing(&in_consensus, "decimals"), "{in_consensus:?}");
}

/// Validation errors for a document `{ "r": value }` against a type whose `r` is a
/// `format: regex` string, at `platform_version`. The contract is registered then read anew,
/// so the protocol version decides which validator checks the value.
fn format_regex_errors(value: &str, platform_version: &PlatformVersion) -> Vec<ConsensusError> {
    let contract = item_contract(platform_value!({
        "r": { "type": "string", "format": "regex", "maxLength": 20, "position": 0 },
    }));
    document_errors(
        &read_anew(&contract),
        platform_value!({ "r": value }),
        platform_version,
    )
}

#[test]
fn should_accept_a_format_regex_value_that_parses_but_does_not_compile_at_protocol_version_15() {
    let latest = PlatformVersion::latest();
    let protocol_version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
    // A value that parses as a regular expression but that the engine does not compile.
    let value = r"\pN{3000}";
    // Protocol version 15 checks the syntax only, so it accepts the value.
    assert!(format_regex_errors(value, latest).is_empty());
    // Protocol version 14 compiles the value, so it refuses it.
    assert!(!format_regex_errors(value, protocol_version_14).is_empty());
}

#[test]
fn should_accept_a_format_regex_value_with_an_unknown_unicode_property_at_protocol_version_15() {
    let latest = PlatformVersion::latest();
    let protocol_version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
    // A value that parses as a regular expression but names a Unicode property the engine
    // holds no character class for.
    let value = r"\p{Foobar}";
    // Protocol version 15 reads the syntax alone, so it accepts the value.
    assert!(format_regex_errors(value, latest).is_empty());
    // Protocol version 14 translates the parsed value, so it refuses the unknown property.
    assert!(!format_regex_errors(value, protocol_version_14).is_empty());
}

#[test]
fn should_refuse_a_format_regex_value_with_a_syntax_error_at_both_protocol_versions() {
    let latest = PlatformVersion::latest();
    let protocol_version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
    // An unbalanced group is not valid regular expression syntax.
    let value = "(";
    assert!(!format_regex_errors(value, latest).is_empty());
    assert!(!format_regex_errors(value, protocol_version_14).is_empty());
}

#[test]
fn should_accept_an_ordinary_format_regex_value_at_both_protocol_versions() {
    let latest = PlatformVersion::latest();
    let protocol_version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
    let value = "^[a-z]+$";
    assert!(format_regex_errors(value, latest).is_empty());
    assert!(format_regex_errors(value, protocol_version_14).is_empty());
}

/// A pattern that parses but that the `regex` crate does not compile under its default size
/// limit.
const PATTERN_BEYOND_THE_SIZE_LIMIT: &str = "^\\p{L}{400}$";

/// How many of `item`'s patterns its native validator has tried to compile: `None` when
/// document validation has built no validator for it, or has refused its schema.
fn attempted_patterns(contract: &DataContract) -> Option<usize> {
    contract
        .document_type_for_name("item")
        .expect("expected the document type")
        .json_schema_validator_ref()
        .native(0)
        .and_then(|slot| slot.get())
        .and_then(|built| built.as_ref().ok())
        .map(DocumentSchemaValidator::attempted_patterns)
}

/// `contract` stored in its compiled form and read back, as a node reads a contract it stores
/// compiled.
fn stored_and_read_back(contract: &DataContract) -> DataContract {
    use crate::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructureTrusted;

    let latest = PlatformVersion::latest();
    let bytes = contract
        .serialize_to_storage_bytes(latest)
        .expect("expected the contract to compile");
    DataContract::versioned_deserialize_trusted(&bytes, false, latest)
        .expect("expected the contract to read back")
}

/// Measuring a contract's compiled size builds its types' native validators, as storing it
/// does, and compiles none of their patterns: a validator checks each pattern's syntax when it
/// is built and compiles it when a document first reaches it. A pattern that parses but does
/// not compile leaves the type stored with its validation schema.
#[test]
fn should_measure_the_compiled_size_without_compiling_a_pattern() {
    let latest = PlatformVersion::latest();
    let mut contract =
        get_data_contract_fixture(None, 0, latest.protocol_version).data_contract_owned();
    // Parsed without full validation, as a contract is before a block registers it
    contract
        .set_document_schema(
            "item",
            platform_value!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "pattern": "^[a-z]+$", "position": 0 },
                    "code": {
                        "type": "string",
                        "pattern": PATTERN_BEYOND_THE_SIZE_LIMIT,
                        "position": 1,
                    },
                },
                "additionalProperties": false,
            }),
            false,
            &mut Vec::new(),
            latest,
        )
        .expect("expected the document type to parse");

    let mut operations = Vec::new();
    assert!(contract
        .validate_compiled_size(&mut operations, latest)
        .expect("expected the compiled size to be measured")
        .is_valid());
    assert_eq!(attempted_patterns(&contract), Some(0));

    let read_back = stored_and_read_back(&contract);
    assert!(read_back
        .document_type_validation_schema("item", latest)
        .is_ok());
}

/// A type's documents are checked alike whether its native validator was kept from
/// registration, which compiled every pattern, or built from the validation schema the type
/// was stored with, or from a schema read without validation; the last two compile a pattern
/// when a document first reaches it. On every path a pattern is compiled with its ECMA
/// escapes rewritten: `\d` is an ASCII digit and `\cA` the control character U+0001.
#[test]
fn should_check_documents_alike_however_the_native_validator_was_built() {
    let latest = PlatformVersion::latest();
    let registered = item_contract(platform_value!({
        "digit": { "type": "string", "maxLength": 1, "pattern": "^\\d$", "position": 0 },
        "control": { "type": "string", "maxLength": 1, "pattern": "^\\cA$", "position": 1 },
        "codes": {
            "type": "array",
            "items": { "type": "string", "maxLength": 3 },
            "maxItems": 2,
            "contains": { "type": "string", "pattern": "^c" },
            "position": 2,
        },
    }));
    assert_eq!(attempted_patterns(&registered), Some(3));
    let read_back = stored_and_read_back(&registered);
    let parsed = read_anew(&registered);

    for contract in [&read_back, &parsed] {
        assert!(document_errors(contract, platform_value!({ "digit": "3" }), latest).is_empty());
        assert_eq!(attempted_patterns(contract), Some(1));
    }

    for (properties, accepted) in [
        (platform_value!({ "digit": "3" }), true),
        (platform_value!({ "digit": "\u{663}" }), false),
        (platform_value!({ "control": "\u{1}" }), true),
        (platform_value!({ "control": "A" }), false),
        (platform_value!({ "codes": ["cat"] }), true),
        (platform_value!({ "codes": ["dog"] }), false),
        (platform_value!({}), true),
    ] {
        let expected = document_errors(&registered, properties.clone(), latest);
        assert_eq!(expected.is_empty(), accepted, "{properties:?}");
        assert_eq!(
            document_errors(&read_back, properties.clone(), latest),
            expected,
            "{properties:?}"
        );
        assert_eq!(
            document_errors(&parsed, properties.clone(), latest),
            expected,
            "{properties:?}"
        );
    }
}

/// The schema of an `item` with a string `title` whose `const` is an object holding
/// `pattern`, and a `code` whose `$ref` reads that object as its schema. Through protocol
/// version 14 no registration check reads a `const` value as a schema, so its pattern is
/// compiled only when a document reaches `code`.
fn pattern_in_a_const(pattern: &str) -> Value {
    platform_value!({
        "title": {
            "type": "string",
            "maxLength": 63,
            "position": 0,
            "const": { "type": "string", "maxLength": 63, "position": 1, "pattern": pattern },
        },
        "code": { "$ref": "#/properties/title/const", "position": 1 },
    })
}

/// The schema of an `item` whose array `codes` has a `contains` reading, through `$ref`, an
/// object held in `title`'s `const` that holds `pattern`.
fn pattern_in_a_const_contains(pattern: &str) -> Value {
    platform_value!({
        "title": {
            "type": "string",
            "maxLength": 63,
            "position": 0,
            "const": { "type": "string", "pattern": pattern },
        },
        "codes": {
            "type": "array",
            "items": { "type": "string", "maxLength": 63 },
            "maxItems": 3,
            "contains": { "$ref": "#/properties/title/const" },
            "position": 1,
        },
    })
}

/// An `item` contract registered at `platform_version` with `properties`, or the error
/// registration refuses it with.
fn item_contract_registered_at(
    properties: Value,
    platform_version: &PlatformVersion,
) -> Result<DataContract, ProtocolError> {
    let mut contract =
        get_data_contract_fixture(None, 0, platform_version.protocol_version).data_contract_owned();
    contract.set_document_schema(
        "item",
        platform_value!({
            "type": "object",
            "properties": properties,
            "additionalProperties": false,
        }),
        true,
        &mut Vec::new(),
        platform_version,
    )?;
    Ok(contract)
}

/// `contract` written in the format transitions carry at `written_at` and read back without
/// validation at `read_at`, as a node reads a contract stored in that format.
fn read_at(
    contract: &DataContract,
    written_at: &PlatformVersion,
    read_at: &PlatformVersion,
) -> DataContract {
    use crate::serialization::{
        PlatformDeserializableWithPotentialValidationFromVersionedStructureTrusted,
        PlatformSerializableWithPlatformVersion,
    };
    let bytes = contract
        .serialize_to_bytes_with_platform_version(written_at)
        .expect("expected the contract to serialize");
    DataContract::versioned_deserialize_trusted(&bytes, false, read_at)
        .expect("expected the contract to read back")
}

/// The keyword of the JSON Schema violation document validation refuses `properties` with,
/// `None` when it accepts them.
fn violated_keyword(
    contract: &DataContract,
    properties: Value,
    platform_version: &PlatformVersion,
) -> Option<String> {
    match document_errors(contract, properties, platform_version).first() {
        None => None,
        Some(ConsensusError::BasicError(BasicError::JsonSchemaError(violation))) => {
            Some(violation.keyword().to_string())
        }
        Some(other) => panic!("unexpected consensus error: {other:?}"),
    }
}

/// Registration at protocol version 15 compiles every pattern of a type, a pattern only a
/// `$ref` reaches included, and refuses one that does not compile.
#[test]
fn should_refuse_at_registration_a_pattern_that_does_not_compile() {
    let error = item_contract_registered_at(
        pattern_in_a_const(PATTERN_BEYOND_THE_SIZE_LIMIT),
        PlatformVersion::latest(),
    )
    .expect_err("expected registration to refuse the pattern");
    let message = error.to_string();
    assert!(message.contains("compiles no `pattern`"), "{message}");
    assert!(
        message.contains("reached through `$ref` `#/properties/title/const`"),
        "{message}"
    );
}

/// A type registered through protocol version 14 can hold a pattern that parses but does not
/// compile where no registration check read it, in a `const` object a `$ref` reaches. The JSON
/// Schema validator compiled it only when a document reached it, and so does the native
/// validator: a document that never reaches the pattern is accepted, and one that does is
/// refused as protocol version 14 refuses it, read from the format transitions carry or from
/// the compiled form a node stores.
#[test]
fn should_refuse_only_where_a_document_reaches_a_pattern_no_registration_compiled() {
    let latest = PlatformVersion::latest();
    let protocol_version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
    for (properties, documents) in [
        (
            pattern_in_a_const(PATTERN_BEYOND_THE_SIZE_LIMIT),
            vec![
                (platform_value!({}), None),
                (platform_value!({ "code": "abc" }), Some("format")),
            ],
        ),
        (
            pattern_in_a_const_contains(PATTERN_BEYOND_THE_SIZE_LIMIT),
            vec![
                (platform_value!({}), None),
                (platform_value!({ "codes": ["abc"] }), Some("contains")),
            ],
        ),
    ] {
        let registered = item_contract_registered_at(properties, protocol_version_14)
            .expect("expected protocol version 14 to register the type");
        let at_14 = read_at(&registered, protocol_version_14, protocol_version_14);
        let at_15 = read_at(&registered, protocol_version_14, latest);
        let stored = stored_and_read_back(&at_15);
        assert!(stored
            .document_type_validation_schema("item", latest)
            .is_ok());
        for (document, keyword) in documents {
            let keyword = keyword.map(str::to_string);
            assert_eq!(
                violated_keyword(&at_14, document.clone(), protocol_version_14),
                keyword,
                "{document:?} at protocol version 14"
            );
            assert_eq!(
                violated_keyword(&at_15, document.clone(), latest),
                keyword,
                "{document:?}"
            );
            assert_eq!(
                violated_keyword(&stored, document.clone(), latest),
                keyword,
                "{document:?} read back from the compiled form"
            );
        }
    }
}

/// A `pattern` applies to strings alone, so a value that is not a string never reaches one.
/// Through protocol version 14 the JSON Schema validator compiled the schema a `$ref` reaches
/// before checking any value against it, and so refused every value reaching a schema holding
/// a pattern that does not compile. From protocol version 15 a value that is not a string is
/// checked against the rest of that schema alone: an integer element meets a `contains`
/// whose schema holds nothing but such a pattern, where protocol version 14 found no match.
#[test]
fn should_match_a_contains_element_no_pattern_applies_to_where_protocol_version_14_did_not() {
    let latest = PlatformVersion::latest();
    let protocol_version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
    let registered = item_contract_registered_at(
        platform_value!({
            "title": {
                "type": "string",
                "maxLength": 63,
                "position": 0,
                "const": { "pattern": PATTERN_BEYOND_THE_SIZE_LIMIT },
            },
            "numbers": {
                "type": "array",
                "items": { "type": "integer" },
                "maxItems": 3,
                "contains": { "$ref": "#/properties/title/const" },
                "position": 1,
            },
        }),
        protocol_version_14,
    )
    .expect("expected protocol version 14 to register the type");
    let at_14 = read_at(&registered, protocol_version_14, protocol_version_14);
    let at_15 = read_at(&registered, protocol_version_14, latest);
    let stored = stored_and_read_back(&at_15);

    let document = platform_value!({ "numbers": [1] });
    assert_eq!(
        violated_keyword(&at_14, document.clone(), protocol_version_14).as_deref(),
        Some("contains")
    );
    assert_eq!(violated_keyword(&at_15, document.clone(), latest), None);
    assert_eq!(violated_keyword(&stored, document, latest), None);
    assert_eq!(attempted_patterns(&at_15), Some(0));
}

/// A pattern that does not parse, in the same place, still leaves the type without a
/// validation schema at protocol version 15: the builder reads every pattern's syntax, so
/// every document of the type is refused, where protocol version 14 accepted one that never
/// reached the pattern.
#[test]
fn should_refuse_every_document_of_a_type_holding_a_pattern_that_does_not_parse() {
    let latest = PlatformVersion::latest();
    let protocol_version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
    let registered = item_contract_registered_at(pattern_in_a_const("(?=a)"), protocol_version_14)
        .expect("expected protocol version 14 to register the type");
    let at_14 = read_at(&registered, protocol_version_14, protocol_version_14);
    assert!(document_errors(&at_14, platform_value!({}), protocol_version_14).is_empty());

    let at_15 = read_at(&registered, protocol_version_14, latest);
    let errors = document_errors(&at_15, platform_value!({}), latest);
    assert!(
        matches!(
            errors.as_slice(),
            [ConsensusError::BasicError(BasicError::ContractError(
                DataContractError::InvalidContractStructure(message)
            ))] if message.contains("compiles no `pattern`")
        ),
        "{errors:?}"
    );
}

/// The document type `item` with a typed array `list` whose elements take one of `members`
/// integers, and a document whose `list` holds `elements` integers none of them takes.
fn enum_list_contract_and_document(members: u64, elements: u64) -> (DataContract, Value) {
    let allowed: Vec<Value> = (0..members).map(Value::U64).collect();
    let contract = item_contract(platform_value!({
        "list": {
            "type": "array",
            "items": { "type": "integer", "enum": allowed },
            "maxItems": elements,
            "position": 0,
        },
    }));
    let list: Vec<Value> = (0..elements)
        .map(|element| Value::U64(members + element))
        .collect();
    (contract, platform_value!({ "list": list }))
}

/// The JSON Schema error `error` is.
fn json_schema_error(error: &ConsensusError) -> &JsonSchemaError {
    match error {
        ConsensusError::BasicError(BasicError::JsonSchemaError(error)) => error,
        other => panic!("expected a JSON Schema error, got {other:?}"),
    }
}

#[test]
fn should_report_one_small_enum_violation_for_a_document_breaking_a_long_enum_many_times() {
    use crate::serialization::{
        PlatformDeserializableUntrusted, PlatformSerializableWithPlatformVersion,
    };
    let latest = PlatformVersion::latest();
    let (contract, document) = enum_list_contract_and_document(300, 300);
    let errors = document_errors(&contract, document, latest);
    // The first violation only, the one consensus reports
    assert_eq!(errors.len(), 1);
    let error = json_schema_error(&errors[0]);
    assert_eq!(error.keyword(), "enum");
    assert_eq!(error.instance_path(), "/list/0");
    let listed = error
        .params()
        .get_optional_array_slice("enum")
        .expect("expected params")
        .expect("expected a list")
        .len();
    assert_eq!(
        Some(listed),
        latest
            .system_limits
            .max_enum_violation_listed_values
            .map(usize::from)
    );
    // A client can read the error back
    let bytes = errors[0]
        .serialize_to_bytes_with_platform_version(latest)
        .expect("expected the error to serialize");
    assert_eq!(
        ConsensusError::deserialize_from_bytes_untrusted(&bytes).expect("expected it to read back"),
        errors[0]
    );
}

#[test]
fn should_report_an_enum_violation_a_client_can_read_whatever_the_allowed_values_hold() {
    use crate::serialization::{
        PlatformDeserializableUntrusted, PlatformSerializableWithPlatformVersion,
    };
    let latest = PlatformVersion::latest();
    let long = "x".repeat(1500);
    // Strings whose JSON fills the listing's bytes, the most a listing holds
    let filling: Vec<Value> = (0..16)
        .map(|member| Value::Text(format!("{member:014}")))
        .collect();
    let object = |last: u64| platform_value!({ "a": long.clone(), "b": [1, 2, last] });
    for allowed in [
        filling,
        vec![Value::Text(long.clone()), Value::Text("b".to_string())],
        vec![Value::Text("a".to_string()), Value::Text(long.clone())],
        vec![object(3), object(4)],
    ] {
        let contract = item_contract(platform_value!({
            "label": { "type": "string", "enum": allowed, "position": 0 },
        }));
        let errors = document_errors(
            &contract,
            platform_value!({ "label": "not an allowed value" }),
            latest,
        );
        assert_eq!(errors.len(), 1);
        assert_eq!(json_schema_error(&errors[0]).keyword(), "enum");
        let bytes = errors[0]
            .serialize_to_bytes_with_platform_version(latest)
            .expect("expected the error to serialize");
        assert_eq!(
            ConsensusError::deserialize_from_bytes_untrusted(&bytes)
                .expect("expected it to read back"),
            errors[0]
        );
    }
}

#[test]
fn should_report_an_enum_violation_at_protocol_version_14_as_before() {
    let protocol_version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
    let (contract, document) = enum_list_contract_and_document(300, 2);
    let errors = document_errors(&contract, document.clone(), protocol_version_14);
    let first = json_schema_error(errors.first().expect("expected an error"));
    assert_eq!(first.keyword(), "enum");
    // Every allowed value listed, as the JSON Schema validator lists them
    assert_eq!(
        first
            .params()
            .get_optional_array_slice("enum")
            .expect("expected params")
            .map(<[Value]>::len),
        Some(300)
    );
    // with the code protocol version 15 reports
    let latest = document_errors(&contract, document, PlatformVersion::latest());
    assert_eq!(
        latest.first().map(ErrorWithCode::code),
        errors.first().map(ErrorWithCode::code)
    );
}

/// A type with a plain byte array `data`, an identifier `owner`, an object `nested` holding a
/// byte array, and a list of identifiers `members`.
fn byte_array_contract() -> DataContract {
    item_contract(platform_value!({
        "data": { "type": "array", "byteArray": true, "maxItems": 64, "position": 0 },
        "owner": {
            "type": "array",
            "byteArray": true,
            "minItems": 32,
            "maxItems": 32,
            "contentMediaType": "application/x.dash.dpp.identifier",
            "position": 1,
        },
        "nested": {
            "type": "object",
            "properties": {
                "blob": { "type": "array", "byteArray": true, "maxItems": 8, "position": 0 },
            },
            "additionalProperties": false,
            "position": 2,
        },
        "members": {
            "type": "array",
            "items": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
            },
            "maxItems": 4,
            "position": 3,
        },
    }))
}

/// Whether the document serializer writes a document holding `properties` as an `item`.
fn serializes(contract: &DataContract, properties: &Value) -> bool {
    use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use crate::document::{Document, DocumentV0};
    let document = Document::V0(DocumentV0 {
        properties: properties
            .clone()
            .into_btree_string_map()
            .expect("expected a map of properties"),
        ..Default::default()
    });
    let document_type = contract
        .document_type_for_name("item")
        .expect("expected the document type");
    document
        .serialize(document_type, contract, PlatformVersion::latest())
        .is_ok()
}

/// The instance path of a single `byteArray` refusal, or a description of what was reported.
fn byte_array_refusal(errors: &[ConsensusError]) -> Result<String, String> {
    match errors {
        [refusal @ ConsensusError::BasicError(BasicError::JsonSchemaError(error))]
            if error.keyword() == "byteArray" && is_unreadable_byte_array_refusal(refusal) =>
        {
            Ok(error.instance_path().to_string())
        }
        other => Err(format!("{other:?}")),
    }
}

fn bytes_of(member: fn(u8) -> Value, count: usize) -> Value {
    Value::Array((0..count).map(|byte| member(byte as u8)).collect())
}

#[test]
fn should_refuse_a_byte_array_holding_integers_that_are_not_bytes_at_protocol_version_15() {
    let contract = byte_array_contract();
    let latest = PlatformVersion::latest();
    let identifier = Value::Identifier([7; 32]);
    let cases = [
        (
            platform_value!({ "data": [Value::U64(1), Value::U64(2)] }),
            "/data/0",
        ),
        (
            platform_value!({ "data": [Value::U8(1), Value::I8(2)] }),
            "/data/1",
        ),
        (
            platform_value!({ "data": [Value::U8(1), Value::U16(255)] }),
            "/data/1",
        ),
        (
            platform_value!({ "owner": bytes_of(|byte| Value::U64(byte as u64), 32) }),
            "/owner/0",
        ),
        (
            platform_value!({ "nested": { "blob": [Value::U8(0), Value::I64(5)] } }),
            "/nested/blob/1",
        ),
        (
            platform_value!({
                "members": [identifier, bytes_of(|byte| Value::U32(byte as u32), 32)],
            }),
            "/members/1/0",
        ),
    ];
    for (properties, path) in cases {
        let errors = document_errors(&contract, properties.clone(), latest);
        assert_eq!(
            byte_array_refusal(&errors),
            Ok(path.to_string()),
            "{properties:?}"
        );
        // Refused for what the serializer refuses
        assert!(!serializes(&contract, &properties), "{properties:?}");
    }
}

#[test]
fn should_accept_byte_arrays_written_as_bytes_at_protocol_version_15() {
    let contract = byte_array_contract();
    let latest = PlatformVersion::latest();
    let cases = [
        platform_value!({ "data": [Value::U8(0), Value::U8(255)] }),
        platform_value!({ "data": Value::Bytes(vec![0, 1, 255]) }),
        platform_value!({ "data": Value::Array(vec![]) }),
        platform_value!({ "data": Value::Bytes32([9; 32]) }),
        platform_value!({ "owner": Value::Identifier([3; 32]) }),
        platform_value!({ "owner": Value::Bytes32([3; 32]) }),
        platform_value!({ "owner": bytes_of(Value::U8, 32) }),
        platform_value!({ "owner": Value::Bytes(vec![4; 32]) }),
        platform_value!({ "nested": { "blob": Value::Bytes(vec![1, 2]) } }),
        platform_value!({ "members": [Value::Identifier([1; 32]), bytes_of(Value::U8, 32)] }),
    ];
    for properties in cases {
        let errors = document_errors(&contract, properties.clone(), latest);
        assert!(errors.is_empty(), "{properties:?}: {errors:?}");
        assert!(serializes(&contract, &properties), "{properties:?}");
    }
}

/// A member above 255, negative or not an integer is refused by the schema check, at both
/// protocol versions, before the byte reading is reached.
#[test]
fn should_refuse_members_out_of_byte_range_by_the_schema_check_at_both_protocol_versions() {
    let contract = byte_array_contract();
    let protocol_version_14 =
        PlatformVersion::get(14).expect("expected protocol version 14 to exist");
    let cases = [
        (Value::U64(300), "maximum"),
        (Value::I64(-1), "type"),
        (Value::Float(1.5), "type"),
        (Value::Float(1.0), "type"),
        (Value::Text("a".to_string()), "type"),
    ];
    for (member, keyword) in cases {
        let properties = platform_value!({ "data": [Value::U8(1), member] });
        for platform_version in [PlatformVersion::latest(), protocol_version_14] {
            let errors = document_errors(&contract, properties.clone(), platform_version);
            assert!(
                matches!(
                    errors.as_slice(),
                    [ConsensusError::BasicError(BasicError::JsonSchemaError(error))]
                        if error.keyword() == keyword && error.instance_path() == "/data/1"
                ),
                "{properties:?} at {}: {errors:?}",
                platform_version.protocol_version
            );
            // A schema violation, not the refusal a batch is refused unpaid for
            assert!(!errors.iter().any(is_unreadable_byte_array_refusal));
        }
        assert!(!serializes(&contract, &properties), "{properties:?}");
    }
}

/// A transient property is dropped before a document is written, so the serializer never
/// reads its value and the byte reading leaves it to the schema check.
#[test]
fn should_not_read_the_bytes_of_a_transient_byte_array() {
    let latest = PlatformVersion::latest();
    let mut contract =
        get_data_contract_fixture(None, 0, latest.protocol_version).data_contract_owned();
    contract
        .set_document_schema(
            "item",
            platform_value!({
                "type": "object",
                "properties": {
                    "data": { "type": "array", "byteArray": true, "maxItems": 64, "position": 0 },
                    "pad": { "type": "array", "byteArray": true, "maxItems": 64, "position": 1 },
                },
                "transient": ["pad"],
                "additionalProperties": false,
            }),
            true,
            &mut Vec::new(),
            latest,
        )
        .expect("expected the document type to register");

    let errors = document_errors(
        &contract,
        platform_value!({ "data": [Value::U8(1)], "pad": [Value::U64(1), Value::U64(2)] }),
        latest,
    );
    assert!(errors.is_empty(), "{errors:?}");

    // The schema check still holds it, and a stored byte array beside it is still read
    let errors = document_errors(
        &contract,
        platform_value!({ "pad": [Value::U64(300)] }),
        latest,
    );
    assert!(
        matches!(
            errors.as_slice(),
            [ConsensusError::BasicError(BasicError::JsonSchemaError(error))]
                if error.keyword() == "maximum"
        ),
        "{errors:?}"
    );
    let errors = document_errors(
        &contract,
        platform_value!({ "data": [Value::U64(1)], "pad": [Value::U64(1)] }),
        latest,
    );
    assert_eq!(byte_array_refusal(&errors), Ok("/data/0".to_string()));
}

/// The refusal is told from any other error under its keyword by the empty schema path and
/// property name it is built with, so nothing else is refused unpaid in its place.
#[test]
fn should_not_take_another_error_under_the_byte_array_keyword_for_the_refusal() {
    let error = |schema_path: &str, property_name: &str| -> ConsensusError {
        JsonSchemaError::new(
            "not bytes".to_string(),
            "byteArray".to_string(),
            "/data/0".to_string(),
            schema_path.to_string(),
            platform_value!({ "byteArray": true }),
            property_name.to_string(),
        )
        .into()
    };
    assert!(is_unreadable_byte_array_refusal(&error("", "")));
    assert!(!is_unreadable_byte_array_refusal(&error(
        "/properties/data/byteArray",
        ""
    )));
    assert!(!is_unreadable_byte_array_refusal(&error("", "data")));
}

/// Protocol version 14 keeps passing a byte array of integers that are not bytes, as it did.
#[test]
fn should_pass_a_byte_array_holding_integers_that_are_not_bytes_at_protocol_version_14() {
    let contract = byte_array_contract();
    let protocol_version_14 =
        PlatformVersion::get(14).expect("expected protocol version 14 to exist");
    for properties in [
        platform_value!({ "data": [Value::U64(1), Value::U64(2)] }),
        platform_value!({ "owner": bytes_of(|byte| Value::U64(byte as u64), 32) }),
    ] {
        let errors = document_errors(&contract, properties.clone(), protocol_version_14);
        assert!(errors.is_empty(), "{properties:?}: {errors:?}");
    }
}

/// The least time `run` takes over three runs, so a stall of the machine is not counted.
fn least_time_of<T>(mut run: impl FnMut() -> T) -> std::time::Duration {
    (0..3)
        .map(|_| {
            let started = std::time::Instant::now();
            std::hint::black_box(run());
            started.elapsed()
        })
        .min()
        .expect("three runs")
}

/// An identifier written as text is refused by the field size check past 5 KiB (10417) and by
/// the schema check below it (10101), whether it is base 58 or not. The byte array reading tells
/// whether the serializer could read the text, which for a text is whether it is base 58,
/// without decoding it, which takes time quadratic in the text's length.
#[test]
fn should_refuse_an_identifier_written_as_a_long_text_without_decoding_it() {
    let contract = byte_array_contract();
    let latest = PlatformVersion::latest();
    for (owner, code) in [
        ("2".repeat(19_500), 10417),
        (format!("{}0", "2".repeat(19_499)), 10417),
        ("2".repeat(5_000), 10101),
        (format!("{}0", "2".repeat(4_999)), 10101),
    ] {
        let properties = platform_value!({ "owner": owner });
        let errors = document_errors(&contract, properties.clone(), latest);
        let codes: Vec<u32> = errors.iter().map(|error| error.code()).collect();
        assert_eq!(codes, vec![code], "{} characters: {errors:?}", owner.len());
        let elapsed = least_time_of(|| document_errors(&contract, properties.clone(), latest));
        assert!(
            elapsed < std::time::Duration::from_millis(10),
            "{} characters took {elapsed:?}",
            owner.len()
        );
    }
}
