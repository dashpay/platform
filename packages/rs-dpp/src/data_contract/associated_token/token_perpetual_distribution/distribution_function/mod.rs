use std::collections::BTreeMap;
use crate::balances::credits::TokenAmount;
use serde::{Deserialize, Serialize};
use std::fmt;

mod encode;
mod validation;
mod methods;

pub const MAX_DISTRIBUTION_PARAM: u64 = 281_474_976_710_655; //u48::Max 2^48 - 1

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub enum DistributionFunction {
    /// Emits a constant (fixed) number of tokens for every period.
    ///
    /// # Formula
    /// For any period `x`, the emitted tokens are:
    ///
    /// ```text
    /// f(x) = n
    /// ```
    ///
    /// # Use Case
    /// - When a predictable, unchanging reward is desired.
    /// - Simplicity and stable emissions.
    ///
    /// # Example
    /// - If `n = 5` tokens per block, then after 3 blocks the total emission is 15 tokens.
    FixedAmount { n: TokenAmount },

    /// Emits tokens that decrease in discrete steps at fixed intervals.
    ///
    /// # Formula
    /// For a given period `x`, the emission is calculated as:
    ///
    /// ```text
    /// f(x) = n * (1 - (decrease_per_interval_numerator / decrease_per_interval_denominator))^((x - s) / step_count)
    /// ```
    ///
    /// # Parameters
    /// - `step_count`: The number of periods between each step.
    /// - `decrease_per_interval_numerator` and `decrease_per_interval_denominator`: Define the reduction factor per step.
    /// - `s`: Optional start period offset (e.g., start block or time). If not provided, the contract creation start is used.
    /// - `n`: The initial token emission.
    /// - `min_value`: Optional minimum emission value.
    ///
    /// # Use Case
    /// - Modeling reward systems similar to Bitcoin or Dash Core.
    /// - Encouraging early participation by providing higher rewards initially.
    ///
    /// # Example
    /// - Bitcoin-style: 50% reduction every 210,000 blocks.
    /// - Dash-style: Approximately a 7% reduction every 210,000 blocks.
    StepDecreasingAmount {
        step_count: u32,
        decrease_per_interval_numerator: u16,
        decrease_per_interval_denominator: u16,
        s: Option<u64>,
        n: TokenAmount,
        min_value: Option<u64>,
    },

    /// Emits tokens in fixed amounts for predefined intervals (steps).
    ///
    /// # Details
    /// - Within each step, the emission remains constant.
    /// - The keys in the `BTreeMap` represent the starting period for each interval,
    ///   and the corresponding values are the fixed token amounts to emit during that interval.
    ///
    /// # Use Case
    /// - Adjusting rewards at specific milestones or time intervals.
    ///
    /// # Example
    /// - Emit 100 tokens per block for the first 1,000 blocks, then 50 tokens per block thereafter.
    Stepwise(BTreeMap<u64, TokenAmount>),

    /// Emits tokens following a linear function that can increase or decrease over time
    /// with fractional precision.
    ///
    /// # Formula
    /// The emission at period `x` is given by:
    ///
    /// ```text
    /// f(x) = (a * (x - s) / d) + b
    /// ```
    ///
    /// # Parameters
    /// - `a`: The slope numerator; determines the rate of change.
    /// - `d`: The slope divisor; together with `a` controls the fractional rate.
    /// - `s`: Optional start period offset. If not set, the contract creation start is assumed.
    /// - `b`: The initial token emission (offset).
    /// - `min_value` / `max_value`: Optional bounds to clamp the emission.
    ///
    /// # Details
    /// - If `a > 0`, emissions increase over time.
    /// - If `a < 0`, emissions decrease over time.
    ///
    /// # Use Case
    /// - When a smooth, gradual change in emissions is needed.
    /// - Useful when fractional (floating-point) rate adjustments are desired.
    ///
    /// # Example
    /// - Starting at 50 tokens and increasing by 0.5 tokens per period:
    ///   ```text
    ///   f(x) = 0.5 * (x - s) / d + 50
    ///   ```
    Linear {
        a: i64,
        d: u64,
        s: Option<u64>,
        b: TokenAmount,
        min_value: Option<u64>,
        max_value: Option<u64>,
    },

    /// Emits tokens following a polynomial curve with integer arithmetic.
    ///
    /// # Formula
    /// The emission at period `x` is given by:
    ///
    /// ```text
    /// f(x) = (a * (x - s + o)^(m/n)) / d + b
    /// ```
    ///
    /// # Parameters
    /// - `a`: Scaling factor for the polynomial term.
    /// - `m` and `n`: Together specify the exponent as a rational number (allowing non-integer exponents).
    /// - `d`: A divisor for scaling.
    /// - `s`: Optional start period offset. If not provided, the contract creation start is used.
    /// - `o`: An offset for the polynomial function, this is useful if s is in None,
    /// - `b`: An offset added to the computed value.
    /// - `min_value` / `max_value`: Optional bounds to constrain the emission.
    ///
    /// # Use Case
    /// - Reward systems where returns diminish (or increase) non-linearly over time.
    ///
    /// # Example
    /// - A quadratic emission curve might look like:
    ///   ```text
    ///   f(x) = 2 * (x - s + o)^2 / d + 20
    ///   ```
    Polynomial {
        a: i64,
        d: u64,
        m: u64,
        n: u64,
        o: i64,
        s: Option<u64>,
        b: TokenAmount,
        min_value: Option<u64>,
        max_value: Option<u64>,
    },

    /// Emits tokens following an exponential function.
    ///
    /// # Formula
    /// The emission at period `x` is given by:
    ///
    /// ```text
    /// f(x) = (a * e^(m * (x - s) / n)) / d + c
    /// ```
    ///
    /// # Parameters
    /// - `a`: The scaling factor.
    /// - `m` and `n`: Define the exponent rate (with `m > 0` for growth and `m < 0` for decay).
    /// - `d`: A divisor used to scale the exponential term.
    /// - `s`: Optional start period offset. If not set, the contract creation start is assumed.
    /// - `o`: An offset for the exp function, this is useful if s is in None.
    /// - `c`: An offset added to the result.
    /// - `min_value` / `max_value`: Optional constraints on the emitted tokens.
    ///
    /// # Use Case
    /// - Reward systems where early contributors receive disproportionately higher rewards.
    ///
    /// # Example
    /// - Starting with 100 tokens and halving emissions each interval (with a minimum of 5 tokens):
    ///   ```text
    ///   f(x) = 100 * e^(m * (x - s + o) / n) / d + 5
    ///   ```
    Exponential {
        a: u64,
        d: u64,
        m: i64,
        n: u64,
        o: i64,
        s: Option<u64>,
        c: TokenAmount,
        min_value: Option<u64>,
        max_value: Option<u64>,
    },

    /// Emits tokens following a logarithmic function.
    ///
    /// # Formula
    /// The emission at period `x` is computed as:
    ///
    /// ```text
    /// f(x) = (a * log(m * (x - s + o) / n)) / d + b
    /// ```
    ///
    /// # Parameters
    /// - `a`: Scaling factor for the logarithmic term.
    /// - `d`: A divisor for scaling.
    /// - `m` and `n`: Adjust the input to the logarithm function.
    /// - `s`: Optional start period offset. If not provided, the contract creation start is used.
    /// - `o`: An offset for the log function, this is useful if s is in None.
    /// - `b`: An offset added to the result.
    /// - `min_value` / `max_value`: Optional bounds to ensure the emission remains within limits.
    ///
    /// # Use Case
    /// - Suitable for long-term reward schedules where emissions need to increase at a diminishing rate.
    ///
    /// # Example
    /// - An emission function following a logarithmic curve:
    ///   ```text
    ///   f(x) = 20 * log(m * (x - s) / n) / d + 5
    ///   ```
    Logarithmic {
        a: i64,
        d: u64,
        m: i64,
        n: u64,
        o: i64,
        s: Option<u64>,
        b: TokenAmount,
        min_value: Option<u64>,
        max_value: Option<u64>,
    },
}

