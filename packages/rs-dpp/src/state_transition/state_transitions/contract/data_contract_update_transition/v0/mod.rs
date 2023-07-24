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

use crate::platform_serialization::PlatformSignable;
use crate::serialization_traits::PlatformSerializable;
use crate::serialization_traits::{PlatformDeserializable, Signable};
use bincode::{config, Decode, Encode};
use platform_serialization::{PlatformDeserialize, PlatformSerialize};

use crate::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use crate::state_transition::StateTransition;
use crate::{
    data_contract::DataContract,
    identity::KeyID,
    state_transition::{StateTransitionFieldTypes, StateTransitionLike, StateTransitionType},
    Convertible, NonConsensusError, ProtocolError,
};

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PlatformDeserialize,
    PlatformSerialize,
    PartialEq,
    PlatformSignable,
)]
#[serde(rename_all = "camelCase")]
#[platform_error_type(ProtocolError)]
#[platform_serialize(allow_nested)]
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
