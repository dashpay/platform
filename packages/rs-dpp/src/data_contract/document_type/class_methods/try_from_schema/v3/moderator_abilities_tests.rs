//! The `moderatorAbilities` doctype keyword (protocol version 14): what each of its keys
//! (`delete`, `deleteWithin`, `changeFields`) requires of the contract and of the document
//! type.
use super::*;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use crate::data_contract::document_type::methods::DocumentTypeBasicMethods;
use platform_value::platform_value;
use std::collections::BTreeSet;

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

/// Every top-level entry of `extra` in place of the one of `schema` with its key.
fn merged(mut schema: Value, extra: Value) -> Value {
    if let (Value::Map(schema_map), Value::Map(extra_map)) = (&mut schema, extra) {
        for (key, value) in extra_map {
            schema_map.retain(|(existing, _)| existing != &key);
            schema_map.push((key, value));
        }
    }
    schema
}

/// `schema` with `moderatorAbilities` left out.
fn without_abilities(mut schema: Value) -> Value {
    if let Value::Map(map) = &mut schema {
        map.retain(|(key, _)| key != &Value::Text("moderatorAbilities".to_string()));
    }
    schema
}

/// A `post` whose moderators may delete it, with `extra` merged in at the top level.
fn post_schema(extra: Value) -> Value {
    merged(
        platform_value!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "maxLength": 50,
                    "position": 0,
                },
            },
            "additionalProperties": false,
            "moderatorAbilities": { "delete": true },
        }),
        extra,
    )
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
                matches!(
                    *error,
                    ConsensusError::BasicError(BasicError::ContractError(
                        DataContractError::InvalidContractStructure(_)
                    ))
                ),
                "schema {schema:?} must be refused as an invalid contract structure (full \
                 validation: {full_validation}), got {error:?}"
            );
        }
    }
}

// ---- delete ------------------------------------------------------------------------------

#[test]
fn should_parse_delete_on_a_moderated_contract() {
    let document_type = parse_moderated(post_schema(platform_value!({}))).expect("parse");
    assert!(document_type.documents_can_be_deleted_by_moderators());
    assert!(document_type.moderator_changeable_fields().is_empty());
}

#[test]
fn should_give_moderators_nothing_without_the_keyword() {
    let document_type =
        parse_moderated(without_abilities(post_schema(platform_value!({})))).expect("parse");
    assert!(!document_type.documents_can_be_deleted_by_moderators());
    assert!(document_type.moderator_changeable_fields().is_empty());
}

#[test]
fn should_keep_delete_independent_of_can_be_deleted() {
    // A post its author can not retract, but moderators can remove.
    let document_type =
        parse_moderated(post_schema(platform_value!({ "canBeDeleted": false }))).expect("parse");
    assert!(!document_type.documents_can_be_deleted());
    assert!(document_type.documents_can_be_deleted_by_moderators());
}

#[test]
fn should_refuse_the_keyword_on_a_contract_without_moderation() {
    let platform_version = PlatformVersion::latest();
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available");
    for abilities in [
        platform_value!({ "delete": true }),
        platform_value!({ "changeFields": ["status"] }),
    ] {
        for full_validation in [true, false] {
            assert_refused_naming(
                parse_with_config(
                    report_schema(abilities.clone()),
                    &config,
                    platform_version.protocol_version,
                    full_validation,
                ),
                &["moderatorAbilities", "moderation"],
            );
        }
    }
}

#[test]
fn should_refuse_delete_on_a_type_that_keeps_history() {
    assert_refused_naming(
        parse_moderated(post_schema(platform_value!({
            "documentsKeepHistory": true,
            "canBeDeleted": false,
        }))),
        &["documentsKeepHistory", "moderatorAbilities.delete"],
    );
}

#[test]
fn should_refuse_delete_on_a_type_that_restricts_creation() {
    assert_refused_naming(
        parse_moderated(post_schema(
            platform_value!({ "creationRestrictionMode": 1 }),
        )),
        &["restricts document creation", "moderatorAbilities.delete"],
    );
}

#[test]
fn should_refuse_delete_on_an_index_only_type() {
    let schema = platform_value!({
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "moderatorAbilities": { "delete": true },
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
        &["indexOnly", "moderatorAbilities.delete"],
    );
}

/// A type with a contested index over `normalizedLabel`, and an optional `status`.
fn contested_schema(abilities: Value) -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": false,
        "moderatorAbilities": abilities,
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
            "status": { "type": "integer", "minimum": 0, "maximum": 3, "position": 1 },
        },
        "required": ["normalizedLabel"],
        "additionalProperties": false,
    })
}

