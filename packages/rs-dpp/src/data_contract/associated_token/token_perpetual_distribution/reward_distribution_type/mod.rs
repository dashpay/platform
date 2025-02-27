use crate::balances::credits::TokenAmount;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
use crate::prelude::{BlockHeightInterval, EpochInterval, TimestampMillisInterval};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub enum RewardDistributionType {
    /// An amount of tokens is emitted every n blocks
    BlockBasedDistribution(BlockHeightInterval, TokenAmount, DistributionFunction),
    /// An amount of tokens is emitted every amount of time given
    TimeBasedDistribution(TimestampMillisInterval, TokenAmount, DistributionFunction),
    /// An amount of tokens is emitted every amount of epochs
    EpochBasedDistribution(EpochInterval, TokenAmount, DistributionFunction),
}

impl fmt::Display for RewardDistributionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RewardDistributionType::BlockBasedDistribution(interval, amount, function) => {
                write!(
                    f,
                    "BlockBasedDistribution: {} tokens every {} blocks using {}",
                    amount, interval, function
                )
            }
            RewardDistributionType::TimeBasedDistribution(interval, amount, function) => {
                write!(
                    f,
                    "TimeBasedDistribution: {} tokens every {} milliseconds using {}",
                    amount, interval, function
                )
            }
            RewardDistributionType::EpochBasedDistribution(interval, amount, function) => {
                write!(
                    f,
                    "EpochBasedDistribution: {} tokens every {} epochs using {}",
                    amount, interval, function
                )
            }
        }
    }
}
