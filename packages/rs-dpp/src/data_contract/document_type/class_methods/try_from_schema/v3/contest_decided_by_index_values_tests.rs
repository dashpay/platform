//! A contested index whose contest its index values do not decide, which registration refuses
//! from protocol version 15 (`check_document_type_schema` 1): a `fieldMatches` field outside
//! the index, or a match telling `""` from `"\0"`, which share an index key, on a string that
//! may be empty. A registration rule: a stored contract still parses, and protocol version 14
//! keeps registering such a contest.
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

/// An immutable `name` type of `label` and `normalizedLabel`, whose unique `byNormalizedLabel`
/// index on `normalizedLabel` is contested on `field` matching `pattern`. `normalizedLabel`
/// declares `normalized_min_length` when given.
fn name_schema(field: &str, pattern: &str, normalized_min_length: Option<u64>) -> Value {
    let mut normalized_label = platform_value!({
        "type": "string",
        "maxLength": 20,
        "position": 1,
    });
    if let Some(min_length) = normalized_min_length {
        normalized_label
            .insert("minLength".to_string(), Value::U64(min_length))
            .expect("expected to set the minimum length");
    }
    platform_value!({
        "type": "object",
        "documentsMutable": false,
        "canBeDeleted": false,
        "properties": {
            "label": { "type": "string", "maxLength": 20, "position": 0 },
            "normalizedLabel": normalized_label,
        },
        "required": ["label", "normalizedLabel"],
        "indices": [{
            "name": "byNormalizedLabel",
            "properties": [{ "normalizedLabel": "asc" }],
            "unique": true,
            "contested": {
                "fieldMatches": [{ "field": field, "regexPattern": pattern }],
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
        message.contains("the contest of index \"byNormalizedLabel\" on document type \"name\""),
        "{message}"
    );
}

#[test]
fn should_refuse_a_contest_matching_a_field_outside_the_index() {
    assert_refused(parse(name_schema("label", "^[a-z]{3,5}$", None)));
}

#[test]
fn should_register_a_contest_matching_a_field_outside_the_index_at_protocol_version_14() {
    parse_at(name_schema("label", "^[a-z]{3,5}$", None), 14, true)
        .expect("protocol version 14 keeps registering it");
}

#[test]
fn should_load_a_stored_contest_matching_a_field_outside_the_index() {
    parse_at(
        name_schema("label", "^[a-z]{3,5}$", None),
        PlatformVersion::latest().protocol_version,
        false,
    )
    .expect("a stored contract runs no registration rule");
}

#[test]
fn should_register_a_contest_matching_a_field_of_the_index() {
    let document_type = parse(name_schema("normalizedLabel", "^[a-z]{3,5}$", None))
        .expect("a match on an index property that judges \"\" and \"\\0\" alike is decided");
    let index = document_type
        .find_contested_index()
        .expect("expected the contested index");
    assert!(index.contest_is_decided_by_index_values(document_type.flattened_properties()));
}

#[test]
fn should_refuse_a_match_telling_the_empty_string_from_nul_on_a_string_that_may_be_empty() {
    assert_refused(parse(name_schema("normalizedLabel", "^[a-z]{0,5}$", None)));
}

#[test]
fn should_register_a_match_telling_the_empty_string_from_nul_on_a_string_that_cannot_be_empty() {
    parse(name_schema("normalizedLabel", "^[a-z]{0,5}$", Some(1)))
        .expect("a string of at least one character is never the empty string");
}

#[test]
fn should_register_a_contest_without_field_matches() {
    let mut schema = name_schema("normalizedLabel", "^[a-z]{3,5}$", None);
    let Some(Value::Array(indices)) = schema.get_mut("indices").expect("indices") else {
        panic!("expected the indices");
    };
    indices[0]
        .insert(
            "contested".to_string(),
            platform_value!({ "resolution": 0 }),
        )
        .expect("expected to replace the contest");
    parse(schema).expect("a contest taking every document is decided by its index values");
}
