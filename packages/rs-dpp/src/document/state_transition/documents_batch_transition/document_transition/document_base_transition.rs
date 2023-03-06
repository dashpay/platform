use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};

use anyhow::bail;
use num_enum::IntoPrimitive;
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::Value;
use serde::{Deserialize, Serialize};
pub use serde_json::Value as JsonValue;
use serde_repr::*;

use crate::document::document_transition::Action::{Create, Delete, Replace};

use crate::document::errors::DocumentError;
use crate::{
    data_contract::DataContract, errors::ProtocolError, identifier::Identifier,
    util::json_value::JsonValueExt,
};

pub(self) mod property_names {
    pub const ID: &str = "$id";
    pub const DATA_CONTRACT_ID: &str = "$dataContractId";
    pub const DOCUMENT_TYPE: &str = "$type";
    pub const ACTION: &str = "$action";
}

pub const IDENTIFIER_FIELDS: [&str; 2] = [property_names::ID, property_names::DATA_CONTRACT_ID];

#[derive(
    Debug, Serialize_repr, Deserialize_repr, Clone, Copy, PartialEq, Eq, Hash, IntoPrimitive,
)]
#[repr(u8)]
pub enum Action {
    Create = 0,
    Replace = 1,
    // 2 - reserved for update
    Delete = 3,
}

impl Default for Action {
    fn default() -> Action {
        Action::Create
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl TryFrom<u8> for Action {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Create),
            1 => Ok(Replace),
            3 => Ok(Delete),
            other => Err(ProtocolError::Document(Box::new(
                DocumentError::InvalidActionError(other),
            ))),
        }
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
    /// The document ID
    #[serde(rename = "$id")]
    pub id: Identifier,
    /// Name of document type found int the data contract associated with the `data_contract_id`
    #[serde(rename = "$type")]
    pub document_type_name: String,
    /// Action the platform should take for the associated document
    #[serde(rename = "$action")]
    pub action: Action,
    /// Data contract ID generated from the data contract's `owner_id` and `entropy`
    #[serde(rename = "$dataContractId")]
    pub data_contract_id: Identifier,

    #[serde(skip)]
    pub data_contract: DataContract,
}

impl DocumentBaseTransition {
    pub fn from_value_map_consume(
        map: &mut BTreeMap<String, Value>,
        data_contract: DataContract,
    ) -> Result<DocumentBaseTransition, ProtocolError> {
        Ok(DocumentBaseTransition {
            id: Identifier::from(
                map.remove_hash256_bytes(property_names::ID)
                    .map_err(ProtocolError::ValueError)?,
            ),
            document_type_name: map
                .remove_string(property_names::DOCUMENT_TYPE)
                .map_err(ProtocolError::ValueError)?,
            action: map
                .remove_integer::<u8>(property_names::ACTION)
                .map_err(ProtocolError::ValueError)?
                .try_into()?,
            data_contract_id: data_contract.id,
            data_contract,
        })
    }
}

impl DocumentTransitionObjectLike for DocumentBaseTransition {
    fn from_json_object(
        json_value: JsonValue,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError> {
        let mut document: DocumentBaseTransition = serde_json::from_value(json_value)?;

        document.data_contract_id = data_contract.id;
        document.data_contract = data_contract;
        Ok(document)
    }

    fn from_raw_object(
        raw_transition: Value,
        data_contract: DataContract,
    ) -> Result<DocumentBaseTransition, ProtocolError> {
        let map = raw_transition
            .into_btree_map()
            .map_err(ProtocolError::ValueError)?;
        Self::from_value_map(map, data_contract)
    }

    fn from_value_map(
        map: BTreeMap<String, Value>,
        data_contract: DataContract,
    ) -> Result<DocumentBaseTransition, ProtocolError> {
        Ok(DocumentBaseTransition {
            id: Identifier::from(
                map.get_hash256_bytes(property_names::ID)
                    .map_err(ProtocolError::ValueError)?,
            ),
            document_type_name: map
                .get_string(property_names::DOCUMENT_TYPE)
                .map_err(ProtocolError::ValueError)?,
            action: map
                .get_integer::<u8>(property_names::ACTION)
                .map_err(ProtocolError::ValueError)?
                .try_into()?,
            data_contract_id: data_contract.id,
            data_contract,
        })
    }

    fn to_object(&self) -> Result<Value, ProtocolError> {
        Ok(self.to_value_map()?.into())
    }

    fn to_value_map(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let mut btree_map = BTreeMap::new();
        btree_map.insert(
            property_names::ID.to_string(),
            Value::Identifier(self.id.buffer),
        );
        btree_map.insert(
            property_names::DATA_CONTRACT_ID.to_string(),
            Value::Identifier(self.data_contract_id.buffer),
        );
        btree_map.insert(
            property_names::ACTION.to_string(),
            Value::U8(self.action as u8),
        );
        btree_map.insert(
            property_names::DOCUMENT_TYPE.to_string(),
            Value::Text(self.document_type_name.clone()),
        );
        Ok(btree_map)
    }

    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        let value = serde_json::to_value(self)?;
        Ok(value)
    }
}

pub trait DocumentTransitionObjectLike {
    /// Creates the Document Transition from JSON representation. The JSON representation contains
    /// binary data encoded in base64, Identifiers encoded in base58
    fn from_json_object(
        json_str: JsonValue,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: std::marker::Sized;
    /// Creates the document transition from Raw Object
    fn from_raw_object(
        raw_transition: Value,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: std::marker::Sized;
    fn from_value_map(
        map: BTreeMap<String, Value>,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: std::marker::Sized;
    /// Object is an [`platform::Value`] instance that preserves the `Vec<u8>` representation
    /// for Identifiers and binary data
    fn to_object(&self) -> Result<Value, ProtocolError>;

    /// Value Map is a Map of string to [`platform::Value`] that represents the state transition
    fn to_value_map(&self) -> Result<BTreeMap<String, Value>, ProtocolError>;

    /// Object is an [`serde_json::Value`] instance that replaces the binary data with
    ///  - base58 string for Identifiers
    ///  - base64 string for other binary data
    fn to_json(&self) -> Result<JsonValue, ProtocolError>;
}
