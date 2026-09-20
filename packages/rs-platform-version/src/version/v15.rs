use crate::version::drive_abci_versions::drive_abci_method_versions::v11::DRIVE_ABCI_METHOD_VERSIONS_V11;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::drive_versions::v10::DRIVE_VERSION_V10;
use crate::version::protocol_version::PlatformVersion;
use crate::version::v14::PLATFORM_V14;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_15: ProtocolVersion = 15;

/// Protocol v15 moves keep-history documents to per-type history trees.
///
/// The primary-key entry stores the current document while retained revisions
/// live in a separate provable count tree keyed by block time and revision.
/// The first v15 block migrates existing v14 histories and index references
/// before v15 state transitions execute. History queries and proofs select the
/// matching layout through Drive's version tables.
pub const PLATFORM_V15: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_15,
    drive: DRIVE_VERSION_V10, // changed: document v5 and contract v5 write the per-type history tree; verify v3 authenticates history pages and lifecycle counts
    drive_abci: DriveAbciVersion {
        methods: DRIVE_ABCI_METHOD_VERSIONS_V11, // changed: the protocol-change hook v2 migrates retained histories on the first v15 block
        ..PLATFORM_V14.drive_abci
    },
    ..PLATFORM_V14
};
