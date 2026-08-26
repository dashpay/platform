/// Functions related to withdrawal documents
pub mod document;

mod calculate_current_withdrawal_limit;

/// How long an entry counts against (a withdrawal reservation) or toward (a credit inflow) the
/// daily withdrawal limit: 25 hours, a day plus an hour of slack. The two must expire on the
/// same schedule so a deposit and the withdrawal it funds cancel exactly over the whole window.
// Best to use a constant here and not a versioned item as this most likely will not change
pub const DAY_AND_A_HOUR_IN_MS: dpp::prelude::TimestampMillis = 90_000_000; //25 hours
/// Functions related to the per-block record of total credits the daily withdrawal limit reads
pub mod fetch_total_credits_in_platform_a_day_ago;
/// Functions and constants related to GroveDB paths
pub mod paths;
/// Functions related to the per-block record of credit inflows the daily withdrawal limit adds
pub mod record_credit_inflow;
/// Functions related to the per-block record of total credits the daily withdrawal limit reads
pub mod record_total_credits_history;
/// Functions related to withdrawal transactions
pub mod transaction;
