use crate::balances::credits::{TokenAmount};
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
use crate::ProtocolError;
// adjust the import path as needed

impl DistributionFunction {
    /// Evaluates the distribution function at the given period `x`.
    ///
    /// If an optional start period (`s`) is not provided, it defaults to 0.
    ///
    /// # Returns
    /// A `Result` with the computed token amount or a `ProtocolError` in case of a
    /// divide-by-zero, undefined operation (e.g. log of non-positive number), or overflow.
    pub fn evaluate(&self, x: u64) -> Result<TokenAmount, ProtocolError> {
        match self {
            DistributionFunction::FixedAmount { n } => {
                // For fixed amount, simply return n.
                Ok(*n)
            }

            DistributionFunction::StepDecreasingAmount {
                step_count,
                decrease_per_interval_numerator,
                decrease_per_interval_denominator,
                s,
                n,
                min_value,
            } => {
                // Check for division by zero in the denominator:
                if *decrease_per_interval_denominator == 0 {
                    return Err(ProtocolError::DivideByZero("StepDecreasingAmount: denominator is 0"));
                }
                let s_val = s.unwrap_or(0);
                // Compute the number of steps passed.
                let steps = if x > s_val {
                    (x - s_val) / (*step_count as u64)
                } else {
                    0
                };
                let reduction = 1.0 - ((*decrease_per_interval_numerator as f64)
                    / (*decrease_per_interval_denominator as f64));
                let factor = reduction.powf(steps as f64);
                let result = (*n as f64) * factor;
                // Clamp to min_value if provided.
                let clamped = if let Some(min) = min_value {
                    result.max(*min as f64)
                } else {
                    result
                };
                if !clamped.is_finite() || clamped > (u64::MAX as f64) || clamped < 0.0 {
                    return Err(ProtocolError::Overflow("StepDecreasingAmount evaluation overflow or negative"));
                }
                Ok(clamped as TokenAmount)
            }

            DistributionFunction::Stepwise(steps) => {
                // Return the emission corresponding to the greatest key <= x.
                Ok(steps
                    .range(..=x)
                    .next_back()
                    .map(|(_, amount)| *amount)
                    .unwrap_or(0))
            }
            // f(x) = (a * (x - s) / d) + b
            DistributionFunction::Linear { a, d, s, b, min_value, max_value } => {
                if *d == 0 {
                    return Err(ProtocolError::DivideByZero("Linear function: divisor d is 0"));
                }
                // Check that the value at x = 0 is within bounds.
                let s_val = s.unwrap_or(0);

                let diff = x.saturating_sub(s_val) as i128;
                let value = (((*a as i128) * diff / (*d as i128)) as i64).checked_add(*b as i64).ok_or(ProtocolError::Overflow("Linear function evaluation overflow or negative"))?;

                let value = if value < 0 {
                    0
                } else {
                    value as u64
                };
                if let Some(min_value) = min_value {
                    if value < *min_value {
                        return Ok(*min_value)
                    }
                }

                if let Some(max_value) = max_value {
                    if value < *max_value {
                        return Ok(*max_value)
                    }
                }
                Ok(value as TokenAmount)
            }
            // f(x) = (a * (x - s + o)^(m/n)) / d + b
            DistributionFunction::Polynomial { a, d, m, n, o, s, b, min_value, max_value } => {
                if *d == 0 {
                    return Err(ProtocolError::DivideByZero("Polynomial function: divisor d is 0"));
                }
                if *n == 0 {
                    return Err(ProtocolError::DivideByZero("Polynomial function: exponent denominator n is 0"));
                }
                let s_val = s.unwrap_or(0);
                let exponent = (*m as f64) / (*n as f64);
                let diff = x as i128 - s_val as i128 + *o as i128;

                if diff < 0 {
                    return Err(ProtocolError::Overflow("Polynomial function: argument is non-positive".into()));
                }

                if diff > u64::MAX as i128 {
                    return Err(ProtocolError::Overflow("Polynomial function: argument is too big (max should be u64::MAX)".into()));
                }
                
                let diff_exp = (diff as f64).powf(exponent);

                if !diff_exp.is_finite() || diff_exp.abs() > (u64::MAX as f64) {
                    return Err(ProtocolError::Overflow("Polynomial function evaluation overflow or negative"));
                }
                
                let pol = diff_exp as i128;

                let value = (((*a as i128) * pol / (*d as i128)) as i64).checked_add(*b as i64).ok_or(ProtocolError::Overflow("Polynomial function evaluation overflow or negative"))?;

                let value = if value < 0 {
                    0
                } else {
                    value as u64
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
                Ok(value)
            },

            DistributionFunction::Exponential { a, d, m, n, o, s, c, min_value, max_value } => {
                if *d == 0 {
                    return Err(ProtocolError::DivideByZero("Exponential function: divisor d is 0"));
                }
                if *n == 0 {
                    return Err(ProtocolError::DivideByZero("Exponential function: exponent denominator n is 0"));
                }
                let s_val = s.unwrap_or(0);
                let diff = x as i128 - s_val as i128 + *o as i128;

                if diff < - (u64::MAX as i128) {
                    return Err(ProtocolError::Overflow("Exponential function: argument is too small (min should be -u64::MAX)".into()));
                }

                if diff > u64::MAX as i128 {
                    return Err(ProtocolError::Overflow("Exponential function: argument is too big (max should be u64::MAX)".into()));
                }

                let exponent = (*m as f64) * (diff as f64) / (*n as f64);
                let value = ((*a as f64) * exponent.exp() / (*d as f64)) + (*c as f64);
                if let Some(max_value) = max_value {
                    if value.is_infinite() && value.is_sign_positive() || value > *max_value as f64 {
                        return Ok(*max_value);
                    }
                }
                if !value.is_finite() || value > (u64::MAX as f64) || value < 0.0 {
                    return Err(ProtocolError::Overflow("Exponential function evaluation overflow or negative"));
                }
                let value_u64 = value as u64;
                if let Some(min_value) = min_value {
                    if value_u64 < *min_value {
                        return Ok(*min_value);
                    }
                }
                Ok(value_u64)
            },

            DistributionFunction::Logarithmic { a, d, m, n, o,  s, b, min_value, max_value } => {
                if *d == 0 {
                    return Err(ProtocolError::DivideByZero("Logarithmic function: divisor d is 0"));
                }
                if *n == 0 {
                    return Err(ProtocolError::DivideByZero("Logarithmic function: n is 0"));
                }
                let s_val = s.unwrap_or(0);
                let diff = x as i128 - s_val as i128 + *o as i128;

                if diff <= 0 {
                    return Err(ProtocolError::Overflow("Logarithmic function: argument for log is non-positive".into()));
                }

                if diff > u64::MAX as i128 {
                    return Err(ProtocolError::Overflow("Logarithmic function: argument for log is too big (max should be u64::MAX)".into()));
                }

                let argument = (*m as f64) * (diff as f64) / (*n as f64);

                let log_val = argument.ln();
                let value = ((*a as f64) * log_val / (*d as f64)) + (*b as f64);
                if let Some(max_value) = max_value {
                    if value.is_infinite() && value.is_sign_positive() || value > *max_value as f64 {
                        return Ok(*max_value);
                    }
                }
                if !value.is_finite() || value > (u64::MAX as f64) || value < 0.0 {
                    return Err(ProtocolError::Overflow("Logarithmic function evaluation overflow or negative"));
                }
                let value_u64 = value as u64;
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
        let distribution = DistributionFunction::FixedAmount { n: 100 };
        assert_eq!(distribution.evaluate(0).unwrap(), 100);
        assert_eq!(distribution.evaluate(50).unwrap(), 100);
        assert_eq!(distribution.evaluate(1000).unwrap(), 100);
    }

    #[test]
    fn test_stepwise_emission() {
        let mut steps = BTreeMap::new();
        steps.insert(0, 100);
        steps.insert(10, 50);
        steps.insert(20, 25);

        let distribution = DistributionFunction::Stepwise(steps);
        assert_eq!(distribution.evaluate(0).unwrap(), 100);
        assert_eq!(distribution.evaluate(5).unwrap(), 100);
        assert_eq!(distribution.evaluate(10).unwrap(), 50);
        assert_eq!(distribution.evaluate(15).unwrap(), 50);
        assert_eq!(distribution.evaluate(20).unwrap(), 25);
        assert_eq!(distribution.evaluate(30).unwrap(), 25);
    }

    #[test]
    fn test_step_decreasing_amount() {
        let distribution = DistributionFunction::StepDecreasingAmount {
            step_count: 10,
            decrease_per_interval_numerator: 1,
            decrease_per_interval_denominator: 2, // 50% reduction per step
            s: Some(0),
            n: 100,
            min_value: Some(10),
        };

        assert_eq!(distribution.evaluate(0).unwrap(), 100);
        assert_eq!(distribution.evaluate(9).unwrap(), 100);
        assert_eq!(distribution.evaluate(10).unwrap(), 50);
        assert_eq!(distribution.evaluate(20).unwrap(), 25);
        assert_eq!(distribution.evaluate(30).unwrap(), 12);
        assert_eq!(distribution.evaluate(40).unwrap(), 10); // Should not go below min_value
    }

    #[test]
    fn test_step_decreasing_amount_divide_by_zero() {
        let distribution = DistributionFunction::StepDecreasingAmount {
            step_count: 10,
            decrease_per_interval_numerator: 1,
            decrease_per_interval_denominator: 0, // Invalid denominator
            s: Some(0),
            n: 100,
            min_value: Some(10),
        };

        assert!(matches!(
            distribution.evaluate(10),
            Err(ProtocolError::DivideByZero(_))
        ));
    }

    #[test]
    fn test_linear_function_increasing() {
        let distribution = DistributionFunction::Linear {
            a: 10,
            d: 2,
            s: Some(0),
            b: 50,
            min_value: None,
            max_value: None,
        };

        assert_eq!(distribution.evaluate(0).unwrap(), 50);
        assert_eq!(distribution.evaluate(2).unwrap(), 60);
        assert_eq!(distribution.evaluate(4).unwrap(), 70);
        assert_eq!(distribution.evaluate(6).unwrap(), 80);
    }

    #[test]
    fn test_linear_function_decreasing() {
        let distribution = DistributionFunction::Linear {
            a: -5,
            d: 1,
            s: Some(0),
            b: 100,
            min_value: Some(10),
            max_value: None,
        };

        assert_eq!(distribution.evaluate(0).unwrap(), 100);
        assert_eq!(distribution.evaluate(10).unwrap(), 50);
        assert_eq!(distribution.evaluate(20).unwrap(), 10); // Should not go below min_value
    }

    #[test]
    fn test_linear_function_divide_by_zero() {
        let distribution = DistributionFunction::Linear {
            a: 10,
            d: 0, // Invalid denominator
            s: Some(0),
            b: 50,
            min_value: None,
            max_value: None,
        };

        assert!(matches!(
            distribution.evaluate(10),
            Err(ProtocolError::DivideByZero(_))
        ));
    }

    #[test]
    fn test_polynomial_function() {
        let distribution = DistributionFunction::Polynomial {
            a: 2,
            d: 1,
            m: 2,
            n: 1,
            o: 0,
            s: Some(0),
            b: 10,
            min_value: None,
            max_value: None,
        };

        assert_eq!(distribution.evaluate(0).unwrap(), 10);
        assert_eq!(distribution.evaluate(2).unwrap(), 18);
        assert_eq!(distribution.evaluate(3).unwrap(), 28);
        assert_eq!(distribution.evaluate(4).unwrap(), 42);
    }

    #[test]
    fn test_polynomial_function_overflow() {
        let distribution = DistributionFunction::Polynomial {
            a: i64::MAX,
            d: 1,
            m: 2,
            n: 1,
            o: 0,
            s: Some(0),
            b: 10,
            min_value: None,
            max_value: None,
        };

        let result = distribution.evaluate(1);
        assert!(matches!(result, Err(ProtocolError::Overflow(_))), "Expected overflow but got {:?}", result);
    }

    #[test]
    fn test_exponential_function() {
        let distribution = DistributionFunction::Exponential {
            a: 1,
            d: 1,
            m: 1,
            n: 1,
            o: 0,
            s: Some(0),
            c: 10,
            min_value: None,
            max_value: None,
        };

        assert_eq!(distribution.evaluate(0).unwrap(), 11);
        assert!(distribution.evaluate(10).unwrap() > 20);
    }

    #[test]
    fn test_exponential_function_divide_by_zero() {
        let distribution = DistributionFunction::Exponential {
            a: 1,
            d: 0, // Invalid denominator
            m: 1,
            n: 1,
            o: 0,
            s: Some(0),
            c: 10,
            min_value: None,
            max_value: None,
        };

        assert!(matches!(
            distribution.evaluate(10),
            Err(ProtocolError::DivideByZero(_))
        ));
    }

    #[test]
    fn test_logarithmic_function() {
        let distribution = DistributionFunction::Logarithmic {
            a: 10,
            d: 1,
            m: 1,
            n: 1,
            o: 1, // Offset ensures (x - s + o) > 0
            s: Some(1), // Start at x=1 to avoid log(0)
            b: 5,
            min_value: None,
            max_value: None,
        };

        assert_eq!(distribution.evaluate(1).unwrap(), 5);
        assert!(distribution.evaluate(10).unwrap() > 5);
    }

    #[test]
    fn test_logarithmic_function_with_min_max_bounds() {
        let distribution = DistributionFunction::Logarithmic {
            a: 10,
            d: 1,
            m: 1,
            n: 1,
            o: 1,
            s: Some(1),
            b: 5,
            min_value: Some(7),  // Minimum bound should be enforced
            max_value: Some(20), // Maximum bound should be enforced
        };

        assert_eq!(distribution.evaluate(1).unwrap(), 7); // Clamped to min_value
        assert!(distribution.evaluate(10).unwrap() <= 20); // Should not exceed max_value
    }

    #[test]
    fn test_logarithmic_function_undefined() {
        let distribution = DistributionFunction::Logarithmic {
            a: 10,
            d: 1,
            m: 1,
            n: 1,
            o: -1, // Invalid offset causing log(0)
            s: Some(1),
            b: 5,
            min_value: None,
            max_value: None,
        };

        assert!(matches!(
        distribution.evaluate(1),
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
            s: Some(10),
            b: 10,
            min_value: None,
            max_value: None,
        };

        let result = distribution.evaluate(100);
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
            s: Some(5),
            b: 5,
            min_value: None,
            max_value: None,
        };

        assert!(matches!(
        distribution.evaluate(10),
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
            s: Some(5),
            b: 5,
            min_value: None,
            max_value: None,
        };

        assert!(matches!(
        distribution.evaluate(10),
        Err(ProtocolError::DivideByZero(_))
    ));
    }
}