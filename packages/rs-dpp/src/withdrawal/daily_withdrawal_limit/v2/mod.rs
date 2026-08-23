use crate::fee::Credits;
use crate::withdrawal::daily_withdrawal_limit::v1::daily_withdrawal_limit_v1;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

/// Relative daily withdrawal limit: `daily_withdrawal_limit_percent` (from the
/// protocol version's system limits) of the total credits Platform held a day ago.
/// Using a day-old base means a sudden jump in the total credits does not raise
/// the limit for a day.
///
/// Two guards keep it usable:
/// * it is never below `max_withdrawal_amount`, so every withdrawal Platform
///   accepts eventually fits the daily maximum and cannot block the pooling
///   queue behind it;
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

    // u128 keeps `total * percent` from overflowing for any u64 total.
    let relative_limit = (total_credits_a_day_ago as u128) * (percent as u128) / 100;
    let relative_limit = Credits::try_from(relative_limit)
        .map_err(|_| ProtocolError::Overflow("daily withdrawal limit overflow"))?;

    Ok(relative_limit.max(platform_version.system_limits.max_withdrawal_amount))
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
        platform_version
    }

    #[test]
    fn should_return_the_configured_percent_of_the_lagged_total() {
        let platform_version = platform_version_with(Some(15));

        assert_eq!(
            daily_withdrawal_limit_v2(Some(dash_to_credits!(30000)), &platform_version)
                .expect("expected limit"),
            dash_to_credits!(4500)
        );
        assert_eq!(
            daily_withdrawal_limit_v2(Some(Credits::MAX), &platform_version)
                .expect("expected limit"),
            ((Credits::MAX as u128) * 15 / 100) as Credits
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
    fn should_keep_the_flat_limit_until_the_lagged_total_is_known() {
        let platform_version = platform_version_with(Some(15));

        assert_eq!(
            daily_withdrawal_limit_v2(None, &platform_version).expect("expected limit"),
            dash_to_credits!(2000)
        );
    }

    #[test]
    fn should_fail_when_the_percent_is_not_configured() {
        let platform_version = platform_version_with(None);

        assert!(matches!(
            daily_withdrawal_limit_v2(Some(dash_to_credits!(100)), &platform_version),
            Err(ProtocolError::CorruptedCodeExecution(_))
        ));
    }
}
