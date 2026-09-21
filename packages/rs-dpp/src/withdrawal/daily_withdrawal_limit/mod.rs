use crate::fee::Credits;
use crate::withdrawal::daily_withdrawal_limit::v0::daily_withdrawal_limit_v0;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;
mod v1;
mod v2;

/// Returns the maximum amount of credits Platform may pool into asset unlock
/// transactions per 24 hours.
///
/// `reference_total_credits` is the base the limit is derived from: the current
/// total credits in Platform for version 0 (10% of it, bounded; required), ignored
/// by version 1 (a flat 2000 Dash), and the total credits Platform held a day ago
/// for version 2 (`daily_withdrawal_limit_percent` of it, never below one maximal
/// withdrawal, and never above `max_daily_withdrawal_amount` if one is set).
/// The most that may leave Platform per window, from a reference balance: up to protocol
/// version 13 Platform's own total credits (version 0) or nothing at all (version 1, a flat
/// 2000 Dash); from protocol version 14 (version 2) the balance of Core's credit pool one window
/// before the chain locked height, which `calculate_current_withdrawal_limit` always supplies,
/// so the `None` branch of version 2 is not reached in block processing.
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
            (dash_to_credits!(10000), dash_to_credits!(1500)),
            (dash_to_credits!(20000), dash_to_credits!(3000)),
            // No absolute cap: the share scales with the pool (Core allows 20% of it).
            (dash_to_credits!(30000), dash_to_credits!(4500)),
            (dash_to_credits!(1000000), dash_to_credits!(150000)),
        ] {
            // v13 keeps the flat 2000 Dash whatever the total is.
            assert_eq!(
                daily_withdrawal_limit(Some(total_credits_a_day_ago), v13)
                    .expect("expected v13 limit"),
                dash_to_credits!(2000)
            );
            // v14 allows 15% of the reference balance (Core's credit pool one window ago).
            assert_eq!(
                daily_withdrawal_limit(Some(total_credits_a_day_ago), v14)
                    .expect("expected v14 limit"),
                expected_v14
            );
        }

        // Without a reference balance v14 keeps v13's flat limit (unreachable in block processing).
        assert_eq!(
            daily_withdrawal_limit(None, v14).expect("expected v14 bootstrap limit"),
            dash_to_credits!(2000)
        );
    }
}
