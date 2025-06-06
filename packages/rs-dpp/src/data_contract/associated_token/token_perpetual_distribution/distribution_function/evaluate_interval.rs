use std::ops::{Div, RangeInclusive};
use crate::balances::credits::TokenAmount;
use crate::block::epoch::EpochIndex;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::reward_ratio::RewardRatio;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use crate::ProtocolError;

/// Details of a single evaluation step within an interval
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationStep {
    /// The step index in the evaluation process
    pub step_index: u64,
    /// The current point being evaluated
    pub current_point: RewardDistributionMoment,
    /// The base amount before any reward ratio adjustments
    pub base_amount: TokenAmount,
    /// The reward ratio applied (if any)
    pub reward_ratio: Option<RewardRatio>,
    /// The final amount after all adjustments
    pub final_amount: TokenAmount,
    /// Running total up to this point
    pub running_total: TokenAmount,
}

/// Detailed explanation of an interval evaluation containing all steps and reasoning
#[derive(Debug, Clone, PartialEq)]
pub struct IntervalEvaluationExplanation {
    /// The distribution function that was evaluated
    pub distribution_function: DistributionFunction,
    /// Starting moment of the distribution (exclusive)
    pub interval_start_excluded: RewardDistributionMoment,
    /// Ending moment of the distribution (inclusive)
    pub interval_end_included: RewardDistributionMoment,
    /// Step size used for evaluation
    pub step: RewardDistributionMoment,
    /// Distribution start moment
    pub distribution_start: RewardDistributionMoment,
    /// Final calculated total amount
    pub total_amount: TokenAmount,
    /// Individual evaluation steps with details
    pub evaluation_steps: Vec<EvaluationStep>,
    /// Whether reward ratios were applied
    pub reward_ratios_applied: bool,
    /// Whether the FixedAmount optimization was used
    pub fixed_amount_optimization_used: bool,
    /// Number of steps calculated
    pub steps_count: u64,
    /// Any special conditions or optimizations applied
    pub optimization_notes: Vec<String>,
}

impl IntervalEvaluationExplanation {
    /// Returns a short explanation of the evaluation result
    pub fn short_explanation(&self) -> String {
        format!(
            "Evaluated {} from {} to {} with step {} and got {} tokens total",
            self.distribution_function,
            self.interval_start_excluded,
            self.interval_end_included,
            self.step,
            self.total_amount
        )
    }

    /// Returns a medium-length explanation with key details
    pub fn medium_explanation(&self) -> String {
        let mut explanation = format!(
            "Distribution Function: {}\n\
             Evaluation Range: {} (excluded) to {} (included)\n\
             Step Size: {}\n\
             Distribution Start: {}\n\
             Total Steps Evaluated: {}\n\
             Final Total Amount: {} tokens\n",
            self.distribution_function,
            self.interval_start_excluded,
            self.interval_end_included,
            self.step,
            self.distribution_start,
            self.steps_count,
            self.total_amount
        );

        if self.fixed_amount_optimization_used {
            explanation.push_str("✓ FixedAmount optimization was used for faster calculation\n");
        }

        if self.reward_ratios_applied {
            explanation.push_str("✓ Reward ratios were applied to adjust emissions\n");
        }

        if !self.optimization_notes.is_empty() {
            explanation.push_str("Special Conditions:\n");
            for note in &self.optimization_notes {
                explanation.push_str(&format!("  • {}\n", note));
            }
        }

        explanation
    }

