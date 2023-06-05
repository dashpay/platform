use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};

use anyhow::bail;
use bincode::{Decode, Encode};
use num_enum::IntoPrimitive;
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::Value;
use serde::{Deserialize, Serialize};
pub use serde_json::Value as JsonValue;
use serde_repr::*;

use crate::document::document_transition::Action::{Create, Delete, Replace};

use crate::document::errors::DocumentError;
use crate::{data_contract::DataContract, errors::ProtocolError, identifier::Identifier};

pub(self) mod property_names {
    pub const ID: &str = "$id";
    pub const DATA_CONTRACT_ID: &str = "$dataContractId";
    pub const DOCUMENT_TYPE: &str = "$type";
    pub const ACTION: &str = "$action";
}

pub const IDENTIFIER_FIELDS: [&str; 2] = [property_names::ID, property_names::DATA_CONTRACT_ID];

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentBaseTransitionV0 {
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

impl DocumentBaseTransitionV0 {
    pub fn from_value_map_consume(
        map: &mut BTreeMap<String, Value>,
        data_contract: DataContract,
    ) -> Result<DocumentBaseTransitionV0, ProtocolError> {
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
            data_contract_id: Identifier::new(
                map.remove_optional_hash256_bytes(property_names::DATA_CONTRACT_ID)
                    .map_err(ProtocolError::ValueError)?
                    .unwrap_or(data_contract.id.to_buffer()),
            ),
            data_contract,
        })
    }
}

impl DocumentTransitionObjectLike for DocumentBaseTransitionV0 {
    #[cfg(feature = "json-object")]
    fn from_json_object(
        json_value: JsonValue,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError> {
        let mut document: DocumentBaseTransitionV0 = serde_json::from_value(json_value)?;

        document.data_contract_id = data_contract.id;
        document.data_contract = data_contract;
        Ok(document)
    }

    #[cfg(feature = "platform-value")]
    fn from_raw_object(
        raw_transition: Value,
        data_contract: DataContract,
    ) -> Result<DocumentBaseTransitionV0, ProtocolError> {
        let map = raw_transition
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;
        Self::from_value_map(map, data_contract)
    }

    #[cfg(feature = "platform-value")]
    fn from_value_map(
        map: BTreeMap<String, Value>,
        data_contract: DataContract,
    ) -> Result<DocumentBaseTransitionV0, ProtocolError> {
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

    #[cfg(feature = "platform-value")]
    fn to_object(&self) -> Result<Value, ProtocolError> {
        Ok(self.to_value_map()?.into())
    }

    #[cfg(feature = "platform-value")]
    fn to_value_map(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let mut btree_map = BTreeMap::new();
        btree_map.insert(
            property_names::ID.to_string(),
            Value::Identifier(self.id.to_buffer()),
        );
        btree_map.insert(
            property_names::DATA_CONTRACT_ID.to_string(),
            Value::Identifier(self.data_contract_id.to_buffer()),
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

    #[cfg(feature = "json-object")]
    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        self.to_object()?
            .try_into()
            .map_err(ProtocolError::ValueError)
    }

    #[cfg(feature = "platform-value")]
    fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
        Ok(self.to_value_map()?.into())
    }
}

pub trait DocumentTransitionObjectLike {
    #[cfg(feature = "json-object")]
    /// Creates the Document Transition from JSON representation. The JSON representation contains
    /// binary data encoded in base64, Identifiers encoded in base58
    fn from_json_object(
        json_str: JsonValue,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: std::marker::Sized;
    #[cfg(feature = "platform-value")]
    /// Creates the document transition from Raw Object
    fn from_raw_object(
        raw_transition: Value,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: std::marker::Sized;
    #[cfg(feature = "platform-value")]
    fn from_value_map(
        map: BTreeMap<String, Value>,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: std::marker::Sized;

    #[cfg(feature = "platform-value")]
    /// Object is an [`platform::Value`] instance that preserves the `Vec<u8>` representation
    /// for Identifiers and binary data
    fn to_object(&self) -> Result<Value, ProtocolError>;

    #[cfg(feature = "platform-value")]
    /// Value Map is a Map of string to [`platform::Value`] that represents the state transition
    fn to_value_map(&self) -> Result<BTreeMap<String, Value>, ProtocolError>;

    #[cfg(feature = "json-object")]
    /// Object is an [`serde_json::Value`] instance that replaces the binary data with
    ///  - base58 string for Identifiers
    ///  - base64 string for other binary data
    fn to_json(&self) -> Result<JsonValue, ProtocolError>;
    #[cfg(feature = "platform-value")]
    fn to_cleaned_object(&self) -> Result<Value, ProtocolError>;
}
