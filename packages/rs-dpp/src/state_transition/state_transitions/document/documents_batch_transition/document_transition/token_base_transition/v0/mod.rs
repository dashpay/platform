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

#[cfg(feature = "state-transition-value-conversion")]
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::identifier::Identifier;
use crate::prelude::IdentityNonce;
#[cfg(feature = "state-transition-value-conversion")]
use crate::state_transition::documents_batch_transition::token_base_transition::property_names;
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
    "token_id",
    "data_contract_id"
)]
pub struct TokenBaseTransitionV0 {
    /// The document ID
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(rename = "$id"))]
    pub id: Identifier,
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$identity-contract-nonce")
    )]
    pub identity_contract_nonce: IdentityNonce,
    /// ID of the token within the contract
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$tokenId")
    )]
    pub token_id: u16,
    /// Data contract ID generated from the data contract's `owner_id` and `entropy`
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$dataContractId")
    )]
    pub data_contract_id: Identifier,
}

impl TokenBaseTransitionV0 {
    #[cfg(feature = "state-transition-value-conversion")]
    pub fn from_value_map_consume(
        map: &mut BTreeMap<String, Value>,
        data_contract: DataContract,
        identity_contract_nonce: IdentityNonce,
    ) -> Result<TokenBaseTransitionV0, ProtocolError> {
        Ok(TokenBaseTransitionV0 {
            id: Identifier::from(
                map.remove_hash256_bytes(property_names::ID)
                    .map_err(ProtocolError::ValueError)?,
            ),
            identity_contract_nonce,
            token_id: map
                .remove_integer(property_names::TOKEN_ID)
                .map_err(ProtocolError::ValueError)?,
            data_contract_id: Identifier::new(
                map.remove_optional_hash256_bytes(property_names::DATA_CONTRACT_ID)
                    .map_err(ProtocolError::ValueError)?
                    .unwrap_or(data_contract.id().to_buffer()),
            ),
        })
    }
}
