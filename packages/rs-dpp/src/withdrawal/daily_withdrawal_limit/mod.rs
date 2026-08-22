use crate::fee::Credits;
use crate::withdrawal::daily_withdrawal_limit::v0::daily_withdrawal_limit_v0;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;
mod v1;
mod v2;

pub fn daily_withdrawal_limit(
    total_credits_in_platform: Credits,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    match platform_version.dpp.methods.daily_withdrawal_limit {
        0 => Ok(daily_withdrawal_limit_v0(total_credits_in_platform)),
        1 => Ok(v1::daily_withdrawal_limit_v1()),
        2 => Ok(v2::daily_withdrawal_limit_v2()),
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
    fn should_double_flat_daily_withdrawal_limit_at_protocol_version_14() {
        let v13 = PlatformVersion::get(13).expect("expected protocol version 13");
        let v14 = PlatformVersion::get(14).expect("expected protocol version 14");

        // Both flat limits are independent of the credits held in Platform.
        for total_credits in [
            dash_to_credits!(50),
            dash_to_credits!(5000),
            dash_to_credits!(1000000),
        ] {
            assert_eq!(
                daily_withdrawal_limit(total_credits, v13).expect("expected v13 limit"),
                dash_to_credits!(2000)
            );
            assert_eq!(
                daily_withdrawal_limit(total_credits, v14).expect("expected v14 limit"),
                dash_to_credits!(4000)
            );
        }
    }
}
