use crate::fee::Credits;
use crate::withdrawal::daily_withdrawal_limit::v1::daily_withdrawal_limit_v1;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

/// Relative daily withdrawal limit: `daily_withdrawal_limit_percent` (from the protocol
/// version's system limits) of the total credits Platform held a day ago, as in version 2.
///
/// Version 3 differs in the floor. Version 2 never goes below one maximal withdrawal
/// (`max_withdrawal_amount`), which kept every accepted withdrawal poolable while pooling
/// stopped at the first withdrawal that did not fit. Protocol version 16 raises the largest
/// withdrawal to 4000 Dash; that floor would raise the daily limit of a Platform holding less
/// than about 26,667 Dash from 15% of its credits to 4000 Dash with it. Version 3 reads its
/// own floor, `daily_withdrawal_limit_floor`, which stays where the old one was, so the limit
/// is the same number at every total. A withdrawal can therefore be larger than the daily
/// maximum; it waits until the total has grown, and pooling
/// (`pool_withdrawals_into_transactions_queue` 3) passes over it meanwhile.
///
/// While the total credits a day ago are not known (`None`: the history is younger than a
/// day), the flat limit of version 1 applies, as in version 2.
pub fn daily_withdrawal_limit_v3(
    total_credits_in_platform_a_day_ago: Option<Credits>,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    let Some(total_credits_a_day_ago) = total_credits_in_platform_a_day_ago else {
        return Ok(daily_withdrawal_limit_v1());
    };

    let system_limits = &platform_version.system_limits;

    let percent = system_limits
        .daily_withdrawal_limit_percent
        .ok_or_else(|| {
            ProtocolError::CorruptedCodeExecution(
                "daily_withdrawal_limit v3 requires system_limits.daily_withdrawal_limit_percent"
                    .to_string(),
            )
        })?;

    let floor = system_limits.daily_withdrawal_limit_floor.ok_or_else(|| {
        ProtocolError::CorruptedCodeExecution(
            "daily_withdrawal_limit v3 requires system_limits.daily_withdrawal_limit_floor"
                .to_string(),
        )
    })?;

    // u128 keeps `total * percent` from overflowing for any u64 total.
    let relative_limit = (total_credits_a_day_ago as u128) * (percent as u128) / 100;
    let relative_limit = Credits::try_from(relative_limit)
        .map_err(|_| ProtocolError::Overflow("daily withdrawal limit overflow"))?;

    Ok(relative_limit.max(floor))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dash_to_credits;

    #[test]
    fn should_return_the_configured_percent_of_the_lagged_total() {
        let platform_version = PlatformVersion::latest();

        assert_eq!(
            daily_withdrawal_limit_v3(Some(dash_to_credits!(20000)), platform_version)
                .expect("expected limit"),
            dash_to_credits!(3000)
        );
        // No fixed cap.
        assert_eq!(
            daily_withdrawal_limit_v3(Some(dash_to_credits!(1000000)), platform_version)
                .expect("expected limit"),
            dash_to_credits!(150000)
        );
    }

    #[test]
    fn should_floor_at_its_own_floor_below_one_maximal_withdrawal() {
        let platform_version = PlatformVersion::latest();
        let floor = platform_version
            .system_limits
            .daily_withdrawal_limit_floor
            .expect("expected a daily withdrawal limit floor");
        assert!(floor < platform_version.system_limits.max_withdrawal_amount);

        // 15% of 2000 Dash is 300 Dash, below the floor.
        assert_eq!(
            daily_withdrawal_limit_v3(Some(dash_to_credits!(2000)), platform_version)
                .expect("expected limit"),
            floor
        );
        assert_eq!(
            daily_withdrawal_limit_v3(Some(0), platform_version).expect("expected limit"),
            floor
        );
        // 15% of 10,000 Dash is 1500 Dash: above the floor, below one maximal withdrawal.
        assert_eq!(
            daily_withdrawal_limit_v3(Some(dash_to_credits!(10000)), platform_version)
                .expect("expected limit"),
            dash_to_credits!(1500)
        );
    }

    #[test]
    fn should_apply_the_flat_limit_until_the_lagged_total_is_known() {
        assert_eq!(
            daily_withdrawal_limit_v3(None, PlatformVersion::latest()).expect("expected limit"),
            daily_withdrawal_limit_v1()
        );
    }

    #[test]
    fn should_fail_without_a_configured_percent_or_floor() {
        let mut without_percent = PlatformVersion::latest().clone();
        without_percent.system_limits.daily_withdrawal_limit_percent = None;
        assert!(matches!(
            daily_withdrawal_limit_v3(Some(dash_to_credits!(20000)), &without_percent),
            Err(ProtocolError::CorruptedCodeExecution(_))
        ));

        let mut without_floor = PlatformVersion::latest().clone();
        without_floor.system_limits.daily_withdrawal_limit_floor = None;
        assert!(matches!(
            daily_withdrawal_limit_v3(Some(dash_to_credits!(20000)), &without_floor),
            Err(ProtocolError::CorruptedCodeExecution(_))
        ));
    }
}
