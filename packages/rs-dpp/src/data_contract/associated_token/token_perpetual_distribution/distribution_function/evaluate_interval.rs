use std::ops::{Div, RangeInclusive};
use platform_version::version::PlatformVersion;
use crate::balances::credits::TokenAmount;
use crate::block::epoch::EpochIndex;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::{DistributionFunction, MAX_DISTRIBUTION_CYCLES_PARAM};
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::reward_ratio::RewardRatio;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use crate::ProtocolError;
use chrono::{Utc, TimeZone};
#[cfg(feature = "token-reward-explanations")]
use chrono_tz::Tz;

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
#[cfg(feature = "token-reward-explanations")]
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
    /// Number of steps calculated
    pub steps_count: u64,
    /// Is this the first claim
    pub is_first_claim: bool,
    /// Any special conditions or optimizations applied
    pub optimization_notes: Vec<String>,
}

/// Helper function to format numbers with thousand separators
fn format_number_with_separators(num: u64) -> String {
    let num_str = num.to_string();
    let chars: Vec<char> = num_str.chars().collect();
    let len = chars.len();

    if len <= 3 {
        return num_str;
    }

    let mut result = String::new();
    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*ch);
    }

    result
}

/// Helper function to format token amounts with decimal offset
fn format_token_amount(amount: TokenAmount, decimal_offset: u8) -> String {
    if decimal_offset == 0 {
        return format_number_with_separators(amount);
    }

    let divisor = 10u64.pow(decimal_offset as u32);
    let whole = amount / divisor;
    let fraction = amount % divisor;

    if fraction == 0 {
        format_number_with_separators(whole)
    } else {
        // Format with appropriate decimal places, removing trailing zeros
        let fraction_str = format!("{:0width$}", fraction, width = decimal_offset as usize);
        let trimmed = fraction_str.trim_end_matches('0');
        if trimmed.is_empty() {
            format_number_with_separators(whole)
        } else {
            format!("{}.{}", format_number_with_separators(whole), trimmed)
        }
    }
}

/// Helper function to get singular or plural form
fn pluralize(count: u64, singular: &str, plural: &str) -> String {
    if count == 1 {
        singular.to_string()
    } else {
        plural.to_string()
    }
}

/// Helper function to format token amount with proper pluralization
fn format_token_amount_with_plural(amount: TokenAmount, decimal_offset: u8) -> String {
    let amount_str = format_token_amount(amount, decimal_offset);
    let divisor = 10u64.pow(decimal_offset as u32);

    // Check if the amount is exactly 1 (considering decimals)
    let is_one = amount == divisor;

    if is_one {
        format!("{} token", amount_str)
    } else {
        format!("{} tokens", amount_str)
    }
}

/// Helper function to check if we're dealing with time-based moments
fn is_time_based(moment: &RewardDistributionMoment) -> bool {
    matches!(moment, RewardDistributionMoment::TimeBasedMoment(_))
}

/// Helper function to format timestamp to human-readable date
#[cfg(feature = "token-reward-explanations")]
fn format_timestamp_to_date(timestamp_ms: u64, time_zone: &str) -> String {
    let datetime = Utc.timestamp_millis_opt(timestamp_ms as i64).unwrap();
    let utc_formatted = datetime
        .format("%A %B %-d %Y at %-I:%M:%S %p UTC")
        .to_string();

    // Try to get local time if timezone is provided
    if time_zone != "UTC" && !time_zone.is_empty() {
        // Try to parse as a timezone name (e.g., "America/New_York")
        if let Ok(tz) = time_zone.parse::<Tz>() {
            let local_datetime = datetime.with_timezone(&tz);
            let local_formatted = local_datetime.format("%-I:%M:%S %p local time").to_string();
            let offset = local_datetime.offset().to_string();
            format!("{} ({}, {})", utc_formatted, local_formatted, offset)
        } else if let Ok(offset_hours) = time_zone.parse::<f32>() {
            // Handle numeric offset like "+5.5" for India or "-8" for PST
            let offset_seconds = (offset_hours * 3600.0) as i32;
            if let Some(fixed_offset) = chrono::FixedOffset::east_opt(offset_seconds) {
                let local_datetime = datetime.with_timezone(&fixed_offset);
                let local_formatted = local_datetime.format("%-I:%M:%S %p local time").to_string();
                format!("{} ({})", utc_formatted, local_formatted)
            } else {
                utc_formatted
            }
        } else {
            // If we can't parse the timezone, just return UTC
            utc_formatted
        }
    } else {
        utc_formatted
    }
}

