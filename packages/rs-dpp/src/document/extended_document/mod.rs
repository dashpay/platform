mod accessors;
mod fields;
mod serialize;
pub(crate) mod v0;

pub use fields::{property_names, IDENTIFIER_FIELDS};

#[cfg(any(feature = "json-conversion", feature = "value-conversion"))]
use crate::data_contract::DataContract;
use crate::ProtocolError;

use crate::document::extended_document::v0::ExtendedDocumentV0;

#[cfg(feature = "validation")]
use crate::validation::SimpleConsensusValidationResult;
use derive_more::From;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use platform_versioning::PlatformVersioned;
#[cfg(feature = "json-conversion")]
use serde_json::Value as JsonValue;
#[cfg(feature = "value-conversion")]
use std::collections::BTreeMap;

#[derive(Debug, Clone, PlatformVersioned, From)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "$extendedFormatVersion")
)]
pub enum ExtendedDocument {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(ExtendedDocumentV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for ExtendedDocument {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for ExtendedDocument {}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::document::extended_document::v0::ExtendedDocumentV0;
    use crate::document::v0::DocumentV0;
    use crate::document::Document;
    use crate::tests::fixtures::get_data_contract_fixture;
    use platform_value::{Bytes32, Identifier};
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn fixture() -> ExtendedDocument {
        let pv = PlatformVersion::latest();
        let created = get_data_contract_fixture(None, 0, pv.protocol_version);
        let data_contract = created.data_contract().clone();
        let data_contract_id = data_contract.id();

        let document = Document::V0(DocumentV0 {
            id: Identifier::new([0xa1; 32]),
            owner_id: Identifier::new([0xb2; 32]),
            properties: BTreeMap::new(),
            revision: Some(1),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        });

        ExtendedDocument::V0(ExtendedDocumentV0 {
            document_type_name: "niceDocument".to_string(),
            data_contract_id,
            document,
            data_contract,
            metadata: None,
            entropy: Bytes32::new([0xcc; 32]),
            token_payment_info: None,
        })
    }

    // Tier 3: ExtendedDocument embeds a full `DataContract` (with all schemas
    // + tokens + groups), so an inline wire-shape assertion would be enormous
    // and brittle. We assert envelope only on the top-level discriminator and
    // deterministic siblings; the embedded `Document` and `DataContract` have
    // their own per-type round-trip tests that lock down their wire shapes.
    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = JsonConvertible::to_json(&original).expect("to_json");
        // Envelope assertions: the inner Document and DataContract are flattened
        // into the root, so the surface includes BOTH wrappers (`$extendedFormatVersion`,
        // `$type`, `$dataContractId`, `$dataContract`, `$entropy`, `$tokenPaymentInfo`,
        // `$metadata`) AND all the embedded Document `$id` / `$ownerId` / `$revision`
        // / `$createdAt*` / `$updatedAt*` / `$transferredAt*` / `$creatorId` keys.
        // We only lock down the wrapper-specific keys and trust that
        // Document and DataContract have their own per-type round-trip tests.
        let obj = json.as_object().expect("json is an object");
        assert_eq!(
            obj.get("$extendedFormatVersion"),
            Some(&serde_json::json!("0"))
        );
        assert_eq!(obj.get("$type"), Some(&serde_json::json!("niceDocument")));
        // entropy is `Bytes32` → base64 in JSON
        assert_eq!(
            obj.get("$entropy"),
            Some(&serde_json::json!(
                "zMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMw="
            ))
        );
        assert_eq!(obj.get("$tokenPaymentInfo"), Some(&serde_json::Value::Null));
        assert_eq!(obj.get("$metadata"), Some(&serde_json::Value::Null));
        assert!(obj.get("$dataContractId").is_some_and(|v| v.is_string()));
        assert!(obj.get("$dataContract").is_some_and(|v| v.is_object()));
        // Document is flattened, so `$formatVersion` (the document's) is at the root too.
        assert_eq!(obj.get("$formatVersion"), Some(&serde_json::json!("0")));
        let recovered = <ExtendedDocument as JsonConvertible>::from_json(json).expect("from_json");
        // ExtendedDocument lacks PartialEq — match variant + assert key fields.
        let ExtendedDocument::V0(orig_v0) = original;
        let ExtendedDocument::V0(rec_v0) = recovered;
        assert_eq!(
            orig_v0.document_type_name, rec_v0.document_type_name,
            "document_type_name"
        );
        assert_eq!(
            orig_v0.data_contract_id, rec_v0.data_contract_id,
            "data_contract_id"
        );
        assert_eq!(orig_v0.entropy, rec_v0.entropy, "entropy");
        assert_eq!(
            orig_v0.token_payment_info, rec_v0.token_payment_info,
            "token_payment_info"
        );
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = ValueConvertible::to_object(&original).expect("to_object");
        // Envelope assertions: see json test above — Document + DataContract
        // are flattened into the root; we only lock down wrapper-specific keys.
        let map = value.as_map().expect("value is a map");
        let get = |key: &str| {
            map.iter()
                .find(|(k, _)| k.as_text() == Some(key))
                .map(|(_, v)| v)
        };
        assert_eq!(
            get("$extendedFormatVersion"),
            Some(&platform_value::Value::Text("0".to_string()))
        );
        assert_eq!(
            get("$type"),
            Some(&platform_value::Value::Text("niceDocument".to_string()))
        );
        assert_eq!(
            get("$entropy"),
            Some(&platform_value::Value::Bytes32([0xcc; 32]))
        );
        assert_eq!(get("$tokenPaymentInfo"), Some(&platform_value::Value::Null));
        assert_eq!(get("$metadata"), Some(&platform_value::Value::Null));
        assert!(get("$dataContractId")
            .is_some_and(|v| matches!(v, platform_value::Value::Identifier(_))));
        assert!(get("$dataContract").is_some_and(|v| v.is_map()));
        // Document is flattened into the root.
        assert_eq!(
            get("$formatVersion"),
            Some(&platform_value::Value::Text("0".to_string()))
        );
        let recovered =
            <ExtendedDocument as ValueConvertible>::from_object(value).expect("from_object");
        let ExtendedDocument::V0(orig_v0) = original;
        let ExtendedDocument::V0(rec_v0) = recovered;
        assert_eq!(
            orig_v0.document_type_name, rec_v0.document_type_name,
            "document_type_name"
        );
        assert_eq!(
            orig_v0.data_contract_id, rec_v0.data_contract_id,
            "data_contract_id"
        );
        assert_eq!(orig_v0.entropy, rec_v0.entropy, "entropy");
    }
}

impl ExtendedDocument {
    #[cfg(feature = "json-conversion")]
    /// Returns the properties of the document as a JSON value.
    ///
    /// # Errors
    ///
    /// Returns a `ProtocolError` if there's an error in converting the properties to JSON.
    pub fn properties_as_json_data(&self) -> Result<JsonValue, ProtocolError> {
        match self {
            ExtendedDocument::V0(v0) => v0.properties_as_json_data(),
        }
    }

