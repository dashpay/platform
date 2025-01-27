use crate::balances::credits::{SignedTokenAmount, TokenAmount};
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};
use std::fmt;

mod encode;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub enum DistributionFunction {
    /// A linear function emits tokens in increasing or decreasing amounts over time (integer precision).
    ///
    /// # Formula
    /// - `f(x) = a * x + b`
    /// - Where `a` is the slope (rate of change) and `b` is the initial value.
    ///
    /// # Description
    /// - `a > 0`: Tokens increase over time.
    /// - `a < 0`: Tokens decrease over time.
    /// - `b` is the starting emission value.
    ///
    /// # Use Case
    /// - Incentivize early adopters with higher rewards (`a < 0`).
    /// - Gradually increase emissions to match ecosystem growth (`a > 0`).
    ///
    /// # Example
    /// - Start with 50 tokens and increase by 10 tokens per epoch: `f(x) = 10x + 50`.
    LinearInteger { a: i64, b: SignedTokenAmount },

    /// A linear function emits tokens in increasing or decreasing amounts over time (floating-point precision).
    ///
    /// # Formula
    /// - `f(x) = a * x + b`
    /// - Where `a` is the slope (rate of change) and `b` is the initial value.
    ///
    /// # Description
    /// - `a > 0`: Tokens increase over time.
    /// - `a < 0`: Tokens decrease over time.
    /// - `b` is the starting emission value.
    ///
    /// # Use Case
    /// - Similar to `LinearInteger`, but supports fractional rates of change.
    ///
    /// # Example
    /// - Start with 50 tokens and increase by 0.5 tokens per epoch: `f(x) = 0.5x + 50`.
    LinearFloat {
        a: NotNan<f64>,
        b: SignedTokenAmount,
    },

    /// A polynomial function emits tokens according to a quadratic or cubic curve (integer precision).
    ///
    /// # Formula
    /// - `f(x) = a * x^n + b`
    /// - Where `n` is the degree of the polynomial, `a` is the scaling factor, and `b` is the base amount.
    ///
    /// # Description
    /// - Higher-degree polynomials allow for flexible emission curves.
    /// - Use for growth or decay patterns that aren't linear.
    ///
    /// # Use Case
    /// - Reward systems with diminishing returns as time progresses.
    ///
    /// # Example
    /// - Emit rewards based on a quadratic curve: `f(x) = 2x^2 + 20`.
    PolynomialInteger {
        a: i64,
        n: i64,
        b: SignedTokenAmount,
    },

    /// A polynomial function emits tokens according to a quadratic or cubic curve (floating-point precision).
    ///
    /// # Formula
    /// - `f(x) = a * x^n + b`
    /// - Where `n` is the degree of the polynomial, `a` is the scaling factor, and `b` is the base amount.
    ///
    /// # Description
    /// - Similar to `PolynomialInteger`, but supports fractional scaling and degrees.
    ///
    /// # Example
    /// - Emit rewards based on a cubic curve with fractional growth: `f(x) = 0.5x^3 + 20`.
    PolynomialFloat {
        a: NotNan<f64>,
        n: NotNan<f64>,
        b: SignedTokenAmount,
    },

    /// An exponential function emits tokens based on exponential growth or decay.
    ///
    /// # Formula
    /// - `f(x) = a * e^(b * x) + c`
    /// - Where `a` is the scaling factor, `b` controls the growth/decay rate, and `c` is an offset.
    ///
    /// # Description
    /// - Exponential growth: `b > 0`, emissions increase rapidly.
    /// - Exponential decay: `b < 0`, emissions decrease rapidly.
    /// - Useful for early incentivization or ecosystem maturity.
    ///
    /// # Use Case
    /// - Reward mechanisms where early contributors get larger rewards.
    ///
    /// # Example
    /// - Start with 100 tokens and halve emissions every interval, with a minimum of 5 tokens: `f(x) = 100 * e^(-0.693 * x) + 5`.
    Exponential {
        a: NotNan<f64>,
        b: NotNan<f64>,
        c: SignedTokenAmount,
    },

    /// A logarithmic function emits tokens based on logarithmic growth.
    ///
    /// # Formula
    /// - `f(x) = a * log_b(x) + c`
    /// - Where `a` is the scaling factor, `b` is the logarithm base, and `c` is an offset.
    ///
    /// # Description
    /// - Growth starts quickly but slows as `x` increases.
    /// - Suitable for sustainable emissions over long periods.
    ///
    /// # Use Case
    /// - Gradual emissions tapering to balance supply and demand.
    ///
    /// # Example
    /// - Emit rewards using a log base-2 curve: `f(x) = 20 * log_2(x) + 5`.
    Logarithmic {
        a: NotNan<f64>,
        b: NotNan<f64>,
        c: SignedTokenAmount,
    },

    /// A stepwise function emits tokens in fixed amounts for predefined intervals.
    ///
    /// # Description
    /// - Emissions remain constant within each step.
    /// - Steps define specific time intervals or milestones.
    ///
    /// # Use Case
    /// - Adjust rewards at specific milestones.
    ///
    /// # Example
    /// - Emit 100 tokens per block for the first 1000 blocks, then 50 tokens thereafter.
    Stepwise(Vec<(u64, TokenAmount)>),
}

impl fmt::Display for DistributionFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DistributionFunction::LinearInteger { a, b } => {
                write!(f, "LinearInteger: f(x) = {} * x + {}", a, b)
            }
            DistributionFunction::LinearFloat { a, b } => {
                write!(f, "LinearFloat: f(x) = {:.3} * x + {}", a.into_inner(), b)
            }
            DistributionFunction::PolynomialInteger { a, n, b } => {
                write!(f, "PolynomialInteger: f(x) = {} * x^{} + {}", a, n, b)
            }
            DistributionFunction::PolynomialFloat { a, n, b } => {
                write!(
                    f,
                    "PolynomialFloat: f(x) = {:.3} * x^{:.3} + {}",
                    a.into_inner(),
                    n.into_inner(),
                    b
                )
            }
            DistributionFunction::Exponential { a, b, c } => {
                write!(
                    f,
                    "Exponential: f(x) = {:.3} * e^({:.3} * x) + {}",
                    a.into_inner(),
                    b.into_inner(),
                    c
                )
            }
            DistributionFunction::Logarithmic { a, b, c } => {
                write!(
                    f,
                    "Logarithmic: f(x) = {:.3} * log_{:.3}(x) + {}",
                    a.into_inner(),
                    b.into_inner(),
                    c
                )
            }
            DistributionFunction::Stepwise(steps) => {
                write!(f, "Stepwise: ")?;
                for (index, (step, amount)) in steps.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "Step {} -> {}", step, amount)?;
                }
                Ok(())
            }
        }
    }
}
