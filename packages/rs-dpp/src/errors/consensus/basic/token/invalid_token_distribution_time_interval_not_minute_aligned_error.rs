use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::prelude::TimestampMillisInterval;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("TimeBasedDistribution interval {interval} is not divisible by 60,000 ms (1 minute).")]
#[platform_serialize(unversioned)]
pub struct InvalidTokenDistributionTimeIntervalNotMinuteAlignedError {
    interval: TimestampMillisInterval,
}

impl InvalidTokenDistributionTimeIntervalNotMinuteAlignedError {
    pub fn new(interval: TimestampMillisInterval) -> Self {
        Self { interval }
    }

    pub fn interval(&self) -> TimestampMillisInterval {
        self.interval
    }
}

impl From<InvalidTokenDistributionTimeIntervalNotMinuteAlignedError> for ConsensusError {
    fn from(err: InvalidTokenDistributionTimeIntervalNotMinuteAlignedError) -> Self {
        Self::BasicError(BasicError::InvalidTokenDistributionTimeIntervalNotMinuteAlignedError(err))
    }
}
