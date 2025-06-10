use std::ops::RangeInclusive;
use crate::balances::credits::TokenAmount;
use crate::block::epoch::EpochIndex;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::reward_ratio::RewardRatio;
#[cfg(feature = "token-reward-explanations")]
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::evaluate_interval::IntervalEvaluationExplanation;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use crate::ProtocolError;

impl RewardDistributionType {
    /// Computes the total rewards emitted in a given interval based on the provided distribution moments.
    ///
    /// This function determines the emission amounts within the range from `start_at_excluded` (exclusive)
    /// up to `current_moment_included` (inclusive). The evaluation depends on the specific type of
    /// distribution (Block-Based, Time-Based, or Epoch-Based) and the associated interval.
    /// If the distribution type has a start moment after the provided start moment, it uses the later start moment.
    /// If the distribution type has an end moment before the provided current moment, it uses the earlier end moment.
    ///
    /// # Parameters
    ///
    /// - `start_at_moment_excluded` (`RewardDistributionMoment`):  
    ///   The last known point after which rewards should be counted (exclusive).
    /// - `current_moment_included` (`RewardDistributionMoment`):  
    ///   The latest point up to which rewards should be counted (inclusive).
    /// - `get_epoch_reward_ratio`: Optional function providing a reward ratio for epoch-based distributions.
    ///
    /// # Returns
    ///
    /// - `Ok(TokenAmount)`: The total sum of emitted rewards in the interval.
    /// - `Err(ProtocolError)`: If any evaluation fails (e.g., overflow, invalid configuration).
    ///
    pub fn rewards_in_interval<F>(
        &self,
        distribution_start: RewardDistributionMoment,
        start_at_moment: RewardDistributionMoment,
        current_moment_included: RewardDistributionMoment,
        get_epoch_reward_ratio: Option<F>,
    ) -> Result<TokenAmount, ProtocolError>
    where
        F: Fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>,
    {
        self.function().evaluate_interval(
            distribution_start,
            start_at_moment,
            current_moment_included,
            self.interval(),
            get_epoch_reward_ratio,
        )
    }

    /// Computes the total rewards emitted in a given interval with detailed explanation.
    ///
    /// This function provides the same calculation as `rewards_in_interval` but returns a comprehensive
    /// explanation structure containing all the steps, calculations, and reasoning behind the result.
    /// This is useful for debugging, auditing, or providing transparency to users about how their
    /// token emissions are calculated.
    ///
    /// # Parameters
    ///
    /// - `distribution_start` (`RewardDistributionMoment`):  
    ///   The starting moment of the distribution.
    /// - `start_at_moment` (`RewardDistributionMoment`):  
    ///   The last known point after which rewards should be counted (exclusive).
    /// - `current_moment_included` (`RewardDistributionMoment`):  
    ///   The latest point up to which rewards should be counted (inclusive).
    /// - `get_epoch_reward_ratio`: Optional function providing a reward ratio for epoch-based distributions.
    /// - `is_first_claim`: Explanation will be based on whether this is the first claim or not.
    ///
    /// # Returns
    ///
    /// - `Ok(IntervalEvaluationExplanation)`: A detailed explanation containing the result and all calculation steps.
    /// - `Err(ProtocolError)`: If any evaluation fails (e.g., overflow, invalid configuration).
    ///
    #[cfg(feature = "token-reward-explanations")]
    pub fn rewards_in_interval_with_explanation<F>(
        &self,
        distribution_start: RewardDistributionMoment,
        start_at_moment: RewardDistributionMoment,
        current_moment_included: RewardDistributionMoment,
        get_epoch_reward_ratio: Option<F>,
        is_first_claim: bool,
    ) -> Result<IntervalEvaluationExplanation, ProtocolError>
    where
        F: Fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>,
    {
        self.function().evaluate_interval_with_explanation(
            distribution_start,
            start_at_moment,
            current_moment_included,
            self.interval(),
            get_epoch_reward_ratio,
            is_first_claim,
        )
    }
}
