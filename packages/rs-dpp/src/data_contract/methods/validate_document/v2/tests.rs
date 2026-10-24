//! Generation 2 of document validation, at protocol version 15 against generation 1 at 14.

use crate::consensus::basic::BasicError;
use crate::consensus::codes::ErrorWithCode;
use crate::consensus::ConsensusError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::property_constraints::DocumentSystemValues;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::methods::validate_document::DataContractDocumentValidationMethodsV0;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::DataContract;
use crate::tests::fixtures::get_data_contract_fixture;
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
        platform_value!({ "data": [1, 2] }),
        platform_value!({ "data": [1, 300] }),
        platform_value!({ "when": "2026-10-22" }),
        platform_value!({ "when": "2026-13-01" }),
        platform_value!({ "other": 1 }),
    ] {
        // A clone holds neither validator built, as a contract read anew
        let contract = contract.clone();
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
    assert!(contract
        .document_type_validation_schema("item", platform_version)
        .is_err());

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
