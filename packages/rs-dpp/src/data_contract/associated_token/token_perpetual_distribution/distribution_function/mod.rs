use crate::balances::credits::TokenAmount;
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
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
/// This is applied to fixed amount distributions.
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

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd)]
#[serde(tag = "$type", rename_all = "camelCase")]
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
    ///   If this is provided before this number we give out the distribution start amount every interval.
    /// - `max_interval_count`: The maximum amount of intervals there can be. Can be up to 1024.
    ///   !!!Very important!!! -> This will default to 128 is default if not set.
    ///   This means that after 128 cycles we will be distributing trailing_distribution_interval_amount per interval.
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
    Stepwise(
        #[cfg_attr(
            feature = "json-conversion",
            serde(with = "crate::serialization::json::safe_integer_map::json_safe_u64_u64_map")
        )]
        BTreeMap<u64, TokenAmount>,
    ),

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
    /// ```text
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
    /// ```
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    mod construction {
        use super::*;

        #[test]
        fn fixed_amount_construction() {
            let dist = DistributionFunction::FixedAmount { amount: 42 };
            match dist {
                DistributionFunction::FixedAmount { amount } => assert_eq!(amount, 42),
                _ => panic!("Expected FixedAmount variant"),
            }
        }

        #[test]
        fn random_construction() {
            let dist = DistributionFunction::Random { min: 10, max: 100 };
            match dist {
                DistributionFunction::Random { min, max } => {
                    assert_eq!(min, 10);
                    assert_eq!(max, 100);
                }
                _ => panic!("Expected Random variant"),
            }
        }

        #[test]
        fn step_decreasing_amount_construction() {
            let dist = DistributionFunction::StepDecreasingAmount {
                step_count: 10,
                decrease_per_interval_numerator: 1,
                decrease_per_interval_denominator: 2,
                start_decreasing_offset: Some(5),
                max_interval_count: Some(128),
                distribution_start_amount: 1000,
                trailing_distribution_interval_amount: 50,
                min_value: Some(10),
            };
            match dist {
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
                    assert_eq!(step_count, 10);
                    assert_eq!(decrease_per_interval_numerator, 1);
                    assert_eq!(decrease_per_interval_denominator, 2);
                    assert_eq!(start_decreasing_offset, Some(5));
                    assert_eq!(max_interval_count, Some(128));
                    assert_eq!(distribution_start_amount, 1000);
                    assert_eq!(trailing_distribution_interval_amount, 50);
                    assert_eq!(min_value, Some(10));
                }
                _ => panic!("Expected StepDecreasingAmount variant"),
            }
        }

        #[test]
        fn stepwise_construction() {
            let mut steps = BTreeMap::new();
            steps.insert(0, 100);
            steps.insert(10, 50);
            steps.insert(20, 25);
            let dist = DistributionFunction::Stepwise(steps.clone());
            match dist {
                DistributionFunction::Stepwise(s) => {
                    assert_eq!(s.len(), 3);
                    assert_eq!(s[&0], 100);
                    assert_eq!(s[&10], 50);
                    assert_eq!(s[&20], 25);
                }
                _ => panic!("Expected Stepwise variant"),
            }
        }

        #[test]
        fn linear_construction() {
            let dist = DistributionFunction::Linear {
                a: -5,
                d: 2,
                start_step: Some(100),
                starting_amount: 500,
                min_value: Some(10),
                max_value: Some(1000),
            };
            match dist {
                DistributionFunction::Linear {
                    a,
                    d,
                    start_step,
                    starting_amount,
                    min_value,
                    max_value,
                } => {
                    assert_eq!(a, -5);
                    assert_eq!(d, 2);
                    assert_eq!(start_step, Some(100));
                    assert_eq!(starting_amount, 500);
                    assert_eq!(min_value, Some(10));
                    assert_eq!(max_value, Some(1000));
                }
                _ => panic!("Expected Linear variant"),
            }
        }

        #[test]
        fn polynomial_construction() {
            let dist = DistributionFunction::Polynomial {
                a: 3,
                d: 1,
                m: 2,
                n: 1,
                o: 0,
                start_moment: Some(0),
                b: 10,
                min_value: None,
                max_value: None,
            };
            match dist {
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
                    assert_eq!(a, 3);
                    assert_eq!(d, 1);
                    assert_eq!(m, 2);
                    assert_eq!(n, 1);
                    assert_eq!(o, 0);
                    assert_eq!(start_moment, Some(0));
                    assert_eq!(b, 10);
                    assert!(min_value.is_none());
                    assert!(max_value.is_none());
                }
                _ => panic!("Expected Polynomial variant"),
            }
        }

        #[test]
        fn exponential_construction() {
            let dist = DistributionFunction::Exponential {
                a: 100,
                d: 10,
                m: 2,
                n: 50,
                o: 0,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(100000),
            };
            match dist {
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
                    assert_eq!(a, 100);
                    assert_eq!(d, 10);
                    assert_eq!(m, 2);
                    assert_eq!(n, 50);
                    assert_eq!(o, 0);
                    assert_eq!(start_moment, Some(0));
                    assert_eq!(b, 5);
                    assert_eq!(min_value, Some(1));
                    assert_eq!(max_value, Some(100000));
                }
                _ => panic!("Expected Exponential variant"),
            }
        }

        #[test]
        fn logarithmic_construction() {
            let dist = DistributionFunction::Logarithmic {
                a: 10,
                d: 1,
                m: 1,
                n: 1,
                o: 1,
                start_moment: Some(0),
                b: 50,
                min_value: None,
                max_value: Some(200),
            };
            match dist {
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
                    assert_eq!(a, 10);
                    assert_eq!(d, 1);
                    assert_eq!(m, 1);
                    assert_eq!(n, 1);
                    assert_eq!(o, 1);
                    assert_eq!(start_moment, Some(0));
                    assert_eq!(b, 50);
                    assert!(min_value.is_none());
                    assert_eq!(max_value, Some(200));
                }
                _ => panic!("Expected Logarithmic variant"),
            }
        }

        #[test]
        fn inverted_logarithmic_construction() {
            let dist = DistributionFunction::InvertedLogarithmic {
                a: 10000,
                d: 1,
                m: 1,
                n: 5000,
                o: 0,
                start_moment: None,
                b: 0,
                min_value: Some(0),
                max_value: None,
            };
            match dist {
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
                    assert_eq!(a, 10000);
                    assert_eq!(d, 1);
                    assert_eq!(m, 1);
                    assert_eq!(n, 5000);
                    assert_eq!(o, 0);
                    assert!(start_moment.is_none());
                    assert_eq!(b, 0);
                    assert_eq!(min_value, Some(0));
                    assert!(max_value.is_none());
                }
                _ => panic!("Expected InvertedLogarithmic variant"),
            }
        }
    }

    mod display {
        use super::*;

        #[test]
        fn fixed_amount_display() {
            let dist = DistributionFunction::FixedAmount { amount: 42 };
            let s = format!("{}", dist);
            assert!(s.contains("FixedAmount"));
            assert!(s.contains("42"));
        }

        #[test]
        fn random_display() {
            let dist = DistributionFunction::Random { min: 10, max: 100 };
            let s = format!("{}", dist);
            assert!(s.contains("Random"));
            assert!(s.contains("10"));
            assert!(s.contains("100"));
        }

        #[test]
        fn step_decreasing_display_with_all_options() {
            let dist = DistributionFunction::StepDecreasingAmount {
                step_count: 10,
                decrease_per_interval_numerator: 1,
                decrease_per_interval_denominator: 2,
                start_decreasing_offset: Some(5),
                max_interval_count: Some(64),
                distribution_start_amount: 1000,
                trailing_distribution_interval_amount: 50,
                min_value: Some(10),
            };
            let s = format!("{}", dist);
            assert!(s.contains("StepDecreasingAmount"));
            assert!(s.contains("1000"));
            assert!(s.contains("period 5"));
            assert!(s.contains("64 intervals"));
            assert!(s.contains("50 tokens"));
            assert!(s.contains("minimum emission 10"));
        }

        #[test]
        fn step_decreasing_display_defaults() {
            let dist = DistributionFunction::StepDecreasingAmount {
                step_count: 10,
                decrease_per_interval_numerator: 1,
                decrease_per_interval_denominator: 2,
                start_decreasing_offset: None,
                max_interval_count: None,
                distribution_start_amount: 1000,
                trailing_distribution_interval_amount: 50,
                min_value: None,
            };
            let s = format!("{}", dist);
            assert!(s.contains("128 intervals (default)"));
        }

        #[test]
        fn stepwise_display() {
            let mut steps = BTreeMap::new();
            steps.insert(0, 100);
            steps.insert(10, 50);
            let dist = DistributionFunction::Stepwise(steps);
            let s = format!("{}", dist);
            assert!(s.contains("Stepwise"));
            assert!(s.contains("Step 0"));
            assert!(s.contains("100 tokens"));
            assert!(s.contains("Step 10"));
            assert!(s.contains("50 tokens"));
        }

        #[test]
        fn linear_display_with_start() {
            let dist = DistributionFunction::Linear {
                a: 5,
                d: 2,
                start_step: Some(10),
                starting_amount: 100,
                min_value: Some(1),
                max_value: Some(200),
            };
            let s = format!("{}", dist);
            assert!(s.contains("Linear"));
            assert!(s.contains("min: 1"));
            assert!(s.contains("max: 200"));
        }

        #[test]
        fn linear_display_without_start() {
            let dist = DistributionFunction::Linear {
                a: 5,
                d: 2,
                start_step: None,
                starting_amount: 100,
                min_value: None,
                max_value: None,
            };
            let s = format!("{}", dist);
            assert!(s.contains("Linear"));
            assert!(!s.contains("min:"));
            assert!(!s.contains("max:"));
        }

        #[test]
        fn polynomial_display() {
            let dist = DistributionFunction::Polynomial {
                a: 2,
                d: 1,
                m: 3,
                n: 2,
                o: 1,
                start_moment: Some(5),
                b: 10,
                min_value: None,
                max_value: Some(100),
            };
            let s = format!("{}", dist);
            assert!(s.contains("Polynomial"));
            assert!(s.contains("max: 100"));
        }

        #[test]
        fn exponential_display() {
            let dist = DistributionFunction::Exponential {
                a: 100,
                d: 10,
                m: 2,
                n: 50,
                o: 3,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(1000),
            };
            let s = format!("{}", dist);
            assert!(s.contains("Exponential"));
            assert!(s.contains("min: 1"));
            assert!(s.contains("max: 1000"));
        }

        #[test]
        fn logarithmic_display() {
            let dist = DistributionFunction::Logarithmic {
                a: 10,
                d: 1,
                m: 1,
                n: 1,
                o: 1,
                start_moment: None,
                b: 50,
                min_value: None,
                max_value: None,
            };
            let s = format!("{}", dist);
            assert!(s.contains("Logarithmic"));
        }

        #[test]
        fn inverted_logarithmic_display() {
            let dist = DistributionFunction::InvertedLogarithmic {
                a: 10,
                d: 1,
                m: 1,
                n: 100,
                o: 1,
                start_moment: Some(0),
                b: 5,
                min_value: Some(1),
                max_value: Some(50),
            };
            let s = format!("{}", dist);
            assert!(s.contains("InvertedLogarithmic"));
            assert!(s.contains("min: 1"));
            assert!(s.contains("max: 50"));
        }
    }

    mod equality_and_clone {
        use super::*;

        #[test]
        fn fixed_amount_equality() {
            let a = DistributionFunction::FixedAmount { amount: 100 };
            let b = DistributionFunction::FixedAmount { amount: 100 };
            let c = DistributionFunction::FixedAmount { amount: 200 };
            assert_eq!(a, b);
            assert_ne!(a, c);
        }

        #[test]
        fn clone_preserves_all_fields() {
            let dist = DistributionFunction::Polynomial {
                a: 3,
                d: 2,
                m: 4,
                n: 5,
                o: -1,
                start_moment: Some(100),
                b: 50,
                min_value: Some(5),
                max_value: Some(500),
            };
            let cloned = dist.clone();
            assert_eq!(dist, cloned);
        }

        #[test]
        fn partial_ord_between_variants() {
            let fixed = DistributionFunction::FixedAmount { amount: 100 };
            let random = DistributionFunction::Random { min: 10, max: 100 };
            assert!(fixed < random);
        }
    }

    mod constants {
        use super::*;

        #[test]
        fn max_distribution_param_is_u48_max() {
            assert_eq!(MAX_DISTRIBUTION_PARAM, (1u64 << 48) - 1);
        }

        #[test]
        fn max_distribution_cycles_param_is_u15_max() {
            assert_eq!(MAX_DISTRIBUTION_CYCLES_PARAM, (1u64 << 15) - 1);
        }

        #[test]
        fn default_step_decreasing_max_cycles() {
            assert_eq!(
                DEFAULT_STEP_DECREASING_AMOUNT_MAX_CYCLES_BEFORE_TRAILING_DISTRIBUTION,
                128
            );
        }

        #[test]
        fn linear_slope_bounds() {
            assert_eq!(MAX_LINEAR_SLOPE_A_PARAM, 256);
            assert_eq!(MIN_LINEAR_SLOPE_A_PARAM, -255);
        }

        #[test]
        #[allow(clippy::assertions_on_constants)]
        fn polynomial_bounds() {
            assert_eq!(MIN_POL_M_PARAM, -8);
            assert_eq!(MAX_POL_M_PARAM, 8);
            assert_eq!(MAX_POL_N_PARAM, 32);
            assert!(MIN_POL_A_PARAM < 0);
            assert!(MAX_POL_A_PARAM > 0);
        }

        #[test]
        fn exponential_bounds() {
            assert_eq!(MAX_EXP_A_PARAM, 256);
            assert_eq!(MAX_EXP_M_PARAM, 8);
            assert_eq!(MIN_EXP_M_PARAM, -8);
            assert_eq!(MAX_EXP_N_PARAM, 32);
        }

        #[test]
        fn log_bounds() {
            assert_eq!(MIN_LOG_A_PARAM, -32_766);
            assert_eq!(MAX_LOG_A_PARAM, 32_767);
        }
    }
}

