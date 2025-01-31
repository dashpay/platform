use crate::block::block_info::BlockInfo;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use crate::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;

pub trait TokenPerpetualDistributionV0Accessors {
    fn distribution_type(&self) -> &RewardDistributionType;

    fn set_distribution_type(&mut self, distribution_type: RewardDistributionType);
}

pub trait TokenPerpetualDistributionV0Methods {
    /// we use u64 as a catch-all for any type of interval we might have, eg time, block or epoch
    fn next_interval(&self, block_info: &BlockInfo) -> u64;
}
