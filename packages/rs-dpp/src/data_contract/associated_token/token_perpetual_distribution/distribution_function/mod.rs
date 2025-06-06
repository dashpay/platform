use crate::balances::credits::TokenAmount;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

mod encode;
mod evaluate;
pub mod evaluate_interval;
pub mod reward_ratio;
mod validation;

pub const MAX_DISTRIBUTION_PARAM: u64 = 281_474_976_710_655; //u48::Max 2^48 - 1
/// The max cycles param is the upper limit of cycles the system can ever support
/// This is applied to linear distribution.
/// For all other distributions we use a versioned max cycles contained in the platform version.
/// That other version is much lower because the calculations for other distributions are more
/// complex.
pub const MAX_DISTRIBUTION_CYCLES_PARAM: u64 = 32_767; //u15::Max 2^(63 - 48) - 1

pub const DEFAULT_STEP_DECREASING_AMOUNT_MAX_CYCLES_BEFORE_TRAILING_DISTRIBUTION: u16 = 128;

pub const MAX_LINEAR_SLOPE_A_PARAM: u64 = 256;

pub const MIN_LINEAR_SLOPE_A_PARAM: i64 = -255;

pub const MIN_POL_M_PARAM: i64 = -8;
pub const MAX_POL_M_PARAM: i64 = 8;

pub const MAX_POL_N_PARAM: u64 = 32;

pub const MIN_LOG_A_PARAM: i64 = -32_766;
pub const MAX_LOG_A_PARAM: i64 = 32_767;
pub const MAX_EXP_A_PARAM: u64 = 256;

pub const MAX_EXP_M_PARAM: u64 = 8;

pub const MIN_EXP_M_PARAM: i64 = -8;

pub const MAX_EXP_N_PARAM: u64 = 32;

