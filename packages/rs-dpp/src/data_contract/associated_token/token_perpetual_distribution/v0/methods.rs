use crate::block::block_info::BlockInfo;
use crate::data_contract::associated_token::token_perpetual_distribution::methods::v0::TokenPerpetualDistributionV0Methods;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment::{BlockBasedMoment, EpochBasedMoment, TimeBasedMoment};
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use crate::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;

impl TokenPerpetualDistributionV0Methods for TokenPerpetualDistributionV0 {
    fn next_interval(&self, block_info: &BlockInfo) -> RewardDistributionMoment {
        match self.distribution_type {
            // If the distribution is based on block height, return the next height where emissions occur.
            RewardDistributionType::BlockBasedDistribution { interval, .. } => BlockBasedMoment(
                (block_info.height - block_info.height % interval).saturating_add(interval),
            ),

            // If the distribution is based on time, return the next timestamp in milliseconds.
            RewardDistributionType::TimeBasedDistribution { interval, .. } => TimeBasedMoment(
                (block_info.time_ms - block_info.time_ms % interval).saturating_add(interval),
            ),

            // If the distribution is based on epochs, return the next epoch index.
            RewardDistributionType::EpochBasedDistribution { interval, .. } => EpochBasedMoment(
                (block_info.epoch.index - block_info.epoch.index % interval)
                    .saturating_add(interval),
            ),
        }
    }

    fn current_interval(&self, block_info: &BlockInfo) -> RewardDistributionMoment {
        match self.distribution_type {
            // If the distribution is based on block height, return the next height where emissions occur.
            RewardDistributionType::BlockBasedDistribution { interval, .. } => {
                BlockBasedMoment(block_info.height - block_info.height % interval)
            }

            // If the distribution is based on time, return the next timestamp in milliseconds.
            RewardDistributionType::TimeBasedDistribution { interval, .. } => {
                TimeBasedMoment(block_info.time_ms - block_info.time_ms % interval)
            }

            // If the distribution is based on epochs, return the next epoch index.
            RewardDistributionType::EpochBasedDistribution { interval, .. } => {
                if interval == 1 {
                    EpochBasedMoment(block_info.epoch.index)
                } else {
                    EpochBasedMoment(block_info.epoch.index - block_info.epoch.index % interval)
                }
            }
        }
    }
}
