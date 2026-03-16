use std::collections::BTreeMap;

use platform_value::Identifier;
use platform_value::Value;

use crate::data_contract::{DefinitionName, DocumentName};

use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::DocumentType;

use crate::metadata::Metadata;

/// `DataContractV0` represents a data contract in a decentralized platform.
///
/// It contains information about the contract, such as its protocol version, unique identifier,
/// schema, version, and owner identifier. The struct also includes details about the document
/// types, metadata, configuration, and document schemas associated with the contract.
///
/// Additionally, `DataContractV0` holds definitions for JSON schemas, entropy, and binary properties
/// of the documents.
#[derive(Debug, Clone, PartialEq)]
pub struct DataContractV0 {
    /// A unique identifier for the data contract.
    /// This field must always present in all versions.
    pub(crate) id: Identifier,

    /// The version of this data contract.
    pub(crate) version: u32,

    /// The identifier of the contract owner.
    pub(crate) owner_id: Identifier,

    /// A mapping of document names to their corresponding document types.
    pub document_types: BTreeMap<DocumentName, DocumentType>,

    // TODO: Move metadata from here
    /// Optional metadata associated with the contract.
    pub(crate) metadata: Option<Metadata>,

    /// Internal configuration for the contract.
    pub(crate) config: DataContractConfig,

    /// Shared subschemas to reuse across documents (see $defs)
    pub(crate) schema_defs: Option<BTreeMap<DefinitionName, Value>>,
}

#[cfg(test)]
mod test {
    use crate::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use crate::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
    use crate::data_contract::conversion::value::v0::{
        DataContractValueConversionMethodsV0, DATA_CONTRACT_IDENTIFIER_FIELDS_V0,
    };
    use crate::data_contract::v0::DataContractV0;
    use crate::data_contract::DataContract;
    use crate::serialization::{
        PlatformDeserializableWithPotentialValidationFromVersionedStructure,
        PlatformSerializableWithPlatformVersion,
    };
    use crate::tests::fixtures::get_data_contract_fixture;
    use crate::version::PlatformVersion;

    fn init() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }

    fn get_data_contract_v0(platform_version: &PlatformVersion) -> DataContractV0 {
        get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned()
            .into_v0()
            .expect("expected V0 data contract for first platform version")
    }

    #[test]
    fn conversion_to_cbor_buffer_from_cbor_buffer() {
        init();
        let platform_version = PlatformVersion::first();
        let data_contract_v0 = get_data_contract_v0(platform_version);
        let data_contract: DataContract = data_contract_v0.clone().into();

        let serialized = data_contract
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("data contract should be serialized");
        let restored = DataContract::versioned_deserialize(&serialized, true, platform_version)
            .expect("data contract should be deserialized");

        assert_eq!(data_contract, restored);
    }

    #[test]
    fn conversion_to_cbor_buffer_from_cbor_buffer_high_version() {
        init();
        let platform_version = PlatformVersion::first();
        let mut data_contract_v0 = get_data_contract_v0(platform_version);
        data_contract_v0.set_version(10_000);
        let data_contract: DataContract = data_contract_v0.clone().into();

        let serialized = data_contract
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("data contract should be serialized");
        let restored = DataContract::versioned_deserialize(&serialized, true, platform_version)
            .expect("data contract should be deserialized")
            .into_v0()
            .expect("expected v0 data contract");

        assert_eq!(data_contract_v0.version(), restored.version());
        assert_eq!(data_contract_v0.id(), restored.id());
        assert_eq!(data_contract_v0.owner_id(), restored.owner_id());
    }

    #[test]
    fn conversion_to_cbor_buffer_from_cbor_buffer_too_high_version() {
        init();
        let platform_version = PlatformVersion::first();
        let data_contract: DataContract = get_data_contract_v0(platform_version).into();
        let mut serialized = data_contract
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("data contract should be serialized");

        serialized[0] = u8::MAX;

        assert!(
            DataContract::versioned_deserialize(&serialized, true, platform_version).is_err(),
            "corrupted version byte should fail deserialization"
        );
    }

    #[test]
    fn conversion_from_json() {
        init();
        let platform_version = PlatformVersion::first();
        let data_contract = get_data_contract_v0(platform_version);

        let json = data_contract
            .to_json(platform_version)
            .expect("should convert contract to json");
        let restored = DataContractV0::from_json(json, true, platform_version)
            .expect("should convert json to contract");

        assert_eq!(data_contract.id(), restored.id());
        assert_eq!(data_contract.owner_id(), restored.owner_id());
        assert_eq!(data_contract.version(), restored.version());
        assert_eq!(
            data_contract.document_types().len(),
            restored.document_types().len()
        );
    }

    #[test]
    fn conversion_to_json() {
        init();
        let platform_version = PlatformVersion::first();
        let data_contract = get_data_contract_v0(platform_version);

        let serialized_contract = serde_json::to_string(
            &data_contract
                .to_json(platform_version)
                .expect("should convert contract to json"),
        )
        .expect("json serialization should succeed");

        assert!(serialized_contract.contains("\"$formatVersion\":\"0\""));
        assert!(serialized_contract.contains("\"documentSchemas\""));
    }

    #[test]
    fn conversion_to_object() {
        let platform_version = PlatformVersion::first();
        let data_contract = get_data_contract_v0(platform_version);

        let validating_json = data_contract
            .to_validating_json(platform_version)
            .expect("should convert to validating json");
        for path in DATA_CONTRACT_IDENTIFIER_FIELDS_V0 {
            assert!(validating_json
                .get(path)
                .expect("the path should exist")
                .is_array());
        }
    }

    #[test]
    fn conversion_from_object() {
        init();
        let platform_version = PlatformVersion::first();
        let data_contract = get_data_contract_v0(platform_version);

        let raw_contract = data_contract
            .to_value(platform_version)
            .expect("contract should convert to value");
        let restored = DataContractV0::from_value(raw_contract, true, platform_version)
            .expect("contract should be restored from value");

        assert_eq!(data_contract.id(), restored.id());
        assert_eq!(data_contract.owner_id(), restored.owner_id());
        assert_eq!(data_contract.version(), restored.version());
    }

    #[test]
    fn deserialize_dpp_cbor() {
        let platform_version = PlatformVersion::first();
        let data_contract_v0 = get_data_contract_v0(platform_version);
        let data_contract: DataContract = data_contract_v0.clone().into();

        let serialized = data_contract
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("data contract should be serialized");
        let restored = DataContract::versioned_deserialize(&serialized, true, platform_version)
            .expect("data contract should be deserialized")
            .into_v0()
            .expect("expected v0 data contract");

        assert_eq!(data_contract_v0.version(), restored.version());
        assert_eq!(data_contract_v0.id(), restored.id());
        assert_eq!(data_contract_v0.owner_id(), restored.owner_id());
    }

    #[test]
    fn serialize_deterministically_serialize_to_cbor() {
        let platform_version = PlatformVersion::first();
        let data_contract: DataContract = get_data_contract_v0(platform_version).into();

        let first = data_contract
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("data contract should be serialized");
        let second = data_contract
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("data contract should be serialized");

        assert_eq!(first, second);
    }

    #[test]
    fn serialize_deterministically_serialize_to_bincode() {
        let platform_version = PlatformVersion::first();
        let data_contract: DataContract = get_data_contract_v0(platform_version).into();

        let by_ref = data_contract
            .clone()
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("data contract should be serialized");
        let by_consume = data_contract
            .serialize_consume_to_bytes_with_platform_version(platform_version)
            .expect("data contract should be serialized");

        assert_eq!(by_ref, by_consume);
    }
}
