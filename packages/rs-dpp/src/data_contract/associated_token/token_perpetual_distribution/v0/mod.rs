use crate::balances::credits::TokenAmount;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
use crate::prelude::{BlockHeightInterval, EpochInterval, TimestampMillisInterval};
use bincode::Encode;
use platform_serialization::de::Decode;
use serde::{Deserialize, Serialize};
use std::fmt;

pub type BaseTokenAmount = TokenAmount;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub enum RewardDistributionType {
    /// A fixed amount of tokens is emitted every n blocks
    BlockFixed(BlockHeightInterval, TokenAmount),
    /// A fixed amount of tokens is emitted every amount of time given
    TimeFixed(TimestampMillisInterval, TokenAmount),
    /// A fixed amount of tokens is emitted every amount of epochs
    EpochFixed(EpochInterval, TokenAmount),
    /// A variable amount of tokens is emitted every amount of time given
    TimeVariable(
        TimestampMillisInterval,
        BaseTokenAmount,
        DistributionFunction,
    ),
    /// A variable amount of tokens is emitted every amount of epochs
    EpochVariable(EpochInterval, BaseTokenAmount, DistributionFunction),
}

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct TokenPerpetualDistributionV0 {
    /// The distribution type that the token will use
    pub distribution_type: RewardDistributionType,
    /// Is the release of the token automatic if the owner id has enough balance?
    pub automatic_release: bool,
}

impl fmt::Display for RewardDistributionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RewardDistributionType::BlockFixed(interval, amount) => {
                write!(f, "BlockFixed: {} tokens every {} blocks", amount, interval)
            }
            RewardDistributionType::TimeFixed(interval, amount) => {
                write!(
                    f,
                    "TimeFixed: {} tokens every {} milliseconds",
                    amount, interval
                )
            }
            RewardDistributionType::EpochFixed(interval, amount) => {
                write!(f, "EpochFixed: {} tokens every {} epochs", amount, interval)
            }
            RewardDistributionType::TimeVariable(interval, base_amount, function) => {
                write!(
                    f,
                    "TimeVariable: Base {} tokens with function {} every {} milliseconds",
                    base_amount, function, interval
                )
            }
            RewardDistributionType::EpochVariable(interval, base_amount, function) => {
                write!(
                    f,
                    "EpochVariable: Base {} tokens with function {} every {} epochs",
                    base_amount, function, interval
                )
            }
        }
    }
}

impl fmt::Display for TokenPerpetualDistributionV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TokenPerpetualDistributionV0 {{\n  distribution_type: {},\n  automatic_release: {}\n}}",
            self.distribution_type,
            self.automatic_release
        )
    }
}
