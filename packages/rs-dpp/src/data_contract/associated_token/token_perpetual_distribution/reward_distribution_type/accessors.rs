use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;

impl RewardDistributionType {
    /// Returns the interval of the distribution.
    ///
    /// # Returns
    /// - `BlockHeightInterval`, `TimestampMillisInterval`, or `EpochInterval`, depending on the variant.
    pub fn interval(&self) -> RewardDistributionMoment {
        match self {
            RewardDistributionType::BlockBasedDistribution { interval, .. } => {
                RewardDistributionMoment::BlockBasedMoment(*interval)
            }
            RewardDistributionType::TimeBasedDistribution { interval, .. } => {
                RewardDistributionMoment::TimeBasedMoment(*interval)
            }
            RewardDistributionType::EpochBasedDistribution { interval, .. } => {
                RewardDistributionMoment::EpochBasedMoment(*interval)
            }
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
}
