use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::{
    DistributionFunction, MAX_DISTRIBUTION_CYCLES_PARAM,
};
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use crate::ProtocolError;

impl RewardDistributionType {
    /// Version 0 of [`RewardDistributionType::max_cycle_moment`]: the arithmetic every
    /// released Platform build ran up to protocol version 13.
    ///
    /// The cap is computed in the moment's own width. For epoch-based distributions that is
    /// `u16`: with a fixed-amount function `interval.saturating_mul(32_767)` already sits at
    /// `u16::MAX` for any interval of three or more, and adding a nonzero start overflows (an
    /// interval of two overflows from a start of two). Release builds carry no overflow
    /// checks, so on every network the sum wrapped, the cap landed below the start and
    /// `evaluate_interval` reported no rewards: such claims were refused every time.
    ///
    /// This generation is frozen. It spells the wrap out with `wrapping_add` so the result is
    /// the same in every build profile and replaying those blocks can never panic. Version 1
    /// fixes the arithmetic.
    pub(super) fn max_cycle_moment_v0(
        &self,
        start_moment: RewardDistributionMoment,
        current_cycle_moment: RewardDistributionMoment,
        max_non_fixed_amount_cycles: u32,
    ) -> Result<RewardDistributionMoment, ProtocolError> {
        let max_cycles = if matches!(self.function(), DistributionFunction::FixedAmount { .. }) {
            // This is much easier to calculate as it's always fixed, so we can have a near unlimited amount of cycles
            MAX_DISTRIBUTION_CYCLES_PARAM
        } else {
            max_non_fixed_amount_cycles as u64
        };
        let interval = self.interval();

        // Calculate maximum allowed moment based on distribution type
        match (start_moment, interval, current_cycle_moment) {
            (
                RewardDistributionMoment::BlockBasedMoment(start),
                RewardDistributionMoment::BlockBasedMoment(step),
                RewardDistributionMoment::BlockBasedMoment(current),
            ) => Ok(RewardDistributionMoment::BlockBasedMoment(
                start
                    .wrapping_add(step.saturating_mul(max_cycles))
                    .min(current),
            )),
            (
                RewardDistributionMoment::TimeBasedMoment(start),
                RewardDistributionMoment::TimeBasedMoment(step),
                RewardDistributionMoment::TimeBasedMoment(current),
            ) => Ok(RewardDistributionMoment::TimeBasedMoment(
                start
                    .wrapping_add(step.saturating_mul(max_cycles))
                    .min(current),
            )),
            (
                RewardDistributionMoment::EpochBasedMoment(start),
                RewardDistributionMoment::EpochBasedMoment(step),
                RewardDistributionMoment::EpochBasedMoment(current),
            ) => Ok(RewardDistributionMoment::EpochBasedMoment(
                // For an epoch reward, if you are in epoch 3 you can't get rewarded for epoch 3, but only epoch 2
                start
                    .wrapping_add(step.saturating_mul(max_cycles as u16))
                    .min(current.saturating_sub(1)),
            )),
            _ => Err(ProtocolError::CorruptedCodeExecution(
                "Mismatch moment types".to_string(),
            )),
        }
    }
}
