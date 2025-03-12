use crate::balances::credits::TokenAmount;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use crate::ProtocolError;

impl DistributionFunction {
    /// Evaluates the total token emission over a specified interval.
    ///
    /// This function calculates the sum of token emissions at discrete points between
    /// `start_not_included` (exclusive) and `end_included` (inclusive), using `step` as the
    /// interval between evaluations. Each evaluation is performed by calling `self.evaluate(x)`
    /// at the appropriate step values.
    ///
    /// # Parameters
    ///
    /// - `start_not_included` (`u64`):  
    ///   The block height after which emissions are considered (exclusive).
    /// - `step` (`u64`):  
    ///   The interval in blocks at which emissions occur (e.g., every 5 blocks).  
    ///   **Must be greater than zero** to avoid division-by-zero errors.
    /// - `end_included` (`u64`):  
    ///   The final block height at which emissions should be considered (inclusive).
    ///
    /// # Behavior
    ///
    /// Given that emissions occur at regular block intervals, this function iterates through
    /// block heights that are spaced by `step`, beginning at `start_not_included + step` and
    /// stopping at or before `end_included`.  
    /// The token emissions at each step are retrieved using `self.evaluate(x)`, and their sum
    /// is returned.
    ///
    /// # Returns
    ///
    /// - `Ok(TokenAmount)`: The total sum of emissions over the interval.
    /// - `Err(ProtocolError)`: If an evaluation results in an error, such as an overflow or
    ///   invalid operation.
    ///
    /// # Errors
    ///
    /// - `ProtocolError::DivideByZero`: If `step` is zero.
    /// - `ProtocolError::Overflow`: If the accumulated sum exceeds the maximum allowable value.
    /// - Any error that `self.evaluate(x)` might return.
    ///
    pub fn evaluate_interval(
        &self,
        start_not_included: u64,
        step: u64,
        end_included: u64,
    ) -> Result<TokenAmount, ProtocolError> {
        if step == 0 {
            return Err(ProtocolError::DivideByZero(
                "evaluate_interval: step cannot be zero".into(),
            ));
        }
        if end_included <= start_not_included {
            return Ok(0);
        }

        let mut total: u64 = 0;
        // Begin at the first period after start_not_included by adding 'step'.
        let mut x = start_not_included + step;
        while x <= end_included {
            // Call evaluate(x) and accumulate the result.
            total = total.checked_add(self.evaluate(x)?).ok_or_else(|| {
                ProtocolError::Overflow("Total evaluation overflow in evaluate_interval".into())
            })?;
            x += step;
        }
        Ok(total)
    }

    /// Evaluates the total token emission over a specified interval, clamped within additional bounds.
    ///
    /// This function calculates the sum of token emissions by invoking `self.evaluate(x)` at discrete
    /// points that lie between `start_not_included` (exclusive) and `end_included` (inclusive), stepping by
    /// `step`. In addition, only evaluation points that also fall within the optional bounds `start_bounds_included`
    /// (inclusive) and `end_bounds_included` (inclusive) are considered.
    ///
    /// # Parameters
    ///
    /// - `start_not_included` (`RewardDistributionMoment`):  
    ///   The moment after which emissions are considered (exclusive).
    /// - `step` (`RewardDistributionMoment`):  
    ///   The interval step between evaluations. **Must be greater than zero**.
    /// - `end_included` (`RewardDistributionMoment`):  
    ///   The final moment at which emissions are considered (inclusive).
    /// - `start_bounds_included` (`Option<RewardDistributionMoment>`):  
    ///   An optional lower bound for evaluation. Only evaluation points ≥ this value will be included.
    /// - `end_bounds_included` (`Option<RewardDistributionMoment>`):  
    ///   An optional upper bound for evaluation. Only evaluation points ≤ this value will be included.
    ///
    /// # Type Consistency
    ///
    /// This function **requires all input values to be of the same variant** (`BlockBasedMoment`, `TimeBasedMoment`, or `EpochBasedMoment`).
    /// If a mismatch occurs, the function returns an error.
    ///
    /// # Returns
    /// - `Ok(TokenAmount)`: The total sum of token emissions over all valid evaluation points.
    /// - `Err(ProtocolError)`: If any evaluation fails (e.g., type mismatch, overflow, division-by-zero, or if
    ///   `step` is zero).
    ///
    /// # Behavior
    /// The function computes the effective start point as the larger of:
    ///   - `start_not_included + step` (i.e., the first natural evaluation point).
    ///   - `start_bounds_included` (if provided).
    ///
    /// Similarly, the effective end point is computed as the smaller of:
    ///   - `end_included`.
    ///   - `end_bounds_included` (if provided).
    ///
    /// It then iterates over these evaluation points, accumulating the token amounts.
    pub fn evaluate_interval_in_bounds(
        &self,
        start_not_included: RewardDistributionMoment,
        step: RewardDistributionMoment,
        end_included: RewardDistributionMoment,
        start_bounds_included: Option<RewardDistributionMoment>,
        end_bounds_included: Option<RewardDistributionMoment>,
    ) -> Result<TokenAmount, ProtocolError> {
        // Ensure that all moments are of the same type.
        if !(start_not_included.same_type(&step)
            && start_not_included.same_type(&end_included)
            && start_bounds_included
                .as_ref()
                .map_or(true, |b| start_not_included.same_type(b))
            && end_bounds_included
                .as_ref()
                .map_or(true, |b| start_not_included.same_type(b)))
        {
            return Err(ProtocolError::AddingDifferentTypes(
                "Mismatched RewardDistributionMoment types".to_string(),
            ));
        }

        if step == 0u64 {
            return Err(ProtocolError::InvalidDistributionStep(
                "evaluate_interval_in_bounds: step cannot be zero".into(),
            ));
        }
        if end_included <= start_not_included {
            return Ok(0);
        }

        // The first natural evaluation point is start_not_included + step.
        let first_point = (start_not_included + step)?;
        // Determine the effective starting point: the larger of first_point and start_bounds_included (if provided).
        let effective_start = if let Some(lb) = start_bounds_included {
            if lb > first_point {
                lb
            } else {
                first_point
            }
        } else {
            first_point
        };

        // Determine the effective ending point: the smallest of end_included and end_bounds_included (if provided).
        let effective_end = if let Some(ub) = end_bounds_included {
            if ub < end_included {
                ub
            } else {
                end_included
            }
        } else {
            end_included
        };

        if effective_end < effective_start {
            return Ok(0);
        }

        let mut total: u64 = 0;
        let mut x = effective_start;
        while x <= effective_end {
            total = total
                .checked_add(self.evaluate(x.to_u64())?)
                .ok_or_else(|| {
                    ProtocolError::Overflow(
                        "Total evaluation overflow in evaluate_interval_in_bounds".into(),
                    )
                })?;
            x = (x + step)?;
        }
        Ok(total)
    }
}