#[test]
fn should_refuse_delete_on_a_type_with_a_contested_index() {
    // A moderator's restore puts a deleted document back through an ordinary insert; a
    // contested index only takes a document through a vote, so such a deletion could never be
    // undone.
    assert_refused_naming(
        parse_moderated(contested_schema(platform_value!({ "delete": true }))),
        &["contested index", "moderatorAbilities.delete"],
    );
}

#[test]
fn should_allow_delete_on_a_transferable_tradeable_type() {
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

// ---- deleteWithin: the window the moderators have ----------------------------------------

fn windowed_schema(extra: Value) -> Value {
    merged(
        post_schema(platform_value!({
            "moderatorAbilities": { "delete": true, "deleteWithin": 86400 },
            "required": ["$updatedAt"],
        })),
        extra,
    )
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
    // `delete` set to false, and left out: the window limits nothing either way.
    for abilities in [
        platform_value!({ "delete": false, "deleteWithin": 86400 }),
        platform_value!({ "deleteWithin": 86400 }),
    ] {
        assert_refused_naming(
            parse_moderated(windowed_schema(
                platform_value!({ "moderatorAbilities": abilities }),
            )),
            &["moderatorAbilities.deleteWithin", "delete: true"],
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
            &["moderatorAbilities.deleteWithin", "$updatedAt"],
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
            "moderatorAbilities.deleteWithin",
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
                windowed_schema(platform_value!({
                    "moderatorAbilities": { "delete": true, "deleteWithin": window.clone() },
                })),
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
    // The keyword is read off the raw schema before the core parser checks that the schema is
    // an object. It must not be the reader that fails first: a schema that is no object
    // carries no keyword, and the core parser is the one that names the real problem.
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

// ---- deleteKeepsRecord and deleteRefundsOwner: what a deletion leaves -------------------

#[test]
fn should_keep_a_record_and_forfeit_the_refund_by_default() {
    let document_type = parse_moderated(post_schema(platform_value!({}))).expect("parse");
    assert!(document_type.moderator_deletions_keep_records());
    assert!(!document_type.moderator_deletions_refund_owner());

    // A type moderators can not delete from keeps no records and refunds nobody.
    let document_type =
        parse_moderated(without_abilities(post_schema(platform_value!({})))).expect("parse");
    assert!(!document_type.moderator_deletions_keep_records());
    assert!(!document_type.moderator_deletions_refund_owner());
}

#[test]
fn should_parse_what_a_deletion_leaves() {
    for (keeps_record, refunds_owner) in [(false, false), (false, true), (true, true)] {
        let document_type = parse_moderated(post_schema(platform_value!({
            "moderatorAbilities": {
                "delete": true,
                "deleteKeepsRecord": keeps_record,
                "deleteRefundsOwner": refunds_owner,
            },
        })))
        .expect("parse");
        assert!(document_type.documents_can_be_deleted_by_moderators());
        assert_eq!(
            document_type.moderator_deletions_keep_records(),
            keeps_record
        );
        assert_eq!(
            document_type.moderator_deletions_refund_owner(),
            refunds_owner
        );
    }
}

#[test]
fn should_refuse_what_a_deletion_leaves_without_a_deletion() {
    let platform_version = PlatformVersion::latest();
    for (abilities, key) in [
        (
            platform_value!({ "deleteKeepsRecord": true, "changeFields": ["status"] }),
            "moderatorAbilities.deleteKeepsRecord",
        ),
        (
            platform_value!({ "delete": false, "deleteRefundsOwner": true }),
            "moderatorAbilities.deleteRefundsOwner",
        ),
    ] {
        for full_validation in [true, false] {
            assert_refused_naming(
                parse_with_config(
                    report_schema(abilities.clone()),
                    &moderated_config(platform_version),
                    platform_version.protocol_version,
                    full_validation,
                ),
                &[key, "delete: true"],
            );
        }
    }
}

#[test]
fn should_refuse_what_a_deletion_leaves_when_it_is_not_a_boolean_on_the_stored_path_too() {
    let platform_version = PlatformVersion::latest();
    for key in ["deleteKeepsRecord", "deleteRefundsOwner"] {
        let mut abilities = platform_value!({ "delete": true });
        abilities
            .insert(key.to_string(), platform_value!("yes"))
            .expect("expected to set the key");
        for full_validation in [true, false] {
            assert!(
                parse_with_config(
                    post_schema(platform_value!({ "moderatorAbilities": abilities.clone() })),
                    &moderated_config(platform_version),
                    platform_version.protocol_version,
                    full_validation,
                )
                .is_err(),
                "`{key}: \"yes\"` must be refused (full validation: {full_validation})"
            );
        }
    }
}

// ---- the shape of the object -------------------------------------------------------------

#[test]
fn should_refuse_a_malformed_object_on_the_stored_path_too() {
    let platform_version = PlatformVersion::latest();
    for (abilities, fragment) in [
        (platform_value!(true), "must be an object"),
        (platform_value!({}), "is empty"),
        (platform_value!({ "remove": true }), "has no key"),
        (platform_value!({ "changeFields": [] }), "lists no property"),
        (platform_value!({ "changeFields": "status" }), "array"),
        (platform_value!({ "changeFields": [3] }), "property name"),
        (platform_value!({ "delete": "yes" }), ""),
    ] {
        for full_validation in [true, false] {
            let result = parse_with_config(
                report_schema(abilities.clone()),
                &moderated_config(platform_version),
                platform_version.protocol_version,
                full_validation,
            );
            let error = result.expect_err("a malformed `moderatorAbilities` must be refused");
            let message = format!("{error:?}");
            // The meta-schema speaks first under full validation; the parser's own words are
            // what a stored contract gets.
            if !full_validation {
                assert!(
                    message.contains(fragment),
                    "{abilities:?} must be refused naming {fragment:?}, got {message}"
                );
            }
        }
    }
}

// ---- changeFields: the fields only moderators write ---------------------------------------

/// A `report` its owner can not replace, with `postId` referring to a post, a `reason`, and
/// the fields a moderator could keep: `status`, `resolution` and `handledAt`, indexed by
/// `status`.
fn report_schema(abilities: Value) -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": false,
        "moderatorAbilities": abilities,
        "indices": [
            { "name": "byStatus", "properties": [{ "status": "asc" }, { "$createdAt": "asc" }] },
        ],
        "properties": {
            "postId": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "position": 0,
                "refersTo": { "type": "identity" },
            },
            "reason": { "type": "integer", "minimum": 0, "maximum": 8, "position": 1 },
            "status": { "type": "integer", "minimum": 0, "maximum": 3, "position": 2 },
            "resolution": { "type": "string", "maxLength": 200, "position": 3 },
            "handledAt": {
                "type": "object",
                "properties": {
                    "time": { "type": "integer", "minimum": 0, "position": 0 },
                },
                "additionalProperties": false,
                "position": 4,
            },
        },
        "required": ["$createdAt", "postId", "reason"],
        "additionalProperties": false,
    })
}

