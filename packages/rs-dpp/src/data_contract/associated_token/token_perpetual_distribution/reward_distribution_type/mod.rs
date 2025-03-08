mod accessors;
mod evaluate_interval;

use crate::balances::credits::TokenAmount;
use crate::block::epoch::EpochIndex;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
use crate::prelude::{BlockHeight, BlockHeightInterval, DataContract, EpochInterval, TimestampMillis, TimestampMillisInterval};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt;
use crate::data_contract::accessors::v1::DataContractV1Getters;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use crate::ProtocolError;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub enum RewardDistributionType {
    /// An amount of tokens is emitted every n blocks
    /// The start and end are included if set
    BlockBasedDistribution {
        interval: BlockHeightInterval,
        amount: TokenAmount,
        function: DistributionFunction,
        start: Option<BlockHeight>,
        end: Option<BlockHeight>,
    },
    /// An amount of tokens is emitted every amount of time given
    /// The start and end are included if set
    TimeBasedDistribution {
        interval: TimestampMillisInterval,
        amount: TokenAmount,
        function: DistributionFunction,
        start: Option<TimestampMillis>,
        end: Option<TimestampMillis>,
    },
    /// An amount of tokens is emitted every amount of epochs
    /// The start and end are included if set
    EpochBasedDistribution {
        interval: EpochInterval,
        amount: TokenAmount,
        function: DistributionFunction,
        start: Option<EpochIndex>,
        end: Option<EpochIndex>,
    },
}

impl RewardDistributionType {
    /// Determines the starting moment of reward distribution based on the contract creation time.
    ///
    /// This function returns the appropriate `RewardDistributionMoment`, which represents when
    /// a reward distribution should begin, based on the type of distribution and when the
    /// `DataContract` was created.
    ///
    /// # Arguments
    ///
    /// * `data_contract` - A reference to the `DataContract`, which contains details about
    ///   when the contract was created in terms of block height, timestamp, and epoch index.
    ///
    /// # Returns
    ///
    /// * `Some(RewardDistributionMoment)` if the contract's creation time can be mapped to
    ///   a valid distribution start moment.
    /// * `None` if the contract creation time is unavailable or not applicable.
    pub fn contract_creation_moment(
        &self,
        data_contract: &DataContract,
    ) -> Option<RewardDistributionMoment> {
        match self {
            RewardDistributionType::BlockBasedDistribution { .. } => data_contract
                .created_at_block_height()
                .map(RewardDistributionMoment::BlockBasedMoment),
            RewardDistributionType::TimeBasedDistribution { .. } => data_contract
                .created_at()
                .map(RewardDistributionMoment::TimeBasedMoment),
            RewardDistributionType::EpochBasedDistribution { .. } => data_contract
                .created_at_epoch()
                .map(RewardDistributionMoment::EpochBasedMoment),
        }
    }
    /// Converts a byte slice into the corresponding `RewardDistributionMoment` variant
    /// based on the type of reward distribution.
    ///
    /// This method interprets the provided bytes according to the expected type of the distribution:
    /// - `BlockBasedDistribution`: Interprets the bytes as a `BlockHeight` (`u64`).
    /// - `TimeBasedDistribution`: Interprets the bytes as a `TimestampMillis` (`u64`).
    /// - `EpochBasedDistribution`: Interprets the bytes as an `EpochIndex` (`u16`).
    ///
    /// # Parameters
    ///
    /// - `bytes`: A byte slice containing the serialized representation of the moment.
    ///
    /// # Returns
    ///
    /// - `Ok(RewardDistributionMoment)`: The successfully parsed reward distribution moment.
    /// - `Err(ProtocolError)`: If the provided bytes are of incorrect length.
    ///
    /// # Errors
    ///
    /// - `ProtocolError::DecodingError`: If the provided bytes slice does not have the expected length
    pub fn moment_from_bytes(
        &self,
        bytes: &[u8],
    ) -> Result<RewardDistributionMoment, ProtocolError> {
        match self {
            RewardDistributionType::BlockBasedDistribution { .. } => {
                if bytes.len() != 8 {
                    return Err(ProtocolError::DecodingError(
                        "Expected 8 bytes for BlockBasedMoment".to_string(),
                    ));
                }
                let mut array = [0u8; 8];
                array.copy_from_slice(bytes);
                Ok(RewardDistributionMoment::BlockBasedMoment(
                    u64::from_be_bytes(array),
                ))
            }
            RewardDistributionType::TimeBasedDistribution { .. } => {
                if bytes.len() != 8 {
                    return Err(ProtocolError::DecodingError(
                        "Expected 8 bytes for TimeBasedMoment".to_string(),
                    ));
                }
                let mut array = [0u8; 8];
                array.copy_from_slice(bytes);
                Ok(RewardDistributionMoment::TimeBasedMoment(
                    u64::from_be_bytes(array),
                ))
            }
            RewardDistributionType::EpochBasedDistribution { .. } => {
                if bytes.len() != 2 {
                    return Err(ProtocolError::DecodingError(
                        "Expected 2 bytes for EpochBasedMoment".to_string(),
                    ));
                }
                let mut array = [0u8; 2];
                array.copy_from_slice(bytes);
                Ok(RewardDistributionMoment::EpochBasedMoment(
                    u16::from_be_bytes(array),
                ))
            }
        }
    }
}
impl fmt::Display for RewardDistributionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RewardDistributionType::BlockBasedDistribution {
                interval,
                amount,
                function,
                start,
                end,
            } => {
                write!(
                    f,
                    "BlockBasedDistribution: {} tokens every {} blocks using {}",
                    amount, interval, function
                )?;
                if let Some(start) = start {
                    write!(f, ", starting at block {}", start)?;
                }
                if let Some(end) = end {
                    write!(f, ", ending at block {}", end)?;
                }
                Ok(())
            }
            RewardDistributionType::TimeBasedDistribution {
                interval,
                amount,
                function,
                start,
                end,
            } => {
                write!(
                    f,
                    "TimeBasedDistribution: {} tokens every {} milliseconds using {}",
                    amount, interval, function
                )?;
                if let Some(start) = start {
                    write!(f, ", starting at timestamp {}", start)?;
                }
                if let Some(end) = end {
                    write!(f, ", ending at timestamp {}", end)?;
                }
                Ok(())
            }
            RewardDistributionType::EpochBasedDistribution {
                interval,
                amount,
                function,
                start,
                end,
            } => {
                write!(
                    f,
                    "EpochBasedDistribution: {} tokens every {} epochs using {}",
                    amount, interval, function
                )?;
                if let Some(start) = start {
                    write!(f, ", starting at epoch {}", start)?;
                }
                if let Some(end) = end {
                    write!(f, ", ending at epoch {}", end)?;
                }
                Ok(())
            }
        }
    }
}
