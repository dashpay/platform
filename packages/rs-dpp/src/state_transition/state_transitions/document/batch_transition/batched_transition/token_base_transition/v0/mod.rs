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
#[cfg(feature = "state-transition-value-conversion")]
use crate::data_contract::accessors::v1::DataContractV1Getters;
use crate::group::GroupStateTransitionInfo;
use crate::identifier::Identifier;
use crate::prelude::IdentityNonce;
#[cfg(feature = "state-transition-value-conversion")]
use crate::state_transition::batch_transition::token_base_transition::property_names;
#[cfg(feature = "state-transition-value-conversion")]
use crate::tokens::errors::TokenError;
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
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$identity-contract-nonce")
    )]
    pub identity_contract_nonce: IdentityNonce,
    /// ID of the token within the contract
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$tokenContractPosition")
    )]
    pub token_contract_position: u16,
    /// Data contract ID generated from the data contract's `owner_id` and `entropy`
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$dataContractId")
    )]
    pub data_contract_id: Identifier,
    /// Token ID generated from the data contract ID and the token position
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$tokenId")
    )]
    pub token_id: Identifier,
    /// Using group multi party rules for authentication
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub using_group_info: Option<GroupStateTransitionInfo>,
}

impl TokenBaseTransitionV0 {
    #[cfg(feature = "state-transition-value-conversion")]
    pub fn from_value_map_consume(
        map: &mut BTreeMap<String, Value>,
        data_contract: DataContract,
        identity_contract_nonce: IdentityNonce,
    ) -> Result<TokenBaseTransitionV0, ProtocolError> {
        let token_contract_position = map
            .remove_integer(property_names::TOKEN_CONTRACT_POSITION)
            .map_err(ProtocolError::ValueError)?;
        Ok(TokenBaseTransitionV0 {
            identity_contract_nonce,
            token_contract_position,
            data_contract_id: Identifier::new(
                map.remove_optional_hash256_bytes(property_names::DATA_CONTRACT_ID)
                    .map_err(ProtocolError::ValueError)?
                    .unwrap_or(data_contract.id().to_buffer()),
            ),
            token_id: map
                .remove_optional_hash256_bytes(property_names::TOKEN_ID)
                .map_err(ProtocolError::ValueError)?
                .map(Identifier::new)
                .unwrap_or(data_contract.token_id(token_contract_position).ok_or(
                    ProtocolError::Token(TokenError::TokenNotFoundAtPositionError.into()),
                )?),
            using_group_info: map
                .remove_optional_integer(property_names::GROUP_CONTRACT_POSITION)
                .map_err(ProtocolError::ValueError)?
                .map(|group_contract_position| {
                    Ok::<GroupStateTransitionInfo, ProtocolError>(GroupStateTransitionInfo {
                        group_contract_position,
                        action_id: map
                            .remove_hash256_bytes(property_names::GROUP_ACTION_ID)
                            .map_err(ProtocolError::ValueError)?
                            .into(),
                        action_is_proposer: map
                            .remove_bool(property_names::GROUP_ACTION_IS_PROPOSER)
                            .map_err(ProtocolError::ValueError)?,
                    })
                })
                .transpose()?,
        })
    }
}
