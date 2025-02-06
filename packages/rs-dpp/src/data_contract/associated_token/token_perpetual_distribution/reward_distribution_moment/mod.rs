use crate::block::epoch::EpochIndex;
use crate::prelude::{BlockHeight, TimestampMillis};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum RewardDistributionMoment {
    /// The reward was distributed at a block height
    BlockBasedMoment(BlockHeight),
    /// The reward was distributed at a time
    TimeBasedMoment(TimestampMillis),
    /// The reward was distributed at an epoch
    EpochBasedMoment(EpochIndex),
}

impl RewardDistributionMoment {
    pub fn to_be_bytes_vec(&self) -> Vec<u8> {
        match self {
            RewardDistributionMoment::BlockBasedMoment(height) => height.to_be_bytes().to_vec(),
            RewardDistributionMoment::TimeBasedMoment(time) => time.to_be_bytes().to_vec(),
            RewardDistributionMoment::EpochBasedMoment(epoch) => epoch.to_be_bytes().to_vec(),
        }
    }
}

/// Implements `Display` for `RewardDistributionMoment`
impl fmt::Display for RewardDistributionMoment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RewardDistributionMoment::BlockBasedMoment(height) => {
                write!(f, "BlockBasedMoment({})", height)
            }
            RewardDistributionMoment::TimeBasedMoment(timestamp) => {
                write!(f, "TimeBasedMoment({})", timestamp)
            }
            RewardDistributionMoment::EpochBasedMoment(epoch) => {
                write!(f, "EpochBasedMoment({})", epoch)
            }
        }
    }
}
