mod identity_signed;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
mod state_transition_like;
mod types;
pub(crate) mod v0_methods;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use platform_serialization_derive::PlatformSignable;

use platform_value::BinaryData;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::{data_contract::DataContract, identity::KeyID, ProtocolError};

use crate::data_contract::created_data_contract::CreatedDataContract;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::prelude::{FeeMultiplier, IdentityNonce};
use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
use bincode::{Decode, Encode};
use platform_version::{TryFromPlatformVersioned, TryIntoPlatformVersioned};

use crate::state_transition::StateTransition;
use crate::version::PlatformVersion;

///DataContractCreateTransitionV0 has the same encoding structure

#[derive(Debug, Clone, Encode, Decode, PartialEq, PlatformSignable)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct DataContractCreateTransitionV0 {
    pub data_contract: DataContractInSerializationFormat,
    pub identity_nonce: IdentityNonce,
    pub fee_multiplier: FeeMultiplier,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

impl From<DataContractCreateTransitionV0> for StateTransition {
    fn from(value: DataContractCreateTransitionV0) -> Self {
        let transition: DataContractCreateTransition = value.into();
        transition.into()
    }
}

impl From<&DataContractCreateTransitionV0> for StateTransition {
    fn from(value: &DataContractCreateTransitionV0) -> Self {
        let transition: DataContractCreateTransition = value.clone().into();
        transition.into()
    }
}

impl TryFromPlatformVersioned<DataContract> for DataContractCreateTransitionV0 {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        Ok(DataContractCreateTransitionV0 {
            data_contract: value.try_into_platform_versioned(platform_version)?,
            identity_nonce: Default::default(),
            fee_multiplier: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        })
    }
}

impl TryFromPlatformVersioned<CreatedDataContract> for DataContractCreateTransitionV0 {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: CreatedDataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        let (data_contract, identity_nonce) = value.data_contract_and_identity_nonce();
        Ok(DataContractCreateTransitionV0 {
            data_contract: data_contract.try_into_platform_versioned(platform_version)?,
            identity_nonce,
            fee_multiplier: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        })
    }
}
