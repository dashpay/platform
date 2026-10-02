use crate::version::system_limits::v4::SYSTEM_LIMITS_V4;
use crate::version::system_limits::SystemLimits;

/// System limits for protocol version 17 (5.0) and above.
///
/// Identical to [`SYSTEM_LIMITS_V4`] except that the additional wait between a compilation
/// readiness crossing and the activation of the bundle is bounded: `clamp(T, 2 minutes, 1 hour)`
/// where `T` is the time the round took to cross. Both bounds are confirmed policy; the block
/// event that applies them reads these fields.
pub const SYSTEM_LIMITS_V5: SystemLimits = SystemLimits {
    readiness_additional_wait_min_ms: Some(120_000), // two minutes, confirmed policy
    readiness_additional_wait_max_ms: Some(3_600_000), // one hour, confirmed policy
    ..SYSTEM_LIMITS_V4
};

// Whatever the revision of the numbers above is, the wait must stay a non-empty interval.
const _: () = assert!(
    match (
        SYSTEM_LIMITS_V5.readiness_additional_wait_min_ms,
        SYSTEM_LIMITS_V5.readiness_additional_wait_max_ms,
    ) {
        (Some(min), Some(max)) => min > 0 && min <= max,
        _ => false,
    },
    "SYSTEM_LIMITS_V5 must carry a non-empty readiness wait interval"
);
