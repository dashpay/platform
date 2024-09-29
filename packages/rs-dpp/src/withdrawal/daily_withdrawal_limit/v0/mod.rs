use crate::fee::Credits;

/// Calculates the daily withdrawal limit based on the total credits available in the platform.
///
/// The function enforces the following rules:
///
/// 1. If the total credits are 1000 Dash in Credits or more:
///     - The withdrawal limit is set to 10% of the total credits.
/// 2. If the total credits are between 100 and 999 Dash in Credits:
///     - The withdrawal limit is capped at 100 credits.
/// 3. If the total credits are less than 100 Dash in Credits:
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
    if total_credits_in_platform >= 100_000_000_000_000 {
        // 1000 Dash
        total_credits_in_platform / 10
    } else if total_credits_in_platform >= 10_000_000_000_000 {
        // 100 Dash
        10_000_000_000_000
    } else {
        total_credits_in_platform
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dash_to_credits;

    #[test]
    fn test_daily_withdrawal_limit() {
        assert_eq!(
            daily_withdrawal_limit_v0(dash_to_credits!(2000)),
            dash_to_credits!(200)
        );
        assert_eq!(
            daily_withdrawal_limit_v0(dash_to_credits!(500)),
            dash_to_credits!(100)
        );
        assert_eq!(
            daily_withdrawal_limit_v0(dash_to_credits!(50)),
            dash_to_credits!(50)
        );
    }
}
