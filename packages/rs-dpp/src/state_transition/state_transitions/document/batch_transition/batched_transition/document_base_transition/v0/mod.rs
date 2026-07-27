pub mod from_document;
pub mod v0_methods;

#[cfg(feature = "value-conversion")]
use std::collections::BTreeMap;

use bincode::{Decode, Encode};
use derive_more::Display;

#[cfg(feature = "value-conversion")]
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
#[cfg(feature = "value-conversion")]
use platform_value::Value;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "json-conversion")]
use serde_json::Value as JsonValue;

#[cfg(feature = "value-conversion")]
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::identifier::Identifier;
use crate::prelude::IdentityNonce;
#[cfg(feature = "value-conversion")]
use crate::state_transition::batch_transition::document_base_transition::property_names;
#[cfg(any(feature = "json-conversion", feature = "value-conversion"))]
use crate::{data_contract::DataContract, errors::ProtocolError};

#[derive(Debug, Clone, Encode, Decode, Default, PartialEq, Display)]
// `json_safe_fields` auto-injects `crate::serialization::json_safe_u64` on
// `identity_contract_nonce: IdentityNonce` (= u64). Large nonces serialize
// as JSON strings to avoid JS Number precision loss; native u64 in non-HR.
#[cfg_attr(feature = "json-conversion", crate::serialization::json_safe_fields)]
#[cfg_attr(
    feature = "serde-conversion",
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
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$id"))]
    pub id: Identifier,
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$identityContractNonce"))]
    pub identity_contract_nonce: IdentityNonce,
    /// Name of document type found int the data contract associated with the `data_contract_id`
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$type"))]
    pub document_type_name: String,
    /// Data contract ID generated from the data contract's `owner_id` and `entropy`
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$dataContractId"))]
    pub data_contract_id: Identifier,
}

impl DocumentBaseTransitionV0 {
    #[cfg(feature = "value-conversion")]
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

/// **KEEP-AS-EXCEPTION** in the JSON/Value canonical-trait migration — this
/// trait is context-aware: the `from_*` constructors need a `DataContract`
/// to type document properties, which `JsonConvertible`/`ValueConvertible`
/// can't carry. NOTE the to-side emits a flat LEGACY shape (`$version: "0"`,
/// no `$action`/`$baseFormatVersion` tags) that intentionally differs from
/// canonical `JsonConvertible::to_json` on the same transition — see the
/// wire-shape comparison tests in `document_create_transition/v0/mod.rs`.
pub trait DocumentTransitionObjectLike {
    #[cfg(feature = "json-conversion")]
    /// Creates the Document Transition from JSON representation. The JSON representation contains
    /// binary data encoded in base64, Identifiers encoded in base58
    fn from_json_object(
        json_str: JsonValue,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    #[cfg(feature = "value-conversion")]
    /// Creates the document transition from Raw Object
    fn from_object(
        raw_transition: Value,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    #[cfg(feature = "value-conversion")]
    fn from_value_map(
        map: BTreeMap<String, Value>,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;

    #[cfg(feature = "value-conversion")]
    /// Object is an [`platform::Value`] instance that preserves the `Vec<u8>` representation
    /// for Identifiers and binary data
    fn to_object(&self) -> Result<Value, ProtocolError>;

    #[cfg(feature = "value-conversion")]
    /// Value Map is a Map of string to [`platform::Value`] that represents the state transition
    fn to_value_map(&self) -> Result<BTreeMap<String, Value>, ProtocolError>;

    #[cfg(feature = "json-conversion")]
    /// Object is an [`serde_json::Value`] instance that replaces the binary data with
    ///  - base58 string for Identifiers
    ///  - base64 string for other binary data
    fn to_json(&self) -> Result<JsonValue, ProtocolError>;
    #[cfg(feature = "value-conversion")]
    fn to_cleaned_object(&self) -> Result<Value, ProtocolError>;
}
