mod accessors;
mod fields;
#[cfg(feature = "document-serde-conversion")]
mod serde_serialize;
mod serialize;
pub(crate) mod v0;

pub use fields::{property_names, IDENTIFIER_FIELDS};

use crate::data_contract::DataContract;
use crate::ProtocolError;

use crate::document::extended_document::v0::ExtendedDocumentV0;

#[cfg(feature = "document-json-conversion")]
use crate::document::serialization_traits::DocumentJsonMethodsV0;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use platform_versioning::PlatformVersioned;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PlatformVersioned)]
pub enum ExtendedDocument {
    V0(ExtendedDocumentV0),
}

impl ExtendedDocument {
    #[cfg(feature = "document-json-conversion")]
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
            ExtendedDocument::V0(v0) => v0.needs_revision(),
        }
    }

    /// Create an extended document from a JSON string and a data contract.
    ///
    /// This function is a passthrough to the `from_json_string` method.
    #[cfg(feature = "document-json-conversion")]
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
    #[cfg(feature = "document-json-conversion")]
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

    /// Convert the extended document to a JSON object.
    ///
    /// This function is a passthrough to the `to_json` method.
    #[cfg(feature = "document-json-conversion")]
    pub fn to_json(&self, platform_version: &PlatformVersion) -> Result<JsonValue, ProtocolError> {
        match self {
            ExtendedDocument::V0(v0) => v0.to_json(platform_version),
        }
    }

    /// Convert the extended document to a pretty JSON object.
    ///
    /// This function is a passthrough to the `to_pretty_json` method.
    #[cfg(feature = "document-json-conversion")]
    pub fn to_pretty_json(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<JsonValue, ProtocolError> {
        match self {
            ExtendedDocument::V0(v0) => v0.to_pretty_json(platform_version),
        }
    }

    /// Convert the extended document to a BTreeMap of string keys and Value instances.
    ///
    /// This function is a passthrough to the `to_map_value` method.
    #[cfg(feature = "document-value-conversion")]
    pub fn to_map_value(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        match self {
            ExtendedDocument::V0(v0) => v0.to_map_value(),
        }
    }

    /// Convert the extended document to a BTreeMap of string keys and Value instances consuming the instance.
    ///
    /// This function is a passthrough to the `into_map_value` method.
    #[cfg(feature = "document-value-conversion")]
    pub fn into_map_value(self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        match self {
            ExtendedDocument::V0(v0) => v0.into_map_value(),
        }
    }

    /// Convert the extended document to a Value instance consuming the instance.
    ///
    /// This function is a passthrough to the `into_value` method.
    #[cfg(feature = "document-value-conversion")]
    pub fn into_value(self) -> Result<Value, ProtocolError> {
        match self {
            ExtendedDocument::V0(v0) => v0.into_value(),
        }
    }

    /// Convert the extended document to a Value instance.
    ///
    /// This function is a passthrough to the `to_value` method.
    #[cfg(feature = "document-value-conversion")]
    pub fn to_value(&self) -> Result<Value, ProtocolError> {
        match self {
            ExtendedDocument::V0(v0) => v0.to_value(),
        }
    }

    /// Convert the extended document to a JSON object for validation.
    ///
    /// This function is a passthrough to the `to_json_object_for_validation` method.
    #[cfg(feature = "document-json-conversion")]
    pub fn to_json_object_for_validation(&self) -> Result<JsonValue, ProtocolError> {
        match self {
            ExtendedDocument::V0(v0) => v0.to_json_object_for_validation(),
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

    /// Retrieve the field specified by the path.
    ///
    /// This function is a passthrough to the `get` method.
    pub fn get(&self, path: &str) -> Option<&Value> {
        match self {
            ExtendedDocument::V0(v0) => v0.get(path),
        }
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use serde_json::{json, Value as JsonValue};
    use std::convert::TryInto;

    use crate::document::extended_document::{ExtendedDocument, IDENTIFIER_FIELDS};

    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::DataContract;
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

    use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use crate::data_contract::document_type::random_document::CreateRandomDocument;
    use crate::document::serialization_traits::ExtendedDocumentPlatformConversionMethodsV0;
    use crate::tests::fixtures::get_dashpay_contract_fixture;

    fn init() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }
    pub(crate) fn data_contract_with_dynamic_properties() -> DataContract {
        let platform_version = PlatformVersion::latest();
        // The following is equivalent to the data contract
        // {
        //     "protocolVersion" :0,
        //     "$id" : vec![0_u8;32],
        //     "$schema" : "schema",
        //     "version" : 0,
        //     "ownerId" : vec![0_u8;32],
        //     "documents" : {
        //         "test" : {
        //             "properties" : {
        //                 "alphaIdentifier" :  {
        //                     "type": "array",
        //                     "byteArray": true,
        //                     "contentMediaType": "application/x.dash.dpp.identifier",
        //                 },
        //                 "alphaBinary" :  {
        //                     "type": "array",
        //                     "byteArray": true,
        //                 }
        //             }
        //         }
        //     }
        // }
        let test_document_properties_alpha_identifier = Value::from([
            ("type", Value::Text("array".to_string())),
            ("byteArray", Value::Bool(true)),
            (
                "contentMediaType",
                Value::Text("application/x.dash.dpp.identifier".to_string()),
            ),
        ]);
        let test_document_properties_alpha_binary = Value::from([
            ("type", Value::Text("array".to_string())),
            ("byteArray", Value::Bool(true)),
        ]);
        let test_document_properties = Value::from([
            ("alphaIdentifier", test_document_properties_alpha_identifier),
            ("alphaBinary", test_document_properties_alpha_binary),
        ]);
        let test_document = Value::from([("properties", test_document_properties)]);
        let documents = Value::from([("test", test_document)]);

        DataContract::from_value(
            Value::from([
                ("protocolVersion", Value::U32(1)),
                ("$id", Value::Identifier([0_u8; 32])),
                ("$schema", Value::Text("schema".to_string())),
                ("version", Value::U32(0)),
                ("ownerId", Value::Identifier([0_u8; 32])),
                ("documents", documents),
            ]),
            &platform_version,
        )
        .unwrap()
    }

    #[test]
    #[cfg(feature = "document-json-conversion")]
    fn test_document_deserialize() -> Result<()> {
        init();
        let platform_version = PlatformVersion::latest();
        let dpns_contract =
            load_system_data_contract(SystemDataContract::DPNS, platform_version.protocol_version)?;
        let document_json = get_data_from_file("src/tests/payloads/document_dpns.json")?;
        let doc =
            ExtendedDocument::from_json_string(&document_json, dpns_contract, &platform_version)?;
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
        assert_eq!(
            doc.properties()
                .get_at_path("records.dashUniqueIdentityId")
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
            .serialize(PlatformVersion::latest())
            .expect("no errors");

        let doc =
            ExtendedDocument::from_bytes(buffer_document.as_slice(), PlatformVersion::latest())
                .expect("document should be created from buffer");

        assert_eq!(init_doc.created_at(), doc.created_at());
        assert_eq!(init_doc.updated_at(), doc.updated_at());
        assert_eq!(init_doc.id(), doc.id());
        assert_eq!(init_doc.data_contract_id(), doc.data_contract_id());
        assert_eq!(init_doc.owner_id(), doc.owner_id());
    }

    #[test]
    fn test_to_object() {
        init();
        let dpns_contract = load_system_data_contract(
            SystemDataContract::DPNS,
            LATEST_PLATFORM_VERSION.protocol_version,
        )
        .unwrap();
        let document_json = get_data_from_file("src/tests/payloads/document_dpns.json").unwrap();
        let document = ExtendedDocument::from_json_string(
            &document_json,
            dpns_contract,
            LATEST_PLATFORM_VERSION,
        )
        .unwrap();
        let document_object = document.to_json_object_for_validation().unwrap();

        for property in IDENTIFIER_FIELDS {
            let id = document_object
                .get(property)
                .unwrap()
                .as_array()
                .expect("the property must be an array");
            assert_eq!(32, id.len())
        }
    }

    #[test]
    fn test_json_serialize() -> Result<()> {
        init();

        let dpns_contract = load_system_data_contract(
            SystemDataContract::DPNS,
            LATEST_PLATFORM_VERSION.protocol_version,
        )?;
        let document_json = get_data_from_file("src/tests/payloads/document_dpns.json")?;
        let document = ExtendedDocument::from_json_string(
            &document_json,
            dpns_contract,
            LATEST_PLATFORM_VERSION,
        )?;
        let string = serde_json::to_string(&document)?;

        assert_eq!("{\"$protocolVersion\":0,\"$type\":\"domain\",\"$dataContractId\":\"566vcJkmebVCAb2Dkj2yVMSgGFcsshupnQqtsz1RFbcy\",\"$id\":\"4veLBZPHDkaCPF9LfZ8fX3JZiS5q5iUVGhdBbaa9ga5E\",\"$ownerId\":\"HBNMY5QWuBVKNFLhgBTC1VmpEnscrmqKPMXpnYSHwhfn\",\"label\":\"user-9999\",\"normalizedLabel\":\"user-9999\",\"normalizedParentDomainName\":\"dash\",\"preorderSalt\":\"BzQi567XVqc8wYiVHS887sJtL6MDbxLHNnp+UpTFSB0=\",\"records\":{\"dashUniqueIdentityId\":\"HBNMY5QWuBVKNFLhgBTC1VmpEnscrmqKPMXpnYSHwhfn\"},\"subdomainRules\":{\"allowSubdomains\":false},\"$revision\":1,\"$createdAt\":null,\"$updatedAt\":null}", string);

        Ok(())
    }

    #[test]
    fn test_document_to_buffer() -> Result<()> {
        init();

        let document_json = get_data_from_file("src/tests/payloads/document_dpns.json")?;
        let dpns_contract = load_system_data_contract(
            SystemDataContract::DPNS,
            LATEST_PLATFORM_VERSION.protocol_version,
        )
        .unwrap();
        ExtendedDocument::from_json_string(&document_json, dpns_contract, LATEST_PLATFORM_VERSION)
            .expect("expected extended document");
        Ok(())
    }

    #[test]
    fn json_should_generate_human_readable_binaries() {
        let data_contract = data_contract_with_dynamic_properties();
        let alpha_value = vec![10_u8; 32];
        let id = vec![11_u8; 32];
        let owner_id = vec![12_u8; 32];
        let data_contract_id = vec![13_u8; 32];

        let raw_document = json!({
            "$protocolVersion"  : 0,
            "$id" : id,
            "$ownerId" : owner_id,
            "$type" : "test",
            "$dataContractId" : data_contract_id,
            "$revision" : 1,
            "alphaBinary" : alpha_value,
            "alphaIdentifier" : alpha_value,
        });

        let document = ExtendedDocument::from_raw_json_document(
            raw_document,
            data_contract,
            LATEST_PLATFORM_VERSION,
        )
        .unwrap();
        let json_document = document
            .to_pretty_json(LATEST_PLATFORM_VERSION)
            .expect("no errors");

        assert_eq!(
            json_document["$id"],
            JsonValue::String(bs58::encode(&id).into_string())
        );
        assert_eq!(
            json_document["$ownerId"],
            JsonValue::String(bs58::encode(&owner_id).into_string())
        );
        assert_eq!(
            json_document["$dataContractId"],
            JsonValue::String(bs58::encode(&data_contract_id).into_string())
        );
        assert_eq!(
            json_document["alphaBinary"],
            JsonValue::String(base64::encode(&alpha_value))
        );
        assert_eq!(
            json_document["alphaIdentifier"],
            JsonValue::String(bs58::encode(&alpha_value).into_string())
        );
    }

    fn document_bytes() -> Vec<u8> {
        new_example_document()
            .serialize(LATEST_PLATFORM_VERSION)
            .unwrap()
    }

    fn new_example_document() -> ExtendedDocument {
        let data_contract =
            get_dashpay_contract_fixture(None, LATEST_PLATFORM_VERSION.protocol_version)
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
        }
        .into()
    }
}
