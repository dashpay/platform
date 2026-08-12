use crate::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

use crate::identity::Purpose;
use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("An identity may have at most {limit} active public key(s) of purpose {purpose}")]
#[platform_serialize(unversioned)]
pub struct TooManyPublicKeysOfPurposeError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    purpose: Purpose,
    limit: u32,
}

impl TooManyPublicKeysOfPurposeError {
    pub fn new(purpose: Purpose, limit: u32) -> Self {
        Self { purpose, limit }
    }

    pub fn purpose(&self) -> Purpose {
        self.purpose
    }

    pub fn limit(&self) -> u32 {
        self.limit
    }
}

impl From<TooManyPublicKeysOfPurposeError> for ConsensusError {
    fn from(err: TooManyPublicKeysOfPurposeError) -> Self {
        Self::BasicError(BasicError::TooManyPublicKeysOfPurposeError(err))
    }
}
