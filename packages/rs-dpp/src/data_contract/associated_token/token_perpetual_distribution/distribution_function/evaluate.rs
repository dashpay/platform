use crate::balances::credits::TokenAmount;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::{
    DistributionFunction, DEFAULT_STEP_DECREASING_AMOUNT_MAX_CYCLES_BEFORE_TRAILING_DISTRIBUTION,
    MAX_DISTRIBUTION_PARAM,
};
use crate::ProtocolError;

impl DistributionFunction {
    /// Evaluates the distribution function at the given period `x`.
    ///
    /// If an optional start period (`s`) is not provided, it defaults to 0.
    /// The contract registration step is the contract registration moment divided
    /// by the step
    ///
    /// # Returns
    /// A `Result` with the computed token amount or a `ProtocolError` in case of a
    /// divide-by-zero, undefined operation (e.g. log of non-positive number), or overflow.
    pub fn evaluate(
        &self,
        contract_registration_step: u64,
        x: u64,
    ) -> Result<TokenAmount, ProtocolError> {
        match self {
            DistributionFunction::FixedAmount { amount: n } => {
                // For fixed amount, simply return n.
                Ok(*n)
            }
            DistributionFunction::Random { min, max } => {
                // Ensure that min is not greater than max.
                if *min > *max {
                    return Err(ProtocolError::Overflow(
                        "Random: min must be less than or equal to max",
                    ));
                }

                // Use x (the period) as the seed for the PRF.
                let seed = x;
                // A simple SplitMix64-based PRF.
                let mut z = seed.wrapping_add(0x9E3779B97F4A7C15);
                z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
                z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
                z = z ^ (z >> 31);

                // Calculate the range size: (max - min + 1)
                let range = max.saturating_sub(*min).saturating_add(1);

                // Map the pseudorandom number into the desired range.
                let value = min.wrapping_add(z % range);

                Ok(value)
            }

            DistributionFunction::StepDecreasingAmount {
                step_count,
                decrease_per_interval_numerator,
                decrease_per_interval_denominator,
                start_decreasing_offset,
                max_interval_count,
                distribution_start_amount,
                trailing_distribution_interval_amount,
                min_value,
            } => {
                if *decrease_per_interval_denominator == 0 {
                    return Err(ProtocolError::DivideByZero(
                        "StepDecreasingAmount: denominator is 0",
                    ));
                }

                let s_val = start_decreasing_offset.unwrap_or(contract_registration_step);

                if x <= s_val {
                    return Ok(*distribution_start_amount);
                }

                let era_intervals_passed = (x - s_val) / (*step_count as u64);
                let max_intervals = max_interval_count.unwrap_or(
                    DEFAULT_STEP_DECREASING_AMOUNT_MAX_CYCLES_BEFORE_TRAILING_DISTRIBUTION,
                ) as u64;

                if era_intervals_passed > max_intervals {
                    return Ok(*trailing_distribution_interval_amount);
                }

                let mut numerator = *distribution_start_amount;
                let denominator = *decrease_per_interval_denominator as u64;
                let reduction_numerator =
                    denominator.saturating_sub(*decrease_per_interval_numerator as u64);

                for _ in 0..era_intervals_passed {
                    numerator = numerator * reduction_numerator / denominator;
                }

                let mut result = numerator;

                if let Some(min) = min_value {
                    if result < *min {
                        result = *min;
                    }
                }

                Ok(result)
            }
            DistributionFunction::Stepwise(steps) => {
                if x < contract_registration_step {
                    return Ok(0);
                }
                // Return the emission corresponding to the greatest key <= x.
                Ok(steps
                    .range(..=(x - contract_registration_step))
                    .next_back()
                    .map(|(_, amount)| *amount)
                    .unwrap_or(0))
            }
            // f(x) = (a * (x - s) / d) + b
            DistributionFunction::Linear {
                a,
                d,
                start_step,
                starting_amount,
                min_value,
                max_value,
            } => {
                if *d == 0 {
                    return Err(ProtocolError::DivideByZero(
                        "Linear function: divisor d is 0",
                    ));
                }
                // Check that the value at x = 0 is within bounds.
                let s_val = start_step.unwrap_or(contract_registration_step);

                let diff = x.saturating_sub(s_val);
                let value = if *d == 1 {
                    // very common case
                    match a.checked_mul(diff as i64) {
                        None => {
                            if *a < 0 {
                                0
                            } else if let Some(max_value) = max_value {
                                *max_value
                            } else {
                                return Err(ProtocolError::Overflow(
                                    "Linear function evaluation overflow on multiplication",
                                ));
                            }
                        }
                        Some(mul) => {
                            let value = mul.checked_add(*starting_amount as i64).ok_or(
                                ProtocolError::Overflow(
                                    "Linear function evaluation overflow or negative",
                                ),
                            )?;
                            if value < 0 {
                                0
                            } else {
                                value as u64
                            }
                        }
                    }
                } else {
                    let value = (((*a as i128) * (diff as i128) / (*d as i128)) as i64)
                        .checked_add(*starting_amount as i64)
                        .ok_or(ProtocolError::Overflow(
                            "Linear function evaluation overflow or negative",
                        ))?;
                    if value < 0 {
                        0
                    } else {
                        value as u64
                    }
                };

                if let Some(min_value) = min_value {
                    if value < *min_value {
                        return Ok(*min_value);
                    }
                }

                if let Some(max_value) = max_value {
                    if value > *max_value {
                        return Ok(*max_value);
                    }
                }
                Ok(value as TokenAmount)
            }
            // f(x) = (a * (x - s + o)^(m/n)) / d + b
            DistributionFunction::Polynomial {
                a,
                d,
                m,
                n,
                o,
                start_moment,
                b,
                min_value,
                max_value,
            } => {
                if *d == 0 {
                    return Err(ProtocolError::DivideByZero(
                        "Polynomial function: divisor d is 0",
                    ));
                }
                if *n == 0 {
                    return Err(ProtocolError::DivideByZero(
                        "Polynomial function: exponent denominator n is 0",
                    ));
                }
                let s_val = start_moment.unwrap_or(contract_registration_step);
                let exponent = (*m as f64) / (*n as f64);
                let diff = x as i128 - s_val as i128 + *o as i128;

                if diff <= 0 {
                    return if let Some(min_value) = min_value {
                        Ok(*min_value)
                    } else {
                        Ok(0)
                    };
                }

                if diff > u64::MAX as i128 {
                    return Err(ProtocolError::Overflow(
                        "Polynomial function: argument is too big (max should be u64::MAX)",
                    ));
                }

                let diff_exp = (diff as f64).powf(exponent);

                if !diff_exp.is_finite() {
                    return if diff_exp.is_sign_positive() {
                        if let Some(max_value) = max_value {
                            Ok(*max_value)
                        } else {
                            Ok(MAX_DISTRIBUTION_PARAM)
                        }
                    } else if let Some(min_value) = min_value {
                        Ok(*min_value)
                    } else {
                        Ok(0)
                    };
                }

                let pol = diff_exp as i128;

                let intermediate = if *d == 1 {
                    (*a as i128).saturating_mul(pol)
                } else {
                    ((*a as i128).saturating_mul(pol)) / *d as i128
                };

                if intermediate > MAX_DISTRIBUTION_PARAM as i128
                    || intermediate < -(MAX_DISTRIBUTION_PARAM as i128)
                {
                    return if intermediate > 0 {
                        if let Some(max_value) = max_value {
                            Ok(*max_value)
                        } else {
                            Ok(MAX_DISTRIBUTION_PARAM)
                        }
                    } else if let Some(min_value) = min_value {
                        Ok(*min_value)
                    } else {
                        Ok(0)
                    };
                }

                let value =
                    (intermediate as i64)
                        .checked_add(*b as i64)
                        .ok_or(ProtocolError::Overflow(
                            "Polynomial function evaluation overflow",
                        ))?;

                let value = if value < 0 { 0 } else { value as u64 };

                if let Some(min_value) = min_value {
                    if value < *min_value {
                        return Ok(*min_value);
                    }
                }
                if let Some(max_value) = max_value {
                    if value > *max_value {
                        return Ok(*max_value);
                    }
                }

                if value > MAX_DISTRIBUTION_PARAM {
                    Ok(MAX_DISTRIBUTION_PARAM)
                } else {
                    Ok(value)
                }
            }

            DistributionFunction::Exponential {
                a,
                d,
                m,
                n,
                o,
                start_moment,
                b,
                min_value,
                max_value,
            } => {
                if *d == 0 {
                    return Err(ProtocolError::DivideByZero(
                        "Exponential function: divisor d is 0",
                    ));
                }
                if *n == 0 {
                    return Err(ProtocolError::DivideByZero(
                        "Exponential function: exponent denominator n is 0",
                    ));
                }
                let s_val = start_moment.unwrap_or(contract_registration_step);
                let diff = x as i128 - s_val as i128 + *o as i128;

                if diff < -(u64::MAX as i128) {
                    return Err(ProtocolError::Overflow(
                        "Exponential function: argument is too small (min should be -u64::MAX)",
                    ));
                }

                if diff > u64::MAX as i128 {
                    return Err(ProtocolError::Overflow(
                        "Exponential function: argument is too big (max should be u64::MAX)",
                    ));
                }

                let exponent = (*m as f64) * (diff as f64) / (*n as f64);
                let value = ((*a as f64) * exponent.exp() / (*d as f64)) + (*b as f64);
                if let Some(max_value) = max_value {
                    if value.is_infinite() && value.is_sign_positive() || value > *max_value as f64
                    {
                        return Ok(*max_value);
                    }
                }
                if !value.is_finite() || value > (u64::MAX as f64) {
                    return Err(ProtocolError::Overflow(
                        "Exponential function evaluation overflow or negative",
                    ));
                }

                if value < 0.0 {
                    return if let Some(min_value) = min_value {
                        Ok(*min_value)
                    } else {
                        Ok(0)
                    };
                }

                let value_u64 = value as u64;
                if let Some(min_value) = min_value {
                    if value_u64 < *min_value {
                        return Ok(*min_value);
                    }
                }
                Ok(value_u64)
            }

            DistributionFunction::Logarithmic {
                a,
                d,
                m,
                n,
                o,
                start_moment,
                b,
                min_value,
                max_value,
            } => {
                if *d == 0 {
                    return Err(ProtocolError::DivideByZero(
                        "Logarithmic function: divisor d is 0",
                    ));
                }
                if *n == 0 {
                    return Err(ProtocolError::DivideByZero("Logarithmic function: n is 0"));
                }
                let s_val = start_moment.unwrap_or(contract_registration_step);
                let diff = x as i128 - s_val as i128 + *o as i128;

                if diff <= 0 {
                    return Err(ProtocolError::Overflow(
                        "Logarithmic function: argument for log is non-positive",
                    ));
                }

                if diff > u64::MAX as i128 {
                    return Err(ProtocolError::Overflow("Logarithmic function: argument for log is too big (max should be u64::MAX)"));
                }

                let argument = if *m == 1 {
                    if *n == 1 {
                        diff as f64
                    } else {
                        (diff as f64) / (*n as f64)
                    }
                } else if *n == 1 {
                    (*m as f64) * (diff as f64)
                } else {
                    (*m as f64) * (diff as f64) / (*n as f64)
                };

                let log_val = argument.ln();

                // Ensure the computed value is finite and within the u64 range.
                if !log_val.is_finite() || log_val > (u64::MAX as f64) {
                    return Err(ProtocolError::Overflow(
                        "InvertedLogarithmic: evaluation overflow",
                    ));
                }

                let intermediate = if *a == 1 {
                    log_val
                } else if *a == -1 {
                    -log_val
                } else {
                    (*a as f64) * log_val
                };

                let value = if d == &1 {
                    if !intermediate.is_finite() || intermediate > (i64::MAX as f64) {
                        if let Some(max_value) = max_value {
                            if intermediate.is_sign_positive() {
                                *max_value as i64
                            } else {
                                return Err(ProtocolError::Overflow(
                                    "InvertedLogarithmic: evaluation overflow intermediate bigger than i64::max",
                                ));
                            }
                        } else {
                            return Err(ProtocolError::Overflow(
                                "InvertedLogarithmic: evaluation overflow intermediate bigger than i64::max",
                            ));
                        }
                    } else {
                        (intermediate.floor() as i64)
                            .checked_add(*b as i64)
                            .or(max_value.map(|max| max as i64))
                            .ok_or(ProtocolError::Overflow(
                                "InvertedLogarithmic: evaluation overflow when adding b",
                            ))?
                    }
                } else {
                    if !intermediate.is_finite() || intermediate > (i64::MAX as f64) {
                        return Err(ProtocolError::Overflow(
                            "InvertedLogarithmic: evaluation overflow intermediate bigger than i64::max",
                        ));
                    }
                    ((intermediate / (*d as f64)).floor() as i64)
                        .checked_add(*b as i64)
                        .or(max_value.map(|max| max as i64))
                        .ok_or(ProtocolError::Overflow(
                            "InvertedLogarithmic: evaluation overflow when adding b",
                        ))?
                };

                if value < 0 {
                    return if let Some(min_value) = min_value {
                        Ok(*min_value)
                    } else {
                        Ok(0)
                    };
                }
                let value_u64 = value as u64;
                if let Some(min_value) = min_value {
                    if value_u64 < *min_value {
                        return Ok(*min_value);
                    }
                }

                if let Some(max_value) = max_value {
                    if value_u64 > *max_value {
                        return Ok(*max_value);
                    }
                }
                Ok(value_u64)
            }
            DistributionFunction::InvertedLogarithmic {
                a,
                d,
                m,
                n,
                o,
                start_moment,
                b,
                min_value,
                max_value,
            } => {
                // Check for division-by-zero: d, n, and m must be non-zero.
                if *d == 0 {
                    return Err(ProtocolError::DivideByZero(
                        "InvertedLogarithmic: divisor d is 0",
                    ));
                }
                if *n == 0 {
                    return Err(ProtocolError::DivideByZero(
                        "InvertedLogarithmic: parameter n is 0",
                    ));
                }
                if *m == 0 {
                    return Err(ProtocolError::DivideByZero(
                        "InvertedLogarithmic: parameter m is 0",
                    ));
                }

                // Use the provided start period or default to 0.
                let s_val = start_moment.unwrap_or(contract_registration_step);

                // Compute the adjusted time difference: (x - s + o).
                // We use i128 to prevent overflow issues.
                let diff = x as i128 - s_val as i128 + *o as i128;

                // For the inverted logarithmic formula f(x) = (a * ln(n / (m * (x - s + o)))) / d + b,
                // the denominator inside the log must be positive.
                if diff <= 0 {
                    return Err(ProtocolError::Overflow(
                        "InvertedLogarithmic: (x - s + o) must be > 0",
                    ));
                }

                // Calculate the denominator for the logarithm: m * (x - s + o)
                let denom_f = if *m == 1 {
                    diff as f64
                } else {
                    (*m as f64) * (diff as f64)
                };
                if denom_f <= 0.0 {
                    return Err(ProtocolError::Overflow(
                        "InvertedLogarithmic: computed denominator is non-positive",
                    ));
                }

                // Compute the logarithm argument: n / (m * (x - s + o))
                let argument = (*n as f64) / denom_f;
                if argument <= 0.0 {
                    return Err(ProtocolError::Overflow(
                        "InvertedLogarithmic: log argument is non-positive",
                    ));
                }

                let log_val = argument.ln();

                // Ensure the computed value is finite and within the u64 range.
                if !log_val.is_finite() || log_val > (u64::MAX as f64) {
                    return Err(ProtocolError::Overflow(
                        "InvertedLogarithmic: evaluation overflow",
                    ));
                }

                let intermediate = if *a == 1 {
                    log_val
                } else if *a == -1 {
                    -log_val
                } else {
                    (*a as f64) * log_val
                };
                if !intermediate.is_finite() || intermediate > (i64::MAX as f64) {
                    return Err(ProtocolError::Overflow(
                        "InvertedLogarithmic: evaluation overflow intermediate bigger than i64::max",
                    ));
                }

                let value = if d == &1 {
                    (intermediate.floor() as i64).checked_add(*b as i64).ok_or(
                        ProtocolError::Overflow(
                            "InvertedLogarithmic: evaluation overflow when adding b",
                        ),
                    )?
                } else {
                    ((intermediate / (*d as f64)).floor() as i64)
                        .checked_add(*b as i64)
                        .ok_or(ProtocolError::Overflow(
                            "InvertedLogarithmic: evaluation overflow when adding b",
                        ))?
                };

                // Clamp to max_value if provided.
                if let Some(max_value) = max_value {
                    if value > *max_value as i64 {
                        return Ok(*max_value);
                    }
                }

                if value < 0 {
                    return if let Some(min_value) = min_value {
                        Ok(*min_value)
                    } else {
                        Ok(0)
                    };
                }

                let value_u64 = value as u64;

                // Clamp to min_value if provided.
                if let Some(min_value) = min_value {
                    if value_u64 < *min_value {
                        return Ok(*min_value);
                    }
                }
                Ok(value_u64)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_fixed_amount() {
        let distribution = DistributionFunction::FixedAmount { amount: 100 };
        assert_eq!(distribution.evaluate(0, 0).unwrap(), 100);
        assert_eq!(distribution.evaluate(0, 50).unwrap(), 100);
        assert_eq!(distribution.evaluate(0, 1000).unwrap(), 100);
    }

    #[test]
    fn test_stepwise_emission() {
        let mut steps = BTreeMap::new();
        steps.insert(0, 100);
        steps.insert(10, 50);
        steps.insert(20, 25);

        let distribution = DistributionFunction::Stepwise(steps);
        assert_eq!(distribution.evaluate(0, 0).unwrap(), 100);
        assert_eq!(distribution.evaluate(0, 5).unwrap(), 100);
        assert_eq!(distribution.evaluate(0, 10).unwrap(), 50);
        assert_eq!(distribution.evaluate(0, 15).unwrap(), 50);
        assert_eq!(distribution.evaluate(0, 20).unwrap(), 25);
        assert_eq!(distribution.evaluate(0, 30).unwrap(), 25);
    }

    #[test]
    fn test_step_decreasing_amount() {
        let distribution = DistributionFunction::StepDecreasingAmount {
            step_count: 10,
            decrease_per_interval_numerator: 1,
            decrease_per_interval_denominator: 2, // 50% reduction per step
            start_decreasing_offset: Some(0),
            max_interval_count: None,
            distribution_start_amount: 100,
            trailing_distribution_interval_amount: 0,
            min_value: Some(10),
        };

        assert_eq!(distribution.evaluate(0, 0).unwrap(), 100);
        assert_eq!(distribution.evaluate(0, 9).unwrap(), 100);
        assert_eq!(distribution.evaluate(0, 10).unwrap(), 50);
        assert_eq!(distribution.evaluate(0, 20).unwrap(), 25);
        assert_eq!(distribution.evaluate(0, 30).unwrap(), 12);
        assert_eq!(distribution.evaluate(0, 40).unwrap(), 10); // Should not go below min_value
    }

    #[test]
    fn test_step_decreasing_amount_divide_by_zero() {
        let distribution = DistributionFunction::StepDecreasingAmount {
            step_count: 10,
            decrease_per_interval_numerator: 1,
            decrease_per_interval_denominator: 0, // Invalid denominator
            start_decreasing_offset: Some(0),
            max_interval_count: None,
            distribution_start_amount: 100,
            trailing_distribution_interval_amount: 0,
            min_value: Some(10),
        };

        assert!(matches!(
            distribution.evaluate(0, 10),
            Err(ProtocolError::DivideByZero(_))
        ));
    }
    mod random {
        use super::*;

        #[test]
        fn test_random_distribution_with_valid_range() {
            let distribution = DistributionFunction::Random { min: 10, max: 100 };

            for x in 0..100 {
                let result = distribution.evaluate(0, x).unwrap();
                assert!(
                    (10..=100).contains(&result),
                    "Random value {} is out of range for x = {}",
                    result,
                    x
                );
            }
        }

        #[test]
        fn test_random_distribution_with_single_value_range() {
            let distribution = DistributionFunction::Random { min: 42, max: 42 };

            for x in 0..10 {
                let result = distribution.evaluate(0, x).unwrap();
                assert_eq!(
                    result, 42,
                    "Expected fixed output 42, got {} for x = {}",
                    result, x
                );
            }
        }

        #[test]
        fn test_random_distribution_invalid_range() {
            let distribution = DistributionFunction::Random { min: 50, max: 40 };

            let result = distribution.evaluate(0, 0);
            assert!(
                matches!(result, Err(ProtocolError::Overflow(_))),
                "Expected ProtocolError::Overflow but got {:?}",
                result
            );
        }

        #[test]
        fn test_random_distribution_deterministic_for_same_x() {
            let distribution = DistributionFunction::Random { min: 10, max: 100 };

            let value1 = distribution.evaluate(0, 42).unwrap();
            let value2 = distribution.evaluate(0, 42).unwrap();

            assert_eq!(
                value1, value2,
                "Random distribution should be deterministic for the same x"
            );
        }

        #[test]
        fn test_random_distribution_varies_for_different_x() {
            let distribution = DistributionFunction::Random { min: 10, max: 100 };

            let value1 = distribution.evaluate(0, 1).unwrap();
            let value2 = distribution.evaluate(0, 2).unwrap();

            assert_ne!(
                value1, value2,
                "Random distribution should vary for different x values"
            );
        }
    }
    mod linear {
        use super::*;
        #[test]
        fn test_linear_function_increasing() {
            let distribution = DistributionFunction::Linear {
                a: 10,
                d: 2,
                start_step: Some(0),
                starting_amount: 50,
                min_value: None,
                max_value: None,
            };

            assert_eq!(distribution.evaluate(0, 0).unwrap(), 50);
            assert_eq!(distribution.evaluate(0, 2).unwrap(), 60);
            assert_eq!(distribution.evaluate(0, 4).unwrap(), 70);
            assert_eq!(distribution.evaluate(0, 6).unwrap(), 80);
        }

        #[test]
        fn test_linear_function_decreasing() {
            let distribution = DistributionFunction::Linear {
                a: -5,
                d: 1,
                start_step: Some(0),
                starting_amount: 100,
                min_value: Some(10),
                max_value: None,
            };

            assert_eq!(distribution.evaluate(0, 0).unwrap(), 100);
            assert_eq!(distribution.evaluate(0, 10).unwrap(), 50);
            assert_eq!(distribution.evaluate(0, 20).unwrap(), 10); // Should not go below min_value
        }

        #[test]
        fn test_linear_function_divide_by_zero() {
            let distribution = DistributionFunction::Linear {
                a: 10,
                d: 0, // Invalid denominator
                start_step: Some(0),
                starting_amount: 50,
                min_value: None,
                max_value: None,
            };

            assert!(matches!(
                distribution.evaluate(0, 10),
                Err(ProtocolError::DivideByZero(_))
            ));
        }
    }
    mod polynomial {
        use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::{MAX_POL_A_PARAM, MAX_POL_M_PARAM};
        use super::*;
        #[test]
        fn test_polynomial_function() {
            let distribution = DistributionFunction::Polynomial {
                a: 2,
                d: 1,
                m: 2,
                n: 1,
                o: 0,
                start_moment: Some(0),
                b: 10,
                min_value: None,
                max_value: None,
            };

            assert_eq!(distribution.evaluate(0, 0).unwrap(), 0);
            assert_eq!(distribution.evaluate(0, 2).unwrap(), 18);
            assert_eq!(distribution.evaluate(0, 3).unwrap(), 28);
            assert_eq!(distribution.evaluate(0, 4).unwrap(), 42);
        }

        #[test]
        fn test_polynomial_function_should_not_overflow() {
            let distribution = DistributionFunction::Polynomial {
                a: MAX_POL_A_PARAM,
                d: 1,
                m: MAX_POL_M_PARAM,
                n: 1,
                o: 0,
                start_moment: Some(0),
                b: 10,
                min_value: None,
                max_value: None,
            };

            let result = distribution.evaluate(0, 100000).expect("expected value");
            assert_eq!(result, MAX_DISTRIBUTION_PARAM);
        }

        // Test: Fractional exponent (exponent = 3/2)
        #[test]
        fn test_polynomial_function_fraction_exponent() {
            let distribution = DistributionFunction::Polynomial {
                a: 1,
                d: 1,
                m: 3, // exponent is 3/2
                n: 2,
                o: 0,
                start_moment: Some(0),
                b: 0,
                min_value: None,
                max_value: None,
            };
            // (4 - 0 + 0)^(3/2) = 4^(3/2) = (sqrt(4))^3 = 2^3 = 8.
            assert_eq!(distribution.evaluate(0, 4).unwrap(), 8);
        }

        // Test: Negative coefficient a (should flip the sign)
        #[test]
        fn test_polynomial_function_negative_a() {
            let distribution = DistributionFunction::Polynomial {
                a: -1,
                d: 1,
                m: 2,
                n: 1,
                o: 0,
                start_moment: Some(0),
                b: 0,
                min_value: None,
                max_value: None,
            };
            // f(x) = -1 * (x^2). For x = 3: -1 * (3^2) = -9.
            assert_eq!(distribution.evaluate(0, 3).unwrap(), 0);
        }

        // Test: Non-zero shift parameter s (shifting the x coordinate)
        #[test]
        fn test_polynomial_function_with_shift() {
            let distribution = DistributionFunction::Polynomial {
                a: 2,
                d: 1,
                m: 2,
                n: 1,
                o: 0,
                start_moment: Some(2),
                b: 10,
                min_value: None,
                max_value: None,
            };
            // since it starts at 2 (that's like the contract registration at 2, so we should get 0
            assert_eq!(distribution.evaluate(0, 2).unwrap(), 0);
            // At x = 3: (3 - 2)^2 = 1, f(3) = 2*1 + 10 = 12.
            assert_eq!(distribution.evaluate(0, 3).unwrap(), 12);
        }

        // Test: Non-zero offset o (shifting the base of the power)
        #[test]
        fn test_polynomial_function_with_offset() {
            let distribution = DistributionFunction::Polynomial {
                a: 2,
                d: 1,
                m: 2,
                n: 1,
                o: 3,
                start_moment: Some(0),
                b: 10,
                min_value: None,
                max_value: None,
            };
            // f(x) = 2 * ((x - 0 + 3)^2) + 10.
            // At x = 1: (1 + 3) = 4, 4^2 = 16, then 2*16 + 10 = 42.
            assert_eq!(distribution.evaluate(0, 1).unwrap(), 42);
        }

        // Test: Linear function when exponent is 1 (m = 1, n = 1)
        #[test]
        fn test_polynomial_function_linear() {
            let distribution = DistributionFunction::Polynomial {
                a: 3,
                d: 1,
                m: 1,
                n: 1,
                o: 0,
                start_moment: Some(0),
                b: 5,
                min_value: None,
                max_value: None,
            };
            // f(x) = 3*x + 5. At x = 10, f(10) = 30 + 5 = 35.
            assert_eq!(distribution.evaluate(0, 10).unwrap(), 35);
        }

        // Test: Cubic function (m = 3, n = 1)
        #[test]
        fn test_polynomial_function_cubic() {
            let distribution = DistributionFunction::Polynomial {
                a: 1,
                d: 1,
                m: 3,
                n: 1,
                o: 0,
                start_moment: Some(0),
                b: 0,
                min_value: None,
                max_value: None,
            };
            // f(x) = x^3. At x = 4, f(4) = 64.
            assert_eq!(distribution.evaluate(0, 4).unwrap(), 64);
        }

        // Test: Combination of non-zero offset and shift
        #[test]
        fn test_polynomial_function_with_offset_and_shift() {
            let distribution = DistributionFunction::Polynomial {
                a: 1,
                d: 1,
                m: 2,
                n: 1,
                o: 2,
                start_moment: Some(1),
                b: 0,
                min_value: None,
                max_value: None,
            };
            // f(x) = ( (x - 1 + 2)^2 ).
            // At x = 3: (3 - 1 + 2) = 4, and 4^2 = 16.
            assert_eq!(distribution.evaluate(0, 3).unwrap(), 16);
        }
    }
    mod exp {
        use super::*;
        #[test]
        fn test_exponential_function() {
            let distribution = DistributionFunction::Exponential {
                a: 1,
                d: 1,
                m: 1,
                n: 1,
                o: 0,
                start_moment: Some(0),
                b: 10,
                min_value: None,
                max_value: None,
            };

            assert_eq!(distribution.evaluate(0, 0).unwrap(), 11);
            assert!(distribution.evaluate(0, 10).unwrap() > 20);
        }

        #[test]
        fn test_exponential_function_divide_by_zero() {
            let distribution = DistributionFunction::Exponential {
                a: 1,
                d: 0, // Invalid denominator
                m: 1,
                n: 1,
                o: 0,
                start_moment: Some(0),
                b: 10,
                min_value: None,
                max_value: None,
            };

            assert!(matches!(
                distribution.evaluate(0, 10),
                Err(ProtocolError::DivideByZero(_))
            ));
        }

        #[test]
        fn test_exponential_function_basic() {
            let distribution = DistributionFunction::Exponential {
                a: 2,
                d: 1,
                m: 1,
                n: 1,
                o: 0,
                start_moment: Some(0),
                b: 5,
                min_value: None,
                max_value: None,
            };

            assert_eq!(distribution.evaluate(0, 0).unwrap(), 7);
            assert_eq!(distribution.evaluate(0, 5).unwrap(), 301);
            assert_eq!(distribution.evaluate(0, 10).unwrap(), 44057);
        }

        #[test]
        fn test_exponential_function_slow_growth() {
            let distribution = DistributionFunction::Exponential {
                a: 1,
                d: 10,
                m: 1,
                n: 10,
                o: 0,
                start_moment: Some(0),
                b: 0,
                min_value: None,
                max_value: None,
            };

            assert_eq!(distribution.evaluate(0, 0).unwrap(), 0);
            assert_eq!(distribution.evaluate(0, 50).unwrap(), 14);
            assert_eq!(distribution.evaluate(0, 100).unwrap(), 2202);
        }

        #[test]
        fn test_exponential_function_rapid_growth() {
            let distribution = DistributionFunction::Exponential {
                a: 1,
                d: 1,
                m: 4,
                n: 1,
                o: 0,
                start_moment: Some(0),
                b: 0,
                min_value: None,
                max_value: Some(100000000),
            };

            assert_eq!(distribution.evaluate(0, 0).unwrap(), 1);
            assert_eq!(distribution.evaluate(0, 2).unwrap(), 2980);
            assert_eq!(distribution.evaluate(0, 4).unwrap(), 8886110);
            assert_eq!(distribution.evaluate(0, 10).unwrap(), 100000000);
            assert_eq!(distribution.evaluate(0, 100000).unwrap(), 100000000);
        }

        #[test]
        fn test_exponential_function_with_no_min_value() {
            let distribution = DistributionFunction::Exponential {
                a: 2,
                d: 1,
                m: -1,
                n: 1,
                o: 0,
                start_moment: Some(0),
                b: 10,
                min_value: None,
                max_value: None,
            };

            assert_eq!(distribution.evaluate(0, 0).unwrap(), 12); // f(0) = (2 * e^(-1 * (0 - 0 + 0) / 1)) / 1 + 10
            assert_eq!(distribution.evaluate(0, 5).unwrap(), 10);
            assert_eq!(distribution.evaluate(0, 10000).unwrap(), 10);
        }

        #[test]
        fn test_exponential_function_with_min_value() {
            let distribution = DistributionFunction::Exponential {
                a: 2,
                d: 1,
                m: -1,
                n: 1,
                o: 0,
                start_moment: Some(0),
                b: 10,
                min_value: Some(11),
                max_value: None,
            };

            assert_eq!(distribution.evaluate(0, 0).unwrap(), 12); // f(0) = (2 * e^(-1 * (0 - 0 + 0) / 1)) / 1 + 10
            assert_eq!(distribution.evaluate(0, 5).unwrap(), 11);
            assert_eq!(distribution.evaluate(0, 100).unwrap(), 11);
        }

        #[test]
        fn test_exponential_function_starting_at_max() {
            let distribution = DistributionFunction::Exponential {
                a: 2,
                d: 1,
                m: 1,
                n: 2,
                o: 0,
                start_moment: Some(0),
                b: 10,
                min_value: Some(1),
                max_value: Some(11), // Set max at the starting value
            };

            assert_eq!(
                distribution.evaluate(0, 0).unwrap(),
                11,
                "Function should start at the max value"
            );
            assert_eq!(
                distribution.evaluate(0, 5).unwrap(),
                11,
                "Function should be clamped at max value"
            );
        }

        #[test]
        fn test_exponential_function_large_x_overflow() {
            let distribution = DistributionFunction::Exponential {
                a: 2,
                d: 1,
                m: 1,
                n: 10,
                o: 0,
                start_moment: Some(0),
                b: 5,
                min_value: None,
                max_value: None,
            };

            let result = distribution.evaluate(0, 100000);
            assert!(
                matches!(result, Err(ProtocolError::Overflow(_))),
                "Expected overflow but got {:?}",
                result
            );
        }
    }
    mod log {
        use super::*;
        #[test]
        fn test_logarithmic_function() {
            let distribution = DistributionFunction::Logarithmic {
                a: 10,
                d: 1,
                m: 1,
                n: 1,
                o: 1,                  // Offset ensures (x - s + o) > 0
                start_moment: Some(1), // Start at x=1 to avoid log(0)
                b: 5,
                min_value: None,
                max_value: None,
            };

            assert_eq!(distribution.evaluate(0, 1).unwrap(), 5);
            assert!(distribution.evaluate(0, 10).unwrap() > 5);
        }

        #[test]
        fn test_logarithmic_function_with_min_max_bounds() {
            let distribution = DistributionFunction::Logarithmic {
                a: 10,
                d: 1,
                m: 1,
                n: 1,
                o: 1,
                start_moment: Some(1),
                b: 5,
                min_value: Some(7),  // Minimum bound should be enforced
                max_value: Some(20), // Maximum bound should be enforced
            };

            assert_eq!(distribution.evaluate(0, 1).unwrap(), 7); // Clamped to min_value
            assert!(distribution.evaluate(0, 10).unwrap() <= 20); // Should not exceed max_value
        }

        #[test]
        fn test_logarithmic_function_undefined() {
            let distribution = DistributionFunction::Logarithmic {
                a: 10,
                d: 1,
                m: 1,
                n: 1,
                o: -1, // Invalid offset causing log(0)
                start_moment: Some(1),
                b: 5,
                min_value: None,
                max_value: None,
            };

            assert!(matches!(
                distribution.evaluate(0, 1),
                Err(ProtocolError::Overflow(_))
            ));
        }

        #[test]
        fn test_logarithmic_function_large_x() {
            let distribution = DistributionFunction::Logarithmic {
                a: 100,
                d: 2,
                m: 1,
                n: 1,
                o: 5,
                start_moment: Some(10),
                b: 10,
                min_value: None,
                max_value: None,
            };

            let result = distribution.evaluate(0, 100);
            assert!(result.is_ok());
            assert!(result.unwrap() > 10); // Function should increase over time
        }

        #[test]
        fn test_logarithmic_function_divide_by_zero_d() {
            let distribution = DistributionFunction::Logarithmic {
                a: 10,
                d: 0, // Invalid: Division by zero
                m: 1,
                n: 1,
                o: 1,
                start_moment: Some(5),
                b: 5,
                min_value: None,
                max_value: None,
            };

            assert!(matches!(
                distribution.evaluate(0, 10),
                Err(ProtocolError::DivideByZero(_))
            ));
        }

        #[test]
        fn test_logarithmic_function_divide_by_zero_n() {
            let distribution = DistributionFunction::Logarithmic {
                a: 10,
                d: 1,
                m: 1,
                n: 0, // Invalid: Division by zero in log denominator
                o: 1,
                start_moment: Some(5),
                b: 5,
                min_value: None,
                max_value: None,
            };

            assert!(matches!(
                distribution.evaluate(0, 10),
                Err(ProtocolError::DivideByZero(_))
            ));
        }
    }
    mod inverted_log {
        use super::*;
        #[test]
        fn test_inverted_logarithmic_basic_decreasing() {
            let distribution = DistributionFunction::InvertedLogarithmic {
                a: 10,
                d: 1,
                m: 1,
                n: 100,
                o: 1,
                start_moment: Some(0),
                b: 5,
                min_value: None,
                max_value: None,
            };

            assert!(distribution.evaluate(0, 1).unwrap() > distribution.evaluate(0, 5).unwrap());
            assert!(distribution.evaluate(0, 5).unwrap() > distribution.evaluate(0, 10).unwrap());
        }

        #[test]
        fn test_inverted_logarithmic_basic_increasing() {
            // f(x) = (-10 * log( 1000 / (x + 10) )) + 5
            let distribution = DistributionFunction::InvertedLogarithmic {
                a: -10,
                d: 1,
                m: 1,
                n: 1000,
                o: 10,
                start_moment: Some(0),
                b: 5,
                min_value: None,
                max_value: None,
            };

            let val1000 = distribution.evaluate(0, 1000).unwrap();
            let val2000 = distribution.evaluate(0, 2000).unwrap();
            let val3000 = distribution.evaluate(0, 3000).unwrap();

            assert!(val1000 < val2000, "Function should be increasing");
            assert!(val2000 < val3000, "Function should be increasing");
        }

        #[test]
        fn test_inverted_logarithmic_negative_clamped_to_0() {
            let distribution = DistributionFunction::InvertedLogarithmic {
                a: -10,
                d: 1,
                m: 1,
                n: 100,
                o: 1,
                start_moment: Some(0),
                b: 5,
                min_value: None,
                max_value: None,
            };

            assert_eq!(distribution.evaluate(0, 1).unwrap(), 0); // Should be clamped to 0
        }

        #[test]
        fn test_inverted_logarithmic_clamped_by_min_value() {
            let distribution = DistributionFunction::InvertedLogarithmic {
                a: 10,
                d: 1,
                m: 1,
                n: 100,
                o: 1,
                start_moment: Some(0),
                b: 5,
                min_value: Some(7),
                max_value: None,
            };

            assert_eq!(distribution.evaluate(0, 1000).unwrap(), 7); // Should be clamped to min_value
        }

        #[test]
        fn test_inverted_logarithmic_clamped_by_max_value() {
            // f(x) = (-10 * log( 100 / (x + 1) )) + 5
            let distribution = DistributionFunction::InvertedLogarithmic {
                a: -10,
                d: 1,
                m: 1,
                n: 100,
                o: 1,
                start_moment: Some(0),
                b: 5,
                min_value: None,
                max_value: Some(20),
            };

            assert_eq!(distribution.evaluate(0, 500).unwrap(), 20); // Should be clamped to max_value
        }

        #[test]
        fn test_inverted_logarithmic_undefined_log_argument_zero() {
            let distribution = DistributionFunction::InvertedLogarithmic {
                a: 10,
                d: 1,
                m: 1,
                n: 100,
                o: -1,
                start_moment: Some(1),
                b: 5,
                min_value: None,
                max_value: None,
            };

            assert!(matches!(
                distribution.evaluate(0, 1),
                Err(ProtocolError::Overflow(_))
            ));
        }

        #[test]
        fn test_inverted_logarithmic_divide_by_zero_n() {
            let distribution = DistributionFunction::InvertedLogarithmic {
                a: 10,
                d: 1,
                m: 1,
                n: 0, // Invalid: n must not be zero
                o: 1,
                start_moment: Some(5),
                b: 5,
                min_value: None,
                max_value: None,
            };

            assert!(matches!(
                distribution.evaluate(0, 10),
                Err(ProtocolError::DivideByZero(_))
            ));
        }

        #[test]
        fn test_inverted_logarithmic_divide_by_zero_d() {
            let distribution = DistributionFunction::InvertedLogarithmic {
                a: 10,
                d: 0, // Invalid: d must not be zero
                m: 1,
                n: 1,
                o: 1,
                start_moment: Some(5),
                b: 5,
                min_value: None,
                max_value: None,
            };

            assert!(matches!(
                distribution.evaluate(0, 10),
                Err(ProtocolError::DivideByZero(_))
            ));
        }

        #[test]
        fn test_inverted_logarithmic_increasing_starts_at_min_value() {
            let distribution = DistributionFunction::InvertedLogarithmic {
                a: -10, // Increasing function
                d: 1,
                m: 1,
                n: 100,
                o: 1,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(10), // Max value set at the starting point
            };

            assert_eq!(
                distribution.evaluate(0, 0).unwrap(),
                1,
                "Function should start at the max value"
            );
            assert_eq!(
                distribution.evaluate(0, 200).unwrap(),
                10,
                "Function should remain clamped at max value"
            );
        }

        #[test]
        fn test_inverted_logarithmic_starts_at_min_value() {
            let distribution = DistributionFunction::InvertedLogarithmic {
                a: 10, // Decreasing function
                d: 1,
                m: 1,
                n: 100,
                o: 1,
                start_moment: Some(0),
                b: 5,
                min_value: Some(3),
                max_value: None,
            };

            assert_eq!(
                distribution.evaluate(0, 1000).unwrap(),
                3,
                "Function should remain clamped at min value"
            );
        }
    }
}
