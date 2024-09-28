use crate::fee::Credits;
use crate::ProtocolError;

/// Calculates the daily withdrawal limit based on the total credits available in the platform.
///
/// The function enforces the following rules:
///
/// 1. If the total credits are 1000 or more:
///     - The withdrawal limit is set to 10% of the total credits.
/// 2. If the total credits are between 100 and 999:
///     - The withdrawal limit is capped at 100 credits.
/// 3. If the total credits are less than 100:
///     - The withdrawal limit is the total available credits, as no more than the available amount can be withdrawn.
///
/// # Parameters
///
/// * `total_credits_in_platform`: The total amount of credits available in the platform.
///
/// # Returns
///
/// * `Credits`: The calculated daily withdrawal limit based on the available credits.
///
pub fn daily_withdrawal_limit_v0(total_credits_in_platform: Credits) -> Credits {
    if total_credits_in_platform >= 1000 {
        total_credits_in_platform / 10
    } else if total_credits_in_platform >= 100 {
        100
    } else {
        total_credits_in_platform
    }
}
