use crate::fee::Credits;
use crate::withdrawal::daily_withdrawal_limit::v0::daily_withdrawal_limit_v0;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;
mod v1;
mod v2;
mod v3;

/// Returns the maximum amount of credits Platform may pool into asset unlock
/// transactions per 24 hours.
///
/// `reference_total_credits` is the base the limit is derived from: the current
/// total credits in Platform for version 0 (10% of it, bounded; required), ignored
/// by version 1 (a flat 2000 Dash), and the total credits Platform held a day ago
/// for version 2 (`daily_withdrawal_limit_percent` of it, never below one maximal
/// withdrawal; the flat limit of version 1 while that day-old total is not known
/// yet) and version 3 (the same, never below `daily_withdrawal_limit_floor`
/// instead).
pub fn daily_withdrawal_limit(
    reference_total_credits: Option<Credits>,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    match platform_version.dpp.methods.daily_withdrawal_limit {
        0 => reference_total_credits
            .map(daily_withdrawal_limit_v0)
            .ok_or_else(|| {
                ProtocolError::CorruptedCodeExecution(
                    "daily_withdrawal_limit v0 requires the current total credits in Platform"
                        .to_string(),
                )
            }),
        1 => Ok(v1::daily_withdrawal_limit_v1()),
        2 => v2::daily_withdrawal_limit_v2(reference_total_credits, platform_version),
        3 => v3::daily_withdrawal_limit_v3(reference_total_credits, platform_version),
        v => Err(ProtocolError::UnknownVersionError(format!(
            "Unknown daily_withdrawal_limit version {v}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dash_to_credits;

    #[test]
    fn should_switch_from_flat_to_relative_daily_withdrawal_limit_at_protocol_version_14() {
        let v13 = PlatformVersion::get(13).expect("expected protocol version 13");
        let v14 = PlatformVersion::get(14).expect("expected protocol version 14");

        for (total_credits_a_day_ago, expected_v14) in [
            // Below one maximal withdrawal (500 Dash) the limit is floored there.
            (dash_to_credits!(50), dash_to_credits!(500)),
            (dash_to_credits!(2000), dash_to_credits!(500)),
            (dash_to_credits!(20000), dash_to_credits!(3000)),
            // No fixed cap: what Core will mine is bounded by the Core-anchored limit instead.
            (dash_to_credits!(30000), dash_to_credits!(4500)),
            (dash_to_credits!(1000000), dash_to_credits!(150000)),
        ] {
            // v13 keeps the flat 2000 Dash whatever the total is.
            assert_eq!(
                daily_withdrawal_limit(Some(total_credits_a_day_ago), v13)
                    .expect("expected v13 limit"),
                dash_to_credits!(2000)
            );
            // v14 allows 15% of the total credits a day ago.
            assert_eq!(
                daily_withdrawal_limit(Some(total_credits_a_day_ago), v14)
                    .expect("expected v14 limit"),
                expected_v14
            );
        }

        // Until the total credits a day ago are known, v14 keeps v13's flat limit.
        assert_eq!(
            daily_withdrawal_limit(None, v14).expect("expected v14 bootstrap limit"),
            dash_to_credits!(2000)
        );
    }

    /// Protocol version 16 raises the largest withdrawal from 500 to 4000 Dash. Version 2,
    /// which protocol version 15 selects, floors the daily limit at the largest withdrawal;
    /// version 3 keeps the floor at 500 Dash, so the limit does not move at any total.
    #[test]
    fn should_keep_the_daily_withdrawal_limit_when_protocol_version_16_raises_the_largest_withdrawal(
    ) {
        let v15 = PlatformVersion::get(15).expect("expected protocol version 15");
        let latest = PlatformVersion::latest();
        assert!(
            latest.system_limits.max_withdrawal_amount > v15.system_limits.max_withdrawal_amount
        );

        for (total_credits_a_day_ago, expected) in [
            (Some(0), dash_to_credits!(500)),
            (Some(dash_to_credits!(2000)), dash_to_credits!(500)),
            (Some(dash_to_credits!(10000)), dash_to_credits!(1500)),
            (Some(dash_to_credits!(20000)), dash_to_credits!(3000)),
            (Some(dash_to_credits!(30000)), dash_to_credits!(4500)),
            (Some(dash_to_credits!(1000000)), dash_to_credits!(150000)),
            (None, dash_to_credits!(2000)),
        ] {
            assert_eq!(
                daily_withdrawal_limit(total_credits_a_day_ago, v15).expect("expected v15 limit"),
                expected
            );
            assert_eq!(
                daily_withdrawal_limit(total_credits_a_day_ago, latest)
                    .expect("expected latest limit"),
                expected
            );
        }
    }
}
