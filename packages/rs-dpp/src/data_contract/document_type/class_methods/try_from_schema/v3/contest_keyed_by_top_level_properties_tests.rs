//! A contested index keyed by a property path or a system property, which registration
//! refuses from protocol version 15 (`check_document_type_schema` 1): no contested create can
//! open or join such a contest. A registration rule: a stored contract still parses, and
//! protocol version 14 keeps registering such an index.
use super::*;
use crate::consensus::basic::BasicError;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use platform_value::platform_value;

fn parse_at(
    schema: Value,
    protocol_version: u32,
    full_validation: bool,
) -> Result<DocumentType, ProtocolError> {
    let platform_version =
        PlatformVersion::get(protocol_version).expect("expected platform version");
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available");
    DocumentType::try_from_schema(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "name",
        schema,
        None,
        &BTreeMap::new(),
        &config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

fn parse(schema: Value) -> Result<DocumentType, ProtocolError> {
    parse_at(schema, PlatformVersion::latest().protocol_version, true)
}

/// An immutable `name` type of `normalizedLabel` and a `records` object holding an `identity`,
/// whose unique `byKeyAndLabel` index on `key` and `normalizedLabel` is contested on
/// `normalizedLabel`.
fn name_schema(key: &str) -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": false,
        "canBeDeleted": false,
        "properties": {
            "normalizedLabel": { "type": "string", "maxLength": 20, "position": 0 },
            "records": {
                "type": "object",
                "properties": {
                    "identity": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "position": 0,
                    },
                },
                "required": ["identity"],
                "additionalProperties": false,
                "position": 1,
            },
            "group": { "type": "string", "maxLength": 20, "position": 2 },
        },
        "required": ["normalizedLabel", "records", "group"],
        "indices": [{
            "name": "byKeyAndLabel",
            "properties": [{ key: "asc" }, { "normalizedLabel": "asc" }],
            "unique": true,
            "contested": {
                "fieldMatches": [
                    { "field": "normalizedLabel", "regexPattern": "^[a-z]{3,5}$" }
                ],
                "resolution": 0,
            },
        }],
        "additionalProperties": false,
    })
}

fn assert_refused(result: Result<DocumentType, ProtocolError>) {
    let error = result.expect_err("the document type must be refused");
    let ProtocolError::ConsensusError(consensus_error) = &error else {
        panic!("expected a paid consensus error, got {error:?}");
    };
    let ConsensusError::BasicError(BasicError::ContractError(
        DataContractError::InvalidContractStructure(message),
    )) = consensus_error.as_ref()
    else {
        panic!("expected InvalidContractStructure, got {consensus_error:?}");
    };
    assert!(
        message.contains(
            "the contest of index \"byKeyAndLabel\" on document type \"name\" must be keyed by \
             top-level properties"
        ),
        "{message}"
    );
}

#[test]
fn should_refuse_a_contest_keyed_by_a_property_path() {
    assert_refused(parse(name_schema("records.identity")));
}

#[test]
fn should_refuse_a_contest_keyed_by_a_system_property() {
    assert_refused(parse(name_schema("$ownerId")));
}

#[test]
fn should_register_a_contest_keyed_by_top_level_properties() {
    let document_type =
        parse(name_schema("group")).expect("a contest on top-level properties registers");
    let index = document_type
        .find_contested_index()
        .expect("expected the contested index");
    assert!(index.contest_is_keyed_by_top_level_properties());
}

#[test]
fn should_register_a_unique_index_without_a_contest_keyed_by_a_property_path() {
    let mut schema = name_schema("records.identity");
    let Some(Value::Array(indices)) = schema.get_mut("indices").expect("indices") else {
        panic!("expected the indices");
    };
    indices[0]
        .remove("contested")
        .expect("expected to remove the contest");
    parse(schema).expect("only a contest must be keyed by top-level properties");
}

#[test]
fn should_register_a_contest_keyed_by_a_property_path_at_protocol_version_14() {
    parse_at(name_schema("records.identity"), 14, true)
        .expect("protocol version 14 keeps registering it");
    parse_at(name_schema("$ownerId"), 14, true).expect("protocol version 14 keeps registering it");
}

#[test]
fn should_load_a_stored_contest_keyed_by_a_property_path() {
    let latest = PlatformVersion::latest().protocol_version;
    parse_at(name_schema("records.identity"), latest, false)
        .expect("a stored contract runs no registration rule");
    parse_at(name_schema("$ownerId"), latest, false)
        .expect("a stored contract runs no registration rule");
}
