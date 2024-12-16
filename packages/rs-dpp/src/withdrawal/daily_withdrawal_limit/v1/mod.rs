use crate::fee::Credits;

/// Set constant withdrawal daily limit to 2000 Dash
///
/// # Returns
///
/// * `Credits`: The calculated daily withdrawal limit based on the available credits.
///
pub fn daily_withdrawal_limit_v1() -> Credits {
    // 2000 Dash
    200_000_000_000_000
}
