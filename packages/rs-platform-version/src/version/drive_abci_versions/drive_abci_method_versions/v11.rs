use crate::version::drive_abci_versions::drive_abci_method_versions::v10::DRIVE_ABCI_METHOD_VERSIONS_V10;
use crate::version::drive_abci_versions::drive_abci_method_versions::{
    DriveAbciMethodVersions, DriveAbciProtocolUpgradeMethodVersions,
};

/// Drive ABCI method versions 11. Introduced in protocol version 17 (5.0).
///
/// One slot changes over `DRIVE_ABCI_METHOD_VERSIONS_V10`:
///
/// * `protocol_upgrade.perform_events_on_first_block_of_protocol_change` becomes `Some(3)`:
///   the generation that, when the chain crosses protocol version 17, creates the compilation
///   readiness subtree under `[Votes]` and the readiness fund tree under
///   `[PreFundedSpecializedBalances]` on upgraded nodes through the same helper genesis uses,
///   on top of everything generation 2 does.
///
/// Everything else matches v10.
pub const DRIVE_ABCI_METHOD_VERSIONS_V11: DriveAbciMethodVersions = DriveAbciMethodVersions {
    protocol_upgrade: DriveAbciProtocolUpgradeMethodVersions {
        perform_events_on_first_block_of_protocol_change: Some(3), // changed in v17: creates the readiness structures on upgrade
        ..DRIVE_ABCI_METHOD_VERSIONS_V10.protocol_upgrade
    },
    ..DRIVE_ABCI_METHOD_VERSIONS_V10
};
