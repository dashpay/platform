use crate::fee::Credits;

/// Set constant withdrawal daily limit to 4000 Dash
/// that corresponds to the limit in Core v24 (`LimitAmountV24`, activated by
/// Core's `DEPLOYMENT_V24` hard fork; DIP-0165).
pub const fn daily_withdrawal_limit_v2() -> Credits {
    // 4000 Dash
    400_000_000_000_000
}
