use crate::fee::Credits;
use platform_version::version::PlatformVersion;

/// Flat daily withdrawal limit, read from the protocol version's system limits
/// (`SystemLimits::daily_withdrawal_limit`): 2000 Dash up to protocol version 13
/// (the limit in Core v22), 4000 Dash from protocol version 14 (Core v24).
pub fn daily_withdrawal_limit_v1(platform_version: &PlatformVersion) -> Credits {
    platform_version.system_limits.daily_withdrawal_limit
}
