//! The `private` doctype keyword (protocol version 15): a private document type, whose
//! documents are fixed-size encrypted entries the chain never reads. What the parse holds such
//! a type to on both paths, the entry size limit at registration, the protocol version gate,
//! the stored form, contract updates, and the references of the same contract that may not
//! name one.

use crate::consensus::basic::BasicError;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::config::v0::DataContractConfigSettersV0;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use crate::data_contract::document_type::serialized_version::DocumentTypeInSerializationFormat;
use crate::data_contract::document_type::{DocumentType, PrivateDocumentConfig};
use crate::data_contract::DataContract;
use crate::ProtocolError;
use platform_value::string_encoding::Encoding;
use platform_value::{platform_value, Identifier, Value};
use platform_version::version::v14::PROTOCOL_VERSION_14;
use platform_version::version::PlatformVersion;
use serde_json::json;
use std::collections::BTreeMap;

const CONTRACT_ID: [u8; 32] = [7; 32];

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

/// `schema` without its top-level `key`.
fn without(mut schema: Value, key: &str) -> Value {
    if let Value::Map(schema_map) = &mut schema {
        schema_map.retain(|(existing, _)| existing.as_text() != Some(key));
    }
    schema
}

fn assert_refused_naming(result: Result<DocumentType, ProtocolError>, fragments: &[&str]) {
    let error = result.expect_err("the document type must be refused");
    // A paid refusal needs the consensus variant: the data contract error variant would
    // surface as an internal error in a block
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
        assert_eq!(
            document_type.private_document(),
            Some(PrivateDocumentConfig::new(704))
        );
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
    assert_eq!(max_entry_size, 4096);

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
        &["entrySize: 4097", "4096 bytes"],
    );
    // A contract read back from state passed the limit when it was registered
    let stored = parse(
        private_schema(platform_value!({ "private": { "entrySize": max_entry_size + 1 } })),
        false,
    )
    .expect("the stored path does not check the limit again");
    assert_eq!(
        stored.private_document(),
        Some(PrivateDocumentConfig::new(4097))
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
        let mut schema = private_schema(platform_value!({}));
        if let Value::Map(schema_map) = &mut schema {
            for (key, value) in schema_map.iter_mut() {
                if key.as_text() == Some("properties") {
                    if let Value::Map(properties) = value {
                        properties.push((Value::Text("recipientId".to_string()), property.clone()));
                    }
                }
            }
        }
        for full_validation in [true, false] {
            assert_refused_naming(parse(schema.clone(), full_validation), &[fragment]);
        }
    }
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
    assert_eq!(
        read_back.private_document(),
        Some(PrivateDocumentConfig::new(704))
    );
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
) -> crate::validation::SimpleConsensusValidationResult {
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

fn contract(
    contract: serde_json::Value,
    full_validation: bool,
) -> Result<DataContract, ProtocolError> {
    let value = platform_value::to_value(contract).expect("the contract should convert");
    DataContract::from_value(value, full_validation, PlatformVersion::latest())
}

#[test]
fn should_register_a_contract_with_a_private_type() {
    let registered = contract(pointer_contract(None, None), true).expect("registers");
    let secret = registered
        .document_type_for_name("secret")
        .expect("expected the secret type");
    assert_eq!(
        secret.private_document(),
        Some(PrivateDocumentConfig::new(512))
    );
}

#[test]
fn should_refuse_a_find_by_into_a_private_type() {
    assert_refused_contract(
        contract(
            pointer_contract(
                Some(json!({
                    "type": "permanentDocument",
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

fn assert_refused_contract(result: Result<DataContract, ProtocolError>, fragment: &str) {
    let error = result.expect_err("the contract must be refused");
    assert!(
        matches!(error, ProtocolError::ConsensusError(_)),
        "expected a consensus error, got {error:?}"
    );
    let message = format!("{error:?}");
    assert!(
        message.contains(fragment),
        "error must say {fragment}, got {message}"
    );
}
