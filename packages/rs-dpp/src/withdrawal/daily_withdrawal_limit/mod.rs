use crate::fee::Credits;
use crate::withdrawal::daily_withdrawal_limit::v0::daily_withdrawal_limit_v0;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;
mod v1;

pub fn daily_withdrawal_limit(
    total_credits_in_platform: Credits,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    match platform_version.dpp.methods.daily_withdrawal_limit {
        0 => Ok(daily_withdrawal_limit_v0(total_credits_in_platform)),
        1 => Ok(v1::daily_withdrawal_limit_v1()),
        v => Err(ProtocolError::UnknownVersionError(format!(
            "Unknown daily_withdrawal_limit version {v}"
        ))),
    }
}
