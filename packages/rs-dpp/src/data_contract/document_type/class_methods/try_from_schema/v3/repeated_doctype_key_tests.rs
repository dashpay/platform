//! A document type schema naming a key twice at its root, which registration refuses from
//! protocol version 15 (`check_document_type_schema` 1).

use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::{DocumentPropertyType, DocumentType};
use crate::data_contract::errors::DataContractError;
use crate::data_contract::serialized_version::v0::DataContractInSerializationFormatV0;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::data_contract::DataContract;
use crate::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted;
use crate::ProtocolError;
use platform_serialization::PLATFORM_BINCODE_CONFIG;
use platform_value::{platform_value, Identifier, Value};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// The latest protocol version with the registration check of protocol version 14 and
/// earlier, `check_document_type_schema` 0.
fn latest_with_the_meta_schema_check() -> &'static PlatformVersion {
    static VERSION: OnceLock<PlatformVersion> = OnceLock::new();
    VERSION.get_or_init(|| {
        let mut version = PlatformVersion::latest().clone();
        version
            .dpp
            .contract_versions
            .document_type_versions
            .schema
            .check_document_type_schema = 0;
        version
    })
}

fn parse_on(
    schema: Value,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<DocumentType, ProtocolError> {
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("expected a default config");
    DocumentType::try_from_schema(
        Identifier::new([7; 32]),
        1,
        config.version(),
        "note",
        schema,
        None,
        &BTreeMap::new(),
        &config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

fn parse(schema: Value, full_validation: bool) -> Result<DocumentType, ProtocolError> {
    parse_on(schema, full_validation, PlatformVersion::latest())
}

/// Requires `error` to be the paid basic error `InvalidContractStructure` saying `key` is
/// declared more than once.
fn assert_repeat_refused(error: ProtocolError, key: &str) {
    let ProtocolError::ConsensusError(consensus_error) = &error else {
        panic!("expected a consensus error, got {error:?}");
    };
    let ConsensusError::BasicError(BasicError::ContractError(
        DataContractError::InvalidContractStructure(message),
    )) = consensus_error.as_ref()
    else {
        panic!("expected InvalidContractStructure, got {consensus_error:?}");
    };
    assert!(
        message.contains(&format!("declares `{key}` more than once")),
        "{message}"
    );
}

fn integer_properties() -> Value {
    platform_value!({ "a": { "type": "integer", "minimum": 0, "maximum": 100, "position": 0 } })
}

fn string_properties() -> Value {
    platform_value!({ "a": { "type": "string", "maxLength": 10, "position": 0 } })
}

/// A schema whose root holds `entries` in order, with `additionalProperties: false`.
fn schema_of(entries: Vec<(&str, Value)>) -> Value {
    let mut map: Vec<(Value, Value)> = vec![("type".into(), "object".into())];
    map.extend(
        entries
            .into_iter()
            .map(|(key, value)| (Value::Text(key.to_string()), value)),
    );
    map.push(("additionalProperties".into(), false.into()));
    Value::Map(map)
}

fn repeating_properties() -> Value {
    schema_of(vec![
        ("properties", string_properties()),
        ("properties", integer_properties()),
    ])
}

/// The parser reads the first entry of a key repeated at the root, every schema check the
/// last, so registration refuses the repeat, whichever key it is.
#[test]
fn should_refuse_a_key_repeated_at_the_root_of_a_document_schema_under_full_validation() {
    for (key, schema) in [
        ("properties", repeating_properties()),
        (
            "documentsMutable",
            schema_of(vec![
                ("properties", integer_properties()),
                ("documentsMutable", false.into()),
                ("documentsMutable", true.into()),
            ]),
        ),
    ] {
        assert_repeat_refused(
            parse(schema, true).expect_err("expected the repeat to be refused"),
            key,
        );
    }
}

/// The refusal belongs to `check_document_type_schema` 1: the registration check of
/// protocol version 14 and earlier still admits the repeat.
#[test]
fn should_leave_the_meta_schema_check_admitting_a_repeated_key() {
    parse_on(
        repeating_properties(),
        true,
        latest_with_the_meta_schema_check(),
    )
    .expect("expected check_document_type_schema 0 to admit the schema");
}

/// The rule holds at registration only: a contract read back from state, parsed without
/// validation, still loads, reading the first entry as it always has.
#[test]
fn should_still_parse_a_repeated_key_without_validation() {
    let document_type =
        parse(repeating_properties(), false).expect("expected a stored schema to load");
    assert!(matches!(
        document_type.flattened_properties()["a"].property_type,
        DocumentPropertyType::String(_)
    ));
}

/// The same schema without the repeat registers.
#[test]
fn should_register_the_schema_once_the_repeat_is_gone() {
    parse(schema_of(vec![("properties", integer_properties())]), true)
        .expect("expected the schema to register");
}

/// A contract decoded from the bytes a create or update transition carries keeps both
/// entries through bincode, and the full validation the transition runs refuses it.
#[test]
fn should_refuse_a_contract_decoded_with_a_repeated_key_under_full_validation() {
    let platform_version = PlatformVersion::latest();
    let contract_bytes = |schema: Value| {
        let format: DataContractInSerializationFormat = DataContractInSerializationFormatV0 {
            id: Identifier::new([7; 32]),
            config: DataContractConfig::default_for_version(platform_version)
                .expect("expected a default config"),
            version: 2,
            owner_id: Identifier::new([8; 32]),
            schema_defs: None,
            document_schemas: BTreeMap::from([("note".to_string(), schema)]),
        }
        .into();
        bincode::encode_to_vec(format, PLATFORM_BINCODE_CONFIG)
            .expect("expected to encode the contract")
    };

    DataContract::versioned_deserialize_untrusted(
        &contract_bytes(schema_of(vec![("properties", integer_properties())])),
        true,
        platform_version,
    )
    .expect("expected the contract to register");

    let error = DataContract::versioned_deserialize_untrusted(
        &contract_bytes(repeating_properties()),
        true,
        platform_version,
    )
    .expect_err("expected the contract to be refused");
    assert_repeat_refused(error, "properties");
}
