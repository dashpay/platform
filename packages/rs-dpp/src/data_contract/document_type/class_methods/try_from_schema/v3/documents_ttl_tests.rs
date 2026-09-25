//! The `ttl` doctype keyword (protocol version 14): the platform deletes each document of
//! the type once `$createdAt` plus the time to live has passed. What it requires of the
//! document type, on both parse paths, and what it makes of the type's deletability.
use super::*;
use crate::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
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
        "note",
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

/// A two-week note: `$createdAt` required, one plain index.
fn note_schema(extra: Value) -> Value {
    let mut schema = platform_value!({
        "type": "object",
        "properties": {
            "text": { "type": "string", "maxLength": 50, "position": 0 },
        },
        "indices": [
            { "name": "byOwner", "properties": [{ "$ownerId": "asc" }] },
        ],
        "required": ["$createdAt"],
        "additionalProperties": false,
        "ttl": 1_209_600,
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
fn should_parse_the_time_to_live() {
    for full_validation in [true, false] {
        let document_type = parse(note_schema(platform_value!({})), full_validation)
            .expect("a two-week note parses");
        assert_eq!(document_type.documents_ttl_seconds(), Some(1_209_600));
    }
}

#[test]
fn should_leave_documents_without_a_time_to_live_by_default() {
    let document_type = parse(
        platform_value!({
            "type": "object",
            "properties": { "text": { "type": "string", "maxLength": 50, "position": 0 } },
            "additionalProperties": false,
        }),
        true,
    )
    .expect("parse");
    assert_eq!(document_type.documents_ttl_seconds(), None);
}

#[test]
fn should_make_a_type_whose_owners_can_not_delete_its_documents_disappear() {
    // `canBeDeleted: false` only stops the owner; the platform still deletes, so a
    // reference that must always resolve may not target the type.
    let document_type = parse(
        note_schema(platform_value!({ "canBeDeleted": false, "documentsMutable": false })),
        true,
    )
    .expect("parse");
    assert!(!document_type.documents_can_be_deleted());
    assert!(document_type.documents_can_disappear());

    let permanent = parse(
        platform_value!({
            "type": "object",
            "canBeDeleted": false,
            "properties": { "text": { "type": "string", "maxLength": 50, "position": 0 } },
            "additionalProperties": false,
        }),
        true,
    )
    .expect("parse");
    assert!(!permanent.documents_can_disappear());
}

#[test]
fn should_refuse_a_time_to_live_without_created_at_on_both_paths() {
    for full_validation in [true, false] {
        assert_refused_naming(
            parse(
                note_schema(platform_value!({ "required": ["$updatedAt"] })),
                full_validation,
            ),
            &["ttl", "$createdAt"],
        );
    }
}

#[test]
fn should_refuse_a_time_to_live_on_a_type_that_keeps_history_on_both_paths() {
    // `canBeDeleted: false` keeps the separate keep-history-and-deletable rule out of
    // the way, so the refusal is the time to live's own.
    for full_validation in [true, false] {
        assert_refused_naming(
            parse(
                note_schema(platform_value!({
                    "documentsKeepHistory": true,
                    "canBeDeleted": false,
                })),
                full_validation,
            ),
            &["documentsKeepHistory", "ttl"],
        );
    }
}

#[test]
fn should_refuse_a_time_to_live_on_a_type_with_a_contested_index_on_both_paths() {
    // A contested document waits in its vote poll until the poll awards it, keeping the
    // `$createdAt` of its create: it could expire before it is stored.
    for full_validation in [true, false] {
        let schema = platform_value!({
            "type": "object",
            "documentsMutable": false,
            "ttl": 86_400,
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
            "required": ["normalizedLabel", "$createdAt"],
            "additionalProperties": false,
        });
        assert_refused_naming(parse(schema, full_validation), &["contested index", "ttl"]);
    }
}

#[test]
fn should_refuse_a_time_to_live_on_an_index_only_type_on_both_paths() {
    for full_validation in [true, false] {
        let schema = platform_value!({
            "type": "object",
            "indexOnly": true,
            "documentsMutable": false,
            "ttl": 86_400,
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
        assert_refused_naming(parse(schema, full_validation), &["indexOnly", "ttl"]);
    }
}

#[test]
fn should_refuse_a_time_to_live_that_is_not_a_positive_number_of_seconds_on_both_paths() {
    // No meta-schema stands in front of a stored contract, and no keyword is read more
    // leniently there: the parser refuses the shape itself.
    for full_validation in [true, false] {
        for ttl in [
            platform_value!(0),
            platform_value!(-5),
            platform_value!(4294967296u64),
            platform_value!("two weeks"),
        ] {
            let result = parse(
                note_schema(platform_value!({ "ttl": ttl.clone() })),
                full_validation,
            );
            assert!(
                result.is_err(),
                "ttl {ttl:?} must be refused (full validation: {full_validation})"
            );
        }
    }
}

#[test]
fn should_cap_the_time_to_live_at_registration_only() {
    let max = PlatformVersion::latest()
        .system_limits
        .max_document_ttl_seconds
        .expect("protocol version 14 caps the time to live");
    let document_type =
        parse(note_schema(platform_value!({ "ttl": max })), true).expect("the cap itself parses");
    assert_eq!(document_type.documents_ttl_seconds(), Some(max));

    assert_refused_naming(
        parse(note_schema(platform_value!({ "ttl": max + 1 })), true),
        &["ttl", "longest time to live"],
    );
    // A stored contract is read back without the registration limits, like
    // `max_typed_array_items`: a lower cap in a later version must not brick it.
    let stored = parse(note_schema(platform_value!({ "ttl": max + 1 })), false)
        .expect("the stored path does not apply the cap");
    assert_eq!(stored.documents_ttl_seconds(), Some(max + 1));
}

#[test]
fn should_allow_a_time_to_live_with_every_owner_and_moderation_feature() {
    // Transfers and trades hand over what is left of a document's life; moderators delete
    // it early; a mutable type replaces it without moving its expiry.
    let document_type = parse(
        note_schema(platform_value!({
            "documentsMutable": true,
            "transferable": 1,
            "tradeMode": 1,
            "canBeDeleted": false,
        })),
        true,
    )
    .expect("parse");
    assert_eq!(document_type.documents_ttl_seconds(), Some(1_209_600));

    let platform_version = PlatformVersion::latest();
    let moderated = DataContractConfig::default_for_version(platform_version)
        .expect("default config available")
        .with_moderation(Some(ContractModerationConfig {
            banlist: false,
            suspensions: false,
            moderators: ContractModerators::ContractOwner,
            warnings: false,
        }));
    let document_type = parse_with_config(
        note_schema(platform_value!({
            "canBeDeletedByModerators": true,
            "canBeDeletedByModeratorsFor": 3600,
            "documentsMutable": false,
        })),
        &moderated,
        platform_version.protocol_version,
        true,
    )
    .expect("parse");
    assert!(document_type.documents_can_be_deleted_by_moderators());
    assert_eq!(document_type.documents_ttl_seconds(), Some(1_209_600));
}

#[test]
fn should_refuse_the_keyword_before_protocol_version_14() {
    // Meta-schema v2 (protocol versions 12 and 13) does not know the keyword, and the
    // generation 2 parser ignores it on the stored path, where no such contract can exist.
    let platform_version = PlatformVersion::get(13).expect("expected platform version");
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available");
    let result = parse_with_config(note_schema(platform_value!({})), &config, 13, true);
    assert!(result.is_err(), "the keyword must not pass meta-schema v2");
    let stored = parse_with_config(note_schema(platform_value!({})), &config, 13, false)
        .expect("generation 2 ignores the doctype keyword it predates");
    assert_eq!(stored.documents_ttl_seconds(), None);
}
