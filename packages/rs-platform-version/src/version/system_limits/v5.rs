use crate::version::system_limits::v4::SYSTEM_LIMITS_V4;
use crate::version::system_limits::SystemLimits;

/// System limits for protocol version 15. Relative to `SYSTEM_LIMITS_V4`, which protocol
/// version 14 runs on:
///
/// * Contracts stored compiled: a contract stored compiled (format version 2) takes at most
///   1 MiB (`max_compiled_contract_size`, `None` before). Registration, update and a token
///   configuration update refuse a contract over it, Drive refuses to store one, and proof
///   verification refuses a compiled contract over it.
/// * Frozen listings: a `TokenDestroyFrozenFunds` destroys at most 16 open listings of the
///   frozen identity with its balance (`max_frozen_listings_destroyed_per_transition`, `None`
///   before).
pub const SYSTEM_LIMITS_V5: SystemLimits = SystemLimits {
    max_compiled_contract_size: Some(1_048_576), // changed: contracts stored compiled take at most 1 MiB
    max_frozen_listings_destroyed_per_transition: Some(16), // changed: a frozen identity's destroy takes at most 16 listings
    ..SYSTEM_LIMITS_V4
};
