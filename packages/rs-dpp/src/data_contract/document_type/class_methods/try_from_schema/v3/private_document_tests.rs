//! The `private` doctype keyword (protocol version 15): a private document type, whose
//! documents are fixed-size encrypted entries the chain never reads. What the parse holds such
//! a type to on both paths, the entry size limit at registration, the protocol version gate,
//! the stored form, contract updates, and the references of the same contract that may not
//! name one.

use crate::consensus::basic::BasicError;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::config::moderation::{
    ContractModerationConfig, ContractModerators, ElectedModerators, InterimModerators,
    ModerationAbility, DEFAULT_ELECTION_WINDOW_SECONDS,
};
use crate::data_contract::config::v0::DataContractConfigSettersV0;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use crate::data_contract::document_type::schema::validation_schema::{
    AnnotationKeyword, AnyKeyword, ArrayKeyword, FormatKeyword, NumberKeyword, ObjectKeyword,
    RelationshipKeyword, ResolutionKeyword, SchemaKeyword, StorageKeyword, StringKeyword,
};
use crate::data_contract::document_type::serialized_version::DocumentTypeInSerializationFormat;
use crate::data_contract::document_type::{DocumentType, PrivateDocumentConfig};
use crate::data_contract::errors::DataContractError;
use crate::data_contract::DataContract;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::string_encoding::Encoding;
use platform_value::{platform_value, Identifier, Value};
use platform_version::version::v14::PROTOCOL_VERSION_14;
use platform_version::version::PlatformVersion;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

const CONTRACT_ID: [u8; 32] = [7; 32];

/// What a type declaring `{ "entrySize": entry_size }` parses to.
fn private_config(entry_size: u32) -> PrivateDocumentConfig {
    PrivateDocumentConfig::new(NonZeroU32::new(entry_size).expect("a test entry size is not 0"))
}

