use std::convert::TryInto;

use ciborium::value::Value as CborValue;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

pub use state_transition::documents_batch_transition::document_transition;
pub use state_transition::documents_batch_transition::validation;
pub use state_transition::documents_batch_transition::DocumentsBatchTransition;

use crate::data_contract::DataContract;
use crate::errors::ProtocolError;
use crate::identifier::Identifier;
use crate::metadata::Metadata;
use crate::util::cbor_value::CborCanonicalMap;
use crate::util::hash::hash;
use crate::util::json_value::{JsonValueExt, ReplaceWith};
use crate::util::{cbor_value, serializer};

pub mod document_factory;
pub mod errors;
pub mod generate_document_id;
mod state_transition;

const IDENTIFIER_FIELDS: [&str; 3] = ["$id", "$dataContractId", "$ownerId"];

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
    pub revision: i64,
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
    pub fn from_raw_document(
        mut raw_document: JsonValue,
        data_contract: DataContract,
    ) -> Result<Document, ProtocolError> {
        let mut document = Document {
            data_contract,
            ..Default::default()
        };

        if let Ok(value) = raw_document.remove("$protocolVersion") {
            document.protocol_version = serde_json::from_value(value)?
        }
        if let Ok(value) = raw_document.remove("$id") {
            let identifier_bytes: Vec<u8> = serde_json::from_value(value)?;
            document.id = Identifier::from_bytes(&identifier_bytes)?
        }
        if let Ok(value) = raw_document.remove("$type") {
            document.document_type = serde_json::from_value(value)?
        }
        if let Ok(value) = raw_document.remove("$dataContractId") {
            let identifier_bytes: Vec<u8> = serde_json::from_value(value)?;
            document.data_contract_id = Identifier::from_bytes(&identifier_bytes)?
        }
        if let Ok(value) = raw_document.remove("$ownerId") {
            let identifier_bytes: Vec<u8> = serde_json::from_value(value)?;
            document.owner_id = Identifier::from_bytes(&identifier_bytes)?
        }
        if let Ok(value) = raw_document.remove("$revision") {
            document.revision = serde_json::from_value(value)?
        }
        if let Ok(value) = raw_document.remove("$createdAt") {
            document.created_at = serde_json::from_value(value)?
        }
        if let Ok(value) = raw_document.remove("$updatedAt") {
            document.updated_at = serde_json::from_value(value)?
        }

        document.data = raw_document;
        Ok(document)
    }

    pub fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        Ok(serde_json::to_value(&self)?)
    }

    pub fn from_buffer(b: impl AsRef<[u8]>) -> Result<Document, ProtocolError> {
        let (protocol_bytes, document_bytes) = b.as_ref().split_at(4);

        let mut json_value: JsonValue = ciborium::de::from_reader(document_bytes)
            .map_err(|e| ProtocolError::EncodingError(format!("{}", e)))?;

        json_value.parse_and_add_protocol_version("$protocolVersion", protocol_bytes)?;
        // TODO identifiers and binary data for dynamic values
        json_value.replace_identifier_paths(IDENTIFIER_FIELDS, ReplaceWith::Base58)?;

        let document: Document = serde_json::from_value(json_value)?;
        Ok(document)
    }

    pub fn from_cbor(cbor_bytes: impl AsRef<[u8]>) -> Result<Document, ProtocolError> {
        let (protocol_version_bytes, document_cbor_bytes) = cbor_bytes.as_ref().split_at(4);

        let cbor_value: CborValue = ciborium::de::from_reader(document_cbor_bytes)
            .map_err(|e| ProtocolError::EncodingError(format!("{}", e)))?;

        let mut json_value = cbor_value::cbor_value_to_json_value(&cbor_value)?;

        json_value.parse_and_add_protocol_version("$protocolVersion", protocol_version_bytes)?;
        // TODO identifiers and binary data for dynamic values
        json_value.replace_identifier_paths(IDENTIFIER_FIELDS, ReplaceWith::Base58)?;

        let document: Document = serde_json::from_value(json_value)?;

        Ok(document)
    }

    pub fn to_object(&self, skip_identifiers_conversion: bool) -> Result<JsonValue, ProtocolError> {
        let mut json_object = serde_json::to_value(&self)?;
        if !skip_identifiers_conversion {
            // TODO identifiers and binary data for dynamic values
            json_object.replace_identifier_paths(IDENTIFIER_FIELDS, ReplaceWith::Bytes)?;
        }
        Ok(json_object)
    }

    pub fn to_cbor(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut result_buf = self.protocol_version.to_le_bytes().to_vec();

        let map = CborValue::serialized(&self)
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

        let mut canonical_map: CborCanonicalMap = map.try_into()?;

        canonical_map.remove("$protocolVersion");

        if self.updated_at.is_none() {
            canonical_map.remove("$updatedAt");
        }

        canonical_map.replace_values(IDENTIFIER_FIELDS, ReplaceWith::Bytes);

        let mut document_buffer = canonical_map
            .to_bytes()
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

        result_buf.append(&mut document_buffer);

        Ok(result_buf)
    }

    pub fn to_buffer(&self) -> Result<Vec<u8>, ProtocolError> {
        let protocol_version = self.protocol_version;
        let mut json_object = self.to_object(false)?;

        if let JsonValue::Object(ref mut o) = json_object {
            o.remove("$protocolVersion");
        };

        serializer::value_to_cbor(json_object, Some(protocol_version))
    }

    pub fn hash(&self) -> Result<Vec<u8>, ProtocolError> {
        Ok(hash(self.to_buffer()?))
    }

    pub fn set_value(&mut self, property: &str, value: JsonValue) -> Result<(), ProtocolError> {
        Ok(self.data.insert(property.to_string(), value)?)
    }

    /// Retrieves field specified by path
    pub fn get(&self, path: &str) -> Option<&JsonValue> {
        match self.data.get_value(path) {
            Ok(v) => Some(v),
            Err(_) => None,
        }
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use serde_json::Value;

    use crate::tests::utils::*;
    use crate::util::string_encoding::Encoding;

    use super::*;

    fn init() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .try_init();
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
    fn test_buffer_serialize_deserialize() -> Result<()> {
        init();
        let init_doc = new_example_document();
        let buffer_document = init_doc.to_buffer()?;

        let doc = Document::from_buffer(&buffer_document)?;

        assert_eq!(init_doc.created_at, doc.created_at);
        assert_eq!(init_doc.updated_at, doc.updated_at);
        assert_eq!(init_doc.id, doc.id);
        assert_eq!(init_doc.data_contract_id, doc.data_contract_id);
        assert_eq!(init_doc.owner_id, doc.owner_id);

        Ok(())
    }

    #[test]
    fn test_to_object() {
        init();
        let document_json = get_data_from_file("src/tests/payloads/document_dpns.json").unwrap();
        let document = serde_json::from_str::<Document>(&document_json).unwrap();
        let document_object = document.to_object(false).unwrap();

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

        let document = Document::from_cbor(&document_cbor)?;

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

        let document = Document::from_cbor(&document_cbor)?;

        let buffer = document.to_cbor()?;

        assert_eq!(document_cbor, buffer);

        Ok(())
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
