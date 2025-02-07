use crate::consensus::basic::data_contract::{InvalidTokenDistributionFunctionDivideByZeroError, InvalidTokenDistributionFunctionInvalidParameterError, InvalidTokenDistributionFunctionInvalidParameterTupleError};
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::{DistributionFunction, MAX_DISTRIBUTION_PARAM};
use crate::ProtocolError;
use crate::validation::SimpleConsensusValidationResult;
impl DistributionFunction {
    pub fn validate(&self, start_moment: u64) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match self {
            DistributionFunction::FixedAmount { n } => {
                // Validate that n is > 0 and does not exceed u32::MAX.
                if *n == 0 || *n > u32::MAX as u64 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "n".to_string(),
                            1,
                            u32::MAX as i64,
                        )
                            .into(),
                    ));
                }
            }

            DistributionFunction::StepDecreasingAmount {
                step_count,
                decrease_per_interval_numerator,
                decrease_per_interval_denominator,
                s,
                n,
                min_value,
            } => {
                // Validate n.
                if *n == 0 || *n > u32::MAX as u64 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "n".to_string(),
                            1,
                            u32::MAX as i64,
                        )
                            .into(),
                    ));
                }
                if *step_count == 0 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionDivideByZeroError::new(self.clone()).into(),
                    ));
                }
                if *decrease_per_interval_denominator == 0 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionDivideByZeroError::new(self.clone()).into(),
                    ));
                }
                if *decrease_per_interval_numerator >= *decrease_per_interval_denominator {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterTupleError::new(
                            "decrease_per_interval_numerator".to_string(),
                            "decrease_per_interval_denominator".to_string(),
                            "smaller than".to_string(),
                        )
                            .into(),
                    ));
                }
                if let Some(min) = min_value {
                    if *n < *min {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterTupleError::new(
                                "n".to_string(),
                                "min_value".to_string(),
                                "greater than or equal to".to_string(),
                            )
                                .into(),
                        ));
                    }
                }

                if let Some(s) = s {
                    if *s > MAX_DISTRIBUTION_PARAM {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "s".to_string(),
                                0,
                                MAX_DISTRIBUTION_PARAM as i64,
                            )
                                .into()))
                    }
                }
            }

            DistributionFunction::Stepwise(steps) => {
                // Ensure at least two distinct steps.
                if steps.is_empty() || steps.len() == 1 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "steps".to_string(),
                            2,
                            u16::MAX as i64,
                        )
                            .into(),
                    ));
                }
            }

            DistributionFunction::Linear { a, d, s, b, min_value, max_value } => {
                if *d == 0 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionDivideByZeroError::new(self.clone()).into(),
                    ));
                }
                if let (Some(min), Some(max)) = (min_value, max_value) {
                    if min > max {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterTupleError::new(
                                "min_value".to_string(),
                                "max_value".to_string(),
                                "smaller than or equal to".to_string(),
                            )
                                .into(),
                        ));
                    }
                }

                if let Some(s) = s {
                    if *s > MAX_DISTRIBUTION_PARAM {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "s".to_string(),
                                0,
                                MAX_DISTRIBUTION_PARAM as i64,
                            )
                                .into()))
                    }
                }

                if let Some(max) = max_value {
                    if *max > MAX_DISTRIBUTION_PARAM {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "max".to_string(),
                                0,
                                MAX_DISTRIBUTION_PARAM as i64,
                            )
                                .into(),
                        ));
                    }
                }

                let start_token_amount = DistributionFunction::Linear {
                    a: *a,
                    d: *d,
                    s: Some(s.unwrap_or(start_moment)),
                    b: *b,
                    min_value: None,
                    max_value: None,
                }.evaluate(start_moment)?;
                
                // Validate that the start_token_amount is within the provided bounds (if any).
                if let Some(min) = min_value {
                    if start_token_amount < *min {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "start_token_amount".to_string(),
                                *min as i64,
                                max_value.unwrap_or(MAX_DISTRIBUTION_PARAM) as i64,
                            )
                                .into(),
                        ));
                    }
                }
                if let Some(max) = max_value {
                    if start_token_amount > *max {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "start_token_amount".to_string(),
                                0,
                                *max as i64,
                            )
                                .into(),
                        ));
                    }
                }
            }

            DistributionFunction::Polynomial { a, d, m, n, o, s, b, min_value, max_value } => {
                if *d == 0 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionDivideByZeroError::new(self.clone()).into(),
                    ));
                }
                if *n == 0 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionDivideByZeroError::new(self.clone()).into(),
                    ));
                }

                if let Some(s) = s {
                    if *s > MAX_DISTRIBUTION_PARAM {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "s".to_string(),
                                0,
                                MAX_DISTRIBUTION_PARAM as i64,
                            )
                                .into()))
                    }
                }

                if *o > MAX_DISTRIBUTION_PARAM as i64 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "o".to_string(),
                            -(MAX_DISTRIBUTION_PARAM as i64),
                            MAX_DISTRIBUTION_PARAM as i64,
                        )
                            .into()))
                }

                if *o < -(MAX_DISTRIBUTION_PARAM as i64) {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "o".to_string(),
                            -(MAX_DISTRIBUTION_PARAM as i64),
                            MAX_DISTRIBUTION_PARAM as i64,
                        )
                            .into()))
                }

                if let Some(max) = max_value {
                    if *max > MAX_DISTRIBUTION_PARAM {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "max".to_string(),
                                0,
                                MAX_DISTRIBUTION_PARAM as i64,
                            )
                                .into(),
                        ));
                    }
                }
                
                if let (Some(min), Some(max)) = (min_value, max_value) {
                    if min > max {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterTupleError::new(
                                "min_value".to_string(),
                                "max_value".to_string(),
                                "smaller than or equal to".to_string(),
                            )
                                .into(),
                        ));
                    }
                }

                let start_token_amount = DistributionFunction::Polynomial {
                    a: *a,
                    d: *d,
                    m: *m,
                    n: *n,
                    o: *o,
                    s: Some(s.unwrap_or(start_moment)),
                    b: *b,
                    min_value: None,
                    max_value: None,
                }.evaluate(start_moment)?;

                if let Some(min) = min_value {
                    if start_token_amount < *min {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "start_token_amount".to_string(),
                                *min as i64,
                                max_value.unwrap_or(MAX_DISTRIBUTION_PARAM) as i64,
                            )
                                .into(),
                        ));
                    }
                }
                if let Some(max) = max_value {
                    if start_token_amount > *max {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "start_token_amount".to_string(),
                                0,
                                *max as i64,
                            )
                                .into(),
                        ));
                    }
                }
            }

            DistributionFunction::Exponential { a, d, m, n, o, s, c, min_value, max_value } => {
                if *d == 0 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionDivideByZeroError::new(self.clone()).into(),
                    ));
                }
                if *n == 0 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionDivideByZeroError::new(self.clone()).into(),
                    ));
                }

                if let Some(s) = s {
                    if *s > MAX_DISTRIBUTION_PARAM {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "s".to_string(),
                                0,
                                MAX_DISTRIBUTION_PARAM as i64,
                            )
                                .into()))
                    }
                }

                if *o > MAX_DISTRIBUTION_PARAM as i64 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "o".to_string(),
                            -(MAX_DISTRIBUTION_PARAM as i64),
                            MAX_DISTRIBUTION_PARAM as i64,
                        )
                            .into()))
                }

                if *o < -(MAX_DISTRIBUTION_PARAM as i64) {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "o".to_string(),
                            -(MAX_DISTRIBUTION_PARAM as i64),
                            MAX_DISTRIBUTION_PARAM as i64,
                        )
                            .into()))
                }

                if let Some(max) = max_value {
                    if *max > MAX_DISTRIBUTION_PARAM {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "max".to_string(),
                                0,
                                MAX_DISTRIBUTION_PARAM as i64,
                            )
                                .into(),
                        ));
                    }
                }
                
                if let (Some(min), Some(max)) = (min_value, max_value) {
                    if min > max {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterTupleError::new(
                                "min_value".to_string(),
                                "max_value".to_string(),
                                "smaller than or equal to".to_string(),
                            )
                                .into(),
                        ));
                    }
                }

                let start_token_amount = DistributionFunction::Exponential {
                    a: *a,
                    d: *d,
                    m: *m,
                    n: *n,
                    o: *o,
                    s: Some(s.unwrap_or(start_moment)),
                    c: *c,
                    min_value: None,
                    max_value: None,
                }.evaluate(start_moment)?;

                if let Some(min) = min_value {
                    if start_token_amount < *min {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "start_token_amount".to_string(),
                                *min as i64,
                                max_value.unwrap_or(MAX_DISTRIBUTION_PARAM) as i64,
                            )
                                .into(),
                        ));
                    }
                }
                if let Some(max) = max_value {
                    if start_token_amount > *max {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "start_token_amount".to_string(),
                                0,
                                *max as i64,
                            )
                                .into(),
                        ));
                    }
                }
            }
            // f(x) = (a * log(m * (x - s + o) / n)) / d + b
            DistributionFunction::Logarithmic { a, d, m, n, o, s, b, min_value, max_value } => {
                if *d == 0 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionDivideByZeroError::new(self.clone()).into(),
                    ));
                }
                if *n == 0 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionDivideByZeroError::new(self.clone()).into(),
                    ));
                }

                if let Some(s) = s {
                    if *s > MAX_DISTRIBUTION_PARAM {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "s".to_string(),
                                0,
                                MAX_DISTRIBUTION_PARAM as i64,
                            )
                                .into()))
                    }
                }

                if let Some(max) = max_value {
                    if *max > MAX_DISTRIBUTION_PARAM {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "max".to_string(),
                                0,
                                MAX_DISTRIBUTION_PARAM as i64,
                            )
                                .into(),
                        ));
                    }
                }

                if *o > MAX_DISTRIBUTION_PARAM as i64 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "o".to_string(),
                            -(MAX_DISTRIBUTION_PARAM as i64),
                            MAX_DISTRIBUTION_PARAM as i64,
                        )
                            .into()))
                }

                if *o < -(MAX_DISTRIBUTION_PARAM as i64) {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "o".to_string(),
                            -(MAX_DISTRIBUTION_PARAM as i64),
                            MAX_DISTRIBUTION_PARAM as i64,
                        )
                            .into()))
                }
                
                if let (Some(min), Some(max)) = (min_value, max_value) {
                    if min > max {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterTupleError::new(
                                "min_value".to_string(),
                                "max_value".to_string(),
                                "smaller than or equal to".to_string(),
                            )
                                .into(),
                        ));
                    }
                }

                let eval_s = s.unwrap_or(start_moment);
                
                if start_moment as i64 - eval_s as i64 + o <= 0 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterTupleError::new(
                            "s".to_string(),
                            "o".to_string(),
                            "(x - s + o) must be bigger than 0 in f(x) = (a * log(m * (x - s + o) / n)) / d + b".to_string(),
                        )
                            .into(),
                    ));
                }

                let start_token_amount = DistributionFunction::Logarithmic {
                    a: *a,
                    d: *d,
                    m: *m,
                    n: *n,
                    o: *o,
                    s: Some(s.unwrap_or(start_moment)),
                    b: *b,
                    min_value: None,
                    max_value: None,
                }.evaluate(start_moment)?;

                if let Some(min) = min_value {
                    if start_token_amount < *min {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "start_token_amount".to_string(),
                                *min as i64,
                                max_value.unwrap_or(MAX_DISTRIBUTION_PARAM) as i64,
                            )
                                .into(),
                        ));
                    }
                }
                if let Some(max) = max_value {
                    if start_token_amount > *max {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "start_token_amount".to_string(),
                                0,
                                *max as i64,
                            )
                                .into(),
                        ));
                    }
                }
            }
        }

        Ok(SimpleConsensusValidationResult::default())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    const START_MOMENT: u64 = 4000;

    #[test]
    fn test_fixed_amount_valid() {
        let dist = DistributionFunction::FixedAmount { n: 100 };
        let result = dist.validate(START_MOMENT);
        assert!(result.expect("no error on test_fixed_amount_valid").first_error().is_none());
    }

    #[test]
    fn test_fixed_amount_zero_invalid() {
        let dist = DistributionFunction::FixedAmount { n: 0 };
        let result = dist.validate(START_MOMENT);
        assert!(result.expect("no error on test_fixed_amount_zero_invalid").first_error().is_some());
    }

    #[test]
    fn test_fixed_amount_max_valid() {
        let dist = DistributionFunction::FixedAmount { n: u32::MAX as u64 };
        let result = dist.validate(START_MOMENT);
        assert!(result.expect("no error on test_fixed_amount_max_valid").first_error().is_none());
    }

    #[test]
    fn test_fixed_amount_exceeds_max_invalid() {
        let dist = DistributionFunction::FixedAmount { n: u32::MAX as u64 + 1 };
        let result = dist.validate(START_MOMENT);
        assert!(result.expect("no error on test_fixed_amount_exceeds_max_invalid").first_error().is_some());
    }

    #[test]
    fn test_step_decreasing_amount_valid() {
        let dist = DistributionFunction::StepDecreasingAmount {
            step_count: 10,
            decrease_per_interval_numerator: 1,
            decrease_per_interval_denominator: 2,
            s: Some(0),
            n: 100,
            min_value: Some(10),
        };
        let result = dist.validate(START_MOMENT);
        assert!(result.expect("no error on test_step_decreasing_amount_valid").first_error().is_none());
    }

    #[test]
    fn test_step_decreasing_amount_invalid_zero_step_count() {
        let dist = DistributionFunction::StepDecreasingAmount {
            step_count: 0,
            decrease_per_interval_numerator: 1,
            decrease_per_interval_denominator: 2,
            s: Some(0),
            n: 100,
            min_value: Some(10),
        };
        let result = dist.validate(START_MOMENT);
        assert!(result.expect("no error on test_step_decreasing_amount_invalid_zero_step_count").first_error().is_some());
    }

    #[test]
    fn test_step_decreasing_amount_invalid_zero_denominator() {
        let dist = DistributionFunction::StepDecreasingAmount {
            step_count: 10,
            decrease_per_interval_numerator: 1,
            decrease_per_interval_denominator: 0,
            s: Some(0),
            n: 100,
            min_value: Some(10),
        };
        let result = dist.validate(START_MOMENT);
        assert!(result.expect("no error on test_step_decreasing_amount_invalid_zero_denominator").first_error().is_some());
    }

    #[test]
    fn test_stepwise_valid() {
        let mut steps = BTreeMap::new();
        steps.insert(0, 100);
        steps.insert(10, 50);
        steps.insert(20, 25);
        let dist = DistributionFunction::Stepwise(steps);
        let result = dist.validate(START_MOMENT);
        assert!(result.expect("no error on test_stepwise_valid").first_error().is_none());
    }

    #[test]
    fn test_stepwise_invalid_single_step() {
        let mut steps = BTreeMap::new();
        steps.insert(0, 100);
        let dist = DistributionFunction::Stepwise(steps);
        let result = dist.validate(START_MOMENT);
        assert!(result.expect("no error on test_stepwise_invalid_single_step").first_error().is_some());
    }

    #[test]
    fn test_linear_valid() {
        let dist = DistributionFunction::Linear {
            a: 1,
            d: 10,
            s: Some(0),
            b: 100,
            min_value: Some(50),
            max_value: Some(150),
        };
        let result = dist.validate(START_MOMENT);
        assert!(result.expect("no error on test_linear_valid").first_error().is_none());
    }

    #[test]
    fn test_linear_invalid_divide_by_zero() {
        let dist = DistributionFunction::Linear {
            a: 1,
            d: 0,
            s: Some(0),
            b: 100,
            min_value: Some(50),
            max_value: Some(150),
        };
        let result = dist.validate(START_MOMENT);
        assert!(result.expect("no error on test_linear_invalid_divide_by_zero").first_error().is_some());
    }

    #[test]
    fn test_linear_invalid_s_exceeds_max() {
        let dist = DistributionFunction::Linear {
            a: 1,
            d: 10,
            s: Some(MAX_DISTRIBUTION_PARAM + 1),
            b: 100,
            min_value: Some(50),
            max_value: Some(150),
        };
        let result = dist.validate(START_MOMENT);
        assert!(result.expect("no error on test_linear_invalid_s_exceeds_max").first_error().is_some());
    }

    #[test]
    fn test_polynomial_valid() {
        let dist = DistributionFunction::Polynomial {
            a: 2,
            d: 10,
            m: 2,
            n: 3,
            o: 0,
            s: Some(0),
            b: 5,
            min_value: Some(1),
            max_value: Some(50),
        };
        let result = dist.validate(START_MOMENT);
        assert!(result.expect("no error on test_polynomial_valid").first_error().is_none());
    }

    #[test]
    fn test_polynomial_invalid_divide_by_zero() {
        let dist = DistributionFunction::Polynomial {
            a: 2,
            d: 0,
            m: 2,
            n: 3,
            o: 0,
            s: Some(0),
            b: 5,
            min_value: Some(1),
            max_value: Some(50),
        };
        let result = dist.validate(START_MOMENT);
        assert!(result.expect("no error on test_polynomial_invalid_divide_by_zero").first_error().is_some());
    }

    #[test]
    fn test_exponential_valid() {
        let dist = DistributionFunction::Exponential {
            a: 3,
            d: 10,
            m: 1,
            n: 2,
            o: 0,
            s: Some(0),
            c: 10,
            min_value: Some(1),
            max_value: Some(100),
        };
        let result = dist.validate(START_MOMENT);
        assert!(result.expect("no error on test_exponential_valid").first_error().is_none());
    }

    #[test]
    fn test_exponential_invalid_zero_n() {
        let dist = DistributionFunction::Exponential {
            a: 3,
            d: 10,
            m: 1,
            n: 0,
            o: 1,
            s: Some(0),
            c: 10,
            min_value: Some(1),
            max_value: Some(100),
        };
        let result = dist.validate(START_MOMENT);
        assert!(result.expect("no error on test_exponential_invalid_zero_n").first_error().is_some());
    }

    #[test]
    fn test_logarithmic_valid() {
        let dist = DistributionFunction::Logarithmic {
            a: 4,
            d: 10,
            m: 1,
            n: 2,
            o: 1,
            s: None,
            b: 10,
            min_value: Some(1),
            max_value: Some(100),
        };
        let result = dist.validate(START_MOMENT);
        assert!(result.expect("no error on test_logarithmic_valid").first_error().is_none());
    }

    #[test]
    fn test_logarithmic_invalid_zero_d() {
        let dist = DistributionFunction::Logarithmic {
            a: 4,
            d: 0, // Invalid: Division by zero
            m: 1,
            n: 2,
            o: 1,
            s: Some(0),
            b: 10,
            min_value: Some(1),
            max_value: Some(100),
        };
        let result = dist.validate(START_MOMENT);
        assert!(
            result.expect("no error on test_logarithmic_invalid_zero_d").first_error().is_some(),
            "Expected division by zero error"
        );
    }

    #[test]
    fn test_logarithmic_invalid_zero_n() {
        let dist = DistributionFunction::Logarithmic {
            a: 4,
            d: 10,
            m: 1,
            n: 0, // Invalid: Division by zero in log denominator
            o: 1,
            s: Some(0),
            b: 10,
            min_value: Some(1),
            max_value: Some(100),
        };
        let result = dist.validate(START_MOMENT);
        assert!(
            result.expect("no error on test_logarithmic_invalid_zero_n").first_error().is_some(),
            "Expected division by zero error"
        );
    }

    #[test]
    fn test_logarithmic_invalid_x_s_o_non_positive() {
        let dist = DistributionFunction::Logarithmic {
            a: 4,
            d: 10,
            m: 1,
            n: 2,
            o: -5, // Causes (x - s + o) to be <= 0
            s: Some(START_MOMENT),
            b: 10,
            min_value: Some(1),
            max_value: Some(100),
        };
        let result = dist.validate(START_MOMENT);
        assert!(
            result.expect("no error on test_logarithmic_invalid_x_s_o_non_positive").first_error().is_some(),
            "Expected error: (x - s + o) must be > 0"
        );
    }

    #[test]
    fn test_logarithmic_invalid_max_greater_than_max_param() {
        let dist = DistributionFunction::Logarithmic {
            a: 4,
            d: 10,
            m: 1,
            n: 2,
            o: 1,
            s: Some(0),
            b: 10,
            min_value: Some(1),
            max_value: Some(MAX_DISTRIBUTION_PARAM + 1), // Invalid: max_value too large
        };
        let result = dist.validate(START_MOMENT);
        assert!(
            result.expect("no error on test_logarithmic_invalid_max_greater_than_max_param").first_error().is_some(),
            "Expected error: max_value exceeds allowed max distribution parameter"
        );
    }

    #[test]
    fn test_logarithmic_invalid_min_greater_than_max() {
        let dist = DistributionFunction::Logarithmic {
            a: 4,
            d: 10,
            m: 1,
            n: 2,
            o: 1,
            s: Some(0),
            b: 10,
            min_value: Some(50), // Invalid: min > max
            max_value: Some(30),
        };
        let result = dist.validate(START_MOMENT);
        assert!(
            result.expect("no error on test_logarithmic_invalid_min_greater_than_max").first_error().is_some(),
            "Expected error: min_value > max_value"
        );
    }

    #[test]
    fn test_logarithmic_valid_with_s_and_o() {
        let dist = DistributionFunction::Logarithmic {
            a: 3,
            d: 5,
            m: 2,
            n: 4,
            o: 3, // Offset ensures (x - s + o) > 0
            s: Some(START_MOMENT - 2),
            b: 8,
            min_value: Some(2),
            max_value: Some(50),
        };
        let result = dist.validate(START_MOMENT);
        assert!(result.expect("no error on test_logarithmic_valid_with_s_and_o").first_error().is_none());
    }

    #[test]
    fn test_logarithmic_valid_edge_case_max() {
        let dist = DistributionFunction::Logarithmic {
            a: 3,
            d: 5,
            m: 2,
            n: 4,
            o: 1,
            s: Some(START_MOMENT),
            b: 8,
            min_value: Some(2),
            max_value: Some(MAX_DISTRIBUTION_PARAM), // Valid max
        };
        let result = dist.validate(START_MOMENT);
        assert!(result.expect("no error on test_logarithmic_valid_edge_case_max").first_error().is_none());
    }
}