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
    use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use crate::data_contract::serialized_version::property_names;
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
        let serialized = data_contract
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("data contract should be serialized");

        // The serialized form is a `bincode::standard()`-encoded
        // `DataContractInSerializationFormat` enum; its leading byte is the
        // single-byte variant tag (0 = V0). Replace that tag with a
        // *well-formed* bincode varint encoding of u32::MAX — the marker
        // byte 0xFC (252) means "u32 follows", and four 0xFF bytes encode
        // u32::MAX. The body of the original message is preserved.
        //
        // The resulting prefix is structurally valid (parses cleanly as a
        // varint) but represents a version that no DataContract variant
        // exists for, so deserialization must fail with an unknown-version
        // error rather than treating the body as corrupted.
        let mut too_high_version_bytes: Vec<u8> = vec![0xFC, 0xFF, 0xFF, 0xFF, 0xFF];
        too_high_version_bytes.extend_from_slice(&serialized[1..]);

        assert!(
            DataContract::versioned_deserialize(&too_high_version_bytes, true, platform_version)
                .is_err(),
            "well-formed but oversized version prefix should fail deserialization"
        );
    }

    /// Fixed external JSON payload for V0 data contract conversion tests.
    ///
    /// This fixture is the *external reference shape* the on-the-wire JSON
    /// representation of a `DataContractV0` is expected to match. It is
    /// intentionally hand-written and committed so that the round-trip tests
    /// below cannot drift together with the implementation under test.
    const FIXED_DATA_CONTRACT_V0_JSON: &str =
        include_str!("../../tests/payloads/data_contract_v0.json");

    const FIXED_CONTRACT_ID_BASE58: &str = "BmKTJeLL3GfH8FxEx7SUbTog4eAKj8vJRDi97gYkxB9p";
    const FIXED_CONTRACT_OWNER_ID_BASE58: &str = "HtQNfXBZJu3WnvjvCFJKgbvfgWYJxWxaFWy23TKoFjg9";

    #[test]
    fn conversion_from_json() {
        init();
        let platform_version = PlatformVersion::first();

        let fixture: serde_json::Value = serde_json::from_str(FIXED_DATA_CONTRACT_V0_JSON)
            .expect("fixture should be valid JSON");

        // Parse the fixed external payload and assert it deserializes to the
        // exact identifiers/version/document types it encodes — this would
        // catch silent drift in identifier decoding, version handling, or
        // document-schema extraction even if `to_json` changes in lockstep.
        let restored = DataContractV0::from_json(fixture, false, platform_version)
            .expect("fixed fixture should convert json to contract");

        assert_eq!(
            FIXED_CONTRACT_ID_BASE58,
            restored
                .id()
                .to_string(platform_value::string_encoding::Encoding::Base58),
        );
        assert_eq!(
            FIXED_CONTRACT_OWNER_ID_BASE58,
            restored
                .owner_id()
                .to_string(platform_value::string_encoding::Encoding::Base58),
        );
        assert_eq!(1, restored.version());
        assert_eq!(
            1,
            restored.document_types().len(),
            "fixture defines exactly one document type",
        );
        assert!(
            restored.document_types().contains_key("note"),
            "fixture's `note` document type must round-trip through from_json",
        );
    }

    #[test]
    fn conversion_to_json() {
        init();
        let platform_version = PlatformVersion::first();

        let fixture: serde_json::Value = serde_json::from_str(FIXED_DATA_CONTRACT_V0_JSON)
            .expect("fixture should be valid JSON");

        // Build the contract from the fixed fixture so the *output* of
        // `to_json` is being compared against an externally committed shape,
        // not against data produced by the same conversion path under test.
        let contract = DataContractV0::from_json(fixture.clone(), false, platform_version)
            .expect("fixed fixture should convert json to contract");

        let produced = contract
            .to_json(platform_version)
            .expect("should convert contract back to json");

        assert!(produced.is_object(), "top-level JSON should be an object");

        // Compare on consensus-relevant top-level fields. We deliberately do
        // not raw-string-match: this asserts structural compatibility with
        // the fixed external payload regardless of key ordering or whitespace.
        for field in &["$formatVersion", "id", "ownerId", "version"] {
            assert_eq!(
                fixture.get(field),
                produced.get(field),
                "produced JSON must agree with fixed external payload on `{field}`",
            );
        }

        // `documentSchemas` is the user-visible payload of a contract — it
        // must round-trip byte-for-byte from the fixture through to_json.
        assert_eq!(
            fixture.get("documentSchemas"),
            produced.get("documentSchemas"),
            "documentSchemas must round-trip identically against fixed payload",
        );
    }

    #[test]
    fn conversion_to_object() {
        let platform_version = PlatformVersion::first();
        let data_contract = get_data_contract_v0(platform_version);

        let validating_json = data_contract
            .to_validating_json(platform_version)
            .expect("should convert to validating json");
        for path in [property_names::ID, property_names::OWNER_ID] {
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
