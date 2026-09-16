use crate::fee::Credits;

/// Flat daily withdrawal limit of 2000 Dash, matching the limit in Core v22
/// (`LimitAmountV22`). In force from protocol version 8 to protocol version 13;
/// superseded by the relative limit of version 2.
pub const fn daily_withdrawal_limit_v1() -> Credits {
    // 2000 Dash
    200_000_000_000_000
}
