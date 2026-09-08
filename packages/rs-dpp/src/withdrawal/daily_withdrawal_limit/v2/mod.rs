use crate::fee::Credits;
use crate::withdrawal::daily_withdrawal_limit::v1::daily_withdrawal_limit_v1;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

/// Relative daily withdrawal limit: `daily_withdrawal_limit_percent` (from the
/// protocol version's system limits) of the total credits Platform held a day ago.
/// Using a day-old base means a sudden jump in the total credits does not raise
/// the limit for a day.
///
/// Three guards keep it usable:
/// * it is never below `max_withdrawal_amount`, so every withdrawal Platform
///   accepts eventually fits the daily maximum and cannot block the pooling
///   queue behind it;
/// * it is never above `max_daily_withdrawal_amount`, Core's credit-pool unlock
///   capacity per day: pooling more than Core will mine only cycles those
///   unlocks through expiry and re-signing;
/// * while the total credits a day ago are not known (`None`: the history is
///   younger than a day, i.e. right after this rule activates), the flat limit
///   of version 1 applies, so the lag cannot be skipped by inflating the total
///   before or at activation.
pub fn daily_withdrawal_limit_v2(
    total_credits_in_platform_a_day_ago: Option<Credits>,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    let Some(total_credits_a_day_ago) = total_credits_in_platform_a_day_ago else {
        return Ok(daily_withdrawal_limit_v1());
    };

    let percent = platform_version
        .system_limits
        .daily_withdrawal_limit_percent
        .ok_or_else(|| {
            ProtocolError::CorruptedCodeExecution(
                "daily_withdrawal_limit v2 requires system_limits.daily_withdrawal_limit_percent"
                    .to_string(),
            )
        })?;

    let max_daily_withdrawal_amount = platform_version
        .system_limits
        .max_daily_withdrawal_amount
        .ok_or_else(|| {
            ProtocolError::CorruptedCodeExecution(
                "daily_withdrawal_limit v2 requires system_limits.max_daily_withdrawal_amount"
                    .to_string(),
            )
        })?;

    let max_withdrawal_amount = platform_version.system_limits.max_withdrawal_amount;
    if max_daily_withdrawal_amount < max_withdrawal_amount {
        // A cap below one maximal withdrawal would let an accepted withdrawal never fit the
        // daily maximum; that is a contradictory configuration, not a limit to apply.
        return Err(ProtocolError::CorruptedCodeExecution(format!(
            "daily_withdrawal_limit v2 requires system_limits.max_daily_withdrawal_amount ({max_daily_withdrawal_amount}) to be at least max_withdrawal_amount ({max_withdrawal_amount})"
        )));
    }

    // u128 keeps `total * percent` from overflowing for any u64 total.
    let relative_limit = (total_credits_a_day_ago as u128) * (percent as u128) / 100;
    let relative_limit = Credits::try_from(relative_limit)
        .map_err(|_| ProtocolError::Overflow("daily withdrawal limit overflow"))?;

    Ok(relative_limit
        .max(max_withdrawal_amount)
        .min(max_daily_withdrawal_amount))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dash_to_credits;

    fn platform_version_with(percent: Option<u8>) -> PlatformVersion {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .system_limits
            .daily_withdrawal_limit_percent = percent;
        platform_version.system_limits.max_withdrawal_amount = dash_to_credits!(500);
        platform_version.system_limits.max_daily_withdrawal_amount = Some(dash_to_credits!(4000));
        platform_version
    }

    #[test]
    fn should_return_the_configured_percent_of_the_lagged_total() {
        let platform_version = platform_version_with(Some(15));

        assert_eq!(
            daily_withdrawal_limit_v2(Some(dash_to_credits!(20000)), &platform_version)
                .expect("expected limit"),
            dash_to_credits!(3000)
        );
        // Rounds down to whole credits: 15% of 4000 Dash + 7 credits is 600 Dash + 1.05 credits.
        assert_eq!(
            daily_withdrawal_limit_v2(Some(dash_to_credits!(4000) + 7), &platform_version)
                .expect("expected limit"),
            dash_to_credits!(600) + 1
        );
    }

    #[test]
    fn should_never_go_below_one_maximal_withdrawal() {
        let platform_version = platform_version_with(Some(15));

        // 15% of 2000 Dash is 300 Dash, below the 500 Dash a single withdrawal may carry.
        assert_eq!(
            daily_withdrawal_limit_v2(Some(dash_to_credits!(2000)), &platform_version)
                .expect("expected limit"),
            dash_to_credits!(500)
        );
        assert_eq!(
            daily_withdrawal_limit_v2(Some(0), &platform_version).expect("expected limit"),
            dash_to_credits!(500)
        );
        // Exactly at the boundary the percent takes over.
        assert_eq!(
            daily_withdrawal_limit_v2(Some(dash_to_credits!(4000)), &platform_version)
                .expect("expected limit"),
            dash_to_credits!(600)
        );
    }

    #[test]
    fn should_never_exceed_cores_unlock_capacity_per_day() {
        let platform_version = platform_version_with(Some(15));

        // 15% of 30000 Dash is 4500 Dash, above what Core mines per day.
        assert_eq!(
            daily_withdrawal_limit_v2(Some(dash_to_credits!(30000)), &platform_version)
                .expect("expected limit"),
            dash_to_credits!(4000)
        );
        assert_eq!(
            daily_withdrawal_limit_v2(Some(Credits::MAX), &platform_version)
                .expect("expected limit"),
            dash_to_credits!(4000)
        );
        // Just under the boundary the percent still applies.
        assert_eq!(
            daily_withdrawal_limit_v2(Some(dash_to_credits!(26666)), &platform_version)
                .expect("expected limit"),
            dash_to_credits!(3999.9)
        );
    }

    #[test]
    fn should_keep_the_flat_limit_until_the_lagged_total_is_known() {
        let platform_version = platform_version_with(Some(15));

        assert_eq!(
            daily_withdrawal_limit_v2(None, &platform_version).expect("expected limit"),
            dash_to_credits!(2000)
        );
    }

    #[test]
    fn should_fail_when_the_percent_or_the_cap_is_not_configured() {
        let platform_version = platform_version_with(None);
        assert!(matches!(
            daily_withdrawal_limit_v2(Some(dash_to_credits!(100)), &platform_version),
            Err(ProtocolError::CorruptedCodeExecution(_))
        ));

        let mut platform_version = platform_version_with(Some(15));
        platform_version.system_limits.max_daily_withdrawal_amount = None;
        assert!(matches!(
            daily_withdrawal_limit_v2(Some(dash_to_credits!(100)), &platform_version),
            Err(ProtocolError::CorruptedCodeExecution(_))
        ));
    }

    #[test]
    fn should_fail_when_the_cap_is_below_one_maximal_withdrawal() {
        let mut platform_version = platform_version_with(Some(15));
        platform_version.system_limits.max_daily_withdrawal_amount =
            Some(dash_to_credits!(500) - 1);

        // Whatever the total, a cap below the floor is a contradictory configuration.
        for total in [0, dash_to_credits!(100), dash_to_credits!(30000)] {
            assert!(matches!(
                daily_withdrawal_limit_v2(Some(total), &platform_version),
                Err(ProtocolError::CorruptedCodeExecution(_))
            ));
        }

        // Exactly the floor is allowed and the limit is that floor.
        platform_version.system_limits.max_daily_withdrawal_amount = Some(dash_to_credits!(500));
        assert_eq!(
            daily_withdrawal_limit_v2(Some(dash_to_credits!(30000)), &platform_version)
                .expect("expected limit"),
            dash_to_credits!(500)
        );
    }
}
