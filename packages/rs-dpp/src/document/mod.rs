use std::convert::TryInto;

use ciborium::value::Value as CborValue;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

pub use state_transition::documents_batch_transition::document_transition;
pub use state_transition::documents_batch_transition::validation;
pub use state_transition::documents_batch_transition::DocumentsBatchTransition;

use crate::data_contract::DataContract;
use crate::errors::ProtocolError;
use crate::identifier::Identifier;
use crate::metadata::Metadata;
use crate::util::cbor_value;
use crate::util::cbor_value::CborCanonicalMap;
use crate::util::cbor_value::FieldType;
use crate::util::hash::hash;
use crate::util::json_value::{JsonValueExt, ReplaceWith};

pub mod document_factory;
pub mod document_validator;
pub mod errors;
pub mod fetch_and_validate_data_contract;
pub mod generate_document_id;
pub mod state_transition;

pub mod property_names {
    pub const PROTOCOL_VERSION: &str = "$protocolVersion";
    pub const ID: &str = "$id";
    pub const DOCUMENT_TYPE: &str = "$type";
    pub const REVISION: &str = "$revision";
    pub const DATA_CONTRACT_ID: &str = "$dataContractId";
    pub const OWNER_ID: &str = "$ownerId";
    pub const CREATED_AT: &str = "$createdAt";
    pub const UPDATED_AT: &str = "$updatedAt";
}

pub const IDENTIFIER_FIELDS: [&str; 3] = [
    property_names::ID,
    property_names::DATA_CONTRACT_ID,
    property_names::OWNER_ID,
];

/// The document object represents the data provided by the platform in response to a query.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Document {
    #[serde(rename = "$protocolVersion")]
    pub protocol_version: u32,
    #[serde(rename = "$id")]
    pub id: Identifier,
    #[serde(rename = "$type")]
    pub document_type: String,
    #[serde(rename = "$revision")]
    pub revision: u32,
    #[serde(rename = "$dataContractId")]
    pub data_contract_id: Identifier,
    #[serde(rename = "$ownerId")]
    pub owner_id: Identifier,
    #[serde(rename = "$createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(rename = "$updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<i64>,
    // the serde_json::Value preserves the order (see .toml file)
    #[serde(flatten)]
    pub data: JsonValue,
    #[serde(skip)]
    pub data_contract: DataContract,
    #[serde(skip)]
    pub metadata: Option<Metadata>,
    #[serde(skip)]
    pub entropy: [u8; 32],
}

impl Document {
    /// Creates a Document from the json form. Json format contains strings instead of
    /// arrays of u8 (bytes)
    pub fn from_json_document(
        json_document: JsonValue,
        data_contract: DataContract,
    ) -> Result<Document, ProtocolError> {
        let mut document = Self::from_value::<String>(json_document, data_contract)?;
        let mut document_data = document.data.take();

        // replace only the dynamic data
        let (identifier_paths, binary_paths) = document.get_identifiers_and_binary_paths()?;
        document_data.replace_binary_paths(binary_paths, ReplaceWith::Base64)?;
        document_data.replace_identifier_paths(identifier_paths, ReplaceWith::Base58)?;

        document.data = document_data;
        Ok(document)
    }

    pub fn from_raw_document(
        raw_document: JsonValue,
        data_contract: DataContract,
    ) -> Result<Document, ProtocolError> {
        Self::from_value::<Vec<u8>>(raw_document, data_contract)
    }

    fn from_value<S>(
        mut document_value: JsonValue,
        data_contract: DataContract,
    ) -> Result<Document, ProtocolError>
    where
        for<'de> S: Deserialize<'de> + TryInto<Identifier, Error = ProtocolError>,
    {
        let mut document = Document {
            data_contract,
            ..Default::default()
        };

        if let Ok(value) = document_value.remove(property_names::PROTOCOL_VERSION) {
            document.protocol_version = serde_json::from_value(value)?
        }
        if let Ok(value) = document_value.remove(property_names::ID) {
            let data: S = serde_json::from_value(value)?;
            document.id = data.try_into()?;
        }
        if let Ok(value) = document_value.remove(property_names::DOCUMENT_TYPE) {
            document.document_type = serde_json::from_value(value)?
        }
        if let Ok(value) = document_value.remove(property_names::DATA_CONTRACT_ID) {
            let data: S = serde_json::from_value(value)?;
            document.data_contract_id = data.try_into()?
        }
        if let Ok(value) = document_value.remove(property_names::OWNER_ID) {
            let data: S = serde_json::from_value(value)?;
            document.owner_id = data.try_into()?
        }
        if let Ok(value) = document_value.remove(property_names::REVISION) {
            document.revision = serde_json::from_value(value)?
        }
        if let Ok(value) = document_value.remove(property_names::CREATED_AT) {
            document.created_at = serde_json::from_value(value)?
        }
        if let Ok(value) = document_value.remove(property_names::UPDATED_AT) {
            document.updated_at = serde_json::from_value(value)?
        }

        document.data = document_value;
        Ok(document)
    }

