pub mod document_transition;
pub mod errors;

use crate::data_contract::DataContract;
use crate::errors::ProtocolError;
use crate::identifier::Identifier;
use crate::metadata::Metadata;
use crate::util::deserializer;
use crate::util::hash::sha;
use crate::util::serializer;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub data: serde_json::Value,
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
    pub fn to_json(&self) -> Result<Value, ProtocolError> {
        serde_json::to_value(&self)
            .map_err(|e| ProtocolError::EncodingError(format!("corrupted data - {}", e)))
    }

    pub fn from_buffer(b: impl AsRef<[u8]>) -> Result<Document, ProtocolError> {
        let (protocol_bytes, document_bytes) = b.as_ref().split_at(4);

        let json_value: Value = ciborium::de::from_reader(document_bytes)
            .map_err(|e| ProtocolError::EncodingError(format!("{}", e)))?;

        let mut json_map = if let Value::Object(v) = json_value {
            v
        } else {
            return Err(ProtocolError::EncodingError(String::from(
                "input data cannot be parsed into the map",
            )));
        };

        deserializer::parse_protocol_version(protocol_bytes, &mut json_map)?;
        deserializer::parse_identities(&mut json_map, &["$id", "$dataContractId", "$ownerId"])?;

        let document: Document = serde_json::from_value(Value::Object(json_map))?;
        Ok(document)
    }

    pub fn to_buffer(&self) -> Result<Vec<u8>, ProtocolError> {
        let protocol_version = self.protocol_version;
        let id: Vec<Value> = self
            .id
            .to_buffer()
            .iter()
            .map(|v| Value::from(*v))
            .collect();

        let data_contract_id: Vec<Value> = self
            .data_contract_id
            .to_buffer()
            .iter()
            .map(|v| Value::from(*v))
            .collect();

        let owner_id: Vec<Value> = self
            .owner_id
            .to_buffer()
            .iter()
            .map(|v| Value::from(*v))
            .collect();

        let mut json_value = serde_json::to_value(&self)
            .map_err(|e| ProtocolError::EncodingError(format!("corrupted data - {}", e)))?;

        match json_value {
            Value::Object(ref mut o) => {
                o.insert(String::from("$id"), Value::Array(id));
                o.insert(
                    String::from("$dataContractId"),
                    Value::Array(data_contract_id),
                );
                o.insert(String::from("$ownerId"), Value::Array(owner_id));
                o.remove("$protocolVersion");
            }
            _ => {
                return Err(ProtocolError::EncodingError(String::from(
                    "corrupted data: document is not an object",
                )))
            }
        };

        serializer::value_to_cbor(json_value, Some(protocol_version))
    }

    pub fn hash(&self) -> Result<Vec<u8>, ProtocolError> {
        Ok(sha(self.to_buffer()?))
    }

    pub fn set_value(_path: &str, _value: Value) -> Result<(), ProtocolError> {
        unimplemented!()
    }

    pub fn get(_path: &str, _value: Value) -> Option<&Value> {
        unimplemented!()
    }
}

#[cfg(test)]
mod test {
    use crate::tests::utils::*;
    use crate::util::string_encoding::Encoding;

    use super::*;
    use anyhow::Result;
    use chrono::Utc;
    use log::debug;
    use serde_json::Value;

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

        debug!("the dynamic data is {:?}", doc.data);
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

        let json = serde_json::to_string(&document)?;
        debug!("serialized document: {}", json);

        Ok(())
    }

    #[test]
    fn test_document_to_buffer() -> Result<()> {
        init();

        let document_json = get_data_from_file("src/tests/payloads/document_dpns.json")?;
        let document: Document = serde_json::from_str::<Document>(&document_json)?;

        debug!("{:?}", document.to_buffer());
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
