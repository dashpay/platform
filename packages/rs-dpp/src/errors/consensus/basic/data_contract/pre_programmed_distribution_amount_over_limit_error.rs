use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::TokenContractPosition;
use crate::errors::ProtocolError;
use crate::prelude::TimestampMillis;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// The amounts a token's pre-programmed distribution releases at one time total more than
/// `i64::MAX`. Each release is stored as a sum tree of its recipients' amounts, so neither an
/// amount nor the total of a release can exceed what the sum tree holds.
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
    "Token at position {} has a pre-programmed distribution at {} whose amounts total more than the maximum of {}",
    token_position,
    timestamp,
    i64::MAX
)]
#[platform_serialize(unversioned)]
pub struct PreProgrammedDistributionAmountOverLimitError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    token_position: TokenContractPosition,
    timestamp: TimestampMillis,
}

impl PreProgrammedDistributionAmountOverLimitError {
    pub fn new(token_position: TokenContractPosition, timestamp: TimestampMillis) -> Self {
        Self {
            token_position,
            timestamp,
        }
    }

    pub fn token_position(&self) -> TokenContractPosition {
        self.token_position
    }

    pub fn timestamp(&self) -> TimestampMillis {
        self.timestamp
    }
}

impl From<PreProgrammedDistributionAmountOverLimitError> for ConsensusError {
    fn from(err: PreProgrammedDistributionAmountOverLimitError) -> Self {
        Self::BasicError(BasicError::PreProgrammedDistributionAmountOverLimitError(
            err,
        ))
    }
}
