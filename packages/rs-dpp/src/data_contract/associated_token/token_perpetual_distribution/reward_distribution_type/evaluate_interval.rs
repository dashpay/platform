use crate::balances::credits::TokenAmount;
use crate::block::block_info::BlockInfo;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use crate::ProtocolError;

impl RewardDistributionType {
    /// Computes the total rewards emitted in a given interval based on the provided distribution moments.
    ///
    /// This function determines the emission amounts within the range from `start_at_excluded` (exclusive)
    /// up to `end_at_moment_included` (inclusive). The evaluation depends on the specific type of
    /// distribution (Block-Based, Time-Based, or Epoch-Based) and the associated interval.
    ///
    /// # Parameters
    ///
    /// - `start_at_moment_excluded` (`RewardDistributionMoment`):  
    ///   The last known point after which rewards should be counted (exclusive).
    /// - `end_at_moment_included` (`RewardDistributionMoment`):  
    ///   The latest point up to which rewards should be counted (inclusive).
    ///
    /// # Returns
    ///
    /// - `Ok(TokenAmount)`: The total sum of emitted rewards in the interval.
    /// - `Err(ProtocolError)`: If any evaluation fails (e.g., overflow, invalid configuration).
    ///
    pub fn rewards_in_interval(
        &self,
        start_at_moment_excluded: RewardDistributionMoment,
        block_info: &BlockInfo,
    ) -> Result<TokenAmount, ProtocolError> {
        let end_reward_moment = RewardDistributionMoment::from_block_info(block_info, self);
        self.function().evaluate_interval_in_bounds(
            start_at_moment_excluded,
            self.interval(),
            end_reward_moment,
            self.start(),
            self.end(),
        )
    }
}
