use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;

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
use crate::identity::PartialIdentity;
use crate::identity::signer::Signer;
use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
use crate::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use crate::state_transition::state_transitions::contract::data_contract_create_transition::fields::{BINARY_FIELDS, IDENTIFIER_FIELDS, U32_FIELDS};

use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;

pub trait DataContractCreateTransitionV0Methods {

}

impl DataContractCreateTransitionV0Methods for DataContractCreateTransitionV0 {

}