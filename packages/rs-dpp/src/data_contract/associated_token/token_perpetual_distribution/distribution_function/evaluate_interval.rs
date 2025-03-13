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
        start_not_included: RewardDistributionMoment,
        end_included: RewardDistributionMoment,
        step: RewardDistributionMoment,
        get_epoch_reward_ratio: Option<F>,
    ) -> Result<TokenAmount, ProtocolError>
    where
        F: Fn(EpochIndex) -> Option<RewardRatio>,
    {
        // Ensure moments are the same type.
        if !(start_not_included.same_type(&step) && start_not_included.same_type(&end_included)) {
            return Err(ProtocolError::AddingDifferentTypes(
                "Mismatched RewardDistributionMoment types".into(),
            ));
        }

        if step == 0u64 {
            return Err(ProtocolError::InvalidDistributionStep(
                "evaluate_interval: step cannot be zero".into(),
            ));
        }

        if end_included <= start_not_included {
            return Ok(0);
        }

        let first_point = (start_not_included + step)?;

        if end_included < first_point {
            return Ok(0);
        }

        // Optimization for FixedAmount
        if let DistributionFunction::FixedAmount { amount: fixed_amount } = self {
            let steps_count = first_point.steps_till(&end_included, &step)?;
            return fixed_amount.checked_mul(steps_count).ok_or_else(|| {
                ProtocolError::Overflow("Overflow in FixedAmount evaluation".into())
            });
        }

        let mut total: u64 = 0;
        let mut current_point = first_point;

        while current_point <= end_included {
            let base_amount = self.evaluate(current_point.to_u64())?;

            let amount = if let (
                RewardDistributionMoment::EpochBasedMoment(epoch_index),
                Some(ref get_ratio_fn),
            ) = (current_point, get_epoch_reward_ratio.as_ref())
            {
                if let Some(ratio) = get_ratio_fn(epoch_index) {
                    base_amount
                        .checked_mul(ratio.numerator)
                        .and_then(|v| v.checked_div(ratio.denominator))
                        .ok_or_else(|| {
                            ProtocolError::Overflow(
                                "Overflow applying reward ratio in evaluate_interval".into(),
                            )
                        })?
                } else {
                    return Err(ProtocolError::MissingEpochInfo(format!("missing epoch info for epoch {}", epoch_index)));
                }
            } else {
                base_amount
            };

            total = total.checked_add(amount).ok_or_else(|| {
                ProtocolError::Overflow(
                    "Overflow in token interval evaluation"
                )
            })?;

            current_point = (current_point + step)?;
        }

        Ok(total)
    }
}
