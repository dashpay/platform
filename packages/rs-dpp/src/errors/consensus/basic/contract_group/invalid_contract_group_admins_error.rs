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
#[error("Contract group declares {} admins: at most {} distinct admins are allowed, none of them the owner", admins_count, max_admins)]
#[platform_serialize(unversioned)]
pub struct InvalidContractGroupAdminsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    admins_count: u32,
    max_admins: u16,
}

impl InvalidContractGroupAdminsError {
    pub fn new(admins_count: u32, max_admins: u16) -> Self {
        Self {
            admins_count,
            max_admins,
        }
    }

    pub fn admins_count(&self) -> u32 {
        self.admins_count
    }

    pub fn max_admins(&self) -> u16 {
        self.max_admins
    }
}

impl From<InvalidContractGroupAdminsError> for ConsensusError {
    fn from(err: InvalidContractGroupAdminsError) -> Self {
        Self::BasicError(BasicError::InvalidContractGroupAdminsError(err))
    }
}