#[test]
fn should_parse_the_fields_only_moderators_write() {
    let document_type = parse_moderated(report_schema(platform_value!({
        "changeFields": ["status", "resolution", "handledAt"],
    })))
    .expect("parse");
    assert_eq!(
        document_type.moderator_changeable_fields(),
        &BTreeSet::from([
            "handledAt".to_string(),
            "resolution".to_string(),
            "status".to_string(),
        ])
    );
    // The fields are theirs to write, not a deletion: the two abilities are independent.
    assert!(!document_type.documents_can_be_deleted_by_moderators());
    // An indexed field is fine: moderators query their queue by it.
    assert!(document_type.indexes().contains_key("byStatus"));
}

#[test]
fn should_keep_a_revision_on_documents_moderators_change_even_when_owners_can_not() {
    // A moderator's change is stored as an update, which needs a revision: an owner can not
    // replace a report, and still every report carries one from its creation.
    let changeable = parse_moderated(report_schema(platform_value!({
        "changeFields": ["status"],
    })))
    .expect("parse");
    assert!(!changeable.documents_mutable());
    assert!(changeable.requires_revision());
    assert_eq!(changeable.initial_revision(), Some(1));

    let deletable_only =
        parse_moderated(report_schema(platform_value!({ "delete": true }))).expect("parse");
    assert!(!deletable_only.requires_revision());
    assert_eq!(deletable_only.initial_revision(), None);
}

#[test]
fn should_parse_both_abilities_together() {
    let document_type = parse_moderated(report_schema(platform_value!({
        "delete": true,
        "changeFields": ["status"],
    })))
    .expect("parse");
    assert!(document_type.documents_can_be_deleted_by_moderators());
    assert_eq!(
        document_type.moderator_changeable_fields(),
        &BTreeSet::from(["status".to_string()])
    );
}

