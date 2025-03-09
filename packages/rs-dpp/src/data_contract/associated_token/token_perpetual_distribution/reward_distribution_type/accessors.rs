use crate::balances::credits::TokenAmount;
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

    /// Returns the optional start moment of the distribution.
    ///
    /// # Returns
    /// - `Some(RewardDistributionMoment::BlockBasedMoment)`, `Some(RewardDistributionMoment::TimeBasedMoment)`,
    ///   or `Some(RewardDistributionMoment::EpochBasedMoment)`, depending on the distribution type.
    /// - `None` if the start moment is not set.
    pub fn start(&self) -> Option<RewardDistributionMoment> {
        match self {
            RewardDistributionType::BlockBasedDistribution { start, .. } => {
                start.map(RewardDistributionMoment::BlockBasedMoment)
            }
            RewardDistributionType::TimeBasedDistribution { start, .. } => {
                start.map(RewardDistributionMoment::TimeBasedMoment)
            }
            RewardDistributionType::EpochBasedDistribution { start, .. } => {
                start.map(RewardDistributionMoment::EpochBasedMoment)
            }
        }
    }

    /// Returns the optional end moment of the distribution.
    ///
    /// # Returns
    /// - `Some(RewardDistributionMoment::BlockBasedMoment)`, `Some(RewardDistributionMoment::TimeBasedMoment)`,
    ///   or `Some(RewardDistributionMoment::EpochBasedMoment)`, depending on the distribution type.
    /// - `None` if the end moment is not set.
    pub fn end(&self) -> Option<RewardDistributionMoment> {
        match self {
            RewardDistributionType::BlockBasedDistribution { end, .. } => {
                end.map(RewardDistributionMoment::BlockBasedMoment)
            }
            RewardDistributionType::TimeBasedDistribution { end, .. } => {
                end.map(RewardDistributionMoment::TimeBasedMoment)
            }
            RewardDistributionType::EpochBasedDistribution { end, .. } => {
                end.map(RewardDistributionMoment::EpochBasedMoment)
            }
        }
    }
}
