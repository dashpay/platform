use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::TokenContractPosition;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Contract Token Positions are not contiguous. Missing position: {}, Followed position: {}",
    missing_position,
    followed_position
)]
#[platform_serialize(unversioned)]
pub struct NonContiguousContractTokenPositionsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    missing_position: TokenContractPosition,
    followed_position: TokenContractPosition,
}

impl NonContiguousContractTokenPositionsError {
    pub fn new(
        missing_position: TokenContractPosition,
        followed_position: TokenContractPosition,
    ) -> Self {
        Self {
            missing_position,
            followed_position,
        }
    }

    pub fn missing_position(&self) -> u16 {
        self.missing_position
    }

    pub fn followed_position(&self) -> u16 {
        self.followed_position
    }
}

impl From<NonContiguousContractTokenPositionsError> for ConsensusError {
    fn from(err: NonContiguousContractTokenPositionsError) -> Self {
        Self::BasicError(BasicError::NonContiguousContractTokenPositionsError(err))
    }
}
