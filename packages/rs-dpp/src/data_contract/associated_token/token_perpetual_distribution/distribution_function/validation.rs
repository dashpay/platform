use crate::consensus::basic::data_contract::{
    InvalidTokenDistributionFunctionDivideByZeroError,
    InvalidTokenDistributionFunctionIncoherenceError,
    InvalidTokenDistributionFunctionInvalidParameterError,
    InvalidTokenDistributionFunctionInvalidParameterTupleError,
};
use crate::consensus::basic::UnsupportedFeatureError;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::{
    DistributionFunction, MAX_DISTRIBUTION_PARAM, MAX_EXP_A_PARAM, MAX_EXP_M_PARAM,
    MAX_EXP_N_PARAM, MAX_LINEAR_SLOPE_A_PARAM, MAX_LOG_A_PARAM, MAX_POL_M_PARAM, MAX_POL_N_PARAM,
    MIN_EXP_M_PARAM, MIN_LINEAR_SLOPE_A_PARAM, MIN_LOG_A_PARAM, MIN_POL_M_PARAM,
};
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
impl DistributionFunction {
    pub fn validate(
        &self,
        start_moment: u64,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match self {
            DistributionFunction::FixedAmount { amount: n } => {
                // Validate that n is > 0 and does not exceed u32::MAX.
                if *n == 0 || *n > MAX_DISTRIBUTION_PARAM {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "n".to_string(),
                            1,
                            MAX_DISTRIBUTION_PARAM as i64,
                            None,
                        )
                        .into(),
                    ));
                }
            }
            DistributionFunction::Random { .. } => {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    UnsupportedFeatureError::new(
                        "token random distribution".to_string(),
                        platform_version.protocol_version,
                    )
                    .into(),
                ));
                // Ensure that `min` is not greater than `max`
                // if *min > *max {
                //     return Ok(SimpleConsensusValidationResult::new_with_error(
                //         InvalidTokenDistributionFunctionInvalidParameterTupleError::new(
                //             "min".to_string(),
                //             "max".to_string(),
                //             "smaller than or equal to".to_string(),
                //         )
                //         .into(),
                //     ));
                // }
                //
                // // Ensure that `max` is within valid bounds
                // if *max > MAX_DISTRIBUTION_PARAM {
                //     return Ok(SimpleConsensusValidationResult::new_with_error(
                //         InvalidTokenDistributionFunctionInvalidParameterError::new(
                //             "max".to_string(),
                //             0,
                //             MAX_DISTRIBUTION_PARAM as i64,
                //             None,
                //         )
                //         .into(),
                //     ));
                // }
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
                // Validate n.
                if *distribution_start_amount == 0
                    || *distribution_start_amount > MAX_DISTRIBUTION_PARAM
                {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "n".to_string(),
                            1,
                            MAX_DISTRIBUTION_PARAM as i64,
                            None,
                        )
                        .into(),
                    ));
                }

                // Ensure trailing amount does not exceed the initial amount
                if *trailing_distribution_interval_amount > *distribution_start_amount {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterTupleError::new(
                            "trailing_distribution_interval_amount".to_string(),
                            "distribution_start_amount".to_string(),
                            "smaller than or equal to".to_string(),
                        )
                        .into(),
                    ));
                }
                if let Some(max_interval_count) = max_interval_count {
                    if *max_interval_count < 2 || *max_interval_count > 1024 {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "max_interval_count".to_string(),
                                2,
                                1024,
                                None,
                            )
                            .into(),
                        ));
                    }
                }
                if *step_count == 0 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionDivideByZeroError::new(self.clone()).into(),
                    ));
                }
                if *decrease_per_interval_numerator == 0 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "decrease_per_interval_numerator".to_string(),
                            1,
                            u16::MAX as i64,
                            None,
                        )
                        .into(),
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
                    if *distribution_start_amount < *min {
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

                if let Some(s) = start_decreasing_offset {
                    if *s > MAX_DISTRIBUTION_PARAM {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "s".to_string(),
                                0,
                                MAX_DISTRIBUTION_PARAM as i64,
                                None,
                            )
                            .into(),
                        ));
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
                            None,
                        )
                        .into(),
                    ));
                }
            }
            // f(x) = (a * (x - s) / d) + b
            DistributionFunction::Linear {
                a,
                d,
                start_step: s,
                starting_amount,
                min_value,
                max_value,
            } => {
                if *d == 0 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionDivideByZeroError::new(self.clone()).into(),
                    ));
                }
                if *a == 0 || *a > MAX_LINEAR_SLOPE_A_PARAM as i64 || *a < MIN_LINEAR_SLOPE_A_PARAM
                {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "a".to_string(),
                            MIN_LINEAR_SLOPE_A_PARAM,
                            MAX_LINEAR_SLOPE_A_PARAM as i64,
                            Some(0),
                        )
                        .into(),
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
                                None,
                            )
                            .into(),
                        ));
                    }
                }

                if let Some(max) = max_value {
                    if *max > MAX_DISTRIBUTION_PARAM {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "max".to_string(),
                                0,
                                MAX_DISTRIBUTION_PARAM as i64,
                                None,
                            )
                            .into(),
                        ));
                    }
                }

                let start_token_amount = DistributionFunction::Linear {
                    a: *a,
                    d: *d,
                    start_step: Some(s.unwrap_or(start_moment)),
                    starting_amount: *starting_amount,
                    min_value: *min_value,
                    max_value: *max_value,
                }
                .evaluate(0, start_moment)?;

                if *a > 0 {
                    // we want to put in the max value to see if we are starting off at the max
                    // value.
                    // if we are starting at the max value there's no point at doing a linear function
                    if let Some(max) = max_value {
                        if start_token_amount == *max {
                            return Ok(SimpleConsensusValidationResult::new_with_error(
                                InvalidTokenDistributionFunctionIncoherenceError::new(
                                    "since a is positive the linear function will increase, however it starts at the maximum value already which makes the function never used".to_string(),
                                )
                                    .into(),
                            ));
                        }
                    }
                    start_token_amount
                } else {
                    if let Some(min) = min_value {
                        if start_token_amount == *min {
                            return Ok(SimpleConsensusValidationResult::new_with_error(
                                InvalidTokenDistributionFunctionIncoherenceError::new(
                                    "since a is negative the linear function will decrease, however it starts at the minimum value which makes the function never used".to_string(),
                                )
                                    .into(),
                            ));
                        }
                    }
                    start_token_amount
                };
            }

            // f(x) = (a * (x - s + o)^(m/n)) / d + b
            DistributionFunction::Polynomial {
                a,
                d,
                m,
                n,
                o,
                start_moment: s,
                b,
                min_value,
                max_value,
            } => {
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

                if *m > 0 && *n == m.unsigned_abs() {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterTupleError::new(
                            "m".to_string(),
                            "n".to_string(),
                            "different than".to_string(),
                        )
                        .into(),
                    ));
                }

                if *a == 0 || *a < MIN_LOG_A_PARAM || *a > MAX_LOG_A_PARAM {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "a".to_string(),
                            MIN_LOG_A_PARAM,
                            MAX_LOG_A_PARAM,
                            Some(0),
                        )
                        .into(),
                    ));
                }

                if *m == 0 || *m < MIN_POL_M_PARAM || *m > MAX_POL_M_PARAM {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "m".to_string(),
                            MIN_POL_M_PARAM,
                            MAX_POL_M_PARAM,
                            Some(0),
                        )
                        .into(),
                    ));
                }

                if *n > MAX_POL_N_PARAM {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "n".to_string(),
                            1,
                            MAX_POL_N_PARAM as i64,
                            None,
                        )
                        .into(),
                    ));
                }

                if let Some(s) = s {
                    if *s > MAX_DISTRIBUTION_PARAM {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "s".to_string(),
                                0,
                                MAX_DISTRIBUTION_PARAM as i64,
                                None,
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
                            None,
                        )
                        .into(),
                    ));
                }

                if *o < -(MAX_DISTRIBUTION_PARAM as i64) {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "o".to_string(),
                            -(MAX_DISTRIBUTION_PARAM as i64),
                            MAX_DISTRIBUTION_PARAM as i64,
                            None,
                        )
                        .into(),
                    ));
                }

                if let Some(max) = max_value {
                    if *max > MAX_DISTRIBUTION_PARAM {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "max".to_string(),
                                0,
                                MAX_DISTRIBUTION_PARAM as i64,
                                None,
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
                    start_moment: Some(s.unwrap_or(start_moment)),
                    b: *b,
                    min_value: *min_value,
                    max_value: *max_value,
                }
                .evaluate(0, start_moment)?;

                // Now, based on the monotonicity implied by (*a) * (*m),
                // check for incoherence:
                #[allow(clippy::comparison_chain)]
                if (*a) * (*m) > 0 {
                    // The function is increasing.
                    if let Some(max) = max_value {
                        if start_token_amount == *max {
                            return Ok(SimpleConsensusValidationResult::new_with_error(
                                InvalidTokenDistributionFunctionIncoherenceError::new(
                                    "Since a and m imply an increasing function, but the start amount is already at the maximum, the function would never produce a higher value."
                                        .to_string(),
                                ).into(),
                            ));
                        }
                    }
                } else if (*a) * (*m) < 0 {
                    // The function is decreasing.
                    if let Some(min) = min_value {
                        if start_token_amount == *min {
                            return Ok(SimpleConsensusValidationResult::new_with_error(
                                InvalidTokenDistributionFunctionIncoherenceError::new(
                                    "Since a and m imply a decreasing function, but the start amount is already at the minimum, the function would never produce a lower value."
                                        .to_string(),
                                ).into(),
                            ));
                        }
                    }
                }
            }
            // f(x) = (a * e^(m * (x - s + o) / n)) / d + b
            DistributionFunction::Exponential {
                a,
                d,
                m,
                n,
                o,
                start_moment: s,
                b,
                min_value,
                max_value,
            } => {
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
                if *n > MAX_EXP_N_PARAM {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "n".to_string(),
                            1,
                            MAX_EXP_N_PARAM as i64,
                            None,
                        )
                        .into(),
                    ));
                }
                if *m == 0 || *m > MAX_EXP_M_PARAM as i64 || *m < MIN_EXP_M_PARAM {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "m".to_string(),
                            MIN_EXP_M_PARAM,
                            MAX_EXP_M_PARAM as i64,
                            Some(0),
                        )
                        .into(),
                    ));
                }
                // Check valid a values
                if *a == 0 || *a > MAX_EXP_A_PARAM {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "a".to_string(),
                            1,
                            MAX_EXP_A_PARAM as i64,
                            None,
                        )
                        .into(),
                    ));
                }

                if *m > 0 {
                    // m is positive means that we need a max value set
                    if max_value.is_none() {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterTupleError::new(
                                "max_value".to_string(),
                                "m".to_string(),
                                "set if the following parameter is positive".to_string(),
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
                                None,
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
                            None,
                        )
                        .into(),
                    ));
                }

                if *b > MAX_DISTRIBUTION_PARAM {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "b".to_string(),
                            0,
                            MAX_DISTRIBUTION_PARAM as i64,
                            None,
                        )
                        .into(),
                    ));
                }

                if *o < -(MAX_DISTRIBUTION_PARAM as i64) {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "o".to_string(),
                            -(MAX_DISTRIBUTION_PARAM as i64),
                            MAX_DISTRIBUTION_PARAM as i64,
                            None,
                        )
                        .into(),
                    ));
                }

                if let Some(max) = max_value {
                    if *max > MAX_DISTRIBUTION_PARAM {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "max".to_string(),
                                0,
                                MAX_DISTRIBUTION_PARAM as i64,
                                None,
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
                    start_moment: Some(s.unwrap_or(start_moment)),
                    b: *b,
                    min_value: *min_value,
                    max_value: *max_value,
                }
                .evaluate(0, start_moment)?;

                if *m > 0 {
                    // we want to put in the max value to see if we are starting off at the max
                    // value.
                    // if we are starting at the max value there's no point at doing an exp
                    if let Some(max) = max_value {
                        if start_token_amount == *max {
                            return Ok(SimpleConsensusValidationResult::new_with_error(
                                InvalidTokenDistributionFunctionIncoherenceError::new(
                                    "since m is positive the exponential function will increase, however it starts at the maximum value already which makes the function never used".to_string(),
                                )
                                    .into(),
                            ));
                        }
                    }
                    start_token_amount
                } else {
                    if let Some(min) = min_value {
                        if start_token_amount == *min {
                            return Ok(SimpleConsensusValidationResult::new_with_error(
                                InvalidTokenDistributionFunctionIncoherenceError::new(
                                    "since m is negative the exponential function will decrease, however it starts at the minimum value which makes the function never used".to_string(),
                                )
                                    .into(),
                            ));
                        }
                    }
                    start_token_amount
                };
            }
            // f(x) = (a * ln(m * (x - s + o) / n)) / d + b
            DistributionFunction::Logarithmic {
                a,
                d,
                m,
                n,
                o,
                start_moment: s,
                b,
                min_value,
                max_value,
            } => {
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
                if *m == 0 || *m > MAX_DISTRIBUTION_PARAM {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "m".to_string(),
                            1,
                            MAX_DISTRIBUTION_PARAM as i64,
                            None,
                        )
                        .into(),
                    ));
                }
                // Check valid a values
                if *a == 0 || *a < MIN_LOG_A_PARAM || *a > MAX_LOG_A_PARAM {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "a".to_string(),
                            MIN_LOG_A_PARAM,
                            MAX_LOG_A_PARAM,
                            Some(0),
                        )
                        .into(),
                    ));
                }

                if *b > MAX_DISTRIBUTION_PARAM {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "b".to_string(),
                            0,
                            MAX_DISTRIBUTION_PARAM as i64,
                            None,
                        )
                        .into(),
                    ));
                }

                if let Some(s) = s {
                    if *s > MAX_DISTRIBUTION_PARAM {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "s".to_string(),
                                0,
                                MAX_DISTRIBUTION_PARAM as i64,
                                None,
                            )
                            .into(),
                        ));
                    }
                }

                if let Some(max) = max_value {
                    if *max > MAX_DISTRIBUTION_PARAM {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "max".to_string(),
                                0,
                                MAX_DISTRIBUTION_PARAM as i64,
                                None,
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
                            None,
                        )
                        .into(),
                    ));
                }

                if *o < -(MAX_DISTRIBUTION_PARAM as i64) {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "o".to_string(),
                            -(MAX_DISTRIBUTION_PARAM as i64),
                            MAX_DISTRIBUTION_PARAM as i64,
                            None,
                        )
                        .into(),
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
                    start_moment: Some(s.unwrap_or(start_moment)),
                    b: *b,
                    min_value: *min_value,
                    max_value: *max_value,
                }
                .evaluate(0, start_moment)?;

                if let Some(max) = max_value {
                    if start_token_amount == *max {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionIncoherenceError::new(
                                "The log function will always increase, however it starts at the maximum value already which makes the function never used".to_string(),
                            )
                                .into(),
                        ));
                    }
                }
            }
            // f(x) = (a * log( n / (m * (x - s + o)) )) / d + b
            DistributionFunction::InvertedLogarithmic {
                a,
                d,
                m,
                n,
                o,
                start_moment: s,
                b,
                min_value,
                max_value,
            } => {
                // Check valid a values
                if *a == 0 || *a < MIN_LOG_A_PARAM || *a > MAX_LOG_A_PARAM {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "a".to_string(),
                            MIN_LOG_A_PARAM,
                            MAX_LOG_A_PARAM,
                            Some(0),
                        )
                        .into(),
                    ));
                }

                if *b > MAX_DISTRIBUTION_PARAM {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "b".to_string(),
                            0,
                            MAX_DISTRIBUTION_PARAM as i64,
                            None,
                        )
                        .into(),
                    ));
                }

                // Check for division by zero.
                if *d == 0 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionDivideByZeroError::new(self.clone()).into(),
                    ));
                }
                if *n == 0 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "n".to_string(),
                            1,
                            MAX_DISTRIBUTION_PARAM as i64,
                            None,
                        )
                        .into(),
                    ));
                }
                if *m == 0 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionDivideByZeroError::new(self.clone()).into(),
                    ));
                }

                // Validate s: if provided, it must not exceed MAX_DISTRIBUTION_PARAM.
                if let Some(s_val) = s {
                    if *s_val > MAX_DISTRIBUTION_PARAM {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "s".to_string(),
                                0,
                                MAX_DISTRIBUTION_PARAM as i64,
                                None,
                            )
                            .into(),
                        ));
                    }
                }
                // Validate o is within allowed bounds.
                if *o > MAX_DISTRIBUTION_PARAM as i64 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "o".to_string(),
                            -(MAX_DISTRIBUTION_PARAM as i64),
                            MAX_DISTRIBUTION_PARAM as i64,
                            None,
                        )
                        .into(),
                    ));
                }
                if *o < -(MAX_DISTRIBUTION_PARAM as i64) {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterError::new(
                            "o".to_string(),
                            -(MAX_DISTRIBUTION_PARAM as i64),
                            MAX_DISTRIBUTION_PARAM as i64,
                            None,
                        )
                        .into(),
                    ));
                }
                // Validate max_value if provided.
                if let Some(max) = max_value {
                    if *max > MAX_DISTRIBUTION_PARAM {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidTokenDistributionFunctionInvalidParameterError::new(
                                "max".to_string(),
                                0,
                                MAX_DISTRIBUTION_PARAM as i64,
                                None,
                            )
                            .into(),
                        ));
                    }
                }
                // If both min_value and max_value are provided, ensure min_value <= max_value.
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

                // Use the provided s or default to start_moment.
                let start_s = s.unwrap_or(start_moment);
                // Ensure the argument for the logarithm is > 0:
                if (start_moment as i64 - start_s as i64 + *o) <= 0 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidTokenDistributionFunctionInvalidParameterTupleError::new(
                            "s".to_string(),
                            "o".to_string(),
                            "(x - s + o) must be > 0 in f(x) = (a * ln(n / (m * (x - s + o)))) / d + b".to_string(),
                        )
                            .into(),
                    ));
                }

                // Evaluate the function at the starting moment.
                let start_token_amount = DistributionFunction::InvertedLogarithmic {
                    a: *a,
                    d: *d,
                    m: *m,
                    n: *n,
                    o: *o,
                    start_moment: Some(start_s),
                    b: *b,
                    min_value: *min_value,
                    max_value: *max_value,
                }
                .evaluate(0, start_moment)?;

                // Determine the function's monotonicity.
                // For InvertedLogarithmic, f'(x) = -a / (d * (x - s + o)).
                // Hence, if a > 0, the function is decreasing;
                // if a < 0, the function is increasing.
                #[allow(clippy::comparison_chain)]
                if *a > 0 {
                    // For a decreasing function, if the start amount is already at min_value,
                    // the function would never decrease further.
                    if let Some(min) = min_value {
                        if start_token_amount == *min {
                            return Ok(SimpleConsensusValidationResult::new_with_error(
                                InvalidTokenDistributionFunctionIncoherenceError::new(
                                    "Since a is positive, the inverted logarithmic function is decreasing, but it starts at the minimum value already, so it will never produce a lower value.".to_string(),
                                )
                                    .into(),
                            ));
                        }
                    }
                } else if *a < 0 {
                    // For an increasing function, if the start amount is already at max_value,
                    // the function would never increase further.
                    if let Some(max) = max_value {
                        if start_token_amount == *max {
                            return Ok(SimpleConsensusValidationResult::new_with_error(
                                InvalidTokenDistributionFunctionIncoherenceError::new(
                                    "Since a is negative, the inverted logarithmic function is increasing, but it starts at the maximum value already, so it will never produce a higher value.".to_string(),
                                )
                                    .into(),
                            ));
                        }
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
    mod fixed_amount {
        use super::*;
        #[test]
        fn test_fixed_amount_valid() {
            let dist = DistributionFunction::FixedAmount { amount: 100 };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(result
                .expect("no error on test_fixed_amount_valid")
                .first_error()
                .is_none());
        }

        #[test]
        fn test_fixed_amount_zero_invalid() {
            let dist = DistributionFunction::FixedAmount { amount: 0 };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(result
                .expect("no error on test_fixed_amount_zero_invalid")
                .first_error()
                .is_some());
        }

        #[test]
        fn test_fixed_amount_max_valid() {
            let dist = DistributionFunction::FixedAmount {
                amount: u32::MAX as u64,
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(result
                .expect("no error on test_fixed_amount_max_valid")
                .first_error()
                .is_none());
        }

        #[test]
        fn test_fixed_amount_exceeds_max_invalid() {
            let dist = DistributionFunction::FixedAmount {
                amount: MAX_DISTRIBUTION_PARAM + 1,
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(result
                .expect("no error on test_fixed_amount_exceeds_max_invalid")
                .first_error()
                .is_some());
        }
    }
    mod step_decreasing {
        use super::*;

        #[test]
        fn test_step_decreasing_amount_valid() {
            let dist = DistributionFunction::StepDecreasingAmount {
                step_count: 10,
                decrease_per_interval_numerator: 1,
                decrease_per_interval_denominator: 2,
                start_decreasing_offset: Some(0),
                max_interval_count: None,
                distribution_start_amount: 100,
                trailing_distribution_interval_amount: 0,
                min_value: Some(10),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(result
                .expect("no error on test_step_decreasing_amount_valid")
                .first_error()
                .is_none());
        }

        #[test]
        fn test_step_decreasing_amount_invalid_zero_step_count() {
            let dist = DistributionFunction::StepDecreasingAmount {
                step_count: 0,
                decrease_per_interval_numerator: 1,
                decrease_per_interval_denominator: 2,
                start_decreasing_offset: Some(0),
                max_interval_count: None,
                distribution_start_amount: 100,
                trailing_distribution_interval_amount: 0,
                min_value: Some(10),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(result
                .expect("no error on test_step_decreasing_amount_invalid_zero_step_count")
                .first_error()
                .is_some());
        }

        #[test]
        fn test_step_decreasing_amount_invalid_zero_denominator() {
            let dist = DistributionFunction::StepDecreasingAmount {
                step_count: 10,
                decrease_per_interval_numerator: 1,
                decrease_per_interval_denominator: 0,
                start_decreasing_offset: Some(0),
                max_interval_count: None,
                distribution_start_amount: 100,
                trailing_distribution_interval_amount: 0,
                min_value: Some(10),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(result
                .expect("no error on test_step_decreasing_amount_invalid_zero_denominator")
                .first_error()
                .is_some());
        }
    }
    mod stepwise {
        use super::*;
        #[test]
        fn test_stepwise_valid() {
            let mut steps = BTreeMap::new();
            steps.insert(0, 100);
            steps.insert(10, 50);
            steps.insert(20, 25);
            let dist = DistributionFunction::Stepwise(steps);
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(result
                .expect("no error on test_stepwise_valid")
                .first_error()
                .is_none());
        }

        #[test]
        fn test_stepwise_invalid_single_step() {
            let mut steps = BTreeMap::new();
            steps.insert(0, 100);
            let dist = DistributionFunction::Stepwise(steps);
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(result
                .expect("no error on test_stepwise_invalid_single_step")
                .first_error()
                .is_some());
        }
    }
    mod linear {
        use super::*;
        #[test]
        fn test_linear_valid() {
            let dist = DistributionFunction::Linear {
                a: 1,
                d: 10,
                start_step: Some(3800),
                starting_amount: 100,
                min_value: Some(50),
                max_value: Some(150),
            };

            let result = dist.validate(START_MOMENT, PlatformVersion::latest());

            // If the test fails, print the exact error message.
            if let Err(err) = &result {
                panic!("Test failed: Expected no error, but got: {:?}", err);
            }

            // If validation succeeds but contains errors, print those errors.
            if let Some(error) = result.expect("no error on test_linear_valid").first_error() {
                panic!("Test failed: Validation error found: {:?}", error);
            }
        }
        #[test]
        fn test_linear_invalid_divide_by_zero() {
            let dist = DistributionFunction::Linear {
                a: 1,
                d: 0,
                start_step: Some(0),
                starting_amount: 100,
                min_value: Some(50),
                max_value: Some(150),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(result
                .expect("no error on test_linear_invalid_divide_by_zero")
                .first_error()
                .is_some());
        }

        #[test]
        fn test_linear_invalid_s_exceeds_max() {
            let dist = DistributionFunction::Linear {
                a: 1,
                d: 10,
                start_step: Some(MAX_DISTRIBUTION_PARAM + 1),
                starting_amount: 100,
                min_value: Some(50),
                max_value: Some(150),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(result
                .expect("no error on test_linear_invalid_s_exceeds_max")
                .first_error()
                .is_some());
        }

        #[test]
        fn test_linear_invalid_a_zero() {
            let dist = DistributionFunction::Linear {
                a: 0, // Invalid: a cannot be zero
                d: 10,
                start_step: Some(0),
                starting_amount: 100,
                min_value: Some(50),
                max_value: Some(150),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_linear_invalid_a_zero")
                    .first_error()
                    .is_some(),
                "Expected error: a cannot be zero"
            );
        }

        #[test]
        fn test_linear_invalid_a_too_large() {
            let dist = DistributionFunction::Linear {
                a: MAX_DISTRIBUTION_PARAM as i64 + 1, // Invalid: a exceeds allowed range
                d: 10,
                start_step: Some(0),
                starting_amount: 100,
                min_value: Some(50),
                max_value: Some(150),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_linear_invalid_a_too_large")
                    .first_error()
                    .is_some(),
                "Expected error: a exceeds MAX_DISTRIBUTION_PARAM"
            );
        }

        #[test]
        fn test_linear_invalid_min_greater_than_max() {
            let dist = DistributionFunction::Linear {
                a: 1,
                d: 10,
                start_step: Some(0),
                starting_amount: 100,
                min_value: Some(200), // Invalid: min > max
                max_value: Some(150),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_linear_invalid_min_greater_than_max")
                    .first_error()
                    .is_some(),
                "Expected error: min_value > max_value"
            );
        }

        #[test]
        fn test_linear_invalid_s_greater_than_max() {
            let dist = DistributionFunction::Linear {
                a: 1,
                d: 10,
                start_step: Some(MAX_DISTRIBUTION_PARAM + 1), // Invalid: s exceeds allowed range
                starting_amount: 100,
                min_value: Some(50),
                max_value: Some(150),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_linear_invalid_s_greater_than_max")
                    .first_error()
                    .is_some(),
                "Expected error: s exceeds MAX_DISTRIBUTION_PARAM"
            );
        }

        #[test]
        fn test_linear_invalid_max_exceeds_max_distribution_param() {
            let dist = DistributionFunction::Linear {
                a: 1,
                d: 10,
                start_step: Some(0),
                starting_amount: 100,
                min_value: Some(50),
                max_value: Some(MAX_DISTRIBUTION_PARAM + 1), // Invalid: max_value exceeds max allowed range
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_linear_invalid_max_exceeds_max_distribution_param")
                    .first_error()
                    .is_some(),
                "Expected error: max_value exceeds MAX_DISTRIBUTION_PARAM"
            );
        }

        #[test]
        fn test_linear_invalid_starting_at_max_value() {
            let dist = DistributionFunction::Linear {
                a: 1,
                d: 10,
                start_step: Some(0),
                starting_amount: 150, // Starts at max value
                min_value: Some(50),
                max_value: Some(150),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_linear_invalid_starting_at_max_value")
                    .first_error()
                    .is_some(),
                "Expected error: function starts at max_value and cannot increase"
            );
        }

        #[test]
        fn test_linear_invalid_starting_at_min_value() {
            let dist = DistributionFunction::Linear {
                a: -1, // Negative slope (decreasing function)
                d: 10,
                start_step: Some(0),
                starting_amount: 50, // Starts at min value
                min_value: Some(50),
                max_value: Some(150),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_linear_invalid_starting_at_min_value")
                    .first_error()
                    .is_some(),
                "Expected error: function starts at min_value and cannot decrease"
            );
        }

        #[test]
        fn test_linear_valid_with_negative_a() {
            let dist = DistributionFunction::Linear {
                a: -5, // Valid decreasing function
                d: 10,
                start_step: Some(START_MOMENT),
                starting_amount: 200,
                min_value: Some(50),
                max_value: Some(250),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());

            match result {
                Ok(validation_result) => {
                    if let Some(error) = validation_result.first_error() {
                        panic!(
                            "Test failed: Expected no error, but got validation error: {:?}",
                            error
                        );
                    }
                }
                Err(protocol_error) => {
                    panic!(
                        "Test failed: Expected validation success, but got ProtocolError: {:?}",
                        protocol_error
                    );
                }
            }
        }

        #[test]
        fn test_linear_valid_with_min_boundary() {
            let dist = DistributionFunction::Linear {
                a: -3,
                d: 5,
                start_step: Some(START_MOMENT),
                starting_amount: 100,
                min_value: Some(10), // Valid min boundary
                max_value: Some(150),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(result
                .expect("no error on test_linear_valid_with_min_boundary")
                .first_error()
                .is_none());
        }

        #[test]
        fn test_linear_valid_with_max_boundary() {
            let dist = DistributionFunction::Linear {
                a: 3,
                d: 5,
                start_step: Some(START_MOMENT),
                starting_amount: 50,
                min_value: Some(10),
                max_value: Some(MAX_DISTRIBUTION_PARAM), // Valid max boundary
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(result
                .expect("no error on test_linear_valid_with_max_boundary")
                .first_error()
                .is_none());
        }
    }
    mod polynomial {
        use super::*;
        #[test]
        fn test_polynomial_valid() {
            // f(x) = (2 * x^(2/3)) / 10 + 5
            let dist = DistributionFunction::Polynomial {
                a: 2,
                d: 10,
                m: 2,
                n: 3,
                o: 0,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(80),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());

            match &result {
                Ok(validation_result) => {
                    if let Some(error) = validation_result.first_error() {
                        panic!("Test failed: validation error found: {:?}", error);
                    }
                }
                Err(protocol_error) => {
                    panic!("Test failed: ProtocolError: {:?}", protocol_error);
                }
            }
        }

        #[test]
        fn test_polynomial_invalid_zero_a() {
            let dist = DistributionFunction::Polynomial {
                a: 0,
                d: 1,
                m: 2,
                n: 3,
                o: 0,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(50),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_exponential_invalid_zero_a")
                    .first_error()
                    .is_some(),
                "Expected error: a cannot be zero"
            );
        }

        #[test]
        fn test_polynomial_invalid_divide_by_zero() {
            let dist = DistributionFunction::Polynomial {
                a: 2,
                d: 0,
                m: 2,
                n: 3,
                o: 0,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(50),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(result
                .expect("no error on test_polynomial_invalid_divide_by_zero")
                .first_error()
                .is_some());
        }

        // 1. Test invalid: n is zero (division by zero in exponent)
        #[test]
        fn test_polynomial_invalid_n_zero() {
            let dist = DistributionFunction::Polynomial {
                a: 2,
                d: 10,
                m: 2,
                n: 0, // Invalid: n == 0
                o: 0,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(50),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected error").first_error().is_some(),
                "Expected an error when n is zero"
            );
        }

        // 2. Test invalid: shift parameter s exceeds MAX_DISTRIBUTION_PARAM.
        #[test]
        fn test_polynomial_invalid_s_exceeds_max() {
            let dist = DistributionFunction::Polynomial {
                a: 2,
                d: 10,
                m: 2,
                n: 3,
                o: 0,
                start_moment: Some(MAX_DISTRIBUTION_PARAM + 1), // Invalid: s too large
                b: 5,
                min_value: Some(1),
                max_value: Some(50),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected error").first_error().is_some(),
                "Expected an error when s exceeds MAX_DISTRIBUTION_PARAM"
            );
        }

        // 3. Test invalid: offset o is too high.
        #[test]
        fn test_polynomial_invalid_o_too_high() {
            let dist = DistributionFunction::Polynomial {
                a: 2,
                d: 10,
                m: 2,
                n: 3,
                o: MAX_DISTRIBUTION_PARAM as i64 + 1, // Invalid: o too high
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(50),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected error").first_error().is_some(),
                "Expected an error when o is above the allowed maximum"
            );
        }

        // 4. Test invalid: offset o is too low.
        #[test]
        fn test_polynomial_invalid_o_too_low() {
            let dist = DistributionFunction::Polynomial {
                a: 2,
                d: 10,
                m: 2,
                n: 3,
                o: -(MAX_DISTRIBUTION_PARAM as i64) - 1, // Invalid: o too low
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(50),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected error").first_error().is_some(),
                "Expected an error when o is below the allowed minimum"
            );
        }

        #[test]
        fn test_polynomial_invalid_a_below_min() {
            let dist = DistributionFunction::Polynomial {
                a: MIN_LOG_A_PARAM - 1,
                d: 1,
                m: 2,
                n: 3,
                o: 0,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(50),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected result").first_error().is_some(),
                "Expected error: a is below minimum"
            );
        }

        #[test]
        fn test_polynomial_invalid_m_equal_n() {
            let dist = DistributionFunction::Polynomial {
                a: 1,
                d: 1,
                m: 3,
                n: 3,
                o: 0,
                start_moment: Some(0),
                b: 5,
                min_value: None,
                max_value: None,
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected result").first_error().is_some(),
                "Expected error: a is below minimum"
            );
        }

        #[test]
        fn test_polynomial_invalid_a_above_max() {
            let dist = DistributionFunction::Polynomial {
                a: MAX_LOG_A_PARAM + 1,
                d: 1,
                m: 2,
                n: 3,
                o: 0,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(50),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected result").first_error().is_some(),
                "Expected error: a is above maximum"
            );
        }

        #[test]
        fn test_polynomial_invalid_m_below_min() {
            let dist = DistributionFunction::Polynomial {
                a: 2,
                d: 1,
                m: MIN_POL_M_PARAM - 1,
                n: 3,
                o: 0,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(50),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected result").first_error().is_some(),
                "Expected error: m is below minimum"
            );
        }

        #[test]
        fn test_polynomial_invalid_m_above_max() {
            let dist = DistributionFunction::Polynomial {
                a: 2,
                d: 1,
                m: MAX_POL_M_PARAM + 1,
                n: 3,
                o: 0,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(50),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected result").first_error().is_some(),
                "Expected error: m is above maximum"
            );
        }

        #[test]
        fn test_polynomial_invalid_n_above_max() {
            let dist = DistributionFunction::Polynomial {
                a: 2,
                d: 1,
                m: 3,
                n: MAX_POL_N_PARAM + 1,
                o: 0,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(50),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected result").first_error().is_some(),
                "Expected error: n is above maximum"
            );
        }

        // 5. Test invalid: max_value exceeds MAX_DISTRIBUTION_PARAM.
        #[test]
        fn test_polynomial_invalid_max_exceeds_max_distribution() {
            let dist = DistributionFunction::Polynomial {
                a: 2,
                d: 10,
                m: 2,
                n: 3,
                o: 0,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(MAX_DISTRIBUTION_PARAM + 1), // Invalid: max_value too high
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected error").first_error().is_some(),
                "Expected an error when max_value exceeds MAX_DISTRIBUTION_PARAM"
            );
        }

        // 6. Test invalid: min_value is greater than max_value.
        #[test]
        fn test_polynomial_invalid_min_greater_than_max() {
            let dist = DistributionFunction::Polynomial {
                a: 2,
                d: 10,
                m: 2,
                n: 3,
                o: 0,
                start_moment: Some(0),
                b: 5,
                min_value: Some(60), // min_value > max_value
                max_value: Some(50),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected error").first_error().is_some(),
                "Expected an error when min_value is greater than max_value"
            );
        }

        // 7. Test invalid: For an increasing polynomial function, the starting value equals max_value.
        #[test]
        fn test_polynomial_invalid_starting_at_max_for_increasing() {
            // For an increasing function (a > 0, m > 0) evaluated at x = s,
            // the result is b. If b equals max_value, then the function starts at the maximum.
            let dist = DistributionFunction::Polynomial {
                a: 2, // positive
                d: 10,
                m: 2, // positive
                n: 1,
                o: 0,
                start_moment: Some(0),
                b: 100, // f(0) = 100
                min_value: Some(1),
                max_value: Some(100), // Starting at max_value
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected error").first_error().is_some(),
                "Expected an incoherence error when an increasing function starts at max_value"
            );
        }

        // 8. Test invalid: For a decreasing polynomial function, the starting value equals min_value.
        #[test]
        fn test_polynomial_invalid_starting_at_min_for_decreasing() {
            // For a decreasing function (a < 0, m > 0 so that a*m < 0),
            // evaluated at x = s, the result is b. If b equals min_value, then it's invalid.
            let dist = DistributionFunction::Polynomial {
                a: -2, // negative
                d: 10,
                m: 2, // positive => a*m is negative (decreasing)
                n: 1,
                o: 0,
                start_moment: Some(0),
                b: 50,               // f(0) = 50
                min_value: Some(50), // Starting at min_value
                max_value: Some(100),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected error").first_error().is_some(),
                "Expected an incoherence error when a decreasing function starts at min_value"
            );
        }

        // 9. Test valid: Polynomial with no min_value or max_value provided.
        #[test]
        fn test_polynomial_valid_no_boundaries() {
            let dist = DistributionFunction::Polynomial {
                a: 3,
                d: 10,
                m: 2,
                n: 1,
                o: 0,
                start_moment: Some(0),
                b: 20,
                min_value: None,
                max_value: None,
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected valid").first_error().is_none(),
                "Expected no validation errors when boundaries are omitted"
            );
        }

        // 10. Test valid: Polynomial with fractional exponent (m/n = 3/2).
        #[test]
        fn test_polynomial_valid_fractional() {
            // f(x) = (a * (x - s + o)^(m/n)) / d + b.
            // Here: a = 1, d = 1, m = 3, n = 2, s = 0, o = 0, b = 0.
            // So f(4) = 4^(3/2) = 8.
            let dist = DistributionFunction::Polynomial {
                a: 1,
                d: 1,
                m: 3,
                n: 2,
                o: 0,
                start_moment: Some(0),
                b: 0,
                min_value: Some(0),
                max_value: Some(100),
            };
            let eval_result = dist.evaluate(0, 4);
            assert_eq!(
                eval_result.unwrap(),
                8,
                "Expected f(4) to be 8 for a fractional exponent of 3/2"
            );
            let validation_result = dist.validate(4, PlatformVersion::latest());
            assert!(
                validation_result
                    .expect("expected valid")
                    .first_error()
                    .is_none(),
                "Expected no validation errors for a properly configured fractional exponent function"
            );
        }
    }
    mod exp {
        use super::*;
        #[test]
        fn test_exponential_valid() {
            let dist = DistributionFunction::Exponential {
                a: 3,
                d: 10,
                m: 1,
                n: 2,
                o: -3999,
                start_moment: Some(0),
                b: 10,
                min_value: Some(1),
                max_value: Some(1000000),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            if let Err(err) = &result {
                panic!("Test failed: unexpected error: {:?}", err);
            }
            if let Some(error) = result
                .expect("no error on test_exponential_valid")
                .first_error()
            {
                panic!("Test failed: validation error: {:?}", error);
            }
        }

        #[test]
        fn test_exponential_invalid_zero_n() {
            let dist = DistributionFunction::Exponential {
                a: 3,
                d: 10,
                m: 1,
                n: 0,
                o: 1,
                start_moment: Some(0),
                b: 10,
                min_value: Some(1),
                max_value: Some(100),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(result
                .expect("no error on test_exponential_invalid_zero_n")
                .first_error()
                .is_some());
        }

        #[test]
        fn test_exponential_invalid_zero_m() {
            let dist = DistributionFunction::Exponential {
                a: 3,
                d: 10,
                m: 0, // Invalid: `m` should not be zero
                n: 2,
                o: 1,
                start_moment: Some(0),
                b: 10,
                min_value: Some(1),
                max_value: Some(100),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_exponential_invalid_zero_m")
                    .first_error()
                    .is_some(),
                "Expected error: m should not be zero"
            );
        }

        #[test]
        fn test_exponential_invalid_zero_a() {
            let dist = DistributionFunction::Exponential {
                a: 0, // Invalid: `a` cannot be zero
                d: 10,
                m: 1,
                n: 2,
                o: 1,
                start_moment: Some(0),
                b: 10,
                min_value: Some(1),
                max_value: Some(100),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_exponential_invalid_zero_a")
                    .first_error()
                    .is_some(),
                "Expected error: a cannot be zero"
            );
        }

        #[test]
        fn test_exponential_invalid_max_missing_when_m_positive() {
            let dist = DistributionFunction::Exponential {
                a: 3,
                d: 10,
                m: 1, // `m > 0`, so `max_value` must be set
                n: 2,
                o: 1,
                start_moment: Some(0),
                b: 10,
                min_value: Some(1),
                max_value: None, // Invalid: max_value must be set
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_exponential_invalid_max_missing_when_m_positive")
                    .first_error()
                    .is_some(),
                "Expected error: max_value must be set when m > 0"
            );
        }

        #[test]
        fn test_exponential_invalid_o_too_large() {
            let dist = DistributionFunction::Exponential {
                a: 3,
                d: 10,
                m: 1,
                n: 2,
                o: MAX_DISTRIBUTION_PARAM as i64 + 1, // Invalid: `o` exceeds allowed range
                start_moment: Some(0),
                b: 10,
                min_value: Some(1),
                max_value: Some(100),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_exponential_invalid_o_too_large")
                    .first_error()
                    .is_some(),
                "Expected error: o exceeds MAX_DISTRIBUTION_PARAM"
            );
        }

        #[test]
        fn test_exponential_invalid_min_greater_than_max() {
            let dist = DistributionFunction::Exponential {
                a: 3,
                d: 10,
                m: -1,
                n: 2,
                o: 1,
                start_moment: Some(0),
                b: 10,
                min_value: Some(50), // Invalid: min > max
                max_value: Some(30),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_exponential_invalid_min_greater_than_max")
                    .first_error()
                    .is_some(),
                "Expected error: min_value > max_value"
            );
        }

        #[test]
        fn test_exponential_valid_with_negative_m() {
            let dist = DistributionFunction::Exponential {
                a: 3,
                d: 5,
                m: -2, // Valid: Decay function (exponential decrease)
                n: 4,
                o: 2,
                start_moment: Some(START_MOMENT),
                b: 8,
                min_value: Some(2),
                max_value: Some(50),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(result
                .expect("no error on test_exponential_valid_with_negative_m")
                .first_error()
                .is_none());
        }

        #[test]
        fn test_exponential_valid_with_max_boundary() {
            let dist = DistributionFunction::Exponential {
                a: 3,
                d: 5,
                m: 2,
                n: 4,
                o: 1,
                start_moment: Some(START_MOMENT),
                b: 8,
                min_value: Some(2),
                max_value: Some(MAX_DISTRIBUTION_PARAM), // Valid max
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(result
                .expect("no error on test_exponential_valid_with_max_boundary")
                .first_error()
                .is_none());
        }

        #[test]
        fn test_exponential_invalid_large_start_token_amount() {
            let dist = DistributionFunction::Exponential {
                a: MAX_DISTRIBUTION_PARAM,
                d: 1,
                m: 1,
                n: 1,
                o: 1,
                start_moment: Some(0),
                b: 10,
                min_value: Some(1),
                max_value: Some(MAX_DISTRIBUTION_PARAM),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_exponential_invalid_large_start_token_amount")
                    .first_error()
                    .is_some(),
                "Expected error: start_token_amount exceeds allowed range"
            );
        }

        #[test]
        fn test_exponential_invalid_a_too_large_for_max() {
            let dist = DistributionFunction::Exponential {
                a: MAX_DISTRIBUTION_PARAM, // Large `a`
                d: 10,
                m: 2, // Increasing
                n: 1,
                o: 0,
                start_moment: Some(0),
                b: 10,
                min_value: Some(1),
                max_value: Some(1000), // Small `max_value`
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_exponential_invalid_a_too_large_for_max")
                    .first_error()
                    .is_some(),
                "Expected error: `a` is too large, leading to immediate max_value"
            );
        }

        #[test]
        fn test_exponential_invalid_starts_at_min() {
            let dist = DistributionFunction::Exponential {
                a: 5,
                d: 10,
                m: -3, // Decreasing
                n: 2,
                o: 0,
                start_moment: Some(0),
                b: 10,
                min_value: Some(10), // Function starts at `min_value`
                max_value: Some(1000),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_exponential_invalid_starts_at_min")
                    .first_error()
                    .is_some(),
                "Expected IncoherenceError: function starts at `min_value`"
            );
        }

        #[test]
        fn test_exponential_invalid_missing_max_for_positive_m() {
            let dist = DistributionFunction::Exponential {
                a: 2,
                d: 10,
                m: 3, // Increasing
                n: 2,
                o: 1,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: None, // Should fail
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_exponential_invalid_missing_max_for_positive_m")
                    .first_error()
                    .is_some(),
                "Expected error: missing `max_value` when `m > 0`"
            );
        }

        #[test]
        fn test_exponential_invalid_large_o_overflow() {
            let dist = DistributionFunction::Exponential {
                a: 2,
                d: 10,
                m: 1,
                n: 1,
                o: i64::MAX / 2, // Large `o`
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(MAX_DISTRIBUTION_PARAM),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_exponential_invalid_large_o_overflow")
                    .first_error()
                    .is_some(),
                "Expected error: `o` is too large and causes overflow"
            );
        }

        #[test]
        fn test_exponential_invalid_a_too_small() {
            let dist = DistributionFunction::Exponential {
                a: 1, // Tiny `a`
                d: 10,
                m: -2, // Decreasing
                n: 2,
                o: 0,
                start_moment: Some(0),
                b: 10,
                min_value: Some(10),
                max_value: Some(100),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_exponential_invalid_a_too_small")
                    .first_error()
                    .is_some(),
                "Expected error: `a` is too small to make meaningful changes"
            );
        }

        #[test]
        fn test_exponential_valid_slow_increase() {
            let dist = DistributionFunction::Exponential {
                a: 1,
                d: 50,
                m: 1, // Small positive `m`
                n: 10,
                o: -3,
                start_moment: Some(0),
                b: 5,
                min_value: Some(10),
                max_value: Some(1000),
            };

            let result = dist.validate(5, PlatformVersion::latest());

            match result {
                Ok(validation_result) => {
                    if let Some(error) = validation_result.first_error() {
                        panic!("Test failed: Expected no error, but got: {:?}", error);
                    }
                }
                Err(protocol_error) => {
                    panic!(
                        "Test failed: Expected validation success, but got ProtocolError: {:?}",
                        protocol_error
                    );
                }
            }
        }

        #[test]
        fn test_exponential_valid_gentle_decay() {
            let dist = DistributionFunction::Exponential {
                a: 3,
                d: 15,
                m: -1, // Small negative `m`
                n: 4,
                o: 2,
                start_moment: Some(START_MOMENT),
                b: 8,
                min_value: Some(5),
                max_value: Some(100),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(result
                .expect("no error on test_exponential_valid_gentle_decay")
                .first_error()
                .is_none());
        }

        #[test]
        fn test_exponential_valid_negative_m_with_o_offset() {
            let dist = DistributionFunction::Exponential {
                a: 5,
                d: 8,
                m: -2, // Decreasing
                n: 3,
                o: 5, // Shift start
                start_moment: Some(START_MOMENT),
                b: 10,
                min_value: Some(5),
                max_value: Some(100),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(result
                .expect("no error on test_exponential_valid_negative_m_with_o_offset")
                .first_error()
                .is_none());
        }
    }
    mod log {
        use super::*;
        #[test]
        fn test_logarithmic_valid() {
            let dist = DistributionFunction::Logarithmic {
                a: 4,
                d: 10,
                m: 1,
                n: 2,
                o: 1,
                start_moment: None,
                b: 10,
                min_value: Some(1),
                max_value: Some(100),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(result
                .expect("no error on test_logarithmic_valid")
                .first_error()
                .is_none());
        }

        #[test]
        fn test_logarithmic_invalid_zero_d() {
            let dist = DistributionFunction::Logarithmic {
                a: 4,
                d: 0, // Invalid: Division by zero
                m: 1,
                n: 2,
                o: 1,
                start_moment: Some(0),
                b: 10,
                min_value: Some(1),
                max_value: Some(100),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_logarithmic_invalid_zero_d")
                    .first_error()
                    .is_some(),
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
                start_moment: Some(0),
                b: 10,
                min_value: Some(1),
                max_value: Some(100),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_logarithmic_invalid_zero_n")
                    .first_error()
                    .is_some(),
                "Expected division by zero error"
            );
        }

        #[test]
        fn test_logarithmic_invalid_zero_m() {
            let dist = DistributionFunction::Logarithmic {
                a: 4,
                d: 10,
                m: 0, // Invalid: this would make it a constant
                n: 1,
                o: 1,
                start_moment: Some(0),
                b: 10,
                min_value: Some(1),
                max_value: Some(100),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_logarithmic_invalid_zero_m")
                    .first_error()
                    .is_some(),
                "Expected m == 0 error"
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
                start_moment: Some(START_MOMENT),
                b: 10,
                min_value: Some(1),
                max_value: Some(100),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_logarithmic_invalid_x_s_o_non_positive")
                    .first_error()
                    .is_some(),
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
                start_moment: Some(0),
                b: 10,
                min_value: Some(1),
                max_value: Some(MAX_DISTRIBUTION_PARAM + 1), // Invalid: max_value too large
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_logarithmic_invalid_max_greater_than_max_param")
                    .first_error()
                    .is_some(),
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
                start_moment: Some(0),
                b: 10,
                min_value: Some(50), // Invalid: min > max
                max_value: Some(30),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_logarithmic_invalid_min_greater_than_max")
                    .first_error()
                    .is_some(),
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
                start_moment: Some(START_MOMENT - 2),
                b: 8,
                min_value: Some(2),
                max_value: Some(50),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(result
                .expect("no error on test_logarithmic_valid_with_s_and_o")
                .first_error()
                .is_none());
        }

        #[test]
        fn test_logarithmic_valid_edge_case_max() {
            let dist = DistributionFunction::Logarithmic {
                a: 3,
                d: 5,
                m: 2,
                n: 4,
                o: 1,
                start_moment: Some(START_MOMENT),
                b: 8,
                min_value: Some(2),
                max_value: Some(MAX_DISTRIBUTION_PARAM), // Valid max
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(result
                .expect("no error on test_logarithmic_valid_edge_case_max")
                .first_error()
                .is_none());
        }
    }
    mod inverted_log {
        use super::*;
        #[test]
        fn test_inverted_logarithmic_valid() {
            let dist = DistributionFunction::InvertedLogarithmic {
                a: -10,
                d: 1,
                m: 1,
                n: 100,
                o: 1,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(50),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result
                    .expect("no error on test_inverted_logarithmic_valid")
                    .first_error()
                    .is_none(),
                "Expected valid inverted logarithmic function"
            );
        }

        #[test]
        fn test_inverted_logarithmic_invalid_zero_a() {
            let dist = DistributionFunction::InvertedLogarithmic {
                a: 0,
                d: 1,
                m: 1,
                n: 100,
                o: 1,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(MAX_DISTRIBUTION_PARAM), // Valid max boundary
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert_eq!(
                result.expect("expected valid").first_error().expect("expected error").to_string(),
                "Invalid parameter `a` in token distribution function. Expected range: -32766 to 32767 except 0 (which we got)"
            );
        }

        #[test]
        fn test_inverted_logarithmic_invalid_too_low_a() {
            let dist = DistributionFunction::InvertedLogarithmic {
                a: -50000,
                d: 1,
                m: 1,
                n: 100,
                o: 1,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(MAX_DISTRIBUTION_PARAM), // Valid max boundary
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert_eq!(
                result.expect("expected valid").first_error().expect("expected error").to_string(),
                "Invalid parameter `a` in token distribution function. Expected range: -32766 to 32767 except 0 (which we got)"
            );
        }

        #[test]
        fn test_inverted_logarithmic_invalid_too_high_a() {
            let dist = DistributionFunction::InvertedLogarithmic {
                a: 50000,
                d: 1,
                m: 1,
                n: 100,
                o: 1,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(MAX_DISTRIBUTION_PARAM), // Valid max boundary
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert_eq!(
                result.expect("expected valid").first_error().expect("expected error").to_string(),
                "Invalid parameter `a` in token distribution function. Expected range: -32766 to 32767 except 0 (which we got)"
            );
        }

        #[test]
        fn test_inverted_logarithmic_invalid_divide_by_zero_d() {
            let dist = DistributionFunction::InvertedLogarithmic {
                a: -10,
                d: 0, // Invalid: d = 0 causes division by zero
                m: 1,
                n: 100,
                o: 1,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(50),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected error").first_error().is_some(),
                "Expected error: division by zero (d = 0)"
            );
        }

        #[test]
        fn test_inverted_logarithmic_invalid_zero_n() {
            let dist = DistributionFunction::InvertedLogarithmic {
                a: -10,
                d: 1,
                m: 1,
                n: 0, // Invalid: n = 0 causes division by zero in log argument
                o: 1,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(50),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected error").first_error().is_some(),
                "Expected error: division by zero (n = 0)"
            );
        }

        #[test]
        fn test_inverted_logarithmic_invalid_zero_m() {
            let dist = DistributionFunction::InvertedLogarithmic {
                a: -10,
                d: 1,
                m: 0, // Invalid: m = 0 causes invalid log argument
                n: 100,
                o: 1,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(50),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected error").first_error().is_some(),
                "Expected error: division by zero (m = 0)"
            );
        }

        #[test]
        fn test_inverted_logarithmic_invalid_negative_log_argument() {
            let dist = DistributionFunction::InvertedLogarithmic {
                a: -10,
                d: 1,
                m: 1,
                n: 100,
                o: -10, // Causes log argument to be non-positive
                start_moment: Some(START_MOMENT),
                b: 5,
                min_value: Some(1),
                max_value: Some(50),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected error").first_error().is_some(),
                "Expected error: log argument must be positive"
            );
        }

        #[test]
        fn test_inverted_logarithmic_invalid_exceeds_max_distribution_param() {
            let dist = DistributionFunction::InvertedLogarithmic {
                a: -10,
                d: 1,
                m: 1,
                n: 100,
                o: 1,
                start_moment: Some(MAX_DISTRIBUTION_PARAM + 1), // Invalid: s exceeds max
                b: 5,
                min_value: Some(1),
                max_value: Some(50),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected error").first_error().is_some(),
                "Expected error: s exceeds MAX_DISTRIBUTION_PARAM"
            );
        }

        #[test]
        fn test_inverted_logarithmic_invalid_min_greater_than_max() {
            let dist = DistributionFunction::InvertedLogarithmic {
                a: -10,
                d: 1,
                m: 1,
                n: 100,
                o: 1,
                start_moment: Some(0),
                b: 5,
                min_value: Some(60), // Invalid: min > max
                max_value: Some(50),
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected error").first_error().is_some(),
                "Expected error: min_value > max_value"
            );
        }

        #[test]
        fn test_inverted_logarithmic_valid_with_max_boundary() {
            let dist = DistributionFunction::InvertedLogarithmic {
                a: -10,
                d: 1,
                m: 1,
                n: 100,
                o: 1,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(MAX_DISTRIBUTION_PARAM), // Valid max boundary
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected valid").first_error().is_none(),
                "Expected valid function with max boundary"
            );
        }

        #[test]
        fn test_inverted_logarithmic_valid_with_min_a() {
            // Since `a` is negative, the inverted logarithmic function is increasing,
            // but it starts at the maximum value already, so it will never produce a higher value.
            let dist = DistributionFunction::InvertedLogarithmic {
                a: i64::MIN,
                d: 1,
                m: 1,
                n: 100,
                o: 1,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(MAX_DISTRIBUTION_PARAM), // Valid max boundary
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert_eq!(
                result.expect("expected valid").first_error().expect("expected error").to_string(),
                "Invalid parameter `a` in token distribution function. Expected range: -32766 to 32767 except 0 (which we got)"
            );
        }

        #[test]
        fn test_inverted_logarithmic_invalid_starting_at_max_for_increasing() {
            let dist = DistributionFunction::InvertedLogarithmic {
                a: -10, // Increasing function
                d: 1,
                m: 1,
                n: 100,
                o: 1,
                start_moment: Some(0),
                b: 50, // Starts at max_value
                min_value: Some(1),
                max_value: Some(50), // Function already at max
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected error").first_error().is_some(),
                "Expected error: increasing function starts at max_value"
            );
        }

        #[test]
        fn test_inverted_logarithmic_invalid_starting_at_min_for_decreasing() {
            let dist = DistributionFunction::InvertedLogarithmic {
                a: 10, // Decreasing function
                d: 1,
                m: 1,
                n: 100,
                o: 1,
                start_moment: Some(0),
                b: 1, // Starts at min_value
                min_value: Some(1),
                max_value: Some(50), // Function already at min
            };
            let result = dist.validate(START_MOMENT, PlatformVersion::latest());
            assert!(
                result.expect("expected error").first_error().is_some(),
                "Expected error: decreasing function starts at min_value"
            );
        }
    }
}
