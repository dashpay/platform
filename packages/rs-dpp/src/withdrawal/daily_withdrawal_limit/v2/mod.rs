use crate::fee::Credits;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

/// Relative daily withdrawal limit: `daily_withdrawal_limit_percent` (from the
/// protocol version's system limits) of the total credits Platform held a day
/// ago. The caller passes that lagged total as `total_credits_in_platform_a_day_ago`;
/// using a day-old base means a sudden jump in the total credits does not raise
/// the limit for a day.
pub fn daily_withdrawal_limit_v2(
    total_credits_in_platform_a_day_ago: Credits,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
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
    let limit = (total_credits_in_platform_a_day_ago as u128) * (percent as u128) / 100;

    Credits::try_from(limit).map_err(|_| ProtocolError::Overflow("daily withdrawal limit overflow"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dash_to_credits;

    #[test]
    fn should_return_the_configured_percent_of_the_lagged_total() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .system_limits
            .daily_withdrawal_limit_percent = Some(15);

        assert_eq!(
            daily_withdrawal_limit_v2(dash_to_credits!(30000), &platform_version)
                .expect("expected limit"),
            dash_to_credits!(4500)
        );
        // Rounds down to whole credits.
        assert_eq!(
            daily_withdrawal_limit_v2(7, &platform_version).expect("expected limit"),
            1
        );
        assert_eq!(
            daily_withdrawal_limit_v2(Credits::MAX, &platform_version).expect("expected limit"),
            ((Credits::MAX as u128) * 15 / 100) as Credits
        );
    }

    #[test]
    fn should_fail_when_the_percent_is_not_configured() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .system_limits
            .daily_withdrawal_limit_percent = None;

        assert!(matches!(
            daily_withdrawal_limit_v2(dash_to_credits!(100), &platform_version),
            Err(ProtocolError::CorruptedCodeExecution(_))
        ));
    }
}