    /// Returns an optional reference to the value associated with the specified key.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up in the properties of the document.
    pub fn get_optional_value(&self, key: &str) -> Option<&Value> {
        match self {
            ExtendedDocument::V0(v0) => v0.get_optional_value(key),
        }
    }

    /// Checks if the document can be modified.
    ///
    /// # Errors
    ///
    /// Returns a `ProtocolError` if the document type is not found in the data contract.
    pub fn can_be_modified(&self) -> Result<bool, ProtocolError> {
        match self {
            ExtendedDocument::V0(v0) => v0.can_be_modified(),
        }
    }

    /// Checks if the document needs a revision.
    ///
    /// # Errors
    ///
    /// Returns a `ProtocolError` if the document type is not found in the data contract.
    pub fn needs_revision(&self) -> Result<bool, ProtocolError> {
        match self {
            ExtendedDocument::V0(v0) => v0.requires_revision(),
        }
    }

    /// Create an extended document from a JSON string and a data contract.
    ///
    /// This function is a passthrough to the `from_json_string` method.
    #[cfg(feature = "json-conversion")]
    pub fn from_json_string(
        string: &str,
        contract: DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        Ok(ExtendedDocument::V0(ExtendedDocumentV0::from_json_string(
            string,
            contract,
            platform_version,
        )?))
    }

