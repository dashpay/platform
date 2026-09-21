//! The `canBeDeletedByModerators` doctype keyword (protocol version 14): what it requires of
//! the contract and of the document type.
use super::*;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use platform_value::platform_value;

fn moderated_config(platform_version: &PlatformVersion) -> DataContractConfig {
    DataContractConfig::default_for_version(platform_version)
        .expect("default config available")
        .with_moderation(Some(ContractModerationConfig {
            banlist: false,
            suspensions: false,
            moderators: ContractModerators::ContractOwner,
            warnings: false,
        }))
}

fn parse_with_config(
    schema: Value,
    config: &DataContractConfig,
    protocol_version: u32,
    full_validation: bool,
) -> Result<DocumentType, ProtocolError> {
    let platform_version =
        PlatformVersion::get(protocol_version).expect("expected platform version");
    DocumentType::try_from_schema(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "post",
        schema,
        None,
        &BTreeMap::new(),
        config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

fn parse_moderated(schema: Value) -> Result<DocumentType, ProtocolError> {
    let platform_version = PlatformVersion::latest();
    parse_with_config(
        schema,
        &moderated_config(platform_version),
        platform_version.protocol_version,
        true,
    )
}

fn post_schema(extra: Value) -> Value {
    let mut schema = platform_value!({
        "type": "object",
        "properties": {
            "text": {
                "type": "string",
                "maxLength": 50,
                "position": 0,
            },
        },
        "additionalProperties": false,
        "canBeDeletedByModerators": true,
    });
    if let (Value::Map(schema_map), Value::Map(extra_map)) = (&mut schema, extra) {
        schema_map.extend(extra_map);
    }
    schema
}

fn assert_refused_naming(result: Result<DocumentType, ProtocolError>, fragments: &[&str]) {
    let error = result.expect_err("the document type must be refused");
    // A paid refusal needs the consensus variant: the data contract error variant would
    // surface as an internal error in a block.
    assert!(
        matches!(error, ProtocolError::ConsensusError(_)),
        "expected a consensus error, got {error:?}"
    );
    let message = format!("{error:?}");
    for fragment in fragments {
        assert!(
            message.contains(fragment),
            "error must name {fragment}, got {message}"
        );
    }
}

#[test]
fn should_preserve_contract_structure_errors_for_non_object_schemas() {
    let platform_version = PlatformVersion::latest();
    let config = moderated_config(platform_version);
    for schema in [
        Value::Null,
        Value::Bool(false),
        Value::Array(vec![]),
        Value::Text("invalid".to_string()),
        Value::U64(1),
    ] {
        for full_validation in [false, true] {
            let error = parse_with_config(
                schema.clone(),
                &config,
                platform_version.protocol_version,
                full_validation,
            )
            .expect_err("a document schema must be an object");
            let ProtocolError::ConsensusError(error) = error else {
                panic!("expected a consensus error, got {error:?}");
            };
            assert!(
                matches!(error.as_ref(),
                    ConsensusError::BasicError(BasicError::ContractError(
                        DataContractError::InvalidContractStructure(message)
                    )) if message == "document schema must be an object: structure error: value is not a map"
                ),
                "schema {schema:?}, full_validation={full_validation}: {error:?}",
            );
        }
    }
}

#[test]
fn should_parse_the_flag_on_a_moderated_contract() {
    let document_type = parse_moderated(post_schema(platform_value!({}))).expect("parse");
    assert!(document_type.documents_can_be_deleted_by_moderators());
}

#[test]
fn should_default_the_flag_to_false() {
    let schema = platform_value!({
        "type": "object",
        "properties": {
            "text": { "type": "string", "maxLength": 50, "position": 0 },
        },
        "additionalProperties": false,
    });
    let document_type = parse_moderated(schema).expect("parse");
    assert!(!document_type.documents_can_be_deleted_by_moderators());
}

#[test]
fn should_keep_the_flag_independent_of_can_be_deleted() {
    // A post its author can not retract, but moderators can remove.
    let document_type =
        parse_moderated(post_schema(platform_value!({ "canBeDeleted": false }))).expect("parse");
    assert!(!document_type.documents_can_be_deleted());
    assert!(document_type.documents_can_be_deleted_by_moderators());
}

#[test]
fn should_refuse_the_flag_on_a_contract_without_moderation() {
    let platform_version = PlatformVersion::latest();
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available");
    for full_validation in [true, false] {
        assert_refused_naming(
            parse_with_config(
                post_schema(platform_value!({})),
                &config,
                platform_version.protocol_version,
                full_validation,
            ),
            &["canBeDeletedByModerators", "moderation"],
        );
    }
}

#[test]
fn should_refuse_the_flag_on_a_type_that_keeps_history() {
    assert_refused_naming(
        parse_moderated(post_schema(platform_value!({
            "documentsKeepHistory": true,
            "canBeDeleted": false,
        }))),
        &["documentsKeepHistory", "canBeDeletedByModerators"],
    );
}

#[test]
fn should_refuse_the_flag_on_a_type_that_restricts_creation() {
    assert_refused_naming(
        parse_moderated(post_schema(
            platform_value!({ "creationRestrictionMode": 1 }),
        )),
        &["restricts document creation", "canBeDeletedByModerators"],
    );
}

#[test]
fn should_refuse_the_flag_on_an_index_only_type() {
    let schema = platform_value!({
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "canBeDeletedByModerators": true,
        "indices": [
            { "name": "byTopic", "properties": [{ "topic": "asc" }] },
        ],
        "properties": {
            "topic": { "type": "string", "maxLength": 50, "position": 0 },
        },
        "required": ["topic"],
        "additionalProperties": false,
    });
    assert_refused_naming(
        parse_moderated(schema),
        &["indexOnly", "canBeDeletedByModerators"],
    );
}

#[test]
fn should_refuse_the_flag_on_a_type_with_a_contested_index() {
    // A moderator's restore puts a deleted document back through an ordinary insert; a
    // contested index only takes a document through a vote, so such a deletion could never be
    // undone.
    let schema = platform_value!({
        "type": "object",
        "documentsMutable": false,
        "canBeDeletedByModerators": true,
        "indices": [
            {
                "name": "byLabel",
                "properties": [{ "normalizedLabel": "asc" }],
                "unique": true,
                "contested": {
                    "fieldMatches": [
                        { "field": "normalizedLabel", "regexPattern": "^[a-z]{3,}$" },
                    ],
                    "resolution": 0,
                },
            },
        ],
        "properties": {
            "normalizedLabel": { "type": "string", "maxLength": 50, "position": 0 },
        },
        "required": ["normalizedLabel"],
        "additionalProperties": false,
    });
    assert_refused_naming(
        parse_moderated(schema),
        &["contested index", "canBeDeletedByModerators"],
    );
}

#[test]
fn should_allow_the_flag_on_a_transferable_tradeable_type() {
    let document_type = parse_moderated(post_schema(platform_value!({
        "transferable": 1,
        "tradeMode": 1,
    })))
    .expect("parse");
    assert!(document_type.documents_can_be_deleted_by_moderators());
}

#[test]
fn should_refuse_the_keyword_before_protocol_version_14() {
    // Meta-schema v2 (protocol versions 12 and 13) does not know the keyword.
    let platform_version = PlatformVersion::get(13).expect("expected platform version");
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available");
    let result = parse_with_config(post_schema(platform_value!({})), &config, 13, true);
    assert!(result.is_err(), "the keyword must not pass meta-schema v2");
}

// ---- canBeDeletedByModeratorsFor: the window the moderators have ----------------------

fn windowed_schema(extra: Value) -> Value {
    let mut schema = post_schema(platform_value!({
        "canBeDeletedByModeratorsFor": 86400,
        "required": ["$updatedAt"],
    }));
    if let (Value::Map(schema_map), Value::Map(extra_map)) = (&mut schema, extra) {
        for (key, value) in extra_map {
            schema_map.retain(|(existing, _)| existing != &key);
            schema_map.push((key, value));
        }
    }
    schema
}

#[test]
fn should_parse_the_window_the_moderators_have() {
    let document_type = parse_moderated(windowed_schema(platform_value!({}))).expect("parse");
    assert!(document_type.documents_can_be_deleted_by_moderators());
    assert_eq!(
        document_type.documents_can_be_deleted_by_moderators_for(),
        Some(86400)
    );
}

#[test]
fn should_leave_moderators_no_limit_without_the_window() {
    let document_type = parse_moderated(post_schema(platform_value!({}))).expect("parse");
    assert_eq!(
        document_type.documents_can_be_deleted_by_moderators_for(),
        None
    );
}

#[test]
fn should_refuse_a_window_on_a_type_moderators_can_not_delete_from() {
    // The flag set to false, and the flag left out: the window limits nothing either way.
    let flag_off = windowed_schema(platform_value!({ "canBeDeletedByModerators": false }));
    let mut flag_absent = windowed_schema(platform_value!({}));
    if let Value::Map(map) = &mut flag_absent {
        map.retain(|(key, _)| key != &Value::Text("canBeDeletedByModerators".to_string()));
    }
    for schema in [flag_off, flag_absent] {
        assert_refused_naming(
            parse_moderated(schema),
            &["canBeDeletedByModeratorsFor", "canBeDeletedByModerators"],
        );
    }
}

#[test]
fn should_measure_the_window_of_documents_that_never_change_from_their_creation() {
    // Nothing modifies such a document after it is created, so `$createdAt` is its last
    // modification and `$updatedAt` is not needed.
    let document_type = parse_moderated(windowed_schema(platform_value!({
        "documentsMutable": false,
        "required": ["$createdAt"],
    })))
    .expect("parse");
    assert_eq!(
        document_type.documents_can_be_deleted_by_moderators_for(),
        Some(86400)
    );
}

#[test]
fn should_refuse_a_window_on_a_type_that_carries_no_clock() {
    for mutable in [true, false] {
        assert_refused_naming(
            parse_moderated(windowed_schema(platform_value!({
                "documentsMutable": mutable,
                "required": ["text"],
            }))),
            &["canBeDeletedByModeratorsFor", "$updatedAt"],
        );
    }
}

#[test]
fn should_refuse_a_window_measured_from_creation_on_documents_that_can_be_replaced() {
    // Measured from creation alone, an author could wait the window out and then rewrite the
    // document into something no moderator can remove any more.
    assert_refused_naming(
        parse_moderated(windowed_schema(platform_value!({
            "documentsMutable": true,
            "required": ["$createdAt"],
        }))),
        &[
            "canBeDeletedByModeratorsFor",
            "can be replaced",
            "$updatedAt",
        ],
    );
    // With both, `$updatedAt` is the clock and the type is fine.
    parse_moderated(windowed_schema(platform_value!({
        "documentsMutable": true,
        "required": ["$createdAt", "$updatedAt"],
    })))
    .expect("parse");
}

#[test]
fn should_refuse_a_window_that_is_not_a_positive_number_of_seconds_on_the_stored_path_too() {
    // No meta-schema stands in front of a stored contract, and no keyword is read more
    // leniently there: the parser refuses the shape itself.
    let platform_version = PlatformVersion::latest();
    for full_validation in [true, false] {
        for window in [
            platform_value!(0),
            platform_value!(-5),
            platform_value!(4294967296u64),
            platform_value!("a day"),
        ] {
            let result = parse_with_config(
                windowed_schema(platform_value!({ "canBeDeletedByModeratorsFor": window.clone() })),
                &moderated_config(platform_version),
                platform_version.protocol_version,
                full_validation,
            );
            assert!(
                result.is_err(),
                "window {window:?} must be refused (full validation: {full_validation})"
            );
        }
    }
}

#[test]
fn should_refuse_a_schema_that_is_not_an_object_as_an_invalid_contract_structure() {
    // The window is read off the raw schema before the core parser checks that
    // the schema is an object. It must not be the reader that fails first: a
    // schema that is no object carries no keyword, and the core parser is the
    // one that names the real problem.
    let platform_version = PlatformVersion::latest();
    for full_validation in [true, false] {
        for schema in [
            platform_value!(null),
            platform_value!("post"),
            platform_value!([]),
        ] {
            let error = parse_with_config(
                schema.clone(),
                &moderated_config(platform_version),
                platform_version.protocol_version,
                full_validation,
            )
            .expect_err("a schema that is not an object must be refused");
            let message = format!("{error:?}");
            assert!(
                message.contains("InvalidContractStructure"),
                "schema {schema:?} must be refused as an invalid contract structure (full \
                 validation: {full_validation}), got {message}"
            );
        }
    }
}