pub const MIN_POL_A_PARAM: i64 = -255;
pub const MAX_POL_A_PARAM: i64 = 256;

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
    FixedAmount { amount: TokenAmount },

    /// Emits a random number of tokens within a specified range.
    ///
    /// # Description
    /// - This function selects a **random** token emission amount between `min` and `max`.
    /// - The value is drawn **uniformly** between the bounds.
    /// - The randomness uses a Pseudo Random Function (PRF) from x.
    ///
    /// # Formula
    /// For any period `x`, the emitted tokens follow:
    ///
    /// ```text
    /// f(x) ∈ [min, max]
    /// ```
    ///
    /// # Parameters
    /// - `min`: The **minimum** possible number of tokens emitted.
    /// - `max`: The **maximum** possible number of tokens emitted.
    ///
    /// # Use Cases
    /// - **Stochastic Rewards**: Introduces randomness into rewards to incentivize unpredictability.
    /// - **Lottery-Based Systems**: Used for randomized emissions, such as block rewards with probabilistic payouts.
    ///
    /// # Example
    /// Suppose a system emits **between 10 and 100 tokens per period**.
    ///
    /// ```text
    /// Random { min: 10, max: 100 }
    /// ```
    ///
    /// | Period (x) | Emitted Tokens (Random) |
    /// |------------|------------------------|
    /// | 1          | 27                     |
    /// | 2          | 94                     |
    /// | 3          | 63                     |
    /// | 4          | 12                     |
    ///
    /// - Each period, the function emits a **random number of tokens** between `min = 10` and `max = 100`.
    /// - Over time, the **average reward trends toward the midpoint** `(min + max) / 2`.
    ///
    /// # Constraints
    /// - **`min` must be ≤ `max`**, otherwise the function is invalid.
    /// - If `min == max`, this behaves like a `FixedAmount` function with a constant emission.
    Random { min: TokenAmount, max: TokenAmount },

    /// Emits tokens that decrease in discrete steps at fixed intervals.
    ///
    /// # Formula
    /// For a given period `x`, the emission is calculated as:
    ///
    /// ```text
    /// f(x) = n * (1 - (decrease_per_interval_numerator / decrease_per_interval_denominator))^((x - s) / step_count)
    /// ```
    ///
    /// For `x <= s`, `f(x) = n`
    ///
    /// # Parameters
    /// - `step_count`: The number of periods between each step.
    /// - `decrease_per_interval_numerator` and `decrease_per_interval_denominator`: Define the reduction factor per step.
    /// - `start_decreasing_offset`: Optional start period offset (e.g., start block or time). If not provided, the contract creation start is used.
    ///     If this is provided before this number we give out the distribution start amount every interval.
    /// - `max_interval_count`: The maximum amount of intervals there can be. Can be up to 1024.
    ///     !!!Very important!!! -> This will default to 128 is default if not set.
    ///     This means that after 128 cycles we will be distributing trailing_distribution_interval_amount per interval.
    /// - `distribution_start_amount`: The initial token emission.
    /// - `trailing_distribution_interval_amount`: The token emission after all decreasing intervals.
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
        start_decreasing_offset: Option<u64>,
        max_interval_count: Option<u16>,
        distribution_start_amount: TokenAmount,
        trailing_distribution_interval_amount: TokenAmount,
        min_value: Option<u64>,
    },

    /// Emits tokens in fixed amounts for predefined intervals (steps).
    ///
    /// # Details
    /// - Within each step, the emission remains constant.
    /// - The keys in the `BTreeMap` represent the starting period for each interval,
    ///   and the corresponding values are the fixed token amounts to emit during that interval.
    /// - VERY IMPORTANT: the steps are the amount of intervals, not the time or the block count.
    ///   So if you have step 5 with interval 10 using blocks that's 50 blocks.
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
    /// f(x) = (a * (x - start_step) / d) + starting_amount
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
    /// # Behavior
    /// - **If `a > 0`**, emissions increase linearly over time.
    /// - **If `a < 0`**, emissions decrease linearly over time.
    /// - **If `a = 0`**, emissions remain constant at `b`.
    ///
    /// # Use Cases
    /// - **Predictable Inflation or Deflation:** A simple mechanism to adjust token supply dynamically.
    /// - **Long-Term Incentive Structures:** Ensures steady and measurable growth or reduction of rewards.
    /// - **Decaying Emissions:** Can be used to gradually taper off token rewards over time.
    /// - **Sustained Growth Models:** Encourages prolonged engagement by steadily increasing rewards.
    ///
    /// # Examples
    ///
    /// ## **1️⃣ Increasing Linear Emission (`a > 0`)**
    /// - Tokens increase by **1 token per block** starting from 10.
    ///
    /// ```text
    /// f(x) = (1 * (x - 0) / 1) + 10
    /// ```
    ///
    /// | Block (x) | f(x) (Tokens) |
    /// |-----------|---------------|
    /// | 0         | 10            |
    /// | 1         | 11            |
    /// | 2         | 12            |
    /// | 3         | 13            |
    ///
    /// **Use Case:** Encourages continued participation by providing increasing rewards over time.
    ///
    /// ---
    ///
    /// ## **2️⃣ Decreasing Linear Emission (`a < 0`)**
    /// - Tokens **start at 100 and decrease by 2 per period**.
    ///
    /// ```text
    /// f(x) = (-2 * (x - 0) / 1) + 100
    /// ```
    ///
    /// | Block (x) | f(x) (Tokens) |
    /// |-----------|---------------|
    /// | 0         | 100           |
    /// | 1         | 98            |
    /// | 2         | 96            |
    /// | 3         | 94            |
    ///
    /// **Use Case:** Suitable for deflationary models where rewards need to decrease over time.
    ///
    /// ---
    ///
    /// ## **3️⃣ Emission with a Delayed Start (`s > 0`)**
    /// - **No emissions before `x = s`** (e.g., rewards start at block `10`).
    ///
    /// ```text
    /// f(x) = (5 * (x - 10) / 1) + 50
    /// ```
    ///
    /// | Block (x) | f(x) (Tokens) |
    /// |-----------|---------------|
    /// | 9         | 50 (no change)|
    /// | 10        | 50            |
    /// | 11        | 55            |
    /// | 12        | 60            |
    ///
    /// **Use Case:** Useful when rewards should only begin at a specific milestone.
    ///
    /// ---
    ///
    /// ## **4️⃣ Clamping Emissions with `min_value` and `max_value`**
    /// - **Start at 50, increase by 2, but never exceed 60.**
    ///
    /// ```text
    /// f(x) = (2 * (x - 0) / 1) + 50
    /// ```
    ///
    /// | Block (x) | f(x) (Tokens) |
    /// |-----------|---------------|
    /// | 0         | 50            |
    /// | 1         | 52            |
    /// | 2         | 54            |
    /// | 5         | 60 (max cap)  |
    ///
    /// **Use Case:** Prevents runaway inflation by limiting the emission range.
    ///
    /// ---
    ///
    /// # Summary
    /// - **Increasing rewards (`a > 0`)**: Encourages longer participation.
    /// - **Decreasing rewards (`a < 0`)**: Supports controlled deflation.
    /// - **Delayed start (`s > 0`)**: Ensures rewards only begin at a specific point.
    /// - **Clamping (`min_value`, `max_value`)**: Maintains controlled emission boundaries.
    Linear {
        a: i64,
        d: u64,
        start_step: Option<u64>,
        starting_amount: TokenAmount,
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
    /// # Behavior & Use Cases
    /// The polynomial function's behavior depends on the values of `a` (scaling factor) and `m` (exponent numerator).
    ///
    /// ## **1️⃣ `a > 0`, `m > 0` (Increasing Polynomial Growth)**
    /// - **Behavior**: Emissions **increase at an accelerating rate** over time.
    /// - **Use Case**: Suitable for models where incentives start small and grow over time (e.g., boosting late-stage participation).
    /// - **Example**:
    ///   ```text
    ///   f(x) = (2 * (x - s + o)^2) / d + 10
    ///   ```
    ///   - If `s = 0`, `o = 0`, and `d = 1`, then:
    ///     - `f(1) = 12`
    ///     - `f(2) = 18`
    ///     - `f(3) = 28` (Emissions **accelerate over time**)
    ///
    /// ## **2️⃣ `a > 0`, `m < 0` (Decreasing Polynomial Decay)**
    /// - **Behavior**: Emissions **start high and gradually decline**.
    /// - **Use Case**: Useful for front-loaded incentives where rewards are larger at the beginning and taper off over time.
    /// - **Example**:
    ///   ```text
    ///   f(x) = (5 * (x - s + o)^(-1)) / d + 10
    ///   ```
    ///   - If `s = 0`, `o = 0`, and `d = 1`, then:
    ///     - `f(1) = 15`
    ///     - `f(2) = 12.5`
    ///     - `f(3) = 11.67` (Emissions **shrink but never hit zero**)
    ///
    /// ## **3️⃣ `a < 0`, `m > 0` (Inverted Growth → Decreasing Over Time)**
    /// - **Behavior**: Emissions **start large but decrease faster over time**.
    /// - **Use Case**: Suitable for cases where high initial incentives quickly drop off (e.g., limited early rewards).
    /// - **Example**:
    ///   ```text
    ///   f(x) = (-3 * (x - s + o)^2) / d + 50
    ///   ```
    ///   - If `s = 0`, `o = 0`, and `d = 1`, then:
    ///     - `f(1) = 47`
    ///     - `f(2) = 38`
    ///     - `f(3) = 23` (Emissions **fall sharply**)
    ///
    /// ## **4️⃣ `a < 0`, `m < 0` (Inverted Decay → Slowing Increase)**
    /// - **Behavior**: Emissions **start low, rise gradually, and then flatten out**.
    /// - **Use Case**: Useful for controlled inflation where rewards increase over time but approach a stable maximum.
    /// - **Example**:
    ///   ```text
    ///   f(x) = (-10 * (x - s + o)^(-2)) / d + 50
    ///   ```
    ///   - If `s = 0`, `o = 0`, and `d = 1`, then:
    ///     - `f(1) = 40`
    ///     - `f(2) = 47.5`
    ///     - `f(3) = 48.89` (Growth **slows as it approaches 50**)
    ///
    /// # Summary
    /// - **Positive `a` means increasing emissions**, while **negative `a` means decreasing emissions**.
    /// - **Positive `m` leads to growth**, while **negative `m` leads to decay**.
    /// - The combination of `a` and `m` defines whether emissions accelerate, decay, or remain stable.
    Polynomial {
        a: i64,
        d: u64,
        m: i64,
        n: u64,
        o: i64,
        start_moment: Option<u64>,
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
    /// f(x) = (a * e^(m * (x - s + o) / n)) / d + b
    /// ```
    ///
    /// # Parameters
    /// - `a`: The scaling factor.
    /// - `m` and `n`: Define the exponent rate (with `m > 0` for growth and `m < 0` for decay).
    /// - `d`: A divisor used to scale the exponential term.
    /// - `s`: Optional start period offset. If not set, the contract creation start is assumed.
    /// - `o`: An offset for the exp function, this is useful if s is in None.
    /// - `b`: An offset added to the result.
    /// - `min_value` / `max_value`: Optional constraints on the emitted tokens.
    ///
    /// # Use Cases
    /// ## **Exponential Growth (`m > 0`):**
    /// - **Incentivized Spending**: Higher emissions over time increase the circulating supply, encouraging users to spend tokens.
    /// - **Progressive Emission Models**: Useful for models where early emissions are low but increase significantly over time.
    /// - **Early-Stage Adoption Strategies**: Helps drive later participation by offering increasing rewards as time progresses.
    ///
    /// ## **Exponential Decay (`m < 0`):**
    /// - **Deflationary Reward Models**: Reduces emissions over time, ensuring token scarcity.
    /// - **Early Participation Incentives**: Encourages early users by distributing more tokens initially and gradually decreasing rewards.
    /// - **Sustainable Emission Models**: Helps manage token supply while preventing runaway inflation.
    ///
    /// # Examples
    /// ## **Example 1: Exponential Growth (`m > 0`)**
    /// - **Use Case**: A staking model where rewards increase over time to encourage long-term participation.
    /// - **Parameters**: `a = 100`, `m = 2`, `n = 50`, `d = 10`, `c = 5`
    /// - **Formula**:
    ///   ```text
    ///   f(x) = (100 * e^(2 * (x - s) / 50)) / 10 + 5
    ///   ```
    /// - **Effect**: Emissions start small but **increase exponentially** over time, rewarding late stakers more than early ones.
    ///
    /// ## **Example 2: Exponential Decay (`m < 0`)**
    /// - **Use Case**: A deflationary model where emissions start high and gradually decrease to ensure scarcity.
    /// - **Parameters**: `a = 500`, `m = -3`, `n = 100`, `d = 20`, `b = 10`
    /// - **Formula**:
    ///   ```text
    ///   f(x) = (500 * e^(-3 * (x - s) / 100)) / 20 + 10
    ///   ```
    /// - **Effect**: Emissions start **high and decay exponentially**, ensuring early participants get larger rewards.
    Exponential {
        a: u64,
        d: u64,
        m: i64,
        n: u64,
        o: i64,
        start_moment: Option<u64>,
        b: TokenAmount,
        min_value: Option<u64>,
        max_value: Option<u64>,
    },

    /// Emits tokens following a natural logarithmic (ln) function.
    ///
    /// # Formula
    /// The emission at period `x` is computed as:
    ///
    /// ```text
    /// f(x) = (a * ln(m * (x - s + o) / n)) / d + b
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
    /// - **Gradual Growth with a Slowing Rate**: Suitable for reward schedules where the emission
    ///   starts at a lower rate, increases quickly at first, but then slows down over time.
    /// - **Predictable Emission Scaling**: Ensures a growing but controlled emission curve that
    ///   does not escalate too quickly.
    /// - **Sustainability and Inflation Control**: Helps prevent runaway token supply growth
    ///   by ensuring rewards increase at a decreasing rate.
    ///
    /// # Example
    /// - Suppose we want token emissions to start at a low value and grow over time, but at a
    ///   **decreasing rate**, ensuring controlled long-term growth.
    ///
    /// - Given the formula:
    ///   ```text
    ///   f(x) = (a * ln(m * (x - s + o) / n)) / d + b
    ///   ```
    ///
    /// - Let’s assume the following parameters:
    ///   - `a = 100`: Scaling factor.
    ///   - `d = 10`: Divisor to control overall scaling.
    ///   - `m = 2`, `n = 1`: Adjust the logarithmic input.
    ///   - `s = 0`, `o = 1`: Starting conditions.
    ///   - `b = 50`: Base amount added.
    ///
    /// - This results in:
    ///   ```text
    ///   f(x) = (100 * ln(2 * (x + 1) / 1)) / 10 + 50
    ///   ```
    ///
    /// - **Expected Behavior:**
    ///   - At `x = 1`, emission = `f(1) = (100 * log(4)) / 10 + 50 ≈ 82`
    ///   - At `x = 10`, emission = `f(10) = (100 * log(22)) / 10 + 50 ≈ 106`
    ///   - At `x = 100`, emission = `f(100) = (100 * log(202)) / 10 + 50 ≈ 130`
    ///
    /// - **Observations:**
    ///   - The emission **increases** over time, but at a **slowing rate**.
    ///   - Early increases are more pronounced, but as `x` grows, the additional reward per
    ///     period gets smaller.
    ///   - This makes it ideal for long-term, controlled emission models.
    Logarithmic {
        a: i64,
        d: u64,
        m: u64,
        n: u64,
        o: i64,
        start_moment: Option<u64>,
        b: TokenAmount,
        min_value: Option<u64>,
        max_value: Option<u64>,
    },
    /// Emits tokens following an inverted natural logarithmic function.
    ///
    /// # Formula
    /// The emission at period `x` is given by:
    ///
    /// ```text
    /// f(x) = (a * ln( n / (m * (x - s + o)) )) / d + b
    /// ```
    ///
    /// # Parameters
    /// - `a`: Scaling factor.
    /// - `d`: Divisor for scaling.
    /// - `m` and `n`: Together control the logarithm argument inversion.
    /// - `o`: Offset applied inside the logarithm.
    /// - `s`: Optional start period offset.
    /// - `b`: Offset added to the computed value.
    /// - `min_value` / `max_value`: Optional boundaries for the emission.
    ///
    /// # Use Case
    /// - **Gradual Decay of Rewards**: Suitable when early adopters should receive higher rewards,
    ///   but later participants should receive smaller but still meaningful amounts.
    /// - **Resource Draining / Controlled Burn**: Used when token emissions should drop significantly
    ///   at first but slow down over time to preserve capital.
    /// - **Airdrop or Grant System**: Ensures early claimants receive larger distributions, but later
    ///   claimants receive diminishing rewards.
    ///
    /// # Example
    ///   ```text
    ///   f(x) = 10000 * ln(5000 / x)
    ///   ```
    /// - Values: a = 10000 n = 5000 m = 1 o = 0 b = 0 d = 0
    ///           y
    ///           ↑
    ///          10000 |*
    ///           9000 | *
    ///           8000 |  *
    ///           7000 |   *
    ///           6000 |    *
    ///           5000 |     *
    ///           4000 |       *
    ///           3000 |         *
    ///           2000 |           *
    ///           1000 |              *
    ///              0 +-------------------*----------→ x
    ///                  0     2000   4000   6000   8000
    ///
    ///   - The emission **starts high** and **gradually decreases**, ensuring early adopters receive
    ///     more tokens while later participants still get rewards.
    ///   - The function **slows down the rate of decrease** over time, preventing emissions from
    ///     hitting zero too quickly.
    InvertedLogarithmic {
        a: i64,
        d: u64,
        m: u64,
        n: u64,
        o: i64,
        start_moment: Option<u64>,
        b: TokenAmount,
        min_value: Option<u64>,
        max_value: Option<u64>,
    },
}

impl fmt::Display for DistributionFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DistributionFunction::FixedAmount { amount: n } => {
                write!(f, "FixedAmount: {} tokens per period", n)
            }
            DistributionFunction::Random { min, max } => {
                write!(f, "Random: tokens ∈ [{}, {}] per period", min, max)
            }
            DistributionFunction::StepDecreasingAmount {
                step_count,
                decrease_per_interval_numerator,
                decrease_per_interval_denominator,
                start_decreasing_offset: s,
                max_interval_count,
                distribution_start_amount,
                trailing_distribution_interval_amount,
                min_value,
            } => {
                write!(
                    f,
                    "StepDecreasingAmount: {} tokens, decreasing by {}/{} every {} steps",
                    distribution_start_amount,
                    decrease_per_interval_numerator,
                    decrease_per_interval_denominator,
                    step_count
                )?;
                if let Some(start) = s {
                    write!(f, ", starting at period {}", start)?;
                }
                if let Some(max_intervals) = max_interval_count {
                    write!(f, ", with a maximum of {} intervals", max_intervals)?;
                } else {
                    write!(f, ", with a maximum of 128 intervals (default)")?;
                }
                write!(
                    f,
                    ", trailing distribution amount {} tokens",
                    trailing_distribution_interval_amount
                )?;
                if let Some(min) = min_value {
                    write!(f, ", minimum emission {} tokens", min)?;
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
                start_step: s,
                starting_amount: b,
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
                start_moment: s,
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
                start_moment,
                b,
                min_value,
                max_value,
            } => {
                write!(f, "Exponential: f(x) = {} * e^( {} * (x", a, m)?;
                if let Some(start) = start_moment {
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
                write!(
                    f,
                    "InvertedLogarithmic: f(x) = {} * log( {} / ({} * (x",
                    a, n, m
                )?;
                if let Some(start) = s {
                    write!(f, " - {} + {})", start, o)?;
                } else {
                    write!(f, " + {})", o)?;
                }
                write!(f, ") ) / {} + {}", d, b)?;
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
