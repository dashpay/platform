use crate::block::block_info::BlockInfo;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;

/// Accessor trait for `TokenPerpetualDistribution`, providing getter and setter methods
/// for its distribution type and recipient.
pub trait TokenPerpetualDistributionV0Accessors {
    /// Gets the distribution type used in the token distribution.
    fn distribution_type(&self) -> &RewardDistributionType;

    /// Sets the distribution type for the token distribution.
    fn set_distribution_type(&mut self, distribution_type: RewardDistributionType);

    /// Gets the distribution recipient for the token distribution.
    fn distribution_recipient(&self) -> TokenDistributionRecipient;

    /// Sets the distribution recipient for the token distribution.
    fn set_distribution_recipient(&mut self, distribution_recipient: TokenDistributionRecipient);
}

/// Methods trait for `TokenPerpetualDistribution`
pub trait TokenPerpetualDistributionV0Methods {
    /// we use u64 as a catch-all for any type of interval we might have, eg time, block or epoch
    fn next_interval(&self, block_info: &BlockInfo) -> RewardDistributionMoment;

    fn current_interval(&self, block_info: &BlockInfo) -> RewardDistributionMoment;
}
