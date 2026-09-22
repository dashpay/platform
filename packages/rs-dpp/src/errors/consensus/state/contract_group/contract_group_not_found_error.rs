use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
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
#[error("Contract group {} was not found", contract_group_id)]
#[platform_serialize(unversioned)]
pub struct ContractGroupNotFoundError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_group_id: Identifier,
}

impl ContractGroupNotFoundError {
    pub fn new(contract_group_id: Identifier) -> Self {
        Self { contract_group_id }
    }

    pub fn contract_group_id(&self) -> &Identifier {
        &self.contract_group_id
    }
}

impl From<ContractGroupNotFoundError> for ConsensusError {
    fn from(err: ContractGroupNotFoundError) -> Self {
        Self::StateError(StateError::ContractGroupNotFoundError(err))
    }
}
