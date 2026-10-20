use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
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
    "Data contract declares {} contract group memberships, at most {} are allowed",
    memberships_count,
    max_memberships
)]
#[platform_serialize(unversioned)]
pub struct ContractGroupMembershipsOverLimitError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    memberships_count: u32,
    max_memberships: u16,
}

impl ContractGroupMembershipsOverLimitError {
    pub fn new(memberships_count: u32, max_memberships: u16) -> Self {
        Self {
            memberships_count,
            max_memberships,
        }
    }

    pub fn memberships_count(&self) -> u32 {
        self.memberships_count
    }

    pub fn max_memberships(&self) -> u16 {
        self.max_memberships
    }
}

impl From<ContractGroupMembershipsOverLimitError> for ConsensusError {
    fn from(err: ContractGroupMembershipsOverLimitError) -> Self {
        Self::BasicError(BasicError::ContractGroupMembershipsOverLimitError(err))
    }
}
