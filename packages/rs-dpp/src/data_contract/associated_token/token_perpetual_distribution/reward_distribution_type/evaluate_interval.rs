use crate::balances::credits::TokenAmount;
use crate::block::block_info::BlockInfo;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use crate::ProtocolError;

impl RewardDistributionType {
    /// Computes the total rewards emitted in a given interval based on the current block information.
    ///
    /// This function determines the emission amounts within the range from `start_at_excluded` (exclusive)
    /// up to the latest relevant point in the blockchain. The evaluation depends on the specific type of
    /// distribution (Block-Based, Time-Based, or Epoch-Based) and the associated interval.
    ///
    /// # Parameters
    ///
    /// - `start_at_excluded` (`u64`):  
    ///   The last known point after which rewards should be counted (exclusive).
    /// - `block_info` (`&BlockInfo`):  
    ///   The current blockchain state information, providing the latest block height, timestamp, and epoch index.
    ///
    /// # Returns
    ///
    /// - `Ok(TokenAmount)`: The total sum of emitted rewards in the interval.
    /// - `Err(ProtocolError)`: If any evaluation fails (e.g., overflow, invalid configuration).
    ///
    pub fn rewards_in_interval(
        &self,
        start_at_excluded: u64,
        block_info: &BlockInfo,
    ) -> Result<TokenAmount, ProtocolError> {
        let end = match self {
            RewardDistributionType::BlockBasedDistribution { .. } => block_info.height,
            RewardDistributionType::TimeBasedDistribution { .. } => block_info.time_ms,
            RewardDistributionType::EpochBasedDistribution { .. } => block_info.epoch.index as u64,
        };
        self.function().evaluate_interval_in_bounds(
            start_at_excluded,
            self.interval(),
            end,
            self.start(),
            self.end(),
        )
    }
}
