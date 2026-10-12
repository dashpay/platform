//! `canBeDeleted: "onlyWhenConsumed"` (protocol version 14): the owner can not delete a
//! document, a create whose `refersTo` declares `consume` can. What the parser records on
//! both paths, what it refuses, and what the value makes of the type for references.
use super::*;
use crate::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use crate::data_contract::document_type::DocumentReferenceKind;
use platform_value::platform_value;

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
        "commitment",
        schema,
        None,
        &BTreeMap::new(),
        config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

fn parse(schema: Value, full_validation: bool) -> Result<DocumentType, ProtocolError> {
    let platform_version = PlatformVersion::latest();
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available");
    parse_with_config(
        schema,
        &config,
        platform_version.protocol_version,
        full_validation,
    )
}

/// An immutable commitment only a consume deletes, with `extra` merged in.
fn commitment_schema(extra: Value) -> Value {
    let mut schema = platform_value!({
        "type": "object",
        "documentsMutable": false,
        "canBeDeleted": "onlyWhenConsumed",
        "properties": {
            "hash": {
                "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32, "position": 0
            },
        },
        "indices": [
            { "name": "byHash", "properties": [{ "hash": "asc" }], "unique": true },
        ],
        "required": ["hash"],
        "additionalProperties": false,
    });
    if let (Value::Map(schema_map), Value::Map(extra_map)) = (&mut schema, extra) {
        for (key, value) in extra_map {
            schema_map.retain(|(existing, _)| existing != &key);
            schema_map.push((key, value));
        }
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
fn should_parse_documents_only_a_consume_deletes_on_both_paths() {
    for full_validation in [true, false] {
        let document_type = parse(commitment_schema(platform_value!({})), full_validation)
            .expect("a commitment only a consume deletes parses");
        // The owner's delete is refused, as for `false`
        assert!(!document_type.documents_can_be_deleted());
        assert!(document_type.documents_deleted_only_when_consumed());
        // A consume can remove one, so a reference that must always resolve may not
        // target the type
        assert!(document_type.documents_can_disappear());
        assert_eq!(
            document_type.document_reference_kind(),
            DocumentReferenceKind::Deletable
        );
    }
}

#[test]
fn should_leave_both_booleans_as_they_were() {
    for (can_be_deleted, kind) in [
        (true, DocumentReferenceKind::Deletable),
        (false, DocumentReferenceKind::Permanent),
    ] {
        let document_type = parse(
            commitment_schema(platform_value!({ "canBeDeleted": can_be_deleted })),
            true,
        )
        .expect("parse");
        assert_eq!(document_type.documents_can_be_deleted(), can_be_deleted);
        assert!(!document_type.documents_deleted_only_when_consumed());
        assert_eq!(document_type.document_reference_kind(), kind);
    }
}

#[test]
fn should_read_the_first_can_be_deleted_entry_as_every_keyword_does() {
    // A schema map may repeat a key; like every doctype keyword, `canBeDeleted` is read from
    // its first entry, so a later "onlyWhenConsumed" does not make a permanent type consumable
    let mut schema = commitment_schema(platform_value!({}));
    if let Value::Map(schema_map) = &mut schema {
        schema_map.retain(|(key, _)| key.as_text() != Some("canBeDeleted"));
        schema_map.insert(
            0,
            (Value::Text("canBeDeleted".to_string()), Value::Bool(false)),
        );
        schema_map.push((
            Value::Text("canBeDeleted".to_string()),
            Value::Text("onlyWhenConsumed".to_string()),
        ));
    }
    let document_type = parse(schema, false).expect("the stored path reads the first entry");
    assert!(!document_type.documents_can_be_deleted());
    assert!(!document_type.documents_deleted_only_when_consumed());
    assert_eq!(
        document_type.document_reference_kind(),
        DocumentReferenceKind::Permanent
    );
}

#[test]
fn should_make_a_type_its_moderators_remove_with_records_deletable_when_a_consume_deletes_it() {
    // A consume removes a document without a removal record, so the type is no longer one
    // whose documents leave state only on the record: a `moderatedDocument` reference could
    // not resolve a consumed one.
    let platform_version = PlatformVersion::latest();
    let moderated = DataContractConfig::default_for_version(platform_version)
        .expect("default config available")
        .with_moderation(Some(ContractModerationConfig {
            banlist: false,
            suspensions: false,
            moderators: ContractModerators::ContractOwner,
            warnings: false,
        }));
    let schema = |can_be_deleted: Value| {
        let mut schema = commitment_schema(platform_value!({
            "moderatorAbilities": { "delete": true },
        }));
        schema
            .insert("canBeDeleted".to_string(), can_be_deleted)
            .expect("expected to set canBeDeleted");
        schema
    };
    let recorded = parse_with_config(
        schema(Value::Bool(false)),
        &moderated,
        platform_version.protocol_version,
        true,
    )
    .expect("parse");
    assert_eq!(
        recorded.document_reference_kind(),
        DocumentReferenceKind::Moderated
    );
    let consumed = parse_with_config(
        schema(Value::Text("onlyWhenConsumed".to_string())),
        &moderated,
        platform_version.protocol_version,
        true,
    )
    .expect("parse");
    assert!(consumed.documents_can_be_deleted_by_moderators());
    assert_eq!(
        consumed.document_reference_kind(),
        DocumentReferenceKind::Deletable
    );
}

#[test]
fn should_refuse_it_on_a_type_that_keeps_history_on_both_paths() {
    // The storage layer never deletes a document whose type keeps history, so nothing could
    // consume one
    for full_validation in [true, false] {
        assert_refused_naming(
            parse(
                commitment_schema(platform_value!({ "documentsKeepHistory": true })),
                full_validation,
            ),
            &["documentsKeepHistory", "onlyWhenConsumed"],
        );
    }
}

#[test]
fn should_refuse_it_on_an_index_only_type_on_both_paths() {
    // An indexOnly type has no stored row a reference could find and consume
    for full_validation in [true, false] {
        let schema = platform_value!({
            "type": "object",
            "indexOnly": true,
            "documentsMutable": false,
            "canBeDeleted": "onlyWhenConsumed",
            "properties": {
                "hashtag": { "type": "string", "maxLength": 63, "position": 0 },
            },
            "required": ["hashtag", "$createdAt"],
            "indices": [
                {
                    "name": "byHashtag",
                    "properties": [{ "hashtag": "asc" }, { "$createdAt": "asc" }],
                    "terminal": "$ownerId",
                },
                {
                    "name": "byOwner",
                    "properties": [{ "$ownerId": "asc" }],
                    "terminal": "hashtag",
                },
            ],
            "additionalProperties": false,
        });
        assert_refused_naming(
            parse(schema, full_validation),
            &["indexOnly", "onlyWhenConsumed"],
        );
    }
}

#[test]
fn should_refuse_the_value_before_protocol_version_14() {
    // Meta-schema v2 (protocol versions 12 and 13) admits only a boolean, and the generation 2
    // core reads `canBeDeleted` as one on the stored path too. The same schema with `false`
    // parses there on both paths, so the refusal is the value's.
    let platform_version = PlatformVersion::get(13).expect("expected platform version");
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available");
    for full_validation in [true, false] {
        parse_with_config(
            commitment_schema(platform_value!({ "canBeDeleted": false })),
            &config,
            13,
            full_validation,
        )
        .expect("a boolean canBeDeleted parses before protocol version 14");
        let error = parse_with_config(
            commitment_schema(platform_value!({})),
            &config,
            13,
            full_validation,
        )
        .expect_err("\"onlyWhenConsumed\" must be refused before protocol version 14");
        if full_validation {
            let message = format!("{error:?}");
            assert!(
                message.contains("canBeDeleted"),
                "the meta-schema must refuse canBeDeleted, got {message}"
            );
        }
    }
}
