mod identity_signed;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
mod state_transition_like;
mod types;
pub(crate) mod v0_methods;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use crate::serialization::{PlatformSerializable, PlatformSerializableWithPlatformVersion};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};

use platform_value::{BinaryData, Bytes32, IntegerReplacementType, ReplacementType, Value};
use serde::{Deserialize, Serialize};

use crate::{
    data_contract::DataContract,
    identity::KeyID,
    prelude::Identifier,
    state_transition::{StateTransitionFieldTypes, StateTransitionLike, StateTransitionType},
    Convertible, NonConsensusError, ProtocolError,
};

use crate::serialization::{PlatformDeserializable, Signable};
use bincode::{config, Decode, Encode};
use crate::data_contract::created_data_contract::CreatedDataContract;
use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
use crate::state_transition::state_transitions::contract::data_contract_create_transition::fields::{BINARY_FIELDS, IDENTIFIER_FIELDS, U32_FIELDS};

use crate::state_transition::StateTransition;
use crate::version::PlatformVersion;
pub use v0_methods::DataContractCreateTransitionV0Methods;

///DataContractCreateTransitionV0 has the same encoding structure

#[derive(Debug, Clone, PlatformDeserialize, PlatformSerialize, PartialEq, PlatformSignable)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[platform_error_type(ProtocolError)]
pub struct DataContractCreateTransitionV0 {
    #[platform_serialize(versioned)]
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

impl From<CreatedDataContract> for DataContractCreateTransitionV0 {
    fn from(value: CreatedDataContract) -> Self {
        let (data_contract, entropy) = value.data_contract_and_entropy_owned();
        DataContractCreateTransitionV0 {
            data_contract,
            entropy,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
    }
}
