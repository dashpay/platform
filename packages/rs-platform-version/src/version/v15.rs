use crate::version::protocol_version::PlatformVersion;
use crate::version::v14::PLATFORM_V14;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_15: ProtocolVersion = 15;

/// Placeholder for the 4.3 protocol version.
///
/// The DashVM allocation register reserves protocol version 15 for the 4.3 release, 16 for 4.4
/// and 17 for 5.0, and the registry is indexed by number, so the 5.0 version cannot exist on
/// this branch without 15 and 16. Until the 4.3 branch merges its real `v15.rs` forward this
/// version is identical to v14.
///
/// It is written as a struct update rather than a copy of `v14.rs` on purpose: when the real
/// file arrives the add/add conflict is resolved by taking the incoming file, and because v16
/// and v17 are struct updates over their predecessor every table the incoming version changes
/// flows into them without a second edit, except the two tables v17 overrides (`fee_version`
/// and `system_limits`, whose 5.0 generations are built on the tables current when they were
/// written). If the incoming version changes either of those, rebase the 5.0 generation onto
/// the incoming one and renumber it; the inheritance test in `system_limits` fails until then.
pub const PLATFORM_V15: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_15,
    ..PLATFORM_V14
};
