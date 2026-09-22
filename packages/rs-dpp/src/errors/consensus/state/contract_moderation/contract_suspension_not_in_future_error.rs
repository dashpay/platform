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
    "Suspension of identity {} on contract {} ends at {} which is not after the block time {}",
    identity_id,
    contract_id,
    until,
    block_time
)]
#[platform_serialize(unversioned)]
pub struct ContractSuspensionNotInFutureError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    identity_id: Identifier,
    until: TimestampMillis,
    block_time: TimestampMillis,
}

impl ContractSuspensionNotInFutureError {
    pub fn new(
        contract_id: Identifier,
        identity_id: Identifier,
        until: TimestampMillis,
        block_time: TimestampMillis,
    ) -> Self {
        Self {
            contract_id,
            identity_id,
            until,
            block_time,
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

    pub fn block_time(&self) -> TimestampMillis {
        self.block_time
    }
}

impl From<ContractSuspensionNotInFutureError> for ConsensusError {
    fn from(err: ContractSuspensionNotInFutureError) -> Self {
        Self::StateError(StateError::ContractSuspensionNotInFutureError(err))
    }
}
