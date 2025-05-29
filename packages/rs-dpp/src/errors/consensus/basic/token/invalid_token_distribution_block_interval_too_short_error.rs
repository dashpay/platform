use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::prelude::BlockHeightInterval;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("BlockBasedDistribution interval is too short: {interval}. Minimum allowed is 100.")]
#[platform_serialize(unversioned)]
pub struct InvalidTokenDistributionBlockIntervalTooShortError {
    interval: BlockHeightInterval,
}

impl InvalidTokenDistributionBlockIntervalTooShortError {
    pub fn new(interval: BlockHeightInterval) -> Self {
        Self { interval }
    }

    pub fn interval(&self) -> BlockHeightInterval {
        self.interval
    }
}

impl From<InvalidTokenDistributionBlockIntervalTooShortError> for ConsensusError {
    fn from(err: InvalidTokenDistributionBlockIntervalTooShortError) -> Self {
        Self::BasicError(BasicError::InvalidTokenDistributionBlockIntervalTooShortError(err))
    }
}
