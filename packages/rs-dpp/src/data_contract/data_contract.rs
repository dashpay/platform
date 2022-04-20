use super::errors::*;
use crate::data_contract::get_binary_properties_from_schema::get_binary_properties;
use crate::util::deserializer;
use crate::util::string_encoding::Encoding;
use crate::{
    errors::ProtocolError,
    identifier::Identifier,
    metadata::Metadata,
    util::{hash::hash, serializer},
};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::convert::TryFrom;

// TODO probably this need to be changed
pub type JsonSchema = JsonValue;
type DocumentType = String;
type PropertyPath = String;

pub const SCHEMA: &str = "https://schema.dash.org/dpp-0-4-0/meta/data-contract";

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
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
    pub documents: BTreeMap<DocumentType, JsonSchema>,
    #[serde(rename = "$defs", skip_serializing_if = "Option::is_none")]
    pub defs: Option<BTreeMap<DocumentType, JsonSchema>>,
    #[serde(skip)]
    pub metadata: Option<Metadata>,
    #[serde(skip)]
    pub entropy: [u8; 32],
    #[serde(skip)]
    pub binary_properties: BTreeMap<DocumentType, BTreeMap<PropertyPath, JsonValue>>,
}

impl DataContract {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_buffer(b: impl AsRef<[u8]>) -> Result<DataContract, ProtocolError> {
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
        deserializer::parse_identities(&mut json_map, &["$id", "$ownerId"])?;

        let mut data_contract: DataContract = serde_json::from_value(JsonValue::Object(json_map))?;
        data_contract.generate_binary_properties();
        Ok(data_contract)
    }

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

        serializer::value_to_cbor(json_object, Some(protocol_version))
    }

    // Returns hash from Data Contract
    pub fn hash(&self) -> Result<Vec<u8>, ProtocolError> {
        Ok(hash(self.to_buffer()?))
    }

    /// Increments version of Data Contract
    pub fn increment_version(&mut self) {
        self.version += 1;
    }

    /// Returns true if document type is defined
    pub fn is_document_defined(&self, doc_type: &str) -> bool {
        self.documents.contains_key(doc_type)
    }

    pub fn set_document_schema(&mut self, doc_type: String, schema: JsonSchema) {
        let binary_properties = get_binary_properties(&schema);
        self.documents.insert(doc_type.clone(), schema);
        self.binary_properties.insert(doc_type, binary_properties);
    }

    pub fn get_document_schema(&self, doc_type: &str) -> Result<&JsonSchema, ProtocolError> {
        let d =
            self.documents
                .get(doc_type)
                .ok_or(DataContractError::InvalidDocumentTypeError {
                    doc_type: doc_type.to_owned(),
                    data_contract: self.clone(),
                })?;
        Ok(d)
    }

    pub fn get_document_schema_ref(&self, doc_type: &str) -> Result<String, ProtocolError> {
        if !self.is_document_defined(doc_type) {
            return Err(DataContractError::InvalidDocumentTypeError {
                doc_type: doc_type.to_owned(),
                data_contract: self.clone(),
            }
            .into());
        };

        return Ok(format!(
            "{}/#documents/{}",
            self.id.to_string(Encoding::Base58),
            doc_type
        ));
    }

    /// Returns the binary properties for the given document type
    /// Comparing to JS version of DPP, the binary_properties are not generated automatically
    /// if they're not present. It is up to the developer to use proper methods like ['DataContract::set_document_schema'] which
    /// automatically generates binary properties when setting the Json Schema
    pub fn get_binary_properties(
        &self,
        doc_type: &str,
    ) -> Result<&BTreeMap<String, JsonValue>, ProtocolError> {
        if !self.is_document_defined(doc_type) {
            return Err(DataContractError::InvalidDocumentTypeError {
                doc_type: doc_type.to_owned(),
                data_contract: self.clone(),
            }
            .into());
        }

        // The rust implementation doesn't set the value if it is not present in `binary_properties`. The difference is caused by
        // required `mut` annotation. As `get_binary_properties` is reused in many other read-only methods, the mutation would require
        // propagating the `mut` to other getters which by the definition shouldn't be mutable.
        self.binary_properties.get(doc_type).ok_or_else(|| {
            {
                anyhow::anyhow!(
                    "document '{}' has not generated binary_properties",
                    doc_type
                )
            }
            .into()
        })
    }

    fn generate_binary_properties(&mut self) {
        self.binary_properties = self
            .documents
            .iter()
            .map(|(doc_type, schema)| (String::from(doc_type), get_binary_properties(schema)))
            .collect();
    }
}

impl TryFrom<JsonValue> for DataContract {
    type Error = ProtocolError;
    fn try_from(v: JsonValue) -> Result<Self, Self::Error> {
        Ok(serde_json::from_value(v)?)
    }
}

impl TryFrom<&str> for DataContract {
    type Error = ProtocolError;
    fn try_from(v: &str) -> Result<Self, Self::Error> {
        Ok(serde_json::from_str(v)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::tests::utils::*;
    use anyhow::Result;
    use log::trace;

    fn init() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Trace)
            .try_init();
    }

    #[test]
    fn test_deserialize_contract() -> Result<()> {
        init();

        let string_contract = get_data_from_file("src/tests/payloads/contract_example.json")?;
        trace!("the string contract is {}", string_contract);
        let contract = DataContract::try_from(string_contract.as_str())?;
        trace!("the parsed contract is {:#?}", contract);

        assert_eq!(contract.protocol_version, 0);
        assert_eq!(
            contract.schema,
            "https://schema.dash.org/dpp-0-4-0/meta/data-contract"
        );
        assert_eq!(contract.version, 5);
        assert_eq!(
            contract.id.to_string(Encoding::Base58),
            "AoDzJxWSb1gUi2dSmvFeUFpSsjZQRJaqCpn7vCLkwwJj"
        );
        assert_eq!(
            contract.documents["note"]["properties"]["message"]["type"],
            "string"
        );
        assert!(contract.is_document_defined("note"));

        Ok(())
    }

    #[test]
    fn test_serialize_contract_json() -> Result<()> {
        init();

        let mut string_contract = get_data_from_file("src/tests/payloads/contract_example.json")?;
        string_contract.retain(|c| !c.is_whitespace());
        trace!("the string contract is {}", string_contract);

        let contract = DataContract::try_from(string_contract.as_str())?;
        trace!("the parsed contract is {:#?}", contract);

        let serialized_contract = serde_json::to_string(&contract.to_json()?)?;
        trace!("serialized contract: {}", serialized_contract);

        assert_eq!(serialized_contract, string_contract);
        Ok(())
    }
}
