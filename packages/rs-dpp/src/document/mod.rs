pub mod errors;
pub mod generate_document_id;
mod state_transition;
pub use state_transition::documents_batch_transition::document_transition;
pub use state_transition::documents_batch_transition::validation;

use crate::data_contract::DataContract;
use crate::errors::ProtocolError;
use crate::identifier::Identifier;
use crate::metadata::Metadata;
use crate::util::deserializer;
use crate::util::hash::hash;
use crate::util::json_value::{JsonValueExt, ReplaceWith};
use crate::util::serializer;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

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
    #[serde(rename = "$createdAt")]
    pub created_at: Option<i64>,
    #[serde(rename = "$updatedAt")]
    pub updated_at: Option<i64>,
    // the serde_json::Value preserves the order (see .toml file)
    #[serde(flatten)]
    pub data: JsonValue,
    #[serde(skip)]
    pub data_contract: DataContract,
    #[serde(skip)]
    pub metadata: Option<Metadata>,
    #[serde(skip)]
    pub entropy: Option<[u8; 32]>,
}

// per https://www.reddit.com/r/rust/comments/d7w6n7/is_it_idiomatic_to_write_setters_and_getters/
// we don't want to use getters and setters
impl Document {
    pub fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        Ok(serde_json::to_value(&self)?)
    }

    pub fn from_buffer(b: impl AsRef<[u8]>) -> Result<Document, ProtocolError> {
        let (protocol_bytes, document_bytes) = b.as_ref().split_at(4);

        let json_value: JsonValue = ciborium::de::from_reader(document_bytes)
            .map_err(|e| ProtocolError::EncodingError(format!("{}", e)))?;

        let mut json_map = if let JsonValue::Object(v) = json_value {
            v
        } else {
            return Err(ProtocolError::EncodingError(String::from(
                "input data cannot be parsed into the map",
            )));
        };

        deserializer::parse_protocol_version(protocol_bytes, &mut json_map)?;
        deserializer::parse_identities(&mut json_map, &IDENTIFIER_FIELDS)?;

        let document: Document = serde_json::from_value(JsonValue::Object(json_map))?;
        Ok(document)
    }

    pub fn to_object(&self, skip_identifiers_conversion: bool) -> Result<JsonValue, ProtocolError> {
        let mut json_object = serde_json::to_value(&self)?;
        if !skip_identifiers_conversion {
            json_object.replace_identifier_paths(IDENTIFIER_FIELDS, ReplaceWith::Bytes)?;
        }
        Ok(json_object)
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

    pub fn set_value(&self, _path: &str, _value: JsonValue) -> Result<(), ProtocolError> {
        unimplemented!()
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
    use crate::tests::utils::*;
    use crate::util::string_encoding::Encoding;

    use super::*;
    use anyhow::Result;
    use chrono::Utc;
    use serde_json::Value;

    fn init() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Trace)
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

    fn new_example_document() -> Document {
        Document {
            id: Identifier::from_bytes(&generate_random_identifier()).unwrap(),
            owner_id: Identifier::from_bytes(&generate_random_identifier()).unwrap(),
            data_contract_id: Identifier::from_bytes(&generate_random_identifier()).unwrap(),
            created_at: Some(Utc::now().timestamp_millis()),
            updated_at: Some(Utc::now().timestamp_millis()),
            ..Default::default()
        }
    }
}
