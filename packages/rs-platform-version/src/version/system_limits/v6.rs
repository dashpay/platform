use crate::version::system_limits::v5::SYSTEM_LIMITS_V5;
use crate::version::system_limits::SystemLimits;

/// System limits for protocol version 16. Relative to `SYSTEM_LIMITS_V5`, which protocol
/// version 15 runs on:
///
/// * Withdrawals: one withdrawal carries at most 4000 Dash (`max_withdrawal_amount`, 500 Dash
///   before), on the identity, the address and the shielded path alike.
/// * Daily limit: its smallest value stays 500 Dash (`daily_withdrawal_limit_floor`, `None`
///   before, when `daily_withdrawal_limit` 2 took `max_withdrawal_amount` for it), read by
///   `daily_withdrawal_limit` 3, so the daily limit is the same number at every total as in
///   protocol version 15 and can now be below one maximal withdrawal.
/// * Pooling: the Core-anchored limit admits a withdrawal larger than
///   `core_credit_pool_unlock_limit_percent` (15) of Core's credit pool, and than
///   `core_credit_pool_unlock_limit_floor` (1500 Dash, which may not rise above Core's 2000),
///   only once the pool has grown: a 4000 Dash withdrawal needs a pool of about 26,667 Dash.
///   `pool_withdrawals_into_transactions_queue` 3 passes over a withdrawal that does not fit
///   and pools the ones behind it, among the 16 oldest queued withdrawals it considers per
///   block (`withdrawal_pooling_candidates_per_block_limit`, `None` before): the 4 it may pool
///   (`withdrawal_transactions_per_block_limit`) and the 12 that can be unpoolable at once, 6
///   larger than 15% of Core's credit pool and 6 larger than 15% of Platform's credits.
pub const SYSTEM_LIMITS_V6: SystemLimits = SystemLimits {
    max_withdrawal_amount: 400_000_000_000_000, // changed: 4000 Dash (500 Dash before)
    daily_withdrawal_limit_floor: Some(50_000_000_000_000), // changed: 500 Dash, the daily limit's floor of its own now that the largest withdrawal is above it
    withdrawal_pooling_candidates_per_block_limit: Some(16), // changed: pooling considers the 16 oldest queued withdrawals and passes over one that does not fit
    ..SYSTEM_LIMITS_V5
};