    pub fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        let mut value = serde_json::to_value(self)?;

        let (identifier_paths, binary_paths) = self
            .data_contract
            .get_identifiers_and_binary_paths(&self.document_type)?;

        value.replace_identifier_paths(identifier_paths, ReplaceWith::Base58)?;
        value.replace_binary_paths(binary_paths, ReplaceWith::Base64)?;

        Ok(value)
    }

    pub fn from_buffer(cbor_bytes: impl AsRef<[u8]>) -> Result<Document, ProtocolError> {
        let (protocol_version_bytes, document_cbor_bytes) = cbor_bytes.as_ref().split_at(4);

        let cbor_value: CborValue = ciborium::de::from_reader(document_cbor_bytes)
            .map_err(|e| ProtocolError::EncodingError(format!("{}", e)))?;

        let mut json_value = cbor_value::cbor_value_to_json_value(&cbor_value)?;

        json_value.parse_and_add_protocol_version(
            property_names::PROTOCOL_VERSION,
            protocol_version_bytes,
        )?;
        json_value.replace_identifier_paths(IDENTIFIER_FIELDS, ReplaceWith::Base58)?;

        let document: Document = serde_json::from_value(json_value)?;

        Ok(document)
    }

    // The skipIdentifierConversion option is removed as it doesn't make sense in the case of
    // of Rust. Rust doesn't distinguish between `Buffer` and `Identifier`
    pub fn to_object(&self) -> Result<JsonValue, ProtocolError> {
        let mut json_object = serde_json::to_value(self)?;

        let (identifier_paths, binary_paths) = self.get_identifiers_and_binary_paths()?;
        let _ = json_object.replace_identifier_paths(identifier_paths, ReplaceWith::Bytes);
        let _ = json_object.replace_binary_paths(binary_paths, ReplaceWith::Bytes);

        Ok(json_object)
    }

    pub fn to_buffer(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut result_buf = self.protocol_version.to_le_bytes().to_vec();

        let map = CborValue::serialized(&self)
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

        let mut canonical_map: CborCanonicalMap = map.try_into()?;

        canonical_map.remove(property_names::PROTOCOL_VERSION);

        if self.updated_at.is_none() {
            canonical_map.remove(property_names::UPDATED_AT);
        }

        let (identifier_paths, binary_paths) = self
            .data_contract
            .get_identifiers_and_binary_paths(&self.document_type)?;

        // The static (part of structure) identifiers are being serialized to the String(base58)
        canonical_map.replace_values(IDENTIFIER_FIELDS, ReplaceWith::Bytes);
        // The DYNAMIC identifiers and binary fields are being serialized to the ArrayInt, therefore
        // they both need to be converted to the the CborValue::Bytes
        canonical_map.replace_paths(
            identifier_paths.into_iter().chain(binary_paths),
            FieldType::ArrayInt,
            FieldType::Bytes,
        );

        let mut document_buffer = canonical_map
            .to_bytes()
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

        result_buf.append(&mut document_buffer);

        Ok(result_buf)
    }

    pub fn hash(&self) -> Result<Vec<u8>, ProtocolError> {
        Ok(hash(self.to_buffer()?))
    }

    /// Set the value under given path.
    /// The path supports syntax from `lodash` JS lib. Example: "root.people[0].name".
    /// If parents are not present they will be automatically created
    pub fn set(&mut self, path: &str, value: JsonValue) -> Result<(), ProtocolError> {
        Ok(self.data.insert_with_path(path, value)?)
    }

    /// Retrieves field specified by path
    pub fn get(&self, path: &str) -> Option<&JsonValue> {
        match self.data.get_value(path) {
            Ok(v) => Some(v),
            Err(_) => None,
        }
    }

    /// Get the Document's data
    pub fn get_data(&self) -> &JsonValue {
        &self.data
    }

    /// Set the Document's data
    pub fn set_data(&mut self, data: JsonValue) {
        self.data = data;
    }

    /// Get entropy
    pub fn get_entropy(&self) -> &[u8] {
        &self.entropy
    }

    pub fn get_identifiers_and_binary_paths(
        &self,
    ) -> Result<(Vec<&str>, Vec<&str>), ProtocolError> {
        let (identifiers_paths, binary_paths) = self
            .data_contract
            .get_identifiers_and_binary_paths(&self.document_type)?;

        Ok((
            identifiers_paths
                .into_iter()
                .chain(IDENTIFIER_FIELDS)
                .unique()
                .collect(),
            binary_paths,
        ))
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use serde_json::{json, Value};

    use super::*;
    use crate::tests::utils::*;
    use crate::util::string_encoding::Encoding;
    use pretty_assertions::assert_eq;

    fn init() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }

    fn data_contract_with_dynamic_properties() -> DataContract {
        let data_contract = json!({
            "protocolVersion" :0,
            "$id" : vec![0_u8;32],
            "$schema" : "schema",
            "version" : 0,
            "ownerId" : vec![0_u8;32],
            "documents" : {
                "test" : {
                    "properties" : {
                        "alphaIdentifier" :  {
                            "type": "array",
                            "byteArray": true,
                            "contentMediaType": "application/x.dash.dpp.identifier",
                        },
                        "alphaBinary" :  {
                            "type": "array",
                            "byteArray": true,
                        }
                    }
                }
            }
        });
        DataContract::from_raw_object(data_contract).unwrap()
    }

    #[test]
    fn test_document_deserialize() -> Result<()> {
        init();
        let document_json = get_data_from_file("src/tests/payloads/document_dpns.json")?;
        let doc = serde_json::from_str::<Document>(&document_json)?;
        assert_eq!(doc.document_type, "domain");
        assert_eq!(doc.protocol_version, 0);
        assert_eq!(
            doc.id.to_buffer(),
            Identifier::from_string(
                "4veLBZPHDkaCPF9LfZ8fX3JZiS5q5iUVGhdBbaa9ga5E",
                Encoding::Base58
            )
            .unwrap()
            .to_buffer()
        );
        assert_eq!(
            doc.data_contract_id.to_buffer(),
            Identifier::from_string(
                "566vcJkmebVCAb2Dkj2yVMSgGFcsshupnQqtsz1RFbcy",
                Encoding::Base58
            )
            .unwrap()
            .to_buffer()
        );

        assert_eq!(doc.data["label"], Value::String("user-9999".to_string()));
        assert_eq!(
            doc.data["records"]["dashUniqueIdentityId"],
            Value::String("HBNMY5QWuBVKNFLhgBTC1VmpEnscrmqKPMXpnYSHwhfn".to_string())
        );
        assert_eq!(
            doc.data["subdomainRules"]["allowSubdomains"],
            Value::Bool(false)
        );
        Ok(())
    }

    #[test]
    fn test_buffer_serialize_deserialize() {
        init();
        let init_doc = new_example_document();
        let buffer_document = init_doc.to_buffer().expect("no errors");

        let doc = Document::from_buffer(&buffer_document)
            .expect("document should be created from buffer");

        assert_eq!(init_doc.created_at, doc.created_at);
        assert_eq!(init_doc.updated_at, doc.updated_at);
        assert_eq!(init_doc.id, doc.id);
        assert_eq!(init_doc.data_contract_id, doc.data_contract_id);
        assert_eq!(init_doc.owner_id, doc.owner_id);
    }

    #[test]
    fn test_to_object() {
        init();
        let document_json = get_data_from_file("src/tests/payloads/document_dpns.json").unwrap();
        let document = serde_json::from_str::<Document>(&document_json).unwrap();
        let document_object = document.to_object().unwrap();

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

        let document_json = get_data_from_file("src/tests/payloads/document_dpns.json")?;
        let document = serde_json::from_str::<Document>(&document_json)?;

        serde_json::to_string(&document)?;
        Ok(())
    }

    #[test]
    fn test_document_to_buffer() -> Result<()> {
        init();

        let document_json = get_data_from_file("src/tests/payloads/document_dpns.json")?;
        serde_json::from_str::<Document>(&document_json)?;
        Ok(())
    }

    #[test]
    fn deserialize_js_cpp_cbor() -> Result<()> {
        let document_cbor = document_cbor_bytes();

        let document = Document::from_buffer(&document_cbor)?;

        assert_eq!(document.protocol_version, 1);
        assert_eq!(
            document.id.to_buffer().to_vec(),
            vec![
                113, 93, 61, 101, 117, 96, 36, 162, 222, 10, 177, 178, 187, 30, 131, 181, 239, 41,
                123, 240, 198, 250, 97, 106, 173, 92, 136, 126, 79, 16, 222, 249
            ]
        );
        assert_eq!(&document.document_type, "niceDocument");
        assert_eq!(
            document.data_contract_id.to_buffer().to_vec(),
            vec![
                122, 188, 95, 154, 180, 188, 208, 97, 46, 214, 202, 206, 194, 4, 221, 109, 116, 17,
                165, 97, 39, 212, 36, 138, 241, 234, 218, 203, 147, 82, 93, 162
            ]
        );
        assert_eq!(
            document.owner_id.to_buffer().to_vec(),
            vec![
                182, 191, 55, 77, 48, 47, 190, 43, 81, 27, 67, 226, 61, 3, 63, 150, 94, 46, 51,
                160, 36, 199, 65, 157, 176, 117, 51, 212, 186, 125, 112, 142
            ]
        );
        assert_eq!(document.revision, 1);
        assert_eq!(document.created_at.unwrap(), 1656583332347);
        assert_eq!(document.data.get("name").unwrap(), "Cutie");

        Ok(())
    }

    #[test]
    fn to_buffer_serialize_to_the_same_format_as_js_dpp() -> Result<()> {
        let document_cbor = document_cbor_bytes();
        let document = Document::from_buffer(&document_cbor)?;

        let buffer = document.to_buffer()?;

        assert_eq!(document_cbor, buffer);
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
            "revision" : 1,
            "alphaBinary" : alpha_value,
            "alphaIdentifier" : alpha_value,
        });

        let document = Document::from_raw_document(raw_document, data_contract).unwrap();
        let json_document = document.to_json().expect("no errors");

        assert_eq!(
            json_document["$id"],
            Value::String(bs58::encode(&id).into_string())
        );
        assert_eq!(
            json_document["$ownerId"],
            Value::String(bs58::encode(&owner_id).into_string())
        );
        assert_eq!(
            json_document["$dataContractId"],
            Value::String(bs58::encode(&data_contract_id).into_string())
        );
        assert_eq!(
            json_document["alphaBinary"],
            Value::String(base64::encode(&alpha_value))
        );
        assert_eq!(
            json_document["alphaIdentifier"],
            Value::String(bs58::encode(&alpha_value).into_string())
        );
    }

    fn document_cbor_bytes() -> Vec<u8> {
        hex::decode("01000000a7632469645820715d3d65756024a2de0ab1b2bb1e83b5ef297bf0c6fa616aad5c887e4f10def9646e616d656543757469656524747970656c6e696365446f63756d656e7468246f776e657249645820b6bf374d302fbe2b511b43e23d033f965e2e33a024c7419db07533d4ba7d708e69247265766973696f6e016a246372656174656441741b00000181b40fa1fb6f2464617461436f6e7472616374496458207abc5f9ab4bcd0612ed6cacec204dd6d7411a56127d4248af1eadacb93525da2").unwrap()
    }

    fn new_example_document() -> Document {
        Document {
            id: Identifier::from_bytes(&generate_random_identifier()).unwrap(),
            owner_id: Identifier::from_bytes(&generate_random_identifier()).unwrap(),
            data_contract_id: Identifier::from_bytes(&generate_random_identifier()).unwrap(),
            created_at: Some(1648013404492),
            updated_at: Some(1648013404492),
            ..Default::default()
        }
    }
}
