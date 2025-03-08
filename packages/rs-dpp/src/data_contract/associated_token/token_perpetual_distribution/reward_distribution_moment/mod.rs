use crate::block::epoch::EpochIndex;
use crate::prelude::{BlockHeight, TimestampMillis};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Add;
use crate::block::block_info::BlockInfo;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use crate::ProtocolError;

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
    /// Checks if two `RewardDistributionMoment`s are of the same type.
    pub fn same_type(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::BlockBasedMoment(_), Self::BlockBasedMoment(_))
                | (Self::TimeBasedMoment(_), Self::TimeBasedMoment(_))
                | (Self::EpochBasedMoment(_), Self::EpochBasedMoment(_))
        )
    }

    /// Converts a `RewardDistributionMoment` into a `u64` representation.
    ///
    /// # Returns
    /// - The underlying numerical value of the moment as a `u64`.
    pub fn to_u64(&self) -> u64 {
        match self {
            RewardDistributionMoment::BlockBasedMoment(height) => *height as u64,
            RewardDistributionMoment::TimeBasedMoment(timestamp) => *timestamp,
            RewardDistributionMoment::EpochBasedMoment(epoch) => *epoch as u64,
        }
    }
}

impl From<RewardDistributionMoment> for u64 {
    /// Converts a `RewardDistributionMoment` into a `u64`.
    ///
    /// This conversion preserves the underlying numerical value.
    fn from(moment: RewardDistributionMoment) -> Self {
        moment.to_u64()
    }
}
impl Add for RewardDistributionMoment {
    type Output = Result<RewardDistributionMoment, ProtocolError>;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (
                RewardDistributionMoment::BlockBasedMoment(a),
                RewardDistributionMoment::BlockBasedMoment(b),
            ) => a
                .checked_add(b)
                .map(RewardDistributionMoment::BlockBasedMoment)
                .ok_or(ProtocolError::Overflow("Block height addition overflow")),
            (
                RewardDistributionMoment::TimeBasedMoment(a),
                RewardDistributionMoment::TimeBasedMoment(b),
            ) => a
                .checked_add(b)
                .map(RewardDistributionMoment::TimeBasedMoment)
                .ok_or(ProtocolError::Overflow("Timestamp addition overflow")),
            (
                RewardDistributionMoment::EpochBasedMoment(a),
                RewardDistributionMoment::EpochBasedMoment(b),
            ) => a
                .checked_add(b)
                .map(RewardDistributionMoment::EpochBasedMoment)
                .ok_or(ProtocolError::Overflow("Epoch index addition overflow")),
            _ => Err(ProtocolError::AddingDifferentTypes(
                "Cannot add different types of RewardDistributionMoment".to_string(),
            )),
        }
    }
}

impl PartialEq<&u64> for RewardDistributionMoment {
    fn eq(&self, other: &&u64) -> bool {
        match self {
            RewardDistributionMoment::BlockBasedMoment(value) => value == *other,
            RewardDistributionMoment::TimeBasedMoment(value) => value == *other,
            RewardDistributionMoment::EpochBasedMoment(value) => {
                if **other > u16::MAX as u64 {
                    false
                } else {
                    value == &(**other as u16)
                }
            }
        }
    }
}

impl PartialEq<u64> for RewardDistributionMoment {
    fn eq(&self, other: &u64) -> bool {
        self == &other
    }
}

impl PartialEq<&u32> for RewardDistributionMoment {
    fn eq(&self, other: &&u32) -> bool {
        match self {
            RewardDistributionMoment::BlockBasedMoment(value) => *value as u32 == **other,
            RewardDistributionMoment::TimeBasedMoment(value) => *value as u32 == **other,
            RewardDistributionMoment::EpochBasedMoment(value) => *value as u32 == **other,
        }
    }
}

impl PartialEq<u32> for RewardDistributionMoment {
    fn eq(&self, other: &u32) -> bool {
        self == &other
    }
}

impl PartialEq<&u16> for RewardDistributionMoment {
    fn eq(&self, other: &&u16) -> bool {
        match self {
            RewardDistributionMoment::BlockBasedMoment(value) => *value as u16 == **other,
            RewardDistributionMoment::TimeBasedMoment(value) => *value as u16 == **other,
            RewardDistributionMoment::EpochBasedMoment(value) => *value == **other,
        }
    }
}

impl PartialEq<u16> for RewardDistributionMoment {
    fn eq(&self, other: &u16) -> bool {
        self == &other
    }
}

impl PartialEq<&usize> for RewardDistributionMoment {
    fn eq(&self, other: &&usize) -> bool {
        match self {
            RewardDistributionMoment::BlockBasedMoment(value) => *value as usize == **other,
            RewardDistributionMoment::TimeBasedMoment(value) => *value as usize == **other,
            RewardDistributionMoment::EpochBasedMoment(value) => *value as usize == **other,
        }
    }
}

impl PartialEq<usize> for RewardDistributionMoment {
    fn eq(&self, other: &usize) -> bool {
        self == &other
    }
}

impl RewardDistributionMoment {
    /// Converts a reference to `BlockInfo` and a `RewardDistributionType` into a `RewardDistributionMoment`.
    ///
    /// This determines the appropriate `RewardDistributionMoment` based on the type of
    /// `RewardDistributionType`. The function selects:
    /// - **Block height** for block-based distributions.
    /// - **Timestamp (milliseconds)** for time-based distributions.
    /// - **Epoch index** for epoch-based distributions.
    ///
    /// # Arguments
    ///
    /// * `block_info` - A reference to the `BlockInfo` struct containing blockchain state details.
    /// * `distribution_type` - The `RewardDistributionType` to determine which moment should be used.
    ///
    /// # Returns
    ///
    /// Returns a `RewardDistributionMoment` corresponding to the type of distribution.
    pub fn from_block_info(
        block_info: &BlockInfo,
        distribution_type: &RewardDistributionType,
    ) -> Self {
        match distribution_type {
            RewardDistributionType::BlockBasedDistribution { .. } => {
                RewardDistributionMoment::BlockBasedMoment(block_info.height)
            }
            RewardDistributionType::TimeBasedDistribution { .. } => {
                RewardDistributionMoment::TimeBasedMoment(block_info.time_ms)
            }
            RewardDistributionType::EpochBasedDistribution { .. } => {
                RewardDistributionMoment::EpochBasedMoment(block_info.epoch.index)
            }
        }
    }
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
