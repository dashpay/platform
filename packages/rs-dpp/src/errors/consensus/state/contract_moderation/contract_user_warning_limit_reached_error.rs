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
    "Identity {} already carries {} warnings on contract {}, the most it may at a time; clear them before warning it again",
    identity_id,
    max_warnings,
    contract_id
)]
#[platform_serialize(unversioned)]
pub struct ContractUserWarningLimitReachedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    identity_id: Identifier,
    max_warnings: u16,
}

impl ContractUserWarningLimitReachedError {
    pub fn new(contract_id: Identifier, identity_id: Identifier, max_warnings: u16) -> Self {
        Self {
            contract_id,
            identity_id,
            max_warnings,
        }
    }

    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    pub fn identity_id(&self) -> Identifier {
        self.identity_id
    }

    pub fn max_warnings(&self) -> u16 {
        self.max_warnings
    }
}

impl From<ContractUserWarningLimitReachedError> for ConsensusError {
    fn from(err: ContractUserWarningLimitReachedError) -> Self {
        Self::StateError(StateError::ContractUserWarningLimitReachedError(err))
    }
}
