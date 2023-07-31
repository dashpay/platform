mod identity_signed;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use platform_value::{BinaryData, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

use crate::serialization::PlatformSerializable;
use crate::serialization::{PlatformDeserializable, Signable};
use bincode::{config, Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_version::version::PlatformVersion;
use platform_version::{TryFromPlatformVersioned, TryIntoPlatformVersioned};

use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use crate::state_transition::StateTransition;
use crate::{
    data_contract::DataContract,
    identity::KeyID,
    state_transition::{StateTransitionFieldTypes, StateTransitionLike, StateTransitionType},
    Convertible, NonConsensusError, ProtocolError,
};

#[derive(Debug, Clone, Encode, Decode, PartialEq, PlatformSignable)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]

pub struct DataContractUpdateTransitionV0 {
    pub data_contract: DataContractInSerializationFormat,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

impl TryFromPlatformVersioned<DataContract> for DataContractUpdateTransitionV0 {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        Ok(DataContractUpdateTransitionV0 {
            data_contract: value.try_into_platform_versioned(platform_version)?,
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
