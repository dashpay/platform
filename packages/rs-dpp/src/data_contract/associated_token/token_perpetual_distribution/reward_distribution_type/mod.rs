mod accessors;
mod evaluate_interval;

use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::{DistributionFunction, MAX_DISTRIBUTION_CYCLES_PARAM};
use crate::prelude::{BlockHeightInterval, DataContract, EpochInterval, TimestampMillisInterval};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt;
use crate::data_contract::accessors::v1::DataContractV1Getters;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use crate::ProtocolError;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub enum RewardDistributionType {
    /// An amount of tokens is emitted every n blocks.
    /// The start and end are included if set.
    /// If start is not set then it will start at the height of the block when the data contract
    /// is registered.
    BlockBasedDistribution {
        interval: BlockHeightInterval,
        function: DistributionFunction,
    },
    /// An amount of tokens is emitted every amount of time given.
    /// The start and end are included if set.
    /// If start is not set then it will start at the time of the block when the data contract
    /// is registered.
    TimeBasedDistribution {
        interval: TimestampMillisInterval,
        function: DistributionFunction,
    },
    /// An amount of tokens is emitted every amount of epochs.
    /// The start and end are included if set.
    /// If start is not set then it will start at the epoch of the block when the data contract
    /// is registered. A distribution would happen at the start of the following epoch, even if it
    /// is just 1 block later.
    EpochBasedDistribution {
        interval: EpochInterval,
        function: DistributionFunction,
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

    /// Determines the maximum cycle moment allowed based on the last paid moment,
    /// the current cycle moment, and the maximum allowed token redemption cycles.
    ///
    /// This function calculates a capped distribution moment (`RewardDistributionMoment`) by limiting
    /// the range between the `last_paid_moment` (or start) and the `current_cycle_moment` to the
    /// maximum allowed number of redemption cycles (`max_cycles`).
    ///
    /// # Arguments
    /// - `last_paid_moment`: Optional last moment at which tokens were claimed.
    /// - `current_cycle_moment`: The current cycle moment as of the current block.
    /// - `max_cycles`: The maximum number of redemption cycles permitted per claim.
    ///
    /// # Returns
    /// - `RewardDistributionMoment`: The maximum allowed cycle moment capped by `max_cycles`.
    pub fn max_cycle_moment(
        &self,
        start_moment: RewardDistributionMoment,
        current_cycle_moment: RewardDistributionMoment,
        max_non_fixed_amount_cycles: u32,
    ) -> Result<RewardDistributionMoment, ProtocolError> {
        let max_cycles = if matches!(self.function(), DistributionFunction::FixedAmount { .. }) {
            // This is much easier to calculate as it's always fixed, so we can have a near unlimited amount of cycles
            //
            MAX_DISTRIBUTION_CYCLES_PARAM
        } else {
            max_non_fixed_amount_cycles as u64
        };
        let interval = self.interval();

        // Calculate maximum allowed moment based on distribution type
        match (start_moment, interval, current_cycle_moment) {
            (
                RewardDistributionMoment::BlockBasedMoment(start),
                RewardDistributionMoment::BlockBasedMoment(step),
                RewardDistributionMoment::BlockBasedMoment(current),
            ) => Ok(RewardDistributionMoment::BlockBasedMoment(
                (start + step.saturating_mul(max_cycles)).min(current),
            )),
            (
                RewardDistributionMoment::TimeBasedMoment(start),
                RewardDistributionMoment::TimeBasedMoment(step),
                RewardDistributionMoment::TimeBasedMoment(current),
            ) => Ok(RewardDistributionMoment::TimeBasedMoment(
                (start + step.saturating_mul(max_cycles)).min(current),
            )),
            (
                RewardDistributionMoment::EpochBasedMoment(start),
                RewardDistributionMoment::EpochBasedMoment(step),
                RewardDistributionMoment::EpochBasedMoment(current),
            ) => Ok(RewardDistributionMoment::EpochBasedMoment(
                // For an epoch reward, if you are in epoch 3 you can't get rewarded for epoch 3, but only epoch 2
                (start + step.saturating_mul(max_cycles as u16)).min(current.saturating_sub(1)),
            )),
            _ => Err(ProtocolError::CorruptedCodeExecution(
                "Mismatch moment types".to_string(),
            )),
        }
    }
}
impl fmt::Display for RewardDistributionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RewardDistributionType::BlockBasedDistribution { interval, function } => {
                write!(
                    f,
                    "BlockBasedDistribution: every {} blocks using {}",
                    interval, function
                )?;
                Ok(())
            }
            RewardDistributionType::TimeBasedDistribution { interval, function } => {
                write!(
                    f,
                    "TimeBasedDistribution: every {} milliseconds using {}",
                    interval, function
                )?;
                Ok(())
            }
            RewardDistributionType::EpochBasedDistribution { interval, function } => {
                write!(
                    f,
                    "EpochBasedDistribution: every {} epochs using {}",
                    interval, function
                )?;
                Ok(())
            }
        }
    }
}