#[cfg(feature = "token-reward-explanations")]
impl IntervalEvaluationExplanation {
    /// Returns a short explanation of the evaluation result
    pub fn short_explanation(
        &self,
        decimal_offset: u8,
        platform_version: &PlatformVersion,
        time_zone: &str,
    ) -> String {
        match &self.distribution_function {
            DistributionFunction::FixedAmount { amount } => {
                let max_claim_text = if self.steps_count == MAX_DISTRIBUTION_CYCLES_PARAM {
                    " (maximum per claim)"
                } else {
                    ""
                };
                let amount_str = format_token_amount(*amount, decimal_offset);
                let interval_word = pluralize(self.steps_count, "interval", "intervals");
                match (
                    &self.interval_start_excluded,
                    &self.interval_end_included,
                    &self.step,
                ) {
                    (
                        RewardDistributionMoment::EpochBasedMoment(start_epoch),
                        RewardDistributionMoment::EpochBasedMoment(end_epoch),
                        RewardDistributionMoment::EpochBasedMoment(_),
                    ) => {
                        if self.is_first_claim {
                            format!(
                                "This token distributes a fixed amount of {} every epoch. \
                                The token contract was registered before epoch {} \
                                and we are currently in epoch {}, the last epoch you can claim \
                                would be {}, you therefore have {} {} of rewards{}. \
                                {} × {} = {}",
                                format_token_amount_with_plural(*amount, decimal_offset),
                                start_epoch.saturating_add(1),
                                end_epoch.saturating_add(1),
                                end_epoch,
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_number_with_separators(self.steps_count),
                                amount_str,
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        } else {
                            format!(
                                "This token distributes a fixed amount of {} every epoch. \
                                The last claim was for epoch {} and we are currently in epoch {}, \
                                the last epoch you can claim would be {}, you therefore have {} \
                                {} of rewards{}. {} × {} = {}",
                                format_token_amount_with_plural(*amount, decimal_offset),
                                start_epoch,
                                end_epoch.saturating_add(1),
                                end_epoch,
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                self.steps_count,
                                amount_str,
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        }
                    }
                    _ => match (&self.interval_start_excluded, &self.interval_end_included) {
                        (
                            RewardDistributionMoment::TimeBasedMoment(start_time),
                            RewardDistributionMoment::TimeBasedMoment(end_time),
                        ) => {
                            if self.is_first_claim {
                                format!(
                                        "This token distributes a fixed amount of {} at each interval. \
                                        The token contract was registered before {} and we are \
                                        currently at {}, you have {} {} of rewards{}. \
                                        {} × {} = {}",
                                        format_token_amount_with_plural(*amount, decimal_offset),
                                        format_timestamp_to_date(start_time + 1, time_zone),
                                        format_timestamp_to_date(end_time + 1, time_zone),
                                        self.steps_count,
                                        interval_word,
                                        max_claim_text,
                                        self.steps_count,
                                        amount_str,
                                        format_token_amount_with_plural(self.total_amount, decimal_offset)
                                    )
                            } else {
                                format!(
                                        "This token distributes a fixed amount of {} at each interval. \
                                        The last claim was on {} and we are currently at {}, \
                                        you have {} {} of rewards{}. {} × {} = {}",
                                        format_token_amount_with_plural(*amount, decimal_offset),
                                        format_timestamp_to_date(*start_time, time_zone),
                                        format_timestamp_to_date(end_time + 1, time_zone),
                                        self.steps_count,
                                        interval_word,
                                        max_claim_text,
                                        self.steps_count,
                                        amount_str,
                                        format_token_amount_with_plural(self.total_amount, decimal_offset)
                                    )
                            }
                        }
                        _ => {
                            let (unit, current) = match &self.interval_end_included {
                                RewardDistributionMoment::BlockBasedMoment(block) => {
                                    ("block", block + 1)
                                }
                                _ => ("period", 0),
                            };
                            if self.is_first_claim {
                                let registration_text = if unit == "block" {
                                    format!(
                                        "The token contract was registered on {} {}",
                                        unit,
                                        self.interval_start_excluded.to_u64()
                                    )
                                } else {
                                    format!(
                                        "The token contract was registered before {} {}",
                                        unit,
                                        self.interval_start_excluded.to_u64() + 1
                                    )
                                };
                                format!(
                                    "This token distributes a fixed amount of {} every {}. \
                                        {} and we are \
                                        currently at {} {}, you have {} {} of rewards{}. \
                                        {} × {} = {}",
                                    format_token_amount_with_plural(*amount, decimal_offset),
                                    unit,
                                    registration_text,
                                    unit,
                                    current,
                                    self.steps_count,
                                    interval_word,
                                    max_claim_text,
                                    format_number_with_separators(self.steps_count),
                                    amount_str,
                                    format_token_amount_with_plural(
                                        self.total_amount,
                                        decimal_offset
                                    )
                                )
                            } else {
                                format!(
                                        "This token distributes a fixed amount of {} every {}. \
                                        The last claim was for {} {} and we are currently at {} {}, \
                                        you have {} {} of rewards{}. {} × {} = {}",
                                        format_token_amount_with_plural(*amount, decimal_offset),
                                        unit,
                                        unit,
                                        self.interval_start_excluded.to_u64(),
                                        unit,
                                        current,
                                        self.steps_count,
                                        interval_word,
                                        max_claim_text,
                                        format_number_with_separators(self.steps_count),
                                        amount_str,
                                        format_token_amount_with_plural(self.total_amount, decimal_offset)
                                    )
                            }
                        }
                    },
                }
            }
            DistributionFunction::Random { min, max } => {
                let interval_word = pluralize(self.steps_count, "interval", "intervals");
                let max_claim_text = if self.steps_count
                    == platform_version.system_limits.max_token_redemption_cycles as u64
                {
                    " (maximum per claim)"
                } else {
                    ""
                };

                match (&self.interval_start_excluded, &self.interval_end_included) {
                    (
                        RewardDistributionMoment::TimeBasedMoment(start_time),
                        RewardDistributionMoment::TimeBasedMoment(end_time),
                    ) => {
                        if self.is_first_claim {
                            format!(
                                "This token distributes a random amount between {} and {} at each interval. \
                                The token contract was registered before {} and we are currently at {}, \
                                you have {} {}{} of rewards. The exact amount is randomly determined \
                                for each interval, Total rewards: {}",
                                format_token_amount_with_plural(*min, decimal_offset),
                                format_token_amount_with_plural(*max, decimal_offset),
                                format_timestamp_to_date(start_time + 1, time_zone),
                                format_timestamp_to_date(end_time + 1, time_zone),
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        } else {
                            format!(
                                "This token distributes a random amount between {} and {} at each interval. \
                                The last claim was on {} and we are currently at {}, \
                                you have {} {}{} of rewards. The exact amount is randomly determined \
                                for each interval, Total rewards: {}",
                                format_token_amount_with_plural(*min, decimal_offset),
                                format_token_amount_with_plural(*max, decimal_offset),
                                format_timestamp_to_date(*start_time, time_zone),
                                format_timestamp_to_date(end_time + 1, time_zone),
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        }
                    }
                    _ => {
                        let period_unit = match &self.step {
                            RewardDistributionMoment::EpochBasedMoment(_) => "epoch",
                            RewardDistributionMoment::BlockBasedMoment(_) => "block",
                            _ => "period",
                        };
                        if self.is_first_claim {
                            let registration_text = if period_unit == "block" {
                                format!(
                                    "The token contract was registered on {} {}",
                                    period_unit,
                                    self.interval_start_excluded.to_u64()
                                )
                            } else {
                                format!(
                                    "The token contract was registered before {} {}",
                                    period_unit,
                                    self.interval_start_excluded.to_u64() + 1
                                )
                            };
                            format!(
                                "This token distributes a random amount between {} and {} per {}. \
                                {} and we are currently at {} {}, \
                                you have {} {}{} of rewards. The exact amount is randomly determined \
                                for each interval, Total rewards: {}",
                                format_token_amount_with_plural(*min, decimal_offset),
                                format_token_amount_with_plural(*max, decimal_offset),
                                period_unit,
                                registration_text,
                                period_unit,
                                self.interval_end_included.to_u64() + 1,
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        } else {
                            format!(
                                "This token distributes a random amount between {} and {} per {}. \
                                The last claim was for {} {} and we are currently at {} {}, \
                                you have {} {}{} of rewards. The exact amount is randomly determined \
                                for each interval, Total rewards: {}",
                                format_token_amount_with_plural(*min, decimal_offset),
                                format_token_amount_with_plural(*max, decimal_offset),
                                period_unit,
                                period_unit,
                                self.interval_start_excluded.to_u64(),
                                period_unit,
                                self.interval_end_included.to_u64() + 1,
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        }
                    }
                }
            }

            DistributionFunction::StepDecreasingAmount {
                step_count,
                decrease_per_interval_numerator,
                decrease_per_interval_denominator,
                distribution_start_amount,
                trailing_distribution_interval_amount,
                ..
            } => {
                let period_unit_plural = match &self.step {
                    RewardDistributionMoment::EpochBasedMoment(_) => "epochs",
                    RewardDistributionMoment::BlockBasedMoment(_) => "blocks",
                    _ => "intervals",
                };

                let decrease_percentage = (*decrease_per_interval_numerator as f64
                    / *decrease_per_interval_denominator as f64)
                    * 100.0;
                let interval_word = pluralize(self.steps_count, "interval", "intervals");
                let max_claim_text = if self.steps_count
                    == platform_version.system_limits.max_token_redemption_cycles as u64
                {
                    " (maximum per claim)"
                } else {
                    ""
                };

                match (
                    &self.interval_start_excluded,
                    &self.interval_end_included,
                    &self.step,
                ) {
                    (
                        RewardDistributionMoment::TimeBasedMoment(start_time),
                        RewardDistributionMoment::TimeBasedMoment(end_time),
                        RewardDistributionMoment::TimeBasedMoment(_),
                    ) => {
                        if self.is_first_claim {
                            format!(
                                "This token starts distributing {} and decreases by {:.1}% every {} intervals. \
                                The token contract was registered on {} and we are currently at {}, \
                                you have {} {}{} of rewards. After all decreasing steps, it distributes {} \
                                per interval. Total rewards: {}",
                                format_token_amount_with_plural(*distribution_start_amount, decimal_offset),
                                decrease_percentage,
                                format_number_with_separators(*step_count as u64),
                                format_timestamp_to_date(*start_time, time_zone),
                                format_timestamp_to_date(*end_time, time_zone),
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(*trailing_distribution_interval_amount, decimal_offset),
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        } else {
                            format!(
                                "This token starts distributing {} and decreases by {:.1}% every {} intervals. \
                                The last claim was on {} and we are currently at {}, \
                                you have {} {}{} of rewards. After all decreasing steps, it distributes {} \
                                per interval. Total rewards: {}",
                                format_token_amount_with_plural(*distribution_start_amount, decimal_offset),
                                decrease_percentage,
                                format_number_with_separators(*step_count as u64),
                                format_timestamp_to_date(*start_time, time_zone),
                                format_timestamp_to_date(*end_time, time_zone),
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(*trailing_distribution_interval_amount, decimal_offset),
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        }
                    }
                    (
                        RewardDistributionMoment::BlockBasedMoment(start_block),
                        RewardDistributionMoment::BlockBasedMoment(end_block),
                        RewardDistributionMoment::BlockBasedMoment(_),
                    ) => {
                        if self.is_first_claim {
                            format!(
                                "This token starts distributing {} and decreases by {:.1}% every {} blocks. \
                                The token contract was registered on block {} and we are currently at block {}, \
                                you have {} {}{} of rewards. After all decreasing steps, it distributes {} \
                                per interval. Total rewards: {}",
                                format_token_amount_with_plural(*distribution_start_amount, decimal_offset),
                                decrease_percentage,
                                format_number_with_separators(*step_count as u64),
                                start_block,
                                end_block + 1,
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(*trailing_distribution_interval_amount, decimal_offset),
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        } else {
                            format!(
                                "This token starts distributing {} and decreases by {:.1}% every {} blocks. \
                                The last claim was for block {} and we are currently at block {}, \
                                you have {} {}{} of rewards. After all decreasing steps, it distributes {} \
                                per interval. Total rewards: {}",
                                format_token_amount_with_plural(*distribution_start_amount, decimal_offset),
                                decrease_percentage,
                                format_number_with_separators(*step_count as u64),
                                start_block,
                                end_block + 1,
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(*trailing_distribution_interval_amount, decimal_offset),
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        }
                    }
                    (
                        RewardDistributionMoment::EpochBasedMoment(start_epoch),
                        RewardDistributionMoment::EpochBasedMoment(end_epoch),
                        RewardDistributionMoment::EpochBasedMoment(_),
                    ) => {
                        if self.is_first_claim {
                            format!(
                                "This token starts distributing {} and decreases by {:.1}% every {} epochs. \
                                The token contract was registered before epoch {} and we are currently at epoch {}, \
                                you have {} {}{} of rewards. After all decreasing steps, it distributes {} \
                                per interval. Total rewards: {}",
                                format_token_amount_with_plural(*distribution_start_amount, decimal_offset),
                                decrease_percentage,
                                format_number_with_separators(*step_count as u64),
                                start_epoch + 1,
                                end_epoch + 1,
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(*trailing_distribution_interval_amount, decimal_offset),
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        } else {
                            format!(
                                "This token starts distributing {} and decreases by {:.1}% every {} epochs. \
                                The last claim was for epoch {} and we are currently in epoch {}, \
                                you have {} {}{} of rewards. After all decreasing steps, it distributes {} \
                                per interval. Total rewards: {}",
                                format_token_amount_with_plural(*distribution_start_amount, decimal_offset),
                                decrease_percentage,
                                format_number_with_separators(*step_count as u64),
                                start_epoch,
                                end_epoch + 1,
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(*trailing_distribution_interval_amount, decimal_offset),
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        }
                    }
                    _ => {
                        // Fallback for mixed types or unknown types
                        if self.is_first_claim {
                            format!(
                                "This token starts distributing {} and decreases by {:.1}% every {} {}. \
                                The token contract was registered before interval {} and we are currently at interval {}, \
                                you have {} {}{} of rewards. After all decreasing steps, it distributes {} \
                                per interval. Total rewards: {}",
                                format_token_amount_with_plural(*distribution_start_amount, decimal_offset),
                                decrease_percentage,
                                step_count,
                                period_unit_plural,
                                self.interval_start_excluded.to_u64() + 1,
                                self.interval_end_included.to_u64() + 1,
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(*trailing_distribution_interval_amount, decimal_offset),
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        } else {
                            format!(
                                "This token starts distributing {} and decreases by {:.1}% every {} {}. \
                                The last claim was for interval {} and we are currently at interval {}, \
                                you have {} {}{} of rewards. After all decreasing steps, it distributes {} \
                                per interval. Total rewards: {}",
                                format_token_amount_with_plural(*distribution_start_amount, decimal_offset),
                                decrease_percentage,
                                step_count,
                                period_unit_plural,
                                self.interval_start_excluded.to_u64(),
                                self.interval_end_included.to_u64() + 1,
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(*trailing_distribution_interval_amount, decimal_offset),
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        }
                    }
                }
            }

            DistributionFunction::Stepwise(steps_map) => {
                let steps_desc = steps_map
                    .iter()
                    .take(3)
                    .map(|(k, v)| {
                        format!(
                            "{} from interval {}",
                            format_token_amount_with_plural(*v, decimal_offset),
                            k
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                let steps_preview = if steps_map.len() > 3 {
                    let step_word = pluralize((steps_map.len() - 3) as u64, "step", "steps");
                    format!(
                        "{}, and {} more {}",
                        steps_desc,
                        steps_map.len() - 3,
                        step_word
                    )
                } else {
                    steps_desc
                };

                let interval_word = pluralize(self.steps_count, "interval", "intervals");
                let max_claim_text = if self.steps_count
                    == platform_version.system_limits.max_token_redemption_cycles as u64
                {
                    " (maximum per claim)"
                } else {
                    ""
                };

                match (&self.interval_start_excluded, &self.interval_end_included) {
                    (
                        RewardDistributionMoment::TimeBasedMoment(start_time),
                        RewardDistributionMoment::TimeBasedMoment(end_time),
                    ) => {
                        if self.is_first_claim {
                            format!(
                                "This token distributes tokens in predefined steps: {}. \
                                The token contract was registered before {} and we are currently at {}, \
                                you have {} {}{} of rewards. Total rewards: {}",
                                steps_preview,
                                format_timestamp_to_date(start_time + 1, time_zone),
                                format_timestamp_to_date(end_time + 1, time_zone),
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        } else {
                            format!(
                                "This token distributes tokens in predefined steps: {}. \
                                The last claim was on {} and we are currently at {}, \
                                you have {} {}{} of rewards. Total rewards: {}",
                                steps_preview,
                                format_timestamp_to_date(*start_time, time_zone),
                                format_timestamp_to_date(end_time + 1, time_zone),
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        }
                    }
                    (
                        RewardDistributionMoment::BlockBasedMoment(start_block),
                        RewardDistributionMoment::BlockBasedMoment(end_block),
                    ) => {
                        if self.is_first_claim {
                            format!(
                                "This token distributes tokens in predefined steps: {}. \
                                The token contract was registered on block {} and we are currently at block {}, \
                                you have {} {}{} of rewards. Total rewards: {}",
                                steps_preview,
                                start_block + 1,
                                end_block + 1,
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        } else {
                            format!(
                                "This token distributes tokens in predefined steps: {}. \
                                The last claim was for block {} and we are currently at block {}, \
                                you have {} {}{} of rewards. Total rewards: {}",
                                steps_preview,
                                start_block,
                                end_block + 1,
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        }
                    }
                    (
                        RewardDistributionMoment::EpochBasedMoment(start_epoch),
                        RewardDistributionMoment::EpochBasedMoment(end_epoch),
                    ) => {
                        if self.is_first_claim {
                            format!(
                                "This token distributes tokens in predefined steps: {}. \
                                The token contract was registered before epoch {} and we are currently at epoch {}, \
                                you have {} {}{} of rewards. Total rewards: {}",
                                steps_preview,
                                start_epoch + 1,
                                end_epoch + 1,
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        } else {
                            format!(
                                "This token distributes tokens in predefined steps: {}. \
                                The last claim was for epoch {} and we are currently in epoch {}, \
                                you have {} {}{} of rewards. Total rewards: {}",
                                steps_preview,
                                start_epoch,
                                end_epoch + 1,
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        }
                    }
                    _ => {
                        // Fallback for mixed types
                        if self.is_first_claim {
                            format!(
                                "This token distributes tokens in predefined steps: {}. \
                                The token contract was registered before interval {} and we are currently at interval {}, \
                                you have {} {}{} of rewards. Total rewards: {}",
                                steps_preview,
                                self.interval_start_excluded.to_u64() + 1,
                                self.interval_end_included.to_u64() + 1,
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        } else {
                            format!(
                                "This token distributes tokens in predefined steps: {}. \
                                The last claim was for interval {} and we are currently at interval {}, \
                                you have {} {}{} of rewards. Total rewards: {}",
                                steps_preview,
                                self.interval_start_excluded.to_u64(),
                                self.interval_end_included.to_u64() + 1,
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        }
                    }
                }
            }

            DistributionFunction::Linear {
                a,
                d,
                starting_amount,
                ..
            } => {
                let interval_word = pluralize(self.steps_count, "interval", "intervals");
                let max_claim_text = if self.steps_count
                    == platform_version.system_limits.max_token_redemption_cycles as u64
                {
                    " (maximum per claim)"
                } else {
                    ""
                };

                let change_desc = if *a > 0 {
                    let token_word = if *a == 1 && *d == 1 {
                        "token"
                    } else {
                        "tokens"
                    };
                    format!("increases by {}/{} {}", a, d, token_word)
                } else if *a < 0 {
                    let abs_a = -a;
                    let token_word = if abs_a == 1 && *d == 1 {
                        "token"
                    } else {
                        "tokens"
                    };
                    format!("decreases by {}/{} {}", abs_a, d, token_word)
                } else {
                    "remains constant".to_string()
                };

                match (&self.interval_start_excluded, &self.interval_end_included) {
                    (
                        RewardDistributionMoment::TimeBasedMoment(start_time),
                        RewardDistributionMoment::TimeBasedMoment(end_time),
                    ) => {
                        if self.is_first_claim {
                            format!(
                                "This token starts at {} and {} at each interval. \
                                The token contract was registered before {} and we are currently at {}, \
                                you have {} {}{} of rewards. Total rewards: {}",
                                format_token_amount_with_plural(*starting_amount, decimal_offset),
                                change_desc,
                                format_timestamp_to_date(start_time + 1, time_zone),
                                format_timestamp_to_date(end_time + 1, time_zone),
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        } else {
                            format!(
                                "This token starts at {} and {} at each interval. \
                                The last claim was on {} and we are currently at {}, \
                                you have {} {}{} of rewards. Total rewards: {}",
                                format_token_amount_with_plural(*starting_amount, decimal_offset),
                                change_desc,
                                format_timestamp_to_date(*start_time, time_zone),
                                format_timestamp_to_date(end_time + 1, time_zone),
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        }
                    }
                    _ => {
                        let period_unit = match &self.step {
                            RewardDistributionMoment::EpochBasedMoment(_) => "epoch",
                            RewardDistributionMoment::BlockBasedMoment(_) => "block",
                            _ => "interval",
                        };
                        if self.is_first_claim {
                            let registration_text = if period_unit == "block" {
                                format!(
                                    "The token contract was registered on {} {}",
                                    period_unit,
                                    self.interval_start_excluded.to_u64()
                                )
                            } else {
                                format!(
                                    "The token contract was registered before {} {}",
                                    period_unit,
                                    self.interval_start_excluded.to_u64() + 1
                                )
                            };
                            format!(
                                "This token starts at {} and {} per {}. \
                                {} and we are currently at {} {}, \
                                you have {} {}{} of rewards. Total rewards: {}",
                                format_token_amount_with_plural(*starting_amount, decimal_offset),
                                change_desc,
                                period_unit,
                                registration_text,
                                period_unit,
                                self.interval_end_included.to_u64() + 1,
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        } else {
                            format!(
                                "This token starts at {} and {} per {}. \
                                The last claim was for {} {} and we are currently at {} {}, \
                                you have {} {}{} of rewards. Total rewards: {}",
                                format_token_amount_with_plural(*starting_amount, decimal_offset),
                                change_desc,
                                period_unit,
                                period_unit,
                                self.interval_start_excluded.to_u64(),
                                period_unit,
                                self.interval_end_included.to_u64() + 1,
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        }
                    }
                }
            }

            DistributionFunction::Polynomial { a, m, b, .. } => {
                let period_unit = match &self.step {
                    RewardDistributionMoment::EpochBasedMoment(_) => "epoch",
                    RewardDistributionMoment::BlockBasedMoment(_) => "block",
                    RewardDistributionMoment::TimeBasedMoment(_) => "time period",
                };

                let interval_word = pluralize(self.steps_count, "interval", "intervals");
                let max_claim_text = if self.steps_count
                    == platform_version.system_limits.max_token_redemption_cycles as u64
                {
                    " (maximum per claim)"
                } else {
                    ""
                };

                let growth_desc = if *a > 0 && *m > 0 {
                    "increases at an accelerating rate"
                } else if *a > 0 && *m < 0 {
                    "starts high and gradually declines"
                } else if *a < 0 && *m > 0 {
                    "decreases at an accelerating rate"
                } else {
                    "follows a polynomial curve"
                };

                let base_amount = if *b > 0 {
                    format!(
                        " with a base amount of {}",
                        format_token_amount_with_plural(*b, decimal_offset)
                    )
                } else {
                    String::new()
                };

                if self.is_first_claim {
                    let registration_text = if period_unit == "block" {
                        format!(
                            "The token contract was registered on {} {}",
                            period_unit,
                            self.interval_start_excluded.to_u64()
                        )
                    } else {
                        format!(
                            "The token contract was registered before {} {}",
                            period_unit,
                            self.interval_start_excluded.to_u64() + 1
                        )
                    };
                    format!(
                        "This token follows a polynomial distribution that {}{}. \
                        {} and we are currently at {} {}, \
                        you have {} {}{} of rewards. Total rewards: {}",
                        growth_desc,
                        base_amount,
                        registration_text,
                        period_unit,
                        self.interval_end_included.to_u64() + 1,
                        self.steps_count,
                        interval_word,
                        max_claim_text,
                        format_token_amount_with_plural(self.total_amount, decimal_offset)
                    )
                } else {
                    format!(
                        "This token follows a polynomial distribution that {}{}. \
                        The last claim was for {} {} and we are currently at {} {}, \
                        you have {} {}{} of rewards. Total rewards: {}",
                        growth_desc,
                        base_amount,
                        period_unit,
                        self.interval_start_excluded.to_u64(),
                        period_unit,
                        self.interval_end_included.to_u64() + 1,
                        self.steps_count,
                        interval_word,
                        max_claim_text,
                        format_token_amount_with_plural(self.total_amount, decimal_offset)
                    )
                }
            }

            DistributionFunction::Exponential { a, m, b, .. } => {
                let period_unit = match &self.step {
                    RewardDistributionMoment::EpochBasedMoment(_) => "epoch",
                    RewardDistributionMoment::BlockBasedMoment(_) => "block",
                    RewardDistributionMoment::TimeBasedMoment(_) => "time period",
                };

                let interval_word = pluralize(self.steps_count, "interval", "intervals");
                let max_claim_text = if self.steps_count
                    == platform_version.system_limits.max_token_redemption_cycles as u64
                {
                    " (maximum per claim)"
                } else {
                    ""
                };

                let growth_desc = if *m > 0 {
                    "grows exponentially"
                } else {
                    "decays exponentially"
                };

                if self.is_first_claim {
                    let registration_text = if period_unit == "block" {
                        format!(
                            "The token contract was registered on {} {}",
                            period_unit,
                            self.interval_start_excluded.to_u64()
                        )
                    } else {
                        format!(
                            "The token contract was registered before {} {}",
                            period_unit,
                            self.interval_start_excluded.to_u64() + 1
                        )
                    };
                    format!(
                        "This token {} starting from a base of {} with scaling factor {}. \
                        {} and we are currently at {} {}, \
                        you have {} {}{} of rewards. Total rewards: {}",
                        growth_desc,
                        format_token_amount_with_plural(*b, decimal_offset),
                        a,
                        registration_text,
                        period_unit,
                        self.interval_end_included.to_u64() + 1,
                        self.steps_count,
                        interval_word,
                        max_claim_text,
                        format_token_amount_with_plural(self.total_amount, decimal_offset)
                    )
                } else {
                    format!(
                        "This token {} starting from a base of {} with scaling factor {}. \
                        The last claim was for {} {} and we are currently at {} {}, \
                        you have {} {}{} of rewards. Total rewards: {}",
                        growth_desc,
                        format_token_amount_with_plural(*b, decimal_offset),
                        a,
                        period_unit,
                        self.interval_start_excluded.to_u64(),
                        period_unit,
                        self.interval_end_included.to_u64() + 1,
                        self.steps_count,
                        interval_word,
                        max_claim_text,
                        format_token_amount_with_plural(self.total_amount, decimal_offset)
                    )
                }
            }

            DistributionFunction::Logarithmic { a, b, .. } => {
                let period_unit = match &self.step {
                    RewardDistributionMoment::EpochBasedMoment(_) => "epoch",
                    RewardDistributionMoment::BlockBasedMoment(_) => "block",
                    RewardDistributionMoment::TimeBasedMoment(_) => "time period",
                };

                let interval_word = pluralize(self.steps_count, "interval", "intervals");
                let max_claim_text = if self.steps_count
                    == platform_version.system_limits.max_token_redemption_cycles as u64
                {
                    " (maximum per claim)"
                } else {
                    ""
                };

                let growth_desc = if *a > 0 {
                    "increases at a slowing rate"
                } else {
                    "decreases at a slowing rate"
                };

                let base_amount = if *b > 0 {
                    format!(
                        " with a base amount of {}",
                        format_token_amount_with_plural(*b, decimal_offset)
                    )
                } else {
                    String::new()
                };

                if self.is_first_claim {
                    let registration_text = if period_unit == "block" {
                        format!(
                            "The token contract was registered on {} {}",
                            period_unit,
                            self.interval_start_excluded.to_u64()
                        )
                    } else {
                        format!(
                            "The token contract was registered before {} {}",
                            period_unit,
                            self.interval_start_excluded.to_u64() + 1
                        )
                    };
                    format!(
                        "This token follows a logarithmic distribution that {}{}. \
                        {} and we are currently at {} {}, \
                        you have {} {}{} of rewards. Total rewards: {}",
                        growth_desc,
                        base_amount,
                        registration_text,
                        period_unit,
                        self.interval_end_included.to_u64() + 1,
                        self.steps_count,
                        interval_word,
                        max_claim_text,
                        format_token_amount_with_plural(self.total_amount, decimal_offset)
                    )
                } else {
                    format!(
                        "This token follows a logarithmic distribution that {}{}. \
                        The last claim was for {} {} and we are currently at {} {}, \
                        you have {} {}{} of rewards. Total rewards: {}",
                        growth_desc,
                        base_amount,
                        period_unit,
                        self.interval_start_excluded.to_u64(),
                        period_unit,
                        self.interval_end_included.to_u64() + 1,
                        self.steps_count,
                        interval_word,
                        max_claim_text,
                        format_token_amount_with_plural(self.total_amount, decimal_offset)
                    )
                }
            }

            DistributionFunction::InvertedLogarithmic { b, .. } => {
                let period_unit = match &self.step {
                    RewardDistributionMoment::EpochBasedMoment(_) => "epoch",
                    RewardDistributionMoment::BlockBasedMoment(_) => "block",
                    RewardDistributionMoment::TimeBasedMoment(_) => "time period",
                };

                let interval_word = pluralize(self.steps_count, "interval", "intervals");
                let max_claim_text = if self.steps_count
                    == platform_version.system_limits.max_token_redemption_cycles as u64
                {
                    " (maximum per claim)"
                } else {
                    ""
                };

                let base_amount = if *b > 0 {
                    format!(
                        ", with a base amount of {}",
                        format_token_amount_with_plural(*b, decimal_offset)
                    )
                } else {
                    String::new()
                };

                if self.is_first_claim {
                    match &self.interval_start_excluded {
                        RewardDistributionMoment::TimeBasedMoment(start_time) => {
                            let current_time = match &self.interval_end_included {
                                RewardDistributionMoment::TimeBasedMoment(end_time) => end_time + 1,
                                _ => 0,
                            };
                            format!(
                                "This token starts with high rewards that gradually decrease following an inverted \
                                logarithmic curve{}. The token contract was registered \
                                before {} and we are currently at {}, you have {} {}{} of rewards. Total rewards: {}",
                                base_amount,
                                format_timestamp_to_date(start_time + 1, time_zone),
                                format_timestamp_to_date(current_time, time_zone),
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        }
                        _ => {
                            let registration_text = if period_unit == "block" {
                                format!(
                                    "The token contract was registered on {} {}",
                                    period_unit,
                                    self.interval_start_excluded.to_u64()
                                )
                            } else {
                                format!(
                                    "The token contract was registered before {} {}",
                                    period_unit,
                                    self.interval_start_excluded.to_u64() + 1
                                )
                            };
                            format!(
                                "This token starts with high rewards that gradually decrease following an inverted \
                                logarithmic curve{}. {} and we are currently at {} {}, you have {} {}{} of rewards. Total rewards: {}",
                                base_amount,
                                registration_text,
                                period_unit,
                                self.interval_end_included.to_u64() + 1,
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        }
                    }
                } else {
                    match &self.interval_start_excluded {
                        RewardDistributionMoment::TimeBasedMoment(start_time) => {
                            let current_time = match &self.interval_end_included {
                                RewardDistributionMoment::TimeBasedMoment(end_time) => end_time + 1,
                                _ => 0,
                            };
                            format!(
                                "This token starts with high rewards that gradually decrease following an inverted \
                                logarithmic curve{}. The last claim was on {} and we are \
                                currently at {}, you have {} {}{} of rewards. Total rewards: {}",
                                base_amount,
                                format_timestamp_to_date(*start_time, time_zone),
                                format_timestamp_to_date(current_time, time_zone),
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        }
                        _ => {
                            format!(
                                "This token starts with high rewards that gradually decrease following an inverted \
                                logarithmic curve{}. The last claim was for {} {} and we are \
                                currently at {} {}, you have {} {}{} of rewards. Total rewards: {}",
                                base_amount,
                                period_unit,
                                self.interval_start_excluded.to_u64(),
                                period_unit,
                                self.interval_end_included.to_u64() + 1,
                                self.steps_count,
                                interval_word,
                                max_claim_text,
                                format_token_amount_with_plural(self.total_amount, decimal_offset)
                            )
                        }
                    }
                }
            }
        }
    }

    /// Returns a detailed explanation with all steps and calculations
    pub fn detailed_explanation(&self) -> String {
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

        if matches!(
            self.distribution_function,
            DistributionFunction::FixedAmount { .. }
        ) {
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
        } else if matches!(
            self.distribution_function,
            DistributionFunction::FixedAmount { .. }
        ) {
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
        if matches!(
            self.distribution_function,
            DistributionFunction::FixedAmount { .. }
        ) {
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
    /// - `is_first_claim`: Whether this is the first claim for this token.
    ///
    /// # Returns
    ///
    /// - `Ok(IntervalEvaluationExplanation)` containing the result and detailed explanation.
    /// - `Err(ProtocolError)` on mismatched types, zero steps, or overflow.
    #[cfg(feature = "token-reward-explanations")]
    pub fn evaluate_interval_with_explanation<F>(
        &self,
        distribution_start: RewardDistributionMoment,
        interval_start_excluded: RewardDistributionMoment,
        interval_end_included: RewardDistributionMoment,
        step: RewardDistributionMoment,
        get_epoch_reward_ratio: Option<F>,
        is_first_claim: bool,
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
            steps_count: 0,
            is_first_claim,
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
mod tests {
    use super::*;

    mod epoch_tests {
        use super::*;

        #[test]
        fn test_fixed_amount_explanation_first_claim() {
            let dist = DistributionFunction::FixedAmount { amount: 10000 };
            let start_excluded = RewardDistributionMoment::EpochBasedMoment(7555);
            let end_included = RewardDistributionMoment::EpochBasedMoment(7740);
            let step = RewardDistributionMoment::EpochBasedMoment(1);
            let distribution_start = RewardDistributionMoment::EpochBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let expected = "This token distributes a fixed amount of 10,000 tokens every epoch. \
                           The token contract was registered before epoch 7556 \
                           and we are currently in epoch 7741, the last epoch you can claim \
                           would be 7740, you therefore have 185 intervals of rewards. \
                           185 × 10,000 = 1,850,000 tokens";

            assert_eq!(
                result.short_explanation(0, PlatformVersion::latest(), "UTC"),
                expected
            );
        }

        #[test]
        fn test_fixed_amount_explanation_not_first_claim() {
            let dist = DistributionFunction::FixedAmount { amount: 10000 };
            let start_excluded = RewardDistributionMoment::EpochBasedMoment(7554);
            let end_included = RewardDistributionMoment::EpochBasedMoment(7740);
            let step = RewardDistributionMoment::EpochBasedMoment(1);
            let distribution_start = RewardDistributionMoment::EpochBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    false,
                )
                .unwrap();

            let expected = "This token distributes a fixed amount of 10,000 tokens every epoch. \
                           The last claim was for epoch 7554 and we are currently in epoch 7741, \
                           the last epoch you can claim would be 7740, you therefore have 186 \
                           intervals of rewards. 186 × 10,000 = 1,860,000 tokens";

            assert_eq!(
                result.short_explanation(0, PlatformVersion::latest(), "UTC"),
                expected
            );
        }

        #[test]
        fn test_random_explanation_first_claim() {
            let dist = DistributionFunction::Random { min: 100, max: 500 };
            let start_excluded = RewardDistributionMoment::EpochBasedMoment(50);
            let end_included = RewardDistributionMoment::EpochBasedMoment(60);
            let step = RewardDistributionMoment::EpochBasedMoment(1);
            let distribution_start = RewardDistributionMoment::EpochBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token distributes a random amount between 100 tokens and 500 tokens per epoch. \
                The token contract was registered before epoch 51 and we are currently at epoch 61, \
                you have 10 intervals of rewards. The exact amount is randomly determined \
                for each interval, Total rewards: {} tokens",
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_random_explanation_not_first_claim() {
            let dist = DistributionFunction::Random { min: 100, max: 500 };
            let start_excluded = RewardDistributionMoment::EpochBasedMoment(50);
            let end_included = RewardDistributionMoment::EpochBasedMoment(60);
            let step = RewardDistributionMoment::EpochBasedMoment(1);
            let distribution_start = RewardDistributionMoment::EpochBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    false,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token distributes a random amount between 100 tokens and 500 tokens per epoch. \
                The last claim was for epoch 50 and we are currently at epoch 61, \
                you have 10 intervals of rewards. The exact amount is randomly determined \
                for each interval, Total rewards: {} tokens",
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_step_decreasing_amount_explanation_first_claim() {
            let dist = DistributionFunction::StepDecreasingAmount {
                step_count: 210000,
                decrease_per_interval_numerator: 7,
                decrease_per_interval_denominator: 100,
                start_decreasing_offset: Some(0),
                max_interval_count: Some(64),
                distribution_start_amount: 50000,
                trailing_distribution_interval_amount: 1000,
                min_value: None,
            };

            let start_excluded = RewardDistributionMoment::EpochBasedMoment(0);
            let end_included = RewardDistributionMoment::EpochBasedMoment(10);
            let step = RewardDistributionMoment::EpochBasedMoment(1);
            let distribution_start = RewardDistributionMoment::EpochBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token starts distributing 50,000 tokens and decreases by 7.0% every 210,000 epochs. \
                The token contract was registered before epoch 1 and we are currently at epoch 11, \
                you have 10 intervals of rewards. After all decreasing steps, it distributes 1,000 tokens \
                per interval. Total rewards: {} tokens",
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_stepwise_explanation_first_claim() {
            use std::collections::BTreeMap;

            let mut steps = BTreeMap::new();
            steps.insert(0, 1000);
            steps.insert(10, 500);
            steps.insert(20, 250);

            let dist = DistributionFunction::Stepwise(steps);
            let start_excluded = RewardDistributionMoment::EpochBasedMoment(5);
            let end_included = RewardDistributionMoment::EpochBasedMoment(25);
            let step = RewardDistributionMoment::EpochBasedMoment(1);
            let distribution_start = RewardDistributionMoment::EpochBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token distributes tokens in predefined steps: 1,000 tokens from interval 0, 500 tokens from interval 10, 250 tokens from interval 20. \
                The token contract was registered before epoch 6 and we are currently at epoch 26, \
                you have 20 intervals of rewards. Total rewards: {} tokens",
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_linear_explanation_first_claim_increasing() {
            let dist = DistributionFunction::Linear {
                a: 100,
                d: 1,
                start_step: Some(0),
                starting_amount: 5000,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::EpochBasedMoment(10);
            let end_included = RewardDistributionMoment::EpochBasedMoment(20);
            let step = RewardDistributionMoment::EpochBasedMoment(1);
            let distribution_start = RewardDistributionMoment::EpochBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token starts at 5,000 tokens and increases by 100/1 tokens per epoch. \
                The token contract was registered before epoch 11 and we are currently at epoch 21, \
                you have 10 intervals of rewards. Total rewards: {} tokens",
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_polynomial_explanation_first_claim() {
            let dist = DistributionFunction::Polynomial {
                a: 10,
                d: 1,
                m: 2,
                n: 1,
                o: 0,
                start_moment: Some(0),
                b: 1000,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::EpochBasedMoment(5);
            let end_included = RewardDistributionMoment::EpochBasedMoment(15);
            let step = RewardDistributionMoment::EpochBasedMoment(1);
            let distribution_start = RewardDistributionMoment::EpochBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token follows a polynomial distribution that increases at an accelerating rate with a base amount of 1,000 tokens. \
                The token contract was registered before epoch 6 and we are currently at epoch 16, \
                you have 10 intervals of rewards. Total rewards: {} tokens",
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_exponential_explanation_first_claim() {
            let dist = DistributionFunction::Exponential {
                a: 100,
                d: 10,
                m: 2,
                n: 50,
                o: 0,
                start_moment: Some(0),
                b: 500,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::EpochBasedMoment(20);
            let end_included = RewardDistributionMoment::EpochBasedMoment(30);
            let step = RewardDistributionMoment::EpochBasedMoment(1);
            let distribution_start = RewardDistributionMoment::EpochBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token grows exponentially starting from a base of 500 tokens with scaling factor 100. \
                The token contract was registered before epoch 21 and we are currently at epoch 31, \
                you have 10 intervals of rewards. Total rewards: {} tokens",
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_logarithmic_explanation_first_claim() {
            let dist = DistributionFunction::Logarithmic {
                a: 1000,
                d: 10,
                m: 2,
                n: 1,
                o: 1,
                start_moment: Some(0),
                b: 2000,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::EpochBasedMoment(100);
            let end_included = RewardDistributionMoment::EpochBasedMoment(110);
            let step = RewardDistributionMoment::EpochBasedMoment(1);
            let distribution_start = RewardDistributionMoment::EpochBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token follows a logarithmic distribution that increases at a slowing rate with a base amount of 2,000 tokens. \
                The token contract was registered before epoch 101 and we are currently at epoch 111, \
                you have 10 intervals of rewards. Total rewards: {} tokens",
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_inverted_logarithmic_explanation_first_claim() {
            let dist = DistributionFunction::InvertedLogarithmic {
                a: 10000,
                d: 1,
                m: 1,
                n: 5000,
                o: 0,
                start_moment: Some(0),
                b: 0,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::EpochBasedMoment(50);
            let end_included = RewardDistributionMoment::EpochBasedMoment(60);
            let step = RewardDistributionMoment::EpochBasedMoment(1);
            let distribution_start = RewardDistributionMoment::EpochBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token starts with high rewards that gradually decrease following an inverted logarithmic curve. \
                The token contract was registered before epoch 51 and we are currently at epoch 61, \
                you have 10 intervals of rewards. Total rewards: {} tokens",
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
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
                    true,
                )
                .unwrap();

            let short = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let long = result.detailed_explanation();

            // Short should be a descriptive explanation
            let expected_short = "This token distributes a fixed amount of 50 tokens every epoch. \
                The token contract was registered before epoch 2 and we are currently in epoch 4, \
                the last epoch you can claim would be 3, you therefore have 2 intervals of rewards. \
                2 × 50 = 100 tokens";
            assert_eq!(short, expected_short);

            // Long should be the most detailed
            assert!(long.contains("==="));
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
                    true,
                )
                .unwrap();

            // For FixedAmount with ratio, it still uses optimization
            let explanation = result.explanation_for_step(1).unwrap();
            assert!(explanation.contains("FixedAmount optimization"));
        }

        #[test]
        fn test_singular_forms_one_epoch() {
            let dist = DistributionFunction::FixedAmount { amount: 1000 };
            let start_excluded = RewardDistributionMoment::EpochBasedMoment(100);
            let end_included = RewardDistributionMoment::EpochBasedMoment(101);
            let step = RewardDistributionMoment::EpochBasedMoment(1);
            let distribution_start = RewardDistributionMoment::EpochBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            assert!(
                actual.contains("you therefore have 1 interval of rewards"),
                "Expected singular 'interval', got: {}",
                actual
            );
            assert!(
                !actual.contains("1 intervals"),
                "Should not contain '1 intervals', got: {}",
                actual
            );
        }

        #[test]
        fn test_decimal_offset_formatting() {
            let dist = DistributionFunction::FixedAmount { amount: 520000 };
            let start_excluded = RewardDistributionMoment::EpochBasedMoment(0);
            let end_included = RewardDistributionMoment::EpochBasedMoment(1);
            let step = RewardDistributionMoment::EpochBasedMoment(1);
            let distribution_start = RewardDistributionMoment::EpochBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            // Test with decimal offset of 5
            let actual = result.short_explanation(5, PlatformVersion::latest(), "UTC");
            assert!(
                actual.contains("5.2 tokens"),
                "Expected '5.2 tokens' with decimal offset 5, got: {}",
                actual
            );

            // Test total amount formatting
            assert!(
                actual.contains("1 × 5.2 = 5.2 tokens"),
                "Expected formatted calculation, got: {}",
                actual
            );
        }

        #[test]
        fn test_singular_token_amount() {
            // Test with exactly 1 token (assuming decimal_offset=0)
            let dist = DistributionFunction::FixedAmount { amount: 1 };
            let start_excluded = RewardDistributionMoment::EpochBasedMoment(0);
            let end_included = RewardDistributionMoment::EpochBasedMoment(1);
            let step = RewardDistributionMoment::EpochBasedMoment(1);
            let distribution_start = RewardDistributionMoment::EpochBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            // Should say "1 token" not "1 token`s`"
            assert!(
                actual.contains("a fixed amount of 1 token"),
                "Expected singular 'token' for amount 1, got: {}",
                actual
            );
            assert!(
                actual.contains("1 × 1 = 1 token"),
                "Expected singular 'token' in total, got: {}",
                actual
            );

            // Test with 1 token but decimal_offset=6 (e.g., 0.000001 token)
            let result_with_decimals = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual_decimals =
                result_with_decimals.short_explanation(6, PlatformVersion::latest(), "UTC");
            assert!(
                actual_decimals.contains("0.000001 token"),
                "Expected singular 'token' for fractional amount, got: {}",
                actual_decimals
            );
        }

        #[test]
        fn test_edge_case_large_numbers_with_decimals() {
            let dist = DistributionFunction::FixedAmount { amount: 123456789 };
            let start_excluded = RewardDistributionMoment::EpochBasedMoment(0);
            let end_included = RewardDistributionMoment::EpochBasedMoment(2);
            let step = RewardDistributionMoment::EpochBasedMoment(1);
            let distribution_start = RewardDistributionMoment::EpochBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            // Test with decimal offset of 8 (common for crypto tokens)
            let actual = result.short_explanation(8, PlatformVersion::latest(), "UTC");
            assert!(
                actual.contains("1.23456789 tokens"),
                "Expected '1.23456789 tokens' with decimal offset 8, got: {}",
                actual
            );
            assert!(
                actual.contains("2 × 1.23456789 = 2.46913578 tokens"),
                "Expected formatted calculation, got: {}",
                actual
            );
        }
    }

    mod block_tests {
        use super::*;

        #[test]
        fn test_singular_token_amount_block() {
            // Test with exactly 1 token (assuming decimal_offset=0)
            let dist = DistributionFunction::FixedAmount { amount: 1 };
            let start_excluded = RewardDistributionMoment::BlockBasedMoment(0);
            let end_included = RewardDistributionMoment::BlockBasedMoment(1);
            let step = RewardDistributionMoment::BlockBasedMoment(1);
            let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            // Should say "1 token" not "1 token`s`"
            assert!(
                actual.contains("a fixed amount of 1 token"),
                "Expected singular 'token' for amount 1, got: {}",
                actual
            );
            assert!(
                actual.contains("1 × 1 = 1 token"),
                "Expected singular 'token' in total, got: {}",
                actual
            );
        }

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
                    false,
                )
                .unwrap();

            // Should have 5 steps: 10, 20, 30, 40, 50
            assert_eq!(result.steps_count, 5);
            assert_eq!(result.total_amount, 500); // 100 tokens * 5 steps
            assert!(!result.reward_ratios_applied);
            assert!(result.evaluation_steps.is_empty()); // No individual steps due to optimization

            // Test explanations
            let short = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            assert!(short.contains("fixed amount of 100 tokens"));
            assert!(short.contains("500 tokens"));

            // Test with is_first_claim = false
            let short_not_first = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            assert!(short_not_first.contains("The last claim was for"));

            let long = result.detailed_explanation();
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
                    true,
                )
                .unwrap();

            assert!(!result.reward_ratios_applied);
            assert!(!result.evaluation_steps.is_empty()); // Should have individual steps

            // Test that explanations contain function type
            let short = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            assert!(short.contains("starts at 50 tokens"));

            let long = result.detailed_explanation();
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
                    true,
                )
                .unwrap();

            assert_eq!(result.total_amount, 0);
            assert_eq!(result.steps_count, 0);
            assert!(result
                .optimization_notes
                .iter()
                .any(|note| note.contains("Start >= End")));

            // Test that empty range still generates explanation
            let short = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = "This token distributes a fixed amount of 100 tokens every block. \
                The token contract was registered on block 10 and we are currently at block 6, \
                you have 0 intervals of rewards. 0 × 100 = 0 tokens";
            assert_eq!(short, expected);
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
                    true,
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
                    true,
                )
                .unwrap();

            // Should still return an explanation even with optimization
            let fixed_step_explanation = fixed_result.explanation_for_step(1).unwrap();
            assert!(fixed_step_explanation.contains("FixedAmount optimization"));
            assert!(fixed_step_explanation.contains("50 tokens"));
        }

        #[test]
        fn test_fixed_amount_block_based_first_claim() {
            let dist = DistributionFunction::FixedAmount { amount: 5000 };
            let start_excluded = RewardDistributionMoment::BlockBasedMoment(1000);
            let end_included = RewardDistributionMoment::BlockBasedMoment(1100);
            let step = RewardDistributionMoment::BlockBasedMoment(10);
            let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = "This token distributes a fixed amount of 5,000 tokens every block. \
                The token contract was registered on block 1000 and we are currently at block 1101, \
                you have 10 intervals of rewards. 10 × 5,000 = 50,000 tokens";
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_singular_forms_one_block() {
            let dist = DistributionFunction::Random {
                min: 500,
                max: 1500,
            };
            let start_excluded = RewardDistributionMoment::BlockBasedMoment(200);
            let end_included = RewardDistributionMoment::BlockBasedMoment(201);
            let step = RewardDistributionMoment::BlockBasedMoment(1);
            let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token distributes a random amount between 500 tokens and 1,500 tokens per block. \
                The token contract was registered on block 200 and we are currently at block 202, \
                you have 1 interval of rewards. The exact amount is randomly determined \
                for each interval, Total rewards: {} tokens",
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_block_based_step_decreasing() {
            let dist = DistributionFunction::StepDecreasingAmount {
                step_count: 100,
                decrease_per_interval_numerator: 10,
                decrease_per_interval_denominator: 100,
                start_decreasing_offset: Some(0),
                max_interval_count: Some(32),
                distribution_start_amount: 10000,
                trailing_distribution_interval_amount: 100,
                min_value: None,
            };

            let start_excluded = RewardDistributionMoment::BlockBasedMoment(500);
            let end_included = RewardDistributionMoment::BlockBasedMoment(600);
            let step = RewardDistributionMoment::BlockBasedMoment(10);
            let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    false,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token starts distributing 10,000 tokens and decreases by 10.0% every 100 blocks. \
                The last claim was for block 500 and we are currently at block 601, \
                you have 10 intervals of rewards. After all decreasing steps, it distributes 100 tokens \
                per interval. Total rewards: {} tokens",
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_block_based_polynomial_distribution() {
            let dist = DistributionFunction::Polynomial {
                a: 5,
                d: 10,
                m: -2,
                n: 1,
                o: 1,
                start_moment: Some(0),
                b: 5000,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::BlockBasedMoment(10000);
            let end_included = RewardDistributionMoment::BlockBasedMoment(11000);
            let step = RewardDistributionMoment::BlockBasedMoment(100);
            let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    false,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token follows a polynomial distribution that starts high and gradually declines with a base amount of 5,000 tokens. \
                The last claim was for block 10000 and we are currently at block 11001, \
                you have 10 intervals of rewards. Total rewards: {} tokens",
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_fixed_amount_explanation_not_first_claim() {
            let dist = DistributionFunction::FixedAmount { amount: 10000 };
            let start_excluded = RewardDistributionMoment::BlockBasedMoment(7554);
            let end_included = RewardDistributionMoment::BlockBasedMoment(7740);
            let step = RewardDistributionMoment::BlockBasedMoment(1);
            let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    false,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = "This token distributes a fixed amount of 10,000 tokens every block. \
                The last claim was for block 7554 and we are currently at block 7741, \
                you have 186 intervals of rewards. 186 × 10,000 = 1,860,000 tokens";
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_random_explanation_first_claim() {
            let dist = DistributionFunction::Random { min: 100, max: 500 };
            let start_excluded = RewardDistributionMoment::BlockBasedMoment(50);
            let end_included = RewardDistributionMoment::BlockBasedMoment(60);
            let step = RewardDistributionMoment::BlockBasedMoment(1);
            let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token distributes a random amount between 100 tokens and 500 tokens per block. \
                The token contract was registered on block 50 and we are currently at block 61, \
                you have 10 intervals of rewards. The exact amount is randomly determined \
                for each interval, Total rewards: {} tokens",
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_random_explanation_not_first_claim() {
            let dist = DistributionFunction::Random { min: 100, max: 500 };
            let start_excluded = RewardDistributionMoment::BlockBasedMoment(50);
            let end_included = RewardDistributionMoment::BlockBasedMoment(60);
            let step = RewardDistributionMoment::BlockBasedMoment(1);
            let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    false,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token distributes a random amount between 100 tokens and 500 tokens per block. \
                The last claim was for block 50 and we are currently at block 61, \
                you have 10 intervals of rewards. The exact amount is randomly determined \
                for each interval, Total rewards: {} tokens",
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_stepwise_explanation_not_first_claim() {
            use std::collections::BTreeMap;

            let mut steps = BTreeMap::new();
            steps.insert(0, 1000);
            steps.insert(10, 500);
            steps.insert(20, 250);

            let dist = DistributionFunction::Stepwise(steps);
            let start_excluded = RewardDistributionMoment::BlockBasedMoment(5);
            let end_included = RewardDistributionMoment::BlockBasedMoment(25);
            let step = RewardDistributionMoment::BlockBasedMoment(1);
            let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    false,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token distributes tokens in predefined steps: 1,000 tokens from interval 0, 500 tokens from interval 10, 250 tokens from interval 20. \
                The last claim was for block 5 and we are currently at block 26, \
                you have 20 intervals of rewards. Total rewards: {} tokens",
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_linear_explanation_not_first_claim_decreasing() {
            let dist = DistributionFunction::Linear {
                a: -50,
                d: 1,
                start_step: Some(0),
                starting_amount: 5000,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::BlockBasedMoment(10);
            let end_included = RewardDistributionMoment::BlockBasedMoment(20);
            let step = RewardDistributionMoment::BlockBasedMoment(1);
            let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    false,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token starts at 5,000 tokens and decreases by 50/1 tokens per block. \
                The last claim was for block 10 and we are currently at block 21, \
                you have 10 intervals of rewards. Total rewards: {} tokens",
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_exponential_explanation_first_claim() {
            let dist = DistributionFunction::Exponential {
                a: 100,
                d: 10,
                m: 2,
                n: 50,
                o: 0,
                start_moment: Some(0),
                b: 500,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::BlockBasedMoment(20);
            let end_included = RewardDistributionMoment::BlockBasedMoment(30);
            let step = RewardDistributionMoment::BlockBasedMoment(1);
            let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token grows exponentially starting from a base of 500 tokens with scaling factor 100. \
                The token contract was registered on block 20 and we are currently at block 31, \
                you have 10 intervals of rewards. Total rewards: {} tokens",
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_exponential_explanation_not_first_claim() {
            let dist = DistributionFunction::Exponential {
                a: 100,
                d: 10,
                m: -2,
                n: 50,
                o: 0,
                start_moment: Some(0),
                b: 500,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::BlockBasedMoment(20);
            let end_included = RewardDistributionMoment::BlockBasedMoment(30);
            let step = RewardDistributionMoment::BlockBasedMoment(1);
            let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    false,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token decays exponentially starting from a base of 500 tokens with scaling factor 100. \
                The last claim was for block 20 and we are currently at block 31, \
                you have 10 intervals of rewards. Total rewards: {} tokens",
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_logarithmic_explanation_first_claim() {
            let dist = DistributionFunction::Logarithmic {
                a: 1000,
                d: 10,
                m: 2,
                n: 1,
                o: 1,
                start_moment: Some(0),
                b: 2000,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::BlockBasedMoment(100);
            let end_included = RewardDistributionMoment::BlockBasedMoment(110);
            let step = RewardDistributionMoment::BlockBasedMoment(1);
            let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token follows a logarithmic distribution that increases at a slowing rate with a base amount of 2,000 tokens. \
                The token contract was registered on block 100 and we are currently at block 111, \
                you have 10 intervals of rewards. Total rewards: {} tokens",
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_logarithmic_explanation_not_first_claim() {
            let dist = DistributionFunction::Logarithmic {
                a: -1000,
                d: 10,
                m: 2,
                n: 1,
                o: 1,
                start_moment: Some(0),
                b: 2000,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::BlockBasedMoment(100);
            let end_included = RewardDistributionMoment::BlockBasedMoment(110);
            let step = RewardDistributionMoment::BlockBasedMoment(1);
            let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    false,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token follows a logarithmic distribution that decreases at a slowing rate with a base amount of 2,000 tokens. \
                The last claim was for block 100 and we are currently at block 111, \
                you have 10 intervals of rewards. Total rewards: {} tokens",
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_inverted_logarithmic_explanation_first_claim() {
            let dist = DistributionFunction::InvertedLogarithmic {
                a: 10000,
                d: 1,
                m: 1,
                n: 5000,
                o: 0,
                start_moment: Some(0),
                b: 0,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::BlockBasedMoment(50);
            let end_included = RewardDistributionMoment::BlockBasedMoment(60);
            let step = RewardDistributionMoment::BlockBasedMoment(1);
            let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token starts with high rewards that gradually decrease following an inverted logarithmic curve. \
                The token contract was registered on block 50 and we are currently at block 61, \
                you have 10 intervals of rewards. Total rewards: {} tokens",
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_inverted_logarithmic_explanation_not_first_claim() {
            let dist = DistributionFunction::InvertedLogarithmic {
                a: 10000,
                d: 1,
                m: 1,
                n: 5000,
                o: 0,
                start_moment: Some(0),
                b: 0,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::BlockBasedMoment(50);
            let end_included = RewardDistributionMoment::BlockBasedMoment(60);
            let step = RewardDistributionMoment::BlockBasedMoment(1);
            let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    false,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token starts with high rewards that gradually decrease following an inverted logarithmic curve. \
                The last claim was for block 50 and we are currently at block 61, \
                you have 10 intervals of rewards. Total rewards: {} tokens",
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_explanation_methods_output() {
            let distribution_function = DistributionFunction::FixedAmount { amount: 50 };
            let start_excluded = RewardDistributionMoment::BlockBasedMoment(1);
            let end_included = RewardDistributionMoment::BlockBasedMoment(3);
            let step = RewardDistributionMoment::BlockBasedMoment(1);
            let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

            let result = distribution_function
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let short = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let long = result.detailed_explanation();

            // Short should be a descriptive explanation
            let expected_short = "This token distributes a fixed amount of 50 tokens every block. \
                The token contract was registered on block 1 and we are currently at block 4, \
                you have 2 intervals of rewards. 2 × 50 = 100 tokens";
            assert_eq!(short, expected_short);

            // Long should be the most detailed
            assert!(long.contains("==="));
        }

        #[test]
        fn test_explanation_for_step_with_reward_ratio() {
            let distribution_function = DistributionFunction::FixedAmount { amount: 100 };
            let start_excluded = RewardDistributionMoment::BlockBasedMoment(0);
            let end_included = RewardDistributionMoment::BlockBasedMoment(2);
            let step = RewardDistributionMoment::BlockBasedMoment(1);
            let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

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
                    true,
                )
                .unwrap();

            // For FixedAmount with ratio, it still uses optimization
            let explanation = result.explanation_for_step(1).unwrap();
            assert!(explanation.contains("FixedAmount optimization"));
        }

        #[test]
        fn test_decimal_offset_formatting() {
            let dist = DistributionFunction::FixedAmount { amount: 520000 };
            let start_excluded = RewardDistributionMoment::BlockBasedMoment(0);
            let end_included = RewardDistributionMoment::BlockBasedMoment(1);
            let step = RewardDistributionMoment::BlockBasedMoment(1);
            let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            // Test with decimal offset of 5
            let actual = result.short_explanation(5, PlatformVersion::latest(), "UTC");
            assert!(
                actual.contains("5.2 tokens"),
                "Expected '5.2 tokens' with decimal offset 5, got: {}",
                actual
            );

            // Test total amount formatting
            assert!(
                actual.contains("1 × 5.2 = 5.2 tokens"),
                "Expected formatted calculation, got: {}",
                actual
            );
        }

        #[test]
        fn test_edge_case_large_numbers_with_decimals() {
            let dist = DistributionFunction::FixedAmount { amount: 123456789 };
            let start_excluded = RewardDistributionMoment::BlockBasedMoment(0);
            let end_included = RewardDistributionMoment::BlockBasedMoment(2);
            let step = RewardDistributionMoment::BlockBasedMoment(1);
            let distribution_start = RewardDistributionMoment::BlockBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            // Test with decimal offset of 8 (common for crypto tokens)
            let actual = result.short_explanation(8, PlatformVersion::latest(), "UTC");
            assert!(
                actual.contains("1.23456789 tokens"),
                "Expected '1.23456789 tokens' with decimal offset 8, got: {}",
                actual
            );
            assert!(
                actual.contains("2 × 1.23456789 = 2.46913578 tokens"),
                "Expected formatted calculation, got: {}",
                actual
            );
        }
    }

    mod time_interval_tests {
        use super::*;

        #[test]
        fn test_singular_token_amount_time() {
            // Test with exactly 1 token (assuming decimal_offset=0)
            let dist = DistributionFunction::FixedAmount { amount: 1 };
            let start_excluded = RewardDistributionMoment::TimeBasedMoment(0);
            let end_included = RewardDistributionMoment::TimeBasedMoment(1);
            let step = RewardDistributionMoment::TimeBasedMoment(1);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            // Should say "1 token" not "1 token`s`"
            // For TimeBasedMoment, the format is different from EpochBasedMoment and BlockBasedMoment
            let expected = format!(
                "This token distributes a fixed amount of 1 token at each interval. \
                The token contract was registered before {} and we are \
                currently at {}, you have 1 interval of rewards. \
                1 × 1 = 1 token",
                format_timestamp_to_date(1, "UTC"),
                format_timestamp_to_date(2, "UTC")
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_fixed_amount_time_based_not_first_claim() {
            let dist = DistributionFunction::FixedAmount { amount: 2500 };
            let start_excluded = RewardDistributionMoment::TimeBasedMoment(3600);
            let end_included = RewardDistributionMoment::TimeBasedMoment(7200);
            let step = RewardDistributionMoment::TimeBasedMoment(300);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    false,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token distributes a fixed amount of 2,500 tokens at each interval. \
                The last claim was on {} and we are currently at {}, \
                you have 12 intervals of rewards. 12 × 2,500 = 30,000 tokens",
                format_timestamp_to_date(3600, "UTC"),
                format_timestamp_to_date(7201, "UTC")
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_decimal_offset_no_trailing_zeros() {
            let dist = DistributionFunction::FixedAmount { amount: 500000 };
            let start_excluded = RewardDistributionMoment::TimeBasedMoment(0);
            let end_included = RewardDistributionMoment::TimeBasedMoment(1800);
            let step = RewardDistributionMoment::TimeBasedMoment(1800);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            // Test with decimal offset of 5 - should show "5" not "5.00000"
            let actual = result.short_explanation(5, PlatformVersion::latest(), "UTC");
            assert!(
                actual.contains("5 tokens"),
                "Expected '5 tokens' without trailing zeros, got: {}",
                actual
            );
            assert!(
                !actual.contains("5.0"),
                "Should not contain trailing zeros, got: {}",
                actual
            );
        }

        #[test]
        fn test_time_based_linear_distribution() {
            let dist = DistributionFunction::Linear {
                a: 25,
                d: 1,
                start_step: Some(0),
                starting_amount: 1000,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::TimeBasedMoment(3600);
            let end_included = RewardDistributionMoment::TimeBasedMoment(7200);
            let step = RewardDistributionMoment::TimeBasedMoment(300);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token starts at 1,000 tokens and increases by 25/1 tokens at each interval. \
                The token contract was registered before {} and we are currently at {}, \
                you have 12 intervals of rewards. Total rewards: {} tokens",
                format_timestamp_to_date(3601, "UTC"),
                format_timestamp_to_date(7201, "UTC"),
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_time_based_stepwise_distribution() {
            use std::collections::BTreeMap;

            let mut steps = BTreeMap::new();
            steps.insert(0, 2000);
            steps.insert(5, 1500);
            steps.insert(10, 1000);
            steps.insert(15, 500);

            let dist = DistributionFunction::Stepwise(steps);
            let start_excluded = RewardDistributionMoment::TimeBasedMoment(1800);
            let end_included = RewardDistributionMoment::TimeBasedMoment(3600);
            let step = RewardDistributionMoment::TimeBasedMoment(600);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token distributes tokens in predefined steps: 2,000 tokens from interval 0, 1,500 tokens from interval 5, 1,000 tokens from interval 10, and 1 more step. \
                The token contract was registered before {} and we are currently at {}, \
                you have 3 intervals of rewards. Total rewards: {} tokens",
                format_timestamp_to_date(1801, "UTC"),
                format_timestamp_to_date(3601, "UTC"),
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_fixed_amount_time_based_first_claim() {
            let dist = DistributionFunction::FixedAmount { amount: 10000 };
            let start_excluded = RewardDistributionMoment::TimeBasedMoment(7555);
            let end_included = RewardDistributionMoment::TimeBasedMoment(7740);
            let step = RewardDistributionMoment::TimeBasedMoment(1);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token distributes a fixed amount of 10,000 tokens at each interval. \
                The token contract was registered before {} and we are \
                currently at {}, you have 185 intervals of rewards. \
                185 × 10,000 = 1,850,000 tokens",
                format_timestamp_to_date(7556, "UTC"),
                format_timestamp_to_date(7741, "UTC")
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_random_explanation_first_claim() {
            let dist = DistributionFunction::Random { min: 100, max: 500 };
            let start_excluded = RewardDistributionMoment::TimeBasedMoment(50);
            let end_included = RewardDistributionMoment::TimeBasedMoment(60);
            let step = RewardDistributionMoment::TimeBasedMoment(1);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token distributes a random amount between 100 tokens and 500 tokens at each interval. \
                The token contract was registered before {} and we are currently at {}, \
                you have 10 intervals of rewards. The exact amount is randomly determined \
                for each interval, Total rewards: {} tokens",
                format_timestamp_to_date(51, "UTC"),
                format_timestamp_to_date(61, "UTC"),
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_random_explanation_not_first_claim() {
            let dist = DistributionFunction::Random { min: 100, max: 500 };
            let start_excluded = RewardDistributionMoment::TimeBasedMoment(50);
            let end_included = RewardDistributionMoment::TimeBasedMoment(60);
            let step = RewardDistributionMoment::TimeBasedMoment(1);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    false,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token distributes a random amount between 100 tokens and 500 tokens at each interval. \
                The last claim was on {} and we are currently at {}, \
                you have 10 intervals of rewards. The exact amount is randomly determined \
                for each interval, Total rewards: {} tokens",
                format_timestamp_to_date(50, "UTC"),
                format_timestamp_to_date(61, "UTC"),
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_step_decreasing_amount_explanation_first_claim() {
            let dist = DistributionFunction::StepDecreasingAmount {
                step_count: 210000,
                decrease_per_interval_numerator: 7,
                decrease_per_interval_denominator: 100,
                start_decreasing_offset: Some(0),
                max_interval_count: Some(64),
                distribution_start_amount: 50000,
                trailing_distribution_interval_amount: 1000,
                min_value: None,
            };

            let start_excluded = RewardDistributionMoment::TimeBasedMoment(0);
            let end_included = RewardDistributionMoment::TimeBasedMoment(10);
            let step = RewardDistributionMoment::TimeBasedMoment(1);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token starts distributing 50,000 tokens and decreases by 7.0% every 210,000 intervals. \
                The token contract was registered on {} and we are currently at {}, \
                you have 10 intervals of rewards. After all decreasing steps, it distributes 1,000 tokens \
                per interval. Total rewards: {} tokens",
                format_timestamp_to_date(0, "UTC"),
                format_timestamp_to_date(10, "UTC"),
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_step_decreasing_amount_explanation_not_first_claim() {
            let dist = DistributionFunction::StepDecreasingAmount {
                step_count: 100,
                decrease_per_interval_numerator: 10,
                decrease_per_interval_denominator: 100,
                start_decreasing_offset: Some(0),
                max_interval_count: Some(32),
                distribution_start_amount: 10000,
                trailing_distribution_interval_amount: 100,
                min_value: None,
            };

            let start_excluded = RewardDistributionMoment::TimeBasedMoment(500);
            let end_included = RewardDistributionMoment::TimeBasedMoment(600);
            let step = RewardDistributionMoment::TimeBasedMoment(10);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    false,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token starts distributing 10,000 tokens and decreases by 10.0% every 100 intervals. \
                The last claim was on {} and we are currently at {}, \
                you have 10 intervals of rewards. After all decreasing steps, it distributes 100 tokens \
                per interval. Total rewards: {} tokens",
                format_timestamp_to_date(500, "UTC"),
                format_timestamp_to_date(600, "UTC"),
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_stepwise_explanation_not_first_claim() {
            use std::collections::BTreeMap;

            let mut steps = BTreeMap::new();
            steps.insert(0, 1000);
            steps.insert(10, 500);
            steps.insert(20, 250);

            let dist = DistributionFunction::Stepwise(steps);
            let start_excluded = RewardDistributionMoment::TimeBasedMoment(5);
            let end_included = RewardDistributionMoment::TimeBasedMoment(25);
            let step = RewardDistributionMoment::TimeBasedMoment(1);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    false,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token distributes tokens in predefined steps: 1,000 tokens from interval 0, 500 tokens from interval 10, 250 tokens from interval 20. \
                The last claim was on {} and we are currently at {}, \
                you have 20 intervals of rewards. Total rewards: {} tokens",
                format_timestamp_to_date(5, "UTC"),
                format_timestamp_to_date(26, "UTC"),
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_linear_explanation_not_first_claim_decreasing() {
            let dist = DistributionFunction::Linear {
                a: -50,
                d: 1,
                start_step: Some(0),
                starting_amount: 5000,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::TimeBasedMoment(10);
            let end_included = RewardDistributionMoment::TimeBasedMoment(20);
            let step = RewardDistributionMoment::TimeBasedMoment(1);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    false,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let expected = format!(
                "This token starts at 5,000 tokens and decreases by 50/1 tokens at each interval. \
                The last claim was on {} and we are currently at {}, \
                you have 10 intervals of rewards. Total rewards: {} tokens",
                format_timestamp_to_date(10, "UTC"),
                format_timestamp_to_date(21, "UTC"),
                format_number_with_separators(result.total_amount)
            );
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_polynomial_explanation_first_claim() {
            let dist = DistributionFunction::Polynomial {
                a: 10,
                d: 1,
                m: 2,
                n: 1,
                o: 0,
                start_moment: Some(0),
                b: 1000,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::TimeBasedMoment(5);
            let end_included = RewardDistributionMoment::TimeBasedMoment(15);
            let step = RewardDistributionMoment::TimeBasedMoment(1);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let expected_contains = vec![
                "This token follows a polynomial distribution that increases at an accelerating rate with a base amount of 1,000 tokens",
                "The token contract was registered before time period 6",
                "we are currently at time period 16",
                "you have 10 intervals of rewards"
            ];

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            for expected_part in expected_contains {
                assert!(
                    actual.contains(expected_part),
                    "Expected to find '{}' in '{}'",
                    expected_part,
                    actual
                );
            }
        }

        #[test]
        fn test_polynomial_explanation_not_first_claim() {
            let dist = DistributionFunction::Polynomial {
                a: 5,
                d: 10,
                m: -2,
                n: 1,
                o: 1,
                start_moment: Some(0),
                b: 5000,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::TimeBasedMoment(10000);
            let end_included = RewardDistributionMoment::TimeBasedMoment(11000);
            let step = RewardDistributionMoment::TimeBasedMoment(100);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    false,
                )
                .unwrap();

            let expected_contains = vec![
                "This token follows a polynomial distribution that starts high and gradually declines with a base amount of 5,000 tokens",
                "The last claim was for time period 10000",
                "we are currently at time period 11001",
                "you have 10 intervals of rewards"
            ];

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            for expected_part in expected_contains {
                assert!(
                    actual.contains(expected_part),
                    "Expected to find '{}' in '{}'",
                    expected_part,
                    actual
                );
            }
        }

        #[test]
        fn test_exponential_explanation_first_claim() {
            let dist = DistributionFunction::Exponential {
                a: 100,
                d: 10,
                m: 2,
                n: 50,
                o: 0,
                start_moment: Some(0),
                b: 500,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::TimeBasedMoment(20);
            let end_included = RewardDistributionMoment::TimeBasedMoment(30);
            let step = RewardDistributionMoment::TimeBasedMoment(1);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let expected_contains = vec![
                "This token grows exponentially starting from a base of 500 tokens with scaling factor 100",
                "The token contract was registered before time period 21",
                "we are currently at time period 31",
                "you have 10 intervals of rewards"
            ];

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            for expected_part in expected_contains {
                assert!(
                    actual.contains(expected_part),
                    "Expected to find '{}' in '{}'",
                    expected_part,
                    actual
                );
            }
        }

        #[test]
        fn test_exponential_explanation_not_first_claim() {
            let dist = DistributionFunction::Exponential {
                a: 100,
                d: 10,
                m: -2,
                n: 50,
                o: 0,
                start_moment: Some(0),
                b: 500,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::TimeBasedMoment(20);
            let end_included = RewardDistributionMoment::TimeBasedMoment(30);
            let step = RewardDistributionMoment::TimeBasedMoment(1);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    false,
                )
                .unwrap();

            let expected_contains = vec![
                "This token decays exponentially starting from a base of 500 tokens with scaling factor 100",
                "The last claim was for time period 20",
                "we are currently at time period 31",
                "you have 10 intervals of rewards"
            ];

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            for expected_part in expected_contains {
                assert!(
                    actual.contains(expected_part),
                    "Expected to find '{}' in '{}'",
                    expected_part,
                    actual
                );
            }
        }

        #[test]
        fn test_logarithmic_explanation_first_claim() {
            let dist = DistributionFunction::Logarithmic {
                a: 1000,
                d: 10,
                m: 2,
                n: 1,
                o: 1,
                start_moment: Some(0),
                b: 2000,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::TimeBasedMoment(100);
            let end_included = RewardDistributionMoment::TimeBasedMoment(110);
            let step = RewardDistributionMoment::TimeBasedMoment(1);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let expected_contains = vec![
                "This token follows a logarithmic distribution that increases at a slowing rate with a base amount of 2,000 tokens",
                "The token contract was registered before time period 101",
                "we are currently at time period 111",
                "you have 10 intervals of rewards"
            ];

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            for expected_part in expected_contains {
                assert!(
                    actual.contains(expected_part),
                    "Expected to find '{}' in '{}'",
                    expected_part,
                    actual
                );
            }
        }

        #[test]
        fn test_logarithmic_explanation_not_first_claim() {
            let dist = DistributionFunction::Logarithmic {
                a: -1000,
                d: 10,
                m: 2,
                n: 1,
                o: 1,
                start_moment: Some(0),
                b: 2000,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::TimeBasedMoment(100);
            let end_included = RewardDistributionMoment::TimeBasedMoment(110);
            let step = RewardDistributionMoment::TimeBasedMoment(1);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    false,
                )
                .unwrap();

            let expected_contains = vec![
                "This token follows a logarithmic distribution that decreases at a slowing rate with a base amount of 2,000 tokens",
                "The last claim was for time period 100",
                "we are currently at time period 111",
                "you have 10 intervals of rewards"
            ];

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            for expected_part in expected_contains {
                assert!(
                    actual.contains(expected_part),
                    "Expected to find '{}' in '{}'",
                    expected_part,
                    actual
                );
            }
        }

        #[test]
        fn test_inverted_logarithmic_explanation_first_claim() {
            let dist = DistributionFunction::InvertedLogarithmic {
                a: 10000,
                d: 1,
                m: 1,
                n: 5000,
                o: 0,
                start_moment: Some(0),
                b: 0,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::TimeBasedMoment(50);
            let end_included = RewardDistributionMoment::TimeBasedMoment(60);
            let step = RewardDistributionMoment::TimeBasedMoment(1);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let expected_contains = vec![
                "This token starts with high rewards that gradually decrease following an inverted logarithmic curve",
                "The token contract was registered before Thursday January 1 1970",
                "we are currently at Thursday January 1 1970",
                "you have 10 intervals of rewards"
            ];

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            for expected_part in expected_contains {
                assert!(
                    actual.contains(expected_part),
                    "Expected to find '{}' in '{}'",
                    expected_part,
                    actual
                );
            }
        }

        #[test]
        fn test_inverted_logarithmic_explanation_not_first_claim() {
            let dist = DistributionFunction::InvertedLogarithmic {
                a: 10000,
                d: 1,
                m: 1,
                n: 5000,
                o: 0,
                start_moment: Some(0),
                b: 0,
                min_value: None,
                max_value: None,
            };

            let start_excluded = RewardDistributionMoment::TimeBasedMoment(50);
            let end_included = RewardDistributionMoment::TimeBasedMoment(60);
            let step = RewardDistributionMoment::TimeBasedMoment(1);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    false,
                )
                .unwrap();

            let expected_contains = vec![
                "This token starts with high rewards that gradually decrease following an inverted logarithmic curve",
                "The last claim was on Thursday January 1 1970",
                "we are currently at Thursday January 1 1970",
                "you have 10 intervals of rewards"
            ];

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            for expected_part in expected_contains {
                assert!(
                    actual.contains(expected_part),
                    "Expected to find '{}' in '{}'",
                    expected_part,
                    actual
                );
            }
        }

        #[test]
        fn test_explanation_methods_output() {
            let distribution_function = DistributionFunction::FixedAmount { amount: 50 };
            let start_excluded = RewardDistributionMoment::TimeBasedMoment(1);
            let end_included = RewardDistributionMoment::TimeBasedMoment(3);
            let step = RewardDistributionMoment::TimeBasedMoment(1);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = distribution_function
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let short = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            let long = result.detailed_explanation();

            // Short should be a descriptive explanation
            assert!(short.contains("This token"));

            // Long should be the most detailed
            assert!(long.contains("==="));
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
            let start_excluded = RewardDistributionMoment::TimeBasedMoment(0);
            let end_included = RewardDistributionMoment::TimeBasedMoment(30);
            let step = RewardDistributionMoment::TimeBasedMoment(10);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = distribution_function
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
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
                    true,
                )
                .unwrap();

            // Should still return an explanation even with optimization
            let fixed_step_explanation = fixed_result.explanation_for_step(1).unwrap();
            assert!(fixed_step_explanation.contains("FixedAmount optimization"));
            assert!(fixed_step_explanation.contains("50 tokens"));
        }

        #[test]
        fn test_explanation_with_zero_range() {
            let distribution_function = DistributionFunction::FixedAmount { amount: 100 };
            let start_excluded = RewardDistributionMoment::TimeBasedMoment(10);
            let end_included = RewardDistributionMoment::TimeBasedMoment(5); // End before start
            let step = RewardDistributionMoment::TimeBasedMoment(1);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = distribution_function
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            assert_eq!(result.total_amount, 0);
            assert_eq!(result.steps_count, 0);
            assert!(result
                .optimization_notes
                .iter()
                .any(|note| note.contains("Start >= End")));

            // Test that empty range still generates explanation
            let short = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            assert!(short.contains("0 tokens"));
        }

        #[test]
        fn test_explanation_for_step_with_reward_ratio() {
            let distribution_function = DistributionFunction::FixedAmount { amount: 100 };
            let start_excluded = RewardDistributionMoment::TimeBasedMoment(0);
            let end_included = RewardDistributionMoment::TimeBasedMoment(2);
            let step = RewardDistributionMoment::TimeBasedMoment(1);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

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
                    true,
                )
                .unwrap();

            // For FixedAmount with ratio, it still uses optimization
            let explanation = result.explanation_for_step(1).unwrap();
            assert!(explanation.contains("FixedAmount optimization"));
        }

        #[test]
        fn test_singular_forms_one_time_period() {
            let dist = DistributionFunction::FixedAmount { amount: 1000 };
            let start_excluded = RewardDistributionMoment::TimeBasedMoment(100);
            let end_included = RewardDistributionMoment::TimeBasedMoment(101);
            let step = RewardDistributionMoment::TimeBasedMoment(1);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            let actual = result.short_explanation(0, PlatformVersion::latest(), "UTC");
            assert!(
                actual.contains("you have 1 interval of rewards"),
                "Expected singular 'interval', got: {}",
                actual
            );
            assert!(
                !actual.contains("1 intervals"),
                "Should not contain '1 intervals', got: {}",
                actual
            );
        }

        #[test]
        fn test_edge_case_large_numbers_with_decimals() {
            let dist = DistributionFunction::FixedAmount { amount: 123456789 };
            let start_excluded = RewardDistributionMoment::TimeBasedMoment(0);
            let end_included = RewardDistributionMoment::TimeBasedMoment(2);
            let step = RewardDistributionMoment::TimeBasedMoment(1);
            let distribution_start = RewardDistributionMoment::TimeBasedMoment(0);

            let result = dist
                .evaluate_interval_with_explanation(
                    distribution_start,
                    start_excluded,
                    end_included,
                    step,
                    None::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>,
                    true,
                )
                .unwrap();

            // Test with decimal offset of 8 (common for crypto tokens)
            let actual = result.short_explanation(8, PlatformVersion::latest(), "UTC");
            assert!(
                actual.contains("1.23456789 tokens"),
                "Expected '1.23456789 tokens' with decimal offset 8, got: {}",
                actual
            );
            assert!(
                actual.contains("2 × 1.23456789 = 2.46913578 tokens"),
                "Expected formatted calculation, got: {}",
                actual
            );
        }
    }
}