    /// Returns a detailed explanation with all steps and calculations
    pub fn long_explanation(&self) -> String {
        let mut explanation = format!(
            "=== Detailed Interval Evaluation Explanation ===\n\n\
             Distribution Function: {}\n\
             Evaluation Parameters:\n\
             - Start (excluded): {}\n\
             - End (included): {}\n\
             - Step size: {}\n\
             - Distribution start: {}\n\n",
            self.distribution_function,
            self.interval_start_excluded,
            self.interval_end_included,
            self.step,
            self.distribution_start
        );

        if self.fixed_amount_optimization_used {
            explanation.push_str(
                "OPTIMIZATION: FixedAmount function detected - using fast calculation method\n\
                 Instead of evaluating each step individually, we calculated:\n\
                 Total = Fixed Amount × Number of Steps\n\n",
            );
        }

        if !self.optimization_notes.is_empty() {
            explanation.push_str("Special Conditions Applied:\n");
            for note in &self.optimization_notes {
                explanation.push_str(&format!("  • {}\n", note));
            }
            explanation.push('\n');
        }

        explanation.push_str(&format!(
            "Evaluation Process:\n\
             Total steps calculated: {}\n",
            self.steps_count
        ));

        if self.reward_ratios_applied {
            explanation.push_str("Reward ratios were applied to adjust base emissions\n");
        }

        if !self.evaluation_steps.is_empty() {
            explanation.push_str("\nStep-by-Step Breakdown:\n");
            for step in &self.evaluation_steps {
                explanation.push_str(&format!(
                    "  Step #{}: Point {} → Base: {} tokens",
                    step.step_index, step.current_point, step.base_amount
                ));

                if let Some(ratio) = &step.reward_ratio {
                    explanation.push_str(&format!(
                        " → Ratio applied ({}/{}) → Final: {} tokens",
                        ratio.numerator, ratio.denominator, step.final_amount
                    ));
                } else {
                    explanation.push_str(&format!(" → Final: {} tokens", step.final_amount));
                }

                explanation.push_str(&format!(" (Running total: {})\n", step.running_total));
            }
        } else if self.fixed_amount_optimization_used {
            explanation.push_str("\nNo individual steps shown due to FixedAmount optimization\n");
        }

        explanation.push_str(&format!(
            "\n=== RESULT ===\n\
             Total tokens emitted over interval: {} tokens\n",
            self.total_amount
        ));

        explanation
    }

