use crate::fee::Credits;

/// Set constant withdrawal daily limit to 2000 Dash
/// that corresponds to the limit in Core v22.
pub const fn daily_withdrawal_limit_v1() -> Credits {
    // 2000 Dash
    200_000_000_000_000
}
