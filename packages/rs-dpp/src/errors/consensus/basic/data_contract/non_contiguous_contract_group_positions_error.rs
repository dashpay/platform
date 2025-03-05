use crate::data_contract::GroupContractPosition;
use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Contract Group Positions are not contiguous. Missing position: {}, Followed position: {}",
    missing_position,
    followed_position
)]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct NonContiguousContractGroupPositionsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub missing_position: GroupContractPosition,
    pub followed_position: GroupContractPosition,
}

impl NonContiguousContractGroupPositionsError {
    pub fn new(
        missing_position: GroupContractPosition,
        followed_position: GroupContractPosition,
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

impl From<NonContiguousContractGroupPositionsError> for ConsensusError {
    fn from(err: NonContiguousContractGroupPositionsError) -> Self {
        Self::BasicError(BasicError::NonContiguousContractGroupPositionsError(err))
    }
}