    /// Create an extended document from a raw JSON document and a data contract.
    ///
    /// This function is a passthrough to the `from_raw_json_document` method.
    #[cfg(feature = "json-conversion")]
    pub fn from_raw_json_document(
        raw_document: JsonValue,
        data_contract: DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        Ok(ExtendedDocument::V0(
            ExtendedDocumentV0::from_raw_json_document(
                raw_document,
                data_contract,
                platform_version,
            )?,
        ))
    }

    #[cfg(feature = "value-conversion")]
    /// Create an extended document from a trusted platform value object where fields are already in
    /// the proper format for the contract.
    ///
    /// # Arguments
    ///
    /// * `document_value` - A `Value` representing the document value.
    /// * `data_contract` - A `DataContract` instance.
    ///
    /// # Errors
    ///
    /// Returns a `ProtocolError` if there is an error processing the trusted platform value.
    pub fn from_trusted_platform_value(
        document_value: Value,
        data_contract: DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .document_versions
            .extended_document_structure_version
        {
            0 => Ok(ExtendedDocument::V0(
                ExtendedDocumentV0::from_trusted_platform_value(
                    document_value,
                    data_contract,
                    platform_version,
                )?,
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ExtendedDocument::from_trusted_platform_value".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    #[cfg(feature = "value-conversion")]
    /// Create an extended document from an untrusted platform value object where fields might not
    /// be in the proper format for the contract.
    ///
    /// # Arguments
    ///
    /// * `document_value` - A `Value` representing the document value.
    /// * `data_contract` - A `DataContract` instance.
    ///
    /// # Errors
    ///
    /// Returns a `ProtocolError` if there is an error processing the untrusted platform value.
    pub fn from_untrusted_platform_value(
        document_value: Value,
        data_contract: DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .document_versions
            .extended_document_structure_version
        {
            0 => Ok(ExtendedDocument::V0(
                ExtendedDocumentV0::from_untrusted_platform_value(
                    document_value,
                    data_contract,
                    platform_version,
                )?,
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ExtendedDocument::from_untrusted_platform_value".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Convert the extended document to a BTreeMap of string keys and Value instances.
    ///
    /// This function is a passthrough to the `to_map_value` method.
    #[cfg(feature = "value-conversion")]
    pub fn to_map_value(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        match self {
            ExtendedDocument::V0(v0) => v0.to_map_value(),
        }
    }

    /// Convert the extended document to a BTreeMap of string keys and Value instances consuming the instance.
    ///
    /// This function is a passthrough to the `into_map_value` method.
    #[cfg(feature = "value-conversion")]
    pub fn into_map_value(self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        match self {
            ExtendedDocument::V0(v0) => v0.into_map_value(),
        }
    }

    /// Calculate the hash of the extended document.
    ///
    /// This function is a passthrough to the `hash` method.
    pub fn hash(&self, platform_version: &PlatformVersion) -> Result<Vec<u8>, ProtocolError> {
        match self {
            ExtendedDocument::V0(v0) => v0.hash(platform_version),
        }
    }

    /// Set the value under the given path.
    ///
    /// This function is a passthrough to the `set` method.
    pub fn set(&mut self, path: &str, value: Value) -> Result<(), ProtocolError> {
        match self {
            ExtendedDocument::V0(v0) => v0.set(path, value),
        }
    }

    /// Set the value under the given path.
    /// The path should be checked against the contract's document type and the value's type
    /// should be modified accordingly.
    /// For example we could go from a base 64 string to bytes.
    ///
    /// This function is a passthrough to the `set_untrusted` method.
    pub fn set_untrusted(&mut self, path: &str, value: Value) -> Result<(), ProtocolError> {
        match self {
            ExtendedDocument::V0(v0) => v0.set_untrusted(path, value),
        }
    }

    /// Retrieve the field specified by the path.
    ///
    /// This function is a passthrough to the `get` method.
    pub fn get(&self, path: &str) -> Option<&Value> {
        match self {
            ExtendedDocument::V0(v0) => v0.get(path),
        }
    }

    #[cfg(feature = "validation")]
    /// Validates the extended document against the data contract
    pub fn validate(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match self {
            ExtendedDocument::V0(v0) => v0.validate(platform_version),
        }
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use serde_json::Value as JsonValue;
    use std::convert::TryInto;

    use crate::document::extended_document::ExtendedDocument;

    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::document::extended_document::v0::ExtendedDocumentV0;

    use crate::prelude::Identifier;
    use crate::system_data_contracts::load_system_data_contract;
    use crate::tests::utils::*;
    use data_contracts::SystemDataContract;

    use platform_value::btreemap_extensions::BTreeValueMapPathHelper;
    use platform_value::string_encoding::Encoding;
    use platform_value::Value;
    use platform_version::version::{PlatformVersion, LATEST_PLATFORM_VERSION};
    use pretty_assertions::assert_eq;

    use crate::data_contract::document_type::random_document::CreateRandomDocument;
    use crate::document::serialization_traits::ExtendedDocumentPlatformConversionMethodsV0;
    use crate::tests::fixtures::get_dashpay_contract_fixture;

    fn init() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }

    #[test]
    #[cfg(feature = "json-conversion")]
    fn test_document_json_deserialize() -> Result<()> {
        init();
        let platform_version = PlatformVersion::latest();
        let dpns_contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)?;
        let document_json = get_data_from_file("src/tests/payloads/document_dpns.json")?;
        let doc =
            ExtendedDocument::from_json_string(&document_json, dpns_contract, platform_version)?;
        assert_eq!(doc.document_type_name(), "domain");
        assert_eq!(doc.feature_version(), 0);
        assert_eq!(
            doc.id().to_buffer(),
            Identifier::from_string(
                "4veLBZPHDkaCPF9LfZ8fX3JZiS5q5iUVGhdBbaa9ga5E",
                Encoding::Base58
            )
            .unwrap()
            .to_buffer()
        );
        assert_eq!(
            doc.data_contract_id().to_buffer(),
            Identifier::from_string(
                "566vcJkmebVCAb2Dkj2yVMSgGFcsshupnQqtsz1RFbcy",
                Encoding::Base58
            )
            .unwrap()
            .to_buffer()
        );
        assert_eq!(
            doc.properties()
                .get("label")
                .expect("expected to get label"),
            &Value::Text("user-9999".to_string())
        );
        println!(
            "{:?}",
            doc.properties()
                .get_at_path("records.identity")
                .expect("expected to get value")
        );
        assert_eq!(
            doc.properties()
                .get_at_path("records.identity")
                .expect("expected to get value"),
            &Value::Identifier(
                bs58::decode("HBNMY5QWuBVKNFLhgBTC1VmpEnscrmqKPMXpnYSHwhfn")
                    .into_vec()
                    .unwrap()
                    .try_into()
                    .unwrap()
            )
        );
        assert_eq!(
            doc.properties()
                .get_at_path("subdomainRules.allowSubdomains")
                .expect("expected to get value"),
            &Value::Bool(false)
        );
        Ok(())
    }

    #[test]
    fn test_buffer_serialize_deserialize() {
        init();
        let init_doc = new_example_document();
        let buffer_document = init_doc
            .serialize_to_bytes(LATEST_PLATFORM_VERSION)
            .expect("no errors");

        let doc = ExtendedDocument::from_bytes(buffer_document.as_slice(), LATEST_PLATFORM_VERSION)
            .expect("document should be created from buffer");

        assert_eq!(init_doc.created_at(), doc.created_at());
        assert_eq!(init_doc.updated_at(), doc.updated_at());
        assert_eq!(
            init_doc.created_at_block_height(),
            doc.created_at_block_height()
        );
        assert_eq!(
            init_doc.updated_at_block_height(),
            doc.updated_at_block_height()
        );
        assert_eq!(
            init_doc.created_at_core_block_height(),
            doc.created_at_core_block_height()
        );
        assert_eq!(
            init_doc.updated_at_core_block_height(),
            doc.updated_at_core_block_height()
        );
        assert_eq!(init_doc.id(), doc.id());
        assert_eq!(init_doc.data_contract_id(), doc.data_contract_id());
        assert_eq!(init_doc.owner_id(), doc.owner_id());
    }

    #[test]
    fn test_json_serialize() -> Result<()> {
        init();

        let dpns_contract =
            load_system_data_contract(SystemDataContract::DPNS, LATEST_PLATFORM_VERSION)?;
        let document_json = get_data_from_file("src/tests/payloads/document_dpns.json")?;
        let document = ExtendedDocument::from_json_string(
            &document_json,
            dpns_contract,
            LATEST_PLATFORM_VERSION,
        )?;
        let value: JsonValue = serde_json::to_value(&document)?;

        assert_eq!(
            value["$extendedFormatVersion"],
            JsonValue::String("0".to_string()),
            "outer enum version is its own key, distinct from the inner Document's $formatVersion",
        );
        assert_eq!(
            value["$formatVersion"],
            JsonValue::String("0".to_string()),
            "inner Document's version surfaces at top level via serde(flatten)",
        );
        assert_eq!(value["$type"], JsonValue::String("domain".to_string()));
        assert_eq!(
            value["$dataContractId"],
            JsonValue::String("566vcJkmebVCAb2Dkj2yVMSgGFcsshupnQqtsz1RFbcy".to_string())
        );
        assert_eq!(
            value["$id"],
            JsonValue::String("4veLBZPHDkaCPF9LfZ8fX3JZiS5q5iUVGhdBbaa9ga5E".to_string())
        );
        assert_eq!(value["label"], JsonValue::String("user-9999".to_string()));

        Ok(())
    }

    #[test]
    fn test_document_to_buffer() -> Result<()> {
        init();

        let document_json = get_data_from_file("src/tests/payloads/document_dpns.json")?;
        let dpns_contract =
            load_system_data_contract(SystemDataContract::DPNS, LATEST_PLATFORM_VERSION).unwrap();
        ExtendedDocument::from_json_string(&document_json, dpns_contract, LATEST_PLATFORM_VERSION)
            .expect("expected extended document");
        Ok(())
    }

    fn document_bytes() -> Vec<u8> {
        new_example_document()
            .serialize_to_bytes(LATEST_PLATFORM_VERSION)
            .unwrap()
    }

    fn new_example_document() -> ExtendedDocument {
        let data_contract =
            get_dashpay_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version)
                .data_contract_owned();
        let document_type = data_contract
            .document_type_for_name("profile")
            .expect("expected to get profile document type");
        let data_contract_id = data_contract.id();
        ExtendedDocumentV0 {
            document_type_name: "profile".to_string(),
            document: document_type
                .random_document(Some(15), LATEST_PLATFORM_VERSION)
                .expect("expected to get a random document"),
            data_contract,
            metadata: None,
            data_contract_id,
            entropy: Default::default(),
            token_payment_info: None,
        }
        .into()
    }
}
