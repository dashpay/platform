use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::data_contract::config::moderation::ContractModerationList;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error("Contract {} does not keep a {}", contract_id, list)]
#[platform_serialize(unversioned)]
pub struct ContractModerationNotEnabledError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    list: ContractModerationList,
}

impl ContractModerationNotEnabledError {
    pub fn new(contract_id: Identifier, list: ContractModerationList) -> Self {
        Self { contract_id, list }
    }

    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    pub fn list(&self) -> ContractModerationList {
        self.list
    }
}

impl From<ContractModerationNotEnabledError> for ConsensusError {
    fn from(err: ContractModerationNotEnabledError) -> Self {
        Self::StateError(StateError::ContractModerationNotEnabledError(err))
    }
}
