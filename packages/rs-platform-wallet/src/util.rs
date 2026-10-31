//! Small crate-internal helpers shared across wallet modules.

use dpp::balances::credits::CREDITS_PER_DUFF;
use dpp::fee::Credits;

/// The smallest whole-duff amount that pays `credits`.
pub(crate) fn ceil_credits_to_duffs(credits: Credits) -> u64 {
    credits / CREDITS_PER_DUFF + u64::from(!credits.is_multiple_of(CREDITS_PER_DUFF))
}

/// Current wall-clock time in milliseconds since the Unix epoch.
///
/// Used only for display-only timestamps and cost rate-limiting (never as a
/// correctness-bearing ordering key), so a clock that is briefly behind or a
/// pre-epoch read (falling back to `0`) is harmless everywhere it is called.
pub(crate) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Current wall-clock time in seconds since the Unix epoch.
///
/// Pre-epoch reads return `0`, which upstream never expires.
pub(crate) fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
