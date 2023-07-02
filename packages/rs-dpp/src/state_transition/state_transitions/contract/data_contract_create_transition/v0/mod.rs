#[cfg(feature = "json-object")]
mod json_conversion;
#[cfg(feature = "platform-value")]
mod value_conversion;
mod state_transition_like;
pub(super) mod v0_methods;
mod types;

use crate::serialization_traits::PlatformSerializable;
use platform_serialization::PlatformSignable;
use platform_serialization::{PlatformDeserialize, PlatformSerialize};

use platform_value::{BinaryData, Bytes32, IntegerReplacementType, ReplacementType, Value};
use serde::{Deserialize, Serialize};

use crate::{Convertible, data_contract::DataContract, identity::KeyID, NonConsensusError, prelude::Identifier, ProtocolError, state_transition::{
    StateTransitionConvert, StateTransitionLike,
    StateTransitionType,
}};

use crate::serialization_traits::{PlatformDeserializable, Signable};
use bincode::{config, Decode, Encode};
use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
use crate::state_transition::state_transitions::contract::data_contract_create_transition::fields::{BINARY_FIELDS, IDENTIFIER_FIELDS, U32_FIELDS};

use crate::state_transition::StateTransition;

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
pub struct DataContractCreateTransitionV0 {
    pub data_contract: DataContract,
    pub entropy: Bytes32,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

impl Default for DataContractCreateTransitionV0 {
    fn default() -> Self {
        DataContractCreateTransitionV0 {
            entropy: Bytes32::default(),
            signature_public_key_id: 0,
            signature: BinaryData::default(),
            data_contract: Default::default(),
        }
    }
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

impl From<DataContract> for DataContractCreateTransitionV0 {
    fn from(value: DataContract) -> Self {
        DataContractCreateTransitionV0 {
            data_contract: value,
            entropy: Default::default(),
            signature_public_key_id: 0,
            signature: Default::default(),
        }
    }
}
