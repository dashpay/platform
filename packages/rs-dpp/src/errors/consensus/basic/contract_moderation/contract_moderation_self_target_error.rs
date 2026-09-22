use crate::consensus::basic::BasicError;
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
#[error("Identity {} can not moderate itself", identity_id)]
#[platform_serialize(unversioned)]
pub struct ContractModerationSelfTargetError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    identity_id: Identifier,
}

impl ContractModerationSelfTargetError {
    pub fn new(identity_id: Identifier) -> Self {
        Self { identity_id }
    }

    pub fn identity_id(&self) -> Identifier {
        self.identity_id
    }
}

impl From<ContractModerationSelfTargetError> for ConsensusError {
    fn from(err: ContractModerationSelfTargetError) -> Self {
        Self::BasicError(BasicError::ContractModerationSelfTargetError(err))
    }
}