    /// Returns a detailed explanation for a specific step in the evaluation process
    ///
    /// # Parameters
    /// - `step_index`: The 1-based index of the step to explain
    ///
    /// # Returns
    /// - `Some(String)` with the step explanation if the step exists
    /// - `None` if the step index is out of bounds or if individual steps weren't tracked
    pub fn explanation_for_step(&self, step_index: u64) -> Option<String> {
        if self.fixed_amount_optimization_used {
            return Some(format!(
                "Step #{}: This evaluation used FixedAmount optimization.\n\
                 Individual steps were not calculated because the result is simply:\n\
                 {} tokens × {} steps = {} total tokens\n\
                 Each step would emit exactly {} tokens.",
                step_index,
                match &self.distribution_function {
                    DistributionFunction::FixedAmount { amount } => amount,
                    _ => &0, // This shouldn't happen
                },
                self.steps_count,
                self.total_amount,
                match &self.distribution_function {
                    DistributionFunction::FixedAmount { amount } => amount,
                    _ => &0,
                }
            ));
        }

        // Find the step with matching index
        self.evaluation_steps
            .iter()
            .find(|step| step.step_index == step_index)
            .map(|step| {
                let mut explanation = format!(
                    "Step #{}: Evaluation at point {}\n\n",
                    step.step_index, step.current_point
                );

                // Explain the distribution function evaluation
                explanation.push_str(&format!(
                    "Distribution Function: {}\n",
                    self.distribution_function
                ));

                // Show the calculation
                explanation.push_str(&format!(
                    "Base calculation at point {}: {} tokens\n",
                    step.current_point, step.base_amount
                ));

                // Explain any reward ratio adjustments
                if let Some(ratio) = &step.reward_ratio {
                    explanation.push_str(&format!(
                        "\nReward Ratio Applied:\n\
                         - Ratio: {}/{}\n\
                         - Calculation: {} × {} ÷ {} = {} tokens\n",
                        ratio.numerator,
                        ratio.denominator,
                        step.base_amount,
                        ratio.numerator,
                        ratio.denominator,
                        step.final_amount
                    ));
                } else {
                    explanation
                        .push_str(&format!("\nFinal amount: {} tokens\n", step.final_amount));
                }

                // Show the running total
                explanation.push_str(&format!(
                    "\nRunning Total after this step: {} tokens\n",
                    step.running_total
                ));

                // Add context about this step's contribution
                let percentage = if self.total_amount > 0 {
                    (step.final_amount as f64 / self.total_amount as f64) * 100.0
                } else {
                    0.0
                };
                explanation.push_str(&format!(
                    "This step contributes {:.2}% of the total emission.\n",
                    percentage
                ));

                explanation
            })
    }
}

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

    /// Evaluates the total amount of tokens emitted over a specified interval with detailed explanation.
    ///
    /// This function provides the same calculation as `evaluate_interval` but returns a comprehensive
    /// explanation structure containing all the steps, calculations, and reasoning behind the result.
    /// This is useful for debugging, auditing, or providing transparency to users about how their
    /// token emissions are calculated.
    ///
    /// # Parameters
    ///
    /// - `distribution_start`: The starting moment of the distribution.
    /// - `interval_start_excluded`: The starting moment (exclusive).
    /// - `interval_end_included`: The end moment (inclusive).
    /// - `step`: The interval between each emission evaluation; must be greater than zero.
    /// - `get_epoch_reward_ratio`: Optional function providing a reward ratio for epoch-based distributions.
    ///
    /// # Returns
    ///
    /// - `Ok(IntervalEvaluationExplanation)` containing the result and detailed explanation.
    /// - `Err(ProtocolError)` on mismatched types, zero steps, or overflow.
    pub fn evaluate_interval_with_explanation<F>(
        &self,
        distribution_start: RewardDistributionMoment,
        interval_start_excluded: RewardDistributionMoment,
        interval_end_included: RewardDistributionMoment,
        step: RewardDistributionMoment,
        get_epoch_reward_ratio: Option<F>,
    ) -> Result<IntervalEvaluationExplanation, ProtocolError>
    where
        F: Fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>,
    {
        let mut explanation = IntervalEvaluationExplanation {
            distribution_function: self.clone(),
            interval_start_excluded,
            interval_end_included,
            step,
            distribution_start,
            total_amount: 0,
            evaluation_steps: Vec::new(),
            reward_ratios_applied: false,
            fixed_amount_optimization_used: false,
            steps_count: 0,
            optimization_notes: Vec::new(),
        };

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
                "evaluate_interval_with_explanation: step cannot be zero",
            ));
        }

        if interval_start_excluded >= interval_end_included {
            explanation
                .optimization_notes
                .push("Start >= End: returning 0 tokens".to_string());
            return Ok(explanation);
        }

        // Optimization for FixedAmount
        if let DistributionFunction::FixedAmount {
            amount: fixed_amount,
        } = self
        {
            explanation.fixed_amount_optimization_used = true;
            explanation.optimization_notes.push(
                "FixedAmount distribution: using optimized calculation (amount × steps)"
                    .to_string(),
            );

            let steps_count =
                interval_start_excluded.steps_till(&interval_end_included, &step, false, true)?;
            explanation.steps_count = steps_count;

            let total_amount = fixed_amount
                .checked_mul(steps_count)
                .ok_or_else(|| ProtocolError::Overflow("Overflow in FixedAmount evaluation"))?;

            explanation.total_amount = if let (
                RewardDistributionMoment::EpochBasedMoment(first_epoch),
                RewardDistributionMoment::EpochBasedMoment(last_epoch),
                Some(ref get_ratio_fn),
            ) = (
                interval_start_excluded,
                interval_end_included,
                get_epoch_reward_ratio.as_ref(),
            ) {
                explanation.reward_ratios_applied = true;
                explanation
                    .optimization_notes
                    .push("Reward ratio applied to FixedAmount calculation".to_string());

                if let Some(ratio) = get_ratio_fn(first_epoch.saturating_add(1)..=last_epoch) {
                    explanation.optimization_notes.push(format!(
                        "Applied ratio: {}/{}",
                        ratio.numerator, ratio.denominator
                    ));
                    total_amount
                        .checked_mul(ratio.numerator)
                        .and_then(|v| v.checked_div(ratio.denominator))
                        .ok_or_else(|| {
                            ProtocolError::Overflow(
                                "Overflow applying reward ratio in evaluate_interval_with_explanation",
                            )
                        })?
                } else {
                    return Err(ProtocolError::MissingEpochInfo(format!(
                        "missing epoch info for an epoch between {} excluded and {} included",
                        first_epoch, last_epoch
                    )));
                }
            } else {
                total_amount
            };

            return Ok(explanation);
        }

        // Standard evaluation with step tracking
        let first_step = ((interval_start_excluded / step)? + 1)?;
        let last_step = (interval_end_included / step)?;

        if first_step > last_step {
            explanation
                .optimization_notes
                .push("First step > last step: returning 0 tokens".to_string());
            return Ok(explanation);
        }

        explanation.steps_count = last_step
            .to_u64()
            .saturating_sub(first_step.to_u64())
            .saturating_add(1);

        let distribution_start_step = distribution_start.div(step)?;

        let mut total: u64 = 0;
        let mut current_point = first_step;
        let mut step_index = 1u64;

        while current_point <= last_step {
            let base_amount =
                self.evaluate(distribution_start_step.to_u64(), current_point.to_u64())?;

            let (amount, reward_ratio) = if let (
                RewardDistributionMoment::EpochBasedMoment(epoch_index),
                Some(ref get_ratio_fn),
            ) = (current_point, get_epoch_reward_ratio.as_ref())
            {
                explanation.reward_ratios_applied = true;
                if let Some(ratio) = get_ratio_fn(epoch_index..=epoch_index) {
                    let adjusted_amount = base_amount
                        .checked_mul(ratio.numerator)
                        .and_then(|v| v.checked_div(ratio.denominator))
                        .ok_or_else(|| {
                            ProtocolError::Overflow(
                                "Overflow applying reward ratio in evaluate_interval_with_explanation",
                            )
                        })?;
                    (adjusted_amount, Some(ratio))
                } else {
                    return Err(ProtocolError::MissingEpochInfo(format!(
                        "missing epoch info for epoch {}",
                        epoch_index
                    )));
                }
            } else {
                (base_amount, None)
            };

            total = total
                .checked_add(amount)
                .ok_or_else(|| ProtocolError::Overflow("Overflow in token interval evaluation"))?;

            explanation.evaluation_steps.push(EvaluationStep {
                step_index,
                current_point,
                base_amount,
                reward_ratio,
                final_amount: amount,
                running_total: total,
            });

            current_point = (current_point + 1)?;
            step_index += 1;
        }

        explanation.total_amount = total;
        Ok(explanation)
    }
}

