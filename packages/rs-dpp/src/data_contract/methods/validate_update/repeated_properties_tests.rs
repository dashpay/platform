//! A contract update whose document type schema names `properties` twice, which
//! registration refuses from protocol version 15 (a document type schema names
//! no key twice at its root).

use crate::data_contract::config::DataContractConfig;
use crate::data_contract::serialized_version::v0::DataContractInSerializationFormatV0;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::data_contract::DataContract;
use crate::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted;
use platform_value::{platform_value, Identifier, Value};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

const TYPE_NAME: &str = "t";

fn integer_properties() -> Value {
    platform_value!({
        "a": {"type": "integer", "minimum": 0, "maximum": 100, "position": 0},
        "b": {"type": "string", "maxLength": 20, "position": 1},
    })
}

fn string_properties() -> Value {
    platform_value!({
        "a": {"type": "string", "maxLength": 10, "position": 0},
        "b": {"type": "string", "maxLength": 20, "position": 1},
    })
}

fn schema(properties: Vec<Value>) -> Value {
    let mut map: Vec<(Value, Value)> = vec![("type".into(), "object".into())];
    map.extend(
        properties
            .into_iter()
            .map(|entry| ("properties".into(), entry)),
    );
    map.push(("additionalProperties".into(), false.into()));
    Value::Map(map)
}

/// The bytes a state transition carries for version `version` of the contract.
fn contract_bytes(version: u32, schema: Value, platform_version: &PlatformVersion) -> Vec<u8> {
    let format: DataContractInSerializationFormat = DataContractInSerializationFormatV0 {
        id: Identifier::new([7; 32]),
        config: DataContractConfig::default_for_version(platform_version)
            .expect("expected a default config"),
        version,
        owner_id: Identifier::new([8; 32]),
        schema_defs: None,
        document_schemas: BTreeMap::from([(TYPE_NAME.to_string(), schema)]),
    }
    .into();
    bincode::encode_to_vec(
        format,
        bincode::config::standard()
            .with_big_endian()
            .with_no_limit(),
    )
    .expect("expected to encode the contract")
}

#[test]
fn should_refuse_a_contract_update_repeating_properties() {
    let platform_version = PlatformVersion::latest();

    // The stored version registers
    DataContract::versioned_deserialize_untrusted(
        &contract_bytes(1, schema(vec![integer_properties()]), platform_version),
        true,
        platform_version,
    )
    .expect("expected the stored version to register");

    // The update keeps both entries through bincode and is refused by the full validation
    // the update transition runs, before `validate_update` could compare it
    let refusal = DataContract::versioned_deserialize_untrusted(
        &contract_bytes(
            2,
            schema(vec![string_properties(), integer_properties()]),
            platform_version,
        ),
        true,
        platform_version,
    )
    .expect_err("expected the update to be refused");
    let message = format!("{refusal:?}");
    assert!(
        message.contains("declares `properties` more than once"),
        "{message}"
    );
}
