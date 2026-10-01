use crate::fee::Credits;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

/// The pool may not end below `window_start_balance - allowed_drop`, where
/// `allowed_drop = max(window_start_balance * percent / 100, floor)`; everything above that
/// line is withdrawable, but never more than the pool holds:
///
/// `limit = min(max(0, allowed_drop - (window_start_balance - balance)), balance)`
///
/// Core v24 applies the same shape with 20% and a 2000 Dash floor over its window (576 blocks,
/// 100 on regtest); the system limits of protocol version 14 set a lower percent and floor, and
/// the caller picks the highest balance among the window starts Core may use, so the result
/// never exceeds what Core admits. Integer arithmetic in u128 throughout; truncation only ever makes
/// the limit stricter.
pub(super) fn core_credit_pool_unlock_limit_v0(
    balance: Credits,
    window_start_balance: Credits,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    let percent = platform_version
        .system_limits
        .core_credit_pool_unlock_limit_percent
        .ok_or_else(|| {
            ProtocolError::CorruptedCodeExecution(
                "core_credit_pool_unlock_limit v0 requires system_limits.core_credit_pool_unlock_limit_percent"
                    .to_string(),
            )
        })?;

    let floor = platform_version
        .system_limits
        .core_credit_pool_unlock_limit_floor
        .ok_or_else(|| {
            ProtocolError::CorruptedCodeExecution(
                "core_credit_pool_unlock_limit v0 requires system_limits.core_credit_pool_unlock_limit_floor"
                    .to_string(),
            )
        })?;

    let allowed_drop =
        ((window_start_balance as u128) * (percent as u128) / 100).max(floor as u128);

    // What may leave: the allowed drop plus whatever the pool gained since the window start,
    // or minus whatever it already lost. u128 holds the sum of any two u64 values.
    let withdrawable =
        (allowed_drop + balance as u128).saturating_sub(window_start_balance as u128);

    let limit = withdrawable.min(balance as u128);

    Credits::try_from(limit).map_err(|_| ProtocolError::Overflow("core credit pool unlock limit"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dash_to_credits;

    fn limit(balance: Credits, window_start_balance: Credits) -> Credits {
        core_credit_pool_unlock_limit_v0(balance, window_start_balance, PlatformVersion::latest())
            .expect("expected the limit")
    }

    #[test]
    fn should_allow_the_percent_of_an_unchanged_pool() {
        // 15% of 37,000 Dash, the mainnet pool #7712 measured.
        assert_eq!(
            limit(dash_to_credits!(37000), dash_to_credits!(37000)),
            dash_to_credits!(5550)
        );
    }

    #[test]
    fn should_apply_the_floor_to_a_small_pool() {
        // 15% of 5,000 Dash is 750 Dash, below the 1,500 Dash floor.
        assert_eq!(
            limit(dash_to_credits!(5000), dash_to_credits!(5000)),
            dash_to_credits!(1500)
        );
    }

    #[test]
    fn should_add_what_the_pool_gained_inside_the_window() {
        // A 4,000 Dash deposit inside the window is withdrawable on top of the allowed drop.
        assert_eq!(
            limit(dash_to_credits!(41000), dash_to_credits!(37000)),
            dash_to_credits!(9550)
        );
    }

    #[test]
    fn should_subtract_what_the_pool_already_lost_inside_the_window() {
        // 2,000 Dash already unlocked inside the window leaves 3,550 of the 5,550 allowed.
        assert_eq!(
            limit(dash_to_credits!(35000), dash_to_credits!(37000)),
            dash_to_credits!(3550)
        );
        // Once the drop reaches the allowance nothing is left, and it never goes negative.
        assert_eq!(limit(dash_to_credits!(31450), dash_to_credits!(37000)), 0);
        assert_eq!(limit(dash_to_credits!(20000), dash_to_credits!(37000)), 0);
    }

    #[test]
    fn should_never_exceed_the_pool() {
        // A pool that grew from nothing inside the window: everything in it is withdrawable,
        // but no more than it holds.
        assert_eq!(limit(dash_to_credits!(1000), 0), dash_to_credits!(1000));
        assert_eq!(limit(0, 0), 0);
    }

    #[test]
    fn should_stay_below_cores_own_v24_limit() {
        // Core v24: max(20% of the window start, 2000 Dash) - (window start - balance), at most
        // the balance, over the same window.
        fn core_v24(balance: Credits, window_start_balance: Credits) -> Credits {
            let allowed_drop = (window_start_balance / 5).max(dash_to_credits!(2000));
            (allowed_drop + balance)
                .saturating_sub(window_start_balance)
                .min(balance)
        }

        for (balance, window_start_balance) in [
            (dash_to_credits!(37000), dash_to_credits!(37000)),
            (dash_to_credits!(41000), dash_to_credits!(37000)),
            (dash_to_credits!(35000), dash_to_credits!(37000)),
            (dash_to_credits!(5000), dash_to_credits!(5000)),
            (dash_to_credits!(12000), dash_to_credits!(11000)),
            (dash_to_credits!(1000), 0),
        ] {
            assert!(
                limit(balance, window_start_balance) <= core_v24(balance, window_start_balance),
                "balance {balance}, window start {window_start_balance}"
            );
        }
    }

    #[test]
    fn should_not_overflow_at_the_largest_balances() {
        assert_eq!(
            limit(Credits::MAX, Credits::MAX),
            ((Credits::MAX as u128) * 15 / 100) as Credits
        );
        assert_eq!(limit(Credits::MAX, 0), Credits::MAX);
        assert_eq!(limit(0, Credits::MAX), 0);
    }

    #[test]
    fn should_fail_when_the_limits_are_not_configured() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .system_limits
            .core_credit_pool_unlock_limit_percent = None;
        assert!(matches!(
            core_credit_pool_unlock_limit_v0(1, 1, &platform_version),
            Err(ProtocolError::CorruptedCodeExecution(_))
        ));

        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .system_limits
            .core_credit_pool_unlock_limit_floor = None;
        assert!(matches!(
            core_credit_pool_unlock_limit_v0(1, 1, &platform_version),
            Err(ProtocolError::CorruptedCodeExecution(_))
        ));
    }
}
