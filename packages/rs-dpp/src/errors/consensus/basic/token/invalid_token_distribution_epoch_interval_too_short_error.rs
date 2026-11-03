use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::prelude::EpochInterval;
use crate::ProtocolError;
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
#[error("EpochBasedDistribution interval is too short: {interval}. Minimum allowed is 1 epoch.")]
#[platform_serialize(unversioned)]
pub struct InvalidTokenDistributionEpochIntervalTooShortError {
    interval: EpochInterval,
}

impl InvalidTokenDistributionEpochIntervalTooShortError {
    pub fn new(interval: EpochInterval) -> Self {
        Self { interval }
    }

    pub fn interval(&self) -> EpochInterval {
        self.interval
    }
}

impl From<InvalidTokenDistributionEpochIntervalTooShortError> for ConsensusError {
    fn from(err: InvalidTokenDistributionEpochIntervalTooShortError) -> Self {
        Self::BasicError(BasicError::InvalidTokenDistributionEpochIntervalTooShortError(err))
    }
}
