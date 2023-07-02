mod types;
#[cfg(feature = "json-object")]
mod json_conversion;
#[cfg(feature = "platform-value")]
mod value_conversion;
pub(super) mod v0_methods;
mod state_transition_like;
mod identity_signed;


use platform_value::{BinaryData, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

use crate::platform_serialization::PlatformSignable;
use crate::serialization_traits::PlatformSerializable;
use crate::serialization_traits::{PlatformDeserializable, Signable};
use bincode::{config, Decode, Encode};
use platform_serialization::{PlatformDeserialize, PlatformSerialize};

use crate::state_transition::StateTransition;
use crate::{data_contract::DataContract, identity::KeyID, state_transition::{
    StateTransitionFieldTypes, StateTransitionLike,
    StateTransitionType,
}, Convertible, ProtocolError, NonConsensusError};
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransition;


#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Encode,
    Decode,
    PlatformDeserialize,
    PlatformSerialize,
    PartialEq,
    PlatformSignable,
)]
#[serde(rename_all = "camelCase")]
#[platform_error_type(ProtocolError)]
pub struct DataContractUpdateTransitionV0 {
    pub data_contract: DataContract,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

impl Default for DataContractUpdateTransitionV0 {
    fn default() -> Self {
        DataContractUpdateTransitionV0 {
            signature_public_key_id: 0,
            signature: BinaryData::default(),
            data_contract: Default::default(),
        }
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