#[cfg(test)]
mod explanation_tests {
    use super::*;

    #[test]
    fn test_fixed_amount_explanation() {
        let distribution_function = DistributionFunction::FixedAmount { amount: 100 };
        let start_excluded = RewardDistributionMoment::BlockBasedMoment(0);
        let end_included = RewardDistributionMoment::BlockBasedMoment(50);
        let step = RewardDistributionMoment::BlockBasedMoment(10);
        let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

        let result = distribution_function
            .evaluate_interval_with_explanation(
                distribution_start,
                start_excluded,
                end_included,
                step,
                None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
            )
            .unwrap();

        // Should have 5 steps: 10, 20, 30, 40, 50 (6 total steps with boundaries)
        assert_eq!(result.steps_count, 6);
        assert_eq!(result.total_amount, 600); // 100 tokens * 6 steps
        assert!(result.fixed_amount_optimization_used);
        assert!(!result.reward_ratios_applied);
        assert!(result.evaluation_steps.is_empty()); // No individual steps due to optimization

        // Test explanations
        let short = result.short_explanation();
        assert!(short.contains("FixedAmount"));
        assert!(short.contains("600 tokens total"));

        let medium = result.medium_explanation();
        assert!(medium.contains("FixedAmount optimization was used"));
        assert!(medium.contains("Total Steps Evaluated: 6"));

        let long = result.long_explanation();
        assert!(long.contains("OPTIMIZATION: FixedAmount function detected"));
        assert!(long.contains("Total = Fixed Amount × Number of Steps"));
    }

    #[test]
    fn test_linear_function_explanation() {
        let distribution_function = DistributionFunction::Linear {
            a: 10,
            d: 1,
            start_step: Some(0),
            starting_amount: 50,
            min_value: None,
            max_value: None,
        };
        let start_excluded = RewardDistributionMoment::BlockBasedMoment(0);
        let end_included = RewardDistributionMoment::BlockBasedMoment(20);
        let step = RewardDistributionMoment::BlockBasedMoment(10);
        let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

        let result = distribution_function
            .evaluate_interval_with_explanation(
                distribution_start,
                start_excluded,
                end_included,
                step,
                None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
            )
            .unwrap();

        assert!(!result.fixed_amount_optimization_used);
        assert!(!result.reward_ratios_applied);
        assert!(!result.evaluation_steps.is_empty()); // Should have individual steps

        // Test that explanations contain function type
        let short = result.short_explanation();
        assert!(short.contains("Linear"));

        let medium = result.medium_explanation();
        assert!(medium.contains("Linear"));

        let long = result.long_explanation();
        assert!(long.contains("Step-by-Step Breakdown"));
    }

