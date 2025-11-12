mod identity_signed;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use platform_value::BinaryData;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;
use platform_version::version::PlatformVersion;
use platform_version::{TryFromPlatformVersioned, TryIntoPlatformVersioned};

use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::prelude::{IdentityNonce, UserFeeIncrease};
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use crate::state_transition::StateTransition;
use crate::{data_contract::DataContract, identity::KeyID, ProtocolError};

#[derive(Debug, Clone, Encode, Decode, PartialEq, PlatformSignable)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct DataContractUpdateTransitionV0 {
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
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