#[test]
fn should_refuse_a_field_that_is_not_a_top_level_property() {
    for (field, fragment) in [
        ("verdict", "not a top-level property"),
        ("handledAt.time", "list the object around a nested property"),
        ("$updatedAt", "not a top-level property"),
    ] {
        assert_refused_naming(
            parse_moderated(report_schema(platform_value!({ "changeFields": [field] }))),
            &["moderatorAbilities.changeFields", field, fragment],
        );
    }
}

#[test]
fn should_refuse_a_required_field() {
    // Nobody but a moderator writes it, so a create by anyone else could never supply it.
    assert_refused_naming(
        parse_moderated(report_schema(
            platform_value!({ "changeFields": ["reason"] }),
        )),
        &["reason", "is required"],
    );
}

#[test]
fn should_refuse_a_transient_field() {
    assert_refused_naming(
        parse_moderated(merged(
            report_schema(platform_value!({ "changeFields": ["resolution"] })),
            platform_value!({ "transient": ["resolution"] }),
        )),
        &["resolution", "transient"],
    );
}

#[test]
fn should_refuse_an_immutable_field() {
    assert_refused_naming(
        parse_moderated(merged(
            report_schema(platform_value!({ "changeFields": ["status"] })),
            platform_value!({ "documentsMutable": true, "immutable": ["status"] }),
        )),
        &["status", "immutable"],
    );
}

#[test]
fn should_refuse_a_field_holding_a_reference() {
    assert_refused_naming(
        parse_moderated(merged(
            report_schema(platform_value!({ "changeFields": ["postId"] })),
            platform_value!({ "required": ["$createdAt", "reason"] }),
        )),
        &["postId", "reference"],
    );
}

#[test]
fn should_refuse_a_field_a_reference_agreement_reads() {
    // `topic` is the referring side of the agreement `postId` declares: a moderator changing
    // it would leave the reference checked against a value it no longer holds.
    let schema = platform_value!({
        "type": "object",
        "moderatorAbilities": { "changeFields": ["topic"] },
        "properties": {
            "topic": { "type": "string", "maxLength": 63, "position": 0 },
            "postId": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "position": 1,
                "refersTo": {
                    "type": "permanentDocument",
                    "documentType": "post",
                    "propertyAgreement": { "topic": "topic" },
                },
            },
        },
        "additionalProperties": false,
    });
    assert_refused_naming(parse_moderated(schema), &["topic", "reference"]);
}

#[test]
fn should_refuse_a_field_a_computed_lookup_key_hashes() {
    // `label` is a param of the function computing the lookup's key: the key is judged when
    // the document is created only, so a moderator changing `label` would leave a stored
    // document that no longer hashes to the commitment it revealed. The type is immutable,
    // which the lookup's own fixed-once-written rule would otherwise accept.
    let schema = platform_value!({
        "type": "object",
        "documentsMutable": false,
        "moderatorAbilities": { "changeFields": ["label"] },
        "indices": [
            { "name": "byHash", "properties": [{ "hash": "asc" }], "unique": true },
        ],
        "properties": {
            "hash": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "position": 0,
            },
            "label": { "type": "string", "maxLength": 63, "position": 1 },
            "salt": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "position": 2,
                "refersTo": {
                    "type": "permanentDocument",
                    "documentType": "post",
                    "lookup": { "index": "byHash" },
                    "propertyAgreement": {
                        "hash": { "function": "sys.hash.sha256d", "params": ["salt", "label"] },
                    },
                },
            },
        },
        "required": ["hash"],
        "transient": ["salt"],
        "additionalProperties": false,
    });
    assert_refused_naming(parse_moderated(schema), &["label", "reference"]);
}

#[test]
fn should_refuse_a_generated_field_and_what_a_generated_field_reads() {
    let schema = |field: &str| {
        platform_value!({
            "type": "object",
            "moderatorAbilities": { "changeFields": [field] },
            "properties": {
                "label": { "type": "string", "maxLength": 32, "position": 0 },
                "normalizedLabel": {
                    "type": "string",
                    "maxLength": 32,
                    "position": 1,
                    "generatedFrom": {
                        "function": "sys.stringTransformations.homographSafeASCII",
                        "params": ["label"],
                    },
                },
            },
            "additionalProperties": false,
        })
    };
    for field in ["label", "normalizedLabel"] {
        assert_refused_naming(parse_moderated(schema(field)), &[field, "generated"]);
    }
}