impl fmt::Display for DistributionFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DistributionFunction::FixedAmount { n } => {
                write!(f, "FixedAmount: {} tokens per period", n)
            }
            DistributionFunction::StepDecreasingAmount {
                step_count,
                decrease_per_interval_numerator,
                decrease_per_interval_denominator,
                s,
                n,
                min_value,
            } => {
                write!(
                    f,
                    "StepDecreasingAmount: {} tokens, decreasing by {}/{} every {} steps",
                    n, decrease_per_interval_numerator, decrease_per_interval_denominator, step_count
                )?;
                if let Some(start) = s {
                    write!(f, " starting at period {}", start)?;
                }
                if let Some(min) = min_value {
                    write!(f, ", with a minimum emission of {}", min)?;
                }
                Ok(())
            }
            DistributionFunction::Stepwise(steps) => {
                write!(f, "Stepwise emission: ")?;
                let mut first = true;
                for (step, amount) in steps {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "[Step {} → {} tokens]", step, amount)?;
                }
                Ok(())
            }
            DistributionFunction::Linear {
                a,
                d,
                s,
                b,
                min_value,
                max_value,
            } => {
                write!(f, "Linear: f(x) = {} * (x", a)?;
                if let Some(start) = s {
                    write!(f, " - {})", start)?;
                } else {
                    write!(f, ")")?;
                }
                write!(f, " / {}) + {}", d, b)?;
                if let Some(min) = min_value {
                    write!(f, ", min: {}", min)?;
                }
                if let Some(max) = max_value {
                    write!(f, ", max: {}", max)?;
                }
                Ok(())
            }
            DistributionFunction::Polynomial {
                a,
                d,
                m,
                n,
                o,
                s,
                b,
                min_value,
                max_value,
            } => {
                write!(f, "Polynomial: f(x) = {} * (x", a)?;
                if let Some(start) = s {
                    write!(f, " - {} + {})", start, o)?;
                } else {
                    write!(f, " + {})", o)?;
                }
                write!(f, "^( {} / {} ) / {} + {}", m, n, d, b)?;
                if let Some(min) = min_value {
                    write!(f, ", min: {}", min)?;
                }
                if let Some(max) = max_value {
                    write!(f, ", max: {}", max)?;
                }
                Ok(())
            }
            DistributionFunction::Exponential {
                a,
                d,
                m,
                n,
                o,
                s,
                c,
                min_value,
                max_value,
            } => {
                write!(f, "Exponential: f(x) = {} * e^( {} * (x", a, m)?;
                if let Some(start) = s {
                    write!(f, " - {} + {})", start, o)?;
                } else {
                    write!(f, " + {})", o)?;
                }
                write!(f, " / {} ) / {} + {}", n, d, c)?;
                if let Some(min) = min_value {
                    write!(f, ", min: {}", min)?;
                }
                if let Some(max) = max_value {
                    write!(f, ", max: {}", max)?;
                }
                Ok(())
            }
            DistributionFunction::Logarithmic {
                a,
                d,
                m,
                n,
                o,
                s,
                b,
                min_value,
                max_value,
            } => {
                write!(f, "Logarithmic: f(x) = {} * log( {} * (x", a, m)?;
                if let Some(start) = s {
                    write!(f, " - {} + {})", start, o)?;
                } else {
                    write!(f, " + {})", o)?;
                }
                write!(f, " / {} ) / {} + {}", n, d, b)?;
                if let Some(min) = min_value {
                    write!(f, ", min: {}", min)?;
                }
                if let Some(max) = max_value {
                    write!(f, ", max: {}", max)?;
                }
                Ok(())
            }
        }
    }
}