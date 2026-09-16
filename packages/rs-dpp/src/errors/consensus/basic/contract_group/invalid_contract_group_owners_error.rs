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
#[error("Contract group declares {} owners: a multi owner group needs at least 2 distinct owners and at most {}", owners_count, max_owners)]
#[platform_serialize(unversioned)]
pub struct InvalidContractGroupOwnersError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    owners_count: u32,
    max_owners: u16,
}

impl InvalidContractGroupOwnersError {
    pub fn new(owners_count: u32, max_owners: u16) -> Self {
        Self {
            owners_count,
            max_owners,
        }
    }

    pub fn owners_count(&self) -> u32 {
        self.owners_count
    }

    pub fn max_owners(&self) -> u16 {
        self.max_owners
    }
}

impl From<InvalidContractGroupOwnersError> for ConsensusError {
    fn from(err: InvalidContractGroupOwnersError) -> Self {
        Self::BasicError(BasicError::InvalidContractGroupOwnersError(err))
    }
}
