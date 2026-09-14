use crate::block::epoch::EpochIndex;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::{
    DistributionFunction, MAX_DISTRIBUTION_CYCLES_PARAM,
};
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use crate::ProtocolError;

impl RewardDistributionType {
    /// Version 1 of [`RewardDistributionType::max_cycle_moment`], selected from protocol
    /// version 14.
    ///
    /// `start + interval * cycles` is taken in `u64` with saturating arithmetic and then capped:
    /// at the current cycle moment for block- and time-based distributions, and at the last
    /// completed cycle moment, `current cycle moment - interval`, for epoch-based ones. Only the
    /// capped value is narrowed back to `EpochIndex`, and it always fits because the cap is
    /// itself an `EpochIndex`. The widest epoch case, `u16::MAX * u32::MAX + u16::MAX`, stays
    /// below `2^49`, so the `u64` sums never saturate in practice; the saturating forms only
    /// guarantee that nothing here can panic or wrap.
    ///
    /// The epoch cap is the running cycle's predecessor rather than the current cycle moment
    /// less one. For an interval of one the two are the same epoch, so nothing changes for the
    /// distributions that could be claimed before this version. For a wider interval they pay
    /// the same cycles (a cycle is claimable once the cycle after it has begun), but a cap on
    /// a cycle boundary keeps every start and end `evaluate_interval` sees on a boundary, the
    /// only shape in which its fixed-amount step count and its per-cycle loop agree, exactly
    /// as the block- and time-based arms already guarantee.
    pub(super) fn max_cycle_moment_v1(
        &self,
        start_moment: RewardDistributionMoment,
        current_cycle_moment: RewardDistributionMoment,
        max_non_fixed_amount_cycles: u32,
    ) -> Result<RewardDistributionMoment, ProtocolError> {
        let max_cycles = if matches!(self.function(), DistributionFunction::FixedAmount { .. }) {
            // A fixed amount costs the same to evaluate over any number of cycles, so a claim may
            // redeem near-unlimited cycles at once.
            MAX_DISTRIBUTION_CYCLES_PARAM
        } else {
            u64::from(max_non_fixed_amount_cycles)
        };

        match (start_moment, self.interval(), current_cycle_moment) {
            (
                RewardDistributionMoment::BlockBasedMoment(start),
                RewardDistributionMoment::BlockBasedMoment(step),
                RewardDistributionMoment::BlockBasedMoment(current),
            ) => Ok(RewardDistributionMoment::BlockBasedMoment(
                start
                    .saturating_add(step.saturating_mul(max_cycles))
                    .min(current),
            )),
            (
                RewardDistributionMoment::TimeBasedMoment(start),
                RewardDistributionMoment::TimeBasedMoment(step),
                RewardDistributionMoment::TimeBasedMoment(current),
            ) => Ok(RewardDistributionMoment::TimeBasedMoment(
                start
                    .saturating_add(step.saturating_mul(max_cycles))
                    .min(current),
            )),
            (
                RewardDistributionMoment::EpochBasedMoment(start),
                RewardDistributionMoment::EpochBasedMoment(step),
                RewardDistributionMoment::EpochBasedMoment(current),
            ) => {
                // For an epoch reward, if you are in epoch 3 you can't get rewarded for epoch 3,
                // but only epoch 2: the cycle that began at the current cycle moment is still
                // running, so the cap is the cycle before it.
                let last_completed_cycle_moment = current.saturating_sub(step);
                let reach = u64::from(step)
                    .saturating_mul(max_cycles)
                    .saturating_add(u64::from(start));
                let capped = reach.min(u64::from(last_completed_cycle_moment));
                let capped = EpochIndex::try_from(capped).map_err(|_| {
                    ProtocolError::Overflow("max cycle moment does not fit an epoch index")
                })?;
                Ok(RewardDistributionMoment::EpochBasedMoment(capped))
            }
            _ => Err(ProtocolError::CorruptedCodeExecution(
                "Mismatch moment types".to_string(),
            )),
        }
    }
}
