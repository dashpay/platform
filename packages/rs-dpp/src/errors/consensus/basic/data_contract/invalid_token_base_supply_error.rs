use crate::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

use bincode::{Decode, Encode};
#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Invalid token base supply. Given base supply: {}, Max allowed base supply: {}",
    base_supply,
    i64::MAX
)]
#[platform_serialize(unversioned)]
pub struct InvalidTokenBaseSupplyError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    base_supply: u64,
}

impl InvalidTokenBaseSupplyError {
    pub fn new(base_supply: u64) -> Self {
        Self { base_supply }
    }

    pub fn base_supply(&self) -> u64 {
        self.base_supply
    }
}

impl From<InvalidTokenBaseSupplyError> for ConsensusError {
    fn from(err: InvalidTokenBaseSupplyError) -> Self {
        Self::BasicError(BasicError::InvalidTokenBaseSupplyError(err))
    }
}
