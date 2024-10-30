use crate::version::drive_abci_versions::drive_abci_method_versions::v4::DRIVE_ABCI_METHOD_VERSIONS_V4;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::protocol_version::PlatformVersion;
use crate::version::v4::PLATFORM_V4;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_5: ProtocolVersion = 5;

/// This version added a fix to withdrawals so we would rotate to first quorum always.

pub const PLATFORM_V5: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_5,
    drive_abci: DriveAbciVersion {
        methods: DRIVE_ABCI_METHOD_VERSIONS_V4, // changed to v4
        ..PLATFORM_V4.drive_abci
    },
    ..PLATFORM_V4
};
