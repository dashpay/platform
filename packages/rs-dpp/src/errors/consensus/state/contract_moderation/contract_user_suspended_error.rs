use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::TimestampMillis;
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
    "Identity {} is suspended on contract {} until {} and can not act on its documents",
    identity_id,
    contract_id,
    until
)]
#[platform_serialize(unversioned)]
pub struct ContractUserSuspendedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    identity_id: Identifier,
    until: TimestampMillis,
}

impl ContractUserSuspendedError {
    pub fn new(contract_id: Identifier, identity_id: Identifier, until: TimestampMillis) -> Self {
        Self {
            contract_id,
            identity_id,
            until,
        }
    }

    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    pub fn identity_id(&self) -> Identifier {
        self.identity_id
    }

    pub fn until(&self) -> TimestampMillis {
        self.until
    }
}

impl From<ContractUserSuspendedError> for ConsensusError {
    fn from(err: ContractUserSuspendedError) -> Self {
        Self::StateError(StateError::ContractUserSuspendedError(err))
    }
}
