use super::DataContractError;
use crate::util::string_encoding::Encoding;
use crate::{
    errors::ProtocolError,
    identifier::Identifier,
    metadata::Metadata,
    util::{hash::sha, serializer},
};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;

// TODO probably this need to be changed
pub type JsonSchema = JsonValue;

pub const SCHEMA: &'static str = "https://schema.dash.org/dpp-0-4-0/meta/data-contract";

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DataContract {
    pub protocol_version: u32,
    #[serde(rename = "$id")]
    pub id: Identifier,
    #[serde(rename = "$schema")]
    pub schema: String,
    pub version: u32,
    pub owner_id: Identifier,
    #[serde(rename = "documents")]
    pub document_schemas: BTreeMap<String, JsonSchema>,
    #[serde(rename = "$defs", skip_serializing_if = "is_empty")]
    pub defs: BTreeMap<String, JsonSchema>,
    #[serde(skip)]
    pub metadata: Option<Metadata>,
    #[serde(skip)]
    pub entropy: [u8; 32],
    #[serde(skip)]
    pub binary_properties: BTreeMap<String, JsonSchema>,
}

impl DataContract {
    pub fn to_object(&self, skip_identifiers_conversion: bool) -> Result<JsonValue, ProtocolError> {
        let mut json_object = serde_json::to_value(&self)?;
        if !json_object.is_object() {
            return Err(anyhow!("the Data Contract isn't a JSON Value Object").into());
        }

        if !skip_identifiers_conversion {
            let id = self.id.to_vec();
            let owner_id = self.owner_id.to_vec();
            if let JsonValue::Object(ref mut o) = json_object {
                o.insert(String::from("$id"), JsonValue::Array(id));
                o.insert(String::from("$ownerId"), JsonValue::Array(owner_id));
            }
        }
        Ok(json_object)
    }

    /// Returns Data Contract as a JSON Value
    pub fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        Ok(serde_json::to_value(&self)?)
    }

    /// Returns Data Contract as a Buffer
    pub fn to_buffer(&self) -> Result<Vec<u8>, ProtocolError> {
        let protocol_version = self.protocol_version;
        let mut json_object = self.to_object(true)?;

        if let JsonValue::Object(ref mut o) = json_object {
            o.remove("protocolVersion");
        };

        Ok(serializer::value_to_cbor(
            json_object,
            Some(protocol_version),
        )?)
    }

    // Returns hash from Data Contract
    pub fn hash(&self) -> Result<Vec<u8>, ProtocolError> {
        Ok(sha(self.to_buffer()?))
    }

    /// Increments version of Data Contract
    pub fn increment_version(&mut self) {
        self.version += 1;
    }

    /// Returns true if document type is defined
    pub fn is_document_defined(&self, doc_type: &str) -> bool {
        self.document_schemas.contains_key(doc_type)
    }

    pub fn set_document_schema(&mut self, doc_type: String, schema: JsonSchema) {
        self.document_schemas.insert(doc_type, schema);
    }

    pub fn get_document_schema(&self, doc_type: &str) -> Result<&JsonSchema, ProtocolError> {
        self.document_schemas.get(doc_type).ok_or(
            DataContractError::InvalidDocumentTypeError {
                doc_type: doc_type.to_owned(),
                data_contract: self.clone(),
            }
            .into(),
        )
    }

    pub fn get_document_schema_ref(&self, doc_type: &str) -> Result<String, ProtocolError> {
        if !self.is_document_defined(doc_type) {
            return Err(DataContractError::InvalidDocumentTypeError {
                doc_type: doc_type.to_owned(),
                data_contract: self.clone(),
            }
            .into());
        }

        return Ok(format!(
            "{}/#documents/{}",
            self.id.to_string(Encoding::Base58),
            doc_type
        ));
    }

    pub fn get_binary_properties() -> JsonValue {
        unimplemented!()
    }
}

fn is_empty<K, V>(collection: &BTreeMap<K, V>) -> bool {
    collection.len() == 0
}

#[cfg(test)]
mod test {
    use super::*;
}