fn parse_on(
    schema: Value,
    config: &DataContractConfig,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<DocumentType, ProtocolError> {
    DocumentType::try_from_schema(
        Identifier::new(CONTRACT_ID),
        1,
        config.version(),
        "contactRequest",
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
        .expect("expected a default config");
    parse_on(schema, &config, full_validation, platform_version)
}

/// A private contact request of 704-byte entries, its plaintext an account reference and a
/// 96-byte key, with `extra` merged in at the top level.
fn private_schema(extra: Value) -> Value {
    let mut schema = platform_value!({
        "type": "object",
        "private": { "entrySize": 704 },
        "documentsMutable": false,
        "canBeDeleted": false,
        "properties": {
            "accountReference": {
                "type": "integer",
                "minimum": 0,
                "maximum": 4_294_967_295u64,
                "position": 0
            },
            "encryptedPublicKey": {
                "type": "array",
                "byteArray": true,
                "minItems": 96,
                "maxItems": 96,
                "position": 1
            },
        },
        "required": ["accountReference", "encryptedPublicKey"],
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

/// `schema` with `property` added to its `properties` under `name`.
fn with_property(mut schema: Value, name: &str, property: Value) -> Value {
    if let Value::Map(schema_map) = &mut schema {
        for (key, value) in schema_map.iter_mut() {
            if key.as_text() == Some("properties") {
                if let Value::Map(properties) = value {
                    properties.push((Value::Text(name.to_string()), property.clone()));
                }
            }
        }
    }
    schema
}

/// `schema` without its top-level `key`.
fn without(mut schema: Value, key: &str) -> Value {
    if let Value::Map(schema_map) = &mut schema {
        schema_map.retain(|(existing, _)| existing.as_text() != Some(key));
    }
    schema
}

/// Whether `error` is the consensus `InvalidContractStructure` every refusal here gives. A
/// paid refusal needs the consensus variant: the data contract error variant would surface as
/// an internal error in a block.
fn is_structure_error(error: &ProtocolError) -> bool {
    matches!(
        error,
        ProtocolError::ConsensusError(error)
            if matches!(
                **error,
                ConsensusError::BasicError(BasicError::ContractError(
                    DataContractError::InvalidContractStructure(_)
                ))
            )
    )
}

fn assert_refused_naming(result: Result<DocumentType, ProtocolError>, fragments: &[&str]) {
    let error = result.expect_err("the document type must be refused");
    assert!(
        is_structure_error(&error),
        "expected InvalidContractStructure, got {error:?}"
    );
    let message = format!("{error:?}");
    for fragment in fragments {
        assert!(
            message.contains(fragment),
            "error must name {fragment}, got {message}"
        );
    }
}

fn is_json_schema_error(result: &Result<DocumentType, ProtocolError>) -> bool {
    matches!(
        result,
        Err(ProtocolError::ConsensusError(error))
            if matches!(**error, ConsensusError::BasicError(BasicError::JsonSchemaError(_)))
    )
}

#[test]
fn should_parse_a_private_document_type_on_both_paths() {
    for full_validation in [true, false] {
        let document_type = parse(private_schema(platform_value!({})), full_validation)
            .expect("a private contact request parses");
        assert_eq!(document_type.private_document(), Some(private_config(704)));
        assert!(document_type.is_private());
        assert!(document_type.indexes().is_empty());
        assert!(!document_type.documents_mutable());
        assert!(!document_type.documents_can_be_deleted());
        // Nothing can make an entry leave state
        assert!(!document_type.documents_can_disappear());
    }
}

#[test]
fn should_leave_every_other_type_public() {
    let document_type = parse(
        without(private_schema(platform_value!({})), "private"),
        true,
    )
    .expect("the same type without `private` parses");
    assert_eq!(document_type.private_document(), None);
    assert!(!document_type.is_private());
}

#[test]
fn should_admit_the_bounded_key_keywords_and_the_plaintext_schema_keywords() {
    parse(
        private_schema(platform_value!({
            "requiresIdentityEncryptionBoundedKey": 2,
            "requiresIdentityDecryptionBoundedKey": 2,
            "documentsKeepHistory": false,
            "minProperties": 2,
            "maxProperties": 2,
            "description": "a contact request only its recipient can read",
            "$comment": "DashPay",
        })),
        true,
    )
    .expect("a private type may declare what describes its plaintext and its keys");
}

/// Before protocol version 15 the keyword does not exist: the meta-schema refuses it at
/// registration, and the stored path (a contract read back from state, check_tx, a client)
/// ignores it exactly as it did before, parsing the same type as without it.
#[test]
fn should_ignore_private_before_protocol_version_15() {
    let platform_version =
        PlatformVersion::get(PROTOCOL_VERSION_14).expect("expected protocol version 14");
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("expected a default config");

    let refused = parse_on(
        private_schema(platform_value!({})),
        &config,
        true,
        platform_version,
    );
    assert!(
        is_json_schema_error(&refused),
        "the meta-schema refuses `private` at registration, got {refused:?}"
    );

    let with_private = parse_on(
        private_schema(platform_value!({ "indices": [
            { "name": "byOwner", "properties": [{ "$ownerId": "asc" }] }
        ] })),
        &config,
        false,
        platform_version,
    )
    .expect("the stored path ignores the keyword");
    let without_private = parse_on(
        without(
            private_schema(platform_value!({ "indices": [
                { "name": "byOwner", "properties": [{ "$ownerId": "asc" }] }
            ] })),
            "private",
        ),
        &config,
        false,
        platform_version,
    )
    .expect("the same type without `private` parses");
    assert_eq!(with_private.private_document(), None);
    assert_eq!(with_private.indexes(), without_private.indexes());
    assert_eq!(
        with_private.flattened_properties(),
        without_private.flattened_properties()
    );
}

#[test]
fn should_refuse_a_malformed_declaration_on_both_paths() {
    for declaration in [
        Value::Null,
        platform_value!(704),
        platform_value!({}),
        platform_value!({ "entrySize": 0 }),
        platform_value!({ "entrySize": -1 }),
        platform_value!({ "entrySize": "704" }),
        platform_value!({ "entrySize": 4_294_967_296u64 }),
        platform_value!({ "entrySize": 704, "constraints": [] }),
    ] {
        for full_validation in [true, false] {
            let result = parse(
                private_schema(platform_value!({ "private": declaration.clone() })),
                full_validation,
            );
            if full_validation {
                // The native schema check reports the shape, as the meta-schema check would
                assert!(
                    is_json_schema_error(&result),
                    "{declaration:?} must be refused by the schema check, got {result:?}"
                );
            } else {
                assert_refused_naming(result, &["private", "entrySize"]);
            }
        }
    }
}

#[test]
fn should_hold_the_entry_size_to_the_limit_at_registration_only() {
    let max_entry_size = PlatformVersion::latest()
        .system_limits
        .max_private_document_entry_size
        .expect("expected protocol version 15 to set the limit");

    parse(
        private_schema(platform_value!({ "private": { "entrySize": max_entry_size } })),
        true,
    )
    .expect("the largest entry is admitted");
    assert_refused_naming(
        parse(
            private_schema(platform_value!({ "private": { "entrySize": max_entry_size + 1 } })),
            true,
        ),
        &[
            &format!("entrySize: {}", max_entry_size + 1),
            &format!("{max_entry_size} bytes"),
        ],
    );
    // A contract read back from state passed the limit when it was registered
    let stored = parse(
        private_schema(platform_value!({ "private": { "entrySize": max_entry_size + 1 } })),
        false,
    )
    .expect("the stored path does not check the limit again");
    assert_eq!(
        stored.private_document(),
        Some(private_config(max_entry_size + 1))
    );
}

#[test]
fn should_refuse_every_keyword_a_private_type_may_not_declare() {
    for (keyword, value) in [
        (
            "indices",
            platform_value!([{ "name": "byOwner", "properties": [{ "$ownerId": "asc" }] }]),
        ),
        ("documentsCountable", platform_value!(true)),
        ("indexOnly", platform_value!(false)),
        ("transferable", platform_value!(0)),
        ("tradeMode", platform_value!(0)),
        ("creationRestrictionMode", platform_value!(0)),
        ("signatureSecurityLevelRequirement", platform_value!(2)),
        ("keepsTransferHistory", platform_value!(false)),
        ("ttl", platform_value!(3600)),
    ] {
        for full_validation in [true, false] {
            let mut extra = BTreeMap::new();
            extra.insert(keyword.to_string(), value.clone());
            assert_refused_naming(
                parse(private_schema(Value::from(extra)), full_validation),
                &[&format!("declares `{keyword}`")],
            );
        }
    }
}

#[test]
fn should_refuse_documents_left_mutable_deletable_or_keeping_history() {
    for full_validation in [true, false] {
        // The default config leaves documents mutable and deletable where a type says nothing
        assert_refused_naming(
            parse(
                without(private_schema(platform_value!({})), "documentsMutable"),
                full_validation,
            ),
            &["documentsMutable: false"],
        );
        assert_refused_naming(
            parse(
                without(private_schema(platform_value!({})), "canBeDeleted"),
                full_validation,
            ),
            &["canBeDeleted: false"],
        );
        assert_refused_naming(
            parse(
                private_schema(platform_value!({ "canBeDeleted": "onlyWhenConsumed" })),
                full_validation,
            ),
            &["canBeDeleted: false"],
        );
        assert_refused_naming(
            parse(
                private_schema(platform_value!({ "documentsKeepHistory": true })),
                full_validation,
            ),
            &["documentsKeepHistory: false"],
        );
    }

    // A contract whose defaults already say so needs neither written out
    let platform_version = PlatformVersion::latest();
    let mut config = DataContractConfig::default_for_version(platform_version)
        .expect("expected a default config");
    config.set_documents_mutable_contract_default(false);
    config.set_documents_can_be_deleted_contract_default(false);
    parse_on(
        without(
            without(private_schema(platform_value!({})), "documentsMutable"),
            "canBeDeleted",
        ),
        &config,
        true,
        platform_version,
    )
    .expect("the contract's defaults make the documents immutable and undeletable");
}

#[test]
fn should_refuse_system_fields() {
    for full_validation in [true, false] {
        assert_refused_naming(
            parse(
                private_schema(platform_value!({
                    "required": ["accountReference", "encryptedPublicKey", "$createdAt"]
                })),
                full_validation,
            ),
            &["system field `$createdAt`"],
        );
    }
}

#[test]
fn should_refuse_property_keywords_the_chain_checks_on_each_document() {
    let identifier = |extra: Value| {
        let mut property = platform_value!({
            "type": "array",
            "byteArray": true,
            "minItems": 32,
            "maxItems": 32,
            "contentMediaType": "application/x.dash.dpp.identifier",
            "position": 2
        });
        if let (Value::Map(map), Value::Map(extra)) = (&mut property, extra) {
            map.extend(extra);
        }
        property
    };
    for (property, fragment) in [
        (
            identifier(platform_value!({ "refersTo": { "type": "identity" } })),
            "refersTo",
        ),
        (
            identifier(platform_value!({ "distinctFrom": "$ownerId" })),
            "distinctFrom",
        ),
    ] {
        let schema = with_property(private_schema(platform_value!({})), "recipientId", property);
        for full_validation in [true, false] {
            assert_refused_naming(parse(schema.clone(), full_validation), &[fragment]);
        }
    }
}

/// The other keywords the chain checks on each document it writes, each on a property of the
/// plaintext it is otherwise valid on.
#[test]
fn should_refuse_generated_encrypted_and_required_since_properties() {
    let string = |position: u32| platform_value!({ "type": "string", "maxLength": 63, "position": position });
    let key_id = |position: u32| {
        platform_value!({
            "type": "integer",
            "minimum": 0,
            "maximum": 4_294_967_295u64,
            "position": position
        })
    };
    let identifier = platform_value!({
        "type": "array",
        "byteArray": true,
        "minItems": 32,
        "maxItems": 32,
        "contentMediaType": "application/x.dash.dpp.identifier",
        "position": 2
    });
    let generated = with_property(
        with_property(private_schema(platform_value!({})), "label", string(2)),
        "slug",
        platform_value!({
            "type": "string",
            "maxLength": 63,
            "generatedFrom": {
                "function": "sys.stringTransformations.lowercase",
                "params": ["label"]
            },
            "position": 3
        }),
    );
    let mut encrypted = with_property(
        private_schema(platform_value!({})),
        "recipientId",
        identifier,
    );
    encrypted = with_property(encrypted, "recipientKeyId", key_id(3));
    encrypted = with_property(encrypted, "senderKeyId", key_id(4));
    encrypted = with_property(
        encrypted,
        "message",
        platform_value!({
            "type": "array",
            "byteArray": true,
            "maxItems": 64,
            "encryptedFor": {
                "recipient": "recipientId",
                "recipientKey": "recipientKeyId",
                "senderKey": "senderKeyId",
                "scheme": "ecdh-secp256k1-aes256-cbc"
            },
            "position": 5
        }),
    );
    let mut tag = key_id(2);
    if let Value::Map(tag_map) = &mut tag {
        tag_map.push((Value::Text("requiredSince".to_string()), Value::U32(1)));
    }
    let required_since = with_property(
        private_schema(platform_value!({
            "required": ["accountReference", "encryptedPublicKey", "tag"]
        })),
        "tag",
        tag,
    );
    for (schema, fragment) in [
        (generated, "`generatedFrom` on property"),
        (encrypted, "`encryptedFor` on property"),
        (required_since, "`requiredSince` on property"),
    ] {
        for full_validation in [true, false] {
            assert_refused_naming(parse(schema.clone(), full_validation), &[fragment]);
        }
    }
}

/// `requiredSince` sits on a top-level property, an object included, which the leaves of
/// the type's flattened properties do not list.
#[test]
fn should_refuse_required_since_on_an_object_property() {
    let schema = with_property(
        private_schema(platform_value!({
            "required": ["accountReference", "encryptedPublicKey", "meta"]
        })),
        "meta",
        platform_value!({
            "type": "object",
            "properties": {
                "tag": { "type": "integer", "minimum": 0, "maximum": 255, "position": 0 }
            },
            "required": ["tag"],
            "additionalProperties": false,
            "requiredSince": 1,
            "position": 2
        }),
    );
    for full_validation in [true, false] {
        assert_refused_naming(
            parse(schema.clone(), full_validation),
            &["`requiredSince` on property", "meta"],
        );
    }
}

#[test]
fn should_refuse_a_system_field_named_in_dependent_required() {
    for dependent_required in [
        platform_value!({ "accountReference": ["$createdAt"] }),
        platform_value!({ "$createdAt": ["accountReference"] }),
    ] {
        for full_validation in [true, false] {
            assert_refused_naming(
                parse(
                    private_schema(platform_value!({
                        "dependentRequired": dependent_required.clone()
                    })),
                    full_validation,
                ),
                &["system field `$createdAt`"],
            );
        }
    }
    parse(
        private_schema(platform_value!({
            "dependentRequired": { "accountReference": ["encryptedPublicKey"] }
        })),
        true,
    )
    .expect("dependentRequired naming the plaintext's own properties is admitted");
}

/// Declared twice, the type would be private to readers keeping the last entry (the native
/// check, JSON readers) and public to one reading the first: refused whatever the entries say.
#[test]
fn should_refuse_private_declared_more_than_once() {
    for (first, last) in [
        (Value::Null, platform_value!({ "entrySize": 704 })),
        (
            platform_value!({ "entrySize": 704 }),
            platform_value!({ "entrySize": 1 }),
        ),
        (
            platform_value!({ "entrySize": 704 }),
            platform_value!({ "entrySize": 704 }),
        ),
    ] {
        let mut schema = without(private_schema(platform_value!({})), "private");
        if let Value::Map(schema_map) = &mut schema {
            schema_map.insert(0, (Value::Text("private".to_string()), first.clone()));
            schema_map.push((Value::Text("private".to_string()), last.clone()));
        }
        for full_validation in [true, false] {
            assert_refused_naming(
                parse(schema.clone(), full_validation),
                &["declares `private` more than once"],
            );
        }
    }
}

/// The config of a contract whose elected moderators moderate `moderated`.
fn elected_config(moderated: &str, platform_version: &PlatformVersion) -> DataContractConfig {
    DataContractConfig::default_for_version(platform_version)
        .expect("expected a default config")
        .with_moderation(Some(ContractModerationConfig {
            banlist: true,
            suspensions: false,
            warnings: false,
            moderators: ContractModerators::Elected(Box::new(ElectedModerators {
                join_window: DEFAULT_ELECTION_WINDOW_SECONDS,
                vote_window: DEFAULT_ELECTION_WINDOW_SECONDS,
                challenge_cool_down: None,
                election_delay: None,
                max_added_moderators: 0,
                moderated_document_types: BTreeMap::from([(
                    moderated.to_string(),
                    BTreeSet::from([ModerationAbility::Ban]),
                )]),
                interim: InterimModerators::ContractOwner,
                owner_protected: false,
            })),
        }))
}

#[test]
fn should_refuse_a_private_type_its_elected_moderators_moderate() {
    let platform_version = PlatformVersion::latest();
    for full_validation in [true, false] {
        assert_refused_naming(
            parse_on(
                private_schema(platform_value!({})),
                &elected_config("contactRequest", platform_version),
                full_validation,
                platform_version,
            ),
            &["elected moderators"],
        );
        parse_on(
            private_schema(platform_value!({})),
            &elected_config("post", platform_version),
            full_validation,
            platform_version,
        )
        .expect("a private type the moderators leave alone parses");
    }
}

/// A table selecting `parse_private_document` without the entry size limit is a fault of the
/// build, which no contract is charged for.
#[test]
fn should_fail_as_a_fault_of_the_build_where_no_entry_size_limit_is_set() {
    let mut platform_version = PlatformVersion::latest().clone();
    platform_version
        .system_limits
        .max_private_document_entry_size = None;
    let config = DataContractConfig::default_for_version(&platform_version)
        .expect("expected a default config");
    let error = parse_on(
        private_schema(platform_value!({})),
        &config,
        true,
        &platform_version,
    )
    .expect_err("registration can not check the entry size");
    assert!(
        matches!(error, ProtocolError::CorruptedCodeExecution(_)),
        "expected an internal error, got {error:?}"
    );
    parse_on(
        private_schema(platform_value!({})),
        &config,
        false,
        &platform_version,
    )
    .expect("the stored path does not read the limit");
}

/// A document type's own `$defs` is refused on every type, so the keywords a private type is
/// told it may declare do not include it.
#[test]
fn should_not_offer_defs_among_the_keywords_a_private_type_declares() {
    let error = parse(private_schema(platform_value!({ "ttl": 3600 })), false)
        .expect_err("a private type may not declare `ttl`");
    let message = format!("{error:?}");
    assert!(message.contains("declares only"), "{message}");
    assert!(!message.contains("$defs"), "{message}");
}

/// A private type's schema may name no doctype-level key twice: the parse reads the first
/// entry where the native check and JSON readers keep the last, so the type could hide what
/// its refusals look for.
#[test]
fn should_refuse_a_doctype_key_a_private_type_repeats() {
    for (key, first, last) in [
        (
            "properties",
            platform_value!({ "accountReference": {
                "type": "integer", "minimum": 0, "maximum": 4_294_967_295u64, "position": 0
            } }),
            platform_value!({
                "accountReference": {
                    "type": "integer", "minimum": 0, "maximum": 4_294_967_295u64, "position": 0
                },
                "$createdAt": { "type": "integer", "minimum": 0 }
            }),
        ),
        (
            "documentsMutable",
            platform_value!(false),
            platform_value!(true),
        ),
    ] {
        let mut schema = private_schema(platform_value!({}));
        if let Value::Map(schema_map) = &mut schema {
            if key == "properties" {
                schema_map.retain(|(existing, _)| existing.as_text() != Some(key));
                schema_map.push((Value::Text(key.to_string()), first.clone()));
            } else {
                schema_map.push((Value::Text(key.to_string()), first.clone()));
            }
            schema_map.push((Value::Text(key.to_string()), last.clone()));
        }
        for full_validation in [true, false] {
            assert_refused_naming(
                parse(schema.clone(), full_validation),
                &[&format!("declares `{key}` more than once")],
            );
        }
    }
}

/// A keyword a private type may not declare is refused as such before any stage reads it,
/// whatever its value, rather than by the stage that would read it.
#[test]
fn should_refuse_a_malformed_keyword_a_private_type_may_not_declare_as_such() {
    for (keyword, value) in [
        (
            "indices",
            platform_value!([{ "name": "byGhost", "properties": [{ "ghost": "asc" }] }]),
        ),
        ("actionFees", platform_value!(5)),
        ("ttl", platform_value!("an hour")),
    ] {
        for full_validation in [true, false] {
            let mut extra = BTreeMap::new();
            extra.insert(keyword.to_string(), value.clone());
            assert_refused_naming(
                parse(private_schema(Value::from(extra)), full_validation),
                &[&format!("declares `{keyword}`")],
            );
        }
    }
}

/// What the native schema check admits and a u32 read refuses, an `entrySize` written as a
/// float or repeated inside the declaration, is refused by the parse on both paths.
#[test]
fn should_refuse_an_entry_size_written_as_a_float_or_repeated() {
    let float = platform_value!({ "entrySize": 704.0 });
    let repeated = Value::Map(vec![
        (Value::Text("entrySize".to_string()), Value::U32(704)),
        (Value::Text("entrySize".to_string()), Value::U32(512)),
    ]);
    for declaration in [float, repeated] {
        for full_validation in [true, false] {
            assert_refused_naming(
                parse(
                    private_schema(platform_value!({ "private": declaration.clone() })),
                    full_validation,
                ),
                &["private", "entrySize"],
            );
        }
    }
}

/// Whether a private type refuses a property keyword: those the chain checks on each document
/// it writes, against more than the value. Exhaustive, so that a new keyword fails to compile
/// here until someone decides what it means on a private type, whose `properties` describe a
/// plaintext the chain never reads.
fn refused_on_a_private_type(keyword: SchemaKeyword) -> bool {
    match keyword {
        SchemaKeyword::Relationship(keyword) => match keyword {
            RelationshipKeyword::RefersTo
            | RelationshipKeyword::DistinctFrom
            | RelationshipKeyword::GeneratedFrom => true,
        },
        SchemaKeyword::Format(keyword) => match keyword {
            FormatKeyword::EncryptedFor => true,
            FormatKeyword::Format => false,
        },
        SchemaKeyword::Storage(keyword) => match keyword {
            StorageKeyword::RequiredSince => true,
            StorageKeyword::Position | StorageKeyword::ContentMediaType => false,
        },
        SchemaKeyword::Any(keyword) => match keyword {
            AnyKeyword::Ref | AnyKeyword::Type | AnyKeyword::Enum | AnyKeyword::Const => false,
        },
        SchemaKeyword::Object(keyword) => match keyword {
            ObjectKeyword::Properties
            | ObjectKeyword::AdditionalProperties
            | ObjectKeyword::Required
            | ObjectKeyword::MinProperties
            | ObjectKeyword::MaxProperties
            | ObjectKeyword::DependentRequired => false,
        },
        SchemaKeyword::Array(keyword) => match keyword {
            ArrayKeyword::Items
            | ArrayKeyword::Contains
            | ArrayKeyword::MinItems
            | ArrayKeyword::MaxItems
            | ArrayKeyword::UniqueItems
            | ArrayKeyword::ByteArray => false,
        },
        SchemaKeyword::String(keyword) => match keyword {
            StringKeyword::MinLength
            | StringKeyword::MaxLength
            | StringKeyword::MaxBytes
            | StringKeyword::Pattern => false,
        },
        SchemaKeyword::Number(keyword) => match keyword {
            NumberKeyword::Minimum
            | NumberKeyword::Maximum
            | NumberKeyword::ExclusiveMinimum
            | NumberKeyword::ExclusiveMaximum
            | NumberKeyword::MultipleOf => false,
        },
        SchemaKeyword::Resolution(keyword) => match keyword {
            ResolutionKeyword::Id => false,
        },
        SchemaKeyword::Annotation(keyword) => match keyword {
            AnnotationKeyword::Comment
            | AnnotationKeyword::Description
            | AnnotationKeyword::Examples => false,
        },
    }
}

/// The property keywords a private type refuses are exactly those the parse refuses
/// (`first_property_checked_on_write` and the type's reference declarations), each covered by
/// `should_refuse_property_keywords_the_chain_checks_on_each_document` and
/// `should_refuse_generated_encrypted_and_required_since_properties`.
#[test]
fn should_decide_every_property_keyword_on_a_private_type() {
    let refused: BTreeSet<&str> = SchemaKeyword::iter()
        .filter(|keyword| refused_on_a_private_type(*keyword))
        .map(SchemaKeyword::name)
        .collect();
    assert_eq!(
        refused,
        BTreeSet::from([
            "refersTo",
            "distinctFrom",
            "generatedFrom",
            "encryptedFor",
            "requiredSince",
        ])
    );
}

/// The compiled stored form of protocol version 15 keeps the declaration, so a private type
/// read back from state without its schema being parsed is still private.
#[test]
fn should_keep_the_declaration_in_the_stored_form() {
    let platform_version = PlatformVersion::latest();
    let schema = private_schema(platform_value!({}));
    let document_type = parse(schema.clone(), true).expect("parses");
    let read_back = DocumentTypeInSerializationFormat::try_from_document_type(
        document_type.as_ref(),
        None,
        platform_version,
    )
    .expect("expected the type to compile")
    .into_document_type("contactRequest", Identifier::new(CONTRACT_ID), schema)
    .expect("expected the type to read back");
    assert_eq!(read_back.private_document(), Some(private_config(704)));
    assert_eq!(read_back, document_type);
}

#[test]
fn should_refuse_an_update_changing_whether_a_type_is_private_or_its_entry_size() {
    let platform_version = PlatformVersion::latest();
    let private = parse(private_schema(platform_value!({})), false).expect("parses");
    let larger = parse(
        private_schema(platform_value!({ "private": { "entrySize": 1024 } })),
        false,
    )
    .expect("parses");
    let public = parse(
        without(private_schema(platform_value!({})), "private"),
        false,
    )
    .expect("parses");

    for (old, new, fragment) in [
        (
            &private,
            &larger,
            "private with 704-byte entries to private with 1024-byte",
        ),
        (
            &private,
            &public,
            "private with 704-byte entries to not private",
        ),
        (
            &public,
            &private,
            "not private to private with 704-byte entries",
        ),
    ] {
        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, platform_version)
            .expect("validate_update should not error");
        assert!(
            result.errors.iter().any(|error| matches!(
                error,
                ConsensusError::StateError(StateError::DocumentTypeUpdateError(error))
                    if error.additional_message().contains(fragment)
            )),
            "expected a document type update error naming {fragment}, got {:?}",
            result.errors
        );
    }

    let unchanged = old_and_new_unchanged(&private, platform_version);
    assert!(unchanged.is_valid(), "{:?}", unchanged.errors);
}

fn old_and_new_unchanged(
    document_type: &DocumentType,
    platform_version: &PlatformVersion,
) -> SimpleConsensusValidationResult {
    document_type
        .as_ref()
        .validate_update(document_type.as_ref(), 2, platform_version)
        .expect("validate_update should not error")
}

// ---- references of the same contract ----

/// A contract with the private `secret` type and a public `pointer` type whose `secretId`
/// declares `refers_to`, and whose rules are `rules` when given.
fn pointer_contract(
    refers_to: Option<serde_json::Value>,
    rules: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut secret_id = json!({
        "type": "array",
        "byteArray": true,
        "minItems": 32,
        "maxItems": 32,
        "contentMediaType": "application/x.dash.dpp.identifier",
        "position": 0
    });
    if let Some(refers_to) = refers_to {
        secret_id["refersTo"] = refers_to;
    }
    let mut pointer = json!({
        "type": "object",
        "properties": { "secretId": secret_id },
        "additionalProperties": false
    });
    if let Some(rules) = rules {
        pointer["propertyConstraints"] = rules;
    }
    json!({
        "$formatVersion": "1",
        "id": Identifier::from(CONTRACT_ID).to_string(Encoding::Base58),
        "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
        "version": 1,
        "documentSchemas": {
            "secret": {
                "type": "object",
                "private": { "entrySize": 512 },
                "documentsMutable": false,
                "canBeDeleted": false,
                "properties": {
                    "note": { "type": "string", "maxLength": 63, "position": 0 }
                },
                "additionalProperties": false
            },
            "pointer": pointer
        }
    })
}

fn contract_on(
    contract: serde_json::Value,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<DataContract, ProtocolError> {
    let value = platform_value::to_value(contract).expect("the contract should convert");
    DataContract::from_value(value, full_validation, platform_version)
}

fn contract(
    contract: serde_json::Value,
    full_validation: bool,
) -> Result<DataContract, ProtocolError> {
    contract_on(contract, full_validation, PlatformVersion::latest())
}

#[test]
fn should_register_a_contract_with_a_private_type() {
    let registered = contract(pointer_contract(None, None), true).expect("registers");
    let secret = registered
        .document_type_for_name("secret")
        .expect("expected the secret type");
    assert_eq!(secret.private_document(), Some(private_config(512)));
}

/// Whatever the kind: a `deletableDocument` lookup is judged against a private type too,
/// rather than left to registration as a lookup into a type of another kind is. The lookups
/// of the same contract are checked under full validation only, as every one is.
#[test]
fn should_refuse_a_find_by_of_any_kind_into_a_private_type() {
    for kind in ["permanentDocument", "deletableDocument"] {
        assert_refused_contract(
            contract(
                pointer_contract(
                    Some(json!({
                        "type": kind,
                        "documentType": "secret",
                        "findBy": { "$ownerId": "." }
                    })),
                    None,
                ),
                true,
            ),
            "is a private document type, which findBy cannot reference",
        );
    }
}

#[test]
fn should_refuse_a_count_of_a_private_type() {
    assert_refused_contract(
        contract(
            pointer_contract(
                None,
                Some(json!({
                    "fewSecrets": { "lessThan": [{ "countOf": ["secret"] }, 100] }
                })),
            ),
            true,
        ),
        "a private type, whose encrypted entries a countOf or sumOf does not total",
    );
}

#[test]
fn should_refuse_an_in_list_into_a_private_type() {
    let mut pointer = pointer_contract(
        Some(json!({
            "type": "permanentDocument",
            "documentType": "secret",
            "findBy": { "$id": "listId" },
            "inList": "note"
        })),
        None,
    );
    pointer["documentSchemas"]["pointer"]["properties"]["listId"] = json!({
        "type": "array",
        "byteArray": true,
        "minItems": 32,
        "maxItems": 32,
        "contentMediaType": "application/x.dash.dpp.identifier",
        "position": 1
    });
    assert_refused_contract(
        contract(pointer, true),
        "is a private document type, whose entries the chain never reads",
    );
}

#[test]
fn should_refuse_a_sum_of_a_private_type() {
    let mut pointer = pointer_contract(
        None,
        Some(json!({
            "fewSecrets": { "lessThan": [{ "sumOf": ["secret", "amount"] }, 100] }
        })),
    );
    pointer["documentSchemas"]["secret"]["properties"]["amount"] = json!({
        "type": "integer",
        "minimum": 0,
        "maximum": 1000,
        "position": 1
    });
    assert_refused_contract(
        contract(pointer, true),
        "a private type, whose encrypted entries a countOf or sumOf does not total",
    );
}

fn assert_refused_contract(result: Result<DataContract, ProtocolError>, fragment: &str) {
    let Err(error) = result else {
        panic!("the contract must be refused: {fragment}");
    };
    assert!(
        is_structure_error(&error),
        "expected InvalidContractStructure, got {error:?}"
    );
    let message = format!("{error:?}");
    assert!(
        message.contains(fragment),
        "error must say {fragment}, got {message}"
    );
}

// ---- protocol version 14 ----

/// What a contract parsed at protocol version 14 comes to: its `pointer` type, or the refusal.
fn pointer_outcome(result: Result<DataContract, ProtocolError>) -> Result<DocumentType, String> {
    result
        .map(|contract| {
            contract
                .document_type_cloned_for_name("pointer")
                .expect("expected the pointer type")
        })
        .map_err(|error| format!("{error:?}"))
}

fn is_json_schema_refusal(result: &Result<DataContract, ProtocolError>) -> bool {
    matches!(
        result,
        Err(ProtocolError::ConsensusError(error))
            if matches!(**error, ConsensusError::BasicError(BasicError::JsonSchemaError(_)))
    )
}

/// The shipped code this keyword changes in place never meets a private type at protocol
/// version 14: the lookups, lists and aggregates the contract parse checks across its types
/// (`create_document_types_from_document_schemas` 1, `DocumentReferenceLookup`,
/// `ListElementReference`, the aggregate rules) run under full validation, after each type's
/// own parse, where meta-schema v3 refuses `private`. The stored path parses the same
/// contract with the key as without it, and the parsed side of `validate_update` sees no type
/// turn private.
#[test]
fn should_judge_a_contract_at_protocol_version_14_the_same_with_private_as_without() {
    let platform_version =
        PlatformVersion::get(PROTOCOL_VERSION_14).expect("expected protocol version 14");

    for (refers_to, rules) in [
        (
            Some(json!({
                "type": "permanentDocument",
                "documentType": "secret",
                "findBy": { "$ownerId": "." }
            })),
            None,
        ),
        (
            Some(json!({
                "type": "deletableDocument",
                "documentType": "secret",
                "findBy": { "$ownerId": "." }
            })),
            None,
        ),
        (
            Some(json!({
                "type": "permanentDocument",
                "documentType": "secret",
                "findBy": { "$id": "listId" },
                "inList": "note"
            })),
            None,
        ),
        (
            None,
            Some(json!({
                "fewSecrets": { "lessThan": [{ "countOf": ["secret"] }, 100] }
            })),
        ),
    ] {
        let mut with_private = pointer_contract(refers_to.clone(), rules.clone());
        with_private["documentSchemas"]["pointer"]["properties"]["listId"] = json!({
            "type": "array",
            "byteArray": true,
            "minItems": 32,
            "maxItems": 32,
            "contentMediaType": "application/x.dash.dpp.identifier",
            "position": 1
        });
        let mut without_private = with_private.clone();
        without_private["documentSchemas"]["secret"]
            .as_object_mut()
            .expect("the secret schema is an object")
            .remove("private");
        let registered = contract_on(with_private.clone(), true, platform_version);
        assert!(
            is_json_schema_refusal(&registered),
            "meta-schema v3 refuses `private` before {refers_to:?} {rules:?} is judged, got \
             {registered:?}"
        );
        assert_eq!(
            pointer_outcome(contract_on(with_private, false, platform_version)),
            pointer_outcome(contract_on(without_private, false, platform_version)),
            "{refers_to:?} {rules:?}"
        );
    }

    // An update adding the key: both sides parse public, so `validate_update` finds no change
    // of whether the type is private
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("expected a default config");
    let public = parse_on(
        without(private_schema(platform_value!({})), "private"),
        &config,
        false,
        platform_version,
    )
    .expect("parses");
    let with_private = parse_on(
        private_schema(platform_value!({})),
        &config,
        false,
        platform_version,
    )
    .expect("parses");
    assert_eq!(with_private.private_document(), None);
    let result = public
        .as_ref()
        .validate_update(with_private.as_ref(), 2, platform_version)
        .expect("validate_update should not error");
    assert!(
        !result.errors.iter().any(|error| matches!(
            error,
            ConsensusError::StateError(StateError::DocumentTypeUpdateError(error))
                if error.additional_message().contains("private")
        )),
        "{:?}",
        result.errors
    );
}