#[test]
fn should_refuse_a_field_in_a_contested_index() {
    assert_refused_naming(
        parse_moderated(merged(
            contested_schema(platform_value!({ "changeFields": ["normalizedLabel"] })),
            platform_value!({ "required": [] }),
        )),
        &["normalizedLabel", "contested index"],
    );
    // A field of the same type outside the contested index is fine.
    parse_moderated(contested_schema(
        platform_value!({ "changeFields": ["status"] }),
    ))
    .expect("parse");
}

#[test]
fn should_refuse_fields_on_an_index_only_type() {
    let schema = platform_value!({
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "moderatorAbilities": { "changeFields": ["topic"] },
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
        &["indexOnly", "moderatorAbilities.changeFields"],
    );
}

// ---- the moderation stamp: `$moderatedAt` and `$moderatedBy` -------------------------------

/// `report_schema(abilities)` with `index` beside its own
fn report_schema_indexing(abilities: Value, index: Value) -> Value {
    let mut schema = report_schema(abilities);
    if let Value::Map(map) = &mut schema {
        for (key, value) in map.iter_mut() {
            if key == &Value::Text("indices".to_string()) {
                if let Value::Array(indices) = value {
                    indices.push(index.clone());
                }
            }
        }
    }
    schema
}

fn stamp_index(unique: bool) -> Value {
    platform_value!({
        "name": "byModerator",
        "properties": [{ "$moderatedBy": "asc" }, { "$moderatedAt": "asc" }],
        "unique": unique,
    })
}

#[test]
fn should_index_the_stamp_of_a_type_with_fields_only_moderators_write() {
    let document_type = parse_moderated(report_schema_indexing(
        platform_value!({ "changeFields": ["status"] }),
        stamp_index(false),
    ))
    .expect("a type whose fields moderators write indexes who wrote them last, and when");
    let index = document_type
        .indexes()
        .get("byModerator")
        .expect("expected the index");
    assert_eq!(index.properties[0].name, "$moderatedBy");
    assert_eq!(index.properties[1].name, "$moderatedAt");
}

#[test]
fn should_refuse_to_index_the_stamp_of_a_type_no_moderator_writes() {
    // Only a moderator's write of a type's `changeFields` stamps its documents: a type whose
    // moderators only delete, or that gives them nothing, would index a value none carries.
    assert_refused_naming(
        parse_moderated(report_schema_indexing(
            platform_value!({ "delete": true }),
            stamp_index(false),
        )),
        &[
            "byModerator",
            "$moderatedBy",
            "moderatorAbilities.changeFields",
        ],
    );
    assert_refused_naming(
        parse_moderated(without_abilities(report_schema_indexing(
            platform_value!({ "changeFields": ["status"] }),
            stamp_index(false),
        ))),
        &["byModerator", "$moderatedBy"],
    );
}

#[test]
fn should_refuse_the_stamp_in_a_unique_index() {
    assert_refused_naming(
        parse_moderated(report_schema_indexing(
            platform_value!({ "changeFields": ["status"] }),
            stamp_index(true),
        )),
        &["unique index", "byModerator", "$moderatedBy"],
    );
}

#[test]
fn should_refuse_to_index_the_stamp_before_protocol_version_14() {
    // Generations before 3 know no such system property: the index names an undefined one.
    let platform_version = PlatformVersion::get(13).expect("expected platform version");
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available");
    let schema = without_abilities(post_schema(platform_value!({
        "indices": [stamp_index(false)],
    })));
    let error = parse_with_config(schema, &config, 13, true)
        .expect_err("the stamp is not indexable before protocol version 14");
    assert!(
        matches!(
            error,
            ProtocolError::ConsensusError(ref error)
                if matches!(**error, ConsensusError::BasicError(BasicError::UndefinedIndexPropertyError(_)))
        ),
        "expected UndefinedIndexPropertyError, got {error:?}"
    );
}

#[test]
fn should_refuse_fields_only_moderators_write_beside_a_required_transient_property() {
    // The salt is checked when a report is filed and never stored: a moderator's change, judged
    // against the schema, would find it missing on every report.
    let schema = platform_value!({
        "type": "object",
        "moderatorAbilities": { "changeFields": ["status"] },
        "transient": ["salt"],
        "properties": {
            "salt": { "type": "string", "maxLength": 32, "position": 0 },
            "status": { "type": "integer", "minimum": 0, "maximum": 3, "position": 1 },
        },
        "required": ["salt"],
        "additionalProperties": false,
    });
    assert_refused_naming(
        parse_moderated(schema),
        &[
            "transient property \\\"salt\\\"",
            "moderatorAbilities.changeFields",
        ],
    );
}
