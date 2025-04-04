use v0::TokenPerpetualDistributionV0Methods;
use crate::block::block_info::BlockInfo;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
use crate::data_contract::associated_token::token_perpetual_distribution::methods::v0::TokenPerpetualDistributionV0Accessors;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use crate::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;

pub mod v0;

impl TokenPerpetualDistributionV0Accessors for TokenPerpetualDistribution {
    fn distribution_type(&self) -> &RewardDistributionType {
        match self {
            TokenPerpetualDistribution::V0(v0) => &v0.distribution_type,
        }
    }

    fn set_distribution_type(&mut self, distribution_type: RewardDistributionType) {
        match self {
            TokenPerpetualDistribution::V0(v0) => v0.distribution_type = distribution_type,
        }
    }

    fn distribution_recipient(&self) -> TokenDistributionRecipient {
        match self {
            TokenPerpetualDistribution::V0(v0) => v0.distribution_recipient,
        }
    }

    fn set_distribution_recipient(&mut self, distribution_recipient: TokenDistributionRecipient) {
        match self {
            TokenPerpetualDistribution::V0(v0) => {
                v0.distribution_recipient = distribution_recipient
            }
        }
    }
}

impl TokenPerpetualDistributionV0Methods for TokenPerpetualDistribution {
    fn next_interval(&self, block_info: &BlockInfo) -> RewardDistributionMoment {
        match self {
            TokenPerpetualDistribution::V0(v0) => v0.next_interval(block_info),
        }
    }

    fn current_interval(&self, block_info: &BlockInfo) -> RewardDistributionMoment {
        match self {
            TokenPerpetualDistribution::V0(v0) => v0.current_interval(block_info),
        }
    }
}
