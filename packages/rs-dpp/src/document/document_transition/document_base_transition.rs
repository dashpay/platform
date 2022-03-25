use crate::{
    data_contract::DataContract, errors::ProtocolError, identifier::Identifier, util::deserializer,
};
use anyhow::bail;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

pub use serde_json::Value as JsonValue;

/**
 * @typedef {Object} RawDocumentTransition
 * @property {Buffer} $id
 * @property {string} $type
 * @property {number} $action
 * @property {Buffer} $dataContractId
 */

/**
 * @typedef {Object} JsonDocumentTransition
 * @property {string} $id
 * @property {string} $type
 * @property {number} $action
 * @property {string} $dataContractId
 */

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentBaseTransition {
    #[serde(rename = "$id")]
    pub id: Identifier,
    #[serde(rename = "$type")]
    pub transition_type: String,
    #[serde(rename = "$action")]
    pub action: Action,
    #[serde(rename = "$dataContractId")]
    pub data_contract_id: Identifier,
    #[serde(skip)]
    pub data_contract: DataContract,
}

impl DocumentBaseTransition {
    pub fn from_raw_document(
        mut raw_value: JsonValue,
        data_contract: DataContract,
    ) -> Result<DocumentBaseTransition, ProtocolError> {
        if let JsonValue::Object(ref mut o) = &mut raw_value {
            deserializer::parse_identities(o, &["$id", "$dataContractId"])?;
        } else {
            return Err("the raw_value is not an Json Value object".into());
        }
        let mut document: DocumentBaseTransition = serde_json::from_value(raw_value)?;
        document.data_contract = data_contract;

        Ok(document)
    }

    pub fn to_object(&self) -> Result<JsonValue, ProtocolError> {
        let mut json_object = serde_json::to_value(&self)?;
        if !json_object.is_object() {
            return Err("the Data Contract isn't a JSON Value Object".into());
        }

        let id = self.id.to_vec();
        let data_contract_id = self.data_contract_id.to_vec();
        if let JsonValue::Object(ref mut o) = json_object {
            o.insert(String::from("$id"), JsonValue::Array(id));
            o.insert(
                String::from("$dataContractId"),
                JsonValue::Array(data_contract_id),
            );
        }
        Ok(json_object)
    }

    pub fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        let value = serde_json::to_value(&self)?;
        Ok(value)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[repr(u8)]
pub enum Action {
    Create = 0,
    Replace = 1,
    // 2 - reserved for update
    Delete = 3,
}

impl TryFrom<&str> for Action {
    // TODO
    type Error = anyhow::Error;

    fn try_from(name: &str) -> Result<Action, Self::Error> {
        match name {
            "create" => Ok(Action::Create),
            "replace" => Ok(Action::Replace),
            "delete" => Ok(Action::Delete),
            _ => {
                bail!("unknown action type: '{}'", name);
            }
        }
    }
}

impl TryFrom<String> for Action {
    type Error = anyhow::Error;
    fn try_from(name: String) -> Result<Action, Self::Error> {
        Action::try_from(name.as_str())
    }
}
