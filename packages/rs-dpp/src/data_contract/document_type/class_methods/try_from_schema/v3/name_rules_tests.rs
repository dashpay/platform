//! Names, generation 3 (protocol version 14): a property name and a document
//! type name are word characters only. Every earlier meta-schema and parser
//! generation admitted `-` as well; a census on 2026-09-23 of every data
//! contract create and update transition on mainnet (72) and testnet (4593),
//! decoded from the raw bytes with dpp, found no property or document type
//! name carrying one, so nothing stored is affected and no contract is left
//! unable to update. The path syntax (`a.b`, `list[]`) was never written for
//! `-`, which is how a hyphenated typed array's elements went unconverted.

use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::DocumentType;
use crate::ProtocolError;
use platform_value::{platform_value, Identifier, Value};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

fn parse(
    name: &str,
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
        name,
        schema,
        None,
        &BTreeMap::new(),
        &config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

fn schema_with_property(property_name: &str) -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            property_name: { "type": "string", "maxLength": 8, "position": 0 }
        },
        "additionalProperties": false
    })
}

fn schema_with_nested_property(property_name: &str) -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "meta": {
                "type": "object",
                "position": 0,
                "properties": {
                    property_name: { "type": "string", "maxLength": 8, "position": 0 }
                },
                "additionalProperties": false
            }
        },
        "additionalProperties": false
    })
}

fn is_json_schema_error(error: &ProtocolError) -> bool {
    matches!(
        error,
        ProtocolError::ConsensusError(boxed)
            if matches!(**boxed, ConsensusError::BasicError(BasicError::JsonSchemaError(_)))
    )
}

#[test]
fn should_refuse_a_hyphen_in_a_property_name_from_protocol_version_14_and_admit_it_before() {
    let v13 = PlatformVersion::get(13).expect("protocol version 13 exists");
    let v14 = PlatformVersion::latest();

    for schema in [
        schema_with_property("member-ids"),
        schema_with_nested_property("member-ids"),
    ] {
        let error = parse("note", schema.clone(), v14, true)
            .expect_err("meta-schema v3 refuses a hyphen in a property name");
        assert!(
            is_json_schema_error(&error),
            "expected the meta-schema refusal, got {error}"
        );
        parse("note", schema.clone(), v13, true)
            .expect("meta-schema v2 admits a hyphen in a property name, as it always did");
        // The stored path never checked names
        parse("note", schema, v14, false).expect("a stored schema is read as it is");
    }

    for schema in [
        schema_with_property("member_ids"),
        schema_with_nested_property("memberIds"),
    ] {
        parse("note", schema, v14, true).expect("word characters stay admitted");
    }
}

#[test]
fn should_refuse_a_hyphen_in_a_document_type_name_from_protocol_version_14_and_admit_it_before() {
    let v13 = PlatformVersion::get(13).expect("protocol version 13 exists");
    let v14 = PlatformVersion::latest();
    let schema = schema_with_property("label");

    let error = parse("journal-entry", schema.clone(), v14, true)
        .expect_err("generation 3 refuses a hyphen in a document type name");
    assert!(
        matches!(
            error,
            ProtocolError::ConsensusError(ref boxed)
                if matches!(
                    **boxed,
                    ConsensusError::BasicError(BasicError::InvalidDocumentTypeNameError(_))
                )
        ),
        "expected InvalidDocumentTypeNameError, got {error}"
    );
    parse("journal-entry", schema.clone(), v13, true)
        .expect("generation 2 admits a hyphen in a document type name, as it always did");
    parse("journal-entry", schema.clone(), v14, false)
        .expect("a stored document type is read under its name as it is");
    parse("journal_entry", schema, v14, true).expect("word characters stay admitted");
}