// --- canonical conversion trait impls (unification pass 1) ---
#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for DistributionFunction {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for DistributionFunction {}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use platform_value::platform_value;
    use serde_json::json;

    // `DistributionFunction` is internally tagged (`#[serde(tag = "$type",
    // rename_all = "camelCase")]`). Round-trip tests cover one variant per
    // shape:
    //   - struct variant with named fields (`FixedAmount` → `fixedAmount`)
    //   - struct variant with multiple named fields (`Random` → `random`)
    //   - newtype-of-map variant (`Stepwise` → `stepwise`, flattened)
    // The other struct variants share the `FixedAmount`/`Random` shape.

    #[test]
    fn json_round_trip_fixed_amount() {
        use crate::serialization::JsonConvertible;
        let original = DistributionFunction::FixedAmount { amount: 1_000 };
        let json = original.to_json().expect("to_json");
        // Internally-tagged struct variant → `{"$type":"fixedAmount", <fields>}`.
        // `TokenAmount` is `u64`; JSON erases the size.
        assert_eq!(json, json!({ "$type": "fixedAmount", "amount": 1_000 }));
        let recovered = DistributionFunction::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_random() {
        use crate::serialization::JsonConvertible;
        let original = DistributionFunction::Random { min: 10, max: 100 };
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!({ "$type": "random", "min": 10, "max": 100 }));
        let recovered = DistributionFunction::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_stepwise() {
        use crate::serialization::JsonConvertible;
        // `Stepwise(BTreeMap<u64, TokenAmount>)` is a newtype-of-map variant.
        // Internal tagging flattens it: the `"type"` discriminator sits
        // alongside the map's numeric-string keys (u64 keys can never collide
        // with `"type"`). Matches the convention's "no data wrapper" rule.
        let original =
            DistributionFunction::Stepwise(std::collections::BTreeMap::from([(0, 100), (100, 50)]));
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!({ "$type": "stepwise", "0": 100, "100": 50 }));
        let recovered = DistributionFunction::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_stepwise() {
        use crate::serialization::ValueConvertible;
        let original =
            DistributionFunction::Stepwise(std::collections::BTreeMap::from([(0, 100), (100, 50)]));
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({ "$type": "stepwise", "0": 100u64, "100": 50u64 })
        );
        let recovered = DistributionFunction::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_fixed_amount() {
        use crate::serialization::ValueConvertible;
        let original = DistributionFunction::FixedAmount { amount: 1_000 };
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({ "$type": "fixedAmount", "amount": 1_000u64 })
        );
        let recovered = DistributionFunction::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_random() {
        use crate::serialization::ValueConvertible;
        let original = DistributionFunction::Random { min: 10, max: 100 };
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({ "$type": "random", "min": 10u64, "max": 100u64 })
        );
        let recovered = DistributionFunction::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
