use crate::block::block_info::BlockInfo;
use crate::data_contract::associated_token::token_perpetual_distribution::methods::v0::TokenPerpetualDistributionV0Methods;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use crate::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;

impl TokenPerpetualDistributionV0Methods for TokenPerpetualDistributionV0 {
    fn next_interval(&self, block_info: &BlockInfo) -> u64 {
        match self.distribution_type {
            // If the distribution is based on block height, return the next height where emissions occur.
            RewardDistributionType::BlockBasedDistribution { interval, .. } => {
                (block_info.height - block_info.height % interval).saturating_add(interval)
            }

            // If the distribution is based on time, return the next timestamp in milliseconds.
            RewardDistributionType::TimeBasedDistribution { interval, .. } => {
                (block_info.time_ms - block_info.time_ms % interval).saturating_add(interval)
            }

            // If the distribution is based on epochs, return the next epoch index.
            RewardDistributionType::EpochBasedDistribution { interval, .. } => {
                (block_info.epoch.index - block_info.epoch.index % interval)
                    .saturating_add(interval) as u64
            }
        }
    }
}
