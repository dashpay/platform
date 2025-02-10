use crate::balances::credits::TokenAmount;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;

impl RewardDistributionType {
    /// Returns the interval of the distribution.
    ///
    /// # Returns
    /// - `BlockHeightInterval`, `TimestampMillisInterval`, or `EpochInterval`, depending on the variant.
    pub fn interval(&self) -> u64 {
        match self {
            RewardDistributionType::BlockBasedDistribution { interval, .. } => *interval,
            RewardDistributionType::TimeBasedDistribution { interval, .. } => *interval,
            RewardDistributionType::EpochBasedDistribution { interval, .. } => *interval as u64,
        }
    }

    /// Returns the amount of tokens distributed at each interval.
    ///
    /// # Returns
    /// - `TokenAmount`: The emission amount per interval.
    pub fn amount(&self) -> TokenAmount {
        match self {
            RewardDistributionType::BlockBasedDistribution { amount, .. } => *amount,
            RewardDistributionType::TimeBasedDistribution { amount, .. } => *amount,
            RewardDistributionType::EpochBasedDistribution { amount, .. } => *amount,
        }
    }

    /// Returns the function defining the emission behavior.
    ///
    /// # Returns
    /// - `&DistributionFunction`: The function used for emission calculation.
    pub fn function(&self) -> &DistributionFunction {
        match self {
            RewardDistributionType::BlockBasedDistribution { function, .. } => function,
            RewardDistributionType::TimeBasedDistribution { function, .. } => function,
            RewardDistributionType::EpochBasedDistribution { function, .. } => function,
        }
    }

    /// Returns the optional start time of the distribution.
    ///
    /// # Returns
    /// - `Some(BlockHeight)`, `Some(TimestampMillis)`, or `Some(EpochIndex)`, depending on the variant.
    /// - `None` if the start time is not set.
    pub fn start(&self) -> Option<u64> {
        match self {
            RewardDistributionType::BlockBasedDistribution { start, .. } => *start,
            RewardDistributionType::TimeBasedDistribution { start, .. } => *start,
            RewardDistributionType::EpochBasedDistribution { start, .. } => start.map(|s| s as u64),
        }
    }

    /// Returns the optional end time of the distribution.
    ///
    /// # Returns
    /// - `Some(BlockHeight)`, `Some(TimestampMillis)`, or `Some(EpochIndex)`, depending on the variant.
    /// - `None` if the end time is not set.
    pub fn end(&self) -> Option<u64> {
        match self {
            RewardDistributionType::BlockBasedDistribution { end, .. } => *end,
            RewardDistributionType::TimeBasedDistribution { end, .. } => *end,
            RewardDistributionType::EpochBasedDistribution { end, .. } => end.map(|e| e as u64),
        }
    }
}
