use crate::balances::credits::TokenAmount;
use crate::block::epoch::EpochIndex;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
use crate::prelude::{
    BlockHeight, BlockHeightInterval, EpochInterval, TimestampMillis, TimestampMillisInterval,
};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt;

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
