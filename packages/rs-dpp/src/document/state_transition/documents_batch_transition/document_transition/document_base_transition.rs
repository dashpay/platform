use crate::{
    data_contract::DataContract,
    errors::ProtocolError,
    identifier::Identifier,
    util::json_value::{JsonValueExt, ReplaceWith},
};
use anyhow::bail;
use serde::{Deserialize, Serialize};
use serde_repr::*;
use std::convert::TryFrom;

pub use serde_json::Value as JsonValue;

const IDENTIFIER_FIELDS: [&str; 2] = ["$id", "$dataContractId"];

#[derive(Debug, Serialize_repr, Deserialize_repr, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Action {
    Create = 0,
    Replace = 1,
    // 2 - reserved for update
    Delete = 3,
}

impl TryFrom<u8> for Action {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Action::Create),
            1 => Ok(Action::Replace),
            3 => Ok(Action::Delete),
            v => bail!("unknown document transition action: '{}'", v),
        }
    }
}

impl Default for Action {
    fn default() -> Action {
        Action::Create
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TryFrom<&str> for Action {
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DocumentBaseTransition {
    #[serde(rename = "$id")]
    /// The document ID
    pub id: Identifier,
    #[serde(rename = "$type")]
    /// Name of document type found int the data contract associated with the `data_contract_id`
    pub document_type: String,
    #[serde(rename = "$action")]
    /// Action the platform should take for the associated document
    pub action: Action,
    #[serde(rename = "$dataContractId")]
    /// Data contract ID generated from the data contract's `owner_id` and `entropy`
    pub data_contract_id: Identifier,
    #[serde(skip)]
    pub data_contract: DataContract,
}

impl DocumentBaseTransition {
    pub fn identifiers_to_strings(
        raw_document_transition: &mut JsonValue,
    ) -> Result<(), ProtocolError> {
        raw_document_transition.replace_identifier_paths(IDENTIFIER_FIELDS, ReplaceWith::Base58)?;
        Ok(())
    }
}

impl DocumentTransitionObjectLike for DocumentBaseTransition {
    fn from_json_str(json_str: &str, data_contract: DataContract) -> Result<Self, ProtocolError> {
        let mut document: DocumentBaseTransition = serde_json::from_str(json_str)?;
        document.data_contract_id = data_contract.id.clone();
        document.data_contract = data_contract;
        Ok(document)
    }

    fn from_raw_document(
        mut raw_transition: JsonValue,
        data_contract: DataContract,
    ) -> Result<DocumentBaseTransition, ProtocolError> {
        Self::identifiers_to_strings(&mut raw_transition)?;
        let mut document: DocumentBaseTransition = serde_json::from_value(raw_transition)?;
        document.data_contract_id = data_contract.id.clone();
        document.data_contract = data_contract;

        Ok(document)
    }

    fn to_object(&self) -> Result<JsonValue, ProtocolError> {
        let mut object = serde_json::to_value(&self)?;

        object.replace_identifier_paths(IDENTIFIER_FIELDS, ReplaceWith::Bytes)?;
        Ok(object)
    }

    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        let value = serde_json::to_value(&self)?;
        Ok(value)
    }
}

pub trait DocumentTransitionObjectLike {
    /// Creates the Document Transition from JSON representation. The JSON representation contains
    /// binary data encoded in base64, Identifiers encoded in base58
    fn from_json_str(json_str: &str, data_contract: DataContract) -> Result<Self, ProtocolError>
    where
        Self: std::marker::Sized;
    /// Creates the document transition from Raw Object
    fn from_raw_document(
        raw_transition: JsonValue,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: std::marker::Sized;
    /// Object is an [`serde_json::Value`] instance that preserves the `Vec<u8>` representation
    /// for Identifiers and binary data
    fn to_object(&self) -> Result<JsonValue, ProtocolError>;
    /// Object is an [`serde_json::Value`] instance that replaces the binary data with
    ///  - base58 string for Identifiers
    ///  - base64 string for other binary data
    fn to_json(&self) -> Result<JsonValue, ProtocolError>;
}