    #[test]
    fn test_explanation_with_zero_range() {
        let distribution_function = DistributionFunction::FixedAmount { amount: 100 };
        let start_excluded = RewardDistributionMoment::BlockBasedMoment(10);
        let end_included = RewardDistributionMoment::BlockBasedMoment(5); // End before start
        let step = RewardDistributionMoment::BlockBasedMoment(1);
        let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

        let result = distribution_function
            .evaluate_interval_with_explanation(
                distribution_start,
                start_excluded,
                end_included,
                step,
                None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
            )
            .unwrap();

        assert_eq!(result.total_amount, 0);
        assert_eq!(result.steps_count, 0);
        assert!(result
            .optimization_notes
            .iter()
            .any(|note| note.contains("Start >= End")));
    }

    #[test]
    fn test_explanation_methods_output() {
        let distribution_function = DistributionFunction::FixedAmount { amount: 50 };
        let start_excluded = RewardDistributionMoment::EpochBasedMoment(1);
        let end_included = RewardDistributionMoment::EpochBasedMoment(3);
        let step = RewardDistributionMoment::EpochBasedMoment(1);
        let distribution_start = RewardDistributionMoment::EpochBasedMoment(0);

        let result = distribution_function
            .evaluate_interval_with_explanation(
                distribution_start,
                start_excluded,
                end_included,
                step,
                None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
            )
            .unwrap();

        let short = result.short_explanation();
        let medium = result.medium_explanation();
        let long = result.long_explanation();

        // Short should be single line
        assert!(!short.contains('\n'));

        // Medium should be multiple lines but not too long
        assert!(medium.contains('\n'));
        assert!(medium.len() > short.len());
        assert!(medium.len() < long.len());

        // Long should be the most detailed
        assert!(long.contains("==="));
        assert!(long.len() > medium.len());
    }

    #[test]
    fn test_explanation_for_step() {
        // Test with a Linear function to get individual steps
        let distribution_function = DistributionFunction::Linear {
            a: 5,
            d: 1,
            start_step: Some(0),
            starting_amount: 100,
            min_value: None,
            max_value: None,
        };
        let start_excluded = RewardDistributionMoment::BlockBasedMoment(0);
        let end_included = RewardDistributionMoment::BlockBasedMoment(30);
        let step = RewardDistributionMoment::BlockBasedMoment(10);
        let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

        let result = distribution_function
            .evaluate_interval_with_explanation(
                distribution_start,
                start_excluded,
                end_included,
                step,
                None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
            )
            .unwrap();

        // Test getting explanation for first step
        let step1_explanation = result.explanation_for_step(1).unwrap();
        assert!(step1_explanation.contains("Step #1"));
        assert!(step1_explanation.contains("Distribution Function: Linear"));
        assert!(step1_explanation.contains("Base calculation"));
        assert!(step1_explanation.contains("Running Total"));
        assert!(step1_explanation.contains("contributes"));

        // Test getting explanation for non-existent step
        let no_step = result.explanation_for_step(999);
        assert!(no_step.is_none());

        // Test with FixedAmount function
        let fixed_function = DistributionFunction::FixedAmount { amount: 50 };
        let fixed_result = fixed_function
            .evaluate_interval_with_explanation(
                distribution_start,
                start_excluded,
                end_included,
                step,
                None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
            )
            .unwrap();

        // Should still return an explanation even with optimization
        let fixed_step_explanation = fixed_result.explanation_for_step(1).unwrap();
        assert!(fixed_step_explanation.contains("FixedAmount optimization"));
        assert!(fixed_step_explanation.contains("50 tokens"));
    }

    #[test]
    fn test_explanation_for_step_with_reward_ratio() {
        let distribution_function = DistributionFunction::FixedAmount { amount: 100 };
        let start_excluded = RewardDistributionMoment::EpochBasedMoment(0);
        let end_included = RewardDistributionMoment::EpochBasedMoment(2);
        let step = RewardDistributionMoment::EpochBasedMoment(1);
        let distribution_start = RewardDistributionMoment::EpochBasedMoment(0);

        // Create a reward ratio function that returns 1/2 ratio
        let get_ratio = |_range: RangeInclusive<EpochIndex>| -> Option<RewardRatio> {
            Some(RewardRatio {
                numerator: 1,
                denominator: 2,
            })
        };

        let result = distribution_function
            .evaluate_interval_with_explanation(
                distribution_start,
                start_excluded,
                end_included,
                step,
                Some(get_ratio),
            )
            .unwrap();

        // For FixedAmount with ratio, it still uses optimization
        let explanation = result.explanation_for_step(1).unwrap();
        assert!(explanation.contains("FixedAmount optimization"));
    }
}
