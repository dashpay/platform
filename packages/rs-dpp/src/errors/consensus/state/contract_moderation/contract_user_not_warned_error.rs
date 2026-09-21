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
#[error(
    "Identity {} carries no warning on contract {}",
    identity_id,
    contract_id
)]
#[platform_serialize(unversioned)]
pub struct ContractUserNotWarnedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    identity_id: Identifier,
}

impl ContractUserNotWarnedError {
    pub fn new(contract_id: Identifier, identity_id: Identifier) -> Self {
        Self {
            contract_id,
            identity_id,
        }
    }

    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    pub fn identity_id(&self) -> Identifier {
        self.identity_id
    }
}

impl From<ContractUserNotWarnedError> for ConsensusError {
    fn from(err: ContractUserNotWarnedError) -> Self {
        Self::StateError(StateError::ContractUserNotWarnedError(err))
    }
}
