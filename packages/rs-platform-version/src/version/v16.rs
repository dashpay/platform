use crate::version::protocol_version::PlatformVersion;
use crate::version::v15::PLATFORM_V15;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_16: ProtocolVersion = 16;

/// Placeholder for the 4.4 protocol version.
///
/// Reserved by the DashVM allocation register (15 for 4.3, 16 for 4.4, 17 for 5.0). Identical to
/// v15 until the 4.4 branch merges its real `v16.rs` forward; see `v15.rs` for why it is a
/// struct update and how the forward merge is resolved. Should 4.4 ship no consensus change, the
/// register drops this activation and the 5.0 version becomes 16.
pub const PLATFORM_V16: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_16,
    ..PLATFORM_V15
};
