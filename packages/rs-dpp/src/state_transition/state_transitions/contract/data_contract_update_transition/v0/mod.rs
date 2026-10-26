mod identity_signed;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
mod version;

#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use platform_value::BinaryData;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::data_contract::serialized_version::TransitionContractFormat;
use crate::state_transition::state_transitions::contract::stored_contract_format::decode_through;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::PlatformSignable;
use platform_version::version::PlatformVersion;
use platform_version::{TryFromPlatformVersioned, TryIntoPlatformVersioned};

use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::prelude::{IdentityNonce, UserFeeIncrease};
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use crate::state_transition::StateTransition;
use crate::{data_contract::DataContract, identity::KeyID, ProtocolError};

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
/// Decoded through [`DataContractUpdateTransitionV0Decoded`], which refuses the stored contract
/// format at its tag.
#[derive(Debug, Clone, Encode, PartialEq, PlatformSignable)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct DataContractUpdateTransitionV0 {
    #[cfg_attr(
        feature = "serde-conversion",
        serde(rename = "$identity-contract-nonce")
    )]
    pub identity_contract_nonce: IdentityNonce,
    pub data_contract: DataContractInSerializationFormat,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

/// [`DataContractUpdateTransitionV0`] as decoding reads it: the same fields in the same order,
/// its contract read as a [`TransitionContractFormat`].
#[derive(Decode, DecodeUntrusted)]
struct DataContractUpdateTransitionV0Decoded {
    identity_contract_nonce: IdentityNonce,
    data_contract: TransitionContractFormat,
    user_fee_increase: UserFeeIncrease,
    signature_public_key_id: KeyID,
    signature: BinaryData,
}

impl From<DataContractUpdateTransitionV0Decoded> for DataContractUpdateTransitionV0 {
    fn from(decoded: DataContractUpdateTransitionV0Decoded) -> Self {
        let DataContractUpdateTransitionV0Decoded {
            identity_contract_nonce,
            data_contract,
            user_fee_increase,
            signature_public_key_id,
            signature,
        } = decoded;
        Self {
            identity_contract_nonce,
            data_contract: data_contract.into(),
            user_fee_increase,
            signature_public_key_id,
            signature,
        }
    }
}

decode_through!(
    DataContractUpdateTransitionV0,
    DataContractUpdateTransitionV0Decoded
);

impl TryFromPlatformVersioned<(DataContract, IdentityNonce)> for DataContractUpdateTransitionV0 {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: (DataContract, IdentityNonce),
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        Ok(DataContractUpdateTransitionV0 {
            identity_contract_nonce: value.1,
            data_contract: value.0.try_into_platform_versioned(platform_version)?,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        })
    }
}

impl From<DataContractUpdateTransitionV0> for StateTransition {
    fn from(value: DataContractUpdateTransitionV0) -> Self {
        let transition: DataContractUpdateTransition = value.into();
        transition.into()
    }
}

impl From<&DataContractUpdateTransitionV0> for StateTransition {
    fn from(value: &DataContractUpdateTransitionV0) -> Self {
        let transition: DataContractUpdateTransition = value.clone().into();
        transition.into()
    }
}
