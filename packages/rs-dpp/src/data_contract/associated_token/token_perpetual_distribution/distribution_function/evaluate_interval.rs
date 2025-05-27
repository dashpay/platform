use std::ops::{Div, RangeInclusive};
use crate::balances::credits::TokenAmount;
use crate::block::epoch::EpochIndex;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::reward_ratio::RewardRatio;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use crate::ProtocolError;

impl DistributionFunction {
    /// Evaluates the total amount of tokens emitted over a specified interval.
    ///
    /// This function calculates the cumulative emission of tokens by evaluating the distribution
    /// function at discrete intervals between two specified moments. The interval calculation begins
    /// after the `start_not_included` moment and includes the `end_included` moment, stepping forward by
    /// the specified `step` interval.
    ///
    /// # Parameters
    ///
    /// - `start_not_included`: The starting moment (exclusive).
    /// - `end_included`: The end moment (inclusive).
    /// - `step`: The interval between each emission evaluation; must be greater than zero.
    /// - `get_epoch_reward_ratio`: Optional function providing a reward ratio for epoch-based distributions.
    ///
    /// # Returns
    ///
    /// - `Ok(TokenAmount)` total emitted tokens.
    /// - `Err(ProtocolError)` on mismatched types, zero steps, or overflow.
    pub fn evaluate_interval<F>(
        &self,
        distribution_start: RewardDistributionMoment,
        interval_start_excluded: RewardDistributionMoment,
        interval_end_included: RewardDistributionMoment,
        step: RewardDistributionMoment,
        get_epoch_reward_ratio: Option<F>,
    ) -> Result<TokenAmount, ProtocolError>
    where
        F: Fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>,
    {
        // Ensure moments are the same type.
        if !(interval_start_excluded.same_type(&step)
            && interval_start_excluded.same_type(&interval_end_included))
        {
            return Err(ProtocolError::AddingDifferentTypes(
                "Mismatched RewardDistributionMoment types".into(),
            ));
        }

        if step == 0u64 {
            return Err(ProtocolError::InvalidDistributionStep(
                "evaluate_interval: step cannot be zero",
            ));
        }

        if interval_start_excluded >= interval_end_included {
            return Ok(0);
        }

        // Optimization for FixedAmount
        if let DistributionFunction::FixedAmount {
            amount: fixed_amount,
        } = self
        {
            let steps_count =
                interval_start_excluded.steps_till(&interval_end_included, &step, false, true)?;
            let total_amount = fixed_amount
                .checked_mul(steps_count)
                .ok_or_else(|| ProtocolError::Overflow("Overflow in FixedAmount evaluation"))?;

            return if let (
                RewardDistributionMoment::EpochBasedMoment(first_epoch),
                RewardDistributionMoment::EpochBasedMoment(last_epoch),
                Some(ref get_ratio_fn),
            ) = (
                interval_start_excluded,
                interval_end_included,
                get_epoch_reward_ratio.as_ref(),
            ) {
                if let Some(ratio) = get_ratio_fn(first_epoch.saturating_add(1)..=last_epoch) {
                    total_amount
                        .checked_mul(ratio.numerator)
                        .and_then(|v| v.checked_div(ratio.denominator))
                        .ok_or_else(|| {
                            ProtocolError::Overflow(
                                "Overflow applying reward ratio in evaluate_interval",
                            )
                        })
                } else {
                    Err(ProtocolError::MissingEpochInfo(format!(
                        "missing epoch info for an epoch between {} excluded and {} included",
                        first_epoch, last_epoch
                    )))
                }
            } else {
                Ok(total_amount)
            };
        }

        // Let's say you have a step 10 going from 10 to 20, the first index would be 2
        // If we are at 10
        let first_step = ((interval_start_excluded / step)? + 1)?;
        let last_step = (interval_end_included / step)?;

        if first_step > last_step {
            return Ok(0);
        }

        let distribution_start_step = distribution_start.div(step)?;

        let mut total: u64 = 0;
        let mut current_point = first_step;

        while current_point <= last_step {
            let base_amount =
                self.evaluate(distribution_start_step.to_u64(), current_point.to_u64())?;

            let amount = if let (
                RewardDistributionMoment::EpochBasedMoment(epoch_index),
                Some(ref get_ratio_fn),
            ) = (current_point, get_epoch_reward_ratio.as_ref())
            {
                if let Some(ratio) = get_ratio_fn(epoch_index..=epoch_index) {
                    base_amount
                        .checked_mul(ratio.numerator)
                        .and_then(|v| v.checked_div(ratio.denominator))
                        .ok_or_else(|| {
                            ProtocolError::Overflow(
                                "Overflow applying reward ratio in evaluate_interval",
                            )
                        })?
                } else {
                    return Err(ProtocolError::MissingEpochInfo(format!(
                        "missing epoch info for epoch {}",
                        epoch_index
                    )));
                }
            } else {
                base_amount
            };

            total = total
                .checked_add(amount)
                .ok_or_else(|| ProtocolError::Overflow("Overflow in token interval evaluation"))?;

            current_point = (current_point + 1)?;
        }

        Ok(total)
    }
}
