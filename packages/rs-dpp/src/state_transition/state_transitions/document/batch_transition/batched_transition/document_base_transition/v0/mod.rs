pub mod from_document;
pub mod v0_methods;

#[cfg(feature = "state-transition-value-conversion")]
use std::collections::BTreeMap;

use bincode::{Decode, Encode};
use derive_more::Display;

#[cfg(feature = "state-transition-value-conversion")]
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
#[cfg(feature = "state-transition-value-conversion")]
use platform_value::Value;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "state-transition-json-conversion")]
use serde_json::Value as JsonValue;

#[cfg(feature = "state-transition-value-conversion")]
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::identifier::Identifier;
use crate::prelude::IdentityNonce;
#[cfg(feature = "state-transition-value-conversion")]
use crate::state_transition::batch_transition::document_base_transition::property_names;
#[cfg(any(
    feature = "state-transition-json-conversion",
    feature = "state-transition-value-conversion"
))]
use crate::{data_contract::DataContract, errors::ProtocolError};

#[derive(Debug, Clone, Encode, Decode, Default, PartialEq, Display)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[display(
    "ID: {}, Type: {}, Contract ID: {}",
    "id",
    "document_type_name",
    "data_contract_id"
)]
pub struct DocumentBaseTransitionV0 {
    /// The document ID
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(rename = "$id"))]
    pub id: Identifier,
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$identityContractNonce")
    )]
    pub identity_contract_nonce: IdentityNonce,
    /// Name of document type found int the data contract associated with the `data_contract_id`
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(rename = "$type"))]
    pub document_type_name: String,
    /// Data contract ID generated from the data contract's `owner_id` and `entropy`
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$dataContractId")
    )]
    pub data_contract_id: Identifier,
}

impl DocumentBaseTransitionV0 {
    #[cfg(feature = "state-transition-value-conversion")]
    pub fn from_value_map_consume(
        map: &mut BTreeMap<String, Value>,
        data_contract: DataContract,
        identity_contract_nonce: IdentityNonce,
    ) -> Result<DocumentBaseTransitionV0, ProtocolError> {
        Ok(DocumentBaseTransitionV0 {
            id: Identifier::from(map.remove_hash256_bytes(property_names::ID)?),
            identity_contract_nonce,
            document_type_name: map.remove_string(property_names::DOCUMENT_TYPE)?,
            data_contract_id: Identifier::new(
                map.remove_optional_hash256_bytes(property_names::DATA_CONTRACT_ID)?
                    .unwrap_or(data_contract.id().to_buffer()),
            ),
        })
    }
}

pub trait DocumentTransitionObjectLike {
    #[cfg(feature = "state-transition-json-conversion")]
    /// Creates the Document Transition from JSON representation. The JSON representation contains
    /// binary data encoded in base64, Identifiers encoded in base58
    fn from_json_object(
        json_str: JsonValue,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    #[cfg(feature = "state-transition-value-conversion")]
    /// Creates the document transition from Raw Object
    fn from_object(
        raw_transition: Value,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    #[cfg(feature = "state-transition-value-conversion")]
    fn from_value_map(
        map: BTreeMap<String, Value>,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;

    #[cfg(feature = "state-transition-value-conversion")]
    /// Object is an [`platform::Value`] instance that preserves the `Vec<u8>` representation
    /// for Identifiers and binary data
    fn to_object(&self) -> Result<Value, ProtocolError>;

    #[cfg(feature = "state-transition-value-conversion")]
    /// Value Map is a Map of string to [`platform::Value`] that represents the state transition
    fn to_value_map(&self) -> Result<BTreeMap<String, Value>, ProtocolError>;

    #[cfg(feature = "state-transition-json-conversion")]
    /// Object is an [`serde_json::Value`] instance that replaces the binary data with
    ///  - base58 string for Identifiers
    ///  - base64 string for other binary data
    fn to_json(&self) -> Result<JsonValue, ProtocolError>;
    #[cfg(feature = "state-transition-value-conversion")]
    fn to_cleaned_object(&self) -> Result<Value, ProtocolError>;
}
